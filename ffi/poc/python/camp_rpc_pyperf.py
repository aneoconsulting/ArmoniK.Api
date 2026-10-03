"""CAMPAIGN req 22a as amended (owner, 2026-09-27; FIX-PLAN WP9): the RPC grid on pyperf, the
codec suite's framework. It replaces camp_rpc.py's hand-written sampler; the cells, their
channels, payloads and checks are still built by camp_rpc.py's `cells()`.

  python3.12 camp_rpc_pyperf.py --variant full|nounk --group ab|c|d --launch N
      --server shipped=unix:...,pinned=unix:... --side DIR [--transports shipped,pinned]
      [--only NAME,...] -o FILE.json --processes 1 --values ROUNDS --warmups W --loops L
      --affinity CPUS --copy-env          (run_campaign.sh builds this command)
  python3.12 camp_rpc_pyperf.py --precheck --variant V --server ...   (the runner's precheck)

pyperf's model, mapped onto the contract:
  benchmark   `rpc|build|transport|dir|payload|cell|k`: one cell, direction, payload and in-flight
              level. pyperf runs each in a worker process of its own: the worker opens that cell's
              channel (a grpcio channel, or a core client) in its setup, the first call of the
              time_func, before any timed loop (one channel per cell per benchmark process)
  invocation  one loop = one batch of k calls in flight (k threads of one pool created in the
              setup, each making one call), counted as k operations (inner_loops = k); `--loops`
              is fixed per group, so no calibration worker is spawned
  groups      one pyperf invocation per build and direction group: ab (a, a+read, b), c (the unary
              upload), d (the streamed upload), each with its own --loops, so the uploads do not
              run as many batches as the small calls; within an invocation the benchmark order is
              rotated by a third per launch and the cells of one (transport, dir, payload, k) block
              by one per launch (pyperf cannot interleave, req 22)
  clock       the time_func returns CLOCK_PROCESS_CPUTIME_ID seconds of the worker (req 21);
              wall (perf_counter) is written to a side file and joined back by the exporter
  checks      every call is checked (req 18: status, length, the server's count and digest for d);
              a failed check raises, the worker fails, pyperf fails the run, and the runner
              discards the whole launch's output (no sample). The worker also fails if a retain
              decode leaves an undelivered buffer, or if its contexts are not per thread
Server (WP10): the Rust rpc_server (poc/rust/SERVER.md). The runner starts the launch's ONE through
poc/rust/serve.sh and warms it (serve.sh warm) before any invocation. Without --server (the gate's
must-fail controls) this script starts its own through serve.sh and warms it with one call per
direction, and with --only it skips the grid precheck, so the named benchmark's own checks meet
the (client-side, AK_CAMP_PLANT) fault.
"""
import gc
import json
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "build", "pyperf"))

import pyperf  # noqa: E402
import camp_meas as M  # noqa: E402

A = sys.argv
# D9 as amended: the allocator mode a pass runs in (AK_CAMPAIGN_ALLOC, passed to the worker as
# --alloc). `default`: glibc's defaults (the main figures); `pinned`: the labelled diagnostic,
# D9's tunables in the environment. A worker checks both its environment and a readback (one
# 16 MiB malloc, mallinfo2's mmapped-block count) at import, before anything is timed, and
# refuses to run (no sample) on a mismatch.
# Req 21 as amended (WP13): the RPC client's CPU figure is perf task-clock of the whole process.
# The counter is opened here, at import, before this process starts any thread, so it inherits
# into every thread the worker creates (pool, core runtime, grpc-core).
M.task_clock_open()


def arg(n, d=None):
    return A[A.index(n) + 1] if n in A else d


ALLOC_RB = M.alloc_check(arg("--alloc", "default")) if "--worker" in A else None


VARIANT = arg("--variant", "full")
NOUNK = VARIANT == "nounk"
INFLIGHT_AB = [1, 8, 16]
INFLIGHT_UP = [1, 8]


