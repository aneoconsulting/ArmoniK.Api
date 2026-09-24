#!/usr/bin/env python3
"""FIX-PLAN WP5 step 6: what the corpus's three oracles do with two inputs no corpus row has.

  * a varint whose 10th byte carries bits beyond 64;
  * a field number above 2^29 - 1 (protobuf's maximum), at the top level and inside a
    skipped group, including 2^32 + 2 (which a 32-bit truncation aliases to field 2).

Run with python3.12 (protobuf 7.36.2, upb), again with
PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python (pure-python), and `--protoc` (protobuf C++ via
`protoc --decode`), against `ffi/corpus/generated/corpus.proto`'s WireZoo. Writes each input
to `--out DIR/<id>.bin` when asked, so the corpus agent can add them as vectors.
"""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PROTO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "corpus", "generated"))
MSG = "armonik.ffi.corpus.v1.WireZoo"


def v(n):
    o = bytearray()
    while n >= 0x80:
        o.append((n & 0x7f) | 0x80)
        n >>= 7
    o.append(n)
    return bytes(o)


CASES = [
    ("varint10-bit63", b"\x10" + b"\xff" * 9 + b"\x01", "10th byte 0x01: 2^63 set, legal"),
    ("varint10-bit64", b"\x10" + b"\xff" * 9 + b"\x02", "10th byte 0x02: a bit beyond 64"),
    ("varint10-7f", b"\x10" + b"\xff" * 9 + b"\x7f", "10th byte 0x7f"),
    ("varint10-over-zero", b"\x10" + b"\x80" * 9 + b"\x7e", "only bits beyond 64 set"),
    ("varint10-int32", b"\x08" + b"\xff" * 9 + b"\x02", "the same into an int32"),
    ("field-max", v((536870911 << 3) | 0) + b"\x01", "field 2^29-1: the maximum, known here"),
    ("field-max+1", v((536870912 << 3) | 0) + b"\x01", "field 2^29"),
    ("field-2^32+2", v(((2 ** 32 + 2) << 3) | 0) + b"\x05", "aliases field 2 if truncated to 32 bits"),
    ("field-max+1-in-group", v((100 << 3) | 3) + v((536870912 << 3) | 0) + b"\x01" + v((100 << 3) | 4),
     "field 2^29 inside a skipped group"),
]


def py_runtime():
    import subprocess as sp
    from google.protobuf import descriptor_pb2, descriptor_pool, message_factory
    import google.protobuf
    from google.protobuf.internal import api_implementation
    with tempfile.TemporaryDirectory() as d:
        desc = os.path.join(d, "r.desc")
        sp.run([sys.executable, "-m", "grpc_tools.protoc", "-I", PROTO,
                "--descriptor_set_out=" + desc, "corpus.proto"], check=True)
        fds = descriptor_pb2.FileDescriptorSet()
        fds.ParseFromString(open(desc, "rb").read())
    pool = descriptor_pool.DescriptorPool()
    for f in fds.file:
        pool.Add(f)
    cls = message_factory.GetMessageClass(pool.FindMessageTypeByName(MSG))
    return "protobuf %s (%s)" % (google.protobuf.__version__, api_implementation.Type()), cls


def main(argv):
    out = argv[argv.index("--out") + 1] if "--out" in argv else None
    if "--protoc" in argv:
        ver = subprocess.run(["protoc", "--version"], capture_output=True, text=True).stdout.strip()
        print("# oracle: protobuf C++ (%s, protoc --decode=%s)" % (ver, MSG))
        for cid, b, what in CASES:
            r = subprocess.run(["protoc", "-I", PROTO, "--decode=" + MSG, "corpus.proto"],
                               input=b, capture_output=True)
            res = ("ACCEPT " + r.stdout.decode().replace("\n", " ").strip()) if r.returncode == 0 \
                else "REFUSE " + r.stderr.decode().strip().splitlines()[-1][:60]
            print("%-22s %-44s %s" % (cid, what, res))
        return 0
    name, cls = py_runtime()
    print("# oracle: %s" % name)
    for cid, b, what in CASES:
        m = cls()
        try:
            m.ParseFromString(b)
            res = "ACCEPT " + str(m).replace("\n", " ").strip()
        except Exception as e:  # noqa: BLE001
            res = "REFUSE %s" % type(e).__name__
        print("%-22s %-44s %s" % (cid, what, res))
        if out:
            os.makedirs(out, exist_ok=True)
            open(os.path.join(out, cid + ".bin"), "wb").write(b)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
