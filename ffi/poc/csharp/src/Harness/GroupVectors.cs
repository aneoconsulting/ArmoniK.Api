// The five GROUP vectors from `ffi/corpus`, and the defect they found.
//
// proto3 cannot express a group, so no payload in `ffi/schema` contains one and
// byte identity against that manifest could never have found this. The facade's
// `Dec.Skip` had cases for the four wire types a proto3 schema produces and
// `ErrMalformed` for everything else, so it REJECTED three vectors that
// `Google.Protobuf` accepts. The shared core had the identical hole (D7, found
// by the python slice's corpus run and fixed in the core); the C++ slice still
// has it; the java slice's was already right.
//
// The fix is not "add case 3". A group carries no length, so its end is an
// END_GROUP tag whose FIELD NUMBER matches the one that opened it. A skipper
// that counts depth instead accepts `X-group-mismatched-end` and then mis-nests
// every group after it -- a wrong parse rather than a rejected one. And a
// skipper that scans for an end tag without checking the buffer walks off the
// end of `X-group-unterminated`. Both are here because both were reachable.
//
// THIS IS NOT CORPUS CONFORMANCE. `ffi/corpus/CONTRACT.md` asks a consumer to
// generate its codec from `generated/corpus.proto`, parse all 287 accept
// vectors, project them, re-encode them and watch all 49 reject vectors fail.
// This slice generates from `ffi/schema/emit/shapes.py` and has no .proto front
// end, so it is not a consumer. What runs here is the five vectors the defect
// is about, read under a root this slice does have, and it is labelled as the
// narrow thing it is.

using System;
using System.IO;
using Armonik.Ffi.Facade;
using Google.Protobuf;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Harness;

public static class GroupVectors
{
    private static string CorpusDir()
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

    private sealed class V
    {
        public string Id;
        public string Root;     // the root this slice reads it under
        public bool Accept;
        public string Note;
    }

