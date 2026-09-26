"""CAMPAIGN req 19 as amended (R-H31): crossing counts of the RPC cells B, C, D and E, per call.

  AK_FFI_MODULE=_akffi_rpc_count python3.12 rpc_counts.py [--variant nounk]   (gate step 105)

The rpc core's COUNTING build (`rpc,count,init-guard`, and `--no-default-features` for the
no-unknown build) behind the same generated shim, the cells exactly as camp_rpc.py builds
them (one channel per cell), against this process's own camp_server.py over a Unix socket
(`shipped`). Per cell and direction: one warm call, then one counted call:
  calls / resets  every ak_* call the client made (the shim's macros; resets apart)
  rpc fwd / rev   the core's RPC counters (ak_rpc_counters)
  codec fwd / rev the core's codec counters of that call's encode or decode
A (grpcio, incumbent) and F (grpcio, host-gen) make no ABI call and are not listed; D makes
codec calls only. Correctness is not re-checked here (the gate's other steps and the RPC
runner do that).
"""
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
NOUNK = "--variant" in sys.argv and sys.argv[sys.argv.index("--variant") + 1] == "nounk"
os.environ.setdefault("AK_FFI_MODULE", "_akffi_rpc_count_nounk" if NOUNK else "_akffi_rpc_count")
import camp_rpc as C  # noqa: E402
arms = C.arms


def main():
    m = arms._ffi
    if m is None or not m.counting():
        print("RPC COUNTS: no counting build loaded (%s)" % os.environ["AK_FFI_MODULE"])
        return 1
    print("# RPC cells, every ABI call per call (req 19); module %s, variant %s"
          % (m.__name__, "no-unknown" if NOUNK else "full"))
    print("# resets' place: decode, ak_dec_reset_<R> before the decode (and after it in retain); encode,")
    print("# ak_enc_reset before the encode (steady state); one warm call precedes every counted call")
    srv, info = C.start_server(tempfile.mkdtemp(prefix="akrpccnt"))
    rows = []
    try:
        cs, keep = C.cells(info["shipped"], "shipped")
        for d, lst in cs.items():
            for name, fn in lst:
                if name[0] not in "BCDE" or "-queue" in name or "-callback" in name:
                    continue
                fn()
                m.reset_counts()
                fn()
                abi, rpc = m.abi_counts(), m.core_counters("rpc")
                cod = m.core_counters("enc" if d == "b" else "dec")
                rows.append("   rpc %-7s %-9s calls %4d resets %2d rpc fwd %3d rev %3d codec fwd %5d rev %5d"
                            % (d, name, abi["calls"], abi["resets"], rpc["forward"], rpc["reverse"],
                               cod["forward"], cod["reverse"]))
        del keep
    finally:
        srv.stdin.close()
        srv.wait(timeout=30)
    for r in rows:
        print(r)
    print("RPC COUNTS: %d rows" % len(rows))
    return 0 if rows else 1


if __name__ == "__main__":
    sys.exit(main())
