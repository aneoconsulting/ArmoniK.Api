"""Harness glue: the campaign runner's per-root operations (design/CAMPAIGN.md, FIX-PLAN WP3).

Glue, not a codec. For each root it emits the calls each timed arm makes -- the incumbent's
production path (the Grpc.Tools marshaller: CalculateSize + WriteTo(IBufferWriter), and
ParseFrom(ReadOnlySequence)), the incumbent's fastest entry point, the managed codec
(`host-gen`, rendered by poc/codec/gen/cs_managed.py) and core-ffi (poc/codec/gen/cs_host.py)
-- plus a `Touch` visitor per message over both object models, which is what requirement 9's
"decode followed by reading every field" reads. Nothing here decides a wire rule.
"""
from glue import Head
import cs_names as N
from cs_types import facade_messages

GP = "Gp"


def _touch_scalar(kind, e):
    if kind == "bool":
        return "(%s ? 1L : 0L)" % e
    if kind == "double":
        return "BitConverter.DoubleToInt64Bits(%s)" % e
    if kind == "enum":
        return "(long)(int)%s" % e
    return "(long)%s" % e


def _touch_facade(o, p, m):
    o += "    public static long F_%s(%s m)" % (m.name, m.name)
    o += "    {"
    o += "        if (m == null) return 0;"
    o += "        long h = 0;"
    for f in m.plain:
        a = "m." + N.field(f.name)
        if f.card == "map":
            o += "        for (int i = 0; i < %s.Count; i++) { var kv = %s.At(i); h += kv.Key.Length + kv.Value.Length; }" % (a, a)
        elif f.card in ("repeated", "packed"):
            if f.kind == "message":
                el = "F_%s(x)" % f.of
            elif f.kind == "string":
                el = "x.Length"
            elif f.kind == "bytes":
                el = "x.Length"
            else:
                el = _touch_scalar(f.kind, "x")
            o += "        foreach (var x in %s) h += %s;" % (a, el)
        elif f.kind == "message":
            o += "        h += F_%s(%s);" % (f.of, a)
        elif f.kind in ("string", "bytes"):
            o += "        h += %s == null ? 0 : %s.Length;" % (a, a)
        elif f.explicit:
            o += "        if (%s.HasValue) h += %s;" % (a, _touch_scalar(f.kind, a + ".Value"))
        else:
            o += "        h += %s;" % _touch_scalar(f.kind, a)
    for oname, members in m.oneofs.items():
        o += "        switch (m.%s)" % N.oneof_case_field(oname)
        o += "        {"
        for g in members:
            a = "m." + N.field(g.name)
            if g.kind == "message":
                v = "F_%s(%s)" % (g.of, a)
            elif g.kind in ("string", "bytes"):
                v = "(%s == null ? 0 : %s.Length)" % (a, a)
            else:
                v = _touch_scalar(g.kind, a)
            o += "            case %s.%s: h += %s; break;" % (N.oneof_case_type(m.name, oname), N.pascal(g.name), v)
        o += "        }"
    o += "        return h;"
    o += "    }"


def _touch_gp(o, p, m):
    o += "    public static long G_%s(%s.%s m)" % (m.name, GP, m.name)
    o += "    {"
    o += "        if (m == null) return 0;"
    o += "        long h = 0;"
    for f in m.plain:
        a = "m." + N.field(f.name)
        if f.card == "map":
            o += "        foreach (var kv in %s) h += kv.Key.Length + kv.Value.Length;" % a
        elif f.card in ("repeated", "packed"):
            if f.kind == "message":
                el = "G_%s(x)" % f.of
            elif f.kind in ("string", "bytes"):
                el = "x.Length"
            else:
                el = _touch_scalar(f.kind, "x")
            o += "        for (int i = 0; i < %s.Count; i++) { var x = %s[i]; h += %s; }" % (a, a, el)
        elif f.kind == "message":
            o += "        h += G_%s(%s);" % (f.of, a)
        elif f.kind in ("string", "bytes"):
            o += "        h += %s.Length;" % a
        elif f.explicit:
            o += "        if (m.Has%s) h += %s;" % (N.pascal(f.name), _touch_scalar(f.kind, a))
        else:
            o += "        h += %s;" % _touch_scalar(f.kind, a)
    for oname, members in m.oneofs.items():
        o += "        switch (m.%sCase)" % N.pascal(oname)
        o += "        {"
        for g in members:
            a = "m." + N.field(g.name)
            if g.kind == "message":
                v = "G_%s(%s)" % (g.of, a)
            elif g.kind in ("string", "bytes"):
                v = "%s.Length" % a
            else:
                v = _touch_scalar(g.kind, a)
            o += "            case %s.%s.%sOneofCase.%s: h += %s; break;" % (GP, m.name, N.pascal(oname), N.pascal(g.name), v)
        o += "        }"
    o += "        return h;"
    o += "    }"


