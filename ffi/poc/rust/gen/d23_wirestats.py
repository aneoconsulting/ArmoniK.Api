#!/usr/bin/env python3
"""FIX-PLAN D23 attribution: per-field wire statistics of one dumped row (fsm_attrib dump):
for every field path (top-level field number, then the field number inside a length-delimited
element when --nested is given), how many occurrences, and for length-delimited bodies decoded
as packed scalars the number of values and their mean byte length. Analysis only.

  gen/d23_wirestats.py FILE [--nested]
"""
import collections
import sys


def rd(b, i):
    v = s = 0
    while True:
        c = b[i]
        i += 1
        v |= (c & 0x7F) << s
        if c < 0x80:
            return v, i
        s += 7


def walk(b, lo, hi):
    out = []
    i = lo
    while i < hi:
        k, i = rd(b, i)
        f, w = k >> 3, k & 7
        if w == 0:
            v, j = rd(b, i)
            out.append((f, w, i, j))
            i = j
        elif w == 1:
            out.append((f, w, i, i + 8))
            i += 8
        elif w == 5:
            out.append((f, w, i, i + 4))
            i += 4
        elif w == 2:
            n, i = rd(b, i)
            out.append((f, w, i, i + n))
            i += n
        else:
            raise SystemExit("group or bad wire type at %d" % i)
    return out


def main():
    b = open(sys.argv[1], "rb").read()
    nested = "--nested" in sys.argv
    occ = collections.Counter()
    vals = collections.Counter()
    vbytes = collections.Counter()
    for f, w, s, e in walk(b, 0, len(b)):
        occ[(f, w)] += 1
        if nested and w == 2:
            try:
                inner = walk(b, s, e)
            except Exception:
                continue
            for g, x, s2, e2 in inner:
                occ[(f, g, x)] += 1
                if x == 2:
                    # as packed varints
                    j, n = s2, 0
                    try:
                        while j < e2:
                            _, j = rd(b, j)
                            n += 1
                        vals[(f, g)] += n
                        vbytes[(f, g)] += e2 - s2
                    except IndexError:
                        pass
    print("# %s: %d bytes" % (sys.argv[1].split("/")[-1], len(b)))
    for k in sorted(occ, key=str):
        extra = ""
        if len(k) == 3 and k[2] == 2 and vals[k[:2]]:
            extra = "  as packed varints: %d values, %.2f B/value" % (vals[k[:2]], vbytes[k[:2]] / vals[k[:2]])
        print("  field %-12s occurrences %6d%s" % (".".join(str(x) for x in k), occ[k], extra))


if __name__ == "__main__":
    main()
