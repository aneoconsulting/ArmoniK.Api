#!/usr/bin/env python3
"""Buckets the cycles of a `perf record -e cycles --call-graph lbr` profile of one
`campaign_rpc --profile` loop into named mechanisms, per call.

  perf_attrib.py SYSTEM_MAP PERF.data PROFILE.out [--top N]  -> one JSON object

Kernel addresses are hidden from an unprivileged user (kptr_restrict = 1), so they are
resolved here with the booted kernel's System.map and the KASLR offset, found as the
2 MiB-aligned shift under which the outermost kernel frames of the samples land on the kernel's
entry points (entry_SYSCALL_64, asm_exc_page_fault, asm_sysvec_*, ...); the share that does is
reported (`kaslr_check`).

A sample's bucket comes from its call chain (leaf first). Kernel leaf: the first of
page faults, futex, epoll, socket write, socket read, memory syscalls, eventfd, scheduling /
interrupts, other kernel, found anywhere in the kernel part of the chain. User leaf: memcpy
(split by who called it: the core's encode, the transport crates, grpc/protobuf, other),
allocation (malloc family), the core's encode, hyper/h2/tonic/http/bytes, tokio/mio (runtime and
park), the core's rpc glue, grpc-core, protobuf, libc synchronisation and syscall wrappers, the
harness, other. The bucket names are listed in BUCKETS below.
"""
import bisect
import collections
import json
import re
import subprocess
import sys

BUCKETS = [
    "k: page faults (the fault path)", "k: futex", "k: epoll_wait", "k: socket write (sendmsg/writev)",
    "k: socket read (recvmsg/read)", "k: mmap/munmap/madvise/brk", "k: eventfd", "k: scheduling, interrupts",
    "k: other syscalls", "k: other",
    "u: memcpy in encode", "u: memcpy in hyper/h2/bytes", "u: memcpy in grpc/protobuf", "u: memcpy other",
    "u: allocation (malloc/free)", "u: encode (core codec)", "u: hyper/h2/tonic/http/bytes",
    "u: tokio/mio runtime and park", "u: core rpc glue (ak_core rpc, FFI entry)", "u: grpc-core (libgrpc, gpr, absl)",
    "u: protobuf", "u: libc sync (pthread, futex wrappers)", "u: libc syscall wrappers", "u: harness", "u: other",
]


def load_map(path):
    addrs, names = [], []
    for line in open(path):
        p = line.split()
        if len(p) >= 3 and p[1] in "tTwW":
            addrs.append(int(p[0], 16)); names.append(p[2])
    order = sorted(range(len(addrs)), key=lambda i: addrs[i])
    return [addrs[i] for i in order], [names[i] for i in order]


def resolver(addrs, names, off):
    """ip -> kernel symbol name. An address past the core kernel's text (_etext) is module code
    (loadable modules sit in the module area above it; System.map does not describe them): it is
    "[module]", not the last core symbol bisect would return."""
    lo = addrs[names.index("_stext")] if "_stext" in names else addrs[0]
    hi = addrs[names.index("_etext")] if "_etext" in names else addrs[-1]

    def kname(ip):
        a = ip - off
        if a >= hi:
            return "[module]"
        if a < lo:
            return "?"
        return names[bisect.bisect_right(addrs, a) - 1]
    return kname


ENTRY = re.compile(r"^(entry_SYSCALL_64.*|asm_exc_.*|asm_sysvec_.*|asm_common_interrupt|ret_from_fork.*|common_interrupt|entry_.*)$")


def parse(data):
    out = subprocess.run(["perf", "script", "-i", data, "-F", "comm,tid,period,ip,sym,dso"],
                         stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True).stdout
    samples, cur = [], None
    for line in out.splitlines():
        if not line.strip():
            continue
        if not line.startswith("\t") and not line.startswith(" "):
            p = line.split()
            cur = {"comm": " ".join(p[:-2]), "period": int(p[-1]), "frames": []}
            samples.append(cur)
        elif cur is not None:
            m = re.match(r"\s*([0-9a-f]+)\s+(.*?)\s+\(([^)]*)\)\s*$", line)
            if m:
                cur["frames"].append((int(m.group(1), 16), m.group(2), m.group(3)))
    return samples


ANCHORS = ("entry_SYSCALL_64_after_hwframe", "entry_SYSCALL_64", "rep_movs_alternative", "clear_page_erms",
           "syscall_return_via_sysret", "asm_sysvec_apic_timer_interrupt", "native_queued_spin_lock_slowpath",
           "asm_exc_page_fault", "_copy_from_iter", "_raw_spin_lock", "entry_SYSRETQ_unsafe_stack")


def kaslr(samples, addrs, names):
    """The 2 MiB-aligned offset under which the most kernel frames fall INSIDE one of a few
    frequent kernel functions (ANCHORS); kaslr_check = the share of all kernel frames that then
    resolve to an anchor or to a function whose name is not a section marker."""
    ext = {}
    for i, n in enumerate(names):
        if n in ANCHORS and i + 1 < len(addrs):
            ext[n] = (addrs[i], addrs[i + 1] - addrs[i])
    votes = collections.Counter()
    kf = [f[0] for s in samples for f in s["frames"] if f[0] >= 0xffff800000000000]
    for ip in kf[:20000]:
        for a, size in ext.values():
            d = ip - a
            if d >= 0 and (d & ((1 << 21) - 1)) < size:
                votes[d & ~((1 << 21) - 1)] += 1
    if not votes:
        return 0, 0.0
    off = votes.most_common(1)[0][0]
    return off, votes[off] / max(1, min(len(kf), 20000))


