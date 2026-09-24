#!/usr/bin/env python3
"""The shared core's generator: ONE command regenerates everything the core owns.

FIX-PLAN WP5 step 1. The pipeline is description -> IR (`ir.py`) -> PLAN (`plan.py`, the
rule layer, every wire rule and layout stated once) -> backends that render plans:

  crates/ak-abi/src/generated/abi.rs        the per-message ABI           (rust_abi.emit_abi)
  crates/ak-abi/src/lib.rs, marked region   the RPC half's ABI (R-G5)     (rust_abi.emit_rpc_abi)
  crates/ak-core/src/generated/codec.rs     the codec                     (rust_abi.emit_codec)
  crates/ak-core/src/generated/layout.rs    section 10's layout export    (cpp_layout.emit)
  crates/ak-core/src/generated/rpc_check.rs the core's RPC signatures, asserted (R-G5)

and the same core generated for the CONFORMANCE CORPUS's reader schema, so the corpus can
be run through the C ABI (WP5 item 6.1), behind the `corpus` feature of ak-abi / ak-core:

  crates/ak-abi/src/generated_corpus/abi.rs
  crates/ak-core/src/generated_corpus/codec.rs
  crates/ak-core/src/generated_corpus/layout.rs

  gen/generate.py            write them
  gen/generate.py --check    fail if what is committed is not what this would write, OR if
                             a backend module imports the IR or the schema instead of plans
                             (WP5 item 7), OR if that guard cannot see a planted violation

The rust, cpp and java slices' generators also write abi.rs, codec.rs and layout.rs (each
slice's `--check` gates the shared core it measures), through the same backends, so the
writers agree by construction; `gen/one_core.sh` checks they still do.
"""
import ast
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import plan as P             # noqa: E402
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

# ---------------------------------------------------------------- the one-generator guard
#
# WP5 item 7. A BACKEND renders plans; it does not read the description. These modules are
# backends, and none of them may import the IR or anything that loads a description.
BACKENDS = ["rust_abi.py", "rust_native.py", "rust_binding.py", "cpp_layout.py",
            "rust_core.py", "rustnames.py"]
FORBIDDEN = {"ir", "shapes", "spec", "values", "payloads", "encode", "walk", "json"}


def imports_of(source):
    out = set()
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.Import):
            out.update(a.name.split(".")[0] for a in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            out.add(node.module.split(".")[0])
    return out


def guard_violations(sources):
    """{module: sorted forbidden imports} for every backend source that breaks the rule."""
    bad = {}
    for name, src in sources.items():
        hit = sorted(imports_of(src) & FORBIDDEN)
        if hit:
            bad[name] = hit
    return bad


def guard():
    sources = {b: open(os.path.join(HERE, b)).read() for b in BACKENDS}
    bad = guard_violations(sources)
    # The guard seen failing: a planted backend that reaches for the IR must be caught,
    # both spellings. A check that has never failed is a check nobody has seen work.
    plant = {"planted.py": "import ir as IR\nfrom shapes import load\n"}
    caught = guard_violations(plant).get("planted.py") == ["ir", "shapes"]
    return bad, caught


def corpus_roots():
    """Every corpus message the ABI can carry; the rest are refused BY NAME (a recursive
    message has no finite group) and run on core-native only."""
    names = P.corpus_messages()
    full = P.load_corpus(names)
    ok, refused = P.expressible_roots(names, full)
    return ok, refused


def targets():
    p = P.load(ROOTS)
    # ABI v1 section 8's refusal, at generator time, before a line is emitted.
    for root in p.roots:
        P.check_direct(p, root)
    # The codec is emitted FIRST because it allocates the length-prefix sites the ABI names.
    codec = rust_abi.emit_codec(p)
    out = {
        "crates/ak-abi/src/generated/abi.rs": rust_abi.emit_abi(p),
        "crates/ak-core/src/generated/codec.rs": codec,
        "crates/ak-core/src/generated/layout.rs": cpp_layout.emit(p),
        "crates/ak-core/src/generated/rpc_check.rs": rust_abi.emit_rpc_check(p),
    }
    roots, _refused = corpus_roots()
    cp = P.load_corpus(roots)
    for root in cp.roots:
        P.check_direct(cp, root)
    ccodec = rust_abi.emit_codec(cp)
    out.update({
        "crates/ak-abi/src/generated_corpus/abi.rs": rust_abi.emit_abi(cp),
        "crates/ak-core/src/generated_corpus/codec.rs": ccodec,
        "crates/ak-core/src/generated_corpus/layout.rs": cpp_layout.emit(cp),
    })
    # The RPC half's region of ak-abi's hand-written lib.rs (see rust_abi.emit_rpc_abi).
    lib = os.path.join(ROOT, "crates/ak-abi/src/lib.rs")
    out["crates/ak-abi/src/lib.rs"] = rust_abi.splice_region(open(lib).read(),
                                                             rust_abi.emit_rpc_abi(p))
    return out


def main(argv):
    check = "--check" in argv
    bad = 0
    if check:
        violations, caught = guard()
        for mod, hit in sorted(violations.items()):
            print("GUARD %s imports %s: a backend takes plans, not the IR or the schema"
                  % (mod, ", ".join(hit)))
            bad += 1
        if not caught:
            print("GUARD the planted violation was NOT caught: the guard is blind")
            bad += 1
        if not violations and caught:
            print("guard %d backend modules import plans only; a planted IR import is caught"
                  % len(BACKENDS))
    for rel, text in targets().items():
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
