#!/usr/bin/env python3
"""Row-by-row comparison of two gen/opt_bench.sh runs (CONTAINER INSTRUMENTATION).

    python3 gen/opt_compare.py BEFORE_DIR AFTER_DIR [--rows all|changed|none] [--tsv FILE]

Both directories must hold summary-codec.tsv with the q25/q75 columns (gen/opt_summary.py
writes them; re-run it on an older run directory to add them) and, optionally,
summary-rpc.tsv.

Codec. For every (file = process, input, direction, core mode) and every ratio
  core-native/inc, core-ffi/inc, core-ffi-pull/inc, armonik/inc, core-ffi/core-native
(inc = incumbent-prod of the SAME process), the ratio of medians is formed in each run and
compared: change = after / before - 1. The ratio's noise band in one run is
[q25(num) / q75(den), q75(num) / q25(den)] from the per-sample quartiles of the two cases.
A row is flagged `noise` when the two runs' bands overlap, else `lower` or `higher` (lower
= fewer CPU ns per op relative to the denominator). This is a heuristic band, not a test.

Also printed: per group (file, ratio, direction, mode) the geometric mean of after/before
over its inputs and the counts of lower / higher / noise, and the drift of the in-process
control (incumbent-prod's own median, after/before, per file): absolutes do not carry
between processes, so a ratio change of the size of that drift is not attributable.

RPC. Per (file, transport, direction, in-flight), each cell's client CPU per call over cell
A's of the same process; band from the rounds' min/max; same flags.
"""
import argparse
import math
import os
import sys
from collections import defaultdict

RATIOS = [
    ("core-native/inc", "core-native", "incumbent-prod"),
    ("core-ffi/inc", "core-ffi", "incumbent-prod"),
    ("core-ffi-pull/inc", "core-ffi-pull", "incumbent-prod"),
    ("armonik/inc", "armonik", "incumbent-prod"),
    ("core-ffi/core-native", "core-ffi", "core-native"),
]


def read_tsv(path):
    with open(path) as f:
        lines = [l.rstrip("\n") for l in f if not l.startswith("#") and l.strip()]
    head = lines[0].split("\t")
    return [dict(zip(head, l.split("\t"))) for l in lines[1:]]


def codec(d):
    p = os.path.join(d, "summary-codec.tsv")
    rows = read_tsv(p)
    if rows and "q25_ns" not in rows[0]:
        sys.exit(f"{p} has no q25/q75 columns: run python3 gen/opt_summary.py {d} first")
    out = {}
    for r in rows:
        k = (r["file"], r["input"], r["dir"], r["arm"], r["mode"])
        out[k] = tuple(float(r[c]) for c in ("median_ns", "q25_ns", "q75_ns"))
    return out


def ratio_rows(c):
    """{(file, input, dir, mode, ratio): (value, lo, hi)}"""
    res = {}
    keys = {(f, i, d, m) for (f, i, d, a, m) in c if m != "default"}
    for (f, i, d, m) in keys:
        for name, num, den in RATIOS:
            n = c.get((f, i, d, num, m if num.startswith("core") else "default"))
            dd = c.get((f, i, d, den, m if den.startswith("core") else "default"))
            if n and dd:
                res[(f, i, d, m, name)] = (n[0] / dd[0], n[1] / dd[2], n[2] / dd[1])
    return res


def flag(b, a):
    if a[1] <= b[2] and b[1] <= a[2]:
        return "noise"
    return "lower" if a[0] < b[0] else "higher"


