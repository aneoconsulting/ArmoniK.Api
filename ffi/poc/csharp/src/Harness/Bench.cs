// The timing harness. R3, R4 and R9.
//
// Every arm runs in ONE process, and every round runs every case once before
// any case runs twice, so a ratio is formed between two arms that saw the same
// machine at the same moment. Absolutes do not travel between runs; ratios
// inside one process do.
//
// R9's hazards, and what is done about each:
//
//   JIT tiering and PGO off handicaps a managed incumbent, so they are left ON
//   at their .NET 8 defaults and `Config` prints what they were. What that
//   costs is a warmup long enough to reach tier 1 with PGO instrumentation
//   retired: each case is run for a fixed budget, the process then SLEEPS so
//   the call-counting background thread can promote, and the warmup runs
//   again. A harness that warms up once and measures has measured tier 0 on
//   whichever case it reached first.
//
//   Two vCPUs is the smallest contention a shared cache line can have. This is
//   four, single threaded throughout, so nothing here is a concurrency figure.
//
// R2's floor arm: `memcpy` copies the payload's own bytes. No encoder can
// produce N bytes for less than one copy of N bytes, so a managed encode near
// that line is a statement bounded on both sides rather than a ratio. On
// decode it is a weak floor except on the bulk rows, where a decode IS a copy,
// and the report says so per row rather than in general.

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Linq;
using System.Threading;
using Armonik.Ffi.Facade;

namespace Armonik.Ffi.Harness;

public sealed class Case
{
    public string Payload;
    public string Dir;          // encode | decode
    public string Arm;
    public Action<int> Run;
    public int Reps;
    public List<double> Times = new List<double>();
    public long AllocPerOp = -1;

    public double Min => Times.Count == 0 ? 0 : Times.Min();
    public double Max => Times.Count == 0 ? 0 : Times.Max();
    public double Med
    {
        get
        {
            if (Times.Count == 0) return 0;
            var s = Times.OrderBy(x => x).ToArray();
            return s.Length % 2 == 1 ? s[s.Length / 2] : (s[s.Length / 2 - 1] + s[s.Length / 2]) / 2;
        }
    }
}

public static unsafe class Bench
{
    private const int Rounds = 7;
    private const double BudgetMs = 40.0;

    public static int Run(params string[] argv)
    {
        var only = Environment.GetEnvironmentVariable("AK_BENCH_ONLY");
        var rows = Manifest.Load();
        var cases = new List<Case>();

        foreach (var a in ArmTable.All())
        {
            if (only != null && !only.Split(',').Select(s => s.Trim()).Contains(a.Id)) continue;
            var row = rows[a.Id];
            a.Build();

            int cap = Math.Max(row.Bytes + 4096, 8192);
            var dst = new byte[cap];
            var arms = a;

            // ---------- encode ----------
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "encode", Arm = "gp-tobytearray",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.GpToByteArray()); },
            });
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "encode", Arm = "gp-writeto",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.GpWriteTo(dst)); },
            });

            var bw = new BufWriter(cap);
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "encode", Arm = "gp-bufferwriter",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.GpWriteToBufferWriter(bw)); },
            });
            var mw = new BufWriter(cap);
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "encode", Arm = "gp-marshaller",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.GpMarshaller(mw)); },
            });

            // The core-ffi arm, M1 only: ResultRaw is the only element type whose
            // binding is built. Widening it is the next piece, not a measurement.
