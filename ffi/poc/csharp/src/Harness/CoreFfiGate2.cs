// The core-ffi arm's correctness gate for M2, encode and decode.
//
// M1 is a leaf, so its whole arm is three crossings for a thousand elements and
// the interesting question was what the interface costs at all. M2 is where the
// interesting question changes: `TaskDetailed` carries four repeated string
// fields and a map, so ABI v1 section 6's batching predicate does not hold and
// the codec calls back into the host per loop slot PER ELEMENT. Encode is about
// ten crossings a task; decode is section 7.2's refusal, `new` then `apply` per
// element plus a run per inner field.
//
// R2: nothing here is timed until every column reads ok.

using System;
using System.Linq;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Harness;

public static class CoreFfiGate2
{
    /// Sizing for the flat run arrays. Deliberately derived from the graph rather
    /// than guessed: an undersized array is caught by the constructor's own check
    /// and not by a wrong payload.
    public static (int blobs, int ents, int bytes) Size(ListTasksDetailedResponse r)
    {
        int blobs = 0, ents = 0, bytes = 0;
        foreach (var t in r.Tasks)
        {
            blobs += t.ParentTaskIds.Count + t.DataDependencies.Count
                   + t.ExpectedOutputIds.Count + t.RetryOfIds.Count;
            foreach (var s in t.ParentTaskIds) bytes += s.Length * 4;
            foreach (var s in t.DataDependencies) bytes += s.Length * 4;
            foreach (var s in t.ExpectedOutputIds) bytes += s.Length * 4;
            foreach (var s in t.RetryOfIds) bytes += s.Length * 4;
            if (t.Options != null)
            {
                ents += t.Options.Options.Count;
                foreach (var kv in t.Options.Options) bytes += (kv.Key.Length + kv.Value.Length) * 4;
                bytes += (t.Options.PartitionId.Length + t.Options.ApplicationName.Length
                        + t.Options.ApplicationVersion.Length + t.Options.ApplicationNamespace.Length
                        + t.Options.ApplicationService.Length + t.Options.EngineType.Length) * 4;
            }
            bytes += (t.Id.Length + t.SessionId.Length + t.OwnerPodId.Length
                    + t.StatusMessage.Length + t.PodHostname.Length + t.InitialTaskId.Length
                    + t.PayloadId.Length + t.CreatedBy.Length) * 4;
            if (t.Output != null) bytes += t.Output.Error.Length * 4;
        }
        return (blobs + 1, ents + 1, bytes + 65536);
    }

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

        int chunk = 0;
        var ce = Environment.GetEnvironmentVariable("AK_CHUNK");
        if (!string.IsNullOrEmpty(ce)) int.TryParse(ce, out chunk);

        Console.WriteLine("M2 through the C ABI. Encode is byte identity against the validated");
        Console.WriteLine("manifest; decode is a re-encode to the same bytes AND a field-by-field");
        Console.WriteLine("comparison, because a re-encode alone passes a decoder that drops a field");
        Console.WriteLine("the encoder also omits -- and on M2 the map is exactly such a field.");
        Console.WriteLine();
        Console.WriteLine("  chunk = {0} elements per ak_elemu_* call",
            chunk <= 0 ? "the whole run" : chunk.ToString());
        Console.WriteLine();
        Console.WriteLine("                                                      encode crossings      decode crossings");
        Console.WriteLine("                                                      total    per element    total    per element");
        Console.WriteLine("payload  elements    bytes  encode  decode  value   fwd   rev   fwd    rev   fwd   rev   fwd    rev");
        Console.WriteLine(new string('-', 112));

