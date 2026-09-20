// Hand-written on purpose, and the ONE place in this slice that is.
//
// R1 forbids a hand-written *codec*: the traversal -- which field, which tag,
// which order -- is emitted from the description in `Generated/Codec.cs`. This
// file is the runtime underneath it: varints, the growable buffer, the learned
// length-placeholder width and the reader. It is the direct analogue of the
// Rust slice's `crates/ak-rt`, and it knows nothing about any message.
//
// ABI v1 section 6: "Length placeholders use a learned width, held in the
// encode context. [...] The table lives in the context, never process-global".
// Section 4: the codec hands the transcoder whatever the buffer has left rather
// than a reservation sized from a declared bound, so the prefix width is
// resolved AFTER the transcode returns. Open decision 5 is what that costs.

using System;
using System.Runtime.CompilerServices;
using System.Text;

namespace Armonik.Ffi.Facade;

public static class W
{
    public const int WireVarint = 0, WireI64 = 1, WireLen = 2, WireI32 = 5;

    /// The deprecated GROUP form. proto3 cannot express one, so nothing this
    /// generator emits ever writes these -- and a conformant reader still has
    /// to SKIP one, because a proto2 peer may send it. `ffi/corpus`'s
    /// `U-root-group`, `U-nested-group` and `U-oneof-group` are exactly that
    /// case and this slice rejected all three until they were run.
    public const int WireGroup = 3, WireEndGroup = 4;

    /// **These are the FACADE's own error codes and they are not the ABI's.**
    /// `ak-rt` numbers malformed -2 and depth -4; this numbers malformed -4.
    /// Nothing converts between them and nothing should: the managed control
    /// does not go through the C ABI at all. Said here because -4 meaning two
    /// different things in one repository is a trap worth naming once.
    public const int ErrTruncated = -3, ErrMalformed = -4, ErrTranscode = -6;

    /// The unknown-group recursion limit, hit before the stack is.
    public const int ErrDepth = -8;

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ulong Key(int tag, int wire) => ((ulong)(uint)tag << 3) | (uint)wire;

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int VarintLen(ulong v)
    {
        int n = 1;
        while (v >= 0x80) { v >>= 7; n++; }
        return n;
    }
}

/// An open length prefix. Held by value so a miss cannot be attributed to the
/// wrong site.
public struct Mark
{
    public int Site;
    public int Hdr;
    public int W;
}

/// The encode buffer. A mutable struct, always passed by `ref`.
///
/// A class here would reload `_buf` and `_pos` from the heap at every write,
/// while `Google.Protobuf`'s own fast path keeps them in a `ref` struct
/// (`WriteContext`). Making the two shapes match is the difference between
/// measuring two codecs and measuring one codec against one indirection.
public struct Enc
{
    public byte[] Buf;
    public int Pos;
    /// One learned width per length-prefix site in the generated code. Per
    /// context: ABI v1 section 6 refuses a process-global table, because two
    /// encoding threads sharing one made both slower than one.
    public byte[] Widths;
    /// Sticky, first error wins (ABI v1 section 5).
    public int Err;
    public long PrefixMoves;
    public long PrefixBytes;
    public long Grows;
#if AK_COUNT
    /// COUNTING BUILD ONLY: which SITE missed, so "the learned width costs X"
    /// can name the field rather than the message. ABI v1 open decision 5 asks
    /// what the mechanism is worth, and an aggregate that does not say where it
    /// thrashes cannot answer it. Kept behind a define so the measured build's
    /// hot path does not carry the store -- the Rust slice's `count` feature,
    /// spelled for C#.
    public int[] SiteMoves;
#endif

    public static Enc New(int sites, int capacity = 4096)
    {
        var e = new Enc { Buf = new byte[capacity], Pos = 0, Widths = new byte[sites], Err = 0 };
        for (int i = 0; i < sites; i++) e.Widths[i] = 1;
#if AK_COUNT
        e.SiteMoves = new int[sites];
#endif
        return e;
    }

