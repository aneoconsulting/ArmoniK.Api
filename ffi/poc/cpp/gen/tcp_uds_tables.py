#!/usr/bin/env python3
"""Tables of a gen/tcp_uds.sh run: UDS against TCP loopback, per workload and cell (absolute per
call, ms; no ratio).

  tcp_uds_tables.py OUT_DIR > OUT_DIR/tables.md

  1. per cell and transport: client CPU and wall, median [p10-p90] over every chunk of every
     process; context switches and minor faults per call (median of the processes); the server's
     CPU per call from perf stat -p (task-clock) and from schedstat (AK_SERVER_PID), its context
     switches per call; TCP_NODELAY on every client TCP socket (processes whose sockets all read 1);
  2. the gap to A per round and transport (median of the cell's process minus A's, same round,
     workload and transport): median and min..max; and TCP minus UDS per cell (median of the
     per-round differences);
  3. syscalls per call from the strace processes: socket writes, bytes per write, socket reads,
     epoll_wait, futex, all.
"""
import collections
import glob
import json
import os
import re
import statistics
import sys

WLS = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("d4k1", "d/4MiB k=1"), ("c54k1", "c/P5.4 k=1"), ("c54k8", "c/P5.4 k=8")]
UNITS = ["cur/A", "cur/D-retain", "cur/Cf-retain", "cur/Cf-q-retain", "stk/Cf-zc-retain"]


def q(xs, p):
    xs = sorted(xs)
    return xs[0] if len(xs) == 1 else statistics.quantiles(xs, n=10, method="inclusive")[{10: 0, 90: 8}[p]]


def mpq(xs):
    return "%.3f [%.3f-%.3f]" % (statistics.median(xs), q(xs, 10), q(xs, 90)) if xs else "-"


def prof(path):
    for l in open(path):
        if l.startswith('{"profile"'):
            return json.loads(l)["profile"]
    return None


def perfstat(path):
    d = {}
    if os.path.exists(path):
        for l in open(path):
            p = l.strip().split(",")
            if len(p) > 3 and p[0] and p[0][0].isdigit():
                d[p[2]] = float(p[0])
    return d


