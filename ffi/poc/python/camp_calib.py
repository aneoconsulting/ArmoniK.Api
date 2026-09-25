"""CAMPAIGN.md section 4.3: crossing counts (a gate) and crossing cost, pinned to AK_CPU_CLIENT.

  python3.12 camp_calib.py --launch N --rounds R --out FILE [--rust-log FILE] [--allow-dirty] [--smoke]

19  Crossing counts from the counting build (`_akffi_count`), every payload and direction,
    both halves (shim -> CPython, core forward and reverse), compared line for line with
    `counts_expected.txt` (committed; logs/python/91). A difference STOPS the run: the log
    carries the diff and no sample.
20  Crossing cost, in THIS host: the shim's `crossing(n, "forward")` (ak_noop) and
    `crossing(n, "reverse")` (ak_noop_reverse into a host function: a forward call that
    makes one reverse call, so reverse alone is the difference, left to the summary), each
    a C loop timed with CLOCK_THREAD_CPUTIME_ID per round. With `perf` present, `perf stat`
    cycles and instructions per iteration are taken for each (a loop of n minus a loop of 0,
    in separate processes) and recorded as samples with `perf_cycles`/`perf_instructions`.
    And the RUST slice's crossing benchmark (poc/rust `bench`, AK_BENCH_ONLY=P1.1, which
    keeps its crossing rows), built from the same snapshot and run pinned; its raw output
    goes to --rust-log, unmodified.
"""
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import camp_lib as L  # noqa: E402

AFFINITY = L.pin("AK_CPU_CLIENT")
ARGS = sys.argv[1:]


def opt(name, default=None, conv=str):
    return conv(ARGS[ARGS.index(name) + 1]) if name in ARGS else default


def counts_gate():
    env = dict(os.environ, AK_USE_COUNT="1")
    r = subprocess.run([sys.executable, os.path.join(HERE, "conformance.py"), "--counts-only"],
                       env=env, capture_output=True, text=True)
    got = [ln.strip() for ln in r.stdout.splitlines() if ln.strip().startswith("P") and "shim" in ln]
    want = [ln.strip() for ln in open(os.path.join(HERE, "counts_expected.txt")) if ln.strip()]
    diff = [("- " + a, "+ " + b) for a, b in zip(want, got) if a != b]
    if len(got) != len(want):
        diff.append(("rows", "%d measured, %d expected" % (len(got), len(want))))
    return got, diff


def perf_per_iter(kind, n):
    if not shutil.which("perf"):
        return None

    def one(k):
        code = ("import sys; sys.path.insert(0, %r); import camp_lib as L; L.pin('AK_CPU_CLIENT'); "
                "sys.path.insert(0, %r); import _akffi as m; m.crossing(%d, %r)"
                % (HERE, os.path.join(HERE, "build", "py%d.%d" % sys.version_info[:2]), k, kind))
        r = subprocess.run(["perf", "stat", "-x,", "-e", "cycles,instructions", sys.executable, "-c", code],
                           capture_output=True, text=True)
        vals = {}
        for ln in r.stderr.splitlines():
            parts = ln.split(",")
            if len(parts) > 2 and parts[0].strip().isdigit():
                vals[parts[2]] = int(parts[0])
        return vals
    a, b = one(n), one(0)
    if "cycles" not in a or "cycles" not in b:
        return None
    return {"perf_cycles": (a["cycles"] - b["cycles"]) / n,
            "perf_instructions": (a.get("instructions", 0) - b.get("instructions", 0)) / n}


def rust_bench(path):
    """Build and run the rust slice's crossing benchmark from the same source tree."""
    snap = os.environ.get("AK_SNAPSHOT_DIR")
    root = os.path.join(snap, "ffi", "poc", "rust") if snap else os.path.join(L.FFI, "poc", "rust")
    tgt = os.path.join(HERE, "build", "cargo-rustbench")
    out = ["# the rust slice's crossing benchmark (CAMPAIGN.md 20; README R13), run on this machine",
           "# source: %s" % root]
    if not os.path.isdir(root):
        out.append("# NOT RUN: %s does not exist (build.sh creates the snapshot)" % root)
        open(path, "w").write("\n".join(out) + "\n")
        return False
    b = subprocess.run(["cargo", "build", "--release", "-q", "-p", "harness", "--bin", "bench"],
                       cwd=root, env=dict(os.environ, CARGO_TARGET_DIR=tgt), capture_output=True, text=True)
    if b.returncode:
        out.append("# BUILD FAILED, not run:\n" + b.stderr[-3000:])
        open(path, "w").write("\n".join(out) + "\n")
        return False
    cmd = [os.path.join(tgt, "release", "bench")]
    cs = os.environ.get("AK_CPU_CLIENT")
    if cs and shutil.which("taskset"):
        cmd = ["taskset", "-c", cs] + cmd
    if shutil.which("perf"):
        cmd = ["perf", "stat", "-e", "cycles,instructions"] + cmd
    out.append("# command: AK_BENCH_ONLY=P1.1 %s" % " ".join(cmd))
    r = subprocess.run(cmd, env=dict(os.environ, AK_BENCH_ONLY="P1.1"), capture_output=True, text=True)
    out.append(r.stdout)
    out.append(r.stderr)
    out.append("# exit %d" % r.returncode)
    open(path, "w").write("\n".join(out) + "\n")
    return r.returncode == 0


def main():
    launch = opt("--launch", 1, int)
    rounds = opt("--rounds", 5, int)
    n = opt("--iters", 2000000, int)
    log = L.Log(opt("--out"), "calib", allow_dirty="--allow-dirty" in ARGS, smoke="--smoke" in ARGS)
    log.header(launch=launch, rounds=rounds, iters=n, affinity=AFFINITY,
               clock="CLOCK_THREAD_CPUTIME_ID around a C loop in the shim", perf=shutil.which("perf") or "absent")
    got, diff = counts_gate()
    for ln in got:
        log.note("count " + ln)
    if diff:
        log.close(False, "crossing counts differ from counts_expected.txt: %r" % diff[:5])
        print("COUNTS DIFFER:", diff[:5])
        return 1
    log.note("crossing counts: %d rows, identical to counts_expected.txt" % len(got))
    sys.path.insert(0, os.path.join(HERE, "build", "py%d.%d" % sys.version_info[:2]))
    import _akffi as m
    m.crossing(n // 10, "forward")
    m.crossing(n // 10, "reverse")
    kinds = ["forward", "reverse"]
    for r in range(rounds):
        for kind in L.rotated(kinds, r):
            t0, w0 = L.thread_cpu_ns(), L.wall_ns()
            m.crossing(n, kind)
            t1, w1 = L.thread_cpu_ns(), L.wall_ns()
            log.sample(arm="crossing-" + ("forward" if kind == "forward" else "fwd+reverse"),
                       launch=launch, round=r + 1, cpu_ns=t1 - t0, wall_ns=w1 - w0, iters=n)
    for kind in kinds:
        p = perf_per_iter(kind, n)
        if p:
            log.sample(arm="crossing-%s-perf" % kind, launch=launch, iters=n, **p)
        else:
            log.note("perf stat for %s: not taken (perf absent or no hardware counters here)" % kind)
    if opt("--rust-log"):
        ok = rust_bench(opt("--rust-log"))
        log.note("rust crossing benchmark: %s, raw output in %s" % ("ran" if ok else "FAILED", os.path.basename(opt("--rust-log"))))
    log.close(True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
