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
    public static long Wall() => (long)(System.Diagnostics.Stopwatch.GetTimestamp() * (1e9 / System.Diagnostics.Stopwatch.Frequency));
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
    /// BDN's default toolchain (one child process per case, req 22a as amended e6c909630): the
    /// clock runs in the CHILD, the diagnoser in the host, so the child records every read and
    /// writes them to AK_CPU_CHILD_DIR at its GlobalCleanup (DumpChild); the host pairs the
    /// last reads with the actual iterations and checks each pair's wall span against BDN's
    /// own measurement (FromChild), so a wrong pairing fails instead of being guessed.
    public static readonly string ChildDir = Environment.GetEnvironmentVariable("AK_CPU_CHILD_DIR");
    static CpuClock() { if (ChildDir != null) Recording = true; }
    public long GetTimestamp()
    {
        if (!Recording) return System.Diagnostics.Stopwatch.GetTimestamp();
        long t;
        if ((Cpu.Count & 1) == 0) { Cpu.Add(ProcCpu.Ns()); t = System.Diagnostics.Stopwatch.GetTimestamp(); }
        else { t = System.Diagnostics.Stopwatch.GetTimestamp(); Cpu.Add(ProcCpu.Ns()); }
        Wall.Add(t);
        return t;
    }

    private static string FileOf(string key)
    {
        var h = System.Security.Cryptography.SHA1.HashData(System.Text.Encoding.UTF8.GetBytes(key));
        return System.IO.Path.Combine(ChildDir, Convert.ToHexString(h) + ".cpu");
    }

    public static void DumpChild(string key)
    {
        if (ChildDir == null) return;
        var sb = new System.Text.StringBuilder();
        for (int i = 0; i < Cpu.Count; i++) sb.Append(Cpu[i]).Append(' ').Append(Wall[i]).Append('\n');
        System.IO.File.WriteAllText(FileOf(key), sb.ToString());
    }

    /// The child's reads for the case's actual iterations (2 per iteration), or null when they
    /// cannot be paired: fewer reads than iterations, or a pair whose wall span differs from
    /// BDN's measurement of that iteration by more than 2 percent plus 20 us.
    public static long[] FromChild(string key, IList<double> actualNs)
    {
        if (ChildDir == null) return null;
        var f = FileOf(key);
        if (!System.IO.File.Exists(f)) return null;
        var rows = System.IO.File.ReadAllLines(f);
        int n = actualNs.Count;
        if (rows.Length < 2 * n || (rows.Length & 1) != 0) return null;
        var o = new long[2 * n];
        double tick = 1e9 / System.Diagnostics.Stopwatch.Frequency;
        for (int i = 0; i < n; i++)
        {
            var a = rows[rows.Length - 2 * n + 2 * i].Split(' '); var b = rows[rows.Length - 2 * n + 2 * i + 1].Split(' ');
            o[2 * i] = long.Parse(a[0]); o[2 * i + 1] = long.Parse(b[0]);
            double span = (long.Parse(b[1]) - long.Parse(a[1])) * tick;
            if (Math.Abs(span - actualNs[i]) > 0.02 * actualNs[i] + 20000) return null;
        }
        return o;
    }
}

