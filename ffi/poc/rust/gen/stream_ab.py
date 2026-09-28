#!/usr/bin/env python3
"""A/B of stream-probe runs: per (size, cell), the median over every round of the A runs and
of the B runs (per-call CPU), B/A, and B/A per process pair (the spread). CONTAINER INSTRUMENTATION.
   gen/stream_ab.py OUT_DIR TAG"""
import glob, json, os, statistics, sys
from collections import defaultdict
out, tag = sys.argv[1], sys.argv[2]
def rows(p):
    return [json.loads(l) for l in open(p) if not l.startswith("#")]
runs = {v: sorted(glob.glob(os.path.join(out, f"{tag}-{v}[0-9]*.jsonl"))) for v in "ab"}
for h in open(runs["a"][0]):
    if h.startswith("# stream probe run"):
        print(h.rstrip()); break
for h in open(runs["b"][0]):
    if h.startswith("# stream probe run"):
        print(h.rstrip()); break
med = defaultdict(lambda: defaultdict(list))   # (size,cell) -> variant -> [per-run medians]
pool = defaultdict(lambda: defaultdict(list))
for v, ps in runs.items():
    for p in ps:
        g = defaultdict(list)
        for r in rows(p):
            g[(r["size"], r["cell"])].append(r["cpu_ns"])
        for k, xs in g.items():
            med[k][v].append(statistics.median(xs))
            pool[k][v].extend(xs)
print(f"{len(runs['a'])} A and {len(runs['b'])} B processes; per-call client CPU, ms")
print("size    cell            A median   B median    B/A   B/A per pair")
for k in sorted(pool, key=lambda k: (k[0] != "16MiB", k[1])):
    a, b = statistics.median(pool[k]["a"]), statistics.median(pool[k]["b"])
    pairs = " ".join(f"{y / x:.3f}" for x, y in zip(med[k]["a"], med[k]["b"]))
    print(f"{k[0]:7} {k[1]:14} {a / 1e6:9.2f} {b / 1e6:9.2f}  {b / a:6.3f}   {pairs}")
