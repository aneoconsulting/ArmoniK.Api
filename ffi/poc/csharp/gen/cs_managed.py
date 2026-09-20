"""Backend: the managed control codec (README R3's no-boundary arm).

A generated pure-C# codec over the facade, **encode and decode**. The C# report
this slice rebuilds against had a managed ENCODE control and no managed decode
control, and the Java report names that missing decode control as the one
measurement that would change its own recommendation.

Four things are emitted per message, and the last two exist to keep the encode
headline honest rather than to add an arm for its own sake:

  Write<M>      one pass, length prefixes resolved after the body is written
                through the learned width of ABI v1 section 6. This is the
                design under test.
  SizeOf<M>     the encoded body size.
  WriteSized<M> two passes, exact prefixes, no learned width and no move. This
                is the shape `Google.Protobuf` uses, so `Write` against
                `WriteSized` is the single-pass term on its own, inside one
                process, as a within-arm delta rather than a ratio to a third
                arm (R4).
  Read<M>       decode into the facade.

**Every length-prefix site gets an index**, allocated here, and the count is
emitted as `Codec.Sites`. The learned-width table is per encode context and
sized from it: ABI v1 section 6 refuses a process-global table.
"""
import csnames as N
from cs_facade import Head, oneof_case_type

SIGNED = {"int32": "int", "int64": "long"}


