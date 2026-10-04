// CAMPAIGN req 21 inside BenchmarkDotNet, shared by the codec suite (Engine.cs) and the RPC
// grid (../Rpc/RpcBench.cs, linked): the process CPU clock and the job's clock that reads it.

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Armonik.Ffi.Bdn;

public static unsafe class ProcCpu
{
    [StructLayout(LayoutKind.Sequential)] private struct Timespec { public long Sec, Nsec; }
    [DllImport("libc")] private static extern int clock_gettime(int clk, Timespec* ts);
    private const int CLOCK_PROCESS_CPUTIME_ID = 2;
    /// This process's CPU time (every thread: GC, JIT, tiering and helper threads included),
    /// nanoseconds, CLOCK_PROCESS_CPUTIME_ID (CAMPAIGN req 21).
    public static long Ns()
    {
        Timespec t;
        if (clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &t) != 0) throw new InvalidOperationException("clock_gettime(CLOCK_PROCESS_CPUTIME_ID)");
        return t.Sec * 1_000_000_000L + t.Nsec;
    }
    [DllImport("libc")] private static extern int getrusage(int who, long* usage);
    /// This process's minor page faults so far (getrusage(RUSAGE_SELF).ru_minflt; CAMPAIGN req
    /// 25 as amended, D9: the minor faults beside every allocator gap).
    public static long MinFlt()
    {
        long* u = stackalloc long[18];
        if (getrusage(0, u) != 0) throw new InvalidOperationException("getrusage");
        return u[8];   // two timevals (4 longs), maxrss, ixrss, idrss, isrss, then minflt
    }
    public static long Wall() => (long)(System.Diagnostics.Stopwatch.GetTimestamp() * (1e9 / System.Diagnostics.Stopwatch.Frequency));
}

/// CAMPAIGN req 21 as amended 2026-10-01 (RPC cells over TCP): perf `task-clock` of the WHOLE
/// process, a counter the process opens itself. perf_event_open(PERF_TYPE_SOFTWARE,
/// PERF_COUNT_SW_TASK_CLOCK) counts one thread, and an inherited counter folds a child
/// thread's count in only when it exits, so this opens one counter per thread
/// (/proc/self/task) and, at every read, opens counters for threads it has not seen yet (a
/// thread born during an iteration is counted from the next read on: stated). The sum
/// includes the softirq time spent on the process's CPUs while its threads run, which the
/// process clock misses on a kernel with IRQ_TIME_ACCOUNTING (findings/physical-probe.md).
/// Enabled by AK_TASK_CLOCK=1 (the RPC grid; inherited by BDN's child processes).
public static unsafe class TaskClock
{
    [DllImport("libc", SetLastError = true)] private static extern long syscall(long n, void* attr, int pid, int cpu, int group, ulong flags);
    [DllImport("libc", SetLastError = true)] private static extern long read(int fd, long* buf, ulong n);
    public static readonly bool Enabled = Environment.GetEnvironmentVariable("AK_TASK_CLOCK") == "1";
    private static readonly Dictionary<int, int> _fds = new Dictionary<int, int>();
    public static int Threads => _fds.Count;
    public static int Failed;

    private static void Scan()
    {
        foreach (var d in System.IO.Directory.EnumerateDirectories("/proc/self/task"))
        {
            if (!int.TryParse(System.IO.Path.GetFileName(d), out int tid) || _fds.ContainsKey(tid)) continue;
            var attr = stackalloc byte[128];
            for (int i = 0; i < 128; i++) attr[i] = 0;
            *(uint*)attr = 1;            // PERF_TYPE_SOFTWARE
            *(uint*)(attr + 4) = 128;    // size
            *(ulong*)(attr + 8) = 1;     // PERF_COUNT_SW_TASK_CLOCK
            long fd = syscall(298, attr, tid, -1, -1, 8 /* PERF_FLAG_FD_CLOEXEC */);
            if (fd < 0) { Failed++; _fds[tid] = -1; continue; }
            _fds[tid] = (int)fd;
        }
    }

