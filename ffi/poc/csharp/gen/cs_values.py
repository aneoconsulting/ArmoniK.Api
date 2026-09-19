"""Backend: the deterministic value rules of `ffi/schema/emit/values.py`, in C#.

No RNG anywhere: a value is a pure function of (field path, element index), so
this slice produces the manifest's bytes by implementing the same function
rather than by reading a data file. The manifest pins the outcome, and a
disagreement here is a defect in this file (`ffi/schema/README.md`).

Re-derived rather than transliterated line by line in two places: the enum
value tables below are EMITTED from `shapes.json`, so the one rule that depends
on the description (`enum_value` cycles the declared values in declaration
order, which is what makes `ResultStatus 127` reachable) cannot drift from it.
"""
from cs_facade import Head
import csnames as N

VOCAB = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
         "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa"]

BODY = r'''
public static class Values
{
    private static readonly string[] Vocab = { %VOCAB% };

    // One SHA-256 per thread: the value rules are called from the payload
    // builders only, but a benchmark harness that built two payloads on two
    // threads would otherwise corrupt both and look like a codec defect.
    [ThreadStatic] private static SHA256 _sha;
    private static SHA256 Sha => _sha ??= SHA256.Create();

    private static byte[] Digest(string s) => Sha.ComputeHash(Encoding.UTF8.GetBytes(s));

    /// The first eight bytes of sha256("path#idx"), little endian.
    public static ulong H64(string path, long idx)
    {
        var d = Digest(path + "#" + idx.ToString(CultureInfo.InvariantCulture));
        ulong n = 0;
        for (int i = 7; i >= 0; i--) n = (n << 8) | d[i];
        return n;
    }

    private static string Hex(byte[] d)
    {
        var c = new char[d.Length * 2];
        const string H = "0123456789abcdef";
        for (int i = 0; i < d.Length; i++) { c[2 * i] = H[d[i] >> 4]; c[2 * i + 1] = H[d[i] & 15]; }
        return new string(c);
    }

    /// 36 ASCII characters, the shape of every id in the real schema. 174 of the
    /// schema's 413 fields are strings and almost all of them are this.
    public static string Guid(string path, long idx)
    {
        var d = Hex(Digest(path + "#" + idx.ToString(CultureInfo.InvariantCulture)));
        var g = d.Substring(0, 8) + "-" + d.Substring(8, 4) + "-" + d.Substring(12, 4)
              + "-" + d.Substring(16, 4) + "-" + d.Substring(20, 12);
        return Skew(g);
    }

    public static string Word(string path, long idx)
    {
        ulong h = H64(path, idx);
        return Skew(Vocab[(int)(h % (ulong)Vocab.Length)]
                  + (h % 1000).ToString(CultureInfo.InvariantCulture));
    }

    public static string Sentence(string path, long idx)
    {
        ulong h = H64(path, idx);
        var sb = new StringBuilder(40);
        for (int i = 0; i < 5; i++)
        {
            if (i > 0) sb.Append(' ');
            sb.Append(Vocab[(int)((h >> (4 * i)) % (ulong)Vocab.Length)]);
        }
        return Skew(sb.ToString());
    }

    public static byte[] Blob(string path, long idx, int n = 16)
    {
        var outp = new byte[n];
        int have = 0, i = 0;
        while (have < n)
        {
            var d = Digest(path + "#" + idx.ToString(CultureInfo.InvariantCulture)
                                + "#" + i.ToString(CultureInfo.InvariantCulture));
            int take = Math.Min(d.Length, n - have);
            Array.Copy(d, 0, outp, have, take);
            have += take; i++;
        }
        return outp;
    }

    /// A multi-megabyte body, cheap to build and not compressible into a pattern
    /// a codec could accidentally special-case.
    public static byte[] Bulk(int n)
    {
        var outp = new byte[n];
        int have = 0, i = 0;
        while (have < n)
        {
            var d = Digest("bulk#" + i.ToString(CultureInfo.InvariantCulture));
            int take = Math.Min(d.Length, n - have);
            Array.Copy(d, 0, outp, have, take);
            have += take; i++;
        }
        return outp;
    }

    public static int ScalarI32(string path, long idx) => (int)(H64(path, idx) % 100000UL);
    public static long ScalarI64(string path, long idx) => (long)(H64(path, idx) % 1000000000000UL);
    public static bool ScalarBool(string path, long idx) => (H64(path, idx) & 1UL) != 0UL;

    /// values.py computes this as `(h % 1000000) / 1000.0`; the `struct.unpack`
    /// term beside it in that file is multiplied by zero and every value it can
    /// produce is a subnormal, so it contributes nothing and cannot be a NaN.
    public static double ScalarF64(string path, long idx) => (H64(path, idx) % 1000000UL) / 1000.0;

    public static long StampSeconds(long idx) => 1700000000L + idx * 37L;
    public static int StampNanos(long idx) => (int)((idx * 7919L) % 1000000000L);
    public static long DurSeconds(long idx) => idx % 3600L;
    public static int DurNanos(long idx) => (int)((idx * 104729L) % 1000000000L);

    /// An explicit-presence field that is always written never exercises the
    /// thing it exists for. Presence cycles per field by its TAG, and one
    /// element in seven carries a present-but-zero value.
    public static bool ExplicitPresent(int tag, long idx) => (idx % (tag + 1)) != 0;

    public static bool PresentZero(long idx) => idx % 7 == 0;

    /// Which of the Output facade's three states element `idx` carries. Cycled
    /// error, ok, invalid so that element 0 is Error: `half_absent` removes the
    /// nested site and P2.5 keeps a written one first.
    public static int AdapterState(long idx) => (int)(idx % 3);   // 0 error, 1 ok, 2 invalid

    // ---- content sets (design/SHAPES.md) ------------------------------
    //
    // ASCII is the default and the only one `ffi/schema` emits, because every
    // id, session id, task id, result id and partition name in the real schema
    // is an ASCII GUID. The other two exist because a UTF-16 host has a
    // NARROWING TRANSCODER on its encode path, and on ASCII that transcoder
    // never has to do anything: one char, one byte.
    //
    // The skew keeps the CHARACTER count of every string identical and changes
    // only which characters they are, so the object graph has the same shape in
    // all three sets and what moves is the byte width and the transcoder's
    // path. There is NO manifest oracle for these -- `schema/` emits ASCII only
    // -- so a set is checked by byte identity of every arm against the
    // INCUMBENT arm, which the manifest validated on ASCII, plus a decode round
    // trip per set.
    //
    // What this prices is NOT what the Rust slice's content-set rows price. A
    // Rust `String` is already UTF-8, so there is no narrowing and what changes
    // there is validation and width. Here it is a transcoder. The two columns
    // are not each other's comparator.
    public const int Ascii = 0, Latin1 = 1, Wide = 2;

    public static int ContentSet = Ascii;

    public static string[] SetNames = { "ascii", "latin1", "wide" };

    /// Map one ASCII character into the active set, preserving the character
    /// count. Latin-1: U+00A0 to U+00FF, two UTF-8 bytes. Wide: U+4E00 and up,
    /// three UTF-8 bytes. A character above U+FFFF would be a surrogate PAIR
    /// and would change the character count, so the sets stop at the BMP; the
    /// unpaired-surrogate case is a correctness question, not a width one, and
    /// is exercised separately.
    ///
    /// **The exact sets are NOT pinned by `ffi/schema`**, which emits ASCII
    /// only and describes the other two as "U+00A0 to U+00FF" and "above
    /// U+00FF". "Above U+00FF" admits both a two-byte and a three-byte
    /// encoding, and the first version of this file picked mostly two-byte:
    /// its `wide` measured 1.78 to 1.84 times the ASCII wire against the Rust
    /// slice's published 2.39 to 2.50, which is not a disagreement between
    /// runtimes but two slices choosing different characters. Under R1 that is
    /// a defect rather than a difference, so this picks three bytes throughout
    /// to land where the Rust slice landed -- and it is raised in STATE.md as a
    /// request, because agreeing by hand is exactly what one description is for.
    public static char Skew(char c, int i)
    {
        if (ContentSet == Ascii) return c;
        int k = c & 0x3f;
        if (ContentSet == Latin1) return (char)(0x00A0 + (k % 0x60));
        return (char)(0x4E00 + k);
    }

    public static string Skew(string s)
    {
        if (ContentSet == Ascii || s.Length == 0) return s;
        var a = s.ToCharArray();
        for (int i = 0; i < a.Length; i++) a[i] = Skew(a[i], i);
        return new string(a);
    }
%ENUMS%}
'''


def emit(ir):
    o = Head("The deterministic value rules of ffi/schema/emit/values.py, in C#.")
    o += "using System;"
    o += "using System.Globalization;"
    o += "using System.Security.Cryptography;"
    o += "using System.Text;"
    o += ""
    o += "namespace Armonik.Ffi.Facade;"
    o += ""

    enums = []
    for ename, edef in ir.enums.items():
        vals = ", ".join("%s.%s" % (ename, N.enum_member(ename, v)) for v in edef["values"])
        enums.append("")
        enums.append("    private static readonly %s[] %sVals = { %s };" % (ename, ename, vals))
        enums.append("")
        enums.append("    /// Cycles the declared values so that a payload with enough elements")
        enums.append("    /// reaches the large one: ResultStatus 127 is a two-byte varint.")
        enums.append("    public static %s %sAt(long idx) => %sVals[(int)(idx %% %d)];"
                     % (ename, ename, ename, len(edef["values"])))

    body = (BODY
            .replace("%VOCAB%", ", ".join('"%s"' % w for w in VOCAB))
            .replace("%ENUMS%", "\n".join(enums) + "\n"))
    for ln in body.strip("\n").split("\n"):
        o += ln
    return str(o)
