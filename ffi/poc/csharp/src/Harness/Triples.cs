// Split out of Conformance.cs so the corpus runner (src/Corpus) can link it alone.

using System;
using System.Collections.Generic;

namespace Armonik.Ffi.Harness;

/// Every (tag, wire type, body) triple in a buffer, sorted. Two encodings of
/// one value that differ only in field ORDER produce the same list, which is
/// what P7.1 is validated by: no canonical writer can interleave two repeated
/// fields, so the control is checked as a permutation rather than as bytes.
public static class Triples
{
    public static bool Same(ReadOnlySpan<byte> a, ReadOnlySpan<byte> b)
    {
        var xa = Of(a);
        var xb = Of(b);
        if (xa == null || xb == null || xa.Count != xb.Count) return false;
        xa.Sort(StringComparer.Ordinal);
        xb.Sort(StringComparer.Ordinal);
        for (int i = 0; i < xa.Count; i++) if (xa[i] != xb[i]) return false;
        return true;
    }

    private static List<string> Of(ReadOnlySpan<byte> s)
    {
        var o = new List<string>();
        int p = 0;
        while (p < s.Length)
        {
            ulong k = 0;
            int shift = 0;
            while (true)
            {
                if (p >= s.Length) return null;
                byte c = s[p++];
                k |= (ulong)(c & 0x7f) << shift;
                if ((c & 0x80) == 0) break;
                shift += 7;
                if (shift > 63) return null;
            }
            int wire = (int)(k & 7);
            int tag = (int)(k >> 3);
            int start = p, len;
            switch (wire)
            {
                case 0:
                    while (p < s.Length && (s[p] & 0x80) != 0) p++;
                    p++;
                    len = p - start;
                    break;
                case 1: len = 8; p += 8; break;
                case 5: len = 4; p += 4; break;
                case 2:
                {
                    ulong n = 0;
                    int sh = 0;
                    while (true)
                    {
                        if (p >= s.Length) return null;
                        byte c = s[p++];
                        n |= (ulong)(c & 0x7f) << sh;
                        if ((c & 0x80) == 0) break;
                        sh += 7;
                    }
                    start = p;
                    // R-G8: compared with the bytes left before any narrowing.
                    if (n > (ulong)(s.Length - p)) return null;
                    len = (int)n;
                    p += len;
                    break;
                }
                default: return null;
            }
            if (p > s.Length) return null;
            o.Add(tag + ":" + wire + ":" + Convert.ToBase64String(s.Slice(start, len).ToArray()));
        }
        return o;
    }
}
