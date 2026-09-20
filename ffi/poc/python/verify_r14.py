"""R14: the baseline is the codec path ArmoniK runs, and this derives it rather than
asserting it.

`packages/python` does not serialise directly.  What does is gRPC's generated stub, so
this generates it from the REAL `Protos/V1/results_service.proto` with the pinned
`grpcio-tools` and prints the serializer and deserializer it names.  A slice whose
baseline is the library's best entry point rather than production's is a defect; so is
one that handicaps the incumbent with work production does not do.
"""
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
PROTOS = os.path.join(REPO, "Protos", "V1")

print("# R14: what ArmoniK's generated gRPC stub actually calls")
print("# protos:      %s" % PROTOS)
try:
    import grpc_tools
    print("# grpcio-tools %s" % getattr(grpc_tools, "__version__", "(version unset)"))
except ImportError:
    print("# grpcio-tools is not installed for this interpreter; cannot derive R14 here.")
    sys.exit(1)

with tempfile.TemporaryDirectory() as td:
    r = subprocess.run(
        [sys.executable, "-m", "grpc_tools.protoc", "-I", PROTOS,
         "--python_out", td, "--grpc_python_out", td, "results_service.proto"],
        capture_output=True, text=True, cwd=PROTOS)
    if r.returncode:
        print("# protoc failed:\n%s" % (r.stderr or r.stdout))
        sys.exit(1)
    stub = os.path.join(td, "results_service_pb2_grpc.py")
    src = open(stub).read()
    pairs = re.findall(
        r"request_serializer=([\w.]+)\.?\n?\s*,?\s*response_deserializer=([\w.]+)", src)
    ser = sorted({a.split(".")[-1] for a, _ in pairs})
    de = sorted({b.split(".")[-1] for _, b in pairs})
    print("# methods:    %d" % len(pairs))
    print("# serializer:   %s" % ", ".join(ser))
    print("# deserializer: %s" % ", ".join(de))
    print()
    if ser == ["SerializeToString"] and de == ["FromString"]:
        print("R14 CONFIRMED: the production path is Message.SerializeToString and")
        print("Message.FromString, with no size pass and no buffer writer between the")
        print("caller and the codec. That is what arms.py measures as the incumbent, so")
        print("the baseline is production's and not the library's best entry point.")
        sys.exit(0)
    print("R14 MISMATCH: the stub names %r / %r, which is not what arms.py measures."
          % (ser, de))
    sys.exit(1)
