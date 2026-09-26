"""Backend: the corpus PROJECTION, emitted from the same walker as the codec.

`ffi/corpus/CONTRACT.md` C2 is the obligation byte identity does not imply: a
codec can round-trip bytes it has misunderstood, so the decoded message is
compared field by field against a JSON the corpus computed with an unrelated
implementation. Section 3 defines the encoding, and the part that matters is
that it is **set fields**, not proto3 JSON: proto3 JSON omits a default and so
erases the difference between absent and present-and-zero, which is the whole of
vector class `empty`.

Emitted rather than written for the same reason `Eq` is. A hand-written
projector can quietly not look at a field, and then C2 passes while testing 26
of 27. This one walks `walk()`, so a field the codec handles and the projector
does not cannot exist.

The rules, from section 3:

  * an implicit-presence scalar equal to its default DOES NOT APPEAR;
  * an explicit-presence field that is set DOES appear, zero or not;
  * a message field that is present appears, empty body or not;
  * a repeated or map field appears when it has at least one element.

and every integer and every enum is a DECIMAL STRING, because an enum value the
descriptor does not declare has no name to use and `"999"` is the point of
several vectors.
"""
from glue import Head
import cs_names as N
from cs_types import facade_messages

NS = "Armonik.Ffi.Corpus"


