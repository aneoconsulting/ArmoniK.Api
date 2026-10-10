"""D27: the cpp slice's RPC per-call counts before and after (rows '  B/C/D/E...').
usage: python3 rdiff.py OLD NEW; prints per (cell family, direction, delta forward/reverse, delta host) the row count."""
import re, sys, collections
R = re.compile(r"^  (\S+)\s+(\S+)\s+(.*?)forward/call\s+([\d.]+)\s+reverse/call\s+([\d.]+).*host ([\d.]+)\)")
def load(p):
    o = {}
    for l in open(p):
        m = R.match(l)
        if m: o[(m.group(1), m.group(2), m.group(3).strip())] = (float(m.group(4)), float(m.group(5)), float(m.group(6)))
    return o
a, b = load(sys.argv[1]), load(sys.argv[2])
print("rows: old %d, new %d, same keys %s" % (len(a), len(b), a.keys() == b.keys()))
c = collections.Counter()
for k in a:
    d = tuple(round(y - x, 3) for x, y in zip(a[k], b.get(k, (0, 0, 0))))
    c[(k[0].split("-")[0], k[1], d)] += 1
for (cell, dr, d), n in sorted(c.items(), key=str):
    print("  %-6s %-4s d(forward, reverse, host)=%-22s %3d" % (cell, dr, d, n))
print("reverse unchanged and forward changes only by the host calls on every row: %s" % all(
    b[k][1] == a[k][1] and round(b[k][0] - a[k][0], 3) == round(b[k][2] - a[k][2], 3) for k in a if k in b))
