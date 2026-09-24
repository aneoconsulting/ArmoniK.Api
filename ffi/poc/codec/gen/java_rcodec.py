"""Java backend: arm `R`, the generated pure-Java codec, rendered from PLANS.

FIX-PLAN WP5 step 3 (R-E4, R-G8). The pre-WP5 `poc/java/gen/java_codec.py` walked the IR
and wrote its own wire rules, and the corpus found five of them wrong: tag 0 accepted, the
map entry's key written unconditionally, `x != 0.0` as the presence test of a double,
REPLACE where protobuf MERGES a repeated singular or oneof message, and an undeclared oneof
case encoded without a refusal. It retires. This module renders `MessagePlan.encode` (the
ordered writes and their presence tests), `MessagePlan.oneof_checks` and
`MessagePlan.decode` (the (field number, wire type) table and its actions) -- the very
lists the Rust core behind the C ABI renders -- and decides nothing about the wire.

What IS decided here (the plan contract's "may"): the facade spelling of each access, the
Java `switch` the decode table becomes, `Enc`'s learned-width length prefixes (the core's
algorithm, ABI v1 section 6), how packed runs grow, and that an error is an exception.

One class per unknown-field mode, because a facade codec has no per-call switch the way
the C ABI has two families: `Codec` (plan Options.unknown = "drop") and `CodecRetain`
("retain", the facade's `unknownFields`), two renderings of one plan (owner position 6).
"""
from plan import GROUP_DEPTH_LIMIT, MAX_FIELD_NUMBER, relower
import java_names as N

WHO = "java_rcodec.py"


class Sites(object):
    """One learned length-prefix width per site (ABI v1 section 6's mechanism)."""

    def __init__(self):
        self.ids = {}

    def of(self, key):
        return self.ids.setdefault(key, len(self.ids))

    def __len__(self):
        return len(self.ids)


# ------------------------------------------------------------------ value spellings

def _sref(expr):
    """The wire form of a facade string: a borrowed view materialises its String (which is
    why the borrowed arm is a DECODE arm and its encode column is not quoted)."""
    return expr if not N.is_borrow() else "%s.str()" % expr


def _nonempty(expr):
    return "!%s.isEmpty()" % expr if not N.is_borrow() else "%s.byteLength() > 0" % expr


def _mk_str(expr):
    return expr if not N.is_borrow() else "%s.of(%s)" % (N.string_type(), expr)


def _empty_str():
    return '""' if not N.is_borrow() else "%s.EMPTY" % N.string_type()


def _write_value(f, v):
    """Write ONE value of `f` WITH its key: the plan's value codec, facade side."""
    return {
        "varint_i32": "e.varintField(%d, (long) (%s));",     # sign-extended: 10 bytes if < 0
        "varint_i64": "e.varintField(%d, %s);",
        "varint_bool": "e.varintField(%d, (%s) ? 1 : 0);",
        "fixed64_f64": "e.f64Field(%d, %s);",
        "fixed32_u32": "e.fixed32Field(%d, %s);",
    }[f.value] % (f.tag, v)


def _write_packed_elem(f, v):
    """One element of a packed run (no key)."""
    return {
        "varint_i32": "e.varint((long) (%s));",
        "varint_i64": "e.varint(%s);",
        "varint_bool": "e.varint((%s) ? 1 : 0);",
        "fixed64_f64": "e.f64(%s);",
        "fixed32_u32": "e.fixed32(%s);",
    }[f.value] % v


def _implicit(f, v):
    """The plan's implicit-presence test for a scalar (ENCODE RULES)."""
    if f.value == "fixed64_f64":
        return "Double.doubleToRawLongBits(%s) != 0L" % v     # R-E3: -0.0 is written
    if f.value == "varint_bool":
        return v
    return "%s != 0" % v


