#!/usr/bin/env python3
"""Tables of gen/profile_4a.sh's output directory (absolute per-call figures, no ratio).

  profile_tables.py OUT_DIR SYSTEM_MAP > OUT_DIR/tables.md

Reads stat/*.out (the loop's profile JSON) and stat/*.perfstat, record/*.data (through
gen/perf_attrib.py, whose JSON is kept as record/*.attrib.json), strace/*.syscalls.txt.
Milliseconds = cycles / 3.3e9 (the machine's locked frequency, recorded in runner.log).
"""
import collections
import glob
import json
import os
import statistics
import subprocess
import sys

GHZ = 3.3e9
CELLS = ["A", "D-retain", "Cf-retain", "Cf-q-retain"]
WLS = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("c54k1", "c/P5.4 k=1")]
THREADS = ["caller", "main", "tokio-rt-worker", "event_engine", "grpc_global_tim", "lifeguard"]


def q(xs, p):
    xs = sorted(xs)
    return xs[0] if len(xs) == 1 else statistics.quantiles(xs, n=10, method="inclusive")[{10: 0, 90: 8}[p]]


def mpq(xs):
    return "%.3f [%.3f-%.3f]" % (statistics.median(xs), q(xs, 10), q(xs, 90)) if xs else ""


def prof(path):
    for l in open(path):
        if l.startswith('{"profile"'):
            return json.loads(l)["profile"]
    return None


def perfstat(path):
    d = {}
    for l in open(path):
        p = l.strip().split(",")
        if len(p) > 3 and p[0] and p[0][0].isdigit():
            d[p[2]] = float(p[0])
    return d


