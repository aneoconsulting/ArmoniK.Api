"""C# backend: the HOST half of the core-ffi binding of a message set, rendered from PLANS.

FIX-PLAN WP5 step 4. Everything the host hands the core or takes back is laid out by
`plan.py`: which members a group has (`group_fields`, `ugroup_fields`), which bit says a
member is present (`presence_bits`), which fields are loop slots, including those on an
inlined child (`loop_slots`), what one element of a run is (`slot_elem`), which element
types batch (`MessagePlan.leaf`, ABI v1 section 7.2), which field is a direct argument
(`direct_fields`, section 8). This module decides only C#: how a group is filled from a
facade object and read back into one, how a run is staged, how a callback is guarded.

Per root `R`, one class `CoreFfi_R`:
  Encode / EncodeU     `ak_encode_R` (unknown fields dropped) / `ak_uencode_R` (the
                       facade's `UnknownFields` bag handed over in `ak_ufix_*` groups);
  Decode / DecodeU     push family, `ak_decode_R`, unknown fields dropped / retained
                       (ABI v1 decision 11: the root-bound context is armed with
                       `ak_dec_R_opts` for the one decode and disarmed after it);
  Pull                 ABI v1 7.1: `ak_parse_R`, then the record stream replayed with the
                       same group readers the push callbacks use; no reverse call but grow.
  TryEncode/TryDecode  the same, returning the core's code rather than throwing (the corpus).

Host rules stated where they are rendered: a string is staged (UTF-8 through `ak_tc_bytes`,
or the host's UTF-16 through `ak_tc_utf16`), an EMPTY implicit string is absent (`tc` null),
a present-and-empty explicit one carries `tc` so the core writes it; every reverse callback
is `[UnmanagedCallersOnly]` and guarded (a managed exception would abort the process); a
decode callback reports through `ak_fail`. The file is compiled only where
`UnmanagedCallersOnly` exists (`NET5_0_OR_GREATER`): net8.0 and net6.0, not net48 (a net48
host would need delegate thunks rooted for the vtable's lifetime; not built).

Retention through the C ABI (decision 11, WP5 steps 7-9): every message position's
buffer arrives as DATA in its decode group (`unknown: ak_unk_buf`), host memory the core
filled through the one grow callback (`UnkHost.Grow`, NativeMemory.Realloc; NULL/0 = a fresh
buffer). The group reader that delivers a message takes its slot into the facade's
`UnknownFields` and frees the native buffer; every other non-NULL slot of a delivered group
(an absent child's, an inactive oneof member's, a map entry's: the facade map has no bag, so
U-map-entry is the one position that drops) is freed. A retained decode tracks every buffer
grow handed out and takes back; one left over after a successful decode is a host defect,
reported as UNDELIVERED, and every one left after a failed decode is freed (rule 3).

The NO-UNKNOWN variant (WP5 step 10: `plan.unknown_compiled_out`, a plan relowered with
unknown="drop") is rendered from the same functions with every retain path left out: no
U_ fills and no `ak_uencode_*`/`ak_uelem*_*` calls, no options, no resets, no grow, no bag
taken or freed, `ak_dec_ctx_new_<Root>()`; `retain` = true is refused (NotSupported).
"""
from plan import (FIXED, as_plan, direct_fields, elem_type, loop_slots, presence_bits, slot_elem,
                  slot_name, unknown_compiled_out)
import cs_names as N
from cs_types import BAG

# WP5 step 10: True while rendering the NO-UNKNOWN variant (a plan relowered with
# unknown="drop"): no u-groups, no ak_uencode/ak_uelem*, no options, no resets, no bag
# capture, `ak_dec_ctx_new_<Root>()`. Set by emit_host from plan.unknown_compiled_out.
_NO = False

# The packed-run entry points, one per host array layout, read from plan.FIXED.run_types
# (R-H15): no backend-local table.
RUN_FN = {t: "ak_run_%s" % t for t in FIXED.run_types}
UCO = "[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]"


def _enc(f, e):
    """facade scalar -> ABI scalar"""
    if f.kind == "bool":
        return "(byte)(%s ? 1 : 0)" % e
    if f.kind == "enum":
        return "(int)%s" % e
    return e


def _dec(f, e):
    """ABI scalar -> facade scalar"""
    if f.kind == "bool":
        return "(%s != 0)" % e
    if f.kind == "enum":
        return "(%s)%s" % (f.of, e)
    return e


def _get(base, p, owner, path):
    """Read-only facade access to the field at `path` from `base` (an `owner`): null when an
    inlined parent on the way is absent."""
    expr, cur = base, owner
    for i, name in enumerate(path):
        f = next(x for x in p.msg(cur).plain if x.name == name)
        expr += (".%s" if i == 0 else "?.%s") % N.field(name)
        if f.kind == "message" and i < len(path) - 1:
            cur = f.of
    return expr


def _make(base, p, owner, path):
    """Facade access that CREATES each inlined parent on the way (decode)."""
    expr, cur = base, owner
    for name in path[:-1]:
        f = next(x for x in p.msg(cur).plain if x.name == name)
        expr = "(%s.%s ??= new %s())" % (expr, N.field(name), f.of)
        cur = f.of
    return "%s.%s" % (expr, N.field(path[-1]))


class Slot:
    def __init__(self, p, owner, path, f, top):
        self.p, self.owner, self.path, self.f, self.top = p, owner, path, f, top
        self.name = slot_name(path)
        self.et = elem_type(f)
        dty, ety = slot_elem(f)
        self.cs_e, self.cs_d = N.abi_type(ety), N.abi_type(dty)
        if f.card == "map":
            self.kind = "map"
        elif f.is_blob:
            self.kind = "blob"
        elif f.card == "packed":
            self.kind = "packed"
            if ety not in RUN_FN:
                raise NotImplementedError("packed %s (%s.%s): the C ABI has no run symbol for this "
                                          "host layout (raise, never skip)" % (f.kind, owner, f.name))
        elif f.kind == "message":
            self.kind = "msg"
        else:
            raise NotImplementedError("slot %s.%s" % (owner, f.name))
        self.leaf = True if self.et is None else p.msg(self.et).leaf
        self.inner = ([Slot(p, self.et, ip, iff, False) for ip, iff in loop_slots(p, self.et)]
                      if (self.kind == "msg" and not self.leaf) else [])
        for i in self.inner:
            if i.kind == "msg" and not i.leaf:
                raise NotImplementedError("a non-leaf element inside a non-leaf element (%s.%s)"
                                          % (self.et, i.name))
        self.has_evt = bool(self.et and loop_slots(p, self.et))

    def u_elem(self):
        return "ak_ufix_%s" % self.et


# ======================================================================== groups

def _emit_groups(o, p):
    """E_/U_/D_ per message: the group filled from a facade object, and read back."""
    o += "public static unsafe class G"
    o += "{"
    o += "    /// A CEILING for ABI v1 decision 13, not an implementation: set, a decode"
    o += "    /// materialises no string at all."
    o += "    public static bool SkipStrings;"
    o += "    /// A CHECK CONTROL, never set in a timed run: AK_GATE_PLANT_HOST_FAIL=apply (1) makes every"
    o += "    /// root apply callback throw, =add (2) every add/new callback, so the host reports"
    o += "    /// AK_ERR_HOST through ak_fail from inside a reverse call (harness hostfail, step a2 (i))."
    o += "    internal static readonly int PlantHostFail = Environment.GetEnvironmentVariable(\"AK_GATE_PLANT_HOST_FAIL\") switch { \"apply\" => 1, \"add\" => 2, _ => 0 };"
    o += ""
    o += "    /// D20 (owner, 2026-10-04, FIX-PLAN D20): every push vtable and every pull context sets all its"
    o += "    /// utf8_skip bits, so the core hands strings over unchecked and the host validates here with"
    o += "    /// a STRICT decoder: malformed UTF-8 throws DecoderFallbackException, which the reverse"
    o += "    /// callbacks report as AK_ERR_TRANSCODE through ak_fail (and the pull replay returns), the"
    o += "    /// code the core returns for it when it validates (proto3's reject rule kept)."
    o += "    private static readonly UTF8Encoding Strict = new UTF8Encoding(false, true);"
    o += "    /// A CHECK CONTROL, never set in a timed run: AK_GATE_PLANT_LOSSY=1 decodes with the lossy"
    o += "    /// Encoding.UTF8 (U+FFFD), so a malformed string is accepted and the corpus must fail."
    o += "    internal static readonly bool PlantLossy = Environment.GetEnvironmentVariable(\"AK_GATE_PLANT_LOSSY\") == \"1\";"
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += '    internal static string Str(byte* b, ak_span s) => s.len == 0 || SkipStrings ? "" : (PlantLossy ? Encoding.UTF8 : Strict).GetString(b + s.off, (int)s.len);'
    o += ""
    if not _NO:
        _emit_unk_helpers(o)
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += "    internal static byte[] Bytes(byte* b, ak_span s)"
    o += "    {"
    o += "        if (s.len == 0) return Array.Empty<byte>();"
    o += "        var a = new byte[s.len];"
    o += "        new ReadOnlySpan<byte>(b + s.off, (int)s.len).CopyTo(a);"
    o += "        return a;"
    o += "    }"
    o += ""
    for name in p.abi_order:
        m = p.msg(name)
        if m.synthetic:
            continue
        for u in ((False,) if _NO else (False, True)):
            _emit_fill(o, p, m, u)
        _emit_unfill(o, p, m)
        if not _NO:
            _emit_free(o, p, m)
    o += "}"
    o += ""


def _emit_unk_helpers(o):
    o += "    /// Decision 11: the arena of the RETAINED decode running on this thread (null in drop"
    o += "    /// mode): every buffer grow hands out comes from it, and it counts the buffers handed"
    o += "    /// out and not yet taken back (step a2 iv, 2026-10-04: replaced a HashSet of malloc'd"
    o += "    /// buffers). Thread-static: the core calls grow on the decoding thread."
    o += "    [ThreadStatic] internal static UnkArena Arena;"
    o += ""
    o += "    /// A delivered message's buffer into its facade bag (null when none or empty); the"
    o += "    /// buffer is given back to the arena's count (its memory is the arena's) and the slot cleared."
    o += "    /// A GATE CONTROL, not a feature (R-H9): set, Take copies the bag but skips the"
    o += "    /// release (the buffer is neither freed nor untracked), so the UNDELIVERED check of a"
    o += "    /// retained decode must fail (Disarm finds the buffer outstanding, frees it, reports it)."
    o += "    internal static readonly bool PlantSkipRelease = Environment.GetEnvironmentVariable(\"AK_GATE_PLANT_SKIP_RELEASE\") == \"1\";"
    o += ""
    o += "    internal static byte[] Take(ref ak_unk_buf u)"
    o += "    {"
    o += "        if (u.data == IntPtr.Zero) return null;"
    o += "        byte[] r = null;"
    o += "        if (u.len != 0) { r = new byte[u.len]; new ReadOnlySpan<byte>((void*)u.data, (int)u.len).CopyTo(r); }"
    o += "        if (PlantSkipRelease) { u = default; return r; }"
    o += "        var a = Arena; if (a != null) a.Outstanding--;"
    o += "        u = default;"
    o += "        return r;"
    o += "    }"
    o += ""
    o += "    /// A non-NULL slot the facade has no place for (inactive, absent, a map entry): given back."
    o += "    internal static void Drop(ref ak_unk_buf u)"
    o += "    {"
    o += "        if (u.data == IntPtr.Zero) return;"
    o += "        var a = Arena; if (a != null) a.Outstanding--;"
    o += "        u = default;"
    o += "    }"
    o += ""


def _emit_fill(o, p, m, u):
    g = "ak_ufix_%s" % m.name if u else "ak_efix_%s" % m.name
    o += "    internal static void %s_%s(ref %s g, %s s, Stage st)" % ("U" if u else "E", m.name, g, m.name)
    o += "    {"
    o += "        g = default;   // the fill is total: every member not assigned below is zero"
    bits = presence_bits(m)
    for f in m.plain:
        if f.card != "singular":
            continue
        acc = "s.%s" % N.field(f.name)
        mem = "g.%s" % f.name
        pres = "g.presence |= AkPresent.%s_%s;" % (m.name, f.name) if f.name in bits else None
        if f.direct:
            o += "        // ABI v1 section 8: the sentinel says the bytes are an argument of the call."
            o += "        %s = new ak_str { data = Abi.AK_STR_DIRECT, len = (nuint)(%s == null ? 0 : %s.Length), tc = IntPtr.Zero };" % (mem, acc, acc)
        elif f.kind == "message":
            o += "        if (%s != null) { %s_%s(ref %s, %s, st); %s }" % (acc, "U" if u else "E", f.of, mem, acc, pres)
        elif f.explicit:
            if f.kind == "string":
                o += "        if (%s != null) { %s = st.StrPresent(%s); %s }" % (acc, mem, acc, pres)
            elif f.kind == "bytes":
                o += "        if (%s != null) { %s = st.BytesPresent(%s); %s }" % (acc, mem, acc, pres)
            else:
                o += "        if (%s.HasValue) { %s = %s; %s }" % (acc, mem, _enc(f, acc + ".Value"), pres)
        elif f.kind == "string":
            o += "        %s = st.Str(%s);" % (mem, acc)
        elif f.kind == "bytes":
            o += "        %s = st.Bytes(%s);" % (mem, acc)
        else:
            o += "        %s = %s;" % (mem, _enc(f, acc))
    for oname, members in m.oneofs.items():
        cf = "s.%s" % N.oneof_case_field(oname)
        ct = N.oneof_case_type(m.name, oname)
        o += "        // The discriminant carries the ACTIVE MEMBER'S TAG; the facade's case is tag-valued."
        o += "        g.%s_case = (uint)%s;" % (oname, cf)
        o += "        switch (%s)" % cf
        o += "        {"
        for gm in members:
            mem = "g.%s_%s" % (oname, gm.name)
            acc = "s.%s" % N.field(gm.name)
            o += "            case %s.%s:" % (ct, N.pascal(gm.name))
            if gm.kind == "string":
                o += "                %s = st.StrPresent(%s ?? \"\"); break;" % (mem, acc)
            elif gm.kind == "bytes":
                o += "                %s = st.BytesPresent(%s ?? Array.Empty<byte>()); break;" % (mem, acc)
            elif gm.kind == "message":
                o += "                if (%s != null) %s_%s(ref %s, %s, st); break;" % (acc, "U" if u else "E", gm.of, mem, acc)
            else:
                o += "                %s = %s; break;" % (mem, _enc(gm, acc))
        o += "            default: break;"
        o += "        }"
    if u:
        o += "        g.unknown = st.Blob(s.%s);" % BAG
    o += "    }"
    o += ""


