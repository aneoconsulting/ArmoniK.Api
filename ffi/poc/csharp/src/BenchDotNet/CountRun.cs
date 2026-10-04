// CAMPAIGN req 19 as amended (R-H31): the crossing counts of every core-ffi case the codec
// suite times, from the COUNTING build (`/p:AkHostCount=true`, AK_HOST_COUNT: every generated
// import counts its own calls by name, poc/codec/gen/cs_binding.py).
//
//   BenchDotNet --counts FILE        (the counting build only; gen/gate.sh compares FILE with
//                                     gen/counts.txt or gen/counts-nounk.txt)
//
// Per case: the case's own timed operation (Cases.Build, the same closure BenchmarkDotNet
// times) is called once untimed, so contexts exist as in the steady state, then the counters
// are zeroed and it is called ONCE more and read:
//   fwd    every exported entry point the operation called (host to core), resets included;
//   rev    the core's calls back into the host (the loop and event callbacks, host tally);
//   grow   of rev, none: the grow callback is counted apart (decision 11's ak_grow_fn), with
//          the timed build's geometric grow (CAMPAIGN req 19 as amended, rule 8);
//   reset  of fwd, the ak_dec_reset_<Root> calls: ONE per decode, BEFORE it (the options
//          armed in retain, NULL in drop; decision 11 rule 7 as amended);
//   then every entry point by name with its count.
// Retain mode starts every decode with no pre-placed buffer (the options name the grow and
// hold no buffer, cs_host.py Arm), so its grows are counted from zero on every decode.
// A pool input's rows run on a pool of 4 graphs here: the count of one call does not depend
// on which graph of the pool it encodes.

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace Armonik.Ffi.Bdn;

public static class CountRun
{
#if AK_NO_UNKNOWN_FIELDS
    private static long Grows() => 0;   // no grow callback exists in the no-unknown build
#else
    private static long Grows() => Armonik.Ffi.Harness.UnkHost.Grows;
#endif

