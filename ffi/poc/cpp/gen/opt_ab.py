#!/usr/bin/env python3
"""Summary of one gen/opt_ab.sh run: per benchmark, the median CPU per operation of every
repetition of every A process and of every B process, B/A, and in how many of the pairs the
B process's median is below the A process's (a consistent direction beats a small ratio).
Writes OUT/ab.txt. Instrumentation."""
import glob, json, os, statistics, sys

out = sys.argv[1]
def load(side):
    runs = []
    for p in sorted(glob.glob(os.path.join(out, side + "-*.json"))):
        d = json.load(open(p))
        r = {}
        for b in d["benchmarks"]:
            if b.get("run_type") != "iteration":
                continue
            name = b["run_name"].split("/")[0]
            k = 1
            if "inflight=" in name:
                k = int(name.split("inflight=")[1].split(",")[0])
            r.setdefault(name, []).append(b["cpu_time"] / k)
        runs.append(r)
    return runs
A, B = load("A"), load("B")
names = sorted(set().union(*[set(r) for r in A]) & set().union(*[set(r) for r in B]))
lines = [open(os.path.join(out, "ab.head")).read().rstrip("\n"),
         "# ns per operation (process CPU; rpc: per call), median of every repetition of every process; "
         "wins = pairs where B's process median < A's",
         "%-100s %12s %12s %7s %5s" % ("benchmark", "A_ns", "B_ns", "B/A", "wins")]
ratios = []
for n in names:
    a = [x for r in A for x in r.get(n, [])]
    b = [x for r in B for x in r.get(n, [])]
    ma, mb = statistics.median(a), statistics.median(b)
    wins = sum(1 for ra, rb in zip(A, B) if n in ra and n in rb and statistics.median(rb[n]) < statistics.median(ra[n]))
    ratios.append(mb / ma)
    lines.append("%-100s %12.1f %12.1f %7.3f %2d/%d" % (n[:100], ma, mb, mb / ma, wins, min(len(A), len(B))))
if ratios:
    lines.append("# geometric mean B/A over %d benchmarks: %.3f" % (len(ratios), statistics.geometric_mean(ratios)))
open(os.path.join(out, "ab.txt"), "w").write("\n".join(lines) + "\n")
print("\n".join(lines[-min(len(lines), 60):]))
