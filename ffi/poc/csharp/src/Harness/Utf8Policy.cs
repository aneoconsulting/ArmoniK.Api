// What each arm does with malformed UTF-8 in a string field, measured.
//
// `ffi/corpus`'s transcode class has 31 `T-dec-*` reject vectors: the wire holds
// malformed UTF-8 in a string field and CONTRACT.md says a conformant parser
// must refuse it. This slice's managed codec accepts all 31, because it reads
// strings through `Encoding.UTF8`, which SUBSTITUTES U+FFFD rather than
// throwing. That is the lossy policy, it is deliberate, and STATE.md has said so
// since stage 1 -- but it was said as a design note and never as a number.
//
// The question the corpus turns into a real one: **does the INCUMBENT reject?**
// If `Google.Protobuf` refuses these bytes and the managed control accepts them,
// then the managed decode column -- the single most valuable measurement in this
// slice -- is a validating parser timed against a non-validating one, and the
// margin is partly the validation the control does not do. R14 makes that a
// defect in the comparison, not a feature of the design.
//
// The corpus reads these under `Surrogate`, a corpus-only root. The incumbent
// assembly is generated from `ffi/schema`'s `shapes.proto` and has no such
// message, so the probe reads the same bytes as `ResultRaw`, whose field 1 is
// also a string. Only the ROOT-site vectors are usable that way and that is
// what is run; the nested, map and repeated sites are named as not covered.

using System;
using System.IO;
using System.Linq;
using System.Text;
using Armonik.Ffi.Facade;
using Google.Protobuf;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Harness;

public static class Utf8Policy
{
    private static string Dir()
    {
        var d = AppContext.BaseDirectory;
        for (int i = 0; i < 12 && d != null; i++)
        {
            var c = Path.Combine(d, "ffi", "corpus", "generated", "vectors");
            if (Directory.Exists(c)) return c;
            d = Path.GetDirectoryName(d.TrimEnd(Path.DirectorySeparatorChar));
        }
        return null;
    }

    public static int Run(params string[] argv)
    {
        var dir = Dir();
        if (dir == null) { Console.WriteLine("ffi/corpus vectors not found"); return 1; }

        Console.WriteLine("Malformed UTF-8 in a string field: what each arm does with it.");
        Console.WriteLine();
        Console.WriteLine("Root-site vectors only. The corpus reads these under `Surrogate`; this");
        Console.WriteLine("reads them as `ResultRaw`, whose field 1 is also a string, because the");
        Console.WriteLine("incumbent assembly is generated from ffi/schema and has no Surrogate. The");
        Console.WriteLine("nested, map-key, map-value and repeated sites are NOT covered here.");
        Console.WriteLine();
        Console.WriteLine("vector                              incumbent                     managed control");
        Console.WriteLine(new string('-', 100));

        int gpRejects = 0, manRejects = 0, n = 0;
        foreach (var f in Directory.GetFiles(dir, "T-dec-root-*.bin").OrderBy(x => x, StringComparer.Ordinal))
        {
            var b = File.ReadAllBytes(f);
            n++;
            string gp;
            try
            {
                var m = Gp.ResultRaw.Parser.ParseFrom(new ReadOnlySpan<byte>(b));
                gp = "accepted, " + m.SessionId.Length + " chars";
            }
            catch (Exception ex) { gp = "REJECTED " + ex.GetType().Name; gpRejects++; }

            string man;
            try
            {
                var d = new Dec { Buf = b, Pos = 0, End = b.Length, Err = 0 };
                var m = new ResultRaw();
                Codec.ReadResultRaw(ref d, m, b.Length);
                man = d.Err != 0 ? "REJECTED err " + d.Err : "accepted, " + m.SessionId.Length + " chars";
                if (d.Err != 0) manRejects++;
            }
            catch (Exception ex) { man = "REJECTED " + ex.GetType().Name; manRejects++; }

            Console.WriteLine("{0,-35} {1,-29} {2}", Path.GetFileNameWithoutExtension(f), gp, man);
        }

        Console.WriteLine();
        Console.WriteLine("{0} vectors: the incumbent rejected {1}, the managed control rejected {2}.",
            n, gpRejects, manRejects);
        Console.WriteLine();
        if (gpRejects == manRejects)
        {
            Console.WriteLine("THE TWO ARMS AGREE, so the decode comparison is like for like and the");
            Console.WriteLine("managed decode margin is not bought by skipping validation. Both are");
            Console.WriteLine("non-conformant against CONTRACT.md's transcode class in the same way,");
            Console.WriteLine("which is ABI v1 open decision 3 and a policy question, not a defect in");
            Console.WriteLine("the comparison.");
        }
        else
        {
            Console.WriteLine("THE ARMS DISAGREE. R14 makes this a defect in the comparison: the");
            Console.WriteLine("managed decode column is a non-validating parser timed against a");
            Console.WriteLine("validating one, and some part of its margin is the validation it does");
            Console.WriteLine("not do. The decode figures need a validating managed arm beside them.");
        }
        Console.WriteLine();
        Console.WriteLine("For reference, what .NET's own strict decoder does with the same bodies:");
        {
            int strict = 0, tot = 0;
            foreach (var f in Directory.GetFiles(dir, "T-dec-root-*.bin").OrderBy(x => x, StringComparer.Ordinal))
            {
                var b = File.ReadAllBytes(f);
                // field 1, wire 2: one key byte, then a varint length.
                int p = 1, shift = 0, len = 0;
                while (p < b.Length) { len |= (b[p] & 0x7f) << shift; if ((b[p++] & 0x80) == 0) break; shift += 7; }
                tot++;
                try { new UTF8Encoding(false, true).GetString(b, p, len); }
                catch (DecoderFallbackException) { strict++; }
                catch { }
            }
            Console.WriteLine("  new UTF8Encoding(false, throwOnInvalidBytes: true) rejects {0} of {1}.", strict, tot);
            Console.WriteLine("  So a validating managed arm is one constructor argument away, and");
            Console.WriteLine("  pricing it is what ABI v1 decision 3 asks for on .NET.");
        }
        return 0;
    }
}
