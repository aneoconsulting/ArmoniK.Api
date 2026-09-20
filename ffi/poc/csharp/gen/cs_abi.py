"""Backend: the C ABI of ABI v1, declared for C#, at the Rust build's offsets.

**The C# slice binds to the SAME core the Rust slice built.** One native core
with N bindings is the proposal under test, so a slice that grows its own core
is not testing it. The core lives once, at `ffi/poc/codec/` (R0), and every
slice depends on it by path. This emits the managed half.

Two things make the C# binding different from every other slice's, and both are
emitted rather than written by hand.

**The layout is declared twice and only agreement makes it work.** Rust owns the
first declaration; a `[StructLayout]` struct is the second. `abi/` prints the
real sizes and offsets and `gen/abi-layout.json` carries them, so the structs
below are emitted with EXPLICIT offsets taken from the Rust build rather than
from C#'s packing rules happening to match. ABI v1 obligation 12.3.

**And the field LISTS are now derived, not transcribed.** They used to be a
hand-written `FIELDS` dict here and a second hand-written list in
`abi/src/main.rs`. Two transcriptions of a generated Rust file produced two
defects in one work unit: a vtable slot invented by analogy with a sibling, and
a presence bit inferred from a field's position. `gen/abi_ir.py` derives both
from the description, following the Rust generator's own rules, so a struct
cannot be in one declaration and not the other.

**A managed exception inside `[UnmanagedCallersOnly]` does not propagate: it
aborts the process.** Every reverse callback is emitted wrapped, and the guard
returns an error code the codec understands.
"""
import json
import os

import abi_ir as A
from cs_facade import Head

HERE = os.path.dirname(os.path.abspath(__file__))


def load_layout():
    with open(os.path.join(HERE, "abi-layout.json")) as f:
        return json.load(f)


def cs_type(rust):
    if rust in A.CS:
        return A.CS[rust]
    if rust.startswith(("ak_efix_", "ak_dfix_")):
        return rust
    raise KeyError("no C# spelling for %r" % rust)


