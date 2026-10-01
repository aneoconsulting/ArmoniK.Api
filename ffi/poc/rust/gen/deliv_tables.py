#!/usr/bin/env python3
"""Tables of gen/delivery.sh: python3 gen/deliv_tables.py OUT_DIR > OUT_DIR/tables.md

RAW figures only (owner): per (session, workload, cell, condition), over every timed round of the
3 processes, median [p10-p90] per call of client CPU (task-clock: every client thread's run time,
softirq work in their context included), of the process clock (CLOCK_PROCESS_CPUTIME_ID, softirq
excluded on this kernel) and of wall; medians per call of voluntary / involuntary context switches,
write-family syscalls (/proc/self/io) and server CPU (task-clock of the server's threads)."""
import collections, glob, json, os, re, statistics as st, sys
out = sys.argv[1]
WORKS = ["d16k1", "d16k8", "d4k1", "c54k1", "c54k8"]
CELL = {"Cf-retain": "Cf", "Cf-cb-retain": "Cf-cb"}
def q(v, p):
    v = sorted(v); return v[min(len(v) - 1, max(0, round(p * (len(v) - 1))))]
def mpp(v, s=1e6):
    return f"{st.median(v)/s:.2f} [{q(v,.1)/s:.2f}-{q(v,.9)/s:.2f}]"
print("# Response-delivery comparison (gen/delivery.sh): raw figures\n")
for sess in ["a", "cf"]:
    d = os.path.join(out, sess)
    if not os.path.isdir(d):
        continue
    rows = collections.defaultdict(list); conds = []
    for f in sorted(glob.glob(os.path.join(d, "*.jsonl"))):
        m = re.match(r"(.+)-(%s)-(\d+)\.jsonl$" % "|".join(WORKS), os.path.basename(f))
        if not m:
            continue
        cond, work, rep = m.groups()
        if cond not in conds:
            conds.append(cond)
        for line in open(f):
            if line.startswith("{"):
                o = json.loads(line)
                if "cpu_ns" in o:
                    rows[(work, CELL.get(o["cell"], o["cell"]), cond)].append(o)
    print(f"## session {sess}\n\n```\n" + open(os.path.join(d, "header.txt")).read() + "```\n")
    if os.path.exists(os.path.join(out, "NOTES.txt")) and sess == "a":
        print("## Notes\n\n" + open(os.path.join(out, "NOTES.txt")).read())
    order = ["A", "A-blk", "A-cb", "A-q", "Cf", "Cf-cb", "Cf-q"]
    for work in WORKS:
        cells = sorted({k[1] for k in rows if k[0] == work}, key=lambda c: order.index(c) if c in order else 9)
        if not cells:
            continue
        print(f"\n### {sess}: {work}\n")
        print("| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | vcs / ics | writes | server CPU ms | n |")
        print("|---|---|---|---|---|---|---|---|---|")
        for cell in cells:
            for cond in conds:
                r = rows.get((work, cell, cond))
                if not r:
                    continue
                col = lambda k: [o[k] for o in r]
                print(f"| {cell} | {cond} | {mpp(col('task_clock_ns'))} | {mpp(col('cpu_ns'))} | {mpp(col('wall_ns'))} | "
                      f"{st.median(col('ru_nvcsw')):.1f} / {st.median(col('ru_nivcsw')):.1f} | {st.median(col('io_syscw')):.0f} | "
                      f"{st.median(col('server_task_clock_ns'))/1e6:.2f} | {len(r)} |")
