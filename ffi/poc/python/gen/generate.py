#!/usr/bin/env python3
"""The python slice's generator.  One description drives everything (README R1).

It imports, READ-ONLY:

*   the shared core's `ir.py` from `poc/codec/gen/`, which itself imports
    `ffi/schema/emit/shapes.py`, so the core this slice measures against is the core
    every slice measures (**R0**) -- N hosts over one core rather than N cores;
*   the cpp slice's `cpp_header.py`, so the `ak_abi.h` the shim compiles against is the
    one the cpp slice already validated against the Rust `#[repr(C)]`.  A group cannot be
    laid out one way for C++ and another for the shim.

**Nothing under `poc/codec/` is written by this script.**  The core's `codec.rs` and
`layout.rs` are emitted by the slices that own those backends and are already current;
this slice consumes them.

What it emits, all into `gen/out/` and all committed:

| file | what it is |
|---|---|
| `facade.py` | the facade, plain and `__slots__` |
| `pycodec.py` | the generated pure-Python codec, encode and decode: **R3's no-boundary control** |
| `payload_values.py` | facade objects carrying exactly the manifest's values |
| `_akcodec_gen.c` | work unit 1's C shim with no core behind it, kept so its column stays defensible |
| `binding.c` | **work unit 2's composed arm**: the generated C shim over the shared core |
| `ak_abi.h` | the C ABI header, from `cpp_header.emit` |

Run:  python gen/generate.py [--check]
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))       # .../poc/python/gen
SLICE = os.path.dirname(HERE)                           # .../poc/python
POC = os.path.dirname(SLICE)                            # .../poc
FFI = os.path.dirname(POC)                              # .../ffi
CODECGEN = os.path.join(POC, "codec", "gen")
CPPGEN = os.path.join(POC, "cpp", "gen")
SCHEMA_EMIT = os.path.join(FFI, "schema", "emit")

# THIS directory first.  Both of the other two contain a `generate.py`, and the cpp slice
# records what happens when the order is wrong: an `import generate` anywhere resolves to
# somebody else's generator and emits their files instead of ours.
for p in (SCHEMA_EMIT, CPPGEN, CODECGEN, HERE):
    if p not in sys.path:
        sys.path.insert(0, p)

import shapes as S       # noqa: E402  (the description, shared)
import ir as IR          # noqa: E402  (shared core, read-only)
import cpp_header        # noqa: E402  (cpp slice, read-only)

import walk as W         # noqa: E402
import py_facade         # noqa: E402
import py_codec          # noqa: E402
import py_shim           # noqa: E402
import py_binding        # noqa: E402
import py_values         # noqa: E402

OUT = os.path.join(HERE, "out")

# The header and the layout facts must match the CORE, which carries every root, so the
# IR is built over the same seven roots the rust, cpp and java slices use.  This slice's
# OWN backends are restricted to `walk.SCOPE`, and they raise on anything outside it.
ABI_ROOTS = [
    "ListResultsResponse",          # M1  <- the only one this slice's backends cover
    "ListTasksDetailedResponse",    # M2
    "ListProbeResponse",            # M3
    "ListTaskSummaryResponse",      # M4
    "UploadResultDataMessage",      # M5
    "ListMetricsResponse",          # M6
    "DualResponse",                 # M7
]


def outputs():
    schema = S.load()
    scope, root = W.SCOPE, W.ROOT
    ir = IR.load(ABI_ROOTS)
    header, _layout_h, _names_h = cpp_header.emit(ir)
    return {
        "facade.py": py_facade.emit(schema, scope),
        "pycodec.py": py_codec.emit(schema, scope, root, W.ROOTS),
        "payload_values.py": py_values.emit(schema, scope, root),
        "_akcodec_gen.c": py_shim.emit(schema, W.SCOPE_M1, root),
        "binding.c": py_binding.emit(ir, scope, W.ROOTS),
        "ak_abi.h": header,
    }


def main():
    os.makedirs(OUT, exist_ok=True)
    outs = outputs()
    check = "--check" in sys.argv
    stale = []
    for name, text in outs.items():
        p = os.path.join(OUT, name)
        old = open(p).read() if os.path.exists(p) else None
        if old != text:
            stale.append(name)
            if not check:
                with open(p, "w") as f:
                    f.write(text)
    if check:
        if stale:
            print("STALE, regenerate: %s" % ", ".join(stale))
            return 1
        print("generated tree is current with shapes.json (%d files)" % len(outs))
        return 0
    print("wrote %d files to %s%s"
          % (len(outs), OUT, (" (changed: %s)" % ", ".join(stale)) if stale else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
