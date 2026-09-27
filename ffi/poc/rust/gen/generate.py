#!/usr/bin/env python3
"""The rust slice's generator.

One description drives everything (README R1): it imports ffi/schema/emit/shapes.py rather
than re-parsing shapes.json, and emits every arm's codec. There is no hand-written codec in
this slice.

  gen/generate.py            write the generated files
  gen/generate.py --check    fail if what is committed is not what this would write

What it emits, and which arm each file serves:

  crates/facade/src/generated/types.rs        the facade types (arms armonik, core-native, core-ffi-rust)
  crates/facade/src/generated/types_nounk.rs  the same without `unknown_fields` (no-unknown build, R-H22);
                                              both rendered by codec/gen/rust_facade.py from the plan
  crates/facade/src/generated/prost_impl.rs   arm `armonik`
  crates/facade/src/generated/build.rs        payload construction over the facade
  crates/facade/src/generated/core_native.rs  arm `core-native`: the no-boundary control (R3),
                                              unknown fields DROPPED
  crates/facade/src/generated/core_native_retain.rs   the same, unknown fields RETAINED
  crates/ak-abi/src/generated/abi.rs          the C ABI surface of design/ABI-v1.md
  crates/ak-core/src/generated/codec.rs       the core behind that ABI
  crates/harness/src/generated/binding.rs     arm `core-ffi-rust`: the host binding

FIX-PLAN WP5 step 1: everything that carries a wire rule or a layout -- core-native, the
core, the ABI, the binding -- is rendered by `poc/codec/gen/` backends from ONE plan
(`poc/codec/gen/plan.py`). What stays here is this slice's own glue: the facade types, the
payload builder, and the `armonik` arm's prost impl (an incumbent-shaped arm, WP5 item 4).
They read the plan's descriptor view, not the IR.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
# R0: `ir.py`, `rust_abi.py`, `rust_core.py` and `rustnames.py` are the shared core's
# emitters and live at `poc/codec/gen/`. THIS directory stays first: both directories
# contain a `generate.py`, and with the other one in front any `import generate`
# silently resolves to it.
CODECGEN = os.path.abspath(os.path.join(HERE, "..", "..", "codec", "gen"))
sys.path.insert(0, CODECGEN)
sys.path.insert(0, HERE)

import plan as P             # noqa: E402  the ONE rule layer
import rust_facade           # noqa: E402
import rust_build            # noqa: E402
import rust_native           # noqa: E402  core-native, from the plan
import rust_abi              # noqa: E402  the core and its ABI, from the plan
import rust_binding          # noqa: E402  the host binding, from the plan
import rust_project          # noqa: E402  corpus glue: the projection of a facade value
import rust_corpus           # noqa: E402  corpus glue: the per-root dispatch table
import rust_campaign         # noqa: E402  campaign glue: per-root arm table, read-every-field

ROOT = os.path.dirname(HERE)

# Stage 2 covers M1 over P1.1 and P1.2. Stage 3 adds the rest of design/SHAPES.md by
# adding roots here; nothing else in the generator is per-payload.
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
        P.check_direct(ir, root)

    # The codec is emitted FIRST because it allocates the length-prefix sites, and the
    # header carries their names.
    codec = rust_abi.emit_codec(ir)
    return {
        "crates/facade/src/generated/types.rs": rust_facade.emit_types(ir),
        # FIX-PLAN R-H22: the no-unknown variant's facade (facade feature `unknown-fields` off).
        "crates/facade/src/generated/types_nounk.rs":
            rust_facade.emit_types(P.relower(ir, ir.options.with_unknown("drop"))),
        "crates/facade/src/generated/prost_impl.rs": rust_facade.emit_prost_impl(ir),
        "crates/facade/src/generated/build.rs": rust_build.emit(ir),
        "crates/facade/src/generated/core_native.rs": rust_native.emit_core_native(ir, "drop"),
        "crates/facade/src/generated/core_native_retain.rs": rust_native.emit_core_native(ir, "retain"),
        # The shared core (R0). Written from here as well as from `poc/codec/gen` and from
        # the other slices' generators, all from one emitter and one description, so
        # this slice's `--check` gates the core it measures.
        "../codec/crates/ak-abi/src/generated/abi.rs": rust_abi.emit_abi(ir),
        "../codec/crates/ak-core/src/generated/codec.rs": codec,
        "crates/harness/src/generated/binding.rs": rust_binding.emit_binding(ir),
        # WP5 step 10: the no-unknown variant's binding (harness feature `unknown-fields` off).
        "crates/harness/src/generated/binding_nounk.rs":
            rust_binding.emit_binding(P.relower(ir, ir.options.with_unknown("drop"))),
        # FIX-PLAN WP3 (design/CAMPAIGN.md): the campaign's per-root table and visitors.
        "crates/campaign/src/generated/roots.rs": rust_campaign.emit(ir),
    }


def corpus_targets():
    """WP5 item 6.1: the conformance corpus through the C ABI and core-native, in both
    unknown-field modes. `poc/rust/corpus/` is a workspace of its own, because it builds
    the core with the test-only `corpus` feature (a different ABI)."""
    names = P.corpus_messages()
    full = P.load_corpus(names)                      # every message: facade, core-native
    abi_roots, refused = P.expressible_roots(names, full)
    abi = P.load_corpus(abi_roots)                   # the SAME roots the corpus core has
    for root in abi.roots:
        P.check_direct(abi, root)
    ccodec = rust_abi.emit_codec(abi)
    return {
        "corpus/crates/facade/src/generated/types.rs": rust_facade.emit_types(full),
        "corpus/crates/facade/src/generated/types_nounk.rs":
            rust_facade.emit_types(P.relower(full, full.options.with_unknown("drop"))),
        "corpus/crates/facade/src/generated/core_native.rs":
            rust_native.emit_core_native(full, "drop"),
        "corpus/crates/facade/src/generated/core_native_retain.rs":
            rust_native.emit_core_native(full, "retain"),
        "corpus/crates/facade/src/generated/project.rs": rust_project.emit(full),
        "corpus/crates/harness/src/generated/binding.rs": rust_binding.emit_binding(abi),
        "corpus/crates/harness/src/generated/binding_nounk.rs":
            rust_binding.emit_binding(P.relower(abi, abi.options.with_unknown("drop"))),
        "corpus/crates/harness/src/generated/dispatch.rs":
            rust_corpus.emit_dispatch(full, abi_roots, refused),
        # The core for the corpus schema, written here too so this slice's --check gates it.
        "../codec/crates/ak-abi/src/generated_corpus/abi.rs": rust_abi.emit_abi(abi),
        "../codec/crates/ak-core/src/generated_corpus/codec.rs": ccodec,
    }


def main(argv):
    check = "--check" in argv
    ir = P.load(ROOTS)
    bad = 0
    all_targets = dict(targets(ir))
    all_targets.update(corpus_targets())
    for rel, text in all_targets.items():
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
