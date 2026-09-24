"""C# backend: the P/Invoke BINDING of a message set, and of the RPC half, rendered from PLANS.

FIX-PLAN WP5 step 4. One generated file per message set (`emit_abi`) and one for the RPC half
(`emit_rpc`). Every group member, its type and its order, every presence bit, every loop
slot, every vtable slot and every element entry point comes from `plan.py`'s ABI layout
functions -- the same ones `rust_abi` renders the Rust declaration from -- so the C#
declaration is not a re-derivation (R-E6). Language levels are conditional compilation in
the one file: each import is `[LibraryImport]` under `#if NET7_0_OR_GREATER` and
`[DllImport]` in `#else`, so the net6.0 and net48 floors build the same text (FIX-PLAN WP3
item 17, WP5 item 3).

Layout. Every struct is `LayoutKind.Sequential`: the C# compiler computes its own offsets
from the member list, and nothing here copies a Rust offset. Two checks then compare that
layout with the core's, and neither is derived from the list it checks:

  * `AbiLayout.Facts()` compares the C# compiler's `sizeof`/offset of every group member
    with what the LOADED core reports through `ak_layout_facts` (ABI v1 section 10), in the
    order `plan.py` enumerates them;
  * `AbiLayout.Table()` is the C# declaration's own field list with the C# compiler's
    offsets and sizes, which `poc/csharp` compares with the layout probe -- a Rust program
    whose struct and field list is parsed out of the RUST declaration text
    (`cs_layout_probe.py`), not out of any plan -- by NAME, both ways.

What the plan does not state and this module therefore declares by hand (reported to the
aggregating session, FIX-PLAN WP5 "if the plan lacks something"): the vocabulary structs'
layouts (`ak_str`, `ak_span`, `ak_blob`, `ak_uspan`, `ak_err`, `AkCounters`, `ak_bdr_rec`),
the fixed codec entry points (contexts, transcoders, runs, counters, the pull family,
`ak_layout_facts`) and the numeric values of the named constants (read from `ak-abi`'s
source at generation time, as `AK_ABI_VERSION` always was). The probe checks the structs.
"""
import os
import re

from plan import (as_plan, abi_order_topo, direct_fields, element_types, elem_type,
                  group_fields, loop_slots, presence_bits, slot_elem, slot_name,
                  ugroup_fields, vtable_messages)
import cs_names as N

HERE = os.path.dirname(os.path.abspath(__file__))
ABI_LIB = os.path.join(HERE, "..", "crates", "ak-abi", "src", "lib.rs")

LOOP_FN = "delegate* unmanaged[Cdecl]<IntPtr, void*, long, int>"
UNK_FN = "delegate* unmanaged[Cdecl]<IntPtr, void*, ak_uspan*, int, void>"


def abi_constants(names):
    """{name: value} for `pub const NAME: T = V;` in ak-abi's hand-written lib.rs. The plan
    names these (lifecycle flags, success codes); their values live in the Rust header."""
    out = {}
    with open(ABI_LIB) as f:
        src = f.read()
    for n in names:
        mt = re.search(r"pub const %s: (u32|i32) = ([^;]+);" % re.escape(n), src)
        if not mt:
            raise KeyError("%s not found in %s" % (n, ABI_LIB))
        expr = mt.group(2).strip()
        m2 = re.fullmatch(r"1 << (\d+)", expr)
        out[n] = (1 << int(m2.group(1))) if m2 else int(expr)
    return out