    /// The learned widths deliberately SURVIVE a reset: that is what makes them
    /// learned. The counters do not, so a per-run figure is a per-run figure.
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void Reset()
    {
        Pos = 0;
        Err = 0;
        PrefixMoves = 0;
        PrefixBytes = 0;
        Grows = 0;
#if AK_COUNT
        Array.Clear(SiteMoves, 0, SiteMoves.Length);
#endif
    }

    public byte[] ToArray()
    {
        var o = new byte[Pos];
        Buffer.BlockCopy(Buf, 0, o, 0, Pos);
        return o;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void Need(int n)
    {
        if (Pos + n > Buf.Length) Grow(n);
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private void Grow(int n)
    {
        Grows++;
        int want = Buf.Length * 2;
        if (want < Pos + n) want = Pos + n;
        if (want < 4096) want = 4096;
        Array.Resize(ref Buf, want);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void Varint(ulong v)
    {
        Need(10);
        var b = Buf;
        int p = Pos;
        while (v >= 0x80) { b[p++] = (byte)((byte)v | 0x80); v >>= 7; }
        b[p++] = (byte)v;
        Pos = p;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void VarintField(int tag, ulong v)
    {
        Varint(W.Key(tag, W.WireVarint));
        Varint(v);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void F64Field(int tag, double v)
    {
        Varint(W.Key(tag, W.WireI64));
        F64(v);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void F64(double v)
    {
        Need(8);
        long bits = BitConverter.DoubleToInt64Bits(v);
        var b = Buf;
        int p = Pos;
        b[p] = (byte)bits; b[p + 1] = (byte)(bits >> 8); b[p + 2] = (byte)(bits >> 16);
        b[p + 3] = (byte)(bits >> 24); b[p + 4] = (byte)(bits >> 32); b[p + 5] = (byte)(bits >> 40);
        b[p + 6] = (byte)(bits >> 48); b[p + 7] = (byte)(bits >> 56);
        Pos = p + 8;
    }

    /// A length-delimited field whose length is known before the body is
    /// written: every blob the HOST hands over by value, because it owns the
    /// bytes and knows how many there are.
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void BlobField(int tag, byte[] v)
    {
        Varint(W.Key(tag, W.WireLen));
        Varint((ulong)(uint)v.Length);
        Need(v.Length);
        Buffer.BlockCopy(v, 0, Buf, Pos, v.Length);
        Pos += v.Length;
    }

    /// Open a length-delimited field whose body length is not yet known.
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public Mark Begin(int tag, int site)
    {
        Varint(W.Key(tag, W.WireLen));
        int w = Widths[site];
        Need(w);
        int hdr = Pos;
        Pos += w;
        return new Mark { Site = site, Hdr = hdr, W = w };
    }

    /// Close it, resolving the prefix width now that the body is written. A
    /// miss MOVES the body; it is never padded, because padding to a learned
    /// width makes the encoder's output depend on its own history and ABI v1
    /// section 6 refuses that explicitly.
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void End(Mark m)
    {
        int body = Pos - m.Hdr - m.W;
        int need = W.VarintLen((ulong)(uint)body);
        if (need != m.W) ResizePrefix(ref m, body, need);
        ulong v = (ulong)(uint)body;
        int i = m.Hdr;
        var b = Buf;
        while (v >= 0x80) { b[i++] = (byte)((byte)v | 0x80); v >>= 7; }
        b[i] = (byte)v;
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private void ResizePrefix(ref Mark m, int body, int need)
    {
        PrefixMoves++;
        PrefixBytes += body;
#if AK_COUNT
        SiteMoves[m.Site]++;
#endif
        Widths[m.Site] = (byte)need;
        int src = m.Hdr + m.W;
        if (need > m.W) { Need(need - m.W); Pos += need - m.W; }
        int dst = m.Hdr + need;
        Buffer.BlockCopy(Buf, src, Buf, dst, body);
        if (need < m.W) Pos -= m.W - need;
        m.W = need;
    }

    /// A string, transcoded straight into the buffer under an open prefix: the
    /// codec does not know the UTF-8 length until the transcoder returns, which
    /// is what the learned width exists for (ABI v1 sections 4 and 6).
    public void StringField(int tag, string s, int site)
    {
        Varint(W.Key(tag, W.WireLen));
        int w = Widths[site];
        // The transcoder is handed whatever is left, not a per-string
        // reservation, so a grow is the exception and not the rhythm. Three
        // bytes per char is the worst case a UTF-16 unit can produce; a
        // surrogate PAIR is four bytes for two units, so it is covered.
        Need(w + s.Length * 3);
        int hdr = Pos;
        int n = Transcode(s, hdr + w);
        Pos = hdr + w + n;
        int need = W.VarintLen((ulong)(uint)n);
        if (need != w)
        {
            PrefixMoves++;
            PrefixBytes += n;
#if AK_COUNT
            SiteMoves[site]++;
#endif
            Widths[site] = (byte)need;
            if (need > w) { Need(need - w); Pos += need - w; }
            Buffer.BlockCopy(Buf, hdr + w, Buf, hdr + need, n);
            if (need < w) Pos -= w - need;
            w = need;
        }
        ulong v = (ulong)(uint)n;
        int i = hdr;
        var b = Buf;
        while (v >= 0x80) { b[i++] = (byte)((byte)v | 0x80); v >>= 7; }
        b[i] = (byte)v;
    }

    /// The one place the floor and the target are different code (README 5.1).
    /// `AK_FLOOR` is set by the netstandard2.0 build AND by an explicit
    /// property on a net8.0 build, which is how README 5.2's arm b exists at
    /// all: floor sources, target runtime, one define flipped.
#if AK_FLOOR
    private unsafe int Transcode(string s, int at)
    {
        if (s.Length == 0) return 0;
        fixed (char* src = s)
        fixed (byte* dst = &Buf[at])
        {
            return Encoding.UTF8.GetBytes(src, s.Length, dst, Buf.Length - at);
        }
    }
#else
    private int Transcode(string s, int at)
    {
        return Encoding.UTF8.GetBytes(s.AsSpan(), Buf.AsSpan(at));
    }
#endif

    /// Exactly what `Encoding.UTF8.GetByteCount` costs, for the two-pass arm.
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int Utf8Len(string s) => Encoding.UTF8.GetByteCount(s);

    /// Write a length-delimited field whose length was computed by a size pass,
    /// which is the shape `Google.Protobuf` uses. No learned width, no move.
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void SizedHeader(int tag, int len)
    {
        Varint(W.Key(tag, W.WireLen));
        Varint((ulong)(uint)len);
    }

    public void StringBodySized(string s, int len)
    {
        Need(len);
        Transcode(s, Pos);
        Pos += len;
    }
}

/// The decode reader.
///
/// Over `byte[]` rather than `ReadOnlySpan<byte>` so that one source serves
/// both language levels: the netstandard2.0 floor has no span overload of
/// `Encoding.GetString`, and a reader that had to diverge there would put the
/// divergence in the hot loop rather than in one transcode call.
public struct Dec
{
    public byte[] Buf;
    public int Pos;
    public int End;
    public int Err;

    public static Dec Over(byte[] b) => new Dec { Buf = b, Pos = 0, End = b.Length, Err = 0 };

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public ulong Varint()
    {
        var b = Buf;
        int p = Pos;
        ulong n = 0;
        int shift = 0;
        while (true)
        {
            if (p >= End) { Err = W.ErrTruncated; Pos = p; return 0; }
            byte c = b[p++];
            n |= (ulong)(c & 0x7f) << shift;
            if ((c & 0x80) == 0) { Pos = p; return n; }
            shift += 7;
            if (shift > 63) { Err = W.ErrMalformed; Pos = p; return 0; }
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public double F64()
    {
        if (Pos + 8 > End) { Err = W.ErrTruncated; return 0.0; }
        var b = Buf;
        int p = Pos;
        long bits = b[p] | ((long)b[p + 1] << 8) | ((long)b[p + 2] << 16) | ((long)b[p + 3] << 24)
                  | ((long)b[p + 4] << 32) | ((long)b[p + 5] << 40) | ((long)b[p + 6] << 48)
                  | ((long)b[p + 7] << 56);
        Pos = p + 8;
        return BitConverter.Int64BitsToDouble(bits);
    }

    /// The end offset of a length-delimited body, having consumed its prefix.
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public int LenEnd()
    {
        int n = (int)Varint();
        if (n < 0 || Pos + n > End) { Err = W.ErrTruncated; return Pos; }
        return Pos + n;
    }

    public string Str()
    {
        int e = LenEnd();
        if (Err != 0) return "";
        int off = Pos;
        Pos = e;
        // `Encoding.UTF8` substitutes U+FFFD rather than throwing, which is the
        // LOSSY policy. Google.Protobuf reads strings through the same
        // encoding object, so the two arms are like for like; ABI v1 open
        // decision 3's rejecting policy is NOT what either of them runs.
        return e == off ? "" : Encoding.UTF8.GetString(Buf, off, e - off);
    }

    public byte[] Bytes()
    {
        int e = LenEnd();
        if (Err != 0) return Array.Empty<byte>();
        int off = Pos;
        Pos = e;
        if (e == off) return Array.Empty<byte>();
        var o = new byte[e - off];
        Buffer.BlockCopy(Buf, off, o, 0, e - off);
        return o;
    }

    /// An unknown field: protobuf's forward compatibility, and the path a
    /// corpus generated from the schema that reads it never executes.
    ///
    /// **It takes the TAG as well as the wire type, and that is the whole fix.**
    /// A group carries no length, so its end is an `END_GROUP` tag whose FIELD
    /// NUMBER matches the one that opened it. A skipper that counts depth
    /// instead accepts `X-group-mismatched-end` and then mis-nests every group
    /// after it, which is a wrong parse rather than a rejected one. This
    /// version had cases for the four wire types a proto3 schema produces and
    /// `ErrMalformed` for everything else, so it rejected three corpus vectors
    /// that `Google.Protobuf` accepts. The shared core had the identical hole
    /// (D7) and the C++ slice still does.
    public void Skip(int tag, int wire)
    {
        switch (wire)
        {
            case W.WireVarint: Varint(); break;
            case W.WireI64: Pos += 8; break;
            case W.WireLen: Pos = LenEnd(); break;
            case W.WireGroup: SkipGroup(tag, 0); break;
            case W.WireI32: Pos += 4; break;
            // 4 is END_GROUP with nothing open; 6 and 7 do not exist.
            default: Err = W.ErrMalformed; return;
        }
        if (Pos > End) Err = W.ErrTruncated;
    }

    /// protobuf's own default recursion limit, applied to nested unknown groups.
    private const int MaxGroupDepth = 100;

    /// Recursive, because groups nest; bounded, because a payload of nothing but
    /// start tags would otherwise be a stack overflow rather than an error.
    private void SkipGroup(int tag, int depth)
    {
        if (depth >= MaxGroupDepth) { Err = W.ErrDepth; return; }
        while (true)
        {
            if (Err != 0) return;
            // An unterminated group: `X-group-unterminated`. A skipper that
            // scans for the next end tag without checking the buffer walks off it.
            if (Pos >= End) { Err = W.ErrTruncated; return; }
            ulong k = Varint();
            if (Err != 0) return;
            int t = (int)(k >> 3), w = (int)(k & 7);
            if (t == 0) { Err = W.ErrMalformed; return; }
            if (w == W.WireEndGroup)
            {
                if (t != tag) Err = W.ErrMalformed;   // `X-group-mismatched-end`
                return;
            }
            if (w == W.WireGroup) { SkipGroup(t, depth + 1); continue; }
            Skip(t, w);
        }
    }
}
