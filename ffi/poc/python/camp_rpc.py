"""CAMPAIGN.md section 4.2: the RPC grid. Client pinned to AK_CPU_CLIENT; the server is
`camp_server.py`, its OWN process pinned to AK_CPU_SERVER (R-C3/R-C4: the client and the
server no longer share one GIL or one CPU set).

  python3.12 camp_rpc.py --launch N --rounds R --calls C --out FILE [--allow-dirty] [--smoke]

Cells (requirement 12), P2.2, loopback TCP:
  A  incumbent codec (SerializeToString / FromString, as grpcio's generated stub calls them)
     + grpcio's channel
  B  incumbent codec + the core's transport, BLOCKING delivery (ak_call_unary)
  C  core codec through the C ABI (the C extension facade) + the core's transport, blocking
  D  core codec through the C ABI + grpcio's channel
  labelled extra rows (requirement 16): B-queue, C-queue, B-callback, C-callback
Directions (14): (a) empty request, P2.2 response, reported bare (`a`) and followed by
reading every field (`a+read`, R-C2; upb's FromString is lazy); (b) P2.2 request the server
decodes, empty response. In flight (15): 1, 8, 16. Transport (17): `shipped` and `pinned`,
and B and C follow the same switch as A and D; within one transport every cell talks to the
SAME server process with the same configuration.

Every call is checked (18): status OK and the response length (P2.2's in (a), 0 in (b)); the
server checks every request. One failure aborts the run and the log carries NO sample.
Samples (21-23, 28): per round, per (transport, direction, in flight), the cells in an order
rotated by one each round; a sample is `calls` calls over `inflight` threads, timed with
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


def core_client(port, transport):
    rt = arms._ffi.rt_new(0)
    uri = "http://127.0.0.1:%d" % port
    if transport == "shipped":
        return rt, arms._ffi.client_new(rt, uri)        # tonic's defaults: the core ships no pin
    # ak_client_opts: stream and connection windows 4 MiB, adaptive OFF, limits raised,
    # Nagle OFF (0).
    return rt, arms._ffi.client_new_opts(rt, uri, WINDOW, WINDOW, 0, MSG_LIMIT, MSG_LIMIT, 0)


def cells(port, transport):
    """{direction: [(cell, fn)]}; every fn makes ONE checked call."""
    root = arms.ROOT_OF[PID]
    R = arms._pb_root(PID)
    plan = arms._PLANS[root]
    body_len = len(arms.reference(PID))
    msg = arms.build_upb_native(PID)                    # built through protobuf's setters
    fc = arms.build_facade(PID, arms.CT_CEXT)
    ch = grpc.insecure_channel("127.0.0.1:%d" % port, options=grpc_options(transport))
    rt, cl = core_client(port, transport)

    def need(b, n):
        if b is None or len(b) != n:
            raise CallFailed("response is %s bytes, want %d" % (None if b is None else len(b), n))
        return b

    core_dec = (lambda b: arms._ffi.decode("cext", root, b, arms.TY_CEXT))
    core_enc = (lambda o: arms._ffi.encode("cext", root, o))
    read_pb = (lambda o: arms._read_pb(o, plan, True))
    read_fa = (lambda o: arms._read(o, plan))
    ident = (lambda b: b)

    def grpc_get(deser):
        return ch.unary_unary(GET, request_serializer=ident,
                              response_deserializer=lambda b: deser(need(b, body_len)))

    A_get = grpc_get(R.FromString)
    D_get = grpc_get(core_dec)
    A_put = ch.unary_unary(PUT, request_serializer=R.SerializeToString,
                           response_deserializer=lambda b: need(b, 0))
    D_put = ch.unary_unary(PUT, request_serializer=core_enc,
                           response_deserializer=lambda b: need(b, 0))

    def core_get():
        return need(arms._ffi.call_unary(cl, GET, b""), body_len)   # raises on status != 0

    def core_put(req):
        return need(arms._ffi.call_unary(cl, PUT, req), 0)

    local = threading.local()

    def queued_get():
        q = getattr(local, "q", None)
        if q is None:
            q = local.q = arms._ffi.queue_new()
        arms._ffi.call_unary_q(cl, GET, b"", q, 1)
        c = arms._ffi.queue_next(q, TIMEOUT_MS)
        if c is None:
            raise CallFailed("no completion")
        _t, status, b = c
        if status != 0:
            raise CallFailed("completion status %d" % status)
        return need(b, body_len)

    def callback_get():
        ev, box = threading.Event(), []

        def cb(tag, status, b):
            box.append((status, b))
            ev.set()
        arms._ffi.call_unary_cb(cl, GET, b"", cb, 1)
        if not ev.wait(TIMEOUT_MS / 1000.0):
            raise CallFailed("no completion")
        status, b = box[0]
        if status != 0:
            raise CallFailed("completion status %d" % status)
        return need(b, body_len)

    out = {
        "a": [("A", lambda: A_get(b"")),
              ("B", lambda: R.FromString(core_get())),
              ("C", lambda: core_dec(core_get())),
              ("D", lambda: D_get(b"")),
              ("B-queue", lambda: R.FromString(queued_get())),
              ("C-queue", lambda: core_dec(queued_get())),
              ("B-callback", lambda: R.FromString(callback_get())),
              ("C-callback", lambda: core_dec(callback_get()))],
        "a+read": [("A", lambda: read_pb(A_get(b""))),
                   ("B", lambda: read_pb(R.FromString(core_get()))),
                   ("C", lambda: read_fa(core_dec(core_get()))),
                   ("D", lambda: read_fa(D_get(b"")))],
        "b": [("A", lambda: A_put(msg)),
              ("B", lambda: core_put(R.SerializeToString(msg))),
              ("C", lambda: core_put(core_enc(fc))),
              ("D", lambda: D_put(fc))],
    }
    keep = (ch, rt, cl)
    return out, keep


def gate(cs):
    """Correctness before timing, per cell: (a) the decoded object re-encodes to P2.2; (b)
    the request each cell sends is P2.2's bytes (the server also decodes and checks it)."""
    ref = arms.reference(PID)
    R = arms._pb_root(PID)
    root = arms.ROOT_OF[PID]
    for name, fn in cs["a"]:
        o = fn()
        back = (o.SerializeToString(deterministic=True) if isinstance(o, R)
                else arms._ffi.encode("cext", root, o))
        if back != ref and R.FromString(back) != R.FromString(ref):
            raise CallFailed("gate: cell %s (a) does not re-encode to P2.2" % name)
    for name, fn in cs["a+read"] + cs["b"]:
        fn()
    msg = arms.build_upb_native(PID)
    fc = arms.build_facade(PID, arms.CT_CEXT)
    if R.FromString(msg.SerializeToString()) != R.FromString(ref) or arms._ffi.encode("cext", root, fc) != ref:
        raise CallFailed("gate: a (b) request is not P2.2")


