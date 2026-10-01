#!/usr/bin/env python3
"""Compact headline of gen/h2_pr903_bench.sh sessions: python3 gen/h2pr903_headline.py OUT_DIR...
Per (transport, workload, cell): per condition, client task-clock ms per call median, wall ms
median, writes per call median, and the per-process gaps to A (task-clock, ms)."""
import collections, glob, json, os, re, statistics as st, sys
NAMES = {"stock": "stock", "p4": "p4 N=16", "prc": "PR core", "prh": "PR host-too", "pp": "PR+p4 N=16"}
CELL = {"Cf-retain": "Cf", "Cf-cb-retain": "Cf-cb"}
for out in sys.argv[1:]:
    rows = collections.defaultdict(list); proc = collections.defaultdict(list)
    for f in glob.glob(os.path.join(out, "*.jsonl")):
        m = re.match(r"(\w+)-(uds|tcp)-(\w+)-(\d+)\.jsonl$", os.path.basename(f))
        if not m:
            continue
        cond, tr, work, rep = m.groups()
        for line in open(f):
            if line.startswith("{"):
                o = json.loads(line)
                if "cpu_ns" in o:
                    c = CELL.get(o["cell"], o["cell"])
                    rows[(tr, work, c, cond)].append(o); proc[(tr, work, cond, rep, c)].append(o["task_clock_ns"])
    conds = [c for c in NAMES if any(k[3] == c for k in rows)]
    print(f"\n### {out}\n")
    print("task-clock ms per call median / wall ms median / writes per call; gap to A per process (task-clock ms)\n")
    print("| transport | workload | cell | " + " | ".join(NAMES[c] for c in conds) + " |")
    print("|---|---|---|" + "---|" * len(conds))
    for tr in ["uds", "tcp"]:
        for w in ["d16k1", "d16k8", "d4k1", "c54k1", "c54k8"]:
            for cell in ["A", "Cf", "Cf-cb", "Cf-zc"]:
                if (tr, w, cell, conds[0]) not in rows:
                    continue
                cols = []
                for c in conds:
                    r = rows[(tr, w, cell, c)]
                    s = f"{st.median([o['task_clock_ns'] for o in r])/1e6:.2f} / {st.median([o['wall_ns'] for o in r])/1e6:.2f} / {st.median([o['io_syscw'] for o in r]):.0f}"
                    if cell != "A":
                        g = [(st.median(proc[(tr, w, c, rep, cell)]) - st.median(proc[(tr, w, c, rep, 'A')])) / 1e6
                             for rep in sorted({k[3] for k in proc if k[:3] == (tr, w, c)}) if proc.get((tr, w, c, rep, cell))]
                        s += " (" + " ".join(f"{x:+.2f}" for x in g) + ")"
                    cols.append(s)
                print(f"| {tr} | {w} | {cell} | " + " | ".join(cols) + " |")
