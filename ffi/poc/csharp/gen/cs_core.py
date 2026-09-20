"""Backend: the `core-ffi` host binding, for EVERY shape.

This replaces the two hand-shaped emitters that covered M1 and M2. They were
written one root at a time and the second one cost a vtable slot invented by
analogy; five more written the same way would have cost five more chances at it.
So the binding is derived from the same `gen/abi_ir.py` that derives the ABI
declaration, and the seven roots differ only in what that derivation returns.

What one root's binding has to cover, and where each comes from:

  * **the by-value group** -- every singular field, a nested group per singular
    message child, a presence bit per optional child, and for a oneof a
    `<name>_case` discriminant carrying the ACTIVE MEMBER'S TAG with every
    member inlined beside it. The facade's own case enum is already tag-valued,
    so the discriminant is a cast;
  * **a loop callback per slot**, and what the host calls forward inside it
    depends on the slot: `ak_blob_run` for a repeated string, `ak_run_i32/i64/
    f64/u8` for a packed scalar, `ak_elem_<Entry>` for a map, and for a repeated
    message `ak_elem_<T>` if the element is a LEAF or `ak_elemu_<T>` if it is
    not. That last distinction is ABI v1 section 6's batching predicate and it
    is the whole reason M1 crosses three times for a thousand elements and M2
    crosses ten times per element;
  * **the decode side's mirror**, where section 7.2 refuses to batch a non-leaf
    element: `add_<slot>` for a leaf, `new_`/`apply_` per element plus a run per
    inner field otherwise.

**Strings are staged as UTF-8 and handed over as data**, with
`tc = ak_tc_bytes()`, a function pointer INTO the core that crosses nothing. The
alternative -- point `data` at the host's UTF-16 and pass `ak_tc_utf16()` -- is
also a pointer into the core and also crosses nothing; it is a separate arm
(`AK_UTF16=1`) because it trades a staging copy for a transcode the core drives.
"""
import abi_ir as A
import csnames as N
from cs_facade import Head, oneof_case_type

RUN_FN = {"i32": "ak_run_i32", "i64": "ak_run_i64", "f64": "ak_run_f64", "u8": "ak_run_u8"}


def facade_path(ir, owner, path):
    """`.A.B.C` for an ABI slot path, in the facade's spelling."""
    parts, cur = [], ir.msg(owner)
    for p in path:
        f = next(x for x in cur.walk() if x.name == p)
        parts.append(f.cs)
        if f.kind == "message":
            cur = ir.msg(f.of)
    return ".".join(parts)


def guard_path(ir, owner, path):
    """A null test for every singular message on the way to a slot, or None."""
    parts, cur, out = [], ir.msg(owner), []
    for p in path[:-1]:
        f = next(x for x in cur.walk() if x.name == p)
        parts.append(f.cs)
        out.append(".".join(parts))
        cur = ir.msg(f.of)
    return out


class Slot:
    """One loop slot, with everything the emitter needs about it precomputed."""

    def __init__(self, ir, owner, path, f):
        self.path, self.f = path, f
        self.name = A.slot_name(path)
        self.acc = facade_path(ir, owner, path)
        self.guards = guard_path(ir, owner, path)
        self.rust = A.slot_elem(f, True)
        self.drust = A.slot_elem(f, False)
        if f.card == "map":
            self.kind = "map"
            self.elem = A.entry_type(f)
        elif f.kind in ("string", "bytes"):
            self.kind = "blob"
            self.elem = None
        elif f.card == "packed":
            self.kind = "packed"
            self.elem = None
        elif f.kind == "message":
            self.kind = "msg"
            self.elem = f.of
        else:
            raise NotImplementedError("slot %s.%s" % (owner, f.name))
        if self.kind == "blob":
            self.cs_elem, self.cs_delem = "ak_str", "ak_span"
        elif self.kind == "packed":
            self.cs_elem = self.cs_delem = A.CS[self.rust]
        else:
            self.cs_elem = "ak_efix_%s" % self.elem
            self.cs_delem = "ak_dfix_%s" % self.elem
        self.leaf = A.is_leaf(ir, self.elem) if self.elem else True
        self.inner = ([Slot(ir, self.elem, p, g) for p, g in A.loop_slots(ir, self.elem)]
                      if self.elem and self.elem in ir.messages else [])

    def forward(self, ctx, ptr, n, tok=None):
        if self.kind == "blob":
            return "Abi.ak_blob_run(%s, %s, %s)" % (ctx, ptr, n)
        if self.kind == "packed":
            return "Abi.%s(%s, %s, (nuint)%s)" % (RUN_FN[self.rust], ctx, ptr, n)
        if self.leaf:
            return "Abi.ak_elem_%s(%s, %s, %s)" % (self.elem, ctx, ptr, n)
        return "Abi.ak_elemu_%s(%s, %s, %s, %s)" % (self.elem, ctx, ptr, n, tok)


