"""Backend: the facade types, and a generated structural comparer for them.

The facade is the ONE object model every arm in this slice operates on, which
is what makes the arms comparable at all: the incumbent's own generated classes
are a second object model, built independently from the same value rules, and
the two construction routes disagreeing is a defect (it is what caught the Rust
slice's D15).

Two facade decisions are load bearing and are argued rather than assumed.

**Public fields, not auto-properties.** An auto-property inlines to the same
code at tier 1, so this buys nothing at the target -- but it removes a question
a reader would otherwise have to take on trust, and on the netstandard2.0 floor
under Mono there is no such guarantee at all.

**The map is insertion ordered, and mirrors `MapField`'s structure.** The
canonical form sorts map entries by key, and `Google.Protobuf`'s `MapField`
writes in insertion order with no deterministic-serialization option on .NET
(unlike the C++ and Java runtimes), so canonical bytes come from the HOST
inserting sorted. A facade backed by `SortedDictionary` would then be paying
red-black inserts on decode while the incumbent pays hash inserts, and the
managed decode column -- the single most valuable number in this slice --
would be a comparison of container types. `OrderedMap` is a `Dictionary` plus a
`List`, which is what `MapField` is.
"""
import ir as IR

CS = {"int32": "int", "int64": "long", "bool": "bool", "double": "double"}


def scalar_cs(kind):
    return CS[kind]


def field_type(f):
    """The facade type of one field."""
    if f.card == "map":
        return "OrderedMap<string, string>"
    if f.card == "packed":
        return "List<%s>" % (f.of if f.kind == "enum" else scalar_cs(f.kind))
    if f.card == "repeated":
        return "List<%s>" % ("string" if f.kind == "string" else f.of)
    # singular
    if f.kind == "string":
        return "string"
    if f.kind == "bytes":
        return "byte[]"
    if f.kind == "enum":
        return "%s?" % f.of if f.presence == "explicit" else f.of
    if f.kind == "message":
        return f.of
    return "%s?" % scalar_cs(f.kind) if f.presence == "explicit" else scalar_cs(f.kind)


def field_init(f):
    """The initialiser, or None for a field whose default is `default`.

    Absence is null for a singular message, for an explicit-presence field and
    for `bytes` under explicit presence; everything else starts at the proto
    zero, which is what makes an implicit-presence field indistinguishable from
    an absent one -- the property design/SHAPES.md asks the explicit-presence
    row to break.
    """
    if f.card == "map":
        return "new OrderedMap<string, string>()"
    if f.card in ("packed", "repeated"):
        return "new %s()" % field_type(f)
    if f.presence == "explicit" or f.kind == "message":
        return None                       # null is absent
    if f.kind == "string":
        return '""'
    if f.kind == "bytes":
        return "EmptyBytes"
    return None                           # 0 / false / the zero enum


def oneof_case_type(m, name):
    import csnames as N
    return "%s%sCase" % (m.cs, N.pascal(name))


def emit_types(ir):
    o = Head("The facade types every arm of this slice operates on.")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += ""
    o += "namespace Armonik.Ffi.Facade;"
    o += ""

    for ename, edef in ir.enums.items():
        import csnames as N
        why = edef.get("why")
        if why:
            o.doc(why)
        o.doc("Open enum: a wire value this build was not generated against is "
              "representable as `(%s)v` and round-trips losslessly, which is what "
              "design/SHAPES.md's open-enum row asks for. A C# enum is open by "
              "construction, so unlike the Rust facade this needs no Unknown case." % ename)
        o += "public enum %s" % ename
        o += "{"
        for vname, v in edef["values"].items():
            o += "    %s = %d," % (N.enum_member(ename, vname), v)
        o += "}"
        o += ""

    for m in ir.messages.values():
        emit_one(o, ir, m)

    return str(o)


def emit_one(o, ir, m):
    import csnames as N
    for oname, members in m.oneofs.items():
        ct = oneof_case_type(m, oname)
        o.doc("Which member of `%s.%s` is set. `None` is not a member: it is the "
              "state design/SHAPES.md says a by-value group cannot reach." % (m.cs, oname))
        o += "public enum %s" % ct
        o += "{"
        o += "    None = 0,"
        for f in members:
            o += "    %s = %d," % (N.pascal(f.name), f.tag)
        o += "}"
        o += ""

    if m.doc:
        o.doc(m.doc)
    o.doc("Batching predicate (ABI v1 6/7.2): this message is %s."
          % ("a LEAF, so a run of them can share one group buffer"
             if m.leaf else "NOT a leaf, so it cannot be batched"))
    o += "public sealed class %s" % m.cs
    o += "{"
    o += "    internal static readonly byte[] EmptyBytes = new byte[0];"
    for f in m.plain():
        init = field_init(f)
        o += "    public %s %s%s;" % (field_type(f), f.cs,
                                      "" if init is None else " = " + init)
    for oname, members in m.oneofs.items():
        ct = oneof_case_type(m, oname)
        o += ""
        o += "    public %s %sCase;" % (ct, N.pascal(oname))
        for f in members:
            o += "    public %s %s;" % (field_type(f), f.cs)
    o += "}"
    o += ""


