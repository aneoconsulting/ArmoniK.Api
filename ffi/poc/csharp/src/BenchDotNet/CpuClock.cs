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
    public long GetTimestamp()
    {
        if (!Recording) return System.Diagnostics.Stopwatch.GetTimestamp();
        if ((Cpu.Count & 1) == 0) { Cpu.Add(ProcCpu.Ns()); return System.Diagnostics.Stopwatch.GetTimestamp(); }
        long t = System.Diagnostics.Stopwatch.GetTimestamp();
        Cpu.Add(ProcCpu.Ns());
        return t;
    }
}