def _emit_unfill(o, p, m):
    o += "    internal static void D_%s(ref ak_dfix_%s d, %s t, byte* b)" % (m.name, m.name, m.name)
    o += "    {"
    bits = presence_bits(m)
    for f in m.plain:
        if f.card != "singular":
            continue
        acc = "t.%s" % N.field(f.name)
        mem = "d.%s" % f.name
        if f.name in bits:
            present = "(d.presence & AkPresent.%s_%s) != 0" % (m.name, f.name)
        if f.kind == "message":
            # In place: a run for a slot on this child may already have created it. An
            # absent child's slots are freed (every non-NULL slot is the host's, rule 3).
            if _NO:
                o += "        if (%s) D_%s(ref %s, %s ??= new %s(), b);" % (present, f.of, mem, acc, f.of)
            else:
                o += "        if (%s) D_%s(ref %s, %s ??= new %s(), b); else F_%s(ref %s);" % (present, f.of, mem, acc, f.of, f.of, mem)
        elif f.explicit:
            val = ("Str(b, %s)" % mem if f.kind == "string" else "Bytes(b, %s)" % mem
                   if f.kind == "bytes" else _dec(f, mem))
            o += "        %s = %s ? %s : null;" % (acc, present, val)
        elif f.kind == "string":
            o += "        %s = Str(b, %s);" % (acc, mem)
        elif f.kind == "bytes":
            o += "        %s = Bytes(b, %s);" % (acc, mem)
        else:
            o += "        %s = %s;" % (acc, _dec(f, mem))
    for oname, members in m.oneofs.items():
        ct = N.oneof_case_type(m.name, oname)
        # Decision 11 rule 4: the oneof's one buffer is in the ACTIVE message member's slot;
        # after a switch to a scalar member it may stay, emptied, in the last message
        # member's. Every slot but the active member's is freed.
        for gm in members:
            if gm.kind == "message" and not _NO:
                o += "        if (d.%s_case != %d) F_%s(ref d.%s_%s);" % (oname, gm.tag, gm.of, oname, gm.name)
        o += "        t.%s = (%s)d.%s_case;" % (N.oneof_case_field(oname), ct, oname)
        o += "        switch (d.%s_case)" % oname
        o += "        {"
        for gm in members:
            mem = "d.%s_%s" % (oname, gm.name)
            acc = "t.%s" % N.field(gm.name)
            o += "            case %d:" % gm.tag
            if gm.kind == "string":
                o += "                %s = Str(b, %s); break;" % (acc, mem)
            elif gm.kind == "bytes":
                o += "                %s = Bytes(b, %s); break;" % (acc, mem)
            elif gm.kind == "message":
                o += "                %s = new %s(); D_%s(ref %s, %s, b); break;" % (acc, gm.of, gm.of, mem, acc)
            else:
                o += "                %s = %s; break;" % (acc, _dec(gm, mem))
        o += "            default: break;"
        o += "        }"
    if not _NO:
        o += "        t.%s = Take(ref d.unknown);   // decision 11: this message's own buffer" % BAG
    o += "    }"
    o += ""


def _emit_free(o, p, m):
    """F_M: free every non-NULL unknown-field slot of a group the facade does not take."""
    o += "    internal static void F_%s(ref ak_dfix_%s d)" % (m.name, m.name)
    o += "    {"
    o += "        Drop(ref d.unknown);"
    for f in m.plain:
        if f.card == "singular" and f.kind == "message":
            o += "        F_%s(ref d.%s);" % (f.of, f.name)
    for oname, members in m.oneofs.items():
        for gm in members:
            if gm.kind == "message":
                o += "        F_%s(ref d.%s_%s);" % (gm.of, oname, gm.name)
    o += "    }"
    o += ""


UNKHOST = r'''
/// Decision 11: the one grow callback every position of every root's options names
/// (`ak_grow_fn`, i32 sizes). `*dst` NULL with `*cap` 0 is a fresh buffer, otherwise the
/// first `*cap` bytes are preserved (realloc semantics; it may move). Step a2 (iv,
/// 2026-10-04): the memory is the running decode's ARENA (`G.Arena`, geometric chunks kept
/// across decodes; a grow of the arena's last allocation extends it in place), not
/// malloc/realloc/free per buffer; the arena counts what it hands out (UNDELIVERED check).
public static unsafe class UnkHost
{
    public static long Grows;
    /// A GATE CONTROL only (CAMPAIGN req 19 as amended 2026-09-26: the counting build grows
    /// geometrically, as the timed build does, decision 11 rule 8): set, every grow allocates
    /// exactly the size asked, and the committed counts must then differ. Never set otherwise.
    public static bool Exact;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int Grow(IntPtr sink, int want, byte** dst, int* cap)
    {
        try
        {
            if (want < 0) return Abi.AK_ERR_LIMIT;
            int c = *cap;
            long nc = Exact ? want : Math.Max((long)want, Math.Max(64L, 2L * c));   // geometric (rule 8), clamped below
            if (nc > int.MaxValue) nc = int.MaxValue;   // rule 8: clamped to INT32_MAX (want <= INT32_MAX)
            var a = G.Arena;
            if (a == null) return Abi.AK_ERR_HOST;   // grow is armed only with an arena
            *dst = a.Grow(*dst, c, (int)nc);
            *cap = (int)nc;
            Grows++;
            return 0;
        }
        catch { return Abi.AK_ERR_HOST; }
    }

    public static IntPtr Fn => (IntPtr)(delegate* unmanaged[Cdecl]<IntPtr, int, byte**, int*, int>)&Grow;
}

/// Step a2 (iv): the native arena of one decode context's retained decodes. Chunks are kept
/// for the context's life and reused in order (the first 64 KiB; a new chunk is max(need, 2 x
/// the largest)); Begin rewinds it before each decode. Every allocation is 8-aligned and never
/// moves until the next Begin, except that growing the LAST allocation extends it in place
/// when the chunk has room. `Outstanding` counts buffers handed out (a fresh grow) and not yet
/// given back (G.Take / G.Drop): non-zero after a successful decode is UNDELIVERED.
public sealed unsafe class UnkArena : IDisposable
{
    private readonly System.Collections.Generic.List<IntPtr> _chunks = new System.Collections.Generic.List<IntPtr>();
    private readonly System.Collections.Generic.List<int> _caps = new System.Collections.Generic.List<int>();
    private int _ci = -1;
    private byte* _cur, _last;
    private int _cap, _at;
    public int Outstanding;

    public void Begin() { _ci = -1; _cur = null; _cap = 0; _at = 0; _last = null; Outstanding = 0; }

    private void Next(int n)
    {
        int i = _ci + 1;
        if (i >= _chunks.Count || _caps[i] < n)
        {
            int big = 1 << 15;
            foreach (var c in _caps) big = Math.Max(big, c);
            long size = Math.Max((long)n, 2L * big);
            if (size > int.MaxValue) size = Math.Max(n, int.MaxValue - 4095);
            _chunks.Insert(i, (IntPtr)NativeMemory.Alloc((nuint)size));
            _caps.Insert(i, (int)size);
        }
        _ci = i; _cur = (byte*)_chunks[i]; _cap = _caps[i]; _at = 0;
    }

    private static int Al(int n) => (n + 7) & ~7;

    /// realloc semantics on the arena: `old` (null, or this decode's allocation of `oldCap`
    /// bytes) to `n` bytes.
    public byte* Grow(byte* old, int oldCap, int n)
    {
        if (old != null && old == _last && (long)(old - _cur) + n <= _cap)
        {
            _at = (int)(old - _cur) + Al(n);   // the last allocation, extended in place
            return old;
        }
        if (_cur == null || (long)_cap - _at < n) Next(n);
        byte* p = _cur + _at;
        _at += Al(n);
        _last = p;
        if (old != null) Buffer.MemoryCopy(old, p, n, Math.Min(oldCap, n));
        else Outstanding++;
        return p;
    }

    public void Dispose()
    {
        foreach (var c in _chunks) NativeMemory.Free((void*)c);
        _chunks.Clear(); _caps.Clear(); Begin();
    }
}

'''


