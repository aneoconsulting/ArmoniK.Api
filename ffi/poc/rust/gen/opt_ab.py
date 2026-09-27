#!/usr/bin/env python3
"""Alternating narrowed runs (gen/opt_narrow.sh), A = before, B = after, run A1 B1 A2 B2 ...

    gen/opt_ab.py DIR [COLS]

DIR holds A1.. and B1.. (each an opt_narrow.sh output). For every (input, dir, variant) row
and column: the median over the A runs, over the B runs, B/A of the medians, and the range
of the per-pair ratios Bi/Ai (so the spread between processes is visible next to the
effect). CONTAINER INSTRUMENTATION. Per column the geometric mean of B/A over the rows.
"""
import math, os, statistics, sys
from collections import defaultdict


def load(d):
    rows, hdr = {}, None
    for l in open(os.path.join(d, "variants-codec.tsv")):
        if l.startswith("#"):
            continue
        f = l.rstrip("\n").split("\t")
        if f[0] == "input":
            hdr = f
            continue
        rows[tuple(f[:3])] = {h: float(v) for h, v in zip(hdr[3:], f[3:]) if v}
    return rows


def main(d, cols):
    runs = sorted(x for x in os.listdir(d) if x[:1] in "AB" and x[1:].isdigit())
    A = [load(os.path.join(d, x)) for x in runs if x[0] == "A"]
    B = [load(os.path.join(d, x)) for x in runs if x[0] == "B"]
    n = min(len(A), len(B))
    print("# CONTAINER INSTRUMENTATION. %s: %d A runs, %d B runs, alternated. Cell: median A -> median B us (B/A) [per-pair range]" % (d, len(A), len(B)))
    g = defaultdict(list)
    for k in sorted(A[0]):
        cells = []
        for c in cols:
            a = [r[k][c] for r in A if k in r and c in r[k]]
            b = [r[k][c] for r in B if k in r and c in r[k]]
            if not a or not b:
                continue
            ma, mb = statistics.median(a), statistics.median(b)
            pr = [B[i][k][c] / A[i][k][c] for i in range(n) if k in A[i] and k in B[i] and c in A[i][k] and c in B[i][k]]
            g[(k[1], k[2].split("/")[0], c)].append(mb / ma)
            cells.append("%s %.2f->%.2f (%.2f) [%.2f,%.2f]" % (c, ma / 1000, mb / 1000, mb / ma, min(pr), max(pr)))
        if cells:
            print("%-12s %-12s %-26s | %s" % (k[0], k[1], k[2], " | ".join(cells)))
    print("# geometric mean of B/A (medians over runs), per direction, variant and column")
    for (dr, v, c), r in sorted(g.items()):
        print("%-12s %-22s %-14s n=%3d gmean %.3f [%.2f, %.2f]" % (dr, v, c, len(r), math.exp(sum(map(math.log, r)) / len(r)), min(r), max(r)))


if __name__ == "__main__":
    main(sys.argv[1], (sys.argv[2] if len(sys.argv) > 2 else
                       "prost,armonik,native-drop,native-retain,ffi-drop,ffi-retain,pull-drop,pull-retain").split(","))
