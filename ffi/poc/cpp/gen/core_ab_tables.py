#!/usr/bin/env python3
"""Tables of a gen/core_ab.sh directory (absolute per-call figures, no ratio).

  core_ab_tables.py OUT_DIR ARM [ARM ...] > OUT_DIR/tables.md

For `measure` (the main figures) and `default` (the default-allocator pass): per workload, cell
and arm, CPU and wall per call median [p10-p90] over the 10 chunks of every process, minor faults
and context switches (voluntary + involuntary) per call (median of the processes); per-thread CPU
for Cf and Cf-q; Cf - A per round (median(Cf process) - median(A process), same arm and round);
`strace`: socket writes, epoll_wait and futex per call.
"""
import collections
import glob
import json
import os
import statistics
import sys

CELLS = ["A", "D-retain", "Cf-retain", "Cf-q-retain", "Cf-enc-retain", "Cf-encp-retain", "Cf-zc-retain", "Cf-zcp-retain", "Cf-zcw-retain"]  # tables read old runs too
WLS = [("d16k1", "d/16MiB k=1"), ("d16k8", "d/16MiB k=8"), ("d4k1", "d/4MiB k=1"), ("d4k8", "d/4MiB k=8"),
       ("c54k1", "c/P5.4 k=1"), ("c54k8", "c/P5.4 k=8")]
THREADS = ["caller", "main", "tokio-rt-worker", "event_engine"]


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


