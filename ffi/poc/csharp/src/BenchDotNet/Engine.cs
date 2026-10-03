// BenchmarkDotNet configured to meet design/CAMPAIGN.md requirements 21 to 28 (req 22a).
//
//   * every RAW workload measurement of the actual stage is exported, one JSON line per
//     iteration (JsonLinesExporter); BDN's outlier removal and overhead subtraction touch
//     only its own summary, never the exported rows (they carry the raw wall time);
//   * CPU (CAMPAIGN req 21 as amended, R-H25): PROCESS CPU PER ITERATION. The job's clock is
//     CpuClock: BDN's engine reads it at the start and the end of every iteration (IClock,
//     Job.WithClock), and CpuClock reads CLOCK_PROCESS_CPUTIME_ID of this process beside the
//     Stopwatch each time, so each exported iteration carries the process CPU of exactly the
//     window its wall time covers (BDN's forced GCs between iterations are outside it, as for
//     the wall time). The diagnoser checks that the actual stage produced exactly two clock
//     reads per actual iteration; any other count fails the case (the pairing is not guessed);
//   * the stage span (BeforeActualRun..AfterActualRun, GCs between iterations included) is
//     kept as a per-case summary row (round 0, span_* fields, no cpu_ns/wall_ns);
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

/// CPU time of the process across BeforeActualRun -> AfterActualRun: the actual stage and
/// BDN's forced GCs between its iterations (BDN has no signal per iteration).
public sealed class CpuDiagnoser : IDiagnoser
{
    public static readonly Dictionary<string, (DateTime T0, DateTime T1, DateTime T2)> Times = new Dictionary<string, (DateTime, DateTime, DateTime)>();
    private DateTime _t0, _t1;
    public static readonly Dictionary<string, (long Cpu, long Wall, int G0, int G1, int G2, long Heap, long Pause)> Stage = new Dictionary<string, (long, long, int, int, int, long, long)>();
    /// Per case: the process CPU clock read at each actual iteration's start and end (CpuClock).
    public static readonly Dictionary<string, long[]> IterCpu = new Dictionary<string, long[]>();
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
            CpuClock.Reset(); CpuClock.Recording = true;
        }
        else if (signal == HostSignal.AfterActualRun)
        {
            CpuClock.Recording = false;
            long c1 = ProcCpu.Ns(), w1 = ProcCpu.Wall();
            IterCpu[parameters.BenchmarkCase.Parameters["Case"].ToString()] = CpuClock.Cpu.ToArray();
            CpuClock.IterMf[parameters.BenchmarkCase.Parameters["Case"].ToString()] = CpuClock.Mf.ToArray();
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

    private static string J(Case c, int launch, int round, long cpu, long wall, long iters, string extra, bool summary = false)
    {
        var sb = new StringBuilder("{\"slice\":\"csharp\",\"suite\":\"codec\"");
        sb.Append(",\"arm\":\"").Append(c.Arm).Append("\",\"payload\":\"").Append(c.Payload).Append("\",\"content\":\"").Append(c.Content)
          .Append("\",\"dir\":\"").Append(c.Dir).Append("\",\"unknown_mode\":\"").Append(c.Mode).Append('"');
        sb.Append(",\"build\":\"").Append(Armonik.Ffi.Harness.AbiVariant.Name).Append('"');   // R-H6
        sb.Append(",\"h2\":\"stock\"");
        sb.Append(",\"alloc\":\"").Append(Alloc.Label).Append('"');   // req 25 as amended (D9)   // D11: the codec suite loads the stock core and makes no transport call
        sb.Append(",\"launch\":").Append(launch).Append(",\"round\":").Append(round);
        if (summary) sb.Append(",\"row\":\"case-summary\",\"span_cpu_ns\":").Append(cpu).Append(",\"span_wall_ns\":").Append(wall);
        else sb.Append(",\"cpu_ns\":").Append(cpu).Append(",\"wall_ns\":").Append(wall);
        sb.Append(",\"iters\":").Append(iters);
        if (extra != null) sb.Append(',').Append(extra);
        return sb.Append('}').ToString();
    }

    public static int JitQuietCases, JitTier0Cases, CpuPairFailed;

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
            // req 21: the process CPU of each actual iteration, paired from CpuClock's reads.
            long[] mf = null;
            if (!CpuDiagnoser.IterCpu.TryGetValue(c.Key, out var ic) || ic.Length != 2 * act.Count)
                ic = CpuClock.FromChild(c.Key, act.Select(m => m.Nanoseconds).ToList(), out _, out mf);   // the default toolchain's child
            else CpuClock.IterMf.TryGetValue(c.Key, out mf);
            if (mf == null || mf.Length != ic?.Length)
            {
                o.Add("# FAILED CASE (minor faults per iteration not recorded, req 25 as amended): " + c.Key);
                CpuPairFailed++;
                continue;
            }
            if (ic == null || ic.Length != 2 * act.Count)
            {
                o.Add("# FAILED CASE (process CPU per iteration not paired: " + (ic == null ? "no clock reads" : ic.Length + " clock reads for " + act.Count + " actual iterations") + "): " + c.Key);
                CpuPairFailed++;
                continue;
            }
            string variant = "";
            if (Cases.IsEnc(c.Dir))
            {
                variant = string.Format(CultureInfo.InvariantCulture, ",\"enc_end\":\"{0}\",\"enc_input\":\"{1}\"",
                    Cases.IsTransport(c.Dir) ? "transport" : "buf", Cases.IsPool(c.Dir) ? "pool" : "hot");
                if (Cases.Pools.TryGetValue(c.Key, out var pl))
                    variant += string.Format(CultureInfo.InvariantCulture, ",\"pool_graphs\":{0},\"pool_bytes\":{1}", pl.N, pl.Bytes);
            }
            int round = 0;
            foreach (var m in act)
            {
                long cpu = ic[2 * round + 1] - ic[2 * round], flt = mf[2 * round + 1] - mf[2 * round];
                o.Add(J(c, _launch, ++round, cpu, (long)Math.Round(m.Nanoseconds), m.Operations,
                    string.Format(CultureInfo.InvariantCulture, "\"minflt\":{0},\"engine\":\"bdn\",\"bdn_warmup\":{1}", flt, warm) + variant));
            }
            // Every stage BDN ran for this case, as mode/stage: count, ops, ns (requirement 24).
            var stages = string.Join(",", all.GroupBy(m => m.IterationMode + "/" + m.IterationStage)
                .Select(g => string.Format(CultureInfo.InvariantCulture, "\"{0}\":[{1},{2},{3}]", g.Key, g.Count(), g.Sum(m => m.Operations), (long)Math.Round(g.Sum(m => m.Nanoseconds)))));
            string jit = "\"jit\":\"not recorded\",";
            if (!Program.Grouped) jit = "\"jit\":\"not recorded: the case ran in its own child process (default toolchain), whose JIT events this process does not see\",";
            else if (CpuDiagnoser.Times.TryGetValue(c.Key, out var tt))
            {
                var ts = CodecSuite.SetupEnd.TryGetValue(c.Key, out var se) ? se : tt.T0;
                jit = JitTiers.Summarise(tt.T0, ts, tt.T1, tt.T2, out int sc, out int hot) + ",";
                if (sc == 0) JitQuietCases++;
                if (hot > 0) JitTier0Cases++;
            }
            if (CpuDiagnoser.Stage.TryGetValue(c.Key, out var st))
                o.Add(J(c, _launch, 0, st.Cpu, st.Wall, act.Sum(m => m.Operations), summary: true, extra:
                    "\"engine\":\"bdn\",\"bdn_stages\":{" + stages + "}," + jit + string.Format(CultureInfo.InvariantCulture, "\"gc\":[{0},{1},{2}],\"gc_pause_ns\":{4},\"heap_bytes\":{3},", st.G0, st.G1, st.G2, st.Heap, st.Pause) + "\"note\":\"a per-case summary, not a sample: span_cpu_ns / span_wall_ns = process CPU (CLOCK_PROCESS_CPUTIME_ID) and wall across BDN BeforeActualRun..AfterActualRun: the actual stage (jitting, pilot and warm-up precede it) INCLUDING the GCs BDN forces between iterations (the samples are the rows with round >= 1: process CPU and wall of each iteration's own window); iters = actual-stage ops; gc_pause_ns = GC pause time inside the span (the forced collections and any the arm caused); bdn_stages = [iterations, ops, ns] per mode/stage; gc = gen0/gen1/gen2 collections across the span (BDN forces 4 full collections per iteration); heap_bytes = GC.GetTotalMemory(false) at its start; jit_pre / jit_span = methods compiled (measured code by name: not the BDN engine, not this harness's instrumentation, not the case's setup) before the span (jitting, pilot, warm-up) and inside it, by tier; tier0_left = methods compiled in this case whose last version at the end of the span is a tier-0 form; hot_tier0 = those of them the runtime promotes later in the process (hot code measured at tier 0)\""));
        }
        File.AppendAllLines(_path, o);
        return new[] { _path };
    }
}
