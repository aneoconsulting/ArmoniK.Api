#!/usr/bin/env python3
"""FIX-PLAN D23 attribution: derived figures from fsm_attrib bench output (container
instrumentation). Per-unit costs are SLOPES between two sizes of the same synthetic probe, so
fixed per-decode costs cancel; per-row deltas are medians from the same process.

  gen/d23_attrib_tables.py BENCH_FILE
"""
import re
import sys

ns = {}
for ln in open(sys.argv[1]):
    m = re.match(r"^(\S+)\s+(\d+)\s+(\d+)\s+(\S+)\s+([\d.]+) (ns|us|ms)", ln)
    if m:
        v = float(m.group(5)) * {"ns": 1, "us": 1e3, "ms": 1e6}[m.group(6)]
        ns[(m.group(1), m.group(4))] = v
        ns[(m.group(1), "_events")] = int(m.group(3))


def slope(a, b, arm, units):
    return (ns[(b, arm)] - ns[(a, arm)]) / units


print("# per-unit slopes (ns), from %s" % sys.argv[1].split("/")[-1])
print("%-34s %10s %10s %10s %10s %10s %10s" % ("unit", "pull-core", "fsm-core", "delta", "pull", "fsm", "push"))
rows = [("per event (EV-empty 64 -> 1024)", "EV-empty-64", "EV-empty-1024", 2049 - 129)]
for k in ("ticks", "codes", "flags", "values"):
    rows.append(("per packed value, %s (256 -> 4096)" % k, "PK-%s-256" % k, "PK-%s-4096" % k, 3840))
for name, a, b, u in rows:
    if (a, "pull") not in ns or (b, "pull") not in ns:
        continue
    pc, fc = slope(a, b, "pull-core", u), slope(a, b, "fsm-core", u)
    print("%-34s %10.2f %10.2f %10.2f %10.2f %10.2f %10.2f" % (name, pc, fc, fc - pc, slope(a, b, "pull", u),
                                                           slope(a, b, "fsm", u), slope(a, b, "push", u)))
print()
print("# per row (ns): core delta = fsm-core - pull-core; facade-side delta = (fsm - fsm-core) - (pull - pull-core);")
print("# collect = fsm-collect - fsm (negative: collecting then replaying is cheaper than dispatching per event)")
print("%-46s %7s %12s %12s %12s %12s %12s" % ("row", "events", "fsm - pull", "core delta", "facade delta", "collect", "push"))
for r in sorted(set(k[0] for k in ns)):
    if (r, "fsm-core") not in ns:
        continue
    d = ns[(r, "fsm")] - ns[(r, "pull")]
    cd = ns[(r, "fsm-core")] - ns[(r, "pull-core")]
    fd = (ns[(r, "fsm")] - ns[(r, "fsm-core")]) - (ns[(r, "pull")] - ns[(r, "pull-core")])
    col = ns.get((r, "fsm-collect"), float("nan")) - ns[(r, "fsm")]
    print("%-46s %7d %12.1f %12.1f %12.1f %12.1f %12.1f" % (r, ns[(r, "_events")], d, cd, fd, col, ns[(r, "push")]))
