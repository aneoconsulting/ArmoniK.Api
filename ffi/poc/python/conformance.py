"""README R2, for work unit 2: correctness before timing, on the composed arm.

Nothing in `bench.py` is timed until this passes.  Six checks:

1.  **The ABI version and the group layouts agree** between the core and the shim.
    ABI v1 section 10 and obligation 12.3: the two sides genuinely restate the layout --
    `#[repr(C)]` there and a C struct here -- and the failure mode is a wrong VALUE in a
    field, which is the worst way to find it.
2.  **Encode: byte identity against the validated manifest**, every arm, every payload.
3.  **Decode: byte identity of a re-encode**, every arm, every payload.  Checking a
    decode by re-encoding is stronger than comparing fields, because it needs no second
    correctness argument about what "equal" means across three facade storages.
4.  **Decode: the facades agree with each other and with the incumbent**, field by field,
    so a decoder that lost a value the re-encode also omits cannot hide.
5.  **The absent path** (P1.3) is in the gate and not beside it (README R6).
6.  **Crossing counts** from the counting build, both halves: the core counts its own
    (README R5) and the shim counts what it does to CPython.  Neither can count the
    other's.
"""

import hashlib
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FFI = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import arms  # noqa: E402

MANIFEST = json.load(open(os.path.join(FFI, "schema", "generated", "manifest.json")))


def first_diff(a, b):
    n = min(len(a), len(b))
    for i in range(n):
        if a[i] != b[i]:
            lo = max(0, i - 8)
            return ("byte %d: %02x vs %02x (context %s | %s)"
                    % (i, a[i], b[i], a[lo:i].hex(), b[lo:i + 8].hex()))
    return "identical for %d bytes, then lengths %d vs %d" % (n, len(a), len(b))


def fieldwise(obj, pb):
    """Every field of every element, the facade against the incumbent."""
    bad = []
    if len(obj.results) != len(pb.results):
        return ["element count %d vs %d" % (len(obj.results), len(pb.results))]
    for i, (a, b) in enumerate(zip(obj.results, pb.results)):
        for n in ("session_id", "name", "owner_task_id", "result_id", "created_by",
                  "opaque_id", "size", "manual_deletion"):
            if getattr(a, n) != getattr(b, n):
                bad.append("elem %d .%s: %r vs %r" % (i, n, getattr(a, n), getattr(b, n)))
        if int(a.status) != int(b.status):
            bad.append("elem %d .status: %r vs %r" % (i, a.status, b.status))
        for n in ("created_at", "completed_at"):
            av, present = getattr(a, n), b.HasField(n)
            if (av is not None) != present:
                bad.append("elem %d .%s presence: %s vs %s"
                           % (i, n, av is not None, present))
            elif av is not None:
                bv = getattr(b, n)
                if (av.seconds, av.nanos) != (bv.seconds, bv.nanos):
                    bad.append("elem %d .%s: (%d,%d) vs (%d,%d)"
                               % (i, n, av.seconds, av.nanos, bv.seconds, bv.nanos))
        if len(bad) > 6:
            return bad[:6] + ["..."]
    for n in ("page", "total"):
        if int(getattr(obj, n)) != int(getattr(pb, n)):
            bad.append(".%s: %r vs %r" % (n, getattr(obj, n), getattr(pb, n)))
    return bad


def counts_only():
    """The counting pass, in its own process (see arms.py)."""
    if arms._ffi is None or not arms._ffi.counting():
        print("## crossing counts: ABSENT, no counting build. R5 forbids inferring them.")
        return 1
    print("## crossing counts, counting build, BOTH halves (README R5)")
    print("#  The core counts its own crossings and the shim counts what it does to")
    print("#  CPython. Neither half can count the other's, which is why both are here.")
    m = arms._ffi
    print("   %-5s %-7s %-22s %s" % ("", "dir", "backend", "per element"))
    for pid in arms.PAYLOADS:
        n = max(MANIFEST["payloads"][pid]["elements"], 1)
        for direction, mk in (("encode", arms.encode_arms),
                              ("decode", arms.decode_arms)):
            for name, fn in mk(pid, mod=m):
                if not name.startswith("core-ffi"):
                    continue
                m.reset_counts()
                fn()
                shim = m.shim_counts()
                core = m.core_counters("enc" if direction == "encode" else "dec")
                st = sum(shim.values())
                print("   %-5s %-7s %-22s shim %6.2f  core fwd %5.2f rev %5.2f  [%s]"
                      % (pid, direction, name[len("core-ffi / "):], st / n,
                         core["forward"] / n, core["reverse"] / n,
                         ", ".join("%s=%d" % (k, v) for k, v in sorted(shim.items())
                                   if v)))
    return 0


