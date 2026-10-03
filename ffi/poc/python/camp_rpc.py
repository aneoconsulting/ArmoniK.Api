"""CAMPAIGN.md section 4.2: the RPC grid's LIBRARY (FIX-PLAN WP9: the grid itself runs on pyperf,
camp_rpc_pyperf.py; the hand-written sampler that was here is removed). What stays: the cells,
their channels and payloads, their checks, the client thread pool, the server start and warm-up.

The text below describes the cells; its sampling paragraphs are superseded by camp_rpc_pyperf.py.
Client pinned to AK_CPU_CLIENT. The server (FIX-PLAN WP10, req 13 as amended at 9f6d579fa) is
the Rust slice's tonic rpc_server, THE server of every slice (poc/rust/SERVER.md), started,
warmed and stopped through poc/rust/serve.sh, its own process pinned to AK_CPU_SERVER.

Cells (requirement 12 as amended), P2.2, over a Unix domain socket (req 17 as amended):
  A  incumbent codec + grpcio: the generated GridStub (proto/campaign_grid.proto), whose
     methods call SerializeToString / FromString
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
Server (req 13 as amended at 9f6d579fa, WP10): the Rust rpc_server, service
armonik.ffi.campaign.v1.Grid (Fetch a, Push b, Upload c, UploadStream d, UploadStreamCheck the
untimed (d) digest check), one process per launch serving both configurations on two Unix
sockets; `shipped` and `pinned` name the client's configuration against the server's socket
of the same name. Without `--server` the caller starts its own through serve.sh (the gate's
must-fail controls, rpc_counts.py). Planted faults are selected on the CLIENT (SERVER.md, req
18): AK_CAMP_PLANT=short calls FetchShort, =count expects one byte more from UploadStream,
=digest expects a wrong digest from UploadStreamCheck; each from a cell's 2nd call on, so a
worker's setup call passes and the fault meets its timed loop (digest: its setup check).
Directions (14): (a) empty request, P2.2 response, reported bare (`a`) and followed by
reading every field (`a+read`, R-C2; upb's FromString is lazy); (b) P2.2 request the server
decodes, empty response. In flight (15): 1, 8, 16. Transport (17): `shipped` and `pinned`,
and B and C follow the same switch as A and D; within one transport every cell talks to the
SAME server process with the same configuration.

Every call is checked (18): status OK and the response length (P2.2's in (a), 0 in (b) and (c);
cell A, whose generated stub hands back a decoded message, by its FromString succeeding and its
task count), (d) the server's byte count; the server checks every request (SERVER.md). One failure aborts the run and the log carries NO sample.
Samples (21-23, 28): per round, per (transport, direction, in flight), the cells in an order
rotated by one each round (req 22); a sample is `calls` calls over `inflight` threads of one
pool created before any timed window and reused (R-H2), timed with
CLOCK_PROCESS_CPUTIME_ID of the client and wall beside it. Warm-up: every cell runs one
sample's calls before round 1. GC on; allocator in the long-lived state (J26).
"""
import os
import sys
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import camp_lib as L  # noqa: E402

AFFINITY = L.pin("AK_CPU_CLIENT")

# D9 as amended (owner, 2026-10-03): the RPC grid's main figures use glibc's DEFAULT allocator,
# as production does, so no mallopt here (J26's M_TOP_PAD stays in the codec suite only); the
# pinned pass is the labelled diagnostic, GLIBC_TUNABLES set by the runner (camp_rpc_pyperf.py
# checks the environment matches the label).
_WARM = None
VARIANT = sys.argv[sys.argv.index("--variant") + 1] if "--variant" in sys.argv else "full"
NOUNK = VARIANT == "nounk"
if NOUNK:
    os.environ.setdefault("AK_FFI_MODULE", "_akffi_rpc_nounk")   # rpc_counts.py sets the counting build
os.environ.setdefault("AK_FFI_MODULE", "_akffi_rpc")
import arms  # noqa: E402
import grpc  # noqa: E402

ARGS = sys.argv[1:]
PID = "P2.2"
SVC = "/armonik.ffi.campaign.v1.Grid/"                  # poc/rust/SERVER.md
GET, GET_SHORT, PUT = SVC + "Fetch", SVC + "FetchShort", SVC + "Push"
UPLOAD, STREAM, STREAM_CHECK = SVC + "Upload", SVC + "UploadStream", SVC + "UploadStreamCheck"
PLANT = os.environ.get("AK_CAMP_PLANT", "")               # short | count | digest (client side)


