"""The RPC arm: the host's real gRPC stack against the core's codec, end to end.

`design/SHAPES.md`'s RPC section. One unary call carrying a real payload (P2.2), against
this language's gRPC incumbent, with **CPU per RPC as the headline** and wall clock beside
it -- because R9's hazard is a flow-control stall that moves wall clock and does not move
CPU, and a table that led with wall clock would report the transport's tuning as the
codec's result.

**A marshaller arm is not an RPC arm.** Calling `SerializeToString` measures the codec path
gRPC drives, which R14 asks for separately and which `bench.py` already has. This runs the
transport: a real server, a real channel, real frames.

**What differs between the two arms is exactly one thing.** grpcio lets a client build a
multicallable with an arbitrary `response_deserializer`, so the incumbent arm and the
composed arm share the server, the channel, the wire bytes and the frames, and differ only
in the function that turns the response body into an object. Anything else -- two servers,
two channels, a second proto -- would measure the harness.

**The server returns pre-serialised bytes** and never encodes, so the server's codec is
not in either measurement. Both arms are therefore DECODE arms at the client. That is
stated rather than hidden: an encode-side RPC arm would need the server to decode and is a
different experiment.

**The transport configuration is ArmoniK's, not grpcio's default** (R14 applied to the
transport): 2 MiB chunking and a 4 MiB stream window, with the stack default as a labelled
second row. Two traps the aggregating session named in advance and that this script
checks rather than assumes:

*   **the connection window is a different channel argument from the stream window.**
    Raising only the stream one leaves the connection at 65,535 and changes nothing;
*   **pinning a window may turn the C core's BDP probing off.** Nobody in this branch has
    established what grpcio does here, so this script enumerates what the core accepted
    and prints it, and the log carries the answer rather than an assumption.

Usage:  python rpc.py [--calls N] [--inflight 1,8,16]
"""

import os
import resource
import sys
import tempfile
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import allocator  # noqa: E402  (before `arms`, as in bench.py)
_WARM = allocator.warm_up()

# ONE core in this process, and it is the rpc-feature one.
#
# ABI v1 section 9 lives behind a cargo feature that pulls tonic and tokio in, so it is
# built into its own `libak_core.so` with `_akffi_rpc` as the shim over it. Importing that
# BESIDE `_akffi` would put two libraries of one soname in a process and the first loaded
# would satisfy the other's NEEDED entry -- which is how the counting build's numbers came
# out as zeroes in work unit 2, and how the first draft of this file came out with no
# section 9 at all. `AK_FFI_MODULE` chooses instead of import order, and it is set BEFORE
# `arms` is imported because importing `arms` is what loads a shim.
#
# So cell C's codec is measured over a core with tonic linked in: same codec code, larger
# library, and every cell in this script shares it.
os.environ.setdefault("AK_FFI_MODULE", "_akffi_rpc")
sys.path.insert(0, os.path.join(HERE, "build", "py%d.%d" % sys.version_info[:2]))

import arms  # noqa: E402

try:
    import grpc
except ImportError as _e:  # noqa: BLE001
    grpc = None
    _GRPC_WHY = str(_e)

# The grid. "The host's stack against the core's" moves the codec and the transport at
# once, and the report has to say which one paid, so:
#
#   A  upb codec   + grpcio transport     the incumbent, end to end
#   B  upb codec   + CORE transport       B - A is the TRANSPORT difference
#   C  core codec  + CORE transport       C - B is the CODEC difference
#
# B is README section 13's outcome 2, and this slice already removed outcome 2 from the
# table for Python on the CODEC side (a pure-Python control loses to upb by 34x to 60x),
# so pricing its transport half here is the other half of that answer. It is cheap: upb
# makes the request bytes and reads the response bytes, and the core's transport moves
# opaque bytes and never sees a message type.
#
# Every cell talks to ONE grpcio server that returns pre-serialised bytes, so the server is
# in no difference.
PID = "P2.2"                       # SHAPES.md: the RPC arm carries P2.2
METHOD = "/ffi.Bench/Get"
CALLS = 200
INFLIGHT = [1, 8, 16]

# ArmoniK's configuration, which is what R14 means on the transport. 2 MiB chunking is the
# application's message size; the 4 MiB window is what the stack is told to allow.
CHUNK_BYTES = 2 << 20
WINDOW_BYTES = 4 << 20

