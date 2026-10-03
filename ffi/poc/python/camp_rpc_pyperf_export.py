"""The RPC grid's pyperf output (camp_rpc_pyperf.py) as the JSON lines of CAMPAIGN.md section 7.

  python3.12 camp_rpc_pyperf_export.py --json FILE.json --side DIR --launch N --variant full|nounk
      --group ab|c|d --server SOCKETS_LINE --pyperf-args "..." --out FILE.jsonl [--smoke] [--allow-dirty]

Every raw measurement is exported, warm-ups included (`phase` "warmup"), each with the labels
the hand-written sampler wrote: cell, payload, dir, transport, inflight, unknown_mode, build,
send_path, launch, round. cpu_ns and wall_ns come from the worker's side file (the time_func's
own CLOCK_PROCESS_CPUTIME_ID and perf_counter around the batches), matched to pyperf's values
in order: one worker per benchmark, no calibration run (--loops fixed), so the i-th side record
is the i-th measurement. A count mismatch stops the export (a harness defect). iters = loops x k.
"""
import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "build", "pyperf"))
import pyperf  # noqa: E402
import camp_lib as L  # noqa: E402
from camp_rpc_pyperf import send_path  # noqa: E402

A = sys.argv[1:]


def opt(n, d=None):
    return A[A.index(n) + 1] if n in A else d


def unknown_mode(cell):
    if cell.endswith("-nounk"):
        return "no-unknown"
    if cell.endswith("-retain"):
        return "retain"
    if cell.endswith("-drop") or cell.startswith("C-"):
        return "drop"
    return "incumbent-default"


