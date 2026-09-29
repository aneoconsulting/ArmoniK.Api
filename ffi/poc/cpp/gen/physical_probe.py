#!/usr/bin/env python3
"""Tables of one gen/physical_probe.sh segment, from its raw samples (DIR/pass-*.jsonl).
Absolute times per call (ms), never a ratio.

  physical_probe.py DIR > DIR/tables.md

A sample is one Google Benchmark repetition: cpu_ns (process CPU) and wall_ns over `iters`
calls (iterations x k), and the process's getrusage deltas over the same timed loop.
  1. per workload (payload, k) and cell: per-call CPU and wall, median [p10-p90] over every
     repetition of the passes of one kind (S = spread, M = main), context switches and minor
     faults per call (median), batches per repetition and wall per repetition (median);
  2. per pass: the median per-call CPU of every cell;
  3. the in-process gap of every cell to A, per pass: median(cell) - median(A), CPU and wall,
     and its range over the passes of that kind; the spread (threshold) = the range over the
     spread passes of D - A and of Cf - A.
"""
import collections
import glob
import json
import os
import statistics
import sys


def q(xs, p):
    xs = sorted(xs)
    if len(xs) == 1:
        return xs[0]
    return statistics.quantiles(xs, n=10, method="inclusive")[{10: 0, 90: 8}[p]]


def main(d):
    rows = []
    for f in sorted(glob.glob(os.path.join(d, "pass-*.jsonl"))):
        for line in open(f):
            if line.startswith("{"):
                rows.append(json.loads(line))
    if not rows:
        print("no samples in %s" % d)
        return 1
    seg = rows[0].get("segment", "?")
    base = lambda c: c.split("-")[0] if c != "A" else "A"
    wl_key = lambda r: (r["dir"], r["payload"], r["inflight"])
    wls = sorted({wl_key(r) for r in rows}, key=lambda w: (w[0], {"P5.4": 0, "4MiB": 1, "16MiB": 2}.get(w[1], 3), w[2]))
    order = ["A", "D", "Cf", "Cf-q", "C", "C-q"]
    cells = sorted({r["cell"] for r in rows}, key=lambda c: next((i for i, o in enumerate(order) if c in (o, o + "-retain", o + "-drop")), 9))
    kinds = sorted({r["pass"][0] for r in rows}, reverse=True)  # S before M
    passes = sorted({r["pass"] for r in rows}, key=lambda p: (p[0] != "S", int(p[1:])))
    per = lambda r: (r["cpu_ns"] / r["iters"] / 1e6, r["wall_ns"] / r["iters"] / 1e6)
    g = collections.defaultdict(list)
    for r in rows:
        g[(wl_key(r), r["cell"], r["pass"])].append(r)
    out = []
    out.append("# cpp physical probe, segment `%s`: per-call client times (ms), absolute, no ratio" % seg)
    out.append("")
    out.append("Samples: %d repetitions from %d passes (%s). Per call = repetition time / (iterations x k). "
               "p10-p90 over every repetition of the passes of one kind. csw = voluntary + involuntary context "
               "switches of the whole client process per call, flt = minor faults per call (getrusage)." %
               (len(rows), len(passes), ", ".join(passes)))
    for kind in kinds:
        kp = [p for p in passes if p[0] == kind]
        out.append("")
        out.append("## 1%s. %s passes (%s): per-call CPU and wall" % (kind, "spread" if kind == "S" else "main", ", ".join(kp)))
        out.append("")
        out.append("| workload | cell | CPU median [p10-p90] | wall median [p10-p90] | vcsw | ivcsw | minflt | batches/rep | wall/rep ms | n |")
        out.append("|---|---|---|---|---|---|---|---|---|---|")
        for w in wls:
            for c in cells:
                rs = [r for p in kp for r in g.get((w, c, p), [])]
                if not rs:
                    continue
                cpu = [per(r)[0] for r in rs]; wall = [per(r)[1] for r in rs]
                calls = [r["iters"] for r in rs]
                pc = lambda k: statistics.median([r.get(k, float("nan")) / r["iters"] for r in rs])
                out.append("| %s %s k=%d | %s | %.3f [%.3f-%.3f] | %.3f [%.3f-%.3f] | %.1f | %.1f | %.0f | %d | %.0f | %d |" % (
                    w[0], w[1], w[2], c, statistics.median(cpu), q(cpu, 10), q(cpu, 90),
                    statistics.median(wall), q(wall, 10), q(wall, 90), pc("ru_nvcsw"), pc("ru_nivcsw"), pc("ru_minflt"),
                    statistics.median(calls) // w[2], statistics.median([r["wall_ns"] / 1e6 for r in rs]), len(rs)))
    out.append("")
    out.append("## 2. Median per-call CPU per pass (ms)")
    out.append("")
    out.append("| workload | cell | " + " | ".join(passes) + " |")
    out.append("|---|---|" + "---|" * len(passes))
    med = {}
    for w in wls:
        for c in cells:
            vals = []
            for p in passes:
                rs = g.get((w, c, p), [])
                if rs:
                    med[(w, c, p)] = (statistics.median([per(r)[0] for r in rs]), statistics.median([per(r)[1] for r in rs]))
                    vals.append("%.3f" % med[(w, c, p)][0])
                else:
                    vals.append("")
            if any(vals):
                out.append("| %s %s k=%d | %s | %s |" % (w[0], w[1], w[2], c, " | ".join(vals)))
    a = next((c for c in cells if c == "A"), None)
    out.append("")
    out.append("## 3. In-process gap to A per pass, median(cell) - median(A) (ms per call), and its range over the passes of one kind")
    out.append("")
    out.append("Spread (the threshold) = the range over the spread passes of D - A and of Cf - A, per workload and clock.")
    for ci, clk in ((0, "CPU"), (1, "wall")):
        out.append("")
        out.append("### %s" % clk)
        out.append("")
        out.append("| workload | cell | " + " | ".join(passes) + " | range S | range M | spread(D-A) | spread(Cf-A) |")
        out.append("|---|---|" + "---|" * len(passes) + "---|---|---|---|")
        for w in wls:
            gaps = {}
            for c in cells:
                if c == a:
                    continue
                gaps[c] = {p: med[(w, c, p)][ci] - med[(w, a, p)][ci] for p in passes if (w, c, p) in med and (w, a, p) in med}
            def rng(c, kind):
                v = [x for p, x in gaps.get(c, {}).items() if p[0] == kind]
                return (max(v) - min(v)) if len(v) > 1 else None
            dcell = next((c for c in gaps if base(c) == "D"), None)
            fcell = next((c for c in gaps if c.startswith("Cf-") and "-q-" not in c), None)
            sd, sf = rng(dcell, "S") if dcell else None, rng(fcell, "S") if fcell else None
            fmt = lambda x: "" if x is None else "%.3f" % x
            for c in gaps:
                out.append("| %s %s k=%d | %s | %s | %s | %s | %s | %s |" % (
                    w[0], w[1], w[2], c, " | ".join("%+.3f" % gaps[c][p] if p in gaps[c] else "" for p in passes),
                    fmt(rng(c, "S")), fmt(rng(c, "M")), fmt(sd), fmt(sf)))
    print("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