#if NET8_0_OR_GREATER
            CoreFfiM1 core = null;
            ListResultsResponse coreSrc = null;
            if (a.Root == "ListResultsResponse")
            {
                coreSrc = a.Id switch
                {
                    "P1.1" => BuildFacade.P1_1(), "P1.2" => BuildFacade.P1_2(),
                    "P1.3" => BuildFacade.P1_3(), _ => null,
                };
                if (coreSrc != null)
                {
                    core = new CoreFfiM1(coreSrc.Results.Count + 1, row.Bytes * 3 + 65536);
                    // AK_CHUNK sets elements per ak_elem_* call. 0 is the whole run.
                    // The Rust slice's host chunks at 150; matching it reproduces its
                    // crossing counts to the digit, so this is also what prices the
                    // difference on a runtime whose crossing is 4x Rust's.
                    var ck = Environment.GetEnvironmentVariable("AK_CHUNK");
                    if (!string.IsNullOrEmpty(ck) && int.TryParse(ck, out int ckv)) core.Chunk = ckv;
                    var warm = core.EncodeToArray(coreSrc);   // learn the length widths
                    var c2 = core; var cs2 = coreSrc;
                    cases.Add(new Case
                    {
                        Payload = a.Id, Dir = "encode", Arm = "core-ffi",
                        Run = n => { for (int i = 0; i < n; i++) { c2.Encode(cs2, out byte* p, out int l); Consume(l); } },
                    });
                    cases.Add(new Case
                    {
                        Payload = a.Id, Dir = "encode", Arm = "core-ffi fill",
                        Run = n => { for (int i = 0; i < n; i++) Consume(c2.Fill(cs2)); },
                    });
                    var wsrc = warm;
                    cases.Add(new Case
                    {
                        Payload = a.Id, Dir = "decode", Arm = "core-ffi",
                        Run = n => { for (int i = 0; i < n; i++) Consume(c2.Decode(wsrc, wsrc.Length).Results.Count); },
                    });
                }
            }

            // The core-ffi arm for M2. TaskDetailed is NOT a leaf, so unlike the
            // block above this one pays five reverse calls and about five forward
            // calls PER ELEMENT on encode, and section 7.2's two-calls-per-element
            // on decode. That difference is the measurement, not an inefficiency
            // in the binding: it is what the batching predicate stops buying.
            CoreFfiM2 core2 = null;
            if (a.Root == "ListTasksDetailedResponse")
            {
                ListTasksDetailedResponse src2 = a.Id switch
                {
                    "P2.1" => BuildFacade.P2_1(), "P2.2" => BuildFacade.P2_2(),
                    "P2.3" => BuildFacade.P2_3(), "P2.4" => BuildFacade.P2_4(),
                    "P2.5" => BuildFacade.P2_5(), _ => null,
                };
                if (src2 != null)
                {
                    var (nb2, ne2, by2) = CoreFfiGate2.Size(src2);
                    core2 = new CoreFfiM2(src2.Tasks.Count + 1, nb2, ne2, by2);
                    var ck2 = Environment.GetEnvironmentVariable("AK_CHUNK");
                    if (!string.IsNullOrEmpty(ck2) && int.TryParse(ck2, out int ckv2)) core2.Chunk = ckv2;
                    var warm2 = core2.EncodeToArray(src2);
                    var c3 = core2; var cs3 = src2;
                    cases.Add(new Case
                    {
                        Payload = a.Id, Dir = "encode", Arm = "core-ffi",
                        Run = n => { for (int i = 0; i < n; i++) { c3.Encode(cs3, out byte* p, out int l); Consume(l); } },
                    });
                    cases.Add(new Case
                    {
                        Payload = a.Id, Dir = "encode", Arm = "core-ffi fill",
                        Run = n => { for (int i = 0; i < n; i++) Consume(c3.Fill(cs3)); },
                    });
                    var w2 = warm2;
                    cases.Add(new Case
                    {
                        Payload = a.Id, Dir = "decode", Arm = "core-ffi",
                        Run = n => { for (int i = 0; i < n; i++) Consume(c3.Decode(w2, w2.Length).Tasks.Count); },
                    });
                }
            }
