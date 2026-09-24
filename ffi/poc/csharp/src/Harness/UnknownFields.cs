// The unknown-field vectors: the path a corpus generated from the schema that
// reads it can never contain (README section 10, design/SHAPES.md).
//
// Hand-built, because they have to be: every payload in `ffi/schema` is emitted
// from the description the decoder was emitted from, so no payload in it can
// carry a field the decoder does not know. The bytes below are a known payload
// with an unrecognised field spliced in.
//
// Two things this establishes, and the second is a behaviour change with a
// language count attached rather than a performance figure:
//
//   1. both decoders accept an unknown field and neither loses a known value;
//   2. `Google.Protobuf` RETAINS the unknown field and writes it back;
//      the managed codec as generated DROPS it. That is ABI v1 open decision
//      11 on .NET, and the direction is the opposite of Rust's: prost drops
//      too, so the Rust slice is structurally the wrong place to price what
//      removing the guarantee costs. Here it costs something visible.
//
// And the case design/SHAPES.md says cannot round trip at all: an unrecognised
// ONEOF member. A parser cannot tell one from any other unknown field, since
// the grouping lives only in the descriptor, so the case stays where it was and
// the payload is dropped. Exercised rather than asserted.

using System;
using System.Collections.Generic;
using System.Linq;
using Armonik.Ffi.Facade;
using Google.Protobuf;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Harness;

public static class UnknownFields
{
    private static byte[] Varint(ulong v)
    {
        var o = new List<byte>();
        while (v >= 0x80) { o.Add((byte)((byte)v | 0x80)); v >>= 7; }
        o.Add((byte)v);
        return o.ToArray();
    }

    private static byte[] Key(int tag, int wire) => Varint(((ulong)(uint)tag << 3) | (uint)wire);

    private static byte[] Cat(params byte[][] parts)
    {
        int n = parts.Sum(p => p.Length);
        var o = new byte[n];
        int at = 0;
        foreach (var p in parts) { Buffer.BlockCopy(p, 0, o, at, p.Length); at += p.Length; }
        return o;
    }

    private static byte[] UnkVarint(int tag, ulong v) => Cat(Key(tag, 0), Varint(v));
    private static byte[] UnkLen(int tag, byte[] body) => Cat(Key(tag, 2), Varint((ulong)body.Length), body);
    private static byte[] UnkI64(int tag) => Cat(Key(tag, 1), new byte[] { 1, 2, 3, 4, 5, 6, 7, 8 });
    private static byte[] UnkI32(int tag) => Cat(Key(tag, 5), new byte[] { 9, 8, 7, 6 });

    /// Splice `extra` into the first element of a list payload, immediately
    /// after its length prefix, so the unknown field sits INSIDE an element and
    /// the element's own length has to be corrected. An unknown field appended
    /// at the root is the easy case and is covered separately.
    private static byte[] IntoFirstElement(byte[] src, byte[] extra)
    {
        int p = 0;
        ulong k = ReadVarint(src, ref p);
        int lenAt = p;
        ulong len = ReadVarint(src, ref p);
        int bodyAt = p;
        var head = src.Take(lenAt).ToArray();
        var body = new byte[len];
        Buffer.BlockCopy(src, bodyAt, body, 0, (int)len);
        var tail = src.Skip(bodyAt + (int)len).ToArray();
        var newBody = Cat(body, extra);
        return Cat(head, Varint((ulong)newBody.Length), newBody, tail);
    }

    private static ulong ReadVarint(byte[] b, ref int p)
    {
        ulong n = 0;
        int sh = 0;
        while (true)
        {
            byte c = b[p++];
            n |= (ulong)(c & 0x7f) << sh;
            if ((c & 0x80) == 0) return n;
            sh += 7;
        }
    }

    private sealed class Vector
    {
        public string Name;
        public byte[] Bytes;
        public string What;
    }

