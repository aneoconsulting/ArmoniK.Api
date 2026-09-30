#!/usr/bin/env python3
"""The k = 8 wall question: per cell and arm, from `campaign_rpc --profile` runs with the server's
threads sampled (AK_SERVER_PID) and the batch trace; absolute per-call figures, no ratio.

  wall_tables.py DIR [DIR ...]   (gen/core_ab.sh measure directories) > tables-wall.md

Per (workload, cell, arm): client CPU and wall per call (median of the chunks of every process);
the batch trace (batch wall; one call's duration median [p10-p90]; span; dispatch; completion);
client run-queue wait per call (callers, core workers, grpc-core); the server during the loop: CPU
per call, its busiest thread's share of the loop's wall, threads above 5 % of the wall, run-queue
wait per call.
"""
import collections
import glob
import json
import os
import statistics
import sys


def prof(path):
    for l in open(path):
        if l.startswith('{"profile"'):
            return json.loads(l)["profile"]
    return None


def med(v):
    return statistics.median(v) if v else float("nan")


def main(dirs):
    rows = collections.defaultdict(list)
    for d in dirs:
        for f in sorted(glob.glob(os.path.join(d, "measure", "*.out"))):
            base = os.path.basename(f)[:-4]
            name, rest = base.split("-", 1)
            cell, arm_r = rest.rsplit("-", 2)[0], rest.rsplit("-", 2)[1:]
            p = prof(f)
            if p:
                rows[(name, cell, arm_r[0])].append(p)
    print("# The k = 8 wall question: client, batch trace and server per call (ms), no perf")
    print("")
    print("| workload | cell | arm | CPU | wall | batch | call [p10-p90] | dispatch | wait: callers / workers / grpc | server CPU | server busiest thread share | server threads >5% | server wait | n |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    order = {"A": 0, "D-retain": 1, "Cf-retain": 2, "Cf-zc-retain": 3, "Cf-zcp-retain": 4}
    for key in sorted(rows, key=lambda k: (k[0] != "d16k8", k[0], k[2], order.get(k[1], 9))):
        ps = rows[key]
        cpu = [ch["cpu_ns"] / (ch["batches"] * p["k"]) / 1e6 for p in ps for ch in p["chunks"]]
        wall = [ch["wall_ns"] / (ch["batches"] * p["k"]) / 1e6 for p in ps for ch in p["chunks"]]
        bt = lambda k: med([p["batch_trace"][k] / 1e6 for p in ps if p.get("batch_trace", {}).get(k, -1) >= 0])
        tw = lambda c: med([p["thread_wait_ns"].get(c, 0) / p["calls"] / 1e6 for p in ps])
        sv = [p["server"] for p in ps if p.get("server", {}).get("pid", 0) > 0]
        print("| %s | %s | %s | %.3f | %.3f | %.2f | %.2f [%.2f-%.2f] | %.3f | %.3f / %.3f / %.3f | %.3f | %.2f | %.0f | %.3f | %d |" % (
            key[0], key[1], key[2], med(cpu), med(wall), bt("batch_wall_ns_median"), bt("call_ns_median"), bt("call_ns_p10"),
            bt("call_ns_p90"), bt("dispatch_last_start_ns_median"), tw("caller"), tw("tokio-rt-worker"), tw("event_engine"),
            med([s["cpu_ns"] / p["calls"] / 1e6 for s, p in zip(sv, ps)]),
            med([s["max_thread_ns"] / s["loop_wall_ns"] for s in sv]), med([s["threads_over_5pct"] for s in sv]),
            med([s["wait_ns"] / p["calls"] / 1e6 for s, p in zip(sv, ps)]), len(ps)))


if __name__ == "__main__":
    main(sys.argv[1:])
