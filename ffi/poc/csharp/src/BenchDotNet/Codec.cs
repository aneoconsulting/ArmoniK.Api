// One benchmark class for the whole codec suite: the case is a parameter, so the arm order
// is the orderer's (rotated per launch, Engine.cs), not the declaration order.

using System;
using System.Collections.Generic;
using BenchmarkDotNet.Attributes;

namespace Armonik.Ffi.Bdn;

public class CodecSuite
{
    [ParamsSource(nameof(CaseKeys))]
    public string Case;

    public IEnumerable<string> CaseKeys => System.Linq.Enumerable.Concat(Cases.PrimeCases(), Cases.All());

    private Func<long> _op;

    /// When each case's setup ended (JitTiers: code first compiled before it is setup code).
    public static readonly Dictionary<string, DateTime> SetupEnd = new Dictionary<string, DateTime>();

    [GlobalSetup]
    public void Setup()
    {
        long t0 = System.Diagnostics.Stopwatch.GetTimestamp(); var p0 = GC.GetTotalPauseDuration(); int g0 = GC.CollectionCount(0), g2 = GC.CollectionCount(2); long a0 = GC.GetTotalAllocatedBytes();
        _op = Cases.Build(Bdn.Case.Parse(Case));
        SetupEnd[Case] = DateTime.UtcNow;   // the pre-warm's setups are overwritten by BDN's
        if (Environment.GetEnvironmentVariable("AK_BDN_TRACE") == "1")
            Console.Error.WriteLine("AKTRACE {0} GlobalSetup {1:F1}ms pause {2:F1}ms gc0 {3} gc2 {4} alloc {5}KB {6}", ProcCpu.Wall() / 1000000, System.Diagnostics.Stopwatch.GetElapsedTime(t0).TotalMilliseconds,
                (GC.GetTotalPauseDuration() - p0).TotalMilliseconds, GC.CollectionCount(0) - g0, GC.CollectionCount(2) - g2, (GC.GetTotalAllocatedBytes() - a0) / 1000, Case);
    }

    /// One call per invocation; BenchmarkDotNet chooses the invocation count per case in its
    /// pilot stage, and consumes the returned value.
    [Benchmark]
    public long Run() => _op();
}
