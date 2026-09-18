#!/usr/bin/env python3
"""The rust slice's generator.

One description drives everything (README R1): it imports ffi/schema/emit/shapes.py rather
than re-parsing shapes.json, and emits every arm's codec. There is no hand-written codec in
this slice.

  gen/generate.py            write the generated files
  gen/generate.py --check    fail if what is committed is not what this would write

What it emits, and which arm each file serves:

  crates/facade/src/generated/types.rs        the facade types (arms armonik, core-native, core-ffi-rust)
  crates/facade/src/generated/prost_impl.rs   arm `armonik`
  crates/facade/src/generated/build.rs        payload construction over the facade
  crates/facade/src/generated/core_native.rs  arm `core-native`: the no-boundary control (R3)
  crates/ak-abi/src/generated/abi.rs          the C ABI surface of design/ABI-v1.md
  crates/ak-core/src/generated/codec.rs       the core behind that ABI
  crates/bench/src/generated/binding.rs       arm `core-ffi-rust`: the host binding
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import ir as IR              # noqa: E402
import rust_facade           # noqa: E402
import rust_build            # noqa: E402
import rust_core            # noqa: E402
import rust_abi             # noqa: E402

ROOT = os.path.dirname(HERE)

# Stage 2 covers M1 over P1.1 and P1.2. Stage 3 adds the rest of design/SHAPES.md by
# adding roots here; nothing else in the generator is per-payload.
ROOTS = ["ListResultsResponse", "ListTasksDetailedResponse"]


def targets(ir):
    # The codec is emitted FIRST because it allocates the length-prefix sites, and the
    # header carries their names.
    codec = rust_abi.emit_codec(ir)
    return {
        "crates/facade/src/generated/types.rs": rust_facade.emit_types(ir),
        "crates/facade/src/generated/prost_impl.rs": rust_facade.emit_prost_impl(ir),
        "crates/facade/src/generated/build.rs": rust_build.emit(ir),
        "crates/facade/src/generated/core_native.rs": rust_core.emit_core_native(ir),
        "crates/ak-abi/src/generated/abi.rs": rust_abi.emit_abi(ir),
        "crates/ak-core/src/generated/codec.rs": codec,
        "crates/harness/src/generated/binding.rs": rust_abi.emit_binding(ir),
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