def main():
    launch = int(opt("--launch", "1"))
    variant = opt("--variant", "full")
    side = {}
    for p in sorted(glob.glob(os.path.join(opt("--side"), "side-*.jsonl"))):
        for ln in open(p):
            r = json.loads(ln)
            side.setdefault(r["name"], []).append(r)
    suite = pyperf.BenchmarkSuite.load(opt("--json"))
    log = L.Log(opt("--out"), "rpc", allow_dirty="--allow-dirty" in A, smoke="--smoke" in A,
                build="nounk" if variant == "nounk" else "full")
    bad = []
    facts = {}
    h2 = opt("--h2", "stock")
    nd_tot = [0, 0]
    for b in suite.get_benchmarks():
        name = b.get_name()
        _, build, transport, d, pid, cell, k = name.split("|")
        recs = side.get(name, [])
        i = 0
        for run in b._runs:
            md = run.get_metadata()
            loops = md.get("loops", 1)
            rnd = 0
            for phase, vals in (("warmup", [v for _l, v in (run.warmups or ())]), ("value", list(run.values))):
                for _v in vals:
                    if i >= len(recs):
                        bad.append(name)
                        break
                    r = recs[i]
                    i += 1
                    if phase == "value":
                        rnd += 1
                    if "facts" in r:
                        facts[name] = r["facts"]
                        if r["facts"].get("h2") != h2:
                            bad.append(name + " (h2 %s, not %s)" % (r["facts"].get("h2"), h2))
                    nd_tot[0] += r["nodelay"]["nodelay_on"]
                    nd_tot[1] += r["nodelay"]["to_server"]
                    q = r["client_cpus_irq"]
                    log.sample(cell=cell, payload=pid, dir=d, transport=transport, inflight=int(k),
                               unknown_mode=unknown_mode(cell), send_path=send_path(cell), h2=h2,
                               launch=launch, round=rnd if phase == "value" else None, phase=phase,
                               cpu_ns=int(round(r["task_clock_s"] * 1e9)),
                               process_cpu_ns=int(round(r["cpu_s"] * 1e9)), wall_ns=int(round(r["wall_s"] * 1e9)),
                               client_softirq_ticks=q["softirq_ticks"], client_irq_ticks=q["irq_ticks"],
                               client_net_rx=q["net_rx"], client_net_tx=q["net_tx"],
                               iters=r["loops"] * int(k))
            del loops
        if i != len(recs):
            bad.append(name)
    log.header(engine="pyperf %s (CAMPAIGN req 22a as amended: the RPC grid on the codec suite's framework)"
               % pyperf.__version__, launch=launch, group=opt("--group"), pyperf_args=opt("--pyperf-args", "?"),
               server="the Rust slice's tonic rpc_server (FIX-PLAN WP10, req 13 as amended at 9f6d579fa; "
                      "poc/rust/SERVER.md), one process per launch started by poc/rust/serve.sh of the snapshot, "
                      "pinned to AK_CPU_SERVER=%s, tokio workers AK_SERVER_THREADS=%s; sockets %s; shipped = tonic's "
                      "server defaults, pinned = stream and connection windows 4 MiB, adaptive window off; receive "
                      "limit 8 MiB; the client's shipped / pinned configuration dials the socket of the same name"
                      % (os.environ.get("AK_CPU_SERVER", "unset"), os.environ.get("AK_SERVER_THREADS", "4"),
                         opt("--server", "?")),
               server_log=" | ".join(ln.strip() for ln in open(opt("--server-log"))) if opt("--server-log") else "not given",
               methods="/armonik.ffi.campaign.v1.Grid/ Fetch (a), Push (b), Upload (c), UploadStream (d, timed: the "
                       "server's byte count checked on every call), UploadStreamCheck (d, untimed: count and SHA-256, "
                       "once per cell in the precheck and in each worker's setup)",
               grpcio_channel="A: the generated GridStub (grpc_tools, proto/campaign_grid.proto; registered methods); "
                              "D and F: unary_unary / stream_unary with _registered_method=True, as the stub makes; "
                              "both configurations set grpc.default_authority=localhost (the server's h2 refuses "
                              "grpcio's percent-encoded unix-socket authority)",
               framework_forces="a worker PROCESS per benchmark (one cell, direction, payload, in-flight level): "
                                "each opens its own channel and its own pool of k threads in its setup, where the "
                                "hand-written sampler used one process per build and launch; one loop is one batch "
                                "of k calls in flight (inner_loops = k), --loops fixed per group (no calibration "
                                "worker); warm-ups are pyperf's per-worker warm-up values (--warmups), replacing "
                                "the per-cell warm-up; the order is pyperf's sequential order, rotated per launch",
               clock="the time_func returns perf task-clock of the worker process per value (req 21 as amended, "
                     "WP13); the process clock and wall (perf_counter) of the same batches from the side file",
               worker_threads="per benchmark worker: a client pool of k threads; one core runtime of "
                              "%s workers where the cell uses the core; grpc-core's threads where it uses grpcio (a D "
                              "or F stream adds one per call); the server's tokio runtime, AK_SERVER_THREADS=%s workers"
                              % (os.environ.get("AK_CORE_WORKERS", os.environ.get("AK_WORKERS", "8")),
                                 os.environ.get("AK_SERVER_THREADS", "4")),
               order="blocks of (transport, direction, payload, k) in list order, the cells rotated by one per "
                     "launch inside a block, the whole list rotated by a third per launch (pyperf cannot interleave)",
               checks="every call checked (req 18); a failed check fails the worker and the run, and the runner "
                      "then discards the whole launch's output (every build and group of that launch)",
               raw_json=os.path.basename(opt("--json")),
               h2="%s (D11 as amended; every sample carries `h2`); the core each worker mapped: %s" % (
                   h2, json.dumps(sorted({json.dumps(f.get("core")) for f in facts.values()}))),
               transport_net="TCP 127.0.0.1 for every cell (WP13, D10, req 17 as amended): every client configuration "
                             "dials the shared server's TCP listener, which runs the PINNED server configuration only, so "
                             "`shipped` and `pinned` differ on the client side only (grpcio channel options; core client "
                             "ak_client_new against ak_client_opts with tcp_nagle 0). TCP_NODELAY read back with getsockopt "
                             "on every live socket to the server after the setup and after every value: %d of %d set"
                             % (nd_tot[0], nd_tot[1]),
               cpu_figure="cpu_ns = perf task-clock of the whole worker process (perf_event_open PERF_COUNT_SW_TASK_CLOCK, "
                          "opened by the process before its first thread, inherited by every thread), pyperf's value; "
                          "process_cpu_ns = CLOCK_PROCESS_CPUTIME_ID beside it; client_softirq_ticks / client_irq_ticks "
                          "(/proc/stat, USER_HZ) and client_net_rx / client_net_tx (/proc/softirqs) over the worker's "
                          "affinity set (the CLIENT CPUs), around the same batches (req 21 as amended)",
               pools="AK_WORKERS=%s (D14). Core runtime: ak_runtime_new(%s) where the cell uses the core. grpc-core: sized "
                     "from sysconf(_SC_NPROCESSORS_CONF), not the affinity mask; the runner preloads build/ncpus_shim.so "
                     "with AK_SHIM_NCPUS=AK_WORKERS so it reads %s (its EventEngine reserves Clamp(n, 4, 16) threads). "
                     "grpcio's Python client has no executor of its own (a ThreadPoolExecutor exists only on servers). "
                     "The k client threads are the benchmark's pool. Per worker, the CPU facts and threads by name: %s" % (
                         os.environ.get("AK_WORKERS", "8"), os.environ.get("AK_CORE_WORKERS", os.environ.get("AK_WORKERS", "8")),
                         os.environ.get("AK_SHIM_NCPUS", "unset"),
                         json.dumps(sorted({json.dumps({"cpu": f.get("cpu"), "threads": f.get("threads")}, sort_keys=True)
                                            for f in facts.values()})[:6])),
               allocator="mallopt(M_TOP_PAD, 8 MiB) at import (allocator.py, req 25); GLIBC_TUNABLES=%s (D9: see STATE)"
                         % (os.environ.get("GLIBC_TUNABLES") or "unset"))
    if nd_tot[1] == 0 or nd_tot[0] != nd_tot[1]:
        bad.append("TCP_NODELAY %d of %d" % tuple(nd_tot))
    if bad:
        log.close(False, "%d benchmark(s) whose side records do not match pyperf's values: %s (harness defect)"
                  % (len(set(bad)), ", ".join(sorted(set(bad))[:5])))
        return 1
    log.close(True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
