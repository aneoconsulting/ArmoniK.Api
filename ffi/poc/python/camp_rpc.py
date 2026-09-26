"""CAMPAIGN.md section 4.2: the RPC grid. Client pinned to AK_CPU_CLIENT; the server is
`camp_server.py`, its OWN process pinned to AK_CPU_SERVER (R-C3/R-C4: the client and the
server no longer share one GIL or one CPU set).

  python3.12 camp_rpc.py --launch N --rounds R --calls C --out FILE [--variant nounk]
                        [--server shipped=unix:...,pinned=unix:...] [--allow-dirty] [--smoke]

Cells (requirement 12 as amended), P2.2, over a Unix domain socket (req 17 as amended):
  A  incumbent codec (SerializeToString / FromString, as grpcio's generated stub calls them)
     + grpcio's channel
  B  incumbent codec + the core's transport, BLOCKING delivery (ak_call_unary)
  C  core codec through the C ABI (the C extension facade) + the core's transport, blocking
  D  core codec through the C ABI + grpcio's channel
  E  host-gen (the generated pure-Python codec, same C-extension facade objects) + the core's
     transport, blocking
  F  host-gen + grpcio's channel
  C and D run in each unknown-field mode this build has: C-retain / D-retain and C-drop /
  D-drop in the full build, C-nounk / D-nounk in the no-unknown build (`--variant nounk`,
  `_akffi_rpc_nounk`, its own process: both cores are `libak_core.so`). E and F run in
  host-gen's modes: E-retain / F-retain and E-drop / F-drop in the full build; E-nounk /
  F-nounk in the no-unknown build (host-gen drop over the no-unknown facade, which has no
  `_unknown`). A and B run the incumbent in its default
  mode. Labelled extras (full build, direction a): B-queue, C-queue, B-callback, C-callback.
  A, D and F use grpcio's idiomatic blocking unary call (req 16 as amended).
Server (req 13 as amended): one camp_server.py per launch, both transports on two sockets,
shared by both builds (`--server`, from run_campaign.sh), warmed by AK_CAMPAIGN_SERVER_WARMUP
calls from each client transport; one channel (grpcio channel or core client) per cell.
Without `--server` the client starts its own (the gate's must-fail control, by-hand runs).
Directions (14): (a) empty request, P2.2 response, reported bare (`a`) and followed by
reading every field (`a+read`, R-C2; upb's FromString is lazy); (b) P2.2 request the server
decodes, empty response. In flight (15): 1, 8, 16. Transport (17): `shipped` and `pinned`,
and B and C follow the same switch as A and D; within one transport every cell talks to the
SAME server process with the same configuration.

Every call is checked (18): status OK and the response length (P2.2's in (a), 0 in (b)); the
server checks every request. One failure aborts the run and the log carries NO sample.
Samples (21-23, 28): per round, per (transport, direction, in flight), the cells in an order
rotated by one each round (req 22); a sample is `calls` calls over `inflight` threads of one
pool created before any timed window and reused (R-H2), timed with
CLOCK_PROCESS_CPUTIME_ID of the client and wall beside it. Warm-up: every cell runs one
sample's calls before round 1. GC on; allocator in the long-lived state (J26).
"""
import gc
import os
import subprocess
import sys
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import camp_lib as L  # noqa: E402

AFFINITY = L.pin("AK_CPU_CLIENT")

import allocator  # noqa: E402
_WARM = allocator.warm_up()
VARIANT = sys.argv[sys.argv.index("--variant") + 1] if "--variant" in sys.argv else "full"
NOUNK = VARIANT == "nounk"
if NOUNK:
    os.environ.setdefault("AK_FFI_MODULE", "_akffi_rpc_nounk")   # rpc_counts.py sets the counting build
os.environ.setdefault("AK_FFI_MODULE", "_akffi_rpc")
import arms  # noqa: E402
import grpc  # noqa: E402

ARGS = sys.argv[1:]
PID = "P2.2"
GET, PUT = "/ffi.Bench/Get", "/ffi.Bench/Put"
WINDOW = 4 << 20
MSG_LIMIT = 16 << 20
INFLIGHT = [1, 8, 16]
TIMEOUT_MS = 30000


