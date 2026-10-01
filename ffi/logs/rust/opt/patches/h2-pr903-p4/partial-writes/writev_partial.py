#!/usr/bin/env python3
"""writev_partial.py TRACE: writev calls of an `strace -f -v -s 0 -e trace=writev` trace, offered
bytes (sum of iov_len) against the return value: full, partial, refused (-1 EAGAIN)."""
import re, sys, collections
tot = big = full = part = eag = 0; iovs = collections.Counter(); ex = []
for l in open(sys.argv[1]):
    m = re.search(r'writev\(\d+, \[(.*)\], (\d+)\) = (-?\d+)', l)
    if not m:
        continue
    lens = [int(x) for x in re.findall(r'iov_len=(\d+)', m.group(1))]
    off, ret = sum(lens), int(m.group(3)); tot += 1; iovs[len(lens)] += 1
    if off > 100000:
        big += 1
        if ret < 0: eag += 1
        elif ret == off: full += 1
        else: part += 1
        if len(ex) < 12: ex.append((off, len(lens), ret))
print(f"writev calls {tot}; with more than 100 kB offered: {big}; of those accepted in full {full}, partially {part}, refused (-1 EAGAIN) {eag}")
print("iovecs per writev (count): " + ", ".join(f"{k}: {v}" for k, v in sorted(iovs.items())))
print("offered bytes / iovecs / returned, first 12 large writes:")
for e in ex: print("  ", e)