class Codec:
    def __init__(self, ir):
        self.ir = ir
        self.sites = []            # (message, field, what) per site index, for the log

    def site(self, m, f, what):
        self.sites.append("%s.%s (%s)" % (m.cs, f.name, what))
        return len(self.sites) - 1

    # ---- expressions ----------------------------------------------------

    def varint_of(self, f, expr):
        """A field value as the u64 a varint carries.

        int32 and int64 are sign extended to 64 bits before encoding, which is
        why a negative int32 is a ten-byte varint on the wire. Nothing in the
        payload set is negative, so this line is not exercised by the corpus and
        is written from the specification rather than from a measurement.
        """
        if f.kind == "bool":
            return "(%s ? 1UL : 0UL)" % expr
        if f.kind == "enum":
            return "(ulong)(long)(int)(%s)" % expr
        if f.kind == "int32":
            return "(ulong)(long)(%s)" % expr
        return "(ulong)(%s)" % expr

    def scalar_from_varint(self, f, expr):
        if f.kind == "bool":
            return "(%s) != 0UL" % expr
        if f.kind == "enum":
            return "(%s)(int)(long)(%s)" % (f.of, expr)
        if f.kind == "int32":
            return "(int)(long)(%s)" % expr
        return "(long)(%s)" % expr

    def nonzero(self, f, expr):
        if f.kind == "bool":
            return expr
        if f.kind == "double":
            # NOT `!= 0.0`. IEEE says -0.0 == 0.0, so the value test omits a
            # minus zero that is a SET field; upb writes it and this used to
            # drop it. `ffi/corpus`'s S-double-minus-zero is the vector, and its
            # own note is "the omit-when-zero rule must compare bits and not
            # value". Google.Protobuf's generated C# writes `!= 0D` and so has
            # the same hole; this slice's managed codec no longer does.
            return "BitConverter.DoubleToInt64Bits(%s) != 0L" % expr
        if f.kind == "fixed32":
            return "%s != 0u" % expr
        if f.kind == "enum":
            return "%s != 0" % expr
        return "%s != 0" % expr

    # ================= ENCODE, one pass =================================

    def emit_write(self, o, m):
        o += "    public static void Write%s(ref Enc e, %s m)" % (m.cs, m.cs)
        o += "    {"
        for f in m.sorted_wire():
            if f.oneof:
                continue
            self.write_field(o, m, f)
        for oname, members in m.oneofs.items():
            self.write_oneof(o, m, oname, members)
        o += "    }"
        o += ""

    def write_field(self, o, m, f, indent="        "):
        p = indent
        acc = "m." + f.cs
        if f.card == "map":
            s = self.site(m, f, "map entry")
            o += "%s{" % p
            o += "%s    int n = %s.Count;" % (p, acc)
            o += "%s    for (int i = 0; i < n; i++)" % p
            o += "%s    {" % p
            o += "%s        var kv = %s.At(i);" % (p, acc)
            o += "%s        var mk = e.Begin(%d, %d);" % (p, f.tag, s)
            o += "%s        if (kv.Key.Length != 0) e.StringField(1, kv.Key, %d);" % (
                p, self.site(m, f, "map key"))
            o += "%s        if (kv.Value.Length != 0) e.StringField(2, kv.Value, %d);" % (
                p, self.site(m, f, "map value"))
            o += "%s        e.End(mk);" % p
            o += "%s    }" % p
            o += "%s}" % p
            return

        if f.card == "packed":
            s = self.site(m, f, "packed run")
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            o += "%s    var mk = e.Begin(%d, %d);" % (p, f.tag, s)
            if f.kind == "double":
                o += "%s    for (int i = 0; i < %s.Count; i++) e.F64(%s[i]);" % (p, acc, acc)
            elif f.kind == "fixed32":
                o += "%s    for (int i = 0; i < %s.Count; i++) e.Fixed32(%s[i]);" % (p, acc, acc)
            else:
                o += "%s    for (int i = 0; i < %s.Count; i++) e.Varint(%s);" % (
                    p, acc, self.varint_of(f, "%s[i]" % acc))
            o += "%s    e.End(mk);" % p
            o += "%s}" % p
            return

        if f.card == "repeated":
            if f.kind == "string":
                s = self.site(m, f, "repeated string")
                o += "%sfor (int i = 0; i < %s.Count; i++) e.StringField(%d, %s[i], %d);" % (
                    p, acc, f.tag, acc, s)
                return
            s = self.site(m, f, "repeated message")
            o += "%sfor (int i = 0; i < %s.Count; i++)" % (p, acc)
            o += "%s{" % p
            o += "%s    var mk = e.Begin(%d, %d);" % (p, f.tag, s)
            o += "%s    Write%s(ref e, %s[i]);" % (p, f.of, acc)
            o += "%s    e.End(mk);" % p
            o += "%s}" % p
            return

        # ---- singular ---------------------------------------------------
        if f.kind == "message":
            s = self.site(m, f, "message")
            o += "%sif (%s != null)" % (p, acc)
            o += "%s{" % p
            o += "%s    var mk = e.Begin(%d, %d);" % (p, f.tag, s)
            o += "%s    Write%s(ref e, %s);" % (p, f.of, acc)
            o += "%s    e.End(mk);" % p
            o += "%s}" % p
            return

        if f.kind == "string":
            s = self.site(m, f, "string")
            if f.presence == "explicit":
                o += "%s// explicit presence: written when set, empty or not" % p
                o += "%sif (%s != null) e.StringField(%d, %s, %d);" % (p, acc, f.tag, acc, s)
            else:
                o += "%sif (%s.Length != 0) e.StringField(%d, %s, %d);" % (p, acc, f.tag, acc, s)
            return

        if f.kind == "bytes":
            if f.presence == "explicit":
                o += "%sif (%s != null) e.BlobField(%d, %s);" % (p, acc, f.tag, acc)
            else:
                o += "%sif (%s.Length != 0) e.BlobField(%d, %s);" % (p, acc, f.tag, acc)
            return

        # scalar or enum
        if f.presence == "explicit":
            o += "%sif (%s.HasValue)" % (p, acc)
            if f.kind == "double":
                o += "%s    e.F64Field(%d, %s.Value);" % (p, f.tag, acc)
            elif f.kind == "fixed32":
                o += "%s    e.Fixed32Field(%d, %s.Value);" % (p, f.tag, acc)
            else:
                o += "%s    e.VarintField(%d, %s);" % (
                    p, f.tag, self.varint_of(f, acc + ".Value"))
            return
        if f.kind == "double":
            o += "%sif (%s) e.F64Field(%d, %s);" % (p, self.nonzero(f, acc), f.tag, acc)
        elif f.kind == "fixed32":
            o += "%sif (%s) e.Fixed32Field(%d, %s);" % (p, self.nonzero(f, acc), f.tag, acc)
        else:
            o += "%sif (%s) e.VarintField(%d, %s);" % (
                p, self.nonzero(f, acc), f.tag, self.varint_of(f, acc))

    def write_oneof(self, o, m, oname, members):
        p = "        "
        ct = oneof_case_type(m, oname)
        o += "%sswitch (m.%sCase)" % (p, N.pascal(oname))
        o += "%s{" % p
        for f in members:
            o += "%s    case %s.%s:" % (p, ct, N.pascal(f.name))
            acc = "m." + f.cs
            # A oneof member is explicit presence: written when selected, zero
            # or not. The by-value group of ABI v1 cannot reach this shape,
            # which is why design/SHAPES.md puts it on the list.
            if f.kind == "message":
                s = self.site(m, f, "oneof message")
                o += "%s    {" % p
                o += "%s        var mk = e.Begin(%d, %d);" % (p, f.tag, s)
                o += "%s        Write%s(ref e, %s);" % (p, f.of, acc)
                o += "%s        e.End(mk);" % p
                o += "%s        break;" % p
                o += "%s    }" % p
                continue
            if f.kind == "string":
                s = self.site(m, f, "oneof string")
                o += "%s        e.StringField(%d, %s, %d);" % (p, f.tag, acc, s)
            elif f.kind == "bytes":
                o += "%s        e.BlobField(%d, %s);" % (p, f.tag, acc)
            elif f.kind == "double":
                o += "%s        e.F64Field(%d, %s);" % (p, f.tag, acc)
            elif f.kind == "fixed32":
                o += "%s        e.Fixed32Field(%d, %s);" % (p, f.tag, acc)
            else:
                o += "%s        e.VarintField(%d, %s);" % (p, f.tag, self.varint_of(f, acc))
            o += "%s        break;" % p
        o += "%s    default: break;" % p
        o += "%s}" % p

    # ================= ENCODE, two pass =================================

    def emit_size(self, o, m):
        o += "    public static int SizeOf%s(%s m)" % (m.cs, m.cs)
        o += "    {"
        o += "        int n = 0;"
        for f in m.sorted_wire():
            if f.oneof:
                continue
            self.size_field(o, m, f)
        for oname, members in m.oneofs.items():
            self.size_oneof(o, m, oname, members)
        o += "        return n;"
        o += "    }"
        o += ""

    def keylen(self, f):
        from ir import WIRE_LEN
        return "W.VarintLen(%dUL)" % f.key

    def size_field(self, o, m, f):
        p = "        "
        acc = "m." + f.cs
        kl = self.keylen(f)

        if f.card == "map":
            o += "%s{" % p
            o += "%s    int c = %s.Count;" % (p, acc)
            o += "%s    for (int i = 0; i < c; i++)" % p
            o += "%s    {" % p
            o += "%s        var kv = %s.At(i);" % (p, acc)
            o += "%s        int b = 0;" % p
            o += "%s        if (kv.Key.Length != 0) { int L = Enc.Utf8Len(kv.Key); b += 1 + W.VarintLen((ulong)(uint)L) + L; }" % p
            o += "%s        if (kv.Value.Length != 0) { int L = Enc.Utf8Len(kv.Value); b += 1 + W.VarintLen((ulong)(uint)L) + L; }" % p
            o += "%s        n += %s + W.VarintLen((ulong)(uint)b) + b;" % (p, kl)
            o += "%s    }" % p
            o += "%s}" % p
            return

        if f.card == "packed":
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            if f.kind == "double":
                o += "%s    int b = %s.Count * 8;" % (p, acc)
            elif f.kind == "fixed32":
                o += "%s    int b = %s.Count * 4;" % (p, acc)
            else:
                o += "%s    int b = 0;" % p
                o += "%s    for (int i = 0; i < %s.Count; i++) b += W.VarintLen(%s);" % (
                    p, acc, self.varint_of(f, "%s[i]" % acc))
            o += "%s    n += %s + W.VarintLen((ulong)(uint)b) + b;" % (p, kl)
            o += "%s}" % p
            return

        if f.card == "repeated":
            if f.kind == "string":
                o += "%sfor (int i = 0; i < %s.Count; i++) { int L = Enc.Utf8Len(%s[i]); n += %s + W.VarintLen((ulong)(uint)L) + L; }" % (
                    p, acc, acc, kl)
                return
            o += "%sfor (int i = 0; i < %s.Count; i++) { int b = SizeOf%s(%s[i]); n += %s + W.VarintLen((ulong)(uint)b) + b; }" % (
                p, acc, f.of, acc, kl)
            return

        if f.kind == "message":
            o += "%sif (%s != null) { int b = SizeOf%s(%s); n += %s + W.VarintLen((ulong)(uint)b) + b; }" % (
                p, acc, f.of, acc, kl)
            return
        if f.kind == "string":
            cond = "%s != null" % acc if f.presence == "explicit" else "%s.Length != 0" % acc
            o += "%sif (%s) { int L = Enc.Utf8Len(%s); n += %s + W.VarintLen((ulong)(uint)L) + L; }" % (
                p, cond, acc, kl)
            return
        if f.kind == "bytes":
            cond = "%s != null" % acc if f.presence == "explicit" else "%s.Length != 0" % acc
            o += "%sif (%s) n += %s + W.VarintLen((ulong)(uint)%s.Length) + %s.Length;" % (
                p, cond, kl, acc, acc)
            return
        if f.presence == "explicit":
            if f.kind == "double":
                o += "%sif (%s.HasValue) n += %s + 8;" % (p, acc, kl)
            elif f.kind == "fixed32":
                o += "%sif (%s.HasValue) n += %s + 4;" % (p, acc, kl)
            else:
                o += "%sif (%s.HasValue) n += %s + W.VarintLen(%s);" % (
                    p, acc, kl, self.varint_of(f, acc + ".Value"))
            return
        if f.kind == "double":
            o += "%sif (%s) n += %s + 8;" % (p, self.nonzero(f, acc), kl)
        elif f.kind == "fixed32":
            o += "%sif (%s) n += %s + 4;" % (p, self.nonzero(f, acc), kl)
        else:
            o += "%sif (%s) n += %s + W.VarintLen(%s);" % (
                p, self.nonzero(f, acc), kl, self.varint_of(f, acc))

    def size_oneof(self, o, m, oname, members):
        p = "        "
        ct = oneof_case_type(m, oname)
        o += "%sswitch (m.%sCase)" % (p, N.pascal(oname))
        o += "%s{" % p
        for f in members:
            acc = "m." + f.cs
            kl = self.keylen(f)
            o += "%s    case %s.%s:" % (p, ct, N.pascal(f.name))
            if f.kind == "message":
                o += "%s        { int b = SizeOf%s(%s); n += %s + W.VarintLen((ulong)(uint)b) + b; }" % (
                    p, f.of, acc, kl)
            elif f.kind == "string":
                o += "%s        { int L = Enc.Utf8Len(%s); n += %s + W.VarintLen((ulong)(uint)L) + L; }" % (
                    p, acc, kl)
            elif f.kind == "bytes":
                o += "%s        n += %s + W.VarintLen((ulong)(uint)%s.Length) + %s.Length;" % (
                    p, kl, acc, acc)
            elif f.kind == "double":
                o += "%s        n += %s + 8;" % (p, kl)
            elif f.kind == "fixed32":
                o += "%s        n += %s + 4;" % (p, kl)
            else:
                o += "%s        n += %s + W.VarintLen(%s);" % (p, kl, self.varint_of(f, acc))
            o += "%s        break;" % p
        o += "%s    default: break;" % p
        o += "%s}" % p

    def emit_write_sized(self, o, m):
        o += "    public static void WriteSized%s(ref Enc e, %s m)" % (m.cs, m.cs)
        o += "    {"
        for f in m.sorted_wire():
            if f.oneof:
                continue
            self.sized_field(o, m, f)
        for oname, members in m.oneofs.items():
            self.sized_oneof(o, m, oname, members)
        o += "    }"
        o += ""

    def sized_field(self, o, m, f):
        p = "        "
        acc = "m." + f.cs
        if f.card == "map":
            o += "%s{" % p
            o += "%s    int c = %s.Count;" % (p, acc)
            o += "%s    for (int i = 0; i < c; i++)" % p
            o += "%s    {" % p
            o += "%s        var kv = %s.At(i);" % (p, acc)
            o += "%s        int kL = kv.Key.Length != 0 ? Enc.Utf8Len(kv.Key) : -1;" % p
            o += "%s        int vL = kv.Value.Length != 0 ? Enc.Utf8Len(kv.Value) : -1;" % p
            o += "%s        int b = (kL >= 0 ? 1 + W.VarintLen((ulong)(uint)kL) + kL : 0)" % p
            o += "%s              + (vL >= 0 ? 1 + W.VarintLen((ulong)(uint)vL) + vL : 0);" % p
            o += "%s        e.SizedHeader(%d, b);" % (p, f.tag)
            o += "%s        if (kL >= 0) { e.SizedHeader(1, kL); e.StringBodySized(kv.Key, kL); }" % p
            o += "%s        if (vL >= 0) { e.SizedHeader(2, vL); e.StringBodySized(kv.Value, vL); }" % p
            o += "%s    }" % p
            o += "%s}" % p
            return
        if f.card == "packed":
            o += "%sif (%s.Count != 0)" % (p, acc)
            o += "%s{" % p
            if f.kind == "double":
                o += "%s    e.SizedHeader(%d, %s.Count * 8);" % (p, f.tag, acc)
                o += "%s    for (int i = 0; i < %s.Count; i++) e.F64(%s[i]);" % (p, acc, acc)
            elif f.kind == "fixed32":
                o += "%s    e.SizedHeader(%d, %s.Count * 4);" % (p, f.tag, acc)
                o += "%s    for (int i = 0; i < %s.Count; i++) e.Fixed32(%s[i]);" % (p, acc, acc)
            else:
                o += "%s    int b = 0;" % p
                o += "%s    for (int i = 0; i < %s.Count; i++) b += W.VarintLen(%s);" % (
                    p, acc, self.varint_of(f, "%s[i]" % acc))
                o += "%s    e.SizedHeader(%d, b);" % (p, f.tag)
                o += "%s    for (int i = 0; i < %s.Count; i++) e.Varint(%s);" % (
                    p, acc, self.varint_of(f, "%s[i]" % acc))
            o += "%s}" % p
            return
        if f.card == "repeated":
            if f.kind == "string":
                o += "%sfor (int i = 0; i < %s.Count; i++) { int L = Enc.Utf8Len(%s[i]); e.SizedHeader(%d, L); e.StringBodySized(%s[i], L); }" % (
                    p, acc, acc, f.tag, acc)
                return
            o += "%sfor (int i = 0; i < %s.Count; i++) { e.SizedHeader(%d, SizeOf%s(%s[i])); WriteSized%s(ref e, %s[i]); }" % (
                p, acc, f.tag, f.of, acc, f.of, acc)
            return
        if f.kind == "message":
            o += "%sif (%s != null) { e.SizedHeader(%d, SizeOf%s(%s)); WriteSized%s(ref e, %s); }" % (
                p, acc, f.tag, f.of, acc, f.of, acc)
            return
        if f.kind == "string":
            cond = "%s != null" % acc if f.presence == "explicit" else "%s.Length != 0" % acc
            o += "%sif (%s) { int L = Enc.Utf8Len(%s); e.SizedHeader(%d, L); e.StringBodySized(%s, L); }" % (
                p, cond, acc, f.tag, acc)
            return
        if f.kind == "bytes":
            cond = "%s != null" % acc if f.presence == "explicit" else "%s.Length != 0" % acc
            o += "%sif (%s) e.BlobField(%d, %s);" % (p, cond, f.tag, acc)
            return
        if f.presence == "explicit":
            if f.kind == "double":
                o += "%sif (%s.HasValue) e.F64Field(%d, %s.Value);" % (p, acc, f.tag, acc)
            elif f.kind == "fixed32":
                o += "%sif (%s.HasValue) e.Fixed32Field(%d, %s.Value);" % (p, acc, f.tag, acc)
            else:
                o += "%sif (%s.HasValue) e.VarintField(%d, %s);" % (
                    p, acc, f.tag, self.varint_of(f, acc + ".Value"))
            return
        if f.kind == "double":
            o += "%sif (%s) e.F64Field(%d, %s);" % (p, self.nonzero(f, acc), f.tag, acc)
        elif f.kind == "fixed32":
            o += "%sif (%s) e.Fixed32Field(%d, %s);" % (p, self.nonzero(f, acc), f.tag, acc)
        else:
            o += "%sif (%s) e.VarintField(%d, %s);" % (
                p, self.nonzero(f, acc), f.tag, self.varint_of(f, acc))

    def sized_oneof(self, o, m, oname, members):
        p = "        "
        ct = oneof_case_type(m, oname)
        o += "%sswitch (m.%sCase)" % (p, N.pascal(oname))
        o += "%s{" % p
        for f in members:
            acc = "m." + f.cs
            o += "%s    case %s.%s:" % (p, ct, N.pascal(f.name))
            if f.kind == "message":
                o += "%s        e.SizedHeader(%d, SizeOf%s(%s)); WriteSized%s(ref e, %s);" % (
                    p, f.tag, f.of, acc, f.of, acc)
            elif f.kind == "string":
                o += "%s        { int L = Enc.Utf8Len(%s); e.SizedHeader(%d, L); e.StringBodySized(%s, L); }" % (
                    p, acc, f.tag, acc)
            elif f.kind == "bytes":
                o += "%s        e.BlobField(%d, %s);" % (p, f.tag, acc)
            elif f.kind == "double":
                o += "%s        e.F64Field(%d, %s);" % (p, f.tag, acc)
            elif f.kind == "fixed32":
                o += "%s        e.Fixed32Field(%d, %s);" % (p, f.tag, acc)
            else:
                o += "%s        e.VarintField(%d, %s);" % (p, f.tag, self.varint_of(f, acc))
            o += "%s        break;" % p
        o += "%s    default: break;" % p
        o += "%s}" % p

    # ================= DECODE ============================================

    def emit_read(self, o, m):
        o += "    public static void Read%s(ref Dec d, %s m, int end)" % (m.cs, m.cs)
        o += "    {"
        o += "        // ABI v1 open decision 7, and protobuf's own limit. Without it a"
        o += "        // message nested 300 deep is 300 managed frames and a successful"
        o += "        // parse, where every protobuf implementation rejects past 100."
        o += "        // ffi/corpus X-depth-101 and X-depth-300."
        o += "        if (++d.Depth > W.MaxDepth) { d.Err = W.ErrDepth; d.Depth--; return; }"
        o += "        while (d.Pos < end && d.Err == 0)"
        o += "        {"
        o += "            ulong k = d.Varint();"
        o += "            int wire = (int)(k & 7UL);"
        o += "            // The TAG travels with the wire type into Skip, because the"
        o += "            // deprecated GROUP form carries no length and its end is an"
        o += "            // END_GROUP whose field number must MATCH. See Wire.cs."
        o += "            int tag = (int)(k >> 3);"
        o += "            // Field number 0 is not a legal tag, and it is the value a reader"
        o += "            // gets from an empty buffer it forgot to bounds-check -- so"
        o += "            // accepting it turns a truncation into a silently empty message."
        o += "            // ffi/corpus X-tag-zero."
        o += "            if (tag == 0) { d.Err = W.ErrMalformed; break; }"
        o += "            switch (tag)"
        o += "            {"
        for f in m.sorted_wire():
            self.read_field(o, m, f)
        o += "                default: d.Skip(tag, wire); break;"
        o += "            }"
        o += "        }"
        o += "        if (d.Pos != end && d.Err == 0) d.Err = W.ErrMalformed;"
        o += "        d.Depth--;"
        o += "    }"
        o += ""

    def read_field(self, o, m, f):
        p = "                "
        acc = "m." + f.cs
        o += "%scase %d:" % (p, f.tag)
        o += "%s{" % p
        b = p + "    "

        def guard(expected):
            o.lines.append("%sif (wire != %d) { d.Skip(%d, wire); break; }" % (b, expected, f.tag))

        if f.card == "map":
            guard(2)
            o += "%sint e2 = d.LenEnd(); if (d.Err != 0) break;" % b
            o += '%sstring mk = "", mv = "";' % b
            o += "%swhile (d.Pos < e2 && d.Err == 0)" % b
            o += "%s{" % b
            o += "%s    ulong k2 = d.Varint(); int w2 = (int)(k2 & 7UL);" % b
            o += "%s    if ((k2 >> 3) == 1 && w2 == 2) mk = d.Str();" % b
            o += "%s    else if ((k2 >> 3) == 2 && w2 == 2) mv = d.Str();" % b
            o += "%s    else d.Skip((int)(k2 >> 3), w2);" % b
            o += "%s}" % b
            o += "%sd.Pos = e2;" % b
            o += "%s%s[mk] = mv;" % (b, acc)
            o += "%sbreak;" % b
            o += "%s}" % p
            return

        if f.card == "packed":
            # Both forms accepted: a conformant parser takes a packed run and a
            # repeated single value for the same field, and a corpus generated
            # from one writer only ever contains one of them.
            o += "%sif (wire == 2)" % b
            o += "%s{" % b
            o += "%s    int e2 = d.LenEnd(); if (d.Err != 0) break;" % b
            if f.kind == "double":
                o += "%s    while (d.Pos < e2 && d.Err == 0) %s.Add(d.F64());" % (b, acc)
            elif f.kind == "fixed32":
                o += "%s    while (d.Pos < e2 && d.Err == 0) %s.Add(d.Fixed32());" % (b, acc)
            else:
                o += "%s    while (d.Pos < e2 && d.Err == 0) %s.Add(%s);" % (
                    b, acc, self.scalar_from_varint(f, "d.Varint()"))
            o += "%s    d.Pos = e2;" % b
            o += "%s}" % b
            if f.kind == "double":
                o += "%selse if (wire == 1) %s.Add(d.F64());" % (b, acc)
            elif f.kind == "fixed32":
                o += "%selse if (wire == 5) %s.Add(d.Fixed32());" % (b, acc)
            else:
                o += "%selse if (wire == 0) %s.Add(%s);" % (
                    b, acc, self.scalar_from_varint(f, "d.Varint()"))
            o += "%selse d.Skip(%d, wire);" % (b, f.tag)
            o += "%sbreak;" % b
            o += "%s}" % p
            return

        if f.card == "repeated":
            guard(2)
            if f.kind == "string":
                o += "%s%s.Add(d.Str());" % (b, acc)
            else:
                o += "%sint e2 = d.LenEnd(); if (d.Err != 0) break;" % b
                o += "%svar c = new %s();" % (b, f.of)
                o += "%sRead%s(ref d, c, e2);" % (b, f.of)
                o += "%sd.Pos = e2;" % b
                o += "%s%s.Add(c);" % (b, acc)
            o += "%sbreak;" % b
            o += "%s}" % p
            return

        # ---- singular -----------------------------------------------
        if f.kind == "message":
            guard(2)
            o += "%sint e2 = d.LenEnd(); if (d.Err != 0) break;" % b
            if f.oneof:
                ct = oneof_case_type(m, f.oneof)
                cf = N.pascal(f.oneof) + "Case"
                o += "%s// A repeated occurrence MERGES into the member already selected," % b
                o += "%s// and REPLACES a different one: protobuf's oneof merge rule." % b
                o += "%svar c = (m.%s == %s.%s && %s != null) ? %s : new %s();" % (
                    b, cf, ct, N.pascal(f.name), acc, acc, f.of)
                o += "%sRead%s(ref d, c, e2);" % (b, f.of)
                o += "%sd.Pos = e2;" % b
                o += "%sm.%s = %s.%s; %s = c;" % (b, cf, ct, N.pascal(f.name), acc)
            else:
                o += "%svar c = %s ?? new %s();" % (b, acc, f.of)
                o += "%sRead%s(ref d, c, e2);" % (b, f.of)
                o += "%sd.Pos = e2;" % b
                o += "%s%s = c;" % (b, acc)
            o += "%sbreak;" % b
            o += "%s}" % p
            return

        if f.kind in ("string", "bytes"):
            guard(2)
            val = "d.Str()" if f.kind == "string" else "d.Bytes()"
            self.assign(o, b, m, f, val)
            o += "%sbreak;" % b
            o += "%s}" % p
            return

        if f.kind == "double":
            guard(1)
            self.assign(o, b, m, f, "d.F64()")
            o += "%sbreak;" % b
            o += "%s}" % p
            return
        if f.kind == "fixed32":
            guard(5)
            self.assign(o, b, m, f, "d.Fixed32()")
            o += "%sbreak;" % b
            o += "%s}" % p
            return

        guard(0)
        self.assign(o, b, m, f, self.scalar_from_varint(f, "d.Varint()"))
        o += "%sbreak;" % b
        o += "%s}" % p

    def assign(self, o, b, m, f, val):
        acc = "m." + f.cs
        if f.oneof:
            ct = oneof_case_type(m, f.oneof)
            cf = N.pascal(f.oneof) + "Case"
            o += "%sm.%s = %s.%s; %s = %s;" % (b, cf, ct, N.pascal(f.name), acc, val)
        else:
            o += "%s%s = %s;" % (b, acc, val)


def emit(ir, ns="Armonik.Ffi.Facade", extra_using=None):
    c = Codec(ir)
    o = Head("The managed control codec: a generated pure-C# codec over the facade.")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += ""
    for u in (extra_using or []):
        o += "using %s;" % u
    o += ""
    o += "namespace %s;" % ns
    o += ""
    o += "public static class Codec"
    o += "{"
    for m in ir.messages.values():
        c.emit_write(o, m)
    for m in ir.messages.values():
        c.emit_size(o, m)
    for m in ir.messages.values():
        c.emit_write_sized(o, m)
    for m in ir.messages.values():
        c.emit_read(o, m)

    o += "    /// One learned width per length-prefix site (ABI v1 section 6)."
    o += "    public const int Sites = %d;" % len(c.sites)
    o += ""
    o += "    /// Which site is which, so a prefix-miss count can name the field"
    o += "    /// rather than the message. ABI v1 open decision 5 asks what the"
    o += "    /// learned width is worth, and an aggregate that cannot say where"
    o += "    /// it thrashes does not answer it."
    o += "    public static readonly string[] SiteNames ="
    o += "    {"
    for s in c.sites:
        o += '        "%s",' % s
    o += "    };"
    o += "}"
    return str(o), c.sites