def opt(name, default=None, conv=str):
    return conv(ARGS[ARGS.index(name) + 1]) if name in ARGS else default


class CallFailed(RuntimeError):
    pass


def grpc_options(transport):
    if transport == "shipped":
        return []           # packages/python's create_channel passes no option to a client
    return [("grpc.http2.lookahead_bytes", WINDOW), ("grpc.http2.bdp_probe", 0),
            ("grpc.max_receive_message_length", MSG_LIMIT),
            ("grpc.max_send_message_length", MSG_LIMIT)]


CORE_WORKERS = int(os.environ.get("AK_CORE_WORKERS", "2"))   # the core runtime's worker threads (req 4)
SERVER_WARMUP = int(os.environ.get("AK_CAMPAIGN_SERVER_WARMUP", "64"))  # calls per client transport (req 13)
RT = []


def runtime():
    """ONE core runtime per client process, shared by every core client (stated: its worker
    thread count is AK_CORE_WORKERS, the core's default when the host passes 0 being 2)."""
    if not RT:
        RT.append(arms._ffi.rt_new(CORE_WORKERS))
    return RT[0]


def core_client(target, transport):
    """A core client dialling `target` (`unix:/path`, req 17 as amended: tonic's
    Endpoint::from_shared dials a Unix domain socket for a `unix:` target)."""
    if transport == "shipped":
        return arms._ffi.client_new(runtime(), target)        # tonic's defaults: the core ships no pin
    # ak_client_opts: stream and connection windows 4 MiB, adaptive OFF, limits raised,
    # Nagle OFF (0; no effect on a Unix socket, stated).
    return arms._ffi.client_new_opts(runtime(), target, WINDOW, WINDOW, 0, MSG_LIMIT, MSG_LIMIT, 0)


