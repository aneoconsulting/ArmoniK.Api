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

--aa A1 A2 (recommended): two runs of the SAME tree (an A/A pair). The within-run band
above is far narrower than the run-to-run movement of one case's median between two
processes (measured: an A/A pair moves single rows by up to tens of percent), so with --aa
a row is flagged only when its |log change| also exceeds the 90th percentile of the A/A
pair's |log change| over the same group (file, ratio, direction, mode), and each group's
geometric mean is printed with the A/A pair's 2-sigma band for a group mean of that size
(the larger of 2 * sd(A/A log changes) / sqrt(n) and twice the A/A pair's own group
mean |log change|, since rows of one process are not independent); a group change inside
that band is `noise`.

Also printed: per group (file, ratio, direction, mode) the geometric mean of after/before
over its inputs and the counts of lower / higher / noise, and the drift of the in-process
control (incumbent-prod's own median, after/before, per file): absolutes do not carry
between processes, so a ratio change of the size of that drift is not attributable.

RPC. Per (file, transport, direction, in-flight), each cell's client CPU per call over cell
A's of the same process; band from the rounds' min/max; same flags.
"""
import argparse
import json
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
    ap.add_argument("--aa", nargs=2, metavar=("A1", "A2"))
    a = ap.parse_args()
    cb, ca = codec(a.before), codec(a.after)
    rb, ra = ratio_rows(cb), ratio_rows(ca)
    # A/A calibration per group: (p90 of |log change|, sd of log change)
    aa = {}
    if a.aa:
        r1, r2 = ratio_rows(codec(a.aa[0])), ratio_rows(codec(a.aa[1]))
        g = defaultdict(list)
        for k in set(r1) & set(r2):
            g[(k[0], k[4], k[2], k[3])].append(math.log(r2[k][0] / r1[k][0]))
        for k, v in g.items():
            av = sorted(abs(x) for x in v)
            mu = sum(v) / len(v)
            sd = math.sqrt(sum((x - mu) ** 2 for x in v) / max(1, len(v) - 1))
            aa[k] = (av[min(len(av) - 1, int(0.9 * len(av)))], sd, mu)
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
        cal = aa.get((f, name, d, m))
        if cal and fl != "noise" and abs(math.log(x[0] / b[0])) <= cal[0]:
            fl = "noise"
        groups[(f, name, d, m)].append((x[0] / b[0], fl))
        line = f"{f}\t{i}\t{d}\t{m}\t{name}\t{b[0]:.3f}\t{x[0]:.3f}\t{ch * 100:+.1f}%\t{fl}"
        if a.rows == "all" or (a.rows == "changed" and fl != "noise"):
            print(line)
        if tsv:
            tsv.write(line + f"\t{b[1]:.3f}\t{b[2]:.3f}\t{x[1]:.3f}\t{x[2]:.3f}\n")
    print("\n# per group: geometric mean of after/before over inputs; counts lower / higher / noise"
          + ("; aa_band = the A/A 2-sigma band of a group mean, verdict noise inside it" if aa else ""))
    print("file\tratio\tdir\tmode\tn\tgmean_after/before\tlower\thigher\tnoise" + ("\taa_band\tgroup" if aa else ""))
    for (f, name, d, m), v in sorted(groups.items()):
        c = defaultdict(int)
        for _, fl in v:
            c[fl] += 1
        gm = gmean([r for r, _ in v])
        extra = ""
        cal = aa.get((f, name, d, m))
        if cal:
            band = max(2 * cal[1] / math.sqrt(len(v)), 2 * abs(cal[2]))
            verdict = "noise" if abs(math.log(gm)) <= band else ("lower" if gm < 1 else "higher")
            extra = f"\t+-{(math.exp(band) - 1) * 100:.1f}%\t{verdict}"
        print(f"{f}\t{name}\t{d}\t{m}\t{len(v)}\t{gm:.3f}\t{c['lower']}\t{c['higher']}\t{c['noise']}{extra}")
    print("\n# in-process control drift: incumbent-prod median after/before, per file and direction (geometric mean over inputs)")
    drift = defaultdict(list)
    for k, v in ca.items():
        if k[3] == "incumbent-prod" and k in cb:
            drift[(k[0], k[2])].append(v[0] / cb[k][0])
    for (f, d), v in sorted(drift.items()):
        print(f"{f}\t{d}\tn={len(v)}\tgmean {gmean(v):.3f}\trange [{min(v):.3f}, {max(v):.3f}]")
    # Temporal structure of the per-case drift (after/before of each case's own median),
    # along the order the cases were run in (the JSONL row order): a high lag-1
    # autocorrelation means the machine's speed drifted over time, not per case.
    print("\n# per-case drift along the run order: sd of log(after/before) and its autocorrelation")
    for fn in sorted(os.listdir(a.after)):
        if not (fn.startswith("codec") and fn.endswith(".jsonl")) or not os.path.exists(os.path.join(a.before, fn)):
            continue
        def meds(p):
            order, d = [], defaultdict(list)
            for l in open(p):
                if l.startswith("#") or not l.strip():
                    continue
                o = json.loads(l)
                k = (o["arm"], o["payload"], o["dir"], o.get("unknown_mode", ""))
                if k not in d:
                    order.append(k)
                d[k].append(o["cpu_ns"] / o["iters"])
            return order, {k: sorted(v)[len(v) // 2] for k, v in d.items()}
        o1, m1 = meds(os.path.join(a.before, fn))
        o2, m2 = meds(os.path.join(a.after, fn))
        x = [math.log(m2[k] / m1[k]) for k in o2 if k in m1]
        if len(x) < 30:
            continue
        mu = sum(x) / len(x)
        var = sum((v - mu) ** 2 for v in x) / len(x) or 1e-12
        ac = lambda lag: sum((x[i] - mu) * (x[i + lag] - mu) for i in range(len(x) - lag)) / (len(x) - lag) / var
        print(f"{fn[:-6]}\tn={len(x)}\tsd {math.sqrt(var):.3f}\tlag1 {ac(1):.2f}\tlag5 {ac(5):.2f}\tlag20 {ac(20):.2f}")
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
