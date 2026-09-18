"""Isolate the stage 1 disagreement: is the zero-leaf defect the ONLY one?

Copies ffi/schema/emit into a scratch directory, applies ONE candidate change there --
omit an implicit-presence leaf of Timestamp/Duration that holds the proto zero, which is
what the canonical form in ffi/schema/README.md already says and what every other scalar
path in emit/payloads.py already does -- regenerates all sixteen payloads, and compares
them byte-for-byte against what prost encodes.

Nothing under ffi/schema/ is written: a slice does not own it, and the change proposed
here is a finding for the aggregating session, not an edit.

  python3 gen/stage1_isolate.py <scratch dir>
"""
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SCHEMA = os.path.abspath(os.path.join(HERE, "..", "..", "..", "schema"))

# emit/payloads.py, enc_field(), the message branch. Every other scalar path in that file
# guards on the value ("return W.i(...) if v else b''"); these two do not.
OLD_FIELD = '''        if f["of"] == "Timestamp":
            t = V.timestamp(path, idx)
            return W.ld(f["tag"], W.i(1, t["seconds"]) + W.i(2, t["nanos"]))
        if f["of"] == "Duration":
            t = V.duration(path, idx)
            return W.ld(f["tag"], W.i(1, t["seconds"]) + W.i(2, t["nanos"]))'''
NEW_FIELD = '''        if f["of"] in ("Timestamp", "Duration"):
            t = (V.timestamp if f["of"] == "Timestamp" else V.duration)(path, idx)
            body = ((W.i(1, t["seconds"]) if t["seconds"] else b"")
                    + (W.i(2, t["nanos"]) if t["nanos"] else b""))
            return W.ld(f["tag"], body)'''

# emit/payloads.py, oneof_member(). Same shortcut, not reachable with a zero in P3.1 today,
# so it is swept rather than fixed where it was found (README R10).
OLD_ONEOF = '''    if f["kind"] == "message":
        t = V.timestamp(path, idx)
        return W.ld(f["tag"], W.i(1, t["seconds"]) + W.i(2, t["nanos"]))'''
NEW_ONEOF = '''    if f["kind"] == "message":
        t = V.timestamp(path, idx)
        body = ((W.i(1, t["seconds"]) if t["seconds"] else b"")
                + (W.i(2, t["nanos"]) if t["nanos"] else b""))
        return W.ld(f["tag"], body)'''

DUMP = '''import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "emit"))
import shapes as S, payloads as E
S.ROOT = os.path.dirname(os.path.abspath(__file__))
schema = S.load(os.path.join(S.ROOT, "shapes.json"))
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
for pid, spec in schema["payloads"].items():
    E.stats["strings"] = E.stats["elements"] = 0
    data = E.build(schema, pid, spec)
    open(os.path.join(out, pid.replace(".", "_") + ".bin"), "wb").write(data)
    print("%-6s %9d B" % (pid, len(data)))
'''


def main(scratch):
    work = os.path.join(scratch, "emit-candidate")
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    shutil.copytree(os.path.join(SCHEMA, "emit"), os.path.join(work, "emit"))
    shutil.copy(os.path.join(SCHEMA, "shapes.json"), work)

    p = os.path.join(work, "emit", "payloads.py")
    s = open(p).read()
    for old, new, where in ((OLD_FIELD, NEW_FIELD, "enc_field"),
                            (OLD_ONEOF, NEW_ONEOF, "oneof_member")):
        assert old in s, "emit/payloads.py has changed: %s no longer matches" % where
        s = s.replace(old, new)
        print("patched %s" % where)
    open(p, "w").write(s)
    open(os.path.join(work, "dump.py"), "w").write(DUMP)

    cand = os.path.join(scratch, "payloads-candidate")
    print()
    print("# sizes with the candidate change")
    subprocess.check_call([sys.executable, os.path.join(work, "dump.py"), cand])

    prost = os.path.join(scratch, "prost-bytes")
    subprocess.check_call(["cargo", "run", "--release", "-q",
                           "--manifest-path", os.path.join(HERE, "..", "Cargo.toml"),
                           "-p", "stage1-validate", "--", "unused", "--emit", prost])

    print()
    print("# byte comparison: prost against the CANDIDATE emitters")
    bad = 0
    for name in sorted(os.listdir(prost)):
        a = open(os.path.join(prost, name), "rb").read()
        b = open(os.path.join(cand, name), "rb").read()
        print("%-10s %s" % (name, "identical" if a == b else "DIFFERS"))
        bad += a != b
    print()
    print("%d payload(s) still disagree with prost after the candidate change" % bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
