// The arms of stage 3, as BenchmarkDotNet benchmarks.
//
// Same `Arms` table, same generated codec, same incumbent, same payloads: the
// arm table is emitted from ffi/schema (R1) and referenced from the harness
// project rather than re-emitted here, so the two harnesses cannot drift apart
// in WHAT they measure. What differs is only HOW.
//
// Encode and Decode are separate classes because BenchmarkDotNet computes its
// Ratio column against one baseline per class, and encode and decode need
// different baselines (`gp-writeto` and `gp-parse`).
//
// What BenchmarkDotNet adds over ../Harness/Bench.cs, and the reason the
// controlled rerun should use this one:
//
//   * it measures an empty method of the same shape and SUBTRACTS that
//     overhead. The hand-rolled loop counts its own dead-code guard, which
//     inflates the memcpy floor rows in particular;
//   * warmup runs to a convergence criterion rather than a fixed 40 ms budget
//     guessed to be long enough to reach tier 1;
//   * it reports a confidence interval and removes outliers, instead of
//     min/median/max over seven rounds;
//   * each benchmark gets its own process.
//
// It also consumes return values itself, so nothing here needs a NoInlining
// sink.

using System;
using System.Collections.Generic;
using System.Linq;
using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Order;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;

namespace Armonik.Ffi.Bdn;

public static class Payloads
{
    /// Every payload of design/SHAPES.md by default, so the controlled rerun
    /// gets the whole table from one invocation. AK_BDN_ONLY cuts it down.
    public static IEnumerable<string> All()
    {
        var only = Environment.GetEnvironmentVariable("AK_BDN_ONLY");
        var ids = ArmTable.All().Select(a => a.Id);
        if (!string.IsNullOrWhiteSpace(only))
        {
            var keep = only.Split(',').Select(s => s.Trim()).ToHashSet(StringComparer.Ordinal);
            ids = ids.Where(keep.Contains);
        }
        return ids.ToArray();
    }
}

[MemoryDiagnoser]
[Orderer(SummaryOrderPolicy.Declared)]
[CategoriesColumn]
public class Encode
{
    [ParamsSource(nameof(PayloadIds))]
    public string Payload;

    public IEnumerable<string> PayloadIds => Payloads.All();

    private Arms _arms;
    private byte[] _dst;
    private byte[] _mcSrc;
    private byte[] _mcDst;
    private Enc _enc;
    private Enc _enc2;

    [GlobalSetup]
    public void Setup()
    {
        _arms = ArmTable.All().First(a => a.Id == Payload);
        _arms.Build();

        // Size every buffer from the payload's own encoded length, so no arm is
        // measuring a buffer growth. The hand-rolled harness does the same.
        var probe = Enc.New(Codec.Sites, 8192);
        _arms.ManagedWrite(ref probe);
        int n = probe.Pos;
        int cap = n + 4096;

        _dst = new byte[cap];
        _enc = Enc.New(Codec.Sites, cap);
        _enc2 = Enc.New(Codec.Sites, cap);
        _mcSrc = probe.ToArray();
        _mcDst = new byte[cap];

        // Warm the learned length-placeholder widths once, so the cold misses
        // of ABI v1 section 6 are not charged to the first measured iteration.
        // A server is in the warm state; the cold cost is paid once per context
        // and is reported in stage2-counts.log instead.
        _enc.Reset(); _arms.ManagedWrite(ref _enc);
        _enc2.Reset(); _arms.ManagedWriteSized(ref _enc2);
    }

    [Benchmark(Baseline = true, Description = "gp-writeto")]
    public int GpWriteTo() => _arms.GpWriteTo(_dst);

    [Benchmark(Description = "gp-tobytearray")]
    public byte[] GpToByteArray() => _arms.GpToByteArray();

    [Benchmark(Description = "managed")]
    public int Managed()
    {
        _enc.Reset();
        _arms.ManagedWrite(ref _enc);
        return _enc.Pos;
    }

    [Benchmark(Description = "managed-2pass")]
    public int Managed2Pass()
    {
        _enc2.Reset();
        _arms.ManagedWriteSized(ref _enc2);
        return _enc2.Pos;
    }

    /// R2's bound: no encoder produces N bytes for less than one copy of N.
    [Benchmark(Description = "memcpy floor")]
    public int MemcpyFloor()
    {
        Buffer.BlockCopy(_mcSrc, 0, _mcDst, 0, _mcSrc.Length);
        return _mcSrc.Length;
    }
}

[MemoryDiagnoser]
[Orderer(SummaryOrderPolicy.Declared)]
[CategoriesColumn]
public class Decode
{
    [ParamsSource(nameof(PayloadIds))]
    public string Payload;

    public IEnumerable<string> PayloadIds => Payloads.All();

    private Arms _arms;
    private byte[] _src;
    private int _len;
    private byte[] _mcDst;

    [GlobalSetup]
    public void Setup()
    {
        _arms = ArmTable.All().First(a => a.Id == Payload);
        _arms.Build();

        var e = Enc.New(Codec.Sites, 8192);
        _arms.ManagedWrite(ref e);
        _src = e.ToArray();
        _len = _src.Length;
        _mcDst = new byte[_len];

        // Correctness is gated by `harness conformance`, not here, but a decode
        // arm that silently threw would otherwise be timed as a throw. Both are
        // run once so the setup fails loudly instead.
        _arms.GpParse(_src, _len);
        _arms.ManagedParse(_src, _len);
    }

    [Benchmark(Baseline = true, Description = "gp-parse")]
    public int GpParse() => _arms.GpParse(_src, _len);

    [Benchmark(Description = "managed-parse")]
    public int ManagedParse() => _arms.ManagedParse(_src, _len);

    /// A weak floor except on the M5 bulk rows, where a decode IS a copy.
    [Benchmark(Description = "memcpy floor")]
    public int MemcpyFloor()
    {
        Buffer.BlockCopy(_src, 0, _mcDst, 0, _len);
        return _len;
    }
}
