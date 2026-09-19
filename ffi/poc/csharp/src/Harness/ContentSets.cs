// design/SHAPES.md's content sets, and the reason this slice cannot skip them.
//
// "A slice that reports one string-path number without saying which content set
// it came from has reported half a number." 174 of the real schema's 413 fields
// are strings, so the string path IS this codec, and every number in
// stage3-arms.log is an ASCII number.
//
// What a content set prices HERE is a narrowing transcoder. The host holds
// UTF-16; on ASCII the transcode is one char to one byte and the transcoder has
// nothing to do. Latin-1 and above-U+00FF give it real work. That is NOT what
// the Rust slice's content-set rows price -- a Rust String is already UTF-8, so
// there is no narrowing at all and what moves there is validation and width --
// and the two columns are not each other's comparator.
//
// There is no manifest oracle for these: `ffi/schema` emits ASCII only. So each
// set is checked the way SHAPES.md says to check it -- byte identity of every
// arm against the INCUMBENT arm, which the manifest validated on ASCII, plus a
// decode round trip per set -- and all three sets are measured in ONE process,
// so the cross-set comparison is a within-process ratio.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Threading;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Harness;

public static class ContentSets
{
    private static readonly string[] Payloads = { "P1.2", "P2.2" };

    public static int Run(params string[] argv)
    {
        int bad = 0;

        Console.WriteLine("Correctness first: every arm byte-identical to the incumbent, per set.");
        Console.WriteLine();
        Console.WriteLine("set     payload   wire bytes   x ascii   gp-wto  managed  2pass  man-roundtrip");
        Console.WriteLine(new string('-', 88));

        var asciiBytes = new Dictionary<string, int>();
        foreach (int cs in new[] { Values.Ascii, Values.Latin1, Values.Wide })
        {
            Values.ContentSet = cs;
            foreach (var pid in Payloads)
            {
                var a = ArmTable.All().First(x => x.Id == pid);
                a.Build();

                var gp = a.GpToByteArray();
                if (cs == Values.Ascii) asciiBytes[pid] = gp.Length;

                var dst = new byte[gp.Length + 4096];
                int n = a.GpWriteTo(dst);
                string wto = (n == gp.Length && Same(dst, n, gp)) ? "ok" : "DIFFERS";

                var e = Enc.New(Codec.Sites, gp.Length + 4096);
                a.ManagedWrite(ref e);
                string man = (e.Pos == gp.Length && Same(e.Buf, e.Pos, gp)) ? "ok" : "DIFFERS";

                var e2 = Enc.New(Codec.Sites, gp.Length + 4096);
                a.ManagedWriteSized(ref e2);
                string two = (e2.Pos == gp.Length && Same(e2.Buf, e2.Pos, gp)) ? "ok" : "DIFFERS";

                var rt = a.ManagedRoundTrip(gp, gp.Length);
                string rts = (rt.Length == gp.Length && Same(rt, rt.Length, gp)) ? "ok" : "DIFFERS";

                if (wto != "ok" || man != "ok" || two != "ok" || rts != "ok") bad++;

                Console.WriteLine("{0,-7} {1,-9} {2,10}   {3,7:F3}   {4,-6}  {5,-7}  {6,-5}  {7}",
                    Values.SetNames[cs], pid, gp.Length,
                    (double)gp.Length / asciiBytes[pid], wto, man, two, rts);
            }
        }
        Console.WriteLine();
        if (bad != 0)
        {
            Console.WriteLine("{0} FAILURE(S). Nothing below is timed.", bad);
            return bad;
        }

        // ---- timings, all three sets in ONE process ---------------------
        var cases = new List<Case>();
        foreach (int cs in new[] { Values.Ascii, Values.Latin1, Values.Wide })
        {
            Values.ContentSet = cs;
            foreach (var pid in Payloads)
            {
                var a = ArmTable.All().First(x => x.Id == pid);
                a.Build();
                var arms = a;
                int cap = asciiBytes[pid] * 4 + 8192;
                var dst = new byte[cap];
                var e = Enc.New(Codec.Sites, cap);
                var e2 = Enc.New(Codec.Sites, cap);
                var e3 = Enc.New(Codec.Sites, cap);
                arms.ManagedWrite(ref e3);
                var src = e3.ToArray();
                int slen = src.Length;
                string set = Values.SetNames[cs];

                cases.Add(new Case { Payload = pid + " " + set, Dir = "encode", Arm = "gp-writeto",
                    Run = n => { for (int i = 0; i < n; i++) Sink(arms.GpWriteTo(dst)); } });
                cases.Add(new Case { Payload = pid + " " + set, Dir = "encode", Arm = "managed",
                    Run = n => { for (int i = 0; i < n; i++) { e.Reset(); arms.ManagedWrite(ref e); Sink(e.Pos); } } });
                cases.Add(new Case { Payload = pid + " " + set, Dir = "encode", Arm = "managed-2pass",
                    Run = n => { for (int i = 0; i < n; i++) { e2.Reset(); arms.ManagedWriteSized(ref e2); Sink(e2.Pos); } } });
                cases.Add(new Case { Payload = pid + " " + set, Dir = "decode", Arm = "gp-parse",
                    Run = n => { for (int i = 0; i < n; i++) Sink(arms.GpParse(src, slen)); } });
                cases.Add(new Case { Payload = pid + " " + set, Dir = "decode", Arm = "managed-parse",
                    Run = n => { for (int i = 0; i < n; i++) Sink(arms.ManagedParse(src, slen)); } });
            }
        }

        Console.WriteLine("Timings. All three sets in ONE process, interleaved, {0} cases.", cases.Count);
        Console.WriteLine();
        Bench.WarmCalibrateMeasure(cases);

        Console.WriteLine("payload        dir     arm              reps      med ns   /incumbent   x its own ascii");
        Console.WriteLine(new string('-', 100));
        var ascii = cases.Where(c => c.Payload.EndsWith("ascii", StringComparison.Ordinal))
                         .ToDictionary(c => c.Payload.Split(' ')[0] + "|" + c.Dir + "|" + c.Arm, c => c.Med);
        foreach (var g in cases.GroupBy(c => c.Payload + "|" + c.Dir))
        {
            var list = g.ToList();
            var bl = list.First(c => c.Arm.StartsWith("gp-", StringComparison.Ordinal));
            foreach (var c in list)
            {
                string key = c.Payload.Split(' ')[0] + "|" + c.Dir + "|" + c.Arm;
                double own = ascii.TryGetValue(key, out var v) && v > 0 ? c.Med / v : 0;
                Console.WriteLine("{0,-14} {1,-6}  {2,-15} {3,6}  {4,10:F1}   {5,10:F3}   {6,15}",
                    c.Payload, c.Dir, c.Arm, c.Reps, c.Med, c.Med / bl.Med,
                    own > 0 ? own.ToString("F3", CultureInfo.InvariantCulture) : "-");
            }
            Console.WriteLine();
        }

        Console.WriteLine("Reading this table.");
        Console.WriteLine("  /incumbent is formed inside this process against the gp arm of the SAME set,");
        Console.WriteLine("    which is the only comparison that means anything across sets: the payloads");
        Console.WriteLine("    are different sizes, so an absolute moves for that reason alone.");
        Console.WriteLine("  `x its own ascii` is R4's within-arm form: each arm against ITSELF on ascii.");
        Console.WriteLine("    The Rust slice found that a content-set finding expressed this way");
        Console.WriteLine("    reproduces almost exactly where the ratio to a third arm drifts.");
        Console.WriteLine("  The wire grows with the set, so part of `x its own ascii` is simply more");
        Console.WriteLine("    bytes. The correctness table above gives the width factor per payload, and");
        Console.WriteLine("    an arm whose time grows FASTER than its bytes is paying for the transcode.");
        Values.ContentSet = Values.Ascii;
        return 0;
    }

    private static bool Same(byte[] a, int n, byte[] b)
    {
        if (n != b.Length) return false;
        for (int i = 0; i < n; i++) if (a[i] != b[i]) return false;
        return true;
    }

    private static long _s;

    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    private static void Sink(int v) { _s += v; }
}
