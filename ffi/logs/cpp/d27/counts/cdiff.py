"""D27: the cpp slice's crossing counts before (committed baseline) and after, row by row.
usage: python3 cdiff.py OLD NEW. Rows '  P...'/'  U...': key = the text before 'forward';
prints, per (row label, delta forward / reverse / core / host / take), how many rows."""
import re, sys, collections
R = re.compile(r"^  (\S+)\s+(.*?)\s+forward\s+(\d+)\s+reverse\s+(\d+).*?\(core (\d+) \+ host (\d+)(?: \+ take (\d+))?\)")
def load(p):
    out = collections.OrderedDict()
    for l in open(p):
        m = R.match(l)
        if m:
            k = (m.group(1), m.group(2).strip())
            out[k] = tuple(int(x or 0) for x in m.groups()[2:])
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
print("rows: old %d, new %d, same keys %s" % (len(a), len(b), a.keys() == b.keys()))
c = collections.Counter()
for k in a:
    if k not in b:
        c[("MISSING", k[1])] += 1; continue
    d = tuple(y - x for x, y in zip(a[k], b[k]))
    lab = re.sub(r"\S+/\S+", "", k[1]).strip()
    c[(lab, d)] += 1
print("(row label, delta forward/reverse/core/host/take): rows")
for (lab, d), n in sorted(c.items(), key=lambda x: str(x)):
    print("  %-34s %-26s %4d" % (lab, d, n))
ok = all(b[k][1] == a[k][1] and b[k][2] == a[k][2] and b[k][0] - a[k][0] == b[k][3] - a[k][3] for k in a if k in b)
print("reverse and core unchanged on every row, forward changes only by the host calls: %s" % ok)
