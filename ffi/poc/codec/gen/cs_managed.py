"""C# backend: the MANAGED codec (README R3's no-boundary control), rendered from PLANS.

FIX-PLAN WP5 step 4. This module renders `plan.py`'s encode and decode plans over the
facade types (`cs_types`) and the hand-written runtime `poc/csharp/src/Facade/Wire.cs`
(varints, the buffer, the learned-width prefix, the reader -- the analogue of `ak-rt`,
knowing no message). It decides nothing about the wire. From the plan: the order of the
writes (the step list), every presence test (`FieldPlan.presence` + the ENCODE RULES), the
(field number, wire type) pairs a field accepts (the decode table: a pair not in it is an
unknown field, which is how a known number at the wrong wire type is handled, R-E2), merge
versus replace, tag 0, the unknown-field behaviour (Options.unknown), the UTF-8 policy
(Options.utf8, R-E7) and the depth limit (Options.recursion_limit). What is decided here
is C#: names, a `switch` on the key, the learned-width buffer strategy, and the sub-range
reader (a nested body is read with `Dec.End` narrowed to it, so no read can cross it).

Four methods per message:
  Write<M>       one pass, length prefixes resolved after the body (ABI v1 section 6's
                 learned width, per encode context);
  SizeOf<M>      the encoded body size;
  WriteSized<M>  two passes, exact prefixes, no learned width (Google.Protobuf's shape);
  Read<M>        decode into the facade: `Read<M>(ref Dec d, M m, int depth)`, reading to
                 `d.End`; a root is read at depth 0.

Length arithmetic on decode is the runtime's `Dec.LenEnd`: the prefix is read as the full
64-bit varint and compared with the bytes LEFT (`n > End - Pos`), never `Pos + n > End` on a
narrowed `n` (R-D1, R-G8); a fixed-width read compares the bytes left with its width.

Unknown fields. Options.unknown = "drop" skips; "retain" captures the whole run (key
included) into `UnknownFields` and the encoder writes it after the known fields; "both" (the
default plan) emits the capture behind the reader's `Dec.Retain` flag, so the host picks per
call, as the C ABI core's two entry families do. A map entry has no facade class and so no
bag: an unknown field inside an entry is skipped in every mode (the rust backends do the
same; `U-map-entry` is a disputed corpus row).
"""
from plan import LEN, as_plan
import cs_names as N
from cs_types import BAG, facade_messages

WRITE_VARINT = {"varint_i32", "varint_i64", "varint_bool"}


def _varint_of(f, expr):
    """A field value as the u64 its varint carries (plan VALUE_CODECS)."""
    v = f.value
    if v == "varint_bool":
        return "(%s ? 1UL : 0UL)" % expr
    if v == "varint_i32":
        # Sign-extended to 64 bits: a negative int32 is a ten-byte varint.
        return "(ulong)(long)(int)(%s)" % expr
    if v == "varint_i64":
        return "(ulong)(%s)" % expr
    raise NotImplementedError("varint of %r" % v)


def _read(f, d="d"):
    """An expression reading one value of `f` from reader `d` (plan VALUE_CODECS)."""
    v = f.value
    if v == "varint_i32":
        # The low 32 bits as a two's-complement i32 (R-E5).
        return "(%s)(int)%s.Varint()" % (f.of, d) if f.kind == "enum" else "(int)%s.Varint()" % d
    if v == "varint_i64":
        return "(long)%s.Varint()" % d
    if v == "varint_bool":
        return "(%s.Varint() != 0UL)" % d
    if v == "fixed64_f64":
        return "%s.F64()" % d
    if v == "fixed32_u32":
        return "%s.Fixed32()" % d
    raise NotImplementedError("read of %r" % v)


def _nonzero(f, expr):
    """ENCODE RULES, implicit presence of a scalar: varint kinds when != 0 (bool when true),
    fixed32 when != 0, double when its BIT PATTERN != 0 (so -0.0 is written, R-E3)."""
    v = f.value
    if v == "varint_bool":
        return expr
    if v == "fixed64_f64":
        return "BitConverter.DoubleToInt64Bits(%s) != 0L" % expr
    if v == "fixed32_u32":
        return "%s != 0u" % expr
    if f.kind == "enum":
        return "(int)%s != 0" % expr
    return "%s != 0" % expr


