"""s16 (owner, 2026-10-10): the reset-on-entry measurement's per-root glue (harness only, no wire
rule). For each root, RoeOps_<Root> binds to a codec-suite case's Ops_<Root> (its graph and its
DEFAULT binding, the explicit-reset path, which the case's own delegate times) and holds a second
binding rendered with reset_on_entry=True (namespace Armonik.Ffi.HarnessRoe), whose calls mirror
Ops_<Root>.EncFfiCore and Ops_<Root>.DecFfi exactly. Compiled only in the AkRoeBench build
(#if AK_ROE_BENCH), so the default build is unchanged."""
from glue import Head

BASE = r'''
/// One root's reset-on-entry path beside a case's explicit-reset path (RoeBench.cs).
public abstract unsafe class RoeOps
{
    public abstract string Root { get; }
    /// The case's Ops_<Root> (untimed): its graph, and its binding as the explicit path.
    public abstract void Bind(RootOps ops);
    /// Ops_<Root>.EncFfiCore through the reset-on-entry binding.
    public abstract int EncCoreR(bool retain);
    public abstract byte[] EncCoreBytesR(bool retain);
    /// Ops_<Root>.DecFfi (the FSM) through the reset-on-entry binding.
    public abstract long DecR(byte[] b, int len, bool retain, bool read);
    /// host-gen's re-encoding of the graph each path decodes (the identity check, untimed).
    public abstract byte[] DecReEncE(byte[] b, int len, bool retain);
    public abstract byte[] DecReEncR(byte[] b, int len, bool retain);
    /// The explicit binding's core contexts: reset-on-entry OFF (they behave as the default
    /// core's, the host resets); call after the explicit path's first call of each kind.
    public abstract void ExplicitRoeOff();
    public abstract long ResetsE();
    public abstract long ResetsR();
    public static RoeOps For(string root)
    {
        switch (root)
        {
%CASES%
            default: throw new ArgumentException("no RoeOps for " + root);
        }
    }
    protected static object Field(object o, string name) =>
        o.GetType().GetField(name, System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance).GetValue(o);
}
'''


def _root(o, root):
    o += "public sealed unsafe class RoeOps_%s : RoeOps" % root
    o += "{"
    o += "    private %s _f;" % root
    o += "    private CoreFfi_%s _e;" % root
    o += "    private readonly Armonik.Ffi.HarnessRoe.CoreFfi_%s _r = new Armonik.Ffi.HarnessRoe.CoreFfi_%s();" % (root, root)
    o += "    public override string Root => \"%s\";" % root
    o += "    public override void Bind(RootOps ops) { _f = (%s)Field(ops, \"_f\"); _e = (CoreFfi_%s)Field(ops, \"_c\"); }" % (root, root)
    o += "    public override int EncCoreR(bool retain) { int rc = _r.EncodeInto(_f, retain); if (rc < 0) throw new InvalidOperationException(\"core encode \" + rc); return rc; }"
    o += "    public override byte[] EncCoreBytesR(bool retain) { int rc = _r.EncodeInto(_f, retain); if (rc < 0) throw new InvalidOperationException(\"core encode \" + rc); return _r.ContextBytes(); }"
    o += "    public override long DecR(byte[] b, int len, bool retain, bool read)"
    o += "    {"
    o += "        int rc = _r.TryFsm(b, len, retain, out var m);   // as Ops_%s.DecFfi" % root
    o += "        if (rc < 0) throw new InvalidOperationException(\"core fsm decode \" + rc);"
    o += "        return read ? Touch.F_%s(m) : 1;" % root
    o += "    }"
    for side, fld in (("E", "_e"), ("R", "_r")):
        o += "    public override byte[] DecReEnc%s(byte[] b, int len, bool retain)" % side
        o += "    {"
        o += "        int rc = %s.TryFsm(b, len, retain, out var m);" % fld
        o += "        if (rc < 0) throw new InvalidOperationException(\"core fsm decode \" + rc);"
        o += "        var e = Enc.New(Codec.Sites, len + 4096);"
        o += "        if (retain) HostR.Write%s(ref e, m); else Codec.Write%s(ref e, m);" % (root, root)
        o += "        return e.ToArray();"
        o += "    }"
    o += "    public override void ExplicitRoeOff()"
    o += "    {"
    o += "        Armonik.Ffi.HarnessRoe.RoeCore.EncSetRoe(_e.EncContext, 0);"
    o += "        var dh = Field(_e, \"_dh\");"
    o += "        if (dh != null) Armonik.Ffi.HarnessRoe.RoeCore.DecSetRoe((IntPtr)dh.GetType().GetField(\"Ctx\").GetValue(dh), 0);"
    o += "    }"
    o += "    public override long ResetsE() => _e.ResetCalls;"
    o += "    public override long ResetsR() => _r.ResetCalls;"
    o += "}"
    o += ""


