#!/usr/bin/env python3
"""Tables of a gen/runtime_probe.sh session (CONTAINER INSTRUMENTATION): per variant and cell,
direction d, k = 1: the client CPU per call (median [p10-p90] over every call of every
iteration, and the range of the per-iteration medians), the difference to the `base` variant
per iteration, the split cells' per-chunk host work, getrusage context switches and minor
faults per call, and the attribution pass (/proc per thread class, allocation shim).
   gen/runtime_tables.py SESSION_DIR"""
import glob, json, os, re, statistics, sys
from collections import defaultdict

d = sys.argv[1]
hdr = open(os.path.join(d, "header.txt")).read().strip()
vorder = [v.split("=")[0] for v in re.search(r"variants: (.*)", hdr).group(1).split()]
cpu = defaultdict(list)                     # (v, size, cell) -> per-call ns, pooled
it_med = defaultdict(dict)                  # (v, size, cell) -> {iteration: median ns}
ru = defaultdict(lambda: defaultdict(list))  # (v, size, cell) -> metric -> per-round per-call values
sp = defaultdict(lambda: defaultdict(list))
rt_line = {}
for p in glob.glob(os.path.join(d, "*-[0-9]*.jsonl")):
    m = re.match(r"(.+)-(\d+)\.jsonl$", os.path.basename(p))
    v, it = m.group(1), int(m.group(2))
    per = defaultdict(list)
    for l in open(p):
        if l.startswith("# runtimes"):
            rt_line[v] = l[2:].strip()
        if l.startswith("#"):
            continue
        r = json.loads(l)
        k = (v, r["size"], r["cell"])
        cpu[k].extend(r["cpu_calls"])
        per[k].extend(r["cpu_calls"])
        for x in ("ru_nvcsw", "ru_nivcsw", "ru_minflt"):
            ru[k][x].append(r[x])
        if "host_encode_cpu" in r:
            n = 8 if r["size"] == "16MiB" else 2
            for x in ("host_encode_cpu", "host_send_cpu", "host_send_wall", "host_recv_wall"):
                sp[k][x].append(r[x] / (n if x != "host_recv_wall" else 1))
    for k, xs in per.items():
        it_med[k][it] = statistics.median(xs)

corder = ["A", "Df-retain", "Df-chan", "Cf-retain", "Cf-cb-retain", "Cf-split", "Cf-cb-split"]
cells = sorted({c for (_, _, c) in cpu}, key=lambda c: (corder.index(c) if c in corder else 50, c))
vs = [v for v in vorder if any(k[0] == v for k in cpu)]
q = lambda xs, f: sorted(xs)[min(len(xs) - 1, int(f * len(xs)))]

print(hdr)
print("\nRuntimes as each variant's probe process recorded them:\n")
for v in vs:
    print(f"- `{v}`: {rt_line.get(v, '?')}")
for size in ("16MiB", "4MiB"):
    print(f"\n## {size}, k = 1: client CPU per call, ms: median [p10-p90] over all calls; below it the range of the per-iteration medians\n")
    print("| cell | " + " | ".join(vs) + " |")
    print("|---|" + "---:|" * len(vs))
    for c in cells:
        row = []
        for v in vs:
            xs = cpu.get((v, size, c))
            if not xs:
                row.append("--")
                continue
            ims = sorted(it_med[(v, size, c)].values())
            row.append(f"{statistics.median(xs) / 1e6:.2f} [{q(xs, .1) / 1e6:.2f}-{q(xs, .9) / 1e6:.2f}]<br>it {ims[0] / 1e6:.2f}-{ims[-1] / 1e6:.2f}")
        print(f"| {c} | " + " | ".join(row) + " |")
    print(f"\n### {size}: variant minus `base`, ms per call, per iteration (the two processes of one iteration ran back to back); `x/n neg` = iterations with a negative difference\n")
    print("| cell | " + " | ".join(v for v in vs if v != "base") + " |")
    print("|---|" + "---:|" * (len(vs) - 1))
    for c in cells:
        row = []
        b = it_med.get(("base", size, c), {})
        for v in vs:
            if v == "base":
                continue
            m = it_med.get((v, size, c), {})
            ds = [(m[i] - b[i]) / 1e6 for i in sorted(m) if i in b]
            if not ds:
                row.append("--")
                continue
            row.append(f"{statistics.median(ds):+.2f} ({min(ds):+.2f}..{max(ds):+.2f}; {sum(x < 0 for x in ds)}/{len(ds)} neg)")
        print(f"| {c} | " + " | ".join(row) + " |")
    print(f"\n### {size}: in-process control: cell minus A of the SAME process, ms per call (per-iteration medians; median over iterations, range)\n")
    print("| cell | " + " | ".join(vs) + " |")
    print("|---|" + "---:|" * len(vs))
    for c in cells:
        if c == "A":
            continue
        row = []
        for v in vs:
            m, a = it_med.get((v, size, c), {}), it_med.get((v, size, "A"), {})
            ds = [(m[i] - a[i]) / 1e6 for i in sorted(m) if i in a]
            row.append(f"{statistics.median(ds):+.2f} ({min(ds):+.2f}..{max(ds):+.2f})" if ds else "--")
        print(f"| {c} | " + " | ".join(row) + " |")
    print(f"\n### {size}: context switches and minor faults per call (getrusage, whole process; medians over rounds): voluntary / involuntary / minflt\n")
    print("| cell | " + " | ".join(vs) + " |")
    print("|---|" + "---:|" * len(vs))
    for c in cells:
        row = []
        for v in vs:
            r = ru.get((v, size, c))
            row.append(f"{statistics.median(r['ru_nvcsw']):.0f} / {statistics.median(r['ru_nivcsw']):.0f} / {statistics.median(r['ru_minflt']):.0f}" if r else "--")
        print(f"| {c} | " + " | ".join(row) + " |")