STAGE = r'''
/// Staging for strings, bytes and unknown-field bags handed to the core as data. Blocks
/// are allocated as needed and never moved or freed before Dispose, so a pointer handed out
/// stays valid until Reset (optimisation step a1, 2026-10-04): every block is KEPT across
/// encodes and reused in order, a new block is max(need, 2 x the last block) (geometric),
/// and a string reserves its maximum UTF-8 size but commits only the bytes it wrote.
public sealed unsafe class Stage : IDisposable
{
    private readonly System.Collections.Generic.List<IntPtr> _blocks = new System.Collections.Generic.List<IntPtr>();
    private readonly System.Collections.Generic.List<int> _caps = new System.Collections.Generic.List<int>();
    private int _bi;          // the block in use

    // ---- D21 (owner, 2026-10-04): the string encode path, chosen at run time (AK_STR_ENC):
    //   E0  (default) the string transcoded to UTF-8 by .NET into this staging, ak_tc_bytes (a copy);
    //   E1  the managed string PINNED (a GCHandle per string, freed when the codec call returns)
    //       and handed as UTF-16 with ak_tc_utf16 (the core's simdutf transcoder, D19): no copy here;
    //   E2  no pin: ak_str.data is 0x10000 + an index into this thread's string table and `tc` is the
    //       C# transcoder TcManaged ([UnmanagedCallersOnly]), which writes UTF-8 straight into the
    //       core's buffer (one reverse call per string);
    //   ETH:<n>  E1 for a string of at least n UTF-16 code units, E0 below.
    // D21 step 7 (owner, 2026-10-04):
    //   E3  E2's table and callback, but the callback pins the string with `fixed` and calls the
    //       core's additive export ak_utf16_to_utf8 (simdutf) into `dst`, after growing to the
    //       worst case (3 bytes per code unit) when `cap` is below it;
    //   E3L the same, sized by ak_utf16_utf8_len first (grow to the exact length when short);
    //   E1R E1 WITHOUT GCHandles: the fill marks the string (data = PinPending, len, ak_tc_utf16)
    //       and the generated frames pin it with `fixed` around the call that reads it: the root
    //       group's strings in one nested `fixed` scope around ak_encode; an element chunk by
    //       BOUNDED RECURSION, one frame per ELEMENT (every singular string of the element and of
    //       its inlined children in one `fixed`), the deepest frame making the element call; a
    //       repeated string field by bounded recursion, one frame per string, delivered as several
    //       ak_blob_run calls of at most PinK strings; a map's strings by GCHandle (fallback, `hpin`);
    //   E1C the same marks and chunks as E1R, pinned by GCHandles that are freed after each chunk's
    //       call (the attribution control: E1's handles, at most one chunk live).
    //   <MODE>:<n> the mode for a string of at least n code units, E0 below; <MODE>:<n>:na also
    //       sends an ASCII-only string to E0 (content-aware: one Ascii.IsValid scan per string).
    public const int E0 = 0, E1 = 1, E2 = 2, ETH = 3, E3 = 4, E3L = 5, E1R = 6, E1C = 7;
    public static int Mode = ParseMode(Environment.GetEnvironmentVariable("AK_STR_ENC"), out Threshold, out NonAsciiOnly);
    public static int Threshold;
    public static bool NonAsciiOnly;
    public static int ParseMode(string v, out int th, out bool na)
    {
        th = 0; na = false;
        if (string.IsNullOrEmpty(v) || v == "E0") return E0;
        var parts = v.Split(':');
        int m;
        switch (parts[0])
        {
            case "E1": m = E1; break;
            case "E2": m = E2; break;
            case "ETH": m = ETH; break;
            case "E3": m = E3; break;
            case "E3L": m = E3L; break;
            case "E1R": m = E1R; break;
            case "E1C": m = E1C; break;
            default: throw new ArgumentException("AK_STR_ENC: E0 | E1 | E2 | E3 | E3L | E1R | E1C [:<chars>[:na]] | ETH:<chars>, not " + v);
        }
        if (parts.Length > 1) th = int.Parse(parts[1], System.Globalization.CultureInfo.InvariantCulture);
        else if (m == ETH) throw new ArgumentException("AK_STR_ENC: ETH needs a threshold (ETH:<chars>)");
        if (parts.Length > 2) { if (parts[2] != "na") throw new ArgumentException("AK_STR_ENC: the third part is `na`, not " + parts[2]); na = true; }
        if (parts.Length > 3) throw new ArgumentException("AK_STR_ENC: " + v);
        return m;
    }
    public static string ModeName => Mode switch
    {
        E0 => "E0",
        ETH => "ETH:" + Threshold,
        _ => (Mode switch { E1 => "E1", E2 => "E2", E3 => "E3", E3L => "E3L", E1R => "E1R", _ => "E1C" })
             + (Threshold > 0 || NonAsciiOnly ? ":" + Threshold : "") + (NonAsciiOnly ? ":na" : "")
             + (Mode == E1R || Mode == E1C ? " (K " + PinK + ")" : ""),
    };
    /// E1R / E1C: 1 or 2 when the generated frames pin (the fill marks), 0 otherwise.
    public static int Defer => Mode == E1R ? 1 : Mode == E1C ? 2 : 0;
    /// The frames of the encode running on this thread: Defer, or 0 when its fill marked no
    /// string (under a threshold, an encode with no long string takes the default path whole:
    /// no frame per element). Set by Go before the root call; read by the loop callbacks.
    [ThreadStatic] public static int DeferNow;
    /// E1R / E1C: the marker the fill leaves in ak_str.data until a frame patches it; never a small
    /// value (ABI v1 section 8 reserves those) and never read by the core (patched before the call).
    public static readonly IntPtr PinPending = (IntPtr)0x30000;
    /// E1R / E1C: the elements (and repeated strings) per chunk, so per recursion depth. AK_STR_PINK.
    public static int PinK = ParsePinK(Environment.GetEnvironmentVariable("AK_STR_PINK"));
    /// The largest K accepted: the recursion uses at most 2 K frames (an element chunk, then a
    /// repeated-string or map chunk inside one element call) plus the core's frames, on a thread
    /// stack that may be .NET's 1.5 MB secondary-thread default. The largest frame the counting
    /// build measured over the shapes and U-* rows is 1,152 bytes at the default JIT tiers (336
    /// fully optimised; logs/csharp/opt/s7/frames/): 2 x 256 x 1,152 = 590 KB, 38 % of 1.5 MB;
    /// a larger K is refused.
    public const int MaxPinK = 256;
    public static int ParsePinK(string v)
    {
        if (string.IsNullOrEmpty(v)) return 64;
        int k = int.Parse(v, System.Globalization.CultureInfo.InvariantCulture);
        if (k < 1 || k > MaxPinK) throw new ArgumentException("AK_STR_PINK: 1.." + MaxPinK + " (stack bound), not " + v);
        return k;
    }
    /// E1R / E1C: marks left by the fill and marks patched by a frame (equal after every call);
    /// of the patches, those of repeated string fields (one frame per string) and of nested maps'
    /// keys and values (one frame per entry). PER THREAD: the per-encode check compares this
    /// thread's marks with its patches, and encodes run concurrently on the RPC callers (a
    /// process-wide counter made a k = 8 caller fail its check: JOURNAL 76).
    [ThreadStatic] public static long Marked, Patched, RepPatched, MapPatched;
    /// E1R / E1C: map strings pinned by the GCHandle fallback (counting build only).
    public static long HandlePins;
    /// E3 / E3L: calls into ak_utf16_to_utf8 / ak_utf16_utf8_len (counting build only; the ABI
    /// does not count these additive exports).
    public static long U16Calls, U16LenCalls;
    /// Stack bytes per recursion frame: the largest (address at depth 0 - address at depth d) / d
    /// seen by the counting build (AK_HOST_COUNT).
    public static long FrameBytes;
    [ThreadStatic] private static byte* _sp0;
    public static void Sp(int depth, byte* sp)
    {
        if (depth == 0) { _sp0 = sp; return; }
        long b = (_sp0 - sp) / depth;
        if (b > FrameBytes) FrameBytes = b;
    }
    [ThreadStatic] private static System.Collections.Generic.List<GCHandle> _chunkPins;
    /// E1C: a GCHandle pin held until ReleaseChunk (after the chunk's call).
    public static IntPtr PinChunk(string s)
    {
        Patched++;
        var h = GCHandle.Alloc(s, GCHandleType.Pinned);
        (_chunkPins ??= new System.Collections.Generic.List<GCHandle>()).Add(h);
        return h.AddrOfPinnedObject();
    }
    /// The handles taken so far on this thread: a chunk releases only what it added (a map or a
    /// repeated field's chunk runs INSIDE an element chunk's call, whose handles must outlive it:
    /// releasing them all there let the core read moved strings (JOURNAL 76)).
    public static int ChunkMark() => _chunkPins?.Count ?? 0;
    public static void ReleaseChunk(int from = 0)
    {
        var l = _chunkPins;
        if (l == null) return;
        for (int i = from; i < l.Count; i++) l[i].Free();
        l.RemoveRange(from, l.Count - from);
    }
    /// A GATE CONTROL, not a mode: AK_GATE_PIN_STRESS=1 runs a blocking compacting GC after every
    /// chunk's call (E1R / E1C), so a core that read a string after its frame had released it
    /// would read moved memory; the byte checks then run under it.
    internal static readonly bool PinStress = Environment.GetEnvironmentVariable("AK_GATE_PIN_STRESS") == "1";
    public static void AfterChunk()
    {
        if (PinStress) GC.Collect(2, GCCollectionMode.Forced, true, true);
    }
    /// A GATE CONTROL for the stress check: AK_GATE_PLANT_EARLY_UNPIN=1 makes E1C release a
    /// chunk's handles BEFORE its call and run the compacting GC, so the core reads strings that
    /// are no longer pinned; under it the byte checks must fail (the stress check can see a
    /// string read after its release).
    internal static readonly bool PlantEarlyUnpin = Environment.GetEnvironmentVariable("AK_GATE_PLANT_EARLY_UNPIN") == "1";
    public static void BeforeChunkCall(int from)
    {
        if (!PlantEarlyUnpin) return;
        ReleaseChunk(from);
        GC.Collect(2, GCCollectionMode.Forced, true, true);
    }
    [DllImport(Abi.Lib, EntryPoint = "ak_utf16_to_utf8", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern int ak_utf16_to_utf8(char* src, nuint len, byte* dst, nuint cap);
    [DllImport(Abi.Lib, EntryPoint = "ak_utf16_utf8_len", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    private static extern int ak_utf16_utf8_len(char* src, nuint len);
    private readonly System.Collections.Generic.List<GCHandle> _pins = new System.Collections.Generic.List<GCHandle>();
    private static IntPtr _tcU16;
    /// E2's table: the strings of the encode running on this thread (the core calls the
    /// transcoder on the encoding thread, inside the codec call).
    [ThreadStatic] private static System.Collections.Generic.List<string> _tab;
    /// Reverse calls into TcManaged (E2), counted in the counting build only (AK_HOST_COUNT).
    public static long TcCalls;
    /// Strings handed by E1's pin (E1, or ETH at or above the threshold), counting build only.
    public static long PinCalls;
    private byte* _cur;
    private int _cap, _at;
    public readonly bool Utf16;
    public readonly IntPtr Tc, TcBytes;

    public Stage(bool utf16)
    {
        Utf16 = utf16;
        Tc = utf16 ? Abi.ak_tc_utf16() : Abi.ak_tc_bytes();
        TcBytes = Abi.ak_tc_bytes();
        _blocks.Add((IntPtr)NativeMemory.Alloc(1 << 16));
        _caps.Add(1 << 16);
        Use(0);
    }

    private void Use(int i)
    {
        _bi = i;
        _cur = (byte*)_blocks[i];
        _cap = _caps[i];
        _at = 0;
    }

    /// Every block is kept: the next encode starts again at the first.
    public void Reset() { Use(0); ReleasePins(); _tab?.Clear(); if (Mode == E1C) ReleaseChunk(); }

    /// E1: the pins of the last codec call, released once it has returned (the core copied
    /// the strings into its own buffer during the call).
    public void ReleasePins()
    {
        for (int i = 0; i < _pins.Count; i++) _pins[i].Free();
        _pins.Clear();
    }

    /// A GATE CONTROL, not a mode: AK_GATE_PLANT_STR=1 makes E1 and E2 hand one code unit (E1) or
    /// one byte (E2) short, so a check that runs under AK_STR_ENC=E1/E2 must fail (the path is live).
    internal static readonly bool PlantStr = Environment.GetEnvironmentVariable("AK_GATE_PLANT_STR") == "1";

    private ak_str Pin(string s)
    {
#if AK_HOST_COUNT
        PinCalls++;
#endif
        var h = GCHandle.Alloc(s, GCHandleType.Pinned);
        _pins.Add(h);
        if (_tcU16 == IntPtr.Zero) _tcU16 = Abi.ak_tc_utf16();
        return new ak_str { data = h.AddrOfPinnedObject(), len = (nuint)(PlantStr ? s.Length - 1 : s.Length), tc = _tcU16 };
    }

    private const nint TabBase = 0x10000;
    private static ak_str Tab(string s, int m)
    {
        var t = _tab ??= new System.Collections.Generic.List<string>();
        t.Add(s);
        // data = TabBase + index: never a small value (ABI v1 section 8 reserves small ak_str.data
        // values as sentinels: 1 is AK_STR_DIRECT; a first attempt with index + 1 was taken for it).
        IntPtr tc = m == E3 ? (IntPtr)(delegate* unmanaged[Cdecl]<void*, nuint, byte*, int, IntPtr, IntPtr, int>)&TcCore
            : m == E3L ? (IntPtr)(delegate* unmanaged[Cdecl]<void*, nuint, byte*, int, IntPtr, IntPtr, int>)&TcCoreLen
            : (IntPtr)(delegate* unmanaged[Cdecl]<void*, nuint, byte*, int, IntPtr, IntPtr, int>)&TcManaged;
        return new ak_str { data = (IntPtr)(TabBase + t.Count - 1), len = (nuint)s.Length, tc = tc };
    }

    /// E3: ak_transcode_fn. As TcManaged, but the UTF-16 is pinned with `fixed` and converted by the
    /// core's ak_utf16_to_utf8 (simdutf, the bytes ak_tc_utf16 writes); `dst` grown to the worst
    /// case (3 bytes per code unit) first when `cap` is below it.
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static int TcCore(void* src, nuint len, byte* dst, int cap, IntPtr grow, IntPtr sink)
    {
        try
        {
#if AK_HOST_COUNT
            TcCalls++; U16Calls++;
#endif
            var s = _tab[(int)((nint)src - TabBase)];
            long worst = (long)s.Length * 3;
            if (cap < worst)
            {
                int rc = ((delegate* unmanaged[Cdecl]<IntPtr, int, byte**, int*, int>)grow)(sink, (int)Math.Min(worst, int.MaxValue), &dst, &cap);
                if (rc < 0) return rc;
            }
            int w;
            fixed (char* c = s) w = ak_utf16_to_utf8(c, (nuint)s.Length, dst, (nuint)cap);
            return PlantStr && w > 0 ? w - 1 : w;
        }
        catch { return Abi.AK_ERR_HOST; }
    }

    /// E3L: as TcCore, sized by ak_utf16_utf8_len first (a grow to the exact length when short).
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static int TcCoreLen(void* src, nuint len, byte* dst, int cap, IntPtr grow, IntPtr sink)
    {
        try
        {
#if AK_HOST_COUNT
            TcCalls++; U16Calls++; U16LenCalls++;
#endif
            var s = _tab[(int)((nint)src - TabBase)];
            int w;
            fixed (char* c = s)
            {
                int need = ak_utf16_utf8_len(c, (nuint)s.Length);
                if (need < 0) return need;
                if (need > cap)
                {
                    int rc = ((delegate* unmanaged[Cdecl]<IntPtr, int, byte**, int*, int>)grow)(sink, need, &dst, &cap);
                    if (rc < 0) return rc;
                }
                w = ak_utf16_to_utf8(c, (nuint)s.Length, dst, (nuint)cap);
            }
            return PlantStr && w > 0 ? w - 1 : w;
        }
        catch { return Abi.AK_ERR_HOST; }
    }

    /// E2: ak_transcode_fn. `src` is TabBase + the string's index in this thread's table; writes the
    /// string's UTF-8 at `dst` (a lone surrogate becomes EF BF BD, as ak_tc_utf16 writes it),
    /// asking `grow` for the exact size when the worst case does not fit.
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static int TcManaged(void* src, nuint len, byte* dst, int cap, IntPtr grow, IntPtr sink)
    {
        try
        {
#if AK_HOST_COUNT
            TcCalls++;
#endif
            var s = _tab[(int)((nint)src - TabBase)];
            if ((long)cap < (long)s.Length * 3)
            {
                int need = Encoding.UTF8.GetByteCount(s);
                if (need > cap)
                {
                    int rc = ((delegate* unmanaged[Cdecl]<IntPtr, int, byte**, int*, int>)grow)(sink, need, &dst, &cap);
                    if (rc < 0) return rc;
                }
            }
            int w = Encoding.UTF8.GetBytes(s, new Span<byte>(dst, cap));
            return PlantStr ? w - 1 : w;
        }
        catch { return Abi.AK_ERR_HOST; }
    }

    /// `n` contiguous bytes at the cursor: the next kept block if it is large enough, else a
    /// new block of max(n, 2 x the largest), placed next so the order of reuse is stable.
    private byte* Room(int n)
    {
        if (_cap - _at >= n) return _cur + _at;
        int next = _bi + 1;
        if (next < _blocks.Count && _caps[next] >= n) { Use(next); return _cur; }
        int last = 0;
        foreach (var c in _caps) last = Math.Max(last, c);
        long size = Math.Max((long)n, 2L * last);
        if (size > int.MaxValue) size = Math.Max(n, int.MaxValue - 4095);
        _blocks.Insert(next, (IntPtr)NativeMemory.Alloc((nuint)size));
        _caps.Insert(next, (int)size);
        Use(next);
        return _cur;
    }

    private void Commit(int n) => _at += (n + 7) & ~7;

    private byte* Take(int n)
    {
        byte* p = Room(n);
        Commit(n);
        return p;
    }

    /// An implicit-presence string: EMPTY is absent (`tc` null).
    public ak_str Str(string s)
    {
        if (s == null || s.Length == 0) return default;
        return StrPresent(s);
    }

    /// An explicit or oneof string: present even when empty, so `tc` is always set.
    public ak_str StrPresent(string s)
    {
        if (s.Length == 0) return new ak_str { data = IntPtr.Zero, len = 0, tc = Tc };
        // The non-default paths in a method of their own, so this one keeps the E0 shape it had
        // before them (JOURNAL 76: the default encode measured about 8 to 15 ns slower otherwise).
        // The length test inline (a threshold variant's short strings never make the call:
        // JOURNAL 76 measured the call per string at about 3 to 6 ns on the grid's rows).
        if (Mode != E0 && s.Length >= Threshold && !Utf16 && Alt(s, out var alt)) return alt;
        if (Utf16)
        {
            // `len` counts CODE UNITS: ak_tc_utf16 reads `*const u16`.
            byte* p = Take(s.Length * 2);
            fixed (char* c = s) Buffer.MemoryCopy(c, p, s.Length * 2, s.Length * 2);
            return new ak_str { data = (IntPtr)p, len = (nuint)s.Length, tc = Tc };
        }
        int max = Encoding.UTF8.GetMaxByteCount(s.Length);
        byte* q = Room(max);   // reserve the maximum, commit what was written
        int n;
        fixed (char* c = s) n = Encoding.UTF8.GetBytes(c, s.Length, q, max);
        Commit(n);
        return new ak_str { data = (IntPtr)q, len = (nuint)n, tc = Tc };
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private bool Alt(string s, out ak_str r)
    {
        r = default;
        int m = Mode;
        if (NonAsciiOnly && IsAscii(s)) return false;
        switch (m)
        {
            case E1: case ETH: r = Pin(s); return true;
            case E2: case E3: case E3L: r = Tab(s, m); return true;
            default:   // E1R, E1C: marked here, pinned by the frame around the call that reads it
                Marked++;
                if (_tcU16 == IntPtr.Zero) _tcU16 = Abi.ak_tc_utf16();
                r = new ak_str { data = PinPending, len = (nuint)(PlantStr ? s.Length - 1 : s.Length), tc = _tcU16 };
                return true;
        }
    }

    private static bool IsAscii(string s)
    {
#if NET8_0_OR_GREATER
        return System.Text.Ascii.IsValid(s);
#else
        foreach (char c in s) if (c > 0x7F) return false;
        return true;
#endif
    }

    /// A map entry's key or value. E1R / E1C have no frame around a map's element call: their
    /// strings take E1's GCHandle (freed when the codec call returns), counted as `hpin`.
    public ak_str StrH(string s)
    {
        if (Defer != 0 && s != null && s.Length != 0 && s.Length >= Threshold && !Utf16 && !(NonAsciiOnly && IsAscii(s)))
        {
#if AK_HOST_COUNT
            HandlePins++;
#endif
            return Pin(s);
        }
        return Str(s);
    }

    public ak_str Bytes(byte[] b)
    {
        if (b == null || b.Length == 0) return default;
        return BytesPresent(b);
    }

    public ak_str BytesPresent(byte[] b)
    {
        if (b.Length == 0) return new ak_str { data = IntPtr.Zero, len = 0, tc = TcBytes };
        byte* p = Take(b.Length);
        fixed (byte* s = b) Buffer.MemoryCopy(s, p, b.Length, b.Length);
        return new ak_str { data = (IntPtr)p, len = (nuint)b.Length, tc = TcBytes };
    }

    /// The unknown-field bag: raw runs, no transcoder (ABI v1 decision 11 candidate).
    public ak_blob Blob(byte[] b)
    {
        if (b == null || b.Length == 0) return default;
        byte* p = Take(b.Length);
        fixed (byte* s = b) Buffer.MemoryCopy(s, p, b.Length, b.Length);
        return new ak_blob { data = (IntPtr)p, len = (nuint)b.Length };
    }

    public void Dispose()
    {
        foreach (var b in _blocks) NativeMemory.Free((void*)b);
        _blocks.Clear();
        _caps.Clear();
    }
}

/// A native array that only grows, re-allocated before any pointer into it is handed out.
public static unsafe class Arr
{
    /// plan ENCODE RULES (WP5 step 6): map entries in ascending order of the key's UTF-8
    /// bytes, which is code-point order. `OrderedMap` keeps INSERTION order, so the
    /// encoder hands entries over in this order whatever order the map was filled in.
    public static int[] Utf8Order(OrderedMap<string, string> m)
    {
        var ix = new int[m.Count];
        for (int i = 0; i < ix.Length; i++) ix[i] = i;
        if (ix.Length > 1) Array.Sort(ix, (x, y) => CmpUtf8(m.At(x).Key, m.At(y).Key));
        return ix;
    }

    private static int Cp(string s, ref int i)
    {
        char c = s[i++];
        if (char.IsHighSurrogate(c) && i < s.Length && char.IsLowSurrogate(s[i]))
            return char.ConvertToUtf32(c, s[i++]);
        return c;
    }

    public static int CmpUtf8(string a, string b)
    {
        int i = 0, j = 0;
        while (i < a.Length && j < b.Length)
        {
            int x = Cp(a, ref i), y = Cp(b, ref j);
            if (x != y) return x < y ? -1 : 1;
        }
        return (i < a.Length ? 1 : 0) - (j < b.Length ? 1 : 0);
    }

    public static void Ensure(ref void* p, ref int cap, int n, int size)
    {
        if (n <= cap && p != null) return;
        int c = Math.Max(n, Math.Max(16, cap * 2));
        p = NativeMemory.Realloc(p, (nuint)((long)c * size));
        cap = c;
    }
}
'''


