"""C# backend: the facade types of a message set, and a structural comparer over them.

FIX-PLAN WP5 step 4. Renders a PLAN (`plan.py`'s contract); imports nothing else from the
generator. The facade is the one object model every C# arm operates on (the managed codec,
the core-ffi binding, the payload builders), so the managed codec and the binding, both
rendered in this package, name its members through `cs_names` and nowhere else.

Two decisions are the host's and are stated where they are made:

  * public fields, not auto-properties (no question for a reader to take on trust);
  * a map is an insertion-ordered `OrderedMap` (a Dictionary plus a List, what
    Google.Protobuf's MapField is), so the canonical form's key order comes from the
    builder inserting sorted and the managed decode is not a comparison of container types.

The unknown-field bag. Owner position 6 (D4) keeps both unknown-field behaviours, so every
facade class carries `UnknownFields`: the raw tag-and-value runs a decoder in RETAIN mode
captured, null when there are none. A drop-mode decode leaves it null; an encode writes it
where the plan says (`unknown_tail`). A map entry has no facade class and so no bag.
"""
from plan import as_plan, unknown_compiled_out
import cs_names as N

BAG = "UnknownFields"


def _map_check(p, f):
    e = p.msg(f.entry)
    kinds = [(x.name, x.kind) for x in e.fields]
    if kinds != [("key", "string"), ("value", "string")]:
        raise NotImplementedError("map %s.%s: the facade has OrderedMap<string, string> only; "
                                  "a %r entry has no case (raise, never skip)" % (f.owner, f.name, kinds))


def field_type(f):
    """The facade type of one field."""
    if f.card == "map":
        return "OrderedMap<string, string>"
    if f.card == "packed":
        return "List<%s>" % (f.of if f.kind == "enum" else N.SCALAR[f.kind])
    if f.card == "repeated":
        if f.kind == "string":
            return "List<string>"
        if f.kind == "bytes":
            return "List<byte[]>"
        return "List<%s>" % f.of
    if f.kind == "string":
        return "string"
    if f.kind == "bytes":
        return "byte[]"
    if f.kind == "message":
        return f.of
    base = f.of if f.kind == "enum" else N.SCALAR[f.kind]
    return base + "?" if f.explicit else base


def field_init(f):
    """The initialiser, or None for a field whose default is `default` (null / zero)."""
    if f.card == "map":
        return "new OrderedMap<string, string>()"
    if f.card in ("packed", "repeated"):
        return "new %s()" % field_type(f)
    if f.explicit or f.kind == "message" or f.oneof:
        return None
    if f.kind == "string":
        return '""'
    if f.kind == "bytes":
        return "EmptyBytes"
    return None


def facade_messages(p):
    """The real messages of the set, in description order (a synthetic map pair is not a
    facade type: the facade map carries it)."""
    return [p.msg(n) for n in p.order if not p.msg(n).synthetic]


