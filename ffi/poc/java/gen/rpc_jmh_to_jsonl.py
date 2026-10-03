#!/usr/bin/env python3
"""JMH's JSON result for ak.RpcJmh -> the section 7 JSON lines (design/CAMPAIGN.md req 22a,
28; FIX-PLAN WP9 item 5).

Every raw measurement iteration JMH recorded (primaryMetric.rawData, one list per fork) becomes
one sample line; nothing is summarised or dropped. Per iteration, from JMH's JSON:
  wall_ns = the primary score (ns per invocation, one invocation = one batch of k calls) x the
            iteration's invocations (callsMade / k);
  cpu_ns  = the rpcTaskClockNs aux counter: perf task-clock of the whole process, softirq
            included (CAMPAIGN req 21 as amended 2026-10-01), summed over the invocations;
  process_cpu_ns = the rpcCpuNs aux counter (CLOCK_PROCESS_CPUTIME_ID), beside it;
  softirq_ticks_client = softirq time on the CLIENT CPUs over the iteration (USER_HZ ticks);
  iters   = the callsMade aux counter (k per invocation).
The `calls` aux counter (OPERATIONS: JMH's time per call) is checked against wall_ns / iters,
so a batch counted as anything but k operations is refused. The labels come from the forked
JVM's output (`<jmh.txt>`): one RPCJMH-CELL line per cell (mode, codec, send path, kind,
transport, build, threads) and one RPCJMH-ITER line per iteration (its combination); a
measurement iteration without its label, or a label without its iteration, is refused.

  gen/rpc_jmh_to_jsonl.py <jmh.json> <jmh.txt> <launch> >> <log.jsonl>
"""
import json
import sys


def main():
    res = json.load(open(sys.argv[1]))
    launch = int(sys.argv[3])
    cells, iters = {}, {}
    for line in open(sys.argv[2], encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        # JMH prints a forked JVM's output where it stands, e.g. after "# Warmup Iteration 1: ".
        at = line.find("RPCJMH-")
        if at < 0:
            continue
        line = line[at:]
        if line.startswith("RPCJMH-CELL\t"):
            f = line.split("\t")
            cells[f[1]] = {"mode": f[2], "codec": f[3], "send_path": f[4], "kind": f[5],
                           "transport": f[6], "build": f[7], "threads": json.loads(f[8]),
                           "h2": f[9], "nodelay": f[10]}
        elif line.startswith("RPCJMH-ITER\t"):
            f = line.split("\t")
            if f[2] == "m":
                iters[(f[1], int(f[3]))] = {"key": f[4], "dir": f[5], "payload": f[6], "k": int(f[7])}
    out = []
    for b in res:
        cell = b["params"]["cell"]
        combo = b["params"].get("combo", "cycle")
        ncombo = 17 if combo == "cycle" else 1   # rounds: a cycle visits 17 per round
        if cell not in cells:
            raise SystemExit("no RPCJMH-CELL line for %s" % cell)
        cl = cells[cell]
        pm, sm = b["primaryMetric"], b.get("secondaryMetrics", {})
        if pm.get("scoreUnit") != "ns/op":
            raise SystemExit("unexpected JMH unit %r for %s" % (pm.get("scoreUnit"), cell))
        for name in ("rpcCpuNs", "callsMade", "calls", "rpcTaskClockNs", "softirqTicks"):
            if name not in sm:
                raise SystemExit("no %s counter for %s" % (name, cell))
        out.append(json.dumps({"meta": {"suite": "rpc", "engine": "jmh " + b.get("jmhVersion", "?"),
            "cell": cell, "combo": combo, "grouped": combo == "cycle", "mode": b["mode"], "threads_benchmark": b.get("threads"), "forks": b["forks"],
            "warmup_iterations": b["warmupIterations"], "warmup_time": b.get("warmupTime"),
            "measurement_iterations": b["measurementIterations"], "measurement_time": b.get("measurementTime"),
            "jdk": b.get("jdkVersion"), "vm": b.get("vmName", "") + " " + b.get("vmVersion", ""),
            "jvm_args": b.get("jvmArgs"), "threads": cl["threads"], "h2": cl["h2"],
            "tcp_nodelay_read_back": cl["nodelay"],
            "cpu_ns": "perf task-clock of the whole process (JVM agent, inherited counter), per invocation summed; process_cpu_ns: CLOCK_PROCESS_CPUTIME_ID beside it; softirq_ticks_client: /proc/stat softirq on the CLIENT CPUs over the iteration, USER_HZ ticks", "combinations": ncombo,
            "order": ("one fork per cell (grouped, AK_RPC_GROUP=1), cells in the order given (rotated per launch); inside the fork JMH iteration i runs combination (i + launch - 1) mod 17, warm-up and measurement counted separately" if combo == "cycle" else "one fork per (cell, combination), JMH's order of the cross product, cells rotated per launch"),
            "ratio_basis": "per-launch medians (CAMPAIGN req 30, R-H24)"}}, separators=(",", ":")))
        seen = 0
        for f, fork in enumerate(pm["rawData"]):
            for i, score in enumerate(fork):
                lab = iters.get((cell + "|" + combo, i))
                if lab is None:
                    raise SystemExit("no RPCJMH-ITER label for %s measurement %d" % (cell, i))
                try:
                    cpu = sm["rpcCpuNs"]["rawData"][f][i]
                    made = sm["callsMade"]["rawData"][f][i]
                    per_call = sm["calls"]["rawData"][f][i]
                    tclock = sm["rpcTaskClockNs"]["rawData"][f][i]
                    irq = sm["softirqTicks"]["rawData"][f][i]
                except (IndexError, KeyError):
                    raise SystemExit("no counter sample for %s measurement %d" % (cell, i))
                k = lab["k"]
                made = int(round(made))
                if made <= 0 or made % k:
                    raise SystemExit("%s measurement %d: %d calls is not a multiple of k=%d" % (cell, i, made, k))
                wall = score * (made // k)
                if abs(per_call * made - wall) > 1e-6 * wall + 1:
                    raise SystemExit("%s measurement %d: JMH's time per call %.3f x %d calls != %.0f ns"
                                     % (cell, i, per_call, made, wall))
                d = lab["dir"]
                if cl["kind"] == "core":
                    delivery = "blocking client stream" if d == "d" else "blocking"
                else:
                    delivery = "grpc-java asyncClientStreamingCall" if d == "d" else "grpc-java blockingUnaryCall"
                rec = {"slice": "java", "suite": "rpc", "cell": cell, "payload": lab["payload"],
                       "unknown_mode": cl["mode"], "build": cl["build"], "codec": cl["codec"],
                       "dir": d, "transport": cl["transport"], "inflight": k, "delivery": delivery,
                       "send_path": cl["send_path"], "engine": "jmh", "launch": launch,
                       "round": i // ncombo + 1, "cpu_ns": int(round(tclock)), "process_cpu_ns": int(round(cpu)),
                       "wall_ns": int(round(wall)), "softirq_ticks_client": int(round(irq)),
                       "h2": cl["h2"], "net": "tcp" if cl["nodelay"] != "uds" else "uds",
                       "iters": made}
                out.append(json.dumps(rec, separators=(",", ":")))
                seen += 1
        nlab = sum(1 for (c, _) in iters if c == cell + "|" + combo)
        if seen != nlab:
            raise SystemExit("%s: %d measurement iterations in JMH's JSON, %d labelled" % (cell, seen, nlab))
    print("\n".join(out))


if __name__ == "__main__":
    main()