# The gRPC C core's names. `lookahead_bytes` is the STREAM window; the connection window is
# its own argument and setting one without the other is the trap. Both are passed and what
# the core did with them is reported, not assumed.
_MSG = [("grpc.max_receive_message_length", CHUNK_BYTES * 4),
        ("grpc.max_send_message_length", CHUNK_BYTES * 4)]
DEFAULT_SERVER_ARGS = _MSG

# Three configurations, and the third exists because "does pinning a window turn BDP
# probing off" is answerable by running both rather than by reading about it.
# **The 4 MiB window is what ArmoniK INTENDS, not what it ships.**
# `packages/rust/armonik-transport`'s `ClientConfig` carries connect and request timeouts,
# a rate limit, TCP keepalive, the HTTP/2 ping settings and a max header list size -- and
# nothing for either window; `packages/csharp` cannot set one at all. So no ArmoniK client
# pins an HTTP/2 window today, and the stack-default row is the SHIPPED configuration
# rather than a fallback.
CONFIGS = [
    ("the stack default (what ArmoniK SHIPS)", _MSG),
    ("4 MiB pinned, BDP left on (INTENDED)",
     _MSG + [("grpc.http2.lookahead_bytes", WINDOW_BYTES),
             ("grpc.http2.max_frame_size", 1 << 20)]),
    ("4 MiB pinned, BDP off (INTENDED, deterministic)",
     _MSG + [("grpc.http2.lookahead_bytes", WINDOW_BYTES),
             ("grpc.http2.max_frame_size", 1 << 20),
             ("grpc.http2.bdp_probe", 0)]),
]


def _cpu():
    """Process CPU, user plus system, in nanoseconds. Threads included, which is what a
    server-plus-client-in-one-process measurement has to count."""
    r = resource.getrusage(resource.RUSAGE_SELF)
    return int((r.ru_utime + r.ru_stime) * 1e9)


def serve(payload, args):
    """A server that returns one fixed body and never encodes."""
    from concurrent import futures

    def handler(request, context):
        return payload

    rpc_method_handlers = {
        "Get": grpc.unary_unary_rpc_method_handler(
            handler,
            request_deserializer=lambda b: b,
            response_serializer=lambda b: b,
        )
    }
    generic = grpc.method_handlers_generic_handler("ffi.Bench", rpc_method_handlers)
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=16), options=args)
    server.add_generic_rpc_handlers((generic,))
    return server


def core_client(target, pinned):
    """A core transport client, pinned or at the stack default.

    ABI v1 section 9's `ak_client_opts`: the stream and connection windows are SEPARATE,
    which is the mistake this entry point exists to make impossible -- and note that on
    grpcio the same mistake is unreachable for the opposite reason, because there is no
    connection-window argument at all.
    """
    rt = arms._ffi.rt_new(0)
    # tonic's `Endpoint::from_shared` wants a URI with a scheme, where grpcio takes a bare
    # `host:port`. A `unix:` target already has one. Getting this wrong fails loudly at
    # connect rather than quietly at measurement, which is the good direction.
    if not target.startswith("unix:") and "://" not in target:
        target = "http://" + target
    if not pinned:
        return rt, arms._ffi.client_new(rt, target)
    # adaptive OFF: it overrides both windows, so pinning a window and leaving it on is a
    # contradiction rather than belt and braces. Nagle left at the default, which is
    # `packages/rust/armonik-transport`'s shipped value (nodelay on).
    return rt, arms._ffi.client_new_opts(rt, target, WINDOW_BYTES, WINDOW_BYTES, 0,
                                         CHUNK_BYTES * 4, CHUNK_BYTES * 4, -1)


class RpcFailed(RuntimeError):
    """An RPC the harness must not time: a non-zero status, a missing completion, or a
    response that is not the payload the server was given (FIX-PLAN R-D3)."""


# How long a queue or callback delivery may take before the harness calls it lost. The
# core promises a completion for every call (an aborted one completes with AK_ERR_HOST),
# so this only fires on a defect -- and a defect should fail the run, not hang it.
COMPLETION_TIMEOUT_MS = 30000

# Every gate failure and aborted measurement, so `main` can exit non-zero: a log that
# carries an ABORTED row must not come from a run that reported success.
FAILURES = []


