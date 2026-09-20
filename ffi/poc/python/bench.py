"""Work unit 2: the composed arm, both directions, against the incumbent.

Ratios are formed against the incumbent inside one process in interleaved rounds
(README R3, R4), and nothing is timed until `conformance.py` passes (R2).

**Absolutes here are instrumentation, not the deliverable.**  The cross-language
comparison is re-taken on a controlled physical machine once every slice exists, so what
this table is for is the sign and the size class of each gap -- which storage, which
direction, and whether the composed arm is above or below the incumbent.  A ranking too
close to call is recorded as ambiguous rather than ground at.

Usage:  python bench.py [--json out.json]
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import arms      # noqa: E402


def _load_harness():
    """`mech/harness.py`, loaded BY PATH rather than by putting `mech/` on sys.path.

    Work unit 1 and work unit 2 each have an `arms.py`, and putting `mech/` on the path
    makes `import arms` resolve to whichever came first -- so `bench.py` would silently
    measure work unit 1's arm table. One harness, imported rather than copied (R0's
    lesson one level down), and no directory on the path that could shadow a module.
    """
    import importlib.util
    p = os.path.join(HERE, "mech", "harness.py")
    spec = importlib.util.spec_from_file_location("wu_harness", p)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


harness = _load_harness()
Case = harness.Case


def main():
    out = sys.stdout
    print("# python slice, work unit 2: the COMPOSED arm over the M1 subtree", file=out)
    print("#", file=out)
    print("# interpreter:  %s" % sys.version.replace("\n", " "), file=out)
    print("# incumbent:    protobuf %s on %s" % (_pbver(), _pbimpl()), file=out)
    print("# baseline:     %s" % arms.BASELINE, file=out)
    print("# core:         ffi/poc/codec, ABI v1, libak_core.so through the dynamic"
          " linker", file=out)
    print("# fill:         ABI v1 decision 9's SPARSE fill (bulk clear, then only what"
          " differs)", file=out)
    print("# rounds:       %d, interleaved; per-case target %.0f ms"
          % (harness.ROUNDS, harness.TARGET_NS / 1e6), file=out)
    print("# payloads:     %s (P1.3 is the absent path)" % ", ".join(arms.PAYLOADS),
          file=out)
    print("# absolutes:    instrumentation, not the deliverable. Read the signs and the",
          file=out)
    print("#               size classes; the controlled rerun owns the decimals.", file=out)
    for a in arms.absent:
        print("# ARM ABSENT: %s" % a, file=out)

    print("\n## the correctness gate (README R2)", file=out)
    r = subprocess.run([sys.executable, os.path.join(HERE, "conformance.py")],
                       capture_output=True, text=True)
    last = r.stdout.strip().splitlines()[-1] if r.stdout else "?"
    print("   conformance.py exit %d: %s" % (r.returncode, last), file=out)
    if r.returncode:
        print("\n".join(ln for ln in r.stdout.splitlines() if "FAIL" in ln), file=out)
        print("\nREFUSING TO TIME: correctness before timing (README R2).", file=out)
        return 1

    cases = []
    for pid in arms.PAYLOADS:
        ref = memoryview(arms.reference(pid))
        for direction, mk in (("encode", arms.encode_arms),
                              ("decode", arms.decode_arms),
                              ("decode+read every field", arms.decode_touch_arms),
                              ("re-read every field", arms.reread_arms)):
            g = "%s %s" % (direction, pid)
            for name, fn in mk(pid):
                cases.append(Case(g, name, lambda reps, _f=fn: _loop(_f, reps)))
            # README R2's floor arm. On encode it is one copy of the output; on decode it
            # is one copy of the input plus one list of N objects, which is the least a
            # decoder that produced host objects could possibly have done.
            if direction.startswith("decode+") or direction.startswith("re-read"):
                pass
            elif direction == "encode":
                cases.append(Case(g, "-- floor: one copy of the output (%d B)" % len(ref),
                                  lambda reps, _r=ref: _copy(_r, reps),
                                  "no traversal. An encode cannot go below this"))
            else:
                n = len(arms.build_facade(pid, arms.PLAIN).results)
                cases.append(Case(
                    g, "-- floor: %d bare objects + one copy" % n,
                    lambda reps, _r=ref, _n=n, _c=arms.PLAIN[1]: _dfloor(_r, _n, _c, reps),
                    "construct N facade elements and copy the input, and nothing else"))

    # The boundary, priced in this process and this build, so every absolute above is
    # also quotable as a multiple of a crossing (README R13).
    if arms._ffi:
        cases.append(Case("the boundary, in this process", "forward (no-op)",
                          lambda reps: arms._ffi.crossing(reps, "forward")))
        cases.append(Case("the boundary, in this process", "fwd+reverse (no-op)",
                          lambda reps: arms._ffi.crossing(reps, "reverse")))

    harness.run(cases)
    rows = harness.report(
        cases, lambda g: "upb (incumbent)" if g[0] in "ed" else None, out)

    if "--json" in sys.argv:
        harness.dump_json(rows, sys.argv[sys.argv.index("--json") + 1])
    return 0


def _loop(f, reps):
    for _ in range(reps):
        f()


def _copy(ref, reps):
    for _ in range(reps):
        ref.tobytes()


def _dfloor(ref, n, ctor, reps):
    for _ in range(reps):
        ref.tobytes()
        [ctor() for _ in range(n)]


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
