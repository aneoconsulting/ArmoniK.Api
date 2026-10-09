#!/usr/bin/env python3
"""D23 fixes A, B, C: tables from gen/d23_fix_bench.sh (container instrumentation).
Per row and mode: push and pull (controls: their code is the same in every stage; shown from the
stage-0 and stage-3 processes), and the FSM per stage (fix0 = before, fix1 = A, fix2 = A+B,
fix3 = A+B+C); each cell the median over launches of the per-process medians, [min .. max]. Then
fsm-core per stage, and the unit-cost slopes per stage.

  gen/d23_fix_tables.py BENCH_DIR [STAGE,STAGE,...]   (the first and last stage are the push/pull
                                                     control processes shown)
"""
import glob
import os
import re
import statistics
import sys

D = sys.argv[1]
STAGES = sys.argv[2].split(",") if len(sys.argv) > 2 else ["fix0", "fix1", "fix2", "fix3"]
val = {}   # (stage, mode, row, arm) -> [ns per launch]
ev = {}
for path in sorted(glob.glob(os.path.join(D, "*-*-*.txt"))):
    m = re.match(r"(\w+)-(drop|retain)-(\d+)\.txt$", os.path.basename(path))
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
F, Z = STAGES[0], STAGES[-1]
print("| mode | row | events | push (%s / %s proc) | pull (%s / %s proc) | %s |" % (F, Z, F, Z, " | ".join("FSM " + s for s in STAGES)))
print("|---|---|---|---|---|" + "---|" * len(STAGES))
for mode, row in rows:
    print("| %s | %s | %d | %s / %s | %s / %s | %s |" % (
        mode, row, ev.get(row, 0),
        cell((F, mode, row, "push")), cell((Z, mode, row, "push")),
        cell((F, mode, row, "pull")), cell((Z, mode, row, "pull")),
        " | ".join(cell((s, mode, row, "fsm")) for s in STAGES)))
print()
print("## Core only (drop): pull-core (%s / %s proc) and fsm-core per stage" % (F, Z))
print()
print("| row | pull-core | %s |" % " | ".join("fsm-core " + s for s in STAGES))
print("|---|---|" + "---|" * len(STAGES))
for mode, row in rows:
    if mode != "drop":
        continue
    print("| %s | %s / %s | %s |" % (row, cell((F, "drop", row, "pull-core")), cell((Z, "drop", row, "pull-core")),
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
print("| unit | pull-core | %s |" % " | ".join("fsm-core " + s for s in STAGES))
print("|---|---|" + "---|" * len(STAGES))
for name, a, b, u in units:
    pc = (med(Z, b, "pull-core") - med(Z, a, "pull-core")) / u
    cells = []
    for s in STAGES:
        x, y = med(s, a, "fsm-core"), med(s, b, "fsm-core")
        cells.append("%.2f" % ((y - x) / u) if x is not None and y is not None else "-")
    print("| %s | %.2f | %s |" % (name, pc, " | ".join(cells)))
