#!/usr/bin/env python3
"""Tables of `gen/tcp_attrib.sh OUT h2` (the h2 PR #903 comparison; absolute per call, ms; no ratio).

  h2pr_tables.py OUT_DIR SESSION [SESSION...] > OUT_DIR/tables-SESSION.md
  (SESSION: h2 = three cores; h2b = four cores, the combined PR #903 + p4 core as pr903p4. Strace
  counts are read from the first SESSION dir that has the unit's file, and the table says which.)

  1. per workload, unit (cell on core) and transport: client CPU from perf stat task-clock (softirq
     run in the process's context included) and the process clock beside it, softirq time and NET_RX
     raises on the client's CPUs (irq_time), wall, context switches (getrusage), the server's
     task-clock and context switches (perf stat -p): median [p10-p90] over chunks for the process clock
     and wall, median [min-max] over processes for the per-process measures;
  2. writes and syscalls per call (strace, one process per unit, workload and transport): socket
     writes, bytes per write, socket reads, epoll_wait, futex, all;
  3. Cf-* minus A per round (same round, workload and transport), per core: task-clock / process
     clock / wall, median (min..max) over rounds.
"""
import json
import os
import statistics
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import tcp_attrib_tables as t  # noqa: E402

WL = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("d4k1", "d/4MiB k=1"), ("c54k1", "c/P5.4 k=1"), ("c54k8", "c/P5.4 k=8")]
CORES = ["ctl", "h16", "pr903", "pr903p4"]
UNITS = ["A", "D"] + ["%s-%s" % (c, k) for c in ("Cf", "Cf-q", "Cf-zc") for k in CORES]


def main(out, sessions):
    d = os.path.join(out, sessions[0])
    P, by = t.load(d, None)
    L = []
    say = L.append
    say("# h2 PR #903 against p4 and crates.io h2, from C++ (per call, ms unless stated; absolute; no ratio)")
    say("")
    say("Units: A and D on the ctl core (grpc++ transport); Cf (ring 6 + lock), Cf-q (ring 24 + lock), Cf-zc (ring 6 + lock, "
        "d only) on each core: ctl = crates.io h2 0.4.19, h16 = p4 (AK_H2_COALESCE=16), pr903 = h2 PR #903 ported onto "
        "0.4.19, pr903p4 = PR #903 and p4 combined (AK_H2_COALESCE=16); one stack (p1-p9) for every core. Session %s: %d "
        "timed processes." % (sessions[0], len(P)))
    say("")
    say("## 1. CPU, wall, switches, server")
    say("")
    say("| workload | unit | tr | task-clock | process clock | softirq client CPUs | NET_RX client | wall | csw | server task-clock | server csw | chunks |")
    say("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for w, wt in WL:
        for u in UNITS:
            for tr in ("uds", "tcp"):
                xs = by.get((w, u, tr))
                if not xs:
                    continue
                say("| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %d |" % (
                    wt, u, tr, t.med(xs, "ctc"), t.mpq([v for x in xs for v in x["cpu"]]), t.med(xs, "c_sirq"),
                    t.med(xs, "c_rx", "%.0f"), t.mpq([v for x in xs for v in x["wall"]]), t.med(xs, "csw", "%.0f"),
                    t.med(xs, "srv"), t.med(xs, "s_csw", "%.0f"), sum(len(x["cpu"]) for x in xs)))
    say("")
    say("## 2. Writes and syscalls per call (strace -f, one process per unit, workload and transport, fewer batches)")
    say("")
    say("| workload | unit | tr | socket writes | bytes/write | socket reads | epoll_wait | futex | all syscalls | from |")
    say("|---|---|---|---|---|---|---|---|---|---|")
    for w, wt in WL:
        for u in UNITS:
            for tr in ("uds", "tcp"):
                f = next((os.path.join(out, ss, "strace-%s-%s-%s.syscalls.txt" % (w, tr, u)) for ss in sessions
                          if os.path.exists(os.path.join(out, ss, "strace-%s-%s-%s.syscalls.txt" % (w, tr, u)))), None)
                if not f:
                    continue
                j = json.loads(open(f).read().splitlines()[1])
                n = t.prof(f.replace(".syscalls.txt", ".out"))["calls"]
                cnt, sock = j["counts"], j["to_socket"]
                sw = sum(sock.get(x, 0) for x in ("write", "writev", "sendmsg", "sendto"))
                sr = sum(sock.get(x, 0) for x in ("read", "readv", "recvmsg", "recvfrom"))
                say("| %s | %s | %s | %.1f | %.0f | %.1f | %.1f | %.1f | %.1f | %s |" % (
                    wt, u, tr, sw / n, j["socket_write_bytes"] / max(1, sw), sr / n,
                    sum(cnt.get(x, 0) for x in ("epoll_wait", "epoll_pwait", "epoll_pwait2")) / n, cnt.get("futex", 0) / n,
                    sum(cnt.values()) / n, os.path.basename(os.path.dirname(f))))
    say("")
    say("## 3. Cf-* minus A per round (same round, workload, transport): task-clock / process clock / wall, median (min..max)")
    say("")
    cores = [c for c in CORES if any(k[1].endswith("-" + c) for k in by)]
    say("| workload | cell | tr | " + " | ".join(cores) + " |")
    say("|---|---|---|" + "---|" * len(cores))
    for w, wt in WL:
        for c in ("D", "Cf", "Cf-q", "Cf-zc"):
            for tr in ("uds", "tcp"):
                a = {x["round"]: x for x in by.get((w, "A", tr), [])}
                cells = []
                for core in (cores if c != "D" else ["-"]):
                    u = c if c == "D" else "%s-%s" % (c, core)
                    pr = [(x, a[x["round"]]) for x in by.get((w, u, tr), []) if x["round"] in a]
                    if not pr:
                        cells.append("-")
                        continue
                    parts = []
                    for k in ("ctc", "mcpu", "mwall"):
                        dd = [x[k] - y[k] for x, y in pr if x[k] == x[k] and y[k] == y[k]]
                        parts.append("%+.3f (%+.3f..%+.3f)" % (statistics.median(dd), min(dd), max(dd)) if dd else "-")
                    cells.append(" / ".join(parts))
                if all(x == "-" for x in cells):
                    continue
                if c == "D":
                    cells = cells + [""] * (len(cores) - 1)
                say("| %s | %s | %s | %s |" % (wt, c, tr, " | ".join(cells)))
    say("")
    print("\n".join(L))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:] or ["h2"])