def emit(ir):
    o = Head("The corpus projection (CONTRACT.md C2 and section 3), from the same plan as the codec.", "cs_proj", ir.source)
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using System.Globalization;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace %s;" % NS
    o += ""
    o.doc("A projected value: a `SortedDictionary` for a message, a `List` for a "
          "repeated field, and a `string` or `bool` for everything else. Sorted so "
          "that a dump is stable; the comparison itself is structural and does not "
          "depend on it.")
    o += "public static class Proj"
    o += "{"
    o += "    private static string Hex(byte[] b)"
    o += "    {"
    o += "        if (b == null || b.Length == 0) return \"\";"
    o += "        var c = new char[b.Length * 2];"
    o += "        const string H = \"0123456789abcdef\";"
    o += "        for (int i = 0; i < b.Length; i++) { c[2 * i] = H[b[i] >> 4]; c[2 * i + 1] = H[b[i] & 15]; }"
    o += "        return new string(c);"
    o += "    }"
    o += ""
    o.doc("`%.17g`, which is what the corpus wrote. C#'s \"G17\" agrees with it on "
          "every finite value the corpus carries; the runner compares numerically "
          "when the two spellings differ, and says so when it has to.", "    ")
    o += "    private static string Dbl(double d)"
    o += "    {"
    o += "        // C's %.17g spells these three differently from C#'s G17, and the"
    o += "        // corpus was written by a C implementation."
    o += "        if (double.IsNaN(d)) return \"nan\";"
    o += "        if (double.IsPositiveInfinity(d)) return \"inf\";"
    o += "        if (double.IsNegativeInfinity(d)) return \"-inf\";"
    o += "        // And minus zero: G17 gives \"-0\" on .NET Core 3.0+ and \"0\" on the"
    o += "        // net48 floor, which is a runtime difference and not a value one."
    o += "        if (d == 0.0) return BitConverter.DoubleToInt64Bits(d) != 0L ? \"-0\" : \"0\";"
    o += "        return d.ToString(\"G17\", CultureInfo.InvariantCulture);"
    o += "    }"
    o += ""
    for m in facade_messages(ir):
        emit_one(o, ir, m)
    o.doc("Project a message by ROOT NAME, which is how the manifest names it.", "    ")
    o += "    public static object ByRoot(string root, object msg)"
    o += "    {"
    o += "        switch (root)"
    o += "        {"
    for m in facade_messages(ir):
        o += "            case \"%s\": return Of%s((%s)msg);" % (m.name, m.name, m.name)
    o += "            default: throw new ArgumentException(\"no projector for root \" + root);"
    o += "        }"
    o += "    }"
    o += "}"
    o += ""
    o.doc("Dispatch by root NAME, because that is how the manifest names one. "
          "Generated for the same reason everything else here is: a hand-written "
          "switch over thirty roots is a place for one of them to be missing, and a "
          "missing root would read as a vector the slice \"cannot run\".")
    o += "#if AK_NO_UNKNOWN_FIELDS"
    o += "/// The no-unknown build has no retain codec: asking for it is refused."
    o += "internal static class RetainCodec"
    o += "{"
    for m in facade_messages(ir):
        o += "    public static void Read%s(ref Dec d, %s m, int depth) => throw new NotSupportedException(\"unknown fields are compiled out of this build\");" % (m.name, m.name)
        o += "    public static void Write%s(ref Enc e, %s m) => throw new NotSupportedException(\"unknown fields are compiled out of this build\");" % (m.name, m.name)
        o += "    public static void WriteSized%s(ref Enc e, %s m) => throw new NotSupportedException(\"unknown fields are compiled out of this build\");" % (m.name, m.name)
    o += "}"
    o += "#else"
    o += "internal static class RetainCodec"
    o += "{"
    for m in facade_messages(ir):
        o += "    public static void Read%s(ref Dec d, %s m, int depth) => CodecRetain.Read%s(ref d, m, depth);" % (m.name, m.name, m.name)
        o += "    public static void Write%s(ref Enc e, %s m) => CodecRetain.Write%s(ref e, m);" % (m.name, m.name, m.name)
        o += "    public static void WriteSized%s(ref Enc e, %s m) => CodecRetain.WriteSized%s(ref e, m);" % (m.name, m.name, m.name)
    o += "}"
    o += "#endif"
    o += ""
    o += "public static class Roots"
    o += "{"
    o += "    public static readonly string[] All = { %s };" % ", ".join(
        '"%s"' % m.name for m in facade_messages(ir))
    o += ""
    o += "    public static object New(string root)"
    o += "    {"
    o += "        switch (root)"
    o += "        {"
    for m in facade_messages(ir):
        o += "            case \"%s\": return new %s();" % (m.name, m.name)
    o += "            default: throw new ArgumentException(\"unknown root \" + root);"
    o += "        }"
    o += "    }"
    o += ""
    o += "    /// `retain`: the retain codec (CodecRetain, R-H11); refused in the no-unknown build."
    o += "    public static void Read(string root, ref Dec d, object m, bool retain = false)"
    o += "    {"
    o += "        switch (root)"
    o += "        {"
    for m in facade_messages(ir):
        o += "            case \"%s\": if (retain) RetainCodec.Read%s(ref d, (%s)m, 0); else Codec.Read%s(ref d, (%s)m, 0); break;" % (m.name, m.name, m.name, m.name, m.name)
    o += "            default: throw new ArgumentException(\"unknown root \" + root);"
    o += "        }"
    o += "    }"
    o += ""
    o += "    /// `retain`: the retain codec (CodecRetain, R-H11); refused in the no-unknown build."
    o += "    public static void Write(string root, ref Enc e, object m, bool retain = false)"
    o += "    {"
    o += "        switch (root)"
    o += "        {"
    for m in facade_messages(ir):
        o += "            case \"%s\": if (retain) RetainCodec.Write%s(ref e, (%s)m); else Codec.Write%s(ref e, (%s)m); break;" % (m.name, m.name, m.name, m.name, m.name)
    o += "            default: throw new ArgumentException(\"unknown root \" + root);"
    o += "        }"
    o += "    }"
    o += ""
    o.doc("The two-pass encoder, so C3 can say which encode path produced the "
          "bytes it is claiming. Both must land on an accepted form.", "    ")
    o += "    /// `retain`: the retain codec (CodecRetain, R-H11); refused in the no-unknown build."
    o += "    public static void WriteSized(string root, ref Enc e, object m, bool retain = false)"
    o += "    {"
    o += "        switch (root)"
    o += "        {"
    for m in facade_messages(ir):
        o += "            case \"%s\": if (retain) RetainCodec.WriteSized%s(ref e, (%s)m); else Codec.WriteSized%s(ref e, (%s)m); break;" % (m.name, m.name, m.name, m.name, m.name)
    o += "            default: throw new ArgumentException(\"unknown root \" + root);"
    o += "        }"
    o += "    }"
    o += "}"
    return str(o)


