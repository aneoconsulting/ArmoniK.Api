#!/usr/bin/env python3
"""D23 fixes A, B, C: tables from gen/d23_fix_bench.sh (container instrumentation).
Per row and mode: push and pull (controls: their code is the same in every stage; shown from the
stage-0 and stage-3 processes), and the FSM per stage (fix0 = before, fix1 = A, fix2 = A+B,
fix3 = A+B+C); each cell the median over launches of the per-process medians, [min .. max]. Then
fsm-core per stage, and the unit-cost slopes per stage.

  gen/d23_fix_tables.py BENCH_DIR
"""
import glob
import os
import re
import statistics
import sys

D = sys.argv[1]
STAGES = ["fix0", "fix1", "fix2", "fix3"]
val = {}   # (stage, mode, row, arm) -> [ns per launch]
ev = {}
for path in sorted(glob.glob(os.path.join(D, "fix*-*-*.txt"))):
    m = re.match(r"(fix\d)-(drop|retain)-(\d+)\.txt$", os.path.basename(path))
    if not m:
        continue
    st, mode = m.group(1), m.group(2)
    for ln in open(path):
        r = re.match(r"^(\S+)\s+(\d+)\s+(\d+)\s+(\S+)\s+([\d.]+) (ns|us|ms)", ln)
        if r:
            v = float(r.group(5)) * {"ns": 1, "us": 1e3, "ms": 1e6}[r.group(6)]
            val.setdefault((st, mode, r.group(1), r.group(4)), []).append(v)
            ev[r.group(1)] = int(r.group(3))


def fmt(x):
    if x >= 1e6:
        return "%.3f ms" % (x / 1e6)
    if x >= 1e3:
        return "%.2f us" % (x / 1e3)
    return "%.1f ns" % x


def cell(k):
    v = val.get(k)
    if not v:
        return "-"
    return "%s [%s .. %s]" % (fmt(statistics.median(v)), fmt(min(v)), fmt(max(v)))


rows = sorted(set((k[1], k[2]) for k in val), key=lambda x: (x[0], x[1].startswith(("PK", "EV")), x[1]))
print("# D23 fixes: absolute times (container instrumentation; %s)" % os.path.basename(D.rstrip("/")))
print()
print("| mode | row | events | push (fix0 / fix3 proc) | pull (fix0 / fix3 proc) | FSM before (fix0) | FSM A (fix1) | FSM A+B (fix2) | FSM A+B+C (fix3) |")
print("|---|---|---|---|---|---|---|---|---|")
for mode, row in rows:
    print("| %s | %s | %d | %s / %s | %s / %s | %s | %s | %s | %s |" % (
        mode, row, ev.get(row, 0),
        cell(("fix0", mode, row, "push")), cell(("fix3", mode, row, "push")),
        cell(("fix0", mode, row, "pull")), cell(("fix3", mode, row, "pull")),
        *[cell((s, mode, row, "fsm")) for s in STAGES]))
print()
print("## Core only (drop): pull-core (fix0 / fix3 proc) and fsm-core per stage")
print()
print("| row | pull-core | fsm-core fix0 | fix1 | fix2 | fix3 |")
print("|---|---|---|---|---|---|")
for mode, row in rows:
    if mode != "drop":
        continue
    print("| %s | %s / %s | %s |" % (row, cell(("fix0", "drop", row, "pull-core")), cell(("fix3", "drop", row, "pull-core")),
                                   " | ".join(cell((s, "drop", row, "fsm-core")) for s in STAGES)))
print()
print("## Unit-cost slopes (ns), fsm-core and pull-core, per stage (median over launches)")
print()


def med(st, row, arm):
    v = val.get((st, "drop", row, arm))
    return statistics.median(v) if v else None


units = [("per event (EV-empty 64 -> 1024)", "EV-empty-64", "EV-empty-1024", 1920)]
for k in ("ticks", "codes", "flags", "values"):
    units.append(("per packed value, %s" % k, "PK-%s-256" % k, "PK-%s-4096" % k, 3840))
print("| unit | pull-core | fsm-core fix0 | fix1 | fix2 | fix3 |")
print("|---|---|---|---|---|---|")
for name, a, b, u in units:
    pc = (med("fix3", b, "pull-core") - med("fix3", a, "pull-core")) / u
    cells = []
    for s in STAGES:
        x, y = med(s, a, "fsm-core"), med(s, b, "fsm-core")
        cells.append("%.2f" % ((y - x) / u) if x is not None and y is not None else "-")
    print("| %s | %.2f | %s |" % (name, pc, " | ".join(cells)))