def emit(ir, root):
    m = ir.msg(root)
    slots = [Slot(ir, root, p, f) for p, f in A.loop_slots(ir, root)]
    cls = "CoreFfi_%s" % root

    o = Head("The core-ffi arm for %s. Emitted from gen/abi_ir.py, like the ABI it calls."
             % root)
    o += "using System;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += "using System.Text;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""

    # ---------------- the run struct -------------------------------
    o.doc("What every callback is handed as `obj`. Native rather than managed: "
          "`[UnmanagedCallersOnly]` cannot capture, and a `GCHandle` round trip would "
          "put a managed allocation on a path whose whole point is not having one.")
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct Run_%s" % root
    o += "{"
    o += "    /// Elements per element-entry call; 0 hands the whole run over at once."
    o += "    public int Chunk;"
    for s in slots:
        o += "    public %s* S_%s;" % (s.cs_elem, s.name)
        o += "    public int N_%s;" % s.name
        for i in s.inner:
            o += "    /// %s.%s, flat across every element, with (offset, count) per element." % (s.elem, i.name)
            o += "    public %s* I_%s_%s;" % (i.cs_elem, s.name, i.name)
            o += "    public int* O_%s_%s;" % (s.name, i.name)
            o += "    public int* C_%s_%s;" % (s.name, i.name)
    o += "}"
    o += ""
    o += "public sealed unsafe class %s : IDisposable" % cls
    o += "{"
    o += "    private IntPtr _ctx, _dctx;"
    o += "    private byte* _staging;"
    o += "    private int _stagingCap;"
    o += "    private Run_%s* _run;" % root
    o += "    private DecRun* _drun;"
    o += "    private static IntPtr _tc;"
    o += "    private readonly bool _utf16;"
    o += "    public int Chunk;"
    o.doc("**Counted where the crossing happens, not predicted from the graph.** The "
      "predicted version was wrong and the R5 check caught it: a decode run is NOT one "
      "`add_` call, because the codec flushes when its element arena fills (ABI v1 7.3, "
      "a BYTE budget divided by the group size) and the host cannot know that size. "
      "Static because `[UnmanagedCallersOnly]` cannot reach an instance; one arm at a "
      "time, which is what the gate and the harness do.", "    ")
    o += "    private static long _fwd, _rev;"
    o += "    public long ForwardCalls => _fwd;"
    o += "    public long ReverseCalls => _rev;"
    for s in slots:
        if s.inner:
            o += "    private ak_evt_%s* _evt_%s;" % (s.elem, s.name)
    o += ""
    o.doc("`cap` is a sizing hint per slot, taken from the graph by the caller. An "
          "undersized array throws here rather than writing past it.", "    ")
    o += "    public %s(Caps c, bool utf16 = false)" % cls
    o += "    {"
    o += "        _ctx = Abi.ak_enc_ctx_new();"
    o += "        if (_ctx == IntPtr.Zero) throw new InvalidOperationException(\"ak_enc_ctx_new returned null\");"
    o += "        // ABI v1 section 4 offers two string forms and BOTH are pointers into"
    o += "        // the core, so neither costs a crossing. ak_tc_bytes takes UTF-8 the"
    o += "        // host already staged; ak_tc_utf16 reads the host's own UTF-16 in"
    o += "        // place and makes the staging copy unnecessary. AK_UTF16=1 picks it."
    o += "        _utf16 = utf16 || Environment.GetEnvironmentVariable(\"AK_UTF16\") == \"1\";"
    o += "        _tc = _utf16 ? Abi.ak_tc_utf16() : Abi.ak_tc_bytes();"
    o += "        _caps = c;"
    o += "        _stagingCap = c.Bytes;"
    o += "        _staging = (byte*)NativeMemory.Alloc((nuint)Math.Max(1, c.Bytes));"
    o += "        _run = (Run_%s*)NativeMemory.AllocZeroed((nuint)sizeof(Run_%s));" % (root, root)
    for s in slots:
        o += "        _run->S_%s = (%s*)NativeMemory.AlignedAlloc((nuint)(sizeof(%s) * Math.Max(1, c.N_%s)), 16);" % (
            s.name, s.cs_elem, s.cs_elem, s.name)
        for i in s.inner:
            o += "        _run->I_%s_%s = (%s*)NativeMemory.AlignedAlloc((nuint)(sizeof(%s) * Math.Max(1, c.N_%s_%s)), 16);" % (
                s.name, i.name, i.cs_elem, i.cs_elem, s.name, i.name)
            o += "        _run->O_%s_%s = (int*)NativeMemory.Alloc((nuint)(sizeof(int) * Math.Max(1, c.N_%s)));" % (
                s.name, i.name, s.name)
            o += "        _run->C_%s_%s = (int*)NativeMemory.Alloc((nuint)(sizeof(int) * Math.Max(1, c.N_%s)));" % (
                s.name, i.name, s.name)
        if s.inner:
            o += "        _evt_%s = (ak_evt_%s*)NativeMemory.AllocZeroed((nuint)sizeof(ak_evt_%s));" % (
                s.name, s.elem, s.elem)
            for i in s.inner:
                o += "        _evt_%s->loop_%s = &Loop_%s_%s;" % (s.name, i.name, s.name, i.name)
    o += "    }"
    o += ""
    o += "    private Caps _caps;"
    o += ""
    o.doc("The sizing hints, one per run array. Derived from the graph by the caller "
          "rather than guessed, because an undersized array is a wrong payload and a "
          "generous one is only memory.", "    ")
    o += "    public sealed class Caps"
    o += "    {"
    o += "        public int Bytes = 65536;"
    for s in slots:
        o += "        public int N_%s = 1;" % s.name
        for i in s.inner:
            o += "        public int N_%s_%s = 1;" % (s.name, i.name)
    o += "    }"
    o += ""
    emit_caps(o, ir, root, slots)
    emit_staging(o)
    emit_loops(o, ir, root, slots)
    emit_encode(o, ir, root, m, slots)
    emit_decode(o, ir, root, m, slots)
    emit_pull(o, ir, root, m, slots)
    emit_tail(o, root, slots)
    o += "}"
    return str(o)


def emit_staging(o):
    o.doc("One string, described to the core. In the staged form it is transcoded to "
          "UTF-8 here and `tc` is `ak_tc_bytes`; in the UTF-16 form the pointer is the "
          "host's own `string` data, pinned for the call, and `tc` is `ak_tc_utf16`. "
          "**An EMPTY string is reported absent** (`tc == null`), which for an "
          "implicit-presence field is the same thing on the wire.", "    ")
    o += "    private ak_str Stage(string s, ref int at)"
    o += "    {"
    o += "        if (s == null || s.Length == 0) return default;"
    o += "        if (_utf16)"
    o += "        {"
    o += "            // **UTF-16 staged, and the core transcodes.** The arm this exists to"
    o += "            // price is WHO DOES THE TRANSCODE, not whether a copy happens: a"
    o += "            // zero-copy form would hand the core a pointer into the managed heap,"
    o += "            // and on .NET that means a pinned GCHandle per string -- 5,000 of them"
    o += "            // for P1.2 -- which is a different and probably worse trade. So this"
    o += "            // copies the UTF-16 (a memcpy, no transcode) and lets ak_tc_utf16 do"
    o += "            // the conversion inside the core, against the staged form's"
    o += "            // Encoding.UTF8.GetBytes in the host followed by ak_tc_bytes copying."
    o += "            int nb = s.Length * 2;"
    o += "            if (at + nb > _stagingCap) throw new InvalidOperationException(\"staging buffer too small\");"
    o += "            fixed (char* cp = s)"
    o += "                Buffer.MemoryCopy(cp, _staging + at, _stagingCap - at, nb);"
    o += "            // `len` is the number of CODE UNITS, not of bytes: ak_tc_utf16 reads"
    o += "            // `*const u16`. Passing the byte count doubles every string and the"
    o += "            // byte-identity gate says so immediately -- 1,572 against 858 on P1.1."
    o += "            var u = new ak_str { data = (IntPtr)(_staging + at), len = (nuint)s.Length, tc = _tc };"
    o += "            at += nb;"
    o += "            return u;"
    o += "        }"
    o += "        int n = Encoding.UTF8.GetBytes(s.AsSpan(), new Span<byte>(_staging + at, _stagingCap - at));"
    o += "        var r = new ak_str { data = (IntPtr)(_staging + at), len = (nuint)n, tc = _tc };"
    o += "        at += n;"
    o += "        return r;"
    o += "    }"
    o += ""
    o += "    private ak_str StageBytes(byte[] b, ref int at)"
    o += "    {"
    o += "        if (b == null || b.Length == 0) return default;"
    o += "        if (at + b.Length > _stagingCap) throw new InvalidOperationException(\"staging buffer too small\");"
    o += "        new ReadOnlySpan<byte>(b).CopyTo(new Span<byte>(_staging + at, _stagingCap - at));"
    o += "        // bytes are bytes in both forms: ak_tc_bytes copies them through."
    o += "        var r = new ak_str { data = (IntPtr)(_staging + at), len = (nuint)b.Length, tc = Abi.ak_tc_bytes() };"
    o += "        at += b.Length;"
    o += "        return r;"
    o += "    }"
    o += ""
    o.doc("The explicit-presence form. A present-and-EMPTY string still has to be "
          "written, so `tc` is set even at length zero -- `enc_blob` returns without "
          "writing when `tc` is null, which is how an absent field is spelled.", "    ")
    o += "    private ak_str StagePresent(string s, ref int at)"
    o += "    {"
    o += "        if (s == null) return default;"
    o += "        if (s.Length == 0) return new ak_str { data = IntPtr.Zero, len = 0, tc = _tc };"
    o += "        return Stage(s, ref at);"
    o += "    }"
    o += ""
    o += "    private ak_str StageBytesPresent(byte[] v, ref int at)"
    o += "    {"
    o += "        if (v == null) return default;"
    o += "        if (v.Length == 0) return new ak_str { data = IntPtr.Zero, len = 0, tc = Abi.ak_tc_bytes() };"
    o += "        return StageBytes(v, ref at);"
    o += "    }"
    o += ""



