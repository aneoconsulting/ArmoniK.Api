#!/usr/bin/env python3
"""Table of gen/h2_pr903_strace.sh: python3 gen/h2pr903_strace_tables.py OUT_DIR > OUT_DIR/tables.md
Syscalls per call (strace -f -c of the whole process: setup and warm-up included, divided by the
calls the process made)."""
import os, re, sys
out = sys.argv[1]
NAMES = {"stock": "stock", "p4": "p4 N=16", "prc": "PR core", "prh": "PR host-too", "pp": "PR+p4 N=16"}
calls = {}
for l in open(os.path.join(out, "calls.txt")):
    c, tr, size, k, cell, _, n = l.split()
    calls[(c, tr, size, k, cell)] = int(n)
print(open(os.path.join(out, "header.txt")).read())
print("syscalls per call: all / writev+write / epoll_wait / futex / recvfrom+read (whole process incl. setup and warm-up, divided by its calls)\n")
print("| transport | workload | cell | " + " | ".join(NAMES.values()) + " |")
print("|---|---|---|" + "---|" * len(NAMES))
for tr in ["uds", "tcp"]:
    for size, k in [("16MiB", "k1"), ("16MiB", "k8")]:
        for cell in ["A", "Cf"]:
            cols = []
            for c in NAMES:
                f = os.path.join(out, f"{c}-{tr}-{size}-{k}-{cell}.strace")
                if not os.path.exists(f):
                    cols.append(""); continue
                d = {}
                for l in open(f):
                    m = re.match(r"\s*[\d.]+\s+[\d.]+\s+\d+\s+(\d+)\s+(?:\d+\s+)?(\w+)$", l)
                    if m:
                        d[m.group(2)] = int(m.group(1))
                n = calls[(c, tr, size, k, cell)]
                g = lambda *ks: sum(d.get(x, 0) for x in ks) / n
                cols.append(f"{g(*[x for x in d if x != 'total']):.0f} / {g('writev','write'):.0f} / {g('epoll_wait'):.0f} / {g('futex'):.0f} / {g('recvfrom','read'):.0f}")
            print(f"| {tr} | {size} {k} | {cell} | " + " | ".join(cols) + " |")
