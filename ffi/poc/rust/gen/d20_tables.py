#!/usr/bin/env python3
"""FIX-PLAN D20: tables from gen/d20_bench.sh's output directory. CONTAINER INSTRUMENTATION.

    gen/d20_tables.py OUT_DIR > tables.md

Per (build, arm, mode, payload, content): ns of process CPU per decode-read; per process
(launch) the median of its 10 criterion samples (cpu_ns / iters). A cell is the median of
the launch medians [min - max of the launch medians]. Ratios are taken per launch between
the two processes of the same launch (same seed, same case order) and shown as the median
[min - max] over launches: after/before (all bits 0 against the tree before D20: different
processes, so the core-native column of the same processes is the control for what process
and build drift alone does) and skipall/after (every string bit set against all bits 0).
"""
import glob
import json
import os
import re
import statistics
import sys

OUT = sys.argv[1]
VARS = ["before", "after", "skipall"]


def fmt(x):
    if x != x:
        return "n/a"
    if x >= 1e6:
        return "%.2f ms" % (x / 1e6)
    if x >= 1e4:
        return "%.1f us" % (x / 1e3)
    if x >= 1e3:
        return "%.2f us" % (x / 1e3)
    return "%.0f ns" % x


def load():
    per = {}  # (variant, build, arm, mode, payload, content) -> {launch: [ns]}
    for f in sorted(glob.glob(os.path.join(OUT, "codec-*-*-*.jsonl"))):
        m = re.match(r"codec-(\w+)-(full|nounk)-(\d+)\.jsonl", os.path.basename(f))
        if not m:
            continue
        v, b = m.group(1), m.group(2)
        for ln in open(f):
            if not ln.startswith("{"):
                continue
            r = json.loads(ln)
            if r.get("dir") != "decode-read" or "cpu_ns" not in r:
                continue
            key = (v, b, r["arm"], r.get("unknown_mode", ""), r["payload"], r.get("content", ""))
            per.setdefault(key, {}).setdefault(r["launch"], []).append(r["cpu_ns"] / r["iters"])
    return per


def meds(d):
    return {k: statistics.median(x) for k, x in d.items()}


def cell(d):
    if not d:
        return "n/a"
    m = list(meds(d).values())
    return "%s [%s - %s]" % (fmt(statistics.median(m)), fmt(min(m)), fmt(max(m)))


def ratio(num, den):
    if not num or not den:
        return "n/a", None
    a, b = meds(num), meds(den)
    rs = [a[k] / b[k] for k in a if k in b]
    if not rs:
        return "n/a", None
    return "%.3f [%.3f - %.3f]" % (statistics.median(rs), min(rs), max(rs)), rs


def order(p):
    m = re.match(r"P(\d+)\.(\d+)(.*)", p)
    return (0, int(m.group(1)), int(m.group(2)), m.group(3)) if m else (1, 0, 0, p)


def main():
    per = load()
    print("# D20: decode-read rows, before / after (all bits 0) / skipall (every string bit set)")
    print()
    print("CONTAINER INSTRUMENTATION, not campaign results. ns of process CPU per decode-read;")
    print("cell = median of the per-process medians [min - max]; ratio = per launch, median [min - max].")
    print()
    hdr = os.path.join(OUT, "header.txt")
    if os.path.exists(hdr):
        print("```")
        print(open(hdr).read().rstrip())
        print("```")
        print()
    summary = {}
    for b in ("full", "nounk"):
        keys = sorted({(k[2], k[3], k[4], k[5]) for k in per if k[1] == b},
                      key=lambda k: (k[1], order(k[2]), k[3], k[0]))
        modes = sorted({k[1] for k in keys})
        for mode in modes:
            print("## %s build, mode %s" % ("full" if b == "full" else "no-unknown", mode))
            print()
            print("| payload | content | core-ffi before | core-ffi after | core-ffi skipall | after/before | control: core-native after/before | skipall/after |")
            print("|---|---|---|---|---|---|---|---|")
            pcs = sorted({(k[2], k[3]) for k in keys if k[1] == mode}, key=lambda x: (order(x[0]), x[1]))
            for p, c in pcs:
                g = lambda v, arm: per.get((v, b, arm, mode, p, c), {})
                ab, rab = ratio(g("after", "core-ffi"), g("before", "core-ffi"))
                cn, rcn = ratio(g("after", "core-native"), g("before", "core-native"))
                sa, rsa = ratio(g("skipall", "core-ffi"), g("after", "core-ffi"))
                for name, rs in (("after/before", rab), ("native after/before", rcn), ("skipall/after", rsa)):
                    if rs:
                        summary.setdefault((b, mode, name), []).append(statistics.median(rs))
                print("| %s | %s | %s | %s | %s | %s | %s | %s |" % (
                    p, c, cell(g("before", "core-ffi")), cell(g("after", "core-ffi")),
                    cell(g("skipall", "core-ffi")), ab, cn, sa))
            print()
    print("## Summary: per-row median ratios, over the rows of each table")
    print()
    print("| build, mode | ratio | rows | median | min | max |")
    print("|---|---|---|---|---|---|")
    for (b, mode, name), v in sorted(summary.items()):
        print("| %s, %s | %s | %d | %.3f | %.3f | %.3f |" % (b, mode, name, len(v), statistics.median(v), min(v), max(v)))
    print()


if __name__ == "__main__":
    main()