def _write_value(f, tag, expr):
    """One scalar field, key and value."""
    v = f.value
    if v == "fixed64_f64":
        return "e.F64Field(%d, %s);" % (tag, expr)
    if v == "fixed32_u32":
        return "e.Fixed32Field(%d, %s);" % (tag, expr)
    return "e.VarintField(%d, %s);" % (tag, _varint_of(f, expr))


def _run_value(f, expr):
    """One element of a packed run, value only."""
    v = f.value
    if v == "fixed64_f64":
        return "e.F64(%s);" % expr
    if v == "fixed32_u32":
        return "e.Fixed32(%s);" % expr
    return "e.Varint(%s);" % _varint_of(f, expr)


def _size_value(f, expr):
    """Bytes of one value, no key."""
    v = f.value
    if v == "fixed64_f64":
        return "8"
    if v == "fixed32_u32":
        return "4"
    return "W.VarintLen(%s)" % _varint_of(f, expr)


def _key_len(tag, wire):
    return "W.VarintLen(%dUL)" % ((tag << 3) | wire)


class Codec:
    def __init__(self, p):
        self.p = p
        self.sites = []
        self.retain = p.options.unknown
        self.limit = p.options.recursion_limit
        self.strf = "StrReject" if p.options.utf8 == "reject" else "StrLossy"

    def site(self, owner, f, what):
        self.sites.append("%s.%s (%s)" % (owner, f.name, what))
        return len(self.sites) - 1

    # ------------------------------------------------------------- guards per step
    def _cond(self, m, st, acc):
        """The plan's presence test for a singular step, as a C# condition."""
        f = st.field
        if st.op == "oneof_member":
            return "m.%s == %s.%s" % (N.oneof_case_field(st.oneof),
                                      N.oneof_case_type(m.name, st.oneof), N.pascal(f.name))
        pr = f.presence
        if pr == "message":
            return "%s != null" % acc
        if pr == "explicit":
            return "%s.HasValue" % acc if f.is_scalar_leaf else "%s != null" % acc
        if pr in ("implicit", "direct"):
            if f.is_blob:
                return "%s != null && %s.Length != 0" % (acc, acc)
            return _nonzero(f, acc)
        raise NotImplementedError("presence %r on %s.%s" % (pr, m.name, f.name))

    def _val(self, st, acc):
        """The value to write once the condition holds."""
        f = st.field
        if f.explicit and f.is_scalar_leaf and st.op != "oneof_member":
            return acc + ".Value"
        if st.op == "oneof_member" and f.is_blob:
            return "(%s ?? %s)" % (acc, '""' if f.kind == "string" else "W.EmptyBytes")
        if st.op == "oneof_member" and f.kind == "message":
            # A selected member is written whatever its value, an absent object as empty.
            return "(%s ?? new %s())" % (acc, f.of)
        return acc

    def _oneof_check(self, m, o, fail):
        """plan.oneof_checks: a case that is neither 0 nor a member tag is REFUSED before any
        write (a host generated against a newer descriptor must be loud)."""
        for oname, tags in m.oneof_checks:
            cf = N.oneof_case_field(oname)
            conds = " && ".join("(int)m.%s != %d" % (cf, t) for t in tags)
            o += "        if ((int)m.%s != 0 && %s) { %s }" % (cf, conds, fail)

    # ======================================================== ENCODE, one pass
    def emit_write(self, o, m):
        o += "    public static void Write%s(ref Enc e, %s m)" % (m.name, m.name)
        o += "    {"
        self._oneof_check(m, o, "e.Err = W.ErrAbi; return;")
        for st in m.encode:
            self._write_step(o, m, st)
        o += "    }"
        o += ""

    def _write_step(self, o, m, st):
        p = "        "
        if st.op == "unknown_tail":
            o += "%sif (m.%s != null) e.Raw(m.%s);   // plan: unknown_tail, after every known field" % (p, BAG, BAG)
            return
        f = st.field
        acc = "m." + N.field(f.name)
        if st.op in ("scalar", "blob", "child", "oneof_member"):
            cond = self._cond(m, st, acc)
            val = self._val(st, acc)
            if f.kind == "message":
                s = self.site(m.name, f, "oneof message" if st.op == "oneof_member" else "message")
                o += "%sif (%s)" % (p, cond)
                o += "%s{" % p
                o += "%s    var mk = e.Begin(%d, %d);" % (p, f.tag, s)
                o += "%s    Write%s(ref e, %s);" % (p, f.of, val)
                o += "%s    e.End(mk);" % p
                o += "%s}" % p
            elif f.kind == "string":
                s = self.site(m.name, f, "string")
                o += "%sif (%s) e.StringField(%d, %s, %d);" % (p, cond, f.tag, val, s)
            elif f.kind == "bytes":
                o += "%sif (%s) e.BlobField(%d, %s);" % (p, cond, f.tag, val)
            else:
                o += "%sif (%s) %s" % (p, cond, _write_value(f, f.tag, val))
            return
        if st.op == "packed":
            s = self.site(m.name, f, "packed run")
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            o += "%s    var mk = e.Begin(%d, %d);" % (p, f.tag, s)
            o += "%s    for (int i = 0; i < %s.Count; i++) %s" % (p, acc, _run_value(f, "%s[i]" % acc))
            o += "%s    e.End(mk);" % p
            o += "%s}" % p
            return
        if st.op == "repeated_blob":
            if f.kind == "string":
                s = self.site(m.name, f, "repeated string")
                o += "%sfor (int i = 0; i < %s.Count; i++) e.StringField(%d, %s[i] ?? \"\", %d);" % (
                    p, acc, f.tag, acc, s)
            else:
                o += "%sfor (int i = 0; i < %s.Count; i++) e.BlobField(%d, %s[i] ?? W.EmptyBytes);" % (
                    p, acc, f.tag, acc)
            return
        if st.op == "repeated_message":
            s = self.site(m.name, f, "repeated message")
            o += "%sfor (int i = 0; i < %s.Count; i++)" % (p, acc)
            o += "%s{" % p
            o += "%s    var mk = e.Begin(%d, %d);" % (p, f.tag, s)
            o += "%s    Write%s(ref e, %s[i]);" % (p, f.of, acc)
            o += "%s    e.End(mk);" % p
            o += "%s}" % p
            return
        if st.op == "map":
            entry = self.p.msg(f.entry)
            s = self.site(m.name, f, "map entry")
            o += "%sfor (int i = 0; i < %s.Count; i++)" % (p, acc)
            o += "%s{" % p
            o += "%s    var kv = %s.At(i);" % (p, acc)
            o += "%s    var mk = e.Begin(%d, %d);" % (p, f.tag, s)
            for est in self._entry_steps(entry):
                x = "kv.Key" if est.field.name == "key" else "kv.Value"
                es = self.site(m.name, f, "map " + est.field.name)
                o += "%s    if (%s != null && %s.Length != 0) e.StringField(%d, %s, %d);" % (
                    p, x, x, est.field.tag, x, es)
            o += "%s    e.End(mk);" % p
            o += "%s}" % p
            return
        raise NotImplementedError("encode step %r (%s)" % (st.op, m.name))

    def _entry_steps(self, entry):
        """The pair message's OWN encode plan: key 1 then value 2, each an implicit blob
        (omitted when empty, the canonical form). A facade map entry has no bag."""
        out = []
        for est in entry.encode:
            if est.op == "unknown_tail":
                continue
            if est.op != "blob" or est.field.kind != "string" or est.field.presence != "implicit":
                raise NotImplementedError("map entry %s: step %r has no case" % (entry.name, est.op))
            out.append(est)
        return out

    # ======================================================== ENCODE, two pass
    def emit_size(self, o, m):
        o += "    public static int SizeOf%s(%s m)" % (m.name, m.name)
        o += "    {"
        o += "        int n = 0;"
        for st in m.encode:
            self._size_step(o, m, st)
        o += "        return n;"
        o += "    }"
        o += ""

    def _size_step(self, o, m, st):
        p = "        "
        if st.op == "unknown_tail":
            o += "%sif (m.%s != null) n += m.%s.Length;" % (p, BAG, BAG)
            return
        f = st.field
        acc = "m." + N.field(f.name)
        kl = _key_len(f.tag, f.wire)
        if st.op in ("scalar", "blob", "child", "oneof_member"):
            cond = self._cond(m, st, acc)
            val = self._val(st, acc)
            if f.kind == "message":
                o += "%sif (%s) { int b = SizeOf%s(%s); n += %s + W.VarintLen((ulong)(uint)b) + b; }" % (
                    p, cond, f.of, val, kl)
            elif f.kind == "string":
                o += "%sif (%s) { int L = Enc.Utf8Len(%s); n += %s + W.VarintLen((ulong)(uint)L) + L; }" % (
                    p, cond, val, kl)
            elif f.kind == "bytes":
                o += "%sif (%s) { int L = %s.Length; n += %s + W.VarintLen((ulong)(uint)L) + L; }" % (
                    p, cond, val, kl)
            else:
                o += "%sif (%s) n += %s + %s;" % (p, cond, kl, _size_value(f, val))
            return
        if st.op == "packed":
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            o += "%s    int b = 0;" % p
            o += "%s    for (int i = 0; i < %s.Count; i++) b += %s;" % (p, acc, _size_value(f, "%s[i]" % acc))
            o += "%s    n += %s + W.VarintLen((ulong)(uint)b) + b;" % (p, kl)
            o += "%s}" % p
            return
        if st.op == "repeated_blob":
            L = "Enc.Utf8Len(%s[i] ?? \"\")" % acc if f.kind == "string" else "(%s[i] ?? W.EmptyBytes).Length" % acc
            o += "%sfor (int i = 0; i < %s.Count; i++) { int L = %s; n += %s + W.VarintLen((ulong)(uint)L) + L; }" % (
                p, acc, L, kl)
            return
        if st.op == "repeated_message":
            o += "%sfor (int i = 0; i < %s.Count; i++) { int b = SizeOf%s(%s[i]); n += %s + W.VarintLen((ulong)(uint)b) + b; }" % (
                p, acc, f.of, acc, kl)
            return
        if st.op == "map":
            entry = self.p.msg(f.entry)
            o += "%sfor (int i = 0; i < %s.Count; i++)" % (p, acc)
            o += "%s{" % p
            o += "%s    var kv = %s.At(i);" % (p, acc)
            o += "%s    int b = 0;" % p
            for est in self._entry_steps(entry):
                x = "kv.Key" if est.field.name == "key" else "kv.Value"
                o += "%s    if (%s != null && %s.Length != 0) { int L = Enc.Utf8Len(%s); b += %s + W.VarintLen((ulong)(uint)L) + L; }" % (
                    p, x, x, x, _key_len(est.field.tag, LEN))
            o += "%s    n += %s + W.VarintLen((ulong)(uint)b) + b;" % (p, kl)
            o += "%s}" % p
            return
        raise NotImplementedError("size step %r (%s)" % (st.op, m.name))

    def emit_write_sized(self, o, m):
        o += "    public static void WriteSized%s(ref Enc e, %s m)" % (m.name, m.name)
        o += "    {"
        self._oneof_check(m, o, "e.Err = W.ErrAbi; return;")
        for st in m.encode:
            self._sized_step(o, m, st)
        o += "    }"
        o += ""

    def _sized_step(self, o, m, st):
        p = "        "
        if st.op == "unknown_tail":
            o += "%sif (m.%s != null) e.Raw(m.%s);" % (p, BAG, BAG)
            return
        f = st.field
        acc = "m." + N.field(f.name)
        if st.op in ("scalar", "blob", "child", "oneof_member"):
            cond = self._cond(m, st, acc)
            val = self._val(st, acc)
            if f.kind == "message":
                o += "%sif (%s) { var c = %s; e.SizedHeader(%d, SizeOf%s(c)); WriteSized%s(ref e, c); }" % (
                    p, cond, val, f.tag, f.of, f.of)
            elif f.kind == "string":
                o += "%sif (%s) { var s = %s; int L = Enc.Utf8Len(s); e.SizedHeader(%d, L); e.StringBodySized(s, L); }" % (
                    p, cond, val, f.tag)
            elif f.kind == "bytes":
                o += "%sif (%s) e.BlobField(%d, %s);" % (p, cond, f.tag, val)
            else:
                o += "%sif (%s) %s" % (p, cond, _write_value(f, f.tag, val))
            return
        if st.op == "packed":
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            o += "%s    int b = 0;" % p
            o += "%s    for (int i = 0; i < %s.Count; i++) b += %s;" % (p, acc, _size_value(f, "%s[i]" % acc))
            o += "%s    e.SizedHeader(%d, b);" % (p, f.tag)
            o += "%s    for (int i = 0; i < %s.Count; i++) %s" % (p, acc, _run_value(f, "%s[i]" % acc))
            o += "%s}" % p
            return
        if st.op == "repeated_blob":
            if f.kind == "string":
                o += "%sfor (int i = 0; i < %s.Count; i++) { var s = %s[i] ?? \"\"; int L = Enc.Utf8Len(s); e.SizedHeader(%d, L); e.StringBodySized(s, L); }" % (
                    p, acc, acc, f.tag)
            else:
                o += "%sfor (int i = 0; i < %s.Count; i++) e.BlobField(%d, %s[i] ?? W.EmptyBytes);" % (
                    p, acc, f.tag, acc)
            return
        if st.op == "repeated_message":
            o += "%sfor (int i = 0; i < %s.Count; i++) { e.SizedHeader(%d, SizeOf%s(%s[i])); WriteSized%s(ref e, %s[i]); }" % (
                p, acc, f.tag, f.of, acc, f.of, acc)
            return
        if st.op == "map":
            entry = self.p.msg(f.entry)
            steps = self._entry_steps(entry)
            o += "%sfor (int i = 0; i < %s.Count; i++)" % (p, acc)
            o += "%s{" % p
            o += "%s    var kv = %s.At(i);" % (p, acc)
            o += "%s    int b = 0;" % p
            for est in steps:
                x = "kv.Key" if est.field.name == "key" else "kv.Value"
                v = "L_" + est.field.name
                o += "%s    int %s = (%s != null && %s.Length != 0) ? Enc.Utf8Len(%s) : -1;" % (p, v, x, x, x)
                o += "%s    if (%s >= 0) b += %s + W.VarintLen((ulong)(uint)%s) + %s;" % (
                    p, v, _key_len(est.field.tag, LEN), v, v)
            o += "%s    e.SizedHeader(%d, b);" % (p, f.tag)
            for est in steps:
                x = "kv.Key" if est.field.name == "key" else "kv.Value"
                v = "L_" + est.field.name
                o += "%s    if (%s >= 0) { e.SizedHeader(%d, %s); e.StringBodySized(%s, %s); }" % (
                    p, v, est.field.tag, v, x, v)
            o += "%s}" % p
            return
        raise NotImplementedError("sized step %r (%s)" % (st.op, m.name))

    # ======================================================== DECODE
    def _capture(self):
        """The unknown-field arm of the dispatch, per Options.unknown."""
        skip = "d.Skip((int)tag, wire, %d);" % self.limit
        grab = "if (d.Err == 0) m.%s = W.Append(m.%s, d.Buf, s0, d.Pos - s0);" % (BAG, BAG)
        if self.retain == "drop":
            return ["// plan (drop): an unknown field -- including a known number at a wire type",
                    "// the table has no entry for (R-E2) -- is skipped.",
                    skip]
        if self.retain == "retain":
            return ["// plan (retain): an unknown field is captured verbatim, key included.",
                    skip, grab]
        return ["// plan (both): skipped, and captured verbatim (key included) when the host",
                "// asked for retention on this reader (Dec.Retain).",
                skip, "if (d.Retain) { %s }" % grab]

    def emit_read(self, o, m):
        o += "    public static void Read%s(ref Dec d, %s m, int depth)" % (m.name, m.name)
        o += "    {"
        o += "        // plan: a message more than %d levels below the root is refused." % self.limit
        o += "        if (depth > Limit) { d.Err = W.ErrDepth; return; }"
        o += "        while (d.Pos < d.End && d.Err == 0)"
        o += "        {"
        o += "            int s0 = d.Pos;"
        o += "            ulong k = d.Varint();"
        o += "            if (d.Err != 0) return;"
        o += "            uint tag = (uint)(k >> 3);"
        o += "            int wire = (int)(k & 7UL);"
        o += "            // plan: field number 0 is malformed, on every message."
        o += "            if (tag == 0) { d.Err = W.ErrMalformed; return; }"
        o += "            switch (((ulong)tag << 3) | (uint)wire)"
        o += "            {"
        for (tag, wire), act in sorted(m.decode.items()):
            self._read_action(o, m, tag, wire, act)
        o += "                default:"
        o += "                {"
        for ln in self._capture():
            o += "                    " + ln
        o += "                    break;"
        o += "                }"
        o += "            }"
        o += "        }"
        o += "    }"
        o += ""

    def _sub(self, o, b, call):
        """Read a nested body with the reader's End narrowed to it."""
        o += "%sint e2 = d.LenEnd(); if (d.Err != 0) return;" % b
        o += "%sint save = d.End; d.End = e2;" % b
        o += "%s%s" % (b, call)
        o += "%sd.End = save;" % b
        o += "%sif (d.Err != 0) return;" % b

    def _str(self, f):
        if f.kind == "string":
            # plan Options.utf8 (R-E7): the policy is the plan's, not the runtime's.
            return "d.%s()" % self.strf
        return "d.Bytes()"

    def _read_action(self, o, m, tag, wire, act):
        f = act.field
        op = act.op
        acc = "m." + N.field(f.name)
        o += "                case %dUL: // (%d, %d) %s %s" % ((tag << 3) | wire, tag, wire, op, f.name)
        o += "                {"
        b = "                    "
        if op == "set_scalar":
            o += "%svar v = %s; if (d.Err != 0) return;" % (b, _read(f))
            o += "%s%s = v;" % (b, acc)
        elif op == "set_blob":
            o += "%svar v = %s; if (d.Err != 0) return;" % (b, self._str(f))
            o += "%s%s = v;" % (b, acc)
        elif op == "merge_child":
            o += "%s// plan: a repeated occurrence MERGES into the one already decoded." % b
            o += "%svar c = %s ?? new %s();" % (b, acc, f.of)
            self._sub(o, b, "Read%s(ref d, c, depth + 1);" % f.of)
            o += "%s%s = c;" % (b, acc)
        elif op == "append_message":
            o += "%svar c = new %s();" % (b, f.of)
            self._sub(o, b, "Read%s(ref d, c, depth + 1);" % f.of)
            o += "%s%s.Add(c);" % (b, acc)
        elif op == "append_blob":
            o += "%svar v = %s; if (d.Err != 0) return;" % (b, self._str(f))
            o += "%s%s.Add(v);" % (b, acc)
        elif op == "packed_run":
            o += "%sint e2 = d.LenEnd(); if (d.Err != 0) return;" % b
            o += "%sint save = d.End; d.End = e2;" % b
            o += "%swhile (d.Pos < d.End) { var v = %s; if (d.Err != 0) break; %s.Add(v); }" % (b, _read(f), acc)
            o += "%sd.End = save;" % b
            o += "%sif (d.Err != 0) return;" % b
        elif op == "packed_one":
            o += "%s// plan: the unpacked form, at the kind's own wire type only." % b
            o += "%svar v = %s; if (d.Err != 0) return;" % (b, _read(f))
            o += "%s%s.Add(v);" % (b, acc)
        elif op == "map_entry":
            entry = self.p.msg(f.entry)
            o += "%sint e2 = d.LenEnd(); if (d.Err != 0) return;" % b
            o += "%sif (depth + 1 > Limit) { d.Err = W.ErrDepth; return; }" % b
            o += "%sint save = d.End; d.End = e2;" % b
            o += '%sstring mk = "", mv = "";' % b
            o += "%swhile (d.Pos < d.End && d.Err == 0)" % b
            o += "%s{" % b
            o += "%s    ulong k2 = d.Varint(); if (d.Err != 0) break;" % b
            o += "%s    uint t2 = (uint)(k2 >> 3); int w2 = (int)(k2 & 7UL);" % b
            o += "%s    if (t2 == 0) { d.Err = W.ErrMalformed; break; }" % b
            o += "%s    switch (((ulong)t2 << 3) | (uint)w2)" % b
            o += "%s    {" % b
            for (etag, ewire), eact in sorted(entry.decode.items()):
                ef = eact.field
                if eact.op != "set_blob" or ef.kind != "string":
                    raise NotImplementedError("map entry %s: action %r has no case" % (entry.name, eact.op))
                x = "mk" if ef.name == "key" else "mv"
                o += "%s        case %dUL: %s = %s; break;" % (b, (etag << 3) | ewire, x, self._str(ef))
            o += "%s        // A facade map entry has no bag: an unknown field inside an entry" % b
            o += "%s        // is skipped in every mode." % b
            o += "%s        default: d.Skip((int)t2, w2, %d); break;" % (b, self.limit)
            o += "%s    }" % b
            o += "%s}" % b
            o += "%sd.End = save;" % b
            o += "%sif (d.Err != 0) return;" % b
            o += "%s// plan: a duplicate key replaces the earlier value." % b
            o += "%s%s[mk] = mv;" % (b, acc)
        elif op == "oneof_set":
            cf = "m." + N.oneof_case_field(act.oneof)
            cv = "%s.%s" % (N.oneof_case_type(m.name, act.oneof), N.pascal(f.name))
            if f.kind == "message":
                o += "%s// plan: the SAME member merges; any other starts from empty." % b
                o += "%svar c = (%s == %s && %s != null) ? %s : new %s();" % (b, cf, cv, acc, acc, f.of)
                self._sub(o, b, "Read%s(ref d, c, depth + 1);" % f.of)
                o += "%s%s = %s; %s = c;" % (b, cf, cv, acc)
            else:
                rd = self._str(f) if f.is_blob else _read(f)
                o += "%svar v = %s; if (d.Err != 0) return;" % (b, rd)
                o += "%s%s = %s; %s = v;" % (b, cf, cv, acc)
        else:
            raise NotImplementedError("decode action %r (%s.%s)" % (op, m.name, f.name))
        o += "%sbreak;" % b
        o += "                }"


