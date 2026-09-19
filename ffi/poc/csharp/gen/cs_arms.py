"""Backend: the per-payload arm table.

Every payload is a different root type, so the harness needs one dispatch per
payload. Emitting it keeps the property R1 exists for: an arm that has no case
for a payload is a generator error, not a row quietly missing from a table.

The arms, and exactly what each one calls:

  gp-tobytearray   `msg.ToByteArray()`   -- what application code writes
  gp-writeto       `msg.CalculateSize()` then `msg.WriteTo(Span<byte>)` into a
                   reused buffer -- the fair encode baseline, no allocation
  managed          `Codec.Write` -- one pass, learned length width
  managed-2pass    `Codec.SizeOf` then `Codec.WriteSized` -- the two-pass shape
                   Google.Protobuf uses, so the difference between it and
                   `managed` is the single-pass term alone, within one arm
  memcpy           `Buffer.BlockCopy` of the payload's own bytes -- R2's floor:
                   no encoder can go below one copy of its output

  gp-parse         `Parser.ParseFrom(ReadOnlySpan<byte>)`
  managed-parse    `Codec.Read` into a fresh facade graph
  memcpy-parse     the same floor for decode
"""
import csnames as N
from cs_facade import Head

GP = "Armonik.Ffi.Shapes.V1"


def emit(ir):
    o = Head("The per-payload arm table: one dispatch per payload, emitted.")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using Google.Protobuf;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""
    o += "public abstract class Arms"
    o += "{"
    o += "    public string Id;"
    o += "    public string Shape;"
    o += "    public string Root;"
    o += "    public int Elements;"
    o += ""
    o += "    /// Both object graphs, built once, from the same value rules by two"
    o += "    /// emitted routes that share no code. Two routes disagreeing is what"
    o += "    /// caught the Rust slice's D15."
    o += "    public abstract void Build();"
    o += ""
    o += "    public abstract byte[] GpToByteArray();"
    o += "    public abstract int GpWriteTo(byte[] dst);"
    o += "    public abstract void ManagedWrite(ref Enc e);"
    o += "    public abstract void ManagedWriteSized(ref Enc e);"
    o += ""
    o += "    public abstract int GpParse(byte[] src, int len);"
    o += "    public abstract int ManagedParse(byte[] src, int len);"
    o += ""
    o += "    /// Decode `src`, re-encode what came back, and hand the bytes over."
    o += "    /// This is the decode half of the oracle: byte identity only covers"
    o += "    /// the encode direction."
    o += "    public abstract byte[] ManagedRoundTrip(byte[] src, int len);"
    o += "    public abstract byte[] GpRoundTrip(byte[] src, int len);"
    o += ""
    o += "    /// Decode `src` into a facade graph and compare it, field by field,"
    o += "    /// with the graph the builder produced. Emitted from the same walker"
    o += "    /// as the codec, so a field the codec handles and the comparer does"
    o += "    /// not cannot exist."
    o += "    public abstract bool ManagedDecodesToBuiltValue(byte[] src, int len);"
    o += "}"
    o += ""

    names = []
    for pid, spec in ir.schema["payloads"].items():
        root = ir.msg(spec["root"])
        cls = "Arms_" + pid.replace(".", "_")
        names.append((pid, cls))
        shape = shape_of(ir, spec)
        o += "public sealed class %s : Arms" % cls
        o += "{"
        o += "    private %s _fac;" % root.cs
        o += "    private %s.%s _gp;" % (GP, root.cs)
        o += ""
        o += "    public %s()" % cls
        o += "    {"
        o += '        Id = "%s"; Shape = "%s"; Root = "%s"; Elements = %d;' % (
            pid, shape, root.cs, elements_of(ir, spec))
        o += "    }"
        o += ""
        o += "    public override void Build()"
        o += "    {"
        o += "        _fac = Build%s.P%s();" % ("Facade", pid[1:].replace(".", "_"))
        o += "        _gp = BuildGp.P%s();" % pid[1:].replace(".", "_")
        o += "    }"
        o += ""
        o += "    public override byte[] GpToByteArray() => _gp.ToByteArray();"
        o += ""
        o += "    public override int GpWriteTo(byte[] dst)"
        o += "    {"
        o += "        int n = _gp.CalculateSize();"
        o += "        _gp.WriteTo(new Span<byte>(dst, 0, n));"
        o += "        return n;"
        o += "    }"
        o += ""
        o += "    public override void ManagedWrite(ref Enc e) => Codec.Write%s(ref e, _fac);" % root.cs
        o += ""
        o += "    public override void ManagedWriteSized(ref Enc e) => Codec.WriteSized%s(ref e, _fac);" % root.cs
        o += ""
        o += "    public override int GpParse(byte[] src, int len)"
        o += "    {"
        o += "        var m = %s.%s.Parser.ParseFrom(new ReadOnlySpan<byte>(src, 0, len));" % (GP, root.cs)
        o += "        return m.CalculateSize();"
        o += "    }"
        o += ""
        o += "    public override int ManagedParse(byte[] src, int len)"
        o += "    {"
        o += "        var d = new Dec { Buf = src, Pos = 0, End = len, Err = 0 };"
        o += "        var m = new %s();" % root.cs
        o += "        Codec.Read%s(ref d, m, len);" % root.cs
        o += "        if (d.Err != 0) throw new InvalidOperationException(Id + \": managed decode failed, err \" + d.Err);"
        o += "        return d.Pos;"
        o += "    }"
        o += ""
        o += "    public override byte[] ManagedRoundTrip(byte[] src, int len)"
        o += "    {"
        o += "        var d = new Dec { Buf = src, Pos = 0, End = len, Err = 0 };"
        o += "        var m = new %s();" % root.cs
        o += "        Codec.Read%s(ref d, m, len);" % root.cs
        o += "        if (d.Err != 0) throw new InvalidOperationException(Id + \": managed decode failed, err \" + d.Err);"
        o += "        var e = Enc.New(Codec.Sites, Math.Max(4096, len + 64));"
        o += "        Codec.Write%s(ref e, m);" % root.cs
        o += "        return e.ToArray();"
        o += "    }"
        o += ""
        o += "    public override byte[] GpRoundTrip(byte[] src, int len)"
        o += "    {"
        o += "        var m = %s.%s.Parser.ParseFrom(new ReadOnlySpan<byte>(src, 0, len));" % (GP, root.cs)
        o += "        return m.ToByteArray();"
        o += "    }"
        o += ""
        o += "    public override bool ManagedDecodesToBuiltValue(byte[] src, int len)"
        o += "    {"
        o += "        var d = new Dec { Buf = src, Pos = 0, End = len, Err = 0 };"
        o += "        var m = new %s();" % root.cs
        o += "        Codec.Read%s(ref d, m, len);" % root.cs
        o += "        if (d.Err != 0) return false;"
        o += "        return Eq.Same%s(m, _fac);" % root.cs
        o += "    }"
        o += "}"
        o += ""

    o += "public static class ArmTable"
    o += "{"
    o += "    public static Arms[] All() => new Arms[]"
    o += "    {"
    for pid, cls in names:
        o += "        new %s()," % cls
    o += "    };"
    o += "}"
    return str(o)


def shape_of(ir, spec):
    root = ir.msg(spec["root"])
    for f in root.walk():
        if f.kind == "message":
            sh = ir.msg(f.of).raw.get("shape")
            if sh:
                return sh
    return "-"


def elements_of(ir, spec):
    if "bulk" in spec:
        return 1
    if spec.get("interleaved"):
        return spec["count"] * 2
    return spec["count"]