    /// The task-clock of every thread seen so far, nanoseconds (an exited thread's counter keeps
    /// its final value).
    public static long Ns()
    {
        Scan();
        long sum = 0, v;
        foreach (var fd in _fds.Values)
            if (fd >= 0 && read(fd, &v, 8) == 8) sum += v;
        return sum;
    }
}

/// CAMPAIGN req 25 as amended 2026-10-03 (D9): the allocator mode of this process. The main
/// figures run glibc's default allocator (GLIBC_TUNABLES unset, as production); the pinned
/// pass, a labelled diagnostic, runs with Pinned. Read from this process's own environment
/// (glibc applies GLIBC_TUNABLES at load), so the label is what the process ran under.
public static class Alloc
{
    public const string Pinned = "glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432";
    public static readonly string Tunables = Environment.GetEnvironmentVariable("GLIBC_TUNABLES");
    public static string Label => string.IsNullOrEmpty(Tunables) ? "default" : Tunables == Pinned ? "pinned" : "other";
    /// The runner's AK_CAMPAIGN_ALLOC, when set, must equal the label, and under the pinned mode the
    /// readback must show the pinned mmap threshold in effect (a mismatch is refused by the mains).
    public static string Mismatch()
    {
        var want = Environment.GetEnvironmentVariable("AK_CAMPAIGN_ALLOC");
        if (want != null && want != Label) return "AK_CAMPAIGN_ALLOC=" + want + " but GLIBC_TUNABLES=" + (Tunables ?? "unset") + " (" + Label + ")";
        if (Label == "pinned" && Probe() != "heap") return "GLIBC_TUNABLES is pinned but a 16 MiB malloc was mmapped (the pinned mmap_threshold is not in effect)";
        return null;
    }

    [StructLayout(LayoutKind.Sequential)] private struct MallInfo2 { public nuint Arena, Ordblks, Smblks, Hblks, Hblkhd, Usmblks, Fsmblks, Uordblks, Fordblks, Keepcost; }
    [DllImport("libc")] private static extern MallInfo2 mallinfo2();
    [DllImport("libc")] private static extern IntPtr malloc(nuint n);
    [DllImport("libc")] private static extern void free(IntPtr p);
    private static string _probe;
    private static IntPtr _probeBlock;   // kept for the process's life, never freed
    /// Readback that the allocator mode is the one running: one malloc(16 MiB) and glibc's own
    /// count of mmapped blocks (mallinfo2().hblks) around it. Pinned (mmap_threshold 32 MiB):
    /// "heap". Default: "mmapped". ONCE per process, and the block is NOT freed: freeing an
    /// mmapped chunk raises glibc's dynamic mmap threshold, so a freeing probe changes the
    /// default mode it verifies (owner decision 2026-10-03, after java 2892e207b).
    public static string Probe()
    {
        if (_probe != null) return _probe;
        nuint a = mallinfo2().Hblks;
        _probeBlock = malloc(16u << 20);
        nuint b = mallinfo2().Hblks;
        return _probe = b > a ? "mmapped" : "heap";
    }

    private static bool _started;
    /// Once per process (BDN's per-case child included: the first GlobalSetup calls it, later
    /// ones return): the probe and the mode check. Throws on a mismatch. No heap pre-grow (owner,
    /// 2026-10-03: reverted; BDN's warm-up runs the real call path on every thread, and the
    /// per-row minflt shows whether it sufficed).
    public static void Startup()
    {
        if (_started) return;
        _started = true;
        Probe();
        var m = Mismatch();
        if (m != null) throw new InvalidOperationException("allocator mode mismatch: " + m);
        CpuGuard.Check();
        WriteChild();
    }
    /// Under the default toolchain the child's probe is written for the host (one file per
    /// child process), and the host puts it on the case's first row.
    private static void WriteChild()
    {
        if (CpuClock.ChildDir == null) return;
        System.IO.File.WriteAllText(System.IO.Path.Combine(CpuClock.ChildDir, "probe-" + Environment.ProcessId + ".txt"), _probe + "\n" + CpuGuard.Fields() + "\n");
        ChildFile = "probe-" + Environment.ProcessId + ".txt";
    }
    public static string ChildFile;
    /// The row field: this process's probe (grouped) or the child's (default toolchain; the
    /// child wrote its pid's file and the case's key file is a copy).
    public static string RowFields(string key)
    {
        string probe = _probe, cpus = CpuGuard.Fields();
        if (CpuClock.ChildDir != null)
        {
            var kf = CpuClock.KeyFile(key) + ".probe";
            if (!System.IO.File.Exists(kf)) return ",\"alloc_probe\":\"not recorded\",\"cpus\":\"not recorded\"";
            var ls = System.IO.File.ReadAllText(kf).Trim().Split('\n');
            probe = ls[0].Trim();
            cpus = ls.Length > 1 ? ls[1].Trim() : "\"cpus\":\"not recorded\"";
        }
        return ",\"alloc_probe\":\"" + probe + "\"," + cpus;
    }
}

