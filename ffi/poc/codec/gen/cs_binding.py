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

The fixed ABI -- the vocabulary structs, the fixed entry points, the codes, the version, the
lifecycle flag values and the RPC counting surface -- is `plan.FIXED` (WP5 step 6, R-G13),
rendered here into C#; nothing is read out of `ak-abi`'s Rust source any more. The probe
(`cs_layout_probe`, which parses the RUST text) still checks the structs, independently.
"""
import os
import re

from plan import (FIXED, as_plan, abi_order_topo, direct_fields, element_types, elem_type,
                  group_fields, loop_slots, presence_bits, slot_elem, slot_name,
                  ugroup_fields, vtable_messages)
import cs_names as N

LOOP_FN = "delegate* unmanaged[Cdecl]<IntPtr, void*, long, int>"


def cs_member(t):
    """The C# spelling of a fixed/RPC STRUCT member type of the plan's vocabulary."""
    t = t.strip()
    if t in N.ABI:
        return N.ABI[t]
    if t.startswith("*") or t.endswith("?") or t.endswith("_fn") or t.endswith("_f") \
            or t.endswith("_cb"):
        return "IntPtr"             # a pointer or a function pointer, as a word
    if t.startswith("ak_"):
        return t                    # a nested struct
    raise KeyError("no C# spelling for member type %r" % t)


_HANDLES = set(["ak_enc_ctx", "ak_dec_ctx"])
_STRUCT_NAMES = set(n for n, _d, _m in FIXED.structs) | {"ak_bytes", "ak_completion", "ak_rpc_counters",
                                                        "ak_client_opts"}


def cs_param(t):
    """The C# spelling of an import PARAMETER or RETURN type of the plan's vocabulary."""
    t = t.strip()
    if t in N.ABI:
        return N.ABI[t]
    if t == "isize":
        return "nint"
    if t == "fn(u64)->u64":
        return "delegate* unmanaged[Cdecl]<ulong, ulong>"
    if t == "ak_completion_cb":
        return "delegate* unmanaged[Cdecl]<IntPtr, ak_completion*, void>"
    if t.endswith("_fn"):
        return "IntPtr"             # a transcoder handed back to the core, opaque here
    if t in ("*mut void", "*const void", "*const char"):
        return "IntPtr"
    if t == "*mut *const u8":
        return "byte**"
    mt = re.fullmatch(r"\*(?:const|mut) (\w+)", t)
    if mt:
        inner = mt.group(1)
        if inner in _HANDLES:
            return "IntPtr"
        if inner in _STRUCT_NAMES:
            return inner + "*"
        return cs_param(inner) + "*"
    raise KeyError("no C# spelling for parameter type %r" % t)


def _pname(n):
    return "@" + n if n in N.KEYWORDS else n


def _fixed_structs():
    """plan.FIXED's vocabulary structs as (name, doc, [(member, C# type)])."""
    return [(n, d, [(mn, cs_member(mt)) for mn, mt in ms]) for n, d, ms in FIXED.structs]


def _fixed_imports():
    """(C# return, name, C# args) for every fixed entry point of plan.FIXED."""
    out = []
    for _g, name, params, ret, _doc in FIXED.all_functions():
        if name == FIXED.functions[0][1]:
            continue                # ak_init is emitted from plan.lifecycle, first
        rt = "void" if ret is None else cs_param(ret)
        out.append((rt, name, ", ".join("%s %s" % (cs_param(t), _pname(pn)) for pn, t in params)))
    return out


def _consts():
    """(C# type, name, literal) for the codes, the version and the lifecycle flags."""
    out = [("uint", "AK_ABI_VERSION", "%du" % FIXED.abi_version)]
    out += [("int", n, str(v)) for n, v, _d in FIXED.codes]
    out += [("uint", n, "%du" % v) for n, v in LIFECYCLE_FLAGS()]
    return out


