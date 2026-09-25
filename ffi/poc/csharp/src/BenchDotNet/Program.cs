// The codec suite through BenchmarkDotNet (design/CAMPAIGN.md 22a), one launch per process.
//
//   BenchDotNet --launch N --out FILE.jsonl --artifacts DIR [--unit ARM:MODE] [--rounds R] [--warmup W]
//   BenchDotNet --launch N --list-units         (the launch's process units, in order)
//               [--iteration-ms T] [--smoke]
//
// run_campaign.sh pins this process (taskset AK_CPU_CLIENT), runs the gate first, writes
// the machine header into FILE, and calls this once per launch.

using System;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime;
using System.Runtime.InteropServices;
using BenchmarkDotNet.Columns;
using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Engines;
using BenchmarkDotNet.Jobs;
using BenchmarkDotNet.Loggers;
using BenchmarkDotNet.Running;
using BenchmarkDotNet.Toolchains.InProcess.Emit;
using Perfolizer.Horology;

namespace Armonik.Ffi.Bdn;

public static class Program
{
    private static string Opt(string[] a, string k, string d) { int i = Array.IndexOf(a, k); return i >= 0 && i + 1 < a.Length ? a[i + 1] : d; }

    public static int Main(string[] a)
    {
        int launch = int.Parse(Opt(a, "--launch", "1"), CultureInfo.InvariantCulture);
        bool smoke = a.Contains("--smoke");
        int rounds = int.Parse(Opt(a, "--rounds", smoke ? "1" : "5"), CultureInfo.InvariantCulture);
        int warm = int.Parse(Opt(a, "--warmup", smoke ? "1" : "10"), CultureInfo.InvariantCulture);
        double itMs = double.Parse(Opt(a, "--iteration-ms", smoke ? "2" : "100"), CultureInfo.InvariantCulture);
        var outp = Opt(a, "--out", "codec.jsonl");
        var art = Opt(a, "--artifacts", "BenchmarkDotNet.Artifacts");
        if (a.Contains("--list-units")) { foreach (var u in Cases.Units(launch)) Console.WriteLine(u); return 0; }
        Cases.Unit = Opt(a, "--unit", null);
        if (Cases.Unit != null && !Cases.Units(1).Contains(Cases.Unit)) { Console.Error.WriteLine("unknown unit " + Cases.Unit); return 2; }
        JitTiers.Start();

        // Requirement 26: byte identity of every timed encode arm, and every arm accepting
        // every unknown row, BEFORE anything is timed. Throws (exit non-zero) otherwise.
        int checks = Cases.Verify();
        // Requirement 24, the tier: a process-level pre-warm before BDN starts. Every case of
        // this process is called 64 times per round, rounds 0.5 s apart, until a round causes
        // no compilation of measured code (JitTiers) or 10 rounds pass. Without it the first
        // cases of a process measured hot code still at tier 0 (JOURNAL 51): tier-up waits for
        // a quiet 100 ms, which BDN's own start-up JIT activity keeps postponing.
        var (prewarmRounds, lastRoundJits) = Prewarm();
        var order = new RotatingOrderer(launch);
        int ncases = Cases.All().Count(), nprime = Cases.PrimeCases().Count();
        if (Environment.GetEnvironmentVariable("AK_BDN_TRACE") == "1")
            Console.Error.WriteLine("AKHEAP after checks and case list: {0} MB live", GC.GetTotalMemory(true) / 1000000);
        var hdr = new[]
        {
            "# engine:         BenchmarkDotNet " + typeof(BenchmarkRunner).Assembly.GetName().Version + " (CAMPAIGN.md 22a), toolchain InProcessEmit (this process, pinned by the runner)",
            "# runtime:        " + RuntimeInformation.FrameworkDescription + "; TieredCompilation=" + Env("DOTNET_TieredCompilation") + " TieredPGO=" + Env("DOTNET_TieredPGO") + " (net8.0 defaults unless set); GC server=" + GCSettings.IsServerGC + ", concurrent (default)",
            "# incumbent:      Google.Protobuf " + Ver(typeof(Google.Protobuf.MessageParser)),
            "# managed codec:  plan utf8=" + Armonik.Ffi.Facade.Codec.Utf8Policy + " unknown=" + Armonik.Ffi.Facade.Codec.UnknownMode,
            string.Format(CultureInfo.InvariantCulture, "# job:            launch {0}; {1} actual iterations (rounds) per case, {2} warm-up iterations (the same for every case) after BDN's jitting stage (1 overhead + 1 workload jitting invocation per case, logged by BDN in the .bdn.log, not exported) and pilot, iteration time {3} ms (the pilot picks the invocation count per case), strategy Throughput, EvaluateOverhead=false (no overhead subtraction anywhere)", launch, rounds, warm, itMs),
            "# clocks:         wall per iteration (BDN, Stopwatch; exported RAW, no outlier removal, no overhead subtraction); process CPU per case (getrusage(RUSAGE_SELF)) across BDN's BeforeActualRun..AfterActualRun = the actual stage including the GCs BDN forces between iterations (their pause time beside it), round 0; per-iteration and thread CPU are NOT available from BDN (no diagnoser or column gives them)",
            "# process unit:   " + (Cases.Unit ?? "all cases") + "; this launch's unit order: " + string.Join(", ", Cases.Units(launch)) + " (arms rotated by launch, modes within an arm too; requirement 22)",
            string.Format(CultureInfo.InvariantCulture, "# pre-warm:       {0} round(s) of 64 calls to every case of this process, 0.5 s apart, before BDN starts; the last round compiled {1} method(s) of measured code (JIT events read back)", prewarmRounds, lastRoundJits),
            "# correctness:    " + checks + " pre-timing checks passed (byte identity of every encode arm per payload and content set; every arm accepts every unknown row; on every unknown row core-ffi retain and host-gen retain re-encode to the incumbent's bytes, i.e. the unknown fields are kept: requirement 10)",
            "# cases:          " + ncases + " exported, after " + nprime + " prime case(s) run first and not exported (copies of the first cases, content \"prime\")",
        };
        File.AppendAllLines(outp, hdr);
        foreach (var h in hdr) Console.WriteLine(h);

        var job = Job.Default
            .WithToolchain(InProcessEmitToolchain.Instance)
            .WithStrategy(RunStrategy.Throughput)
            .WithLaunchCount(1)
            .WithWarmupCount(warm)
            .WithIterationCount(rounds)
            .WithIterationTime(TimeInterval.FromMilliseconds(itMs))
            .WithEvaluateOverhead(false)
            .WithId("campaign");
        var config = ManualConfig.CreateEmpty()
            .AddJob(job)
            .AddLogger(ConsoleLogger.Default)
            .AddColumnProvider(DefaultColumnProviders.Instance)
            .AddDiagnoser(new CpuDiagnoser())
            .AddExporter(new JsonLinesExporter(outp, launch))
            .WithOrderer(order)
            .WithArtifactsPath(art)
            .WithOptions(ConfigOptions.JoinSummary);
        var summary = BenchmarkRunner.Run<CodecSuite>(config);
        int failed = summary.Reports.Count(r => !r.Success);
        File.AppendAllLines(outp, new[]
        {
            "# jit read back:  " + JsonLinesExporter.JitQuietCases + " of " + ncases + " cases compiled nothing of the measured code in their actual stage; "
                + JsonLinesExporter.JitTier0Cases + " cases measured hot code at tier 0 (a method compiled in the case, still tier 0 at the end of its span, promoted later; per-case detail in the round-0 rows; "
                + JitTiers.Received + " MethodLoadVerbose events received)",
            "# jit check:      " + (JsonLinesExporter.JitTier0Cases == 0 ? "PASS: no exported case measured hot code at tier 0" : "FAIL: " + JsonLinesExporter.JitTier0Cases + " case(s) measured hot code at tier 0; their rows are marked by hot_tier0 > 0"),
            "# end: " + summary.Reports.Length + " BDN cases (" + nprime + " prime), " + failed + " failed",
        });
        return failed == 0 && summary.Reports.Length == ncases + nprime ? 0 : 1;
    }

    private static (int, long) Prewarm()
    {
        var keys = Cases.All().ToList();
        long before = JitTiers.Received, last = -1;
        int r = 0;
        while (r < 10)
        {
            r++;
            foreach (var k in keys)
            {
                // Through the benchmark method itself, so CodecSuite.Run tiers up too.
                var b = new CodecSuite { Case = k };
                b.Setup();
                for (int i = 0; i < 64; i++) b.Run();
            }
            // Let the tiering delay pass and the late JIT events arrive.
            long seen;
            do { seen = JitTiers.Received; System.Threading.Thread.Sleep(500); } while (seen != JitTiers.Received);
            last = JitTiers.Received - before;
            before = JitTiers.Received;
            if (last == 0) break;
        }
        return (r, last);
    }

    private static string Env(string k) { var v = Environment.GetEnvironmentVariable(k); return string.IsNullOrEmpty(v) ? "default" : v; }
    private static string Ver(Type t) =>
        (t.Assembly.GetCustomAttributes(typeof(System.Reflection.AssemblyInformationalVersionAttribute), false).FirstOrDefault()
         as System.Reflection.AssemblyInformationalVersionAttribute)?.InformationalVersion?.Split('+')[0] ?? "?";
}