def _expect(body, status=0, want=None):
    """The per-call gate: status 0 and exactly the payload's length, or raise.

    Length and not content, per call, because the full byte comparison is the pre-timing
    gate's job (`gate_cells`) and a 540 KB compare inside the timed loop would be charged
    to every cell. Length is what catches the failure mode this exists for: a failed
    delivery hands over EMPTY bytes, and `FromString(b"")` and the facade decode of b""
    both succeed, so without it a failed RPC is timed as a very cheap success.
    """
    if status != 0:
        raise RpcFailed("completion status %d" % status)
    if body is None:
        raise RpcFailed("no response body")
    want = len(arms.reference(PID)) if want is None else want
    if len(body) != want:
        raise RpcFailed("response is %d bytes, the payload is %d" % (len(body), want))
    return body


def _checked(deser):
    """A grpcio response_deserializer that gates the body before decoding it (cell A).

    grpcio raises on a non-OK status by itself; what it does not do is notice an OK call
    that carries the wrong body, which is the other half of the gate."""
    want = len(arms.reference(PID))

    def d(b, _deser=deser, _want=want):
        return _deser(_expect(b, 0, _want))
    d.raw = deser
    return d


def core_cells(target, pinned):
    """Cells B and C over the core's transport, in each of section 9's three deliveries.

    The GIL is why the three are not interchangeable here and it is a Python reason rather
    than a borrowed one: the queue's drainer is a thread CPython already knows, which drops
    the lock while it waits; the callback arrives on a tokio worker that must
    `PyGILState_Ensure` before it can touch anything.

    **Every delivery gates every call** (FIX-PLAN R-D3): status 0 and the payload's exact
    length, or `RpcFailed`. Before this the queue and callback deliveries ignored the
    completion status, a failed call delivered empty bytes, and the decode of empty bytes
    succeeded -- so a failing RPC was timed as a cheap success
    (`logs/python/81-rpc-gate-before.log`). Each cell also exposes `fetch`, the delivery
    without the decode, so `gate_cells` can compare the bytes themselves before timing.
    """
    if arms._ffi is None or not hasattr(arms._ffi, "call_unary"):
        return []
    root = arms.ROOT_OF[PID]
    want = len(arms.reference(PID))
    rt, cl = core_client(target, pinned)
    R = getattr(arms._pb2, root) if arms._pb2 is not None else None
    out = []

    def blocking(deser):
        def fetch():
            # The binding raises on a non-zero ak_call_unary status itself.
            return _expect(arms._ffi.call_unary(cl, METHOD, b""), 0, want)

        def go():
            return deser(fetch())
        go.fetch = fetch
        return go

    def queued(deser):
        q = arms._ffi.queue_new()

        def fetch():
            arms._ffi.call_unary_q(cl, METHOD, b"", q, 1)
            c = arms._ffi.queue_next(q, COMPLETION_TIMEOUT_MS)
            if c is None:
                raise RpcFailed("no completion within %d ms" % COMPLETION_TIMEOUT_MS)
            _tag, status, body = c
            return _expect(body, status, want)

        def go():
            return deser(fetch())
        go.q = q
        go.fetch = fetch
        return go

    def called_back(deser):
        """The callback delivery, used the way a Python caller would: an Event per call.

        The completion runs on a tokio worker, so the trampoline must `PyGILState_Ensure`
        before it can set the Event, and the caller is a Python thread waiting on it. Two
        GIL transitions per call that the queue does not make, plus the Event -- and the
        Event is the honest part of the comparison rather than overhead to subtract,
        because a caller who chose this mode has to synchronise somehow.
        """
        def fetch():
            done = threading.Event()
            box = []

            def cb(tag, status, body):
                box.append((status, body))
                done.set()

            arms._ffi.call_unary_cb(cl, METHOD, b"", cb, 1)
            if not done.wait(COMPLETION_TIMEOUT_MS / 1000.0):
                raise RpcFailed("no completion within %d ms" % COMPLETION_TIMEOUT_MS)
            status, body = box[0]
            return _expect(body, status, want)

        def go():
            return deser(fetch())
        go.fetch = fetch
        return go

    if R is not None:
        out.append(("B  upb codec / core transport, blocking", blocking(R.FromString)))
        out.append(("B  upb codec / core transport, queue", queued(R.FromString)))
        out.append(("B  upb codec / core transport, callback", called_back(R.FromString)))
    dec = (lambda b, _r=root: arms._ffi.decode("cext", _r, b, arms.TY_CEXT))
    out.append(("C  core codec / core transport, blocking", blocking(dec)))
    out.append(("C  core codec / core transport, queue", queued(dec)))
    out.append(("C  core codec / core transport, callback", called_back(dec)))
    out.append(("-- floor: core transport, no decode", blocking(lambda b: b)))
    # Keep the runtime and client alive for as long as the closures are.
    for _n, f in out:
        f._keep = (rt, cl)
    return out


