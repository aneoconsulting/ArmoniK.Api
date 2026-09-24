"""The RPC arm's gate, and the failure-injection run that shows it (FIX-PLAN R-D3).

`rpc.py` is a harness: it times cells A, B and C of the RPC grid. This script asks the
question a harness has to answer before any of its figures can be used -- **what does each
cell report when the RPC fails?** -- by making the RPC fail on purpose, four ways, and
printing what every cell and every delivery does with it. It prints NO timings. A row
says whether the harness would have produced a figure, how many errors it counted, and
what it would have divided by; never the figure.

The injections, each against the same cells `rpc.py` builds:

  healthy      the real server: every cell must return P2.2's exact bytes
  status       the server aborts every call with UNAVAILABLE
  empty        the server returns an EMPTY body with status OK. `FromString(b"")` and the
               facade decode of b"" both succeed and yield an empty message, so a cell
               that checks nothing times this as a very cheap success
  short        the server returns a well-formed but SHORTER response (250 of 500
               elements) with status OK. Decodes cleanly to the wrong answer
  closed       the server is stopped after the clients have connected, so every call
               meets a closed socket

What a gated cell must do on every injection but `healthy`: raise on the single call, and
abort the measurement rather than return a figure.

Usage:  AK_FFI_MODULE=_akffi_rpc python rpc_gate.py      (rpc.py sets it by default)
"""

import os
import sys
import tempfile
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
os.environ.setdefault("AK_FFI_MODULE", "_akffi_rpc")

import rpc  # noqa: E402  (loads arms with the rpc shim)
from rpc import arms, grpc  # noqa: E402

PID = rpc.PID
BODY = arms.reference(PID)
ROOT = arms.ROOT_OF[PID]
ELEM = arms.elem_fields(PID)[0][0]
R = getattr(arms._pb2, ROOT)
N_ELEM = len(getattr(R.FromString(BODY), ELEM))


