import sys, collections
def load(p):
    r = {}
    for l in open(p):
        if l.startswith("#"): continue
        f = l.split(); r[(f[0], f[1], f[2])] = tuple(int(x) for x in f[3:6])
    return r
n = load(sys.argv[1])
ok = bad = 0; revrows = collections.Counter(); ex = []
for (i, d, m), v in n.items():
    if d not in ("decode", "decode-read") or i.startswith("rpc:"): continue
    p = n[(i, "decode-push", m)]
    events = v[0] - v[2] - 1          # FSM forward minus resets minus ak_dec_err
    grows = v[1]
    # push: forward = ak_decode + resets; reverse = events (vtable calls) + grows
    if p[0] - p[2] == 1 and events + grows == p[1] and v[2] == p[2]: ok += 1
    else:
        bad += 1
        if len(ex) < 5: ex.append(((i, d, m), v, p))
    if grows: revrows[m] += 1
print("FSM row == push row (forward: begin + next per further event + ak_dec_err + resets; reverse: unknown-field grows only; push reverse = events + grows):", ok, "hold,", bad, "do not")
print("FSM rows with reverse > 0 (grows), by mode:", dict(revrows))
for e in ex: print("  ", e)