def emit_loops(o, ir, root, slots):
    o.doc("One reverse call per slot. For a LEAF element the whole run goes over in "
          "one forward call whatever its length, which is ABI v1 section 6's batching "
          "predicate; for a non-leaf the codec calls back per element and this is where "
          "the crossing count stops being constant.", "    ")
    for s in slots:
        o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
        o += "    private static int Loop_%s(IntPtr ctx, void* obj, long token)" % s.name
        o += "    {"
        o += "        _rev++;"
        o += "        try"
        o += "        {"
        o += "            var run = (Run_%s*)obj;" % root
        o += "            int n = run->N_%s;" % s.name
        o += "            if (n == 0) return 0;"
        if s.kind == "msg":
            o += "            int chunk = run->Chunk <= 0 ? n : run->Chunk;"
            o += "            for (int off = 0; off < n; off += chunk)"
            o += "            {"
            o += "                int k = n - off; if (k > chunk) k = chunk;"
            o += "                // tok0 is the run's FIRST index, so the token an element's own"
            o += "                // loop callbacks get is the global index whatever the chunk size."
            o += "                _fwd++;"
            o += "                int rc = %s;" % s.forward("ctx", "run->S_%s + off" % s.name, "k", "off")
            o += "                if (rc < 0) return rc;"
            o += "            }"
            o += "            return 0;"
        else:
            o += "            _fwd++;"
            o += "            return %s;" % s.forward("ctx", "run->S_%s" % s.name, "n")
        o += "        }"
        o += "        catch { return Abi.AK_ERR_HOST; }"
        o += "    }"
        o += ""
        for i in s.inner:
            o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
            o += "    private static int Loop_%s_%s(IntPtr ctx, void* obj, long token)" % (s.name, i.name)
            o += "    {"
            o += "        _rev++;"
            o += "        try"
            o += "        {"
            o += "            var run = (Run_%s*)obj;" % root
            o += "            int e = (int)token;"
            o += "            int n = run->C_%s_%s[e];" % (s.name, i.name)
            o += "            if (n == 0) return 0;"
            o += "            _fwd++;"
            o += "            return %s;" % i.forward(
                "ctx", "run->I_%s_%s + run->O_%s_%s[e]" % (s.name, i.name, s.name, i.name), "n")
            o += "        }"
            o += "        catch { return Abi.AK_ERR_HOST; }"
            o += "    }"
            o += ""


# ------------------------------------------------------------------ encode

def fill(ir, msg, g, src, ind, lines, pres):
    """The by-value group, filled from the facade. Recurses into singular children."""
    bits = A.presence_bits(msg)
    for f in msg.plain():
        if f.card != "singular":
            continue                       # a loop slot, not part of the group
        gg, ss = "%s.%s" % (g, f.name), "%s.%s" % (src, f.cs)
        if f.kind == "message":
            lines.append("%sif (%s != null)" % (ind, ss))
            lines.append("%s{" % ind)
            fill(ir, ir.msg(f.of), gg, ss, ind + "    ", lines, pres)
            lines.append("%s    %s.presence |= AkPresent.%s;" % (
                ind, g, A.presence_const(msg, f.name, True)))
            lines.append("%s}" % ind)
        elif f.presence == "explicit":
            # **The presence WORD carries this, not the value**, and that is the
            # whole of design/SHAPES.md's M3. A present-and-empty string has
            # length 0 and must still be written; a by-value group reports it
            # identically to absent unless it is designed not to. Checked BEFORE
            # the kind dispatch, because an explicit string that fell through to
            # the implicit case set no bit at all and lost 1,351 bytes of P3.1.
            bit = A.presence_const(msg, f.name, True)
            if f.kind in ("string", "bytes"):
                lines.append("%sif (%s != null)" % (ind, ss))
                lines.append("%s{" % ind)
                lines.append("%s    %s = %s(%s, ref at);" % (
                    ind, gg, "StagePresent" if f.kind == "string" else "StageBytesPresent", ss))
            else:
                lines.append("%sif (%s.HasValue)" % (ind, ss))
                lines.append("%s{" % ind)
                lines.append("%s    %s = %s;" % (ind, gg, scalar_in(f, ss + ".Value")))
            lines.append("%s    %s.presence |= AkPresent.%s;" % (ind, g, bit))
            lines.append("%s}" % ind)
        elif f.kind == "string":
            lines.append("%s%s = Stage(%s, ref at);" % (ind, gg, ss))
        elif f.kind == "bytes":
            lines.append("%s%s = StageBytes(%s, ref at);" % (ind, gg, ss))
        else:
            lines.append("%s%s = %s;" % (ind, gg, scalar_in(f, ss)))
    for oname, members in msg.oneofs.items():
        ct = oneof_case_type(msg, oname)
        lines.append("%s// The discriminant carries the ACTIVE MEMBER'S TAG, and the facade's" % ind)
        lines.append("%s// own case enum is already tag-valued, so this is a cast." % ind)
        lines.append("%s%s.%s_case = (uint)%s.%sCase;" % (ind, g, oname, src, N.pascal(oname)))
        lines.append("%sswitch (%s.%sCase)" % (ind, src, N.pascal(oname)))
        lines.append("%s{" % ind)
        for gm in members:
            gg = "%s.%s_%s" % (g, oname, gm.name)
            ss = "%s.%s" % (src, gm.cs)
            lines.append("%s    case %s.%s:" % (ind, ct, N.pascal(gm.name)))
            if gm.kind == "string":
                lines.append("%s        %s = Stage(%s, ref at); break;" % (ind, gg, ss))
            elif gm.kind == "bytes":
                lines.append("%s        %s = StageBytes(%s, ref at); break;" % (ind, gg, ss))
            elif gm.kind == "message":
                lines.append("%s        {" % ind)
                sub = []
                fill(ir, ir.msg(gm.of), gg, ss, ind + "            ", sub, pres)
                lines.extend(sub)
                lines.append("%s            break;" % ind)
                lines.append("%s        }" % ind)
            else:
                lines.append("%s        %s = %s; break;" % (ind, gg, scalar_in(gm, ss)))
        lines.append("%s    default: break;" % ind)
        lines.append("%s}" % ind)


def scalar_in(f, e):
    if f.kind == "bool":
        return "%s ? (byte)1 : (byte)0" % e
    if f.kind == "enum":
        return "(int)%s" % e
    return e


def scalar_out(f, e):
    if f.kind == "bool":
        return "%s != 0" % e
    if f.kind == "enum":
        return "(%s)%s" % (f.of, e)
    return e