# The C# spelling of the vocabulary structs plan.py names but does not lay out. The probe
# (the Rust declaration) is the check.
VOCAB_STRUCTS = [
    ("ak_str", "ABI v1 section 4: an encode blob. `tc == null` is absent; `len` counts source code units.",
     [("data", "IntPtr"), ("len", "nuint"), ("tc", "IntPtr")]),
    ("ak_span", "ABI v1 section 4: a decode blob, an offset into the buffer the host handed in.",
     [("off", "uint"), ("len", "uint"), ("coder", "uint")]),
    ("ak_blob", "The unknown-field bag's slot: raw tag-and-value runs, two words.",
     [("data", "IntPtr"), ("len", "nuint")]),
    ("ak_uspan", "One captured unknown run, by element token.",
     [("token", "long"), ("off", "uint"), ("len", "uint")]),
    ("ak_err", "ak_init's out-parameter.",
     [("code", "int"), ("detail", "uint")]),
    ("AkCounters", "R5, in the CORE's own convention (counting build; zeroes otherwise).",
     [("forward", "ulong"), ("reverse", "ulong"), ("transcode", "ulong"),
      ("prefix_moves", "ulong"), ("prefix_bytes", "ulong"), ("grows", "ulong")]),
    ("ak_bdr_rec", "ABI v1 7.1: one pull record header, 24 bytes; the payload follows padded to 8.",
     [("op", "uint"), ("slot", "uint"), ("token", "long"), ("n", "uint"), ("bytes", "uint")]),
]

# The codec's fixed entry points (ak-abi's lib.rs and abi.rs), which plan.py does not list.
FIXED_IMPORTS = [
    ("uint", "ak_abi_version", ""),
    ("IntPtr", "ak_enc_ctx_new", ""),
    ("void", "ak_enc_ctx_free", "IntPtr ctx"),
    ("void", "ak_enc_reset", "IntPtr ctx"),
    ("int", "ak_enc_take", "IntPtr ctx, byte** ptr, nuint* len"),
    ("int", "ak_enc_err", "IntPtr ctx"),
    ("IntPtr", "ak_dec_ctx_new", ""),
    ("void", "ak_dec_ctx_free", "IntPtr ctx"),
    ("int", "ak_dec_err", "IntPtr ctx"),
    ("void", "ak_dec_err_reset", "IntPtr ctx"),
    ("void", "ak_fail", "IntPtr ctx, int code, byte* msg, uint msgLen"),
    ("IntPtr", "ak_tc_bytes", ""),
    ("IntPtr", "ak_tc_utf8_trusted", ""),
    ("IntPtr", "ak_tc_utf16", ""),
    ("IntPtr", "ak_tc_latin1", ""),
    ("int", "ak_run_i32", "IntPtr ctx, int* p, nuint n"),
    ("int", "ak_run_i64", "IntPtr ctx, long* p, nuint n"),
    ("int", "ak_run_f64", "IntPtr ctx, double* p, nuint n"),
    ("int", "ak_run_u8", "IntPtr ctx, byte* p, nuint n"),
    ("int", "ak_blob_run", "IntPtr ctx, ak_str* elems, int n"),
    ("void", "ak_enc_counters", "IntPtr ctx, AkCounters* outp"),
    ("void", "ak_enc_counters_reset", "IntPtr ctx"),
    ("void", "ak_dec_counters", "IntPtr ctx, AkCounters* outp"),
    ("void", "ak_dec_counters_reset", "IntPtr ctx"),
    ("void", "ak_bdr_reset", "IntPtr ctx"),
    ("int", "ak_bdr_ptr", "IntPtr ctx, byte** ptr, nuint* len"),
    ("int", "ak_bdr_reserve", "IntPtr ctx, nuint bytes"),
    ("nuint", "ak_bdr_footprint", "IntPtr ctx"),
    ("nuint", "ak_layout_facts", "uint* outp, nuint cap"),
]

CONSTS = ["AK_ABI_VERSION", "AK_OK", "AK_ALREADY_INITIALIZED", "AK_ERR_HOST", "AK_ERR_MALFORMED",
          "AK_ERR_TRUNCATED", "AK_ERR_DEPTH", "AK_ERR_LIMIT", "AK_ERR_TRANSCODE",
          "AK_ERR_CAPACITY", "AK_ERR_INVALID_STATE", "AK_ERR_PANIC", "AK_ERR_UNINITIALIZED",
          "AK_ERR_ABI", "AK_INIT_OWN_LOGGING", "AK_INIT_NO_PANIC_HOOK", "AK_INIT_NO_CRYPTO"]


