#!/usr/bin/env python3
"""The java slice's generator.

R1: one description drives everything, and there is no hand-written codec anywhere in the
comparison. This file imports, READ-ONLY:

  * the rust slice's `ir.py` (which itself imports `ffi/schema/emit/shapes.py`) and
    `rust_abi.py`, so the core this slice measures against is emitted by the SAME emitter
    the rust and cpp slices measure -- three hosts over one core rather than three cores;
  * the cpp slice's `cpp_header.py` and `cpp_layout.py`, so the C header the JNI shim
    compiles against and the core's run-time layout export are the ones the cpp slice
    already validated, and a group cannot be laid out one way for C++ and another for the
    shim.

Nothing under `ffi/poc/rust/` or `ffi/poc/cpp/` is written by this script.

What is this slice's own, as new backends over the same IR:

  src/generated/java/ak/shapes/*.java   the facade, the payload builder, arm R's codec,
                                        the binding, the group offsets
  src/generated/java8/...               the SAME description emitted at the Java 8 level
  native/generated/*.c                  the JNI shim: entry points and loop trampolines
  core/src/generated/codec.rs           the core behind the C ABI  (rust_abi.emit_codec)
  core/src/generated/layout.rs          section 10's run-time layout export
  native/generated/ak_abi.h             the C ABI header             (cpp_header.emit)

  gen/generate.py            write the generated files
  gen/generate.py --check    fail if what is committed is not what this would write
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RUSTGEN = os.path.abspath(os.path.join(HERE, "..", "..", "rust", "gen"))
CPPGEN = os.path.abspath(os.path.join(HERE, "..", "..", "cpp", "gen"))
# THIS directory first. Both of the other two contain a `generate.py`, and the cpp slice
# records what happens when the order is wrong: an `import generate` anywhere resolves to
# somebody else's generator and emits their files instead of ours.
sys.path.insert(0, CPPGEN)
sys.path.insert(0, RUSTGEN)
sys.path.insert(0, HERE)

import ir as IR              # noqa: E402  (rust slice, read-only)
import rust_abi              # noqa: E402  (rust slice, read-only)
import cpp_header            # noqa: E402  (cpp slice, read-only)
import cpp_layout            # noqa: E402  (cpp slice, read-only)
import javanames as N        # noqa: E402
import java_facade           # noqa: E402
import java_build            # noqa: E402
import java_codec            # noqa: E402
import java_layout           # noqa: E402
import java_binding          # noqa: E402
import java_jni              # noqa: E402
import java_pbbuild          # noqa: E402
import java_arms             # noqa: E402
import pbarms                # noqa: E402
import java_ffiarms          # noqa: E402

ROOT = os.path.dirname(HERE)

# The same roots as the rust and cpp slices, so the three columns cover the same shapes.
ROOTS = [
    "ListResultsResponse",          # M1
    "ListTasksDetailedResponse",    # M2
    "ListProbeResponse",            # M3
    "ListTaskSummaryResponse",      # M4, the adapter site
    "UploadResultDataMessage",      # M5, bulk bytes
    "ListMetricsResponse",          # M6, packed scalars (control) and a packed enum (not)
    "DualResponse",                 # M7, control, decode only
]

# README 5.1: the floor and the target may be different code, and in Java they must be,
# because Java has no preprocessor. One description, one generator, TWO EMITTED SOURCE
# TREES -- which is literally what README 5.1's first condition asks for. The only thing
# the level changes is how the binding reaches a String's code units; see java_binding.
LEVELS = [("java17", 17), ("java8", 8)]

JDIR = "src/generated/%s/ak/shapes"


def targets(ir):
    # ABI v1 section 8's refusal, at generator time, before a line is emitted.
    for root in ir.roots:
        IR.check_direct(ir, root)

    # The codec is emitted FIRST because it allocates the length-prefix sites.
    codec = rust_abi.emit_codec(ir)
    header, layout_h, _layout_names_h = cpp_header.emit(ir)

    out = {
        "native/generated/ak_abi.h": header,
        "native/generated/ak_layout.h": layout_h,
        "core/src/generated/codec.rs": codec,
        "core/src/generated/layout.rs": cpp_layout.emit(ir),
        "native/generated/shim.c": java_jni.emit(ir),
        "src/generated/shared/ak/NativeEntry.java": java_jni.emit_java(ir),
    }

    for level_dir, level in LEVELS:
        d = JDIR % level_dir
        for fn, text in java_facade.emit_enums(ir).items():
            out["%s/%s" % (d, fn)] = text
        for fn, text in java_facade.emit_types(ir).items():
            out["%s/%s" % (d, fn)] = text
        out["%s/Build.java" % d] = java_build.emit(ir)
        out["%s/Codec.java" % d] = java_codec.emit(ir)
        out["%s/Layout.java" % d] = java_layout.emit_java(ir)
        out["%s/Binding.java" % d] = java_binding.emit(ir, level=level)
        out["%s/PbBuild.java" % d] = java_pbbuild.emit(ir)
        out["%s/Arms.java" % d] = java_arms.emit(ir)
        out["%s/PbArms.java" % d] = pbarms.emit(ir)
        out["%s/FfiArms.java" % d] = java_ffiarms.emit(ir)

    # The BORROWED facade and its decode binding: the same emitters with a view type in
    # place of `String`. ABI v1 open decision 13's measurement arm, and it needs no ABI
    # change -- `ak_span` is already an offset into the buffer the host handed in.
    N.set_string_type("ak.Utf8View")
    try:
        # Emitted at BOTH levels. A borrowed view is Java 8 clean, so the floor can carry
        # the arm too; what the floor is not asked for is a ratio from it (README 5.2).
        for level_dir, level in LEVELS:
            b = "src/generated/%s/ak/borrow" % level_dir
            for fn, text in java_facade.emit_types(ir, ns="ak.borrow").items():
                out["%s/%s" % (b, fn)] = text
            out["%s/Codec.java" % b] = java_codec.emit(ir, ns="ak.borrow")
            out["%s/Binding.java" % b] = java_binding.emit(ir, level=level, ns="ak.borrow")
            out["%s/FfiArms.java" % b] = java_ffiarms.emit(ir, ns="ak.borrow")
    finally:
        N.set_string_type("String")
    return out


def main(argv):
    check = "--check" in argv
    ir = IR.load(ROOTS)
    bad = 0
    for rel, text in sorted(targets(ir).items()):
        path = os.path.join(ROOT, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        old = open(path).read() if os.path.exists(path) else None
        if check:
            if old != text:
                print("STALE %s" % rel)
                bad += 1
            else:
                print("ok    %s" % rel)
        else:
            if old != text:
                open(path, "w").write(text)
                print("wrote %s  (%d lines)" % (rel, text.count("\n") + 1))
            else:
                print("same  %s" % rel)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
