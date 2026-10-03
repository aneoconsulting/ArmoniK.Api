// The RPC grid on BenchmarkDotNet (CAMPAIGN req 22a as amended 2026-09-27, FIX-PLAN WP9),
// the same framework, toolchain, clock and orderer as the codec suite (../BenchDotNet).
//
//   akrpc bench --sock PATH --transport shipped|pinned --unit CELL --launch N --out FILE.jsonl
//               [--rounds R] [--warmup W] [--iteration-ms T] [--inflight 1,8,16]
//               [--core-workers 2] [--artifacts DIR] [--list-units]
//
// run_campaign.sh starts the launch's one server (req 13) and warms it (req 24) first, then
// runs one pinned process per UNIT, a unit being one cell (A, B, Bf, C-retain, ..., the
// extras): its cases are that cell's directions, payloads and in-flight levels. In each
// process the cell's channels are opened in the first case's GlobalSetup and kept for the
// process (one channel per cell per benchmark process). A case is "cell|dir|payload|k"; ONE
// INVOCATION IS ONE BATCH OF k CALLS IN FLIGHT, counted as k operations (OperationsPerInvoke
// = k, so BDN's per-operation figures are per call): blocking cells (B, C, E and their twins)
// on the k caller threads of a CallerPool made in setup, async cells (A, D, F, the extras) as
// k concurrent async calls. Every call is checked (req 18) and throws on any failure; BDN
// runs with StopOnFirstError, the exporter writes no sample when any case failed, the process
// exits non-zero and the runner discards the launch's output.
//
// Clocks (req 21): wall and process CPU per iteration from CpuClock (the job's clock, as in
// the codec suite). Warm-up (req 24): BDN's jitting stage, pilot and --warmup iterations per
// case, every value in the header. Order (req 22): the unit order of a launch is a seeded
// shuffle (the runner), and inside a process the cases run in a seeded shuffle (IOrderer).

using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Armonik.Ffi.Bdn;
using Armonik.Ffi.Harness;
using Armonik.Ffi.Rpc;
using BenchmarkDotNet.Analysers;
using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Columns;
using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Diagnosers;
using BenchmarkDotNet.Engines;
using BenchmarkDotNet.Exporters;
using BenchmarkDotNet.Jobs;
using BenchmarkDotNet.Loggers;
using BenchmarkDotNet.Order;
using BenchmarkDotNet.Reports;
using BenchmarkDotNet.Running;
using BenchmarkDotNet.Toolchains.InProcess.Emit;
using BenchmarkDotNet.Validators;
using Perfolizer.Horology;

namespace Armonik.Ffi.Campaign;

/// The process's one set of cells (the unit's), built on first use by a case's GlobalSetup.
internal static class RpcCtx
{
    public static string Sock, Transport, Unit;
    public static string H2 = "unknown";
    public static (int Found, int NoDelay) NoDelaySeen;

    /// D11: which h2 the loaded core was built with, from the core library itself (the
    /// patched source's path in h2's panic locations), checked against the runner's AK_H2.
    public static string LoadedH2()
    {
        foreach (var line in File.ReadAllLines("/proc/self/maps"))
        {
            int i = line.IndexOf('/');
            if (i < 0 || !line.EndsWith("libak_core.so", StringComparison.Ordinal)) continue;
            var bytes = File.ReadAllBytes(line.Substring(i));
            var mark = Encoding.ASCII.GetBytes("h2-batch-src/src/codec/framed_write.rs");
            return bytes.AsSpan().IndexOf(mark) >= 0 ? "h2-batch" : "stock";
        }
        return "not loaded";
    }

    /// D8 / D14: every pool sized to AK_WORKERS (8 on the campaign machine): the .NET thread
    /// pool's worker minimum and maximum, and the core runtime's workers (Workers).
    public static void SizePools()
    {
        int w = int.Parse(Environment.GetEnvironmentVariable("AK_WORKERS") ?? "8", CultureInfo.InvariantCulture);
        if (Environment.GetEnvironmentVariable("AK_RPC_CORE_WORKERS") is string cw) Workers = int.Parse(cw, CultureInfo.InvariantCulture); else Workers = w;
        ThreadPool.GetMaxThreads(out _, out int maxIo);
        ThreadPool.SetMinThreads(w, w);
        ThreadPool.SetMaxThreads(w, maxIo);
    }
    public static bool Grouped;
    public static int Workers = 2, Want;
    public static int[] Levels = { 1, 8, 16 };
    private static List<CampaignMain.Cell> _cells;
    private static IntPtr _rt;
    private static readonly List<IDisposable> _owned = new List<IDisposable>();
    public static readonly List<string> Chans = new List<string>();
    public static CallerPool Pool;
    private static readonly HashSet<string> _checked = new HashSet<string>();
    public static int UploadChecks;

