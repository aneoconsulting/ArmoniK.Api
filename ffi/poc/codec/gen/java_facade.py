"""Java backend: the facade types and enums, rendered from a plan (FIX-PLAN WP5 step 3).

Moved from `poc/java/gen/java_facade.py`. The facade's SHAPE is the shape a Java author
would write by hand -- plain classes, public fields, `null` for an absent child,
`java.util.List` for a repeated field -- and arm R, the binding and the harnesses all read
the same classes, so they are rendered once, here, beside the backends that depend on
their conventions. No wire rule: a facade is storage.

One addition over the pre-WP5 facade: `unknownFields`, where a codec rendered in retain
mode (plan Options.unknown = "retain") keeps the message's captured unknown runs, key
included, for re-emission after the known fields. A drop-mode codec never writes it.
"""
import java_names as N

WHO = "java_facade.py"


def emit_enums(p, ns=N.PKG):
    """One `public final class` of `int` constants per enum: an open enum must be able to
    hold a value the reader was not built against (999 has to round-trip)."""
    out = {}
    for ename in p.enum_order:
        o = [N.java_head(p, WHO), "package %s;" % ns, "",
             "/** %s. An `int`, not a Java enum: an open enum must be able to hold a" % ename,
             " *  value the reader was not built against, and 999 has to round-trip. */",
             "public final class %s {" % ename,
             "  private %s() {}" % ename]
        vals = []
        for vname, v in p.enums[ename]["values"].items():
            o.append("  public static final int %s = %d;" % (N.const(ename, vname), v))
            vals.append(v)
        o.append("")
        o.append("  /** The declared values in declaration order. */")
        o.append("  public static final int[] DECLARED = {%s};" % ", ".join(str(v) for v in vals))
        o.append("}")
        o.append("")
        out["%s.java" % ename] = "\n".join(o)
    return out


def emit_types(p, ns=N.PKG):
    """One file per (non-synthetic) message."""
    out = {}
    for name in p.order:
        m = p.msg(name)
        if m.synthetic:
            continue
        o = [N.java_head(p, WHO), "package %s;" % ns, ""]
        src = m.raw.get("source", "")
        if src:
            o.append("/** %s */" % src)
        o.append("public final class %s {" % name)
        o.append("  static final byte[] EMPTY_BYTES = new byte[0];")
        o.append("  static final int[] EMPTY_I32 = new int[0];")
        o.append("  static final long[] EMPTY_I64 = new long[0];")
        o.append("  static final double[] EMPTY_F64 = new double[0];")
        o.append("  static final boolean[] EMPTY_BOOL = new boolean[0];")
        o.append("")
        for f in m.plain:
            if f.card == "map":
                N.check_map(p, f)
            o.append("  public %s %s%s;" % (N.facade_type(f), f.name, N.facade_init(f)))
            if N.has_flag(f):
                o.append("  public boolean has_%s;" % f.name)
        for oname, members in m.oneofs.items():
            o.append("")
            o.append("  /** oneof `%s`: a case tag plus one slot per member. The case value" % oname)
            o.append("   *  is the member's TAG, as in the ABI group; `%s_CASE_NONE` is none." % N.screaming(oname))
            o.append("   *  A case that is neither 0 nor a member tag is REFUSED by every")
            o.append("   *  encoder (plan: oneof_checks). */")
            o.append("  public static final int %s_CASE_NONE = 0;" % N.screaming(oname))
            for g in members:
                o.append("  public static final int %s_CASE_%s = %d;"
                         % (N.screaming(oname), N.screaming(g.name), g.tag))
            o.append("  public int %s_case;" % oname)
            for g in members:
                o.append("  public %s %s_%s%s;" % (N.member_type_java(g), oname, g.name,
                                                  N.member_init(g)))
        o.append("")
        o.append("  /** Unknown fields captured by a retain-mode codec, verbatim, key included;")
        o.append("   *  re-emitted after the known fields. `null` when none. */")
        o.append("  public byte[] %s;" % N.UNKNOWN)
        o.append("}")
        o.append("")
        out["%s.java" % name] = "\n".join(o)
    return out