/// CAMPAIGN req 21 inside BenchmarkDotNet: the job's clock. The engine calls GetTimestamp at
/// an iteration's start and at its end (IClock.Start / StartedClock.GetElapsed); while the
/// diagnoser has recording on (the actual stage), each call also reads the process CPU clock:
/// before the Stopwatch on an even (start) read, after it on an odd (end) read, so the CPU
/// window contains the wall window. Single-threaded, as BDN's engine is.
public sealed class CpuClock : Perfolizer.Horology.IClock
{
    public static readonly CpuClock Instance = new CpuClock();
    public string Title => "Stopwatch + CLOCK_PROCESS_CPUTIME_ID";
    public bool IsAvailable => true;
    public Perfolizer.Horology.Frequency Frequency => new Perfolizer.Horology.Frequency(System.Diagnostics.Stopwatch.Frequency);
    public static bool Recording;
    public static readonly List<long> Cpu = new List<long>(64);
    public static readonly List<long> Wall = new List<long>(64);
    /// task-clock beside the process clock at every read (AK_TASK_CLOCK=1), else empty.
    public static readonly List<long> Tc = new List<long>(64);
    /// minor page faults of the process at every read (req 25 as amended, D9).
    public static readonly List<long> Mf = new List<long>(64);
    public static void Reset() { Cpu.Clear(); Wall.Clear(); Tc.Clear(); Mf.Clear(); }
    /// Per case, the minor faults at each actual iteration's start and end (grouped mode).
    public static readonly Dictionary<string, long[]> IterMf = new Dictionary<string, long[]>();
    /// BDN's default toolchain (one child process per case, req 22a as amended e6c909630): the
    /// clock runs in the CHILD, the diagnoser in the host, so the child records every read and
    /// writes them to AK_CPU_CHILD_DIR at its GlobalCleanup (DumpChild); the host pairs the
    /// last reads with the actual iterations and checks each pair's wall span against BDN's
    /// own measurement (FromChild), so a wrong pairing fails instead of being guessed.
    /// Read at each use: the host sets AK_CPU_CHILD_DIR in Main, after the allocator check may
    /// already have initialised this class (a cached value was null there and no case paired).
    public static string ChildDir => Environment.GetEnvironmentVariable("AK_CPU_CHILD_DIR");
    static CpuClock() { if (ChildDir != null) Recording = true; }
    public long GetTimestamp()
    {
        if (!Recording) return System.Diagnostics.Stopwatch.GetTimestamp();
        long t;
        if ((Cpu.Count & 1) == 0)
        {
            Mf.Add(ProcCpu.MinFlt());
            if (TaskClock.Enabled) Tc.Add(TaskClock.Ns());
            Cpu.Add(ProcCpu.Ns()); t = System.Diagnostics.Stopwatch.GetTimestamp();
        }
        else
        {
            t = System.Diagnostics.Stopwatch.GetTimestamp(); Cpu.Add(ProcCpu.Ns());
            if (TaskClock.Enabled) Tc.Add(TaskClock.Ns());
            Mf.Add(ProcCpu.MinFlt());
        }
        Wall.Add(t);
        return t;
    }