def _read(f):
    """Read one value of `f` as its facade type (plan VALUE_CODECS)."""
    return {
        "varint_i32": "(int) d.readVarint()",        # the low 32 bits, two's complement
        "varint_i64": "d.readVarint()",
        "varint_bool": "d.readVarint() != 0",
        "fixed64_f64": "d.readDouble()",
        "fixed32_u32": "d.readFixed32()",
    }[f.value]


def _blob_write(f, v):
    if f.kind == "string":
        return "e.stringField(%d, %s);" % (f.tag, _sref(v))
    return "e.blobField(%d, %s);" % (f.tag, v)


def _blob_read(f):
    if f.kind == "string":
        return _mk_str("d.readString()")
    return "d.readBytes()"


# ------------------------------------------------------------------ encode

def _enc_message(p, m, o, sites, retain):
    name = m.name
    # Plan `oneof_checks`: before any write, a case that is neither 0 nor a member tag is
    # REFUSED (ERR_ABI). A host built against a newer descriptor must be loud (R-E4).
    for oname, tags in m.oneof_checks:
        cond = " && ".join(["o.%s_case != 0" % oname] + ["o.%s_case != %d" % (oname, t) for t in tags])
        o.append("    if (%s)" % cond)
        o.append("      throw new Enc.Refused(Enc.ERR_ABI, \"%s.%s: undeclared oneof case \" + o.%s_case);"
                 % (name, oname, oname))
    for st in m.encode:
        f = st.field
        if st.op == "unknown_tail":
            if retain:
                o.append("    if (o.%s != null) e.raw(o.%s);   // plan: after every known field"
                         % (N.UNKNOWN, N.UNKNOWN))
            continue
        v = "o.%s" % f.name
        if st.op == "scalar":
            cond = ("o.has_%s" % f.name) if f.presence == "explicit" else _implicit(f, v)
            o.append("    if (%s) %s" % (cond, _write_value(f, v)))
        elif st.op == "blob":
            if f.presence == "explicit":
                cond = "%s != null" % v
            elif f.kind == "string":
                cond = _nonempty(v)
            else:
                cond = "%s.length > 0" % v
            o.append("    if (%s) %s" % (cond, _blob_write(f, v)))
        elif st.op == "child":
            s = sites.of((name, f.name))
            o.append("    if (%s != null) { long mk = e.begin(%d, %d); enc%s(e, %s); e.end(mk); }"
                     % (v, f.tag, s, f.of, v))
        elif st.op == "packed":
            s = sites.of(("packed", name, f.name))
            o.append("    if (%s.length > 0) {" % v)
            o.append("      long mk = e.begin(%d, %d);" % (f.tag, s))
            o.append("      for (int i = 0; i < %s.length; i++) %s" % (v, _write_packed_elem(f, "%s[i]" % v)))
            o.append("      e.end(mk);")
            o.append("    }")
        elif st.op == "repeated_blob":
            o.append("    for (%s x : %s) %s" % (N.elem_type_java(f), v, _blob_write(f, "x")))
        elif st.op == "repeated_message":
            s = sites.of((name, f.name))
            o.append("    for (%s x : %s) { long mk = e.begin(%d, %d); enc%s(e, x); e.end(mk); }"
                     % (f.of, v, f.tag, s, f.of))
        elif st.op == "map":
            N.check_map(p, f)
            s = sites.of(("map", name, f.name))
            entry = p.msg(f.entry)
            st_t = N.string_type()
            # plan ENCODE RULES: ascending UTF-8 key order (a String TreeMap is UTF-16 order).
            src = ("utf8Sorted(%s)" % v) if st_t == "String" else ("%s.entrySet()" % v)
            o.append("    for (java.util.Map.Entry<%s, %s> en : %s) {" % (st_t, st_t, src))
            o.append("      long mk = e.begin(%d, %d);" % (f.tag, s))
            # The pair message's OWN encode plan: key 1 and value 2, each an implicit-
            # presence string, each omitted when empty (the canonical form, R-E4).
            for est in entry.encode:
                if est.op == "unknown_tail":
                    continue                          # a facade map entry has no bag
                ef = est.field
                if est.op != "blob" or ef.kind != "string" or ef.presence != "implicit":
                    raise NotImplementedError("map entry %s: step %s %s" % (entry.name, est.op, ef.kind))
                x = "en.getKey()" if ef.name == "key" else "en.getValue()"
                o.append("      if (%s) %s" % (_nonempty(x), _blob_write(ef, x)))
            o.append("      e.end(mk);")
            o.append("    }")
        elif st.op == "oneof_member":
            case = N.case_const(name, st.oneof, f.name)
            mv = "o.%s_%s" % (st.oneof, f.name)
            # Written iff the case selects it, WHATEVER its value (plan).
            if f.kind == "message":
                s = sites.of(("oneof", name, st.oneof, f.name))
                o.append("    if (o.%s_case == %s) { long mk = e.begin(%d, %d);"
                         " if (%s != null) enc%s(e, %s); e.end(mk); }"
                         % (st.oneof, case, f.tag, s, mv, f.of, mv))
            elif f.kind == "string":
                o.append("    if (o.%s_case == %s) %s" % (
                    st.oneof, case, _blob_write(f, "(%s == null ? %s : %s)" % (mv, _empty_str(), mv))))
            elif f.kind == "bytes":
                o.append("    if (o.%s_case == %s) %s" % (
                    st.oneof, case, _blob_write(f, "(%s == null ? EMPTY : %s)" % (mv, mv))))
            else:
                o.append("    if (o.%s_case == %s) %s" % (st.oneof, case, _write_value(f, mv)))
        else:
            raise NotImplementedError("encode step %r (%s)" % (st.op, name))


