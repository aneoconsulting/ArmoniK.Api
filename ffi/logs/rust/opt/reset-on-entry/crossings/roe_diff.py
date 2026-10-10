"""reset-on-entry: the variant's crossing counts against the default's, row by row.
usage: python3 roe_diff.py DEFAULT VARIANT. Prints every class of change (direction, mode,
delta forward / reverse / resets) with its row count, and the rows that change otherwise."""
import sys, collections
def load(p):
    r = {}
    for l in open(p):
        if l.startswith("#"): continue
        f = l.split(); r[(f[0], f[1], f[2])] = tuple(int(x) for x in f[3:6])
    return r
a, b = load(sys.argv[1]), load(sys.argv[2])
assert a.keys() == b.keys(), "row sets differ"
cls = collections.Counter(); ex = {}
for k in a:
    d = tuple(y - x for x, y in zip(a[k], b[k]))
    kind = "rpc:" + k[0].split(":")[1] if k[0].startswith("rpc:") else "codec"
    c = (kind, k[1], k[2], d)
    cls[c] += 1; ex.setdefault(c, (k, a[k], b[k]))
print("rows %d; (row kind, direction, mode, delta forward/reverse/resets): rows, example (default -> variant)" % len(a))
for c, n in sorted(cls.items()):
    k, x, y = ex[c]
    print("  %-10s %-12s %-10s d=%-12s %5d   e.g. %s %s -> %s" % (c[0], c[1], c[2], c[3], n, k[0], x, y))
# The summary rule: every row's forward falls by exactly its default resets, reverse unchanged,
# and the variant's resets column is 0 on every row.
ok = sum(1 for k in a if b[k][0] == a[k][0] - a[k][2] and b[k][1] == a[k][1] and b[k][2] == 0)
print("rows where variant forward = default forward - default resets, reverse equal, variant resets 0: %d of %d" % (ok, len(a)))
print("rows with a default reset (so changed): %d; unchanged rows: %d" % (sum(1 for k in a if a[k][2]), sum(1 for k in a if a[k] == b[k])))
