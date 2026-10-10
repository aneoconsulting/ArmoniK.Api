// s16 (owner, 2026-10-10): reset-on-entry measured in ONE PROCESS, as the Rust slice's
// roe_bench does. Compiled only in the AkRoeBench build (#if AK_ROE_BENCH), run on a core built
// with the reset-on-entry feature. For every core-ffi case of the core grid's encode-core-hot
// and decode-read rows (drop and retain; E0 encode, FSM decode), three paths are timed in
// interleaved rounds (the order rotated each round), process CPU per op over a block:
//   E   the case's own delegate (Cases.Build: Ops_<Root>.EncFfiCore / DecFfi, the DEFAULT
//       binding, which resets explicitly), with the core's reset-on-entry switched OFF on that
//       binding's contexts (ak_measure_{enc,dec}_set_roe), so they behave as the default core's;
//   R   the same call through the binding rendered with reset_on_entry=True (no ak_enc_reset;
//       ak_dec_reset_<Root> only when the options pointer changes), contexts left ON;
//   E2  E again: the A/A control, the noise floor of a paired difference.
// Before the timing: the bytes (encode) or host-gen's re-encoding of the decoded graph
// (decode) are the same on both paths, and the resets per call are recorded. Then one
// ak_enc_reset and one ak_dec_reset_<Root> (drop: NULL; retain: options naming the grow at
// every position) timed alone. CONTAINER INSTRUMENTATION.
#if AK_ROE_BENCH
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Armonik.Ffi.Campaign;
using Armonik.Ffi.Harness;

namespace Armonik.Ffi.Bdn;

public static class RoeBench
{
    public static unsafe int Run(string outp, int rounds, double blockMs, int warmMs)
    {
        Armonik.Ffi.HarnessRoe.RoeCore.Check();
        var onlyEnv = Environment.GetEnvironmentVariable("AK_ROE_ONLY");
        var only = string.IsNullOrWhiteSpace(onlyEnv) ? null : new HashSet<string>(onlyEnv.Split(','), StringComparer.Ordinal);
        var keys = Cases.All().Where(k =>
        {
            var c = Case.Parse(k);
            return c.Arm == "core-ffi" && (c.Dir == "encode-core-hot" || c.Dir == "decode-read") && (only == null || only.Contains(c.Payload));
        }).Distinct().ToList();
        var lines = new List<string> { "payload\tcontent\tdir\tmode\tpath\tround\tcpu_ns_per_op\tops\tresets_per_call" };
        var paths = new[] { "E", "R", "E2" };
        int bad = 0;
        foreach (var k in keys)
        {
            var c = Case.Parse(k);
            bool enc = Cases.IsEnc(c.Dir), retain = c.Mode == "retain", read = c.Dir == "decode-read";
            var fE = Cases.Build(c);
            var ops = Cases.LastOps;
            byte[] wire = enc ? null : c.Content == "corpus" ? Cases.Row(c.Payload).Item2 : Cases.DecodeWire(c.Payload, ops);
            var roe = RoeOps.For(ops.Root);
            roe.Bind(ops);
            Func<long> fR = enc ? () => { ops.Next(); return roe.EncCoreR(retain); } : () => roe.DecR(wire, wire.Length, retain, read);
            fE(); fR();
            roe.ExplicitRoeOff();   // after the first decode created the explicit binding's decode context
            fE(); fR();
            byte[] a = enc ? ops.EncFfiCoreBytes(retain) : roe.DecReEncE(wire, wire.Length, retain);
            byte[] b = enc ? roe.EncCoreBytesR(retain) : roe.DecReEncR(wire, wire.Length, retain);
            if (!a.AsSpan().SequenceEqual(b)) { Console.Error.WriteLine("ROE IDENTITY FAILED: " + k); bad++; continue; }
            long e0 = roe.ResetsE(), r0 = roe.ResetsR();
            fE(); fR();
            long rsE = roe.ResetsE() - e0, rsR = roe.ResetsR() - r0;
            var fs = new[] { fE, fR, fE };
            var rs = new[] { rsE, rsR, rsE };
            long Iters(Func<long> f)
            {
                long it = 1; double t;
                while (true)
                {
                    long w0 = ProcCpu.Wall();
                    for (long i = 0; i < it; i++) f();
                    t = (ProcCpu.Wall() - w0) / 1e6;
                    if (t > blockMs / 4 || it > 1 << 26) break;
                    it *= 2;
                }
                return Math.Max(1, (long)(it * blockMs / Math.Max(t, 1e-3)));
            }
            var iters = new long[3];
            long ws = ProcCpu.Wall();
            while ((ProcCpu.Wall() - ws) / 1e6 < warmMs)
                for (int q = 0; q < 3; q++) iters[q] = Iters(fs[q]);
            long sink = 0;
            for (int r = 0; r < rounds; r++)
                for (int j = 0; j < 3; j++)
                {
                    int q = (j + r) % 3;
                    var f = fs[q];
                    long n = iters[q];
                    long c0 = ProcCpu.Ns();
                    for (long i = 0; i < n; i++) sink += f();
                    long c1 = ProcCpu.Ns();
                    lines.Add(string.Join("\t", c.Payload, c.Content, c.Dir, c.Mode, paths[q], r + 1,
                        ((double)(c1 - c0) / n).ToString("F2", CultureInfo.InvariantCulture), n, rs[q]));
                }
            GC.KeepAlive(sink);
        }
        File.WriteAllLines(outp, lines);
        // One reset alone, back to back, through the default P/Invoke binding.
        var rl = new List<string> { "call\troot\topts\tround\tcpu_ns_per_call\tcalls" };
        const long N = 2_000_000;
        IntPtr ec = Armonik.Ffi.HarnessRoe.RoeTiming.NewEnc();
        for (int w = 0; w < 3; w++) Armonik.Ffi.HarnessRoe.RoeTiming.EncReset(ec, N);
        for (int r = 0; r < rounds; r++)
            rl.Add(string.Join("\t", "ak_enc_reset", "-", "-", r + 1, Armonik.Ffi.HarnessRoe.RoeTiming.EncReset(ec, N).ToString("F2", CultureInfo.InvariantCulture), N));
        foreach (var root in Armonik.Ffi.HarnessRoe.RoeTiming.Roots)
        {
            IntPtr dc = Armonik.Ffi.HarnessRoe.RoeTiming.NewDec(root, out void* opts);
            foreach (var withOpts in new[] { false, true })
            {
                void* o = withOpts ? opts : null;
                for (int w = 0; w < 3; w++) Armonik.Ffi.HarnessRoe.RoeTiming.DecReset(root, dc, o, N / 4);
                for (int r = 0; r < rounds; r++)
                    rl.Add(string.Join("\t", "ak_dec_reset_" + root, root, withOpts ? "retain" : "NULL", r + 1,
                        Armonik.Ffi.HarnessRoe.RoeTiming.DecReset(root, dc, o, N / 4).ToString("F2", CultureInfo.InvariantCulture), N / 4));
            }
        }
        File.WriteAllLines(outp + ".resets.tsv", rl);
        Console.WriteLine("roebench: " + keys.Count + " cases (" + bad + " identity failure(s)) to " + outp + "; resets to " + outp + ".resets.tsv");
        return bad == 0 ? 0 : 1;
    }
}
#endif
