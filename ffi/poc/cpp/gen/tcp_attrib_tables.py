#!/usr/bin/env python3
"""Tables of a gen/tcp_attrib.sh run (absolute per call, ms; no ratio).

  tcp_attrib_tables.py OUT_DIR SYSTEM_MAP > OUT_DIR/tables.md

  1. perf: client and server cycles per call split by network path (gen/perf_net_attrib.py; cached
     next to each perf.data as .net.json), and the receive-softirq cycles found under a user-side
     socket write;
  2. wall: client CPU and wall per call (chunks), the per-call duration p10 / median / p90 (batch
     trace), and from the `ss -tinm` samples of the loop's connection, per side: Send-Q, Recv-Q,
     notsent, cwnd, rtt, snd_wnd, rcv_space (median [p90] over the samples), and busy,
     rwnd_limited, sndbuf_limited as ss reports them at the last sample (cumulative over the
     connection's life);
     then per thread class from strace -f -T: socket writes, time inside them, writes over 50 us,
     EAGAIN on write and read, epoll_wait (and blocked), futex, per call;
  3. p4: per unit, workload and transport, client CPU and wall, the server's task-clock per call
     (perf stat -p), median [p10-p90]; the per-round difference Cf-h16 minus Cf-ctl and each Cf minus
     A; socket writes per call and bytes per write (strace).
"""
import collections
import glob
import json
import os
import re
import statistics
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
WL = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("d4k1", "d/4MiB k=1"), ("c54k1", "c/P5.4 k=1")]
CELLS = ["A", "D-retain", "Cf-retain", "Cf-zc-retain"]


def q(xs, p):
    xs = sorted(xs)
    return xs[0] if len(xs) == 1 else statistics.quantiles(xs, n=10, method="inclusive")[{10: 0, 90: 8}[p]]


def mpq(xs, f="%.3f"):
    return (f + " [" + f + "-" + f + "]") % (statistics.median(xs), q(xs, 10), q(xs, 90)) if xs else "-"


def prof(path):
    if not os.path.exists(path):
        return None
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


