#!/usr/bin/env python3
"""Tables of a gen/attrib.sh directory: per workload and cell, per repetition, perf stat's counters
per call (timed rounds only), the probe's client CPU per call median [p10-p90] and wall, getrusage
switches and faults per call; then the /proc per-thread-class split (CPU, write and read syscalls,
bytes written per call) and strace's syscall counts per call (warm-up and setup included).
   gen/attrib_tables.py DIR [DIR...]   (several directories: one block per directory)"""
import glob, json, os, re, statistics as st, sys
from collections import defaultdict

ORDER = ["A", "Df", "Cf", "Ff", "Cf-cb", "Cf-cb-1rt", "Cn-1rt", "Df-chan"]


def cellkey(c):
    c = re.sub(r"-(retain|drop)$", "", c)
    return (ORDER.index(c) if c in ORDER else 50, c)


def q(xs, f):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(f * len(xs)))]


def mpp(xs, s=1e6):
    return f"{st.median(xs) / s:.2f} [{q(xs, .1) / s:.2f}-{q(xs, .9) / s:.2f}]" if xs else "--"


def probe_rows(p):
    rows = [json.loads(l) for l in open(p) if not l.startswith("#")]
    return rows


for d in sys.argv[1:]:
    print(open(os.path.join(d, "header.txt")).read().rstrip())
    stat = defaultdict(lambda: defaultdict(list))   # (w, cell) -> ev -> per rep per-call
    cpu = defaultdict(list); wall = defaultdict(list); ru = defaultdict(lambda: defaultdict(list))
    for p in glob.glob(os.path.join(d, "stat-*.perf")):
        m = re.match(r"stat-(\w+)-(.+)-(\d+)\.perf$", os.path.basename(p))
        w, cell, rep = m.group(1), m.group(2), int(m.group(3))
        rows = [r for r in probe_rows(p[:-5] + ".jsonl") if re.sub(r"-(retain|drop)$", "", r["cell"]) == cell]
        calls = sum(r["calls"] for r in rows)
        for l in open(p):
            f = l.strip().split(",")
            if len(f) < 3 or l.startswith("#") or not f[0].replace(".", "").isdigit():
                continue
            stat[(w, cell)][f[2]].append((rep, float(f[0]) / calls))
        for r in rows:
            cpu[(w, cell)].extend(r["cpu_calls"])
            wall[(w, cell)].append(r["wall_ns"])
            for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt"):
                ru[(w, cell)][x].append(r[x])
    ws = sorted({k[0] for k in stat}, key=lambda w: ["d16k1", "d16k8", "c54k1", "d4k1"].index(w) if w in ["d16k1", "d16k8", "c54k1", "d4k1"] else 9)
    for w in ws:
        cells = sorted({k[1] for k in stat if k[0] == w}, key=cellkey)
        print(f"\n## {w}: perf stat per call, timed rounds only (one value per repetition, in repetition order)\n")
        print("| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |")
        print("|---|---|---|---|---|---|---|---|---|---|---|")
        for c in cells:
            s = stat[(w, c)]
            g = lambda ev, sc=1.0, fmt="{:.1f}": " ".join(fmt.format(v * sc) for _, v in sorted(s.get(ev, [])))
            ipc = " ".join(f"{a / b:.2f}" for (_, a), (_, b) in zip(sorted(s["instructions"]), sorted(s["cycles"])))
            rr = ru[(w, c)]
            print(f"| {c} | {mpp(cpu[(w, c)])} | {mpp(wall[(w, c)])} | {g('cycles', 1e-6)} | {g('instructions', 1e-6)} | {ipc} | {g('cache-misses', 1e-3)} | {g('page-faults', 1, '{:.0f}')} | {g('context-switches', 1, '{:.0f}')} | {g('task-clock', 1, '{:.2f}')} | "
                  + " / ".join(f"{st.median(rr[x]):.1f}" for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt")) + " |")
        # /proc split
        print(f"\n### {w}: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written\n")
        print("| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |")
        print("|---|---|---|---|---|---|---|")
        for c in cells:
            p = os.path.join(d, f"proc-{w}-{c}.jsonl")
            if not os.path.exists(p):
                continue
            rows = probe_rows(p)
            classes = sorted({k[4:] for r in rows for k in r if k.startswith("cpu_") and k != "cpu_ns" and k != "cpu_calls"})
            for cl in classes:
                med = lambda k: st.median([r.get(f"{k}_{cl}", 0) for r in rows])
                print(f"| {c} | {cl} | {med('cpu') / 1e6:.2f} | {med('syscw'):.0f} | {med('syscr'):.0f} | {med('wchar') / 1e6:.2f} | {med('minflt'):.0f} |")
        # strace
        print(f"\n### {w}: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)\n")
        tops = defaultdict(dict)
        for c in cells:
            p = os.path.join(d, f"strace-{w}-{c}.txt")
            if not os.path.exists(p):
                continue
            rows = probe_rows(os.path.join(d, f"strace-{w}-{c}.jsonl"))
            hdr = open(os.path.join(d, f"strace-{w}-{c}.jsonl")).readline()
            n = sum(r["calls"] for r in rows)
            warm = int(re.search(r"warm (\d+)", hdr).group(1)) * (rows[0].get("k", 1) if rows else 1)
            for l in open(p):
                f = l.split()
                if len(f) >= 5 and f[0][0].isdigit() and f[-1].isalpha() and f[-1] != "total":
                    tops[c][f[-1]] = int(f[3]) / (n + warm)
        sysc = sorted({s for v in tops.values() for s in v}, key=lambda s: -max(v.get(s, 0) for v in tops.values()))[:8]
        print("| cell | " + " | ".join(sysc) + " |")
        print("|---|" + "---|" * len(sysc))
        for c in cells:
            print(f"| {c} | " + " | ".join(f"{tops[c].get(s, 0):.0f}" for s in sysc) + " |")
