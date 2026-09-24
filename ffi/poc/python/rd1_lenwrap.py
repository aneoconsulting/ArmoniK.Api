"""R-D1 through this slice: a length varint that wraps 2^64, decoded by the shim.

FIX-PLAN WP4 item 1. In a release core without overflow checks, `pos + n` wrapped for a
length near 2^64 and the bounds check passed: the finding's 11-byte input hung the root
loop, a string span could reach the host with length 0xFFFFFFFF, and a nested length
panicked across `extern "C"`. Each input below is decoded in its OWN subprocess under a
timeout, so a hang shows as TIMEOUT and an abort as a signal rather than taking this
script down. What must happen: every decode returns an error, promptly.

  finding     7A F5 FF FF FF FF FF FF FF FF 01   the review's bytes (field 15, LEN, wrapped)
  root        results (field 1), LEN = 2^64 - 1
  nested      one results element whose session_id (field 1) has LEN = 2^64 - 1
  control     P1.2 from the manifest, which must still decode

Usage:  python rd1_lenwrap.py            (AK_FFI_MODULE selects the shim, as everywhere)
No timings.
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

WRAP = bytes.fromhex("FFFFFFFFFFFFFFFFFF01")                  # 2^64 - 1 as a varint
INNER = bytes([0x0A]) + WRAP                                 # session_id, wrapped
INPUTS = [
    ("finding", bytes.fromhex("7AF5FFFFFFFFFFFFFFFF01")),
    ("root", bytes([0x0A]) + WRAP),
    ("nested", bytes([0x0A, len(INNER)]) + INNER),
]
BACKENDS = [("C ext type", "cext", "TY_CEXT"), ("plain", "attr", "TY_PLAIN")]
TIMEOUT_S = 10


def _core():
    try:
        ps = sorted({ln.split()[-1] for ln in open("/proc/self/maps")
                     if ln.rstrip().endswith("libak_core.so")})
    except OSError:
        return "?"
    return ",".join("/".join(p.split("/")[-4:]) for p in ps) or "none"


def child(name, backend, types):
    import arms
    buf = dict(INPUTS).get(name) if name != "control" else arms.reference("P1.2")
    try:
        o = arms._ffi.decode(backend, "ListResultsResponse", buf, getattr(arms, types))
    except Exception as e:  # noqa: BLE001
        print("ERROR %s: %s  [core %s]" % (type(e).__name__, e, _core()))
        return 0
    print("DECODED %d results  [core %s]" % (len(o.results), _core()))
    return 0


def main():
    if "--child" in sys.argv:
        i = sys.argv.index("--child")
        return child(*sys.argv[i + 1:i + 4])
    mod = os.environ.get("AK_FFI_MODULE", "_akffi")
    print("# R-D1 length-varint wrap through the python shim (%s), ListResultsResponse decode" % mod)
    print("# interpreter: %s; each decode in its own subprocess, timeout %d s" % (sys.version.split()[0], TIMEOUT_S))
    fails = 0
    for name, buf in INPUTS + [("control", None)]:
        for label, backend, types in BACKENDS:
            try:
                r = subprocess.run([sys.executable, os.path.abspath(__file__), "--child",
                                    name, backend, types],
                                   capture_output=True, text=True, timeout=TIMEOUT_S)
                out = (r.stdout.strip() or "(no output)")
                if r.returncode < 0:
                    out = "KILLED by signal %d; stderr: %s" % (-r.returncode, ((r.stderr.strip().splitlines() or [""])[-1])[:100])
                elif r.returncode:
                    out = "EXIT %d; stderr: %s" % (r.returncode, ((r.stderr.strip().splitlines() or [""])[-1])[:100])
            except subprocess.TimeoutExpired:
                out = "TIMEOUT after %d s (hang)" % TIMEOUT_S
            want = "DECODED" if name == "control" else "ERROR"
            ok = out.startswith(want)
            fails += not ok
            print("   %-4s %-8s %-11s %-26s %s"
                  % ("ok" if ok else "FAIL", name, label,
                     (buf.hex() if buf is not None else "P1.2")[:26], out))
    print("\n%s" % ("ALL WRAPPED LENGTHS REFUSED, CONTROL DECODES" if not fails
                    else "%d FAILURE(S)" % fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
