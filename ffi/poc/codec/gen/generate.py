#!/usr/bin/env python3
"""The shared core's generator. README R0.

Everything under `crates/` that is generated is generated here, from
`ffi/schema/shapes.json` by way of `ir.py`, and there is exactly one copy of each emitted
file because there is exactly one core:

  crates/ak-abi/src/generated/abi.rs    the ABI structs        (rust_abi.emit_abi)
  crates/ak-core/src/generated/codec.rs the codec              (rust_abi.emit_codec)
  crates/ak-core/src/generated/layout.rs section 10's run-time layout export
                                                               (cpp_layout.emit)

  gen/generate.py            write them
  gen/generate.py --check    fail if what is committed is not what this would write

**Every slice's generator also writes these three paths, and that is deliberate.** The
rust, cpp and java generators each already emitted the core they measured; after R0 they
emit it *here* instead of into their own tree, so each slice's `gen/generate.py --check`
gates the shared core as well as its own files. That is R0's "every addition re-runs every
slice's gate", for free and without a new mechanism -- a change to `rust_abi.py` that only
one slice regenerates is a red `--check` in the other three. The three writers agree by
construction (one emitter, one description) and `gen/one_core.sh` checks that they still
do; they are not safe to run CONCURRENTLY, and no runner does.

The emitters themselves live beside this file rather than inside a slice, which was the
other half of the same defect: `ir.py`, `rust_abi.py`, `rust_core.py` and `rustnames.py`
were in `poc/rust/gen/` and three slices reached into that directory to import them.
`cpp_layout.py` keeps the name it was born with in `poc/cpp/gen/`; it emits Rust for this
crate and nothing C++, and renaming it would be an edit to a slice's imports rather than a
path change.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import ir as IR              # noqa: E402
import rust_abi              # noqa: E402
import cpp_layout            # noqa: E402

ROOT = os.path.dirname(HERE)

# The roots every slice covers, so one core serves the same shapes to all of them.
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
    return {
        "crates/ak-abi/src/generated/abi.rs": rust_abi.emit_abi(ir),
        "crates/ak-core/src/generated/codec.rs": codec,
        "crates/ak-core/src/generated/layout.rs": cpp_layout.emit(ir),
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
