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
using System.Buffers;
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
    private BufWriter _bw;
    private BufWriter _mw;
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
        _bw = new BufWriter(cap);
        _mw = new BufWriter(cap);
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

    /// The same official API family with NO top-level size pass. Reset, not
    /// Clear: ArrayBufferWriter.Clear() zeroes the written span, which is the
    /// per-iteration buffer wipe that handicapped the C++ slice's incumbent.
    [Benchmark(Description = "gp-bufferwriter")]
    public int GpWriteToBufferWriter() => _arms.GpWriteToBufferWriter(_bw);

    [Benchmark(Description = "gp-tobytearray")]
    public byte[] GpToByteArray() => _arms.GpToByteArray();

    /// **R14's path**: CalculateSize then WriteTo(IBufferWriter), which is the
    /// exact sequence Grpc.Tools emits into the generated marshaller. It is the
    /// baseline the ratios in STATE.md are quoted against, and it was missing
    /// here while the hand-rolled harness had it -- so the controlled rerun
    /// would have come back without the column the report uses.
    [Benchmark(Description = "gp-marshaller")]
    public int GpMarshaller() => _arms.GpMarshaller(_mw);

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
    private ReadOnlySequence<byte> _seq;

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
        _seq = new ReadOnlySequence<byte>(_src, 0, _len);

        // Correctness is gated by `harness conformance`, not here, but a decode
        // arm that silently threw would otherwise be timed as a throw. Both are
        // run once so the setup fails loudly instead.
        _arms.GpParse(_src, _len);
        _arms.ManagedParse(_src, _len);
    }

    [Benchmark(Baseline = true, Description = "gp-parse")]
    public int GpParse() => _arms.GpParse(_src, _len);

    /// **R14's inward path**: ParseFrom(ReadOnlySequence), which is what the
    /// generated marshaller hands the parser. Same omission as gp-marshaller
    /// above.
    [Benchmark(Description = "gp-parse-seq")]
    public int GpParseSequence() => _arms.GpParseSequence(_seq);

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

/// The payloads the `core-ffi` binding exists for. M1 and M2, which is what is
/// built; adding M3 here is a one-line change once its binding is.
public static class CorePayloads
{
    public static IEnumerable<string> All()
    {
        var only = Environment.GetEnvironmentVariable("AK_BDN_ONLY");
        var ids = ArmTable.All()
            .Where(a => a.Root == "ListResultsResponse" || a.Root == "ListTasksDetailedResponse")
            .Select(a => a.Id);
        if (!string.IsNullOrWhiteSpace(only))
        {
            var keep = only.Split(',').Select(s => s.Trim()).ToHashSet(StringComparer.Ordinal);
            ids = ids.Where(keep.Contains);
        }
        return ids.ToArray();
    }
}

/// The `core-ffi` arm, in its own class because it does not exist for every
/// payload and a benchmark that returns 0 for two thirds of the parameter set
/// would be measuring an empty method.
///
/// The baseline is **gp-marshaller** rather than gp-writeto: R14 says the
/// incumbent is the codec path ArmoniK actually runs, and the whole point of
/// this arm is what replacing that path would be worth. `managed` rides along so
/// the interface cost -- core-ffi against the no-boundary control, R3's third
/// arm -- can be read off one table computed in one process.
[MemoryDiagnoser]
[Orderer(SummaryOrderPolicy.Declared)]
[CategoriesColumn]
public unsafe class CoreFfiEncode
{
    [ParamsSource(nameof(PayloadIds))]
    public string Payload;

    public IEnumerable<string> PayloadIds => CorePayloads.All();

    private Arms _arms;
    private BufWriter _mw;
    private Enc _enc;
    private CoreFfiM1 _m1;
    private ListResultsResponse _src1;
    private CoreFfiM2 _m2;
    private ListTasksDetailedResponse _src2;

    [GlobalSetup]
    public void Setup()
    {
        _arms = ArmTable.All().First(a => a.Id == Payload);
        _arms.Build();
        var probe = Enc.New(Codec.Sites, 8192);
        _arms.ManagedWrite(ref probe);
        int cap = probe.Pos + 4096;
        _mw = new BufWriter(cap);
        _enc = Enc.New(Codec.Sites, cap);
        _enc.Reset(); _arms.ManagedWrite(ref _enc);

        switch (Payload)
        {
            case "P1.1": _src1 = BuildFacade.P1_1(); break;
            case "P1.2": _src1 = BuildFacade.P1_2(); break;
            case "P1.3": _src1 = BuildFacade.P1_3(); break;
            case "P2.1": _src2 = BuildFacade.P2_1(); break;
            case "P2.2": _src2 = BuildFacade.P2_2(); break;
            case "P2.3": _src2 = BuildFacade.P2_3(); break;
            case "P2.4": _src2 = BuildFacade.P2_4(); break;
            case "P2.5": _src2 = BuildFacade.P2_5(); break;
            default: throw new InvalidOperationException("no core-ffi binding for " + Payload);
        }
        if (_src1 != null)
        {
            _m1 = new CoreFfiM1(_src1.Results.Count + 1, probe.Pos * 3 + 65536);
            _m1.EncodeToArray(_src1);        // learn the length widths
        }
        else
        {
            var (nb, ne, by) = CoreFfiGate2.Size(_src2);
            _m2 = new CoreFfiM2(_src2.Tasks.Count + 1, nb, ne, by);
            _m2.EncodeToArray(_src2);
        }
        var ck = Environment.GetEnvironmentVariable("AK_CHUNK");
        if (!string.IsNullOrEmpty(ck) && int.TryParse(ck, out int ckv))
        {
            if (_m1 != null) _m1.Chunk = ckv; else _m2.Chunk = ckv;
        }
    }