def emit_encode(o, ir, root, m, slots):
    o.doc("Fill the by-value group and the run arrays, then ONE forward call. `Fill` "
          "stops before that call, so the difference between the two is the codec plus "
          "every crossing -- measured rather than subtracted.", "    ")
    o += "    public int Fill(%s src) => Go(src, false, out _, out _);" % root
    o += ""
    o += "    public int Encode(%s src, out byte* outPtr, out int outLen) => Go(src, true, out outPtr, out outLen);" % root
    o += ""
    o += "    private int Go(%s src, bool call, out byte* outPtr, out int outLen)" % root
    o += "    {"
    o += "        Abi.ak_enc_reset(_ctx);"
    o += "        int at = 0;"
    o += "        var fix = new ak_efix_%s();" % root
    lines = []
    fill(ir, m, "fix", "src", "        ", lines, True)
    for ln in lines:
        o += ln
    o += "        _run->Chunk = Chunk;"
    for s in slots:
        guards = " && ".join("src.%s != null" % g for g in s.guards)
        acc = "src." + s.acc
        o += "        {"
        o += "            var lst = %s;" % (("(%s) ? %s : null" % (guards, acc)) if guards else acc)
        if guards:
            o += "            // A slot inside a singular child that is absent has no run."
        o += "            int n = lst == null ? 0 : lst.Count;"
        o += "            if (n > _caps.N_%s) throw new InvalidOperationException(\"run %s: %%d > cap %%d\".Replace(\"%%d\", \"\") + n + \" > \" + _caps.N_%s);" % (
            s.name, s.name, s.name)
        o += "            _run->N_%s = n;" % s.name
        if s.kind == "blob":
            o += "            for (int i = 0; i < n; i++) _run->S_%s[i] = Stage(lst[i], ref at);" % s.name
        elif s.kind == "packed":
            o += "            for (int i = 0; i < n; i++) _run->S_%s[i] = %s;" % (
                s.name, scalar_in(s.f, "lst[i]"))
        elif s.kind == "map":
            o += "            // Indexed, not foreach: OrderedMap.GetEnumerator returns the"
            o += "            // interface, so a foreach boxes the List enumerator per call."
            o += "            for (int i = 0; i < n; i++)"
            o += "            {"
            o += "                var kv = lst.At(i);"
            o += "                ref var e = ref _run->S_%s[i];" % s.name
            o += "                e = default;"
            o += "                e.key = Stage(kv.Key, ref at);"
            o += "                e.value = Stage(kv.Value, ref at);"
            o += "            }"
        else:
            for i in s.inner:
                o += "            int bi_%s = 0;" % i.name
            o += "            for (int i = 0; i < n; i++)"
            o += "            {"
            o += "                var t = lst[i];"
            o += "                ref var g = ref _run->S_%s[i];" % s.name
            o += "                g = default;"
            el = []
            fill(ir, ir.msg(s.elem), "g", "t", "                ", el, True)
            for ln in el:
                o += ln
            for i in s.inner:
                iacc = "t." + i.acc
                iguards = " && ".join("t.%s != null" % gp for gp in i.guards)
                o += "                {"
                o += "                    var il = %s;" % (("(%s) ? %s : null" % (iguards, iacc)) if iguards else iacc)
                o += "                    int m2 = il == null ? 0 : il.Count;"
                o += "                    _run->O_%s_%s[i] = bi_%s;" % (s.name, i.name, i.name)
                o += "                    _run->C_%s_%s[i] = m2;" % (s.name, i.name)
                if i.kind == "blob":
                    o += "                    for (int j = 0; j < m2; j++) _run->I_%s_%s[bi_%s++] = Stage(il[j], ref at);" % (
                        s.name, i.name, i.name)
                elif i.kind == "packed":
                    o += "                    for (int j = 0; j < m2; j++) _run->I_%s_%s[bi_%s++] = %s;" % (
                        s.name, i.name, i.name, scalar_in(i.f, "il[j]"))
                elif i.kind == "map":
                    o += "                    for (int j = 0; j < m2; j++)"
                    o += "                    {"
                    o += "                        var kv = il.At(j);"
                    o += "                        ref var e = ref _run->I_%s_%s[bi_%s++];" % (s.name, i.name, i.name)
                    o += "                        e = default;"
                    o += "                        e.key = Stage(kv.Key, ref at);"
                    o += "                        e.value = Stage(kv.Value, ref at);"
                    o += "                    }"
                else:
                    raise NotImplementedError("inner slot kind %s" % i.kind)
                o += "                    if (bi_%s > _caps.N_%s_%s) throw new InvalidOperationException(\"inner run %s_%s too small\");" % (
                    i.name, s.name, i.name, s.name, i.name)
                o += "                }"
            o += "            }"
        o += "        }"
    o += "        var vt = new ak_evt_%s" % root
    o += "        {"
    for s in slots:
        o += "            loop_%s = &Loop_%s," % (s.name, s.name)
        if s.inner:
            o += "            elem_%s = _evt_%s," % (s.name, s.name)
    if not slots:
        o += "            _reserved = IntPtr.Zero,"
    o += "        };"
    o += "        // Before the tally, so the fill-only arm does not count crossings it"
    o += "        // did not make."
    o += "        if (!call) { outPtr = null; outLen = at; return at; }"
    o += "        _fwd++;   // ak_encode_*; every other crossing is counted where it happens"
    o += "        nint rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix);" % root
    o += "        if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\");"
    o += "        byte* p; nuint len;"
    o += "        int tk = Abi.ak_enc_take(_ctx, &p, &len);"
    o += "        if (tk != 0) throw new InvalidOperationException($\"ak_enc_take failed: {tk}\");"
    o += "        outPtr = p; outLen = (int)len;"
    o += "        return outLen;"
    o += "    }"
    o += ""
    o += "    public byte[] EncodeToArray(%s src)" % root
    o += "    {"
    o += "        Encode(src, out byte* p, out int len);"
    o += "        var a = new byte[len];"
    o += "        new ReadOnlySpan<byte>(p, len).CopyTo(a);"
    o += "        return a;"
    o += "    }"
    o += ""


# ------------------------------------------------------------------ decode