def emit(p):
    o = Head("The reset-on-entry measurement's per-root glue (s16).", "cs_roebench")
    o += "#if AK_ROE_BENCH"
    o += "using System;"
    o += "using Armonik.Ffi.Facade;"
    o += "using Armonik.Ffi.Harness;"
    o += ""
    o += "namespace Armonik.Ffi.Campaign;"
    cases = "\n".join("            case \"%s\": return new RoeOps_%s();" % (r, r) for r in p.roots)
    for ln in BASE.replace("%CASES%", cases).strip("\n").split("\n"):
        o += ln
    o += ""
    for r in p.roots:
        _root(o, r)
    o += "#endif"
    return str(o)


TIMING = r'''
/// s16: one ak_enc_reset and one ak_dec_reset_<Root> timed from C# in isolation, through the
/// default P/Invoke binding (Abi), process CPU per call over a block of back-to-back calls.
public static unsafe class RoeTiming
{
    [StructLayout(LayoutKind.Sequential)] private struct Timespec { public long Sec, Nsec; }
    [DllImport("libc")] private static extern int clock_gettime(int clk, Timespec* ts);
    public static long Cpu() { Timespec t; clock_gettime(2, &t); return t.Sec * 1_000_000_000L + t.Nsec; }
    public static string[] Roots = { %ROOTS% };
    public static IntPtr NewEnc() => Abi.ak_enc_ctx_new();
    public static double EncReset(IntPtr ctx, long n)
    {
        long c0 = Cpu();
        for (long i = 0; i < n; i++) Abi.ak_enc_reset(ctx);
        return (double)(Cpu() - c0) / n;
    }
    /// A root's decode context (drop mode) and an options struct naming the one grow at every
    /// position (as the binding arms a retained decode), native, zeroed otherwise.
    public static IntPtr NewDec(string root, out void* opts)
    {
        var g = UnkHost.Fn;
        switch (root)
        {
%NEWDEC%
            default: throw new ArgumentException(root);
        }
    }
    /// ns per ak_dec_reset_<root>(ctx, opts) (opts null: the drop reset).
    public static double DecReset(string root, IntPtr ctx, void* opts, long n)
    {
        long c0;
        switch (root)
        {
%DECRESET%
            default: throw new ArgumentException(root);
        }
        return (double)(Cpu() - c0) / n;
    }
}
'''


def emit_timing(p):
    from plan import unk_opts_layout, unk_opts_name
    o = Head("The reset-on-entry measurement's reset timing (s16).", "cs_roebench")
    o += "#if AK_ROE_BENCH"
    o += "using System;"
    o += "using System.Runtime.InteropServices;"
    o += "using Armonik.Ffi.Harness;"
    o += ""
    o += "namespace Armonik.Ffi.HarnessRoe;"
    newdec, decreset = [], []
    for r in p.roots:
        on = unk_opts_name(r)
        lay = unk_opts_layout(p, r)
        newdec.append("            case \"%s\":" % r)
        newdec.append("            {")
        newdec.append("                var o = (%s*)NativeMemory.AllocZeroed((nuint)sizeof(%s));" % (on, on))
        for n, _m, _t in lay:
            newdec.append("                o->%s.grow = g;" % n)
        newdec.append("                opts = o;")
        newdec.append("                return Abi.ak_dec_ctx_new_%s(null);" % r)
        newdec.append("            }")
        decreset.append("            case \"%s\":" % r)
        decreset.append("            {")
        decreset.append("                var o = (%s*)opts;" % on)
        decreset.append("                c0 = Cpu();")
        decreset.append("                for (long i = 0; i < n; i++) Abi.ak_dec_reset_%s(ctx, o);" % r)
        decreset.append("                break;")
        decreset.append("            }")
    t = (TIMING.replace("%ROOTS%", ", ".join('"%s"' % r for r in p.roots))
         .replace("%NEWDEC%", "\n".join(newdec)).replace("%DECRESET%", "\n".join(decreset)))
    for ln in t.strip("\n").split("\n"):
        o += ln
    o += "#endif"
    return str(o)
