"""CAMPAIGN.md requirement 30: the only summaries a slice produces.

  python3.12 camp_summary.py <dir with *.jsonl>  > summary.txt

Per (build, arm or cell, payload, content, direction, unknown mode, transport, in flight): the
median, minimum and maximum over every round of every launch of CPU and wall per iteration
(per call for RPC), and the ratio to the baseline -- `incumbent-prod` (codec: same build,
payload, content, direction) or cell A (RPC: same build, transport, direction, in flight) --
**formed from per-launch medians** (owner, 2026-09-26, R-H24): per launch, the median over that
launch's rounds of the arm divided by the median of the baseline; reported as the median and
range over launches. pyperf runs every benchmark in its own worker process, so no two values
share a process: every ratio here is cross-process, and it is labelled so.

Every sample carries `build` ("full" or "nounk", R-H1). References and groups are keyed on it,
so the no-unknown build's incumbent never stands in for the full build's, and no row is pooled
across builds. A sample with no `build` (logs older than R-H1) is read as "full".

No significance claim, no verdict word. Crossing-cost samples (calib) get their per-iteration
median and range only.
"""
import glob
import json
import os
import statistics
import sys


def load(d):
    rows, smoke = [], False
    for p in sorted(glob.glob(os.path.join(d, "*.jsonl"))):
        for ln in open(p):
            if ln.startswith("# INSTRUMENTATION"):
                smoke = True
            if ln.startswith("{"):
                rows.append(json.loads(ln))
    return rows, smoke


def rng(v):
    return "%.4g [%.4g, %.4g]" % (statistics.median(v), min(v), max(v)) if v else "-"


def build_of(r):
    return r.get("build", "full")


def group_key(suite, r):
    who = "cell" if suite == "rpc" else "arm"
    return (build_of(r), r.get(who), r.get("payload"), r.get("content"), r.get("dir"),
            r.get("unknown_mode"), r.get("transport"), r.get("inflight"))


def base_key(suite, r):
    """The baseline this row is divided by: same build, and same payload/content/direction
    (codec) or transport/direction/in flight (RPC). The launch is added by the caller."""
    if suite == "codec":
        return (build_of(r), r.get("payload"), r.get("content"), r.get("dir"))
    return (build_of(r), r.get("transport"), r.get("dir"), r.get("inflight"))


def is_base(suite, r):
    return (suite == "codec" and r.get("arm") == "incumbent-prod") or (suite == "rpc" and r.get("cell") == "A")


def per_iter(r):
    return r["cpu_ns"] / r["iters"]


def summarise(rows):
    """{suite: [(group key, cpu list, wall list, ratio list over launches)]}."""
    out = {}
    for suite in ("codec", "rpc", "calib"):
        # pyperf's warm-up and calibration measurements are exported (22a) but are not rounds.
        rs = [r for r in rows if r.get("suite") == suite and r.get("iters")
              and r.get("phase", "value") == "value" and "cpu_ns" in r]
        if not rs:
            continue
        # per-launch medians of the baseline
        bvals = {}
        for r in rs:
            if is_base(suite, r):
                bvals.setdefault(base_key(suite, r) + (r["launch"],), []).append(per_iter(r))
        bmed = {k: statistics.median(v) for k, v in bvals.items()}
        groups = {}
        for r in rs:
            groups.setdefault(group_key(suite, r), []).append(r)
        res = []
        for k in sorted(groups, key=lambda x: tuple(str(y) for y in x)):
            g = groups[k]
            cpu = [per_iter(r) for r in g]
            wall = [r["wall_ns"] / r["iters"] for r in g if r.get("wall_ns") is not None]
            rat = []
            if suite != "calib":
                bylaunch = {}
                for r in g:
                    bylaunch.setdefault(r["launch"], []).append(per_iter(r))
                for launch, v in sorted(bylaunch.items()):
                    b = bmed.get(base_key(suite, g[0]) + (launch,))
                    if b:
                        rat.append(statistics.median(v) / b)
            res.append((k, cpu, wall, rat))
        out[suite] = res
    return out


def main(d):
    rows, smoke = load(d)
    if smoke:
        print("# INSTRUMENTATION: summarised from a smoke run; no figure here is a result")
    for suite, res in summarise(rows).items():
        print("\n## %s: median [min, max] over all rounds and launches; ratio to the %s from per-launch "
              "medians, median [min, max] over launches, CROSS-PROCESS (pyperf: one worker per benchmark)"
              % (suite, "cell A" if suite == "rpc" else "incumbent-prod of the same build"))
        print("   %-70s %-26s %-26s %s" % ("build/key", "cpu ns/iter", "wall ns/iter", "ratio (cpu)"))
        for k, cpu, wall, rat in res:
            print("   %-70s %-26s %-26s %s" % ("/".join(str(x) for x in k if x is not None),
                                               rng(cpu), rng(wall), rng(rat)))


if __name__ == "__main__":
    main(sys.argv[1])
