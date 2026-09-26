#!/usr/bin/env python3
"""Summaries of one gen/opt_bench.sh run (CONTAINER INSTRUMENTATION). ABSOLUTE times.

    python3 gen/opt_summary.py OUT_DIR

Reads OUT_DIR/codec-*.jsonl, rpc-*.jsonl, calib.jsonl (the raw samples) and writes:

  summary-codec.tsv   one row per case: build (full | nounk), file (= process), input,
                      payload, content, arm, direction, unknown mode, encode variant
                      (end_state/input, or "-"), samples, median / min / max / q25 / q75 ns
                      per operation (process CPU of one criterion sample / its iterations),
                      spread = (max - min) / median
  variants-codec.tsv  the payload inputs (P*), one row per (input, direction, encode variant),
                      one COLUMN per variant, median ns per operation: prost, armonik,
                      native-drop, native-retain, ffi-drop, ffi-retain, pull-drop,
                      pull-retain, ffi-zc-drop, ffi-zc-retain (the full build's process),
                      then the no-unknown build's process: prost@nounk, armonik@nounk,
                      native-nounk, ffi-nounk, pull-nounk, ffi-zc-nounk. Empty = not run
                      (pull and ffi-zc decode only; ffi-zc on P5.* only; P7.1 decode only).
  variants-codec.txt  the same table, aligned, in microseconds, for reading
  unknown-retain-vs-drop.tsv  the U-* rows (full build): per arm (native, ffi, pull) and
                      direction (and encode variant), over the rows: the geometric mean,
                      min and max of retain / drop (same process), and the median absolute
                      ns of each mode; plus the no-unknown build's median absolute ns
  summary-rpc.tsv     one row per (file, transport, cell, dir, in-flight): ns per call,
                      process CPU and wall, median / min / max over rounds
  summary-calib.tsv   one row per arm: ns per iteration

No sample is dropped. Absolute times from different processes (the full and the no-unknown
build) are from different processes: compare them with that in mind.
"""
import json
import math
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


def quart(xs):
    xs = sorted(xs)
    def q(p):
        k = (len(xs) - 1) * p
        i = int(k)
        j = min(i + 1, len(xs) - 1)
        return xs[i] + (xs[j] - xs[i]) * (k - i)
    return q(0.25), q(0.75)


def gmean(v):
    return math.exp(sum(math.log(x) for x in v) / len(v)) if v else float("nan")


COLS = [("prost", "full", "incumbent-prod", "default"), ("armonik", "full", "armonik", "default"),
        ("native-drop", "full", "core-native", "drop"), ("native-retain", "full", "core-native", "retain"),
        ("ffi-drop", "full", "core-ffi", "drop"), ("ffi-retain", "full", "core-ffi", "retain"),
        ("pull-drop", "full", "core-ffi-pull", "drop"), ("pull-retain", "full", "core-ffi-pull", "retain"),
        ("ffi-zc-drop", "full", "core-ffi-zc", "drop"), ("ffi-zc-retain", "full", "core-ffi-zc", "retain"),
        ("prost@nounk", "nounk", "incumbent-prod", "default"), ("armonik@nounk", "nounk", "armonik", "default"),
        ("native-nounk", "nounk", "core-native", "no-unknown"), ("ffi-nounk", "nounk", "core-ffi", "no-unknown"),
        ("pull-nounk", "nounk", "core-ffi-pull", "no-unknown"), ("ffi-zc-nounk", "nounk", "core-ffi-zc", "no-unknown")]


