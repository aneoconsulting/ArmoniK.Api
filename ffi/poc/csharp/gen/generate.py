#!/usr/bin/env python3
"""The C# slice's generator driver. FIX-PLAN WP5 step 4.

Every codec and binding this slice runs is rendered by the SHARED C# backends in
`ffi/poc/codec/gen/` from `plan.py`'s plans (the one generator, W14):

  cs_types.py     the facade types and the structural comparer        (per message set)
  cs_managed.py   the managed codec: encode (one and two pass), decode (per message set)
  cs_binding.py   the P/Invoke binding: groups, vtables, imports with LibraryImport under
                  `#if NET7_0_OR_GREATER` and DllImport otherwise, ak_init (R-G7), the
                  layout tables; and the RPC half from plan.rpc          (per message set)
  cs_host.py      the core-ffi host classes (push, push+capture, pull, encode, uencode)
  cs_layout_probe.py  the layout probe, parsed from the RUST declaration (R-E6)

This directory keeps harness glue only: `cs_values.py` (the value rules), `cs_build.py`
(payload builders), `cs_arms.py` (the arm table), `cs_proj.py` (the corpus projection and
root dispatch), `cs_registry.py` (the core-ffi arm registry and the corpus dispatch). None of
them decides a wire rule or a layout; they read the plan's descriptor view.

  gen/generate.py            write every generated file
  gen/generate.py --check    fail on drift, or if a shared C# backend imports anything but
                             the plan (the one-generator guard, seen failing on a plant)
"""
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CODECGEN = os.path.abspath(os.path.join(HERE, "..", "..", "codec", "gen"))
# THIS directory first: both directories contain a `generate.py`.
sys.path.insert(0, CODECGEN)
sys.path.insert(0, HERE)

import plan as P            # noqa: E402  the ONE rule layer
import cs_types             # noqa: E402  shared C# backends
import cs_managed           # noqa: E402
import cs_binding           # noqa: E402
import cs_host              # noqa: E402
import cs_layout_probe      # noqa: E402
import cs_values            # noqa: E402  glue
import cs_build             # noqa: E402
import cs_arms              # noqa: E402
import cs_proj              # noqa: E402
import cs_registry          # noqa: E402

ROOT = os.path.dirname(HERE)

ROOTS = [
    "ListResultsResponse",          # M1
    "ListTasksDetailedResponse",    # M2
    "ListProbeResponse",            # M3, the oneof and explicit presence
    "ListTaskSummaryResponse",      # M4, the adapter site
    "UploadResultDataMessage",      # M5, bulk bytes (a direct argument, ABI v1 section 8)
    "ListMetricsResponse",          # M6, packed scalars and a packed enum
    "DualResponse",                 # M7, control, decode only
]

SHARED = ["cs_names.py", "cs_types.py", "cs_managed.py", "cs_binding.py", "cs_host.py",
          "cs_layout_probe.py"]
GLUE = ["glue.py", "cs_values.py", "cs_build.py", "cs_arms.py", "cs_proj.py", "cs_registry.py",
        "generate.py"]


def payload_roots():
    path = os.path.normpath(os.path.join(ROOT, "..", "..", "schema", "generated", "manifest.json"))
    with open(path) as f:
        man = json.load(f)
    return [(pid, row["root"]) for pid, row in man["payloads"].items()]


def corpus_plans():
    names = P.corpus_messages()
    full = P.load_corpus(names)                    # every message: facade, managed codec
    abi_roots, refused = P.expressible_roots(names, full)
    abi = P.load_corpus(abi_roots)                 # the SAME roots the corpus core has
    for root in abi.roots:
        P.check_direct(abi, root)
    return full, abi, refused