    public static string KeyFile(string key) => FileOf(key);
    private static string FileOf(string key)
    {
        var h = System.Security.Cryptography.SHA1.HashData(System.Text.Encoding.UTF8.GetBytes(key));
        return System.IO.Path.Combine(ChildDir, Convert.ToHexString(h) + ".cpu");
    }

    public static void DumpChild(string key)
    {
        if (ChildDir == null) return;
        var sb = new System.Text.StringBuilder();
        for (int i = 0; i < Cpu.Count; i++) sb.Append(Cpu[i]).Append(' ').Append(Wall[i]).Append(' ').Append(i < Tc.Count ? Tc[i] : -1).Append(' ').Append(i < Mf.Count ? Mf[i] : -1).Append('\n');
        System.IO.File.WriteAllText(FileOf(key), sb.ToString());
        if (Alloc.ChildFile != null) System.IO.File.Copy(System.IO.Path.Combine(ChildDir, Alloc.ChildFile), FileOf(key) + ".probe", true);
    }

    /// The child's reads for the case's actual iterations (2 per iteration), or null when they
    /// cannot be paired: fewer reads than iterations, or a pair whose wall span differs from
    /// BDN's measurement of that iteration by more than 2 percent plus 20 us.
    public static long[] FromChild(string key, IList<double> actualNs) => FromChild(key, actualNs, out _);

    /// The same, with the task-clock reads (null when the child did not record them).
    public static long[] FromChild(string key, IList<double> actualNs, out long[] tc) => FromChild(key, actualNs, out tc, out _);

    /// The same, with the minor-fault reads too (null when absent).
    public static long[] FromChild(string key, IList<double> actualNs, out long[] tc, out long[] mf)
    {
        tc = null; mf = null;
        if (ChildDir == null) return null;
        var f = FileOf(key);
        if (!System.IO.File.Exists(f)) return null;
        var rows = System.IO.File.ReadAllLines(f);
        int n = actualNs.Count;
        if (rows.Length < 2 * n || (rows.Length & 1) != 0) return null;
        var o = new long[2 * n];
        var t = new long[2 * n];
        var q = new long[2 * n];
        bool hasTc = true, hasMf = true;
        double tick = 1e9 / System.Diagnostics.Stopwatch.Frequency;
        for (int i = 0; i < n; i++)
        {
            var a = rows[rows.Length - 2 * n + 2 * i].Split(' '); var b = rows[rows.Length - 2 * n + 2 * i + 1].Split(' ');
            o[2 * i] = long.Parse(a[0]); o[2 * i + 1] = long.Parse(b[0]);
            if (a.Length > 2 && b.Length > 2 && a[2] != "-1" && b[2] != "-1") { t[2 * i] = long.Parse(a[2]); t[2 * i + 1] = long.Parse(b[2]); } else hasTc = false;
            if (a.Length > 3 && b.Length > 3 && a[3] != "-1" && b[3] != "-1") { q[2 * i] = long.Parse(a[3]); q[2 * i + 1] = long.Parse(b[3]); } else hasMf = false;
            double span = (long.Parse(b[1]) - long.Parse(a[1])) * tick;
            if (Math.Abs(span - actualNs[i]) > 0.02 * actualNs[i] + 20000) return null;
        }
        if (hasTc) tc = t;
        if (hasMf) mf = q;
        return o;
    }
}


