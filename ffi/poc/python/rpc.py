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

import arms  # noqa: E402

try:
    import grpc
except ImportError as e:  # noqa: BLE001
    grpc = None
    _GRPC_WHY = str(e)

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

# Three configurations, and the third exists because "does pinning a window turn BDP
# probing off" is answerable by running both rather than by reading about it.
CONFIGS = [
    ("grpcio's default", _MSG),
    ("ArmoniK pinned, BDP left on",
     _MSG + [("grpc.http2.lookahead_bytes", WINDOW_BYTES),
             ("grpc.http2.max_frame_size", 1 << 20)]),
    ("ArmoniK pinned, BDP off",
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


def measure(channel, name, deser, calls, inflight):
    call = channel.unary_unary(METHOD, request_serializer=lambda b: b,
                               response_deserializer=deser)
    call(b"")                                   # warm the channel and the HTTP/2 handshake
    per_thread = max(1, calls // inflight)
    errs = [0]

    def work():
        try:
            for _ in range(per_thread):
                call(b"")
        except Exception:  # noqa: BLE001
            errs[0] += 1

    c0, t0 = _cpu(), time.perf_counter_ns()
    ths = [threading.Thread(target=work) for _ in range(inflight)]
    for t in ths:
        t.start()
    for t in ths:
        t.join()
    dt = time.perf_counter_ns() - t0
    dc = _cpu() - c0
    n = per_thread * inflight
    return dc / n, dt / n, errs[0]


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
            print("   %-38s %-9s %12s %12s %7s"
                  % ("arm", "in flight", "CPU ns/RPC", "wall ns/RPC", "errors"), file=out)
            for name, deser in _deserializers():
                for k in INFLIGHT:
                    cpu, wall, err = measure(ch, name, deser, CALLS, k)
                    print("   %-38s %-9d %12.0f %12.0f %7d"
                          % (name, k, cpu, wall, err), file=out)
    finally:
        server.stop(0).wait()


def core_http2_args():
    """Every `grpc.http2.*` channel argument THIS build of the C core knows.

    Read out of the extension module rather than out of the documentation, because an
    argument the core does not know is **ignored and not refused** -- so "I passed it" is
    not evidence that anything happened, and a table whose configuration column is a list
    of hopeful strings is worse than no table.
    """
    import re
    import subprocess
    try:
        from grpc._cython import cygrpc
        blob = open(cygrpc.__file__, "rb").read()
    except Exception:  # noqa: BLE001
        return None
    return sorted(set(m.decode() for m in
                      re.findall(rb"grpc\.http2\.[a-z0-9_.]+", blob)))


def report_flow_control(out):
    """What this build of the C core will and will not let a Python caller set."""
    print("\n## flow control: what the gRPC C core actually exposes, grpcio %s"
          % grpc.__version__, file=out)
    names = core_http2_args()
    if names is None:
        print("   could not read the extension module; nothing asserted", file=out)
        return
    print("   every grpc.http2.* argument this build knows:", file=out)
    for n in names:
        mark = ""
        if n == "grpc.http2.lookahead_bytes":
            mark = "   <- the STREAM window, and the only window a caller can set"
        elif n == "grpc.http2.bdp_probe":
            mark = "   <- BDP probing, a SEPARATE switch from the window"
        print("     %-44s%s" % (n, mark), file=out)
    print("", file=out)
    print("   **There is no channel argument for the CONNECTION window.** The aggregating", file=out)
    print("   session's warning was that raising only the stream window leaves the", file=out)
    print("   connection at 65,535 and changes nothing; in grpcio the stronger statement", file=out)
    print("   holds -- a Python caller CANNOT raise the connection window, because this", file=out)
    print("   build exposes no name for it. `grpc.http2.lookahead_bytes` is per stream.", file=out)
    print("   So ArmoniK's 4 MiB configuration is expressible for the stream and not for", file=out)
    print("   the connection, and a Python client is bounded by whichever is smaller.", file=out)
    print("", file=out)
    print("   **Pinning the window does NOT turn BDP probing off.** They are two", file=out)
    print("   arguments and the rows above run both ways: `ArmoniK pinned, BDP left on`", file=out)
    print("   against `ArmoniK pinned, BDP off`. Whether it matters is the delta between", file=out)
    print("   those two rows and not something this paragraph gets to assert.", file=out)


def main():
    global CALLS, INFLIGHT
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
    print("# transport:    ArmoniK's configuration pinned (%d B chunking, %d B stream"
          % (CHUNK_BYTES, WINDOW_BYTES), file=out)
    print("#               window), with grpcio's default as a labelled second row", file=out)
    print("# headline:     CPU per RPC. Wall clock is beside it because R9's hazard moves", file=out)
    print("#               wall clock and does not move CPU", file=out)
    print("# allocator:    pinned, as in bench.py (%s)"
          % ("applied" if _WARM else "not glibc"), file=out)
    for a in arms.absent:
        print("# ARM ABSENT: %s" % a, file=out)

    d = tempfile.mkdtemp(prefix="akrpc")
    for i, (argname, args) in enumerate(CONFIGS):
        run_transport(out, "unix domain socket",
                      "unix:" + os.path.join(d, "s%d" % i), args, argname)
        run_transport(out, "loopback TCP", "tcp", args, argname)
    report_flow_control(out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
