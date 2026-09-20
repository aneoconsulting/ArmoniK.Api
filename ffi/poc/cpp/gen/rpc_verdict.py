#!/usr/bin/env python3
"""Aggregate the RPC grid's within-round deltas across every configuration it was run in.

The grid is run in twelve configurations (2 transports x 2 pinnings x 3 in-flight levels)
and each prints its own delta table. The question a reader has is not "what did uds/pin/8
say" but "which of these differences survives being asked twelve times", so this reads the
log back and counts.

Derived from the log rather than written into it: a verdict typed by hand next to a table
drifts from the table on the next run, and this branch has that failure twice already.
"""
import re
import sys

ROW = re.compile(
    r"^   (\S.*?)\s{2,}(-?\d+)\s+(-?\d+)\s+(-?[\d.]+)\s+(-?[\d.]+)\s+(yes|NO)\s*$")
ADD = re.compile(r"^   \(C-B\) - \(D-A\)\D*(-?\d+)\s+(-?\d+)\s+(yes|NO)\s*$")
GRID = re.compile(r"^== the grid: transport (\w+), pinning (\w+) ==")
ARM = re.compile(r"^A\s+A\s+blocking\s+(\d+)\s")


SMALL = re.compile(r"^== the deliveries on a call small enough")


def main(path):
    tr = pin = lvl = None
    rows = []          # (section, config, name, lo%, hi%, sign)
    order = {"grid": [], "ping": []}
    add = []
    sect = "grid"
    for line in open(path):
        m = GRID.match(line)
        if m:
            tr, pin = m.group(1), m.group(2)
            sect = "grid"
            continue
        if SMALL.match(line):
            tr, pin, sect = "uds", "pin", "ping"
            continue
        m = ARM.match(line)
        if m:
            lvl = m.group(1)
            continue
        m = ROW.match(line)
        if m and tr:
            name = m.group(1).strip()
            if name.startswith("(C-B)"):
                continue
            if name not in order[sect]:
                order[sect].append(name)
            rows.append((sect, "%s/%s/%s" % (tr, pin, lvl), name, float(m.group(4)),
                         float(m.group(5)), m.group(6)))
            continue
        m = ADD.match(line)
        if m:
            add.append(m.group(3) or "NO")

    print()
    print("== every difference, counted across every configuration it was asked in ==")
    print()
    print("   A difference is counted as SEPARATED in a configuration only where its lo and")
    print("   hi over the nine within-round deltas share a sign. The range below is over the")
    print("   configurations where it did separate, as a percentage of cell A; quoting a")
    print("   range over the subset with the wanted sign is exactly what the sign column")
    print("   exists to stop, so the COUNT is printed beside it and is the thing to read.")
    print("   A difference that separates in a minority of configurations has not separated.")
    for sect, title in (("grid", "the P2.2 grid: 2 transports x 2 pinnings x 3 in-flight "
                                 "levels"),
                        ("ping", "the Ping block (no codec on either side): UDS, pinned, "
                                 "3 in-flight levels")):
        if not order[sect]:
            continue
        print()
        print("   -- %s --" % title)
        print("   %-40s %10s   %s" % ("difference", "separates",
                                      "range where it does, % of A"))
        for name in order[sect]:
            rs = [r for r in rows if r[0] == sect and r[2] == name]
            yes = [r for r in rs if r[5] == "yes"]
            rng = ("%+.2f%% to %+.2f%%" % (min(r[3] for r in yes), max(r[4] for r in yes))
                   if yes else "--")
            print("   %-40s %6d / %-3d  %s" % (name[:40], len(yes), len(rs), rng))
        if sect == "grid":
            nadd = sum(1 for a in add if a == "yes")
            print("   %-40s %6d / %-3d  %s" % ("(C-B) - (D-A)  do the halves add?", nadd,
                                               len(add), "--" if not nadd else
                                               "see the per-configuration tables"))
    print()


if __name__ == "__main__":
    main(sys.argv[1])