def gmean(xs):
    return math.exp(sum(math.log(x) for x in xs) / len(xs)) if xs else float("nan")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("before")
    ap.add_argument("after")
    ap.add_argument("--rows", default="changed", choices=["all", "changed", "none"])
    ap.add_argument("--tsv")
    a = ap.parse_args()
    cb, ca = codec(a.before), codec(a.after)
    rb, ra = ratio_rows(cb), ratio_rows(ca)
    common = sorted(set(rb) & set(ra))
    print(f"# opt_compare  before={a.before}  after={a.after}  (CONTAINER INSTRUMENTATION)")
    print(f"# {len(common)} codec ratio rows in both runs ({len(set(rb) - set(ra))} only before, {len(set(ra) - set(rb))} only after)")
    tsv = open(a.tsv, "w") if a.tsv else None
    if tsv:
        tsv.write("file\tinput\tdir\tmode\tratio\tbefore\tafter\tchange\tflag\tbefore_lo\tbefore_hi\tafter_lo\tafter_hi\n")
    groups = defaultdict(list)
    if a.rows != "none":
        print("\nfile\tinput\tdir\tmode\tratio\tbefore\tafter\tchange\tflag")
    for k in common:
        b, x = rb[k], ra[k]
        fl = flag(b, x)
        ch = x[0] / b[0] - 1
        f, i, d, m, name = k
        groups[(f, name, d, m)].append((x[0] / b[0], fl))
        line = f"{f}\t{i}\t{d}\t{m}\t{name}\t{b[0]:.3f}\t{x[0]:.3f}\t{ch * 100:+.1f}%\t{fl}"
        if a.rows == "all" or (a.rows == "changed" and fl != "noise"):
            print(line)
        if tsv:
            tsv.write(line + f"\t{b[1]:.3f}\t{b[2]:.3f}\t{x[1]:.3f}\t{x[2]:.3f}\n")
    print("\n# per group: geometric mean of after/before over inputs; counts lower / higher / noise")
    print("file\tratio\tdir\tmode\tn\tgmean_after/before\tlower\thigher\tnoise")
    for (f, name, d, m), v in sorted(groups.items()):
        c = defaultdict(int)
        for _, fl in v:
            c[fl] += 1
        print(f"{f}\t{name}\t{d}\t{m}\t{len(v)}\t{gmean([r for r, _ in v]):.3f}\t{c['lower']}\t{c['higher']}\t{c['noise']}")
    print("\n# in-process control drift: incumbent-prod median after/before, per file and direction (geometric mean over inputs)")
    drift = defaultdict(list)
    for k, v in ca.items():
        if k[3] == "incumbent-prod" and k in cb:
            drift[(k[0], k[2])].append(v[0] / cb[k][0])
    for (f, d), v in sorted(drift.items()):
        print(f"{f}\t{d}\tn={len(v)}\tgmean {gmean(v):.3f}\trange [{min(v):.3f}, {max(v):.3f}]")
    # ---- rpc
    pb, pa = os.path.join(a.before, "summary-rpc.tsv"), os.path.join(a.after, "summary-rpc.tsv")
    if os.path.exists(pb) and os.path.exists(pa):
        def rpc(p):
            out = {}
            for r in read_tsv(p):
                out[(r["file"], r["transport"], r["dir"], r["inflight"], r["cell"])] = tuple(
                    float(r[c]) for c in ("cpu_median_ns", "cpu_min_ns", "cpu_max_ns"))
            return out
        xb, xa = rpc(pb), rpc(pa)
        def rr(x):
            res = {}
            for (f, t, d, k, cell), v in x.items():
                A = x.get((f, t, d, k, "A"))
                if cell != "A" and A:
                    res[(f, t, d, k, cell)] = (v[0] / A[0], v[1] / A[2], v[2] / A[1])
            return res
        qb, qa = rr(xb), rr(xa)
        print("\n# RPC: client CPU per call, cell / cell A of the same process; band from the rounds' min/max")
        print("file\ttransport\tdir\tinflight\tcell\tbefore\tafter\tchange\tflag")
        for k in sorted(set(qb) & set(qa)):
            b, x = qb[k], qa[k]
            print("\t".join(k) + f"\t{b[0]:.3f}\t{x[0]:.3f}\t{(x[0] / b[0] - 1) * 100:+.1f}%\t{flag(b, x)}")
        print("\n# RPC control drift: cell A client CPU per call, after/before")
        for k in sorted(set(xb) & set(xa)):
            if k[4] == "A":
                print("\t".join(k[:4]) + f"\t{xa[k][0] / xb[k][0]:.3f}")


if __name__ == "__main__":
    main()