    /// The unit's cases, in declaration order (the orderer shuffles them).
    public static IEnumerable<string> Keys(int k) =>
        Cells().Where(c => Levels.Contains(k) && (!c.Upload || k == 1 || k == 8)).Select(c => string.Join("|", c.Name, c.Dir, c.Payload, k));

    public static List<CampaignMain.Cell> Cells()
    {
        if (_cells != null) return _cells;
        SizePools();
        H2 = LoadedH2();
        // Under BDN's default (out-of-process) toolchain this runs in the benchmark's child
        // process, which does not run Main: the context comes from the environment Main set.
        Sock ??= Environment.GetEnvironmentVariable("AK_RPC_BENCH_SOCK");
        Transport ??= Environment.GetEnvironmentVariable("AK_RPC_BENCH_TRANSPORT");
        Unit ??= Environment.GetEnvironmentVariable("AK_RPC_BENCH_UNIT");
        if (Want == 0) Want = int.Parse(Environment.GetEnvironmentVariable("AK_RPC_BENCH_WANT") ?? "0", CultureInfo.InvariantCulture);
        AppContext.SetSwitch("System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing", true);
        _rt = AkRpc.ak_runtime_new((uint)Workers);
        if (_rt == IntPtr.Zero) throw new InvalidOperationException("ak_runtime_new");
        _cells = CampaignMain.BuildCells(Sock, Transport == "pinned", _rt, Want, _owned, Chans, extras: true, keep: n => n == Unit);
        if (_cells.Count == 0) throw new InvalidOperationException("no cell named " + Unit + " in this build");
        Pool = new CallerPool(Levels.Max());
        return _cells;
    }

    public static CampaignMain.Cell Find(string key)
    {
        var f = key.Split('|');
        return Cells().First(c => c.Name == f[0] && c.Dir == f[1] && c.Payload == f[2]);
    }

    /// Req 18/26 for the upload directions, once per cell before its first timing: every c cell
    /// accepted, every d stream's count and SHA-256 through StreamCheck.
    public static void CheckOnce(CampaignMain.Cell c)
    {
        if (!c.Upload) return;
        var id = c.Name + "|" + c.Dir + "|" + c.Payload;
        if (!_checked.Add(id)) return;
        if (c.Check != null) c.Check(); else c.CheckAsync().GetAwaiter().GetResult();
        UploadChecks++;
    }

    public static void Close()
    {
        Pool?.Dispose();
        foreach (var d in _owned) d.Dispose();
        if (_rt != IntPtr.Zero) AkRpc.ak_runtime_destroy(_rt);
    }
}

/// One benchmark class per in-flight level, because OperationsPerInvoke is a constant of the
/// [Benchmark] attribute: RpcK1, RpcK8, RpcK16, each with the unit's cases at its k.
public abstract class RpcBase
{
    protected abstract int K { get; }
    [ParamsSource(nameof(CaseKeys))]
    public string Case;
    public IEnumerable<string> CaseKeys => RpcCtx.Keys(K);
    private CampaignMain.Cell _c;

    public static readonly Dictionary<string, DateTime> SetupEnd = new Dictionary<string, DateTime>();

    [GlobalSetup]
    public void Setup()
    {
        Alloc.Startup();   // once per process: a no-op after the host's or this child's first
        _c = RpcCtx.Find(Case);
        RpcCtx.CheckOnce(_c);
        // D10: one untimed call opens the cell's connection, then Nagle off is READ BACK on
        // every live socket of this process to the server (getsockopt TCP_NODELAY); a socket
        // without it fails the case, so every exported sample ran with Nagle off.
        if (CampaignMain.IsTcp(RpcCtx.Sock))
        {
            if (_c.One != null) _c.One(); else _c.OneAsync().GetAwaiter().GetResult();
            var (found, nd) = NoDelay.Check(CampaignMain.TcpPort(RpcCtx.Sock));
            if (found == 0 || nd != found) throw new InvalidOperationException("TCP_NODELAY read back: " + nd + " of " + found + " sockets to the server");
            RpcCtx.NoDelaySeen = (found, nd);
        }
        SetupEnd[Case] = DateTime.UtcNow;
    }

    protected Task Batch() => CampaignMain.RunCell(_c, RpcCtx.Pool, K, K);

    [GlobalCleanup]
    public void Cleanup() => CpuClock.DumpChild(Case);
}