# ======================================================================== per root

def _stage_elem(o, s, arr, idx, src, ind, retain_expr):
    """One element of slot `s` staged into `arr[idx]` from facade element `src`."""
    if s.kind == "blob":
        fn = "st.Str" if s.f.kind == "string" else "st.Bytes"
        o += "%s((ak_str*)%s)[%s] = %s(%s);" % (ind, arr, idx, fn.replace("st.", "_st."), src)
    elif s.kind == "packed":
        o += "%s((%s*)%s)[%s] = %s;" % (ind, s.cs_e, arr, idx, _enc(s.f, src))
    elif s.kind == "map":
        o += "%s{ ref var e = ref ((%s*)%s)[%s]; e = default; e.key = _st.%s(%s.Key); e.value = _st.%s(%s.Value); }" % (
            ind, s.cs_e, arr, idx, "StrH" if s.top else "Str", src, "StrH" if s.top else "Str", src)
    else:
        if retain_expr and s.top and not _NO:
            o += "%sif (%s) G.U_%s(ref ((%s*)%s)[%s], %s, _st);" % (ind, retain_expr, s.et, s.u_elem(), arr, idx, src)
            o += "%selse G.E_%s(ref ((%s*)%s)[%s], %s, _st);" % (ind, s.et, s.cs_e, arr, idx, src)
        else:
            o += "%sG.E_%s(ref ((%s*)%s)[%s], %s, _st);" % (ind, s.et, s.cs_e, arr, idx, src)


def _elem_size(s):
    if s.kind == "msg" and s.top and not _NO:
        return "Math.Max(sizeof(%s), sizeof(%s))" % (s.cs_e, s.u_elem())
    return "sizeof(%s)" % s.cs_e


def _loop_forward(s, arr, n, tok):
    """The forward call a loop callback makes for slot `s` (plan: leaf batches)."""
    if s.kind == "blob":
        return "Abi.ak_blob_run(ctx, (ak_str*)%s, %s)" % (arr, n)
    if s.kind == "packed":
        return "Abi.%s(ctx, (%s*)%s, (nuint)%s)" % (RUN_FN[slot_elem(s.f)[1]], s.cs_e, arr, n)
    if s.kind == "map":
        return "Abi.ak_elem_%s(ctx, (%s*)%s, %s)" % (s.et, s.cs_e, arr, n)
    if s.leaf:
        return "Abi.ak_elem_%s(ctx, (%s*)%s, %s)" % (s.et, s.cs_e, arr, n)
    return "Abi.ak_elemu_%s(ctx, (%s*)%s, %s, %s)" % (s.et, s.cs_e, arr, n, tok)


def _loop_forward_u(s, arr, n, tok):
    if s.leaf:
        return "Abi.ak_uelem_%s(ctx, (%s*)%s, %s)" % (s.et, s.u_elem(), arr, n)
    return "Abi.ak_uelemu_%s(ctx, (%s*)%s, %s, %s)" % (s.et, s.u_elem(), arr, n, tok)


def _add_body(o, s, lst, xs, n, ind, var="i"):
    """Append a decoded run to facade list `lst`."""
    if s.kind == "blob":
        fn = "G.Str" if s.f.kind == "string" else "G.Bytes"
        o += "%sfor (int %s = 0; %s < %s; %s++) %s.Add(%s(b, %s[%s]));" % (ind, var, var, n, var, lst, fn, xs, var)
    elif s.kind == "packed":
        o += "%sfor (int %s = 0; %s < %s; %s++) %s.Add(%s);" % (ind, var, var, n, var, lst, _dec(s.f, "%s[%s]" % (xs, var)))
    elif s.kind == "map":
        if _NO:
            o += "%s// plan: a duplicate key replaces the earlier value." % ind
            o += "%sfor (int %s = 0; %s < %s; %s++) %s[G.Str(b, %s[%s].key)] = G.Str(b, %s[%s].value);" % (
                ind, var, var, n, var, lst, xs, var, xs, var)
        else:
            o += "%s// plan: a duplicate key replaces the earlier value. The facade map has no bag:" % ind
            o += "%s// an entry's unknown-field buffer (decision 11) is freed (U-map-entry)." % ind
            o += "%sfor (int %s = 0; %s < %s; %s++) { %s[G.Str(b, %s[%s].key)] = G.Str(b, %s[%s].value); G.Drop(ref %s[%s].unknown); }" % (
                ind, var, var, n, var, lst, xs, var, xs, var, xs, var)
    else:
        o += "%sfor (int %s = 0; %s < %s; %s++) { var x = new %s(); G.D_%s(ref %s[%s], x, b); %s.Add(x); }" % (
            ind, var, var, n, var, s.et, s.et, xs, var, lst)


# ---------------------------------------------------------------- D21 step 7: E1R / E1C frames

def _pin_fields(p, mname, sv):
    """Every string a group of `mname` carries as an ak_str member (its own singular and explicit
    strings, its oneof string members, and those of every inlined singular or oneof message
    child), as (facade expression, group member path), with the local declarations the
    expressions use (`sv` is the facade object, never null). The fill (_emit_fill) assigns
    exactly these members from exactly these facade fields."""
    decls, pins, n = [], [], [0]

    def walk(mn, v, gpath, nullable):
        m = p.msg(mn)
        dot = "?." if nullable else "."
        for f in m.plain:
            if f.card != "singular" or f.direct:
                continue
            if f.kind == "string":
                pins.append(("%s%s%s" % (v, dot, N.field(f.name)), gpath + f.name))
            elif f.kind == "message":
                n[0] += 1
                cv = "__c%d" % n[0]
                decls.append("var %s = %s%s%s;" % (cv, v, dot, N.field(f.name)))
                walk(f.of, cv, gpath + f.name + ".", True)
        for oname, members in m.oneofs.items():
            ct = N.oneof_case_type(m.name, oname)
            for gm in members:
                cond = "%s%s.%s == %s.%s" % ("%s != null && " % v if nullable else "", v, N.oneof_case_field(oname), ct, N.pascal(gm.name))
                if gm.kind == "string":
                    pins.append(("(%s ? (%s.%s ?? \"\") : null)" % (cond, v, N.field(gm.name)), gpath + "%s_%s" % (oname, gm.name)))
                elif gm.kind == "message":
                    n[0] += 1
                    cv = "__c%d" % n[0]
                    decls.append("var %s = (%s) ? %s.%s : null;" % (cv, cond, v, N.field(gm.name)))
                    walk(gm.of, cv, gpath + "%s_%s." % (oname, gm.name), True)
    walk(mname, sv, "", False)
    # Only the children a pin reaches (a child with no string needs no local).
    import re
    used = set(re.findall(r"__c\d+", " ".join(e for e, _ in pins)))
    for d in reversed(decls):
        name = d.split()[1]
        if name in used:
            used |= set(re.findall(r"__c\d+", d.split("=", 1)[1]))
    return [d for d in decls if d.split()[1] in used], pins


def _pin_patch_r(o, ind, gp, pins, extra=""):
    """E1R: patch each marked member with its `fixed` pointer __p<i>."""
    for k, (_, path) in enumerate(pins):
        o += "%sif (%s->%s.data == Stage.PinPending) { %s->%s.data = (IntPtr)__p%d; Stage.Patched++;%s }" % (ind, gp, path, gp, path, k, extra)


def _pin_patch_h(o, ind, gp, pins, extra=""):
    """E1C: patch each marked member with a chunk-lived GCHandle pin."""
    for expr, path in pins:
        o += "%sif (%s->%s.data == Stage.PinPending) { %s->%s.data = Stage.PinChunk(%s);%s }" % (ind, gp, path, gp, path, expr, extra)


def _fixed_open(pins):
    return "fixed (char* %s)" % ", ".join("__p%d = %s" % (k, e) for k, (e, _) in enumerate(pins))


