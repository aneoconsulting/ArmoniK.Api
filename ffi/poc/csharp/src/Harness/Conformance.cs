// R2: correctness before timing, and byte identity across every arm.
//
// Nothing in Bench.cs runs until this passes. The oracle is
// ffi/schema/generated/manifest.json, which the Rust slice validated against
// two independent protobuf implementations, so no arm here is checked against
// another arm alone.
//
// Four checks, and the last two exist because the first two cannot see a decode
// defect at all:
//
//   1. byte identity, every encode arm against the manifest hash;
//   2. the committed vector, byte for byte, where the manifest carries one;
//   3. decode-then-re-encode reproduces the canonical bytes, every decode arm;
//   4. decode-then-COMPARE against the graph the builder made, field by field,
//      through a comparer emitted from the same walker as the codec.
//
// Check 3 alone would pass a decoder that dropped a field the encoder also
// omits. Check 4 is what catches that, and it is why the comparer is generated.

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Linq;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Harness;

public static class Conformance
{
    public static int Run(bool verbose = true)
    {
        var rows = Manifest.Load();
        var arms = ArmTable.All();
        int bad = 0, checks = 0;

        Console.WriteLine("payload  shape  root                        bytes  gp-tba  gp-wto  gp-bw   managed  2pass  vector  mrt  grt  value");
        Console.WriteLine(new string('-', 126));

        foreach (var a in arms)
        {
            if (!rows.TryGetValue(a.Id, out var row))
            {
                Console.WriteLine("{0,-8} NO MANIFEST ROW -- the arm table and the description disagree", a.Id);
                bad++;
                continue;
            }
            a.Build();

            var results = new List<string>();
            byte[] canon = Manifest.Vector(row);          // null over 64 KB

            // ---- 1 and 2: the encode arms -------------------------------
            bool p71 = a.Id == "P7.1";

            byte[] tba = a.GpToByteArray();
            results.Add(Verdict(row, tba, tba.Length, canon, p71, ref checks, ref bad));

            var dst = new byte[Math.Max(row.Bytes + 4096, 8192)];
            int n = a.GpWriteTo(dst);
            results.Add(Verdict(row, dst, n, canon, p71, ref checks, ref bad));

            var bw = new BufWriter(Math.Max(row.Bytes + 4096, 8192));
            int bn = a.GpWriteToBufferWriter(bw);
            results.Add(Verdict(row, bw.WrittenSpan.ToArray(), bn, canon, p71, ref checks, ref bad));

            var e = Enc.New(Codec.Sites, Math.Max(row.Bytes + 4096, 8192));
            a.ManagedWrite(ref e);
            results.Add(Verdict(row, e.Buf, e.Pos, canon, p71, ref checks, ref bad));

            var e2 = Enc.New(Codec.Sites, Math.Max(row.Bytes + 4096, 8192));
            a.ManagedWriteSized(ref e2);
            results.Add(Verdict(row, e2.Buf, e2.Pos, canon, p71, ref checks, ref bad));

            // ---- the committed vector, byte for byte --------------------
            if (canon != null)
            {
                checks++;
                bool same = e.Pos == canon.Length && new ReadOnlySpan<byte>(e.Buf, 0, e.Pos).SequenceEqual(canon);
                if (p71)
                {
                    // No canonical writer can produce P7.1's bytes: it interleaves
                    // two repeated fields and a writer that emits a repeated field
                    // contiguously cannot. Validated as a PERMUTATION of the same
                    // (tag, wire type, body) triples instead.
                    same = Triples.Same(new ReadOnlySpan<byte>(e.Buf, 0, e.Pos), canon);
                    results.Add(same ? "perm" : "PERM!");
                }
                else
                {
                    results.Add(same ? "ok" : "BYTES!");
                }
                if (!same) bad++;
            }
            else
            {
                results.Add("-");
            }

            // ---- 3: decode, re-encode, compare --------------------------
            byte[] src = canon ?? e.ToArray();
            int slen = src.Length;

            checks++;
            byte[] mrt = a.ManagedRoundTrip(src, slen);
            bool mok = p71 ? Triples.Same(mrt, src) : Manifest.Sha(mrt, mrt.Length) == row.Sha256;
            results.Add(mok ? "ok" : "RT!");
            if (!mok) bad++;

            checks++;
            byte[] grt = a.GpRoundTrip(src, slen);
            bool gok = p71 ? Triples.Same(grt, src) : Manifest.Sha(grt, grt.Length) == row.Sha256;
            results.Add(gok ? "ok" : "RT!");
            if (!gok) bad++;

            // ---- 4: decode, compare VALUES ------------------------------
            checks++;
            bool vok = a.ManagedDecodesToBuiltValue(src, slen);
            if (p71)
            {
                // The interleaved wire cannot be reproduced by an object model,
                // so the built graph is the contiguous permutation and equality
                // against it is exactly what P7.1 asks a slice to check.
                results.Add(vok ? "ok" : "VAL!");
            }
            else
            {
                results.Add(vok ? "ok" : "VAL!");
            }
            if (!vok) bad++;

            Console.WriteLine("{0,-8} {1,-6} {2,-24} {3,7}  {4,-6}  {5,-6}  {6,-6}  {7,-7}  {8,-5}  {9,-6}  {10,-3}  {11,-3}  {12}",
                a.Id, a.Shape, a.Root, row.Bytes,
                results[0], results[1], results[2], results[3], results[4],
                results[5], results[6], results[7], results[8]);
        }

        Console.WriteLine();
        Console.WriteLine("{0} checks, {1} failure(s)", checks, bad);
        return bad;
    }

    private static string Verdict(PayloadRow row, byte[] b, int len, byte[] canon, bool p71,
                                  ref int checks, ref int bad)
    {
        checks++;
        if (p71)
        {
            // Not asked to reproduce P7.1's bytes; asked to produce a permutation
            // of the same triples. `canon` is null only over 64 KB and P7.1 is small.
            bool ok = canon != null && Triples.Same(new ReadOnlySpan<byte>(b, 0, len), canon);
            if (!ok) bad++;
            return ok ? "perm" : "PERM!";
        }
        if (len != row.Bytes)
        {
            bad++;
            return "LEN " + len;
        }
        string sha = Manifest.Sha(b, len);
        if (sha != row.Sha256) { bad++; return "SHA!"; }
        return "ok";
    }
}

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