def cells(target, transport):
    """{direction: [(cell, fn)]}; every fn makes ONE checked call. Req 13 as amended: ONE
    channel per cell per launch (a grpcio channel for A, D-*, F-*; a core client for B, C-*,
    E-* and each labelled extra), opened here, before round 1, and warmed by the warm-up."""
    root = arms.ROOT_OF[PID]
    R = arms._pb_root(PID)
    plan = arms._PLANS[root]
    body_len = len(arms.reference(PID))
    msg = arms.build_upb_native(PID)                    # built through protobuf's setters
    fc = arms.build_facade(PID, arms.CT_CEXT)
    chans, clis = {}, {}

    def chan(cell):
        if cell not in chans:
            chans[cell] = grpc.insecure_channel(target, options=grpc_options(transport))
        return chans[cell]

    def cli(cell):
        if cell not in clis:
            clis[cell] = core_client(target, transport)
        return clis[cell]

    def need(b, n):
        if b is None or len(b) != n:
            raise CallFailed("response is %s bytes, want %d" % (None if b is None else len(b), n))
        return b

    core_dec = (lambda b: arms._ffi.decode("cext", root, b, arms.TY_CEXT, None, False))
    core_enc = (lambda o: arms._ffi.encode("cext", root, o, None, False))
    ret_dec = (lambda b: arms._ffi.decode("cext", root, b, arms.TY_CEXT, None, True))
    ret_enc = (lambda o: arms._ffi.encode("cext", root, o, None, True))
    # host-gen (cells E and F, req 12 as amended): the pure-Python codec over the same
    # C-extension facade objects (R-H16), in each mode host-gen has in the codec suite
    hg = {"drop": arms.pycodec, "retain": arms.pycodec_retain, "nounk": arms.pycodec}
    hg_dec = {k: (lambda b, _m=m: getattr(_m, "decode_root_" + root)(b, arms.CT_CEXT)) for k, m in hg.items() if m}
    hg_enc = {k: (lambda o, _m=m: getattr(_m, "encode_root_" + root)(o)) for k, m in hg.items() if m}
    read_pb = (lambda o: arms._read_pb(o, plan, True))
    read_fa = (lambda o: arms._read(o, plan))
    ident = (lambda b: b)

    # A, D and F use grpcio's idiomatic call (req 16 as amended): the generated-stub style
    # blocking unary multicallable, with the codec as its (de)serializer.
    def grpc_get(cell, deser):
        return chan(cell).unary_unary(GET, request_serializer=ident,
                                      response_deserializer=lambda b: deser(need(b, body_len)))

    def grpc_put(cell, ser):
        return chan(cell).unary_unary(PUT, request_serializer=ser,
                                      response_deserializer=lambda b: need(b, 0))

    def core_get(cell):
        c = cli(cell)
        return lambda: need(arms._ffi.call_unary(c, GET, b""), body_len)   # raises on status != 0

    def core_put(cell):
        c = cli(cell)
        return lambda req: need(arms._ffi.call_unary(c, PUT, req), 0)

    local = threading.local()

    def queued_get(cell):
        c = cli(cell)

        def run():
            q = getattr(local, "q", None)
            if q is None:
                q = local.q = arms._ffi.queue_new()
            arms._ffi.call_unary_q(c, GET, b"", q, 1)
            r = arms._ffi.queue_next(q, TIMEOUT_MS)
            if r is None:
                raise CallFailed("no completion")
            _t, status, b = r
            if status != 0:
                raise CallFailed("completion status %d" % status)
            return need(b, body_len)
        return run

    def callback_get(cell):
        c = cli(cell)

        def run():
            ev, box = threading.Event(), []

            def cb(tag, status, b):
                box.append((status, b))
                ev.set()
            arms._ffi.call_unary_cb(c, GET, b"", cb, 1)
            if not ev.wait(TIMEOUT_MS / 1000.0):
                raise CallFailed("no completion")
            status, b = box[0]
            if status != 0:
                raise CallFailed("completion status %d" % status)
            return need(b, body_len)
        return run

    modes_c = ["nounk"] if NOUNK else ["retain", "drop"]
    cdec = {"retain": ret_dec, "drop": core_dec, "nounk": core_dec}
    cenc = {"retain": ret_enc, "drop": core_enc, "nounk": core_enc}
    out = {"a": [], "a+read": [], "b": []}
    A_get, A_put = grpc_get("A", R.FromString), grpc_put("A", R.SerializeToString)
    B_get, B_put = core_get("B"), core_put("B")
    out["a"] += [("A", lambda: A_get(b"")), ("B", lambda: R.FromString(B_get()))]
    out["a+read"] += [("A", lambda: read_pb(A_get(b""))), ("B", lambda: read_pb(R.FromString(B_get())))]
    out["b"] += [("A", lambda: A_put(msg)), ("B", lambda: B_put(R.SerializeToString(msg)))]
    for m in modes_c:
        cg, cp = core_get("C-" + m), core_put("C-" + m)
        dg, dp = grpc_get("D-" + m, cdec[m]), grpc_put("D-" + m, cenc[m])
        out["a"] += [("C-" + m, lambda _g=cg, _d=cdec[m]: _d(_g())), ("D-" + m, lambda _g=dg: _g(b""))]
        out["a+read"] += [("C-" + m, lambda _g=cg, _d=cdec[m]: read_fa(_d(_g()))),
                          ("D-" + m, lambda _g=dg: read_fa(_g(b"")))]
        out["b"] += [("C-" + m, lambda _p=cp, _e=cenc[m]: _p(_e(fc))), ("D-" + m, lambda _p=dp: _p(fc))]
    # E and F in host-gen's modes: retain and drop in the full build; in the no-unknown build,
    # host-gen drop over the no-unknown facade (no `_unknown`, R-H22) is its no-unknown mode.
    for m in (["nounk"] if NOUNK else ["retain", "drop"]):
        eg, ep = core_get("E-" + m), core_put("E-" + m)
        fg, fpu = grpc_get("F-" + m, hg_dec[m]), grpc_put("F-" + m, hg_enc[m])
        out["a"] += [("E-" + m, lambda _g=eg, _d=hg_dec[m]: _d(_g())), ("F-" + m, lambda _g=fg: _g(b""))]
        out["a+read"] += [("E-" + m, lambda _g=eg, _d=hg_dec[m]: read_fa(_d(_g()))),
                          ("F-" + m, lambda _g=fg: read_fa(_g(b"")))]
        out["b"] += [("E-" + m, lambda _p=ep, _e=hg_enc[m]: _p(_e(fc))), ("F-" + m, lambda _p=fpu: _p(fc))]
    if not NOUNK:
        qg, kg = queued_get("B-queue"), callback_get("B-callback")
        qc, kc = queued_get("C-queue"), callback_get("C-callback")
        out["a"] += [("B-queue", lambda: R.FromString(qg())), ("C-queue", lambda: core_dec(qc())),
                     ("B-callback", lambda: R.FromString(kg())), ("C-callback", lambda: core_dec(kc()))]
    keep = (chans, clis)
    return out, keep


