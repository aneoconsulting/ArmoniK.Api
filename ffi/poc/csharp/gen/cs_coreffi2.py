"""Backend: the `core-ffi` arm's host binding for M2, encode and decode.

M1's element is a LEAF: its encode vtable is empty, the whole run crosses once,
and the crossing count is constant in the element count. **M2's element is not,
and that is the whole point of measuring it.** `TaskDetailed` carries four
repeated string fields and a map, so ABI v1 section 6's batching predicate does
not hold: the codec calls back into the host once per loop slot PER ELEMENT, and
each of those callbacks calls forward again to hand the run over. Five slots, so
about ten crossings per task on encode against three for a whole thousand-element
M1 response. Decode is worse in kind: section 7.2 refuses to batch a non-leaf, so
it is `new_tasks` then `apply_tasks` per element plus a run per inner field.

The published claim this arm exists to test is ".NET's composed arm beats its own
managed codec on decode", which so far is a claim about one flat leaf message.

**What is emitted rather than written.** The group fill is 27 fields deep through
four nested message types, and the presence bits are read from
`gen/abi-layout.json`, which the Rust build printed. Inferring a presence bit from
a field's position among the singular message fields would have been right today
and is still a guess; the same class of guess invented a vtable slot two commits
ago. The bit values come from the codec's own constants or the emitter raises.
"""
import json
import os

from cs_facade import Head

HERE = os.path.dirname(os.path.abspath(__file__))

ROOT = "ListTasksDetailedResponse"


def load_presence():
    with open(os.path.join(HERE, "abi-layout.json")) as f:
        return json.load(f)["presence"]


def loop_slots(ir, msg, prefix=()):
    """Every loop slot of `msg`'s encode vtable, in the codec's own order.

    A slot is a repeated or map field, of the message OR of any singular message
    it embeds -- the group is flat, so a nested message's repeated field is
    hoisted into the element's vtable under an underscore-joined path. That is
    why `TaskOptions.options` appears as `loop_options_options` on
    `ak_evt_TaskDetailed` and not on a vtable of its own.
    """
    out = []
    for f in msg.walk():
        if f.card in ("repeated", "map"):
            out.append(("_".join(prefix + (f.name,)), prefix, f))
        elif f.kind == "message" and f.card == "singular":
            out.extend(loop_slots(ir, ir.msg(f.of), prefix + (f.name,)))
    return out


def pbit(pres, kind, owner, field):
    """The presence bit for `owner.field`, from the Rust build's own constant."""
    name = "AK_%sFIX_%s_PRESENT_%s" % (kind.upper(), owner.upper(), field.upper())
    if name not in pres:
        raise KeyError(
            "%s is not in abi-layout.json's presence map. A singular message field "
            "must have one; add it to abi/src/main.rs and re-run the probe rather "
            "than inferring the bit." % name)
    return name, pres[name]


def fill(ir, pres, msg, gpath, spath, ind, lines):
    """Emit the group fill for one message, recursing into its singular children."""
    for f in msg.walk():
        if f.card in ("repeated", "map"):
            continue                      # a loop slot, not part of the group
        g = "%s.%s" % (gpath, f.name)
        s = "%s.%s" % (spath, f.cs)
        if f.kind == "string":
            lines.append("%s%s = Stage(%s, ref at);" % (ind, g, s))
        elif f.kind == "bytes":
            lines.append("%s%s = StageBytes(%s, ref at);" % (ind, g, s))
        elif f.kind == "enum":
            lines.append("%s%s = (int)%s;" % (ind, g, s))
        elif f.kind == "bool":
            lines.append("%s%s = %s ? (byte)1 : (byte)0;" % (ind, g, s))
        elif f.kind in ("int32", "int64", "double"):
            lines.append("%s%s = %s;" % (ind, g, s))
        elif f.kind == "message":
            name, bit = pbit(pres, "e", msg.name, f.name)
            lines.append("%sif (%s != null)" % (ind, s))
            lines.append("%s{" % ind)
            fill(ir, pres, ir.msg(f.of), g, s, ind + "    ", lines)
            lines.append("%s    %s.presence |= %d;   // %s" % (ind, gpath, bit, name))
            lines.append("%s}" % ind)
        else:
            raise KeyError("no group-fill case for %s.%s (%s)" % (msg.name, f.name, f.kind))


