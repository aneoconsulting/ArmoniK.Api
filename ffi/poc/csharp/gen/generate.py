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

  src/Harness/Generated/Abi.cs      the C ABI of ABI v1, at the Rust build's offsets
  src/Harness/Generated/CoreFfi.cs  the core-ffi host binding for M1
  src/Harness/Generated/CoreFfi2.cs the core-ffi host binding for M2

The core-ffi backends were held while ABI v1 open decision 1 was unsettled, on
the grounds that a binding built against a draft is a number about the draft.
Decision 1 is settled and they are emitted.
"""
import json
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
import cs_abi              # noqa: E402
import cs_coreffi          # noqa: E402
import cs_coreffi2         # noqa: E402
import cs_proj             # noqa: E402
import cs_core             # noqa: E402
import protoparse          # noqa: E402

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


CORPUS = os.path.normpath(os.path.join(ROOT, "..", "..", "corpus", "generated"))


def corpus_ir():
    """The corpus's READER view, parsed from `corpus.proto`.

    `ffi/corpus/CONTRACT.md` rule 0: generate from `corpus.proto`, NEVER from
    `corpus_superset.proto`. The difference between the two files is the
    corpus's entire unknown-field claim, and a slice that generates from the
    superset knows every field and tests nothing.
    """
    schema = protoparse.parse(os.path.join(CORPUS, "corpus.proto"))
    check_front_ends_agree(schema)
    return IR.Ir(schema, list(schema["messages"]))


def check_front_ends_agree(corpus):
    """The two front ends must describe the shared messages identically.

    `corpus.proto` and `shapes.json` are generated from one description
    (`ffi/corpus/emit/build.py` reads `../schema/shapes.json`), so a field that
    differs between what this slice parses out of the .proto and what it reads
    out of the JSON is a defect in THIS parser, not a disagreement between the
    descriptions. Nineteen messages and three enums overlap; every attribute
    the backends read is compared.
    """
    schema = IR.S.load()
    bad = []
    for name in sorted(set(corpus["messages"]) & set(schema["messages"])):
        a = {f["name"]: f for f in corpus["messages"][name]["fields"]}
        b = {f["name"]: f for f in IR.S.fields(schema["messages"][name])}
        if set(a) != set(b):
            bad.append("%s: field sets differ (%s)" % (name, sorted(set(a) ^ set(b))))
            continue
        for k in sorted(a):
            for attr in ("tag", "kind", "of", "card", "presence", "key", "value_kind"):
                if a[k].get(attr) != b[k].get(attr):
                    bad.append("%s.%s %s: proto=%r json=%r"
                               % (name, k, attr, a[k].get(attr), b[k].get(attr)))
    for name in sorted(set(corpus["enums"]) & set(schema["enums"])):
        if corpus["enums"][name]["values"] != schema["enums"][name]["values"]:
            bad.append("enum %s differs" % name)
    if bad:
        raise SystemExit("the .proto front end disagrees with shapes.json:\n  "
                         + "\n  ".join(bad))
    return len(set(corpus["messages"]) & set(schema["messages"]))


def payload_roots():
    """[(payload id, root)] in manifest order, from ffi/schema's own manifest."""
    import re
    path = os.path.normpath(os.path.join(ROOT, "..", "..", "schema", "generated", "manifest.json"))
    with open(path) as f:
        man = json.load(f)
    return [(pid, row["root"]) for pid, row in man["payloads"].items()]


def targets(ir):
    codec, sites = cs_managed.emit(ir)
    cir = corpus_ir()
    ccodec, csites = cs_managed.emit(cir, ns="Armonik.Ffi.Corpus",
                                     extra_using=["Armonik.Ffi.Facade"])
    return {
        "src/Harness/Corpus/Types.cs": cs_facade.emit_types(cir, ns="Armonik.Ffi.Corpus"),
        "src/Harness/Corpus/Eq.cs": cs_facade.emit_eq(cir, ns="Armonik.Ffi.Corpus"),
        "src/Harness/Corpus/Codec.cs": ccodec,
        "src/Harness/Corpus/Proj.cs": cs_proj.emit(cir),
        "src/Facade/Generated/Types.cs": cs_facade.emit_types(ir),
        "src/Facade/Generated/Eq.cs": cs_facade.emit_eq(ir),
        "src/Facade/Generated/Values.cs": cs_values.emit(ir),
        "src/Facade/Generated/Build.cs": cs_build.emit(ir, cs_build.FacadeSink(), ROOTS),
        "src/Facade/Generated/Codec.cs": codec,
        "src/Harness/Generated/BuildGp.cs": cs_build.emit(ir, cs_build.GpSink(), ROOTS),
        "src/Harness/Generated/Arms.cs": cs_arms.emit(ir),
        "src/Harness/Generated/Abi.cs": cs_abi.emit(ir),
        **{"src/Harness/Generated/Core_%s.cs" % r: cs_core.emit(ir, r) for r in ROOTS},
        "src/Harness/Generated/CoreArms.cs": cs_core.emit_registry(ir, ROOTS, payload_roots()),
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
