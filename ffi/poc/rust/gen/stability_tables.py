#!/usr/bin/env python3
"""Tables of a stability campaign (gen/stability_campaign.sh, i.e. gen/inproc.sh with many repetitions):
per condition, workload and cell:
  1. absolute client CPU and wall per call, pooled over every process: median [p10-p90];
  2. the per-process gap to A (cell median - A median in the same process): median, p10-p90,
     min..max over the processes; A2 - A is the same-code floor;
  3. drift: the per-process medians in run order, and the median of the first and of the last
     third of the processes;
  4. voluntary context switches and minor faults per call (medians).
Absolutes and differences only.   gen/stability_tables.py DIR"""
import glob, json, os, re, statistics as st, sys
from collections import defaultdict

d = sys.argv[1]
hdr = open(os.path.join(d, "header.txt")).read().rstrip()
conds = re.findall(r"# condition (\S+):", hdr)
ORDER = ["A", "A2", "Cf", "Cf-cb", "Cf-zc", "Cf-zcw", "Cf-zcp", "Cf-encp"]
WORD = ["d16k1", "d16k8", "d4k1", "d4k8", "c54k1", "c54k8"]
short = lambda c: re.sub(r"-(retain|drop)$", "", c)
ck = lambda c: (ORDER.index(c) if c in ORDER else 50, c)
q = lambda xs, f: sorted(xs)[min(len(xs) - 1, int(f * len(xs)))]
ms = lambda x: f"{x / 1e6:.2f}"
mpp = lambda xs: f"{ms(st.median(xs))} [{ms(q(xs, .1))}-{ms(q(xs, .9))}]" if xs else "--"

cpu = defaultdict(list); wall = defaultdict(list); ru = defaultdict(lambda: defaultdict(list))
med = defaultdict(dict)   # (cond, w, cell) -> {rep: median}
for p in glob.glob(os.path.join(d, "*.jsonl")):
    m = re.match(r"(.+)-(\w+)-(\d+)\.jsonl$", os.path.basename(p))
    if not m or m.group(1) not in conds:
        continue
    cond, w, rep = m.group(1), m.group(2), int(m.group(3))
    per = defaultdict(list)
    for l in open(p):
        if l.startswith("#"):
            continue
        r = json.loads(l); c = short(r["cell"])
        cpu[(cond, w, c)].extend(r["cpu_calls"]); per[c].extend(r["cpu_calls"])
        wall[(cond, w, c)].append(r["wall_ns"])
        for x in ("ru_nvcsw", "ru_minflt"):
            ru[(cond, w, c)][x].append(r[x])
    for c, xs in per.items():
        med[(cond, w, c)][rep] = st.median(xs)

print(hdr)
ws = [w for w in WORD if any(k[1] == w for k in cpu)]
print("\n## 1. Absolute client CPU and wall per call, pooled over every process: median [p10-p90]\n")
for w in ws:
    cells = sorted({k[2] for k in cpu if k[1] == w}, key=ck)
    print(f"\n### {w}\n")
    print("| cell | " + " | ".join(f"{c} CPU ms | {c} wall ms | {c} processes" for c in conds) + " |")
    print("|---|" + "---|---|---|" * len(conds))
    for cell in cells:
        row = []
        for c in conds:
            k = (c, w, cell)
            row += [mpp(cpu.get(k, [])), mpp(wall.get(k, [])), str(len(med.get(k, {})))] if k in cpu else ["--"] * 3
        print(f"| {cell} | " + " | ".join(row) + " |")

print("\n## 2. Per-process gap to A (cell median - A median of the same process), ms: median, p10-p90, min..max over the processes\n")
for w in ws:
    cells = [c for c in sorted({k[2] for k in cpu if k[1] == w}, key=ck) if c != "A"]
    print(f"\n### {w}\n")
    print("| cell | " + " | ".join(f"{c}: median | {c}: p10-p90 | {c}: min..max" for c in conds) + " |")
    print("|---|" + "---|---|---|" * len(conds))
    for cell in cells:
        row = []
        for c in conds:
            a = med.get((c, w, "A"), {}); b = med.get((c, w, cell), {})
            g = [b[r] - a[r] for r in b if r in a]
            row += [f"{st.median(g) / 1e6:+.2f}", f"{q(g, .1) / 1e6:+.2f}..{q(g, .9) / 1e6:+.2f}", f"{min(g) / 1e6:+.2f}..{max(g) / 1e6:+.2f}"] if g else ["--"] * 3
        print(f"| {cell} | " + " | ".join(row) + " |")

print("\n## 3. Drift: per-process medians in run order (ms), and the medians of the first and last third of the processes\n")
for c in conds:
    for w in ws:
        for cell in sorted({k[2] for k in cpu if k[0] == c and k[1] == w}, key=ck):
            m = med[(c, w, cell)]
            xs = [m[r] for r in sorted(m)]
            t = max(1, len(xs) // 3)
            print(f"- {c} {w} {cell}: first third {ms(st.median(xs[:t]))}, last third {ms(st.median(xs[-t:]))}; run order: " + " ".join(ms(x) for x in xs))

print("\n## 4. Voluntary context switches / minor faults per call (medians over rounds)\n")
for w in ws:
    cells = sorted({k[2] for k in cpu if k[1] == w}, key=ck)
    print(f"\n### {w}\n")
    print("| cell | " + " | ".join(conds) + " |")
    print("|---|" + "---|" * len(conds))
    for cell in cells:
        print(f"| {cell} | " + " | ".join((f"{st.median(ru[(c, w, cell)]['ru_nvcsw']):.0f} / {st.median(ru[(c, w, cell)]['ru_minflt']):.1f}" if (c, w, cell) in cpu else "--") for c in conds) + " |")