# ------------------------------------------------------------------ decode

def _sub(o, ind, call):
    """A length-delimited body decoded by `call`, with the reader's limit narrowed to it."""
    p = " " * ind
    o.append("%s{ int outer = d.push(); %s; d.pop(outer); }" % (p, call))


def _dec_action(p, m, act, o):
    f = act.field
    op = act.op
    r = "r.%s" % f.name
    if op == "set_scalar":
        o.append("            %s = %s;" % (r, _read(f)))
        if N.has_flag(f):
            o.append("            r.has_%s = true;" % f.name)
    elif op == "set_blob":
        o.append("            %s = %s;" % (r, _blob_read(f)))
    elif op == "merge_child":
        # Plan rule: a repeated occurrence MERGES into the one already decoded (R-E4).
        o.append("            if (%s == null) %s = new %s();" % (r, r, f.of))
        _sub(o, 12, "dec%s(d, %s, depth + 1)" % (f.of, r))
    elif op == "append_message":
        o.append("            { %s x = new %s();" % (f.of, f.of))
        _sub(o, 14, "dec%s(d, x, depth + 1)" % f.of)
        o.append("              %s.add(x); }" % r)
    elif op == "append_blob":
        o.append("            %s.add(%s);" % (r, _blob_read(f)))
    elif op == "packed_run":
        jt = N.FSCALAR[f.kind]
        o.append("            { int outer = d.push();")
        o.append("              %s[] acc = new %s[8];" % (jt, jt))
        o.append("              int n = 0;")
        o.append("              while (!d.done()) {")
        o.append("                if (n == acc.length) acc = java.util.Arrays.copyOf(acc, n * 2);")
        o.append("                acc[n++] = %s;" % _read(f))
        o.append("              }")
        o.append("              d.pop(outer);")
        o.append("              %s = append(%s, acc, n); }" % (r, r))
    elif op == "packed_one":
        jt = N.FSCALAR[f.kind]
        o.append("            { %s[] one = {%s}; %s = append(%s, one, 1); }" % (jt, _read(f), r, r))
    elif op == "map_entry":
        N.check_map(p, f)
        entry = p.msg(f.entry)
        st_t = N.string_type()
        o.append("            {")
        o.append("              if (depth + 1 > LIMIT) throw Dec.err(Dec.ERR_DEPTH, \"nesting past \" + LIMIT);")
        o.append("              int outer = d.push();")
        o.append("              %s ek = %s, ev = %s;" % (st_t, _empty_str(), _empty_str()))
        o.append("              while (!d.done()) {")
        o.append("                long kk = d.readVarint();")
        o.append("                int et = (int) (kk >>> 3), ew = (int) (kk & 7);")
        o.append("                if (et == 0 || Long.compareUnsigned(kk >>> 3, %dL) > 0) throw Dec.err(Dec.ERR_MALFORMED, \"field number 0 or above 2^29-1\");" % MAX_FIELD_NUMBER)
        # The pair message's OWN decode plan.
        first = True
        for (etag, ewire), eact in sorted(entry.decode.items()):
            ef = eact.field
            if eact.op != "set_blob" or ef.kind != "string":
                raise NotImplementedError("map entry %s: action %s" % (entry.name, eact.op))
            x = "ek" if ef.name == "key" else "ev"
            o.append("                %sif (et == %d && ew == %d) %s = %s;"
                     % ("" if first else "else ", etag, ewire, x, _blob_read(ef)))
            first = False
        o.append("                // A facade map entry has no bag: an unknown field inside an")
        o.append("                // entry is skipped in both modes (`U-map-entry`, disputed).")
        o.append("                else d.skip(et, ew, MAX_FIELD_NUMBER, GROUP_DEPTH_LIMIT);")
        o.append("              }")
        o.append("              d.pop(outer);")
        o.append("              // Plan rule: a duplicate key replaces the earlier value.")
        o.append("              %s.put(ek, ev);" % r)
        o.append("            }")
    elif op == "oneof_set":
        case = N.case_const(m.name, act.oneof, f.name)
        mv = "r.%s_%s" % (act.oneof, f.name)
        if f.kind == "message":
            # Plan rule: the SAME member merges; another one starts from empty (R-E4).
            o.append("            if (r.%s_case != %s || %s == null) %s = new %s();"
                     % (act.oneof, case, mv, mv, f.of))
            o.append("            r.%s_case = %s;" % (act.oneof, case))
            _sub(o, 12, "dec%s(d, %s, depth + 1)" % (f.of, mv))
        elif f.is_blob:
            o.append("            %s = %s;" % (mv, _blob_read(f)))
            o.append("            r.%s_case = %s;" % (act.oneof, case))
        else:
            o.append("            %s = %s;" % (mv, _read(f)))
            o.append("            r.%s_case = %s;" % (act.oneof, case))
    else:
        raise NotImplementedError("decode action %r (%s.%s)" % (op, m.name, f.name))


