import sys, collections
def load(p):
    rows = collections.OrderedDict(); hdr = []
    for l in open(p):
        if l.startswith("#"): hdr.append(l.rstrip("\n")); continue
        f = l.split()
        rows[(f[0], f[1], f[2])] = tuple(int(x) for x in f[3:6])
    return hdr, rows
oh, o = load(sys.argv[1]); nh, n = load(sys.argv[2])
print("old rows %d, new rows %d" % (len(o), len(n)))
# 1. every old row other than decode/decode-read of codec inputs and rpc C/D a/a+read is unchanged
chg = collections.Counter(); bad = []
for k, v in o.items():
    inp, d, m = k
    if k not in n: bad.append(("missing", k)); continue
    if n[k] != v:
        cls = ("rpc:" + inp.split(":")[1][0] if inp.startswith("rpc:") else "codec") + " " + d
        chg[cls] += 1
added = [k for k in n if k not in o]
print("old rows changed, by class:", dict(chg))
print("missing old rows:", bad[:5], len(bad))
print("added rows: %d, directions %s" % (len(added), collections.Counter(k[1] for k in added)))
# 2. decode-push == old decode, every input and mode
pm = [k for k in added if k[1] == "decode-push" and n[k] != o.get((k[0], "decode", k[2]))]
print("decode-push rows != old (push) decode rows:", len(pm), pm[:3])
# 3. new decode: reverse 0? forward = old forward + old reverse - ? relation
rel = collections.Counter(); rev0 = 0; ex = []
for k, v in n.items():
    if k[1] in ("decode", "decode-read") and not k[0].startswith("rpc:"):
        ov = o[k]; rev0 += v[1] == 0
        # FSM forward = events (= push reverse) + resets + ak_dec_err(1)? report delta
        rel[(v[0] - ov[1] - v[2], ov[0] - ov[2], v[2] - ov[2])] += 1
        if len(ex) < 4: ex.append((k, ov, v))
print("FSM decode rows with reverse 0: %d of %d" % (rev0, sum(rel.values())))
print("(new fwd - old push reverse - new resets, old push fwd - old resets, resets delta): count", dict(rel))
for e in ex: print("  e.g.", e)
for k, v in n.items():
    if k[0].startswith("rpc:") and n[k] != o.get(k):
        print("  rpc changed", k, o.get(k), "->", v)