def emit_eq(ir):
    """A structural comparer, generated.

    Byte identity is the oracle (R2), but it only covers the encode direction.
    Decode is checked by decoding into the facade and comparing VALUES, and a
    hand-written comparer is the same hazard as a hand-written codec: it can
    quietly not look at a field. This one is emitted from the same walker, so a
    field the codec handles and the comparer does not cannot exist.
    """
    o = Head("Structural equality over the facade, for the decode half of the oracle.")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += ""
    o += "namespace Armonik.Ffi.Facade;"
    o += ""
    o += "public static class Eq"
    o += "{"
    o += "    public static bool Bytes(byte[] a, byte[] b)"
    o += "    {"
    o += "        if (ReferenceEquals(a, b)) return true;"
    o += "        if (a == null || b == null) return (a == null ? 0 : a.Length) == (b == null ? 0 : b.Length);"
    o += "        if (a.Length != b.Length) return false;"
    o += "        for (int i = 0; i < a.Length; i++) if (a[i] != b[i]) return false;"
    o += "        return true;"
    o += "    }"
    o += ""
    o += "    public static bool Map(OrderedMap<string, string> a, OrderedMap<string, string> b)"
    o += "    {"
    o += "        if (a.Count != b.Count) return false;"
    o += "        foreach (var kv in a) { if (!b.TryGetValue(kv.Key, out var v) || v != kv.Value) return false; }"
    o += "        return true;"
    o += "    }"
    o += ""
    o += "    public static bool List<T>(List<T> a, List<T> b, Func<T, T, bool> same)"
    o += "    {"
    o += "        if (a.Count != b.Count) return false;"
    o += "        for (int i = 0; i < a.Count; i++) if (!same(a[i], b[i])) return false;"
    o += "        return true;"
    o += "    }"
    for m in ir.messages.values():
        emit_eq_one(o, ir, m)
    o += "}"
    return str(o)


def eq_expr(f, a, b):
    """`a` and `b` are expressions of the field's own type."""
    if f.kind == "bytes":
        return "Eq.Bytes(%s, %s)" % (a, b)
    if f.kind == "message":
        return "Same%s(%s, %s)" % (f.of, a, b)
    if f.kind == "double":
        # Bit equality: a payload may legitimately carry a zero of either sign,
        # and `==` would call those equal while the wire does not.
        return "BitConverter.DoubleToInt64Bits(%s) == BitConverter.DoubleToInt64Bits(%s)" % (a, b)
    return "%s == %s" % (a, b)


def emit_eq_one(o, ir, m):
    import csnames as N
    o += ""
    o += "    public static bool Same%s(%s a, %s b)" % (m.cs, m.cs, m.cs)
    o += "    {"
    o += "        if (ReferenceEquals(a, b)) return true;"
    o += "        if (a == null || b == null) return false;"
    for f in m.plain():
        if f.card == "map":
            o += "        if (!Eq.Map(a.%s, b.%s)) return false;" % (f.cs, f.cs)
        elif f.card in ("repeated", "packed"):
            el = eq_expr(f, "x", "y")
            o += "        if (!Eq.List(a.%s, b.%s, (x, y) => %s)) return false;" % (f.cs, f.cs, el)
        else:
            o += "        if (!(%s)) return false;" % eq_expr(f, "a." + f.cs, "b." + f.cs)
    for oname, members in m.oneofs.items():
        cf = N.pascal(oname) + "Case"
        o += "        if (a.%s != b.%s) return false;" % (cf, cf)
        o += "        switch (a.%s)" % cf
        o += "        {"
        for f in members:
            o += "            case %s.%s: if (!(%s)) return false; break;" % (
                oneof_case_type(m, oname), N.pascal(f.name), eq_expr(f, "a." + f.cs, "b." + f.cs))
        o += "            default: break;"
        o += "        }"
    o += "        return true;"
    o += "    }"


class Head:
    """A line buffer with the generated-file banner already in it."""

    def __init__(self, what):
        self.lines = [
            "// @generated by ffi/poc/csharp/gen/generate.py from ffi/schema/shapes.json.",
            "// Do not edit. `gen/generate.py --check` fails if this file is not what the",
            "// generator writes today.",
            "//",
            "// " + what,
            "",
        ]

    def __iadd__(self, line):
        self.lines.append(line)
        return self

    def doc(self, text, indent=""):
        import textwrap
        for ln in textwrap.wrap(text, 88 - len(indent) - 4):
            self.lines.append("%s/// %s" % (indent, ln))

    def __str__(self):
        return "\n".join(self.lines).rstrip() + "\n"
