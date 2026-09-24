"""Java backend: arm `core-ffi`, the Java half of the binding, rendered from a plan
(FIX-PLAN WP5 step 3; moved from poc/java/gen/java_binding.py, which read the IR).

Every group member it fills or reads, in order, comes from `plan.group_fields` (via
`_group_members`), every presence bit from `plan.presence_bits`, every loop slot from
`plan.loop_slots`, the vtable layout and trampoline numbering from `plan`, and the
direct-argument field from `plan.direct_fields`. The level parameter (17 or 8) changes the
string staging and nothing else. Two rules swept in the port: a direct field now carries
`AK_STR_DIRECT` (the core never took section 8's path before), and the sparse fill tests a
double by its bit pattern (R-E3), as the plan's implicit-presence rule says.

The C half (`java_jni`) does three small things. This half does the work: it fills the
by-value group of ABI v1 section 6 into off-heap memory at offsets `java_layout` computed
by hand, drives the host's own containers through the loop slots, and on decode resolves
`ak_span` offsets into the buffer the host handed in (section 7.4).

Four things here are decisions rather than transliterations, and each is an arm:

* **How a String reaches the core.** The group carries an `ak_str` -- a pointer, a length
  in SOURCE CODE UNITS and a transcoder (section 4). A JVM host holds UTF-16 or, from JDK
  9, LATIN1 when it fits, so the binding stages the string's own storage off-heap and names
  the matching core transcoder. The floor has no compact form and stages through
  `getChars`: see `ak.Str17`, and README 5.1's "one emitted source tree per target level".
* **Batching.** ABI v1 section 6 says a host may decline; the `batch` flag switches the two
  arms inside one process (R4), so the question is a paired delta and not two ratios.
* **Decision 9's fill.** `zeroed` bulk-clears the chunk and assigns only what differs. The
  rust and cpp slices both measured it a win and both said a managed host decides for
  itself, so it is a flag here rather than a conclusion.
* **Where the wire lives on decode.** The primary arm holds the bytes in a `byte[]`, as
  protobuf-java does, copies them once into native scratch for the core to parse, and
  resolves spans against the `byte[]` it already holds. The Java report attributed the
  push-family deficit to exactly that copy, so `wireNative` measures the other choice.
"""
from plan import (abi_order_topo, direct_fields, elem_type, group_fields, loop_slots,
                  presence_bits, slot_name, vtable_messages)
import plan as A
import java_layout as L
import java_names as N
import java_pull as PULL

WHO = "java_binding.py"

# The offsets live in ONE class per description, and the per-message natives in ONE class
# per description; both are set for the duration of one `emit` (the borrowed facade of
# decision 13 is a second package over the SAME ABI, so it shares them).
LAYOUT = ["ak.shapes.Layout"]
ENTRY = ["ak.NativeEntry"]

ARENA_BYTES = 32 * 1024


def _efix(name):
    return "ak_efix_%s" % name


def _dfix(name):
    return "ak_dfix_%s" % name


def _off(sname, member):
    return "%s.%s" % (LAYOUT[0], L.const(sname, member))


def _size(sname):
    return "%s.%s" % (LAYOUT[0], L.const(sname))


def _group_members(ir, m, enc):
    """(field-or-None, member name, oneof-or-None) in the plan's GROUP ORDER
    (`plan.group_fields`), carrying the field so the fill knows what to write. `None`
    marks a oneof discriminant. The member list is the plan's; this only finds the field
    each member was made from."""
    by_member = {}
    for f in m.plain:
        by_member[f.name] = (f, None)
    for oname, members in m.oneofs.items():
        by_member["%s_case" % oname] = (None, oname)
        for g in members:
            by_member["%s_%s" % (oname, g.name)] = (g, oname)
    out = []
    for member, _abi in group_fields(m, enc):
        if member not in by_member:
            raise NotImplementedError("group member %s.%s has no field" % (m.name, member))
        f, oname = by_member[member]
        out.append((f, member, oname))
    return out


# --------------------------------------------------------------------- encode fill

def _fill_scalar(o, ind, f, sname, member, src):
    p = " " * ind
    m = {"int32": "putInt", "enum": "putInt", "int64": "putLong",
         "double": "putDouble", "fixed32": "putInt"}[f.kind] if f.kind != "bool" else None
    if f.kind == "bool":
        o.append("%sMem.U.putByte(g + %s, (byte) (%s ? 1 : 0));" % (p, _off(sname, member), src))
    else:
        o.append("%sMem.U.%s(g + %s, %s);" % (p, m, _off(sname, member), src))


def _emit_fill(ir, o, name, sparse):
    """`fillX(long g, X o)`: the by-value group of ABI v1 section 6.

    The total fill is the specified path (section 6: "every scalar, every count and all
    three words of every `ak_str` are assigned unconditionally, and the presence word is
    assigned rather than OR-ed"). The sparse fill is open decision 9's candidate and is
    correct ONLY because a bulk clear precedes it.
    """
    m = ir.msg(name)
    sname = _efix(name)
    bits = presence_bits(m)
    suffix = "Sparse" if sparse else ""
    o.append("")
    o.append("  /** %s the encode group of `%s`. */"
             % ("Sparsely fill" if sparse else "Fill", name))
    o.append("  void fill%s%s(long g, %s o) {" % (name, suffix, name))
    if not sparse:
        o.append("    int pres = 0;")
    else:
        o.append("    int pres = 0;   // the clear already wrote every other slot")
    for f, member, oname in _group_members(ir, m, True):
        if f is None:
            o.append("    if (o.%s_case != 0) Mem.U.putInt(g + %s, o.%s_case);"
                     % (oname, _off(sname, member), oname)
                     if sparse else
                     "    Mem.U.putInt(g + %s, o.%s_case);" % (_off(sname, member), oname))
            continue
        src = "o.%s" % member
        if oname:
            # A oneof member: only the active one is written, so the sparse and total
            # fills agree here. The codec reads only the slot the discriminant names.
            o.append("    if (o.%s_case == %s.%s_CASE_%s) {"
                     % (oname, name, N.screaming(oname), N.screaming(f.name)))
            _fill_one(ir, o, 6, f, sname, member, src, bits, name, sparse=False,
                      oneof=True)
            o.append("    }")
            continue
        _fill_one(ir, o, 4, f, sname, member, src, bits, name, sparse)
    for fname, bit in bits.items():
        pass
    o.append("    Mem.U.putInt(g + %s, pres);" % _off(sname, "presence"))
    o.append("  }")


