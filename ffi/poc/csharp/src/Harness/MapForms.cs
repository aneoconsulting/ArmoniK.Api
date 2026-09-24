// P2.5 has two valid encodings, and this establishes which one .NET produces
// and that both are read.
//
// An empty map VALUE is an implicit-presence leaf holding the proto zero. The
// manifest omits it, which is prost's rule; protobuf C++ and upb write it, at
// +80 B on P2.5 (20 elements x 2 emptied entries x 2 bytes for `tag 2, len 0`).
// Neither is a defect: both are legal wire and a conformant parser reads both.
//
// The aggregating session relayed an expectation that `Google.Protobuf` writes
// it too. **It does not**, and this check is the evidence rather than the
// assertion: on .NET the incumbent produces the manifest's 19,632 B, which is
// also why stage 1 passes byte identity on P2.5 without any special case.
//
// What still had to be TESTED rather than reasoned about is the other
// direction: that both decoders accept the +80 B form and produce the same
// facade values. A decoder that initialises the map value to "" and only
// overwrites it when field 2 is present reads both by construction -- but "by
// construction" is what this directory keeps being wrong about, so the form is
// built and run.

using System;
using System.IO;
using System.Collections.Generic;
using System.Linq;
using Armonik.Ffi.Facade;
using Google.Protobuf;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Harness;

public static class MapForms
{
    private static int _inserted;

    public static int Run(params string[] argv)
    {
        var rows = Manifest.Load();
        var row = rows["P2.5"];
        var canon = Manifest.Vector(row);
        int bad = 0;

        var a = ArmTable.All().First(x => x.Id == "P2.5");
        a.Build();

        Console.WriteLine("Which form does each encoder produce on P2.5?");
        Console.WriteLine();
        Console.WriteLine("  manifest (prost's rule, empty map value OMITTED) : {0} B", row.Bytes);

        var gp = a.GpToByteArray();
        var e = Enc.New(Codec.Sites, row.Bytes + 4096);
        a.ManagedWrite(ref e);

        Console.WriteLine("  Google.Protobuf ToByteArray                      : {0} B  {1}",
            gp.Length, gp.Length == row.Bytes ? "-> OMITS it, same as the manifest" : "-> WRITES it");
        Console.WriteLine("  the generated managed codec                      : {0} B  {1}",
            e.Pos, e.Pos == row.Bytes ? "-> OMITS it, same as the manifest" : "-> WRITES it");
        Console.WriteLine();
        Console.WriteLine("  So on .NET both arms produce the manifest's form. protobuf C++ and upb");
        Console.WriteLine("  write the empty value and would produce {0} B. That is a difference", row.Bytes + 80);
        Console.WriteLine("  between runtimes, not a defect in any of them.");
        Console.WriteLine();

        // ---- build the OTHER form -------------------------------------
        _inserted = 0;
        var wide = Rewrite(canon, new[] { 1, 10, 1 }, 0);
        Console.WriteLine("The +80 B form, built by rewriting the committed vector:");
        Console.WriteLine("  empty map values inserted : {0}", _inserted);
        Console.WriteLine("  bytes                     : {0} (manifest {1}, delta {2})",
            wide.Length, canon.Length, wide.Length - canon.Length);

        bool sized = wide.Length == canon.Length + 80 && _inserted == 40;
        Console.WriteLine("  expected                  : {0} B and 40 insertions -- {1}",
            canon.Length + 80, sized ? "ok" : "MISMATCH, the rewriter built the wrong thing");
        if (!sized) bad++;
        Console.WriteLine();

        // ---- both decoders must read it, and agree with the canonical form ----
        Console.WriteLine("Does every decoder read BOTH forms, and agree on the value?");
        Console.WriteLine();
        Console.WriteLine("decoder         form      result");
        Console.WriteLine(new string('-', 64));

        foreach (var (name, data) in new[] { ("canonical", canon), ("+80 B", wide) })
        {
            string mv;
            try
            {
                mv = a.ManagedDecodesToBuiltValue(data, data.Length)
                    ? "ok, equals the built graph" : "DIFFERS from the built graph";
            }
            catch (Exception ex) { mv = "THREW " + ex.GetType().Name; }
            if (!mv.StartsWith("ok", StringComparison.Ordinal)) bad++;
            Console.WriteLine("{0,-15} {1,-9} {2}", "managed", name, mv);

            string gv;
            try
            {
                var m = Gp.ListTasksDetailedResponse.Parser.ParseFrom(new ReadOnlySpan<byte>(data));
                var re = m.ToByteArray();
                gv = Manifest.Sha(re, re.Length) == row.Sha256
                    ? "ok, re-encodes to the canonical form"
                    : "re-encodes to " + re.Length + " B, NOT the canonical form";
            }
            catch (Exception ex) { gv = "THREW " + ex.GetType().Name; }
            if (!gv.StartsWith("ok", StringComparison.Ordinal)) bad++;
            Console.WriteLine("{0,-15} {1,-9} {2}", "gp-parse", name, gv);
        }

        Console.WriteLine();
        Console.WriteLine("Reading this.");
        Console.WriteLine("  Both decoders normalise the +80 B form back to the canonical one, because");
        Console.WriteLine("  an empty map value and an absent map value are the SAME facade value and");
        Console.WriteLine("  neither encoder writes it. So the two forms round-trip to one another's");
        Console.WriteLine("  values and this slice's byte-identity gate is unaffected.");
        Console.WriteLine();
        Console.WriteLine("  A P2.5 TIMING row therefore carries the manifest's 19,632 B in every arm");
        Console.WriteLine("  of this slice, and is not comparable with a slice whose encoder writes");
        Console.WriteLine("  19,712 B: 80 bytes and 40 extra fields are not the same work.");
        Console.WriteLine();
        Console.WriteLine("{0} failure(s)", bad);
        return bad;
    }