def emit_import(o, ret, name, args, lib="Lib", indent="    "):
    """ONE declaration, both language levels (FIX-PLAN WP5 item 3)."""
    o += "#if NET7_0_OR_GREATER"
    o += "%s[LibraryImport(%s)]" % (indent, lib)
    o += "%s[UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]" % indent
    o += "%sinternal static partial %s %s(%s);" % (indent, ret, name, args)
    o += "#else"
    o += "%s[DllImport(%s, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]" % (indent, lib)
    o += "%sinternal static extern %s %s(%s);" % (indent, ret, name, args)
    o += "#endif"


def _struct(o, name, fields, doc=None, unsafe=False):
    if doc:
        o.doc(doc)
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public %sstruct %s" % ("unsafe " if unsafe else "", name)
    o += "{"
    for fn, ft in fields:
        o += "    public %s %s;" % (ft, fn)
    o += "}"
    o += ""


def group_decls(p):
    """[(struct name, [(member, C# type)])] for every group, in plan.abi_order_topo order,
    e/d/u per message -- the enumeration section 10's `ak_layout_facts` uses."""
    out = []
    for name in abi_order_topo(p):
        m = p.msg(name)
        for pre in ("e", "d", "u"):
            fields = ugroup_fields(m) if pre == "u" else group_fields(m, pre != "d")
            out.append(("ak_%sfix_%s" % (pre, name),
                        [(fn, N.abi_type(t)) for fn, t in fields] + [("presence", "uint")]))
    return out


def vtable_decls(p):
    """[(struct name, [(member, C# type)])] for every vtable, member order as rust_abi."""
    out = []
    for name in vtable_messages(p):
        slots = loop_slots(p, name)
        ev = []
        if not slots:
            ev.append(("_reserved", "IntPtr"))
        for path, f in slots:
            ev.append(("loop_%s" % slot_name(path), LOOP_FN))
            et = elem_type(f)
            if et and loop_slots(p, et):
                ev.append(("elem_%s" % slot_name(path), "ak_evt_%s*" % et))
        out.append(("ak_evt_%s" % name, ev))
        dv = [("apply", "delegate* unmanaged[Cdecl]<IntPtr, void*, ak_dfix_%s*, void>" % name),
              ("unknown", UNK_FN)]
        for path, f in slots:
            sn = slot_name(path)
            dty, _ = slot_elem(f)
            et = elem_type(f)
            if et:
                dv.append(("unk_%s" % sn, UNK_FN))
            if not (et and not p.msg(et).leaf):
                dv.append(("add_%s" % sn, "delegate* unmanaged[Cdecl]<IntPtr, void*, long, %s*, int, void>"
                           % N.abi_type(dty)))
            else:
                dv.append(("new_%s" % sn, "delegate* unmanaged[Cdecl]<IntPtr, void*, long>"))
                dv.append(("apply_%s" % sn, "delegate* unmanaged[Cdecl]<IntPtr, void*, long, ak_dfix_%s*, void>" % et))
                for ipath, iff in loop_slots(p, et):
                    idty, _ = slot_elem(iff)
                    iet = elem_type(iff)
                    if iet and not p.msg(iet).leaf:
                        raise NotImplementedError("a non-leaf element inside a non-leaf element "
                                                  "(%s.%s): refused, as rust_abi does" % (et, slot_name(ipath)))
                    dv.append(("add_%s_%s" % (sn, slot_name(ipath)),
                               "delegate* unmanaged[Cdecl]<IntPtr, void*, long, %s*, int, void>" % N.abi_type(idty)))
        out.append(("ak_dvt_%s" % name, dv))
    return out


