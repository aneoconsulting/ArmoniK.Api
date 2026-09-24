"""Java backend: every generated Java and JNI file of one description, from ONE plan.

FIX-PLAN WP5 step 3. The java slice's `gen/generate.py` calls `emit` once per description
(ffi/schema/shapes.json, and the corpus's reader schema) and writes what it returns under
`poc/java/`; it holds no wire rule, no IR and no layout derivation of its own (CLAUDE.md,
one generator). Nothing here writes a file the core owns.

    emit(p, pkg, entry, java_root, native_dir, shared_dir, floor_pkg=None, borrow_pkg=None)
        -> {path relative to poc/java: text}

Java has no preprocessor, so its floor (Java 8) and target (JDK 17) are two TREES from this
one backend with a level parameter (README 5.1, CLAUDE.md): the only level-dependent text
is the binding's string staging (`java_binding`, STAGE_17 vs STAGE_8). Arm R's codec, the
facade, the layout and the shim are the same text at both levels.

Modules (each imports `plan` and other Java-backend modules, never the IR or a schema):
  java_names    spellings, facade typing
  java_facade   facade classes and enums
  java_rcodec   arm R, drop and retain (plan.encode / oneof_checks / decode)
  java_layout   group offsets from plan.group_fields, asserted at run time and compile time
  java_abi      the C header (groups, vtables, plan.rpc, plan.lifecycle) and slot tables
  java_jni      the JNI shim and NativeEntry (ak_init rendered, R-G7)
  java_binding  the Java half of the binding, per level; java_pull, its pull family
"""
import java_binding
import java_abi
import java_facade
import java_jni
import java_layout
import java_names as N
import java_rcodec
from plan import check_direct

BACKEND_MODULES = ["java_backend.py", "java_names.py", "java_facade.py", "java_rcodec.py",
                   "java_layout.py", "java_abi.py", "java_jni.py", "java_binding.py",
                   "java_pull.py"]

LEVELS = [("java17", 17), ("java8", 8)]


def _dir(root, level_dir, pkg):
    return "%s/%s/%s" % (root, level_dir, pkg.replace(".", "/"))


def emit(p, pkg, entry, java_root, native_dir, shared_dir, floor_pkg=None, borrow_pkg=None,
         pa=None):
    """`pa`: the plan of the roots that cross the C ABI, when that is not `p` (the corpus's
    `Nest` is refused by plan.check_expressible and runs on arm R only): the facade and
    arm R render `p`, the header, shim, layout and binding render `pa` -- which must be the
    plan the core it links against was generated from, or the layout guard fails."""
    out = {}
    pa = pa or p
    for root in pa.roots:
        check_direct(pa, root)      # ABI v1 section 8's refusal, before a line is emitted
    layout = "%s.Layout" % pkg
    out["%s/ak_abi.h" % native_dir] = java_abi.emit_header(pa)
    out["%s/shim.c" % native_dir] = java_jni.emit_c(pa, entry)
    out["%s/%s.java" % (shared_dir, entry.replace(".", "/"))] = java_jni.emit_java(pa, entry)
    for level_dir, level in LEVELS:
        d = _dir(java_root, level_dir, pkg)
        for fn, text in java_facade.emit_enums(p, ns=pkg).items():
            out["%s/%s" % (d, fn)] = text
        for fn, text in java_facade.emit_types(p, ns=pkg).items():
            out["%s/%s" % (d, fn)] = text
        out["%s/Codec.java" % d] = java_rcodec.emit(p, ns=pkg, unknown="drop")
        out["%s/CodecRetain.java" % d] = java_rcodec.emit(p, ns=pkg, unknown="retain")
        out["%s/Layout.java" % d] = java_layout.emit_java(pa, ns=pkg)
        out["%s/Binding.java" % d] = java_binding.emit(pa, level=level, ns=pkg,
                                                       layout=layout, entry=entry)
        if floor_pkg:
            # README 5.2 arm b: the FLOOR binding in a package of its own over the same
            # facade, so it shares a process (and a paired ratio) with the target. In the
            # java8 tree it is the same code as `pkg`, a positive control.
            fd = _dir(java_root, level_dir, floor_pkg)
            out["%s/Binding.java" % fd] = java_binding.emit(pa, level=8, ns=floor_pkg,
                                                            facade_ns=pkg, layout=layout,
                                                            entry=entry)
        if borrow_pkg:
            # Decision 13's borrowed facade: the same plan, `ak.Utf8View` for `String`.
            bd = _dir(java_root, level_dir, borrow_pkg)
            with N.string_type_as("ak.Utf8View"):
                for fn, text in java_facade.emit_types(p, ns=borrow_pkg).items():
                    out["%s/%s" % (bd, fn)] = text
                out["%s/Codec.java" % bd] = java_rcodec.emit(p, ns=borrow_pkg, unknown="drop")
                out["%s/Binding.java" % bd] = java_binding.emit(pa, level=level, ns=borrow_pkg,
                                                                layout=layout, entry=entry)
    return out