        bool coreCounts = false;
        foreach (var id in new[] { "P2.1", "P2.2", "P2.3", "P2.4", "P2.5" })
        {
            var row = rows[id];
            ListTasksDetailedResponse facade = id switch
            {
                "P2.1" => BuildFacade.P2_1(),
                "P2.2" => BuildFacade.P2_2(),
                "P2.3" => BuildFacade.P2_3(),
                "P2.4" => BuildFacade.P2_4(),
                "P2.5" => BuildFacade.P2_5(),
                _ => null,
            };
            if (facade == null) { Console.WriteLine("{0,-8} no facade builder", id); bad++; continue; }
            var (nb, ne, nbytes) = Size(facade);

            using var core = new CoreFfiM2(facade.Tasks.Count + 1, nb, ne, nbytes);
            core.Chunk = chunk;
            string enc = "-", dec = "-", val = "-";
            byte[] got = null;
            try
            {
                got = core.EncodeToArray(facade);
                string sha = Manifest.Sha(got, got.Length);
                enc = got.Length != row.Bytes ? "LEN " + got.Length
                    : sha != row.Sha256 ? "SHA!" : "ok";
            }
            catch (Exception ex) { enc = "THREW " + ex.GetType().Name + ":" + ex.Message; }
            if (enc != "ok") bad++;

            if (got != null && enc == "ok")
            {
                try
                {
                    var back = core.Decode(got, got.Length);
                    var re = core.EncodeToArray(back);
                    dec = Manifest.Sha(re, re.Length) == row.Sha256 ? "ok" : "RT!";
                    val = Eq.SameListTasksDetailedResponse(back, facade) ? "ok" : "VAL!";
                }
                catch (Exception ex) { dec = "THREW " + ex.GetType().Name; }
                if (dec != "ok") bad++;
                if (val != "ok") bad++;
            }

            // Encode and decode crossings counted separately and each on its own
            // operation, so the per-element figure is comparable with the rust
            // slice's: the columns above already ran an encode, a decode and a
            // re-encode between them.
            long f0 = core.ForwardCalls, r0 = core.ReverseCalls;
            core.EncCountersReset();
            try { core.EncodeToArray(facade); } catch { }
            long ef = core.ForwardCalls - f0, er = core.ReverseCalls - r0;
            var ec = core.EncCounters();
            f0 = core.ForwardCalls; r0 = core.ReverseCalls;
            core.DecCountersReset();
            try { if (got != null) core.Decode(got, got.Length); } catch { }
            long df = core.ForwardCalls - f0, dr = core.ReverseCalls - r0;
            var dc = core.DecCounters();
            int n = facade.Tasks.Count;

            // R5, checked rather than asserted. The core's counters and the host's
            // tally are the SAME quantity -- the only thing in one and not the other
            // is a transcoder call, and this binding stages so it makes none. If they
            // ever diverge, the host tally is the one that is wrong.
            if (ec.forward != 0 || ec.reverse != 0 || dc.forward != 0 || dc.reverse != 0)
            {
                coreCounts = true;
                if ((long)ec.forward != ef || (long)ec.reverse != er
                 || (long)dc.forward != df || (long)dc.reverse != dr)
                {
                    Console.WriteLine("  {0}: R5 MISMATCH. host encode {1}/{2} core {3}/{4}; "
                        + "host decode {5}/{6} core {7}/{8}",
                        id, ef, er, ec.forward, ec.reverse, df, dr, dc.forward, dc.reverse);
                    bad++;
                }
            }

            Console.WriteLine("{0,-8} {1,8} {2,8}  {3,-6}  {4,-6}  {5,-5} {6,5} {7,5} {8,5:F2} {9,6:F2} {10,5} {11,5} {12,5:F2} {13,6:F2}",
                id, row.Elements, row.Bytes, enc, dec, val,
                ef, er, (double)ef / n, (double)er / n,
                df, dr, (double)df / n, (double)dr / n);
        }

        Console.WriteLine();
        Console.WriteLine("The columns are the HOST's tally. The core's own ak_enc_counters read {0}",
            coreCounts ? "the same numbers, checked above and equal on every row"
                       : "zero, because this core was not built with --features count");
        Console.WriteLine();
        Console.WriteLine("Reading the crossing columns. THE TWO CONVENTIONS ARE THE SAME QUANTITY.");
        Console.WriteLine("  Where they used to disagree it was a CHUNK SIZE and not a convention:");
        Console.WriteLine("  set AK_CHUNK to what the other host chunks at and the columns match to");
        Console.WriteLine("  the digit. The core's `reverse` additionally counts transcoder calls, and");
        Console.WriteLine("  this binding stages its strings so it makes none. See JOURNAL.md 19.");
        Console.WriteLine();
        Console.WriteLine("  ENCODE, per element, with every loop slot populated:");
        Console.WriteLine("    4 repeated string fields + 1 map = 5 reverse calls, and the host calls");
        Console.WriteLine("    forward again for each run that is not empty, so about 10 crossings a");
        Console.WriteLine("    task. Constant in the element count only in the sense that it is");
        Console.WriteLine("    LINEAR in it, which is exactly what the batching predicate buys on M1");
        Console.WriteLine("    and cannot buy here.");
        Console.WriteLine("  DECODE: ABI v1 section 7.2 refuses to batch a non-leaf, so it is");
        Console.WriteLine("    new_tasks then apply_tasks per element plus one run per inner field");
        Console.WriteLine("    that occurred: 2 + 5 = 7 reverse calls a task against ONE for a whole");
        Console.WriteLine("    thousand-element M1 response.");
        Console.WriteLine();
        Console.WriteLine("{0} failure(s)", bad);
        return bad;
    }
}
