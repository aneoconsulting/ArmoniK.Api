#!/usr/bin/env python3
"""The cpp slice's generator: build and harness glue over the ONE generator.

FIX-PLAN WP5 step 2. The pipeline is the shared one -- description -> IR -> PLAN
(`poc/codec/gen/plan.py`, every wire rule and layout stated once) -> backends -- and every
C++ file that carries a wire rule, a layout or a binding is rendered by the SHARED C++
backend in `poc/codec/gen/`, which imports `plan` only:

  c_abi.py         include/ak_abi.h, include/generated/ak_layout{,_names}.h
  cpp_facade.py    src/generated/types.{h,cpp}, odr.h (and the borrowed facade)
  cpp_native.py    src/generated/core_native.{h,cpp}   arm `core-native-cpp` (R3)
  cpp_binding.py   src/generated/binding.{h,cpp}       arm `core-ffi` (renders ak_init)

What is THIS slice's own is glue that decides nothing about the wire:

  cpp_build.py     src/generated/build.{h,cpp}     payload builder (value rules of payloads.py)
  cpp_pbbuild.py   src/generated/pb_build.{h,cpp}  the same over protoc's types (incumbent)
  cpp_cases.py     src/generated/cases.h           the arm table (manifest hashes)
  cpp_project.py   src/generated/project.{h,cpp}   CONTRACT.md section 3's projection

and the same backends once more for the CONFORMANCE CORPUS's reader schema (WP5 item 6.1),
into `corpus/`: the facade, core-native in both unknown-field modes, the binding over the
core built `--features corpus,init-guard`, its header, and the projection.

It reads plans only (`plan.load`, `plan.load_corpus`); it imports neither `ir` nor the
schema. The shared core files (`codec.rs`, `layout.rs`) are also written here through the
shared Rust backends, so this slice's `--check` gates the core it measures (R0).

  gen/generate.py            write the generated files
  gen/generate.py --check    fail if what is committed is not what this would write, or if
                             a shared C++ backend module imports anything but plans
"""
import ast
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CODECGEN = os.path.abspath(os.path.join(HERE, "..", "..", "codec", "gen"))
# HERE first: both directories contain a `generate.py` (see refusal_test.py).
sys.path.insert(0, CODECGEN)
sys.path.insert(0, HERE)

import plan as P             # noqa: E402  (shared rule layer)
import rust_abi              # noqa: E402  (shared Rust backend: the core it measures)
import cpp_layout            # noqa: E402  (shared)
import c_abi                 # noqa: E402  (the ONE C header backend)
import cpp_facade            # noqa: E402  (shared C++ backend)
import cpp_native            # noqa: E402  (shared C++ backend)
import cpp_binding           # noqa: E402  (shared C++ backend)
import cpp_names             # noqa: E402  (shared C++ backend)
import cpp_build             # noqa: E402  (glue)
import cpp_pbbuild           # noqa: E402  (glue)
import cpp_cases             # noqa: E402  (glue)
import cpp_project           # noqa: E402  (glue)
import cpp_touch             # noqa: E402  (glue: the campaign's read-every-field pass)

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

# The shared C++ backend: the modules that must import plans and nothing else (WP5 item 7).
# `poc/codec/gen/generate.py`'s BACKENDS list is the aggregating session's; until these are
# added there, this slice runs the SAME guard function over them.
CPP_BACKENDS = ["c_abi.py", "cpp_facade.py", "cpp_native.py", "cpp_binding.py",
                "cpp_names.py"]
FORBIDDEN = {"ir", "shapes", "spec", "values", "payloads", "encode", "walk", "json"}
# This slice's glue must not reach for the IR either.
GLUE = ["generate.py", "cpp_touch.py", "campaign_summary.py", "cpp_build.py", "cpp_pbbuild.py", "cpp_cases.py", "cpp_project.py",
        "corpus_all.py", "refusal_test.py"]
GLUE_FORBIDDEN = {"ir", "shapes", "spec", "rust_core", "cppnames", "cpp_core"}