def warm_server(target, transport):
    """Req 13 as amended: before round 1 the server is warmed by SERVER_WARMUP calls from each
    client transport (a grpcio channel and a core client of this process), on connections that
    are then closed; each cell's own channel is warmed by the per-cell warm-up after this."""
    ch = grpc.insecure_channel(target, options=grpc_options(transport))
    g = ch.unary_unary(GET, request_serializer=lambda b: b, response_deserializer=lambda b: b)
    c = core_client(target, transport)
    n = len(arms.reference(PID))
    for _ in range(SERVER_WARMUP):
        if len(g(b"")) != n or len(arms._ffi.call_unary(c, GET, b"")) != n:
            raise CallFailed("server warm-up: a short response")
    ch.close()
    return "%d Get calls from grpcio and %d from the core client, %s" % (SERVER_WARMUP, SERVER_WARMUP, transport)


def gate(cs):
    """Correctness before timing, per cell: (a) the decoded object re-encodes to P2.2; (b)
    the request each cell sends is P2.2's bytes (the server also decodes and checks it)."""
    ref = arms.reference(PID)
    R = arms._pb_root(PID)
    root = arms.ROOT_OF[PID]
    for name, fn in cs["a"]:
        o = fn()
        back = (o.SerializeToString(deterministic=True) if isinstance(o, R)
                else arms._ffi.encode("cext", root, o, None, name.endswith("-retain")))
        if back != ref and R.FromString(back) != R.FromString(ref):
            raise CallFailed("gate: cell %s (a) does not re-encode to P2.2" % name)
    for name, fn in cs["a+read"] + cs["b"]:
        fn()
    msg = arms.build_upb_native(PID)
    fc = arms.build_facade(PID, arms.CT_CEXT)
    if R.FromString(msg.SerializeToString()) != R.FromString(ref) or arms._ffi.encode("cext", root, fc) != ref:
        raise CallFailed("gate: a (b) request is not P2.2")
    if bool(arms._ffi.nounk()) != NOUNK:
        raise CallFailed("gate: %s is not the %s build" % (arms._ffi.__name__, VARIANT))
    if NOUNK:
        return nounk_control()
    if arms._ffi.encode("cext", root, fc, None, True) != ref:
        raise CallFailed("gate: the retain encode of the P2.2 request is not P2.2")
    return retain_control()


UNK = bytes([0xC0, 0x3E, 0x01])     # field 1000, varint 1: in no schema message


def nounk_control():
    """The no-unknown build: P2.2 with field 1000 appended decodes (the field skipped) and
    re-encodes to P2.2; a retain call is refused; nothing is grown."""
    ref = arms.reference(PID)
    root = arms.ROOT_OF[PID]
    buf = ref + UNK
    t0 = arms._ffi.unk_totals()
    lost = arms._ffi.encode("cext", root, arms._ffi.decode("cext", root, buf, arms.TY_CEXT, None, False), None, False)
    t1 = arms._ffi.unk_totals()
    if lost != ref:
        raise CallFailed("control: the no-unknown build did not give P2.2 back")
    try:
        arms._ffi.decode("cext", root, buf, arms.TY_CEXT, None, True)
        raise CallFailed("control: the no-unknown build accepted a retain decode")
    except ValueError as e:
        if "compiled out" not in str(e):
            raise
    if (t1[0] - t0[0], t1[1] - t0[1], t1[2] - t0[2]) != (1, 0, 0):
        raise CallFailed("control: unk_totals moved by %r, want (1, 0, 0)" % ((t1[0] - t0[0], t1[1] - t0[1], t1[2] - t0[2]),))
    return ("no-unknown control: %s; P2.2 + field 1000 decodes to P2.2 (%d bytes); a retain decode is refused"
            % (arms._ffi.__name__, len(lost)))


