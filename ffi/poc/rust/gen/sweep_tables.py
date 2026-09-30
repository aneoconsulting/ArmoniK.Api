#!/usr/bin/env python3
"""Tables of gen/workers_sweep.sh: per workload and cell, one row per condition (core workers x
p9): client CPU per call median [p10-p90], wall per call median [p10-p90], throughput (calls per
second of batch wall: 1e9 / wall per call), the in-process gap to A (per-process min..max), and
voluntary context switches per call; then the attribution pass's CPU and switches per thread class.
   gen/sweep_tables.py SWEEP_DIR"""
import glob, json, os, re, statistics as st, sys
from collections import defaultdict

root = sys.argv[1]
q = lambda xs, f: sorted(xs)[min(len(xs) - 1, int(f * len(xs)))]
mpp = lambda xs: f"{st.median(xs) / 1e6:.2f} [{q(xs, .1) / 1e6:.2f}-{q(xs, .9) / 1e6:.2f}]"
short = lambda c: re.sub(r"-(retain|drop)$", "", c)
WORD = ["d16k1", "d16k8", "d16k16", "d16k32", "c54k1", "c54k8", "c54k16", "c54k32"]
ORDER = ["A", "Cf", "Cf-m4", "Cf-cb", "Cf-cb-m4", "Cf-zc", "Cf-zc-m4"]
CONDS = [f"w{w}-p9{p}" for w in (1, 2, 4, 8) for p in (0, 1)]
cpu = defaultdict(list); wall = defaultdict(list); vcs = defaultdict(list); med = defaultdict(dict)
for sess in ("low", "high"):
    d = os.path.join(root, sess)
    for p in glob.glob(os.path.join(d, "*.jsonl")):
        m = re.match(r"(w\d-p9\d)-(\w+)-(\d+)\.jsonl$", os.path.basename(p))
        if not m:
            continue
        cond, w, rep = m.group(1), m.group(2), int(m.group(3))
        per = defaultdict(list)
        for l in open(p):
            if l.startswith("#"):
                continue
            r = json.loads(l); c = short(r["cell"])
            cpu[(w, c, cond)].extend(r["cpu_calls"]); per[c].extend(r["cpu_calls"])
            wall[(w, c, cond)].append(r["wall_ns"]); vcs[(w, c, cond)].append(r["ru_nvcsw"])
        for c, xs in per.items():
            med[(w, c, cond)][rep] = st.median(xs)
for sess in ("low", "high"):
    h = os.path.join(root, sess, "header.txt")
    if os.path.exists(h):
        print(f"<!-- {sess} session header -->")
        print("\n".join(l for l in open(h).read().splitlines() if not l.startswith("# condition")))
print("\nConditions: wN-p9P = AK_CORE_WORKERS=N, AK_CB_AT_TAKE=P (patch p9), host runtimes 8 workers, stack binary, pinned allocator, 3 processes each. Cells -m4: 4 core clients (4 connections, each its own core runtime of N workers), call i on client i % 4.")
for w in WORD:
    cells = sorted({k[1] for k in cpu if k[0] == w}, key=lambda c: ORDER.index(c) if c in ORDER else 99)
    if not cells:
        continue
    print(f"\n## {w}\n")
    print("| cell | condition | CPU ms/call | wall ms/call | calls/s | gap to A (min..max) | vcs/call |")
    print("|---|---|---|---|---:|---|---:|")
    for c in cells:
        for cond in CONDS:
            k = (w, c, cond)
            if k not in cpu:
                continue
            a = med.get((w, "A", cond), {})
            g = [med[k][r] - a[r] for r in med[k] if r in a]
            gs = f"{min(g) / 1e6:+.2f}..{max(g) / 1e6:+.2f}" if g and c != "A" else ""
            print(f"| {c} | {cond} | {mpp(cpu[k])} | {mpp(wall[k])} | {1e9 / st.median(wall[k]):.0f} | {gs} | {st.median(vcs[k]):.0f} |")
print("\n## Attribution pass (untimed, /proc per thread class, medians over 3 rounds): CPU ms per call and voluntary switches per call, by class\n")
print("| workload | condition | cell | CPU by class | vcs by class | threads by class |")
print("|---|---|---|---|---|---|")
for p in sorted(glob.glob(os.path.join(root, "attr", "*.jsonl"))):
    m = re.match(r"(w\d-p9\d)-(\S+)-k(\d+)\.jsonl$", os.path.basename(p))
    cond, size, k = m.group(1), m.group(2), m.group(3)
    thr = ""
    rows = defaultdict(list)
    for l in open(p):
        if l.startswith("# threads by class"):
            thr = l.split(":", 1)[1].strip()
        if l.startswith("#"):
            continue
        r = json.loads(l); rows[short(r["cell"])].append(r)
    for c in sorted(rows, key=lambda c: ORDER.index(c) if c in ORDER else 99):
        rs = rows[c]
        cls = sorted({x[4:] for r in rs for x in r if x.startswith("cpu_") and x not in ("cpu_ns", "cpu_calls")})
        cpus = ", ".join(f"{cl} {st.median([r.get('cpu_' + cl, 0) for r in rs]) / 1e6:.2f}" for cl in cls)
        sw = ", ".join(f"{cl} {st.median([r.get('vcs_' + cl, 0) for r in rs]):.0f}" for cl in cls)
        print(f"| {size} k={k} | {cond} | {c} | {cpus} | {sw} | {thr} |")
