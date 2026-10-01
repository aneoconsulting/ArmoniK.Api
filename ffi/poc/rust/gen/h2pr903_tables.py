#!/usr/bin/env python3
"""Tables of gen/h2_pr903_bench.sh: python3 gen/h2pr903_tables.py OUT_DIR > OUT_DIR/h2pr903.md

One table per (workload, transport); rows: cell x condition. Values per call over every timed
round of the 3 processes: median [p10-p90]. CPU = the probe's inherited task-clock (softirq work
in the process's context included); process clock = CLOCK_PROCESS_CPUTIME_ID (excludes it on this
kernel). Gap to A = per process, median(cell) - median(A) of task-clock, one value per process,
then (range). Writes / reads = write-family / read-family syscalls (/proc/self/io). Server CPU =
task-clock of the server's threads (per-thread counters opened after the warm-up)."""
import collections, glob, json, os, re, statistics as st, sys

out = sys.argv[1]
CONDS = ["stock", "p4", "prc", "prh", "pp"]
NAMES = {"stock": "stock h2", "p4": "p4 N=16", "prc": "PR 903 core-only", "prh": "PR 903 host-too", "pp": "PR 903 + p4 N=16"}
CELL = {"Cf-retain": "Cf", "Cf-cb-retain": "Cf-cb"}
rows = collections.defaultdict(list)       # (work, tr, cond, cell) -> [round dicts]
proc = collections.defaultdict(list)       # (work, tr, cond, rep, cell) -> [task-clock]
for f in glob.glob(os.path.join(out, "*.jsonl")):
    m = re.match(r"(\w+)-(uds|tcp)-(\w+)-(\d+)\.jsonl$", os.path.basename(f))
    if not m:
        continue
    cond, tr, work, rep = m.groups()
    for line in open(f):
        if not line.startswith("{"):
            continue
        o = json.loads(line)
        if "cpu_ns" not in o:
            continue
        cell = CELL.get(o["cell"], o["cell"])
        rows[(work, tr, cond, cell)].append(o)
        proc[(work, tr, cond, rep, cell)].append(o["task_clock_ns"])

def q(v, p):
    v = sorted(v)
    return v[min(len(v) - 1, max(0, round(p * (len(v) - 1))))]

def mpp(v, s=1e6, d=2):
    return f"{st.median(v)/s:.{d}f} [{q(v,.1)/s:.{d}f}-{q(v,.9)/s:.{d}f}]"

works = [w for w in ["d16k1", "d16k8", "d4k1", "c54k1", "c54k8"] if any(k[0] == w for k in rows)]
print("# h2 PR 903: timed in-process tables\n")
print(open(os.path.join(out, "header.txt")).read().replace("\n", "  \n"))
if os.path.exists(os.path.join(out, "NOTES.txt")):
    print("\n## Notes\n")
    print(open(os.path.join(out, "NOTES.txt")).read())
for w in works:
    for tr in ["uds", "tcp"]:
        print(f"\n## {w} {tr}\n")
        print("| cell | condition | CPU ms (task-clock) | process clock ms | wall ms | gap to A per process, task-clock ms (range) | writes / call | reads / call | vcs / ics | server CPU ms (task-clock) | n |")
        print("|---|---|---|---|---|---|---|---|---|---|---|")
        cells = sorted({k[3] for k in rows if k[0] == w and k[1] == tr}, key=lambda c: ["A", "Cf", "Cf-cb", "Cf-zc"].index(c) if c in ["A", "Cf", "Cf-cb", "Cf-zc"] else 9)
        for cell in cells:
            for cond in CONDS:
                r = rows.get((w, tr, cond, cell))
                if not r:
                    continue
                gaps = ""
                if cell != "A":
                    g = []
                    for rep in sorted({k[3] for k in proc if k[:3] == (w, tr, cond)}):
                        a, c = proc.get((w, tr, cond, rep, "A")), proc.get((w, tr, cond, rep, cell))
                        if a and c:
                            g.append((st.median(c) - st.median(a)) / 1e6)
                    if g:
                        gaps = " ".join(f"{x:+.2f}" for x in g) + f" ({max(g)-min(g):.2f})"
                col = lambda k: [o[k] for o in r]
                print(f"| {cell} | {NAMES[cond]} | {mpp(col('task_clock_ns'))} | {mpp(col('cpu_ns'))} | {mpp(col('wall_ns'))} | {gaps} | "
                      f"{st.median(col('io_syscw')):.1f} | {st.median(col('io_syscr')):.1f} | {st.median(col('ru_nvcsw')):.1f} / {st.median(col('ru_nivcsw')):.1f} | "
                      f"{mpp(col('server_task_clock_ns'))} | {len(r)} |")