def grid_names(nounk):
    """{key: [cell, ...]}: the cells camp_rpc.cells() builds, in its order (a worker checks
    that its cell is among what cells() built)."""
    mc = ["nounk"] if nounk else ["retain", "drop"]
    fam_up = ["A", "B", "Bf"]
    for m in mc:
        fam_up += ["C-" + m, "Cf-" + m, "D-" + m, "Cc-" + m]
    for m in mc:
        fam_up += ["E-" + m, "Ef-" + m, "F-" + m]
    a = ["A", "B"] + [x for m in mc for x in ("C-" + m, "D-" + m)] + [x for m in mc for x in ("E-" + m, "F-" + m)]
    b = ["A", "B"] + [x for m in mc for x in ("C-" + m, "D-" + m, "Cc-" + m)] + \
        [x for m in mc for x in ("E-" + m, "F-" + m)] + ["Bf"] + ["Cf-" + m for m in mc] + ["Ef-" + m for m in mc]
    out = {"a": a + ([] if nounk else ["B-queue", "C-queue", "B-callback", "C-callback"]), "a+read": list(a), "b": b}
    for pid in ("P5.3", "P5.4"):
        out["c:" + pid] = list(fam_up)
    for label in ("4MiB", "16MiB"):
        out["d:" + label] = list(fam_up)
    return out


def send_path(cell):
    if cell.startswith(("Bf", "Cf-", "Ef-")):
        return "framed"
    if cell.startswith("Cc-"):
        return "copy"
    if cell.startswith("C-"):
        return "move"
    if cell.startswith(("A", "D-", "F-")):
        return "grpcio"
    return "reference"


