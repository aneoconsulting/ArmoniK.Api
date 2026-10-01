#!/usr/bin/env python3
"""Tables of gen/tcp_sweep.sh: python3 gen/sweep_tcp_tables.py OUT_DIR > OUT_DIR/tables.md

Per (workload, cell): one row per condition (h2 variant x core workers). Values per call over every
timed round of every process: median [p10-p90] of client CPU (task-clock: the scheduler's run time
of every client thread, softirq work in their context included) and of the process clock beside it
(CLOCK_PROCESS_CPUTIME_ID, which excludes softirq on this kernel), of wall; throughput = calls per
second from the median wall per call (k calls in flight: wall per call = batch wall / k) and the
payload's MB/s; voluntary / involuntary context switches, write-family syscalls (/proc/self/io)
and server CPU (task-clock of the server's threads) per call, medians."""
import collections, glob, json, os, re, statistics as st, sys

out = sys.argv[1]
SIZE_MB = {"16MiB": 16 * 1048576 / 1e6, "P5.4": None}
CELL = {"Cf-retain": "Cf", "Cf-cb-retain": "Cf-cb", "Cf-retain-m4": "Cf-m4", "Cf-cb-retain-m4": "Cf-cb-m4"}
rows = collections.defaultdict(list)
for sess in ["low", "high"]:
    for f in glob.glob(os.path.join(out, sess, "*.jsonl")):
        m = re.match(r"(stock|h2-batch)-w(\d+)-(\w+)-(\d+)\.jsonl$", os.path.basename(f))
        if not m:
            continue
        v, cw, work, rep = m.groups()
        for line in open(f):
            if line.startswith("{"):
                o = json.loads(line)
                if "cpu_ns" in o:
                    rows[(work, CELL.get(o["cell"], o["cell"]), v, int(cw))].append(o)

def q(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, max(0, round(p * (len(v) - 1))))]

def mpp(v, s=1e6):
    return f"{st.median(v)/s:.2f} [{q(v,.1)/s:.2f}-{q(v,.9)/s:.2f}]"

print("# TCP core worker sweep (gen/tcp_sweep.sh)\n")
for sess in ["low", "high"]:
    h = os.path.join(out, sess, "header.txt")
    if os.path.exists(h):
        print(f"## header ({sess})\n")
        print("```\n" + open(h).read() + "```\n")
print("CPU = client task-clock; P5.4 throughput in calls/s only.\n")
order = ["A", "Cf", "Cf-cb", "Cf-m4", "Cf-cb-m4"]
for work in ["d16k1", "d16k8", "d16k16", "d16k32", "c54k1", "c54k8", "c54k16", "c54k32"]:
    cells = sorted({k[1] for k in rows if k[0] == work}, key=lambda c: order.index(c) if c in order else 9)
    if not cells:
        continue
    print(f"\n## {work}\n")
    print("| cell | h2 | core workers | CPU ms (task-clock) | process clock ms | wall ms | calls/s | MB/s | vcs / ics | writes | server CPU ms | n |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for cell in cells:
        for v in ["stock", "h2-batch"]:
            for cw in [1, 2, 4, 8]:
                r = rows.get((work, cell, v, cw))
                if not r:
                    continue
                col = lambda k: [o[k] for o in r]
                wall = st.median(col("wall_ns"))
                cps = 1e9 / wall
                mb = SIZE_MB.get(r[0]["size"]) if r[0]["size"] in SIZE_MB else None
                print(f"| {cell} | {v} | {cw} | {mpp(col('task_clock_ns'))} | {mpp(col('cpu_ns'))} | {mpp(col('wall_ns'))} | {cps:.0f} | "
                      f"{'' if mb is None else f'{cps*mb:.0f}'} | {st.median(col('ru_nvcsw')):.1f} / {st.median(col('ru_nivcsw')):.1f} | "
                      f"{st.median(col('io_syscw')):.0f} | {st.median(col('server_task_clock_ns'))/1e6:.2f} | {len(r)} |")
