#!/usr/bin/env python3
"""EXPERIMENT backward-encode: tables from gen/bwd_bench.sh's output directory.

    gen/bwd_tables.py OUT_DIR [COUNTS_LOG] > tables.md

Absolute figures only, per variant (committed core, backward core), no ratios. Codec: ns of
process CPU per encode; per launch the median of its 10 criterion samples (cpu_ns / iters);
the cell shows the median of the launch medians and [min - max] of the launch medians, then
the p10-p90 of all samples of all launches. RPC: client process CPU per call (stream_probe
cpu_ns), median over rounds and processes, [min - max] of the round values.
CONTAINER INSTRUMENTATION.
"""
import glob
import json
import os
import re
import statistics
import sys

OUT = sys.argv[1]
COUNTS = sys.argv[2] if len(sys.argv) > 2 else None
VARS = os.environ.get("BWD_VARS", "committed,backward").split(",")


def q(v, p):
    v = sorted(v)
    if not v:
        return float("nan")
    k = (len(v) - 1) * p
    lo, hi = int(k), min(int(k) + 1, len(v) - 1)
    return v[lo] + (v[hi] - v[lo]) * (k - lo)


def fmt(x):
    if x >= 1e6:
        return "%.2f ms" % (x / 1e6)
    if x >= 1e4:
        return "%.1f us" % (x / 1e3)
    if x >= 1e3:
        return "%.2f us" % (x / 1e3)
    return "%.0f ns" % x


def load_codec():
    per = {}  # (variant, key) -> {launch: [ns per op]}
    for v in VARS:
        for f in sorted(glob.glob(os.path.join(OUT, "codec-%s-*.jsonl" % v))):
            for ln in open(f):
                if not ln.startswith("{"):
                    continue
                r = json.loads(ln)
                if r.get("dir") != "encode":
                    continue
                key = (r["payload"], r["arm"], r.get("unknown_mode", "default"), r.get("end_state", ""), r.get("input", ""))
                per.setdefault((v, key), {}).setdefault(r["launch"], []).append(r["cpu_ns"] / r["iters"])
    return per


def cell(d):
    if not d:
        return "n/a"
    meds = [statistics.median(x) for x in d.values()]
    allv = [y for x in d.values() for y in x]
    return "%s [%s - %s] (p10-p90 %s - %s, %d launches)" % (
        fmt(statistics.median(meds)), fmt(min(meds)), fmt(max(meds)), fmt(q(allv, 0.1)), fmt(q(allv, 0.9)), len(meds))


def order(p):
    m = re.match(r"P(\d+)\.(\d+)(.*)", p)
    return (int(m.group(1)), int(m.group(2)), m.group(3)) if m else (99, 0, p)


def main():
    print("# backward-encode: encode tables (CONTAINER INSTRUMENTATION, not campaign results)")
    print()
    hdr = os.path.join(OUT, "header.txt")
    if os.path.exists(hdr):
        print("```")
        print(open(hdr).read().rstrip())
        print("```")
        print()
    print("Absolute ns of process CPU per encode, per variant; no ratio is formed here. Each cell: the median of the")
    print("per-launch medians (10 criterion samples per launch), [min - max] of the launch medians, then p10-p90 of")
    print("every sample of every launch. `committed` = this branch's core and binding; `backward` = the patched core")
    print("with the binding that delivers every repeated field last to first. core-native (the forward ak_rt::Enc,")
    print("unchanged) is the in-process control row of each process. End states (requirement 11): reused-buffer, and")
    print("transport-ready-tonic (core-ffi: ak_enc_take_owned's moved buffer; core-native: Enc::take). Input: hot.")
    print()
    per = load_codec()
    keys = sorted({k for (_, k) in per}, key=lambda k: (order(k[0]), k[1], k[2], k[3], k[4]))
    pids = []
    for k in keys:
        if k[0] not in pids:
            pids.append(k[0])
    for pid in pids:
        print("## %s" % pid)
        print()
        print("| arm | mode | end state | " + " | ".join(VARS) + " |")
        print("|---|---|---|" + "---|" * len(VARS))
        for k in keys:
            if k[0] != pid:
                continue
            print("| %s | %s | %s | " % (k[1], k[2], k[3]) + " | ".join(cell(per.get((v, k))) for v in VARS) + " |")
        print()

    rpc = {}
    for v in VARS:
        for f in sorted(glob.glob(os.path.join(OUT, "rpc-%s-*.jsonl" % v))):
            for ln in open(f):
                if not ln.startswith("{"):
                    continue
                r = json.loads(ln)
                rpc.setdefault((r["cell"], r["size"]), {}).setdefault(v, []).append(r["cpu_ns"])
    if rpc:
        print("## RPC: directions d (16 MiB in 2 MiB chunks) and c (P5.4), k = 1, TCP 127.0.0.1, pinned configuration")
        print()
        print("Client process CPU per call (stream_probe `cpu_ns`, every call checked: status and the server's byte count),")
        print("median over every round of both processes per variant, [min - max] of the round values. Cell A (tonic +")
        print("prost, no core in its path) is the in-process control.")
        print()
        print("| cell | workload | committed | backward |")
        print("|---|---|---|---|")
        for (c, s), d in sorted(rpc.items()):
            def rc(x):
                return "%s [%s - %s] (%d rounds)" % (fmt(statistics.median(x)), fmt(min(x)), fmt(max(x)), len(x)) if x else "n/a"
            print("| %s | %s | %s | %s |" % (c, "d/" + s if "MiB" in s else "c/" + s, rc(d.get("committed", [])), rc(d.get("backward", []))))
        print()
    if COUNTS and os.path.exists(COUNTS):
        print("## Element calls per payload (counting build of the backward core, one warm encode)")
        print()
        print("```")
        txt = open(COUNTS).read()
        m = txt.split("## counting build", 1)
        body = m[1] if len(m) > 1 else txt
        for ln in body.splitlines():
            if ln.startswith("#") or re.match(r"^P\d", ln) or ln.startswith("payload"):
                print(ln)
            if ln.startswith("BWD CHECK"):
                break
        print("```")


main()