#endif

            var e = Enc.New(Codec.Sites, cap);
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "encode", Arm = "managed",
                Run = n => { for (int i = 0; i < n; i++) { e.Reset(); arms.ManagedWrite(ref e); Consume(e.Pos); } },
            });
            var e2 = Enc.New(Codec.Sites, cap);
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "encode", Arm = "managed-2pass",
                Run = n => { for (int i = 0; i < n; i++) { e2.Reset(); arms.ManagedWriteSized(ref e2); Consume(e2.Pos); } },
            });

            // The canonical bytes, for the decode arms and for the floor.
            var e3 = Enc.New(Codec.Sites, cap);
            arms.ManagedWrite(ref e3);
            var src = e3.ToArray();
            int slen = src.Length;
            var mc = new byte[cap];
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "encode", Arm = "memcpy floor",
                Run = n => { for (int i = 0; i < n; i++) { Buffer.BlockCopy(src, 0, mc, 0, slen); Consume(mc[0]); } },
            });

            // ---------- decode ----------
            var seq1 = new System.Buffers.ReadOnlySequence<byte>(src, 0, slen);
            var seqN = Segmented(src, slen, 16 * 1024);
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "decode", Arm = "gp-parse-seq",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.GpParseSequence(seq1)); },
            });
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "decode", Arm = "gp-parse-seg",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.GpParseSequence(seqN)); },
            });
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "decode", Arm = "gp-parse",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.GpParse(src, slen)); },
            });
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "decode", Arm = "managed-parse",
                Run = n => { for (int i = 0; i < n; i++) Consume(arms.ManagedParse(src, slen)); },
            });
            cases.Add(new Case
            {
                Payload = a.Id, Dir = "decode", Arm = "memcpy floor",
                Run = n => { for (int i = 0; i < n; i++) { Buffer.BlockCopy(src, 0, mc, 0, slen); Consume(mc[0]); } },
            });
        }

        if (cases.Count == 0) { Console.Error.WriteLine("no cases"); return 2; }

        Console.WriteLine("# {0} cases, {1} rounds, interleaved: every case runs once before any runs twice",
            cases.Count, Rounds);
        Console.WriteLine("# warmup budget {0} ms per case, twice, with a sleep between so the tiering",
            BudgetMs);
        Console.WriteLine("#   background thread can promote. R9: tiering and PGO stay ON.");
        Console.WriteLine();

        Warm(cases);
        Thread.Sleep(500);
        Warm(cases);
        Calibrate(cases);

        var sw = new Stopwatch();
        for (int r = 0; r < Rounds; r++)
        {
            foreach (var c in cases)
            {
                sw.Restart();
                c.Run(c.Reps);
                sw.Stop();
                c.Times.Add(sw.Elapsed.TotalMilliseconds * 1e6 / c.Reps);   // ns per op
            }
        }

        Allocations(cases);
        Report(cases, rows);
        return 0;
    }

    /// Shared with ContentSets so the two harnesses cannot drift in how they
    /// warm up, which is the one part of a managed measurement that silently
    /// changes what was measured (R9: tiering stays ON, so the warmup is what
    /// makes the timing a tier-1 timing).
    public static void WarmCalibrateMeasure(List<Case> cases)
    {
        Warm(cases);
        Thread.Sleep(500);
        Warm(cases);
        Calibrate(cases);
        var sw = new Stopwatch();
        for (int r = 0; r < Rounds; r++)
            foreach (var c in cases)
            {
                sw.Restart();
                c.Run(c.Reps);
                sw.Stop();
                c.Times.Add(sw.Elapsed.TotalMilliseconds * 1e6 / c.Reps);
            }
        Allocations(cases);
    }

    private static void Warm(List<Case> cases)
    {
        var sw = new Stopwatch();
        foreach (var c in cases)
        {
            int n = 1;
            sw.Restart();
            while (sw.Elapsed.TotalMilliseconds < BudgetMs)
            {
                c.Run(n);
                if (n < 1 << 20) n *= 2;
            }
        }
    }

    private static void Calibrate(List<Case> cases)
    {
        var sw = new Stopwatch();
        foreach (var c in cases)
        {
            int n = 1;
            while (true)
            {
                sw.Restart();
                c.Run(n);
                sw.Stop();
                if (sw.Elapsed.TotalMilliseconds >= BudgetMs || n >= (1 << 24)) break;
                double f = Math.Max(2.0, BudgetMs / Math.Max(sw.Elapsed.TotalMilliseconds, 0.01));
                n = (int)Math.Min((long)(n * Math.Min(f, 8.0)) + 1, 1 << 24);
            }
            c.Reps = n;
        }
    }

    /// Allocation per operation, measured OUTSIDE the timed rounds so that the
    /// measurement's own cost is not in the timing. `GetAllocatedBytesForCurrentThread`
    /// counts the allocation budget consumed, which includes what the GC later
    /// frees -- which is the number that matters for a managed codec, because
    /// what it allocates it will also collect.
    private static void Allocations(List<Case> cases)
    {
        foreach (var c in cases)
        {
            int n = Math.Max(1, Math.Min(c.Reps, 200));
            c.Run(n);                                       // settle any lazy init
            long before = GC.GetAllocatedBytesForCurrentThread();
            c.Run(n);
            long after = GC.GetAllocatedBytesForCurrentThread();
            c.AllocPerOp = (after - before) / n;
        }
    }

    private static void Report(List<Case> cases, Dictionary<string, PayloadRow> rows)
    {
        Console.WriteLine("payload  dir     arm              reps      min ns      med ns      max ns   ns/elem   alloc B/op   /gp-writeto  /gp-tba");
        Console.WriteLine(new string('-', 132));

        foreach (var g in cases.GroupBy(c => c.Payload + "|" + c.Dir))
        {
            var list = g.ToList();
            var row = rows[list[0].Payload];
            int elems = Math.Max(1, ArmTable.All().First(a => a.Id == list[0].Payload).Elements);
            // R14: the headline denominator is the path ArmoniK RUNS.
            var mar = list.FirstOrDefault(c => c.Arm == "gp-marshaller");
            var tba = list.FirstOrDefault(c => c.Arm == "gp-tobytearray");
            var pseq = list.FirstOrDefault(c => c.Arm == "gp-parse-seq");
            var baseEnc = mar ?? pseq;
            var baseAlt = tba ?? pseq;

            foreach (var c in list)
            {
                string r1 = baseEnc == null || baseEnc.Med == 0 ? "-" : F(c.Med / baseEnc.Med);
                string r2 = baseAlt == null || baseAlt.Med == 0 ? "-" : F(c.Med / baseAlt.Med);
                Console.WriteLine("{0,-8} {1,-6}  {2,-15} {3,6}  {4,10:F1}  {5,10:F1}  {6,10:F1}  {7,8:F2}  {8,11}   {9,-11}  {10}",
                    c.Payload, c.Dir, c.Arm, c.Reps, c.Min, c.Med, c.Max,
                    c.Med / elems, c.AllocPerOp, r1, r2);
            }
            Console.WriteLine();
        }

        Console.WriteLine("Reading this table.");
        Console.WriteLine("  THE BASELINE COLUMN IS THE PATH ARMONIK RUNS (R14). On encode that is");
        Console.WriteLine("    gp-marshaller: CalculateSize then WriteTo(IBufferWriter), which is the exact");
        Console.WriteLine("    sequence Grpc.Tools emits into the stub. On decode it is gp-parse-seq:");
        Console.WriteLine("    ParseFrom(ReadOnlySequence), which is what the marshaller hands the parser.");
        Console.WriteLine("    gp-writeto and gp-parse are the span forms, kept as second rows.");
        Console.WriteLine("  gp-bufferwriter is WriteTo(IBufferWriter) with NO top-level size pass, over a");
        Console.WriteLine("    reused BufWriter that resets rather than clearing. It is NOT a baseline an");
        Console.WriteLine("    ArmoniK client could use: Grpc.Tools' generated marshaller calls");
        Console.WriteLine("    SetPayloadLength(CalculateSize()) and THEN WriteTo(bufferWriter), because");
        Console.WriteLine("    gRPC needs the payload length before the frame header. What this arm prices");
        Console.WriteLine("    is therefore the SIZE PASS IN ISOLATION, not a faster incumbent.");
        Console.WriteLine("  /gp-tba is Google.Protobuf's ToByteArray, which is the call application code");
        Console.WriteLine("    actually writes. The two differ by one allocation of the output, and the gap");
        Console.WriteLine("    between the two columns is that allocation with nothing else in it.");
        Console.WriteLine("  managed against managed-2pass is a WITHIN-ARM delta in the same rounds (R4): one");
        Console.WriteLine("    pass with a learned length width against two passes with exact prefixes, which");
        Console.WriteLine("    is the shape Google.Protobuf uses. It prices the single pass ALONE.");
        Console.WriteLine("  memcpy floor is R2's bound: no encoder produces N bytes for less than one copy of");
        Console.WriteLine("    N bytes. On DECODE it is a weak floor except on the M5 bulk rows, where a");
        Console.WriteLine("    decode is a copy; elsewhere read it as a scale and not as a bound.");
        Console.WriteLine("  ns/elem divides by the element count, which for P1.1 (4 elements) is a per-MESSAGE");
        Console.WriteLine("    cost divided by four and is NOT comparable with P1.2's.");
        Console.WriteLine("  There are no crossing counts here: every arm in this table is in-process managed");
        Console.WriteLine("    code and crosses nothing. The counts R5 asks for belong to the core-ffi arm,");
        Console.WriteLine("    which is not built.");
    }

    private static string F(double d) => d.ToString("F3", CultureInfo.InvariantCulture);

    /// A ReadOnlySequence in N-byte segments. A 540 KB response does not arrive
    /// contiguous, and a single-segment sequence is the optimistic case.
    private static System.Buffers.ReadOnlySequence<byte> Segmented(byte[] src, int len, int seg)
    {
        if (len <= seg) return new System.Buffers.ReadOnlySequence<byte>(src, 0, len);
        Seg first = null, prev = null;
        for (int off = 0; off < len; off += seg)
        {
            var mem = new ReadOnlyMemory<byte>(src, off, Math.Min(seg, len - off));
            var node = new Seg(mem, off);
            if (first == null) first = node; else prev.SetNext(node);
            prev = node;
        }
        return new System.Buffers.ReadOnlySequence<byte>(first, 0, prev, prev.Memory.Length);
    }

    private sealed class Seg : System.Buffers.ReadOnlySequenceSegment<byte>
    {
        public Seg(ReadOnlyMemory<byte> m, long run) { Memory = m; RunningIndex = run; }
        public void SetNext(Seg n) => Next = n;
    }

    private static long _sink;

    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    private static void Consume(int v) { _sink += v; }

    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    private static void Consume(byte v) { _sink += v; }

    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
    private static void Consume(byte[] v) { _sink += v.Length; }
}
