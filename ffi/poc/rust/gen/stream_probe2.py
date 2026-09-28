#!/usr/bin/env python3
"""Summary of gen/stream_probe2.sh: per (size, cell), per-call client CPU, pooled over the
iterations: the grid's (criterion samples: cpu_ns / iters) and each probe variant's (per-call
values of every round, and the per-round means), plus for probe variants with /proc reads the
per-call write syscalls, bytes per write, read syscalls by thread class. CONTAINER INSTRUMENTATION."""
import glob, json, os, re, statistics, sys
from collections import defaultdict
out = sys.argv[1]
print(open(os.path.join(out, "header.txt")).read().rstrip())
data = defaultdict(lambda: defaultdict(list))     # (size, cell) -> source -> [per-call ns]
firsts = defaultdict(lambda: defaultdict(list))   # first call of a round
io = defaultdict(lambda: defaultdict(lambda: defaultdict(list)))
for p in sorted(glob.glob(os.path.join(out, "*-[0-9]*.jsonl"))):
    src = re.sub(r"-\d+\.jsonl$", "", os.path.basename(p))
    for l in open(p):
        if l.startswith("#"):
            continue
        r = json.loads(l)
        if src == "grid":
            k = "" if r["inflight"] == 1 else f" k{r['inflight']}"
            key = r["payload"] if r["dir"] == "d" else f"{r['dir']}/{r['payload']}"
            data[(key + k, r["cell"])]["grid"].append(r["cpu_ns"] / r["iters"])
            continue
        k = (r["size"], r["cell"])
        cc = r["cpu_calls"]
        data[k][src].extend(cc)
        firsts[k][src].append(cc[0])
        data[k][src + "/later"].extend(cc[1:])
        for c in ("caller", "cell-rt", "core-rt", "main", "other"):
            for m in ("syscw", "wchar", "syscr", "rchar", "cpu"):
                if f"{m}_{c}" in r:
                    io[k][src][f"{m}_{c}"].append(r[f"{m}_{c}"])
srcs = sorted({s for v in data.values() for s in v}, key=lambda s: (s != "grid", s))
print("\nper-call client CPU, ms: median [p10-p90] (n)")
for k in sorted(data, key=lambda k: (k[0] != "16MiB", k[1])):
    print(f"{k[0]} {k[1]}")
    for s in srcs:
        xs = sorted(data[k].get(s, []))
        if not xs:
            continue
        n = len(xs)
        print(f"    {s:12} {xs[n // 2] / 1e6:7.2f} [{xs[n // 10] / 1e6:.2f}-{xs[9 * n // 10] / 1e6:.2f}] ({n})")
    for s in firsts[k]:
        xs = sorted(firsts[k][s]); n = len(xs)
        print(f"    {s + '/first':12} {xs[n // 2] / 1e6:7.2f} [{xs[n // 10] / 1e6:.2f}-{xs[9 * n // 10] / 1e6:.2f}] ({n})")
print("\nper call, by thread class (medians over rounds): write syscalls, bytes per write, read syscalls, bytes per read, CPU ms")
for k in sorted(io, key=lambda k: (k[0] != "16MiB", k[1])):
    for s, d in io[k].items():
        parts = []
        for c in ("caller", "cell-rt", "core-rt"):
            if f"syscw_{c}" not in d:
                continue
            w, wb = statistics.median(d[f"syscw_{c}"]), statistics.median(d[f"wchar_{c}"])
            rd, rb = statistics.median(d[f"syscr_{c}"]), statistics.median(d[f"rchar_{c}"])
            cpu = statistics.median(d[f"cpu_{c}"]) / 1e6
            if w == 0 and rd == 0 and cpu < 0.05:
                continue
            parts.append(f"{c}: {w:.0f} writes x {wb / w / 1024 if w else 0:.0f} KiB, {rd:.0f} reads x {rb / rd if rd else 0:.0f} B, cpu {cpu:.2f}")
        print(f"{k[0]} {k[1]:12} [{s}] " + " | ".join(parts))
