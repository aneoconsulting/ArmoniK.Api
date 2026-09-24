"""U1: does the incumbent keep a map entry that carries an unknown field?

A map entry is a message on the wire (`key = 1`, `value = 2`). An unknown field inside it
must be skipped and the entry kept; proto3 has no rule that discards the entry. This
script hands ONE such entry to four readers and prints what each returns:

  upb            protobuf's default backend, the incumbent (R14)
  python         the same protobuf version's pure-Python backend (a subprocess, because the
                 backend is chosen once per process)
  core-ffi       this slice's composed arm (the shared core behind the CPython shim)
  pycodec        this slice's pure-Python control

The input is a `ListTasksDetailedResponse` with one task whose `options` map holds one
entry {"k": "v"} plus an unknown varint field 3 inside the entry, and a control input
identical but without the unknown field. No timings.
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)


def _v(n):
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        out.append(b | (0x80 if n else 0))
        if not n:
            return bytes(out)


def _len(tag, body):
    return _v((tag << 3) | 2) + _v(len(body)) + body


def wire(with_unknown):
    entry = _len(1, b"k") + _len(2, b"v") + ((_v((3 << 3) | 0) + _v(7)) if with_unknown else b"")
    task_options = _len(1, entry)                       # TaskOptions.options = 1
    task = _len(10, task_options)                       # TaskDetailed.options = 10
    return _len(1, task)                                # ListTasksDetailedResponse.tasks = 1


def child():
    """Run in a subprocess: parse with whichever protobuf backend this process has."""
    from google.protobuf.internal import api_implementation
    import arms
    R = arms._pb2.ListTasksDetailedResponse
    for label, unk in (("control, no unknown field", False), ("entry WITH an unknown field", True)):
        m = R.FromString(wire(unk))
        print("   %-8s %-30s -> options = %r"
              % (api_implementation.Type(), label, dict(m.tasks[0].options.options)))


def main():
    if "--child" in sys.argv:
        child()
        return 0
    print("# U1: a map entry carrying an unknown field, four readers (no timings)")
    print("# interpreter: %s" % sys.version.split()[0])
    import google.protobuf
    print("# protobuf:    %s" % google.protobuf.__version__)
    print("# input:       %s" % wire(True).hex())
    print("# control:     %s" % wire(False).hex())
    print()
    for impl in ("upb", "python"):
        env = dict(os.environ, PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=impl)
        r = subprocess.run([sys.executable, os.path.abspath(__file__), "--child"],
                           env=env, capture_output=True, text=True)
        sys.stdout.write(r.stdout)
        if r.returncode:
            sys.stdout.write("   %s: FAILED %s\n" % (impl, r.stderr.strip().splitlines()[-1:]))
    import arms
    for label, unk in (("control, no unknown field", False), ("entry WITH an unknown field", True)):
        o = arms._ffi.decode("cext", "ListTasksDetailedResponse", wire(unk), arms.TY_CEXT)
        print("   %-8s %-30s -> options = %r" % ("core-ffi", label, dict(o.tasks[0].options.options)))
        p = arms.pycodec.decode_root_ListTasksDetailedResponse(wire(unk), arms.CT_PLAIN)
        print("   %-8s %-30s -> options = %r" % ("pycodec", label, dict(p.tasks[0].options.options)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
