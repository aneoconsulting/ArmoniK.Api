#!/usr/bin/env python3
"""Tables of one gen/patch_ab.sh directory: HEAD core against patched core, per workload and
cell, absolute per-call figures (no ratio).

  patch_tables.py OUT_DIR > OUT_DIR/tables.md

measure/*.out: CPU and wall per call, median [p10-p90] over the 10 chunks of each of the rounds'
processes; minor faults and context switches (voluntary + involuntary) per call, median of the
processes; per-thread CPU per call by class (median of the processes). alloc/*.out: allocations of
at least 1 MiB per call (one process, allocprobe preloaded).
"""
import collections
import glob
import json
import os
import statistics
import sys

CELLS = ["A", "D-retain", "Cf-retain", "Cf-q-retain"]
WLS = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("d4k1", "d/4MiB k=1"), ("c54k1", "c/P5.4 k=1")]
THREADS = ["caller", "main", "tokio-rt-worker", "event_engine"]


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
    L = []
    say = L.append
    say("# %s: HEAD core against the patched core, from C++ (one-cell `campaign_rpc --profile` processes, no perf)" % os.path.basename(out))
    say("")
    say("Knobs and cores: runner.log. Per call: CPU (process) and wall, median [p10-p90] over the 10 chunks of each "
        "round's process; flt = minor faults, csw = voluntary + involuntary context switches (median of the processes); "
        "big = allocations of at least 1 MiB (a separate process with the allocation probe).")
    say("")
    say("| workload | cell | core | CPU ms | wall ms | flt | csw | big | n |")
    say("|---|---|---|---|---|---|---|---|---|")
    thr = []
    for w, wt in WLS:
        for c in CELLS:
            for core in ("head", "patched"):
                fs = sorted(glob.glob(os.path.join(out, "measure", "%s-%s-%s-r*.out" % (w, c, core))))
                if not fs:
                    continue
                cpu, wall, flt, csw = [], [], [], []
                per = collections.defaultdict(list)
                for f in fs:
                    p = prof(f)
                    k = p["k"]
                    for ch in p["chunks"]:
                        n = ch["batches"] * k
                        cpu.append(ch["cpu_ns"] / n / 1e6); wall.append(ch["wall_ns"] / n / 1e6)
                    ru = p["rusage"]
                    flt.append(ru["minflt"] / p["calls"]); csw.append((ru["nvcsw"] + ru["nivcsw"]) / p["calls"])
                    for cls, v in p["thread_cpu_ns"].items():
                        per[cls].append(v["ns"] / p["calls"] / 1e6)
                a = os.path.join(out, "alloc", "%s-%s-%s.out" % (w, c, core))
                big = "-"
                if os.path.exists(a):
                    pa = prof(a)
                    if pa and pa.get("big_allocs", -1) >= 0:
                        big = "%.2f" % (pa["big_allocs"] / pa["calls"])
                say("| %s | %s | %s | %s | %s | %.1f | %.1f | %s | %d |" % (wt, c, core, mpq(cpu), mpq(wall),
                                                                        statistics.median(flt), statistics.median(csw), big, len(cpu)))
                if c.startswith("Cf"):
                    thr.append((wt, c, core, {t: statistics.median(per[t]) for t in THREADS if per.get(t)}))
    say("")
    say("Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):")
    say("")
    say("| workload | cell | core | " + " | ".join(THREADS) + " |")
    say("|---|---|---|" + "---|" * len(THREADS))
    for wt, c, core, d in thr:
        say("| %s | %s | %s | %s |" % (wt, c, core, " | ".join(("%.3f" % d[t]) if t in d else "" for t in THREADS)))
    say("")
    say("Median per-call CPU per process (ms), rounds in run order:")
    say("")
    for w, wt in WLS:
        for c in CELLS:
            row = []
            for core in ("head", "patched"):
                fs = sorted(glob.glob(os.path.join(out, "measure", "%s-%s-%s-r*.out" % (w, c, core))))
                v = []
                for f in fs:
                    p = prof(f)
                    v.append(statistics.median([ch["cpu_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]]))
                row.append("%s %s" % (core, " ".join("%.3f" % x for x in v)))
            say("- %s %s: %s" % (wt, c, "; ".join(row)))
    print("\n".join(L))


if __name__ == "__main__":
    main(sys.argv[1])
