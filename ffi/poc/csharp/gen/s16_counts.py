#!/usr/bin/env python3
"""gen/s16_counts.py DEFAULT ROE: s16, the reset-on-entry variant's codec counts against the
committed ones, row by row: the forward-call change and where it comes from. Every row must
satisfy  fwd_roe = fwd_default - ak_enc_reset_default - (ak_dec_reset_* removed)  with reverse
calls and grows unchanged; prints a summary per kind of row and every row that breaks it."""
import collections, re, sys


def load(p):
    rows = {}
    for l in open(p):
        if l.startswith("#") or "|" not in l:
            continue
        key, mid, ent = [x.strip() for x in l.split("|", 2)]
        m = dict(re.findall(r"(\w+) (\d+)", mid))
        e = dict((a, int(b)) for a, b in re.findall(r"(\w+)=(\d+)", ent))
        rows[key] = ({k: int(v) for k, v in m.items()}, e)
    return rows


d, r = load(sys.argv[1]), load(sys.argv[2])
bad, kinds = 0, collections.Counter()
if set(d) != set(r):
    print("ROW SETS DIFFER: only default %d, only roe %d" % (len(set(d) - set(r)), len(set(r) - set(d))))
    bad += 1
for k in sorted(set(d) & set(r)):
    (md, ed), (mr, er) = d[k], r[k]
    df = mr["fwd"] - md["fwd"]
    enc = ed.get("ak_enc_reset", 0) - er.get("ak_enc_reset", 0)
    dec = md.get("reset", 0) - mr.get("reset", 0)
    ok = df == -(enc + dec) and mr.get("rev") == md.get("rev") and mr.get("grow") == md.get("grow")
    others = {n: (ed.get(n, 0), er.get(n, 0)) for n in set(ed) | set(er)
              if n != "ak_enc_reset" and not n.startswith("ak_dec_reset_") and ed.get(n, 0) != er.get(n, 0)}
    if others:
        ok = False
    f = k.split()
    kinds[(f[3], f[4], df, enc, dec)] += 1
    if not ok:
        bad += 1
        print("BREAKS %s: fwd %d -> %d, ak_enc_reset %d -> %d, resets %d -> %d, other entries %s" % (
            k, md["fwd"], mr["fwd"], ed.get("ak_enc_reset", 0), er.get("ak_enc_reset", 0), md.get("reset", 0), mr.get("reset", 0), others))
print("# rows by direction, mode and change: fwd delta (= -(ak_enc_reset removed + ak_dec_reset_* removed)); rev and grow unchanged on every row")
print("| dir | mode | fwd change | ak_enc_reset removed | ak_dec_reset removed | rows |")
print("|---|---|---:|---:|---:|---:|")
for (dr, mo, df, enc, dec), n in sorted(kinds.items()):
    print("| %s | %s | %d | %d | %d | %d |" % (dr, mo, df, enc, dec, n))
print("%d rows compared, %d break the rule" % (len(set(d) & set(r)), bad))
sys.exit(1 if bad else 0)