OPS_BASE = r'''
/// One root's calls, per arm. `F`/`G` hold the built graphs (null for a corpus row, which
/// is only decoded and re-encoded). Every method returns something the caller sinks.
public abstract unsafe class RootOps
{
    public abstract string Root { get; }
    /// The incumbent's own encoding of the built graph (the payload's wire bytes).
    public abstract byte[] IncumbentBytes();
    public abstract int EncIncProd(BufWriter w);
    public abstract int EncIncBest(BufWriter w);
    public abstract int EncHost(ref Enc e, bool retain);
    public abstract int EncFfi(bool retain);
    public abstract byte[] EncFfiBytes(bool retain);
    /// CAMPAIGN req 11, end state (ii): the form each arm's gRPC path hands to Grpc.Net, built
    /// by the SAME serializer the RPC grid's marshaller runs (Ser* below) into a GrpcFrame.
    public abstract int EncIncTransport(GrpcFrame c);
    public abstract int EncHostTransport(bool retain, GrpcFrame c);
    public abstract int EncFfiTransport(bool retain, GrpcFrame c);
    public abstract long DecIncProd(ReadOnlySequence<byte> seq, bool read);
    public abstract long DecIncBest(byte[] b, int len, bool read);
    public abstract long DecHost(byte[] b, int len, bool retain, bool read);
    public abstract long DecFfi(byte[] b, int len, bool retain, bool read);
    public abstract long DecFfiPull(byte[] b, int len, bool read);
    /// Decode then re-encode (the unknown-field rows, FIX-PLAN WP3 item 21).
    public abstract int RtIncProd(ReadOnlySequence<byte> seq, BufWriter w);
    public abstract byte[] RtHost(byte[] b, int len, bool retain);
    public abstract byte[] RtFfi(byte[] b, int len, bool retain);
    public abstract byte[] RtIncBytes(byte[] b);
    /// CAMPAIGN req 7 (R-H27): ops over graphs the named arm decoded from `b` (untimed), so
    /// a corpus row can be ENCODED: `how` 0 = the incumbent's parse, 1/2 = host-gen drop/retain,
    /// 3/4 = core-ffi drop/retain. A retaining decode keeps the row's unknown fields.
    public abstract RootOps FromWire(byte[] b, int how);
    /// CAMPAIGN req 11, input: a pool of distinct graphs (`fs` facade, `gs` incumbent; null
    /// keeps the one graph). Next() moves to the next graph, round robin; a hot input is a
    /// pool of one, so every encode row runs the same Next().
    public abstract void SetPool(object[] fs, object[] gs);
    public abstract void Next();
    /// The counting run (CAMPAIGN req 19): this root's core-ffi host tally.
    public abstract long FfiReverse();
    public abstract void FfiCallsReset();
    public abstract long FfiResets();
}
'''