def emit_types(x, ns):
    p = as_plan(x)
    o = N.Head("The facade types of this message set.", p.source, "cs_types")
    o += "using System;"
    o += "using System.Collections.Generic;"
    if ns != "Armonik.Ffi.Facade":
        o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace %s;" % ns
    o += ""
    for ename in p.enum_order:
        edef = p.enums[ename]
        if edef.get("why"):
            o.doc(edef["why"])
        o.doc("Open enum: a wire value this build was not generated against is representable "
              "as `(%s)v` and round-trips. A C# enum is open by construction." % ename)
        o += "public enum %s" % ename
        o += "{"
        for vname, v in edef["values"].items():
            o += "    %s = %d," % (N.enum_member(ename, vname), v)
        o += "}"
        o += ""
    for m in facade_messages(p):
        for f in m.fields:
            if f.card == "map":
                _map_check(p, f)
            if N.field(f.name) == BAG:
                raise NotImplementedError("%s.%s collides with the unknown-field bag" % (m.name, f.name))
        for oname, members in m.oneofs.items():
            ct = N.oneof_case_type(m.name, oname)
            o.doc("Which member of `%s.%s` is set, by TAG (the ABI's `%s_case` carries the "
                  "same number). `None` is no member." % (m.name, oname, oname))
            o += "public enum %s" % ct
            o += "{"
            o += "    None = 0,"
            for g in members:
                o += "    %s = %d," % (N.pascal(g.name), g.tag)
            o += "}"
            o += ""
        doc = m.raw.get("source") or m.raw.get("why") or ""
        if doc:
            o.doc(doc)
        o.doc("Batching predicate (ABI v1 6/7.2): %s." % (
            "a LEAF" if m.leaf else "NOT a leaf"))
        o += "public sealed class %s" % m.name
        o += "{"
        o += "    internal static readonly byte[] EmptyBytes = new byte[0];"
        for f in m.plain:
            init = field_init(f)
            o += "    public %s %s%s;" % (field_type(f), N.field(f.name),
                                          "" if init is None else " = " + init)
        for oname, members in m.oneofs.items():
            o += ""
            o += "    public %s %s;" % (N.oneof_case_type(m.name, oname), N.oneof_case_field(oname))
            for g in members:
                o += "    public %s %s;" % (field_type(g), N.field(g.name))
        if not unknown_compiled_out(p):
            # R-H22 / CAMPAIGN req 10 (owner 2026-09-26): the NO-UNKNOWN build has no member.
            o += "    /// The unknown-field bag: captured by a RETAIN-mode decode, written back by an"
            o += "    /// encode after the known fields (plan: unknown_tail). Null when empty."
            o += "    public byte[] %s;" % BAG
        o += "}"
        o += ""
    return str(o)


def _eq_expr(f, a, b):
    if f.kind == "bytes":
        return "Eq.Bytes(%s, %s)" % (a, b)
    if f.kind == "message":
        return "Same%s(%s, %s)" % (f.of, a, b)
    if f.kind == "double":
        # Bit equality: -0.0 and +0.0 are different values on the wire.
        return "BitConverter.DoubleToInt64Bits(%s) == BitConverter.DoubleToInt64Bits(%s)" % (a, b)
    return "%s == %s" % (a, b)


def emit_eq(x, ns):
    """A structural comparer, generated from the same plan as the codec, so a field the
    codec handles and the comparer does not cannot exist. Compares the bag too (null and
    empty are equal)."""
    p = as_plan(x)
    o = N.Head("Structural equality over the facade, for the decode half of the oracle.",
               p.source, "cs_types")
    o += "using System;"
    o += "using System.Collections.Generic;"
    if ns != "Armonik.Ffi.Facade":
        o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace %s;" % ns
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
    for m in facade_messages(p):
        o += ""
        o += "    public static bool Same%s(%s a, %s b)" % (m.name, m.name, m.name)
        o += "    {"
        o += "        if (ReferenceEquals(a, b)) return true;"
        o += "        if (a == null || b == null) return false;"
        for f in m.plain:
            fn = N.field(f.name)
            if f.card == "map":
                o += "        if (!Eq.Map(a.%s, b.%s)) return false;" % (fn, fn)
            elif f.card in ("repeated", "packed"):
                o += "        if (!Eq.List(a.%s, b.%s, (x, y) => %s)) return false;" % (
                    fn, fn, _eq_expr(f, "x", "y"))
            else:
                o += "        if (!(%s)) return false;" % _eq_expr(f, "a." + fn, "b." + fn)
        for oname, members in m.oneofs.items():
            cf = N.oneof_case_field(oname)
            o += "        if (a.%s != b.%s) return false;" % (cf, cf)
            o += "        switch (a.%s)" % cf
            o += "        {"
            for g in members:
                gn = N.field(g.name)
                o += "            case %s.%s: if (!(%s)) return false; break;" % (
                    N.oneof_case_type(m.name, oname), N.pascal(g.name), _eq_expr(g, "a." + gn, "b." + gn))
            o += "            default: break;"
            o += "        }"
        if not unknown_compiled_out(p):
            o += "        if (!Eq.Bytes(a.%s, b.%s)) return false;" % (BAG, BAG)
        o += "        return true;"
        o += "    }"
    o += "}"
    return str(o)
