#!/usr/bin/env python3
"""The C# slice's generator.

One description drives everything (README R1): it imports
`ffi/schema/emit/shapes.py` rather than re-parsing `shapes.json`, and emits
every arm's codec and both payload-construction routes. There is no
hand-written codec in this slice; `src/Facade/Wire.cs` is the runtime beneath
the generated traversal and knows nothing about any message.

  gen/generate.py            write the generated files
  gen/generate.py --check    fail if what is committed is not what this writes

What it emits, and which arm each file serves:

  src/Facade/Generated/Types.cs    the facade types (every arm)
  src/Facade/Generated/Eq.cs       structural equality, for the decode oracle
  src/Facade/Generated/Values.cs   the deterministic value rules
  src/Facade/Generated/Build.cs    payload construction over the facade
  src/Facade/Generated/Codec.cs    the MANAGED CONTROL: encode and decode (R3)
  src/Harness/Generated/BuildGp.cs payload construction over Google.Protobuf
  src/Harness/Generated/Arms.cs    the per-payload arm table

The `core-ffi` arm is NOT emitted. ABI v1 open decision 1 is unsettled, so the
ABI is not frozen and a binding built against a draft would be a number about
the draft. `ir.py` and the sink split in `cs_build.py` are laid out so that
backend drops in beside these without moving anything.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import ir as IR             # noqa: E402
import cs_facade           # noqa: E402
import cs_values           # noqa: E402
import cs_build            # noqa: E402
import cs_managed          # noqa: E402
import cs_arms             # noqa: E402

ROOT = os.path.dirname(HERE)

ROOTS = [
    "ListResultsResponse",          # M1
    "ListTasksDetailedResponse",    # M2
    "ListProbeResponse",            # M3, the oneof and explicit presence
    "ListTaskSummaryResponse",      # M4, the adapter site
    "UploadResultDataMessage",      # M5, bulk bytes
    "ListMetricsResponse",          # M6, packed scalars (control) and a packed enum (not)
    "DualResponse",                 # M7, control, decode only
]


def targets(ir):
    codec, sites = cs_managed.emit(ir)
    return {
        "src/Facade/Generated/Types.cs": cs_facade.emit_types(ir),
        "src/Facade/Generated/Eq.cs": cs_facade.emit_eq(ir),
        "src/Facade/Generated/Values.cs": cs_values.emit(ir),
        "src/Facade/Generated/Build.cs": cs_build.emit(ir, cs_build.FacadeSink(), ROOTS),
        "src/Facade/Generated/Codec.cs": codec,
        "src/Harness/Generated/BuildGp.cs": cs_build.emit(ir, cs_build.GpSink(), ROOTS),
        "src/Harness/Generated/Arms.cs": cs_arms.emit(ir),
    }, sites


def main(argv):
    check = "--check" in argv
    ir = IR.load(ROOTS)
    files, sites = targets(ir)
    bad = 0
    for rel, text in sorted(files.items()):
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
                with open(path, "w") as f:
                    f.write(text)
                print("wrote %s (%d lines)" % (rel, text.count("\n")))
            else:
                print("same  %s" % rel)
    print("%d messages, %d length-prefix sites, walker guard passed on %d message(s) with a oneof"
          % (len(ir.messages), len(sites), IR.check_walker(ir)))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