def per_call(p):
    return ([ch["cpu_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]],
            [ch["wall_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]])


def net(data, out, smap):
    cache = data + ".net.json"
    if not os.path.exists(cache):
        r = subprocess.run([sys.executable, os.path.join(HERE, "perf_net_attrib.py"), smap, data, out],
                           capture_output=True, text=True)
        if r.returncode != 0:
            return None
        open(cache, "w").write(r.stdout)
    return json.loads(open(cache).read())


SHORT = [("netfilter", r"netfilter"), ("send", r"tcp send path|unix send path"), ("lo tx", r"loopback transmit"), ("lo rx softirq", r"loopback receive"),
         ("wakeup", r"wakeup"), ("epoll", r"epoll"), ("futex", r"futex"), ("sched", r"sched"), ("read", r"socket read"),
         ("faults", r"page fault"), ("mem", r"memcpy|copy|clear_page|alloc")]


def split(buckets):
    o = collections.Counter()
    for k, v in buckets.items():
        lab = next((s for s, rx in SHORT if re.search(rx, k)), "other")
        o[lab] += v
    return o


def ss_parse(path, server_port, client_ports):
    import gzip
    side = {"client": collections.defaultdict(list), "server": collections.defaultdict(list)}
    last = {}
    hdr = None
    for l in gzip.open(path, "rt", errors="replace"):
        if l.startswith("@"):
            continue
        if not l.startswith(("\t", " ")):
            f = l.split()
            hdr = None
            if len(f) >= 5:
                lp, pp = int(f[3].rsplit(":", 1)[1]), int(f[4].rsplit(":", 1)[1])
                if lp == server_port and pp in client_ports:
                    hdr = ("server", int(f[1]), int(f[2]))
                elif pp == server_port and lp in client_ports:
                    hdr = ("client", int(f[1]), int(f[2]))
            continue
        if not hdr:
            continue
        s, rq, sq = hdr
        d = side[s]
        d["recv_q"].append(rq)
        d["send_q"].append(sq)
        for k in ("cwnd", "snd_wnd", "rcv_space", "notsent", "unacked"):
            m = re.search(r"\b%s:(\d+)" % k, l)
            d[k].append(int(m.group(1)) if m else 0)
        m = re.search(r"\brtt:([\d.]+)/", l)
        if m:
            d["rtt"].append(float(m.group(1)))
        m = re.search(r"skmem:\(r(\d+),rb\d+,t(\d+),tb(\d+)", l)
        if m:
            d["skmem_r"].append(int(m.group(1)))
            d["skmem_t"].append(int(m.group(2)))
            d["tb"].append(int(m.group(3)))
        last[s] = {k: (re.search(r"\b%s:(\S+)" % k, l).group(1) if re.search(r"\b%s:(\S+)" % k, l) else "-")
                   for k in ("busy", "rwnd_limited", "sndbuf_limited")}
    return side, last


def mp90(xs, f="%.0f"):
    return (f + " [" + f + "]") % (statistics.median(xs), q(xs, 90)) if xs else "-"


def main(out, smap):
    L = []
    say = L.append
    say("# TCP inversion attribution (per call, ms unless stated; absolute; no ratio)")
    say("")
    pdir = os.path.join(out, "perf")
    if os.path.isdir(pdir):
        say("## 1. perf: cycles per call by network path (client, then server), ms at 3.3 GHz")
        say("")
        say("Client: `perf record -e cycles -F 4000 --call-graph lbr` on the process; server: `perf record -p` on the "
            "rpc_server, both enabled by the client around its loop. Kernel bucket = first network-path symbol walking the "
            "kernel chain from the leaf (gen/perf_net_attrib.py). `rx under write` = kernel samples whose chain holds a "
            "receive-softirq symbol AND whose user caller is a socket-write wrapper; `rx anywhere` = any chain holding one.")
        say("")
        cols = [s for s, _ in SHORT] + ["other"]
        say("| workload | cell | tr | side | total | user | kernel | " + " | ".join(cols) + " | rx anywhere | rx under write | module code at leaf |")
        say("|---|---|---|---|---|---|---|" + "---|" * len(cols) + "---|---|---|")
        for w, wt in WL:
            for c in CELLS:
                for tr in ("uds", "tcp"):
                    base = os.path.join(pdir, "%s-%s-%s" % (w, tr, c))
                    for side, data in (("client", base + ".data"), ("server", base + ".server.data")):
                        if not os.path.exists(data) and not os.path.exists(data + ".net.json"):
                            continue  # committed logs keep DATA.gz and the analysis cache DATA.net.json
                        j = net(data, base + ".out", smap)
                        if not j:
                            say("| %s | %s | %s | %s | analysis failed |" % (wt, c, tr, side))
                            continue
                        m = j["ms_per_call"]
                        b = split(j["kernel_buckets_ms_per_call"])
                        say("| %s | %s | %s | %s | %.3f | %.3f | %.3f | %s | %.3f | %.3f | %.3f |" % (
                            wt, c, tr, side, m["total"], m["user"], m["kernel"], " | ".join("%.3f" % b[x] for x in cols),
                            m["receive_softirq_anywhere"], m["receive_softirq_under_a_socket_write"],
                            m.get("module_code_at_the_leaf", float("nan"))))
        say("")
    wdir = os.path.join(out, "wall")
    if os.path.isdir(wdir):
        say("## 2. wall: per-call durations, the connection's TCP state, and where the threads wait")
        say("")
        say("| workload | cell | tr | CPU | wall | call p10 / median / p90 (batch trace) | server CPU (schedstat) |")
        say("|---|---|---|---|---|---|---|")
        for w, wt in WL:
            for c in CELLS:
                for tr in ("uds", "tcp"):
                    p = prof(os.path.join(wdir, "%s-%s-%s.out" % (w, tr, c)))
                    if not p:
                        continue
                    cpu, wall = per_call(p)
                    bt = p["batch_trace"]
                    say("| %s | %s | %s | %s | %s | %.3f / %.3f / %.3f | %.3f |" % (
                        wt, c, tr, mpq(cpu), mpq(wall), bt["call_ns_p10"] / 1e6, bt["call_ns_median"] / 1e6,
                        bt["call_ns_p90"] / 1e6, p["server"]["cpu_ns"] / p["calls"] / 1e6))
        say("")
        say("TCP state, `ss -tinm` every ~5 ms during the process, the loop's connection only (client ports from the "
            "process's own getsockname): median [p90] over samples. Queues and windows in bytes, rtt in ms. busy, "
            "rwnd_limited, sndbuf_limited: the kernel's cumulative tcp_info at the last sample (time and share of busy).")
        say("")
        say("| workload | cell | side | samples | Send-Q | Recv-Q | notsent | skmem t (wmem queued) | skmem r | cwnd | rtt | snd_wnd | rcv_space | busy | rwnd_limited | sndbuf_limited |")
        say("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
        for w, wt in WL:
            for c in CELLS:
                f = os.path.join(wdir, "%s-tcp-%s.ss.txt.gz" % (w, c))
                p = prof(os.path.join(wdir, "%s-tcp-%s.out" % (w, c)))
                if not p or not os.path.exists(f):
                    continue
                socks = p.get("tcp_sockets") or []
                if not socks:
                    continue
                sp = socks[0]["peer_port"]
                side, last = ss_parse(f, sp, {s["local_port"] for s in socks})
                for s in ("client", "server"):
                    d = side[s]
                    lt = last.get(s, {})
                    say("| %s | %s | %s | %d | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |" % (
                        wt, c, s, len(d["send_q"]), mp90(d["send_q"]), mp90(d["recv_q"]), mp90(d["notsent"]),
                        mp90(d["skmem_t"]), mp90(d["skmem_r"]), mp90(d["cwnd"]), mp90(d["rtt"], "%.3f"),
                        mp90(d["snd_wnd"]), mp90(d["rcv_space"]), lt.get("busy", "-"), lt.get("rwnd_limited", "-"),
                        lt.get("sndbuf_limited", "-")))
        say("")
        say("strace -f -T (one process, fewer batches), per call: all threads summed; writer threads = those that wrote the socket.")
        say("")
        say("| workload | cell | tr | socket writes | us in writes | writes > 50 us | write EAGAIN | socket reads | read EAGAIN | epoll_wait | epoll blocked | us in epoll | futex | futex blocked | us in futex | writer threads |")
        say("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
        for w, wt in WL:
            for c in CELLS:
                for tr in ("uds", "tcp"):
                    f = os.path.join(wdir, "%s-%s-%s.st.threads.json" % (w, tr, c))
                    if not os.path.exists(f):
                        continue
                    j = json.loads(open(f).read())
                    pc, pu = j["per_call"], j["per_call_us"]
                    g = lambda k: pc.get(k, 0.0)
                    say("| %s | %s | %s | %.1f | %.0f | %.1f | %.1f | %.1f | %.1f | %.1f | %.1f | %.0f | %.1f | %.1f | %.0f | %d |" % (
                        wt, c, tr, g("socket_writes"), pu.get("socket_write_us", 0), g("socket_write_over_block_us"),
                        g("socket_write_eagain"), g("socket_reads"), g("socket_read_eagain"), g("epoll_wait"),
                        g("epoll_blocked"), pu.get("epoll_us", 0), g("futex"), g("futex_blocked"), pu.get("futex_us", 0),
                        j["threads_writing_the_socket"]))
        say("")
    cpu_section(out, say)
    for d in ("p4", "p4-run1"):
        p4_section(out, d, say)
    print("\n".join(L))

def load(dirp, unit_rx):
    P = []
    for f in sorted(glob.glob(os.path.join(dirp, "[0-9]*.out"))):
        m = re.match(r"(\d+)-r(\d+)-([a-z0-9]+)-(uds|tcp)-(.+)\.out$", os.path.basename(f))
        p = prof(f)
        if not m or not p:
            continue
        n = p["calls"]
        cpu, wall = per_call(p)
        stem = f[:-len(".out")]  # gen/tcp_attrib.sh writes STEM.out, STEM.client.perfstat, STEM.server.perfstat
        ps = perfstat(stem + ".server.perfstat")
        pc = perfstat(stem + ".client.perfstat")
        it = p.get("irq_time")
        hz = it["user_hz"] if it else 100
        g = lambda d, k: d.get(k, float("nan"))
        P.append({"round": int(m.group(2)), "wl": m.group(3), "tr": m.group(4), "unit": m.group(5), "cpu": cpu, "wall": wall,
                  "mcpu": statistics.median(cpu), "mwall": statistics.median(wall),
                  # perf stat: task-clock in msec, cycles in counts (ms at 3.3 GHz)
                  "srv": g(ps, "task-clock") / n, "ctc": g(pc, "task-clock") / n,
                  "ccyc": g(pc, "cycles") / n / 3.3e6, "ccyc_k": g(pc, "cycles:k") / n / 3.3e6, "ccyc_u": g(pc, "cycles:u") / n / 3.3e6,
                  "c_sirq": it["client_cpus"]["softirq_ticks"] * 1e3 / hz / n if it else float("nan"),
                  "s_sirq": it["server_cpus"]["softirq_ticks"] * 1e3 / hz / n if it else float("nan"),
                  "c_rx": it["client_cpus"]["net_rx"] / n if it else float("nan"),
                  "s_rx": it["server_cpus"]["net_rx"] / n if it else float("nan"),
                  "c_irq": it["client_cpus"]["irq_ticks"] * 1e3 / hz / n if it else float("nan"),
                  "csw": (p["rusage"]["nvcsw"] + p["rusage"]["nivcsw"]) / n,
                  "s_csw": g(ps, "context-switches") / n})
    by = collections.defaultdict(list)
    for x in P:
        by[(x["wl"], x["unit"], x["tr"])].append(x)
    return P, by


def med(xs, k, f="%.3f"):
    v = [x[k] for x in xs if x[k] == x[k]]
    return (f + " [" + f + "-" + f + "]") % (statistics.median(v), min(v), max(v)) if v else "-"


def cpu_section(out, say):
    d = os.path.join(out, "cpu")
    if not os.path.isdir(d):
        return
    P, by = load(d, None)
    say("## 3. Client CPU with and without softirq time (A, D, Cf, Cf-zc; both transports)")
    say("")
    say("CONFIG_IRQ_TIME_ACCOUNTING=y: softirq time is charged to no task. `process clock` = CLOCK_PROCESS_CPUTIME_ID per chunk "
        "(median [p10-p90] over chunks), what every earlier CPU figure used; `task-clock` and `cycles` = perf stat on the client "
        "process around the same loop, which do include softirq run in the process's context; `softirq on client CPUs` = "
        "/proc/stat's softirq time summed over the client's 8 CPUs across the loop (USER_HZ ticks); NET_RX = /proc/softirqs "
        "raises on those CPUs. Per-process values: median [min-max] over processes (2 rounds).")
    say("")
    say("| workload | cell | tr | process clock | task-clock | cycles (all / user / kernel), ms at 3.3 GHz | softirq on client CPUs | NET_RX client | softirq on server CPUs | NET_RX server | server task-clock | wall |")
    say("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for w, wt in WL:
        for c in CELLS:
            for tr in ("uds", "tcp"):
                xs = by.get((w, c, tr))
                if not xs:
                    continue
                say("| %s | %s | %s | %s | %s | %s / %s / %s | %s | %s | %s | %s | %s | %s |" % (
                    wt, c, tr, mpq([v for x in xs for v in x["cpu"]]), med(xs, "ctc"), med(xs, "ccyc", "%.2f"),
                    med(xs, "ccyc_u", "%.2f"), med(xs, "ccyc_k", "%.2f"), med(xs, "c_sirq"), med(xs, "c_rx", "%.0f"),
                    med(xs, "s_sirq"), med(xs, "s_rx", "%.0f"), med(xs, "srv"), mpq([v for x in xs for v in x["wall"]])))
    say("")
    say("Gap to A per round (same transport), median (min..max) over rounds: process clock / task-clock / wall.")
    say("")
    say("| workload | cell | UDS | TCP |")
    say("|---|---|---|---|")
    for w, wt in WL:
        for c in CELLS[1:]:
            cells = []
            for tr in ("uds", "tcp"):
                a = {x["round"]: x for x in by.get((w, "A", tr), [])}
                pr = [(x, a[x["round"]]) for x in by.get((w, c, tr), []) if x["round"] in a]
                if not pr:
                    cells.append("-")
                    continue
                parts = []
                for k in ("mcpu", "ctc", "mwall"):
                    dd = [x[k] - y[k] for x, y in pr]
                    parts.append("%+.3f (%+.3f..%+.3f)" % (statistics.median(dd), min(dd), max(dd)))
                cells.append(" / ".join(parts))
            say("| %s | %s | %s |" % (wt, c, " | ".join(cells)))
    say("")


def p4_section(out, name, say):
    p4 = os.path.join(out, name)
    if not os.path.isdir(p4):
        return
    P, by = load(p4, None)
    say("## 4%s. p4 on TCP and UDS (%s): Cf over the p1-p3 stack, crates.io h2 (ctl, AK_H2_COALESCE=1) against the p4 h2 "
        "(h16, AK_H2_COALESCE=16); A on the ctl arm" % ("" if name == "p4" else "b", name))
    say("")
    if name != "p4":
        say("This run predates the client perf stat: its client CPU is the process clock only (no softirq time).")
        say("")
    say("| workload | unit | tr | client process clock | client task-clock | softirq on client CPUs | wall | server task-clock (perf stat -p) | socket writes | bytes/write | chunks |")
    say("|---|---|---|---|---|---|---|---|---|---|---|")
    for w, wt in WL:
        for u in ("A-ctl", "Cf-ctl", "Cf-h16"):
            for tr in ("uds", "tcp"):
                xs = by.get((w, u, tr))
                if not xs:
                    continue
                sw = bpw = "-"
                sf = os.path.join(p4, "strace-%s-%s-%s.syscalls.txt" % (w, tr, u))
                if os.path.exists(sf):
                    d = json.loads(open(sf).read().splitlines()[1])
                    n = prof(sf.replace(".syscalls.txt", ".out"))["calls"]
                    s = sum(d["to_socket"].get(k, 0) for k in ("write", "writev", "sendmsg", "sendto"))
                    sw, bpw = "%.1f" % (s / n), "%.0f" % (d["socket_write_bytes"] / max(1, s))
                say("| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %d |" % (
                    wt, u, tr, mpq([c for x in xs for c in x["cpu"]]), med(xs, "ctc"), med(xs, "c_sirq"),
                    mpq([c for x in xs for c in x["wall"]]), med(xs, "srv"), sw, bpw, sum(len(x["cpu"]) for x in xs)))
    say("")
    say("Per-round differences (same round, workload and transport), median (min..max) over rounds: client process clock / "
        "client task-clock / wall / server task-clock.")
    say("")
    say("| workload | tr | Cf-h16 - Cf-ctl | Cf-ctl - A | Cf-h16 - A |")
    say("|---|---|---|---|---|")
    for w, wt in WL:
        for tr in ("uds", "tcp"):
            cells = []
            for a, b in (("Cf-h16", "Cf-ctl"), ("Cf-ctl", "A-ctl"), ("Cf-h16", "A-ctl")):
                bb = {x["round"]: x for x in by.get((w, b, tr), [])}
                pr = [(x, bb[x["round"]]) for x in by.get((w, a, tr), []) if x["round"] in bb]
                if not pr:
                    cells.append("-")
                    continue
                parts = []
                for k in ("mcpu", "ctc", "mwall", "srv"):
                    dd = [x[k] - y[k] for x, y in pr if x[k] == x[k] and y[k] == y[k]]
                    parts.append("%+.3f (%+.3f..%+.3f)" % (statistics.median(dd), min(dd), max(dd)) if dd else "-")
                cells.append(" / ".join(parts))
            say("| %s | %s | %s |" % (wt, tr, " | ".join(cells)))
    say("")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