def unfill(ir, msg, d, t, ind, lines, keep=()):
    for f in msg.plain():
        if f.card != "singular":
            continue
        dd, tt = "%s.%s" % (d, f.name), "%s.%s" % (t, f.cs)
        if f.kind == "message":
            lines.append("%sif ((%s.presence & AkPresent.%s) != 0)" % (
                ind, d, A.presence_const(msg, f.name, False)))
            lines.append("%s{" % ind)
            lines.append("%s    %s %s= new %s();" % (ind, tt, "??" if f.name in keep else "", f.of))
            unfill(ir, ir.msg(f.of), dd, tt, ind + "    ", lines, keep)
            lines.append("%s}" % ind)
        elif f.presence == "explicit":
            # Absent stays null / no value. Reading the span unconditionally
            # turns an ABSENT explicit string into an empty one, which is the
            # difference M3 exists to test.
            val = ("Str(b, %s)" % dd if f.kind == "string"
                   else "Bytes(b, %s)" % dd if f.kind == "bytes" else scalar_out(f, dd))
            lines.append("%s%s = (%s.presence & AkPresent.%s) != 0 ? %s : null;" % (
                ind, tt, d, A.presence_const(msg, f.name, False), val))
        elif f.kind == "string":
            lines.append("%s%s = Str(b, %s);" % (ind, tt, dd))
        elif f.kind == "bytes":
            lines.append("%s%s = Bytes(b, %s);" % (ind, tt, dd))
        else:
            lines.append("%s%s = %s;" % (ind, tt, scalar_out(f, dd)))
    for oname, members in msg.oneofs.items():
        ct = oneof_case_type(msg, oname)
        lines.append("%s%s.%sCase = (%s)%s.%s_case;" % (ind, t, N.pascal(oname), ct, d, oname))
        lines.append("%sswitch (%s.%s_case)" % (ind, d, oname))
        lines.append("%s{" % ind)
        for gm in members:
            dd = "%s.%s_%s" % (d, oname, gm.name)
            tt = "%s.%s" % (t, gm.cs)
            lines.append("%s    case %d:" % (ind, gm.tag))
            if gm.kind == "string":
                lines.append("%s        %s = Str(b, %s); break;" % (ind, tt, dd))
            elif gm.kind == "bytes":
                lines.append("%s        %s = Bytes(b, %s); break;" % (ind, tt, dd))
            elif gm.kind == "message":
                lines.append("%s        {" % ind)
                lines.append("%s            %s = new %s();" % (ind, tt, gm.of))
                sub = []
                unfill(ir, ir.msg(gm.of), dd, tt, ind + "            ", sub, keep)
                lines.extend(sub)
                lines.append("%s            break;" % ind)
                lines.append("%s        }" % ind)
            else:
                lines.append("%s        %s = %s; break;" % (ind, tt, scalar_out(gm, dd)))
        lines.append("%s    default: break;" % ind)
        lines.append("%s}" % ind)


def emit_decode(o, ir, root, m, slots):
    o.doc("The decode callbacks' `obj`: a handle to the graph being filled, and the "
          "buffer `ak_span` offsets are relative to.", "    ")
    o += "    [StructLayout(LayoutKind.Sequential)]"
    o += "    private struct DecRun { public IntPtr Target; public byte* Buf; }"
    o += ""
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += "    private static %s Tgt(void* obj) => (%s)GCHandle.FromIntPtr(((DecRun*)obj)->Target).Target;" % (root, root)
    o += ""
    o.doc("**A CEILING for ABI v1 decision 13, not an implementation of it.** The "
          "decode side already hands the host `ak_span` OFFSETS into its own buffer, "
          "so the ABI is ready for a borrowed string view and the facade's `string` "
          "is what is not. Redesigning the facade is a public-surface change with a "
          "lifetime rule attached; bounding the prize first is cheaper and says "
          "whether it is worth it. With `SkipStrings` set, the decode does everything "
          "it otherwise does and materialises no string at all, so the gap between "
          "that and the real arm is the MOST a borrowed view could ever save. R2's "
          "floor-arm logic, applied to a design question.", "    ")
    o += "    public static bool SkipStrings;"
    o += ""
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += '    private static string Str(byte* b, ak_span s) => s.len == 0 || SkipStrings ? "" : Encoding.UTF8.GetString(b + s.off, (int)s.len);'
    o += ""
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += "    private static byte[] Bytes(byte* b, ak_span s)"
    o += "    {"
    o += "        if (s.len == 0) return Array.Empty<byte>();"
    o += "        var a = new byte[s.len];"
    o += "        new ReadOnlySpan<byte>(b + s.off, (int)s.len).CopyTo(a);"
    o += "        return a;"
    o += "    }"
    o += ""
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static void ApplyRoot(IntPtr ctx, void* obj, ak_dfix_%s* fix)" % root
    o += "    {"
    o += "        _rev++;"
    o += "        try"
    o += "        {"
    o += "            var run = (DecRun*)obj;"
    o += "            var t = (%s)GCHandle.FromIntPtr(run->Target).Target;" % root
    o += "            byte* b = run->Buf;"
    o += "            ref var d = ref *fix;"
    lines = []
    unfill(ir, m, "d", "t", "            ", lines)
    for ln in lines:
        o += ln
    o += "        }"
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
    o += "    }"
    o += ""
    for s in slots:
        emit_decode_slot(o, ir, root, s)
    o += "    public %s Decode(byte[] src, int len)" % root
    o += "    {"
    o += "        if (_dctx == IntPtr.Zero)"
    o += "        {"
    o += "            _dctx = Abi.ak_dec_ctx_new();"
    o += "            _drun = (DecRun*)NativeMemory.Alloc((nuint)sizeof(DecRun));"
    o += "        }"
    o += "        Abi.ak_dec_err_reset(_dctx);"
    o += "        var target = new %s();" % root
    o += "        var h = GCHandle.Alloc(target);"
    o += "        try"
    o += "        {"
    o += "            fixed (byte* b = src)"
    o += "            {"
    o += "                _drun->Target = GCHandle.ToIntPtr(h);"
    o += "                _drun->Buf = b;"
    o += "                var vt = new ak_dvt_%s" % root
    o += "                {"
    o += "                    apply = &ApplyRoot,"
    o += "                    unknown = IntPtr.Zero,"
    for s in slots:
        if s.elem:
            o += "                    unk_%s = IntPtr.Zero," % s.name
        if s.leaf:
            o += "                    add_%s = &Add_%s," % (s.name, s.name)
        else:
            o += "                    new_%s = &New_%s," % (s.name, s.name)
            o += "                    apply_%s = &Apply_%s," % (s.name, s.name)
            for i in s.inner:
                o += "                    add_%s_%s = &Add_%s_%s," % (s.name, i.name, s.name, i.name)
    o += "                };"
    o += "                _fwd++;   // ak_decode_*"
    o += "                int rc = Abi.ak_decode_%s(_dctx, _drun, b, (nuint)len, &vt);" % root
    o += "                if (rc < 0) throw new InvalidOperationException($\"core decode failed: {rc}\");"
    o += "                int he = Abi.ak_dec_err(_dctx);"
    o += "                if (he != 0) throw new InvalidOperationException($\"host reported {he} through ak_fail\");"
    o += "            }"
    o += "        }"
    o += "        finally { h.Free(); }"
    o += "        return target;"
    o += "    }"
    o += ""