    public static int Run(string path)
    {
#if !AK_HOST_COUNT
        Console.Error.WriteLine("--counts needs the counting build (dotnet build -p:AkHostCount=true)");
        return 2;
#else
        var vwhy = Armonik.Ffi.Harness.AbiVariant.CheckLoadedCore();
        if (vwhy != null) { Console.Error.WriteLine("core variant mismatch: " + vwhy); return 3; }
#if !AK_NO_UNKNOWN_FIELDS
        // CAMPAIGN req 19 as amended: the counting build grows geometrically, as the timed one.
        // A GATE CONTROL, not a mode: AK_COUNT_GROW=exact grows to the exact size asked, and the
        // retain rows with unknown fields must then differ from the committed counts.
        Armonik.Ffi.Harness.UnkHost.Exact = Environment.GetEnvironmentVariable("AK_COUNT_GROW") == "exact";
#endif
        Cases.PoolCap = 4;
        var o = new List<string>
        {
            "# CAMPAIGN req 19 (R-H31): per core-ffi case of the codec suite (" + Armonik.Ffi.Harness.AbiVariant.Name + " build), one call of the timed operation, counted by name in the host (AK_HOST_COUNT).",
            "# fields: payload content arm dir mode | fwd N (every exported entry point called, resets included) rev N (core->host callbacks, grow excluded) grow N (ak_grow_fn calls, geometric grow as timed) reset N (of fwd: ak_dec_reset_<Root>, one per decode, before it) | entry=count ...",
        };
        // D21: under a string path other than the incumbent E0 (AK_STR_ENC), the file says which,
        // and E2's rows carry `tc N`: of rev, the core's calls into the C# transcoder (TcManaged),
        // one per non-empty string the encode hands it; E1's and ETH's carry `pin N`, the strings
        // handed pinned (no boundary call: the core's own transcoder). Under E0 the file is unchanged.
        // D21 step 7: E3 / E3L rows carry `tc N` (of rev, as E2) and `u16 N` (`u16len N` for E3L):
        // the callback's calls into the core's additive exports ak_utf16_to_utf8 /
        // ak_utf16_utf8_len (forward calls the ABI does not count, so not in fwd); E1R / E1C rows
        // carry `mark N patch N` (strings the fill marked, marks a frame patched: equal), `hpin N`
        // (map strings on the GCHandle fallback); E1R's stack bytes per frame go to FILE.frames.
        var M = Armonik.Ffi.Harness.Stage.Mode;
        bool e2 = M == Armonik.Ffi.Harness.Stage.E2;
        bool e3 = M == Armonik.Ffi.Harness.Stage.E3 || M == Armonik.Ffi.Harness.Stage.E3L;
        bool defer = M == Armonik.Ffi.Harness.Stage.E1R || M == Armonik.Ffi.Harness.Stage.E1C;
        bool pinm = M == Armonik.Ffi.Harness.Stage.E1 || M == Armonik.Ffi.Harness.Stage.ETH;
        if (M != Armonik.Ffi.Harness.Stage.E0)
            o.Insert(1, "# string encode path (D21, AK_STR_ENC): " + Armonik.Ffi.Harness.Stage.ModeName
                + (e2 ? "; `tc N` = of rev, the core's calls into the C# transcoder (one per non-empty string)"
                   : e3 ? "; `tc N` = of rev, the core's calls into the C# transcoder (one per string); `u16 N` / `u16len N` = the transcoder's calls into ak_utf16_to_utf8 / ak_utf16_utf8_len (additive exports, not in fwd)"
                   : defer ? "; `mark N patch N` = strings marked by the fill / patched by a frame; `rstr N` / `mstr N` = of patch, the strings of repeated string fields (one frame per string) / of nested maps (key and value, one frame per entry); `hpin N` = map strings pinned by GCHandle (fallback); fwd includes the chunked element and ak_blob_run calls (K = " + Armonik.Ffi.Harness.Stage.PinK + ")"
                   : "; `pin N` = the strings handed pinned as UTF-16 (ak_tc_utf16), not a boundary call"));
        // E1R's stack bytes per recursion frame depend on the JIT tier the frame ran at, so they are
        // written apart (FILE.frames), not in the compared rows.
        var frames = new List<string> { "# E1R: stack bytes per recursion frame (largest in the case), counting build, JIT tier as run (DOTNET_TieredCompilation=" + (Environment.GetEnvironmentVariable("DOTNET_TieredCompilation") ?? "default") + ")" };
        int n = 0;
        foreach (var k in Cases.CountKeys())
        {
            var c = Case.Parse(k);
            if (!c.Arm.StartsWith("core-ffi", StringComparison.Ordinal)) continue;
            var op = Cases.Build(c);
            var ops = Cases.LastOps;
            op();
            Armonik.Ffi.Harness.Abi.EntryReset();
            ops.FfiCallsReset();
            Armonik.Ffi.Harness.Stage.TcCalls = 0;   // D21 E2: the C# transcoder's reverse calls
            Armonik.Ffi.Harness.Stage.PinCalls = 0;  // D21 E1 / ETH: the strings handed pinned
            Armonik.Ffi.Harness.Stage.U16Calls = 0; Armonik.Ffi.Harness.Stage.U16LenCalls = 0;
            Armonik.Ffi.Harness.Stage.Marked = 0; Armonik.Ffi.Harness.Stage.Patched = 0;
            Armonik.Ffi.Harness.Stage.RepPatched = 0; Armonik.Ffi.Harness.Stage.MapPatched = 0;
            Armonik.Ffi.Harness.Stage.HandlePins = 0; Armonik.Ffi.Harness.Stage.FrameBytes = 0;
#if !AK_NO_UNKNOWN_FIELDS
            Armonik.Ffi.Harness.UnkHost.Grows = 0;
#endif
            op();
            var entries = Armonik.Ffi.Harness.Abi.EntryCounts().OrderBy(e => e.Name, StringComparer.Ordinal).ToList();
            long fwd = entries.Sum(e => e.Calls);
            long resets = entries.Where(e => e.Name.StartsWith("ak_dec_reset_", StringComparison.Ordinal)).Sum(e => e.Calls);
            // The host's own reset tally (cs_host's ResetCalls) must be the imports' count.
            if (resets != ops.FfiResets())
            {
                Console.Error.WriteLine("reset tally mismatch on " + k + ": imports " + resets + ", host " + ops.FfiResets());
                return 1;
            }
            o.Add(string.Format(System.Globalization.CultureInfo.InvariantCulture, "{0} {1} {2} {3} {4} | fwd {5} rev {6} grow {7} reset {8}{10} | {9}",
                c.Payload, c.Content, c.Arm, c.Dir, c.Mode, fwd, ops.FfiReverse() + Armonik.Ffi.Harness.Stage.TcCalls, Grows(), resets,
                string.Join(" ", entries.Select(e => e.Name + "=" + e.Calls)), e2 ? " tc " + Armonik.Ffi.Harness.Stage.TcCalls
                : pinm ? " pin " + Armonik.Ffi.Harness.Stage.PinCalls
                : e3 ? " tc " + Armonik.Ffi.Harness.Stage.TcCalls + " u16 " + Armonik.Ffi.Harness.Stage.U16Calls + (M == Armonik.Ffi.Harness.Stage.E3L ? " u16len " + Armonik.Ffi.Harness.Stage.U16LenCalls : "")
                : defer ? " mark " + Armonik.Ffi.Harness.Stage.Marked + " patch " + Armonik.Ffi.Harness.Stage.Patched + " rstr " + Armonik.Ffi.Harness.Stage.RepPatched + " mstr " + Armonik.Ffi.Harness.Stage.MapPatched + " hpin " + Armonik.Ffi.Harness.Stage.HandlePins

                : ""));
            if (!pinm && !defer && Armonik.Ffi.Harness.Stage.PinCalls != 0) { Console.Error.WriteLine("string pinned outside E1 / ETH / the E1R-E1C map fallback on " + k); return 1; }
            if (!e2 && !e3 && Armonik.Ffi.Harness.Stage.TcCalls != 0) { Console.Error.WriteLine("C# transcoder called outside E2 / E3 on " + k); return 1; }
            if (M == Armonik.Ffi.Harness.Stage.E1R) frames.Add(c.Payload + " " + c.Content + " " + c.Dir + " " + c.Mode + " frame " + Armonik.Ffi.Harness.Stage.FrameBytes);
            if (Armonik.Ffi.Harness.Stage.Marked != Armonik.Ffi.Harness.Stage.Patched) { Console.Error.WriteLine("E1R/E1C: " + Armonik.Ffi.Harness.Stage.Marked + " marked, " + Armonik.Ffi.Harness.Stage.Patched + " patched on " + k); return 1; }
            n++;
        }
        File.WriteAllLines(path, o);
        if (frames.Count > 1) File.WriteAllLines(path + ".frames", frames);
        Console.WriteLine("counts: {0} core-ffi cases written to {1}", n, path);
        return n > 0 ? 0 : 1;
#endif
    }
}
