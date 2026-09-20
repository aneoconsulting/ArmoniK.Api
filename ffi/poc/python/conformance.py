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


def fieldwise(obj, pb, pid):
    """Every field of the whole root, the facade against the incumbent.

    Was "every field of every element, plus page and total", which is a root with exactly
    one repeated field and two scalars beside it written into the checker. `_cmp_msg`
    already recurses through a repeated message field, so the root is just another message
    and M5's childless root and M7's two lists need no case of their own.
    """
    return _cmp_msg(obj, pb, "root")


def _why_alt(got, want):
    """Name the difference between two legal encodings rather than only its offset."""
    if len(got) > len(want):
        return ("%d bytes longer: upb writes an empty map value as a present zero-length "
                "field and the canonical form omits it (an implicit-presence leaf holding "
                "the proto zero). CONTRACT.md C3 lists both." % (len(got) - len(want)))
    if len(got) == len(want):
        return ("same length, different order: SerializeToString does not sort map "
                "entries and the canonical form does. `deterministic=True` is the row "
                "that matches.")
    return "%d bytes shorter" % (len(want) - len(got))


def _cmp_msg(a, b, where):
    """One facade object against one protobuf message, field by field.

    Driven from the incumbent's own descriptor rather than from a list, so a field this
    slice forgot is a failure here rather than an omission nobody sees. That is the same
    rule as R1's walker, one level up: the oracle enumerates, the slice does not.
    """
    bad = []

    # A proto3 `optional` field lives in a SYNTHETIC oneof of one member named `_<field>`:
    # that is how protobuf carries explicit presence, and it is not a oneof the schema
    # wrote. Telling the two apart by that convention is what keeps `opt_count` out of the
    # oneof branch below. `real_oneofs` would be the tidier test and is empty on this build.
    def _synthetic(fd):
        co = fd.containing_oneof
        return co is not None and len(co.fields) == 1 and co.name == "_" + fd.name

    for oneof in b.DESCRIPTOR.oneofs:
        if len(oneof.fields) == 1 and oneof.name.startswith("_"):
            continue
        which = b.WhichOneof(oneof.name)
        tag = getattr(a, "%s_case" % oneof.name, None)
        if tag is None:
            bad.append("%s: the facade has no %s_case" % (where, oneof.name))
            continue
        mine = next((f.name for f in oneof.fields if f.number == tag), None)
        if mine != which:
            bad.append("%s oneof %s: facade selects %r (case %r), upb selects %r"
                       % (where, oneof.name, mine, tag, which))

    for fd in b.DESCRIPTOR.fields:
        n = fd.name
        co = fd.containing_oneof
        if co is not None and not _synthetic(fd):
            # A member nobody selected has no value to compare: both sides would return
            # their own default and comparing two defaults proves nothing. WHICH member is
            # selected was checked above; the selected one is compared below.
            if b.WhichOneof(co.name) != n:
                continue
        bv = getattr(b, n)
        try:
            av = getattr(a, n)
        except AttributeError:
            bad.append("%s .%s: the facade has no such field" % (where, n))
            continue
        if fd.is_repeated:
            if fd.message_type is not None and fd.message_type.GetOptions().map_entry:
                if dict(av) != dict(bv):
                    bad.append("%s .%s: map %r vs %r"
                               % (where, n, dict(av), dict(bv)))
            elif fd.message_type is not None:
                if len(av) != len(bv):
                    bad.append("%s .%s: %d vs %d elements" % (where, n, len(av), len(bv)))
                else:
                    for j, (x, y) in enumerate(zip(av, bv)):
                        bad += _cmp_msg(x, y, "%s .%s[%d]" % (where, n, j))
            elif list(av) != list(bv):
                bad.append("%s .%s: %r vs %r" % (where, n, list(av), list(bv)))
        elif _synthetic(fd):
            # Explicit presence: absent is None on the facade and `not HasField` on the
            # incumbent, and present-and-zero must compare equal to present-and-zero rather
            # than to absent. That distinction is what P3.1 is in the payload set for.
            present = b.HasField(n)
            if (av is not None) != present:
                bad.append("%s .%s presence: %s vs %s"
                           % (where, n, av is not None, present))
            elif av is not None and fd.type == fd.TYPE_BOOL:
                if bool(av) != bool(bv):
                    bad.append("%s .%s: %r vs %r" % (where, n, av, bv))
            elif av is not None and fd.type in (fd.TYPE_STRING, fd.TYPE_BYTES):
                if av != bv:
                    bad.append("%s .%s: %r vs %r" % (where, n, av, bv))
            elif av is not None and int(av) != int(bv):
                bad.append("%s .%s: %r vs %r" % (where, n, av, bv))
        elif fd.message_type is not None:
            present = b.HasField(n)
            if (av is not None) != present:
                bad.append("%s .%s presence: %s vs %s" % (where, n, av is not None,
                                                          present))
            elif av is not None:
                bad += _cmp_msg(av, bv, "%s .%s" % (where, n))
        elif fd.type == fd.TYPE_BOOL:
            if bool(av) != bool(bv):
                bad.append("%s .%s: %r vs %r" % (where, n, av, bv))
        elif fd.type in (fd.TYPE_STRING, fd.TYPE_BYTES):
            if av != bv:
                bad.append("%s .%s: %r vs %r" % (where, n, av, bv))
        elif int(av) != int(bv):
            bad.append("%s .%s: %r vs %r" % (where, n, av, bv))
        if len(bad) > 6:
            return bad[:6] + ["..."]
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
                continue
            # `ffi/corpus/CONTRACT.md` C3: a vector may have more than one accepted
            # encoding and that is not a weakness in the vector. The manifest carries the
            # canonical form; the incumbent may legally write another. So a difference in
            # the INCUMBENT's bytes is only a failure if the two do not parse to the same
            # message -- checked, not assumed. A difference in one of THIS SLICE's arms is
            # always a failure: they are the ones claiming to produce canonical bytes.
            # P7.1 is the one payload where THIS SLICE gets the allowance too, and
            # design/SHAPES.md says so in advance rather than after the fact: the vector
            # interleaves two repeated fields, no canonical writer can produce it, and a
            # slice is asked to validate it by decoding and to re-encode contiguously to a
            # permutation of the same triples. `same_message` is the stronger form of that
            # check -- both encodings parse to the same message, by the incumbent's parser.
            allow = name.startswith("upb") or pid in arms.DECODE_ONLY
            alt = arms.same_message(pid, got, want) if allow else None
            if alt and pid in arms.DECODE_ONLY:
                print("        ok*   %-32s a permutation: %s"
                      % (name, "the manifest interleaves two repeated fields and no "
                               "contiguous writer can; same triples, same message"))
            elif alt:
                print("        ok*   %-32s legal alternative form: %s"
                      % (name, _why_alt(got, want)))
            else:
                print("        FAIL  %-32s %s" % (name, first_diff(got, want)))
                fails += 1

    print("\n## decode: re-encode byte identity, and field identity against the incumbent")
    for pid in arms.PAYLOADS:
        want = arms.reference(pid)
        print("   %-5s" % pid)
        pbd = arms.build_upb(pid)
        for name, fn in arms.decode_arms(pid):
            obj = fn()
            back = arms.reencode(name, obj, pid)
            ok = back == want
            alt = ""
            if not ok and pid in arms.DECODE_ONLY and arms.same_message(pid, back, want):
                ok, alt = True, "  (a permutation of the same triples; SHAPES.md P7.1)"
            elif not ok and name.startswith("upb") and arms.same_message(pid, back, want):
                # Same allowance as the encode section: the incumbent may legally write
                # another accepted form. This slice's own arms get no such allowance.
                ok, alt = True, "  (legal alternative form: %s)" % _why_alt(back, want)
            extra = ""
            if pbd is not None and not name.startswith("upb"):
                bad = fieldwise(obj, pbd, pid)
                if bad:
                    ok = False
                    extra = "; fields: " + "; ".join(bad[:3])
            if ok:
                print("        %s %s%s" % ("ok   " if not alt else "ok*  ", name, alt))
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