def names_of(group, transports, launch):
    g = grid_names(NOUNK)
    keys = {"ab": ["a", "a+read", "b"], "c": [k for k in g if k.startswith("c:")],
            "d": [k for k in g if k.startswith("d:")]}[group]
    out = []
    for t in transports:
        for key in keys:
            d, pid = (key.split(":") + ["P2.2"])[:2]
            for k in (INFLIGHT_AB if group == "ab" else INFLIGHT_UP):
                cl = list(g[key])
                r = (launch - 1) % len(cl)
                for cell in cl[r:] + cl[:r]:
                    out.append("rpc|%s|%s|%s|%s|%s|%d" % (VARIANT, t, d, pid, cell, k))
    n = max(1, len(out) // 3)
    s = ((launch - 1) * n) % len(out) if out else 0
    return out[s:] + out[:s]


_W = {}


def setup(name):
    """The worker's setup, once, before any timed loop: the shared camp_rpc module (pinning,
    the shim of this build), this benchmark's cell and channel, its correctness check, and
    the pool of k client threads."""
    if _W:
        return _W
    if arg("--h2"):
        os.environ["AK_H2"] = arg("--h2")       # the h2 variant's shims (arms.py), before camp_rpc
    alloc = arg("--alloc", "default")
    if M.task_clock() is None:
        raise SystemExit("benchmark %s: perf_event_open refused the task-clock counter (%s); req 21 as "
                         "amended needs it" % (name, M.task_clock_refusal()))
    if arg("--plant"):
        # pyperf gives a worker a clean environment unless --copy-env; the controls' plant is
        # passed as an argument so it reaches the worker whatever the pyperf options
        os.environ["AK_CAMP_PLANT"] = arg("--plant")
    import camp_rpc as C
    _, build, transport, d, pid, cell, k = name.split("|")
    key = d if d in ("a", "a+read", "b") else "%s:%s" % (d, pid)
    srv = C.parse_server(arg("--server"))
    cs, keep = C.cells(C.timed_target(srv), transport, keys={key})
    fns = dict(cs[key])
    if cell not in fns:
        raise SystemExit("benchmark %s: cells() built no cell %s for %s" % (name, cell, key))
    fn = fns[cell]
    # correctness before timing (req 26, in the worker): one call; for (a) the decoded object
    # re-encodes to P2.2 (the per-cell check camp_rpc.gate made)
    try:
        if d == "d":
            fn(check=True)       # UploadStreamCheck: the server's count and digest (untimed)
        o = fn()
    except Exception as e:
        raise SystemExit("benchmark %s: the setup's checked call failed: %s: %s" % (name, type(e).__name__, str(e)[:200]))
    if d == "a" and not cell.endswith(("-queue", "-callback")):
        R = C.arms._pb_root(C.PID)
        ref = C.arms.reference(C.PID)
        back = (o.SerializeToString(deterministic=True) if isinstance(o, R)
                else C.arms._ffi.encode("cext", C.arms.ROOT_OF[C.PID], o, None, cell.endswith("-retain")))
        if back != ref and R.FromString(back) != R.FromString(ref):
            raise SystemExit("benchmark %s: cell %s (a) does not re-encode to P2.2" % (name, cell))
    port = C.tcp_port(srv)
    nd = M.nodelay_summary(port)
    if not nd["to_server"] or nd["nodelay_on"] != nd["to_server"]:
        raise SystemExit("benchmark %s: TCP_NODELAY read back on %d of %d live socket(s) to the server (req 17)"
                         % (name, nd["nodelay_on"], nd["to_server"]))
    pool = C.Pool(int(k))
    facts = {"h2": C.arms.H2, "allocator": alloc, "alloc_readback": ALLOC_RB, "glibc_tunables": os.environ.get("GLIBC_TUNABLES"), "core": loaded_core(), "core_workers": C.CORE_WORKERS if C.RT else None,
             "workers": C.WORKERS, "cpu": M.cpu_facts(), "nodelay_setup": nd}
    _W.update(C=C, fn=fn, k=int(k), keep=keep, pool=pool, u0=C.arms._ffi.unk_totals(),
              t0=C.arms._ffi.tls_created(), port=port, cpus=set(os.sched_getaffinity(0)), facts=facts, first=True)
    return _W


def loaded_core():
    """The libak_core.so this process mapped, and its sha256 (the h2 variant actually running)."""
    import hashlib
    with open("/proc/self/maps") as f:
        paths = sorted({ln.split()[-1] for ln in f if ln.rstrip().endswith("libak_core.so")})
    return [{"path": p, "sha256": hashlib.sha256(open(p, "rb").read()).hexdigest()[:16]} for p in paths]


def time_func(loops, name, side):
    w = setup(name)
    C, fn, k, pool = w["C"], w["fn"], w["k"], w["pool"]
    failed = []
    stop = __import__("threading").Event()
    gc.collect()
    f0 = __import__("resource").getrusage(0).ru_minflt      # RUSAGE_SELF: every thread
    q0 = M.irq_snap(w["cpus"])
    c0, t0, w0 = M.task_clock(), time.clock_gettime(time.CLOCK_PROCESS_CPUTIME_ID), time.perf_counter()
    for _ in range(loops):
        pool.run(k, fn, 1, stop, failed)      # one batch: k calls in flight, one per thread
        if failed:
            break
    t1, w1, c1 = time.clock_gettime(time.CLOCK_PROCESS_CPUTIME_ID), time.perf_counter(), M.task_clock()
    q1 = M.irq_snap(w["cpus"])
    f1 = __import__("resource").getrusage(0).ru_minflt
    if failed:
        e = failed[0]
        raise SystemExit("benchmark %s: a call failed: %s: %s" % (name, type(e).__name__, str(e)[:200]))
    u = C.arms._ffi.unk_totals()
    if u[2] != w["u0"][2]:
        raise SystemExit("benchmark %s: %d unknown-field buffer(s) left undelivered" % (name, u[2] - w["u0"][2]))
    if C.arms._ffi.tls_created() - w["t0"] > 2 * k + 64 + loops * 4 * k:
        raise SystemExit("benchmark %s: contexts are not per thread" % name)
    nd = M.nodelay_summary(w["port"])
    if not nd["to_server"] or nd["nodelay_on"] != nd["to_server"]:
        raise SystemExit("benchmark %s: TCP_NODELAY read back on %d of %d live socket(s) to the server (req 17)"
                         % (name, nd["nodelay_on"], nd["to_server"]))
    rec = {"name": name, "loops": loops, "task_clock_s": (c1 - c0) / 1e9, "cpu_s": t1 - t0, "wall_s": w1 - w0,
           "client_cpus_irq": M.irq_delta(q0, q1), "nodelay": nd, "minflt": f1 - f0}
    if w["first"]:
        w["first"] = False
        rec["facts"] = dict(w["facts"], threads=M.thread_classes())
    with open(os.path.join(side, "side-%d.jsonl" % os.getpid()), "a") as f:
        f.write(json.dumps(rec) + "\n")
    return (c1 - c0) / 1e9                    # req 21 as amended: task-clock is pyperf's value


def add_args(cmd, args):
    cmd.extend(["--variant", VARIANT, "--group", args.group, "--launch", str(args.launch),
                "--server", args.server, "--side", args.side, "--transports", args.transports])
    if args.only:
        cmd.extend(["--only", args.only])
    if os.environ.get("AK_CAMP_PLANT"):
        cmd.extend(["--plant", os.environ["AK_CAMP_PLANT"]])
    cmd.extend(["--h2", os.environ.get("AK_H2", "stock"), "--alloc", os.environ.get("AK_CAMPAIGN_ALLOC", "default")])


def precheck_main():
    """Before the build's invocations, in one process of that build (req 26): every cell of
    every direction built and called once on each client configuration over TCP, with the checks
    camp_rpc.gate makes (re-encodings, the requests' bytes, the retain or no-unknown control);
    TCP_NODELAY read back on every live socket to the server; and the h2 variant's write-count
    marker: the process's write syscalls per (d)/16MiB call on the framed core cell at k = 1
    (stock h2 writes one 16 KiB DATA frame per syscall, about 1,030 per call; h2-batch coalesces,
    about 75: poc/codec/h2-batch, WP12). A count on the wrong side of 400 fails: the variant
    the label names is not the one running."""
    if arg("--h2"):
        os.environ["AK_H2"] = arg("--h2")
    import camp_rpc as C
    srv = C.parse_server(arg("--server"))
    port = C.tcp_port(srv)
    h2 = C.arms.H2
    print("   precheck %s, h2 %s, core %s" % (VARIANT, h2, json.dumps(loaded_core())))
    for t in arg("--transports", "shipped,pinned").split(","):
        cs, keep = C.cells(C.timed_target(srv), t)
        print("   precheck %s %s %s: %s" % (VARIANT, h2, t, C.gate(cs)))
        nd = M.nodelay_summary(port)
        if not nd["to_server"] or nd["nodelay_on"] != nd["to_server"]:
            raise SystemExit("precheck: TCP_NODELAY read back on %d of %d live socket(s) to the server"
                             % (nd["nodelay_on"], nd["to_server"]))
        print("   precheck %s %s %s: TCP_NODELAY read back on %d of %d live sockets to 127.0.0.1:%d"
              % (VARIANT, h2, t, nd["nodelay_on"], nd["to_server"], port))
        cell = "Cf-" + ("nounk" if NOUNK else "drop")
        fn = dict(cs["d:16MiB"])[cell]
        fn()

        def syscw():
            with open("/proc/self/io") as f:
                return int([ln for ln in f if ln.startswith("syscw:")][0].split()[1])
        w0 = syscw()
        for _ in range(3):
            fn()
        per = (syscw() - w0) / 3.0
        ok = per > 400 if h2 == "stock" else per < 400
        print("   precheck %s %s %s: write syscalls per d/16MiB call on %s at k = 1: %.0f (%s, want %s 400)"
              % (VARIANT, h2, t, cell, per, "ok" if ok else "WRONG VARIANT", ">" if h2 == "stock" else "<"))
        if not ok:
            raise SystemExit("precheck: the write count %.0f does not match h2 %s" % (per, h2))
        del keep
    return 0


def main():
    if "--precheck" in A:
        return precheck_main()
    own = None
    if not arg("--server") and "--worker" not in A:
        # the gate's must-fail controls: this process's own start of the shared server
        # (serve.sh start, then warm 1), stopped at the end
        import tempfile
        import subprocess
        sys.path.insert(0, HERE)
        from camp_rpc_srv import Server
        own = Server(tempfile.mkdtemp(prefix="akrpcpp"), warm=1)
        socks = "shipped=%s,pinned=%s,tcp=%s" % (own.info["shipped"], own.info["pinned"], own.info["tcp"])
        A.extend(["--server", socks])
        # with --only (the controls: named benchmarks), the grid precheck is skipped and each
        # worker's own setup check is what stands before its timed loop, so a planted fault
        # is caught by the benchmark's per-call checks and not before them
        if not arg("--only"):
            r = subprocess.run([sys.executable, __file__, "--precheck", "--server", socks, "--variant", VARIANT,
                                "--transports", arg("--transports", "shipped,pinned"),
                                "--h2", os.environ.get("AK_H2", "stock")])
            if r.returncode:
                own.stop()
                return 1
    runner = pyperf.Runner(add_cmdline_args=add_args)
    ap = runner.argparser
    ap.add_argument("--variant", default="full", choices=["full", "nounk"])
    ap.add_argument("--group", default="ab", choices=["ab", "c", "d"])
    ap.add_argument("--launch", type=int, default=1)
    ap.add_argument("--server", required=True)
    ap.add_argument("--side", required=True)
    ap.add_argument("--transports", default="shipped,pinned")
    ap.add_argument("--only", default="")
    ap.add_argument("--plant", default="")
    ap.add_argument("--h2", default="stock", choices=["stock", "h2-batch"])
    ap.add_argument("--alloc", default="default", choices=["default", "pinned"])
    args = runner.parse_args()
    os.makedirs(args.side, exist_ok=True)
    names = names_of(args.group, args.transports.split(","), args.launch)
    if args.only:
        keep = set(args.only.split(","))
        names = [n for n in names if n in keep]
    try:
        for name in names:
            k = int(name.rsplit("|", 1)[1])
            runner.bench_time_func(name, time_func, name, args.side, inner_loops=k)
    finally:
        if own is not None:
            own.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