def targets():
    p = P.load(ROOTS)
    for root in p.roots:
        P.check_direct(p, root)
    codec, _ = cs_managed.emit(p, "Armonik.Ffi.Facade")
    full, abi, refused = corpus_plans()
    ccodec, _ = cs_managed.emit(full, "Armonik.Ffi.Corpus", ["Armonik.Ffi.Facade"])
    return {
        "src/Facade/Generated/Types.cs": cs_types.emit_types(p, "Armonik.Ffi.Facade"),
        "src/Facade/Generated/Eq.cs": cs_types.emit_eq(p, "Armonik.Ffi.Facade"),
        "src/Facade/Generated/Codec.cs": codec,
        "src/Facade/Generated/Values.cs": cs_values.emit(p),
        "src/Facade/Generated/Build.cs": cs_build.emit(p, cs_build.FacadeSink(), ROOTS),
        "src/Harness/Generated/BuildGp.cs": cs_build.emit(p, cs_build.GpSink(), ROOTS),
        "src/Harness/Generated/Arms.cs": cs_arms.emit(p),
        "src/Harness/Generated/Abi.cs": cs_binding.emit_abi(p, "Armonik.Ffi.Harness"),
        "src/Harness/Generated/CoreFfi.cs": cs_host.emit_host(p, "Armonik.Ffi.Harness", "Armonik.Ffi.Facade"),
        "src/Harness/Generated/CoreArms.cs": cs_registry.emit_registry(p, payload_roots()),
        "src/Rpc/Generated/RpcAbi.cs": cs_binding.emit_rpc(p, "Armonik.Ffi.Rpc"),
        "src/Corpus/Generated/Types.cs": cs_types.emit_types(full, "Armonik.Ffi.Corpus"),
        "src/Corpus/Generated/Eq.cs": cs_types.emit_eq(full, "Armonik.Ffi.Corpus"),
        "src/Corpus/Generated/Codec.cs": ccodec,
        "src/Corpus/Generated/Proj.cs": cs_proj.emit(full),
        "src/Corpus/Generated/Abi.cs": cs_binding.emit_abi(abi, "Armonik.Ffi.Corpus"),
        "src/Corpus/Generated/CoreFfi.cs": cs_host.emit_host(abi, "Armonik.Ffi.Corpus", "Armonik.Ffi.Corpus"),
        "src/Corpus/Generated/Dispatch.cs": cs_registry.emit_corpus_dispatch(abi, refused),
        "abi/src/main.rs": cs_layout_probe.emit(),
    }


def guard():
    """The one-generator guard (FIX-PLAN WP5 item 7), applied to the shared C# backends
    with the shared generator's own rule (`poc/codec/gen/generate.py`, loaded by path, not
    edited): a backend imports the plan, never the IR or a description. Seen failing on a
    plant. And this directory's glue must not import the IR either."""
    spec = importlib.util.spec_from_file_location("codec_generate", os.path.join(CODECGEN, "generate.py"))
    cg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cg)
    sources = {b: open(os.path.join(CODECGEN, b)).read() for b in SHARED}
    bad = cg.guard_violations(sources)
    glue = {g: open(os.path.join(HERE, g)).read() for g in GLUE}
    gl = cg.guard_violations(glue)
    # json is how generate.py reads ffi/schema's MANIFEST (payload ids), not a description.
    gl.pop("generate.py", None) if gl.get("generate.py") == ["json"] else None
    plant = cg.guard_violations({"planted.py": "import ir as IR\nfrom shapes import load\n"})
    caught = plant.get("planted.py") == ["ir", "shapes"]
    return bad, gl, caught


def main(argv):
    check = "--check" in argv
    bad = 0
    if check:
        violations, glue_bad, caught = guard()
        for mod, hit in sorted(violations.items()):
            print("GUARD %s imports %s: a backend takes plans only" % (mod, ", ".join(hit)))
            bad += 1
        for mod, hit in sorted(glue_bad.items()):
            print("GUARD glue %s imports %s: glue reads the plan, not the IR" % (mod, ", ".join(hit)))
            bad += 1
        if not caught:
            print("GUARD the planted violation was NOT caught: the guard is blind")
            bad += 1
        if not violations and not glue_bad and caught:
            print("guard %d shared C# backends and %d glue modules import no IR or description; "
                  "a planted IR import is caught" % (len(SHARED), len(GLUE)))
    for rel, text in sorted(targets().items()):
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
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
