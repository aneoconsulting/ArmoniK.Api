#!/usr/bin/env python3
"""The cpp slice's generator.

R1: one description drives everything, and there is no hand-written codec anywhere in the
comparison. This file imports the rust slice's generator READ-ONLY -- `ir.py` (which itself
imports `ffi/schema/emit/shapes.py`), `rust_abi.py` and `rust_core.py` -- so the core this
slice measures against is emitted by the SAME emitter the rust slice measures, and the two
columns are two hosts over one core rather than two cores. Nothing under `ffi/poc/rust/` is
written by this script.

What is this slice's own, as new backends over the same IR:

  include/ak_abi.h                the C header of ABI-v1.md, emitted from the rust
                                  backend's own `group_fields`/`presence_bits`, so a group
                                  layout cannot be restated differently on the two sides
  include/generated/ak_layout.h   the host's compile-time view of those layouts
  src/generated/types.{h,cpp}     the C++ facade
  src/generated/odr.h             the layout facts the C++11 and C++17 TUs compare
  src/generated/build.{h,cpp}     the payload builder over the facade
  src/generated/pb_build.{h,cpp}  the payload builder over protoc's own types
  src/generated/core_native.{h,cpp}  arm `core-native-cpp`, the no-boundary control (R3)
  src/generated/binding.{h,cpp}   arm `core-ffi`, the host binding

  core/src/generated/codec.rs     the core behind the C ABI  (rust_abi.emit_codec)
  core/src/generated/layout.rs    section 10's run-time layout export

  gen/generate.py            write the generated files
  gen/generate.py --check    fail if what is committed is not what this would write
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RUSTGEN = os.path.abspath(os.path.join(HERE, "..", "..", "rust", "gen"))
# The rust generator first, so `import ir` and `import rust_abi` resolve there; this
# directory second, for the cpp backends.
sys.path.insert(0, HERE)
sys.path.insert(0, RUSTGEN)

import ir as IR              # noqa: E402  (rust slice, read-only)
import rust_abi              # noqa: E402  (rust slice, read-only)
import cpp_header            # noqa: E402
import cpp_facade            # noqa: E402
import cpp_build             # noqa: E402
import cpp_pbbuild           # noqa: E402
import cpp_core              # noqa: E402
import cpp_binding           # noqa: E402
import cpp_layout            # noqa: E402
import cpp_cases             # noqa: E402

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
    header, layout_h = cpp_header.emit(ir)
    return {
        "include/ak_abi.h": header,
        "include/generated/ak_layout.h": layout_h,
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
        "src/generated/cases.h": cpp_cases.emit(ir),
        "core/src/generated/codec.rs": codec,
        "core/src/generated/layout.rs": cpp_layout.emit(ir),
    }


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