    /// Rewrite a length-delimited tree, descending the tag path, and at the end
    /// of the path treat each element as a map entry: if it has no field 2,
    /// append an empty one. Every enclosing length prefix is rebuilt, which is
    /// the part that makes this worth writing rather than patching by hand.
    private static byte[] Rewrite(byte[] src, int[] path, int depth)
    {
        var o = new List<byte>();
        int p = 0;
        while (p < src.Length)
        {
            ulong key = ReadVarint(src, ref p);
            int tag = (int)(key >> 3), wire = (int)(key & 7);
            int start = p;
            if (wire != 2)
            {
                Skip(src, ref p, tag, wire);
                o.AddRange(Varint(key));
                o.AddRange(src.Skip(start).Take(p - start));
                continue;
            }
            // D8 / R-G8: the 64-bit prefix against the bytes LEFT, never narrowed first.
            int len = Len(src, ref p);
            var body = new byte[len];
            Buffer.BlockCopy(src, p, body, 0, len);
            p += len;

            byte[] outBody;
            if (tag == path[depth] && depth == path.Length - 1)
            {
                outBody = HasField2(body) ? body : Concat(body, new byte[] { 0x12, 0x00 });
                if (!HasField2(body)) _inserted++;
            }
            else if (tag == path[depth])
            {
                outBody = Rewrite(body, path, depth + 1);
            }
            else
            {
                outBody = body;
            }

            o.AddRange(Varint(key));
            o.AddRange(Varint((ulong)outBody.Length));
            o.AddRange(outBody);
        }
        return o.ToArray();
    }

    private static bool HasField2(byte[] entry)
    {
        int p = 0;
        while (p < entry.Length)
        {
            ulong k = ReadVarint(entry, ref p);
            if ((k >> 3) == 2) return true;
            Skip(entry, ref p, (int)(k >> 3), (int)(k & 7));
        }
        return false;
    }

    private static byte[] Concat(byte[] a, byte[] b)
    {
        var o = new byte[a.Length + b.Length];
        Buffer.BlockCopy(a, 0, o, 0, a.Length);
        Buffer.BlockCopy(b, 0, o, a.Length, b.Length);
        return o;
    }

    /// Takes the TAG as well as the wire type, for the same reason `Dec.Skip`
    /// does: a group carries no length, so its end is an END_GROUP whose field
    /// number MATCHES the one that opened it. This is a harness rewriter and
    /// only ever walks bytes this slice emitted, where proto3 cannot produce a
    /// group -- so the group arm here will never execute on any payload in the
    /// tree. It is here anyway because "this one cannot be reached" is exactly
    /// what was believed about the facade's skipper, and because a helper that
    /// silently mis-parses is worse than one that is simply correct.
    /// A length prefix, read as the full 64-bit varint and compared with the bytes
    /// left (R-G8, this slice's D8): the harness rewriter used to cast it to `int`
    /// first, the class `Dec.LenEnd` was fixed for.
    private static int Len(byte[] b, ref int p)
    {
        ulong n = ReadVarint(b, ref p);
        if (n > (ulong)(b.Length - p)) throw new InvalidDataException("length prefix " + n + " exceeds the " + (b.Length - p) + " bytes left");
        return (int)n;
    }

    private static void Skip(byte[] b, ref int p, int tag, int wire)
    {
        switch (wire)
        {
            case 0: ReadVarint(b, ref p); break;
            case 1: p += 8; break;
            case 5: p += 4; break;
            case 3: SkipGroup(b, ref p, tag, 0); break;
            // NOT `p += (int)ReadVarint(b, ref p)`. C# loads the left operand
            // of `+=` BEFORE evaluating the right, and ReadVarint advances `p`
            // itself, so that spelling adds the body length to the PRE-varint
            // offset and lands short by the width of the length prefix. It
            // parsed as wire type 4 two fields later, which is how it was
            // found. The generated decoder is unaffected: it spells this
            // `Pos = LenEnd()`.
            case 2: { int n = Len(b, ref p); p += n; break; }
            // 4 is END_GROUP with nothing open; 6 and 7 do not exist.
            default: throw new InvalidOperationException("wire type " + wire);
        }
    }

    /// protobuf's own default recursion limit, so a nest of start tags is an
    /// exception and not a stack overflow.
    private const int MaxGroupDepth = 100;

    private static void SkipGroup(byte[] b, ref int p, int tag, int depth)
    {
        if (depth >= MaxGroupDepth) throw new InvalidOperationException("group nesting past " + MaxGroupDepth);
        while (true)
        {
            if (p >= b.Length) throw new InvalidOperationException("unterminated group, tag " + tag);
            ulong k = ReadVarint(b, ref p);
            int t = (int)(k >> 3), w = (int)(k & 7);
            if (t == 0) throw new InvalidOperationException("tag 0");
            if (w == 4)
            {
                if (t != tag) throw new InvalidOperationException("END_GROUP tag " + t + " closes group " + tag);
                return;
            }
            if (w == 3) { SkipGroup(b, ref p, t, depth + 1); continue; }
            Skip(b, ref p, t, w);
        }
    }

    private static ulong ReadVarint(byte[] b, ref int p)
    {
        ulong n = 0; int sh = 0;
        while (true)
        {
            byte c = b[p++];
            n |= (ulong)(c & 0x7f) << sh;
            if ((c & 0x80) == 0) return n;
            sh += 7;
        }
    }

    private static byte[] Varint(ulong v)
    {
        var o = new List<byte>();
        while (v >= 0x80) { o.Add((byte)((byte)v | 0x80)); v >>= 7; }
        o.Add((byte)v);
        return o.ToArray();
    }
}
