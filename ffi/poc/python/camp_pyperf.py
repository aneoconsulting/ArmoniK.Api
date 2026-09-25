"""CAMPAIGN.md 22a: the codec suite on pyperf (owner, 2026-09-25).

  python3.12 camp_pyperf.py --family shapes|unknown --launch N [--only a,b] --side DIR \\
      -o launchN.json --processes 1 --values ROUNDS --warmups W --affinity CPUS --copy-env
  (run_campaign.sh builds this command; camp_pyperf_export.py turns the JSON into section 7)

pyperf's model, mapped onto requirement 23 (run_campaign.sh):
  launch   one pyperf invocation per launch (>= 3), the benchmark order ROTATED by the launch
           number, so no arm is always first
  round    one pyperf value; `--values ROUNDS` (>= 5) per worker process
  process  pyperf spawns a fresh worker per benchmark (`--processes 1` per launch), plus its
           loop-calibration worker; each worker imports the arms, applies the allocator
           warm-up (M_TOP_PAD, J26) and is pinned by `--affinity` to AK_CPU_CLIENT
Warm-up and calibration are pyperf's: `--warmups W` values discarded per worker, `loops`
calibrated to `--min-time` in the calibration worker; both are in the JSON and are exported
as `phase` "warmup" / "calibration" lines, never dropped.

Clock: the time_func returns CLOCK_THREAD_CPUTIME_ID seconds, so pyperf's values ARE CPU
time per loop (requirement 21). Wall (perf_counter) for the same call is written to a side
file (--side) and joined back by the exporter. GC stays ON (bench_time_func does not touch it);
`gc.collect()` runs before every timed call, untimed.

A benchmark is `family|payload|content|dir|arm|mode`; a worker builds only its own payload's
cases (camp_codec's builders, with their correctness check) the first time it is called.
"""
import gc
import json
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "build", "pyperf"))

import pyperf  # noqa: E402

# WP5 step 10: `--variant nounk` times the no-unknown build, a separately built extension
# over ak-core without `unknown-fields`. The flag is passed on to every worker
# (add_cmdline_args) and sets AK_VARIANT before any shim is imported, so a worker process
# only ever loads one variant's libak_core.so (the two share a soname).
if "--variant" in sys.argv and sys.argv[sys.argv.index("--variant") + 1] == "nounk":
    os.environ["AK_VARIANT"] = "nounk"
VARIANT = os.environ.get("AK_VARIANT", "full")

ARMS = {
    "shapes": [("incumbent-prod", "incumbent-default"), ("incumbent-best", "incumbent-default"),
               ("core-ffi", "drop"), ("core-ffi", "retain"), ("core-ffi-attr", "drop"),
               ("host-gen", "drop"), ("host-gen", "retain")],
    "unknown": [("incumbent-prod", "incumbent-default"), ("core-ffi", "drop"), ("core-ffi", "retain"),
                ("host-gen", "drop"), ("host-gen", "retain")],
}
ARMS_NOUNK = {
    "shapes": [("incumbent-prod", "incumbent-default"), ("incumbent-best", "incumbent-default"),
               ("core-ffi", "no-unknown")],
    "unknown": [("incumbent-prod", "incumbent-default"), ("core-ffi", "no-unknown")],
}
DIRS = ["encode", "decode", "decode+read"]


def add_args(cmd, args):
    cmd.extend(["--family", args.family, "--launch", str(args.launch), "--side", args.side,
                "--variant", args.variant])
    if args.only:
        cmd.extend(["--only", args.only])


def payloads(family, only):
    if family == "shapes":
        import payload_values  # noqa: F401  (loads the schema only, not the core)
        import shapes as S
        pids = list(S.load()["payloads"])
        out = []
        for p in pids:
            out.append((p, "ascii"))
            if p == "P2.4":
                out += [(p, "latin1"), (p, "wide")]
    else:
        tag = "py%d.%d" % sys.version_info[:2]
        sys.path.insert(0, os.path.join(HERE, "build", tag))
        ffi = __import__("_akffi_corpus_nounk" if VARIANT == "nounk" else "_akffi_corpus")
        man = json.load(open(os.path.join(HERE, "..", "..", "corpus", "generated", "manifest.json")))["vectors"]
        roots = set(ffi.roots())
        out = [(k, "ascii") for k, r in sorted(man.items()) if k.startswith("U-") and r["expect"] == "accept"
               and r.get("verdict") != "disputed" and r["root"] in roots]
    if only:
        keep = set(only.split(","))
        out = [x for x in out if x[0] in keep]
    return out


_CASES = {}


class _Quiet:
    def note(self, s):
        pass


def case(family, pid, content, d, arm, mode):
    key = (pid,)
    if key not in _CASES:
        import camp_codec as CC
        cs, gates = (CC.shapes_cases if family == "shapes" else CC.unknown_cases)(_Quiet(), only=pid)
        if gates:
            raise SystemExit("correctness gate failed before timing: %s" % "; ".join(gates[:3]))
        _CASES[key] = {(c.content, c.dir, c.arm, c.mode): c for c in cs}
    return _CASES[key][(content, d, arm, mode)]


def time_func(loops, family, pid, content, d, arm, mode, side, name):
    c = case(family, pid, content, d, arm, mode)
    gc.collect()
    t0, w0 = time.clock_gettime(time.CLOCK_THREAD_CPUTIME_ID), time.perf_counter()
    c.fn(loops)
    t1, w1 = time.clock_gettime(time.CLOCK_THREAD_CPUTIME_ID), time.perf_counter()
    cpu = t1 - t0
    with open(os.path.join(side, "side-%d.jsonl" % os.getpid()), "a") as f:
        f.write(json.dumps({"name": name, "loops": loops, "cpu_s": cpu, "wall_s": w1 - w0}) + "\n")
    return cpu


def main():
    runner = pyperf.Runner(add_cmdline_args=add_args)
    ap = runner.argparser
    ap.add_argument("--family", default="shapes")
    ap.add_argument("--launch", type=int, default=1)
    ap.add_argument("--side", required=True)
    ap.add_argument("--only", default="")
    ap.add_argument("--variant", default="full", choices=["full", "nounk"])
    args = runner.parse_args()
    os.makedirs(args.side, exist_ok=True)
    names = []
    for pid, content in payloads(args.family, args.only):
        for d in DIRS:
            for arm, mode in (ARMS_NOUNK if VARIANT == "nounk" else ARMS)[args.family]:
                names.append((pid, content, d, arm, mode))
    # The order rotated between launches: launch l starts (l-1)/3 of the way through the list.
    k = ((args.launch - 1) * max(1, len(names) // 3)) % len(names) if names else 0
    names = names[k:] + names[:k]
    for pid, content, d, arm, mode in names:
        name = "%s|%s|%s|%s|%s|%s" % (args.family, pid, content, d, arm, mode)
        runner.bench_time_func(name, time_func, args.family, pid, content, d, arm, mode, args.side, name)


if __name__ == "__main__":
    main()