def _dec_message(p, m, o, retain):
    by_tag = {}
    for (tag, wire), act in m.decode.items():
        by_tag.setdefault(tag, []).append((wire, act))
    o.append("    // Plan rule: a message more than LIMIT levels below the root is refused.")
    o.append("    if (depth > LIMIT) throw Dec.err(Dec.ERR_DEPTH, \"nesting past \" + LIMIT);")
    o.append("    while (!d.done()) {")
    o.append("      int s0 = d.pos;")
    o.append("      long k = d.readVarint();")
    o.append("      int tag = (int) (k >>> 3), wire = (int) (k & 7);")
    o.append("      // Plan rule: field number 0 is malformed on every message (R-E4).")
    o.append("      if (tag == 0 || Long.compareUnsigned(k >>> 3, %dL) > 0) throw Dec.err(Dec.ERR_MALFORMED, \"field number 0 or above 2^29-1\");" % MAX_FIELD_NUMBER)
    o.append("      known: {")
    o.append("        switch (tag) {")
    for tag in sorted(by_tag):
        o.append("          case %d:" % tag)
        first = True
        for wire, act in sorted(by_tag[tag], key=lambda x: x[0]):
            o.append("            %sif (wire == %d) {   // %s %s" % ("" if first else "} else ",
                                                               wire, act.op, act.field.name))
            first = False
            body = []
            _dec_action(p, m, act, body)
            o.extend("  " + ln for ln in body)
        o.append("            } else break;")
        o.append("            break known;")
    o.append("          default: break;")
    o.append("        }")
    if retain:
        o.append("        // Plan rule (retain): not in the table -> captured verbatim, key")
        o.append("        // included, and re-emitted after the known fields on encode.")
        o.append("        d.skip(tag, wire, MAX_FIELD_NUMBER, GROUP_DEPTH_LIMIT);")
        o.append("        r.%s = Dec.append(r.%s, d.b, s0, d.pos);" % (N.UNKNOWN, N.UNKNOWN))
    else:
        o.append("        // Plan rule (drop): not in the table -- including a known number at a")
        o.append("        // wire type the table has no entry for (R-E2) -- is skipped.")
        o.append("        d.skip(tag, wire, MAX_FIELD_NUMBER, GROUP_DEPTH_LIMIT);")
    o.append("      }")
    o.append("    }")


