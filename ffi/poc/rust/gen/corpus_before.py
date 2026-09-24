"""The BEFORE half of FIX-PLAN WP5 item 6.1: the corpus against the PRE-WP5 generator.

Reproduction (logs/rust/wp5-corpus-before.log):
    git worktree add <WT> c10e934
    cp poc/codec/gen/ir.py <WT>/ffi/poc/codec/gen/ir.py      # the front end only (load_corpus)
    cp -r poc/rust/corpus poc/rust/gen/rust_project.py into <WT>'s ffi/poc/rust/{corpus,gen}
    python3 poc/rust/gen/corpus_before.py <WT>/ffi/poc
    in <WT>/ffi/poc/rust/corpus: drop the `corpus` feature from the two core deps, remove
    `core_native_retain` from the facade lib, stub the two plan constants main.rs prints;
    cargo build --release; target/release/corpus --manifest <this repo>/ffi/corpus/generated/manifest.json

Generates, with the OLD generator (commit c10e934, before plan.py existed), the core and
core-native for the corpus reader schema. WireZoo (fixed32, which the old generator has no
case for) and Nest (recursive) are excluded; the old facade cannot express either.
"""
import os, sys, json
W = os.path.abspath(sys.argv[1])
sys.path.insert(0, W + "/rust/gen")
sys.path.insert(0, W + "/codec/gen")
import ir as IR
import rust_abi, rust_core, rust_facade, rust_project
schema = IR.corpus_schema()
names = [n for n in schema["messages"] if n not in ("WireZoo", "Nest")]
ir = IR.Ir(schema, names)
codec = rust_abi.emit_codec(ir)
abi = rust_abi.emit_abi(ir)
out = {
    W + "/codec/crates/ak-abi/src/generated/abi.rs": abi,
    W + "/codec/crates/ak-core/src/generated/codec.rs": codec,
    W + "/rust/corpus/crates/facade/src/generated/types.rs": rust_facade.emit_types(ir),
    W + "/rust/corpus/crates/facade/src/generated/core_native.rs": rust_core.emit_core_native(ir),
    W + "/rust/corpus/crates/facade/src/generated/project.rs": rust_project.emit(ir),
}
# The HOST BINDING is glue, not the codec under comparison, and the old binding generator
# could not render a root whose loop slot lives on an inlined child. The layout it renders
# is the plan's, which is the same layout the old core emits (abi.rs was byte-identical
# across the change), so the NEW binding generator is used over the OLD core.
import importlib.util
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec); sys.modules[name] = m; spec.loader.exec_module(m); return m
NEW = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "codec", "gen"))
P = load("plan", NEW + "/plan.py")
RB = load("rust_binding", NEW + "/rust_binding.py")
out[W + "/rust/corpus/crates/harness/src/generated/binding.rs"] = RB.emit_binding(P.lower(ir))
# layout.rs: the old cpp_layout
import cpp_layout
out[W + "/codec/crates/ak-core/src/generated/layout.rs"] = cpp_layout.emit(ir)
# dispatch: native-retain arm maps to the drop module (the old generator had no retain
# rendering), reported as such.
def snake(c):
    o = []
    for i, ch in enumerate(c):
        if ch.isupper() and i: o.append("_")
        o.append(ch.lower())
    return "".join(o)
d = ["#![allow(clippy::all)]", "use crate::{ffi, native, Arm, Cx, Outcome};",
     "use crate::generated::binding;", "use facade::generated::{core_native, project};",
     "pub const NOT_IN_ABI: &[(&str, &str)] = &[(\"WireZoo\", \"before-run: no fixed32 case in the old generator\"), (\"Nest\", \"before-run: recursive\")];",
     "pub fn run(root: &str, arm: Arm, b: &[u8], cx: &Cx) -> Outcome {", "    match root {"]
for n in ir.order:
    s = snake(n)
    d.append('        "%s" => match arm {' % n)
    d.append("            Arm::FfiDrop => ffi(b, cx, binding::decode_with_%s, binding::encode_into_%s, project::project_%s)," % (s, s, s))
    d.append("            Arm::FfiRetain => ffi(b, cx, binding::decode_with_%s_unk, binding::encode_into_%s_unk, project::project_%s)," % (s, s, s))
    d.append("            Arm::NativeDrop | Arm::NativeRetain => native(b, core_native::decode_%s, core_native::encode_%s, project::project_%s)," % (s, s, s))
    d.append("        },")
d += ['        "WireZoo" | "Nest" => Outcome::NotInAbi,', "        _ => Outcome::UnknownRoot,", "    }", "}", ""]
out[W + "/rust/corpus/crates/harness/src/generated/dispatch.rs"] = "\n".join(d)
for p, t in out.items():
    open(p, "w").write(t)
print("ok", len(names), "roots")