def emit_decode_slot(o, ir, root, s):
    if s.leaf:
        o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
        o += "    private static void Add_%s(IntPtr ctx, void* obj, long token, %s* xs, int n)" % (
            s.name, s.cs_delem)
        o += "    {"
        o += "        _rev++;"
        o += "        try"
        o += "        {"
        o += "            var run = (DecRun*)obj;"
        o += "            var t = (%s)GCHandle.FromIntPtr(run->Target).Target;" % root
        o += "            byte* b = run->Buf;"
        for gp in s.guards:
            o += "            t.%s ??= new %s();" % (gp, ir_child_type(ir, root, s, gp))
        o += "            var lst = t.%s;" % s.acc
        if s.kind == "blob":
            o += "            for (int i = 0; i < n; i++) lst.Add(Str(b, xs[i]));"
        elif s.kind == "packed":
            o += "            for (int i = 0; i < n; i++) lst.Add(%s);" % scalar_out(s.f, "xs[i]")
        elif s.kind == "map":
            o += "            for (int i = 0; i < n; i++) lst[Str(b, xs[i].key)] = Str(b, xs[i].value);"
        else:
            o += "            for (int i = 0; i < n; i++)"
            o += "            {"
            o += "                ref var d = ref xs[i];"
            o += "                var e = new %s();" % s.elem
            lines = []
            unfill(ir, ir.msg(s.elem), "d", "e", "                ", lines)
            for ln in lines:
                o += ln
            o += "                lst.Add(e);"
            o += "            }"
        o += "        }"
        o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
        o += "    }"
        o += ""
        return

    o.doc("ABI v1 section 7.2's refusal, in the host: `new` then `apply` per element, "
          "because there would be nothing to attach the element's own inner runs to.", "    ")
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static long New_%s(IntPtr ctx, void* obj)" % s.name
    o += "    {"
    o += "        _rev++;"
    o += "        try { var t = Tgt(obj); t.%s.Add(new %s()); return t.%s.Count - 1; }" % (
        s.acc, s.elem, s.acc)
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); return -1; }"
    o += "    }"
    o += ""
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static void Apply_%s(IntPtr ctx, void* obj, long token, ak_dfix_%s* fix)" % (
        s.name, s.elem)
    o += "    {"
    o += "        _rev++;"
    o += "        try"
    o += "        {"
    o += "            var run = (DecRun*)obj;"
    o += "            var t = (%s)GCHandle.FromIntPtr(run->Target).Target;" % root
    o += "            byte* b = run->Buf;"
    o += "            var e = t.%s[(int)token];" % s.acc
    o += "            ref var d = ref *fix;"
    lines = []
    keep = tuple(i.path[0] for i in s.inner if len(i.path) > 1)
    unfill(ir, ir.msg(s.elem), "d", "e", "            ", lines, keep=keep)
    for ln in lines:
        o += ln
    o += "        }"
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
    o += "    }"
    o += ""
    for i in s.inner:
        o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
        o += "    private static void Add_%s_%s(IntPtr ctx, void* obj, long token, %s* xs, int n)" % (
            s.name, i.name, i.cs_delem)
        o += "    {"
        o += "        _rev++;"
        o += "        try"
        o += "        {"
        o += "            var run = (DecRun*)obj;"
        o += "            var t = (%s)GCHandle.FromIntPtr(run->Target).Target;" % root
        o += "            byte* b = run->Buf;"
        o += "            var e = t.%s[(int)token];" % s.acc
        holder, cur = "e", ir.msg(s.elem)
        for p in i.path[:-1]:
            fld = next(x for x in cur.walk() if x.name == p)
            o += "            %s.%s ??= new %s();" % (holder, fld.cs, fld.of)
            holder = "%s.%s" % (holder, fld.cs)
            cur = ir.msg(fld.of)
        o += "            var lst = %s.%s;" % (holder, i.acc.split(".")[-1])
        if i.kind == "blob":
            o += "            for (int k = 0; k < n; k++) lst.Add(Str(b, xs[k]));"
        elif i.kind == "packed":
            o += "            for (int k = 0; k < n; k++) lst.Add(%s);" % scalar_out(i.f, "xs[k]")
        elif i.kind == "map":
            o += "            for (int k = 0; k < n; k++) lst[Str(b, xs[k].key)] = Str(b, xs[k].value);"
        o += "        }"
        o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
        o += "    }"
        o += ""


def ir_child_type(ir, root, s, gpath):
    cur = ir.msg(root)
    last = None
    for p in s.path[:-1]:
        f = next(x for x in cur.walk() if x.name == p)
        last = f.of
        cur = ir.msg(f.of)
        if ".".join(x for x in [last]) and f.cs == gpath.split(".")[-1]:
            return f.of
    return last


def emit_tail(o, root, slots):
    o += "    /// Zero the host's own tally. The core's counters have their own reset."
    o += "    public void CallsReset() { _fwd = 0; _rev = 0; }"
    o.doc("What the PULL family trades the upcalls FOR: a record buffer in the "
          "decode context, proportional to the decoded payload. `ak_bdr_footprint` "
          "reports the high-water mark, so the trade can be read in both directions "
          "rather than only in time.", "    ")
    o += "    public long PullFootprint() => _dctx == IntPtr.Zero ? 0 : (long)Abi.ak_bdr_footprint(_dctx);"
    o += ""
    o += "    public AkCounters EncCounters() { AkCounters c; Abi.ak_enc_counters(_ctx, &c); return c; }"
    o += "    public void EncCountersReset() => Abi.ak_enc_counters_reset(_ctx);"
    o += "    public AkCounters DecCounters() { AkCounters c; if (_dctx == IntPtr.Zero) return default; Abi.ak_dec_counters(_dctx, &c); return c; }"
    o += "    public void DecCountersReset() { if (_dctx != IntPtr.Zero) Abi.ak_dec_counters_reset(_dctx); }"
    o += ""
    o += "    public void Dispose()"
    o += "    {"
    o += "        if (_ctx != IntPtr.Zero) { Abi.ak_enc_ctx_free(_ctx); _ctx = IntPtr.Zero; }"
    o += "        if (_dctx != IntPtr.Zero) { Abi.ak_dec_ctx_free(_dctx); _dctx = IntPtr.Zero; }"
    o += "        if (_staging != null) { NativeMemory.Free(_staging); _staging = null; }"
    o += "        if (_run != null)"
    o += "        {"
    for s in slots:
        o += "            if (_run->S_%s != null) NativeMemory.AlignedFree(_run->S_%s);" % (s.name, s.name)
        for i in s.inner:
            o += "            if (_run->I_%s_%s != null) NativeMemory.AlignedFree(_run->I_%s_%s);" % (
                s.name, i.name, s.name, i.name)
            o += "            if (_run->O_%s_%s != null) NativeMemory.Free(_run->O_%s_%s);" % (
                s.name, i.name, s.name, i.name)
            o += "            if (_run->C_%s_%s != null) NativeMemory.Free(_run->C_%s_%s);" % (
                s.name, i.name, s.name, i.name)
    o += "            NativeMemory.Free(_run); _run = null;"
    o += "        }"
    for s in slots:
        if s.inner:
            o += "        if (_evt_%s != null) { NativeMemory.Free(_evt_%s); _evt_%s = null; }" % (
                s.name, s.name, s.name)
    o += "        if (_drun != null) { NativeMemory.Free(_drun); _drun = null; }"
    o += "    }"


