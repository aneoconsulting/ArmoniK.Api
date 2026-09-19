"""Work unit 1, part 2: the facade storage and the premise control, over a real
message with a validated byte oracle.

`bench_mech.py` prices one field read.  This prices a whole `ResultRaw`, which is
what actually decides README 9.1's storage question, because a per-read figure
does not say how many reads a message needs and the crossing counts in
`conformance.py` say it is 29 per element for one storage and 7 for another.

Ratios are formed against the **incumbent** (`protobuf` on upb), inside one
process, in interleaved rounds (README R3, R4).  Every arm has passed
`conformance.py` byte-for-byte against the validated manifest on all three
payloads, the absent path included, before a single number here exists (R2).

Usage:  python bench_codec.py [--json out.json]
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import arms  # noqa: E402
import harness  # noqa: E402
from harness import Case  # noqa: E402


def main():
    out = sys.stdout
    print("# python slice, work unit 1: the codec arms over the M1 subtree", file=out)
    print("#", file=out)
    print("# interpreter:  %s" % sys.version.replace("\n", " "), file=out)
    print("# incumbent:    protobuf %s on %s"
          % (_pbver(), _pbimpl()), file=out)
    print("# rounds:       %d, interleaved; per-case target %.0f ms"
          % (harness.ROUNDS, harness.TARGET_NS / 1e6), file=out)
    print("# payloads:     %s (design/SHAPES.md; P1.3 is the absent path)"
          % ", ".join(arms.PAYLOADS), file=out)
    print("# direction:    encode only. Decode is not built in work unit 1", file=out)
    print("# NOT in any arm: the Rust core. This prices the shim-to-FACADE edge,", file=out)
    print("#                 which is the one that is a crossing per field.", file=out)
    for a in arms.absent:
        print("# ARM ABSENT: %s" % a, file=out)

    print("\n## the correctness gate (README R2)", file=out)
    r = subprocess.run([sys.executable, os.path.join(HERE, "conformance.py")],
                       capture_output=True, text=True)
    tail = [ln for ln in r.stdout.splitlines()
            if ln.startswith(("ALL CHECKS", "  ", "%d" % 0)) and "FAIL" in ln]
    print("   conformance.py exit %d: %s"
          % (r.returncode, r.stdout.strip().splitlines()[-1] if r.stdout else "?"),
          file=out)
    if r.returncode:
        print("\n".join(tail), file=out)
        print("\nREFUSING TO TIME: correctness before timing (README R2).", file=out)
        return 1

    cases = []
    for pid in arms.PAYLOADS:
        fac = arms.build_facades(pid)
        upb = arms.build_upb(pid)
        g = "encode %s" % pid
        for name, fn in arms.arms(fac, upb):
            cases.append(Case(g, name, lambda reps, _f=fn: _loop(_f, reps)))
        # README R2: "a ratio far enough from 1 to be surprising gets a floor
        # arm before it is reported". The C-extension-type arm came out BELOW
        # the incumbent, and an encode of an N-byte message cannot cost less
        # than producing N bytes. This row is exactly that: one copy of the
        # finished output into a fresh `bytes`, no traversal at all. Every arm
        # above pays it, so what sits on it is doing nothing else, and the
        # distance above it is what each arm's traversal costs.
        # `bytes(b)` on a `bytes` returns the SAME object and copies nothing:
        # the first spelling of this row measured 58 ns at every payload size,
        # which is what a floor that is not doing the work looks like (defect
        # D2 in STATE.md). A memoryview copy is a real copy.
        ref = memoryview(arms.gvalues.reference(pid))
        cases.append(Case(g, "-- floor: one copy of the output (%d B)" % len(ref),
                          lambda reps, _r=ref: _copy(_r, reps),
                          "no traversal. An encode cannot go below this"))

    harness.run(cases)
    rows = harness.report(cases, lambda g: "upb (incumbent)", out)

    print("\n## the two questions this table answers", file=out)
    print("#", file=out)
    print("# 1. STORAGE. Compare `cshim getattr / plain` with `cshim member /", file=out)
    print("#    C ext type`: the same generated traversal, the same bytes, and", file=out)
    print("#    29 crossings per element against 7 (conformance.py).", file=out)
    print("# 2. THE PREMISE (README 9.1, last bullet). Compare `cshim getattr /", file=out)
    print("#    plain` with `cshim pyacc (python fn) / plain`: the same C", file=out)
    print("#    traversal, the same crossing COUNT, and the only difference is", file=out)
    print("#    whether a field is reached by a C-API read or by a call into the", file=out)
    print("#    interpreter. That is a within-arm delta, which README R4 says", file=out)
    print("#    survives what a ratio to a third arm does not.", file=out)

    if "--json" in sys.argv:
        harness.dump_json(rows, sys.argv[sys.argv.index("--json") + 1])
    return 0


def _loop(f, reps):
    for _ in range(reps):
        f()


def _copy(ref, reps):
    for _ in range(reps):
        ref.tobytes()


def _pbver():
    try:
        import google.protobuf
        return google.protobuf.__version__
    except Exception:  # noqa: BLE001
        return "absent"


def _pbimpl():
    try:
        from google.protobuf.internal import api_implementation
        return api_implementation.Type()
    except Exception:  # noqa: BLE001
        return "absent"


if __name__ == "__main__":
    sys.exit(main())
