#!/usr/bin/env python3
"""Resolve the AKP_BT lines of gen/probe/allocprobe.c (stderr of a probe run with AKP_BT=N):
group the >= 1 MiB allocations by their resolved call stack (addr2line on each object at its
file offset). Attribution tooling only.
   gen/probe/akp_bt.py STDERR_FILE [FRAMES]"""
import collections, re, subprocess, sys
txt = open(sys.argv[1], errors="replace").read()
nf = int(sys.argv[2]) if len(sys.argv) > 2 else 14
maps = []
m = re.search(r"AKP_MAPS_BEGIN\n(.*?)AKP_MAPS_END", txt, re.S)
for l in (m.group(1).splitlines() if m else []):
    p = l.split()
    if len(p) >= 6 and "x" in p[1]:
        a, b = (int(x, 16) for x in p[0].split("-"))
        maps.append((a, b, int(p[2], 16), p[5]))
cache = {}
def res(addr):
    for a, b, off, path in maps:
        if a <= addr < b:
            key = (path, addr - a + off)
            if key not in cache:
                o = subprocess.run(["addr2line", "-f", "-C", "-i", "-e", path, hex(addr - a + off - 1)], capture_output=True, text=True).stdout.split("\n")
                cache[key] = (o[0] if o and o[0] != "??" else "?") + " @" + path.rsplit("/", 1)[-1]
            return cache[key]
    return hex(addr)
groups = collections.Counter(); sizes = collections.defaultdict(list)
for l in txt.splitlines():
    if l.startswith("AKP_BT "):
        p = l.split()
        size = int(p[1], 16)
        fr = tuple(res(int(x, 16)) for x in p[4:4 + nf])  # skip backtrace, bt, count/malloc frames
        groups[fr] += 1; sizes[fr].append(size)
for fr, n in groups.most_common(12):
    print(f"{n} allocations, sizes {sorted(set(sizes[fr]))[:4]}")
    for f in fr:
        print("    " + f[:160])
