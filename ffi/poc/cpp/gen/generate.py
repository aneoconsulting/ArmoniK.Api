#!/usr/bin/env python3
"""The cpp slice's generator.

R1: one description drives everything, and there is no hand-written codec anywhere in the
comparison. This file imports the SHARED core's emitters from `poc/codec/gen/` -- `ir.py` (which
itself imports `ffi/schema/emit/shapes.py`), `rust_abi.py`, `rust_core.py` and
`cpp_layout.py` -- so the core this slice measures against is the one every slice measures
(R0), and the columns are N hosts over one core rather than N cores. The three core files
below are written into `poc/codec/` and nowhere else; nothing under another slice is
written by this script.

What is this slice's own, as new backends over the same IR:

  include/ak_abi.h                the C header of ABI-v1.md, emitted from the rust
                                  backend's own `group_fields`/`presence_bits`, so a group
                                  layout cannot be restated differently on the two sides
  include/generated/ak_layout.h   the host's compile-time view of those layouts
  include/generated/ak_layout_names.h  one name per fact, so a disagreement is NAMED
  src/generated/types.{h,cpp}     the C++ facade
  src/generated/odr.h             the layout facts the C++11 and C++17 TUs compare
  src/generated/build.{h,cpp}     the payload builder over the facade
  src/generated/pb_build.{h,cpp}  the payload builder over protoc's own types
  src/generated/core_native.{h,cpp}  arm `core-native-cpp`, the no-boundary control (R3)
  src/generated/binding.{h,cpp}   arm `core-ffi`, the host binding

  ../codec/crates/ak-core/src/generated/codec.rs   the shared core behind the C ABI
  ../codec/crates/ak-core/src/generated/layout.rs  section 10's run-time layout export

  gen/generate.py            write the generated files
  gen/generate.py --check    fail if what is committed is not what this would write
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CODECGEN = os.path.abspath(os.path.join(HERE, "..", "..", "codec", "gen"))
# THIS directory first and the shared core's generator second. Order matters and the
# comment that used to be here had it backwards: both directories contain a `generate.py`,
# so with CODECGEN in front any module that does `import generate` -- `gen/refusal_test.py`
# does -- silently gets the SHARED generator and emits its three files instead of this
# slice's twenty-one. Everything this file imports from the shared generator (`ir`,
# `rust_abi`, `rust_core`, `cpp_layout`) exists only there, so HERE-first resolves each
# import to the one place it lives.
sys.path.insert(0, CODECGEN)
sys.path.insert(0, HERE)

import ir as IR              # noqa: E402  (shared core)
import rust_abi              # noqa: E402  (shared core)
import cpp_header            # noqa: E402
import cpp_facade            # noqa: E402
import cpp_build             # noqa: E402
import cpp_pbbuild           # noqa: E402
import cpp_core              # noqa: E402
import cpp_binding           # noqa: E402
import cpp_layout            # noqa: E402  (shared core)
import cpp_cases             # noqa: E402
import cppnames              # noqa: E402

ROOT = os.path.dirname(HERE)

# The same roots as the rust slice, so the two columns cover the same shapes.
ROOTS = [
    "ListResultsResponse",          # M1
    "ListTasksDetailedResponse",    # M2
    "ListProbeResponse",            # M3
    "ListTaskSummaryResponse",      # M4, the adapter site
    "UploadResultDataMessage",      # M5, bulk bytes
    "ListMetricsResponse",          # M6, packed scalars (control) and a packed enum (not)
    "DualResponse",                 # M7, control, decode only
]


def targets(ir):
    # ABI v1 section 8's refusal, at generator time, before a line is emitted.
    for root in ir.roots:
        IR.check_direct(ir, root)

    # The codec is emitted FIRST because it allocates the length-prefix sites.
    codec = rust_abi.emit_codec(ir)
    header, layout_h, layout_names_h = cpp_header.emit(ir)
    return {
        "include/ak_abi.h": header,
        "include/generated/ak_layout.h": layout_h,
        "include/generated/ak_layout_names.h": layout_names_h,
        "src/generated/types.h": cpp_facade.emit_types(ir),
        "src/generated/types.cpp": cpp_facade.emit_types_impl(ir),
        "src/generated/odr.h": cpp_facade.emit_odr_asserts(ir),
        "src/generated/build.h": cpp_build.emit_header(ir),
        "src/generated/build.cpp": cpp_build.emit(ir),
        "src/generated/pb_build.h": cpp_pbbuild.emit_header(ir),
        "src/generated/pb_build.cpp": cpp_pbbuild.emit(ir),
        "src/generated/core_native.h": cpp_core.emit_header(ir),
        "src/generated/core_native.cpp": cpp_core.emit(ir),
        "src/generated/binding.h": cpp_binding.emit_header(ir),
        "src/generated/binding.cpp": cpp_binding.emit(ir),
        # The BORROWED facade and its binding: the same emitters with `ak::StringView` in
        # place of `std::string` and a namespace of their own. A measurement arm for
        # `bench` (how much of decode is the string copy), never the shipping facade --
        # the strings are valid only while the input buffer lives. ABI v1 needs no change:
        # `ak_span` is an offset into the buffer the host handed in.
        **_borrow(ir),
        "src/generated/cases.h": cpp_cases.emit(ir),
        # The shared core (R0), written here as well as from `poc/codec/gen` and from
        # the other slices' generators -- one emitter, one description, so this slice's
        # `--check` gates the core it measures.
        "../codec/crates/ak-core/src/generated/codec.rs": codec,
        "../codec/crates/ak-core/src/generated/layout.rs": cpp_layout.emit(ir),
    }


def _borrow(ir):
    cppnames.set_string_type("ak::StringView")
    try:
        out = {
            "src/generated/types_borrow.h":
                cpp_facade.emit_types(ir, ns="shapes_borrow", guard="AK_TYPES_BORROW_H"),
            "src/generated/types_borrow.cpp":
                cpp_facade.emit_types_impl(ir, ns="shapes_borrow",
                                           header="generated/types_borrow.h"),
            "src/generated/binding_borrow.h":
                cpp_binding.emit_header(ir, ns="shapes_borrow", guard="AK_BINDING_BORROW_H",
                                        types_h="generated/types_borrow.h"),
            "src/generated/binding_borrow.cpp":
                cpp_binding.emit(ir, ns="shapes_borrow", hdr="generated/binding_borrow.h"),
        }
    finally:
        cppnames.set_string_type("std::string")
        cpp_binding.NS[0] = "shapes"
    return out


def main(argv):
    check = "--check" in argv
    ir = IR.load(ROOTS)
    bad = 0
    for rel, text in targets(ir).items():
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