def main(out, smap):
    L = []
    say = L.append
    say("# cpp physical probe step 4a: where the client CPU goes (A, D, Cf, Cf-q)")
    say("")
    say("Main configuration: CLIENT 1-4,11-14, SERVER 5-8,15-18, server 8 workers, core 8 workers, grpc-core sized "
        "for 8 CPUs (ncpus_shim), pinned transport, retain mode; the machine at 3.3 GHz (turbo off). Each client "
        "process runs `campaign_rpc --profile N` on ONE cell (only its own channel or client open unless stated): "
        "5 warm batches, then N batches in 10 chunks on the benchmark's own path; perf counts only that loop. "
        "Absolute per-call figures (ms = cycles / 3.3e9); median [p10-p90] over the chunks of the 3 rounds.")
    # ---- 1. per-call CPU, wall, perf stat, threads -------------------------------------------
    for sub, dirn, tag, title in (("a", "stat", "alone", "each cell alone in its process, under perf stat"),
                                  ("b", "stat", "withgrpc", "Cf and Cf-q with A, D, Cf, Cf-q all open in the process, under perf stat"),
                                  ("c", "plain", "alone", "each cell alone in its process, NO perf attached (the control)")):
        say("")
        say("## 1%s. Per call, %s" % (sub, title))
        say("")
        say("| workload | cell | CPU ms | wall ms | cycles u / k (M) | instr u / k (M) | cache-miss (k) | faults | ctx sw | n |")
        say("|---|---|---|---|---|---|---|---|---|---|")
        for w, wt in WLS:
            for c in CELLS:
                fs = sorted(glob.glob(os.path.join(out, dirn, "%s-%s-%s-r*.out" % (w, c, tag))))
                if not fs:
                    continue
                cpu, wall, ps = [], [], collections.defaultdict(list)
                for f in fs:
                    p = prof(f)
                    if not p:
                        continue
                    k = p["k"]
                    for ch in p["chunks"]:
                        n = ch["batches"] * k
                        cpu.append(ch["cpu_ns"] / n / 1e6); wall.append(ch["wall_ns"] / n / 1e6)
                    st = perfstat(f[:-4] + ".perfstat") if os.path.exists(f[:-4] + ".perfstat") else {}
                    for key, v in st.items():
                        ps[key].append(v / p["calls"])
                m = lambda key: statistics.median(ps[key]) if ps.get(key) else float("nan")
                if not ps:
                    say("| %s | %s | %s | %s | - | - | - | - | - | %d |" % (wt, c, mpq(cpu), mpq(wall), len(cpu)))
                    continue
                say("| %s | %s | %s | %s | %.2f / %.2f | %.2f / %.2f | %.0f | %.0f | %.0f | %d |" % (
                    wt, c, mpq(cpu), mpq(wall), m("cycles:u") / 1e6, m("cycles:k") / 1e6, m("instructions:u") / 1e6,
                    m("instructions:k") / 1e6, m("cache-misses") / 1e3, m("page-faults"), m("context-switches"), len(cpu)))
        say("")
        say("Per-thread CPU per call (ms, median of the rounds; schedstat, every thread of the class):")
        say("")
        say("| workload | cell | " + " | ".join(THREADS) + " | all |")
        say("|---|---|" + "---|" * (len(THREADS) + 1))
        for w, wt in WLS:
            for c in CELLS:
                fs = sorted(glob.glob(os.path.join(out, dirn, "%s-%s-%s-r*.out" % (w, c, tag))))
                if not fs:
                    continue
                per = collections.defaultdict(list)
                nthr = {}
                for f in fs:
                    p = prof(f)
                    tot = 0
                    for cls, v in p["thread_cpu_ns"].items():
                        per[cls].append(v["ns"] / p["calls"] / 1e6); tot += v["ns"]
                        nthr[cls] = v["threads"]
                    per["all"].append(tot / p["calls"] / 1e6)
                say("| %s | %s | %s | %.3f |" % (wt, c, " | ".join(
                    ("%.3f (%d)" % (statistics.median(per[t]), nthr[t])) if per.get(t) else "" for t in THREADS),
                    statistics.median(per["all"])))
    # ---- 2. attribution --------------------------------------------------------------------------
    say("")
    say("## 2. Where the cycles go (perf record, cycles, LBR call stacks; ms per call; one process per cell)")
    say("")
    say("Buckets from gen/perf_attrib.py (k: kernel, classed by the syscall wrapper or fault path in the call chain; "
        "u: user, classed by the leaf function, memcpy by its caller). Kernel symbols resolved with the booted "
        "kernel's System.map at the KASLR offset in each file's attrib JSON.")
    att = {}
    for w, _ in WLS:
        for c in CELLS:
            data = os.path.join(out, "record", "%s-%s.data" % (w, c))  # committed gzip'd (.data.gz)
            jf = data[:-5] + ".attrib.json"
            if not os.path.exists(data) and not os.path.exists(jf):
                continue
            if os.path.exists(data) and (not os.path.exists(jf) or os.path.getmtime(jf) < os.path.getmtime(data)):
                r = subprocess.run([sys.executable, os.path.join(os.path.dirname(__file__), "perf_attrib.py"), smap, data,
                                    data[:-5] + ".out", "--top", "25"], stdout=subprocess.PIPE, text=True)
                open(jf, "w").write(r.stdout)
            att[(w, c)] = json.load(open(jf))
    for w, wt in WLS:
        cs = [c for c in CELLS if (w, c) in att]
        if not cs:
            continue
        buckets = []
        for c in cs:
            for b in att[(w, c)]["buckets_cycles_per_call"]:
                if b not in buckets:
                    buckets.append(b)
        buckets.sort(key=lambda b: (b[0] != "k", b))
        say("")
        say("### %s" % wt)
        say("")
        say("| bucket | " + " | ".join(cs) + " | Cf - D | Cf-q - Cf |")
        say("|---|" + "---|" * (len(cs) + 2))
        g = lambda c, b: att[(w, c)]["buckets_cycles_per_call"].get(b, 0.0) / GHZ * 1e3 if (w, c) in att else 0.0
        for b in buckets:
            say("| %s | %s | %+.3f | %+.3f |" % (b, " | ".join("%.3f" % g(c, b) for c in cs),
                                              g("Cf-retain", b) - g("D-retain", b), g("Cf-q-retain", b) - g("Cf-retain", b)))
        tot = lambda c: att[(w, c)]["cycles_per_call"] / GHZ * 1e3 if (w, c) in att else 0.0
        say("| **total (sampled)** | %s | %+.3f | %+.3f |" % (" | ".join("%.3f" % tot(c) for c in cs),
                                                            tot("Cf-retain") - tot("D-retain"), tot("Cf-q-retain") - tot("Cf-retain")))
        say("")
        say("KASLR offset / anchor share: " + ", ".join("%s %s %.2f" % (c, att[(w, c)]["kaslr_offset"], att[(w, c)]["kaslr_check"]) for c in cs))
        for c in cs:
            say("")
            say("Top symbols, %s %s (ms per call): " % (wt, c) + "; ".join(
                "%s %.3f" % (s[:70], v / GHZ * 1e3) for s, v in att[(w, c)]["top_symbols_cycles_per_call"][:14]))
    # ---- 3. syscalls ------------------------------------------------------------------------------
    say("")
    say("## 3. Syscalls per call (strace -f -yy, between the loop's markers; fewer calls than sections 1-2)")
    say("")
    say("| workload | cell | calls | socket writes | bytes/write | socket reads | epoll_wait | futex | write (eventfd) | read | mmap/munmap/madvise | EAGAIN |")
    say("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for w, wt in WLS:
        for c in CELLS:
            f = os.path.join(out, "strace", "%s-%s.syscalls.txt" % (w, c))
            if not os.path.exists(f):
                continue
            lines = open(f).read().splitlines()
            d = json.loads(lines[1]); p = prof(f.replace(".syscalls.txt", ".out")); n = p["calls"]
            cnt, sock = d["counts"], d["to_socket"]
            sw = sum(sock.get(x, 0) for x in ("write", "writev", "sendmsg", "sendto"))
            sr = sum(sock.get(x, 0) for x in ("read", "readv", "recvmsg", "recvfrom"))
            say("| %s | %s | %d | %.1f | %.0f | %.1f | %.1f | %.1f | %.1f | %.1f | %.1f | %.1f |" % (
                wt, c, n, sw / n, d["socket_write_bytes"] / max(1, sw), sr / n, (cnt.get("epoll_wait", 0) + cnt.get("epoll_pwait", 0) + cnt.get("epoll_pwait2", 0)) / n,
                cnt.get("futex", 0) / n, (cnt.get("write", 0) - sock.get("write", 0)) / n, (cnt.get("read", 0) - sock.get("read", 0)) / n,
                (cnt.get("mmap", 0) + cnt.get("munmap", 0) + cnt.get("madvise", 0)) / n, sum(d["errors"].values()) / n))
    say("")
    say("Socket write sizes (count per call by power-of-two ceiling, bytes):")
    for w, wt in WLS:
        for c in CELLS:
            f = os.path.join(out, "strace", "%s-%s.syscalls.txt" % (w, c))
            if os.path.exists(f):
                d = json.loads(open(f).read().splitlines()[1]); n = prof(f.replace(".syscalls.txt", ".out"))["calls"]
                say("- %s %s: " % (wt, c) + ", ".join("<=%s: %.1f" % (k, v / n) for k, v in d["socket_write_size_le_pow2"].items()))
    print("\n".join(L))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