def LIFECYCLE_FLAGS():
    from plan import LIFECYCLE
    return LIFECYCLE.flag_values


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
        # WP5 step 7: no `unknown` / `unk_<slot>` members (decision 11: unknown fields travel
        # as data in the groups); the order is plan.dec_vtable's.
        dv = [("apply", "delegate* unmanaged[Cdecl]<IntPtr, void*, ak_dfix_%s*, void>" % name)]
        for path, f in slots:
            sn = slot_name(path)
            dty, _ = slot_elem(f)
            et = elem_type(f)
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
    o = N.Head("The C ABI of ABI v1 for this message set, declared for C#.", p.source, "cs_binding")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += ""
    o += "namespace %s;" % ns
    o += ""
    # ---- vocabulary structs, ak_init_opts among them (plan.FIXED.structs)
    oname = lc.opts_struct[0]
    for name, doc, fields in _fixed_structs():
        _struct(o, name, fields, doc)

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
    for cty, n, lit in _consts():
        o += "    public const %s %s = %s;" % (cty, n, lit)
    o += "    public const long AK_TOKEN_ROOT = -1;"
    o += "    /// ABI v1 section 8's direct-argument sentinel (`AK_STR_DIRECT`)."
    o += "    public static readonly IntPtr AK_STR_DIRECT = (IntPtr)1;"
    o += ""
    emit_import(o, "int", lc.init[0], "%s* opts, ak_err* err" % oname)
    for ret, name, args in _fixed_imports() + root_imports(p):
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
    allstructs = ([(n, fs) for n, _, fs in _fixed_structs()] + groups + vts)
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

def emit_rpc(x, ns, lib="ak_core"):
    """The RPC half's structs and prototypes from plan.rpc (R-G5), both language levels.
    `ak_init` is rendered here too, from plan.lifecycle: this binding is a binding (R-G7)."""
    p = as_plan(x)
    r = p.rpc
    lc = p.lifecycle
    _HANDLES.update(r.handles)
    consts = {n: v for _t, n, v in _consts()}
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
        fields = [(mn, cs_member(mt)) for mn, mt, _ in members]
        structs.append((name, fields))
        _struct(o, name, fields, doc)
    oname = lc.opts_struct[0]
    fixed = {n: (d, fs) for n, d, fs in _fixed_structs()}
    init_members = fixed[oname][1]
    err_members = fixed["ak_err"][1]
    _struct(o, oname, init_members, fixed[oname][0])
    _struct(o, "ak_err", err_members, fixed["ak_err"][0])
    # plan.FIXED's RPC counting surface (README R5 for section 9), rendered like the rest
    # (D40): the struct, and the three imports below with both language levels.
    cname_, cdoc_, cmembers_ = FIXED.rpc_counters_struct
    counters = [(mn, N.ABI[mt]) for mn, mt in cmembers_]
    structs.append((cname_, counters))
    _struct(o, cname_, counters, cdoc_)
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
        o += "    public const uint %s = %s;" % (n, consts[n])
    for n in ("AK_OK", "AK_ALREADY_INITIALIZED"):
        o += "    public const int %s = %s;" % (n, consts[n])
    o += ""
    emit_import(o, "int", lc.init[0], "%s* opts, ak_err* err" % oname)
    for fname, params, ret, doc in r.functions:
        if doc:
            o += "    /// %s" % doc
        rt = "void" if ret is None else cs_param(ret)
        args = ", ".join("%s %s" % (cs_param(t), _pname(pn)) for pn, t in params)
        emit_import(o, rt, fname, args)
    for _g, fname, params, ret, doc in FIXED.rpc_counting:
        if doc:
            o += "    /// %s" % doc
        rt = "void" if ret is None else cs_param(ret)
        args = ", ".join("%s %s" % (cs_param(t), _pname(pn)) for pn, t in params)
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
    for sname, fields in structs + [(oname, init_members), ("ak_err", err_members)]:
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