# A page fault is the fault path itself. clear_page_erms alone is NOT one: on this kernel it also
# zeroes the pages alloc_skb_with_frags takes for a socket write (seen in the p6 attribution).
PF = re.compile(r"exc_page_fault|handle_mm_fault|do_anonymous_page|do_fault|do_user_addr_fault|alloc_anon_folio")


def bucket(frames, kname):
    ks = [kname(f[0]) for f in frames if f[0] >= 0xffff800000000000]
    us = [(f[1], f[2]) for f in frames if f[0] < 0xffff800000000000]
    if frames and frames[0][0] >= 0xffff800000000000:
        j = " ".join(ks)
        # the syscall wrapper: the first user frame that is not glibc's cancellation helper
        w = next((u[0] for u in us if not re.search(r"syscall_cancel|__internal_syscall", u[0])), "")
        if PF.search(j):
            return "k: page faults (the fault path)"
        if re.search(r"writev|sendmsg|sendto", w):
            return "k: socket write (sendmsg/writev)"
        if re.search(r"recvmsg|recvfrom|readv|__libc_recv$|^recv$", w):
            return "k: socket read (recvmsg/read)"
        if re.search(r"epoll", w):
            return "k: epoll_wait"
        if re.search(r"futex|pthread|lll_|sem_", w) or "futex" in j:
            return "k: futex"
        if re.search(r"munmap|madvise|mmap|brk|mremap", w):
            return "k: mmap/munmap/madvise/brk"
        if re.search(r"(^|_)(write|read)$|__libc_write|__libc_read|__GI___libc_write|__GI___libc_read", w):
            return "k: eventfd"
        if re.search(r"asm_sysvec|irq|interrupt|schedule|__switch_to|finish_task_switch", j):
            return "k: scheduling, interrupts"
        if us:
            return "k: other syscalls"
        return "k: other"
    if not us:
        return "u: other"
    leaf, leafdso = us[0]
    chain = " ".join(u[0] for u in us)
    dsos = " ".join(u[1] for u in us[:6])
    if re.search(r"__mem(move|cpy|set)|memcpy|memmove", leaf):
        if re.search(r"ak_core::enc|ak_uencode|ak_encode|encode_into|enc_blob|Enc::", chain):
            return "u: memcpy in encode"
        if re.search(r"h2::|hyper::|bytes::|tonic::|http_body|http::", chain):
            return "u: memcpy in hyper/h2/bytes"
        if re.search(r"libgrpc|libprotobuf|libgpr", dsos) or re.search(r"grpc|protobuf", chain):
            return "u: memcpy in grpc/protobuf"
        return "u: memcpy other"
    if re.search(r"malloc|_int_free|free|realloc|calloc|sysmalloc|unlink_chunk|tcache|arena|consolidate", leaf) and "libc" in leafdso:
        return "u: allocation (malloc/free)"
    if re.search(r"ak_core::enc|ak_uencode|ak_encode|encode_into|enc_blob|fill_", leaf):
        return "u: encode (core codec)"
    if re.search(r"^(<)?(h2|hyper|tonic|http|http_body|bytes|http_body_util)::", leaf) or re.search(r"<(h2|hyper|tonic|bytes|http)::", leaf):
        return "u: hyper/h2/tonic/http/bytes"
    if re.search(r"tokio|mio::|parking_lot|crossbeam|std::sys::.*(futex|sync|thread)|std::thread", leaf):
        return "u: tokio/mio runtime and park"
    if "libak_core" in leafdso:
        return "u: core rpc glue (ak_core rpc, FFI entry)"
    if re.search(r"libgrpc|libgpr|libabsl|libaddress_sorting|libupb|libre2|libcares", leafdso):
        return "u: grpc-core (libgrpc, gpr, absl)"
    if "libprotobuf" in leafdso:
        return "u: protobuf"
    if "libc" in leafdso and re.search(r"pthread|lll_|futex|__GI___pthread|cond|mutex|sem_", leaf):
        return "u: libc sync (pthread, futex wrappers)"
    if "libc" in leafdso:
        return "u: libc syscall wrappers"
    if "campaign_rpc" in leafdso or "libstdc++" in leafdso:
        return "u: harness"
    return "u: other"


def main(argv):
    smap, data, prof = argv[1], argv[2], argv[3]
    top = int(argv[5]) if len(argv) > 5 and argv[4] == "--top" else 20
    pj = next((json.loads(l)["profile"] for l in open(prof) if l.startswith('{"profile"')), None)
    calls = pj["calls"] if pj else 1
    addrs, names = load_map(smap)
    samples = parse(data)
    off, share = kaslr(samples, addrs, names)

    kname = resolver(addrs, names, off)
    b = collections.Counter()
    sym = collections.Counter()
    comm = collections.Counter()
    tot = 0
    for s in samples:
        if not s["frames"]:
            continue
        p = s["period"]
        tot += p
        b[bucket(s["frames"], kname)] += p
        f = s["frames"][0]
        sym[("[k] " + kname(f[0])) if f[0] >= 0xffff800000000000 else f[1]] += p
        comm[s["comm"]] += p
    print(json.dumps({
        "profile": {k: pj[k] for k in ("cell", "payload", "dir", "k", "calls")} if pj else None,
        "samples": len(samples), "cycles_total": tot, "cycles_per_call": tot / calls,
        "kaslr_offset": hex(off), "kaslr_check": round(share, 4),
        "buckets_cycles_per_call": {k: b[k] / calls for k in BUCKETS if b[k]},
        "threads_cycles_per_call": {k: v / calls for k, v in comm.most_common()},
        "top_symbols_cycles_per_call": [[k, v / calls] for k, v in sym.most_common(top)],
    }))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