def _ops(o, root):
    g = "%s.%s" % (GP, root)
    o += "public sealed unsafe class Ops_%s : RootOps" % root
    o += "{"
    o += "    private %s _f;" % root
    o += "    private %s _g;" % g
    o += "    private readonly CoreFfi_%s _c = new CoreFfi_%s();" % (root, root)
    o += "    public Ops_%s(%s f, %s g) { _f = f; _g = g; }" % (root, root, g)
    o += "    public override string Root => \"%s\";" % root
    o += "    public override byte[] IncumbentBytes() => _g.ToByteArray();"
    o += "    public override int EncIncProd(BufWriter w) { int n = _g.CalculateSize(); w.Reset(); _g.WriteTo(w); return n | w.WrittenCount; }"
    o += "    public override int EncIncBest(BufWriter w) { w.Reset(); _g.WriteTo(w); return w.WrittenCount; }"
    # R-H11: host-gen drop is the drop codec (`Codec`), host-gen retain the retain codec
    # (`CodecRetain`, absent from the no-unknown build).
    o += "    public override int EncHost(ref Enc e, bool retain) { e.Reset(); if (retain) HostR.Write%s(ref e, _f); else Codec.Write%s(ref e, _f); if (e.Err != 0) throw new InvalidOperationException(\"managed encode \" + e.Err); return e.Pos; }" % (root, root)
    o += "    public override int EncFfi(bool retain) { int rc = _c.TryEncode(_f, retain, out byte* p, out int n); if (rc < 0) throw new InvalidOperationException(\"core encode \" + rc); return n; }"
    o += "    public override byte[] EncFfiBytes(bool retain) => _c.EncodeToArray(_f, retain);"
    # CAMPAIGN req 11 end state (ii): the serializers the RPC grid's marshallers run (Ser*),
    # static so the grid calls exactly these.
    o += "    public static void SerInc(%s m, SerializationContext c) { c.SetPayloadLength(m.CalculateSize()); m.WriteTo(c.GetBufferWriter()); c.Complete(); }" % g
    o += "    [ThreadStatic] private static Enc _se;"
    o += "    public static void SerHost(%s m, bool retain, SerializationContext c)" % root
    o += "    {"
    o += "        if (_se.Buf == null) _se = Enc.New(Codec.Sites, 1 << 16);"
    o += "        _se.Reset();"
    o += "        if (retain) HostR.Write%s(ref _se, m); else Codec.Write%s(ref _se, m);" % (root, root)
    o += "        if (_se.Err != 0) throw new InvalidOperationException(\"managed encode \" + _se.Err);"
    o += "        int n = _se.Pos;"
    o += "        c.SetPayloadLength(n);"
    o += "        var w = c.GetBufferWriter();"
    o += "        new ReadOnlySpan<byte>(_se.Buf, 0, n).CopyTo(w.GetSpan(n));"
    o += "        w.Advance(n);"
    o += "        c.Complete();"
    o += "    }"
    o += "    public static void SerFfi(CoreFfi_%s core, %s m, bool retain, SerializationContext c)" % (root, root)
    o += "    {"
    o += "        int rc = core.TryEncode(m, retain, out byte* p, out int n);"
    o += "        if (rc < 0) throw new InvalidOperationException(\"core encode \" + rc);"
    o += "        c.SetPayloadLength(n);"
    o += "        var w = c.GetBufferWriter();"
    o += "        new ReadOnlySpan<byte>(p, n).CopyTo(w.GetSpan(n));"
    o += "        w.Advance(n);"
    o += "        c.Complete();"
    o += "    }"
    o += "    public static readonly Marshaller<%s> MInc = Marshallers.Create<%s>(SerInc, c => throw new NotSupportedException());" % (g, g)
    o += "    public static readonly Marshaller<%s> MHostDrop = Marshallers.Create<%s>((m, c) => SerHost(m, false, c), c => throw new NotSupportedException());" % (root, root)
    o += "    public static readonly Marshaller<%s> MHostRetain = Marshallers.Create<%s>((m, c) => SerHost(m, true, c), c => throw new NotSupportedException());" % (root, root)
    o += "    public override int EncIncTransport(GrpcFrame c) { MInc.ContextualSerializer(_g, c); int n = c.WrittenCount; c.Release(); return n; }"
    o += "    public override int EncHostTransport(bool retain, GrpcFrame c) { (retain ? MHostRetain : MHostDrop).ContextualSerializer(_f, c); int n = c.WrittenCount; c.Release(); return n; }"
    o += "    private Marshaller<%s> _mfd, _mfr;" % root
    o += "    public override int EncFfiTransport(bool retain, GrpcFrame c)"
    o += "    {"
    o += "        var mm = retain ? (_mfr ??= Marshallers.Create<%s>((m, x) => SerFfi(_c, m, true, x), x => throw new NotSupportedException()))" % root
    o += "                        : (_mfd ??= Marshallers.Create<%s>((m, x) => SerFfi(_c, m, false, x), x => throw new NotSupportedException()));" % root
    o += "        mm.ContextualSerializer(_f, c); int n = c.WrittenCount; c.Release(); return n;"
    o += "    }"
    o += "    public override RootOps FromWire(byte[] b, int how)"
    o += "    {"
    o += "        switch (how)"
    o += "        {"
    o += "            case 0: return new Ops_%s(null, %s.Parser.ParseFrom(b));" % (root, g)
    o += "            case 1: case 2:"
    o += "            {"
    o += "                var d = new Dec { Buf = b, Pos = 0, End = b.Length, Err = 0 };"
    o += "                var m = new %s();" % root
    o += "                if (how == 2) HostR.Read%s(ref d, m, 0); else Codec.Read%s(ref d, m, 0);" % (root, root)
    o += "                if (d.Err != 0) throw new InvalidOperationException(\"managed decode \" + d.Err);"
    o += "                return new Ops_%s(m, null);" % root
    o += "            }"
    o += "            default:"
    o += "            {"
    o += "                int rc = _c.TryDecode(b, b.Length, how == 4, out var m);"
    o += "                if (rc < 0) throw new InvalidOperationException(\"core decode \" + rc);"
    o += "                return new Ops_%s(m, null);" % root
    o += "            }"
    o += "        }"
    o += "    }"
    o += "    private %s[] _fs;" % root
    o += "    private %s[] _gs;" % g
    o += "    private int _i, _n = 1;"
    o += "    public override void SetPool(object[] fs, object[] gs)"
    o += "    {"
    o += "        _fs = fs == null ? null : Array.ConvertAll(fs, x => (%s)x);" % root
    o += "        _gs = gs == null ? null : Array.ConvertAll(gs, x => (%s)x);" % g
    o += "        _n = Math.Max(_fs?.Length ?? 1, _gs?.Length ?? 1);"
    o += "        _i = 0;"
    o += "        if (_fs != null) _f = _fs[0];"
    o += "        if (_gs != null) _g = _gs[0];"
    o += "    }"
    o += "    public override void Next()"
    o += "    {"
    o += "        if (++_i >= _n) _i = 0;"
    o += "        if (_fs != null) _f = _fs[_i];"
    o += "        if (_gs != null) _g = _gs[_i];"
    o += "    }"
    o += "    public override long FfiReverse() => _c.ReverseCalls;"
    o += "    public override void FfiCallsReset() => _c.CallsReset();"
    o += "    public override long FfiResets() => _c.ResetCalls;"
    o += "    public override long DecIncProd(ReadOnlySequence<byte> seq, bool read) { var m = %s.Parser.ParseFrom(seq); return read ? Touch.G_%s(m) : 1; }" % (g, root)
    o += "    public override long DecIncBest(byte[] b, int len, bool read) { var m = %s.Parser.ParseFrom(new ReadOnlySpan<byte>(b, 0, len)); return read ? Touch.G_%s(m) : 1; }" % (g, root)
    o += "    public override long DecHost(byte[] b, int len, bool retain, bool read)"
    o += "    {"
    o += "        var d = new Dec { Buf = b, Pos = 0, End = len, Err = 0 };"
    o += "        var m = new %s();" % root
    o += "        if (retain) HostR.Read%s(ref d, m, 0); else Codec.Read%s(ref d, m, 0);" % (root, root)
    o += "        if (d.Err != 0) throw new InvalidOperationException(\"managed decode \" + d.Err);"
    o += "        return read ? Touch.F_%s(m) : 1;" % root
    o += "    }"
    o += "    public override long DecFfi(byte[] b, int len, bool retain, bool read)"
    o += "    {"
    o += "        int rc = _c.TryDecode(b, len, retain, out var m);"
    o += "        if (rc < 0) throw new InvalidOperationException(\"core decode \" + rc);"
    o += "        return read ? Touch.F_%s(m) : 1;" % root
    o += "    }"
    o += "    public override long DecFfiPull(byte[] b, int len, bool read) { var m = _c.Pull(b, len); return read ? Touch.F_%s(m) : 1; }" % root
    o += "    public override int RtIncProd(ReadOnlySequence<byte> seq, BufWriter w) { var m = %s.Parser.ParseFrom(seq); int n = m.CalculateSize(); w.Reset(); m.WriteTo(w); return n | w.WrittenCount; }" % g
    o += "    public override byte[] RtIncBytes(byte[] b) => %s.Parser.ParseFrom(b).ToByteArray();" % g
    o += "    public override byte[] RtHost(byte[] b, int len, bool retain)"
    o += "    {"
    o += "        var d = new Dec { Buf = b, Pos = 0, End = len, Err = 0 };"
    o += "        var m = new %s();" % root
    o += "        if (retain) HostR.Read%s(ref d, m, 0); else Codec.Read%s(ref d, m, 0);" % (root, root)
    o += "        if (d.Err != 0) throw new InvalidOperationException(\"managed decode \" + d.Err);"
    o += "        var e = Enc.New(Codec.Sites, len + 4096);"
    o += "        if (retain) HostR.Write%s(ref e, m); else Codec.Write%s(ref e, m);" % (root, root)
    o += "        return e.ToArray();"
    o += "    }"
    o += "    public override byte[] RtFfi(byte[] b, int len, bool retain)"
    o += "    {"
    o += "        int rc = _c.TryDecode(b, len, retain, out var m);"
    o += "        if (rc < 0) throw new InvalidOperationException(\"core decode \" + rc);"
    o += "        return _c.EncodeToArray(m, retain);"
    o += "    }"
    o += "}"
    o += ""