def emit_caps(o, ir, root, slots):
    """A `Caps` built from the graph, emitted rather than written per root.

    The hand-written version of this covered one root and had to know its field
    names; getting a byte budget wrong there is an exception at best and a
    truncated string at worst. This walks the same slot table the binding does.
    """
    o.doc("Size every run array from the graph that will be encoded. Emitted from the "
          "slot table, so a slot cannot be sized and then not filled, or filled and not "
          "sized. The byte budget is `4 * UTF-16 length`, the worst case for UTF-8.", "    ")
    o += "    public static Caps CapsFor(%s src)" % root
    o += "    {"
    o += "        var c = new Caps();"
    o += "        int bytes = 4096;"
    m = ir.msg(root)
    o += "        bytes += %s;" % (bytes_of(ir, m, "src") or "0")
    for s in slots:
        guards = " && ".join("src.%s != null" % g for g in s.guards)
        acc = ("(%s) ? src.%s : null" % (guards, s.acc)) if guards else "src." + s.acc
        o += "        {"
        o += "            var lst = %s;" % acc
        o += "            int n = lst == null ? 0 : lst.Count;"
        o += "            c.N_%s = n + 1;" % s.name
        if s.kind == "blob":
            o += "            for (int i = 0; i < n; i++) bytes += 4 * (lst[i] == null ? 0 : lst[i].Length);"
        elif s.kind == "map":
            o += "            for (int i = 0; i < n; i++) { var kv = lst.At(i); bytes += 4 * (kv.Key.Length + (kv.Value == null ? 0 : kv.Value.Length)); }"
        elif s.kind == "msg":
            for i in s.inner:
                o += "            int t_%s = 0;" % i.name
            o += "            for (int i = 0; i < n; i++)"
            o += "            {"
            o += "                var t = lst[i];"
            b = bytes_of(ir, ir.msg(s.elem), "t")
            if b:
                o += "                bytes += %s;" % b
            for i in s.inner:
                ig = " && ".join("t.%s != null" % gp for gp in i.guards)
                iacc = ("(%s) ? t.%s : null" % (ig, i.acc)) if ig else "t." + i.acc
                o += "                {"
                o += "                    var il = %s;" % iacc
                o += "                    int k = il == null ? 0 : il.Count;"
                o += "                    t_%s += k;" % i.name
                if i.kind == "blob":
                    o += "                    for (int j = 0; j < k; j++) bytes += 4 * (il[j] == null ? 0 : il[j].Length);"
                elif i.kind == "map":
                    o += "                    for (int j = 0; j < k; j++) { var kv = il.At(j); bytes += 4 * (kv.Key.Length + (kv.Value == null ? 0 : kv.Value.Length)); }"
                o += "                }"
            o += "            }"
            for i in s.inner:
                o += "            c.N_%s_%s = t_%s + 1;" % (s.name, i.name, i.name)
        o += "        }"
    o += "        c.Bytes = bytes;"
    o += "        return c;"
    o += "    }"
    o += ""


def bytes_of(ir, msg, src, depth=0):
    """A worst-case UTF-8 byte budget for one message's SINGULAR string and bytes
    fields, oneof members included. Recurses into singular children."""
    if depth > 6:
        return None
    terms = []
    for f in msg.walk():
        if f.card != "singular":
            continue
        acc = "%s.%s" % (src, f.cs)
        if f.kind == "string":
            terms.append("4 * (%s == null ? 0 : %s.Length)" % (acc, acc))
        elif f.kind == "bytes":
            terms.append("(%s == null ? 0 : %s.Length)" % (acc, acc))
        elif f.kind == "message":
            inner = bytes_of(ir, ir.msg(f.of), acc, depth + 1)
            if inner:
                terms.append("(%s == null ? 0 : %s)" % (acc, inner))
    return " + ".join(terms) if terms else None


def emit_registry(ir, roots, payloads):
    """A by-payload registry, so the gate and the bench do not each hand-write
    a switch over sixteen payloads and seven roots.

    The hand-written version of this covered M1 and then M2 and was going to be
    written five more times. Each copy is a place for a payload to be quietly
    missing, and a missing payload reads as "not covered" rather than as a bug.
    """
    o = Head("Every payload's core-ffi arm, by id. Emitted, so none can be missing.")
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""
    o.doc("One payload's binding, behind an interface so the gate and the bench can "
          "drive all sixteen without knowing which root each one is.")
    o += "public unsafe interface ICoreArm : IDisposable"
    o += "{"
    o += "    string Root { get; }"
    o += "    int Chunk { get; set; }"
    o += "    long ForwardCalls { get; }"
    o += "    long ReverseCalls { get; }"
    o += "    byte[] EncodeToArray();"
    o += "    int EncodeNoCopy();"
    o += "    int Fill();"
    o += "    /// Decodes and returns an O(1) value, so the arm is not timed building a"
    o += "    /// string of the result. The graph is parked in `Sink`."
    o += "    int Decode(byte[] src, int len);"
    o += "    /// ABI v1 7.1's PULL family: parse into a record stream, then replay it."
    o += "    /// No reverse call is made at all, which is the claim under test."
    o += "    int Pull(byte[] src, int len);"
    o += "    object Sink { get; }"
    o += "    bool SameAsSource();"
    o += "    AkCounters EncCounters();"
    o += "    void EncCountersReset();"
    o += "    AkCounters DecCounters();"
    o += "    void DecCountersReset();"
    o += "    void CallsReset();"
    o += "    /// See CoreFfi_*.SkipStrings: a CEILING for decision 13, not an implementation."
    o += "    bool SkipStrings { get; set; }"
    o += "    long PullFootprint();"
    o += "}"
    o += ""
    for r in roots:
        slots = [Slot(ir, r, p, f) for p, f in A.loop_slots(ir, r)]
        first = slots[0].acc if slots else None
        o += "public sealed unsafe class Arm_%s : ICoreArm" % r
        o += "{"
        o += "    private readonly CoreFfi_%s _c;" % r
        o += "    private readonly %s _src;" % r
        o += "    private %s _sink;" % r
        o += "    public Arm_%s(%s src, bool utf16 = false) { _src = src; _c = new CoreFfi_%s(CoreFfi_%s.CapsFor(src), utf16); }" % (r, r, r, r)
        o += "    public string Root => \"%s\";" % r
        o += "    public int Chunk { get => _c.Chunk; set => _c.Chunk = value; }"
        o += "    public long ForwardCalls => _c.ForwardCalls;"
        o += "    public long ReverseCalls => _c.ReverseCalls;"
        o += "    public byte[] EncodeToArray() => _c.EncodeToArray(_src);"
        o += "    public int EncodeNoCopy() { _c.Encode(_src, out byte* p, out int l); return l; }"
        o += "    public int Fill() => _c.Fill(_src);"
        o += "    public int Decode(byte[] src, int len) { _sink = _c.Decode(src, len); return %s; }" % (
            "_sink.%s.Count" % first if first else "1")
        o += "    public int Pull(byte[] src, int len) { _sink = _c.Pull(src, len); return %s; }" % (
            "_sink.%s.Count" % first if first else "1")
        o += "    public object Sink => _sink;"
        o += "    public bool SameAsSource() => Eq.Same%s(_sink, _src);" % r
        o += "    public AkCounters EncCounters() => _c.EncCounters();"
        o += "    public void EncCountersReset() => _c.EncCountersReset();"
        o += "    public AkCounters DecCounters() => _c.DecCounters();"
        o += "    public void DecCountersReset() => _c.DecCountersReset();"
        o += "    public void CallsReset() => _c.CallsReset();"
        o += "    public bool SkipStrings { get => CoreFfi_%s.SkipStrings; set => CoreFfi_%s.SkipStrings = value; }" % (r, r)
        o += "    public long PullFootprint() => _c.PullFootprint();"
        o += "    public void Dispose() => _c.Dispose();"
        o += "}"
        o += ""
    o += "public static class CoreArms"
    o += "{"
    o.doc("Every payload that has a binding, in manifest order. A payload whose root "
          "has no core-ffi binding is simply absent, and the gate reports that as a gap "
          "with a reason rather than as a pass.", "    ")
    o += "    public static readonly string[] Ids = { %s };" % ", ".join(
        '"%s"' % p for p, _ in payloads)
    o += ""
    o += "    public static ICoreArm New(string id, bool utf16 = false)"
    o += "    {"
    o += "        switch (id)"
    o += "        {"
    for pid, r in payloads:
        o += "            case \"%s\": return new Arm_%s(BuildFacade.%s());" % (
            pid, r, pid.replace(".", "_"))
    o += "            default: throw new ArgumentException(\"no core-ffi arm for \" + id);"
    o += "        }"
    o += "    }"
    o += "}"
    return str(o)