def _emit_pin_frames(o, p, root, s):
    """E1R's recursion (one frame per ELEMENT of a chunk) and E1C's chunk pinning for the element
    slot `s` of `root` (a top-level message slot); None when its element carries no string."""
    decls, pins = _pin_fields(p, s.et, "__e")
    if not pins:
        return False
    lst = "System.Collections.Generic.List<%s>" % s.et
    fwd_u = _loop_forward_u(s, "((%s*)run->S_%s + off)" % (s.u_elem(), s.name), "k", "off") if not _NO else None
    fwd_e = _loop_forward(s, "((%s*)run->S_%s + off)" % (s.cs_e, s.name), "k", "off")
    o += "    /// E1R: one frame per element of the chunk [off, off + k); each `fixed`s every string of"
    o += "    /// its element, patches the element's marked members, and recurses; the deepest frame"
    o += "    /// makes the element call, and unwinding releases the pins."
    o += "    private static int Rec_%s(IntPtr ctx, Run_%s* run, %s lst, int off, int k, int i)" % (s.name, root, lst)
    o += "    {"
    o += "#if AK_HOST_COUNT"
    o += "        Stage.Sp(i, (byte*)&i);"
    o += "#endif"
    o += "        if (i == k)"
    o += "        {"
    o += "            _fwd++;"
    if _NO:
        o += "            return %s;" % fwd_e
    else:
        o += "            return run->Retain != 0 ? %s : %s;" % (fwd_u, fwd_e)
    o += "        }"
    o += "        var __e = lst[off + i];"
    for d in decls:
        o += "        %s" % d
    o += "        %s" % _fixed_open(pins)
    o += "        {"
    if not _NO:
        o += "            if (run->Retain != 0)"
        o += "            {"
        o += "                var __g = (%s*)run->S_%s + off + i;" % (s.u_elem(), s.name)
        _pin_patch_r(o, "                ", "__g", pins)
        o += "            }"
        o += "            else"
    o += "            {"
    o += "                var __g = (%s*)run->S_%s + off + i;" % (s.cs_e, s.name)
    _pin_patch_r(o, "                ", "__g", pins)
    o += "            }"
    o += "            return Rec_%s(ctx, run, lst, off, k, i + 1);" % s.name
    o += "        }"
    o += "    }"
    o += ""
    o += "    /// E1C: the chunk's strings pinned by GCHandles, freed once its element call has returned."
    o += "    private static int ChunkH_%s(IntPtr ctx, Run_%s* run, %s lst, int off, int k)" % (s.name, root, lst)
    o += "    {"
    o += "        int __h = Stage.ChunkMark();   // this chunk releases only the handles it adds"
    o += "        for (int i = 0; i < k; i++)"
    o += "        {"
    o += "            var __e = lst[off + i];"
    for d in decls:
        o += "            %s" % d
    if not _NO:
        o += "            if (run->Retain != 0)"
        o += "            {"
        o += "                var __g = (%s*)run->S_%s + off + i;" % (s.u_elem(), s.name)
        _pin_patch_h(o, "                ", "__g", pins)
        o += "            }"
        o += "            else"
    o += "            {"
    o += "                var __g = (%s*)run->S_%s + off + i;" % (s.cs_e, s.name)
    _pin_patch_h(o, "                ", "__g", pins)
    o += "            }"
    o += "        }"
    o += "        Stage.BeforeChunkCall(__h);"
    o += "        _fwd++;"
    if _NO:
        o += "        int rc = %s;" % fwd_e
    else:
        o += "        int rc = run->Retain != 0 ? %s : %s;" % (fwd_u, fwd_e)
    o += "        Stage.ReleaseChunk(__h);"
    o += "        return rc;"
    o += "    }"
    o += ""
    return True


def _emit_pin_elems_inner(o, p, s, i):
    """E1R / E1C for a nested element slot `i` (leaf elements inside each element of `s`): the
    same per-element frames, over the element's run in I_<s>_<i>, chunks of at most Stage.PinK."""
    decls, pins = _pin_fields(p, i.et, "__e")
    name = "%s_%s" % (s.name, i.name)
    lst = "System.Collections.Generic.List<%s>" % i.et
    o += "    private static int Rec_%s(IntPtr ctx, %s* arr, %s lst, int off, int k, int j)" % (name, i.cs_e, lst)
    o += "    {"
    o += "#if AK_HOST_COUNT"
    o += "        Stage.Sp(j, (byte*)&j);"
    o += "#endif"
    o += "        if (j == k) { _fwd++; return %s; }" % _loop_forward(i, "(arr + off)", "k", None)
    o += "        var __e = lst[off + j];"
    for d in decls:
        o += "        %s" % d
    o += "        %s" % _fixed_open(pins)
    o += "        {"
    o += "            var __g = arr + off + j;"
    _pin_patch_r(o, "            ", "__g", pins)
    o += "            return Rec_%s(ctx, arr, lst, off, k, j + 1);" % name
    o += "        }"
    o += "    }"
    o += ""
    o += "    private static int ChunkH_%s(IntPtr ctx, %s* arr, %s lst, int off, int k)" % (name, i.cs_e, lst)
    o += "    {"
    o += "        int __h = Stage.ChunkMark();   // this chunk releases only the handles it adds"
    o += "        for (int j = 0; j < k; j++)"
    o += "        {"
    o += "            var __e = lst[off + j];"
    for d in decls:
        o += "            %s" % d
    o += "            var __g = arr + off + j;"
    _pin_patch_h(o, "            ", "__g", pins)
    o += "        }"
    o += "        Stage.BeforeChunkCall(__h);"
    o += "        _fwd++;"
    o += "        int rc = %s;" % _loop_forward(i, "(arr + off)", "k", None)
    o += "        Stage.ReleaseChunk(__h);"
    o += "        return rc;"
    o += "    }"
    o += ""
    o += "    private static int PinElems_%s(IntPtr ctx, %s* arr, %s lst, int n)" % (name, i.cs_e, lst)
    o += "    {"
    o += "        int K = Stage.PinK, d = Stage.DeferNow;"
    o += "        for (int off = 0; off < n; off += K)"
    o += "        {"
    o += "            int k = n - off; if (k > K) k = K;"
    o += "            int rc = d == 1 ? Rec_%s(ctx, arr, lst, off, k, 0) : ChunkH_%s(ctx, arr, lst, off, k);" % (name, name)
    o += "            Stage.AfterChunk();"
    o += "            if (rc < 0) return rc;"
    o += "        }"
    o += "        return 0;"
    o += "    }"
    o += ""


def _emit_pin_map_inner(o, s, i):
    """E1R / E1C for a map nested in each element of `s`: one frame per ENTRY (its key and value
    in one `fixed`), chunks of at most Stage.PinK entries, each one ak_elem_<Entry> call."""
    name = "%s_%s" % (s.name, i.name)
    m = "OrderedMap<string, string>"
    o += "    private static int RecM_%s(IntPtr ctx, %s* arr, %s m, int off, int k, int j)" % (name, i.cs_e, m)
    o += "    {"
    o += "#if AK_HOST_COUNT"
    o += "        Stage.Sp(j, (byte*)&j);"
    o += "#endif"
    o += "        if (j == k) { _fwd++; return %s; }" % _loop_forward(i, "(arr + off)", "k", None)
    o += "        var kv = m.At(off + j);"
    o += "        fixed (char* __p0 = kv.Key, __p1 = kv.Value)"
    o += "        {"
    o += "            var __g = arr + off + j;"
    _pin_patch_r(o, "            ", "__g", [("kv.Key", "key"), ("kv.Value", "value")], " Stage.MapPatched++;")
    o += "            return RecM_%s(ctx, arr, m, off, k, j + 1);" % name
    o += "        }"
    o += "    }"
    o += ""
    o += "    private static int ChunkHM_%s(IntPtr ctx, %s* arr, %s m, int off, int k)" % (name, i.cs_e, m)
    o += "    {"
    o += "        int __h = Stage.ChunkMark();   // this chunk releases only the handles it adds"
    o += "        for (int j = 0; j < k; j++)"
    o += "        {"
    o += "            var kv = m.At(off + j);"
    o += "            var __g = arr + off + j;"
    _pin_patch_h(o, "            ", "__g", [("kv.Key", "key"), ("kv.Value", "value")], " Stage.MapPatched++;")
    o += "        }"
    o += "        Stage.BeforeChunkCall(__h);"
    o += "        _fwd++;"
    o += "        int rc = %s;" % _loop_forward(i, "(arr + off)", "k", None)
    o += "        Stage.ReleaseChunk(__h);"
    o += "        return rc;"
    o += "    }"
    o += ""
    o += "    private static int PinMap_%s(IntPtr ctx, %s* arr, %s m, int n)" % (name, i.cs_e, m)
    o += "    {"
    o += "        int K = Stage.PinK, d = Stage.DeferNow;"
    o += "        for (int off = 0; off < n; off += K)"
    o += "        {"
    o += "            int k = n - off; if (k > K) k = K;"
    o += "            int rc = d == 1 ? RecM_%s(ctx, arr, m, off, k, 0) : ChunkHM_%s(ctx, arr, m, off, k);" % (name, name)
    o += "            Stage.AfterChunk();"
    o += "            if (rc < 0) return rc;"
    o += "        }"
    o += "        return 0;"
    o += "    }"
    o += ""


def _emit_pin_strs(o, name):
    """E1R / E1C for a repeated string field: one frame per string (E1R) or the chunk's handles
    (E1C), each chunk one ak_blob_run of at most Stage.PinK strings."""
    o += "    /// E1R: one frame per string of [off, off + k) of a repeated string field; the deepest"
    o += "    /// frame makes the chunk's ak_blob_run."
    o += "    private static int RecS_%s(IntPtr ctx, ak_str* arr, System.Collections.Generic.List<string> l, int off, int k, int j)" % name
    o += "    {"
    o += "#if AK_HOST_COUNT"
    o += "        Stage.Sp(j, (byte*)&j);"
    o += "#endif"
    o += "        if (j == k) { _fwd++; return Abi.ak_blob_run(ctx, arr + off, k); }"
    o += "        fixed (char* __p = l[off + j])"
    o += "        {"
    o += "            if (arr[off + j].data == Stage.PinPending) { arr[off + j].data = (IntPtr)__p; Stage.Patched++; Stage.RepPatched++; }"
    o += "            return RecS_%s(ctx, arr, l, off, k, j + 1);" % name
    o += "        }"
    o += "    }"
    o += ""
    o += "    private static int ChunkHS_%s(IntPtr ctx, ak_str* arr, System.Collections.Generic.List<string> l, int off, int k)" % name
    o += "    {"
    o += "        int __h = Stage.ChunkMark();   // this chunk releases only the handles it adds"
    o += "        for (int j = 0; j < k; j++) if (arr[off + j].data == Stage.PinPending) { arr[off + j].data = Stage.PinChunk(l[off + j]); Stage.RepPatched++; }"
    o += "        Stage.BeforeChunkCall(__h);"
    o += "        _fwd++;"
    o += "        int rc = Abi.ak_blob_run(ctx, arr + off, k);"
    o += "        Stage.ReleaseChunk(__h);"
    o += "        return rc;"
    o += "    }"
    o += ""
    o += "    private static int PinStrs_%s(IntPtr ctx, ak_str* arr, System.Collections.Generic.List<string> l, int n)" % name
    o += "    {"
    o += "        int K = Stage.PinK, d = Stage.DeferNow;"
    o += "        for (int off = 0; off < n; off += K)"
    o += "        {"
    o += "            int k = n - off; if (k > K) k = K;"
    o += "            int rc = d == 1 ? RecS_%s(ctx, arr, l, off, k, 0) : ChunkHS_%s(ctx, arr, l, off, k);" % (name, name)
    o += "            Stage.AfterChunk();"
    o += "            if (rc < 0) return rc;"
    o += "        }"
    o += "        return 0;"
    o += "    }"
    o += ""


def _root_call(o, ind, call, rpins, x, ds):
    """The root encode call; under E1R / E1C a call to the root's pin method (RootPinR_/H_<x>),
    kept out of Go so the default path's code is not the frame's (the `fixed` scope and its
    pinned locals in Go cost the E0 encode about 15 ns: JOURNAL 76). An encode whose fill
    marked nothing takes the default path (DeferNow 0)."""
    decls, pins = rpins
    o += ind + "if (__d != 0) { if (Stage.Marked == __mk0) __d = 0; else { _pinSrc = src; Stage.DeferNow = __d; } }"
    if not pins:
        o += ind + call
        return
    args = "_run, _ctx, &vt, &fix, src%s" % (", direct" if ds else "")
    o += ind + "if (__d == 0) " + call
    o += ind + "else rc = __d == 1 ? RootPinR_%s(%s) : RootPinH_%s(%s);" % (x, args, x, args)


def _emit_root_pins(o, p, root, ds, rpins):
    """E1R / E1C: the root group's strings pinned around the root encode call (one `fixed` scope
    with every root string, or chunk-lived GCHandles), in methods of their own."""
    decls, pins = rpins
    if not pins:
        return
    for x, fix, fn in ([("e", "ak_efix", "ak_encode")] + ([] if _NO else [("u", "ak_ufix", "ak_uencode")])):
        sig = "Run_%s* _run, IntPtr _ctx, ak_evt_%s* vt, %s_%s* __g, %s src%s" % (root, root, fix, root, root, ", byte[] direct" if ds else "")
        call = "Abi.%s_%s(_run, _ctx, vt, __g%s)" % (fn, root, ", dp, (nuint)direct.Length" if ds else "")
        o += "    [MethodImpl(MethodImplOptions.NoInlining)]"
        o += "    private static nint RootPinR_%s(%s)" % (x, sig)
        o += "    {"
        for d in decls:
            o += "        " + d
        o += "        " + _fixed_open(pins)
        o += "        {"
        _pin_patch_r(o, "            ", "__g", pins)
        o += "            %sreturn %s;" % ("fixed (byte* dp = direct) " if ds else "", call)
        o += "        }"
        o += "    }"
        o += ""
        o += "    [MethodImpl(MethodImplOptions.NoInlining)]"
        o += "    private static nint RootPinH_%s(%s)" % (x, sig)
        o += "    {"
        o += "        int __h = Stage.ChunkMark();   // this chunk releases only the handles it adds"
        for d in decls:
            o += "        " + d
        _pin_patch_h(o, "        ", "__g", pins)
        o += "        nint rc;"
        o += "        %src = %s;" % ("fixed (byte* dp = direct) " if ds else "", call)
        o += "        Stage.ReleaseChunk(__h);"
        o += "        return rc;"
        o += "    }"
        o += ""