def _fill_one(ir, o, ind, f, sname, member, src, bits, owner, sparse,
              oneof=False):
    p = " " * ind
    if f.kind == "message":
        # A ONEOF member has no presence bit: the discriminant carries presence, which is
        # what makes the group's layout independent of which member is largest (ABI v1
        # section 6 refuses a union for exactly that reason).
        bit = bits.get(f.name)
        o.append("%sif (%s != null) {" % (p, src))
        if bit is not None:
            o.append("%s  pres |= %d;" % (p, 1 << bit))
        o.append("%s  fill%s(g + %s, %s);" % (p, f.of, _off(sname, member), src))
        if not sparse:
            o.append("%s} else {" % p)
            o.append("%s  // The total fill is unconditional (ABI v1 section 6): the codec"
                     % p)
            o.append("%s  // does not reset the group between elements, so an unwritten"
                     % p)
            o.append("%s  // child silently inherits the previous element's value." % p)
            o.append("%s  Mem.zero(g + %s, %s);" % (p, _off(sname, member), _size(_efix(f.of))))
        o.append("%s}" % p)
        return
    if f.kind in ("string", "bytes"):
        absent = None
        if f.presence == "direct":
            # ABI v1 section 8: the group carries the SENTINEL and the bytes travel as the
            # call's direct argument. The pre-WP5 fill staged the whole array with the
            # passthrough transcoder instead, so the core never took the direct path (it
            # reads the argument only when `data == AK_STR_DIRECT`) and the 4 MB bulk
            # payload was copied into the arena on every encode.
            o.append("%sputDirect(g + %s, %s);" % (p, _off(sname, member), src))
            return
        if f.explicit and not oneof:
            bit = bits[f.name]
            o.append("%sif (%s != null) {" % (p, src))
            o.append("%s  pres |= %d;" % (p, 1 << bit))
            o.append("%s  %s(g + %s, %s);" % (p, _putter(f), _off(sname, member), src))
            o.append("%s} else {" % p)
            o.append("%s  absentStr(g + %s);" % (p, _off(sname, member)))
            o.append("%s}" % p)
        else:
            o.append("%s%s(g + %s, %s);" % (p, _putter(f), _off(sname, member), src))
        del absent
        return
    # a scalar leaf
    if f.explicit and not oneof:
        bit = bits[f.name]
        o.append("%sif (o.has_%s) {" % (p, f.name))
        o.append("%s  pres |= %d;" % (p, 1 << bit))
        _fill_scalar(o, ind + 2, f, sname, member, src)
        if not sparse:
            o.append("%s} else {" % p)
            _fill_scalar(o, ind + 2, f, sname, member, N.ZERO[f.kind])
        o.append("%s}" % p)
        return
    if sparse:
        # The plan's implicit-presence test decides what differs from the cleared group,
        # and for a double it is the BIT PATTERN (R-E3): `x != 0.0` is false for -0.0, so
        # the pre-WP5 sparse fill left +0.0 in the cleared slot and the core dropped the
        # field -- the same defect as arm R's E5, in the zeroed arm (latent: shapes.json
        # has no singular double).
        if f.kind == "bool":
            cond = src
        elif f.kind == "double":
            cond = "Double.doubleToRawLongBits(%s) != 0L" % src
        else:
            cond = "%s != %s" % (src, N.ZERO[f.kind])
        o.append("%sif (%s) " % (p, cond))
        _fill_scalar(o, ind + 2, f, sname, member, src)
    else:
        _fill_scalar(o, ind, f, sname, member, src)


def _putter(f):
    return "putStr" if f.kind == "string" else "putBytes"


# --------------------------------------------------------------------- decode apply

def _emit_apply(ir, o, name):
    m = ir.msg(name)
    sname = _dfix(name)
    bits = presence_bits(m)
    o.append("")
    o.append("  /** Read the decode group of `%s` into the facade. ABI v1 7.4: resolve a"
             % name)
    o.append("   *  span against the base pointer you already hold -- one add and then the")
    o.append("   *  same copy (rather than indexing the managed array). */")

    o.append("  void apply%s(long g, %s o) {" % (name, name))
    o.append("    int pres = Mem.U.getInt(g + %s);" % _off(sname, "presence"))
    o.append("    if (pres == 0) { /* keep the branch honest: the absent path is P1.3 */ }")
    for f, member, oname in _group_members(ir, m, False):
        if f is None:
            o.append("    o.%s_case = Mem.U.getInt(g + %s);" % (oname, _off(sname, member)))
            continue
        dst = "o.%s" % member
        if oname:
            o.append("    if (o.%s_case == %s.%s_CASE_%s)"
                     % (oname, name, N.screaming(oname), N.screaming(f.name)))
            _apply_one(ir, o, 6, f, sname, member, dst, bits, oneof=True)
            continue
        _apply_one(ir, o, 4, f, sname, member, dst, bits, oneof=False)
    o.append("  }")


def _apply_one(ir, o, ind, f, sname, member, dst, bits, oneof):
    p = " " * ind
    if f.kind == "message":
        bit = bits.get(f.name)
        ctor = "new %s()" % f.of
        if oneof:
            o.append("%s{ %s = %s; apply%s(g + %s, %s); }"
                     % (p, dst, ctor, f.of, _off(sname, member), dst))
            return
        # ABI v1 open decision 10, and it is not theoretical: `apply` arrives AFTER the
        # runs that populated the element, so a child the host created to hold a run
        # already exists by the time the group is delivered. Constructing a fresh one here
        # -- which is the obvious thing to write -- DISCARDS the run. It cost this slice a
        # run to find: encode was byte-identical on every payload and only the decode
        # round trip showed it, as `TaskOptions.options` vanishing from every M2 payload.
        #
        # So: fill into what is there, and on absent leave an existing child alone, because
        # a run having created one means the field was present on the wire whatever the
        # group's presence bit says about the group's own fields.
        o.append("%sif ((pres & %d) != 0) {" % (p, 1 << bit))
        o.append("%s  if (%s == null) %s = %s;" % (p, dst, dst, ctor))
        o.append("%s  apply%s(g + %s, %s);" % (p, f.of, _off(sname, member), dst))
        o.append("%s}" % p)
        return
    if f.kind == "string":
        if f.explicit:
            bit = bits[f.name]
            o.append("%sif ((pres & %d) != 0) %s = getStr(g + %s);"
                     % (p, 1 << bit, dst, _off(sname, member)))
            o.append("%selse %s = null;" % (p, dst))
        else:
            o.append("%s%s = getStr(g + %s);" % (p, dst, _off(sname, member)))
        return
    if f.kind == "bytes":
        o.append("%s%s = getBytes(g + %s);" % (p, dst, _off(sname, member)))
        return
    getter = {"int32": "getInt", "enum": "getInt", "int64": "getLong",
              "double": "getDouble", "fixed32": "getInt"}.get(f.kind)
    if f.kind == "bool":
        expr = "Mem.U.getByte(g + %s) != 0" % _off(sname, member)
    else:
        expr = "Mem.U.%s(g + %s)" % (getter, _off(sname, member))
    if f.explicit and not oneof:
        bit = bits[f.name]
        o.append("%sif ((pres & %d) != 0) { %s = %s; o.has_%s = true; }"
                 % (p, 1 << bit, dst, expr, f.name))
        o.append("%selse { %s = %s; o.has_%s = false; }"
                 % (p, dst, N.ZERO[f.kind], f.name))
    else:
        o.append("%s%s = %s;" % (p, dst, expr))