def emit(ir):
    lay = load_layout()
    st, pres, vts = lay["structs"], lay["presence"], lay["vtables"]
    roots = ir.roots
    o = Head("The C ABI of ABI v1, declared for C#, at the offsets the Rust build reports.")
    o += "using System;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""

    # ---- the structs, at explicit offsets ---------------------------
    decls = [("ak_str", [("data", "IntPtr"), ("len", "nuint"), ("tc", "IntPtr")]),
             ("ak_span", [("off", "uint"), ("len", "uint"), ("coder", "uint")])]
    for enc in (True, False):
        pre = "ak_efix_" if enc else "ak_dfix_"
        for name in A.structs_for(ir, roots):
            decls.append((pre + name,
                          [(f, cs_type(t)) for f, t in A.group_fields(ir.msg(name), enc)]))
        for ename, f in A.map_entries(ir, roots):
            blob = "ak_str" if enc else "ak_span"
            decls.append((pre + ename,
                          [("key", blob), ("value", blob), ("presence", "uint")]))

    for name, fields in decls:
        if name not in st:
            raise KeyError("%s is not in abi-layout.json; re-run abi/ and regenerate" % name)
        info = st[name]
        o.doc("Rust `%s`: %d bytes, align %d. Offsets are the Rust build's own, "
              "not C#'s packing." % (name, info["size"], info["align"]))
        o += "[StructLayout(LayoutKind.Explicit, Size = %d)]" % info["size"]
        o += "public struct %s" % name
        o += "{"
        for fname, ftype in fields:
            if fname not in info["fields"]:
                raise KeyError("%s.%s is not in the layout probe's output" % (name, fname))
            o += "    [FieldOffset(%d)] public %s %s;" % (info["fields"][fname], ftype, fname)
        o += "}"
        o += ""

    o.doc("The core's own counters. **This is the convention the cross-language table "
          "uses** (R5): `forward` is every `extern \"C\"` entry point the host called, "
          "`reverse` is every function pointer the core invoked INCLUDING transcoders. "
          "The host's own tally is the same quantity minus transcoders, and this slice "
          "stages its strings so it invokes none -- the gate asserts they are equal.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public struct AkCounters"
    o += "{"
    for f in ("forward", "reverse", "transcode", "prefix_moves", "prefix_bytes", "grows"):
        o += "    public ulong %s;" % f
    o += "}"
    o += ""

    # ---- the presence constants -------------------------------------
    o.doc("The presence bits, as the Rust build reports them. A bit is NOT inferred "
          "from a field's position among the singular message children: that rule is "
          "right today and is still a guess, and the same class of guess invented a "
          "vtable slot once already.")
    o += "public static class AkPresent"
    o += "{"
    for k in sorted(pres):
        o += "    public const uint %s = %d;" % (k, pres[k])
    o += "}"
    o += ""

    # ---- the imports -------------------------------------------------
    o.doc("ABI v1 section 6: what a host calls in the codec are PLAIN EXPORTS, not a "
          "table, so the host declares the symbols it uses and a missing one is a load "
          "failure rather than a null slot found at the wrong moment.")
    o += "public static unsafe partial class Abi"
    o += "{"
    o += '    public const string Lib = "ak_core";'
    o += ""
    o += "    public const int AK_TOKEN_ROOT = -1;"
    o += "    /// What the abort guard returns when the host throws. ABI v1 section 5:"
    o += "    /// sticky, first error wins."
    o += "    public const int AK_ERR_HOST = -9;"
    o += ""
    imports = [
        ("uint", "ak_abi_version", ""),
        ("IntPtr", "ak_enc_ctx_new", ""),
        ("void", "ak_enc_ctx_free", "IntPtr ctx"),
        ("void", "ak_enc_reset", "IntPtr ctx"),
        ("int", "ak_enc_take", "IntPtr ctx, byte** ptr, nuint* len"),
        ("int", "ak_enc_err", "IntPtr ctx"),
        ("IntPtr", "ak_tc_bytes", ""),
        ("IntPtr", "ak_tc_utf8_trusted", ""),
        # ABI v1 section 4's OTHER string form: a transcoder that reads the host's
        # own UTF-16 in place. It is a pointer INTO the core, so it crosses
        # nothing -- which makes the "host transcoder costs a reverse crossing
        # per string" prediction wrong, and makes the staging copy optional.
        ("IntPtr", "ak_tc_utf16", ""),
        ("IntPtr", "ak_tc_latin1", ""),
        ("IntPtr", "ak_dec_ctx_new", ""),
        ("void", "ak_dec_ctx_free", "IntPtr ctx"),
        ("int", "ak_dec_err", "IntPtr ctx"),
        ("void", "ak_dec_err_reset", "IntPtr ctx"),
        ("void", "ak_fail", "IntPtr ctx, int code, byte* msg, uint msgLen"),
        # The packed-scalar runs. One per wire family, because the tag and the
        # site come from the context and bool and enum ride in i32.
        ("int", "ak_run_i32", "IntPtr ctx, int* p, nuint n"),
        ("int", "ak_run_i64", "IntPtr ctx, long* p, nuint n"),
        ("int", "ak_run_f64", "IntPtr ctx, double* p, nuint n"),
        ("int", "ak_run_u8", "IntPtr ctx, byte* p, nuint n"),
        ("int", "ak_blob_run", "IntPtr ctx, ak_str* elems, int n"),
        # R5, in the CORE's own convention. Counting build only; a non-counting
        # core returns zeroes, which is why the harness reports which it read.
        ("void", "ak_enc_counters", "IntPtr ctx, AkCounters* outp"),
        ("void", "ak_enc_counters_reset", "IntPtr ctx"),
        ("void", "ak_dec_counters", "IntPtr ctx, AkCounters* outp"),
        ("void", "ak_dec_counters_reset", "IntPtr ctx"),
        # ABI v1 7.1's PULL family: the codec writes a record stream and the host
        # replays it, so a decode makes no reverse calls at all.
        ("void", "ak_bdr_reset", "IntPtr ctx"),
        ("int", "ak_bdr_ptr", "IntPtr ctx, byte** ptr, nuint* len"),
        ("int", "ak_bdr_reserve", "IntPtr ctx, nuint bytes"),
        ("nuint", "ak_bdr_footprint", "IntPtr ctx"),
        ("ulong", "ak_bdr_count_forward", "IntPtr ctx"),
    ]
    # Every element entry point, derived: a LEAF element batches and gets
    # `ak_elem_*`; a non-leaf cannot (ABI v1 7.2) and gets `ak_elemu_*`, which
    # takes the first token of the run so the element's own loop callbacks can
    # name themselves.
    for ename, f in A.elem_types(ir, roots):
        if A.is_leaf(ir, ename):
            imports.append(("int", "ak_elem_%s" % ename,
                            "IntPtr ctx, ak_efix_%s* elems, int n" % ename))
        else:
            imports.append(("int", "ak_elemu_%s" % ename,
                            "IntPtr ctx, ak_efix_%s* elems, int n, long tok0" % ename))
    for r in roots:
        imports.append(("nint", "ak_encode_%s" % r,
                        "void* obj, IntPtr ctx, ak_evt_%s* vt, ak_efix_%s* fix" % (r, r)))
        imports.append(("int", "ak_decode_%s" % r,
                        "IntPtr ctx, void* obj, byte* buf, nuint len, ak_dvt_%s* vt" % r))
        imports.append(("int", "ak_parse_%s" % r, "IntPtr ctx, byte* buf, nuint len"))
    for ret, name, args in imports:
        o += '    [LibraryImport(Lib)]'
        o += '    [UnmanagedCallConv(CallConvs = new[] { typeof(System.Runtime.CompilerServices.CallConvCdecl) })]'
        o += "    internal static partial %s %s(%s);" % (ret, name, args)
        o += ""
    o += "}"
    o += ""

    # ---- the vtables -------------------------------------------------
    o.doc("The vtables, derived. An ENCODE vtable has one slot per loop slot, plus a "
          "pointer to the element's vtable for every slot whose element has slots of "
          "its own. A DECODE vtable is `apply`, `unknown`, then per slot either an "
          "`add_` run (the element is a LEAF, so the whole run crosses once) or "
          "`new_`/`apply_` per element plus a run per inner field (it is not, and ABI "
          "v1 7.2 refuses to batch it). Slot COUNTS are asserted below.")
    LOOP = "delegate* unmanaged[Cdecl]<IntPtr, void*, long, int>"
    for name in A.vtable_messages(ir, roots):
        slots = A.loop_slots(ir, name) if name in ir.messages else []
        o += "[StructLayout(LayoutKind.Sequential)]"
        o += "public unsafe struct ak_evt_%s" % name
        o += "{"
        if not slots:
            o += "    /// Reserved. An empty struct has no defined size in C, so a vtable"
            o += "    /// for a message that needs no call still carries one slot."
            o += "    public IntPtr _reserved;"
        for path, f in slots:
            o += "    public %s loop_%s;" % (LOOP, A.slot_name(path))
        for path, f in slots:
            et = A.entry_type(f) if f.card == "map" else (f.of if f.kind == "message" else None)
            if et and et in ir.messages and A.loop_slots(ir, et):
                o += "    /// The element type has loop slots of its own, so the codec"
                o += "    /// needs its vtable to reach them (ABI v1 section 6)."
                o += "    public ak_evt_%s* elem_%s;" % (et, A.slot_name(path))
        o += "}"
        o += ""
        o += "[StructLayout(LayoutKind.Sequential)]"
        o += "public unsafe struct ak_dvt_%s" % name
        o += "{"
        o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, ak_dfix_%s*, void> apply;" % name
        o += "    public IntPtr unknown;          // decision 11: null is today's behaviour"
        for path, f in slots:
            sn = A.slot_name(path)
            et = A.entry_type(f) if f.card == "map" else (f.of if f.kind == "message" else None)
            # `unk_<slot>` exists only where the run's elements are MESSAGES: a run
            # of strings or of packed scalars has nowhere to carry an unknown field.
            if et:
                o += "    public IntPtr unk_%s;      // decision 11" % sn
            if not et or A.is_leaf(ir, et):
                o += "    /// Batchable: the element type is a leaf, so a run crosses once per"
                o += "    /// chunk. Append; never size to the count you were handed."
                o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, %s*, int, void> add_%s;" % (
                    cs_type(A.slot_elem(f, False)), sn)
            else:
                o += "    // NOT batchable: %s carries repeated or map fields of its own," % et
                o += "    // so there would be nothing to attach the inner elements to"
                o += "    // (ABI v1 7.2). `new` then `apply` per element, plus one run"
                o += "    // per inner field that occurred."
                o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long> new_%s;" % sn
                o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, ak_dfix_%s*, void> apply_%s;" % (et, sn)
                for ipath, iff in A.loop_slots(ir, et):
                    o += "    public delegate* unmanaged[Cdecl]<IntPtr, void*, long, %s*, int, void> add_%s_%s;" % (
                        cs_type(A.slot_elem(iff, False)), sn, A.slot_name(ipath))
        o += "}"
        o += ""

    # ---- the layout assert -------------------------------------------
    o.doc("ABI v1 obligation 12.3, as far as it can be taken here.")
    o += "public static class AbiLayout"
    o += "{"
    o += "    /// Checks the MANAGED declaration against the Rust build, and the loaded"
    o += "    /// core's ABI version."
    o += "    ///"
    o += "    /// **What this does not check, and it is a gap in the ABI rather than in"
    o += "    /// this slice**: the core exports `ak_abi_version` but no LAYOUT. So this"
    o += "    /// verifies C# against the Rust SOURCE at generator time, not against the"
    o += "    /// `.so` actually loaded. A core rebuilt with a changed group layout and an"
    o += "    /// unchanged version number would pass here and fail as a wrong payload."
    o += "    public static string Check()"
    o += "    {"
    o += "        var bad = new System.Collections.Generic.List<string>();"
    nstruct = 0
    for name, fields in decls:
        info = st[name]
        nstruct += 1
        o += "        if (Unsafe.SizeOf<%s>() != %d) bad.Add($\"%s size {Unsafe.SizeOf<%s>()} != %d\");" % (
            name, info["size"], name, name, info["size"])
        for f, _ in fields:
            o += "        if ((int)Marshal.OffsetOf<%s>(\"%s\") != %d) bad.Add($\"%s.%s offset {(int)Marshal.OffsetOf<%s>(\"%s\")} != %d\");" % (
                name, f, info["fields"][f], name, f, name, f, info["fields"][f])
    nvt = 0
    for name in A.vtable_messages(ir, roots):
        for pre in ("ak_evt_", "ak_dvt_"):
            t = pre + name
            if t not in vts:
                raise KeyError("%s is not in the layout probe's vtable output" % t)
            nvt += 1
            o += "        if (Unsafe.SizeOf<%s>() != %d) bad.Add($\"%s has {Unsafe.SizeOf<%s>() / IntPtr.Size} slots, expected %d\");" % (
                t, vts[t]["size"], t, t, vts[t]["slots"])
    o += "        uint v = Abi.ak_abi_version();"
    o += "        return bad.Count == 0"
    o += "            ? $\"ok: %d structs and %d vtables match the Rust build; core ak_abi_version()={v}\"" % (nstruct, nvt)
    o += "            : string.Join(\"; \", bad);"
    o += "    }"
    o += "}"
    return str(o)