def main(out):
    files = sorted(os.listdir(out))
    cases, order = defaultdict(list), []
    for fn in files:
        if not (fn.startswith("codec") and fn.endswith(".jsonl")):
            continue
        tag = fn[:-len(".jsonl")]
        build = "nounk" if "nounk" in tag else "full"
        for o in rows(os.path.join(out, fn)):
            var = "%s/%s" % (o["end_state"], o["input"]) if "end_state" in o else "-"
            k = (build, tag, o["payload"], o["content"], o["arm"], o["dir"], o.get("unknown_mode", "default"), var)
            if k not in cases:
                order.append(k)
            cases[k].append(o["cpu_ns"] / o["iters"])
    med = {}
    with open(os.path.join(out, "summary-codec.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh); ns per operation = process CPU per criterion sample / iterations\n")
        f.write("build\tfile\tinput\tpayload\tcontent\tarm\tdir\tmode\tvariant\tsamples\tmedian_ns\tmin_ns\tmax_ns\tq25_ns\tq75_ns\tspread\n")
        for k in order:
            v = sorted(cases[k])
            m = statistics.median(v)
            q1, q3 = quart(v)
            med[k] = m
            b, tag, inp, content, arm, d, mode, var = k
            f.write("\t".join([b, tag, inp, inp.split("/")[0], content, arm, d, mode, var, str(len(v)),
                               "%.1f" % m, "%.1f" % v[0], "%.1f" % v[-1], "%.1f" % q1, "%.1f" % q3,
                               "%.3f" % ((v[-1] - v[0]) / m if m else 0)]) + "\n")
    print("summary-codec.tsv: %d cases" % len(order))
    # ---- variants table (payloads)
    by = {}
    rowkeys = []
    for (b, tag, inp, content, arm, d, mode, var), m in med.items():
        if not inp.startswith("P"):
            continue
        rk = (inp, d, var)
        if rk not in by:
            by[rk] = {}
            rowkeys.append(rk)
        by[rk][(b, arm, mode)] = m
    def sk(rk):
        inp, d, var = rk
        p = inp.split("/")[0][1:].split(".")
        return (int(p[0]), int(p[1]), inp, ["encode", "decode", "decode-read"].index(d) if d in ("encode", "decode", "decode-read") else 9, var)
    rowkeys.sort(key=sk)
    with open(os.path.join(out, "variants-codec.tsv"), "w") as f, open(os.path.join(out, "variants-codec.txt"), "w") as g:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh): median ns per operation (process CPU), ABSOLUTE; full-build columns and @nounk columns are two processes\n")
        f.write("\t".join(["input", "dir", "variant"] + [c[0] for c in COLS]) + "\n")
        g.write("# CONTAINER INSTRUMENTATION: median microseconds per operation (process CPU), absolute; columns up to ffi-zc-retain are the full build's process, the last six the no-unknown build's\n")
        w = [12, 12, 26] + [max(9, len(c[0])) for c in COLS]
        g.write(" ".join(h.rjust(x) if i > 2 else h.ljust(x) for i, (h, x) in enumerate(zip(["input", "dir", "variant"] + [c[0] for c in COLS], w))) + "\n")
        for rk in rowkeys:
            vals = [by[rk].get((b, arm, mode)) for _, b, arm, mode in COLS]
            f.write("\t".join(list(rk) + ["%.1f" % v if v is not None else "" for v in vals]) + "\n")
            g.write(" ".join([rk[0].ljust(w[0]), rk[1].ljust(w[1]), rk[2].ljust(w[2])] +
                             [("%.2f" % (v / 1000) if v is not None else "").rjust(x) for v, x in zip(vals, w[3:])]) + "\n")
    print("variants-codec.tsv / .txt: %d rows" % len(rowkeys))
    # ---- U-* retain vs drop
    urows = defaultdict(lambda: defaultdict(dict))
    for (b, tag, inp, content, arm, d, mode, var), m in med.items():
        if inp.startswith("U-"):
            urows[(arm, d, var)][inp][(b, mode)] = m
    with open(os.path.join(out, "unknown-retain-vs-drop.tsv"), "w") as f:
        f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh): the U-* rows; retain/drop = same process (full build); absolute medians over the rows in ns\n")
        f.write("arm\tdir\tvariant\trows\tgmean_retain/drop\tmin\tmax\tmedian_abs_drop_ns\tmedian_abs_retain_ns\tmedian_abs_nounk_ns\n")
        for (arm, d, var) in sorted(urows):
            if arm not in ("core-native", "core-ffi", "core-ffi-pull"):
                continue
            rs = urows[(arm, d, var)]
            ratio = [x[("full", "retain")] / x[("full", "drop")] for x in rs.values()
                     if ("full", "retain") in x and ("full", "drop") in x]
            ab = lambda key: statistics.median([x[key] for x in rs.values() if key in x]) if any(key in x for x in rs.values()) else float("nan")
            if not ratio:
                continue
            f.write("\t".join([arm, d, var, str(len(ratio)), "%.3f" % gmean(ratio), "%.3f" % min(ratio), "%.3f" % max(ratio),
                               "%.1f" % ab(("full", "drop")), "%.1f" % ab(("full", "retain")), "%.1f" % ab(("nounk", "no-unknown"))]) + "\n")
    print("unknown-retain-vs-drop.tsv written")
    # ---- rpc
    rp, rorder = defaultdict(lambda: ([], [])), []
    for fn in files:
        if not (fn.startswith("rpc-") and fn.endswith(".jsonl")):
            continue
        tag = fn[:-len(".jsonl")]
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
                c, w = sorted(rp[k][0]), sorted(rp[k][1])
                f.write("\t".join(map(str, k)) + "\t%d\t%.1f\t%.1f\t%.1f\t%.1f\t%.1f\t%.1f\n" % (
                    len(c), statistics.median(c), c[0], c[-1], statistics.median(w), w[0], w[-1]))
        print("summary-rpc.tsv: %d rows" % len(rorder))
    p = os.path.join(out, "calib.jsonl")
    if os.path.exists(p):
        cb = defaultdict(list)
        for o in rows(p):
            cb[o["arm"]].append(o["cpu_ns"] / o["iters"])
        with open(os.path.join(out, "summary-calib.tsv"), "w") as f:
            f.write("# CONTAINER INSTRUMENTATION (gen/opt_bench.sh); ns per iteration; reverse = forward-reverse - forward\n")
            f.write("arm\trounds\tmedian_ns\tmin_ns\tmax_ns\n")
            for a, v in cb.items():
                v = sorted(v)
                f.write("%s\t%d\t%.3f\t%.3f\t%.3f\n" % (a, len(v), statistics.median(v), v[0], v[-1]))
        print("summary-calib.tsv: %d arms" % len(cb))


if __name__ == "__main__":
    main(sys.argv[1])
