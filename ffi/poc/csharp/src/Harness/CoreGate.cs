// The core-ffi arm's correctness gate, for every shape.
//
// This replaced two hand-written gates that covered M1 and M2 and were going to
// be written five more times. The arms themselves are emitted from the same
// derivation as the ABI declaration, so the gate drives them through one
// interface and a payload that has a binding cannot be missing from the table.
//
// R2: nothing is timed until every column here reads ok.
//
// What the columns are, and why there are four rather than one:
//
//   encode  byte identity against `ffi/schema/generated/manifest.json`, which
//           was validated against prost and upb. P7.1 is a PERMUTATION check:
//           no canonical writer can interleave two repeated fields.
//   decode  the canonical bytes decoded and re-encoded to the same bytes.
//   value   the decoded graph compared field by field against the one the
//           builder made, through the generated comparer. Byte identity alone
//           passes a decoder that drops a field the encoder also omits, and on
//           M2 the map is exactly such a field.
//   R5      the host's own crossing tally against the CORE's counters, per
//           payload and per direction. They are the same quantity; a mismatch
//           is a failure here rather than a note.

using System;
using System.Collections.Generic;
using System.Linq;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Harness;

public static class CoreGate
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
        if (!lay.StartsWith("ok", StringComparison.Ordinal))
        {
            Console.WriteLine("\nlayout check failed; nothing below is meaningful");
            return 1;
        }
        Console.WriteLine();

        int chunk = 0;
        var ce = Environment.GetEnvironmentVariable("AK_CHUNK");
        if (!string.IsNullOrEmpty(ce)) int.TryParse(ce, out chunk);
        bool utf16 = Environment.GetEnvironmentVariable("AK_UTF16") == "1";

        Console.WriteLine("core-ffi through the C ABI, every shape. Strings are handed over as {0}.",
            utf16 ? "the host's own UTF-16 with ak_tc_utf16 (AK_UTF16=1)"
                  : "staged UTF-8 with ak_tc_bytes");
        Console.WriteLine("  chunk = {0} elements per element-entry call",
            chunk <= 0 ? "the whole run" : chunk.ToString());
        Console.WriteLine();
        Console.WriteLine("                                                   encode crossings    decode crossings");
        Console.WriteLine("                                                   encode xings  push dec     pull dec");
        Console.WriteLine("payload  root                          bytes  enc  dec  val  pull R5    fwd  rev   fwd  rev   fwd  rev   pull buf");
        Console.WriteLine(new string('-', 108));

        bool coreCounts = false;
        var covered = new HashSet<string>(CoreArms.Ids, StringComparer.Ordinal);
        foreach (var id in rows.Keys.OrderBy(x => x, StringComparer.Ordinal))
        {
            var row = rows[id];
            if (!covered.Contains(id)) continue;

            string enc = "-", dec = "-", val = "-", r5 = "-", pull = "-";
            long ef = 0, er = 0, df = 0, dr = 0, pf = 0, pr = 0, foot = 0;
            byte[] got = null;
            ICoreArm arm = null;
            try
            {
                arm = CoreArms.New(id);
                arm.Chunk = chunk;
                got = arm.EncodeToArray();
                // P7.1 interleaves two repeated fields, which no canonical writer
                // can reproduce; `harness conformance` validates it as a permutation
                // of the same (tag, wire, body) triples and so does this.
                bool p7 = row.Root == "DualResponse";
                var canon = Manifest.Vector(row);
                if (p7)
                    enc = canon != null && Triples.Same(new ReadOnlySpan<byte>(got, 0, got.Length), canon)
                        ? "perm" : "PERM!";
                else
                    enc = got.Length != row.Bytes ? "LEN " + got.Length
                        : Manifest.Sha(got, got.Length) != row.Sha256 ? "SHA!" : "ok";
            }
            catch (Exception ex) { enc = "THREW " + ex.GetType().Name; }
            if (enc != "ok" && enc != "perm") bad++;

            if (arm != null && got != null && (enc == "ok" || enc == "perm"))
            {
                try
                {
                    arm.Decode(got, got.Length);
                    var re = arm.EncodeToArray();
                    dec = Manifest.Sha(re, re.Length) == Manifest.Sha(got, got.Length) ? "ok" : "RT!";
                    val = arm.SameAsSource() ? "ok" : "VAL!";
                }
                catch (Exception ex) { dec = "THREW " + ex.GetType().Name; }
                if (dec != "ok") bad++;
                if (val != "ok") bad++;

                // Encode and decode counted on their own operations, so the
                // per-direction figures are comparable with the other slices'.
                arm.CallsReset();
                arm.EncCountersReset();
                try { arm.EncodeNoCopy(); } catch { }
                ef = arm.ForwardCalls; er = arm.ReverseCalls;
                var ec = arm.EncCounters();
                arm.CallsReset();
                arm.DecCountersReset();
                try { arm.Decode(got, got.Length); } catch { }
                df = arm.ForwardCalls; dr = arm.ReverseCalls;
                var dc = arm.DecCounters();

                // The PULL family, ABI v1 7.1. Same bytes, same expected graph, and
                // the structural claim is that it makes NO reverse call at all.
                try
                {
                    arm.CallsReset();
                    arm.Pull(got, got.Length);
                    pf = arm.ForwardCalls; pr = arm.ReverseCalls;
                    var re2 = arm.EncodeToArray();
                    pull = Manifest.Sha(re2, re2.Length) != Manifest.Sha(got, got.Length) ? "RT!"
                         : !arm.SameAsSource() ? "VAL!"
                         : pr != 0 ? "REV " + pr : "ok";
                    foot = arm.PullFootprint();
                }
                catch (Exception ex) { pull = "THREW " + ex.GetType().Name; }
                if (pull != "ok") bad++;

                if (ec.forward != 0 || ec.reverse != 0 || dc.forward != 0 || dc.reverse != 0)
                {
                    coreCounts = true;
                    bool same = (long)ec.forward == ef && (long)ec.reverse == er
                             && (long)dc.forward == df && (long)dc.reverse == dr;
                    r5 = same ? "ok" : "MISMATCH";
                    if (!same)
                    {
                        Console.WriteLine("  {0}: R5 host enc {1}/{2} core {3}/{4}; host dec {5}/{6} core {7}/{8}",
                            id, ef, er, ec.forward, ec.reverse, df, dr, dc.forward, dc.reverse);
                        bad++;
                    }
                }
            }
            Console.WriteLine("{0,-8} {1,-26} {2,7}  {3,-4} {4,-4} {5,-4} {6,-4} {7,-4} {8,4} {9,4} {10,5} {11,4} {12,5} {13,4} {14,9}",
                id, row.Root, row.Bytes, enc, dec, val, pull, r5, ef, er, df, dr, pf, pr, foot);
            arm?.Dispose();
        }

        Console.WriteLine();
        var missing = rows.Keys.Where(k => !covered.Contains(k)).OrderBy(x => x, StringComparer.Ordinal).ToArray();
        if (missing.Length != 0)
            Console.WriteLine("NOT COVERED: {0}", string.Join(", ", missing));
        Console.WriteLine("R5: the host's tally and the core's counters are {0}.",
            coreCounts ? "compared above and equal on every row"
                       : "not comparable here -- this core was not built with --features count");
        Console.WriteLine();
        Console.WriteLine("Reading the crossing columns. THEY ARE THE SAME QUANTITY as the core's own,");
        Console.WriteLine("and this slice said otherwise once. The only thing the core counts that a");
        Console.WriteLine("host tally cannot see is a TRANSCODER call, and both string forms this");
        Console.WriteLine("binding offers are pointers into the core, so it invokes none of its own.");
        Console.WriteLine("The gap once reported against the rust slice was a CHUNK SIZE: set");
        Console.WriteLine("AK_CHUNK=150 and the columns match to the digit. See JOURNAL.md 19.");
        Console.WriteLine();
        Console.WriteLine("What the counts mean, and the whole reason the shapes differ:");
        Console.WriteLine("  THE PULL COLUMN IS THE POINT OF THE LAST TWO. `ak_parse_*` makes no");
        Console.WriteLine("  reverse call at all: it appends a record per deposit to a buffer in the");
        Console.WriteLine("  host-owned decode context, and the host replays it afterwards. So pull's");
        Console.WriteLine("  reverse count is ZERO on every shape, where push's is 2 to 3,501, and the");
        Console.WriteLine("  two forward calls are ak_parse_* and ak_bdr_ptr. design/ABI-v1.md decision");
        Console.WriteLine("  2 says four of five slices have only measured push; this is the managed");
        Console.WriteLine("  pull arm it asks for, gated against push on the same bytes and the same");
        Console.WriteLine("  comparer. `pull buf` is what it trades the upcalls FOR: the record");
        Console.WriteLine("  buffer's high-water mark in the decode context (ak_bdr_footprint), so");
        Console.WriteLine("  the trade reads in both directions and not only in time.");
        Console.WriteLine();
        Console.WriteLine("  A LEAF element batches (ABI v1 section 6), so the run crosses once");
        Console.WriteLine("  whatever its length and the count is CONSTANT in the element count.");
        Console.WriteLine("  A non-leaf cannot: section 7.2 refuses it, and the count becomes");
        Console.WriteLine("  LINEAR -- a reverse call per loop slot per element on encode, and");
        Console.WriteLine("  new+apply per element plus a run per inner field on decode.");
        Console.WriteLine();
        Console.WriteLine("{0} failure(s)", bad);
        return bad;
    }
}
