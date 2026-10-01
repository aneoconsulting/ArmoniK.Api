#!/usr/bin/env python3
"""Tables of `gen/tcp_attrib.sh OUT sweep` (the TCP worker sweep; absolute per call, ms; no ratio).

  sweep_tables.py OUT_DIR > OUT_DIR/tables-sweep.md

  1. per workload, cell, h2 variant and core workers W: client task-clock (perf stat around the loop;
     softirq run in the process's context included), median [p10-p90] over the processes' values, the
     process clock and the wall median [p10-p90] over every chunk, softirq time and NET_RX raises on the
     client CPUs, context switches per call (getrusage), the server's task-clock and context switches
     (perf stat -p); A and D (grpc++ transport, W = 8, stock core) head each workload;
  2. socket writes per call, bytes per write, epoll_wait, futex and all syscalls per call (strace, one
     process per unit at the W of SW_STRACE_WORKERS);
  3. Cf-* minus A per round (same round and workload): task-clock / wall, median (min..max).
"""
import json
import os
import re
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import tcp_attrib_tables as t  # noqa: E402

WL = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("d16k16", "d/16MiB k=16"),
      ("c54k1", "c/P5.4 k=1"), ("c54k8", "c/P5.4 k=8"), ("c54k16", "c/P5.4 k=16")]


def order(units):
    def key(u):
        if u in ("A", "D"):
            return (0, u, 0, 0)
        m = re.match(r"(Cf(?:-q)?)-(stock|batch)-w(\d+)$", u)
        return (1, m.group(1), {"stock": 0, "batch": 1}[m.group(2)], int(m.group(3))) if m else (2, u, 9, 0)
    return sorted(units, key=key)


def mpq_proc(xs, k):
    v = [x[k] for x in xs if x[k] == x[k]]
    if not v:
        return "-"
    return "%.3f [%.3f-%.3f]" % (statistics.median(v), t.q(v, 10), t.q(v, 90))


def main(out):
    d = os.path.join(out, "sweep")
    P, by = t.load(d, None)
    units = order({k[1] for k in by})
    L = []
    say = L.append
    say("# TCP worker sweep from C++ (per call, ms unless stated; absolute; no ratio): %d timed processes" % len(P))
    say("")
    say("Units: A and D (grpc++ transport; stock core, core workers 8); Cf and Cf-q per h2 variant (stock = crates.io h2, "
        "batch = h2-batch at AK_H2_COALESCE=16) and core --workers W. runner.log: cores, knobs, endpoint, machine.")
    say("")
    say("## 1. CPU, wall, switches, server")
    say("")
    say("task-clock: median [p10-p90] over the processes; process clock and wall: median [p10-p90] over every chunk.")
    say("")
    say("| workload | unit | task-clock | process clock | wall | softirq client CPUs | NET_RX client | csw | server task-clock | server csw | processes |")
    say("|---|---|---|---|---|---|---|---|---|---|---|")
    for w, wt in WL:
        for u in units:
            xs = by.get((w, u, "tcp"))
            if not xs:
                continue
            say("| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %d |" % (
                wt, u, mpq_proc(xs, "ctc"), t.mpq([v for x in xs for v in x["cpu"]]),
                t.mpq([v for x in xs for v in x["wall"]]), t.med(xs, "c_sirq"), t.med(xs, "c_rx", "%.0f"),
                t.med(xs, "csw", "%.0f"), t.med(xs, "srv"), t.med(xs, "s_csw", "%.0f"), len(xs)))
    say("")
    say("## 2. Writes and syscalls per call (strace -f, one process per unit, fewer batches)")
    say("")
    say("| workload | unit | socket writes | bytes/write | socket reads | epoll_wait | futex | all syscalls |")
    say("|---|---|---|---|---|---|---|---|")
    for w, wt in WL:
        for u in units:
            f = os.path.join(d, "strace-%s-tcp-%s.syscalls.txt" % (w, u))
            if not os.path.exists(f):
                continue
            j = json.loads(open(f).read().splitlines()[1])
            n = t.prof(f.replace(".syscalls.txt", ".out"))["calls"]
            cnt, sock = j["counts"], j["to_socket"]
            sw = sum(sock.get(x, 0) for x in ("write", "writev", "sendmsg", "sendto"))
            sr = sum(sock.get(x, 0) for x in ("read", "readv", "recvmsg", "recvfrom"))
            say("| %s | %s | %.1f | %.0f | %.1f | %.1f | %.1f | %.1f |" % (
                wt, u, sw / n, j["socket_write_bytes"] / max(1, sw), sr / n,
                sum(cnt.get(x, 0) for x in ("epoll_wait", "epoll_pwait", "epoll_pwait2")) / n, cnt.get("futex", 0) / n,
                sum(cnt.values()) / n))
    say("")
    say("## 3. Unit minus A per round (same round and workload): task-clock / wall, median (min..max)")
    say("")
    say("| workload | unit | task-clock | wall | rounds |")
    say("|---|---|---|---|---|")
    for w, wt in WL:
        a = {x["round"]: x for x in by.get((w, "A", "tcp"), [])}
        for u in units:
            if u == "A":
                continue
            pr = [(x, a[x["round"]]) for x in by.get((w, u, "tcp"), []) if x["round"] in a]
            if not pr:
                continue
            cells = []
            for k in ("ctc", "mwall"):
                dd = [x[k] - y[k] for x, y in pr if x[k] == x[k] and y[k] == y[k]]
                cells.append("%+.3f (%+.3f..%+.3f)" % (statistics.median(dd), min(dd), max(dd)) if dd else "-")
            say("| %s | %s | %s | %s | %d |" % (wt, u, cells[0], cells[1], len(pr)))
    say("")
    print("\n".join(L))


if __name__ == "__main__":
    main(sys.argv[1])
