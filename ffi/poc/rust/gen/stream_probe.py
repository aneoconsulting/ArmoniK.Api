#!/usr/bin/env python3
"""Summary of stream-probe runs: per (cell, size), medians over rounds of the per-call values.
   gen/stream_probe.py RUN.jsonl [RUN2.jsonl ...]   (several runs: pooled rounds per run, printed per run)
CONTAINER INSTRUMENTATION."""
import json, statistics, sys
from collections import defaultdict

def load(p):
    rows, head = [], []
    for l in open(p):
        (head if l.startswith("#") else rows).append(l if l.startswith("#") else json.loads(l))
    return head, rows

def summary(p):
    head, rows = load(p)
    for h in head:
        print(h.rstrip())
    g = defaultdict(list)
    for r in rows:
        g[(r["size"], r["cell"])].append(r)
    classes = ["main", "caller", "cell-rt", "core-rt", "other"]
    print("size    cell         rounds  cpu/call ms [min-max]   wall ms | cpu by thread ms: " + " ".join(f"{c:>8}" for c in classes)
          + " | minflt/call total | allocs/call  >=1MiB/call | user/sys per call (mean of ticks) | ctx switches per call by class")
    for (size, cell) in sorted(g, key=lambda k: (k[0] != "16MiB", k[1])):
        rs = g[(size, cell)]
        med = lambda k: statistics.median(r.get(k, 0.0) for r in rs)
        cpu = sorted(r["cpu_ns"] for r in rs)
        by = " ".join(f"{med('cpu_' + c) / 1e6:8.2f}" for c in classes)
        mf = sum(med("minflt_" + c) for c in classes)
        al = f"{med('allocs'):10.1f} {med('allocs_big'):6.2f}" if "allocs" in rs[0] else "   (no shim)"
        tot = lambda k: sum(r.get(f"{k}_{c}", 0.0) for r in rs for c in classes)
        n = len(rs)
        ut, st = tot("uticks") / n * 10, tot("sticks") / n * 10  # clock ticks of 10 ms, averaged over rounds
        cs = " ".join(f"{sum(r.get('vcs_' + c, 0) + r.get('ics_' + c, 0) for r in rs) / n:6.0f}" for c in classes)
        print(f"{size:7} {cell:12} {len(rs):6}  {statistics.median(cpu) / 1e6:8.2f} [{cpu[0] / 1e6:.2f}-{cpu[-1] / 1e6:.2f}]  {med('wall_ns') / 1e6:8.2f} | {'':17}{by} | {mf:12.0f} | {al} | user {ut:5.2f} sys {st:5.2f} ms | ctx switches/call {cs}")

for p in sys.argv[1:]:
    summary(p)
    print()