def root_imports(p):
    """[(ret, name, args)] of the per-message-set entry points, as rust_abi declares them."""
    out = []
    for root in p.roots:
        direct = ", byte* direct, nuint direct_len" if direct_fields(p, root) else ""
        out.append(("nint", "ak_encode_%s" % root,
                    "void* obj, IntPtr ctx, ak_evt_%s* vt, ak_efix_%s* fix%s" % (root, root, direct)))
        out.append(("nint", "ak_uencode_%s" % root,
                    "void* obj, IntPtr ctx, ak_evt_%s* vt, ak_ufix_%s* fix%s" % (root, root, direct)))
        out.append(("int", "ak_decode_%s" % root,
                    "IntPtr ctx, void* obj, byte* buf, nuint len, ak_dvt_%s* vt" % root))
        out.append(("int", "ak_parse_%s" % root, "IntPtr ctx, byte* buf, nuint len"))
    for et in sorted(element_types(p)):
        if p.msg(et).leaf:
            out.append(("int", "ak_elem_%s" % et, "IntPtr ctx, ak_efix_%s* elems, int n" % et))
            out.append(("int", "ak_uelem_%s" % et, "IntPtr ctx, ak_ufix_%s* elems, int n" % et))
        else:
            out.append(("int", "ak_elemu_%s" % et, "IntPtr ctx, ak_efix_%s* elems, int n, long tok0" % et))
            out.append(("int", "ak_uelemu_%s" % et, "IntPtr ctx, ak_ufix_%s* elems, int n, long tok0" % et))
    return out


