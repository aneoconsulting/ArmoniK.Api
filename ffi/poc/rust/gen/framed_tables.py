#!/usr/bin/env python3
"""Tables of a gen/stream_probe2.sh session (CONTAINER INSTRUMENTATION): the grid's per-call client
CPU per (direction/payload, k) and cell, with the ratio to cell A of the same session; the probe's
k = 1 direction d per cell; the host's per-chunk split of the split cells.
   gen/framed_tables.py SESSION_DIR"""
import glob, json, os, statistics, sys
from collections import defaultdict
d = sys.argv[1]
g = defaultdict(list)
for p in glob.glob(os.path.join(d, "grid-*.jsonl")):
    for l in open(p):
        if l.startswith("#"):
            continue
        r = json.loads(l)
        key = (f"{r['dir']}/{r['payload']}" if r["dir"] in ("c", "d") else f"{r['dir']}", r["inflight"])
        g[(key, r["cell"])].append(r["cpu_ns"] / r["iters"])
keys = sorted({k for k, _ in g}, key=lambda k: (k[0], k[1]))
order = ["A", "Df-retain", "Cf-cb-retain", "C-cb-retain", "Cf-retain", "C-retain", "Bf-cb", "B-cb", "Bf", "B",
         "Ef-cb-retain", "E-cb-retain", "Ef-retain", "E-retain"]
cells = [c for c in order if any((k, c) in g for k in keys)]
print(open(os.path.join(d, "header.txt")).read().strip())
print("\n## grid (rpc_suite narrowed): per-call client CPU, ms, median over the pooled samples; (x A) = over cell A of the same (dir, k)\n")
print("| cell | " + " | ".join(f"{k[0]} k={k[1]}" for k in keys) + " |")
print("|---|" + "---:|" * len(keys))
for c in cells:
    row = []
    for k in keys:
        xs = g.get((k, c))
        a = g.get((k, "A"))
        if not xs:
            row.append("")
            continue
        m = statistics.median(xs) / 1e6
        row.append(f"{m:.2f} ({statistics.median(xs) / statistics.median(a):.2f})" if a else f"{m:.2f}")
    print(f"| {c} | " + " | ".join(row) + " |")
pr = defaultdict(list)
sp = defaultdict(lambda: defaultdict(list))
for p in glob.glob(os.path.join(d, "blk-*.jsonl")):
    for l in open(p):
        if l.startswith("#"):
            continue
        r = json.loads(l)
        pr[(r["size"], r["cell"])].extend(r["cpu_calls"])
        if "host_encode_cpu" in r:
            n = 8 if r["size"] == "16MiB" else 2
            for m in ("host_encode_cpu", "host_send_cpu", "host_send_wall", "host_recv_wall"):
                sp[(r["size"], r["cell"])][m].append(r[m] / (n if m != "host_recv_wall" else 1))
print("\n## probe, direction d, k = 1: per-call client CPU, ms, median [p10-p90] (x A)\n")
print("| cell | 16 MiB | 4 MiB |\n|---|---:|---:|")
pc = sorted({c for _, c in pr}, key=lambda c: (order.index(c) if c in order else 50, c))
for c in pc:
    row = []
    for s in ("16MiB", "4MiB"):
        xs = sorted(pr.get((s, c), []))
        a = statistics.median(pr[(s, "A")])
        n = len(xs)
        row.append(f"{xs[n // 2] / 1e6:.2f} [{xs[n // 10] / 1e6:.2f}-{xs[9 * n // 10] / 1e6:.2f}] ({xs[n // 2] / a:.2f})" if xs else "")
    print(f"| {c} | " + " | ".join(row) + " |")
print("\n## the host's work per chunk in the split cells (medians over rounds): encode CPU, send-entry CPU, send wall (blocking: until the send returned; callback: until the send's completion arrived), recv wall per call\n")
print("| cell | size | encode us/chunk | send CPU us/chunk | send wall us/chunk | recv wall ms/call |\n|---|---|---:|---:|---:|---:|")
for (s, c), v in sorted(sp.items(), key=lambda x: (x[0][0] != "16MiB", x[0][1])):
    m = lambda k: statistics.median(v[k])
    print(f"| {c} | {s} | {m('host_encode_cpu') / 1e3:.0f} | {m('host_send_cpu') / 1e3:.1f} | {m('host_send_wall') / 1e3:.0f} | {m('host_recv_wall') / 1e6:.2f} |")