def emit(x, ns, extra_using=()):
    p = as_plan(x)
    c = Codec(p)
    o = N.Head("The managed codec: the plan's encode and decode plans over the facade.",
               p.source, "cs_managed")
    o += "using System;"
    o += "using System.Collections.Generic;"
    for u in extra_using:
        o += "using %s;" % u
    o += ""
    o += "namespace %s;" % ns
    o += ""
    o += "public static class Codec"
    o += "{"
    o += "    /// The plan's options, for a log to name (Options: %r)." % p.options
    o += "    public const string Utf8Policy = \"%s\";" % p.options.utf8
    o += "    public const string UnknownMode = \"%s\";" % p.options.unknown
    o += "    public const int Limit = %d;" % p.options.recursion_limit
    o += ""
    msgs = facade_messages(p)
    for m in msgs:
        c.emit_write(o, m)
    for m in msgs:
        c.emit_size(o, m)
    for m in msgs:
        c.emit_write_sized(o, m)
    for m in msgs:
        c.emit_read(o, m)
    o += "    /// One learned width per length-prefix site (ABI v1 section 6)."
    o += "    public const int Sites = %d;" % len(c.sites)
    o += ""
    o += "    /// Which site is which, so a prefix-miss count can name the field."
    o += "    public static readonly string[] SiteNames ="
    o += "    {"
    for s in c.sites:
        o += '        "%s",' % s
    o += "    };"
    o += "}"
    return str(o), c.sites
