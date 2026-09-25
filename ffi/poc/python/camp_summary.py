"""CAMPAIGN.md requirement 30: the only summaries a slice produces.

  python3.12 camp_summary.py <dir with *.jsonl>  > summary.txt

Per (arm or cell, payload, content, direction, unknown mode, transport, in flight): the median,
minimum and maximum over every round of every launch of CPU and wall per iteration (per call
for RPC), and the PER-ROUND ratio to `incumbent-prod` (codec: same payload, content,
direction, launch, round) or to cell A (RPC: same transport, direction, in flight, launch,
round), with its median and range. No significance claim, no verdict word. Crossing-cost
samples (calib) get their per-iteration median and range only.
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


def main(d):
    rows, smoke = load(d)
    if smoke:
        print("# INSTRUMENTATION: summarised from a smoke run; no figure here is a result")
    for suite in ("codec", "rpc", "calib"):
        # pyperf's warm-up and calibration measurements are exported (22a) but are not rounds.
        rs = [r for r in rows if r.get("suite") == suite and r.get("iters")
              and r.get("phase", "value") == "value"]
        if not rs:
            continue
        print("\n## %s: median [min, max] over all rounds and launches" % suite)
        who = "cell" if suite == "rpc" else "arm"
        keyf = lambda r: (r.get(who), r.get("payload"), r.get("content"), r.get("dir"),  # noqa: E731
                          r.get("unknown_mode"), r.get("transport"), r.get("inflight"))
        base = {}
        for r in rs:
            if "cpu_ns" not in r:
                continue
            if suite == "codec" and r.get("arm") == "incumbent-prod":
                base[(r["payload"], r.get("content"), r["dir"], r["launch"], r["round"])] = r
            if suite == "rpc" and r.get("cell") == "A":
                base[(r["transport"], r["dir"], r["inflight"], r["launch"], r["round"])] = r
        groups = {}
        for r in rs:
            groups.setdefault(keyf(r), []).append(r)
        print("   %-60s %-26s %-26s %s" % ("key", "cpu ns/iter", "wall ns/iter", "per-round ratio to the baseline (cpu)"))
        for k in sorted(groups, key=lambda x: tuple(str(y) for y in x)):
            g = groups[k]
            if "cpu_ns" not in g[0]:
                print("   %-60s perf: %s" % ("/".join(str(x) for x in k if x is not None),
                                              ", ".join("%s=%.4g" % (a, g[0][a]) for a in g[0] if a.startswith("perf_"))))
                continue
            cpu = [r["cpu_ns"] / r["iters"] for r in g]
            wall = [r["wall_ns"] / r["iters"] for r in g if "wall_ns" in r]
            rat = []
            for r in g:
                bk = ((r["payload"], r.get("content"), r["dir"], r["launch"], r["round"]) if suite == "codec"
                      else (r.get("transport"), r.get("dir"), r.get("inflight"), r["launch"], r["round"]))
                b = base.get(bk)
                if b and suite != "calib":
                    rat.append((r["cpu_ns"] / r["iters"]) / (b["cpu_ns"] / b["iters"]))
            print("   %-60s %-26s %-26s %s" % ("/".join(str(x) for x in k if x is not None),
                                               rng(cpu), rng(wall), rng(rat)))


if __name__ == "__main__":
    main(sys.argv[1])
