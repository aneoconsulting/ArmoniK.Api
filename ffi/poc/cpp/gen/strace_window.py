#!/usr/bin/env python3
"""The syscalls of a `campaign_rpc --profile` loop, from `strace -f -yy` output: only the lines
between its two marker writes (write(<devnull>, "AK_PROFILE_BEGIN") and "AK_PROFILE_END").

  strace_window.py STRACE.txt   -> first line: a summary; then JSON: counts per syscall, per
                                   syscall the number that went to a socket, the byte sizes
                                   of the socket writes and reads (histogram), errors (EAGAIN)

Handles strace -f's split lines (`<unfinished ...>` / `<... NAME resumed>`).
"""
import collections
import json
import re
import sys

LINE = re.compile(r"^(\d+)\s+(.*)$")
CALL = re.compile(r"^([a-z_0-9]+)\((\d+)?(<[^>]*>)?")
RES = re.compile(r"^<\.\.\. ([a-z_0-9]+) resumed>")
RET = re.compile(r"= (-?\d+|\?)(?: ([A-Z]+))?")
WR = ("write", "writev", "sendmsg", "sendto")
RD = ("read", "readv", "recvmsg", "recvfrom")


def main(path):
    inside = False
    counts = collections.Counter()
    sock = collections.Counter()
    errs = collections.Counter()
    wsz, rsz = collections.Counter(), collections.Counter()
    wbytes = rbytes = 0
    pending = {}
    seen_begin = seen_end = False
    for raw in open(path, errors="replace"):
        m = LINE.match(raw.rstrip("\n"))
        if not m:
            continue
        pid, rest = m.group(1), m.group(2)
        if "AK_PROFILE_BEGIN" in rest:
            inside = True; seen_begin = True
            continue
        if "AK_PROFILE_END" in rest:
            inside = False; seen_end = True
            continue
        r = RES.match(rest)
        if r:
            name = r.group(1)
            fd = pending.pop((pid, name), "")
        else:
            c = CALL.match(rest)
            if not c:
                continue
            name, fd = c.group(1), (c.group(3) or "")
            if rest.endswith("<unfinished ...>"):
                pending[(pid, name)] = fd
                if inside:
                    counts[name] += 1
                    if "UNIX" in fd or "TCP" in fd or "socket" in fd:
                        sock[name] += 1
                continue
            if inside:
                counts[name] += 1
                if "UNIX" in fd or "TCP" in fd or "socket" in fd:
                    sock[name] += 1
        if not inside:
            continue
        t = RET.search(rest)
        if not t:
            continue
        if t.group(2):
            errs[name + " " + t.group(2)] += 1
        if (("UNIX" in fd) or ("TCP" in fd) or ("socket" in fd)) and t.group(1) not in ("?",) and int(t.group(1)) > 0:
            n = int(t.group(1))
            b = 1 << max(0, (n - 1).bit_length())
            if name in WR:
                wbytes += n; wsz[b] += 1
            elif name in RD:
                rbytes += n; rsz[b] += 1
    ok = seen_begin and seen_end
    print("window %s: %d syscalls, socket writes %d (%d B), socket reads %d (%d B)" % (
        "found" if ok else "NOT FOUND", sum(counts.values()), sum(sock[n] for n in WR), wbytes,
        sum(sock[n] for n in RD), rbytes))
    print(json.dumps({"window": ok, "counts": dict(counts), "to_socket": dict(sock), "errors": dict(errs),
                      "socket_write_bytes": wbytes, "socket_read_bytes": rbytes,
                      "socket_write_size_le_pow2": {str(k): v for k, v in sorted(wsz.items())},
                      "socket_read_size_le_pow2": {str(k): v for k, v in sorted(rsz.items())}}, sort_keys=True))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
