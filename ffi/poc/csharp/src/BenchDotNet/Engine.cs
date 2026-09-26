// BenchmarkDotNet configured to meet design/CAMPAIGN.md requirements 21 to 28 (req 22a).
//
//   * every RAW workload measurement of the actual stage is exported, one JSON line per
//     iteration (JsonLinesExporter); BDN's outlier removal and overhead subtraction touch
//     only its own summary, never the exported rows (they carry the raw wall time);
//   * CPU: BDN measures WALL time per iteration (Stopwatch). CPU time is added by a
//     diagnoser (CpuDiagnoser) that reads getrusage(RUSAGE_SELF) across BDN's
//     BeforeActualRun..AfterActualRun span, which is the ACTUAL stage (BDN signals it after
//     the warm-up), including BDN's forced GCs between iterations (their pause time is
//     recorded beside it, gc_pause_ns): one extra row per case, round 0. Per-iteration CPU is not available
//     from BDN; that is stated in the header;
//   * warm-up: a FIXED number of warm-up iterations for every case (WithWarmupCount), after
//     BDN's own jitting stage; both counts are exported per case;
//   * the arm order is rotated per launch (RotatingOrderer, requirement 22).

using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using BenchmarkDotNet.Analysers;
using BenchmarkDotNet.Columns;
using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Diagnosers;
using BenchmarkDotNet.Engines;
using BenchmarkDotNet.Exporters;
using BenchmarkDotNet.Loggers;
using BenchmarkDotNet.Order;
using BenchmarkDotNet.Reports;
using BenchmarkDotNet.Running;
using BenchmarkDotNet.Validators;

namespace Armonik.Ffi.Bdn;

internal static unsafe class ProcCpu
{
    [StructLayout(LayoutKind.Sequential)] private struct Timeval { public long Sec, Usec; }
    [StructLayout(LayoutKind.Sequential)]
    private struct Rusage { public Timeval Utime, Stime; public long a, b, c, d, e, f, g, h, i, j, k, l, m, n; }
    [DllImport("libc")] private static extern int getrusage(int who, Rusage* ru);
    public static long Ns() { Rusage r; getrusage(0, &r); return (r.Utime.Sec + r.Stime.Sec) * 1_000_000_000L + (r.Utime.Usec + r.Stime.Usec) * 1000L; }
    public static long Wall() => (long)(System.Diagnostics.Stopwatch.GetTimestamp() * (1e9 / System.Diagnostics.Stopwatch.Frequency));
}

/// CPU time of the process across BeforeActualRun -> AfterActualRun: the actual stage and
/// BDN's forced GCs between its iterations (BDN has no signal per iteration).
public sealed class CpuDiagnoser : IDiagnoser
{
    public static readonly Dictionary<string, (DateTime T0, DateTime T1, DateTime T2)> Times = new Dictionary<string, (DateTime, DateTime, DateTime)>();
    private DateTime _t0, _t1;
    public static readonly Dictionary<string, (long Cpu, long Wall, int G0, int G1, int G2, long Heap, long Pause)> Stage = new Dictionary<string, (long, long, int, int, int, long, long)>();
    private long _c0, _w0, _heap;
    private TimeSpan _p0;
    private int _g0, _g1, _g2;
    public IEnumerable<string> Ids => new[] { "AkProcessCpu" };
    public IEnumerable<IExporter> Exporters => Array.Empty<IExporter>();
    public IEnumerable<IAnalyser> Analysers => Array.Empty<IAnalyser>();
    public RunMode GetRunMode(BenchmarkCase benchmarkCase) => RunMode.NoOverhead;
    public bool RequiresBlockingAcknowledgments(BenchmarkCase benchmarkCase) => true;
    private static readonly bool Trace = Environment.GetEnvironmentVariable("AK_BDN_TRACE") == "1";
    public void Handle(HostSignal signal, DiagnoserActionParameters parameters)
    {
        if (Trace) Console.Error.WriteLine("AKTRACE {0} {1} {2} {3} {4}", ProcCpu.Wall() / 1000000, signal, (long)GC.GetTotalPauseDuration().TotalMilliseconds, GC.CollectionCount(2), GC.GetTotalMemory(false) / 1000000);
        if (signal == HostSignal.BeforeAnythingElse) _t0 = DateTime.UtcNow;
        if (signal == HostSignal.BeforeActualRun)
        {
            _t1 = DateTime.UtcNow;
            _heap = GC.GetTotalMemory(false); _p0 = GC.GetTotalPauseDuration(); _g0 = GC.CollectionCount(0); _g1 = GC.CollectionCount(1); _g2 = GC.CollectionCount(2);
            _w0 = ProcCpu.Wall(); _c0 = ProcCpu.Ns();
        }
        else if (signal == HostSignal.AfterActualRun)
        {
            long c1 = ProcCpu.Ns(), w1 = ProcCpu.Wall();
            Times[parameters.BenchmarkCase.Parameters["Case"].ToString()] = (_t0, _t1, DateTime.UtcNow);
            Stage[parameters.BenchmarkCase.Parameters["Case"].ToString()] = (c1 - _c0, w1 - _w0,
                GC.CollectionCount(0) - _g0, GC.CollectionCount(1) - _g1, GC.CollectionCount(2) - _g2, _heap,
                (long)((GC.GetTotalPauseDuration() - _p0).TotalMilliseconds * 1e6));
        }
    }
    public IEnumerable<Metric> ProcessResults(DiagnoserResults results) => Array.Empty<Metric>();
    public void DisplayResults(ILogger logger) { }
    public IEnumerable<ValidationError> Validate(ValidationParameters validationParameters) => Array.Empty<ValidationError>();
}