    public static int Run(params string[] argv)
    {
        var rows = Manifest.Load();
        var p11 = Manifest.Vector(rows["P1.1"]);     // M1, 4 elements, leaf
        var p31 = Manifest.Vector(rows["P3.1"]);     // M3, the oneof
        int bad = 0;

        var vectors = new List<Vector>
        {
            new Vector { Name = "root-varint-between",
                Bytes = Cat(p11, UnkVarint(4, 12345)),
                What = "unknown varint at the ROOT, tag 4, above every known root tag (1..3)" },
            new Vector { Name = "root-len",
                Bytes = Cat(p11, UnkLen(9, new byte[] { 1, 2, 3, 4, 5 })),
                What = "unknown length-delimited field at the root" },
            new Vector { Name = "elem-varint-interleaved",
                Bytes = IntoFirstElement(p11, UnkVarint(7, 99)),
                What = "unknown varint INSIDE element 0, at tag 7 -- the gap ResultRaw leaves "
                     + "between `completed_at` (6) and `result_id` (8), so it sits numerically "
                     + "BETWEEN two known tags" },
            new Vector { Name = "elem-i64",
                Bytes = IntoFirstElement(p11, UnkI64(20)),
                What = "unknown fixed64 inside element 0" },
            new Vector { Name = "elem-i32",
                Bytes = IntoFirstElement(p11, UnkI32(21)),
                What = "unknown fixed32 inside element 0" },
            new Vector { Name = "elem-len-nested",
                Bytes = IntoFirstElement(p11, UnkLen(22, Cat(UnkVarint(1, 7), UnkLen(2, new byte[] { 65, 66 })))),
                What = "unknown length-delimited field inside element 0 whose body is itself a "
                     + "message -- a decoder that recursed into it rather than skipping it would "
                     + "report the inner tags as unknown fields of the OUTER message" },
            new Vector { Name = "oneof-unrecognised-member",
                Bytes = IntoFirstElement(p31, UnkVarint(15, 77)),
                What = "an unrecognised ONEOF member: tag 15 sits one past `as_nothing` (14) in "
                     + "Probe.body. design/SHAPES.md's row says this CANNOT round trip in any "
                     + "arm, because the grouping lives only in the descriptor and a parser "
                     + "cannot tell it from any other unknown field" },
        };

        Console.WriteLine("vector                     bytes  gp-dec  man-dec  known-values-agree  gp-retains  man-retains  man-retain-mode = gp");
        Console.WriteLine(new string('-', 130));

        foreach (var v in vectors)
        {
            bool isProbe = v.Name.StartsWith("oneof", StringComparison.Ordinal);
            string gpDec, manDec, agree, gpRet, manRet;

            byte[] gpOut = null, manOut = null;
            try
            {
                if (isProbe)
                {
                    var m = Gp.ListProbeResponse.Parser.ParseFrom(new ReadOnlySpan<byte>(v.Bytes));
                    gpOut = m.ToByteArray();
                }
                else
                {
                    var m = Gp.ListResultsResponse.Parser.ParseFrom(new ReadOnlySpan<byte>(v.Bytes));
                    gpOut = m.ToByteArray();
                }
                gpDec = "ok";
            }
            catch (Exception ex) { gpDec = "FAIL " + ex.GetType().Name; bad++; }

            try
            {
                var d = new Dec { Buf = v.Bytes, Pos = 0, End = v.Bytes.Length, Err = 0 };
                var e = Enc.New(Codec.Sites, v.Bytes.Length + 4096);
                if (isProbe)
                {
                    var m = new ListProbeResponse();
                    Codec.ReadListProbeResponse(ref d, m, 0);
                    if (d.Err != 0) throw new InvalidOperationException("err " + d.Err);
                    Codec.WriteListProbeResponse(ref e, m);
                }
                else
                {
                    var m = new ListResultsResponse();
                    Codec.ReadListResultsResponse(ref d, m, 0);
                    if (d.Err != 0) throw new InvalidOperationException("err " + d.Err);
                    Codec.WriteListResultsResponse(ref e, m);
                }
                manOut = e.ToArray();
                manDec = "ok";
            }
            catch (Exception ex) { manDec = "FAIL " + ex.GetType().Name; bad++; }

            // RETAIN mode (the plan's Options.unknown = "both", host picks per call): the
            // captured runs are written back after the known fields, as Google.Protobuf
            // writes its UnknownFieldSet. Byte identity with the incumbent's re-encode is
            // the retain-mode gate (FIX-PLAN WP3 item 21).
            string retMode;
            try
            {
                var d = new Dec { Buf = v.Bytes, Pos = 0, End = v.Bytes.Length, Err = 0, Retain = true };
                var e = Enc.New(Codec.Sites, v.Bytes.Length + 4096);
                if (isProbe)
                {
                    var m = new ListProbeResponse();
                    Codec.ReadListProbeResponse(ref d, m, 0);
                    if (d.Err != 0) throw new InvalidOperationException("err " + d.Err);
                    Codec.WriteListProbeResponse(ref e, m);
                }
                else
                {
                    var m = new ListResultsResponse();
                    Codec.ReadListResultsResponse(ref d, m, 0);
                    if (d.Err != 0) throw new InvalidOperationException("err " + d.Err);
                    Codec.WriteListResultsResponse(ref e, m);
                }
                var r = e.ToArray();
                retMode = gpOut != null && Same(r, gpOut) ? "ok (identical)" : "DIFFERS (" + r.Length + " B)";
            }
            catch (Exception ex) { retMode = "FAIL " + ex.GetType().Name; }
            if (!retMode.StartsWith("ok", StringComparison.Ordinal)) bad++;

            // Do the two arms agree on the KNOWN values? Compare each one's
            // re-encode against the ORIGINAL payload with the unknown field
            // removed -- which for these vectors is the source payload itself.
            byte[] baseline = isProbe ? p31 : p11;
            gpRet = gpOut == null ? "-" : (Same(gpOut, baseline) ? "no" : "YES");
            manRet = manOut == null ? "-" : (Same(manOut, baseline) ? "no" : "YES");
            agree = (gpOut != null && manOut != null && Same(manOut, baseline)) ? "ok" : "see below";
            if (manOut != null && !Same(manOut, baseline))
            {
                agree = "MAN-DIFFERS";
                bad++;
            }

            Console.WriteLine("{0,-25} {1,6}  {2,-6}  {3,-7}  {4,-18}  {5,-10}  {6,-11}  {7}",
                v.Name, v.Bytes.Length, gpDec, manDec, agree, gpRet, manRet, retMode);
        }

        Console.WriteLine();
        foreach (var v in vectors) Console.WriteLine("  {0,-25} {1}", v.Name, v.What);

        Console.WriteLine();
        Console.WriteLine("Reading the two retention columns.");
        Console.WriteLine("  `YES` means the arm's re-encode differs from the payload with the unknown");
        Console.WriteLine("  field removed, which for these vectors means it wrote the unknown field back.");
        Console.WriteLine();
        Console.WriteLine("  Google.Protobuf keeps an UnknownFieldSet per message and writes it back, so a");
        Console.WriteLine("  message round-tripped through .NET today preserves a field this build has never");
        Console.WriteLine("  heard of. The managed codec's DEFAULT reader drops it; with Dec.Retain set it keeps");
        Console.WriteLine("  it and writes it back (last column: required byte-identical to Google.Protobuf). Both accept");
        Console.WriteLine("  the input and neither loses a KNOWN value, so this is not a correctness gate;");
        Console.WriteLine("  it is ABI v1 open decision 11 with a cost attached, and on .NET the cost is a");
        Console.WriteLine("  guarantee that exists today and would be removed. prost drops unknown fields");
        Console.WriteLine("  too, which is why the Rust slice could not price this.");
        Console.WriteLine();
        Console.WriteLine("  The oneof vector is the one design/SHAPES.md says cannot round trip in ANY arm.");
        Console.WriteLine("  Google.Protobuf retains the BYTES of tag 15 but the case stays at whatever");
        Console.WriteLine("  member the element already carried; the grouping is descriptor-only, so no");
        Console.WriteLine("  parser can recover the intent. Retention makes the bytes survive and does not");
        Console.WriteLine("  make the VALUE survive, and those are different claims.");

        Console.WriteLine();
        Console.WriteLine("{0} failure(s) -- a FAIL in a decode column, or a managed re-encode that is not",
            bad);
        Console.WriteLine("the baseline (the managed codec is supposed to drop, so differing means a known");
        Console.WriteLine("value moved, which would be a real defect).");
        return bad;
    }

    private static bool Same(byte[] a, byte[] b)
    {
        if (a.Length != b.Length) return false;
        for (int i = 0; i < a.Length; i++) if (a[i] != b[i]) return false;
        return true;
    }
}
