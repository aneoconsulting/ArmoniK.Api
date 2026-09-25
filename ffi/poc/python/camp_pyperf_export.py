"""pyperf JSON + the wall side files -> CAMPAIGN.md section 7 JSON lines (requirement 22a).

  python3.12 camp_pyperf_export.py --json launchN.json --side DIR --launch N --out FILE
         [--pyperf-args "..."] [--allow-dirty] [--smoke]

Every raw measurement pyperf holds is exported, nothing dropped: each run's calibration values
(`phase` "calibration"), its warm-up values (`phase` "warmup") and its values (`phase`
"value", `round` = the value's index in its worker, 1-based). `cpu_ns` = value x loops (the
time_func returned CLOCK_THREAD_CPUTIME_ID), `wall_ns` from the side file of the same call,
`iters` = loops. A value with no matching side record is a harness defect and stops the export.
"""
import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "build", "pyperf"))
import pyperf  # noqa: E402
import camp_lib as L  # noqa: E402

A = sys.argv[1:]


def opt(n, d=None):
    return A[A.index(n) + 1] if n in A else d


def main():
    launch = int(opt("--launch", "1"))
    side = {}
    for p in glob.glob(os.path.join(opt("--side"), "side-*.jsonl")):
        for ln in open(p):
            r = json.loads(ln)
            side.setdefault((r["name"], r["loops"], r["cpu_s"] / r["loops"]), []).append(r["wall_s"])
    suite = pyperf.BenchmarkSuite.load(opt("--json"))
    log = L.Log(opt("--out"), "codec", allow_dirty="--allow-dirty" in A, smoke="--smoke" in A)
    fam = None
    missing = 0
    for b in suite.get_benchmarks():
        name = b.get_name()
        f, pid, content, d, arm, mode = name.split("|")
        fam = f
        for run in b._runs:
            md = run.get_metadata()
            rnd = 0
            phase_w = "calibration" if run._is_calibration() else "warmup"
            for phase, pairs in ((phase_w, run.warmups or ()), ("value", [(md.get("loops", 1), v) for v in run.values])):
                for loops, v in pairs:
                    key = (name, loops, v)
                    w = side.get(key)
                    wall = w.pop(0) if w else None
                    if wall is None:
                        missing += 1
                    if phase == "value":
                        rnd += 1
                    log.sample(arm=arm, payload=pid, content=content, dir=d, unknown_mode=mode,
                               launch=launch, round=rnd if phase == "value" else None, phase=phase,
                               cpu_ns=int(round(v * loops * 1e9)),
                               wall_ns=int(round(wall * 1e9)) if wall is not None else None,
                               iters=loops)
    log.header(engine="pyperf %s (CAMPAIGN.md 22a)" % pyperf.__version__, family=fam, launch=launch,
               pyperf_args=opt("--pyperf-args", "?"),
               clock="pyperf values are CLOCK_THREAD_CPUTIME_ID per loop (the time_func's return); wall "
                     "(perf_counter) of the same call from the side file",
               warmup_calibration="pyperf's: warm-up values and the loop-calibration run are exported "
                                  "with phase warmup / calibration",
               gc="ON (bench_time_func leaves it); gc.collect() before every timed call, untimed",
               allocator="M_TOP_PAD in every worker at import (J26)",
               variant_build=("no-unknown variant (WP5 step 10): _akffi_nounk / _akffi_corpus_nounk over ak-core "
                      "--no-default-features; a SEPARATE build, compared with the full build only through "
                      "the incumbent rows timed in the same launch" if opt("--variant") == "nounk"
                      else "full build (unknown-fields on)"),
               core_ffi_unknown=("no-unknown (compiled out)" if opt("--variant") == "nounk" else
                                 "drop and retain (decision 11: every position armed; ak_uencode_* on encode)"),
               raw_json=os.path.basename(opt("--json")))
    if missing:
        log.close(False, "%d pyperf values have no side record (wall): harness defect" % missing)
        return 1
    log.close(True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