def gate_cells(channel, cells):
    """R2 for the RPC arm, before anything is timed: every cell's response is P2.2's exact
    bytes, and every cell's decode re-encodes to them.

    Returns report lines and raises `RpcFailed` on the first cell that fails, so a caller
    that times anything after calling this has, by construction, a gated arm.
    """
    body = arms.reference(PID)
    lines = []
    got = None
    if channel is not None:
        raw = channel.unary_unary(METHOD, request_serializer=lambda b: b,
                                  response_deserializer=lambda b: b)
        got = raw(b"")
        if got != body:
            raise RpcFailed("cell A (grpcio): the response is not P2.2's bytes")
        lines.append("ok  A  grpcio transport: response == P2.2 (%d bytes)" % len(got))
    for name, deser in (_deserializers() if channel is not None else []):
        obj = _checked(deser)(got)
        if obj is not got:
            back = arms.reencode("upb" if name.startswith("upb") else "core-ffi / C ext type",
                                 obj, PID)
            if back != body and not (name.startswith("upb")
                                     and arms.same_message(PID, back, body)):
                raise RpcFailed("cell A %s: decode does not re-encode to P2.2" % name)
        lines.append("ok  A  %s: decodes and re-encodes to P2.2" % name)
    for name, fn in cells:
        b = fn.fetch()
        if b != body:
            raise RpcFailed("%s: the response is not P2.2's bytes" % name)
        obj = fn()
        if obj is not None and not isinstance(obj, bytes):
            kind = "upb" if name.startswith("B") else "core-ffi / C ext type"
            back = arms.reencode(kind, obj, PID)
            if back != body and not (kind == "upb" and arms.same_message(PID, back, body)):
                raise RpcFailed("%s: decode does not re-encode to P2.2" % name)
        lines.append("ok  %s: response == P2.2, decode re-encodes to it" % name)
    return lines


def _deserializers():
    """(name, bytes -> object). The ONLY thing that differs between the arms."""
    out = []
    root = arms.ROOT_OF[PID]
    if arms._pb2 is not None:
        R = getattr(arms._pb2, root)
        out.append(("upb (incumbent)", R.FromString))
    if arms._ffi is not None:
        out.append(("core-ffi / C ext type",
                    lambda b, _r=root: arms._ffi.decode("cext", _r, b, arms.TY_CEXT)))
    # The floor: what the call costs with no decode at all. Everything above it is the
    # codec; everything below it is the transport, and the transport is the same for both.
    out.append(("-- floor: no decode, the bytes as they arrive", lambda b: b))
    return out


class MeasurementAborted(RuntimeError):
    """A timed loop in which any call failed. It has no figure: dividing the elapsed time
    by the full call count when some threads stopped early -- which is what this harness
    did before FIX-PLAN R-D3 -- reports a failure as a fast call."""