# --------------------------------------------------------------------- encode loops

def _chunk_n(ir, name):
    """ABI v1 7.3: a byte budget divided by the group size, not an element count, so the
    scratch is the same 32 KB whatever the schema does. The core computes this with
    `ak_rt::arena_n` and the host must agree or a run would overrun the codec's own
    expectation of what a chunk is."""
    return "Math.max(1, %d / %s)" % (ARENA_BYTES, _size(_efix(name)))


def _emit_loop(ir, o, owner, path, f, slot, zeroed):
    """One loop slot: the host drives iteration over its own container.

    ABI v1 section 6: batched element runs are host-driven and chunked at 32 KB; the host
    fills an array of element groups from objects it is already walking and hands over
    extracted data. The codec never names a host object and nothing is pinned.
    """
    sn = slot_name(path)
    et = elem_type(f)
    suffix = "Zeroed" if zeroed else ""
    src = "e" + "".join(".%s" % p for p in path[:-1]) if len(path) > 1 else "e"
    o.append("")
    o.append("  int loop%d%s(long ctx, long token) {   // %s.%s" % (slot, suffix, owner, sn))
    o.append("    %s e = (%s) elemOf(%d, token);" % (owner, owner, slot))
    # Walk the inlined-child path to the container.
    holder = "e"
    for step in path[:-1]:
        o.append("    if (%s.%s == null) return 0;" % (holder, step))
        holder = "%s.%s" % (holder, step)
    cont = "%s.%s" % (holder, path[-1])
    mark = "long mk = arena.mark();"

    if f.card == "packed":
        # "The host's own array, handed over whole." A Java primitive array is the wire
        # layout for i32/i64/f64 and, because HotSpot stores boolean[] one byte per
        # element, for bool too -- so nothing is materialised and nothing is copied
        # except the bulk copy off-heap that the ABI's pointer requires. That copy IS the
        # "a host that does not store the wire form pays it back" sentence of section 6,
        # and it is one memcpy per field rather than per element.
        jty = {"int32": "int", "int64": "long", "bool": "boolean",
               "double": "double", "enum": "int"}[f.kind]
        run = {"int32": "runI32", "int64": "runI64", "bool": "runU8",
               "double": "runF64", "enum": "runI32"}[f.kind]
        scale = {"int32": 4, "int64": 8, "bool": 1, "double": 8, "enum": 4}[f.kind]
        base = {"int32": "Mem.INT_BASE", "int64": "Mem.LONG_BASE", "bool": "Mem.BOOL_BASE",
                "double": "Mem.DOUBLE_BASE", "enum": "Mem.INT_BASE"}[f.kind]
        o.append("    %s[] a = %s;" % (jty, cont))
        o.append("    if (a.length == 0) return 0;")
        o.append("    %s" % mark)
        o.append("    long p = arena.alloc((long) a.length * %d);" % scale)
        o.append("    Mem.U.copyMemory(a, %s, null, p, (long) a.length * %d);" % (base, scale))
        o.append("    int rc = Native.%s(ctx, p, a.length);" % run)
        o.append("    arena.release(mk);")
        o.append("    return rc;")
        o.append("  }")
        return

    if f.kind in ("string", "bytes") and f.card == "repeated":
        o.append("    java.util.List<%s> a = %s;"
                 % (N.string_type() if f.kind == "string" else "byte[]", cont))
        o.append("    int n = a.size();")
        o.append("    if (n == 0) return 0;")
        o.append("    final int chunk = batch ? %d : 1;" % (ARENA_BYTES // 24))
        o.append("    %s" % mark)
        o.append("    long runp = arena.alloc(24L * chunk);")
        o.append("    int i = 0;")
        o.append("    long strMark = arena.mark();")
        o.append("    for (int k = 0; k < n; k++) {")
        o.append("      %s(runp + 24L * i, a.get(k));"
                 % ("putStr" if f.kind == "string" else "putBytes"))
        o.append("      if (++i == chunk) {")
        o.append("        int rc = Native.blobRun(ctx, runp, i);")
        o.append("        if (rc < 0) { arena.release(mk); return rc; }")
        o.append("        i = 0;")
        o.append("        arena.release(strMark);   // the staged units are consumed")
        o.append("      }")
        o.append("    }")
        o.append("    if (i > 0) {")
        o.append("      int rc = Native.blobRun(ctx, runp, i);")
        o.append("      if (rc < 0) { arena.release(mk); return rc; }")
        o.append("    }")
        o.append("    arena.release(mk);")
        o.append("    return 0;")
        o.append("  }")
        return

    # a repeated message, or a map (which ABI v1 section 11 makes a repeated pair message)
    leaf = ir.msg(et).leaf
    entry = f.card == "map"
    o.append("    final int chunk = batch ? %s : 1;" % _chunk_n(ir, et))
    o.append("    %s" % mark)
    o.append("    long chunkp = arena.alloc((long) chunk * %s);" % _size(_efix(et)))
    if zeroed:
        o.append("    // Decision 9's candidate, with the cpp slice's correction: clear")
        o.append("    // only the elements that will be FILLED. Clearing the whole 32 KB")
        o.append("    // arena is O(arena) where the fill is O(elements), which inverted")
        o.append("    // the candidate on P1.1 and P2.1 until the cpp slice measured it.")
    if entry:
        o.append("    java.util.TreeMap<%s, %s> mp = %s;"
                 % (N.string_type(), N.string_type(), cont))
        o.append("    int n = mp.size();")
    else:
        o.append("    java.util.List<%s> a = %s;" % (et, cont))
        o.append("    int n = a.size();")
    o.append("    if (n == 0) { arena.release(mk); return 0; }")
    if zeroed and not entry:
        o.append("    Mem.zero(chunkp, (long) Math.min(n, chunk) * %s);" % _size(_efix(et)))
    o.append("    int i = 0;")
    o.append("    long strMark = arena.mark();")
    if not leaf:
        o.append("    long tok0 = tokenBase();")
    if entry:
        # plan ENCODE RULES: ascending UTF-8 key order (a String TreeMap is UTF-16 order).
        src = "Codec.utf8Sorted(mp)" if N.string_type() == "String" else "mp.entrySet()"
        o.append("    for (java.util.Map.Entry<%s, %s> en : %s) {"
                 % (N.string_type(), N.string_type(), src))
        o.append("      long gp = chunkp + (long) i * %s;" % _size(_efix(et)))
        o.append("      putStr(gp + %s, en.getKey());" % _off(_efix(et), "key"))
        o.append("      putStr(gp + %s, en.getValue());" % _off(_efix(et), "value"))
        o.append("      Mem.U.putInt(gp + %s, 0);" % _off(_efix(et), "presence"))
    else:
        o.append("    for (int k = 0; k < n; k++) {")
        o.append("      long gp = chunkp + (long) i * %s;" % _size(_efix(et)))
        if not leaf:
            o.append("      pushToken(a.get(k));")
        o.append("      fill%s%s(gp, a.get(k));" % (et, "Sparse" if zeroed else ""))
    o.append("      if (++i == chunk) {")
    o.append("        int rc = %s;" % _run_call(et, leaf, entry))
    o.append("        if (rc < 0) { arena.release(mk); return rc; }")
    if not leaf:
        o.append("        tok0 = tokenBase();")
    o.append("        i = 0;")
    o.append("        arena.release(strMark);")
    if zeroed:
        o.append("        Mem.zero(chunkp, (long) Math.min(n, chunk) * %s);"
                 % _size(_efix(et)))
    o.append("      }")
    o.append("    }")
    o.append("    if (i > 0) {")
    o.append("      int rc = %s;" % _run_call(et, leaf, entry))
    o.append("      if (rc < 0) { arena.release(mk); return rc; }")
    o.append("    }")
    o.append("    arena.release(mk);")
    o.append("    return 0;")
    o.append("  }")


def _run_call(et, leaf, entry):
    if leaf:
        return "%s.elem%s(ctx, chunkp, i)" % (ENTRY[0], et)
    return "%s.elemu%s(ctx, chunkp, i, tok0)" % (ENTRY[0], et)


# --------------------------------------------------------------------- decode slots

def _emit_dec_slot(ir, o, owner, kind, sn, fld, slot):
    """One decode callback. ABI v1 7.1's push family: the codec fills a bounded arena and
    calls the host, so a run crosses once per chunk rather than once per element."""
    if kind == "apply":
        o.append("")
        o.append("  void dec%d(long fix) {   // %s.apply" % (slot, owner))
        o.append("    apply%s(fix, (%s) decRoot);" % (owner, owner))
        o.append("  }")
        return
    if kind == "new":
        et = fld
        o.append("")
        o.append("  long dec%d() {   // %s.new %s" % (slot, owner, sn))
        o.append("    %s x = new %s();" % (et, et))
        path = [q for q, _f in loop_slots(ir, owner) if slot_name(q) == sn][0]
        o.append("    %s e = (%s) decRoot;" % (owner, owner))
        cont, walk = "e", owner
        for step in path[:-1]:
            o.append("    if (%s.%s == null) %s.%s = new %s();" % (
                cont, step, cont, step, _child_type(ir, walk, step)))
            walk = _child_type(ir, walk, step)
            cont = "%s.%s" % (cont, step)
        o.append("    %s.%s.add(x);" % (cont, path[-1]))
        o.append("    return pushDecToken(x);")
        o.append("  }")
        return
    if kind == "applyelem":
        et = fld
        o.append("")
        o.append("  void dec%d(long token, long fix) {   // %s.apply %s" % (slot, owner, sn))
        o.append("    apply%s(fix, (%s) decTokenAt(token));" % (et, et))
        o.append("  }")
        return

    # add / addinner
    f = fld if hasattr(fld, "kind") else None
    o.append("")
    o.append("  void dec%d(long token, long p, int n) {   // %s.add %s" % (slot, owner, sn))
    o.append("    // ABI v1 7.4: a batched add may be called more than once per field.")
    o.append("    // Append; never size to the count you were handed.")
    holder, cont, f2 = _dec_target(ir, owner, sn, kind, f)
    o.append("    %s" % holder)
    et = elem_type(f2)
    if f2.card == "packed":
        jty = {"int32": "int", "int64": "long", "bool": "boolean",
               "double": "double", "enum": "int"}[f2.kind]
        scale = {"int32": 4, "int64": 8, "bool": 1, "double": 8, "enum": 4}[f2.kind]
        base = {"int32": "Mem.INT_BASE", "int64": "Mem.LONG_BASE", "bool": "Mem.BOOL_BASE",
                "double": "Mem.DOUBLE_BASE", "enum": "Mem.INT_BASE"}[f2.kind]
        o.append("    %s[] old = %s;" % (jty, cont))
        o.append("    %s[] out = java.util.Arrays.copyOf(old, old.length + n);" % jty)
        o.append("    Mem.U.copyMemory(null, p, out, %s + (long) old.length * %d,"
                 " (long) n * %d);" % (base, scale, scale))
        o.append("    %s = out;" % cont)
    elif f2.card == "map":
        o.append("    for (int i = 0; i < n; i++) {")
        o.append("      long g = p + (long) i * %s;" % _size(_dfix(et)))
        o.append("      %s.put(getStr(g + %s), getStr(g + %s));"
                 % (cont, _off(_dfix(et), "key"), _off(_dfix(et), "value")))
        o.append("    }")
    elif f2.kind in ("string", "bytes"):
        o.append("    for (int i = 0; i < n; i++)")
        o.append("      %s.add(%s(p + 12L * i));"
                 % (cont, "getStr" if f2.kind == "string" else "getBytes"))
    else:
        o.append("    for (int i = 0; i < n; i++) {")
        o.append("      %s x = new %s();" % (et, et))
        o.append("      apply%s(p + (long) i * %s, x);" % (et, _size(_dfix(et))))
        o.append("      %s.add(x);" % cont)
        o.append("    }")
    o.append("  }")


def _dec_target(ir, owner, sn, kind, f):
    """Which container an `add` appends to, and the statement that reaches it."""
    if kind == "addinner":
        # `sn` is "<outer slot>_<inner path>"; the object is the element named by token.
        outer = None
        for path, ff in loop_slots(ir, owner):
            p = slot_name(path)
            if sn.startswith(p + "_"):
                outer, rest = ff, sn[len(p) + 1:]
                break
        et = elem_type(outer)
        # Re-resolve the inner path against the element type.
        for ipath, iff in loop_slots(ir, et):
            if slot_name(ipath) == rest:
                holder = "%s e = (%s) decTokenAt(token);" % (et, et)
                cont = "e"
                walk = et
                for step in ipath[:-1]:
                    holder += "\n    if (%s.%s == null) %s.%s = new %s();" % (
                        cont, step, cont, step, _child_type(ir, walk, step))
                    walk = _child_type(ir, walk, step)
                    cont = "%s.%s" % (cont, step)
                cont = "%s.%s" % (cont, ipath[-1])
                return holder, cont, iff
        raise NotImplementedError("inner slot %s of %s" % (rest, et))
    # A plain `add`: the container is on the owner, reached through the slot path. The
    # owner is the root when the token is AK_TOKEN_ROOT and the element otherwise.
    for path, ff in loop_slots(ir, owner):
        if slot_name(path) == sn:
            holder = "%s e = (%s) decOwner(token);" % (owner, owner)
            cont = "e"
            walk = owner
            for step in path[:-1]:
                holder += "\n    if (%s.%s == null) %s.%s = new %s();" % (
                    cont, step, cont, step, _child_type(ir, walk, step))
                walk = _child_type(ir, walk, step)
                cont = "%s.%s" % (cont, step)
            cont = "%s.%s" % (cont, path[-1])
            return holder, cont, ff
    raise NotImplementedError("slot %s of %s" % (sn, owner))


def _child_type(ir, owner, step):
    for f in ir.msg(owner).plain:
        if f.name == step:
            return f.of
    raise KeyError("%s.%s" % (owner, step))


# --------------------------------------------------------------------- the class

def emit(ir, level=17, ns=N.PKG, facade_ns=None, layout="ak.shapes.Layout",
         entry="ak.NativeEntry"):
    """The Binding class of one facade package at one Java level. `layout` and `entry` are
    the description's Layout and NativeEntry classes (one per description)."""
    LAYOUT[0], ENTRY[0] = layout, entry
    enc = A.enc_slots(ir)
    dec = A.dec_slots(ir)
    enc_ix = {(msg, slot_name(path)): i for i, (msg, path, _f) in enumerate(enc)}
    dec_ix = {(msg, kind, sn): i for i, (msg, kind, sn, _et) in enumerate(dec)}

    o = [N.java_head(ir, WHO), "package %s;" % ns, "",
         "import ak.Arena;", "import ak.Mem;", "import ak.Native;", "import ak.Str17;",
         "import ak.Utf8;"]
    if facade_ns and facade_ns != ns:
        # README 5.2 arm b wants the FLOOR implementation on the TARGET runtime, and R4
        # wants the ratio formed inside one process. Two source trees with the same class
        # names cannot share a classpath, so the floor binding is emitted into a package of
        # its own over the SAME facade types: one process, one paired ratio, and the only
        # thing that differs between the two arms is how a String reaches the core.
        o.append("import %s.*;" % facade_ns)
    o += ["", DOC % (level, "the target" if level >= 17 else "the floor"),
         "public final class Binding implements AutoCloseable, ak.Callbacks {"]
    o.append(PRELUDE % (level, ARENA_BYTES, ENTRY[0], LAYOUT[0]))

    # ---- the string staging, which is the whole floor/target divergence
    o.append(STAGE_BORROW if N.is_borrow()
             else STAGE_17 if level >= 17 else STAGE_8)

    # ---- span readers, typed to the facade's string type
    o.append(SPAN % (N.string_type(),
                     '""' if not N.is_borrow() else "ak.Utf8View.EMPTY",
                     "Utf8.decode(wireHeap, off, len)" if not N.is_borrow()
                     else "ak.Utf8View.of(wireHeap, off, len)"))

    # ---- vtables
    o.append("")
    o.append("  /** Every vtable, built once into off-heap memory. Each slot is a")
    o.append("   *  pointer, so the layout is index times eight and needs no offset")
    o.append("   *  table of its own. */")
    o.append("  private void buildVtables() {")
    vt_addr = {}
    total = 0
    order = vtable_messages(ir)
    for name in order:
        rows = A.enc_vtable(ir, name)
        vt_addr[("e", name)] = total
        total += len(rows)
    for name in ir.roots:
        rows = A.dec_vtable(ir, name)
        vt_addr[("d", name)] = total
        total += len(rows)
    o.append("    vt = Mem.alloc(%dL * 8);" % total)
    for name in order:
        base = vt_addr[("e", name)]
        for j, (kind, sn, et) in enumerate(A.enc_vtable(ir, name)):
            at = base + j
            if kind == "loop":
                o.append("    Mem.U.putLong(vt + %d * 8, %s.encTrampoline(%d));   // %s.loop %s"
                         % (at, ENTRY[0], enc_ix[(name, sn)], name, sn))
            elif kind == "elem":
                o.append("    Mem.U.putLong(vt + %d * 8, vt + %d * 8);   // %s.elem %s -> %s"
                         % (at, vt_addr[("e", et)], name, sn, et))
            else:
                o.append("    Mem.U.putLong(vt + %d * 8, 0L);   // %s: reserved" % (at, name))
    for name in ir.roots:
        base = vt_addr[("d", name)]
        for j, (kind, sn, et) in enumerate(A.dec_vtable(ir, name)):
            at = base + j
            if kind in ("unknown", "unk"):
                o.append("    Mem.U.putLong(vt + %d * 8, 0L);   // %s.%s %s: decision 11's"
                         " bag is not built" % (at, name, kind, sn))
            else:
                o.append("    Mem.U.putLong(vt + %d * 8, %s.decTrampoline(%d));   // %s.%s %s"
                         % (at, ENTRY[0], dec_ix[(name, kind, sn)], name, kind, sn))
    for name in order:
        o.append("    evt%s = vt + %d * 8;" % (name, vt_addr[("e", name)]))
    for name in ir.roots:
        o.append("    dvt%s = vt + %d * 8;" % (name, vt_addr[("d", name)]))
    o.append("  }")
    o.append("")
    o.append("  private long vt;")
    for name in order:
        o.append("  private long evt%s;" % name)
    for name in ir.roots:
        o.append("  private long dvt%s;" % name)

    # ---- fills and applies
    for name in abi_order_topo(ir):
        if ir.msg(name).synthetic:
            continue
        _emit_fill(ir, o, name, sparse=False)
        _emit_fill(ir, o, name, sparse=True)
    for name in abi_order_topo(ir):
        if ir.msg(name).synthetic:
            continue
        _emit_apply(ir, o, name)
    # A synthetic map-entry group has no facade class of its own: ABI v1 section 11
    # gives a map no case, so the host reads the pair's two spans where the run arrives
    # and puts them straight into its own map. Nothing to apply into.

    # ---- loops
    for i, (msg, path, f) in enumerate(enc):
        _emit_loop(ir, o, msg, path, f, i, zeroed=False)
        _emit_loop(ir, o, msg, path, f, i, zeroed=True)

    o.append("")
    o.append("  /** The reverse call of ABI v1 section 6, dispatched on the slot the")
    o.append("   *  trampoline was built for. The body is wrapped because an exception")
    o.append("   *  crossing `extern \"C\"` into Rust is undefined behaviour; the C half")
    o.append("   *  also checks, and that half is what catches what JNI itself raises. */")
    o.append("  @Override public int encLoop(long ctx, int slot, long token) {")
    o.append("    try {")
    o.append("      switch (slot) {")
    for i, (msg, path, f) in enumerate(enc):
        o.append("        case %d: return zeroed ? loop%dZeroed(ctx, token)"
                 " : loop%d(ctx, token);" % (i, i, i))
    o.append("        default: return ak.Native.ERR_ABI;")
    o.append("      }")
    o.append("    } catch (Throwable t) {")
    o.append("      lastHostError = t;")
    o.append("      return ak.Native.ERR_HOST;")
    o.append("    }")
    o.append("  }")

    # ---- decode slots
    for i, (msg, kind, sn, et) in enumerate(dec):
        _emit_dec_slot(ir, o, msg, kind, sn, et, i)

    for name, sig, args, ret in (
            ("decApply",
             "@Override public void decApply(long ctx, int slot, long fix)", "fix", None),
            ("decNew", "@Override public long decNew(long ctx, int slot)", "", 0),
            ("decApplyElem",
             "@Override public void decApplyElem(long ctx, int slot, long token, long fix)",
             "token, fix", None),
            ("decAdd",
             "@Override public void decAdd(long ctx, int slot, long token, long p, int n)",
             "token, p, n", None)):
        kind = {"decApply": "apply", "decNew": "new", "decApplyElem": "applyelem"}.get(name)
        o.append("")
        o.append("  %s {" % sig)
        o.append("    try {")
        o.append("      switch (slot) {")
        for i, (msg, k, sn, et) in enumerate(dec):
            want = (k == kind) if kind else (k in ("add", "addinner"))
            if not want:
                continue
            if ret is not None:
                o.append("        case %d: return dec%d();" % (i, i))
            else:
                o.append("        case %d: dec%d(%s); return;" % (i, i, args))
        o.append("        default: break;")
        o.append("      }")
        if ret is not None:
            o.append("      return -1;")
        o.append("    } catch (Throwable t) {")
        o.append("      lastHostError = t;")
        o.append("      Native.fail(ctx, ak.Native.ERR_HOST);")
        if ret is not None:
            o.append("      return -1;")
        o.append("    }")
        o.append("  }")

    # ---- entry points
    o.append("")
    o.append("  // ---- the entry points -----------------------------------------------")
    for root in ir.roots:
        direct = direct_fields(ir, root)
        o.append("")
        o.append("  public int encode%s(%s o) {" % (root, root))
        o.append("    Native.encReset(encCtx);")
        o.append("    encLast = -1;")
        o.append("    arena.reset();")
        o.append("    encTokN = 0;")
        o.append("    encRoot = o;")
        o.append("    lastHostError = null;")
        o.append("    long g = arena.alloc(%s);" % _size(_efix(root)))
        if False:
            pass
        o.append("    fill%s%s(g, o);" % (root, ""))
        if direct:
            dpath, dfield = direct[0]
            expr = "o" + "".join(".%s" % s for s in dpath)
            o.append("    // ABI v1 section 8: the bulk field is a DIRECT ARGUMENT of the")
            o.append("    // call, so the host pins its own array for the duration and the")
            o.append("    // bytes are never staged. The generator-time refusal has already")
            o.append("    // proved this tree makes no reverse call, which is what makes a")
            o.append("    // critical section legal here.")
            guards = ["o" + "".join(".%s" % s for s in dpath[:k + 1]) + " == null"
                      for k in range(len(dpath))]
            o.append("    byte[] direct = (%s) ? ak.Native.NO_BYTES : %s;" % (" || ".join(guards), expr))
            o.append("    long rc = %s.encodeDirect%s(this, encCtx, evt%s, g,"
                     " direct, direct.length);" % (ENTRY[0], root, root))
        else:
            o.append("    long rc = %s.encode%s(this, encCtx, evt%s, g);"
                     % (ENTRY[0], root, root))
        # The core's encode returns the encoded length. Recording it here is what
        # lets `takeBytes` allocate the result without asking again: D6 removed one
        # redundant crossing from the `ffi` arm and left the same one standing in the
        # `-take` arm, which is the arm the incumbent is compared against.
        o.append("    return encLast = check((int) rc);")
        o.append("  }")
        o.append("")
        o.append("  public %s decode%s(byte[] wire, int off, int len) {" % (root, root))
        o.append("    %s r = new %s();" % (root, root))
        o.append("    beginDecode(wire, off, len);")
        o.append("    decRoot = r;")
        o.append("    int rc = %s.decode%s(this, decCtx, wireNative, len,"
                 " dvt%s);" % (ENTRY[0], root, root))
        o.append("    check(rc);")
        o.append("    return r;")
        o.append("  }")

    PULL.emit(ir, dec_ix, o, ENTRY[0])

    o.append("")
    o.append("  /** The encoded bytes, copied into a Java array -- the same place")
    o.append("   *  protobuf-java's `toByteArray` leaves them, so neither arm is one copy")
    o.append("   *  cheaper than the other for a reason that is not the ABI. */")
    o.append("  public byte[] take() { return takeBytes(); }")
    o.append("}")
    o.append("")
    return "\n".join(o)


def _null_guard(expr):
    return "%s == null ? ak.Native.NO_BYTES : %s" % (expr, expr)


DOC = '''/**
 * Arm `core-ffi`: the Java host binding over the C ABI, emitted at the Java %d level
 * (%s of README section 5).
 *
 * <p>Every buffer here is instance state. A static would make two encoding threads share
 * one staging arena, and the rust slice's own concurrency defect is the argument: shared
 * mutable state that worked at one call in flight and failed outright at eight, which is
 * the GOOD failure mode -- the bad one is a wrong byte under contention.
 */'''

PRELUDE = '''
  /** ABI v1 7.3: a byte budget divided by the group size, not an element count, so the
   *  scratch is the same 32 KB whatever the schema does. The core computes the same
   *  number with `ak_rt::arena_n` and the two must agree. */
  public static final int LEVEL = %d;
  public static final int ARENA_BYTES = %d;

  /** The staging block. Sized once and never grown, because an `ak_str` already in a
   *  group holds an ADDRESS into it: a realloc would leave every string of the current
   *  chunk pointing into freed memory, and the payload set would still pass because the
   *  old block is usually still mapped. A fill that would overflow flushes the chunk
   *  instead, which ABI v1 section 6 permits -- "every entry point appends". */
  final Arena arena = new Arena(8L << 20);

  static {
    // Before any field initialiser that calls into the core. A `long encCtx =
    // Native.encCtxNew()` field would run before the constructor body and fail with an
    // UnsatisfiedLinkError, which is the loud version of this and cost one run to see.
    Native.ensureBound();
    // ABI v1 section 3, rendered from plan.lifecycle (R-G7): ak_init before the first
    // codec call, against a core built with `init-guard` in every gate.
    %s.ensureInit();
    // ABI v1 section 10: this description's group offsets against the core's own.
    %s.assertAgreement();
  }

  public long encCtx = Native.encCtxNew();
  public long decCtx = Native.decCtxNew();

  /** The batching predicate of ABI v1 section 6, as a flag rather than a verdict: the two
   *  arms are paired in one process (R4), never two ratios to a third arm. */
  public boolean batch = true;

  /** Open decision 9's candidate: bulk-clear the chunk and assign only what differs from
   *  the default. Rust and C++ both measured it a win and both said a managed host decides
   *  for itself. */
  public boolean zeroed = false;

  Object encRoot;
  Object[] encTok = new Object[256];
  int encTokN;
  Object decRoot;
  Object[] decTok = new Object[256];
  int decTokN;

  /** The wire on decode. The primary arm holds the bytes in a `byte[]`, as protobuf-java
   *  does, copies them ONCE into native scratch for the core to parse, and resolves spans
   *  against the `byte[]` it already holds (ABI v1 7.4). The Java report attributed the
   *  push family's deficit to exactly that copy; `wireOffHeapOnly` measures the other
   *  choice, where nothing is copied and every string costs an extra one instead. */
  byte[] wireHeap;
  long wireNative;
  long wireCap;
  public boolean wireOffHeapOnly = false;

  /** The last host exception, kept so a failed encode says why rather than returning a
   *  code. ABI v1 section 5's channel carries a code and a message and no source chain
   *  (open decision 12), so this is the Java side of that gap. */
  public Throwable lastHostError;

  /** The length the core returned from the last encode. ABI v1's encode entry hands
   *  it back, so `takeBytes` does not have to cross the boundary to ask for it. */
  int encLast = -1;

  public Binding() {
    Mem.checkBoolScale();
    buildVtables();
  }

  @Override public void close() {
    Native.encCtxFree(encCtx);
    Native.decCtxFree(decCtx);
    arena.close();
    if (wireNative != 0) Mem.free(wireNative);
    Mem.free(vt);
    encCtx = decCtx = wireNative = vt = 0;
  }

  int check(int rc) {
    if (rc < 0) {
      if (lastHostError != null)
        throw new IllegalStateException("host failed inside a reverse call", lastHostError);
      throw new IllegalStateException("core returned " + rc);
    }
    return rc;
  }

  long tokenBase() { return encTokN; }

  void pushToken(Object o) {
    if (encTokN == encTok.length) encTok = java.util.Arrays.copyOf(encTok, encTokN * 2);
    encTok[encTokN++] = o;
  }

  Object elemOf(int slot, long token) {
    return token < 0 ? encRoot : encTok[(int) token];
  }

  long pushDecToken(Object o) {
    if (decTokN == decTok.length) decTok = java.util.Arrays.copyOf(decTok, decTokN * 2);
    decTok[decTokN++] = o;
    return decTokN - 1;
  }

  Object decTokenAt(long t) { return decTok[(int) t]; }

  Object decOwner(long t) { return t < 0 ? decRoot : decTok[(int) t]; }

  void beginDecode(byte[] wire, int off, int len) {
    decTokN = 0;
    lastHostError = null;
    // NOT `Native.decErrReset(decCtx)`. `ak_decode_*` clears the sticky slot at entry
    // itself and says why: "doing it here rather than in the host costs no extra
    // crossing and takes the obligation off the binding author". Calling it anyway was
    // one forward crossing per decode charged to this arm for nothing -- D6 and D7 again,
    // and this time on the arm the pull family is about to be compared against.
    if (wireCap < len) {
      if (wireNative != 0) Mem.free(wireNative);
      wireNative = Mem.alloc(Math.max(len, 1 << 16));
      wireCap = Math.max(len, 1 << 16);
    }
    Mem.copyFromBytes(wire, off, wireNative, len);
    wireHeap = wire;
    wireBase = off;
  }

  int wireBase;

  /** One allocation and ONE copy, which is exactly what `toByteArray` costs the
   *  incumbent. The earlier form asked the core for the length over the boundary and
   *  then copied the bytes twice, native into a reused scratch array and the scratch
   *  array into the result -- a handicap on this arm worth an entire memcpy of the
   *  payload, which on the 4 MB bulk payload was most of the arm's reported cost. */
  byte[] takeBytes() {
    int n = encLast;
    if (n < 0) throw new IllegalStateException("no encode to take, or it failed");
    byte[] r = new byte[n];
    int rc = Native.encTake(encCtx, r);
    if (rc < 0) throw new IllegalStateException("core returned " + rc);
    return r;
  }

  /** ABI v1 section 8: a direct field's slot carries `AK_STR_DIRECT` and its length; the
   *  bytes are the entry point's direct argument, pinned by the shim for the call. */
  static void putDirect(long dst, byte[] b) {
    Mem.U.putLong(dst, 1L);                         // AK_STR_DIRECT
    Mem.U.putLong(dst + 8, b == null ? 0L : (long) b.length);
    Mem.U.putLong(dst + 16, 0L);
  }

  /** The encoded length without the copy, for the arms that only need the size. */
  public int encodedLength() { return Native.encLen(encCtx); }
'''

STAGE_17 = '''
  // ---- staging a String, at the JDK 17 level ----------------------------------------
  //
  // From JDK 9 a String is a `byte[] value` plus a `byte coder`: LATIN1 when every
  // character fits in one byte, UTF16 otherwise. ABI v1 section 4 has a transcoder for
  // each, so the binding hands the core ONE bulk copy of the string's own storage and
  // names the matching entry -- and for an ASCII GUID, which is what every id in the real
  // schema is, that is 36 bytes rather than 72.
  //
  // `Str17.available()` is checked rather than assumed: it encodes known strings through
  // both paths and refuses the fast one unless they agree. When it refuses, this level
  // falls back to the floor's staging, which is correct but is the floor's cost, and the
  // bench prints which path ran.
  private final long TC_UTF16 = Native.tcUtf16();
  private final long TC_LATIN1 = Native.tcLatin1();
  private final long TC_BYTES = Native.tcBytes();
  private final boolean compact = Str17.available();
  private char[] chars = new char[256];

  public boolean usingCompactStrings() { return compact; }

  void absentStr(long dst) {
    // `tc == NULL` is how a group tells ABSENT from present-and-empty (ABI v1 section 4).
    Mem.U.putLong(dst, 0L);
    Mem.U.putLong(dst + 8, 0L);
    Mem.U.putLong(dst + 16, 0L);
  }

  void putStr(long dst, String s) {
    if (s == null) { absentStr(dst); return; }
    int n = s.length();
    if (n == 0) {
      Mem.U.putLong(dst, arena.base);
      Mem.U.putLong(dst + 8, 0L);
      Mem.U.putLong(dst + 16, TC_BYTES);
      return;
    }
    if (compact) {
      byte[] v = Str17.value(s);
      boolean latin1 = Str17.coder(s) == Str17.LATIN1;
      long p = arena.allocRaw(v.length);
      Mem.copyFromBytes(v, 0, p, v.length);
      Mem.U.putLong(dst, p);
      Mem.U.putLong(dst + 8, n);            // SOURCE CODE UNITS, per ABI v1 section 4
      Mem.U.putLong(dst + 16, latin1 ? TC_LATIN1 : TC_UTF16);
      return;
    }
    stageChars(dst, s, n);
  }

  void stageChars(long dst, String s, int n) {
    if (chars.length < n) chars = new char[Integer.highestOneBit(n - 1) * 2];
    s.getChars(0, n, chars, 0);
    long p = arena.allocRaw((long) n * 2);
    Mem.copyFromChars(chars, 0, p, n);
    Mem.U.putLong(dst, p);
    Mem.U.putLong(dst + 8, n);
    Mem.U.putLong(dst + 16, TC_UTF16);
  }

  void putBytes(long dst, byte[] b) {
    if (b == null) { absentStr(dst); return; }
    long p = b.length == 0 ? arena.base : arena.allocRaw(b.length);
    if (b.length != 0) Mem.copyFromBytes(b, 0, p, b.length);
    Mem.U.putLong(dst, p);
    Mem.U.putLong(dst + 8, b.length);
    Mem.U.putLong(dst + 16, TC_BYTES);
  }
'''

STAGE_8 = '''
  // ---- staging a String, at the Java 8 floor ----------------------------------------
  //
  // A Java 8 String is a `char[]`, with no compact form and no coder, so there is no
  // LATIN1 case to take: every string stages as UTF-16 through `getChars` into a scratch
  // array and then off-heap. Two copies where the target does one, and twice the bytes for
  // an ASCII id -- which is what every id in the real schema is.
  //
  // This is the floor/target divergence README 5.1 predicts for Java, and it is the WHOLE
  // of it: nothing else in this binding differs between the two levels. `getChars` is the
  // only substitution, and there is no second.
  private final long TC_UTF16 = Native.tcUtf16();
  private final long TC_BYTES = Native.tcBytes();
  private char[] chars = new char[256];

  public boolean usingCompactStrings() { return false; }

  void absentStr(long dst) {
    Mem.U.putLong(dst, 0L);
    Mem.U.putLong(dst + 8, 0L);
    Mem.U.putLong(dst + 16, 0L);
  }

  void putStr(long dst, String s) {
    if (s == null) { absentStr(dst); return; }
    int n = s.length();
    if (n == 0) {
      Mem.U.putLong(dst, arena.base);
      Mem.U.putLong(dst + 8, 0L);
      Mem.U.putLong(dst + 16, TC_BYTES);
      return;
    }
    stageChars(dst, s, n);
  }

  void stageChars(long dst, String s, int n) {
    if (chars.length < n) chars = new char[Integer.highestOneBit(n - 1) * 2];
    s.getChars(0, n, chars, 0);
    long p = arena.allocRaw((long) n * 2);
    Mem.copyFromChars(chars, 0, p, n);
    Mem.U.putLong(dst, p);
    Mem.U.putLong(dst + 8, n);
    Mem.U.putLong(dst + 16, TC_UTF16);
  }

  void putBytes(long dst, byte[] b) {
    if (b == null) { absentStr(dst); return; }
    long p = b.length == 0 ? arena.base : arena.allocRaw(b.length);
    if (b.length != 0) Mem.copyFromBytes(b, 0, p, b.length);
    Mem.U.putLong(dst, p);
    Mem.U.putLong(dst + 8, b.length);
    Mem.U.putLong(dst + 16, TC_BYTES);
  }
'''

SPAN = '''
  // ---- reading a span (ABI v1 7.4) ---------------------------------------------------
  //
  // "Resolve spans against the base pointer you already hold. The host pinned the buffer
  // to make the call, so an offset is one add and then the same fused transcode."
  // Here the base pointer is the `byte[]` the host was handed; the native copy the core
  // parsed has the same contents at the same offsets, which is the whole reason the copy
  // is worth making.

  %s getStr(long span) {
    int off = Mem.U.getInt(span);
    int len = Mem.U.getInt(span + 4);
    if (len == 0) return %s;
    off += wireBase;
    return %s;
  }

  byte[] getBytes(long span) {
    int len = Mem.U.getInt(span + 4);
    if (len == 0) return NO_BYTES;
    int off = Mem.U.getInt(span) + wireBase;
    byte[] b = new byte[len];
    System.arraycopy(wireHeap, off, b, 0, len);
    return b;
  }

  static final byte[] NO_BYTES = new byte[0];
'''


STAGE_BORROW = """
  // ---- staging a borrowed view (open decision 13's arm) ------------------------------
  //
  // A `Utf8View` already holds UTF-8, so it stages as bytes: one memcpy and the
  // passthrough transcoder, with no UTF-16 conversion at all. That makes this arm's
  // ENCODE path a different mechanism from the owning facade's, and it is why decision 13
  // is a DECODE question and this arm's encode column is not quoted. It exists so the
  // correctness gate can round-trip the borrowed facade against the same manifest as
  // every other arm, which is the only way to know the views hold the right bytes.
  private final long TC_BYTES = Native.tcBytes();

  public boolean usingCompactStrings() { return false; }

  void absentStr(long dst) {
    Mem.U.putLong(dst, 0L);
    Mem.U.putLong(dst + 8, 0L);
    Mem.U.putLong(dst + 16, 0L);
  }

  void putStr(long dst, ak.Utf8View v) {
    if (v == null) { absentStr(dst); return; }
    long p = v.len == 0 ? arena.base : arena.allocRaw(v.len);
    if (v.len != 0) Mem.copyFromBytes(v.buf, v.off, p, v.len);
    Mem.U.putLong(dst, p);
    Mem.U.putLong(dst + 8, v.len);
    Mem.U.putLong(dst + 16, TC_BYTES);
  }

  void putBytes(long dst, byte[] b) {
    if (b == null) { absentStr(dst); return; }
    long p = b.length == 0 ? arena.base : arena.allocRaw(b.length);
    if (b.length != 0) Mem.copyFromBytes(b, 0, p, b.length);
    Mem.U.putLong(dst, p);
    Mem.U.putLong(dst + 8, b.length);
    Mem.U.putLong(dst + 16, TC_BYTES);
  }
"""