def emit_abi(x, ns, lib="ak_core"):
    p = as_plan(x)
    lc = p.lifecycle
    consts = abi_constants(CONSTS)
    o = N.Head("The C ABI of ABI v1 for this message set, declared for C#.", p.source, "cs_binding")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += ""
    o += "namespace %s;" % ns
    o += ""
    # ---- vocabulary structs (hand-declared: the plan names them, the probe checks them)
    for name, doc, fields in VOCAB_STRUCTS:
        _struct(o, name, fields, doc)
    # ak_init_opts from plan.lifecycle's member list.
    oname, members = lc.opts_struct
    cs_members = []
    for mn, mt in members:
        cs_members.append((mn, {"u32": "uint", "*mut void": "IntPtr"}.get(mt, "IntPtr")))
    _struct(o, oname, cs_members, "ABI v1 section 3, members from plan.lifecycle. `log` null: "
            "this host installs no log bridge.")

    # ---- groups
    groups = group_decls(p)
    for sname, fields in groups:
        _struct(o, sname, fields)

    # ---- presence bits
    o.doc("The presence bits (plan.presence_bits), one per singular message child and per "
          "explicit field, in tag order; the same bit in the e, d and u group.")
    o += "public static class AkPresent"
    o += "{"
    for name in p.abi_order:
        for fname, b in presence_bits(p.msg(name)).items():
            o += "    public const uint %s_%s = 1u << %d;" % (name, fname, b)
    o += "}"
    o += ""

    # ---- vtables
    vts = vtable_decls(p)
    for vname, fields in vts:
        _struct(o, vname, fields, unsafe=True)

    # ---- the imports
    o.doc("ABI v1 section 6: plain exports, declared per symbol, so a missing one is a load "
          "failure. Every declaration carries both language levels.")
    o += "public static unsafe partial class Abi"
    o += "{"
    o += '    public const string Lib = "%s";' % lib
    o += ""
    o += "    /// R-G7: every entry point below requires `ak_init`. The static constructor runs"
    o += "    /// before the first call to ANY of them, so no path reaches the core uninitialised."
    o += "    static Abi() { AbiInit.Run(); }"
    o += ""
    for n in CONSTS:
        o += "    public const %s %s = %s;" % ("uint" if n.startswith(("AK_INIT_", "AK_ABI")) else "int",
                                              n, "%du" % consts[n] if n.startswith(("AK_INIT_", "AK_ABI"))
                                              else str(consts[n]))
    o += "    public const long AK_TOKEN_ROOT = -1;"
    o += "    /// ABI v1 section 8's direct-argument sentinel (`AK_STR_DIRECT`)."
    o += "    public static readonly IntPtr AK_STR_DIRECT = (IntPtr)1;"
    o += ""
    emit_import(o, "int", lc.init[0], "%s* opts, ak_err* err" % oname)
    for ret, name, args in FIXED_IMPORTS + root_imports(p):
        emit_import(o, ret, name, args)
    o += "}"
    o += ""

    # ---- ak_init (plan.lifecycle, R-G7)
    flags = " | ".join("Abi.%s" % fl for fl in lc.default_flags)
    o.doc("ABI v1 section 3, rendered from plan.lifecycle: `%s` with `%s`; %s are success, "
          "anything else is loud. `Ensure` is for code that calls the core through its own "
          "imports and must initialise it the same way." % (lc.init[0], " | ".join(lc.default_flags),
                                                             " and ".join(lc.success)))
    o += "public static unsafe class AbiInit"
    o += "{"
    o += "    public const uint Flags = %s;" % flags
    o += "    public static int Code { get; private set; } = int.MinValue;"
    o += "    public static void Ensure() => RuntimeHelpers.RunClassConstructor(typeof(Abi).TypeHandle);"
    o += "    internal static void Run()"
    o += "    {"
    o += "        // A GATE CONTROL, not a feature: set, the call is skipped, and against a core"
    o += "        // built with `%s` the first codec call must fail (the planted control)." % lc.gate_feature
    o += "        if (Environment.GetEnvironmentVariable(\"AK_GATE_PLANT_NO_INIT\") == \"1\") { Code = -999; return; }"
    o += "        var o = new %s { abi_version = Abi.AK_ABI_VERSION, flags = Flags };" % oname
    o += "        var e = new ak_err();"
    o += "        int rc = Abi.%s(&o, &e);" % lc.init[0]
    o += "        Code = rc;"
    o += "        if (rc != Abi.%s && rc != Abi.%s)" % lc.success
    o += "            throw new InvalidOperationException($\"ak_init failed: {rc} (code {e.code}, detail {e.detail})\");"
    o += "    }"
    o += "}"
    o += ""

    # ---- the layout checks
    o.doc("The C# side of the layout agreement. `Table` is the C# declaration with the C# "
          "compiler's own offsets and sizes (the probe comparison lives in the harness, by "
          "name, both ways); `Facts` compares the same compiler's numbers with the loaded "
          "core's `ak_layout_facts` (ABI v1 section 10).")
    o += "public static unsafe class AbiLayout"
    o += "{"
    o += "    public sealed class S { public string Name; public int Size; public int Fields; public List<(string Name, int Off, int Size)> F = new List<(string, int, int)>(); }"
    o += ""
    o += "    private static int Fsz<T>(T* _) where T : unmanaged => sizeof(T);"
    o += ""
    o += "    /// Every struct this binding declares, with every member: name, offset, size."
    o += "    public static List<S> Table()"
    o += "    {"
    o += "        var all = new List<S>();"
    allstructs = ([(n, [(fn, ft) for fn, ft in fs]) for n, _, fs in VOCAB_STRUCTS]
                  + [(oname, cs_members)] + groups + vts)
    for sname, fields in allstructs:
        o += "        {"
        o += "            var s = new S { Name = \"%s\", Size = sizeof(%s), Fields = typeof(%s).GetFields(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.NonPublic).Length };" % (sname, sname, sname)
        o += "            var v = default(%s); %s* z = &v;" % (sname, sname)
        for fn, ft in fields:
            fsz = "sizeof(IntPtr)" if ("*" in ft or ft.startswith("delegate*")) else "Fsz(&z->%s)" % fn
            o += "            s.F.Add((\"%s\", (int)((byte*)&z->%s - (byte*)z), %s));" % (fn, fn, fsz)
        o += "            all.Add(s);"
        o += "        }"
    o += "        return all;"
    o += "    }"
    o += ""
    nfacts = sum(1 + len(fs) for _, fs in groups)
    o += "    /// ABI v1 section 10: the loaded core's own sizeof/offsetof of every group member,"
    o += "    /// in plan order, against this compiler's. Returns the mismatches (empty = agree)."
    o += "    public static List<string> Facts()"
    o += "    {"
    o += "        var bad = new List<string>();"
    o += "        var mine = new List<(string, int)>();"
    for sname, fields in groups:
        o += "        { var v = default(%s); %s* z = &v; mine.Add((\"sizeof %s\", sizeof(%s)));" % (sname, sname, sname, sname)
        for fn, _ in fields:
            o += "          mine.Add((\"%s.%s\", (int)((byte*)&z->%s - (byte*)z)));" % (sname, fn, fn)
        o += "        }"
    o += "        var core = new uint[%d];" % max(1, nfacts)
    o += "        nuint have;"
    o += "        fixed (uint* c = core) have = Abi.ak_layout_facts(c, (nuint)core.Length);"
    o += "        if ((int)have != mine.Count) bad.Add($\"the core reports {have} layout facts, this binding has {mine.Count}\");"
    o += "        int n = Math.Min((int)have, mine.Count);"
    o += "        for (int i = 0; i < n; i++)"
    o += "            if (core[i] != (uint)mine[i].Item2) bad.Add($\"{mine[i].Item1}: core {core[i]}, C# {mine[i].Item2}\");"
    o += "        return bad;"
    o += "    }"
    o += "    public const int FactCount = %d;" % nfacts
    o += "}"
    return str(o)