def unfill(ir, pres, msg, dpath, tpath, ind, lines, keep=()):
    """Emit the decode fill: the facade graph from the `ak_dfix_*` group.

    `keep` names the fields whose facade child may ALREADY EXIST when this runs,
    because a run of its own inner elements arrived before `apply_tasks` did. The
    map entries of `options` are the case: the codec flushes them mid-element and
    `apply` comes last, so overwriting the child here would drop the map.
    """
    for f in msg.walk():
        if f.card in ("repeated", "map"):
            continue
        d = "%s.%s" % (dpath, f.name)
        t = "%s.%s" % (tpath, f.cs)
        if f.kind == "string":
            lines.append("%s%s = Str(b, %s);" % (ind, t, d))
        elif f.kind == "bytes":
            lines.append("%s%s = Bytes(b, %s);" % (ind, t, d))
        elif f.kind == "enum":
            lines.append("%s%s = (%s)%s;" % (ind, t, f.of, d))
        elif f.kind == "bool":
            lines.append("%s%s = %s != 0;" % (ind, t, d))
        elif f.kind in ("int32", "int64", "double"):
            lines.append("%s%s = %s;" % (ind, t, d))
        elif f.kind == "message":
            name, bit = pbit(pres, "d", msg.name, f.name)
            lines.append("%sif ((%s.presence & %d) != 0)   // %s" % (ind, dpath, bit, name))
            lines.append("%s{" % ind)
            if f.name in keep:
                lines.append("%s    %s ??= new %s();" % (ind, t, f.of))
            else:
                lines.append("%s    %s = new %s();" % (ind, t, f.of))
            unfill(ir, pres, ir.msg(f.of), d, t, ind + "    ", lines, keep)
            lines.append("%s}" % ind)
        else:
            raise KeyError("no decode case for %s.%s (%s)" % (msg.name, f.name, f.kind))