APPEND = '''
  // A batched `add` may be called more than once per field (ABI v1 7.4), so the binding
  // APPENDS and never sizes to the count it was handed; this arm does the same, so the two
  // differ in where the bytes are parsed and in nothing else.
  static int[] append(int[] a, int[] b, int n) {
    int[] out = java.util.Arrays.copyOf(a, a.length + n);
    System.arraycopy(b, 0, out, a.length, n);
    return out;
  }

  static long[] append(long[] a, long[] b, int n) {
    long[] out = java.util.Arrays.copyOf(a, a.length + n);
    System.arraycopy(b, 0, out, a.length, n);
    return out;
  }

  static double[] append(double[] a, double[] b, int n) {
    double[] out = java.util.Arrays.copyOf(a, a.length + n);
    System.arraycopy(b, 0, out, a.length, n);
    return out;
  }

  static boolean[] append(boolean[] a, boolean[] b, int n) {
    boolean[] out = java.util.Arrays.copyOf(a, a.length + n);
    System.arraycopy(b, 0, out, a.length, n);
    return out;
  }

  static final byte[] EMPTY = new byte[0];
'''


def emit(p, ns=N.PKG, unknown="drop"):
    """One arm R codec class for one unknown-field mode: `Codec` (drop) or `CodecRetain`."""
    if unknown not in ("drop", "retain"):
        raise ValueError("a facade codec is one mode; render both for both")
    if p.options.unknown != unknown:
        p = relower(p, p.options.with_unknown(unknown))
    if p.options.utf8 != "reject":
        raise NotImplementedError("utf8=%r: the Java runtime renders the reject policy only"
                                  % p.options.utf8)
    retain = unknown == "retain"
    cls = "CodecRetain" if retain else "Codec"
    sites = Sites()
    body = []
    for name in p.order:
        m = p.msg(name)
        if m.synthetic:
            continue
        body.append("")
        body.append("  static void enc%s(Enc e, %s o) {" % (name, name))
        _enc_message(p, m, body, sites, retain)
        body.append("  }")
    for name in p.order:
        m = p.msg(name)
        if m.synthetic:
            continue
        body.append("")
        body.append("  static %s dec%s(Dec d) {" % (name, name))
        body.append("    %s r = new %s();" % (name, name))
        body.append("    dec%s(d, r, 0);" % name)
        body.append("    return r;")
        body.append("  }")
        body.append("")
        body.append("  /** Decode INTO `r`, which is what makes a repeated occurrence merge. */")
        body.append("  static void dec%s(Dec d, %s r, int depth) {" % (name, name))
        _dec_message(p, m, body, retain)
        body.append("  }")

    o = [N.java_head(p, WHO), "package %s;" % ns, "", "import ak.Dec;", "import ak.Enc;", "",
         "/** Arm `R` (unknown fields %s): the generated pure-Java codec, rendered from the" % unknown.upper(),
         " *  plans of poc/codec/gen/plan.py -- README R3's no-boundary control and section 13",
         " *  outcome 2's fallback architecture. An error is a {@link Dec.Malformed} or an",
         " *  {@link Enc.Refused}, and nothing is returned after it (plan rule, R-G6). */",
         "public final class %s {" % cls,
         "  private %s() {}" % cls,
         "",
         "  /** One learned length-prefix width per site (ABI v1 section 6). */",
         "  public static final int SITES = %d;" % len(sites),
         "  /** The plan's recursion limit (Options.recursion_limit). */",
         "  public static final int LIMIT = %d;" % p.options.recursion_limit,
         "  /** The plan's MAX_FIELD_NUMBER and GROUP_DEPTH_LIMIT, handed to `Dec.skip`, whose",
         "   *  group skip applies them to every key inside a group (D38). */",
         "  public static final long MAX_FIELD_NUMBER = %dL;" % MAX_FIELD_NUMBER,
         "  public static final int GROUP_DEPTH_LIMIT = %d;" % GROUP_DEPTH_LIMIT,
         "  /** The plan's unknown-field mode and UTF-8 policy, for a log to name. */",
         "  public static final String UNKNOWN_FIELDS = \"%s\";" % unknown,
         "  public static final String UTF8_POLICY = \"%s\";" % p.options.utf8,
         APPEND, JAVA_MAP_ORDER]
    o.extend(body)
    o.append("")
    o.append("  // ---- the roots ------------------------------------------------------")
    for root in p.roots:
        o.append("")
        o.append("  public static byte[] encode%s(Enc e, %s o) {" % (root, root))
        o.append("    e.reset();")
        o.append("    enc%s(e, o);" % root)
        o.append("    return e.toBytes();")
        o.append("  }")
        o.append("")
        o.append("  public static void encodeInto%s(Enc e, %s o) {" % (root, root))
        o.append("    e.reset();")
        o.append("    enc%s(e, o);" % root)
        o.append("  }")
        o.append("")
        o.append("  public static %s decode%s(Dec d, byte[] buf, int off, int len) {" % (root, root))
        o.append("    d.reset(buf, off, len);")
        o.append("    return dec%s(d);" % root)
        o.append("  }")
    o.append("}")
    o.append("")
    return "\n".join(o)