print("\n## the split cells' host work per chunk (medians over rounds): encode CPU us / send-entry CPU us / send wall us (blocking: until the send returned; callback: until its completion arrived); recv wall ms per call\n")
print("| cell | size | " + " | ".join(vs) + " |")
print("|---|---|" + "---:|" * len(vs))
for size in ("16MiB", "4MiB"):
    for c in [c for c in cells if (c.endswith("-split") or c == "Df-chan")]:
        row = []
        for v in vs:
            s = sp.get((v, size, c))
            if not s:
                row.append("--")
                continue
            m = lambda x: statistics.median(s[x])
            row.append(f"{m('host_encode_cpu') / 1e3:.0f} / {m('host_send_cpu') / 1e3:.1f} / {m('host_send_wall') / 1e3:.0f}; {m('host_recv_wall') / 1e6:.2f}")
        print(f"| {c} | {size} | " + " | ".join(row) + " |")

# attribution pass
att = defaultdict(lambda: defaultdict(list))
for p in glob.glob(os.path.join(d, "attr-*.jsonl")):
    v = re.match(r"attr-(.+)\.jsonl$", os.path.basename(p)).group(1)
    for l in open(p):
        if l.startswith("#"):
            continue
        r = json.loads(l)
        k = (v, r["size"], r["cell"])
        for x, y in r.items():
            if isinstance(y, (int, float)) and (x.startswith(("cpu_", "vcs_", "ics_", "minflt_")) or x in ("allocs_big", "ru_nvcsw", "ru_nivcsw", "ru_minflt")) and x != "cpu_calls":
                att[k][x].append(y)
if att:
    print("\n## attribution pass (not timed: /proc reads and the allocation shim loaded), 16 MiB, per call, medians over rounds\n")
    print("Per thread class: context switches voluntary+involuntary (`cs`), CPU ms; process: minor faults, allocations >= 1 MiB.\n")
    print("| variant | cell | main | caller | cell-rt | core-rt | minflt | allocs >= 1 MiB |")
    print("|---|---|---:|---:|---:|---:|---:|---:|")
    for v in vs:
        for c in cells:
            a = att.get((v, "16MiB", c))
            if not a:
                continue
            med = lambda x: statistics.median(a[x]) if x in a else 0.0
            cls = []
            for cl in ("main", "caller", "cell-rt", "core-rt"):
                if f"cpu_{cl}" not in a or (med(f"cpu_{cl}") < 5e4 and med(f"vcs_{cl}") + med(f"ics_{cl}") < 0.5):
                    cls.append("")
                    continue
                cls.append(f"cs {med(f'vcs_{cl}'):.0f}+{med(f'ics_{cl}'):.0f}, {med(f'cpu_{cl}') / 1e6:.2f}")
            mf = sum(med(x) for x in a if x.startswith("minflt_"))
            print(f"| {v} | {c} | " + " | ".join(cls) + f" | {mf:.0f} | {med('allocs_big'):.2f} |")
