#!/usr/bin/env python3
"""Classify a `perf script -F comm,tid,period,ip,sym,dso` dump (perf record --call-graph dwarf):
shares of the sampled cycles by thread class, by leaf crate (the Rust crate of the leaf frame, or
libc:mem / libc:alloc / libc:other, or kernel), the caller crate of libc and kernel leaves (the
first frame above them in a Rust crate other than std/core/alloc), the syscall wrapper of kernel
leaves, the top leaf symbols, and the inclusive share of each crate (samples with any frame in it).
Weights are the sample periods (cycles).
   gen/perf_classify.py SCRIPT_FILE [SYSTEM_MAP]

With SYSTEM_MAP (the booted kernel's, /run/booted-system/kernel's directory), kernel addresses,
hidden from an unprivileged user by kptr_restrict = 1, are resolved at the KASLR offset found as
the 2 MiB-aligned shift under which the most kernel frames land inside a few frequent kernel
functions (the method of poc/cpp/gen/perf_attrib.py, reimplemented here); the share of kernel
frames that land on an anchor is printed. Adds the kernel leaf functions and a kernel bucket
table (the socket write path split into copy from user, skb allocation and the rest).
"""
import re, sys
from collections import Counter

import bisect, collections
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
        cur[2].append((m.group(2), m.group(3), int(m.group(1), 16)))

tot = sum(s[0] for s in samples) or 1
pct = lambda x: f"{100.0 * x / tot:5.1f}%"
print(f"# {len(samples)} samples, {tot:,} cycles sampled (weights = periods)")

by_t = Counter()
leaf_c = Counter()
leaf_s = Counter()
lib_caller = Counter()
kern = Counter()
incl = Counter()
KB = 0xffff800000000000
kname = lambda ip: None
if len(sys.argv) > 2:
    addrs, names = [], []
    for l in open(sys.argv[2]):
        p = l.split()
        if len(p) >= 3 and p[1] in "tTwW":
            addrs.append(int(p[0], 16)); names.append(p[2])
    o = sorted(range(len(addrs)), key=lambda i: addrs[i]); addrs = [addrs[i] for i in o]; names = [names[i] for i in o]
    ANCH = ("entry_SYSCALL_64_after_hwframe", "entry_SYSCALL_64", "rep_movs_alternative", "clear_page_erms",
            "syscall_return_via_sysret", "asm_sysvec_apic_timer_interrupt", "asm_exc_page_fault", "_copy_from_iter",
            "_raw_spin_lock", "entry_SYSRETQ_unsafe_stack")
    ext = {n: (addrs[i], addrs[i + 1] - addrs[i]) for i, n in enumerate(names) if n in ANCH and i + 1 < len(addrs)}
    votes = collections.Counter()
    kf = [f[2] for s_ in samples for f in s_[2] if f[2] >= KB][:20000]
    for ip in kf:
        for a, size in ext.values():
            d = ip - a
            if d >= 0 and (d & ((1 << 21) - 1)) < size:
                votes[d & ~((1 << 21) - 1)] += 1
    off = votes.most_common(1)[0][0] if votes else 0
    print(f"# kernel symbols: {sys.argv[2]}, KASLR offset {off:#x}, anchor share of kernel frames {votes[off] / max(1, len(kf)):.3f}")
    def kname(ip, off=off):
        i = bisect.bisect_right(addrs, ip - off) - 1
        return names[i] if i >= 0 else "?"
kleaf = Counter(); kbucket = Counter()
def kb(ks, wrap):
    j = " ".join(ks)
    leaf = ks[0] if ks else "?"
    if re.search(r"exc_page_fault|handle_mm_fault|do_anonymous_page", j):
        return "page fault"
    if re.search(r"writev|sendmsg|sendto", wrap) or re.search(r"do_writev|unix_stream_sendmsg|sock_write_iter", j):
        if re.search(r"rep_movs|copy_from_iter|copy_user|_copy_from", leaf) or re.search(r"^(rep_movs_alternative|_copy_from_iter)", leaf):
            return "socket write: copy from user"
        if re.search(r"clear_page|memset", leaf):
            return "socket write: zeroing new skb pages"
        if re.search(r"alloc_skb|kmalloc|kmem_cache_alloc|__alloc_pages|memcg|charge|skb_|free|pgalloc|slab|rmqueue|page_ext", " ".join(ks[:6])):
            return "socket write: skb alloc/free, memcg"
        if re.search(r"sock_def_readable|wake_up|try_to_wake|ttwu|select_task_rq", " ".join(ks[:8])):
            return "socket write: wake the reader"
        return "socket write: other (entry, locks, unix stream)"
    if re.search(r"recv|readv", wrap) or re.search(r"unix_stream_recvmsg|unix_stream_read", j):
        return "socket read"
    if "epoll" in wrap or "ep_poll" in j or "epoll" in j:
        return "epoll_wait"
    if "futex" in j or "futex" in wrap:
        return "futex"
    if re.search(r"asm_sysvec|irq|interrupt|schedule|__switch_to|finish_task_switch", j):
        return "scheduling, interrupts"
    return "other kernel"
for w, comm, fr in samples:
    by_t[tclass(comm)] += w
    if fr and fr[0][2] >= KB and len(sys.argv) > 2:
        ks = [kname(f[2]) for f in fr if f[2] >= KB]
        wrap = next((re.sub(r"\+0x[0-9a-f]+$", "", f[0]) for f in fr if f[2] < KB and not re.search(r"syscall_cancel|__internal_syscall", f[0])), "")
        kleaf[ks[0]] += w
        kbucket[kb(ks, wrap)] += w
    if not fr:
        leaf_c["(no frames)"] += w
        continue
    sym, dso = fr[0][0], fr[0][1]
    c = crate(sym, dso)
    leaf_c[c] += w
    leaf_s[(c, re.sub(r"\+0x[0-9a-f]+$", "", sym)[:110])] += w
    crates = [crate(s, d) for s, d, _ in fr]
    caller = next((x for x in crates[1:] if not x.startswith("libc") and x not in BORING and x != "kernel" and x != "rust-rt"), "?")
    if c.startswith("libc"):
        lib_caller[(c, caller)] += w
    if c == "kernel":
        wrap = next((re.sub(r"\+0x[0-9a-f]+$", "", s) for s, d, _ in fr if not d.startswith("[kernel") and s != "[unknown]"), "?")
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
if kbucket:
    table("kernel buckets (System.map)", kbucket, 20)
    table("kernel leaf functions (System.map)", kleaf, 30)
