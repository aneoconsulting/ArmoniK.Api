"""Harness glue: the by-payload registry of core-ffi arms, and the corpus dispatch table.

Glue, not a codec: each entry wraps the `CoreFfi_<Root>` class the shared backend
(`ffi/poc/codec/gen/cs_host.py`) rendered from the plan, so the gate and the corpus runner
drive every root through one interface and none can be missing.
"""
from glue import Head
from plan import unknown_compiled_out


def emit_registry(p, payloads):
    """[(payload id, root)] from ffi/schema's manifest -> the ICoreArm table."""
    o = Head("Every payload's core-ffi arm, by id. Emitted, so none can be missing.", "cs_registry")
    o += "#if NET5_0_OR_GREATER"
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""
    o += "public unsafe interface ICoreArm : IDisposable"
    o += "{"
    o += "    string Root { get; }"
    o += "    int Chunk { get; set; }"
    o += "    long ForwardCalls { get; }"
    o += "    long ReverseCalls { get; }"
    o += "    byte[] EncodeToArray();"
    o += "    /// `ak_uencode_*`: the unknown-field bags handed over (empty on these payloads)."
    o += "    byte[] EncodeToArrayU();"
    o += "    int EncodeNoCopy();"
    o += "    int Fill();"
    o += "    int Decode(byte[] src, int len);"
    o += "    /// Push decode with the unknown-field capture callbacks installed."
    o += "    int DecodeU(byte[] src, int len);"
    o += "    int Pull(byte[] src, int len);"
    o += "    object Sink { get; }"
    o += "    bool SameAsSource();"
    o += "    AkCounters EncCounters();"
    o += "    void EncCountersReset();"
    o += "    AkCounters DecCounters();"
    o += "    void DecCountersReset();"
    o += "    void CallsReset();"
    o += "    bool SkipStrings { get; set; }"
    o += "    long PullFootprint();"
    o += "}"
    o += ""
    for r in p.roots:
        o += "public sealed unsafe class Arm_%s : ICoreArm" % r
        o += "{"
        o += "    private readonly CoreFfi_%s _c;" % r
        o += "    private readonly %s _src;" % r
        o += "    private %s _sink;" % r
        o += "    public Arm_%s(%s src, bool utf16 = false) { _src = src; _c = new CoreFfi_%s(utf16); }" % (r, r, r)
        o += "    public string Root => \"%s\";" % r
        o += "    public int Chunk { get => _c.Chunk; set => _c.Chunk = value; }"
        o += "    public long ForwardCalls => _c.ForwardCalls;"
        o += "    public long ReverseCalls => _c.ReverseCalls;"
        o += "    public byte[] EncodeToArray() => _c.EncodeToArray(_src);"
        o += "    public byte[] EncodeToArrayU() => _c.EncodeToArray(_src, true);"
        o += "    public int EncodeNoCopy() { _c.Encode(_src, out byte* p, out int l); return l; }"
        o += "    public int Fill() => _c.Fill(_src);"
        o += "    public int Decode(byte[] src, int len) { _sink = _c.Decode(src, len); return 1; }"
        if unknown_compiled_out(p):
            # WP5 step 10: the retain paths do not exist in this build.
            o += "    public int DecodeU(byte[] src, int len) => throw new NotSupportedException(\"unknown fields are compiled out of this build\");"
        else:
            o += "    public int DecodeU(byte[] src, int len) { _sink = _c.DecodeU(src, len); return 1; }"
        o += "    public int Pull(byte[] src, int len) { _sink = _c.Pull(src, len); return 1; }"
        o += "    public object Sink => _sink;"
        o += "    public bool SameAsSource() => Eq.Same%s(_sink, _src);" % r
        o += "    public AkCounters EncCounters() => _c.EncCounters();"
        o += "    public void EncCountersReset() => _c.EncCountersReset();"
        o += "    public AkCounters DecCounters() => _c.DecCounters();"
        o += "    public void DecCountersReset() => _c.DecCountersReset();"
        o += "    public void CallsReset() => _c.CallsReset();"
        o += "    public bool SkipStrings { get => G.SkipStrings; set => G.SkipStrings = value; }"
        o += "    public long PullFootprint() => _c.PullFootprint();"
        o += "    public void Dispose() => _c.Dispose();"
        o += "}"
        o += ""
    o += "public static class CoreArms"
    o += "{"
    o += "    public static readonly string[] Ids = { %s };" % ", ".join('"%s"' % pid for pid, _ in payloads)
    o += ""
    o += "    public static ICoreArm New(string id, bool utf16 = false)"
    o += "    {"
    o += "        switch (id)"
    o += "        {"
    for pid, r in payloads:
        o += "            case \"%s\": return new Arm_%s(BuildFacade.%s(), utf16);" % (pid, r, pid.replace(".", "_"))
    o += "            default: throw new ArgumentException(\"no core-ffi arm for \" + id);"
    o += "        }"
    o += "    }"
    o += "}"
    o += "#endif"
    return str(o)


