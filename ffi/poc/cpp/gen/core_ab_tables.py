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

CELLS = ["A", "D-retain", "Cf-retain", "Cf-q-retain", "Cf-enc-retain", "Cf-encp-retain"]
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
    say("# %s: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)" % os.path.basename(out))
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
        say("| workload | cell | arm | CPU ms | wall ms | flt | csw | n |")
        say("|---|---|---|---|---|---|---|---|")
        thr, med = [], {}
        for w, wt in WLS:
            for c in CELLS:
                for a in arms:
                    fs = sorted(glob.glob(os.path.join(out, phase, "%s-%s-%s-r*.out" % (w, c, a))))
                    if not fs:
                        continue
                    cpu, wall, flt, csw = [], [], [], []
                    per = collections.defaultdict(list)
                    for f in fs:
                        p = prof(f)
                        cc, ww = chunks(p)
                        cpu += cc; wall += ww
                        r = int(f.rsplit("-r", 1)[1].split(".")[0])
                        med[(w, c, a, r)] = (statistics.median(cc), statistics.median(ww))
                        ru = p["rusage"]
                        flt.append(ru["minflt"] / p["calls"]); csw.append((ru["nvcsw"] + ru["nivcsw"]) / p["calls"])
                        for cls, v in p["thread_cpu_ns"].items():
                            per[cls].append(v["ns"] / p["calls"] / 1e6)
                    say("| %s | %s | %s | %s | %s | %.1f | %.1f | %d |" % (wt, c, a, mpq(cpu), mpq(wall),
                                                                     statistics.median(flt), statistics.median(csw), len(cpu)))
                    if c.startswith("Cf"):
                        thr.append((wt, c, a, {t: statistics.median(per[t]) for t in THREADS if per.get(t)}))
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
    print("\n".join(L))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])