    public static int Run(params string[] argv)
    {
        var dir = CorpusDir();
        if (dir == null)
        {
            Console.WriteLine("ffi/corpus/generated/vectors not found; group vectors SKIPPED");
            return 1;
        }

        var vs = new[]
        {
            new V { Id = "U-root-group", Root = "ListResultsResponse", Accept = true,
                Note = "a group at the top level of the message that crosses the wire" },
            new V { Id = "U-nested-group", Root = "ListResultsResponse", Accept = true,
                Note = "a group INSIDE an element of a repeated field" },
            new V { Id = "U-oneof-group", Root = "ListProbeResponse", Accept = true,
                Note = "a group on a message that actually has a oneof" },
            // The corpus reads these two under `WireZoo`, a root this slice does
            // not generate. Their only top-level field is tag 120, which is
            // unknown to ListResultsResponse as well, so reading them under it
            // exercises EXACTLY the skipper the vector is about and nothing else.
            new V { Id = "X-group-unterminated", Root = "ListResultsResponse", Accept = false,
                Note = "a group that is never closed (corpus root WireZoo; read here under "
                     + "ListResultsResponse, where tag 120 is equally unknown)" },
            new V { Id = "X-group-mismatched-end", Root = "ListResultsResponse", Accept = false,
                Note = "END_GROUP with a field number that does not match the group that "
                     + "opened (same substitution)" },
        };

        Console.WriteLine("The GROUP vectors from ffi/corpus. proto3 cannot express one, so this is");
        Console.WriteLine("the one class of unknown field the schema's own manifest cannot test.");
        Console.WriteLine();
        Console.WriteLine("vector                   bytes  expect  incumbent  managed  agree  note");
        Console.WriteLine(new string('-', 118));

        int bad = 0;
        foreach (var v in vs)
        {
            var path = Path.Combine(dir, v.Id + ".bin");
            if (!File.Exists(path)) { Console.WriteLine("{0,-22} MISSING", v.Id); bad++; continue; }
            var b = File.ReadAllBytes(path);

            // The incumbent, as the oracle. R14's library decides what conformant
            // means here, not this slice's reading of the spec.
            bool gpOk;
            try
            {
                if (v.Root == "ListProbeResponse") Gp.ListProbeResponse.Parser.ParseFrom(new ReadOnlySpan<byte>(b));
                else Gp.ListResultsResponse.Parser.ParseFrom(new ReadOnlySpan<byte>(b));
                gpOk = true;
            }
            catch { gpOk = false; }

            bool manOk;
            try
            {
                var d = new Dec { Buf = b, Pos = 0, End = b.Length, Err = 0 };
                if (v.Root == "ListProbeResponse")
                    Codec.ReadListProbeResponse(ref d, new ListProbeResponse(), b.Length);
                else
                    Codec.ReadListResultsResponse(ref d, new ListResultsResponse(), b.Length);
                manOk = d.Err == 0;
            }
            catch { manOk = false; }

            string gpS = gpOk == v.Accept ? "ok" : (gpOk ? "ACCEPTED!" : "REJECTED!");
            string manS = manOk == v.Accept ? "ok" : (manOk ? "ACCEPTED!" : "REJECTED!");
            if (gpS != "ok") bad++;
            if (manS != "ok") bad++;
            string agree = gpOk == manOk ? "yes" : "NO";
            if (agree != "yes") bad++;

            Console.WriteLine("{0,-22} {1,6}  {2,-6}  {3,-9}  {4,-7}  {5,-5}  {6}",
                v.Id, b.Length, v.Accept ? "accept" : "reject", gpS, manS, agree, v.Note);
        }

        Console.WriteLine();
        int depth = 200;
        var ne = Environment.GetEnvironmentVariable("AK_NEST");
        if (!string.IsNullOrEmpty(ne)) int.TryParse(ne, out depth);
        Console.WriteLine("And the case no vector covers, because a corpus of malformed bytes has to");
        Console.WriteLine("stop somewhere: {0} nested start tags, past the 100 the skipper allows.", depth);
        Console.WriteLine("AK_NEST sets the depth. At 200 the bound is what makes the rejection say");
        Console.WriteLine("ErrDepth rather than ErrTruncated -- an unbounded skipper still rejects");
        Console.WriteLine("this one, for the wrong reason. Deep enough and an unbounded skipper does");
        Console.WriteLine("not reject at all: it overflows the stack, which .NET cannot catch.");
        {
            // Tag 120, wire 3: key = (120 << 3) | 3 = 963, varint 0xc3 0x07.
            var nest = new byte[2 * depth];
            for (int i = 0; i < depth; i++) { nest[2 * i] = 0xc3; nest[2 * i + 1] = 0x07; }
            var d = new Dec { Buf = nest, Pos = 0, End = nest.Length, Err = 0 };
            bool threw = false;
            try { Codec.ReadListResultsResponse(ref d, new ListResultsResponse(), nest.Length); }
            catch (Exception ex) { threw = true; Console.WriteLine("  THREW {0}", ex.GetType().Name); bad++; }
            if (!threw)
            {
                Console.WriteLine("  err = {0}, {1}", d.Err,
                    d.Err == W.ErrDepth ? "ErrDepth, and the process is still here"
                                        : "EXPECTED ErrDepth (" + W.ErrDepth + ")");
                if (d.Err != W.ErrDepth) bad++;
            }
            // The incumbent's own answer to the same bytes, as the oracle.
            string gp;
            try { Gp.ListResultsResponse.Parser.ParseFrom(new ReadOnlySpan<byte>(nest)); gp = "ACCEPTED"; }
            catch (Exception ex) { gp = "rejected, " + ex.GetType().Name; }
            Console.WriteLine("  Google.Protobuf: {0}", gp);
        }

        // And the depth that actually kills an unbounded skipper on this runtime,
        // measured rather than assumed: with the bound removed, 20,000 nests still
        // return (wrongly, as ErrTruncated) and 200,000 print "Stack overflow." and
        // abort with SIGABRT, which .NET cannot catch and no gate can survive. With
        // the bound, the same bytes cost 100 frames. The cheap case above checks the
        // REASON; this one checks that the process is still here to print a reason.
        Console.WriteLine();
        Console.WriteLine("The same at 200,000, which is past where an unbounded skipper aborts:");
        {
            var deep = new byte[400000];
            for (int i = 0; i < 200000; i++) { deep[2 * i] = 0xc3; deep[2 * i + 1] = 0x07; }
            var d = new Dec { Buf = deep, Pos = 0, End = deep.Length, Err = 0 };
            Codec.ReadListResultsResponse(ref d, new ListResultsResponse(), deep.Length);
            Console.WriteLine("  err = {0}, {1}", d.Err,
                d.Err == W.ErrDepth ? "ErrDepth, and the process is still here" : "EXPECTED ErrDepth");
            if (d.Err != W.ErrDepth) bad++;
        }

        Console.WriteLine();
        Console.WriteLine("{0} failure(s)", bad);
        return bad;
    }
}
