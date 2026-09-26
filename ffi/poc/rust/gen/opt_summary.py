#!/usr/bin/env python3
"""Compact summaries of one gen/opt_bench.sh run (CONTAINER INSTRUMENTATION).

    python3 gen/opt_summary.py OUT_DIR

Reads OUT_DIR/codec-*.jsonl, rpc-*.jsonl, calib.jsonl (the raw samples) and writes:

  summary-codec.tsv  one row per case: file (= process), input id, payload, content, arm,
                     direction, unknown mode, samples, median / min / max ns per operation
                     (thread CPU of one criterion sample divided by its iteration count),
                     spread = (max - min) / median
  ratios-codec.tsv   one row per (file, input, direction, core mode): ratios of MEDIANS formed
                     inside that one process -- armonik, core-native, core-ffi, core-ffi-pull
                     over incumbent-prod, and core-ffi over core-native -- plus the same
                     ratios of the per-case MINIMA as a check on the medians
  summary-rpc.tsv    one row per (file, transport, cell, dir, in-flight): ns per call,
                     process CPU and wall, median / min / max over rounds
  summary-calib.tsv  one row per arm: ns per iteration, median / min / max over rounds

No sample is dropped. Nothing here compares across files (processes) except by the ratios
each file carries against its own incumbent-prod.
"""
import json
import os
import statistics
import sys
from collections import defaultdict


def rows(path):
    with open(path) as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            yield json.loads(line)


def stats(xs):
    xs = sorted(xs)
    med = statistics.median(xs)
    return med, xs[0], xs[-1], (xs[-1] - xs[0]) / med if med else float("nan")


def fmt(x, nd=1):
    return f"{x:.{nd}f}"


def main(out):
    files = sorted(os.listdir(out))
    # ---- codec
    cases = defaultdict(list)
    order = []
    for fn in files:
        if not (fn.startswith("codec") and fn.endswith(".jsonl")):
            continue
        tag = fn[: -len(".jsonl")]
        for o in rows(os.path.join(out, fn)):
            k = (tag, o["payload"], o["content"], o["arm"], o["dir"], o.get("unknown_mode", "default"))
            if k not in cases:
                order.append(k)
            cases[k].append(o["cpu_ns"] / o["iters"])
    med = {}
    with open(os.path.join(out, "summary-codec.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh); ns per operation = thread CPU per criterion sample / iterations\n")
        f.write("file\tinput\tpayload\tcontent\tarm\tdir\tmode\tsamples\tmedian_ns\tmin_ns\tmax_ns\tspread\n")
        for k in order:
            tag, inp, content, arm, d, mode = k
            m, lo, hi, sp = stats(cases[k])
            med[k] = (m, lo)
            payload = inp.split("/")[0]
            f.write(f"{tag}\t{inp}\t{payload}\t{content}\t{arm}\t{d}\t{mode}\t{len(cases[k])}\t{fmt(m)}\t{fmt(lo)}\t{fmt(hi)}\t{fmt(sp, 3)}\n")
    # ratios per (file, input, dir, core mode), against the same process's incumbent-prod
    groups = defaultdict(dict)
    for (tag, inp, content, arm, d, mode), v in med.items():
        groups[(tag, inp, d)][(arm, mode)] = v
    nratio = 0
    with open(os.path.join(out, "ratios-codec.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh); ratios of per-case medians inside one process "
                "(and, *_min, of per-case minima); < 1 means fewer CPU ns than the denominator; empty = arm not run\n")
        f.write("file\tinput\tdir\tmode\tincumbent_prod_median_ns\tarmonik/inc\tcore-native/inc\tcore-ffi/inc\t"
                "core-ffi-pull/inc\tcore-ffi/core-native\tcore-native/inc_min\tcore-ffi/inc_min\n")
        seen = set()
        for k in order:
            tag, inp, d = k[0], k[1], k[4]
            if (tag, inp, d) in seen:
                continue
            seen.add((tag, inp, d))
            g = groups[(tag, inp, d)]
            inc = g.get(("incumbent-prod", "default"))
            modes = sorted({m for (a, m) in g if m != "default"})
            for mode in modes:
                def r(a, b, i=0):
                    x, y = g.get(a), g.get(b)
                    return fmt(x[i] / y[i], 3) if x and y else ""
                incm = ("incumbent-prod", "default")
                f.write("\t".join([
                    tag, inp, d, mode, fmt(inc[0]) if inc else "",
                    r(("armonik", "default"), incm), r(("core-native", mode), incm),
                    r(("core-ffi", mode), incm), r(("core-ffi-pull", mode), incm),
                    r(("core-ffi", mode), ("core-native", mode)),
                    r(("core-native", mode), incm, 1), r(("core-ffi", mode), incm, 1),
                ]) + "\n")
                nratio += 1
    print(f"summary-codec.tsv: {len(order)} cases; ratios-codec.tsv: {nratio} rows")
    # ---- rpc
    rp = defaultdict(lambda: ([], []))
    rorder = []
    for fn in files:
        if not (fn.startswith("rpc-") and fn.endswith(".jsonl")):
            continue
        tag = fn[: -len(".jsonl")]
        for o in rows(os.path.join(out, fn)):
            k = (tag, o["transport"], o["cell"], o["dir"], o["inflight"])
            if k not in rp:
                rorder.append(k)
            rp[k][0].append(o["cpu_ns"] / o["iters"])
            rp[k][1].append(o["wall_ns"] / o["iters"])
    if rorder:
        with open(os.path.join(out, "summary-rpc.tsv"), "w") as f:
            f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh); ns per call = client process CPU (or wall) per round / calls in the round\n")
            f.write("file\ttransport\tcell\tdir\tinflight\trounds\tcpu_median_ns\tcpu_min_ns\tcpu_max_ns\twall_median_ns\twall_min_ns\twall_max_ns\n")
            for k in rorder:
                c, w = rp[k]
                cm, clo, chi, _ = stats(c)
                wm, wlo, whi, _ = stats(w)
                f.write("\t".join(map(str, k)) + f"\t{len(c)}\t{fmt(cm)}\t{fmt(clo)}\t{fmt(chi)}\t{fmt(wm)}\t{fmt(wlo)}\t{fmt(whi)}\n")
        print(f"summary-rpc.tsv: {len(rorder)} rows")
    # ---- calib
    p = os.path.join(out, "calib.jsonl")
    if os.path.exists(p):
        cb = defaultdict(list)
        for o in rows(p):
            cb[o["arm"]].append(o["cpu_ns"] / o["iters"])
        with open(os.path.join(out, "summary-calib.tsv"), "w") as f:
            f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh); ns per iteration, thread CPU; reverse = forward-reverse - forward\n")
            f.write("arm\trounds\tmedian_ns\tmin_ns\tmax_ns\n")
            for a, v in cb.items():
                m, lo, hi, _ = stats(v)
                f.write(f"{a}\t{len(v)}\t{fmt(m, 3)}\t{fmt(lo, 3)}\t{fmt(hi, 3)}\n")
        print(f"summary-calib.tsv: {len(cb)} arms")


if __name__ == "__main__":
    main(sys.argv[1])
