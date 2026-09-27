#!/usr/bin/env python3
"""Framed cells (Xf...) against their reference twins, from rpc_suite JSON lines in DIR
(any rpc-*.jsonl). Per (transport, cell, dir, in-flight): median client CPU per call of each
cell over every round of every launch, framed/reference, and the range of the per-launch
ratios (the two cells share each launch's process). CONTAINER INSTRUMENTATION."""
import json, math, os, statistics, sys
from collections import defaultdict

d = sys.argv[1]
per = defaultdict(list)          # (T, cell, dir, k) -> [ns per call]
perl = defaultdict(list)         # (T, cell, dir, k, launch) -> [...]
for fn in sorted(os.listdir(d)):
    if not (fn.startswith("rpc-") and fn.endswith(".jsonl")):
        continue
    for l in open(os.path.join(d, fn)):
        if l.startswith("#") or not l.strip():
            continue
        o = json.loads(l)
        v = o["cpu_ns"] / o["iters"]
        k = (o["transport"], o["cell"], (o["dir"] if o["dir"] not in ("c", "d") else o["dir"] + "/" + o["payload"]), o["inflight"])
        per[k].append(v)
        perl[k + (o["launch"],)].append(v)
print("# CONTAINER INSTRUMENTATION: client process CPU per call (us), median over rounds x launches; framed/reference; [per-launch ratio range]")
g = defaultdict(list)
g10 = defaultdict(list)
for (t, c, dr, k), v in sorted(per.items()):
    if len(c) < 2 or c[1] != "f":
        continue
    ref = c[0] + c[2:]
    if (t, ref, dr, k) not in per:
        continue
    a, b = statistics.median(per[(t, ref, dr, k)]), statistics.median(v)
    launches = sorted({x[4] for x in perl if x[:4] == (t, c, dr, k)})
    rs = [statistics.median(perl[(t, c, dr, k, L)]) / statistics.median(perl[(t, ref, dr, k, L)])
          for L in launches if (t, ref, dr, k, L) in perl]
    g[(dr, c[0])].append(b / a)
    # The round distribution here is heavy-tailed (rounds 3-5x the median); the 10th
    # percentile is printed beside the median as the tail-robust figure.
    p10 = lambda x: sorted(x)[len(x) // 10]
    pa, pb = p10(per[(t, ref, dr, k)]), p10(v)
    g10[(dr, c[0])].append(pb / pa)
    print("%-8s %-10s %-7s k=%-3d ref %8.1f  framed %8.1f  (%.3f) [%.2f, %.2f]   p10 %8.1f -> %8.1f (%.3f)" % (t, c, dr, k, a / 1e3, b / 1e3, b / a, min(rs), max(rs), pa / 1e3, pb / 1e3, pb / pa))
print("# geometric mean framed/reference per direction and cell letter")
for (dr, c), v in sorted(g.items()):
    w = g10[(dr, c)]
    print("%-7s %s n=%2d gmean %.3f [%.2f, %.2f]   p10 gmean %.3f [%.2f, %.2f]" % (dr, c, len(v), math.exp(sum(map(math.log, v)) / len(v)), min(v), max(v),
          math.exp(sum(map(math.log, w)) / len(w)), min(w), max(w)))
