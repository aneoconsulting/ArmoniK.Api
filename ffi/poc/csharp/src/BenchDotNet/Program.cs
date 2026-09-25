// The codec suite through BenchmarkDotNet (design/CAMPAIGN.md 22a), one launch per process.
//
//   BenchDotNet --launch N --out FILE.jsonl --artifacts DIR [--rounds R] [--warmup W]
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

        // Requirement 26: byte identity of every timed encode arm, and every arm accepting
        // every unknown row, BEFORE anything is timed. Throws (exit non-zero) otherwise.
        int checks = Cases.Verify();
        var order = new RotatingOrderer(launch);
        int ncases = Cases.All().Count();
        var hdr = new[]
        {
            "# engine:         BenchmarkDotNet " + typeof(BenchmarkRunner).Assembly.GetName().Version + " (CAMPAIGN.md 22a), toolchain InProcessEmit (this process, pinned by the runner)",
            "# runtime:        " + RuntimeInformation.FrameworkDescription + "; TieredCompilation=" + Env("DOTNET_TieredCompilation") + " TieredPGO=" + Env("DOTNET_TieredPGO") + " (net8.0 defaults unless set); GC server=" + GCSettings.IsServerGC + ", concurrent (default)",
            "# incumbent:      Google.Protobuf " + Ver(typeof(Google.Protobuf.MessageParser)),
            "# managed codec:  plan utf8=" + Armonik.Ffi.Facade.Codec.Utf8Policy + " unknown=" + Armonik.Ffi.Facade.Codec.UnknownMode,
            string.Format(CultureInfo.InvariantCulture, "# job:            launch {0}; {1} actual iterations (rounds) per case, {2} warm-up iterations (the same for every case) after BDN's jitting stage (1 overhead + 1 workload jitting invocation per case, logged by BDN in the .bdn.log, not exported) and pilot, iteration time {3} ms (the pilot picks the invocation count per case), strategy Throughput, EvaluateOverhead=false (no overhead subtraction anywhere)", launch, rounds, warm, itMs),
            "# clocks:         wall per iteration (BDN, Stopwatch; exported RAW, no outlier removal, no overhead subtraction); process CPU per case (getrusage(RUSAGE_SELF)) across BDN's warm-up + actual span, round 0; per-iteration and thread CPU are NOT available from BDN (no diagnoser or column gives them)",
            "# arm order:      " + string.Join(", ", order.ArmOrder) + " (rotated by launch, requirement 22)",
            "# correctness:    " + checks + " pre-timing checks passed (byte identity of every encode arm per payload and content set; every arm accepts every unknown row)",
            "# cases:          " + ncases,
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
        File.AppendAllLines(outp, new[] { "# end: " + summary.Reports.Length + " cases, " + failed + " failed" });
        return failed == 0 && summary.Reports.Length == ncases ? 0 : 1;
    }

    private static string Env(string k) { var v = Environment.GetEnvironmentVariable(k); return string.IsNullOrEmpty(v) ? "default" : v; }
    private static string Ver(Type t) =>
        (t.Assembly.GetCustomAttributes(typeof(System.Reflection.AssemblyInformationalVersionAttribute), false).FirstOrDefault()
         as System.Reflection.AssemblyInformationalVersionAttribute)?.InformationalVersion?.Split('+')[0] ?? "?";
}
