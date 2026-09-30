#!/usr/bin/env python3
"""Tables of a gen/inproc.sh directory: per workload, per condition and cell, client CPU per call
median [p10-p90] and wall per call median [p10-p90] pooled over repetitions; the in-process gap to
A (cell median - A median of the same process) per repetition and its range; getrusage voluntary
and involuntary context switches and minor faults per call (medians over rounds); the client's
threads by class after the warm-up. Absolutes and differences only.
   gen/inproc_tables.py DIR"""
import glob, json, os, re, statistics as st, sys
from collections import defaultdict

d = sys.argv[1]
print(open(os.path.join(d, "header.txt")).read().rstrip())
ORDER = ["A", "A2", "Df", "Cf", "Ff", "Cf-cb", "Cf-cb-1rt", "Cn-1rt", "Df-chan", "C", "C-cb"]
WORD = ["d16k1", "d16k8", "d16k16", "d16k32", "d4k1", "d4k8", "c54k1", "c54k8", "c54k16", "c54k32"]
short = lambda c: re.sub(r"-(retain|drop)$", "", c)
ck = lambda c: (ORDER.index(c) if c in ORDER else 50, c)
q = lambda xs, f: sorted(xs)[min(len(xs) - 1, int(f * len(xs)))]
mpp = lambda xs: f"{st.median(xs) / 1e6:.2f} [{q(xs, .1) / 1e6:.2f}-{q(xs, .9) / 1e6:.2f}]" if xs else "--"
cpu = defaultdict(list); wall = defaultdict(list); ru = defaultdict(lambda: defaultdict(list))
med = defaultdict(dict); threads = defaultdict(set)
conds = []
for l in open(os.path.join(d, "header.txt")):
    m = re.match(r"# condition (\S+):", l)
    if m:
        conds.append(m.group(1))
for p in sorted(glob.glob(os.path.join(d, "*.jsonl"))):
    m = re.match(r"(.+)-(\w+)-(\d+)\.jsonl$", os.path.basename(p))
    if not m or m.group(1) not in conds:
        continue
    cond, w, rep = m.group(1), m.group(2), int(m.group(3))
    per = defaultdict(list)
    for l in open(p):
        if l.startswith("# threads by class"):
            threads[(cond, w)].add(l[2:].strip())
        if l.startswith("#"):
            continue
        r = json.loads(l)
        c = short(r["cell"])
        cpu[(w, cond, c)].extend(r["cpu_calls"])
        per[c].extend(r["cpu_calls"])
        wall[(w, cond, c)].append(r["wall_ns"])
        for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt"):
            ru[(w, cond, c)][x].append(r[x])
    for c, xs in per.items():
        med[(w, cond, c)][rep] = st.median(xs)
ws = sorted({k[0] for k in cpu}, key=lambda w: WORD.index(w) if w in WORD else 9)
for w in ws:
    cells = sorted({k[2] for k in cpu if k[0] == w}, key=ck)
    cs = [c for c in conds if any(k[0] == w and k[1] == c for k in cpu)]
    print(f"\n## {w}\n")
    print("| cell | " + " | ".join(f"{c}: CPU ms | {c}: wall ms | {c}: gap to A per rep (range) | {c}: vcs / ics / minflt" for c in cs) + " |")
    print("|---|" + "---|---|---|---|" * len(cs))
    for cell in cells:
        row = []
        for c in cs:
            k = (w, c, cell)
            if k not in cpu:
                row += ["--"] * 4
                continue
            a = med.get((w, c, "A"), {})
            g = [med[k][r] - a[r] for r in sorted(med[k]) if r in a]
            gs = " ".join(f"{x / 1e6:+.2f}" for x in g) + (f" ({(max(g) - min(g)) / 1e6:.2f})" if g else "")
            rr = ru[k]
            row += [mpp(cpu[k]), mpp(wall[k]), gs if cell != "A" else "", " / ".join(f"{st.median(rr[x]):.1f}" for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt"))]
        print(f"| {cell} | " + " | ".join(row) + " |")
    for c in cs:
        for t in sorted(threads[(w, c)]):
            print(f"\n- {c}: {t}")
