#!/usr/bin/env python3
"""Before / after of two gen/opt_bench.sh runs, per variant, in ABSOLUTE time
(CONTAINER INSTRUMENTATION).

    python3 gen/opt_variants_compare.py BEFORE_DIR AFTER_DIR > OUT.txt

Reads both runs' variants-codec.tsv (gen/opt_summary.py) and prints, per payload input,
direction and encode variant, for every variant column that both runs have: the median in
microseconds before and after, and after / before. Absolute times of two runs are two
processes on a machine whose speed drifts: the incumbent's own column (prost, which neither
side changed) is the control, printed first, and its after / before over all rows closes
the file.
"""
import math
import sys


def load(d):
    rows, head = {}, None
    for line in open(d + "/variants-codec.tsv"):
        if line.startswith("#"):
            continue
        p = line.rstrip("\n").split("\t")
        if head is None:
            head = p
            continue
        rows[tuple(p[:3])] = dict(zip(head[3:], p[3:]))
    return head[3:], rows


def main(b, a):
    cols, rb = load(b)
    _, ra = load(a)
    print("# CONTAINER INSTRUMENTATION. before = %s, after = %s. Median microseconds per operation (process CPU)." % (b, a))
    print("# Each cell: before -> after (after/before). prost is the control (unchanged code on both sides).")
    keys = [k for k in rb if k in ra]
    ctrl = []
    per_col = {c: [] for c in cols}
    for k in keys:
        cells = []
        for c in cols:
            x, y = rb[k].get(c, ""), ra[k].get(c, "")
            if x and y:
                x, y = float(x), float(y)
                per_col[c].append(y / x)
                cells.append("%s %.2f->%.2f (%.2f)" % (c, x / 1000, y / 1000, y / x))
        print("%-12s %-12s %-22s | %s" % (k[0], k[1], k[2], " | ".join(cells)))
    print()
    print("# geometric mean of after/before per column, over the rows both runs have (control: prost, prost@nounk)")
    for c in cols:
        v = per_col[c]
        if v:
            print("%-14s n=%3d  gmean %.3f  range [%.2f, %.2f]" % (c, len(v), math.exp(sum(map(math.log, v)) / len(v)), min(v), max(v)))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
