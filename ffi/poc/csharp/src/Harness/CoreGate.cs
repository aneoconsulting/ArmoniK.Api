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
using Google.Protobuf;

namespace Armonik.Ffi.Harness;

#if NET5_0_OR_GREATER

public static class CoreGate
{
    public static int Run(params string[] argv)
    {
        var rows = Manifest.Load();
        int bad = 0;

        // The layout first: by name both ways against the Rust declaration (when the
        // probe's output is given), and ABI v1 section 10 against the loaded core.
        var probe = Environment.GetEnvironmentVariable("AK_LAYOUT_PROBE");
        int lay;
        try
        {
            lay = !string.IsNullOrEmpty(probe) ? LayoutCheck.Run(probe) : AbiLayout.Facts().Count;
            if (string.IsNullOrEmpty(probe))
                Console.WriteLine("section 10 (ak_layout_facts): {0}; AK_LAYOUT_PROBE unset, the by-name check was not run",
                    lay == 0 ? "agree" : lay + " disagreement(s)");
        }
        catch (Exception ex) { Console.WriteLine("  THREW " + ex.GetType().Name + ": " + ex.Message); lay = 1; }
        if (lay != 0)
        {
            Console.WriteLine("\nlayout check failed; nothing below is meaningful");
            return 1;
        }
        Console.WriteLine();

        int chunk = 0;
        var ce = Environment.GetEnvironmentVariable("AK_CHUNK");
        if (!string.IsNullOrEmpty(ce)) int.TryParse(ce, out chunk);
        bool utf16 = Environment.GetEnvironmentVariable("AK_UTF16") == "1";

        // WP5 step 10: the binding's variant, and whether the loaded core is the same one.
        var vwhy = AbiVariant.CheckLoadedCore();
        Console.WriteLine("binding variant: {0}; loaded core: {1}", AbiVariant.Name, vwhy ?? "the same variant (checked by its u-family exports)");
        if (vwhy != null) bad++;
        // R-H22 / CAMPAIGN req 10: the no-unknown build's facade has NO unknown-field member.
        bool hasBag = typeof(ListResultsResponse).GetField("UnknownFields") != null;
        Console.WriteLine("facade member UnknownFields: {0} (this build: {1})", hasBag ? "present" : "absent", AbiVariant.Name);
        if (hasBag == AbiVariant.UnknownCompiledOut) { Console.WriteLine("  FAIL the facade does not match the build variant"); bad++; }
        Console.WriteLine("core-ffi through the C ABI, every shape. Strings are handed over as {0}.",
            utf16 ? "the host's own UTF-16 with ak_tc_utf16 (AK_UTF16=1)"
                  : "staged UTF-8 with ak_tc_bytes");
        Console.WriteLine("  chunk = {0} elements per element-entry call",
            chunk <= 0 ? "the whole run" : chunk.ToString());
        Console.WriteLine();
        Console.WriteLine("                                                   encode crossings    decode crossings");
        Console.WriteLine("                                                   encode xings  push dec     pull dec");
        Console.WriteLine("payload  root                          bytes  enc  dec  val  pull ret  R5    fwd  rev   fwd  rev   fwd  rev   pull buf");
        Console.WriteLine(new string('-', 113));

        bool coreCounts = false;
        // design/CAMPAIGN.md requirement 19: the crossing counts gate a campaign run. With
        // AK_CROSSINGS_EXPECT set, every row's six counts must equal the committed ones.
        var expect = new Dictionary<string, string>(StringComparer.Ordinal);
        var xpath = Environment.GetEnvironmentVariable("AK_CROSSINGS_EXPECT");
        bool checkCounts = !string.IsNullOrEmpty(xpath);
        if (!string.IsNullOrEmpty(xpath))
            foreach (var l in System.IO.File.ReadAllLines(xpath))
                if (l.Length != 0 && l[0] != '#') { var f = l.Split(' ', 2); expect[f[0]] = f[1]; }
        var wrote = new List<string>();
        var covered = new HashSet<string>(CoreArms.Ids, StringComparer.Ordinal);
        // WP6 step 1: P1.2 in the Latin-1 and wide content sets too (SHAPES.md), as the rust
        // slice counts them. The expected bytes are the incumbent's for the same graph (the
        // manifest has the ASCII set only); the arm's source graph is built under the set.
        var contentSets = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var (cs, name) in new[] { (Values.Latin1, "latin1"), (Values.Wide, "wide") })
        {
            if (!rows.TryGetValue("P1.2", out var baseRow)) break;
            Values.ContentSet = cs;
            var gp = BuildGp.P1_2().ToByteArray();
            Values.ContentSet = Values.Ascii;
            var cid = "P1.2/" + name;
            rows[cid] = new PayloadRow { Id = cid, Root = baseRow.Root, Bytes = gp.Length, Sha256 = Manifest.Sha(gp, gp.Length), Vector = null };
            contentSets[cid] = cs;
            covered.Add(cid);
        }
        foreach (var id in rows.Keys.OrderBy(x => x, StringComparer.Ordinal))
        {
            var row = rows[id];
            if (!covered.Contains(id)) continue;

            string enc = "-", dec = "-", val = "-", r5 = "-", pull = "-", uret = "-";
            long ef = 0, er = 0, df = 0, dr = 0, pf = 0, pr = 0, foot = 0;
            byte[] got = null;
            ICoreArm arm = null;
            try
            {
                if (contentSets.TryGetValue(id, out var cset))
                {
                    Values.ContentSet = cset;
                    try { arm = CoreArms.New(id.Substring(0, id.IndexOf('/'))); }
                    finally { Values.ContentSet = Values.Ascii; }
                }
                else arm = CoreArms.New(id);
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

                // Retain mode through the C ABI: ak_uencode_* with the (empty) bags, and
                // the push decode with the capture callbacks installed. Same bytes.
#if AK_NO_UNKNOWN_FIELDS
                // WP5 step 10: this build has unknown fields compiled out: no retain path to check.
                uret = "n/a";
#else
                try
                {
                    var ue = arm.EncodeToArrayU();
                    arm.DecodeU(ue, ue.Length);
                    var ur = arm.EncodeToArrayU();
                    uret = Manifest.Sha(ue, ue.Length) == Manifest.Sha(got, got.Length)
                        && Manifest.Sha(ur, ur.Length) == Manifest.Sha(got, got.Length) && arm.SameAsSource() ? "ok" : "RT!";
                }
                catch (Exception ex) { uret = "THREW " + ex.GetType().Name; }
                if (uret != "ok") bad++;
#endif

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
            var counts = string.Join(" ", ef, er, df, dr, pf, pr);
            wrote.Add(id + " " + counts);
            if (checkCounts && (!expect.TryGetValue(id, out var want) || want != counts))
            {
                Console.WriteLine("  {0}: CROSSINGS {1}, committed {2}", id, counts, want ?? "(none)");
                bad++;
            }
            Console.WriteLine("{0,-8} {1,-26} {2,7}  {3,-4} {4,-4} {5,-4} {6,-4} {15,-4} {7,-4} {8,4} {9,4} {10,5} {11,4} {12,5} {13,4} {14,9}",
                id, row.Root, row.Bytes, enc, dec, val, pull, r5, ef, er, df, dr, pf, pr, foot, uret);
            arm?.Dispose();
        }

        Console.WriteLine();
        var missing = rows.Keys.Where(k => !covered.Contains(k)).OrderBy(x => x, StringComparer.Ordinal).ToArray();
        if (missing.Length != 0)
            Console.WriteLine("NOT COVERED: {0}", string.Join(", ", missing));
        if (checkCounts)
        {
            // R-H3: a committed row this run did not produce FAILS, as does an empty file.
            var produced = new HashSet<string>(wrote.Select(w => w.Split(' ')[0]), StringComparer.Ordinal);
            var absent = expect.Keys.Where(k => !produced.Contains(k)).OrderBy(k => k, StringComparer.Ordinal).ToList();
            foreach (var k in absent) { Console.WriteLine("  {0}: CROSSINGS committed ({1}) but not produced by this run", k, expect[k]); bad++; }
            if (expect.Count == 0) { Console.WriteLine("  CROSSINGS: {0} has no rows: nothing to compare against", xpath); bad++; }
            Console.WriteLine("Crossing counts against {0}: {1}", xpath,
                expect.Count != 0 && absent.Count == 0 && expect.Count == wrote.Count ? "compared on every row, every committed row produced" : "INCOMPLETE (see above)");
        }
        var wr = Environment.GetEnvironmentVariable("AK_CROSSINGS_WRITE");
        if (!string.IsNullOrEmpty(wr))
            System.IO.File.WriteAllLines(wr, new[] { "# payload  encode fwd rev  push-decode fwd rev  pull-decode fwd rev (whole run per element call; counting core)" }.Concat(wrote));
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
#endif
