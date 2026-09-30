#!/usr/bin/env python3
"""The consolidated table of gen/inproc.sh sessions (physical-probe/opt-stack): per workload, per
cell and condition, client CPU per call median [p10-p90], wall median [p10-p90], the in-process
gap to A per process (min..max), and voluntary switches / minor faults per call. Absolutes and
differences only.   gen/optstack_tables.py DIR [DIR...]"""
import glob, json, os, re, statistics as st, sys
from collections import defaultdict
ORDER = ["A", "Df", "Df-1f", "Cn-1rt", "C", "Cf", "Cf-cb", "Cf-encp", "Cf-zc", "Cf-zcp", "Cf-zcw"]
WORD = ["d16k1", "d16k8", "d4k1", "d4k8", "c54k1", "c54k8"]
NAME = {"d16k1": "d/16 MiB, k = 1", "d16k8": "d/16 MiB, k = 8", "d4k1": "d/4 MiB, k = 1", "d4k8": "d/4 MiB, k = 8", "c54k1": "c/P5.4, k = 1", "c54k8": "c/P5.4, k = 8"}
short = lambda c: re.sub(r"-(retain|drop)$", "", c)
q = lambda xs, f: sorted(xs)[min(len(xs) - 1, int(f * len(xs)))]
mpp = lambda xs: f"{st.median(xs) / 1e6:.2f} [{q(xs, .1) / 1e6:.2f}-{q(xs, .9) / 1e6:.2f}]" if xs else "--"
for d in sys.argv[1:]:
    hdr = open(os.path.join(d, "header.txt")).read().rstrip()
    conds = re.findall(r"# condition (\S+):", hdr)
    cpu = defaultdict(list); wall = defaultdict(list); ru = defaultdict(lambda: defaultdict(list)); med = defaultdict(dict)
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
            cpu[(w, cond, c)].extend(r["cpu_calls"]); per[c].extend(r["cpu_calls"])
            wall[(w, cond, c)].append(r["wall_ns"])
            for x in ("ru_nvcsw", "ru_minflt"):
                ru[(w, cond, c)][x].append(r[x])
        for c, xs in per.items():
            med[(w, cond, c)][rep] = st.median(xs)
    print(f"# {os.path.basename(d)}\n")
    print("\n".join(l for l in hdr.splitlines() if l.startswith("# condition") or "wall time" in l or l.startswith("# in-process") or l.startswith("# client")))
    for w in [x for x in WORD if any(k[0] == x for k in cpu)]:
        cells = sorted({k[2] for k in cpu if k[0] == w}, key=lambda c: ORDER.index(c) if c in ORDER else 99)
        print(f"\n## {NAME[w]}: client CPU ms per call median [p10-p90]; gap to A per process (min..max); wall ms median [p10-p90]\n")
        print("| cell | " + " | ".join(f"{c} CPU | {c} gap to A | {c} wall" for c in conds) + " |")
        print("|---|" + "---|---|---|" * len(conds))
        for cell in cells:
            row = []
            for c in conds:
                k = (w, c, cell)
                if k not in cpu:
                    row += ["--"] * 3; continue
                a = med.get((w, c, "A"), {})
                g = [med[k][r] - a[r] for r in med[k] if r in a]
                row += [mpp(cpu[k]), (f"{min(g) / 1e6:+.2f}..{max(g) / 1e6:+.2f}" if g and cell != "A" else ""), mpp(wall[k])]
            print(f"| {cell} | " + " | ".join(row) + " |")
        print(f"\nVoluntary context switches / minor faults per call (medians):\n")
        print("| cell | " + " | ".join(conds) + " |")
        print("|---|" + "---|" * len(conds))
        for cell in cells:
            print(f"| {cell} | " + " | ".join((f"{st.median(ru[(w, c, cell)]['ru_nvcsw']):.0f} / {st.median(ru[(w, c, cell)]['ru_minflt']):.1f}" if (w, c, cell) in cpu else "--") for c in conds) + " |")
    print()