def _imports(src):
    out = set()
    for node in ast.walk(ast.parse(src)):
        if isinstance(node, ast.Import):
            out.update(a.name.split(".")[0] for a in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            out.add(node.module.split(".")[0])
    return out


def guard():
    bad = []
    for b in CPP_BACKENDS:
        hit = sorted(_imports(open(os.path.join(CODECGEN, b)).read()) & FORBIDDEN)
        if hit:
            bad.append("GUARD shared backend %s imports %s" % (b, ", ".join(hit)))
    for g in GLUE:
        path = os.path.join(HERE, g)
        if not os.path.exists(path):
            continue
        hit = sorted(_imports(open(path).read()) & GLUE_FORBIDDEN)
        if hit:
            bad.append("GUARD slice glue %s imports %s" % (g, ", ".join(hit)))
    # The guard seen failing: a planted backend reaching for the IR must be caught.
    caught = sorted(_imports("import ir as IR\nfrom shapes import load\n") & FORBIDDEN) == ["ir", "shapes"]
    if not caught:
        bad.append("GUARD the planted violation was NOT caught: the guard is blind")
    return bad


def corpus_plans():
    """(full, abi, refused): the corpus reader schema lowered for every message (core-native
    renders all of them, `Nest` included) and for the roots the C ABI can carry -- the same
    computation as `poc/codec/gen/generate.py`'s corpus core, so the header here matches
    the core built with `--features corpus`."""
    names = P.corpus_messages()
    full = P.load_corpus(names)
    ok, refused = P.expressible_roots(names, full)
    abi = P.load_corpus(ok)
    for root in abi.roots:
        P.check_direct(abi, root)
    return full, abi, refused


def corpus_targets():
    full, abi, refused = corpus_plans()
    hdr, lay, names = c_abi.emit(abi)
    nd_h, nd_c = cpp_native.emit(full, "drop", ns="corpus", stem="core_native")
    nr_h, nr_c = cpp_native.emit(full, "retain", ns="corpus", stem="core_native_retain")
    return {
        "corpus/include/ak_abi.h": hdr,
        "corpus/include/generated/ak_layout.h": lay,
        "corpus/include/generated/ak_layout_names.h": names,
        "corpus/src/generated/types.h": cpp_facade.emit_types(full, ns="corpus", guard="AK_CORPUS_TYPES_H"),
        "corpus/src/generated/types.cpp": cpp_facade.emit_types_impl(full, ns="corpus"),
        "corpus/src/generated/core_native.h": nd_h,
        "corpus/src/generated/core_native.cpp": nd_c,
        "corpus/src/generated/core_native_retain.h": nr_h,
        "corpus/src/generated/core_native_retain.cpp": nr_c,
        "corpus/src/generated/binding.h": cpp_binding.emit_header(abi, ns="corpus", guard="AK_CORPUS_BINDING_H", retain=True),
        "corpus/src/generated/binding.cpp": cpp_binding.emit(abi, ns="corpus", retain=True),
        "corpus/src/generated/project.h": cpp_project.emit_header(full, ns="corpus", guard="AK_CORPUS_PROJECT_H"),
        "corpus/src/generated/project.cpp": cpp_project.emit(full, ns="corpus"),
        "corpus/src/generated/dispatch.cpp": emit_dispatch(full, abi, refused),
    }


# Whether the shared binding renders the retain family (`decode_with_*_unk` /
# `encode_into_*_unk`); the dispatch reports the ffi-retain arm NOT BUILT otherwise.
FFI_RETAIN = hasattr(cpp_binding, "RETAIN") and cpp_binding.RETAIN


def emit_dispatch(full, abi, refused):
    """Corpus glue: the per-root call table (four arms). No wire rule: a table of calls. A
    root the C ABI cannot carry has its two ffi arms reported NOT IN THE ABI, by name."""
    sn = cpp_names.snake
    o = ["// @generated by ffi/poc/cpp/gen/generate.py (corpus glue, plan %s). Do not edit."
         % full.source,
         '#include "corpus_harness.h"', "",
         "namespace corpus {", "",
         "// Roots the C ABI refuses at generator time (plan.check_expressible).",
         "const char *const kNotInAbi[][2] = {"]
    for r, why in sorted(refused.items()):
        o.append('  {"%s", "%s"},' % (r, why.replace('"', "'")))
    o.append("  {NULL, NULL}")
    o.append("};")
    o.append("const bool kFfiRetainBuilt = %s;" % ("true" if FFI_RETAIN else "false"))
    o.append("")
    o.append("Outcome run_arm(const std::string &root, Arm arm, const uint8_t *b, size_t n, Cx &cx) {")
    for name in full.order:
        if full.msg(name).synthetic:
            continue
        s = sn(name)
        o.append('  if (root == "%s") {' % name)
        o.append("    switch (arm) {")
        o.append("      case kNativeDrop: return native_arm<%s>(b, n, native::kSites, native::decode_%s,"
                 " native::encode_into_%s, project::project_%s);" % (name, s, s, s))
        o.append("      case kNativeRetain: return native_arm<%s>(b, n, native_retain::kSites,"
                 " native_retain::decode_%s, native_retain::encode_into_%s, project::project_%s);"
                 % (name, s, s, s))
        if name in abi.roots:
            o.append("      case kFfiDrop: return ffi_arm<%s>(b, n, cx, ffi::decode_with_%s,"
                     " ffi::encode_into_%s, project::project_%s);" % (name, s, s, s))
            if FFI_RETAIN:
                o.append("      case kFfiRetain: return ffi_arm<%s>(b, n, cx, ffi::decode_with_%s_unk,"
                         " ffi::encode_into_%s_unk, project::project_%s);" % (name, s, s, s))
            else:
                o.append("      case kFfiRetain: return Outcome::not_built();")
        else:
            o.append("      case kFfiDrop: case kFfiRetain: return Outcome::not_in_abi();")
        o.append("    }")
        o.append("  }")
    o.append("  return Outcome::unknown_root();")
    o.append("}")
    o.append("")
    o.append("}  // namespace corpus")
    o.append("")
    return "\n".join(o)


def targets():
    p = P.load(ROOTS)
    # ABI v1 section 8's refusal, at generator time, before a line is emitted.
    for root in p.roots:
        P.check_direct(p, root)
    # The core's codec is emitted FIRST because it allocates the length-prefix sites.
    codec = rust_abi.emit_codec(p)
    header, layout_h, layout_names_h = c_abi.emit(p)
    nat_h, nat_c = cpp_native.emit(p, "drop")
    natr_h, natr_c = cpp_native.emit(p, "retain", stem="core_native_retain")
    out = {
        "include/ak_abi.h": header,
        "include/generated/ak_layout.h": layout_h,
        "include/generated/ak_layout_names.h": layout_names_h,
        # D38: the plan's decode-rule constants for the hand-written runtime (ak/rt.h).
        "include/generated/ak_rules.h": cpp_native.emit_rules(p),
        "src/generated/types.h": cpp_facade.emit_types(p),
        "src/generated/types.cpp": cpp_facade.emit_types_impl(p),
        "src/generated/odr.h": cpp_facade.emit_odr_asserts(p),
        "src/generated/build.h": cpp_build.emit_header(p),
        "src/generated/build.cpp": cpp_build.emit(p),
        "src/generated/pb_build.h": cpp_pbbuild.emit_header(p),
        "src/generated/pb_build.cpp": cpp_pbbuild.emit(p),
        "src/generated/core_native.h": nat_h,
        "src/generated/core_native.cpp": nat_c,
        # design/CAMPAIGN.md requirement 10: host-gen in RETAIN mode as well as drop.
        "src/generated/core_native_retain.h": natr_h,
        "src/generated/core_native_retain.cpp": natr_c,
        # Requirement 9: decode followed by reading every field, facade and protobuf.
        "src/generated/touch.h": cpp_touch.emit_header(p),
        "src/generated/touch.cpp": cpp_touch.emit(p),
        "src/generated/binding.h": cpp_binding.emit_header(p),
        "src/generated/binding.cpp": cpp_binding.emit(p),
        # The BORROWED facade and its binding: the same backends with `ak::StringView` in
        # place of `std::string`. A measurement arm for `bench`, never the shipping facade.
        **_borrow(p),
        "src/generated/cases.h": cpp_cases.emit(p),
        # ffi/corpus/CONTRACT.md obligation C2: what a reader must SEE.
        "src/generated/project.h": cpp_project.emit_header(p),
        "src/generated/project.cpp": cpp_project.emit(p),
        # The shared core (R0), written here as well as from `poc/codec/gen`.
        "../codec/crates/ak-core/src/generated/codec.rs": codec,
        "../codec/crates/ak-core/src/generated/layout.rs": cpp_layout.emit(p),
    }
    out.update(corpus_targets())
    return out


def _borrow(p):
    cpp_names.set_string_type("ak::StringView")
    try:
        out = {
            "src/generated/types_borrow.h":
                cpp_facade.emit_types(p, ns="shapes_borrow", guard="AK_TYPES_BORROW_H"),
            "src/generated/types_borrow.cpp":
                cpp_facade.emit_types_impl(p, ns="shapes_borrow",
                                           header="generated/types_borrow.h"),
            "src/generated/binding_borrow.h":
                cpp_binding.emit_header(p, ns="shapes_borrow", guard="AK_BINDING_BORROW_H",
                                        types_h="generated/types_borrow.h"),
            "src/generated/binding_borrow.cpp":
                cpp_binding.emit(p, ns="shapes_borrow", hdr="generated/binding_borrow.h"),
        }
    finally:
        cpp_names.set_string_type("std::string")
        cpp_binding.NS[0] = "shapes"
    return out


def main(argv):
    check = "--check" in argv
    bad = 0
    if check:
        g = guard()
        for line in g:
            print(line)
        bad += len(g)
        if not g:
            print("guard %d shared C++ backend modules import plans only; slice glue imports "
                  "no IR; a planted IR import is caught" % len(CPP_BACKENDS))
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