def _emit_root(o, p, root, facade_ns):
    slots = [Slot(p, root, path, f, True) for path, f in loop_slots(p, root)]
    ds = direct_fields(p, root)
    cls = "CoreFfi_%s" % root
    o += "[StructLayout(LayoutKind.Sequential)]"
    o += "public unsafe struct Run_%s" % root
    o += "{"
    o += "    public int Chunk;"
    o += "    public int Retain;"
    for s in slots:
        o += "    public void* S_%s; public int N_%s;" % (s.name, s.name)
        for i in s.inner:
            o += "    public void* I_%s_%s; public int* O_%s_%s; public int* C_%s_%s;" % (
                s.name, i.name, s.name, i.name, s.name, i.name)
    o += "}"
    o += ""
    o.doc("core-ffi for `%s`: every group, slot and entry point from the plan." % root)
    o += "public sealed unsafe class %s : IDisposable" % cls
    o += "{"
    o += "    private IntPtr _ctx, _dctx;"
    o += "    private readonly Stage _st;"
    o += "    private Run_%s* _run;" % root
    o += "    private DecRun* _drun;"
    o += "    public int Chunk;"
    o += "    /// Counted where each crossing happens (R5). Static: an [UnmanagedCallersOnly]"
    o += "    /// callback cannot reach an instance; one arm at a time."
    o += "    private static long _fwd, _rev;"
    o += "    public long ForwardCalls => _fwd;"
    o += "    public long ReverseCalls => _rev;"
    o += "    public void CallsReset() { _fwd = 0; _rev = 0;%s }" % ("" if _NO else " _resets = 0;")
    for s in slots:
        o += "    private int _cap_%s;" % s.name
        for i in s.inner:
            o += "    private int _cap_%s_%s, _capo_%s_%s;" % (s.name, i.name, s.name, i.name)
        if s.has_evt:
            o += "    private ak_evt_%s* _evt_%s;" % (s.et, s.name)
    o += ""
    o += "    public %s(bool utf16 = false)" % cls
    o += "    {"
    o += "        _ctx = Abi.ak_enc_ctx_new();"
    o += "        if (_ctx == IntPtr.Zero) throw new InvalidOperationException(\"ak_enc_ctx_new returned null\");"
    o += "        _st = new Stage(utf16 || Environment.GetEnvironmentVariable(\"AK_UTF16\") == \"1\");"
    o += "        _run = (Run_%s*)NativeMemory.AllocZeroed((nuint)sizeof(Run_%s));" % (root, root)
    for s in slots:
        if s.has_evt:
            o += "        _evt_%s = (ak_evt_%s*)NativeMemory.AllocZeroed((nuint)sizeof(ak_evt_%s));" % (s.name, s.et, s.et)
            for i in s.inner:
                o += "        _evt_%s->loop_%s = &Loop_%s_%s;" % (s.name, i.name, s.name, i.name)
    o += "    }"
    o += ""
    # ---------------- loops
    o += "    /// E1R / E1C (D21 step 7): the facade root of the encode running on this thread, for the"
    o += "    /// frames inside the loop callbacks to pin its strings."
    o += "    [ThreadStatic] private static %s _pinSrc;" % root
    o += ""
    for s in slots:
        pinned = s.kind == "msg" and _emit_pin_frames(o, p, root, s)
        if s.kind == "blob" and s.f.kind == "string":
            _emit_pin_strs(o, s.name)
        for i in s.inner:
            if i.kind == "blob" and i.f.kind == "string":
                _emit_pin_strs(o, "%s_%s" % (s.name, i.name))
            elif i.kind == "msg" and _pin_fields(p, i.et, "__e")[1]:
                _emit_pin_elems_inner(o, p, s, i)
            elif i.kind == "map":
                _emit_pin_map_inner(o, s, i)
        o += "    %s" % UCO
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
            if pinned:
                o += "            int d = Stage.DeferNow;"
                o += "            if (d != 0 && chunk > Stage.PinK) chunk = Stage.PinK;"
            o += "            for (int off = 0; off < n; off += chunk)"
            o += "            {"
            o += "                int k = n - off; if (k > chunk) k = chunk;"
            if pinned:
                lst = _get("_pinSrc", p, root, s.path)
                o += "                if (d != 0)"
                o += "                {"
                o += "                    int rp = d == 1 ? Rec_%s(ctx, run, %s, off, k, 0) : ChunkH_%s(ctx, run, %s, off, k);" % (s.name, lst, s.name, lst)
                o += "                    Stage.AfterChunk();"
                o += "                    if (rp < 0) return rp;"
                o += "                    continue;"
                o += "                }"
            o += "                _fwd++;"
            if _NO:
                o += "                int rc = %s;" % _loop_forward(s, "((%s*)run->S_%s + off)" % (s.cs_e, s.name), "k", "off")
            else:
                o += "                int rc = run->Retain != 0"
                o += "                    ? %s" % _loop_forward_u(s, "((%s*)run->S_%s + off)" % (s.u_elem(), s.name), "k", "off")
                o += "                    : %s;" % _loop_forward(s, "((%s*)run->S_%s + off)" % (s.cs_e, s.name), "k", "off")
            o += "                if (rc < 0) return rc;"
            o += "            }"
            o += "            return 0;"
        else:
            if s.kind == "blob" and s.f.kind == "string":
                o += "            if (Stage.DeferNow != 0) return PinStrs_%s(ctx, (ak_str*)run->S_%s, %s, n);" % (s.name, s.name, _get("_pinSrc", p, root, s.path))
            o += "            _fwd++;"
            o += "            return %s;" % _loop_forward(s, "run->S_%s" % s.name, "n", None)
        o += "        }"
        o += "        catch { return Abi.AK_ERR_HOST; }"
        o += "    }"
        o += ""
        for i in s.inner:
            o += "    %s" % UCO
            o += "    private static int Loop_%s_%s(IntPtr ctx, void* obj, long token)" % (s.name, i.name)
            o += "    {"
            o += "        _rev++;"
            o += "        try"
            o += "        {"
            o += "            var run = (Run_%s*)obj;" % root
            o += "            int e = (int)token;"
            o += "            int n = run->C_%s_%s[e];" % (s.name, i.name)
            o += "            if (n == 0) return 0;"
            if i.kind == "blob" and i.f.kind == "string":
                el = "%s[e]" % _get("_pinSrc", p, root, s.path)
                o += "            if (Stage.DeferNow != 0) return PinStrs_%s_%s(ctx, (ak_str*)run->I_%s_%s + run->O_%s_%s[e], %s, n);" % (
                    s.name, i.name, s.name, i.name, s.name, i.name, _get(el, p, s.et, i.path))
            elif i.kind == "map":
                el = "%s[e]" % _get("_pinSrc", p, root, s.path)
                o += "            if (Stage.DeferNow != 0) return PinMap_%s_%s(ctx, (%s*)run->I_%s_%s + run->O_%s_%s[e], %s, n);" % (
                    s.name, i.name, i.cs_e, s.name, i.name, s.name, i.name, _get(el, p, s.et, i.path))
            elif i.kind == "msg" and _pin_fields(p, i.et, "__e")[1]:
                el = "%s[e]" % _get("_pinSrc", p, root, s.path)
                o += "            if (Stage.DeferNow != 0) return PinElems_%s_%s(ctx, (%s*)run->I_%s_%s + run->O_%s_%s[e], %s, n);" % (
                    s.name, i.name, i.cs_e, s.name, i.name, s.name, i.name, _get(el, p, s.et, i.path))
            o += "            _fwd++;"
            o += "            return %s;" % _loop_forward(
                i, "((%s*)run->I_%s_%s + run->O_%s_%s[e])" % (i.cs_e, s.name, i.name, s.name, i.name), "n", None)
            o += "        }"
            o += "        catch { return Abi.AK_ERR_HOST; }"
            o += "    }"
            o += ""
    # ---------------- encode
    o += "    public int Fill(%s src) { Go(src, false, false, out _, out _); return 0; }" % root
    o += "    public void Encode(%s src, out byte* p, out int len) { int rc = Go(src, false, true, out p, out len); if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\"); }" % root
    if not _NO:
        o += "    public void EncodeU(%s src, out byte* p, out int len) { int rc = Go(src, true, true, out p, out len); if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\"); }" % root
    o += "    public int TryEncode(%s src, bool retain, out byte* p, out int len) => Go(src, retain, true, out p, out len);" % root
    o += "    /// ABI v1 section 9's move path (WP8): encode and leave the output IN the encode context,"
    o += "    /// no ak_enc_take, for ak_call_unary_enc / ak_call_send_enc to move as the request body."
    o += "    public int EncodeInto(%s src, bool retain)" % root
    o += "    {"
    o += "        _keep = true;"
    o += "        try { return Go(src, retain, true, out _, out _); }"
    o += "        finally { _keep = false; }"
    o += "    }"
    o += "    /// A copy of the bytes an EncodeInto left in the context (ak_enc_take reads them without"
    o += "    /// consuming them); for checks, never on a timed path."
    o += "    public byte[] ContextBytes()"
    o += "    {"
    o += "        byte* bp; nuint blen;"
    o += "        int tk = Abi.ak_enc_take(_ctx, &bp, &blen);"
    o += "        if (tk != 0) throw new InvalidOperationException($\"core encode failed: {tk}\");"
    o += "        return new ReadOnlySpan<byte>(bp, (int)blen).ToArray();"
    o += "    }"
    o += "    /// The encode context, for the move path's entries."
    o += "    public IntPtr EncContext => _ctx;"
    o += "    private bool _keep;"
    o += "    public byte[] EncodeToArray(%s src, bool retain = false)" % root
    o += "    {"
    o += "        int rc = Go(src, retain, true, out byte* p, out int len);"
    o += "        if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\");"
    o += "        var a = new byte[len];"
    o += "        new ReadOnlySpan<byte>(p, len).CopyTo(a);"
    o += "        return a;"
    o += "    }"
    o += ""
    _emit_root_pins(o, p, root, ds, _pin_fields(p, root, "src"))
    o += "    private int Go(%s src, bool retain, bool call, out byte* outPtr, out int outLen)" % root
    o += "    {"
    o += "        outPtr = null; outLen = 0;"
    o += "        Abi.ak_enc_reset(_ctx);"
    o += "        _st.Reset();"
    o += "        int __d = Stage.Defer;   // E1R / E1C: read once; the default path tests this local only"
    o += "        long __mk0 = 0, __pt0 = 0;"
    o += "        if (__d != 0) { __mk0 = Stage.Marked; __pt0 = Stage.Patched; }   // this encode's marks and patches"
    o += "        _run->Chunk = Chunk;"
    if _NO:
        o += "        if (retain) throw new NotSupportedException(\"unknown fields are compiled out of this build (WP5 step 10): no ak_uencode\");"
    else:
        o += "        _run->Retain = retain ? 1 : 0;"
    for s in slots:
        o += "        {"
        o += "            var lst = %s;" % _get("src", p, root, s.path)
        o += "            int n = lst == null ? 0 : lst.Count;"
        o += "            _run->N_%s = n;" % s.name
        o += "            Arr.Ensure(ref _run->S_%s, ref _cap_%s, n, %s);" % (s.name, s.name, _elem_size(s))
        if s.kind == "map":
            # plan ENCODE RULES: entries in ascending UTF-8 key order (WP5 step 6).
            o += "            var ord = n == 0 ? null : Arr.Utf8Order(lst);"
            o += "            for (int i = 0; i < n; i++) { var kv = lst.At(ord[i]); %s }" % _inline(_stage_elem, s, "_run->S_%s" % s.name, "i", "kv", "retain")
        else:
            o += "            for (int i = 0; i < n; i++)"
            o += "            {"
            _stage_elem(o, s, "_run->S_%s" % s.name, "i", "lst[i]", "                ", "retain")
            o += "            }"
        for i in s.inner:
            ig = _get("lst[i]", p, s.et, i.path)
            o += "            {"
            o += "                int tot = 0;"
            o += "                for (int i = 0; i < n; i++) { var il = %s; tot += il == null ? 0 : il.Count; }" % ig
            o += "                Arr.Ensure(ref _run->I_%s_%s, ref _cap_%s_%s, tot, %s);" % (s.name, i.name, s.name, i.name, _elem_size(i))
            o += "                { void* po = _run->O_%s_%s; int c0 = _capo_%s_%s; Arr.Ensure(ref po, ref c0, n, sizeof(int)); _run->O_%s_%s = (int*)po;" % (
                s.name, i.name, s.name, i.name, s.name, i.name)
            o += "                  void* pc = _run->C_%s_%s; int c1 = _capo_%s_%s; Arr.Ensure(ref pc, ref c1, n, sizeof(int)); _run->C_%s_%s = (int*)pc; _capo_%s_%s = c1; }" % (
                s.name, i.name, s.name, i.name, s.name, i.name, s.name, i.name)
            o += "                int bi = 0;"
            o += "                for (int i = 0; i < n; i++)"
            o += "                {"
            o += "                    var il = %s;" % ig
            o += "                    int m2 = il == null ? 0 : il.Count;"
            o += "                    _run->O_%s_%s[i] = bi; _run->C_%s_%s[i] = m2;" % (s.name, i.name, s.name, i.name)
            if i.kind == "map":
                o += "                    for (int j = 0; j < m2; j++) { var kv = il.At(j); %s bi++; }" % _inline(
                    _stage_elem, i, "_run->I_%s_%s" % (s.name, i.name), "bi", "kv", None)
            else:
                o += "                    for (int j = 0; j < m2; j++, bi++)"
                o += "                    {"
                _stage_elem(o, i, "_run->I_%s_%s" % (s.name, i.name), "bi", "il[j]", "                        ", None)
                o += "                    }"
            o += "                }"
            o += "            }"
        o += "        }"
    o += "        var vt = new ak_evt_%s" % root
    o += "        {"
    if not slots:
        o += "            _reserved = IntPtr.Zero,"
    for s in slots:
        o += "            loop_%s = &Loop_%s," % (s.name, s.name)
        if s.has_evt:
            o += "            elem_%s = _evt_%s," % (s.name, s.name)
    o += "        };"
    dargs = ""
    if ds:
        dpath = ds[0][0]
        o += "        // ABI v1 section 8: the one direct-argument field of this tree, pinned for the call."
        o += "        byte[] direct = %s ?? Array.Empty<byte>();" % _get("src", p, root, dpath)
        dargs = ", dp, (nuint)direct.Length"
    o += "        nint rc;"
    rpins = _pin_fields(p, root, "src")
    if _NO:
        o += "        {"
        o += "            var fix = new ak_efix_%s();" % root
        o += "            G.E_%s(ref fix, src, _st);" % root
        o += "            if (!call) return 0;"
        o += "            _fwd++;"
        _root_call(o, "            ", ("fixed (byte* dp = direct) rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix%s);" % (root, dargs)) if ds
                   else ("rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix);" % root), rpins, "e", ds)
        o += "        }"
    else:
        o += "        if (retain)"
        o += "        {"
        o += "            var fix = new ak_ufix_%s();" % root
        o += "            G.U_%s(ref fix, src, _st);" % root
        o += "            if (!call) return 0;"
        o += "            _fwd++;"
        _root_call(o, "            ", ("fixed (byte* dp = direct) rc = Abi.ak_uencode_%s(_run, _ctx, &vt, &fix%s);" % (root, dargs)) if ds
                   else ("rc = Abi.ak_uencode_%s(_run, _ctx, &vt, &fix);" % root), rpins, "u", ds)
        o += "        }"
        o += "        else"
        o += "        {"
        o += "            var fix = new ak_efix_%s();" % root
        o += "            G.E_%s(ref fix, src, _st);" % root
        o += "            if (!call) return 0;"
        o += "            _fwd++;"
        _root_call(o, "            ", ("fixed (byte* dp = direct) rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix%s);" % (root, dargs)) if ds
                   else ("rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix);" % root), rpins, "e", ds)
        o += "        }"
    o += "        _st.ReleasePins();   // D21 E1: the core has copied every pinned string"
    o += "        if (__d != 0)"
    o += "        {"
    o += "            Stage.DeferNow = 0;   // the next encode on this thread starts from the default"
    o += "            _pinSrc = null;"
    o += "            // E1R / E1C: every mark the fill left was patched by a frame before the core read it."
    o += "            if (Stage.Marked - __mk0 != Stage.Patched - __pt0 && rc >= 0) rc = Abi.AK_ERR_HOST;"
    o += "        }"
    o += "        if (rc < 0) return (int)rc;"
    o += "        if (_keep) return 0;   // EncodeInto: the output stays in the context (the move path)"
    o += "        byte* bp; nuint blen;"
    o += "        int tk = Abi.ak_enc_take(_ctx, &bp, &blen);"
    o += "        if (tk != 0) return tk;"
    o += "        outPtr = bp; outLen = (int)blen;"
    o += "        return 0;"
    o += "    }"
    o += ""
    _emit_decode(o, p, root, slots)
    _emit_pull(o, p, root, slots)
    # ---------------- tail
    o += "    public long PullFootprint() => _dctx == IntPtr.Zero ? 0 : (long)Abi.ak_bdr_footprint(_dctx);"
    o += "    public AkCounters EncCounters() { AkCounters c; Abi.ak_enc_counters(_ctx, &c); return c; }"
    o += "    public void EncCountersReset() => Abi.ak_enc_counters_reset(_ctx);"
    o += "    public AkCounters DecCounters() { AkCounters c; if (_dctx == IntPtr.Zero) return default; Abi.ak_dec_counters(_dctx, &c); return c; }"
    o += "    public void DecCountersReset() { if (_dctx != IntPtr.Zero) Abi.ak_dec_counters_reset(_dctx); }"
    o += ""
    o += "    public void Dispose()"
    o += "    {"
    o += "        if (_ctx != IntPtr.Zero) { Abi.ak_enc_ctx_free(_ctx); _ctx = IntPtr.Zero; }"
    o += "        if (_dctx != IntPtr.Zero) { Abi.ak_dec_ctx_free(_dctx); _dctx = IntPtr.Zero; }"
    o += "        if (_th.IsAllocated) _th.Free();"
    if not _NO:
        o += "        _arena?.Dispose();"
    o += "        _st.Dispose();"
    o += "        if (_run != null)"
    o += "        {"
    for s in slots:
        o += "            NativeMemory.Free(_run->S_%s);" % s.name
        for i in s.inner:
            o += "            NativeMemory.Free(_run->I_%s_%s); NativeMemory.Free(_run->O_%s_%s); NativeMemory.Free(_run->C_%s_%s);" % (
                s.name, i.name, s.name, i.name, s.name, i.name)
    o += "            NativeMemory.Free(_run); _run = null;"
    o += "        }"
    for s in slots:
        if s.has_evt:
            o += "        if (_evt_%s != null) { NativeMemory.Free(_evt_%s); _evt_%s = null; }" % (s.name, s.name, s.name)
    o += "        if (_drun != null) { NativeMemory.Free(_drun); _drun = null; }"
    if not _NO:
        o += "        if (_uo != null) { NativeMemory.Free(_uo); _uo = null; }"
    o += "    }"
    o += "}"
    o += ""


