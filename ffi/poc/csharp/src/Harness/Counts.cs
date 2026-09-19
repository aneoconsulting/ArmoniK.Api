// R5, and ABI v1 open decision 5.
//
// R5 asks for boundary-call counts per payload per direction, from a counting
// build. **Every arm in this slice is in-process managed code and crosses
// nothing**, so the honest count is zero, and this file says so rather than
// leaving the column blank. The counts R5 exists for belong to the `core-ffi`
// arm, which is not built because ABI v1 open decision 1 is unsettled.
//
// What IS counted here is the thing the managed codec can be counted for: the
// learned length-placeholder width of ABI v1 section 6, and how often it
// misses. Decision 5 asks what the mechanism is worth, and an aggregate that
// cannot say WHICH site thrashes does not answer it -- so the miss is recorded
// per site, and `Codec.SiteNames` names the field.
//
// The Rust slice's answer, for comparison and not for arithmetic: zero warm
// misses on every uniform payload; on P2.4, one miss per element, moving
// 980,938 of 981,222 bytes; about 1 to 3 percentage points of an encode on the
// payload built to defeat it, and nothing on any uniform one. P2.4 is the only
// payload whose element bodies alternate across a varint length boundary.

using System;
using System.Collections.Generic;
using System.Linq;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Harness;

public static class Counts
{
    public static int Run(params string[] argv)
    {
        var rows = Manifest.Load();

        Console.WriteLine("Boundary crossings per payload per direction, every arm in this slice:");
        Console.WriteLine();
        Console.WriteLine("    gp-tobytearray   0     managed         0");
        Console.WriteLine("    gp-writeto       0     managed-2pass   0");
        Console.WriteLine("    gp-parse         0     managed-parse   0");
        Console.WriteLine();
        Console.WriteLine("  Not an estimate and not an omission: there is no boundary in any of them.");
        Console.WriteLine("  The core-ffi arm, which is the one R5's counts are about, is not built.");
        Console.WriteLine();
        Console.WriteLine("ABI v1 open decision 5: the learned length-placeholder width, and its misses.");
        Console.WriteLine();
        Console.WriteLine("payload  elements     bytes   cold misses   warm misses   warm bytes moved   grows");
        Console.WriteLine(new string('-', 98));

        var perSite = new Dictionary<string, long>();

        foreach (var a in ArmTable.All())
        {
            var row = rows[a.Id];
            a.Build();
            int cap = Math.Max(row.Bytes + 4096, 8192);
            var e = Enc.New(Codec.Sites, cap);

            // Cold: the widths all start at 1, so the first encode pays for
            // every site whose body is longer than 127 bytes.
            a.ManagedWrite(ref e);
            long cold = e.PrefixMoves;

            // Warm: the widths now carry what the last encode learned. This is
            // the steady state a server is in, and it is the number decision 5
            // is about.
            e.Reset();
            a.ManagedWrite(ref e);
            long warm = e.PrefixMoves, warmBytes = e.PrefixBytes, grows = e.Grows;

            Console.WriteLine("{0,-8} {1,8}  {2,8}  {3,12}  {4,12}  {5,17}  {6,6}",
                a.Id, a.Elements, row.Bytes, cold, warm, warmBytes, grows);

            if (warm > 0)
            {
                // Which site. Re-run with a per-site tally by encoding once per
                // site-isolating pass would cost 72 encodes; instead the site
                // names are printed for the payloads that miss, from a build
                // that tallies them directly.
                perSite[a.Id] = warm;
            }
        }

        Console.WriteLine();
        Console.WriteLine("  cold misses:  the first encode on a fresh context, every width still 1.");
        Console.WriteLine("  warm misses:  the second encode, widths carrying what the first learned. A");
        Console.WriteLine("                server is in this state; the cold column is a startup cost paid");
        Console.WriteLine("                once per context.");
        Console.WriteLine("  grows:        buffer reallocations. The harness gives the context a buffer big");
        Console.WriteLine("                enough for the payload, so a nonzero grow here is a defect in the");
        Console.WriteLine("                sizing and not a property of the codec.");
        Console.WriteLine();
        if (perSite.Count == 0)
        {
            Console.WriteLine("  No payload misses warm. Including P2.4, which is the payload BUILT to defeat");
            Console.WriteLine("  a learned width: its element bodies alternate 3 and 150 repeated strings, so");
            Console.WriteLine("  a per-call-site learned width is wrong on every element of it by construction.");
            Console.WriteLine("  If that row reads zero, read it as a defect in this harness before reading it");
            Console.WriteLine("  as a result -- see the per-site breakdown below.");
        }
        else
        {
            Console.WriteLine("  Payloads that miss warm: {0}", string.Join(", ",
                perSite.Select(kv => kv.Key + " (" + kv.Value + ")")));
        }

        Console.WriteLine();
        Console.WriteLine("Per-site warm misses on P2.4, the payload built to defeat a learned width.");
        Console.WriteLine();
        PerSite("P2.4", rows);
        Console.WriteLine();
        Console.WriteLine("Per-site warm misses on P2.2, the shape the control plane actually moves.");
        Console.WriteLine();
        PerSite("P2.2", rows);
        return 0;
    }

    /// The per-site tally, taken by encoding twice and diffing the width table:
    /// a site whose learned width CHANGED between the two encodes is a site
    /// that missed. That undercounts a site that misses and lands back on the
    /// same width, so it is a lower bound and the header says so.
    private static void PerSite(string id, Dictionary<string, PayloadRow> rows)
    {
        var a = ArmTable.All().First(x => x.Id == id);
        var row = rows[id];
        a.Build();
        var e = Enc.New(Codec.Sites, Math.Max(row.Bytes + 4096, 8192));
        a.ManagedWrite(ref e);
        var after1 = (byte[])e.Widths.Clone();
        e.Reset();
        a.ManagedWrite(ref e);
        long warm = e.PrefixMoves;

        var changed = new List<string>();
#if AK_COUNT
        Console.WriteLine("  {0} warm misses in total, tallied per site by the counting build:", warm);
        for (int i = 0; i < e.SiteMoves.Length; i++)
            if (e.SiteMoves[i] != 0)
                changed.Add(string.Format("    site {0,3}  {1,-46}  {2} miss(es), width now {3}",
                    i, Codec.SiteNames[i], e.SiteMoves[i], e.Widths[i]));
#else
        Console.WriteLine("  {0} warm misses in total. NOT a counting build, so the per-site tally is", warm);
        Console.WriteLine("  unavailable and what follows is the WIDTH DIFF, which undercounts: a site");
        Console.WriteLine("  that misses and lands back on the same width does not appear. Build with");
        Console.WriteLine("  /p:AkCount=true for the real tally.");
        for (int i = 0; i < e.Widths.Length; i++)
            if (e.Widths[i] != after1[i])
                changed.Add(string.Format("    site {0,3}  {1,-46}  width {2} -> {3}",
                    i, Codec.SiteNames[i], after1[i], e.Widths[i]));
#endif
        if (changed.Count == 0) Console.WriteLine("    none");
        else foreach (var c in changed) Console.WriteLine(c);
    }
}
