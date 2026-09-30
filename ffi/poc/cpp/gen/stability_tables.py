#!/usr/bin/env python3
"""Tables of a gen/stability.sh run (absolute per-call figures and differences, no ratio).

  stability_tables.py OUT_DIR > OUT_DIR/tables.md

  1. pooled per unit and workload: CPU and wall per call, median [p10-p90] over every chunk of
     every process; context switches and minor faults per call (median of the processes);
  2. the gap to A per round: median(unit's process) - median(A's process) of the same round and
     workload, CPU and wall; its distribution over the rounds: median, p10-p90, min-max; the
     D - A floor is the cur/D-retain row;
  3. drift: every unit's per-process median CPU (and wall) in run order, per workload.
"""
import collections
import glob
import json
import os
import re
import statistics
import sys

WLS = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("d4k1", "d/4MiB k=1"), ("c54k1", "c/P5.4 k=1"), ("c54k8", "c/P5.4 k=8")]
UNITS = ["cur/A", "cur/D-retain", "cur/Cf-retain", "cur/Cf-q-retain", "stk/Cf-retain", "stk/Cf-q-retain", "stk/Cf-zc-retain", "stk/Cf-zcw-retain"]


def q(xs, p):
    xs = sorted(xs)
    return xs[0] if len(xs) == 1 else statistics.quantiles(xs, n=10, method="inclusive")[{10: 0, 90: 8}[p]]


def mpq(xs):
    return "%.3f [%.3f-%.3f]" % (statistics.median(xs), q(xs, 10), q(xs, 90)) if xs else "-"


def prof(path):
    for l in open(path):
        if l.startswith('{"profile"'):
            return json.loads(l)["profile"]
    return None


def main(out):
    P = []
    for f in sorted(glob.glob(os.path.join(out, "proc", "*.out"))):
        m = re.match(r"(\d+)-r(\d+)-([a-z0-9]+)-(cur|stk)-(.+)\.out$", os.path.basename(f))
        p = prof(f)
        if not m or not p:
            continue
        cpu = [ch["cpu_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]]
        wall = [ch["wall_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]]
        ru = p["rusage"]
        P.append({"seq": int(m.group(1)), "round": int(m.group(2)), "wl": m.group(3), "unit": m.group(4) + "/" + m.group(5),
                  "cpu": cpu, "wall": wall, "mcpu": statistics.median(cpu), "mwall": statistics.median(wall),
                  "csw": (ru["nvcsw"] + ru["nivcsw"]) / p["calls"], "flt": ru["minflt"] / p["calls"]})
    L = []
    say = L.append
    rounds = sorted({x["round"] for x in P})
    say("# Stability campaign: %d processes, %d rounds (per call, ms; absolute; no ratio)" % (len(P), len(rounds)))
    say("")
    say("## 1. Pooled per unit: CPU and wall per call, median [p10-p90] over every chunk of every process")
    say("")
    say("| workload | unit | CPU | wall | csw | flt | processes |")
    say("|---|---|---|---|---|---|---|")
    by = collections.defaultdict(list)
    for x in P:
        by[(x["wl"], x["unit"])].append(x)
    for w, wt in WLS:
        for u in UNITS:
            xs = by.get((w, u))
            if not xs:
                continue
            say("| %s | %s | %s | %s | %.1f | %.1f | %d |" % (wt, u, mpq([c for x in xs for c in x["cpu"]]),
                                                      mpq([c for x in xs for c in x["wall"]]),
                                                      statistics.median([x["csw"] for x in xs]),
                                                      statistics.median([x["flt"] for x in xs]), len(xs)))
    say("")
    say("## 2. Gap to A per round (median of the unit's process minus median of cur/A's process, same round and workload)")
    say("")
    say("The cur/D-retain rows are the D - A floor (same codec as the core cells, grpc++ transport).")
    say("")
    say("| workload | unit | CPU gap: median [p10-p90] (min..max) | wall gap: median [p10-p90] (min..max) | rounds |")
    say("|---|---|---|---|---|")
    for w, wt in WLS:
        a = {x["round"]: x for x in by.get((w, "cur/A"), [])}
        for u in UNITS[1:]:
            xs = [x for x in by.get((w, u), []) if x["round"] in a]
            if not xs:
                continue
            gc = [x["mcpu"] - a[x["round"]]["mcpu"] for x in xs]
            gw = [x["mwall"] - a[x["round"]]["mwall"] for x in xs]
            f = lambda g: "%+.3f [%+.3f..%+.3f] (%+.3f..%+.3f)" % (statistics.median(g), q(g, 10), q(g, 90), min(g), max(g))
            say("| %s | %s | %s | %s | %d |" % (wt, u, f(gc), f(gw), len(xs)))
    say("")
    say("## 3. Drift: per-process median CPU (wall) per call in run order")
    for w, wt in WLS:
        say("")
        say("### %s" % wt)
        say("")
        for u in UNITS:
            xs = sorted(by.get((w, u), []), key=lambda x: x["seq"])
            if xs:
                say("- %s: %s" % (u, " ".join("%.2f(%.2f)" % (x["mcpu"], x["mwall"]) for x in xs)))
    print("\n".join(L))


if __name__ == "__main__":
    main(sys.argv[1])