def scalar(f, e):
    """One value, in the encoding of CONTRACT.md section 3."""
    if f.kind == "string":
        return e
    if f.kind == "bytes":
        return "Hex(%s)" % e
    if f.kind == "bool":
        return "(object)(%s)" % e                      # a JSON literal, not a string
    if f.kind == "double":
        return "Dbl(%s)" % e
    if f.kind == "enum":
        # A decimal string of the NUMBER: an undeclared wire value has no name.
        return "((int)%s).ToString(CultureInfo.InvariantCulture)" % e
    if f.kind == "fixed32":
        return "((uint)%s).ToString(CultureInfo.InvariantCulture)" % e
    if f.kind == "int32":
        return "((int)%s).ToString(CultureInfo.InvariantCulture)" % e
    if f.kind == "int64":
        return "((long)%s).ToString(CultureInfo.InvariantCulture)" % e
    raise KeyError("no projection case for %s.%s (%s)" % (f.owner, f.name, f.kind))


def zero_test(f, acc):
    if f.kind == "bool":
        return acc
    if f.kind == "double":
        # Bits, not value: a minus zero IS a set field. Same rule as the codec.
        return "BitConverter.DoubleToInt64Bits(%s) != 0L" % acc
    if f.kind == "string":
        return "!string.IsNullOrEmpty(%s)" % acc
    if f.kind == "bytes":
        return "%s != null && %s.Length != 0" % (acc, acc)
    if f.kind == "fixed32":
        return "%s != 0u" % acc
    return "%s != 0" % acc


def emit_one(o, ir, m):
    p = "        "
    o += "    public static SortedDictionary<string, object> Of%s(%s m)" % (m.name, m.name)
    o += "    {"
    o += "        var o = new SortedDictionary<string, object>(StringComparer.Ordinal);"
    o += "        if (m == null) return o;"
    for f in m.fields:
        if f.oneof:
            continue
        acc = "m." + N.field(f.name)
        if f.card == "map":
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            o += "%s    var mp = new SortedDictionary<string, object>(StringComparer.Ordinal);" % p
            o += "%s    for (int i = 0; i < %s.Count; i++) { var kv = %s.At(i); mp[kv.Key] = kv.Value; }" % (p, acc, acc)
            o += "%s    o[\"%s\"] = mp;" % (p, f.name)
            o += "%s}" % p
        elif f.card in ("repeated", "packed"):
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            o += "%s    var a = new List<object>(%s.Count);" % (p, acc)
            if f.kind == "message":
                o += "%s    for (int i = 0; i < %s.Count; i++) a.Add(Of%s(%s[i]));" % (p, acc, f.of, acc)
            else:
                o += "%s    for (int i = 0; i < %s.Count; i++) a.Add(%s);" % (
                    p, acc, scalar(f, "%s[i]" % acc))
            o += "%s    o[\"%s\"] = a;" % (p, f.name)
            o += "%s}" % p
        elif f.kind == "message":
            # Present or absent, never "empty means absent": an empty submessage
            # is a set field and `E-*` turns on exactly that.
            o += "%sif (%s != null) o[\"%s\"] = Of%s(%s);" % (p, acc, f.name, f.of, acc)
        elif f.explicit:
            # A nullable value type for a scalar, a nullable reference for a
            # string or bytes -- the facade's own spelling, which the codec's
            # explicit-presence branch already follows.
            if f.kind in ("string", "bytes"):
                o += "%sif (%s != null) o[\"%s\"] = %s;" % (p, acc, f.name, scalar(f, acc))
            else:
                o += "%sif (%s.HasValue) o[\"%s\"] = %s;" % (
                    p, acc, f.name, scalar(f, acc + ".Value"))
        else:
            o += "%sif (%s) o[\"%s\"] = %s;" % (p, zero_test(f, acc), f.name, scalar(f, acc))
    for oname, members in m.oneofs.items():
        ct = N.oneof_case_type(m.name, oname)
        o += "%sswitch (m.%sCase)" % (p, N.pascal(oname))
        o += "%s{" % p
        for f in members:
            acc = "m." + N.field(f.name)
            o += "%s    case %s.%s:" % (p, ct, N.pascal(f.name))
            if f.kind == "message":
                o += "%s        o[\"%s\"] = Of%s(%s); break;" % (p, f.name, f.of, acc)
            else:
                # A selected oneof member is present whatever its value.
                o += "%s        o[\"%s\"] = %s; break;" % (p, f.name, scalar(f, acc))
        o += "%s    default: break;" % p
        o += "%s}" % p
    o += "        return o;"
    o += "    }"
    o += ""
