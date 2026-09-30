#!/usr/bin/env python3
"""Classify a `perf script -F comm,tid,period,ip,sym,dso` dump (perf record --call-graph dwarf):
shares of the sampled cycles by thread class, by leaf crate (the Rust crate of the leaf frame, or
libc:mem / libc:alloc / libc:other, or kernel), the caller crate of libc and kernel leaves (the
first frame above them in a Rust crate other than std/core/alloc), the syscall wrapper of kernel
leaves, the top leaf symbols, and the inclusive share of each crate (samples with any frame in it).
Weights are the sample periods (cycles).
   gen/perf_classify.py SCRIPT_FILE"""
import re, sys
from collections import Counter

FRAME = re.compile(r"^\s*([0-9a-f]+)\s+(.*?)\s+\((.*)\)\s*$")
BORING = {"std", "core", "alloc", "__rust_alloc", "__rdl_alloc"}


def crate(sym, dso):
    if dso.startswith("[kernel") or dso == "[unknown]" and sym == "[unknown]":
        return "kernel"
    s = re.sub(r"\+0x[0-9a-f]+$", "", sym)
    if "libc.so" in dso or "ld-linux" in dso or "libpthread" in dso:
        if re.search(r"mem(cpy|move|set)|__memmove|__memcpy|__memset", s):
            return "libc:mem"
        if re.search(r"malloc|free|realloc|calloc|_int_|tcache|arena|sysmalloc|memalign", s):
            return "libc:alloc"
        return "libc:other"
    s = s.lstrip("<&*").replace("mut ", "").replace("dyn ", "")
    m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)::", s)
    if m:
        return m.group(1)
    if s.startswith("ak_"):
        return "ak_core(C ABI)"
    if s.startswith("__rust") or s.startswith("_ZN"):
        return "rust-rt"
    return f"other:{dso.rsplit('/', 1)[-1]}"


def tclass(comm):
    if comm.startswith("cell-rt"):
        return "host-rt (cell-rt)"
    if comm.startswith("tokio-rt") or comm.startswith("tokio-runtime"):
        return "core-rt"
    if comm == "caller":
        return "caller"
    return "main" if comm.startswith("stream_probe") else f"other:{comm}"


samples = []  # (weight, comm, frames[(sym, dso)])
cur = None
for line in open(sys.argv[1], errors="replace"):
    if not line.strip():
        continue
    if not line[0].isspace():
        parts = line.split()
        # comm may contain spaces: tid and period are the last two integer fields
        nums = [i for i, p in enumerate(parts) if p.isdigit()]
        w = int(parts[nums[-1]]) if nums else 1
        comm = " ".join(parts[:nums[0]]) if nums else parts[0]
        cur = [w, comm, []]
        samples.append(cur)
        continue
    m = FRAME.match(line)
    if m and cur is not None:
        cur[2].append((m.group(2), m.group(3)))

tot = sum(s[0] for s in samples) or 1
pct = lambda x: f"{100.0 * x / tot:5.1f}%"
print(f"# {len(samples)} samples, {tot:,} cycles sampled (weights = periods)")

by_t = Counter()
leaf_c = Counter()
leaf_s = Counter()
lib_caller = Counter()
kern = Counter()
incl = Counter()
for w, comm, fr in samples:
    by_t[tclass(comm)] += w
    if not fr:
        leaf_c["(no frames)"] += w
        continue
    sym, dso = fr[0]
    c = crate(sym, dso)
    leaf_c[c] += w
    leaf_s[(c, re.sub(r"\+0x[0-9a-f]+$", "", sym)[:110])] += w
    crates = [crate(s, d) for s, d in fr]
    caller = next((x for x in crates[1:] if not x.startswith("libc") and x not in BORING and x != "kernel" and x != "rust-rt"), "?")
    if c.startswith("libc"):
        lib_caller[(c, caller)] += w
    if c == "kernel":
        wrap = next((re.sub(r"\+0x[0-9a-f]+$", "", s) for s, d in fr if not d.startswith("[kernel") and s != "[unknown]"), "?")
        kern[(wrap[:40], caller)] += w
    for x in set(crates):
        incl[x] += w

def table(title, cnt, n=40, fmt=lambda k: str(k)):
    print(f"\n## {title}\n")
    for k, v in cnt.most_common(n):
        print(f"  {pct(v)}  {fmt(k)}")

table("thread class", by_t)
table("leaf crate (self)", leaf_c)
table("libc leaves by caller crate", lib_caller, 30, lambda k: f"{k[0]:12} <- {k[1]}")
table("kernel leaves by user entry (first resolved user frame) and caller crate", kern, 30, lambda k: f"{k[0]:40} <- {k[1]}")
table("inclusive: samples with any frame in the crate", incl, 30)
table("top leaf symbols", leaf_s, 30, lambda k: f"[{k[0]}] {k[1]}")