/// Opt-in, AK_BDN_MEMORY=1 (an exploration switch, off in the campaign): BenchmarkDotNet's own
/// MemoryDiagnoser. BDN runs ONE extra workload iteration after the actual stage (same
/// invocation count, outside the job's clock, so no exported sample changes) and reads
/// GC.GetTotalAllocatedBytes(precise) and the gen0/1/2 collection counts around it; every
/// thread of the process counts. Its result goes on each case's first sample row.
public static class MemStats
{
    public static bool On => Environment.GetEnvironmentVariable("AK_BDN_MEMORY") == "1";
    public const string Header = "# memory:         AK_BDN_MEMORY=1: BenchmarkDotNet's MemoryDiagnoser (an exploration switch, not a campaign setting): one extra workload iteration per case AFTER the actual stage, at the actual stage's invocation count, outside the job's clock (no exported sample is affected); on each case's first sample row mem_alloc_bytes_per_op (GC.GetTotalAllocatedBytes(precise), every thread, divided by its operations), mem_gen = [gen0, gen1, gen2] collections during it, mem_ops its operations, mem_alloc_bytes its total";
    public static string Fields(BenchmarkDotNet.Reports.BenchmarkReport r)
    {
        if (!On) return "";
        var g = r.GcStats;
        var per = g.GetBytesAllocatedPerOperation(r.BenchmarkCase);
        return string.Format(System.Globalization.CultureInfo.InvariantCulture,
            ",\"mem_alloc_bytes_per_op\":{0},\"mem_gen\":[{1},{2},{3}],\"mem_ops\":{4},\"mem_alloc_bytes\":{5}",
            per.HasValue ? per.Value.ToString(System.Globalization.CultureInfo.InvariantCulture) : "null",
            g.Gen0Collections, g.Gen1Collections, g.Gen2Collections, g.TotalOperations, g.GetTotalAllocatedBytes(true));
    }
}

/// The CPUs this process may run on, and a refusal of a timed managed process pinned to ONE
/// CPU (2026-10-04, the aggregating session, JOURNAL 73): .NET 8 multiplies the tier-up
/// call-counting delay (TC_CallCountingDelayMs, 100 ms) by TC_DelaySingleProcMultiplier (10)
/// when the process is affinitized to a single processor, so tier 1 arrives ~1 s after the last
/// tier-0 JIT, later than a BDN child measures: a one-CPU run measures tier-0 code. Checked once
/// per process (the host and every BDN child, Alloc.Startup); AK_ALLOW_SINGLE_CPU=1 overrides,
/// and the row says so. The effective count goes on each case's first row (the child's own
/// under the default toolchain).
public static unsafe class CpuGuard
{
    [DllImport("libc", SetLastError = true)] private static extern int sched_getaffinity(int pid, nuint size, ulong* mask);
    public static int Affinity()
    {
        ulong* m = stackalloc ulong[16];
        for (int i = 0; i < 16; i++) m[i] = 0;
        if (sched_getaffinity(0, 16 * 8, m) != 0) return -1;
        int n = 0;
        for (int i = 0; i < 16; i++) n += System.Numerics.BitOperations.PopCount(m[i]);
        return n;
    }
    public static bool Override => Environment.GetEnvironmentVariable("AK_ALLOW_SINGLE_CPU") == "1";
    public static void Check()
    {
        int n = Affinity();
        if (n == 1 && !Override)
            throw new InvalidOperationException("refused: this timed process is affinitized to ONE CPU (sched_getaffinity), where .NET 8 delays tier-up 10x (TC_DelaySingleProcMultiplier) and the measurement runs tier-0 code; give the client >= 2 CPUs, or set AK_ALLOW_SINGLE_CPU=1 (labelled on every case's first row)");
    }
    public static string Fields() => string.Format(System.Globalization.CultureInfo.InvariantCulture,
        "\"cpus_affinity\":{0},\"cpus_runtime\":{1},\"single_cpu_override\":{2}", Affinity(), Environment.ProcessorCount, Override && Affinity() == 1 ? "true" : "false");
    public static string Header => "# cpus:           this process: " + Affinity() + " CPU(s) in its affinity mask (sched_getaffinity), Environment.ProcessorCount " + Environment.ProcessorCount
        + "; single-CPU guard (JOURNAL 73): a timed process (host or BDN child) with ONE CPU is refused unless AK_ALLOW_SINGLE_CPU=1 (here: " + (Override ? "SET" : "unset") + "); each case's first row carries cpus_affinity, cpus_runtime and single_cpu_override as its own process saw them"
        + (Affinity() == 1 ? " -- SINGLE CPU UNDER OVERRIDE: .NET 8's tier-up delay is 10x (about 1 s); managed timings here likely include tier-0 code" : "");
}