def emit(p, payloads):
    o = Head("The campaign runner's per-root calls and field visitors.", "cs_campaign")
    o += "using System;"
    o += "using System.Buffers;"
    o += "using Armonik.Ffi.Facade;"
    o += "using Armonik.Ffi.Harness;"
    o += "using Google.Protobuf;"
    o += "using Grpc.Core;"
    o += "using Gp = Armonik.Ffi.Shapes.V1;"
    o += ""
    o += "namespace Armonik.Ffi.Campaign;"
    o += ""
    o += "/// Requirement 9's second decode: every field of the decoded graph read, per object model."
    o += "public static class Touch"
    o += "{"
    for m in facade_messages(p):
        _touch_facade(o, p, m)
        _touch_gp(o, p, m)
    o += "}"
    for ln in OPS_BASE.strip("\n").split("\n"):
        o += ln
    o += ""
    # R-H11: the retain codec of host-gen, absent from the no-unknown build (refused there).
    o += "#if AK_NO_UNKNOWN_FIELDS"
    o += "internal static class HostR"
    o += "{"
    for r in p.roots:
        o += "    public static void Read%s(ref Dec d, %s m, int depth) => throw new NotSupportedException(\"unknown fields are compiled out of this build\");" % (r, r)
        o += "    public static void Write%s(ref Enc e, %s m) => throw new NotSupportedException(\"unknown fields are compiled out of this build\");" % (r, r)
    o += "}"
    o += "#else"
    o += "internal static class HostR"
    o += "{"
    for r in p.roots:
        o += "    public static void Read%s(ref Dec d, %s m, int depth) => CodecRetain.Read%s(ref d, m, depth);" % (r, r, r)
        o += "    public static void Write%s(ref Enc e, %s m) => CodecRetain.Write%s(ref e, m);" % (r, r, r)
    o += "}"
    o += "#endif"
    o += ""
    for r in p.roots:
        _ops(o, r)
    o += "public static class OpsTable"
    o += "{"
    o += "    public static readonly string[] Payloads = { %s };" % ", ".join('"%s"' % pid for pid, _ in payloads)
    o += "    public static string RootOf(string id) => id switch { %s, _ => throw new ArgumentException(id) };" % ", ".join(
        '"%s" => "%s"' % (pid, r) for pid, r in payloads)
    o += "    /// A payload's ops over freshly built graphs (the current Values.ContentSet)."
    o += "    public static RootOps ForPayload(string id) => id switch"
    o += "    {"
    for pid, r in payloads:
        f = pid.replace(".", "_")
        o += "        \"%s\" => new Ops_%s(BuildFacade.%s(), BuildGp.%s())," % (pid, r, f, f)
    o += "        _ => throw new ArgumentException(id),"
    o += "    };"
    o += "    /// CAMPAIGN req 11's pool input: one fresh graph of a payload per call (the current"
    o += "    /// Values.ContentSet), facade (host-gen, core-ffi) or incumbent object model."
    o += "    public static object BuildF(string id) => id switch"
    o += "    {"
    for pid, r in payloads:
        o += "        \"%s\" => BuildFacade.%s()," % (pid, pid.replace(".", "_"))
    o += "        _ => throw new ArgumentException(id),"
    o += "    };"
    o += "    public static object BuildG(string id) => id switch"
    o += "    {"
    for pid, r in payloads:
        o += "        \"%s\" => BuildGp.%s()," % (pid, pid.replace(".", "_"))
    o += "        _ => throw new ArgumentException(id),"
    o += "    };"
    o += "    /// A root's ops for bytes only (a corpus row: decode, and decode then re-encode)."
    o += "    public static RootOps ForRoot(string root) => root switch"
    o += "    {"
    for r in p.roots:
        o += "        \"%s\" => new Ops_%s(null, null)," % (r, r)
    o += "        _ => null,"
    o += "    };"
    o += "}"
    return str(o)