def _short_body():
    m = R.FromString(BODY)
    del getattr(m, ELEM)[len(getattr(m, ELEM)) // 2:]
    return m.SerializeToString(deterministic=True)


def serve_injected(kind):
    """A grpcio server shaped like `rpc.serve`, with the fault planted in the handler."""
    from concurrent import futures
    short = _short_body()

    def handler(request, context):
        if kind == "status":
            context.abort(grpc.StatusCode.UNAVAILABLE, "injected by rpc_gate.py")
        if kind == "empty":
            return b""
        if kind == "short":
            return short
        return BODY

    h = {"Get": grpc.unary_unary_rpc_method_handler(
        handler, request_deserializer=lambda b: b, response_serializer=lambda b: b)}
    srv = grpc.server(futures.ThreadPoolExecutor(max_workers=16),
                      options=rpc.DEFAULT_SERVER_ARGS)
    srv.add_generic_rpc_handlers((grpc.method_handlers_generic_handler("ffi.Bench", h),))
    return srv


def describe(obj):
    """What a cell handed back, in words. Never a timing."""
    if obj is None:
        return "returned None"
    if isinstance(obj, (bytes, bytearray)):
        tag = "== P2.2" if obj == BODY else "!= P2.2"
        return "returned %d bytes (%s)" % (len(obj), tag)
    try:
        n = len(getattr(obj, ELEM))
    except Exception:  # noqa: BLE001
        return "returned %r" % type(obj).__name__
    return "returned a %s with %d %s (P2.2 has %d)" % (type(obj).__name__, n, ELEM, N_ELEM)


def _why(e):
    """One line for an exception; grpcio's carry their status code and details."""
    if grpc is not None and isinstance(e, grpc.RpcError) and hasattr(e, "code"):
        return "%s: %s %s" % (type(e).__name__, e.code().name, (e.details() or "")[:70])
    return "%s: %s" % (type(e).__name__, (str(e).splitlines() or [""])[0][:90])


def one_call(fn):
    try:
        return "OK    " + describe(fn())
    except Exception as e:  # noqa: BLE001
        return "RAISED " + _why(e)


def timed(measure, *a):
    """Run a `rpc.py` measurement at a tiny call count and report what it WOULD publish."""
    try:
        r = measure(*a)
    except Exception as e:  # noqa: BLE001
        return "ABORTED, no figure (%s)" % _why(e)[:110]
    _cpu, _wall, errs = r
    if errs:
        return "FIGURE PRODUCED although %d thread(s) failed: time divided by the full call count" % errs
    return "figure produced, no call failed"


def cells_for(target, pinned):
    """Every cell rpc.py times over the core transport, plus cell A over grpcio."""
    return rpc.core_cells(target, pinned)


def raw_binding(client, out):
    """What the BINDING hands a caller that ignores the status, one layer below rpc.py.

    Before the fix a failed completion's body was b"", which every decoder accepts as an
    empty message; after it the body is None, which no decoder accepts."""
    f = arms._ffi
    rt, cl = client
    q = f.queue_new()
    f.call_unary_q(cl, rpc.METHOD, b"", q, 7)
    c = f.queue_next(q, 30000)
    print("   binding queue_next on a failed call    -> status %s, body %r"
          % ((c[1], c[2] if not c[2] else "%d bytes" % len(c[2])) if c else ("timeout", None)),
          file=out)
    done, box = threading.Event(), []
    f.call_unary_cb(cl, rpc.METHOD, b"", lambda t, st, b: (box.append((st, b)), done.set()), 7)
    done.wait(30)
    st, b = box[0] if box else ("timeout", None)
    print("   binding callback on a failed call      -> status %s, body %r"
          % (st, b if not b else "%d bytes" % len(b)), file=out)
    del rt, cl, q


def run(kind, out):
    d = tempfile.mkdtemp(prefix="akgate")
    target = "unix:" + os.path.join(d, "s")
    srv = serve_injected("healthy" if kind == "closed" else kind)
    srv.add_insecure_port(target)
    srv.start()
    ch = grpc.insecure_channel(target, options=rpc.DEFAULT_SERVER_ARGS)
    try:
        core = cells_for(target, False)
        raw_client = rpc.core_client(target, False)    # connected while the server is up
        a_cells = rpc._deserializers()
        # Warm the grpcio channel while the server is still up, so `closed` exercises a
        # dead connection rather than a failed first connect.
        warm = ch.unary_unary(rpc.METHOD, request_serializer=lambda b: b,
                              response_deserializer=lambda b: b)
        try:
            warm(b"")
        except Exception:  # noqa: BLE001
            pass
        if kind == "closed":
            srv.stop(0).wait()
        print("\n## injection: %s" % kind, file=out)
        print("   %-46s %s" % ("cell", "one call"), file=out)
        # Cell A's single call goes through the SAME deserializer the timed loop uses:
        # `rpc._checked(deser)` where it exists (after the R-D3 fix), the bare one before.
        gated = getattr(rpc, "_checked", lambda d: d)
        for name, deser in a_cells:
            call = ch.unary_unary(rpc.METHOD, request_serializer=lambda b: b,
                                  response_deserializer=gated(deser))
            print("   A %-44s %s" % (name, one_call(lambda: call(b""))), file=out)
        for name, fn in core:
            print("   %-46s %s" % (name, one_call(fn)), file=out)
        if kind in ("status", "closed"):
            raw_binding(raw_client, out)
        print("   %-46s %s" % ("cell", "what the timed loop does (8 calls, 1 and 4 in flight)"),
              file=out)
        for name, deser in a_cells:
            for k in (1, 4):
                print("   A %-40s x%-2d %s"
                      % (name, k, timed(rpc.measure, ch, name, deser, 8, k)), file=out)
        for name, fn in core:
            for k in (1, 4):
                print("   %-42s x%-2d %s" % (name, k, timed(rpc.measure_fn, fn, 8, k)),
                      file=out)
    finally:
        ch.close()
        srv.stop(0).wait()


def main():
    out = sys.stdout
    print("# python slice: the RPC arm's gate under injected failure (FIX-PLAN R-D3)", file=out)
    print("# interpreter: %s" % sys.version.split()[0], file=out)
    print("# shim:        %s over %s" % (os.environ["AK_FFI_MODULE"], "the rpc-feature core"),
          file=out)
    print("# payload:     %s, %d bytes; P2.2 has %d %s"
          % (PID, len(BODY), N_ELEM, ELEM), file=out)
    print("# NO TIMINGS. A row says whether the harness would publish a figure, never the figure.",
          file=out)
    for a in arms.absent:
        print("# ARM ABSENT: %s" % a, file=out)
    gate = getattr(rpc, "gate_cells", None)
    if gate is not None:
        print("\n## rpc.gate_cells(), the pre-timing byte-identity gate, on a healthy server",
              file=out)
        d = tempfile.mkdtemp(prefix="akgate")
        target = "unix:" + os.path.join(d, "s")
        srv = serve_injected("healthy")
        srv.add_insecure_port(target)
        srv.start()
        try:
            ch = grpc.insecure_channel(target, options=rpc.DEFAULT_SERVER_ARGS)
            for line in gate(ch, rpc.core_cells(target, False)):
                print("   " + line, file=out)
            ch.close()
        finally:
            srv.stop(0).wait()
    for kind in ("healthy", "status", "empty", "short", "closed"):
        run(kind, out)
    # Let the tokio runtimes wind down before the interpreter tears the module down.
    threading.Event().wait(0.2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