def retain_control():
    """Is retain in the build? P2.2 with one unknown field appended at the root, decoded and
    re-encoded by the same calls the C-retain / D-retain and -drop cells make: retain must
    re-emit the unknown bytes, drop must give P2.2 back, and no buffer is left undelivered."""
    ref = arms.reference(PID)
    root = arms.ROOT_OF[PID]
    if not arms._ffi.unk_positions(root):
        raise CallFailed("control: root %s has no unknown-field position" % root)
    buf = ref + UNK
    t0 = arms._ffi.unk_totals()
    kept = arms._ffi.encode("cext", root, arms._ffi.decode("cext", root, buf, arms.TY_CEXT, None, True), None, True)
    lost = arms._ffi.encode("cext", root, arms._ffi.decode("cext", root, buf, arms.TY_CEXT, None, False), None, False)
    t1 = arms._ffi.unk_totals()
    if UNK not in kept or len(kept) != len(buf):
        raise CallFailed("control: retain did not re-emit the unknown field (%d bytes, want %d)" % (len(kept), len(buf)))
    if lost != ref:
        raise CallFailed("control: drop did not give P2.2 back")
    if (t1[0] - t0[0], t1[1] - t0[1], t1[2] - t0[2]) != (1, 1, 0):
        raise CallFailed("control: unk_totals moved by %r, want (1, 1, 0)" % ((t1[0] - t0[0], t1[1] - t0[1], t1[2] - t0[2]),))
    return "retain control: P2.2 + field 1000 re-emitted by retain (%d bytes), dropped by drop; 0 leaked" % len(kept)


threading_starts = [0]


def unknown_mode(cell):
    if cell.endswith("-nounk"):
        return "no-unknown"
    if cell.endswith("-retain"):
        return "retain"
    if cell.endswith("-drop") or cell.startswith("C-"):
        return "drop"
    return "incumbent-default"


class Pool:
    """R-H2: the client threads are created ONCE, before any timed window, and reused by every
    sample of every cell (per-thread decode contexts included). A sample hands `per` calls to
    each of the first `inflight` threads and waits for them; idle threads block on an Event and
    burn no CPU. Before this, every sample started and joined `inflight` threads inside the
    window, which timed thread creation with the calls."""

    def __init__(self, n):
        self.go = [threading.Event() for _ in range(n)]
        self.done = threading.Semaphore(0)
        self.task = None
        self.ths = [threading.Thread(target=self._loop, args=(i,), daemon=True) for i in range(n)]
        for t in self.ths:
            t.start()
        threading_starts[0] += n

    def _loop(self, i):
        while True:
            self.go[i].wait()
            self.go[i].clear()
            task = self.task
            if task is None:
                return
            fn, per, stop, failed = task
            try:
                for _ in range(per):
                    if stop.is_set():
                        break
                    fn()
            except BaseException as e:  # noqa: BLE001
                failed.append(e)
                stop.set()
            self.done.release()

    def run(self, k, fn, per, stop, failed):
        self.task = (fn, per, stop, failed)
        for i in range(k):
            self.go[i].set()
        for _ in range(k):
            self.done.acquire()

    def close(self):
        self.task = None
        for g in self.go:
            g.set()
        for t in self.ths:
            t.join()


POOL = []