def _inline(fn, *args):
    """Render a one-element staging statement on one line."""
    class L(list):
        def __iadd__(self, x):
            self.append(x.strip())
            return self
    buf = L()
    fn(buf, *args[:4], "", args[4])
    return " ".join(buf)


def _emit_decode(o, p, root, slots):
    o += "    [StructLayout(LayoutKind.Sequential)]"
    o += "    private struct DecRun { public IntPtr Target; public byte* Buf; }"
    o += ""
    o += "    private static %s Tgt(void* obj) => (%s)GCHandle.FromIntPtr(((DecRun*)obj)->Target).Target;" % (root, root)
    o += ""
    o += "    %s" % UCO
    o += "    private static void ApplyRoot(IntPtr ctx, void* obj, ak_dfix_%s* fix)" % root
    o += "    {"
    o += "        _rev++;"
    o += "        try { if (G.PlantHostFail == 1) throw new InvalidOperationException(\"planted host failure (apply)\"); G.D_%s(ref *fix, Tgt(obj), ((DecRun*)obj)->Buf); }" % root
    o += "        catch (DecoderFallbackException) { Abi.ak_fail(ctx, Abi.AK_ERR_TRANSCODE, null, 0); } catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
    o += "    }"
    o += ""
    for si, s in enumerate(slots, 1):
        lst = _make("Tgt(obj)", p, root, s.path)
        if s.leaf:
            o += "    %s" % UCO
            o += "    private static void Add_%s(IntPtr ctx, void* obj, long token, %s* xs, int n)" % (s.name, s.cs_d)
            o += "    {"
            o += "        _rev++;"
            o += "        try"
            o += "        {"
            o += "            if (G.PlantHostFail == 2) throw new InvalidOperationException(\"planted host failure (add)\");"
            o += "            byte* b = ((DecRun*)obj)->Buf;"
            o += "            var lst = %s;" % lst
            _add_body(o, s, "lst", "xs", "n", "            ")
            o += "        }"
            o += "        catch (DecoderFallbackException) { Abi.ak_fail(ctx, Abi.AK_ERR_TRANSCODE, null, 0); } catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
            o += "    }"
            o += ""
            continue
        o += "    %s" % UCO
        o += "    private static long New_%s(IntPtr ctx, void* obj)" % s.name
        o += "    {"
        o += "        _rev++;"
        o += "        try { if (G.PlantHostFail == 2) throw new InvalidOperationException(\"planted host failure (new)\"); var lst = %s; lst.Add(new %s()); return lst.Count - 1; }" % (lst, s.et)
        o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); return -1; }"
        o += "    }"
        o += ""
        o += "    %s" % UCO
        o += "    private static void Apply_%s(IntPtr ctx, void* obj, long token, ak_dfix_%s* fix)" % (s.name, s.et)
        o += "    {"
        o += "        _rev++;"
        o += "        try { var lst = %s; G.D_%s(ref *fix, lst[(int)token], ((DecRun*)obj)->Buf); }" % (lst, s.et)
        o += "        catch (DecoderFallbackException) { Abi.ak_fail(ctx, Abi.AK_ERR_TRANSCODE, null, 0); } catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
        o += "    }"
        o += ""
        for i in s.inner:
            o += "    %s" % UCO
            o += "    private static void Add_%s_%s(IntPtr ctx, void* obj, long token, %s* xs, int n)" % (s.name, i.name, i.cs_d)
            o += "    {"
            o += "        _rev++;"
            o += "        try"
            o += "        {"
            o += "            byte* b = ((DecRun*)obj)->Buf;"
            o += "            var e = %s[(int)token];" % lst
            o += "            var il = %s;" % _make("e", p, s.et, i.path)
            _add_body(o, i, "il", "xs", "n", "            ", "k")
            o += "        }"
            o += "        catch (DecoderFallbackException) { Abi.ak_fail(ctx, Abi.AK_ERR_TRANSCODE, null, 0); } catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
            o += "    }"
            o += ""
    o += "    private static readonly byte[] One = new byte[1];"
    o += ""
    _emit_unk(o, p, root)
    o += "    public %s Decode(byte[] src, int len) { int rc = TryDecode(src, len, false, out var t); if (rc < 0) throw new InvalidOperationException($\"core decode failed: {rc}\"); return t; }" % root
    if not _NO:
        o += "    public %s DecodeU(byte[] src, int len) { int rc = TryDecode(src, len, true, out var t); if (rc < 0) throw new InvalidOperationException($\"core decode failed: {rc}\"); return t; }" % root
    o += ""
    o += "    /// The core's code (< 0) on failure; the output is then unspecified and discarded (R-G6)."
    o += "    public int TryDecode(byte[] src, int len, bool retain, out %s result) => DecodeArmed(src, len, retain ? -1 : -2, out result);" % root
    o += ""
    if not _NO:
        o += "    /// A control (decision 11 DISCARD): retain everywhere except `position` (an index into"
        o += "    /// UnkPositionNames), whose entry is all zero when armed."
        o += "    public int TryDecodeZeroing(byte[] src, int len, int position, out %s result) => DecodeArmed(src, len, position, out result);" % root
        o += ""
    o += "    /// Step 5b (owner, 2026-10-04): the push vtable, built ONCE per root and kept in native"
    o += "    /// memory (NativeMemory.Alloc, never freed: one per root for the process), so no decode"
    o += "    /// writes it and the GC cannot move it. Not a static field of the struct type taken by"
    o += "    /// address: a non-primitive struct static lives in a boxed object on the GC heap, which"
    o += "    /// compaction may move, so its address is not stable without a pin. The core only reads"
    o += "    /// the vtable during the call."
    o += "    private static readonly ak_dvt_%s* Vt = MakeVt();" % root
    o += "    private static ak_dvt_%s* MakeVt()" % root
    o += "    {"
    o += "        var v = (ak_dvt_%s*)NativeMemory.AllocZeroed((nuint)sizeof(ak_dvt_%s));" % (root, root)
    o += "        v->utf8_skip = AkUtf8Skip.%s_ALL;   // D20: every bit set, G.Str validates (strict)" % root
    o += "        v->apply = &ApplyRoot;"
    for s_ in slots:
        if s_.leaf:
            o += "        v->add_%s = &Add_%s;" % (s_.name, s_.name)
        else:
            o += "        v->new_%s = &New_%s;" % (s_.name, s_.name)
            o += "        v->apply_%s = &Apply_%s;" % (s_.name, s_.name)
            for i_ in s_.inner:
                o += "        v->add_%s_%s = &Add_%s_%s;" % (s_.name, i_.name, s_.name, i_.name)
    o += "        return v;"
    o += "    }"
    o += "    /// Step 5b: the pull family's bits, once per root in native memory (the setter copies them)."
    o += "    private static readonly ak_pvt_%s* Pvt = MakePvt();" % root
    o += "    private static ak_pvt_%s* MakePvt() { var v = (ak_pvt_%s*)NativeMemory.AllocZeroed((nuint)sizeof(ak_pvt_%s)); v->utf8_skip = AkUtf8Skip.%s_ALL; return v; }" % (root, root, root, root)
    o += "    private GCHandle _th;"
    o += "    private int DecodeArmed(byte[] src, int len, int mode, out %s result)" % root
    o += "    {"
    o += "        result = null;"
    o += "        EnsureDec();"
    o += "        // Step a2 (i): no ak_dec_err_reset / ak_dec_err: ak_decode_* clears the context's"
    o += "        // sticky slot on entry and returns it (a host failure reported through ak_fail"
    o += "        // from a reverse call included), so its return value carries every error."
    o += "        int ar = ArmFor(mode);"
    o += "        if (ar != 0) { Disarm(ar); return ar; }"
    o += "        var target = new %s();" % root
    o += "        // Step a2 (ii): one GCHandle per instance, its Target set for this decode."
    o += "        if (!_th.IsAllocated) _th = GCHandle.Alloc(null);"
    o += "        _th.Target = target;"
    o += "        int rc = Abi.AK_ERR_HOST;"
    o += "        try"
    o += "        {"
    o += "            fixed (byte* b0 = src)"
    o += "            fixed (byte* one = One)"
    o += "            {"
    o += "                // (NULL, 0) is never handed to the core: an empty buffer is a valid pointer and 0."
    o += "                byte* b = len == 0 ? one : b0;"
    o += "                _drun->Target = GCHandle.ToIntPtr(_th);"
    o += "                _drun->Buf = b;"
    o += "                _fwd++;"
    o += "                rc = Abi.ak_decode_%s(_dctx, _drun, b, (nuint)len, Vt);" % root
    o += "            }"
    o += "        }"
    o += "        finally { _th.Target = null; rc = Disarm(rc); }"
    o += "        if (rc < 0) return rc;"
    o += "        result = target;"
    o += "        return 0;"
    o += "    }"
    o += ""