public class RpcK1 : RpcBase { protected override int K => 1; [Benchmark(OperationsPerInvoke = 1)] public Task Run() => Batch(); }
public class RpcK8 : RpcBase { protected override int K => 8; [Benchmark(OperationsPerInvoke = 8)] public Task Run() => Batch(); }
public class RpcK16 : RpcBase { protected override int K => 16; [Benchmark(OperationsPerInvoke = 16)] public Task Run() => Batch(); }

/// Process CPU per actual iteration (CpuClock's reads) and the stage times for the JIT read back.
public sealed class RpcCpuDiagnoser : IDiagnoser
{
    public static readonly Dictionary<string, long[]> IterCpu = new Dictionary<string, long[]>();
    public static readonly Dictionary<string, long[]> IterTc = new Dictionary<string, long[]>();
    /// req 21 as amended: softirq and irq time on the CLIENT CPUs across each case's actual
    /// stage (/proc/stat, USER_HZ ticks; system-wide per CPU, so read here even when the case
    /// ran in a child process).
    public static readonly Dictionary<string, (long SoftIrq, long Irq)> IrqSpan = new Dictionary<string, (long, long)>();
    private (long SoftIrq, long Irq) _irq0;
    public static readonly Dictionary<string, (DateTime T0, DateTime T1, DateTime T2)> Times = new Dictionary<string, (DateTime, DateTime, DateTime)>();
    private DateTime _t0, _t1;
    public IEnumerable<string> Ids => new[] { "AkRpcProcessCpu" };
    public IEnumerable<IExporter> Exporters => Array.Empty<IExporter>();
    public IEnumerable<IAnalyser> Analysers => Array.Empty<IAnalyser>();
    public BenchmarkDotNet.Diagnosers.RunMode GetRunMode(BenchmarkCase benchmarkCase) => BenchmarkDotNet.Diagnosers.RunMode.NoOverhead;
    public bool RequiresBlockingAcknowledgments(BenchmarkCase benchmarkCase) => true;
    public void Handle(HostSignal signal, DiagnoserActionParameters parameters)
    {
        if (signal == HostSignal.BeforeAnythingElse) _t0 = DateTime.UtcNow;
        if (signal == HostSignal.BeforeActualRun)
        {
            _t1 = DateTime.UtcNow; _irq0 = Irq.Client();
            if (RpcCtx.Grouped) { CpuClock.Reset(); CpuClock.Recording = true; }
        }
        else if (signal == HostSignal.AfterActualRun)
        {
            var key = parameters.BenchmarkCase.Parameters["Case"].ToString();
            var irq1 = Irq.Client();
            IrqSpan[key] = (irq1.SoftIrq - _irq0.SoftIrq, irq1.Irq - _irq0.Irq);
            if (RpcCtx.Grouped)
            {
                CpuClock.Recording = false;
                IterCpu[key] = CpuClock.Cpu.ToArray();
                IterTc[key] = CpuClock.Tc.ToArray();
                CpuClock.IterMf[key] = CpuClock.Mf.ToArray();
            }
            Times[key] = (_t0, _t1, DateTime.UtcNow);
        }
    }
    public IEnumerable<Metric> ProcessResults(DiagnoserResults results) => Array.Empty<Metric>();
    public void DisplayResults(ILogger logger) { }
    public IEnumerable<ValidationError> Validate(ValidationParameters validationParameters) => Array.Empty<ValidationError>();
}

/// Req 22: the cases of a process in a seeded shuffle (seed from launch and unit).
public sealed class RpcOrderer : IOrderer
{
    private readonly int _seed;
    public RpcOrderer(int seed) { _seed = seed; }
    public int Seed => _seed;
    public IEnumerable<BenchmarkCase> GetExecutionOrder(ImmutableArray<BenchmarkCase> cases, IEnumerable<BenchmarkLogicalGroupRule> order = null)
    {
        var rng = new Random(_seed);
        return cases.OrderBy(c => c.Descriptor.Type.Name + "|" + c.Parameters["Case"], StringComparer.Ordinal).Select(c => (c, rng.Next())).OrderBy(t => t.Item2).Select(t => t.c).ToList();
    }
    public IEnumerable<BenchmarkCase> GetSummaryOrder(ImmutableArray<BenchmarkCase> cases, Summary summary) => GetExecutionOrder(cases);
    public string GetHighlightGroupKey(BenchmarkCase benchmarkCase) => null;
    public string GetLogicalGroupKey(ImmutableArray<BenchmarkCase> allBenchmarksCases, BenchmarkCase benchmarkCase) => "rpc";
    public IEnumerable<IGrouping<string, BenchmarkCase>> GetLogicalGroupOrder(IEnumerable<IGrouping<string, BenchmarkCase>> logicalGroups, IEnumerable<BenchmarkLogicalGroupRule> order = null) => logicalGroups;
    public bool SeparateLogicalGroups => false;
}