JAVA_MAP_ORDER = '''
  /** plan ENCODE RULES (WP5 step 6): map entries in ascending order of the key's UTF-8
   *  bytes, i.e. code-point order. A {@code TreeMap<String, ...>} iterates in UTF-16 code
   *  unit order, which differs for a key with a supplementary character, so the entries
   *  are handed over re-sorted. (A {@code Utf8View} key already compares by bytes.) */
  public static <V> java.util.List<java.util.Map.Entry<String, V>> utf8Sorted(java.util.Map<String, V> m) {
    java.util.ArrayList<java.util.Map.Entry<String, V>> l = new java.util.ArrayList<java.util.Map.Entry<String, V>>(m.entrySet());
    if (l.size() > 1) {
      java.util.Collections.sort(l, new java.util.Comparator<java.util.Map.Entry<String, V>>() {
        public int compare(java.util.Map.Entry<String, V> a, java.util.Map.Entry<String, V> b) {
          return cmpUtf8(a.getKey(), b.getKey());
        }
      });
    }
    return l;
  }

  public static int cmpUtf8(String a, String b) {
    int i = 0, j = 0;
    while (i < a.length() && j < b.length()) {
      int x = a.codePointAt(i), y = b.codePointAt(j);
      if (x != y) return x < y ? -1 : 1;
      i += Character.charCount(x);
      j += Character.charCount(y);
    }
    return (i < a.length() ? 1 : 0) - (j < b.length() ? 1 : 0);
  }
'''