def _clear(p, mname, path, x, ind, o, depth=0):
    """C# statements clearing the facade bags at position `path` below facade object `x`
    (of message `mname`): the zeroed-position control's expectation."""
    from plan import _unk_child
    if not path:
        o += "%s%s.%s = null;" % (ind, x, BAG)
        return
    k = path[0]
    m = p.msg(mname)
    f = next(g for g in m.fields if (g.oneof or g.name) == k and _unk_child(g) is not None)
    if f.oneof:
        for gm in m.oneofs[f.oneof]:
            if gm.kind == "message":
                o += "%sif (%s.%s != null) %s.%s.%s = null;" % (ind, x, N.field(gm.name), x, N.field(gm.name), BAG)
        return
    acc = "%s.%s" % (x, N.field(f.name))
    if f.card == "map":
        o += "%s// a map entry: the facade map has no bag (U-map-entry), nothing to clear" % ind
        return
    v = "e%d" % depth
    if f.card == "repeated":
        o += "%sif (%s != null) foreach (var %s in %s)" % (ind, acc, v, acc)
    else:
        o += "%sif (%s != null) { var %s = %s;" % (ind, acc, v, acc)
    o += "%s{" % ind if f.card == "repeated" else ""
    _clear(p, _unk_child(f), path[1:], v, ind + "    ", o, depth + 1)
    o += "%s}" % ind


def _emit_unk_nounk(o, p, root):
    """The NO-UNKNOWN variant: a root-bound context with no options, nothing to arm."""
    o += "    /// WP5 step 10: this build has unknown fields COMPILED OUT (no options, no resets)."
    o += "    public const bool UnknownCompiledOut = true;"
    o += "    public const int UNDELIVERED = -1001;   // never returned by this build"
    o += "    public int Undelivered => 0;"
    o += "    public long ResetCalls => 0;"
    o += ""
    o += "    private void EnsureDec()"
    o += "    {"
    o += "        if (_dctx != IntPtr.Zero) return;"
    o += "        _dctx = Abi.ak_dec_ctx_new_%s();   // rule 6: bound to this root; no options exist" % root
    o += "        if (_dctx == IntPtr.Zero) throw new InvalidOperationException(\"ak_dec_ctx_new_%s returned NULL\");" % root
    o += "        // D20: the pull family's bits, copied into this root-bound context (every bit: G.Str validates)."
    o += "        int sp = Abi.ak_dec_set_pvt_%s(_dctx, Pvt);   // step 5b: the root's one native pvt" % root
    o += "        if (sp != 0) throw new InvalidOperationException(\"ak_dec_set_pvt_%s: \" + sp);" % root
    o += "        _drun = (DecRun*)NativeMemory.AllocZeroed((nuint)sizeof(DecRun));"
    o += "    }"
    o += ""
    o += "    private int ArmFor(int mode) => mode == -2 ? 0 : throw new NotSupportedException(\"unknown fields are compiled out of this build (WP5 step 10)\");"
    o += "    private int Disarm(int rc) => rc;"
    o += ""


def _emit_unk(o, p, root):
    """Decision 11 for this root: the options (native, unmoved while armed), arming and
    disarming around one decode, and the controls' position helpers."""
    if _NO:
        _emit_unk_nounk(o, p, root)
        return
    from plan import unk_opts_layout, unk_opts_name, unk_positions
    lay = unk_opts_layout(p, root)
    pos = unk_positions(p, root)
    on = unk_opts_name(root)
    o.doc("Decision 11: every message position of this root, in plan.unk_positions order (the "
          "options' member order).", "    ")
    o += "    public static readonly string[] UnkPositionNames = { %s };" % ", ".join('"%s"' % n for n, _m, _t in lay)
    o += ""
    o += "    /// A retained decode ended with a buffer grow handed out that no delivered group"
    o += "    /// carried back to the host: a host defect, never a core code."
    o += "    public const int UNDELIVERED = -1001;"
    o += "    public int Undelivered { get; private set; }"
    o += "    private %s* _uo;" % on
    o += "    private UnkArena _arena;"
    o += "    private int _armed = int.MinValue;   // the `zero` the options at _uo were written for"
    o += "    /// The one reset that arms each decode (rule 7): a forward crossing, counted"
    o += "    /// apart from ForwardCalls because the core's R5 counters do not count them."
    o += "    private static long _resets;"
    o += "    public long ResetCalls => _resets;"
    o += "    public const bool UnknownCompiledOut = false;"
    o += ""
    o += "    private void EnsureDec()"
    o += "    {"
    o += "        if (_dctx != IntPtr.Zero) return;"
    o += "        _dctx = Abi.ak_dec_ctx_new_%s(null);   // rule 6: bound to this root, drop mode" % root
    o += "        if (_dctx == IntPtr.Zero) throw new InvalidOperationException(\"ak_dec_ctx_new_%s returned NULL\");" % root
    o += "        // D20: the pull family's bits, copied into this root-bound context (every bit: G.Str validates)."
    o += "        int sp = Abi.ak_dec_set_pvt_%s(_dctx, Pvt);   // step 5b: the root's one native pvt" % root
    o += "        if (sp != 0) throw new InvalidOperationException(\"ak_dec_set_pvt_%s: \" + sp);" % root
    o += "        _drun = (DecRun*)NativeMemory.AllocZeroed((nuint)sizeof(DecRun));"
    o += "    }"
    o += ""
    o += "    /// The options, rewritten before every retained decode: every entry names the one"
    o += "    /// grow and holds no pre-allocated buffer (all from grow, so the core never needs"
    o += "    /// a refill); `zero` >= 0 leaves that position's entry all zero (DISCARD)."
    o += "    private %s* Arm(int zero)" % on
    o += "    {"
    o += "        // Step a2 (iii): rewritten only when the mode changed. The core never writes a"
    o += "        // grow-only entry (it takes and clears pre-placed buffers only, and there are none)."
    o += "        if (_uo != null && _armed == zero) return _uo;"
    o += "        if (_uo == null) _uo = (%s*)NativeMemory.AllocZeroed((nuint)sizeof(%s));" % (on, on)
    o += "        *_uo = default;"
    o += "        _armed = zero;"
    o += "        var g = UnkHost.Fn;"
    for i, (n, _m, _t) in enumerate(lay):
        o += "        if (zero != %d) _uo->%s.grow = g;" % (i, n)
    o += "        return _uo;"
    o += "    }"
    o += ""
    o += "    /// mode -2: drop (reset with NULL); -1: retain everywhere; k >= 0: retain but k."
    o += "    private int ArmFor(int mode)"
    o += "    {"
    o += "        Undelivered = 0;"
    o += "        _resets++;"
    o += "        if (mode == -2) { G.Arena = null; return Abi.ak_dec_reset_%s(_dctx, null); }" % root
    o += "        _arena ??= new UnkArena();"
    o += "        _arena.Begin();"
    o += "        G.Arena = _arena;"
    o += "        return Abi.ak_dec_reset_%s(_dctx, Arm(mode));" % root
    o += "    }"
    o += ""
    o += "    /// After every decode: a buffer still outstanding (the arena's count) after a success is"
    o += "    /// UNDELIVERED. No reset here: decision 11 rule 7 (as amended 2026-09-26) takes ONE"
    o += "    /// reset per decode, the arming one before it (ArmFor); the options stay at their"
    o += "    /// stable native address (_uo) until the next decode's reset rewrites them."
    o += "    private int Disarm(int rc)"
    o += "    {"
    o += "        var a = G.Arena;"
    o += "        G.Arena = null;"
    o += "        if (a == null || a.Outstanding == 0) return rc;"
    o += "        int left = a.Outstanding;   // the memory stays the arena's, rewound by the next Begin"
    o += "        if (rc < 0) return rc;"
    o += "        Undelivered = left;"
    o += "        return UNDELIVERED;"
    o += "    }"
    o += ""
    o += "    /// The zeroed-position control's expectation: the facade bags at `position` cleared."
    o += "    public static void ClearPosition(%s t, int position)" % root
    o += "    {"
    o += "        switch (position)"
    o += "        {"
    for i, (path, _m) in enumerate(pos):
        o += "            case %d:" % i
        o += "            {"
        _clear(p, root, path, "t", "                ", o)
        o += "                break;"
        o += "            }"
    o += "            default: throw new ArgumentOutOfRangeException(nameof(position));"
    o += "        }"
    o += "    }"
    o += ""


def _emit_pull(o, p, root, slots):
    o.doc("ABI v1 7.1's PULL family: parse into the context's record stream (no reverse call "
          "but grow), then replay it with the same group readers the push callbacks use. The "
          "context is armed exactly as for push.", "    ")
    o += "    public %s Pull(byte[] src, int len) { int rc = TryPull(src, len, false, out var t); if (rc < 0) throw new InvalidOperationException($\"core parse failed: {rc}\"); return t; }" % root
    if not _NO:
        o += "    public %s PullU(byte[] src, int len) { int rc = TryPull(src, len, true, out var t); if (rc < 0) throw new InvalidOperationException($\"core parse failed: {rc}\"); return t; }" % root
    o += ""
    o += "    public int TryPull(byte[] src, int len, bool retain, out %s result)" % root
    o += "    {"
    o += "        result = null;"
    o += "        EnsureDec();"
    o += "        int ar = ArmFor(retain ? -1 : -2);"
    o += "        if (ar != 0) { Disarm(ar); return ar; }"
    o += "        var t = new %s();" % root
    o += "        int rc = Abi.AK_ERR_HOST;"
    o += "        try"
    o += "        {"
    o += "            fixed (byte* b0 = src)"
    o += "            fixed (byte* one = One)"
    o += "            {"
    o += "                byte* b = len == 0 ? one : b0;"
    o += "                _fwd++;"
    o += "                rc = Abi.ak_parse_%s(_dctx, b, (nuint)len);" % root
    o += "                if (rc >= 0)"
    o += "                {"
    o += "                    byte* recs; nuint rlen;"
    o += "                    _fwd++;"
    o += "                    rc = Abi.ak_bdr_ptr(_dctx, &recs, &rlen);"
    o += "                    if (rc == 0) { try { Replay(t, b, recs, (int)rlen); } catch (DecoderFallbackException) { rc = Abi.AK_ERR_TRANSCODE; } }"
    o += "                }"
    o += "            }"
    o += "        }"
    o += "        finally { rc = Disarm(rc); }"
    o += "        if (rc < 0) return rc;"
    o += "        result = t;"
    o += "        return 0;"
    o += "    }"
    o += ""
    o += "    private const uint OP_APPLY = 1, OP_ADD = 2, OP_NEW = 3, OP_APPLY_ELEM = 4;   // ak_bdr_rec.op"
    o += "    private static void Replay(%s t, byte* b, byte* p, int len)" % root
    o += "    {"
    o += "        int at = 0;"
    o += "        while (len - at >= sizeof(ak_bdr_rec))"
    o += "        {"
    o += "            ref var r = ref *(ak_bdr_rec*)(p + at);"
    o += "            byte* body = p + at + sizeof(ak_bdr_rec);"
    o += "            at += sizeof(ak_bdr_rec) + (int)r.bytes;"
    o += "            uint outer = r.slot >> 16, inner = r.slot & 0xFFFF;"
    o += "            switch (r.op)"
    o += "            {"
    o += "                case OP_APPLY: G.D_%s(ref *(ak_dfix_%s*)body, t, b); break;" % (root, root)
    for si, s in enumerate(slots, 1):
        lst = _make("t", p, root, s.path)
        if s.leaf:
            o += "                case OP_ADD when outer == 0 && inner == %d:   // a root-level run" % si
            o += "                {"
            o += "                    var xs = (%s*)body; var lst = %s;" % (s.cs_d, lst)
            _add_body(o, s, "lst", "xs", "(int)r.n", "                    ")
            o += "                    break;"
            o += "                }"
        else:
            o += "                case OP_NEW when outer == %d: %s.Add(new %s()); break;" % (si, lst, s.et)
            o += "                case OP_APPLY_ELEM when outer == %d: G.D_%s(ref *(ak_dfix_%s*)body, %s[(int)r.token], b); break;" % (
                si, s.et, s.et, lst)
            for ii, i in enumerate(s.inner, 1):
                o += "                case OP_ADD when outer == %d && inner == %d:" % (si, ii)
                o += "                {"
                o += "                    var xs = (%s*)body; var e = %s[(int)r.token]; var il = %s;" % (
                    i.cs_d, lst, _make("e", p, s.et, i.path))
                _add_body(o, i, "il", "xs", "(int)r.n", "                    ", "k")
                o += "                    break;"
                o += "                }"
    o += "                default: break;"
    o += "            }"
    o += "        }"
    o += "    }"
    o += ""


def emit_host(x, ns, facade_ns):
    global _NO
    p = as_plan(x)
    _NO = unknown_compiled_out(p)
    o = N.Head("The core-ffi host binding for every root of this message set.", p.source, "cs_host")
    o += "#if NET5_0_OR_GREATER"
    o += "using System;"
    o += "using System.Collections.Generic;"
    o += "using System.Runtime.CompilerServices;"
    o += "using System.Runtime.InteropServices;"
    o += "using System.Text;"
    o += "using Armonik.Ffi.Facade;"
    if facade_ns != "Armonik.Ffi.Facade":
        o += "using %s;" % facade_ns
    o += ""
    o += "namespace %s;" % ns
    stage = STAGE if not _NO else STAGE[:STAGE.index("    /// The unknown-field bag: raw runs")] + STAGE[STAGE.index("    public void Dispose()"):]
    for ln in stage.strip("\n").split("\n"):
        o += ln
    if not _NO:
        for ln in UNKHOST.strip("\n").split("\n"):
            o += ln
    o += ""
    _emit_groups(o, p)
    for root in p.roots:
        _emit_root(o, p, root, facade_ns)
    o += "#endif"
    return str(o)