# ============================================================== the RPC half (plan.rpc)

def _rpc_param(t):
    if t in ("u32", "u64", "i32", "usize"):
        return N.ABI[t]
    if t in ("*const u8", "*mut u8"):
        return "byte*"
    if t == "*mut void":
        return "IntPtr"
    if t == "ak_completion_cb":
        return "delegate* unmanaged[Cdecl]<IntPtr, ak_completion*, void>"
    mt = re.fullmatch(r"\*(?:const|mut) (ak_\w+)", t)
    if mt:
        h = mt.group(1)
        return "IntPtr" if h in as_plan_rpc_handles() else h + "*"
    raise KeyError("no C# spelling for RPC type %r" % t)


_HANDLES = []


def as_plan_rpc_handles():
    return _HANDLES


def _rpc_member(t):
    if t in ("u32", "u64", "i32", "usize"):
        return N.ABI[t]
    if t in ("*const u8", "*mut void"):
        return "IntPtr"
    if t.startswith("ak_"):
        return t
    raise KeyError("no C# spelling for RPC member type %r" % t)


def emit_rpc(x, ns, lib="ak_core"):
    """The RPC half's structs and prototypes from plan.rpc (R-G5), both language levels.
    `ak_init` is rendered here too, from plan.lifecycle: this binding is a binding (R-G7)."""
    p = as_plan(x)
    r = p.rpc
    lc = p.lifecycle
    _HANDLES[:] = list(r.handles)
    consts = abi_constants(["AK_ABI_VERSION", "AK_OK", "AK_ALREADY_INITIALIZED",
                            "AK_INIT_NO_PANIC_HOOK", "AK_INIT_NO_CRYPTO"])
    o = N.Head("ABI v1 section 9, the RPC half, declared for C# from plan.rpc.", p.source, "cs_binding")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += ""
    o += "namespace %s;" % ns
    o += ""
    structs = []
    for name, doc, members in r.structs:
        fields = [(mn, _rpc_member(mt)) for mn, mt, _ in members]
        structs.append((name, fields))
        _struct(o, name, fields, doc)
    oname, members = lc.opts_struct
    init_members = [(mn, {"u32": "uint"}.get(mt, "IntPtr")) for mn, mt in members]
    _struct(o, oname, init_members, "ABI v1 section 3 (plan.lifecycle).")
    _struct(o, "ak_err", [("code", "int"), ("detail", "uint")], "ak_init's out-parameter.")
    o += "public static unsafe partial class AkRpc"
    o += "{"
    o += '    public const string Lib = "%s";' % lib
    o += "    static AkRpc() { RpcInit.Run(); }"
    o += ""
    for cname, ctype, cval, cdoc in r.constants:
        if cdoc:
            o += "    /// %s" % cdoc
        o += "    public const %s %s = %d;" % (N.ABI[ctype], cname, cval)
    for n in ("AK_ABI_VERSION", "AK_INIT_NO_PANIC_HOOK", "AK_INIT_NO_CRYPTO"):
        o += "    public const uint %s = %du;" % (n, consts[n])
    for n in ("AK_OK", "AK_ALREADY_INITIALIZED"):
        o += "    public const int %s = %d;" % (n, consts[n])
    o += ""
    emit_import(o, "int", lc.init[0], "%s* opts, ak_err* err" % oname)
    for fname, params, ret, doc in r.functions:
        if doc:
            o += "    /// %s" % doc
        rt = "void" if ret is None else _rpc_param(ret)
        args = ", ".join("%s %s" % (_rpc_param(t), "@" + pn if pn in N.KEYWORDS else pn) for pn, t in params)
        emit_import(o, rt, fname, args)
    o += "}"
    o += ""
    flags = " | ".join("AkRpc.%s" % fl for fl in lc.default_flags)
    o += "public static unsafe class RpcInit"
    o += "{"
    o += "    public const uint Flags = %s;" % flags
    o += "    public static int Code { get; private set; } = int.MinValue;"
    o += "    public static void Ensure() => RuntimeHelpers.RunClassConstructor(typeof(AkRpc).TypeHandle);"
    o += "    internal static void Run()"
    o += "    {"
    o += "        if (Environment.GetEnvironmentVariable(\"AK_GATE_PLANT_NO_INIT\") == \"1\") { Code = -999; return; }"
    o += "        var o = new %s { abi_version = AkRpc.AK_ABI_VERSION, flags = Flags };" % oname
    o += "        var e = new ak_err();"
    o += "        int rc = AkRpc.%s(&o, &e);" % lc.init[0]
    o += "        Code = rc;"
    o += "        if (rc != AkRpc.%s && rc != AkRpc.%s)" % lc.success
    o += "            throw new InvalidOperationException($\"ak_init failed: {rc} (code {e.code}, detail {e.detail})\");"
    o += "    }"
    o += "}"
    o += ""
    o += "public static unsafe class RpcLayout"
    o += "{"
    o += "    private static int Fsz<T>(T* _) where T : unmanaged => sizeof(T);"
    o += "    public static List<(string Name, int Size, int Fields, List<(string Name, int Off, int Size)> F)> Table()"
    o += "    {"
    o += "        var all = new List<(string, int, int, List<(string, int, int)>)>();"
    for sname, fields in structs + [(oname, init_members), ("ak_err", [("code", "int"), ("detail", "uint")])]:
        o += "        {"
        o += "            var v = default(%s); %s* z = &v; var f = new List<(string, int, int)>();" % (sname, sname)
        for fn, ft in fields:
            fsz = "sizeof(IntPtr)" if ("*" in ft) else "Fsz(&z->%s)" % fn
            o += "            f.Add((\"%s\", (int)((byte*)&z->%s - (byte*)z), %s));" % (fn, fn, fsz)
        o += "            all.Add((\"%s\", sizeof(%s), typeof(%s).GetFields(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.NonPublic).Length, f));" % (sname, sname, sname)
        o += "        }"
    o += "        return all;"
    o += "    }"
    o += "}"
    return str(o)
