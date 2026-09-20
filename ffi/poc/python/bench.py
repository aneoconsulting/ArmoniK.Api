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

import allocator  # noqa: E402  (before `arms`: `arms` allocates, and this must be first)

# Pin the allocator BEFORE the first large allocation.  Without this, the encode absolute
# for any payload bigger than what the process has allocated so far is a property of the
# case list rather than of the codec: glibc trims the buffer back to the OS between calls
# and the next call faults it in again.  `allocator.py` is the experiment -- 1.92x on the
# incumbent's P1.2 encode, 3.81x on its P2.4 -- and it is what moved this slice's P1.2
# encode ratio from 0.70 in work unit 2 (M1 only in the run, so nothing ever allocated
# past 218 KiB) to 1.26 in work unit 3 (M2 in the run, so P2.4 warmed it).  Warm is the
# state a long-lived gRPC server is in, and it is also the state that helps the incumbent
# more than it helps us, so it is the conservative choice as well as the realistic one.
_WARM = allocator.warm_up()

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
    print("# python slice, work unit 3: the COMPOSED arm over M1 and M2", file=out)
    print("#", file=out)
    print("# interpreter:  %s" % sys.version.replace("\n", " "), file=out)
    print("# incumbent:    protobuf %s on %s" % (_pbver(), _pbimpl()), file=out)
    print("# baseline:     %s" % arms.BASELINE, file=out)
    print("# core:         ffi/poc/codec, ABI v1, libak_core.so through the dynamic"
          " linker", file=out)
    print("# fill:         ABI v1 decision 9's SPARSE fill (bulk clear, then only what"
          " differs)", file=out)
    print("# gc:           DISABLED for the measured rounds, so a ratio is the codec's.",
          file=out)
    print("#               The collector is a real cost and it is the FACADE's -- 1.09 to",
          file=out)
    print("#               1.26 on its decode, nothing on any other row -- but it cannot",
          file=out)
    print("#               be attributed inside an interleaved run. 57-gc-bias.log prices",
          file=out)
    print("#               it in isolation. Defect D11, and its first fix was worse.",
          file=out)
    print("# allocator:    mallopt(M_TOP_PAD, 8 MiB) %s, set before the first allocation."
          % ("APPLIED" if _WARM else "NOT AVAILABLE (not glibc)"), file=out)
    print("#               Without it an encode above ~128 KiB is timed against glibc",
          file=out)
    print("#               handing the buffer back to the OS between calls, and the",
          file=out)
    print("#               figure then depends on which payloads precede it in the run.",
          file=out)
    print("#               allocator.py is the experiment; logs/python/55-allocator.log.",
          file=out)
    print("# rounds:       %d, interleaved; per-case target %.0f ms"
          % (harness.ROUNDS, harness.TARGET_NS / 1e6), file=out)
    print("# payloads:     %s" % ", ".join(arms.PAYLOADS), file=out)
    print("#               P1.x: M1, a FLAT message whose element is a LEAF, so the",
          file=out)
    print("#               batching predicate admits it and the crossing count per",
          file=out)
    print("#               element is constant. P2.x: M2, whose element is NOT a leaf.",
          file=out)
    print("#               P1.3 and P2.5 are the absent paths.", file=out)
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
            elif arms.elem_field(pid) is None:
                # M5's root has no repeated field, so there is no element floor to state:
                # the decode constructs one root and one child whatever the blob's size.
                cases.append(Case(g, "-- floor: one copy of the input (%d B)" % len(ref),
                                  lambda reps, _r=ref: _copy(_r, reps),
                                  "no elements to construct; the blob is the whole cost"))
            else:
                elem = arms.elem_type(pid)
                n = len(getattr(arms.build_facade(pid, arms.CT_PLAIN),
                                arms.elem_field(pid)))
                ctor = arms.CT_PLAIN[elem]
                cases.append(Case(
                    g, "-- floor: %d bare %s + one copy" % (n, elem),
                    lambda reps, _r=ref, _n=n, _c=ctor: _dfloor(_r, _n, _c, reps),
                    "construct N facade elements and copy the input, and nothing else"))

    # The boundary, priced in this process and this build, so every absolute above is
    # also quotable as a multiple of a crossing (README R13).
    if arms._ffi:
        cases.append(Case("the boundary, in this process", "forward (no-op)",
                          lambda reps: arms._ffi.crossing(reps, "forward")))
        cases.append(Case("the boundary, in this process", "fwd+reverse (no-op)",
                          lambda reps: arms._ffi.crossing(reps, "reverse")))

    # GC DISABLED here, and the collector's cost reported SEPARATELY by `gcbias.py`.
    #
    # The first fix for D11 was to enable it, and that was wrong -- worse than the defect.
    # With the collector on, an interleaved run of ~500 cases attributes each collection to
    # whichever case happens to trip the threshold, so P2.2's decode came out at 7.3x the
    # incumbent here against 1.26x for the identical call measured in isolation. Neither
    # the codec nor the live set explains the gap: holding all sixteen payloads' fixtures
    # alive makes the isolated figure slightly FASTER, so it is attribution and nothing
    # else. A figure that depends on what else is in the run is the same defect as the
    # allocator one, and this bench already refuses that class.
    #
    # So the collector stays off where a ratio is formed, and `gcbias.py` prices it where
    # it can be attributed: one payload, one direction, one process, nothing interleaved.
    # The honest sentence is "these ratios exclude the collector, which adds 1.09 to 1.26
    # to the facade's decode and nothing to any other row", and that sentence needs both
    # halves to be measured -- which is what the two scripts are.
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
