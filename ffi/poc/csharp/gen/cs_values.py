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
        return d.Substring(0, 8) + "-" + d.Substring(8, 4) + "-" + d.Substring(12, 4)
             + "-" + d.Substring(16, 4) + "-" + d.Substring(20, 12);
    }

    public static string Word(string path, long idx)
    {
        ulong h = H64(path, idx);
        return Vocab[(int)(h % (ulong)Vocab.Length)] + (h % 1000).ToString(CultureInfo.InvariantCulture);
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
        return sb.ToString();
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