def main(out):
    P = []
    for f in sorted(glob.glob(os.path.join(out, "proc", "*.out"))):
        m = re.match(r"(\d+)-r(\d+)-([a-z0-9]+)-(uds|tcp)-(cur|stk)-(.+)\.out$", os.path.basename(f))
        p = prof(f)
        if not m or not p:
            continue
        n = p["calls"]
        ps = perfstat(f + ".server.perfstat")
        cpu = [ch["cpu_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]]
        wall = [ch["wall_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]]
        ru = p["rusage"]
        P.append({"round": int(m.group(2)), "wl": m.group(3), "tr": m.group(4), "unit": m.group(5) + "/" + m.group(6),
                  "cpu": cpu, "wall": wall, "mcpu": statistics.median(cpu), "mwall": statistics.median(wall),
                  "csw": (ru["nvcsw"] + ru["nivcsw"]) / n, "flt": ru["minflt"] / n,
                  "srv_perf": ps.get("task-clock", float("nan")) / n / 1e6 if ps else float("nan"),
                  "srv_csw": ps.get("context-switches", float("nan")) / n if ps else float("nan"),
                  "srv_sched": p.get("server", {}).get("cpu_ns", float("nan")) / n / 1e6,
                  "nodelay": (p.get("tcp_nodelay_on", 0), p.get("tcp_sockets_n", 0))})
    by = collections.defaultdict(list)
    for x in P:
        by[(x["wl"], x["unit"], x["tr"])].append(x)
    L = []
    say = L.append
    say("# UDS against TCP loopback: %d processes (per call, ms; absolute; no ratio)" % len(P))
    say("")
    say("## 1. Per cell and transport")
    say("")
    say("Server CPU: perf stat -p on the server enabled around the client's loop (task-clock) / the same from schedstat. "
        "nodelay: TCP processes whose every TCP socket read TCP_NODELAY = 1 (getsockopt on the live socket).")
    say("")
    say("| workload | cell | transport | CPU | wall | csw | flt | server CPU (perf / schedstat) | server csw | nodelay | n |")
    say("|---|---|---|---|---|---|---|---|---|---|---|")
    for w, wt in WLS:
        for u in UNITS:
            for tr in ("uds", "tcp"):
                xs = by.get((w, u, tr))
                if not xs:
                    continue
                nd = "%d/%d" % (sum(1 for x in xs if x["nodelay"][1] > 0 and x["nodelay"][0] == x["nodelay"][1]),
                                len(xs)) if tr == "tcp" else "-"
                say("| %s | %s | %s | %s | %s | %.1f | %.1f | %.3f / %.3f | %.1f | %s | %d |" % (
                    wt, u, tr, mpq([c for x in xs for c in x["cpu"]]), mpq([c for x in xs for c in x["wall"]]),
                    statistics.median([x["csw"] for x in xs]), statistics.median([x["flt"] for x in xs]),
                    statistics.median([x["srv_perf"] for x in xs]), statistics.median([x["srv_sched"] for x in xs]),
                    statistics.median([x["srv_csw"] for x in xs]), nd, len(xs) * 10))
    say("")
    say("## 2. Gaps to A per round (same transport), and TCP minus UDS per cell (same round)")
    say("")
    say("| workload | cell | gap to A, UDS: CPU median (min..max) / wall median | gap to A, TCP: CPU median (min..max) / wall median | TCP - UDS: CPU / wall / server CPU (median of rounds) |")
    say("|---|---|---|---|---|")
    for w, wt in WLS:
        for u in UNITS:
            cells = []
            for tr in ("uds", "tcp"):
                a = {x["round"]: x for x in by.get((w, "cur/A", tr), [])}
                xs = [x for x in by.get((w, u, tr), []) if x["round"] in a]
                if u == "cur/A" or not xs:
                    cells.append("-")
                    continue
                gc = [x["mcpu"] - a[x["round"]]["mcpu"] for x in xs]
                gw = [x["mwall"] - a[x["round"]]["mwall"] for x in xs]
                cells.append("%+.3f (%+.3f..%+.3f) / %+.3f" % (statistics.median(gc), min(gc), max(gc), statistics.median(gw)))
            ud = {x["round"]: x for x in by.get((w, u, "uds"), [])}
            td = [(x, ud[x["round"]]) for x in by.get((w, u, "tcp"), []) if x["round"] in ud]
            if not td:
                continue
            d = "%+.3f / %+.3f / %+.3f" % (statistics.median([t["mcpu"] - v["mcpu"] for t, v in td]),
                                           statistics.median([t["mwall"] - v["mwall"] for t, v in td]),
                                           statistics.median([t["srv_perf"] - v["srv_perf"] for t, v in td]))
            say("| %s | %s | %s | %s | %s |" % (wt, u, cells[0], cells[1], d))
    say("")
    say("## 3. Syscalls per call (strace, one process per cell and transport, fewer batches)")
    say("")
    say("| workload | cell | transport | socket writes | bytes/write | socket reads | epoll_wait | futex | all |")
    say("|---|---|---|---|---|---|---|---|---|")
    for w, wt in WLS:
        for u in UNITS:
            for tr in ("uds", "tcp"):
                f = os.path.join(out, "strace", "%s-%s-%s.syscalls.txt" % (w, tr, u.replace("/", "-")))
                if not os.path.exists(f):
                    continue
                d = json.loads(open(f).read().splitlines()[1])
                n = prof(f.replace(".syscalls.txt", ".out"))["calls"]
                cnt, sock = d["counts"], d["to_socket"]
                sw = sum(sock.get(x, 0) for x in ("write", "writev", "sendmsg", "sendto"))
                sr = sum(sock.get(x, 0) for x in ("read", "readv", "recvmsg", "recvfrom"))
                say("| %s | %s | %s | %.1f | %.0f | %.1f | %.1f | %.1f | %.1f |" % (
                    wt, u, tr, sw / n, d["socket_write_bytes"] / max(1, sw), sr / n,
                    sum(cnt.get(x, 0) for x in ("epoll_wait", "epoll_pwait", "epoll_pwait2")) / n, cnt.get("futex", 0) / n,
                    sum(cnt.values()) / n))
    print("\n".join(L))


if __name__ == "__main__":
    main(sys.argv[1])