/// Requirement 22 (amended): blocks by arm, the arm order rotated between launches.
public sealed class RotatingOrderer : IOrderer
{
    /// R-H23: BenchmarkDotNet has no built-in random order, but an IOrderer decides the
    /// execution order, so this one RANDOMISES it: the prime cases first (they must run first),
    /// then every case in a seeded shuffle (seed = launch), recorded in the header.
    private readonly int _seed;
    public RotatingOrderer(int launch) { _seed = launch * 104729; }
    public int Seed => _seed;
    public IEnumerable<BenchmarkCase> GetExecutionOrder(ImmutableArray<BenchmarkCase> benchmarksCase, IEnumerable<BenchmarkLogicalGroupRule> order = null)
    {
        var rng = new Random(_seed);
        var keyed = benchmarksCase.Select(b => (b, k: rng.Next())).ToList();
        return keyed.OrderBy(t => Case.Parse(t.b.Parameters["Case"].ToString()).Content == Cases.Prime ? 0 : 1).ThenBy(t => t.k).Select(t => t.b);
    }
    public IEnumerable<BenchmarkCase> GetSummaryOrder(ImmutableArray<BenchmarkCase> benchmarksCases, Summary summary) => GetExecutionOrder(benchmarksCases);
    public string GetHighlightGroupKey(BenchmarkCase benchmarkCase) => null;
    public string GetLogicalGroupKey(ImmutableArray<BenchmarkCase> allBenchmarksCases, BenchmarkCase benchmarkCase) => "codec";
    public IEnumerable<IGrouping<string, BenchmarkCase>> GetLogicalGroupOrder(IEnumerable<IGrouping<string, BenchmarkCase>> logicalGroups, IEnumerable<BenchmarkLogicalGroupRule> order = null) => logicalGroups;
    public bool SeparateLogicalGroups => false;
}

/// Section 7: one JSON object per raw measurement, appended to the launch's log.
public sealed class JsonLinesExporter : IExporter
{
    private readonly string _path;
    private readonly int _launch;
    public JsonLinesExporter(string path, int launch) { _path = path; _launch = launch; }
    public string Name => "ak-jsonl";
    public void ExportToLog(Summary summary, ILogger logger) { }

    private static string J(Case c, int launch, int round, long cpu, long wall, long iters, string extra)
    {
        var sb = new StringBuilder("{\"slice\":\"csharp\",\"suite\":\"codec\"");
        sb.Append(",\"arm\":\"").Append(c.Arm).Append("\",\"payload\":\"").Append(c.Payload).Append("\",\"content\":\"").Append(c.Content)
          .Append("\",\"dir\":\"").Append(c.Dir).Append("\",\"unknown_mode\":\"").Append(c.Mode).Append('"');
        sb.Append(",\"build\":\"").Append(Armonik.Ffi.Harness.AbiVariant.Name).Append('"');   // R-H6
        sb.Append(",\"launch\":").Append(launch).Append(",\"round\":").Append(round);
        if (cpu >= 0) sb.Append(",\"cpu_ns\":").Append(cpu);
        sb.Append(",\"wall_ns\":").Append(wall).Append(",\"iters\":").Append(iters);
        if (extra != null) sb.Append(',').Append(extra);
        return sb.Append('}').ToString();
    }

