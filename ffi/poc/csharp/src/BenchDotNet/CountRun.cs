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
//          UnkHost.Exact set, so every grow allocates exactly what the core asked for;
//   reset  of fwd, the ak_dec_reset_<Root> calls: two per decode, one BEFORE it (the options
//          armed in retain, NULL in drop) and one AFTER it (NULL);
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
        // A GATE CONTROL, not a mode: AK_COUNT_GROW=doubling keeps the timed runs' growth policy,
        // and the retain rows with unknown fields must then differ from the committed counts.
        Armonik.Ffi.Harness.UnkHost.Exact = Environment.GetEnvironmentVariable("AK_COUNT_GROW") != "doubling";
#endif
        Cases.PoolCap = 4;
        var o = new List<string>
        {
            "# CAMPAIGN req 19 (R-H31): per core-ffi case of the codec suite (" + Armonik.Ffi.Harness.AbiVariant.Name + " build), one call of the timed operation, counted by name in the host (AK_HOST_COUNT).",
            "# fields: payload content arm dir mode | fwd N (every exported entry point called, resets included) rev N (core->host callbacks, grow excluded) grow N (ak_grow_fn calls, exact-size) reset N (of fwd: ak_dec_reset_<Root>, one before and one after each decode) | entry=count ...",
        };
        int n = 0;
        foreach (var k in Cases.All())
        {
            var c = Case.Parse(k);
            if (!c.Arm.StartsWith("core-ffi", StringComparison.Ordinal)) continue;
            var op = Cases.Build(c);
            var ops = Cases.LastOps;
            op();
            Armonik.Ffi.Harness.Abi.EntryReset();
            ops.FfiCallsReset();
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
            o.Add(string.Format(System.Globalization.CultureInfo.InvariantCulture, "{0} {1} {2} {3} {4} | fwd {5} rev {6} grow {7} reset {8} | {9}",
                c.Payload, c.Content, c.Arm, c.Dir, c.Mode, fwd, ops.FfiReverse(), Grows(), resets,
                string.Join(" ", entries.Select(e => e.Name + "=" + e.Calls))));
            n++;
        }
        File.WriteAllLines(path, o);
        Console.WriteLine("counts: {0} core-ffi cases written to {1}", n, path);
        return n > 0 ? 0 : 1;
#endif
    }
}