def sample(fn, calls, inflight):
    if not POOL:
        POOL.append(Pool(max(INFLIGHT)))      # outside every timed window
    per = max(1, calls // inflight)
    failed = []
    stop = threading.Event()
    gc.collect()
    c0, w0 = L.proc_cpu_ns(), L.wall_ns()
    POOL[0].run(inflight, fn, per, stop, failed)
    c1, w1 = L.proc_cpu_ns(), L.wall_ns()
    if failed:
        e = failed[0]
        raise CallFailed("%s: %s" % (type(e).__name__, (str(e).splitlines() or [""])[0][:160]))
    return c1 - c0, w1 - w0, per * inflight


def start_server(d):
    """This process's own server (camp_server.py, both transports, UDS in `d`), for a run with
    no --server: the gate's must-fail control and by-hand runs. run_campaign.sh starts ONE
    server per launch instead and passes it to both builds' clients (req 13 as amended)."""
    p = subprocess.Popen([sys.executable, os.path.join(HERE, "camp_server.py"), "--dir", d],
                         stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    line = p.stdout.readline().split()
    if not line or line[0] != "SOCKETS":
        p.kill()
        raise CallFailed("the server did not start")
    return p, parse_server(" ".join(line))


def parse_server(line):
    """`SOCKETS shipped=unix:... pinned=unix:... AFFINITY a WORKERS n THREADS t` -> dict."""
    f = line.split()
    out = {"affinity": "?", "workers": "?", "threads": "?"}
    for x in f[1:]:
        if "=" in x:
            k, v = x.split("=", 1)
            out[k] = v
    for key in ("AFFINITY", "WORKERS", "THREADS"):
        if key in f:
            out[key.lower()] = f[f.index(key) + 1]
    return out


def os_threads():
    try:
        return len(os.listdir("/proc/self/task"))
    except OSError:
        return -1


def main():
    launch = opt("--launch", 1, int)
    rounds = opt("--rounds", 5, int)
    calls = opt("--calls", 400, int)
    transports = opt("--transports", "shipped,pinned").split(",")
    own = None
    if opt("--server"):
        srvinfo = parse_server("SOCKETS " + opt("--server").replace(",", " "))
    else:
        import tempfile
        tmpd = tempfile.mkdtemp(prefix="akrpc")
        own, srvinfo = start_server(tmpd)
    log = L.Log(opt("--out"), "rpc", allow_dirty="--allow-dirty" in ARGS, smoke="--smoke" in ARGS,
                build=VARIANT)
    log.header(launch=launch, rounds=rounds, calls_per_sample=calls, inflight=INFLIGHT,
               affinity_client=AFFINITY, payload="%s (%d bytes)" % (PID, len(arms.reference(PID))),
               transport_shipped="grpcio: no channel option (packages/python create_channel); server: grpcio "
                                 "defaults; core: ak_client_new (tonic defaults)",
               transport_pinned="grpcio client and server: grpc.http2.lookahead_bytes=4 MiB, bdp_probe=0, "
                                "message limits 16 MiB; no connection-window argument exists in grpcio; "
                                "grpcio sets TCP_NODELAY itself (stated; log 80, which showed it, was deleted under R-C9). core: ak_client_new_opts "
                                "stream=connection=4 MiB, adaptive=0, limits 16 MiB, tcp_nagle=0 (off)",
               network="Unix domain socket (req 17 as amended): %s" % ", ".join(
                   "%s=%s" % (t, srvinfo.get(t, "?")) for t in ("shipped", "pinned")),
               server=("camp_server.py, ONE separate process for this launch (%s), both transports, AK_CPU_SERVER "
                       "affinity %s, grpcio executor workers %s, server threads at start %s; pre-serialised P2.2 on "
                       "Get; Put decodes with upb and checks"
                       % ("shared by both builds, started by run_campaign.sh" if own is None else "this client's own",
                          srvinfo.get("affinity"), srvinfo.get("workers"), srvinfo.get("threads"))),
               order="per round, per (transport, direction, in flight), the cells rotated by one (req 22)",
               idiomatic="A, D and F: grpcio's blocking unary multicallable (the generated stub's call), the "
                         "codec as its (de)serializer (req 16 as amended)",
               allocator="M_TOP_PAD %s" % ("applied" if _WARM else "not available"),
               gc="ON; gc.collect() before every sample",
               warmup="the server: %d Get calls from each client transport (grpcio, core) per server transport; "
                      "then one sample's calls per cell and in-flight value before round 1, on the cell's own "
                      "channel (one channel per cell per launch, req 13)" % SERVER_WARMUP,
               clock="CLOCK_PROCESS_CPUTIME_ID of the client (cpu_ns), perf_counter_ns (wall_ns)",
               delivery="B and C blocking; queue and callback are labelled extra cells, direction a only",
               threads="a pool of max(in flight) client threads created once, before the first timed window, "
                       "reused by every sample of every cell (R-H2)",
               variant_build=("no-unknown variant (WP5 step 10): _akffi_rpc_nounk over ak-core --no-default-features "
                      "--features rpc,init-guard; A and B are this process's controls" if NOUNK
                      else "full build (unknown-fields on): _akffi_rpc"),
               unknown_modes=("C-nounk, D-nounk, E-nounk, F-nounk: no-unknown (compiled out; E and F: host-gen drop over the "
                              "no-unknown facade); A and B: incumbent default" if NOUNK else
                              "C and D: retain (decision 11, every position armed, per-thread contexts) and "
                              "drop (every entry zero); A and B: incumbent default; C-queue and C-callback: drop"))
    try:
        for transport in transports:
            target = srvinfo[transport]
            t0th = os_threads()
            if True:
                log.note("server warm-up: " + warm_server(target, transport))
                cs, keep = cells(target, transport)
                log.note(gate(cs))
                u0, tl0, th0 = arms._ffi.unk_totals(), arms._ffi.tls_created(), threading_starts[0]
                for d, lst in cs.items():
                    for k in INFLIGHT:
                        for name, fn in lst:
                            sample(fn, calls, k)            # warm-up
                for r in range(rounds):
                    for d, lst in cs.items():
                        for k in INFLIGHT:
                            for name, fn in L.rotated(lst, r):
                                cpu, wall, n = sample(fn, calls, k)
                                log.sample(cell=name, payload=PID, dir=d, transport=transport,
                                           unknown_mode=unknown_mode(name),
                                           inflight=k, launch=launch, round=r + 1,
                                           cpu_ns=cpu, wall_ns=wall, iters=n)
                u1, tl1, th1 = arms._ffi.unk_totals(), arms._ffi.tls_created(), threading_starts[0]
                du = (u1[0] - u0[0], u1[1] - u0[1], u1[2] - u0[2])
                log.note("%s: core-ffi decodes drop %d, retain %d, leaked buffers %d; contexts created %d "
                         "over %d threads started" % (transport, du[0], du[1], du[2], tl1 - tl0, th1 - th0))
                if du[2] != 0:
                    raise CallFailed("leak: %d unknown-field buffer(s) reclaimed undelivered" % du[2])
                if NOUNK and (du[1] != 0 or du[0] == 0):
                    raise CallFailed("the no-unknown build: drop %d, retain %d decodes" % du[:2])
                if not NOUNK and (du[1] == 0 or du[0] == 0):
                    raise CallFailed("a mode did not run: drop %d, retain %d decodes" % du[:2])
                if tl1 - tl0 > 2 * (th1 - th0) + 64:
                    raise CallFailed("contexts created %d for %d threads: not per thread" % (tl1 - tl0, th1 - th0))
                log.note("%s: worker threads (req 4): client pool %d, core runtime workers %d (one runtime for "
                         "%d core clients), grpcio channels %d; OS threads in this process %d before the "
                         "channels, %d after the run" % (transport, max(INFLIGHT), CORE_WORKERS, len(keep[1]),
                                                        len(keep[0]), t0th, os_threads()))
                del keep
    except Exception as e:  # noqa: BLE001  (grpc.RpcError included: any failure aborts)
        log.close(False, "%s: %s" % (type(e).__name__, (str(e).splitlines() or [""])[0][:200]))
        print("ABORTED: %s" % e)
        if own is not None:
            own.kill()
        return 1
    if POOL:
        POOL[0].close()
    if own is not None:
        own.stdin.close()
        own.wait(timeout=30)
    log.close(True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