def planted(ok, bad):
    """A planted cell: its first call is `ok`, every later one `bad` (req 18's controls only)."""
    n = [0]

    def f(*a, **k):
        n[0] += 1
        return (ok if n[0] == 1 else bad)(*a, **k)
    return f
# CAMPAIGN req 14 as amended (2026-09-27): (c) a unary upload of P5.3 / P5.4, (d) the streamed
# upload in 2 MiB chunks (ids on the first), 4 MiB and 16 MiB; both at 1 and 8 in flight.
C_PAYLOADS = ["P5.3", "P5.4"]
D_PAYLOADS = [("4MiB", 2), ("16MiB", 8)]
CHUNK = 2 * 1024 * 1024
UP_INFLIGHT = [1, 8]


def stream_payload(chunks):
    """(d)'s messages: M5 per chunk, deterministic data, the ids on the first message only;
    as upb messages and as C-extension facade objects, and the SHA-256 the server must see."""
    import hashlib
    import random
    rnd = random.Random(0x5EED0000 + chunks)
    M5, U = arms._pb2.UploadResultDataMessage, arms._pb2.UploadResultData
    C = arms.CT_CEXT
    ups, fcs, h = [], [], hashlib.sha256()
    for i in range(chunks):
        data = rnd.randbytes(CHUNK)
        sid, rid = ("session-u2", "result-u2") if i == 0 else ("", "")
        u = M5(upload=U(session_id=sid, result_id=rid, data_chunk=data))
        ups.append(u)
        fcs.append(C["UploadResultDataMessage"](upload=C["UploadResultData"](session_id=sid, result_id=rid, data_chunk=data)))
        h.update(u.SerializeToString())
    return ups, fcs, chunks * CHUNK, h.digest()
WINDOW = 4 << 20
MSG_LIMIT = 16 << 20
INFLIGHT = [1, 8, 16]
TIMEOUT_MS = 30000


def opt(name, default=None, conv=str):
    return conv(ARGS[ARGS.index(name) + 1]) if name in ARGS else default


class CallFailed(RuntimeError):
    pass


# WP10: grpcio's default :authority on a `unix:` target is the percent-encoded socket path,
# which the Rust server's HTTP/2 stack (h2, under tonic) refuses with RST_STREAM PROTOCOL_ERROR
# on every call (checked: every grpcio cell failed, every core cell passed). Both
# configurations therefore name the authority; nothing else about the channel changes.
AUTHORITY = [("grpc.default_authority", "localhost")]


def grpc_options(transport):
    if transport == "shipped":
        return list(AUTHORITY)  # packages/python's create_channel passes no other option to a client
    return AUTHORITY + [("grpc.http2.lookahead_bytes", WINDOW), ("grpc.http2.bdp_probe", 0),
            ("grpc.max_receive_message_length", MSG_LIMIT),
            ("grpc.max_send_message_length", MSG_LIMIT)]


# D14 (owner, 2026-10-03): every pool sized to AK_WORKERS (8, campaign.machine); AK_CORE_WORKERS
# overrides the core runtime's alone (an exploration knob, stated in the header)
WORKERS = int(os.environ.get("AK_WORKERS", "8"))
CORE_WORKERS = int(os.environ.get("AK_CORE_WORKERS", str(WORKERS)))   # the core runtime's worker threads (req 4)
RT = []


def runtime():
    """ONE core runtime per client process, shared by every core client (stated: its worker
    thread count is CORE_WORKERS: AK_WORKERS, D14)."""
    if not RT:
        RT.append(arms._ffi.rt_new(CORE_WORKERS))
    return RT[0]


def grpc_target(target):
    """grpcio's form of a server address: `127.0.0.1:PORT` over TCP (WP13, D10), or `unix:...`."""
    return target


def core_target(target):
    """The core's (tonic Endpoint::from_shared) form: `http://127.0.0.1:PORT`, or `unix:...`."""
    return target if target.startswith("unix:") else "http://" + target


def core_client(target, transport):
    """A core client dialling `target`: TCP 127.0.0.1 since WP13 (D10, req 17 as amended).
    shipped: ak_client_new, no options (tonic's defaults, its Endpoint sets TCP_NODELAY by
    default); pinned: ak_client_opts with tcp_nagle 0 (Nagle off). Both read back on the live
    socket by the worker (camp_meas.nodelay_summary)."""
    target = core_target(target)
    if transport == "shipped":
        return arms._ffi.client_new(runtime(), target)        # tonic's defaults: the core ships no pin
    # ak_client_opts: stream and connection windows 4 MiB, adaptive OFF, limits raised,
    # Nagle OFF (0; no effect on a Unix socket, stated).
    return arms._ffi.client_new_opts(runtime(), target, WINDOW, WINDOW, 0, MSG_LIMIT, MSG_LIMIT, 0)