def emit(ir):
    pres = load_presence()
    with open(os.path.join(HERE, "abi-layout.json")) as f:
        gsize = json.load(f)["structs"]["ak_efix_TaskDetailed"]["size"]
    root = ir.msg(ROOT)
    ef, elem = ir.root_element(ROOT)
    slots = loop_slots(ir, elem)
    blobs = [s for s in slots if s[2].card == "repeated"]
    maps = [s for s in slots if s[2].card == "map"]
    if len(maps) != 1:
        raise KeyError("this emitter assumes exactly one map slot on %s" % elem.name)
    mslot = maps[0]
    mentry = "ak_efix_%s%sEntry" % (mslot[2].owner, "".join(
        p.capitalize() for p in mslot[2].name.split("_")))
    dentry = "ak_dfix_%s%sEntry" % (mslot[2].owner, "".join(
        p.capitalize() for p in mslot[2].name.split("_")))
    nb = len(blobs)

    o = Head("The core-ffi arm for M2: a NON-LEAF element, where the batching predicate stops holding.")
    o += "using System;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += "using System.Text;"
    o += "using Armonik.Ffi.Facade;"
    o += ""
    o += "namespace Armonik.Ffi.Harness;"
    o += ""
    o.doc("What every callback is handed as `obj`. Native rather than managed, for the "
          "same reason M1's is: `[UnmanagedCallersOnly]` cannot capture. The per-slot "
          "runs are laid out as ONE flat array plus an (offset, count) pair per "
          "(element, slot), because the alternative is %d arrays whose bounds move with "
          "the payload." % nb)
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct M2Run"
    o += "{"
    o += "    public ak_efix_%s* Groups;" % elem.name
    o += "    public int Count;"
    o += "    /// Elements per ak_elemu_* call; 0 hands the whole run over at once."
    o += "    public int Chunk;"
    o += "    /// Every repeated-string run of every element, concatenated."
    o += "    public ak_str* Blobs;"
    o += "    /// Indexed [element * %d + slot]." % nb
    o += "    public int* BlobOff;"
    o += "    public int* BlobCnt;"
    o += "    /// Every map run of every element, concatenated."
    o += "    public %s* Ents;" % mentry
    o += "    public int* EntOff;"
    o += "    public int* EntCnt;"
    o += "}"
    o += ""
    o += "public sealed unsafe class CoreFfiM2 : IDisposable"
    o += "{"
    o += "    private IntPtr _ctx;"
    o += "    private IntPtr _dctx;"
    o += "    private byte* _staging;"
    o += "    private int _stagingCap;"
    o += "    private ak_efix_%s* _groups;" % elem.name
    o += "    private int _groupCap;"
    o += "    private ak_str* _blobs;"
    o += "    private int _blobCap;"
    o += "    private %s* _ents;" % mentry
    o += "    private int _entCap;"
    o += "    private int* _blobOff;"
    o += "    private int* _blobCnt;"
    o += "    private int* _entOff;"
    o += "    private int* _entCnt;"
    o += "    private M2Run* _run;"
    o += "    private DecRun2* _drun;"
    o.doc("The ELEMENT's encode vtable, allocated once and pointed at by the root's. "
          "It has to outlive the call, so it cannot be a local.", "    ")
    o += "    private ak_evt_%s* _evt;" % elem.name
    o += "    private static IntPtr _tcBytes;"
    o += "    public int Chunk;"
    o += ""
    o += "    /// The host's own tally. The core's `ak_enc_counters` is the quantity the"
    o += "    /// cross-language table uses; these two agree once the chunk size does."
    o += "    public long ForwardCalls;"
    o += "    public long ReverseCalls;"
    o += ""
    o += "    public CoreFfiM2(int elements, int blobs, int entries, int stagingBytes)"
    o += "    {"
    o += "        _ctx = Abi.ak_enc_ctx_new();"
    o += "        if (_ctx == IntPtr.Zero) throw new InvalidOperationException(\"ak_enc_ctx_new returned null\");"
    o += "        _tcBytes = Abi.ak_tc_bytes();"
    o += "        _groupCap = elements; _blobCap = blobs; _entCap = entries;"
    o += "        _groups = (ak_efix_%s*)NativeMemory.AlignedAlloc((nuint)(sizeof(ak_efix_%s) * elements), 16);" % (elem.name, elem.name)
    o += "        _blobs = (ak_str*)NativeMemory.AlignedAlloc((nuint)(sizeof(ak_str) * blobs), 16);"
    o += "        _ents = (%s*)NativeMemory.AlignedAlloc((nuint)(sizeof(%s) * entries), 16);" % (mentry, mentry)
    o += "        _blobOff = (int*)NativeMemory.Alloc((nuint)(sizeof(int) * elements * %d));" % nb
    o += "        _blobCnt = (int*)NativeMemory.Alloc((nuint)(sizeof(int) * elements * %d));" % nb
    o += "        _entOff = (int*)NativeMemory.Alloc((nuint)(sizeof(int) * elements));"
    o += "        _entCnt = (int*)NativeMemory.Alloc((nuint)(sizeof(int) * elements));"
    o += "        _stagingCap = stagingBytes;"
    o += "        _staging = (byte*)NativeMemory.Alloc((nuint)stagingBytes);"
    o += "        _run = (M2Run*)NativeMemory.Alloc((nuint)sizeof(M2Run));"
    o += "        _evt = (ak_evt_%s*)NativeMemory.Alloc((nuint)sizeof(ak_evt_%s));" % (elem.name, elem.name)
    for i, (path, _, f) in enumerate(slots):
        o += "        _evt->loop_%s = &Loop_%s;" % (path, path)
    o += "    }"
    o += ""
    # ---- staging -----------------------------------------------------
    o.doc("The same staging decision M1 made and for the same reason: transcode up "
          "front and hand the codec UTF-8 with `ak_tc_bytes`, rather than point at the "
          "host's UTF-16 and pay a reverse crossing per string. On M2 that choice is "
          "worth more, not less: P2.2 carries 500 tasks with 3 + 3 + 3 + 3 repeated ids, "
          "4 map entries and 14 scalar strings apiece.", "    ")
    o += "    private ak_str Stage(string s, ref int at)"
    o += "    {"
    o += "        if (s == null || s.Length == 0) return default;"
    o += "        int n = Encoding.UTF8.GetBytes(s.AsSpan(), new Span<byte>(_staging + at, _stagingCap - at));"
    o += "        var r = new ak_str { data = (IntPtr)(_staging + at), len = (nuint)n, tc = _tcBytes };"
    o += "        at += n;"
    o += "        return r;"
    o += "    }"
    o += ""
    o += "    private ak_str StageBytes(byte[] b, ref int at)"
    o += "    {"
    o += "        if (b == null || b.Length == 0) return default;"
    o += "        new ReadOnlySpan<byte>(b).CopyTo(new Span<byte>(_staging + at, _stagingCap - at));"
    o += "        var r = new ak_str { data = (IntPtr)(_staging + at), len = (nuint)b.Length, tc = _tcBytes };"
    o += "        at += b.Length;"
    o += "        return r;"
    o += "    }"
    o += ""
    # ---- the loop callbacks -----------------------------------------
    o.doc("The root's loop over `%s`. One reverse call for the whole response, as on "
          "M1 -- everything that follows is what the NON-LEAF element costs on top." % ef.name, "    ")
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static int LoopTasks(IntPtr ctx, void* obj, long token)"
    o += "    {"
    o += "        try"
    o += "        {"
    o += "            var run = (M2Run*)obj;"
    o += "            int chunk = run->Chunk <= 0 ? run->Count : run->Chunk;"
    o += "            for (int off = 0; off < run->Count; off += chunk)"
    o += "            {"
    o += "                int n = run->Count - off; if (n > chunk) n = chunk;"
    o += "                // tok0 is the run's FIRST index, so the token the element's own"
    o += "                // loop callbacks are handed is the global element index whatever"
    o += "                // the chunk size. ak_elemu_* adds i to it."
    o += "                int rc = Abi.ak_elemu_%s(ctx, run->Groups + off, n, off);" % elem.name
    o += "                if (rc < 0) return rc;"
    o += "            }"
    o += "            return 0;"
    o += "        }"
    o += "        catch { return Abi.AK_ERR_HOST; }"
    o += "    }"
    o += ""
    o.doc("One of these per loop slot, per element. This is the cost ABI v1's batching "
          "predicate removes on a leaf and cannot remove here. A run of length zero is "
          "NOT handed over: an empty repeated field writes nothing, so the forward call "
          "would buy the codec nothing at all.", "    ")
    for i, (path, prefix, f) in enumerate(slots):
        o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
        o += "    private static int Loop_%s(IntPtr ctx, void* obj, long token)" % path
        o += "    {"
        o += "        try"
        o += "        {"
        o += "            var run = (M2Run*)obj;"
        o += "            int i = (int)token;"
        if f.card == "repeated":
            j = blobs.index((path, prefix, f))
            o += "            int n = run->BlobCnt[i * %d + %d];" % (nb, j)
            o += "            if (n == 0) return 0;"
            o += "            return Abi.ak_blob_run(ctx, run->Blobs + run->BlobOff[i * %d + %d], n);" % (nb, j)
        else:
            o += "            int n = run->EntCnt[i];"
            o += "            if (n == 0) return 0;"
            o += "            return Abi.ak_elem_%s%sEntry(ctx, run->Ents + run->EntOff[i], n);" % (
                f.owner, "".join(p.capitalize() for p in f.name.split("_")))
        o += "        }"
        o += "        catch { return Abi.AK_ERR_HOST; }"
        o += "    }"
        o += ""
    # ---- encode ------------------------------------------------------
    o.doc("Fill the group array, then one forward call. The group is **%d bytes per "
          "element**, three times M1's, and it is memset and refilled every encode: "
          "decision 9's sparse fill is the amendment that would stop that, and this arm "
          "is where its cost shows up." % gsize, "    ")
    o.doc("The host-side half, on its own. **This exists so that \"the rest is the "
          "group fill\" is a measurement and not a subtraction argument**: it does "
          "everything `Encode` does -- zero the group, stage every string, build the "
          "run arrays -- and then stops without calling the codec, so the difference "
          "between the two arms is the codec plus every crossing. Nothing in the timed "
          "path branches on which of the two is running.", "    ")
    o += "    public int Fill(%s src) => FillOrEncode(src, false, out _, out _);" % ROOT
    o += ""
    o += "    public int Encode(%s src, out byte* outPtr, out int outLen)" % ROOT
    o += "        => FillOrEncode(src, true, out outPtr, out outLen);"
    o += ""
    o += "    private int FillOrEncode(%s src, bool call, out byte* outPtr, out int outLen)" % ROOT
    o += "    {"
    o += "        int n = src.%s.Count;" % ef.cs
    o += "        if (n > _groupCap) throw new InvalidOperationException(\"group array too small\");"
    o += "        Abi.ak_enc_reset(_ctx);"
    o += "        int at = 0, bi = 0, ei = 0;"
    o += "        for (int i = 0; i < n; i++)"
    o += "        {"
    o += "            var t = src.%s[i];" % ef.cs
    o += "            ref var g = ref _groups[i];"
    o += "            g = default;"
    lines = []
    fill(ir, pres, elem, "g", "t", "            ", lines)
    for ln in lines:
        o += ln
    for j, (path, prefix, f) in enumerate(blobs):
        # the facade path: prefix components are singular messages
        parts = []
        cur = elem
        for p in prefix:
            fp = next(x for x in cur.walk() if x.name == p)
            parts.append(fp.cs)
            cur = ir.msg(fp.of)
        fld = next(x for x in cur.walk() if x.name == f.name)
        parts.append(fld.cs)
        acc = "t." + ".".join(parts)
        o += "            {"
        o += "                var lst = %s;" % acc
        o += "                _blobOff[i * %d + %d] = bi;" % (nb, j)
        o += "                _blobCnt[i * %d + %d] = lst == null ? 0 : lst.Count;" % (nb, j)
        o += "                if (lst != null) for (int k = 0; k < lst.Count; k++) _blobs[bi++] = Stage(lst[k], ref at);"
        o += "            }"
    # the map slot
    parts = []
    cur = elem
    for p in mslot[1]:
        fp = next(x for x in cur.walk() if x.name == p)
        parts.append(fp.cs)
        cur = ir.msg(fp.of)
    mfld = next(x for x in cur.walk() if x.name == mslot[2].name)
    parts.append(mfld.cs)
    macc = "t." + ".".join(parts)
    mguard = "t." + ".".join(parts[:-1]) if len(parts) > 1 else None
    o += "            {"
    if mguard:
        o += "                var mp = %s == null ? null : %s;" % (mguard, macc)
    else:
        o += "                var mp = %s;" % macc
    o += "                _entOff[i] = ei;"
    o += "                _entCnt[i] = mp == null ? 0 : mp.Count;"
    o += "                if (mp != null)"
    o += "                {"
    o += "                    // OrderedMap preserves insertion order and the builder"
    o += "                    // inserts sorted, so this is the canonical order already."
    o += "                    // Indexed, not foreach: OrderedMap.GetEnumerator returns the"
    o += "                    // interface, so a foreach boxes the List enumerator once per"
    o += "                    // ELEMENT. Measured at 24,000 B/op on P2.2 before this line."
    o += "                    for (int k = 0; k < mp.Count; k++)"
    o += "                    {"
    o += "                        var kv = mp.At(k);"
    o += "                        ref var e = ref _ents[ei++];"
    o += "                        e = default;"
    o += "                        e.key = Stage(kv.Key, ref at);"
    o += "                        e.value = Stage(kv.Value, ref at);"
    o += "                    }"
    o += "                }"
    o += "            }"
    o += "        }"
    o += "        if (bi > _blobCap || ei > _entCap) throw new InvalidOperationException(\"run arrays too small\");"
    o += "        _run->Groups = _groups; _run->Count = n; _run->Chunk = Chunk;"
    o += "        _run->Blobs = _blobs; _run->BlobOff = _blobOff; _run->BlobCnt = _blobCnt;"
    o += "        _run->Ents = _ents; _run->EntOff = _entOff; _run->EntCnt = _entCnt;"
    o += "        var vt = new ak_evt_%s { loop_%s = &LoopTasks, elem_%s = _evt };" % (ROOT, ef.name, ef.name)
    o += "        var fix = new ak_efix_%s { page = src.Page, total = src.Total };" % ROOT
    o += "        // Before the tally, so the fill-only arm does not count crossings it"
    o += "        // did not make."
    o += "        if (!call) { outPtr = null; outLen = at; return at; }"
    o += "        ForwardCalls++;   // ak_encode_*"
    o += "        ReverseCalls++;   // loop_%s" % ef.name
    o += "        ForwardCalls += Chunk <= 0 ? 1 : (n + Chunk - 1) / Chunk;   // ak_elemu_* per chunk"
    o += "        for (int i = 0; i < n; i++)"
    o += "        {"
    o += "            // Every slot is populated, so the codec calls back once per slot per"
    o += "            // element; the host calls forward again for each run that is not empty."
    for j, (path, prefix, f) in enumerate(blobs):
        o += "            ReverseCalls++; if (_blobCnt[i * %d + %d] != 0) ForwardCalls++;" % (nb, j)
    o += "            ReverseCalls++; if (_entCnt[i] != 0) ForwardCalls++;"
    o += "        }"
    o += "        nint rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix);" % ROOT
    o += "        if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\");"
    o += "        byte* p; nuint len;"
    o += "        int tk = Abi.ak_enc_take(_ctx, &p, &len);"
    o += "        if (tk != 0) throw new InvalidOperationException($\"ak_enc_take failed: {tk}\");"
    o += "        outPtr = p; outLen = (int)len;"
    o += "        return outLen;"
    o += "    }"
    o += ""
    o += "    public byte[] EncodeToArray(%s src)" % ROOT
    o += "    {"
    o += "        Encode(src, out byte* p, out int len);"
    o += "        var a = new byte[len];"
    o += "        new ReadOnlySpan<byte>(p, len).CopyTo(a);"
    o += "        return a;"
    o += "    }"
    o += ""
    # ---- decode ------------------------------------------------------
    o.doc("The decode callbacks' `obj`. `ak_span` is an OFFSET, so the base the codec "
          "never sees has to travel with the handle.", "    ")
    o += "    [StructLayout(LayoutKind.Sequential)]"
    o += "    private struct DecRun2"
    o += "    {"
    o += "        public IntPtr Target;"
    o += "        public byte* Buf;"
    o += "    }"
    o += ""
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += "    private static %s Tgt(void* obj) => (%s)GCHandle.FromIntPtr(((DecRun2*)obj)->Target).Target;" % (ROOT, ROOT)
    o += ""
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += '    private static string Str(byte* b, ak_span s)'
    o += '        => s.len == 0 ? "" : Encoding.UTF8.GetString(b + s.off, (int)s.len);'
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
    o += "    private static void ApplyRoot(IntPtr ctx, void* obj, ak_dfix_%s* fix)" % ROOT
    o += "    {"
    o += "        try { var t = Tgt(obj); t.Page = fix->page; t.Total = fix->total; }"
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
    o += "    }"
    o += ""
    o.doc("ABI v1 section 7.2's refusal, in the host. A leaf element arrives as a RUN "
          "and costs one reverse call for the whole response; this one costs `new` then "
          "`apply` per element because there would be nothing to attach its own inner "
          "elements to. The token is an index into the list `new` appended to.", "    ")
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static long NewTask(IntPtr ctx, void* obj)"
    o += "    {"
    o += "        try { var t = Tgt(obj); t.%s.Add(new %s()); return t.%s.Count - 1; }" % (ef.cs, elem.name, ef.cs)
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); return -1; }"
    o += "    }"
    o += ""
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static void ApplyTask(IntPtr ctx, void* obj, long token, ak_dfix_%s* fix)" % elem.name
    o += "    {"
    o += "        try"
    o += "        {"
    o += "            var run = (DecRun2*)obj;"
    o += "            var t = (%s)GCHandle.FromIntPtr(run->Target).Target;" % ROOT
    o += "            byte* b = run->Buf;"
    o += "            var m = t.%s[(int)token];" % ef.cs
    o += "            ref var d = ref *fix;"
    lines = []
    keep = tuple(mslot[1])
    unfill(ir, pres, elem, "d", "m", "            ", lines, keep=keep)
    for ln in lines:
        o += ln
    o += "        }"
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
    o += "    }"
    o += ""
    for path, prefix, f in slots:
        if f.card != "repeated":
            continue
        parts = []
        cur = elem
        for p in prefix:
            fp = next(x for x in cur.walk() if x.name == p)
            parts.append(fp.cs)
            cur = ir.msg(fp.of)
        fld = next(x for x in cur.walk() if x.name == f.name)
        parts.append(fld.cs)
        o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
        o += "    private static void Add_%s(IntPtr ctx, void* obj, long token, ak_span* xs, int n)" % path
        o += "    {"
        o += "        try"
        o += "        {"
        o += "            var run = (DecRun2*)obj;"
        o += "            var t = (%s)GCHandle.FromIntPtr(run->Target).Target;" % ROOT
        o += "            byte* b = run->Buf;"
        o += "            var lst = t.%s[(int)token].%s;" % (ef.cs, ".".join(parts))
        o += "            for (int i = 0; i < n; i++) lst.Add(Str(b, xs[i]));"
        o += "        }"
        o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
        o += "    }"
        o += ""
    o.doc("The map run. It arrives BEFORE `apply_tasks`, so the parent has to be made "
          "here and `apply_tasks` must not replace it. That ordering is not obvious from "
          "the header and is the kind of thing a byte-identity gate does not catch: the "
          "re-encode would still match if the map were dropped on both sides.", "    ")
    o += "    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"
    o += "    private static void Add_%s(IntPtr ctx, void* obj, long token, %s* xs, int n)" % (mslot[0], dentry)
    o += "    {"
    o += "        try"
    o += "        {"
    o += "            var run = (DecRun2*)obj;"
    o += "            var t = (%s)GCHandle.FromIntPtr(run->Target).Target;" % ROOT
    o += "            byte* b = run->Buf;"
    o += "            var m = t.%s[(int)token];" % ef.cs
    holder = "m"
    cur = elem
    for p in mslot[1]:
        fp = next(x for x in cur.walk() if x.name == p)
        o += "            %s.%s ??= new %s();" % (holder, fp.cs, fp.of)
        holder = "%s.%s" % (holder, fp.cs)
        cur = ir.msg(fp.of)
    o += "            var mp = %s.%s;" % (holder, mfld.cs)
    o += "            for (int i = 0; i < n; i++) mp[Str(b, xs[i].key)] = Str(b, xs[i].value);"
    o += "        }"
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
    o += "    }"
    o += ""
    o += "    public %s Decode(byte[] src, int len)" % ROOT
    o += "    {"
    o += "        if (_dctx == IntPtr.Zero)"
    o += "        {"
    o += "            _dctx = Abi.ak_dec_ctx_new();"
    o += "            _drun = (DecRun2*)NativeMemory.Alloc((nuint)sizeof(DecRun2));"
    o += "        }"
    o += "        Abi.ak_dec_err_reset(_dctx);"
    o += "        var target = new %s();" % ROOT
    o += "        var h = GCHandle.Alloc(target);"
    o += "        try"
    o += "        {"
    o += "            fixed (byte* b = src)"
    o += "            {"
    o += "                _drun->Target = GCHandle.ToIntPtr(h);"
    o += "                _drun->Buf = b;"
    o += "                var vt = new ak_dvt_%s" % ROOT
    o += "                {"
    o += "                    apply = &ApplyRoot,"
    o += "                    unknown = IntPtr.Zero,"
    o += "                    unk_%s = IntPtr.Zero," % ef.name
    o += "                    new_%s = &NewTask," % ef.name
    o += "                    apply_%s = &ApplyTask," % ef.name
    for path, prefix, f in slots:
        o += "                    add_%s_%s = &Add_%s," % (ef.name, path, path)
    o += "                };"
    o += "                ForwardCalls++;   // ak_decode_*"
    o += "                ReverseCalls++;   // apply"
    o += "                int rc = Abi.ak_decode_%s(_dctx, _drun, b, (nuint)len, &vt);" % ROOT
    o += "                if (rc < 0) throw new InvalidOperationException($\"core decode failed: {rc}\");"
    o += "                int he = Abi.ak_dec_err(_dctx);"
    o += "                if (he != 0) throw new InvalidOperationException($\"host reported {he} through ak_fail\");"
    o += "            }"
    o += "        }"
    o += "        finally { h.Free(); }"
    o += "        // The per-element reverse calls, tallied from the graph that came back"
    o += "        // rather than predicted: `new` and `apply` per element, plus one run per"
    o += "        // inner field that actually occurred."
    o += "        for (int i = 0; i < target.%s.Count; i++)" % ef.cs
    o += "        {"
    o += "            ReverseCalls += 2;"
    o += "            var m = target.%s[i];" % ef.cs
    for path, prefix, f in slots:
        parts = []
        cur = elem
        for p in prefix:
            fp = next(x for x in cur.walk() if x.name == p)
            parts.append(fp.cs)
            cur = ir.msg(fp.of)
        fld = next(x for x in cur.walk() if x.name == f.name)
        parts.append(fld.cs)
        acc = "m." + ".".join(parts)
        guard = "m." + ".".join(parts[:-1]) if len(parts) > 1 else None
        if guard:
            o += "            if (%s != null && %s.Count != 0) ReverseCalls++;" % (guard, acc)
        else:
            o += "            if (%s.Count != 0) ReverseCalls++;" % acc
    o += "        }"
    o += "        return target;"
    o += "    }"
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
    o += "        if (_groups != null) { NativeMemory.AlignedFree(_groups); _groups = null; }"
    o += "        if (_blobs != null) { NativeMemory.AlignedFree(_blobs); _blobs = null; }"
    o += "        if (_ents != null) { NativeMemory.AlignedFree(_ents); _ents = null; }"
    o += "        if (_blobOff != null) { NativeMemory.Free(_blobOff); _blobOff = null; }"
    o += "        if (_blobCnt != null) { NativeMemory.Free(_blobCnt); _blobCnt = null; }"
    o += "        if (_entOff != null) { NativeMemory.Free(_entOff); _entOff = null; }"
    o += "        if (_entCnt != null) { NativeMemory.Free(_entCnt); _entCnt = null; }"
    o += "        if (_staging != null) { NativeMemory.Free(_staging); _staging = null; }"
    o += "        if (_run != null) { NativeMemory.Free(_run); _run = null; }"
    o += "        if (_evt != null) { NativeMemory.Free(_evt); _evt = null; }"
    o += "        if (_drun != null) { NativeMemory.Free(_drun); _drun = null; }"
    o += "    }"
    o += "}"
    return str(o)
