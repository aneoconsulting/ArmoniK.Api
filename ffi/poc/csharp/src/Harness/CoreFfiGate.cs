// The core-ffi arm's correctness gate. R2: nothing is timed until it passes.
//
// M1 only, encode only, for now. `ResultRaw` is a leaf, so its encode vtable is
// empty and an element costs no reverse call; the whole arm is one forward call
// into `ak_encode_ListResultsResponse`, one reverse call back out to
// `loop_results`, and one forward call to `ak_elem_ResultRaw` carrying the whole
// batch. Three crossings for a thousand elements, which is the property ABI v1's
// batching predicate exists to create and the thing this arm is here to check.

using System;
using System.Linq;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Harness;

public static class CoreFfiGate
{
    public static int Run(params string[] argv)
    {
        var rows = Manifest.Load();
        int bad = 0;

        Console.WriteLine("Layout agreement (ABI v1 obligation 12.3, as far as it goes here):");
        string lay;
        try { lay = AbiLayout.Check(); }
        catch (Exception ex) { lay = "THREW " + ex.GetType().Name + ": " + ex.Message; }
        Console.WriteLine("  {0}", lay);
        if (!lay.StartsWith("ok", StringComparison.Ordinal)) { Console.WriteLine("\nlayout check failed; nothing below is meaningful"); return 1; }
        Console.WriteLine();

        Console.WriteLine("M1 through the C ABI, against the validated manifest. Encode is byte");
        Console.WriteLine("identity; decode is a re-encode to the same bytes AND a field-by-field");
        Console.WriteLine("comparison against the graph the builder made, because a re-encode alone");
        Console.WriteLine("passes a decoder that drops a field the encoder also omits.");
        Console.WriteLine();
        Console.WriteLine("                                                      host      core (R5)");
        Console.WriteLine("payload  elements    bytes  encode  decode  value   fwd  rev    fwd   rev  note");
        Console.WriteLine(new string('-', 92));

        int chunk = 0;
        var ce = Environment.GetEnvironmentVariable("AK_CHUNK");
        if (!string.IsNullOrEmpty(ce)) int.TryParse(ce, out chunk);
        Console.WriteLine("  chunk = {0} elements per ak_elem_* call",
            chunk <= 0 ? "the whole run" : chunk.ToString());
        Console.WriteLine();

        foreach (var id in new[] { "P1.1", "P1.2", "P1.3" })
        {
            var row = rows[id];
            var arms = ArmTable.All().First(a => a.Id == id);
            arms.Build();
            // The same generated builder every other arm uses, so the core-ffi arm
            // and the managed arm encode the identical object graph (R1).
            ListResultsResponse facade = id switch
            {
                "P1.1" => BuildFacade.P1_1(),
                "P1.2" => BuildFacade.P1_2(),
                "P1.3" => BuildFacade.P1_3(),
                _ => null,
            };
            if (facade == null) { Console.WriteLine("{0,-8} no facade builder", id); bad++; continue; }

            // Staging has to hold every string of every element at once, because the
            // batch is handed over whole. Sized from the payload, generously.
            using var core = new CoreFfiM1(facade.Results.Count + 1, row.Bytes * 3 + 65536);
            core.Chunk = chunk;
            string enc = "-", dec = "-", val = "-";
            long fwd = 0, rev = 0;
            AkCounters cc = default;
            byte[] got = null;
            try
            {
                got = core.EncodeToArray(facade);
                string sha = Manifest.Sha(got, got.Length);
                enc = got.Length != row.Bytes ? "LEN " + got.Length
                    : sha != row.Sha256 ? "SHA!" : "ok";
            }
            catch (Exception ex) { enc = "THREW " + ex.GetType().Name; }
            if (enc != "ok") bad++;

            // Decode the canonical bytes back and re-encode: byte identity only
            // covers one direction, so the decoded graph is ALSO compared field by
            // field against the one the builder made, through the same generated
            // comparer every other arm uses.
            if (got != null && enc == "ok")
            {
                try
                {
                    var back = core.Decode(got, got.Length);
                    var re = core.EncodeToArray(back);
                    dec = Manifest.Sha(re, re.Length) == row.Sha256 ? "ok" : "RT!";
                    val = Eq.SameListResultsResponse(back, facade) ? "ok" : "VAL!";
                }
                catch (Exception ex) { dec = "THREW " + ex.GetType().Name; }
                if (dec != "ok") bad++;
                if (val != "ok") bad++;
            }
            fwd = core.ForwardCalls; rev = core.ReverseCalls;
            cc = core.EncCounters();
            Console.WriteLine("{0,-8} {1,8} {2,8}  {3,-6}  {4,-6}  {5,-5}  {6,4} {7,4}  {8,5} {9,5}  {10}",
                id, row.Elements, row.Bytes, enc, dec, val, fwd, rev,
                cc.forward, cc.reverse,
                id == "P1.3" ? "absent path" : "");
        }

        Console.WriteLine();
        Console.WriteLine("Reading the crossing columns. THE TWO COLUMNS ARE THE SAME QUANTITY, and");
        Console.WriteLine("this slice said otherwise once and was wrong; the correction is here so the");
        Console.WriteLine("report does not repeat it.");
        Console.WriteLine();
        Console.WriteLine("  core (R5) is the CORE's own counters, read through ak_enc_counters. Zero");
        Console.WriteLine("    unless the core was built with --features count.");
        Console.WriteLine("  host is this binding's own tally of the calls it issued.");
        Console.WriteLine("  They AGREE, entry point for entry point. The only quantity in one and not");
        Console.WriteLine("    the other is a TRANSCODER invocation, which the core counts as reverse");
        Console.WriteLine("    and a host tally cannot see -- and this binding stages its strings as");
        Console.WriteLine("    UTF-8 with ak_tc_bytes(), a pointer INTO the core, so it makes none.");
        Console.WriteLine();
        Console.WriteLine("  The gap this slice once reported against the rust slice was a CHUNK SIZE.");
        Console.WriteLine("  Their host hands the codec 150 elements per ak_elem_* call; this one hands");
        Console.WriteLine("  over the whole run. Set AK_CHUNK=150 and the columns match to the digit,");
        Console.WriteLine("  at a cost of 0.7 percent, which is inside the spread. See JOURNAL.md 19.");
        Console.WriteLine("  They are CONSTANT in the element count, in BOTH directions, whether the");
        Console.WriteLine("  payload carries 4 elements or 1,000. The 5 and 4 cover an encode, a decode");
        Console.WriteLine("  and a re-encode:");
        Console.WriteLine("    encode     2 forward (ak_encode_*, ak_elem_*)  1 reverse (loop_results)");
        Console.WriteLine("    decode     1 forward (ak_decode_*)             2 reverse (apply, add_results)");
        Console.WriteLine("  ResultRaw is a LEAF, so encode hands the whole run over in one ak_elem_*");
        Console.WriteLine("  call and decode gets it back in one add_results. That is ABI v1's batching");
        Console.WriteLine("  predicate doing the thing it exists for, on .NET.");
        Console.WriteLine();
        Console.WriteLine("{0} failure(s)", bad);
        return bad;
    }
}
