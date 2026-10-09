#!/usr/bin/env python3
"""FIX-PLAN D23: the tables of gen/d23_bench.sh (container instrumentation).

  gen/d23_tables.py OUT_DIR EVENTS_LOG

Main table: absolute decode-read times per (input, mode) for core-ffi (push), core-ffi-pull
and core-ffi-fsm: the median over launches of each launch's median ns per op (criterion's
10 rounds), with the range of the launch medians. No ratios. Second table: the FSM's events
per decode (= pull's records) and its calls, from the counting build's fsm_diff (EVENTS_LOG).
"""
import glob
import json
import os
import re
import statistics
import sys

ARMS = [("core-ffi", "push"), ("core-ffi-pull", "pull"), ("core-ffi-fsm", "fsm")]


def fmt(ns):
    if ns >= 1e6:
        return "%.3g ms" % (ns / 1e6)
    if ns >= 1e3:
        return "%.3g us" % (ns / 1e3)
    return "%.3g ns" % ns


def main(out, events_log):
    per = {}   # (payload, mode, arm) -> {launch: [ns/op per round]}
    heads = []
    for path in sorted(glob.glob(os.path.join(out, "codec-*.jsonl"))):
        for ln in open(path):
            if ln.startswith("#"):
                if "arm order" in ln:
                    heads.append(ln.strip())
                continue
            r = json.loads(ln)
            if r["dir"] != "decode-read":
                continue
            k = (r["payload"], r.get("unknown_mode", "default"), r["arm"])
            per.setdefault(k, {}).setdefault(r["launch"], []).append(r["cpu_ns"] / r["iters"])
    events = {}
    if events_log and os.path.exists(events_log):
        for ln in open(events_log):
            m = re.match(r"^(\S+)\s+(\S+)\s+(\d+)\s+(drop|retain|no-unknown)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+ok", ln)
            if m:
                events[(m.group(1), m.group(4))] = (int(m.group(3)), int(m.group(5)), int(m.group(6)), int(m.group(7)),
                                                    int(m.group(8)), int(m.group(9)), int(m.group(10)), int(m.group(11)))
    o = ["# D23: decode-read through push, pull and the FSM (container instrumentation)", "",
         "Source: `%s` (`gen/d23_bench.sh`); every figure is container instrumentation, not a "
         "campaign result. Per cell: the median over launches of each launch's median ns per "
         "decode (criterion, 10 rounds, process CPU), and in brackets the range of the launch "
         "medians. One process per launch holds all three arms (same core, same binding, same "
         "facade); arm blocks and cases in a seeded random order per launch." % os.path.basename(out.rstrip("/")), ""]
    for h in heads:
        o.append("    " + h[:200])
    o.append("")
    o.append("| input | mode | push | pull | fsm |")
    o.append("|---|---|---|---|---|")
    keys = sorted(set((p, m) for p, m, _a in per), key=lambda x: (x[0].startswith("U-"), x[0], x[1]))
    for p, m in keys:
        cells = []
        for arm, _short in ARMS:
            d = per.get((p, m, arm))
            if not d:
                cells.append("-")
                continue
            meds = [statistics.median(v) for _l, v in sorted(d.items())]
            cells.append("%s [%s .. %s] (n=%d)" % (fmt(statistics.median(meds)), fmt(min(meds)), fmt(max(meds)), len(meds)))
        o.append("| %s | %s | %s |" % (p, m, " | ".join(cells)))
    o.append("")
    o.append("## Events per decode (counting build, `fsm_diff --no-malformed`)")
    o.append("")
    o.append("FSM events = pull records on every row (checked record for record); FSM calls = begin + "
             "(events - 1) next. Forward crossings counted in the core: pull = 1 (ak_parse_*), FSM = its calls; "
             "reverse = unknown-field grows (retain).")
    o.append("")
    o.append("| input | mode | bytes | pull records | FSM events | FSM calls | pull fwd | FSM fwd | pull rev | FSM rev |")
    o.append("|---|---|---|---|---|---|---|---|---|---|")
    for (p, m), v in sorted(events.items(), key=lambda x: (x[0][0].startswith("U-"), x[0])):
        o.append("| %s | %s | %d | %d | %d | %d | %d | %d | %d | %d |" % ((p, m) + v))
    print("\n".join(o))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None)