def emit_corpus_dispatch(abi, refused):
    """The corpus runner's per-root table over the core-ffi binding of the corpus core."""
    o = Head("The corpus runner's core-ffi dispatch, by root name.", "cs_registry", abi.source)
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace Armonik.Ffi.Corpus;"
    o += ""
    o += "public sealed class UnkRow"
    o += "{"
    o += "    public int Positions;"
    o += "    public bool PullEqual;"
    o += "    public string Error;"
    o += "    public List<string> Mismatched = new List<string>();"
    o += "    public List<string> Changed = new List<string>();"
    o += "}"
    o += ""
    o += "public static unsafe class Ffi"
    o += "{"
    o += "    public static readonly string[] Roots = { %s };" % ", ".join('"%s"' % r for r in abi.roots)
    o += "    /// Roots the C ABI cannot carry, REFUSED by the generator by name (plan.check_expressible)."
    o += "    public static readonly string[][] NotInAbi ="
    o += "    {"
    for r, why in sorted(refused.items()):
        o += "        new[] { \"%s\", \"%s\" }," % (r, why.replace("\\", "\\\\").replace('"', '\\"'))
    o += "    };"
    o += ""
    for r in abi.roots:
        o += "    private static CoreFfi_%s _%s;" % (r, r)
    o += ""
    o += "    /// < 0: the core's code (the output is unspecified, R-G6); 1: not in the C ABI."
    o += "    public static int Decode(string root, byte[] b, bool retain, out object msg)"
    o += "    {"
    o += "        msg = null;"
    o += "        switch (root)"
    o += "        {"
    for r in abi.roots:
        o += "            case \"%s\": { var c = _%s ??= new CoreFfi_%s(); int rc = c.TryDecode(b, b.Length, retain, out var t); msg = t; return rc; }" % (r, r, r)
    o += "            default: return 1;"
    o += "        }"
    o += "    }"
    o += ""
    o += "    /// Decision 11's controls on one accept row (WP5 step 9). Retained push decode as the"
    o += "    /// reference; (1) each position zeroed in turn must equal the reference with that"
    o += "    /// position's facade bags cleared (`plant`: not cleared, so rows with unknowns MUST"
    o += "    /// mismatch); (2) the pull family must deliver what push does. Compared as retained"
    o += "    /// re-encodings (ak_uencode_*), which carry every bag. null: root not in the C ABI."
    o += "    public static UnkRow UnkControl(string root, byte[] b, bool plant)"
    o += "    {"
    if unknown_compiled_out(abi):
        o += "        return null;   // WP5 step 10: no options exist in this build"
        o += "    }"
    o += "        switch (root)" if not unknown_compiled_out(abi) else ""
    o += "        {" if not unknown_compiled_out(abi) else ""
    for r in ([] if unknown_compiled_out(abi) else abi.roots):
        o += "            case \"%s\":" % r
        o += "            {"
        o += "                var c = _%s ??= new CoreFfi_%s();" % (r, r)
        o += "                var u = new UnkRow { Positions = CoreFfi_%s.UnkPositionNames.Length };" % r
        o += "                int rc = c.TryDecode(b, b.Length, true, out var all);"
        o += "                if (rc < 0) { u.Error = \"retained decode \" + rc + (rc == CoreFfi_%s.UNDELIVERED ? \" (UNDELIVERED \" + c.Undelivered + \")\" : \"\"); return u; }" % r
        o += "                var want = c.EncodeToArray(all, true);"
        o += "                rc = c.TryPull(b, b.Length, true, out var pl);"
        o += "                u.PullEqual = rc == 0 && c.EncodeToArray(pl, true).AsSpan().SequenceEqual(want);"
        o += "                for (int i = 0; i < u.Positions; i++)"
        o += "                {"
        o += "                    rc = c.TryDecodeZeroing(b, b.Length, i, out var z);"
        o += "                    if (rc < 0) { u.Mismatched.Add(CoreFfi_%s.UnkPositionNames[i] + \" rc \" + rc); continue; }" % r
        o += "                    var zb = c.EncodeToArray(z, true);"
        o += "                    c.TryDecode(b, b.Length, true, out var exp);"
        o += "                    if (!plant) CoreFfi_%s.ClearPosition(exp, i);" % r
        o += "                    if (!zb.AsSpan().SequenceEqual(c.EncodeToArray(exp, true))) u.Mismatched.Add(CoreFfi_%s.UnkPositionNames[i]);" % r
        o += "                    if (!zb.AsSpan().SequenceEqual(want)) u.Changed.Add(CoreFfi_%s.UnkPositionNames[i]);" % r
        o += "                }"
        o += "                return u;"
        o += "            }"
    if not unknown_compiled_out(abi):
        o += "            default: return null;"
        o += "        }"
        o += "    }"
    o += ""
    a, b = abi.roots[0], abi.roots[1]
    nounk = unknown_compiled_out(abi)
    o += "    /// Decision 11 rule 6: a context bound to %s, used for %s, is refused" % (a, b)
    o += "    /// (AK_ERR_INVALID_STATE) by decode, parse and reset, and still serves its own root."
    o += "    public static (int Decode, int Parse, int Reset, int OwnReset, int OwnParse) WrongRoot()"
    o += "    {"
    o += "        AbiInit.Ensure();"
    o += "        IntPtr ctx = Abi.ak_dec_ctx_new_%s(%s);" % (a, "" if nounk else "null")
    o += "        byte one = 0;"
    o += "        var vt = default(ak_dvt_%s);" % b
    o += "        int d = Abi.ak_decode_%s(ctx, null, &one, 0, &vt);" % b
    o += "        int p = Abi.ak_parse_%s(ctx, &one, 0);" % b
    if nounk:
        o += "        // WP5 step 10: no ak_dec_reset_* in this build: int.MinValue = not applicable."
        o += "        int r = int.MinValue, or = int.MinValue;"
    else:
        o += "        int r = Abi.ak_dec_reset_%s(ctx, null);" % b
        o += "        int or = Abi.ak_dec_reset_%s(ctx, null);" % a
    o += "        int op = Abi.ak_parse_%s(ctx, &one, 0);" % a
    o += "        Abi.ak_dec_ctx_free(ctx);"
    o += "        return (d, p, r, or, op);"
    o += "    }"
    o += "    public static readonly string WrongRootPair = \"%s context, %s entry points\";" % (a, b)
    o += ""
    o += "    public static int Encode(string root, object msg, bool retain, out byte[] bytes)"
    o += "    {"
    o += "        bytes = null;"
    o += "        switch (root)"
    o += "        {"
    for r in abi.roots:
        o += "            case \"%s\": { var c = _%s ??= new CoreFfi_%s(); int rc = c.TryEncode((%s)msg, retain, out byte* p, out int n); if (rc < 0) return rc; bytes = new ReadOnlySpan<byte>(p, n).ToArray(); return 0; }" % (r, r, r, r)
    o += "            default: return 1;"
    o += "        }"
    o += "    }"
    o += "}"
    return str(o)
