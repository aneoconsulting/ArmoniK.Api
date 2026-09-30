#!/usr/bin/env python3
"""Per-thread syscalls of a `campaign_rpc --profile` loop from `strace -f -T -yy` output, between
the loop's two marker writes: per thread, the socket writes, the epoll_wait calls and how many of
them blocked (took more than BLOCK_US), the futex calls, the time spent inside epoll_wait and
futex. Threads are strace's tids; the tids that wrote to the socket are the ones that ran the
connection's writes (a count above one is the connection moving between workers).

  strace_threads.py STRACE.txt CALLS [BLOCK_US]  -> one JSON object
"""
import collections
import json
import re
import sys

LINE = re.compile(r"^(\d+)\s+(.*)$")
T = re.compile(r"<(\d+\.\d+)>\s*$")


def main(path, calls, block_us=50.0):
    calls = float(calls)
    block_us = float(block_us)
    inside = False
    per = collections.defaultdict(collections.Counter)
    tm = collections.defaultdict(collections.Counter)
    pend = {}
    for raw in open(path, errors="replace"):
        m = LINE.match(raw.rstrip("\n"))
        if not m:
            continue
        tid, rest = m.group(1), m.group(2)
        if "AK_PROFILE_BEGIN" in rest:
            inside = True
            continue
        if "AK_PROFILE_END" in rest:
            break
        if not inside:
            continue
        r = re.match(r"^<\.\.\. ([a-z_0-9]+) resumed>", rest)
        if r:
            name, fd = r.group(1), pend.pop((tid, r.group(1)), "")
        else:
            c = re.match(r"^([a-z_0-9]+)\((\d+)?(<[^>]*>)?", rest)
            if not c:
                continue
            name, fd = c.group(1), c.group(3) or ""
            if rest.endswith("<unfinished ...>"):
                pend[(tid, name)] = fd
                continue
        t = T.search(rest)
        dur = float(t.group(1)) * 1e6 if t else 0.0
        sock = "UNIX" in fd or "TCP" in fd or "socket" in fd
        if name in ("writev", "sendmsg", "write", "sendto") and sock:
            per[tid]["socket_writes"] += 1
        elif name.startswith("epoll"):
            per[tid]["epoll_wait"] += 1
            tm[tid]["epoll_us"] += dur
            if dur > block_us:
                per[tid]["epoll_blocked"] += 1
        elif name == "futex":
            per[tid]["futex"] += 1
            tm[tid]["futex_us"] += dur
            if dur > block_us:
                per[tid]["futex_blocked"] += 1
        elif name in ("recvfrom", "recvmsg", "read") and sock:
            per[tid]["socket_reads"] += 1
    writers = [t for t in per if per[t]["socket_writes"] > 0]
    tot = collections.Counter()
    for t in per:
        tot.update(per[t])
    ttot = collections.Counter()
    for t in tm:
        ttot.update(tm[t])
    print(json.dumps({
        "calls": calls, "threads_with_syscalls": len(per), "threads_writing_the_socket": len(writers),
        "per_call": {k: v / calls for k, v in sorted(tot.items())},
        "per_call_us": {k: v / calls for k, v in sorted(ttot.items())},
        "writers_share_of_writes": sorted((round(per[t]["socket_writes"] / max(1, tot["socket_writes"]), 3) for t in writers), reverse=True),
        "block_us": block_us}, sort_keys=True))


if __name__ == "__main__":
    main(*sys.argv[1:4])