def chunks(p):
    return [ch["cpu_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]], \
           [ch["wall_ns"] / (ch["batches"] * p["k"]) / 1e6 for ch in p["chunks"]]


def main(out, arms):
    L = []
    say = L.append
    say("# %s: core builds compared from C++ (one-cell `campaign_rpc --profile` processes)" % os.path.basename(out))
    say("")
    say("Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median "
        "[p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary "
        "context switches per call (median of the processes).")
    for phase, title in (("measure", "main figures (every process under AB_ENV, the allocator setting; 3 rounds)"),
                         ("default", "the default-allocator pass (1 round)")):
        if not os.path.isdir(os.path.join(out, phase)):
            continue
        say("")
        say("## %s: %s" % (phase, title))
        say("")
        say("task-clock: perf stat on the client around the loop (softirq run in the process's context included), median "
            "[min-max] over processes ('-' for runs before 2026-10-01); CPU: the process clock per chunk.")
        say("")
        say("| workload | cell | arm | task-clock ms | CPU ms | wall ms | flt | csw | n |")
        say("|---|---|---|---|---|---|---|---|---|")
        thr, med = [], {}
        for w, wt in WLS:
            for c in CELLS:
                for a in arms:
                    fs = sorted(glob.glob(os.path.join(out, phase, "%s-%s-%s-r*.out" % (w, c, a))))
                    if not fs:
                        continue
                    cpu, wall, flt, csw, tcs = [], [], [], [], []
                    per = collections.defaultdict(list)
                    for f in fs:
                        p = prof(f)
                        cc, ww = chunks(p)
                        cpu += cc; wall += ww
                        r = int(f.rsplit("-r", 1)[1].split(".")[0])
                        med[(w, c, a, r)] = (statistics.median(cc), statistics.median(ww))
                        pcf = f[:-len(".out")] + ".client.perfstat"
                        if os.path.exists(pcf):
                            for l in open(pcf):
                                cc_ = l.strip().split(",")
                                if len(cc_) > 3 and cc_[2] == "task-clock" and cc_[0][:1].isdigit():
                                    tcs.append(float(cc_[0]) / p["calls"])  # msec
                        ru = p["rusage"]
                        flt.append(ru["minflt"] / p["calls"]); csw.append((ru["nvcsw"] + ru["nivcsw"]) / p["calls"])
                        for cls, v in p["thread_cpu_ns"].items():
                            per[cls].append(v["ns"] / p["calls"] / 1e6)
                    tcf = "%.3f [%.3f-%.3f]" % (statistics.median(tcs), min(tcs), max(tcs)) if tcs else "-"
                    say("| %s | %s | %s | %s | %s | %s | %.1f | %.1f | %d |" % (wt, c, a, tcf, mpq(cpu), mpq(wall),
                                                                     statistics.median(flt), statistics.median(csw), len(cpu)))
                    if c.startswith("Cf"):
                        thr.append((wt, c, a, {t: statistics.median(per[t]) for t in THREADS if per.get(t)}))
        # run-queue wait and the batch trace, when the client recorded them
        if any("thread_wait_ns" in prof(f) for f in glob.glob(os.path.join(out, phase, "*.out"))[:1]):
            say("")
            say("Run-queue wait per call (ms; schedstat's second field, every thread of the class; median of the "
                "processes) and the batch trace (blocking path: per batch of k calls, median over the batches of the "
                "processes, ms): batch wall, one call's duration median [p10-p90], first call start to last call end "
                "(span), batch start to its last call's start (dispatch), last call end to batch end (completion).")
            say("")
            say("| workload | cell | arm | wait: caller | main | tokio-rt-worker | event_engine | batch wall | call [p10-p90] | span | dispatch | completion |")
            say("|---|---|---|---|---|---|---|---|---|---|---|---|")
            for w, wt in WLS:
                for c in CELLS:
                    for a in arms:
                        fs = sorted(glob.glob(os.path.join(out, phase, "%s-%s-%s-r*.out" % (w, c, a))))
                        if not fs:
                            continue
                        wv, bt = collections.defaultdict(list), collections.defaultdict(list)
                        for f in fs:
                            p = prof(f)
                            for cls, v in p.get("thread_wait_ns", {}).items():
                                wv[cls].append(v / p["calls"] / 1e6)
                            for kk, v in p.get("batch_trace", {}).items():
                                if isinstance(v, (int, float)) and v >= 0:
                                    bt[kk].append(v / 1e6)
                        m = lambda k: ("%.3f" % statistics.median(wv[k])) if wv.get(k) else ""
                        b = lambda k: ("%.3f" % statistics.median(bt[k])) if bt.get(k) else "-"
                        say("| %s | %s | %s | %s | %s | %s | %s | %s | %s [%s-%s] | %s | %s | %s |" % (
                            wt, c, a, m("caller"), m("main"), m("tokio-rt-worker"), m("event_engine"), b("batch_wall_ns_median"),
                            b("call_ns_median"), b("call_ns_p10"), b("call_ns_p90"), b("calls_span_ns_median"),
                            b("dispatch_last_start_ns_median"), b("completion_lag_ns_median")))
        if any("thread_csw" in (prof(f) or {}) for f in glob.glob(os.path.join(out, phase, "*.out"))[:1]):
            say("")
            say("Context switches per call by thread class, voluntary / involuntary (median of the processes), and the "
                "server during the loop (AK_SERVER_PID: CPU per call, ms; its threads' voluntary switches are not read):")
            say("")
            say("| workload | cell | arm | caller | main | tokio-rt-worker | event_engine | server CPU |")
            say("|---|---|---|---|---|---|---|---|")
            for w, wt in WLS:
                for c in CELLS:
                    for a in arms:
                        fs = sorted(glob.glob(os.path.join(out, phase, "%s-%s-%s-r*.out" % (w, c, a))))
                        if not fs:
                            continue
                        cv = collections.defaultdict(list)
                        sv = []
                        for f in fs:
                            p = prof(f)
                            for cls, v in p.get("thread_csw", {}).items():
                                cv[cls].append((v[0] / p["calls"], v[1] / p["calls"]))
                            if p.get("server", {}).get("pid", 0) > 0:
                                sv.append(p["server"]["cpu_ns"] / p["calls"] / 1e6)
                        g = lambda k: ("%.1f / %.1f" % (statistics.median([x[0] for x in cv[k]]), statistics.median([x[1] for x in cv[k]]))) if cv.get(k) else ""
                        say("| %s | %s | %s | %s | %s | %s | %s | %s |" % (wt, c, a, g("caller"), g("main"), g("tokio-rt-worker"),
                                                                        g("event_engine"), ("%.3f" % statistics.median(sv)) if sv else "-"))
        say("")
        say("Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):")
        say("")
        say("| workload | cell | arm | " + " | ".join(THREADS) + " |")
        say("|---|---|---|" + "---|" * len(THREADS))
        for wt, c, a, d in thr:
            say("| %s | %s | %s | %s |" % (wt, c, a, " | ".join(("%.3f" % d[t]) if t in d else "" for t in THREADS)))
        say("")
        say("Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A "
            "process of the same arm and round):")
        say("")
        say("| workload | cell | arm | per round (CPU) | per round (wall) |")
        say("|---|---|---|---|---|")
        for w, wt in WLS:
            for c in [x for x in CELLS if x.startswith("Cf")]:
                for a in arms:
                    rs = sorted(r for (ww, cc, aa, r) in med if (ww, cc, aa) == (w, c, a) and (w, "A", a, r) in med)
                    if not rs:
                        continue
                    say("| %s | %s | %s | %s | %s |" % (wt, c, a,
                        " ".join("%+.3f" % (med[(w, c, a, r)][0] - med[(w, "A", a, r)][0]) for r in rs),
                        " ".join("%+.3f" % (med[(w, c, a, r)][1] - med[(w, "A", a, r)][1]) for r in rs)))
    if os.path.isdir(os.path.join(out, "strace")):
        say("")
        say("## strace: syscalls per call (between the loop's markers, fewer batches)")
        say("")
        say("| workload | cell | arm | calls | socket writes | bytes/write | socket reads | epoll_wait | futex | all syscalls |")
        say("|---|---|---|---|---|---|---|---|---|---|")
        for w, wt in WLS:
            for c in CELLS:
                for a in arms:
                    f = os.path.join(out, "strace", "%s-%s-%s.syscalls.txt" % (w, c, a))
                    if not os.path.exists(f):
                        continue
                    d = json.loads(open(f).read().splitlines()[1]); n = prof(f.replace(".syscalls.txt", ".out"))["calls"]
                    cnt, sock = d["counts"], d["to_socket"]
                    sw = sum(sock.get(x, 0) for x in ("write", "writev", "sendmsg", "sendto"))
                    sr = sum(sock.get(x, 0) for x in ("read", "readv", "recvmsg", "recvfrom"))
                    say("| %s | %s | %s | %d | %.1f | %.0f | %.1f | %.1f | %.1f | %.1f |" % (
                        wt, c, a, n, sw / n, d["socket_write_bytes"] / max(1, sw), sr / n,
                        sum(cnt.get(x, 0) for x in ("epoll_wait", "epoll_pwait", "epoll_pwait2")) / n,
                        cnt.get("futex", 0) / n, sum(cnt.values()) / n))
    if os.path.isdir(os.path.join(out, "perf")):
        perf_section(out, arms, say)
    print("\n".join(L))


def perfstat(path):
    d = {}
    for l in open(path):
        p = l.strip().split(",")
        if len(p) > 3 and p[0] and p[0][0].isdigit():
            d[p[2]] = float(p[0])
    return d


def perf_section(out, arms, say):
    """perf stat counters and perf record buckets per call (the split; absolutes are the no-perf
    phases'). Kernel symbols from the System.map in AK_SYSTEM_MAP (the booted kernel's)."""
    import subprocess
    smap = os.environ.get("AK_SYSTEM_MAP", "")
    GHZ = 3.3e9
    say("")
    say("## perf: the split (perf attached; absolutes above are the no-perf phases')")
    say("")
    say("perf stat per call (one process per cell, perf enabled around the loop): cycles and instructions user / kernel "
        "(M), cache-references and cache-misses (k), LLC-load-misses (k), faults, context switches.")
    say("")
    say("| workload | cell | arm | cycles u / k | instr u / k | cache-ref | cache-miss | LLC-load-miss | faults | csw |")
    say("|---|---|---|---|---|---|---|---|---|---|")
    att = {}
    for w, wt in WLS:
        for c in CELLS:
            for a in arms:
                base = os.path.join(out, "perf", "%s-%s-%s" % (w, c, a))
                if not os.path.exists(base + ".perfstat"):
                    continue
                p = prof(base + ".stat.out"); n = p["calls"]
                d = {k: v / n for k, v in perfstat(base + ".perfstat").items()}
                g = lambda k: d.get(k, float("nan"))
                say("| %s | %s | %s | %.2f / %.2f | %.2f / %.2f | %.0f | %.0f | %.0f | %.1f | %.1f |" % (
                    wt, c, a, g("cycles:u") / 1e6, g("cycles:k") / 1e6, g("instructions:u") / 1e6, g("instructions:k") / 1e6,
                    g("cache-references") / 1e3, g("cache-misses") / 1e3, g("LLC-load-misses") / 1e3, g("page-faults"), g("context-switches")))
                if os.path.exists(base + ".perfstat.server"):
                    sd = {k: v / n for k, v in perfstat(base + ".perfstat.server").items()}
                    h = lambda k: sd.get(k, float("nan"))
                    say("| %s | %s | %s (SERVER) | %.2f / %.2f | %.2f / %.2f | - | %.0f | - | - | %.1f |" % (
                        wt, c, a, h("cycles:u") / 1e6, h("cycles:k") / 1e6, h("instructions:u") / 1e6, h("instructions:k") / 1e6,
                        h("cache-misses") / 1e3, h("context-switches")))
                for suffix, key in (("", (w, c, a)), (".server", (w, c, a + " SERVER"))):
                    data = base + ".data" + suffix
                    jf = base + ".attrib%s.json" % suffix
                    if os.path.exists(data) and smap and (not os.path.exists(jf) or os.path.getmtime(jf) < os.path.getmtime(data)):
                        r = subprocess.run([sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), "perf_attrib.py"),
                                            smap, data, base + ".out", "--top", "25"], stdout=subprocess.PIPE, text=True)
                        open(jf, "w").write(r.stdout)
                    if os.path.exists(jf):
                        att[key] = json.load(open(jf))
    for (w, wt), side in [(x, sd) for x in WLS for sd in ("client", "server")]:
        keys = [(c, a) for c in CELLS for a in (arms if side == "client" else [x + " SERVER" for x in arms]) if (w, c, a) in att]
        if not keys:
            continue
        buckets = []
        for c, a in keys:
            for b in att[(w, c, a)]["buckets_cycles_per_call"]:
                if b not in buckets:
                    buckets.append(b)
        buckets.sort(key=lambda b: (b[0] != "k", b))
        ref = next(((c, a) for c, a in keys if c == "Cf-retain"), None)
        say("")
        say("### perf record buckets, %s, %s (ms per call = cycles / 3.3e9; one process per cell)" % (wt, "the server process" if side == "server" else "the client"))
        say("")
        say("| bucket | " + " | ".join("%s %s" % ka for ka in keys) + (" | " + " | ".join("%s - Cf" % c for c, a in keys if ref and (c, a) != ref) if ref else "") + " |")
        say("|---|" + "---|" * (len(keys) + (len(keys) - 1 if ref else 0)))
        gv = lambda ka, b: att[(w,) + ka]["buckets_cycles_per_call"].get(b, 0.0) / GHZ * 1e3
        for b in buckets + ["total"]:
            vals = [(att[(w,) + ka]["cycles_per_call"] / GHZ * 1e3 if b == "total" else gv(ka, b)) for ka in keys]
            row = "| %s | %s" % ("**total (sampled)**" if b == "total" else b, " | ".join("%.3f" % v for v in vals))
            if ref:
                r0 = vals[keys.index(ref)]
                row += " | " + " | ".join("%+.3f" % (v - r0) for ka, v in zip(keys, vals) if ka != ref)
            say(row + " |")
        say("")
        say("Threads (ms per call, sampled): " + "; ".join("%s %s: %s" % (c, a, ", ".join(
            "%s %.3f" % (t, v / GHZ * 1e3) for t, v in att[(w, c, a)]["threads_cycles_per_call"].items())) for c, a in keys))
        for c, a in keys:
            say("")
            say("Top symbols, %s %s %s (ms per call): " % (wt, c, a) + "; ".join(
                "%s %.3f" % (sname[:70], v / GHZ * 1e3) for sname, v in att[(w, c, a)]["top_symbols_cycles_per_call"][:16]))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])