/// Section 7's JSON lines: one object per raw actual iteration, with every label the
/// hand-written sampler wrote, and none at all when any case failed (req 18).
public sealed class RpcJsonExporter : IExporter
{
    private readonly string _path;
    private readonly int _launch;
    public static bool Written, Failed;
    public RpcJsonExporter(string path, int launch) { _path = path; _launch = launch; }
    public string Name => "ak-rpc-jsonl";
    public void ExportToLog(Summary summary, ILogger logger) { }

    private static string SendPath(string cell) =>
        cell.StartsWith("A", StringComparison.Ordinal) || cell.StartsWith("D", StringComparison.Ordinal) || cell.StartsWith("F", StringComparison.Ordinal) ? "grpc.net"
        : cell.Contains(".callback", StringComparison.Ordinal) ? "core-callback"
        : cell.Contains(".queue", StringComparison.Ordinal) ? "core-queue"
        : cell.StartsWith("Bf", StringComparison.Ordinal) || cell.StartsWith("Cf", StringComparison.Ordinal) || cell.StartsWith("Ef", StringComparison.Ordinal) ? "core-framed"
        : "core-reference";

    public IEnumerable<string> ExportToFiles(Summary summary, ILogger consoleLogger)
    {
        long seen = -1;
        for (int i = 0; i < 20 && seen != JitTiers.Received; i++) { seen = JitTiers.Received; Thread.Sleep(250); }
        var o = new List<string>();
        var bad = summary.Reports.Where(r => !r.Success || r.AllMeasurements == null || r.AllMeasurements.Count == 0).ToList();
        if (bad.Count > 0 || summary.Reports.Length == 0)
        {
            Failed = true;
            o.Add("# ABORT: " + bad.Count + " case(s) failed (requirement 18: a failed call fails its benchmark; no sample is written): " + string.Join(", ", bad.Select(r => r.BenchmarkCase.Parameters["Case"].ToString())));
            o.Add("# no samples written");
            File.AppendAllLines(_path, o);
            return new[] { _path };
        }
        foreach (var r in summary.Reports)
        {
            var key = r.BenchmarkCase.Parameters["Case"].ToString();
            var f = key.Split('|');
            var cell = RpcCtx.Find(key);
            var all = r.AllMeasurements;
            int warm = all.Count(m => m.IterationMode == IterationMode.Workload && m.IterationStage == IterationStage.Warmup);
            var act = all.Where(m => m.IterationMode == IterationMode.Workload && m.IterationStage == IterationStage.Actual).ToList();
            long[] ic, tc, mf;
            if (RpcCtx.Grouped) { RpcCpuDiagnoser.IterCpu.TryGetValue(key, out ic); RpcCpuDiagnoser.IterTc.TryGetValue(key, out tc); CpuClock.IterMf.TryGetValue(key, out mf); }
            else ic = CpuClock.FromChild(key, act.Select(m => m.Nanoseconds).ToList(), out tc, out mf);   // the default toolchain's child
            if (tc == null || tc.Length != 2 * act.Count)
            {
                Failed = true;
                File.AppendAllLines(_path, new[] { "# ABORT: task-clock per iteration not recorded for " + key + " (req 21 as amended); no samples written" });
                return new[] { _path };
            }
            if (ic == null || ic.Length != 2 * act.Count)
            {
                Failed = true;
                File.AppendAllLines(_path, new[] { "# ABORT: process CPU per iteration not paired for " + key + "; no samples written" });
                return new[] { _path };
            }
            if (mf == null || mf.Length != 2 * act.Count)
            {
                Failed = true;
                File.AppendAllLines(_path, new[] { "# ABORT: minor faults per iteration not recorded for " + key + " (req 25 as amended); no samples written" });
                return new[] { _path };
            }
            string jit = "\"jit\":\"not recorded\"";
            if (!RpcCtx.Grouped) jit = "\"jit\":\"not recorded: the case ran in its own child process (default toolchain), whose JIT events this process does not see\"";
            else if (RpcCpuDiagnoser.Times.TryGetValue(key, out var tt))
                jit = JitTiers.Summarise(tt.T0, RpcBase.SetupEnd.TryGetValue(key, out var se) ? se : tt.T0, tt.T1, tt.T2, out _, out _);
            int round = 0;
            foreach (var m in act)
            {
                long cpu = tc[2 * round + 1] - tc[2 * round], pcpu = ic[2 * round + 1] - ic[2 * round], flt = mf[2 * round + 1] - mf[2 * round];
                round++;
                var sb = new StringBuilder("{\"slice\":\"csharp\",\"suite\":\"rpc\"");
                sb.Append(",\"cell\":\"").Append(f[0]).Append("\",\"payload\":\"").Append(f[2]).Append("\",\"dir\":\"").Append(f[1])
                  .Append("\",\"unknown_mode\":\"").Append(cell.Mode).Append("\",\"transport\":\"").Append(RpcCtx.Transport)
                  .Append("\",\"build\":\"").Append(AbiVariant.Name).Append("\",\"send_path\":\"").Append(SendPath(f[0])).Append("\",\"h2\":\"").Append(RpcCtx.H2).Append("\",\"net\":\"").Append(CampaignMain.IsTcp(RpcCtx.Sock) ? "tcp" : "uds").Append("\",\"alloc\":\"").Append(Alloc.Label).Append('"');
                sb.Append(",\"inflight\":").Append(f[3]).Append(",\"launch\":").Append(_launch).Append(",\"round\":").Append(round);
                sb.Append(",\"cpu_ns\":").Append(cpu).Append(",\"proc_cpu_ns\":").Append(pcpu).Append(",\"wall_ns\":").Append((long)Math.Round(m.Nanoseconds)).Append(",\"iters\":").Append(m.Operations).Append(",\"minflt\":").Append(flt);
                sb.Append(string.Format(CultureInfo.InvariantCulture, ",\"engine\":\"bdn\",\"bdn_warmup\":{0},\"invocations\":{1}", warm, m.Operations / int.Parse(f[3], CultureInfo.InvariantCulture)));
                if (round == 1)
                {
                    sb.Append(',').Append(jit).Append(Alloc.RowFields(key));
                    if (RpcCpuDiagnoser.IrqSpan.TryGetValue(key, out var iq))
                        sb.Append(string.Format(CultureInfo.InvariantCulture, ",\"client_softirq_ticks\":{0},\"client_irq_ticks\":{1}", iq.SoftIrq, iq.Irq));
                }
                o.Add(sb.Append('}').ToString());
            }
        }
        File.AppendAllLines(_path, o);
        Written = true;
        return new[] { _path };
    }
}

