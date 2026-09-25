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
"""
from plan import (as_plan, direct_fields, elem_type, loop_slots, presence_bits, slot_elem,
                  slot_name)
import cs_names as N
from cs_types import BAG

RUN_FN = {"i32": "ak_run_i32", "i64": "ak_run_i64", "f64": "ak_run_f64", "u8": "ak_run_u8"}
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
    o += ""
    o += "    [MethodImpl(MethodImplOptions.AggressiveInlining)]"
    o += '    internal static string Str(byte* b, ak_span s) => s.len == 0 || SkipStrings ? "" : Encoding.UTF8.GetString(b + s.off, (int)s.len);'
    o += ""
    o += "    /// Decision 11: the buffers a RETAINED decode has been handed by grow and not yet taken"
    o += "    /// back (null in drop mode). Thread-static: the core calls grow on the decoding thread."
    o += "    [ThreadStatic] internal static HashSet<IntPtr> Live;"
    o += ""
    o += "    /// A delivered message's buffer into its facade bag (null when none or empty); the"
    o += "    /// native buffer is freed and the slot cleared."
    o += "    internal static byte[] Take(ref ak_unk_buf u)"
    o += "    {"
    o += "        if (u.data == IntPtr.Zero) return null;"
    o += "        byte[] r = null;"
    o += "        if (u.len != 0) { r = new byte[u.len]; new ReadOnlySpan<byte>((void*)u.data, (int)u.len).CopyTo(r); }"
    o += "        Live?.Remove(u.data);"
    o += "        NativeMemory.Free((void*)u.data);"
    o += "        u = default;"
    o += "        return r;"
    o += "    }"
    o += ""
    o += "    /// A non-NULL slot the facade has no place for (inactive, absent, a map entry): freed."
    o += "    internal static void Drop(ref ak_unk_buf u)"
    o += "    {"
    o += "        if (u.data == IntPtr.Zero) return;"
    o += "        Live?.Remove(u.data);"
    o += "        NativeMemory.Free((void*)u.data);"
    o += "        u = default;"
    o += "    }"
    o += ""
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
        for u in (False, True):
            _emit_fill(o, p, m, u)
        _emit_unfill(o, p, m)
        _emit_free(o, p, m)
    o += "}"
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
            if gm.kind == "message":
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


STAGE = r'''
/// Staging for strings, bytes and unknown-field bags handed to the core as data. Blocks
/// are allocated as needed and never moved, so a pointer handed out stays valid until
/// Reset; the first block is kept across calls.
public sealed unsafe class Stage : IDisposable
{
    private readonly System.Collections.Generic.List<IntPtr> _blocks = new System.Collections.Generic.List<IntPtr>();
    private byte* _cur;
    private int _cap, _at;
    public readonly bool Utf16;
    public readonly IntPtr Tc, TcBytes;

    public Stage(bool utf16)
    {
        Utf16 = utf16;
        Tc = utf16 ? Abi.ak_tc_utf16() : Abi.ak_tc_bytes();
        TcBytes = Abi.ak_tc_bytes();
        NewBlock(1 << 16);
    }

    private void NewBlock(int n)
    {
        _cur = (byte*)NativeMemory.Alloc((nuint)n);
        _blocks.Add((IntPtr)_cur);
        _cap = n;
        _at = 0;
    }

    public void Reset()
    {
        for (int i = 1; i < _blocks.Count; i++) NativeMemory.Free((void*)_blocks[i]);
        if (_blocks.Count > 1) _blocks.RemoveRange(1, _blocks.Count - 1);
        _cur = (byte*)_blocks[0];
        _cap = 1 << 16;
        _at = 0;
    }

    private byte* Take(int n)
    {
        if (_cap - _at < n) NewBlock(Math.Max(1 << 16, n));
        byte* p = _cur + _at;
        _at += (n + 7) & ~7;
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
        if (Utf16)
        {
            // `len` counts CODE UNITS: ak_tc_utf16 reads `*const u16`.
            byte* p = Take(s.Length * 2);
            fixed (char* c = s) Buffer.MemoryCopy(c, p, s.Length * 2, s.Length * 2);
            return new ak_str { data = (IntPtr)p, len = (nuint)s.Length, tc = Tc };
        }
        int max = Encoding.UTF8.GetMaxByteCount(s.Length);
        byte* q = Take(max);
        int n;
        fixed (char* c = s) n = Encoding.UTF8.GetBytes(c, s.Length, q, max);
        return new ak_str { data = (IntPtr)q, len = (nuint)n, tc = Tc };
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
    }
}

/// Decision 11: the one grow callback every position of every root's options names
/// (`ak_grow_fn`, i32 sizes). NativeMemory.Realloc: `*dst` NULL with `*cap` 0 is a fresh
/// buffer, otherwise the first `*cap` bytes are preserved (realloc semantics; it may move).
/// A retained decode tracks what it hands out (`G.Live`), so nothing leaks on failure.
public static unsafe class UnkHost
{
    public static long Grows;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int Grow(IntPtr sink, int want, byte** dst, int* cap)
    {
        try
        {
            if (want < 0) return Abi.AK_ERR_LIMIT;
            int c = *cap;
            long nc = Math.Max((long)want, Math.Max(64L, 2L * c));
            if (nc > int.MaxValue) nc = want;
            void* old = *dst;
            void* np = NativeMemory.Realloc(old, (nuint)nc);
            var live = G.Live;
            if (live != null) { if (old != null) live.Remove((IntPtr)old); live.Add((IntPtr)np); }
            *dst = (byte*)np;
            *cap = (int)nc;
            Grows++;
            return 0;
        }
        catch { return Abi.AK_ERR_HOST; }
    }

    public static IntPtr Fn => (IntPtr)(delegate* unmanaged[Cdecl]<IntPtr, int, byte**, int*, int>)&Grow;
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
        o += "%s{ ref var e = ref ((%s*)%s)[%s]; e = default; e.key = _st.Str(%s.Key); e.value = _st.Str(%s.Value); }" % (
            ind, s.cs_e, arr, idx, src, src)
    else:
        if retain_expr and s.top:
            o += "%sif (%s) G.U_%s(ref ((%s*)%s)[%s], %s, _st);" % (ind, retain_expr, s.et, s.u_elem(), arr, idx, src)
            o += "%selse G.E_%s(ref ((%s*)%s)[%s], %s, _st);" % (ind, s.et, s.cs_e, arr, idx, src)
        else:
            o += "%sG.E_%s(ref ((%s*)%s)[%s], %s, _st);" % (ind, s.et, s.cs_e, arr, idx, src)


def _elem_size(s):
    if s.kind == "msg" and s.top:
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
        o += "%s// plan: a duplicate key replaces the earlier value. The facade map has no bag:" % ind
        o += "%s// an entry's unknown-field buffer (decision 11) is freed (U-map-entry)." % ind
        o += "%sfor (int %s = 0; %s < %s; %s++) { %s[G.Str(b, %s[%s].key)] = G.Str(b, %s[%s].value); G.Drop(ref %s[%s].unknown); }" % (
            ind, var, var, n, var, lst, xs, var, xs, var, xs, var)
    else:
        o += "%sfor (int %s = 0; %s < %s; %s++) { var x = new %s(); G.D_%s(ref %s[%s], x, b); %s.Add(x); }" % (
            ind, var, var, n, var, s.et, s.et, xs, var, lst)


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
    o += "    public void CallsReset() { _fwd = 0; _rev = 0; _resets = 0; }"
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
    for s in slots:
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
            o += "            for (int off = 0; off < n; off += chunk)"
            o += "            {"
            o += "                int k = n - off; if (k > chunk) k = chunk;"
            o += "                _fwd++;"
            o += "                int rc = run->Retain != 0"
            o += "                    ? %s" % _loop_forward_u(s, "((%s*)run->S_%s + off)" % (s.u_elem(), s.name), "k", "off")
            o += "                    : %s;" % _loop_forward(s, "((%s*)run->S_%s + off)" % (s.cs_e, s.name), "k", "off")
            o += "                if (rc < 0) return rc;"
            o += "            }"
            o += "            return 0;"
        else:
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
    o += "    public void EncodeU(%s src, out byte* p, out int len) { int rc = Go(src, true, true, out p, out len); if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\"); }" % root
    o += "    public int TryEncode(%s src, bool retain, out byte* p, out int len) => Go(src, retain, true, out p, out len);" % root
    o += "    public byte[] EncodeToArray(%s src, bool retain = false)" % root
    o += "    {"
    o += "        int rc = Go(src, retain, true, out byte* p, out int len);"
    o += "        if (rc < 0) throw new InvalidOperationException($\"core encode failed: {rc}\");"
    o += "        var a = new byte[len];"
    o += "        new ReadOnlySpan<byte>(p, len).CopyTo(a);"
    o += "        return a;"
    o += "    }"
    o += ""
    o += "    private int Go(%s src, bool retain, bool call, out byte* outPtr, out int outLen)" % root
    o += "    {"
    o += "        outPtr = null; outLen = 0;"
    o += "        Abi.ak_enc_reset(_ctx);"
    o += "        _st.Reset();"
    o += "        _run->Chunk = Chunk;"
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
    o += "        if (retain)"
    o += "        {"
    o += "            var fix = new ak_ufix_%s();" % root
    o += "            G.U_%s(ref fix, src, _st);" % root
    o += "            if (!call) return 0;"
    o += "            _fwd++;"
    if ds:
        o += "            fixed (byte* dp = direct) rc = Abi.ak_uencode_%s(_run, _ctx, &vt, &fix%s);" % (root, dargs)
    else:
        o += "            rc = Abi.ak_uencode_%s(_run, _ctx, &vt, &fix);" % root
    o += "        }"
    o += "        else"
    o += "        {"
    o += "            var fix = new ak_efix_%s();" % root
    o += "            G.E_%s(ref fix, src, _st);" % root
    o += "            if (!call) return 0;"
    o += "            _fwd++;"
    if ds:
        o += "            fixed (byte* dp = direct) rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix%s);" % (root, dargs)
    else:
        o += "            rc = Abi.ak_encode_%s(_run, _ctx, &vt, &fix);" % root
    o += "        }"
    o += "        if (rc < 0) return (int)rc;"
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
    o += "        try { G.D_%s(ref *fix, Tgt(obj), ((DecRun*)obj)->Buf); }" % root
    o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
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
            o += "            byte* b = ((DecRun*)obj)->Buf;"
            o += "            var lst = %s;" % lst
            _add_body(o, s, "lst", "xs", "n", "            ")
            o += "        }"
            o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
            o += "    }"
            o += ""
            continue
        o += "    %s" % UCO
        o += "    private static long New_%s(IntPtr ctx, void* obj)" % s.name
        o += "    {"
        o += "        _rev++;"
        o += "        try { var lst = %s; lst.Add(new %s()); return lst.Count - 1; }" % (lst, s.et)
        o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); return -1; }"
        o += "    }"
        o += ""
        o += "    %s" % UCO
        o += "    private static void Apply_%s(IntPtr ctx, void* obj, long token, ak_dfix_%s* fix)" % (s.name, s.et)
        o += "    {"
        o += "        _rev++;"
        o += "        try { var lst = %s; G.D_%s(ref *fix, lst[(int)token], ((DecRun*)obj)->Buf); }" % (lst, s.et)
        o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
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
            o += "        catch { Abi.ak_fail(ctx, Abi.AK_ERR_HOST, null, 0); }"
            o += "    }"
            o += ""
    o += "    private static readonly byte[] One = new byte[1];"
    o += ""
    _emit_unk(o, p, root)
    o += "    public %s Decode(byte[] src, int len) { int rc = TryDecode(src, len, false, out var t); if (rc < 0) throw new InvalidOperationException($\"core decode failed: {rc}\"); return t; }" % root
    o += "    public %s DecodeU(byte[] src, int len) { int rc = TryDecode(src, len, true, out var t); if (rc < 0) throw new InvalidOperationException($\"core decode failed: {rc}\"); return t; }" % root
    o += ""
    o += "    /// The core's code (< 0) on failure; the output is then unspecified and discarded (R-G6)."
    o += "    public int TryDecode(byte[] src, int len, bool retain, out %s result) => DecodeArmed(src, len, retain ? -1 : -2, out result);" % root
    o += ""
    o += "    /// A control (decision 11 DISCARD): retain everywhere except `position` (an index into"
    o += "    /// UnkPositionNames), whose entry is all zero when armed."
    o += "    public int TryDecodeZeroing(byte[] src, int len, int position, out %s result) => DecodeArmed(src, len, position, out result);" % root
    o += ""
    o += "    private int DecodeArmed(byte[] src, int len, int mode, out %s result)" % root
    o += "    {"
    o += "        result = null;"
    o += "        EnsureDec();"
    o += "        Abi.ak_dec_err_reset(_dctx);"
    o += "        int ar = ArmFor(mode);"
    o += "        if (ar != 0) { Disarm(ar); return ar; }"
    o += "        var target = new %s();" % root
    o += "        var h = GCHandle.Alloc(target);"
    o += "        int rc = Abi.AK_ERR_HOST;"
    o += "        try"
    o += "        {"
    o += "            fixed (byte* b0 = src)"
    o += "            fixed (byte* one = One)"
    o += "            {"
    o += "                // (NULL, 0) is never handed to the core: an empty buffer is a valid pointer and 0."
    o += "                byte* b = len == 0 ? one : b0;"
    o += "                _drun->Target = GCHandle.ToIntPtr(h);"
    o += "                _drun->Buf = b;"
    o += "                var vt = new ak_dvt_%s" % root
    o += "                {"
    o += "                    apply = &ApplyRoot,"
    for s in slots:
        if s.leaf:
            o += "                    add_%s = &Add_%s," % (s.name, s.name)
        else:
            o += "                    new_%s = &New_%s," % (s.name, s.name)
            o += "                    apply_%s = &Apply_%s," % (s.name, s.name)
            for i in s.inner:
                o += "                    add_%s_%s = &Add_%s_%s," % (s.name, i.name, s.name, i.name)
    o += "                };"
    o += "                _fwd++;"
    o += "                rc = Abi.ak_decode_%s(_dctx, _drun, b, (nuint)len, &vt);" % root
    o += "                if (rc >= 0) { int he = Abi.ak_dec_err(_dctx); if (he != 0) rc = he; }"
    o += "            }"
    o += "        }"
    o += "        finally { h.Free(); rc = Disarm(rc); }"
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


def _emit_unk(o, p, root):
    """Decision 11 for this root: the options (native, unmoved while armed), arming and
    disarming around one decode, and the controls' position helpers."""
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
    o += "    private HashSet<IntPtr> _live;"
    o += "    /// The resets that arm and disarm each decode (rule 7): forward crossings, counted"
    o += "    /// apart from ForwardCalls because the core's R5 counters do not count them."
    o += "    private static long _resets;"
    o += "    public long ResetCalls => _resets;"
    o += ""
    o += "    private void EnsureDec()"
    o += "    {"
    o += "        if (_dctx != IntPtr.Zero) return;"
    o += "        _dctx = Abi.ak_dec_ctx_new_%s(null);   // rule 6: bound to this root, drop mode" % root
    o += "        if (_dctx == IntPtr.Zero) throw new InvalidOperationException(\"ak_dec_ctx_new_%s returned NULL\");" % root
    o += "        _drun = (DecRun*)NativeMemory.AllocZeroed((nuint)sizeof(DecRun));"
    o += "    }"
    o += ""
    o += "    /// The options, rewritten before every retained decode: every entry names the one"
    o += "    /// grow and holds no pre-allocated buffer (all from grow, so the core never needs"
    o += "    /// a refill); `zero` >= 0 leaves that position's entry all zero (DISCARD)."
    o += "    private %s* Arm(int zero)" % on
    o += "    {"
    o += "        if (_uo == null) _uo = (%s*)NativeMemory.AllocZeroed((nuint)sizeof(%s));" % (on, on)
    o += "        *_uo = default;"
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
    o += "        if (mode == -2) { G.Live = null; return Abi.ak_dec_reset_%s(_dctx, null); }" % root
    o += "        _live ??= new HashSet<IntPtr>();"
    o += "        _live.Clear();"
    o += "        G.Live = _live;"
    o += "        return Abi.ak_dec_reset_%s(_dctx, Arm(mode));" % root
    o += "    }"
    o += ""
    o += "    /// After every decode: the core forgets the options (reset with NULL), and a buffer"
    o += "    /// still outstanding is freed; after a success that is UNDELIVERED."
    o += "    private int Disarm(int rc)"
    o += "    {"
    o += "        _resets++;"
    o += "        Abi.ak_dec_reset_%s(_dctx, null);" % root
    o += "        var live = G.Live;"
    o += "        G.Live = null;"
    o += "        if (live == null || live.Count == 0) return rc;"
    o += "        foreach (var q in live) NativeMemory.Free((void*)q);"
    o += "        int left = live.Count;"
    o += "        live.Clear();"
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
    o += "                    if (rc == 0) Replay(t, b, recs, (int)rlen);"
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
    p = as_plan(x)
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
    for ln in STAGE.strip("\n").split("\n"):
        o += ln
    o += ""
    _emit_groups(o, p)
    for root in p.roots:
        _emit_root(o, p, root, facade_ns)
    o += "#endif"
    return str(o)
