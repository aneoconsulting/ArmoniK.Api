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
                    log.sample(cell=cell, payload=pid, dir=d, transport=transport, inflight=int(k),
                               unknown_mode=unknown_mode(cell), send_path=send_path(cell),
                               launch=launch, round=rnd if phase == "value" else None, phase=phase,
                               cpu_ns=int(round(r["cpu_s"] * 1e9)), wall_ns=int(round(r["wall_s"] * 1e9)),
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
               clock="the time_func returns CLOCK_PROCESS_CPUTIME_ID of the worker per value (req 21); wall "
                     "(perf_counter) of the same batches from the side file",
               worker_threads="per benchmark worker: a client pool of k threads; one core runtime of "
                              "AK_CORE_WORKERS (%s) workers where the cell uses the core; grpcio's own threads "
                              "where it uses grpcio (a D or F stream adds one per call); the server's tokio runtime, "
                              "AK_SERVER_THREADS workers (default 4)"
                              % os.environ.get("AK_CORE_WORKERS", "2"),
               order="blocks of (transport, direction, payload, k) in list order, the cells rotated by one per "
                     "launch inside a block, the whole list rotated by a third per launch (pyperf cannot interleave)",
               checks="every call checked (req 18); a failed check fails the worker and the run, and the runner "
                      "then discards the whole launch's output (every build and group of that launch)",
               raw_json=os.path.basename(opt("--json")))
    if bad:
        log.close(False, "%d benchmark(s) whose side records do not match pyperf's values: %s (harness defect)"
                  % (len(set(bad)), ", ".join(sorted(set(bad))[:5])))
        return 1
    log.close(True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
