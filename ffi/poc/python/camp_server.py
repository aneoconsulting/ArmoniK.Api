"""CAMPAIGN.md 13: the RPC grid's server, a SEPARATE process pinned to AK_CPU_SERVER.

  python3.12 camp_server.py --dir DIR [--workers N]

CAMPAIGN req 13 as amended (R-H33): ONE server process per launch, serving every cell of both
builds. It hosts the two transport configurations of req 17 as two grpcio servers in this one
process, each on a Unix domain socket (req 17 as amended, R-H28): DIR/shipped.sock and
DIR/pinned.sock. Prints `SOCKETS shipped=unix:... pinned=unix:... AFFINITY ... WORKERS n` once
both listen, and serves until its stdin closes.

  /ffi.Bench/Get   direction (a): an empty request; returns P2.2's PRE-SERIALISED bytes, so
                   the server's work is identical whatever cell the client is
  /ffi.Bench/Put   direction (b): a P2.2-sized request that the server DECODES (upb
                   FromString, the production path) and checks (500 tasks); empty response
Every request is checked; a wrong one is answered INVALID_ARGUMENT, which fails the client's
call and aborts its run (requirement 18). The server never loads the core.
"""
import os
import sys
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import camp_lib as L  # noqa: E402

AFFINITY = L.pin("AK_CPU_SERVER")

import grpc  # noqa: E402
from concurrent import futures  # noqa: E402

TAG = "py%d.%d" % sys.version_info[:2]
for p in (os.path.join(HERE, "mech", "build", "pb2"), os.path.join(HERE, "build", TAG, "pb2")):
    if os.path.exists(os.path.join(p, "shapes_pb2.py")):
        sys.path.insert(0, p)
import shapes_pb2  # noqa: E402
import payload_values as V  # noqa: E402

PID = "P2.2"
WINDOW = 4 << 20
MSG_LIMIT = 16 << 20


def options(transport):
    """The server half of requirement 17. `shipped`: grpcio's defaults (packages/python
    configures no server or channel option beyond an authority). `pinned`: a 4 MiB stream
    window with BDP probing OFF (grpcio's lookahead is a floor while BDP runs; log 80, which showed it, was deleted under R-C9),
    message limits raised. grpcio has no connection-window argument and sets TCP_NODELAY
    itself; both stated in the client's header."""
    if transport == "shipped":
        return []
    return [("grpc.http2.lookahead_bytes", WINDOW), ("grpc.http2.bdp_probe", 0),
            ("grpc.max_receive_message_length", MSG_LIMIT),
            ("grpc.max_send_message_length", MSG_LIMIT)]


def main():
    d = sys.argv[sys.argv.index("--dir") + 1]
    workers = int(sys.argv[sys.argv.index("--workers") + 1]) if "--workers" in sys.argv else 32
    body = V.reference(PID)
    R = shapes_pb2.ListTasksDetailedResponse

    plant = os.environ.get("AK_CAMP_PLANT", "")
    count = [0]

    def get(req, ctx):
        if req:
            ctx.abort(grpc.StatusCode.INVALID_ARGUMENT, "Get takes an empty request")
        count[0] += 1
        # The must-fail control of requirement 18 (run_campaign.sh --suite gate): one short
        # body in 50, which every cell must refuse and which must abort the whole run.
        if plant == "short" and count[0] % 50 == 0:
            return body[:-1]
        return body

    def put(req, ctx):
        if len(req) != len(body):
            ctx.abort(grpc.StatusCode.INVALID_ARGUMENT, "Put: %d bytes, want %d" % (len(req), len(body)))
        m = R.FromString(req)
        if len(m.tasks) != 500:
            ctx.abort(grpc.StatusCode.INVALID_ARGUMENT, "Put: %d tasks" % len(m.tasks))
        return b""

    ident = (lambda b: b)
    h = {"Get": grpc.unary_unary_rpc_method_handler(get, request_deserializer=ident, response_serializer=ident),
         "Put": grpc.unary_unary_rpc_method_handler(put, request_deserializer=ident, response_serializer=ident)}
    srvs, socks = [], []
    for transport in ("shipped", "pinned"):
        path = os.path.join(d, transport + ".sock")
        if os.path.exists(path):
            os.unlink(path)
        srv = grpc.server(futures.ThreadPoolExecutor(max_workers=workers), options=options(transport))
        srv.add_generic_rpc_handlers((grpc.method_handlers_generic_handler("ffi.Bench", h),))
        if not srv.add_insecure_port("unix:" + path):
            raise SystemExit("cannot listen on unix:" + path)
        srv.start()
        srvs.append(srv)
        socks.append("%s=unix:%s" % (transport, path))
    print("SOCKETS %s AFFINITY %s WORKERS %d THREADS %d" % (" ".join(socks), ",".join(map(str, AFFINITY)), workers,
                                                           threading.active_count()), flush=True)
    done = threading.Event()

    def watch():
        sys.stdin.read()
        done.set()
    threading.Thread(target=watch, daemon=True).start()
    done.wait()
    print("SERVED %d Get calls" % count[0], flush=True)
    for srv in srvs:
        srv.stop(0).wait()


if __name__ == "__main__":
    main()