def cells(target, transport, keys=None):
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
            chans[cell] = grpc.insecure_channel(grpc_target(target), options=grpc_options(transport))
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

    # A uses grpcio's generated stub (campaign_grid_pb2_grpc.GridStub, from proto/ by
    # grpc_tools, as ArmoniK's Python client is generated): a registered blocking unary
    # multicallable with SerializeToString / FromString (req 16 as amended, R14). D and F make
    # the same call the stub makes (unary_unary with _registered_method=True) with the core's
    # or host-gen's codec as its (de)serializer.
    import campaign_grid_pb2_grpc as GG
    stubs = {}

    def stub(cell):
        if cell not in stubs:
            stubs[cell] = GG.GridStub(chan(cell))
        return stubs[cell]

    def with_short(make):
        return planted(make(GET), make(GET_SHORT)) if PLANT == "short" else make(GET)

    def grpc_get(cell, deser):
        return with_short(lambda path: chan(cell).unary_unary(
            path, request_serializer=ident, response_deserializer=lambda b: deser(need(b, body_len)),
            _registered_method=True))

    def grpc_put(cell, ser):
        return chan(cell).unary_unary(PUT, request_serializer=ser, response_deserializer=lambda b: need(b, 0),
                                      _registered_method=True)

    ntasks = len(R.FromString(arms.reference(PID)).tasks)
    empty = arms._pb2.Empty()

    def a_checked(o):
        # the stub's FromString raises on a malformed body (a short one included); the count
        # of tasks is the length check a typed response allows without re-encoding it
        if len(o.tasks) != ntasks:
            raise CallFailed("response has %d tasks, want %d" % (len(o.tasks), ntasks))
        return o

    def stub_get(cell):
        g = with_short(lambda path: getattr(stub(cell), "Fetch" if path == GET else "FetchShort"))
        return lambda: a_checked(g(empty))

    def core_get(cell):
        c = cli(cell)
        return with_short(lambda path: (lambda: need(arms._ffi.call_unary(c, path, b""), body_len)))   # raises on status != 0

    def core_put(cell):
        c = cli(cell)
        return lambda req: need(arms._ffi.call_unary(c, PUT, req), 0)

    local = threading.local()

    def queued_get(cell):
        c = cli(cell)
        path = [GET_SHORT if PLANT == "short" else GET]

        def run():
            q = getattr(local, "q", None)
            if q is None:
                q = local.q = arms._ffi.queue_new()
            arms._ffi.call_unary_q(c, path[0], b"", q, 1)
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
        path = [GET_SHORT if PLANT == "short" else GET]

        def run():
            ev, box = threading.Event(), []

            def cb(tag, status, b):
                box.append((status, b))
                ev.set()
            arms._ffi.call_unary_cb(c, path[0], b"", cb, 1)
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
    A_get, A_put = stub_get("A"), stub("A").Push
    B_get, B_put = core_get("B"), core_put("B")
    out["a"] += [("A", A_get), ("B", lambda: R.FromString(B_get()))]
    out["a+read"] += [("A", lambda: read_pb(A_get())), ("B", lambda: read_pb(R.FromString(B_get())))]
    out["b"] += [("A", lambda: A_put(msg)), ("B", lambda: B_put(R.SerializeToString(msg)))]
    for m in modes_c:
        cg, cp = core_get("C-" + m), core_put("C-" + m)
        dg, dp = grpc_get("D-" + m, cdec[m]), grpc_put("D-" + m, cenc[m])
        out["a"] += [("C-" + m, lambda _g=cg, _d=cdec[m]: _d(_g())), ("D-" + m, lambda _g=dg: _g(b""))]
        out["a+read"] += [("C-" + m, lambda _g=cg, _d=cdec[m]: read_fa(_d(_g()))),
                          ("D-" + m, lambda _g=dg: read_fa(_g(b"")))]
        # C sends through the MOVE path (WP8 parity with the Rust and C++ slices): the facade is
        # encoded into the core's context and the context's buffer becomes the request
        # (ak_call_unary_enc); no host bytes object. The copy path (ak_enc_take to a bytes,
        # then ak_call_unary) is the labelled extra Cc-*. D cannot take the core's buffer:
        # grpcio's request must be a `bytes` (a memoryview or bytearray is refused), so D
        # keeps the one copy into a bytes object.
        cc = cli("C-" + m)
        ccp = core_put("Cc-" + m)
        out["b"] += [("C-" + m, lambda _c=cc, _r=(m == "retain"): need(arms._ffi.encode("cext", root, fc, None, _r, (_c, PUT)), 0)),
                     ("D-" + m, lambda _p=dp: _p(fc)),
                     ("Cc-" + m, lambda _p=ccp, _e=cenc[m]: _p(_e(fc)))]
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
    # ---- (c) and (d), every cell, plus the framed twins of the core-transport cells (the
    # framed send path, ak_client_set_framed, beside its reference; grpcio has no such path)
    R5 = arms._pb2.UploadResultDataMessage
    r5 = "UploadResultDataMessage"
    enc5 = {"inc": lambda o: o.SerializeToString()}
    for m in modes_c:
        enc5["C-" + m] = (lambda o, _ret=(m == "retain"): arms._ffi.encode("cext", r5, o, None, _ret))
    for m in ([] if NOUNK else ["retain", "drop"]) + (["nounk"] if NOUNK else []):
        enc5["E-" + m] = (lambda o, _m=hg[m]: getattr(_m, "encode_root_" + r5)(o))
    fam = [("A", "grpc", "inc"), ("B", "core", "inc"), ("Bf", "core", "inc")]
    for m in modes_c:
        fam += [("C-" + m, "core", "C-" + m), ("Cf-" + m, "core", "C-" + m), ("D-" + m, "grpc", "C-" + m),
                ("Cc-" + m, "core", "C-" + m)]
    for m in (["nounk"] if NOUNK else ["retain", "drop"]):
        fam += [("E-" + m, "core", "E-" + m), ("Ef-" + m, "core", "E-" + m), ("F-" + m, "grpc", "E-" + m)]

    def ccli(cell):
        c = cli(cell)
        if cell.startswith(("Bf", "Cf-", "Ef-")):
            arms._ffi.client_set_framed(c, True)
        return c

    # direction b's framed twins (the P2.2 request on the framed send path)
    def fput(cell):
        c = ccli(cell)
        return lambda req: need(arms._ffi.call_unary(c, PUT, req), 0)
    bp = fput("Bf")
    out["b"].append(("Bf", lambda: bp(R.SerializeToString(msg))))
    for m in modes_c:
        cf = ccli("Cf-" + m)
        out["b"].append(("Cf-" + m, lambda _c=cf, _r=(m == "retain"): need(arms._ffi.encode("cext", root, fc, None, _r, (_c, PUT)), 0)))
    for m in (["nounk"] if NOUNK else ["retain", "drop"]):
        ep2 = fput("Ef-" + m)
        out["b"].append(("Ef-" + m, lambda _p=ep2, _e=hg_enc[m]: _p(_e(fc))))
    for pid in C_PAYLOADS:
        if keys is not None and "c:" + pid not in keys:
            continue                                   # a benchmark worker builds only its own
        ref5 = arms.reference(pid)
        up5, fc5 = arms.build_upb_native(pid), arms.build_facade(pid, arms.CT_CEXT)
        key = "c:" + pid
        out[key] = []
        for cell, tr, e in fam:
            obj = up5 if e == "inc" else fc5
            if enc5[e](obj) != ref5:                   # correctness before timing
                raise CallFailed("gate: cell %s (c) %s does not encode to the reference" % (cell, pid))
            if cell == "A":
                out[key].append((cell, lambda _st=stub("A").Upload, _o=obj: _st(_o)))
            elif tr == "grpc":
                st = chan(cell).unary_unary(UPLOAD, request_serializer=enc5[e], response_deserializer=lambda b: need(b, 0),
                                            _registered_method=True)
                out[key].append((cell, lambda _st=st, _o=obj: _st(_o)))
            else:
                c = ccli(cell)
                if e.startswith("C-") and not cell.startswith("Cc-"):     # the move path
                    out[key].append((cell, lambda _c=c, _r=(e == "C-retain"), _o=obj:
                                     need(arms._ffi.encode("cext", r5, _o, None, _r, (_c, UPLOAD)), 0)))
                else:
                    out[key].append((cell, lambda _c=c, _e=enc5[e], _o=obj: need(arms._ffi.call_unary(_c, UPLOAD, _e(_o)), 0)))
    for label, chunks in D_PAYLOADS:
        if keys is not None and "d:" + label not in keys:
            continue
        ups, fcs, nbytes, sha = stream_payload(chunks)

        def verdicts(count, digest, _l=label):
            """UploadStream answers the byte count (8 B, timed); UploadStreamCheck the count and
            the SHA-256 of the messages as received (40 B, the untimed check, SERVER.md)."""
            w8, w40 = count.to_bytes(8, "little"), count.to_bytes(8, "little") + digest

            def v(b):
                if b != w8:
                    raise CallFailed("(d) %s: the server counted %s bytes, want %d"
                                     % (_l, int.from_bytes(b, "little") if len(b) == 8 else "%d-byte answer" % len(b), count))
                return b

            def vc(b):
                if b != w40:
                    raise CallFailed("(d) %s check: the server answered %d B (count %s), not the bytes and digest sent"
                                     % (_l, len(b), int.from_bytes(b[:8], "little") if len(b) >= 8 else None))
                return b
            return v, vc
        verdict, vcheck = verdicts(nbytes, sha)
        if PLANT == "count":
            verdict = planted(verdict, verdicts(nbytes + 1, sha)[0])
        if PLANT == "digest":
            vcheck = verdicts(nbytes, bytes(32))[1]
        key = "d:" + label
        out[key] = []
        for cell, tr, e in fam:
            msgs = ups if e == "inc" else fcs
            for g, u in zip(msgs, ups):                # correctness before timing
                if enc5[e](g) != u.SerializeToString():
                    raise CallFailed("gate: cell %s (d) %s: a chunk does not encode as upb does" % (cell, label))
            if tr == "grpc":
                st, stc = (chan(cell).stream_unary(p, request_serializer=enc5[e], response_deserializer=ident,
                                                   _registered_method=True) for p in (STREAM, STREAM_CHECK))

                def grpc_stream(check=False, _st=st, _stc=stc, _m=msgs, _v=verdict, _vc=vcheck):
                    # grpcio consumes a request iterator on a thread of its own per call, so a
                    # D or F stream encodes on a fresh thread each call: one encode context
                    # per call there (counted, and allowed for in the per-thread check)
                    GRPC_STREAM_CALLS[0] += 1
                    if check:
                        return _vc(_stc(iter(_m)))
                    return _v(_st(iter(_m)))
                out[key].append((cell, grpc_stream))
            else:
                c = ccli(cell)

                mv = (e == "C-retain") if (e.startswith("C-") and not cell.startswith("Cc-")) else None

                def core_stream(check=False, _c=c, _e=enc5[e], _m=msgs, _v=verdict, _vc=vcheck, _mv=mv):   # bound per payload
                    h = arms._ffi.call_open(_c, STREAM_CHECK if check else STREAM)
                    n = len(_m)
                    try:
                        for i, g in enumerate(_m):
                            if _mv is not None:        # C: ak_call_send_enc, the move path
                                arms._ffi.encode("cext", r5, g, None, _mv, (h, i + 1 == n))
                            else:
                                arms._ffi.call_send(h, _e(g), i + 1 == n)
                    except Exception:
                        arms._ffi.call_cancel(h)
                        raise
                    return (_vc if check else _v)(arms._ffi.call_recv(h))
                out[key].append((cell, core_stream))
    keep = (chans, clis)
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
                else arms._ffi.encode("cext", root, o, None, name.endswith("-retain")))
        if back != ref and R.FromString(back) != R.FromString(ref):
            raise CallFailed("gate: cell %s (a) does not re-encode to P2.2" % name)
    # (c) and (d): one call per cell; (d) through UploadStreamCheck (the server's byte count and
    # digest), then through the timed UploadStream (its byte count)
    for key in [k for k in cs if k[:2] in ("c:", "d:")]:
        for name, fn in cs[key]:
            if key.startswith("d:"):
                fn(check=True)
            fn()
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
GRPC_STREAM_CALLS = [0]


def dir_plan(key, calls):
    """(direction, payload, in-flight values, calls per sample) of a cells() key. (c) and (d)
    run at 1 and 8 in flight; (d) makes a third of the calls (a 16 MiB upload per call)."""
    if key.startswith("c:"):
        return "c", key[2:], UP_INFLIGHT, calls
    if key.startswith("d:"):
        return "d", key[2:], UP_INFLIGHT, max(1, -(-calls // 3))
    return key, PID, INFLIGHT, calls


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


# the shared server's start, warm and stop, and its socket list (no heavy import: the pyperf
# master process uses it too)
from camp_rpc_srv import Server, parse_server, serve_sh, timed_target, tcp_port  # noqa: E402,F401


def start_server(d, warm=0):
    s = Server(d, warm)
    return s, s.info


def os_threads():
    try:
        return len(os.listdir("/proc/self/task"))
    except OSError:
        return -1


if __name__ == "__main__":
    sys.exit("camp_rpc.py is the RPC grid's library (cells, channels, checks); the grid runs on pyperf: "
             "camp_rpc_pyperf.py (CAMPAIGN req 22a as amended, FIX-PLAN WP9)")