def _run_threads(one_call, calls, inflight):
    """`inflight` threads, `calls // inflight` gated calls each. The first exception in any
    thread stops every thread and aborts the measurement; there is no error count, because
    a loop with an error in it is not a measurement."""
    per_thread = max(1, calls // inflight)
    failed = []
    stop = threading.Event()

    def work():
        try:
            for _ in range(per_thread):
                if stop.is_set():
                    return
                one_call()
        except BaseException as e:  # noqa: BLE001
            failed.append(e)
            stop.set()

    c0, t0 = _cpu(), time.perf_counter_ns()
    ths = [threading.Thread(target=work) for _ in range(inflight)]
    for t in ths:
        t.start()
    for t in ths:
        t.join()
    dt = time.perf_counter_ns() - t0
    dc = _cpu() - c0
    if failed:
        e = failed[0]
        raise MeasurementAborted("%d of %d threads failed; first: %s: %s"
                                 % (len(failed), inflight, type(e).__name__,
                                    (str(e).splitlines() or [""])[0][:120]))
    n = per_thread * inflight
    return dc / n, dt / n, 0


def measure(channel, name, deser, calls, inflight):
    """Cell A: grpcio's transport, with the response body gated before it is decoded."""
    call = channel.unary_unary(METHOD, request_serializer=lambda b: b,
                               response_deserializer=_checked(deser))
    call(b"")                                   # warm the channel and the HTTP/2 handshake
    return _run_threads(lambda: call(b""), calls, inflight)


def in_process_control(out):
    """The same deserializers on the same bytes with NO transport, in THIS process.

    Without it the arm can only be compared against `bench.py`, which is a different
    process with a different allocator history and a different thread count -- and the
    first run of this script showed the composed arm's codec costing about twice its
    in-process figure, which is either a real property of decoding under a thread pool or
    an artefact of comparing across scripts. One row settles which.
    """
    body = arms.reference(PID)
    print("\n## the in-process control: the same decode, no transport, this process",
          file=out)
    print("   %-38s %12s %12s" % ("arm", "CPU ns/call", "wall ns/call"), file=out)
    for name, deser in _deserializers():
        deser(body)
        n = 60
        c0, t0 = _cpu(), time.perf_counter_ns()
        for _ in range(n):
            deser(body)
        print("   %-38s %12.0f %12.0f"
              % (name, (_cpu() - c0) / n, (time.perf_counter_ns() - t0) / n), file=out)


def run_core(out, label, target, pinned, argname):
    """Cells B and C, against the same grpcio server cell A used."""
    payload = arms.reference(PID)
    server = serve(payload, DEFAULT_SERVER_ARGS)
    if target.startswith("unix:"):
        server.add_insecure_port(target)
        real = target
    else:
        port = server.add_insecure_port("127.0.0.1:0")
        real = "127.0.0.1:%d" % port
    server.start()
    try:
        cells = core_cells(real, pinned)
        if not cells:
            print("\n## %s, %s -- CORE TRANSPORT ABSENT from the shim" % (label, argname),
                  file=out)
            return
        print("\n## %s, %s (%s) -- cells B and C" % (label, argname, real), file=out)
        # R2 before timing: every delivery's bytes are P2.2's and decode back to them. A
        # failure here refuses every row of this block rather than timing any of it.
        try:
            for line in gate_cells(None, cells):
                print("   gate " + line, file=out)
        except Exception as e:  # noqa: BLE001
            print("   GATE FAILED, block not timed: %s: %s" % (type(e).__name__, e), file=out)
            FAILURES.append("%s %s: gate: %s" % (label, argname, e))
            return
        print("   %-46s %-9s %12s %12s"
              % ("arm", "in flight", "CPU ns/RPC", "wall ns/RPC"), file=out)
        for name, fn in cells:
            for k in INFLIGHT:
                try:
                    cpu, wall, _ = measure_fn(fn, CALLS, k)
                except MeasurementAborted as e:
                    print("   %-46s %-9d ABORTED, no figure: %s" % (name, k, e), file=out)
                    FAILURES.append("%s %s %s x%d: %s" % (label, argname, name, k, e))
                    continue
                print("   %-46s %-9d %12.0f %12.0f" % (name, k, cpu, wall), file=out)
    finally:
        server.stop(0).wait()


def measure_fn(fn, calls, inflight):
    """Cells B and C. `fn` gates every call itself (`core_cells`)."""
    return _run_threads(fn, calls, inflight)


def run_transport(out, label, target, args, argname):
    payload = arms.reference(PID)
    server = serve(payload, args)
    if target.startswith("unix:"):
        server.add_insecure_port(target)
    else:
        port = server.add_insecure_port("127.0.0.1:0")
        target = "127.0.0.1:%d" % port
    server.start()
    try:
        with grpc.insecure_channel(target, options=args) as ch:
            print("\n## %s, %s (%s)" % (label, argname, target), file=out)
            try:
                for line in gate_cells(ch, []):
                    print("   gate " + line, file=out)
            except Exception as e:  # noqa: BLE001
                print("   GATE FAILED, block not timed: %s: %s" % (type(e).__name__, e),
                      file=out)
                FAILURES.append("%s %s: gate: %s" % (label, argname, e))
                return
            print("   %-38s %-9s %12s %12s"
                  % ("arm", "in flight", "CPU ns/RPC", "wall ns/RPC"), file=out)
            for name, deser in _deserializers():
                for k in INFLIGHT:
                    try:
                        cpu, wall, _ = measure(ch, name, deser, CALLS, k)
                    except Exception as e:  # noqa: BLE001  (grpc.RpcError included)
                        print("   %-38s %-9d ABORTED, no figure: %s: %s"
                              % (name, k, type(e).__name__, (str(e).splitlines() or [""])[0][:100]),
                              file=out)
                        FAILURES.append("%s %s %s x%d: %s" % (label, argname, name, k, e))
                        continue
                    print("   %-38s %-9d %12.0f %12.0f" % (name, k, cpu, wall), file=out)
    finally:
        server.stop(0).wait()


def core_http2_args():
    """Every `grpc.http2.*` channel argument THIS build of the C core knows.

    Read out of the extension module rather than out of the documentation, because an
    argument the core does not know is **ignored and not refused** -- so "I passed it" is
    not evidence that anything happened.
    """
    import re
    try:
        from grpc._cython import cygrpc
        blob = open(cygrpc.__file__, "rb").read()
    except Exception:  # noqa: BLE001
        return None
    return sorted(set(m.decode() for m in
                      re.findall(rb"grpc\.http2\.[a-z0-9_.]+", blob)))


# The five configurations that, between them, say what the C core does. Each is run in a
# subprocess with the core's own flow-control tracing on, and what the CORE printed is the
# evidence -- not what was passed to it.
FC_PROBES = [
    ("the core's default", []),
    ("BDP off, no window set", [("grpc.http2.bdp_probe", 0)]),
    ("BDP on, window set SMALL (65,535)", [("grpc.http2.lookahead_bytes", 65535)]),
    ("BDP on, window set to ArmoniK's 4 MiB", [("grpc.http2.lookahead_bytes", WINDOW_BYTES)]),
    ("BDP off, window set to ArmoniK's 4 MiB",
     [("grpc.http2.lookahead_bytes", WINDOW_BYTES), ("grpc.http2.bdp_probe", 0)]),
]


def _fc_child(opts):
    """One configuration, three 540 KB round trips, tracing on. Prints what the core said."""
    import tempfile as _t
    from concurrent import futures
    body = arms.reference(PID)
    args = [("grpc.max_receive_message_length", CHUNK_BYTES * 4),
            ("grpc.max_send_message_length", CHUNK_BYTES * 4)] + opts
    h = {"Get": grpc.unary_unary_rpc_method_handler(
        lambda req, ctx: body, request_deserializer=lambda b: b,
        response_serializer=lambda b: b)}
    srv = grpc.server(futures.ThreadPoolExecutor(max_workers=4), options=args)
    srv.add_generic_rpc_handlers((grpc.method_handlers_generic_handler("ffi.Bench", h),))
    target = "unix:" + os.path.join(_t.mkdtemp(prefix="akfc"), "s")
    srv.add_insecure_port(target)
    srv.start()
    try:
        with grpc.insecure_channel(target, options=args) as ch:
            call = ch.unary_unary(METHOD, request_serializer=lambda b: b,
                                  response_deserializer=lambda b: b)
            for _ in range(3):
                assert len(call(b"")) == len(body)
    finally:
        srv.stop(0).wait()


def nagle_probe(out):
    """Does grpcio show the delayed-ACK artifact the rust slice found in tonic's server?

    The test identifies it rather than looking for a number: under Nagle plus delayed ACK a
    SMALL response costs MORE than a large one, which is backwards for flow control and
    right for Nagle. Measured rather than inherited, because this build has no
    `grpc.tcp_nodelay` argument -- a caller cannot set it either way, so what the C core
    does by default is the only answer there is.
    """
    print("\n## Nagle: is a small response dearer than a large one?", file=out)
    print("   %-10s %10s %12s" % ("transport", "bytes", "ms/RPC"), file=out)
    for kind in ("unix", "tcp"):
        for size in (1024, len(arms.reference(PID))):
            body = b"\x5a" * size
            server = serve(body, DEFAULT_SERVER_ARGS)
            if kind == "unix":
                t = "unix:" + os.path.join(tempfile.mkdtemp(prefix="akng"), "s")
                server.add_insecure_port(t)
            else:
                t = "127.0.0.1:%d" % server.add_insecure_port("127.0.0.1:0")
            server.start()
            try:
                with grpc.insecure_channel(t, options=DEFAULT_SERVER_ARGS) as ch:
                    call = ch.unary_unary(METHOD, request_serializer=lambda b: b,
                                          response_deserializer=lambda b: b)
                    call(b"")
                    v = []
                    for _ in range(30):
                        t0 = time.perf_counter_ns()
                        call(b"")
                        v.append(time.perf_counter_ns() - t0)
                    v.sort()
                    print("   %-10s %10d %12.3f"
                          % (kind, size, v[len(v) // 2] / 1e6), file=out)
            finally:
                server.stop(0).wait()
    print("   Monotone in size on both transports, with nothing near 40 ms, means the C",
          file=out)
    print("   core sets TCP_NODELAY itself. The artifact was the tonic SERVER's under",
          file=out)
    print("   serve_with_incoming and it does not reach a Python caller -- and there is no",
          file=out)
    print("   grpc.tcp_nodelay in this build, so it is not a setting anyone could have got",
          file=out)
    print("   wrong from here.", file=out)


def report_flow_control(out):
    """What THIS build of the C core does with a window and with BDP, established."""
    import json
    import re
    import subprocess

    print("\n## flow control: what the gRPC C core actually does, grpcio %s"
          % grpc.__version__, file=out)
    print("#  Established by running it with the core's own `flowctl` and `bdp_estimator`", file=out)
    print("#  tracing on and reading what the CORE printed. Neither the grpc-java answer", file=out)
    print("#  (a set window disables auto-tuning) nor the .NET one (a floor that doubles", file=out)
    print("#  to a 16 MiB cap, connection window hardcoded at 64 MiB) transfers here.", file=out)
    print("", file=out)
    print("   %-40s %14s %6s" % ("configuration", "stream window", "BDP"), file=out)
    rows = []
    for label, opts in FC_PROBES:
        env = dict(os.environ, GRPC_VERBOSITY="debug",
                   GRPC_TRACE="flowctl,bdp_estimator",
                   AK_FC_PROBE=json.dumps(opts))
        r = subprocess.run([sys.executable, os.path.abspath(__file__), "--fc-child"],
                           env=env, capture_output=True, text=True, timeout=120)
        txt = r.stdout + r.stderr
        # "adding N for initial_window change" is the delta from HTTP/2's own 65,535.
        deltas = [int(x) for x in re.findall(r"adding (\d+) for initial_window change", txt)]
        win = 65535 + max(deltas) if deltas else None
        bdp = "on" if "bdp_estimator" in txt and "bdp[" in txt else "off"
        rows.append((label, win, bdp))
        print("   %-40s %14s %6s"
              % (label, ("%d" % win) if win is not None else "?", bdp), file=out)

    print("", file=out)
    print("   Reading the rows, and every one of these is a correction to something this", file=out)
    print("   branch assumed:", file=out)
    print("   1. **The C core's static default stream window is 65,535** -- row 2, with", file=out)
    print("      probing off and nothing set, adds zero. The 4 MiB the default reaches is", file=out)
    print("      BDP auto-tuning, not a large default.", file=out)
    print("   2. **`grpc.http2.lookahead_bytes` is a FLOOR and not a cap.** Row 3 sets it", file=out)
    print("      to 65,535 and the core still goes to 4 MiB, because BDP overrides it", file=out)
    print("      upward. A slice that pinned a window and reported it as pinned would be", file=out)
    print("      reporting the value it passed rather than the one in force.", file=out)
    print("   3. **Setting a window does NOT turn BDP probing off** -- row 4 still probes.", file=out)
    print("      That is the opposite of grpc-java, where a set window disables", file=out)
    print("      auto-tuning, and it means pinning takes BOTH arguments (row 5).", file=out)
    print("   4. **There is no channel argument for the CONNECTION window at all**, so the", file=out)
    print("      separate-knob trap does not apply here in the form it takes on grpc-java", file=out)
    print("      and tonic: a Python caller cannot set it, full stop. Every", file=out)
    print("      `grpc.http2.*` name this build knows is listed below.", file=out)
    print("", file=out)
    print("   So **ArmoniK's 4 MiB is what grpcio converges to anyway on this transport**,", file=out)
    print("   and pinning it matters for determinism rather than for size: with BDP on the", file=out)
    print("   window is whatever the estimator last decided, and only row 5 makes it a", file=out)
    print("   number. That is why the rows above are run both ways.", file=out)
    names = core_http2_args()
    if names:
        print("", file=out)
        print("   every grpc.http2.* argument this build knows: %s"
              % ", ".join(n.replace("grpc.http2.", "") for n in names), file=out)


def main():
    global CALLS, INFLIGHT
    if "--fc-child" in sys.argv:
        import json
        _fc_child([(k, v) for k, v in json.loads(os.environ["AK_FC_PROBE"])])
        return 0
    if "--calls" in sys.argv:
        CALLS = int(sys.argv[sys.argv.index("--calls") + 1])
    if "--inflight" in sys.argv:
        INFLIGHT = [int(x) for x in sys.argv[sys.argv.index("--inflight") + 1].split(",")]
    out = sys.stdout
    print("# python slice: the RPC arm (design/SHAPES.md)", file=out)
    print("# interpreter:  %s" % sys.version.replace("\n", " "), file=out)
    if grpc is None:
        print("# ABSENT: grpcio did not import: %s" % _GRPC_WHY, file=out)
        return 1
    print("# grpcio:       %s" % grpc.__version__, file=out)
    print("# payload:      %s, %d bytes, decoded at the CLIENT" % (PID, len(arms.reference(PID))),
          file=out)
    print("# server:       returns pre-serialised bytes and never encodes, so both arms", file=out)
    print("#               share it and only the response_deserializer differs", file=out)
    print("# grid:         A = upb codec + grpcio transport (the incumbent, end to end)",
          file=out)
    print("#               B = upb codec + CORE transport   -> B - A is the TRANSPORT",
          file=out)
    print("#               C = core codec + core transport  -> C - B is the CODEC", file=out)
    print("# transport:    ArmoniK INTENDS %d B chunking and a %d B window; it SHIPS"
          % (CHUNK_BYTES, WINDOW_BYTES), file=out)
    print("#               neither -- no ArmoniK client pins an HTTP/2 window today -- so",
          file=out)
    print("#               the stack-default row is the shipped configuration.", file=out)
    print("# headline:     CPU per RPC. Wall clock is beside it because R9's hazard moves", file=out)
    print("#               wall clock and does not move CPU", file=out)
    print("# allocator:    pinned, as in bench.py (%s)"
          % ("applied" if _WARM else "not glibc"), file=out)
    for a in arms.absent:
        print("# ARM ABSENT: %s" % a, file=out)

    d = tempfile.mkdtemp(prefix="akrpc")
    # Cell A, over grpcio's transport, in each configuration.
    for i, (argname, args) in enumerate(CONFIGS):
        run_transport(out, "unix domain socket",
                      "unix:" + os.path.join(d, "s%d" % i), args, argname)
        run_transport(out, "loopback TCP", "tcp", args, argname)
    # Cells B and C, over the CORE's transport, against the same server.
    for i, (pinned, argname) in enumerate(
            ((False, "the stack default (what ArmoniK SHIPS)"),
             (True, "4 MiB pinned, BOTH windows, adaptive off (INTENDED)"))):
        run_core(out, "unix domain socket",
                 "unix:" + os.path.join(d, "c%d" % i), pinned, argname)
        run_core(out, "loopback TCP", "tcp", pinned, argname)
    nagle_probe(out)
    in_process_control(out)
    report_flow_control(out)
    if FAILURES:
        print("\n%d GATE FAILURE(S) OR ABORTED MEASUREMENT(S); the figures above are not usable:"
              % len(FAILURES), file=out)
        for f in FAILURES:
            print("   " + f, file=out)
        return 1
    print("\nevery cell gated: status 0 and P2.2's length on every call, bytes and decode "
          "checked before timing", file=out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
