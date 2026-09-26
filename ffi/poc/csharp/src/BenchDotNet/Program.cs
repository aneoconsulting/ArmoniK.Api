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
        if (a.Contains("--counts")) return CountRun.Run(Opt(a, "--counts", "counts.txt"));
        Cases.Unit = Opt(a, "--unit", null);
        if (Cases.Unit != null && !Cases.Units(1).Contains(Cases.Unit)) { Console.Error.WriteLine("unknown unit " + Cases.Unit); return 2; }
        JitTiers.Start();

        // Requirement 26: byte identity of every timed encode arm, and every arm accepting
        // every unknown row, BEFORE anything is timed. Throws (exit non-zero) otherwise.
        // WP5 step 10: the build's unknown-field variant, and the loaded core must be the same.
        var vwhy = Armonik.Ffi.Harness.AbiVariant.CheckLoadedCore();
        if (vwhy != null) { Console.Error.WriteLine("core variant mismatch: " + vwhy); return 3; }
        int checks = Cases.Verify();
        // Requirement 24, the tier: a process-level pre-warm before BDN starts. Every case of
        // this process is called 64 times per round, rounds 0.5 s apart, until a round causes
        // no compilation of measured code (JitTiers) or 10 rounds pass. Without it the first
        // cases of a process measured hot code still at tier 0 (JOURNAL 51): tier-up waits for
        // a quiet 100 ms, which BDN's own start-up JIT activity keeps postponing.
        Cases.PoolCap = 64;   // the pre-warm's pools are small; the timed cases build full ones
        var (prewarmRounds, lastRoundJits) = Prewarm();
        Cases.PoolCap = 0;
        Cases.Pools.Clear();
        var order = new RotatingOrderer(launch);
        int ncases = Cases.All().Count(), nprime = Cases.PrimeCases().Count();
        if (Environment.GetEnvironmentVariable("AK_BDN_TRACE") == "1")
            Console.Error.WriteLine("AKHEAP after checks and case list: {0} MB live", GC.GetTotalMemory(true) / 1000000);
        var hdr = new[]
        {
            "# engine:         BenchmarkDotNet " + typeof(BenchmarkRunner).Assembly.GetName().Version + " (CAMPAIGN.md 22a), toolchain InProcessEmit (this process, pinned by the runner)",
            "# runtime:        " + RuntimeInformation.FrameworkDescription + "; TieredCompilation=" + Env("DOTNET_TieredCompilation") + " TieredPGO=" + Env("DOTNET_TieredPGO") + " (net8.0 defaults unless set); GC server=" + GCSettings.IsServerGC + ", concurrent (default)",
            "# incumbent:      Google.Protobuf " + Ver(typeof(Google.Protobuf.MessageParser)),
            "# build:          " + Armonik.Ffi.Harness.AbiVariant.Name + " (WP5 step 10; the loaded core checked to be the same variant by its u-family exports)",
            "# managed codec:  plan utf8=" + Armonik.Ffi.Facade.Codec.Utf8Policy + " unknown=" + Armonik.Ffi.Facade.Codec.UnknownMode,
            string.Format(CultureInfo.InvariantCulture, "# job:            launch {0}; {1} actual iterations (rounds) per case, {2} warm-up iterations (the same for every case) after BDN's jitting stage (1 overhead + 1 workload jitting invocation per case, logged by BDN in the .bdn.log, not exported) and pilot, iteration time {3} ms (the pilot picks the invocation count per case), strategy Throughput, EvaluateOverhead=false (no overhead subtraction anywhere)", launch, rounds, warm, itMs),
            "# clocks:         per iteration (every row with round >= 1): wall (Stopwatch) AND process CPU (CLOCK_PROCESS_CPUTIME_ID of this process: every thread, GC and JIT included; CAMPAIGN req 21 as amended, R-H25), both read by the job's clock (CpuClock, Job.WithClock) at the start and end of the same iteration window, exported RAW (no outlier removal, no overhead subtraction); BDN's forced GCs between iterations are outside the window; the round-0 row of each case is a summary of the whole actual stage (span_cpu_ns, span_wall_ns), not a sample",
            string.Format(CultureInfo.InvariantCulture, "# threads:        thread pool min {0} worker / {1} IO, max {2} worker / {3} IO, {4} pool thread(s) now; {5} logical CPUs visible; the codec suite creates no core runtime (no RPC call is made); the timed loop runs on BDN's one thread", Tp().MinW, Tp().MinIo, Tp().MaxW, Tp().MaxIo, System.Threading.ThreadPool.ThreadCount, Environment.ProcessorCount),
            string.Format(CultureInfo.InvariantCulture, "# encode:         CAMPAIGN req 11 (R-H29): dir encode = pool input + reused buffer; encode-hot = one graph + reused buffer; encode-transport = pool + the Grpc.Net form; encode-transport-hot = one graph + the Grpc.Net form. Reused buffer: incumbent-prod CalculateSize + WriteTo(IBufferWriter) and incumbent-best WriteTo(IBufferWriter) into one BufWriter, host-gen into its one Enc, core-ffi into the core's encode buffer (ak_encode_*/ak_uencode_*; that buffer is also what cell C hands the core's transport, and host-gen's Enc what cell E hands it, so their transport forms on the core's transport ARE the buffer rows). Grpc.Net form (cells A, F, D): the serializer the RPC grid's marshaller runs (RootOps.SerInc/SerHost/SerFfi), into a frame like Grpc.Net.Client 2.71's GrpcCallSerializationContext: one ArrayPool array, the 5-byte gRPC header, the body, the array returned after (GrpcFrame.cs); incumbent-best has no gRPC path and no transport row. Pool: distinct graphs built in the case's setup whose RETAINED heap is >= {0} bytes (2 x AK_LLC_BYTES, default 13.75 MB; AK_POOL_BYTES overrides; per row pool_graphs and pool_bytes), round robin one graph per call; a hot input is a pool of one (the same Next() per call); graph construction is never in the timed window", Cases.PoolBytes),
            "# U-* rows:       CAMPAIGN req 7 (R-H27): the " + Cases.UnknownRows().Count + " accepted, non-disputed U-* rows at the shapes core's 7 ABI roots, through this timed shapes core: encode-hot (a graph the arm decoded from the row, untimed; retain arms re-emit the unknown fields, drop arms the dropped form), decode, decode-read, and decode-reencode (a labelled extra; no incumbent-best row)",
            "# content sets:   CAMPAIGN req 7 (R-H26): ascii, latin1 and wide on " + string.Join(", ", Cases.ContentPayloads) + "; ascii only elsewhere",
            "# order:          seeded shuffles (below); ratios, where the aggregation forms them, come from per-launch medians (CAMPAIGN req 30): every unit is its own process",
            "# process unit:   " + (Cases.Unit ?? "all cases") + "; this launch's unit order: " + string.Join(", ", Cases.Units(launch)) + " (a seeded shuffle, seed " + Cases.UnitSeed(launch) + "; within this process the cases run in a seeded shuffle too, seed " + order.Seed + ", the 2 prime cases first; requirement 22 as amended, R-H23)",
            string.Format(CultureInfo.InvariantCulture, "# pre-warm:       {0} round(s) of 64 calls to every case of this process, 0.5 s apart, before BDN starts; the last round compiled {1} method(s) of measured code (JIT events read back)", prewarmRounds, lastRoundJits),
#if AK_NO_UNKNOWN_FIELDS
            "# correctness:    " + checks + " pre-timing checks passed (byte identity of every encode arm per payload and content set; every arm accepts every unknown row; on every unknown row core-ffi no-unknown and host-gen no-unknown re-encode to the same DROPPED form)",
#else
            "# correctness:    " + checks + " pre-timing checks passed (byte identity of every encode arm per payload and content set; every arm accepts every unknown row; on every unknown row core-ffi retain and host-gen retain re-encode to the incumbent's bytes, i.e. the unknown fields are kept: requirement 10)",
#endif
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
            .WithClock(CpuClock.Instance)
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
            "# cpu check:      " + (JsonLinesExporter.CpuPairFailed == 0 ? "PASS: every exported case has one process-CPU value per actual iteration" : "FAIL: " + JsonLinesExporter.CpuPairFailed + " case(s) without a paired process-CPU value per iteration"),
            "# end: " + summary.Reports.Length + " BDN cases (" + nprime + " prime), " + failed + " failed",
        });
        // R-H18: a JIT-check failure fails the unit (and so the launch), not only a warning.
        return failed == 0 && summary.Reports.Length == ncases + nprime && JsonLinesExporter.JitTier0Cases == 0 && JsonLinesExporter.CpuPairFailed == 0 ? 0 : 1;
    }

    private static (int MinW, int MinIo, int MaxW, int MaxIo) Tp()
    {
        System.Threading.ThreadPool.GetMinThreads(out int a, out int b);
        System.Threading.ThreadPool.GetMaxThreads(out int c, out int d);
        return (a, b, c, d);
    }

    private static (int, long) Prewarm()
    {
        // The job's clock is read inside every timed window: bring it to its final tier too.
        CpuClock.Recording = true;
        for (int i = 0; i < 20000; i++) { CpuClock.Instance.GetTimestamp(); if (CpuClock.Cpu.Count > 1000) CpuClock.Cpu.Clear(); }
        CpuClock.Recording = false;
        CpuClock.Cpu.Clear();
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