def sample(fn, calls, inflight):
    per = max(1, calls // inflight)
    failed = []
    stop = threading.Event()

    def work():
        try:
            for _ in range(per):
                if stop.is_set():
                    return
                fn()
        except BaseException as e:  # noqa: BLE001
            failed.append(e)
            stop.set()
    ths = [threading.Thread(target=work) for _ in range(inflight)]
    gc.collect()
    c0, w0 = L.proc_cpu_ns(), L.wall_ns()
    for t in ths:
        t.start()
    for t in ths:
        t.join()
    c1, w1 = L.proc_cpu_ns(), L.wall_ns()
    if failed:
        e = failed[0]
        raise CallFailed("%s: %s" % (type(e).__name__, (str(e).splitlines() or [""])[0][:160]))
    return c1 - c0, w1 - w0, per * inflight


def start_server(transport):
    p = subprocess.Popen([sys.executable, os.path.join(HERE, "camp_server.py"), "--transport", transport],
                         stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    line = p.stdout.readline().split()
    if not line or line[0] != "PORT":
        p.kill()
        raise CallFailed("the server did not start")
    return p, int(line[1]), line[3] if len(line) > 3 else "?"


def main():
    launch = opt("--launch", 1, int)
    rounds = opt("--rounds", 5, int)
    calls = opt("--calls", 400, int)
    transports = opt("--transports", "shipped,pinned").split(",")
    log = L.Log(opt("--out"), "rpc", allow_dirty="--allow-dirty" in ARGS, smoke="--smoke" in ARGS)
    log.header(launch=launch, rounds=rounds, calls_per_sample=calls, inflight=INFLIGHT,
               affinity_client=AFFINITY, payload="%s (%d bytes)" % (PID, len(arms.reference(PID))),
               transport_shipped="grpcio: no channel option (packages/python create_channel); server: grpcio "
                                 "defaults; core: ak_client_new (tonic defaults)",
               transport_pinned="grpcio client and server: grpc.http2.lookahead_bytes=4 MiB, bdp_probe=0, "
                                "message limits 16 MiB; no connection-window argument exists in grpcio; "
                                "grpcio sets TCP_NODELAY itself (logs/python/80). core: ak_client_new_opts "
                                "stream=connection=4 MiB, adaptive=0, limits 16 MiB, tcp_nagle=0 (off)",
               network="loopback TCP 127.0.0.1", server="camp_server.py, separate process, AK_CPU_SERVER, "
               "pre-serialised P2.2 on Get; Put decodes with upb and checks",
               allocator="M_TOP_PAD %s" % ("applied" if _WARM else "not available"),
               gc="ON; gc.collect() before every sample", warmup="one sample's calls per cell before round 1",
               clock="CLOCK_PROCESS_CPUTIME_ID of the client (cpu_ns), perf_counter_ns (wall_ns)",
               delivery="B and C blocking; queue and callback are labelled extra cells, direction a only")
    try:
        for transport in transports:
            srv, port, saff = start_server(transport)
            log.note("server (%s): pid %d, port %d, affinity %s" % (transport, srv.pid, port, saff))
            try:
                cs, keep = cells(port, transport)
                gate(cs)
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
                                           inflight=k, launch=launch, round=r + 1,
                                           cpu_ns=cpu, wall_ns=wall, iters=n)
                del keep
            finally:
                srv.stdin.close()
                srv.wait(timeout=30)
    except Exception as e:  # noqa: BLE001  (grpc.RpcError included: any failure aborts)
        log.close(False, "%s: %s" % (type(e).__name__, (str(e).splitlines() or [""])[0][:200]))
        print("ABORTED: %s" % e)
        return 1
    log.close(True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