    [GlobalCleanup]
    public void Cleanup() { _m1?.Dispose(); _m2?.Dispose(); }

    [Benchmark(Baseline = true, Description = "gp-marshaller")]
    public int GpMarshaller() => _arms.GpMarshaller(_mw);

    [Benchmark(Description = "managed")]
    public int Managed()
    {
        _enc.Reset();
        _arms.ManagedWrite(ref _enc);
        return _enc.Pos;
    }

    [Benchmark(Description = "core-ffi")]
    public int CoreFfi()
    {
        if (_m1 != null) { _m1.Encode(_src1, out byte* p, out int l); return l; }
        _m2.Encode(_src2, out byte* q, out int m);
        return m;
    }

    /// The host-side half alone: zero the by-value group, stage every string,
    /// build the run arrays, and stop before calling the codec. The difference
    /// between this and core-ffi is the codec plus every crossing, measured
    /// rather than subtracted.
    [Benchmark(Description = "core-ffi fill")]
    public int CoreFfiFill() => _m1 != null ? _m1.Fill(_src1) : _m2.Fill(_src2);
}

/// The decode half. The baseline is **gp-parse-seq**, R14's inward path.
[MemoryDiagnoser]
[Orderer(SummaryOrderPolicy.Declared)]
[CategoriesColumn]
public unsafe class CoreFfiDecode
{
    [ParamsSource(nameof(PayloadIds))]
    public string Payload;

    public IEnumerable<string> PayloadIds => CorePayloads.All();

    private Arms _arms;
    private byte[] _src;
    private int _len;
    private ReadOnlySequence<byte> _seq;
    private CoreFfiM1 _m1;
    private CoreFfiM2 _m2;

    [GlobalSetup]
    public void Setup()
    {
        _arms = ArmTable.All().First(a => a.Id == Payload);
        _arms.Build();
        var e = Enc.New(Codec.Sites, 8192);
        _arms.ManagedWrite(ref e);
        _src = e.ToArray();
        _len = _src.Length;
        _seq = new ReadOnlySequence<byte>(_src, 0, _len);

        ListResultsResponse s1 = Payload switch
        {
            "P1.1" => BuildFacade.P1_1(), "P1.2" => BuildFacade.P1_2(),
            "P1.3" => BuildFacade.P1_3(), _ => null,
        };
        if (s1 != null) { _m1 = new CoreFfiM1(s1.Results.Count + 1, _len * 3 + 65536); }
        else
        {
            ListTasksDetailedResponse s2 = Payload switch
            {
                "P2.1" => BuildFacade.P2_1(), "P2.2" => BuildFacade.P2_2(),
                "P2.3" => BuildFacade.P2_3(), "P2.4" => BuildFacade.P2_4(),
                "P2.5" => BuildFacade.P2_5(),
                _ => throw new InvalidOperationException("no core-ffi binding for " + Payload),
            };
            var (nb, ne, by) = CoreFfiGate2.Size(s2);
            _m2 = new CoreFfiM2(s2.Tasks.Count + 1, nb, ne, by);
        }
        // Run each once so a throwing arm fails the setup instead of being timed.
        _arms.GpParseSequence(_seq);
        _arms.ManagedParse(_src, _len);
        if (_m1 != null) _m1.Decode(_src, _len); else _m2.Decode(_src, _len);
    }

    [GlobalCleanup]
    public void Cleanup() { _m1?.Dispose(); _m2?.Dispose(); }

    [Benchmark(Baseline = true, Description = "gp-parse-seq")]
    public int GpParseSequence() => _arms.GpParseSequence(_seq);

    [Benchmark(Description = "managed-parse")]
    public int ManagedParse() => _arms.ManagedParse(_src, _len);

    [Benchmark(Description = "core-ffi")]
    public int CoreFfi()
        => _m1 != null ? _m1.Decode(_src, _len).Results.Count
                       : _m2.Decode(_src, _len).Tasks.Count;
}