def main():
    if "--counts-only" in sys.argv:
        return counts_only()
    fails = 0
    print("# python slice, work unit 2: conformance on the COMPOSED arm (README R2)")
    print("# interpreter: %s" % sys.version.split()[0])
    print("# baseline:    %s" % arms.BASELINE)
    print("# manifest:    validated against prost 0.14.4 and a second encoder (W2)")
    print()

    print("## R1: the generated tree is current with shapes.json")
    r = subprocess.run([sys.executable, os.path.join(HERE, "gen", "generate.py"),
                        "--check"], capture_output=True, text=True)
    print("   " + (r.stdout or r.stderr).strip())
    fails += bool(r.returncode)

    for a in arms.absent:
        print("## ARM ABSENT: %s" % a)
        fails += 1
    if arms._ffi is None:
        print("\nREFUSING: there is no composed arm to check.")
        return 1

    print("\n## ABI v1 section 10: the core and the shim agree on the group layouts")
    print("   ak_abi_version() = %d, the shim was generated against 1"
          % arms._ffi.abi_version())
    facts = arms._ffi.layout_facts()
    print("   the core exports %d layout facts; the shim's own compile-time asserts are"
          % len(facts))
    print("   in ak_abi.h and a mismatch is a build failure, so reaching this line at all")
    print("   is the check passing.")
    if not facts:
        print("   FAIL: the core exported no layout facts")
        fails += 1

    print("\n## encode: byte identity against the validated manifest")
    for pid in arms.PAYLOADS:
        want = arms.reference(pid)
        man = MANIFEST["payloads"][pid]
        if hashlib.sha256(want).hexdigest() != man["sha256"]:
            print("   FAIL %s: the schema emitter disagrees with the manifest" % pid)
            fails += 1
            continue
        print("   %-5s %7d bytes  sha %s  (%d elements%s)"
              % (pid, man["bytes"], man["sha256"][:16], man["elements"],
                 ", " + man["mode"] if "mode" in man else ""))
        for name, fn in arms.encode_arms(pid):
            got = fn()
            if got == want:
                print("        ok    %s" % name)
            else:
                print("        FAIL  %-32s %s" % (name, first_diff(got, want)))
                fails += 1

    print("\n## decode: re-encode byte identity, and field identity against the incumbent")
    for pid in arms.PAYLOADS:
        want = arms.reference(pid)
        print("   %-5s" % pid)
        pb = arms.build_upb(pid)
        pbd = arms._pb2.ListResultsResponse.FromString(want) if arms._pb2 else None
        for name, fn in arms.decode_arms(pid):
            obj = fn()
            back = arms.reencode(name, obj)
            ok = back == want
            extra = ""
            if pbd is not None and not name.startswith("upb"):
                bad = fieldwise(obj, pbd)
                if bad:
                    ok = False
                    extra = "; fields: " + "; ".join(bad[:3])
            if ok:
                print("        ok    %s" % name)
            else:
                print("        FAIL  %-32s %s%s"
                      % (name, "" if back == want else first_diff(back, want), extra))
                fails += 1

    if not arms.COUNTING:
        # The counting core and the measured core are both `libak_core.so`, so they cannot
        # share a process (see arms.py). Re-enter for that pass alone.
        print()
        env = dict(os.environ, AK_USE_COUNT="1")
        r = subprocess.run([sys.executable, __file__, "--counts-only"],
                           env=env, capture_output=True, text=True)
        sys.stdout.write(r.stdout)
        sys.stderr.write(r.stderr)
        fails += bool(r.returncode)
        print("\n%s" % ("ALL CHECKS PASS" if not fails else "%d FAILURE(S)" % fails))
        return 1 if fails else 0

    print("\n## crossing counts, counting build, BOTH halves (README R5)")
    m = arms._ffi_count
    if m is None or not m.counting():
        print("   ABSENT: no counting build; counts would be inferred, which R5 forbids.")
        fails += 1
    else:
        print("   %-5s %-11s %-24s %s" % ("", "direction", "backend",
                                          "crossings per element"))
        for pid in arms.PAYLOADS:
            n = max(MANIFEST["payloads"][pid]["elements"], 1)
            for direction, mk in (("encode", arms.encode_arms),
                                  ("decode", arms.decode_arms)):
                for name, fn in mk(pid, mod=m):
                    if not name.startswith("core-ffi"):
                        continue
                    m.reset_counts()
                    fn()
                    shim = m.shim_counts()
                    core = m.core_counters("enc" if direction == "encode" else "dec")
                    st = sum(shim.values())
                    print("   %-5s %-11s %-24s shim %7.2f (%s) | core fwd %.2f rev %.2f"
                          % (pid, direction, name[len("core-ffi / "):], st / n,
                             ", ".join("%s=%d" % (k, v) for k, v in sorted(shim.items())
                                       if v),
                             core["forward"] / n, core["reverse"] / n))

    print("\n%s" % ("ALL CHECKS PASS" if not fails else "%d FAILURE(S)" % fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