    public static int JitQuietCases, JitTier0Cases;

    public IEnumerable<string> ExportToFiles(Summary summary, ILogger consoleLogger)
    {
        // The runtime delivers JIT events late (its dispatch thread); give it time to drain.
        long seen = -1;
        for (int i = 0; i < 20 && seen != JitTiers.Received; i++) { seen = JitTiers.Received; Thread.Sleep(250); }
        var o = new List<string>();
        foreach (var r in summary.Reports)
        {
            var c = Case.Parse(r.BenchmarkCase.Parameters["Case"].ToString());
            if (c.Content == Cases.Prime) { o.Add("# prime case (BDN engine warm-up, timed by BDN, not exported): " + c.Key + (r.Success ? "" : " FAILED")); continue; }
            if (!r.Success || r.AllMeasurements == null || r.AllMeasurements.Count == 0)
            {
                o.Add("# FAILED CASE (no measurement): " + c.Key);
                continue;
            }
            var all = r.AllMeasurements;
            int warm = all.Count(m => m.IterationMode == IterationMode.Workload && m.IterationStage == IterationStage.Warmup);
            var act = all.Where(m => m.IterationMode == IterationMode.Workload && m.IterationStage == IterationStage.Actual).ToList();
            int round = 0;
            foreach (var m in act)
                o.Add(J(c, _launch, ++round, -1, (long)Math.Round(m.Nanoseconds), m.Operations,
                    string.Format(CultureInfo.InvariantCulture, "\"engine\":\"bdn\",\"bdn_warmup\":{0}", warm)));
            // Every stage BDN ran for this case, as mode/stage: count, ops, ns (requirement 24).
            var stages = string.Join(",", all.GroupBy(m => m.IterationMode + "/" + m.IterationStage)
                .Select(g => string.Format(CultureInfo.InvariantCulture, "\"{0}\":[{1},{2},{3}]", g.Key, g.Count(), g.Sum(m => m.Operations), (long)Math.Round(g.Sum(m => m.Nanoseconds)))));
            string jit = "\"jit\":\"not recorded\",";
            if (CpuDiagnoser.Times.TryGetValue(c.Key, out var tt))
            {
                var ts = CodecSuite.SetupEnd.TryGetValue(c.Key, out var se) ? se : tt.T0;
                jit = JitTiers.Summarise(tt.T0, ts, tt.T1, tt.T2, out int sc, out int hot) + ",";
                if (sc == 0) JitQuietCases++;
                if (hot > 0) JitTier0Cases++;
            }
            if (CpuDiagnoser.Stage.TryGetValue(c.Key, out var st))
                o.Add(J(c, _launch, 0, st.Cpu, st.Wall, act.Sum(m => m.Operations),
                    "\"engine\":\"bdn\",\"bdn_stages\":{" + stages + "}," + jit + string.Format(CultureInfo.InvariantCulture, "\"gc\":[{0},{1},{2}],\"gc_pause_ns\":{4},\"heap_bytes\":{3},", st.G0, st.G1, st.G2, st.Heap, st.Pause) + "\"note\":\"round 0 = CPU (getrusage RUSAGE_SELF) and wall of the process across BDN BeforeActualRun..AfterActualRun: the actual stage (jitting, pilot and warm-up precede it) INCLUDING the GCs BDN forces between iterations; iters = actual-stage ops; gc_pause_ns = GC pause time inside the span (the forced collections and any the arm caused); bdn_stages = [iterations, ops, ns] per mode/stage; gc = gen0/gen1/gen2 collections across the span (BDN forces 4 full collections per iteration); heap_bytes = GC.GetTotalMemory(false) at its start; jit_pre / jit_span = methods compiled (measured code by name: not the BDN engine, not this harness's instrumentation, not the case's setup) before the span (jitting, pilot, warm-up) and inside it, by tier; tier0_left = methods compiled in this case whose last version at the end of the span is a tier-0 form; hot_tier0 = those of them the runtime promotes later in the process (hot code measured at tier 0)\""));
        }
        File.AppendAllLines(_path, o);
        return new[] { _path };
    }
}