public static class RpcBenchMain
{
    private static string Opt(string[] a, string k, string d) { int i = Array.IndexOf(a, k); return i >= 0 && i + 1 < a.Length ? a[i + 1] : d; }

    /// The units of a build: its cells, in a seeded shuffle of the launch (req 22).
    public static List<string> Units(int launch)
    {
#if AK_NO_UNKNOWN_FIELDS
        var u = new List<string> { "A", "B", "Bf", "C-nounk", "Cf-nounk", "Cc-nounk", "D-nounk", "E-nounk", "Ef-nounk", "F-nounk" };
#else
        var u = new List<string> { "A", "B", "Bf" };
        foreach (var m in new[] { "-retain", "-drop" }) u.AddRange(new[] { "C" + m, "Cf" + m, "Cc" + m, "D" + m, "E" + m, "Ef" + m, "F" + m });
        u.AddRange(new[] { "B.callback", "B.queue", "C.callback", "C.queue" });
#endif
        var rng = new Random(launch * 7907);
        return u.OrderBy(x => x, StringComparer.Ordinal).OrderBy(_ => rng.Next()).ToList();
    }

    public static int Run(string[] a)
    {
        int launch = int.Parse(Opt(a, "--launch", "1"), CultureInfo.InvariantCulture);
        if (a.Contains("--list-units")) { foreach (var u in Units(launch)) Console.WriteLine(u); return 0; }
        JitTiers.Start();
        var vwhy = AbiVariant.CheckLoadedCore();
        var outp = Opt(a, "--out", "rpc.jsonl");
        if (vwhy != null) { File.AppendAllLines(outp, new[] { "# ABORT: core variant mismatch: " + vwhy, "# no samples written" }); return 3; }
        var awhy = Alloc.Mismatch();
        if (awhy != null) { File.AppendAllLines(outp, new[] { "# ABORT: allocator mode mismatch: " + awhy, "# no samples written" }); return 3; }
        Alloc.Startup();   // once per process; the mismatch was refused above
        RpcCtx.Sock = Opt(a, "--sock", null);
        RpcCtx.Transport = Opt(a, "--transport", "shipped");
        RpcCtx.Unit = Opt(a, "--unit", null);
        // D14: the core runtime's workers = AK_WORKERS unless --core-workers (exploration) says.
        var cwo = Opt(a, "--core-workers", null);
        if (cwo != null) Environment.SetEnvironmentVariable("AK_RPC_CORE_WORKERS", cwo);
        Environment.SetEnvironmentVariable("AK_TASK_CLOCK", "1");   // req 21 as amended: task-clock (CpuClock, children included)
        RpcCtx.Levels = Opt(a, "--inflight", "1,8,16").Split(',').Select(x => int.Parse(x, CultureInfo.InvariantCulture)).ToArray();
        int rounds = int.Parse(Opt(a, "--rounds", "5"), CultureInfo.InvariantCulture);
        int warm = int.Parse(Opt(a, "--warmup", "10"), CultureInfo.InvariantCulture);
        double itMs = double.Parse(Opt(a, "--iteration-ms", "100"), CultureInfo.InvariantCulture);
        var art = Opt(a, "--artifacts", "BenchmarkDotNet.Artifacts");
        RpcCtx.Want = CampaignMain.P22Wire().Length;
        // A CONTROL (run_campaign.sh --plant): a wrong expected length must abort with no sample.
        if (Environment.GetEnvironmentVariable("AK_CAMPAIGN_PLANT") == "len") RpcCtx.Want += 1;
        bool pinned = RpcCtx.Transport == "pinned";
        // CAMPAIGN req 22a (e6c909630): the campaign runs BDN's native isolation, one process per
        // benchmark case (the default toolchain); `--toolchain grouped` (InProcessEmit, every case
        // of the unit in this process) is for smoke and small exploration runs only.
        bool grouped = RpcCtx.Grouped = Opt(a, "--toolchain", "process") == "grouped";
        if (!grouped)
        {
            var cd = Path.Combine(art, "cpu-" + Environment.ProcessId);
            Directory.CreateDirectory(cd);
            Environment.SetEnvironmentVariable("AK_CPU_CHILD_DIR", cd);   // inherited by the children's CpuClock
        }
        Environment.SetEnvironmentVariable("AK_RPC_BENCH_SOCK", RpcCtx.Sock);
        Environment.SetEnvironmentVariable("AK_RPC_BENCH_TRANSPORT", RpcCtx.Transport);
        Environment.SetEnvironmentVariable("AK_RPC_BENCH_UNIT", RpcCtx.Unit);
        Environment.SetEnvironmentVariable("AK_RPC_BENCH_WANT", RpcCtx.Want.ToString(CultureInfo.InvariantCulture));

        // The cells (and their channels) are built here, before BDN, so the header can list them;
        // the cases' GlobalSetup then only looks its cell up. Upload cells are checked there.
        RpcCtx.Cells();
        var wantH2 = Environment.GetEnvironmentVariable("AK_H2");
        if (!string.IsNullOrEmpty(wantH2) && wantH2 != RpcCtx.H2)
        {
            File.AppendAllLines(outp, new[] { "# ABORT: the loaded core's h2 is " + RpcCtx.H2 + ", the runner asked for " + wantH2 + "; no samples written" });
            return 3;
        }
        int seed = launch * 104729 + RpcCtx.Unit.GetHashCode(StringComparison.Ordinal) % 1000;
        var orderer = new RpcOrderer(launch * 104729 + Units(1).IndexOf(RpcCtx.Unit));
        var o = CampaignMain.CoreOpts(pinned);
        var hdr = new List<string>();
        var sw = new StringWriter();
        var old = Console.Out;
        Console.SetOut(sw);
        CampaignMain.Header("rpc", string.Format(CultureInfo.InvariantCulture,
            "build " + AbiVariant.Name + "; launch {0}; unit {1} (one process per cell, WP9); transport {2} (the client's configuration against the server's {2} socket: Grpc.Net DisableDynamicWindowSizing{3}; server socket {4}; core: ak_client_opts stream {5} connection {6} adaptive 0 nagle {7}); Unix socket {8} (req 17: UDS); in flight {9} (c and d at 1 and 8 only)",
            launch, RpcCtx.Unit, RpcCtx.Transport, pinned ? " + InitialHttp2StreamWindowSize 4 MiB" : ", no window set", pinned ? "4 MiB windows, adaptive off (tonic)" : "tonic's defaults",
            o.stream_window, o.connection_window, o.tcp_nagle, RpcCtx.Sock, string.Join("/", RpcCtx.Levels)));
        Console.SetOut(old);
        hdr.AddRange(sw.ToString().TrimEnd('\n').Split('\n'));
        hdr.Add("# engine:         BenchmarkDotNet " + typeof(BenchmarkRunner).Assembly.GetName().Version + " (CAMPAIGN req 22a as amended 2026-09-27, WP9), toolchain " + (grouped ? "InProcessEmit, GROUPED: every case of this unit in this process (the runner's grouped switch: smoke and small exploration runs only, req 22a as amended e6c909630)" : "BDN's default, ONE CHILD PROCESS PER CASE (the campaign's native isolation, req 22a as amended e6c909630)") + ", pinned by the runner, StopOnFirstError; one invocation = one batch of k calls in flight (OperationsPerInvoke = k; `iters` = calls, `invocations` = batches); the benchmark classes RpcK1, RpcK8, RpcK16 hold the k = 1, 8, 16 cases");
        hdr.Add(string.Format(CultureInfo.InvariantCulture, "# job:            {0} actual iterations (rounds) per case, {1} warm-up iterations (the same for every case) after BDN's jitting stage and pilot, iteration time {2} ms (the pilot picks the invocation count, unroll factor 1), strategy Throughput, EvaluateOverhead=false", rounds, warm, itMs));
        hdr.Add("# clocks:         per iteration, read by the job's clock (CpuClock) at the same iteration boundaries: wall (Stopwatch); cpu_ns = perf task-clock of the WHOLE client process (req 21 as amended 2026-10-01: a counter per thread this process opens itself with perf_event_open, PERF_COUNT_SW_TASK_CLOCK, new threads picked up at the next read; softirq-inclusive while the process runs); proc_cpu_ns = CLOCK_PROCESS_CPUTIME_ID beside it (misses softirq time with IRQ_TIME_ACCOUNTING); per case, client_softirq_ticks / client_irq_ticks = /proc/stat on the CLIENT CPUs (" + (Environment.GetEnvironmentVariable("AK_CPU_CLIENT") ?? "unset") + ") across the actual stage, USER_HZ ticks; the server is another process");
        hdr.Add("# network:        " + (CampaignMain.IsTcp(RpcCtx.Sock) ? "TCP 127.0.0.1:" + CampaignMain.TcpPort(RpcCtx.Sock) + " (D10, req 17 as amended): Nagle off on every client socket (Grpc.Net: Socket.NoDelay in the connect callback; the core: ak_client_opts.tcp_nagle = 0), READ BACK on the live sockets of the measuring process in each case's setup after one untimed call (getsockopt TCP_NODELAY on every socket to the server; one without it fails the case); the server's TCP listener runs the PINNED server configuration only, so shipped and pinned differ on the client side only (shipped: Grpc.Net DisableDynamicWindowSizing and no window, the core's windows at 0; pinned: 4 MiB windows on both client transports)" : "Unix socket " + RpcCtx.Sock + " (not the campaign's transport since D10)"));
        hdr.Add("# allocator:      " + Alloc.Label + " (readback at process start: a 16 MiB malloc came from the " + Alloc.Probe() + " (mallinfo2; the block kept, never freed); no heap pre-grow (owner, 2026-10-03): BDN's warm-up runs the real call path on every thread, and minflt per row shows whether it sufficed; under the default toolchain each child probes once, on its case's first row as alloc_probe; GLIBC_TUNABLES=" + (Alloc.Tunables ?? "unset") + "; CAMPAIGN req 25 as amended 2026-10-03, D9: the main figures run glibc's default allocator as production does; the pinned pass (" + Alloc.Pinned + ") is a labelled diagnostic; the core's buffers and transport allocate through glibc malloc in this process; every row carries `alloc` and `minflt`, the client process's minor page faults over the iteration, so faults per call = minflt / iters)");
        hdr.Add("# h2:             the loaded core's h2 = " + RpcCtx.H2 + " (D11 as amended: read from the core library; the runner's AK_H2 = " + (wantH2 ?? "unset") + "); every row carries it");
        hdr.Add(string.Format(CultureInfo.InvariantCulture, "# pools:          D8 / D14: AK_WORKERS = {0}; the core runtime {1} worker thread(s) (ak_runtime_new, shared by every core channel of the process); the .NET thread pool's worker minimum and maximum set to {0} (ThreadPool.SetMin/MaxThreads); the caller pool is the in-flight level (k dedicated caller threads for the blocking cells, not a worker pool); grpc-core is not used by this slice (Grpc.Net is managed)", Environment.GetEnvironmentVariable("AK_WORKERS") ?? "8 (default)", RpcCtx.Workers));
        hdr.Add("# order:          this launch's unit order: " + string.Join(", ", Units(launch)) + " (seed " + (launch * 7907) + "); the cases of this process in a seeded shuffle (seed " + orderer.Seed + "); ratios, where the aggregation forms them, from per-launch medians (req 30)");
        hdr.Add("# cells:          A incumbent over Grpc.Net; B incumbent over the core's transport (Bf its framed send path); C core-ffi over the core's transport on the MOVE path (ak_call_unary_enc / ak_call_send_enc; Cf framed, Cc the copy path, a labelled extra); D core-ffi over Grpc.Net (a copy into the call's buffer: Grpc.Net's serializer cannot take a core buffer); E host-gen over the core's transport (Ef framed); F host-gen over Grpc.Net; B/C .callback/.queue the core's other deliveries (labelled extras); -retain/-drop/-nounk the unknown-field mode (req 10, 12)");
        hdr.Add("# directions:     a empty request, P2.2 response decoded; a+read the same then every field read; b P2.2 request decoded by the server; c unary upload of P5.3 / P5.4 (M5, 1 MB / 4 MB); d the streamed upload, M5 messages of 2 MiB (ids on the first), 4 MiB / 16 MiB (req 14); every call checked: status, response length or the server's byte count (req 18); the upload cells' count and SHA-256 checked once in setup before timing");
        hdr.Add("# delivery:       B, C, E the core's BLOCKING call on k caller threads (a CallerPool created before BDN starts); A, D, F Grpc.Net's idiomatic async call (AsyncUnaryCall / AsyncClientStreamingCall awaited), k concurrent async calls per invocation; the core's cells share one core runtime with " + RpcCtx.Workers + " worker thread(s) (req 16)");
        hdr.Add("# limits:         the client's max send and receive 64 MiB on both transports (D44, enforced); the server's receive limit 8 MiB per message (SERVER.md)");
        foreach (var ch in RpcCtx.Chans) hdr.Add("# channel:        " + ch + " (opened in this process before its first case, kept to its end: one channel per cell per benchmark process)");
        hdr.Add(CampaignMain.ThreadLine("caller threads " + RpcCtx.Levels.Max() + "; core runtime 1, " + RpcCtx.Workers + " worker thread(s)"));
        File.AppendAllLines(outp, hdr);
        foreach (var h in hdr) Console.WriteLine(h);

        var job0 = grouped ? Job.Default.WithToolchain(InProcessEmitToolchain.Instance) : Job.Default;
        var job = job0
            .WithStrategy(RunStrategy.Throughput)
            .WithLaunchCount(1)
            .WithWarmupCount(warm)
            .WithIterationCount(rounds)
            .WithIterationTime(TimeInterval.FromMilliseconds(itMs))
            .WithUnrollFactor(1)
            .WithEvaluateOverhead(false)
            .WithClock(CpuClock.Instance)
            .WithId("rpc");
        var config = ManualConfig.CreateEmpty()
            .AddJob(job)
            .AddLogger(ConsoleLogger.Default)
            .AddColumnProvider(DefaultColumnProviders.Instance)
            .AddDiagnoser(new RpcCpuDiagnoser())
            .AddExporter(new RpcJsonExporter(outp, launch))
            .WithOrderer(orderer)
            .WithArtifactsPath(art)
            .WithOptions(ConfigOptions.JoinSummary | ConfigOptions.StopOnFirstError);
        var types = new List<Type>();
        if (RpcCtx.Levels.Contains(1) && RpcCtx.Keys(1).Any()) types.Add(typeof(RpcK1));
        if (RpcCtx.Levels.Contains(8) && RpcCtx.Keys(8).Any()) types.Add(typeof(RpcK8));
        if (RpcCtx.Levels.Contains(16) && RpcCtx.Keys(16).Any()) types.Add(typeof(RpcK16));
        Summary[] sums;
        try { sums = BenchmarkRunner.Run(types.ToArray(), config); }
        catch (Exception e)
        {
            File.AppendAllLines(outp, new[] { "# ABORT: " + e.GetType().Name + ": " + e.Message, "# no samples written" });
            return 1;
        }
        int ncases = types.Sum(t => RpcCtx.Keys((int)t.GetProperty("K", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance).GetValue(Activator.CreateInstance(t))).Count());
        int reports = sums.Sum(s => s.Reports.Length), failed = sums.Sum(s => s.Reports.Count(r => !r.Success));
        File.AppendAllLines(outp, new[]
        {
            "# upload check:   " + RpcCtx.UploadChecks + " upload case(s) of this unit checked in setup (c accepted; d count and SHA-256 equal)",
            "# end: " + reports + " BDN case(s) of " + ncases + ", " + failed + " failed" + (RpcJsonExporter.Written ? "" : "; NO SAMPLES WRITTEN"),
        });
        RpcCtx.Close();
        return RpcJsonExporter.Written && !RpcJsonExporter.Failed && failed == 0 && reports == ncases ? 0 : 1;
    }
}