def emit_pull(o, ir, root, m, slots):
    """The PULL decode family, ABI v1 7.1, on a managed host.

    `design/ABI-v1.md` decision 2: the parameterised emitter is buildable and
    pull REMOVES the upcalls rather than reducing them, but four of the five
    slices have only ever measured push, so every decode figure in the branch is
    a push figure. This is the managed pull arm the decision says settles it.

    `ak_parse_*` makes NO reverse call at all. It appends a record per deposit to
    a buffer in the host-owned decode context, and the host reads that buffer
    afterwards. A drained buffer is a log of the reverse calls push would have
    made, in the order push would have made them, so the replay below is the same
    per-slot code the vtable would have carried -- which is what makes the two
    families comparable rather than two decoders.

    The record header is 24 bytes and the payload that follows it is padded to 8,
    because every `ak_dfix_*` carries an `i64` or an `f64`. The slot id is
    `(outer << 16) | inner` with both halves 1-based and 0 meaning "the group
    itself", so one `u32` names a root slot, an element's group and an inner run
    without a second field.
    """
    o.doc("The pull family's record header. 24 bytes; the payload follows, padded to 8.",
          "    ")
    o += "    [StructLayout(LayoutKind.Sequential)]"
    o += "    private struct Rec { public uint Op, Slot; public long Token; public uint N, Bytes; }"
    o += ""
    o += "    private const uint OP_APPLY = 1, OP_ADD = 2, OP_NEW = 3, OP_APPLY_ELEM = 4;"
    o += ""
    o.doc("Parse, then replay. The parse makes ZERO reverse calls, which is the whole "
          "claim; the replay is managed code walking a byte buffer, so it makes none "
          "either. `ForwardCalls` counts two -- `ak_parse_*` and `ak_bdr_ptr` -- and "
          "`ReverseCalls` stays at zero, and that is the measurement.", "    ")
    o += "    public %s Pull(byte[] src, int len)" % root
    o += "    {"
    o += "        if (_dctx == IntPtr.Zero)"
    o += "        {"
    o += "            _dctx = Abi.ak_dec_ctx_new();"
    o += "            _drun = (DecRun*)NativeMemory.Alloc((nuint)sizeof(DecRun));"
    o += "        }"
    o += "        var t = new %s();" % root
    o += "        fixed (byte* b = src)"
    o += "        {"
    o += "            _fwd++;"
    o += "            int rc = Abi.ak_parse_%s(_dctx, b, (nuint)len);" % root
    o += "            if (rc < 0) throw new InvalidOperationException($\"core parse failed: {rc}\");"
    o += "            byte* recs; nuint rlen;"
    o += "            _fwd++;"
    o += "            int pr = Abi.ak_bdr_ptr(_dctx, &recs, &rlen);"
    o += "            if (pr != 0) throw new InvalidOperationException($\"ak_bdr_ptr failed: {pr}\");"
    o += "            Replay(t, b, recs, (int)rlen);"
    o += "        }"
    o += "        return t;"
    o += "    }"
    o += ""
    o += "    private static void Replay(%s t, byte* b, byte* p, int len)" % root
    o += "    {"
    o += "        int at = 0;"
    o += "        while (at + sizeof(Rec) <= len)"
    o += "        {"
    o += "            ref var r = ref *(Rec*)(p + at);"
    o += "            byte* body = p + at + sizeof(Rec);"
    o += "            at += sizeof(Rec) + (int)r.Bytes;"
    o += "            uint outer = r.Slot >> 16, inner = r.Slot & 0xFFFF;"
    o += "            switch (r.Op)"
    o += "            {"
    o += "                case OP_APPLY:"
    o += "                {"
    o += "                    ref var d = ref *(ak_dfix_%s*)body;" % root
    lines = []
    unfill(ir, m, "d", "t", "                    ", lines)
    for ln in lines:
        o += ln
    o += "                    break;"
    o += "                }"
    for si, s in enumerate(slots, 1):
        if s.leaf:
            o += "                case OP_ADD when r.Slot == %d:" % si
            o += "                {"
            o += "                    var xs = (%s*)body;" % s.cs_delem
            emit_replay_run(o, ir, s, "t." + s.acc, "                    ")
            o += "                    break;"
            o += "                }"
        else:
            o += "                case OP_NEW when outer == %d:" % si
            o += "                    t.%s.Add(new %s());" % (s.acc, s.elem)
            o += "                    break;"
            o += "                case OP_APPLY_ELEM when outer == %d:" % si
            o += "                {"
            o += "                    var e = t.%s[(int)r.Token];" % s.acc
            o += "                    ref var d = ref *(ak_dfix_%s*)body;" % s.elem
            el = []
            keep = tuple(i.path[0] for i in s.inner if len(i.path) > 1)
            unfill(ir, ir.msg(s.elem), "d", "e", "                    ", el, keep=keep)
            for ln in el:
                o += ln
            o += "                    break;"
            o += "                }"
            for ii, i in enumerate(s.inner, 1):
                o += "                case OP_ADD when outer == %d && inner == %d:" % (si, ii)
                o += "                {"
                o += "                    var e = t.%s[(int)r.Token];" % s.acc
                holder, cur = "e", ir.msg(s.elem)
                for pp in i.path[:-1]:
                    fld = next(x for x in cur.walk() if x.name == pp)
                    o += "                    e.%s ??= new %s();" % (fld.cs, fld.of)
                    holder = "%s.%s" % (holder, fld.cs)
                    cur = ir.msg(fld.of)
                o += "                    var xs = (%s*)body;" % i.cs_delem
                emit_replay_run(o, ir, i, "%s.%s" % (holder, i.acc.split(".")[-1]),
                                "                    ")
                o += "                    break;"
                o += "                }"
    o += "                default: break;"
    o += "            }"
    o += "        }"
    o += "    }"
    o += ""


def emit_replay_run(o, ir, s, lst, ind):
    o += "%svar lst = %s;" % (ind, lst)
    if s.kind == "blob":
        o += "%sfor (int i = 0; i < r.N; i++) lst.Add(Str(b, xs[i]));" % ind
    elif s.kind == "packed":
        o += "%sfor (int i = 0; i < r.N; i++) lst.Add(%s);" % (ind, scalar_out(s.f, "xs[i]"))
    elif s.kind == "map":
        o += "%sfor (int i = 0; i < r.N; i++) lst[Str(b, xs[i].key)] = Str(b, xs[i].value);" % ind
    else:
        o += "%sfor (int i = 0; i < r.N; i++)" % ind
        o += "%s{" % ind
        o += "%s    ref var d = ref xs[i];" % ind
        o += "%s    var e = new %s();" % (ind, s.elem)
        lines = []
        unfill(ir, ir.msg(s.elem), "d", "e", ind + "    ", lines)
        for ln in lines:
            o += ln
        o += "%s    lst.Add(e);" % ind
        o += "%s}" % ind
