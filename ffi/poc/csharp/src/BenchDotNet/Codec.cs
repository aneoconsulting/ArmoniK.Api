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

    public IEnumerable<string> CaseKeys => Cases.All();

    private Func<long> _op;

    [GlobalSetup]
    public void Setup() => _op = Cases.Build(Bdn.Case.Parse(Case));

    /// One call per invocation; BenchmarkDotNet chooses the invocation count per case in its
    /// pilot stage, and consumes the returned value.
    [Benchmark]
    public long Run() => _op();
}
