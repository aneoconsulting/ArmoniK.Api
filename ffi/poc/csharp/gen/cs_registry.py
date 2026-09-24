"""Harness glue: the by-payload registry of core-ffi arms, and the corpus dispatch table.

Glue, not a codec: each entry wraps the `CoreFfi_<Root>` class the shared backend
(`ffi/poc/codec/gen/cs_host.py`) rendered from the plan, so the gate and the corpus runner
drive every root through one interface and none can be missing.
"""
from glue import Head


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
