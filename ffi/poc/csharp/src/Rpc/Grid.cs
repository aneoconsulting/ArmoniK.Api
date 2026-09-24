// The RPC arm as a 2x2 grid, because "the host's stack against the core's" moves
// the codec and the transport at once and the report has to say which one paid.
//
//   cell   codec              transport      what it is
//   A      Google.Protobuf    grpc-dotnet    R14's baseline, what ArmoniK runs
//   B      Google.Protobuf    core (tonic)   README section 13's outcome 2
//   C      core-ffi           core (tonic)   both halves of the proposal
//   D      core-ffi           grpc-dotnet    the codec alone, measured since stage 15
//
// B - A is the transport difference. C - B is the codec difference under the
// core's transport. D - A is the codec difference under grpc-dotnet.
//
// **The question only this slice can ask is whether C - B and D - A agree.** They
// are both "what the codec is worth", one under each transport. If they disagree
// then the two halves of the proposal are NOT additive and the report must stop
// presenting them as if they were. Nobody has had all four cells before.
//
// **The cells are built to MIRROR each other, which is what makes the subtraction
// legal.** Cell A parses straight out of gRPC's `ReadOnlySequence` with no
// flatten, because that is the sequence `Grpc.Tools` emits; cell B parses straight
// out of the core's own buffer with no copy. Cell D flattens into a reused array
// because the facade's `Dec` is over `byte[]`; cell C copies out of the core's
// buffer into the same reused array for the same reason. So each column carries
// the same copy and the subtraction is codec against codec.
//
// Every cell is the DOWNSTREAM direction: the client decodes a P2.2 response and
// the request is empty, exactly as stage 15's downstream table. The server is
// grpc-dotnet in every cell -- the core's transport is a client -- and its
// marshaller is a `byte[]` passthrough, so no cell does server-side codec work.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Text;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Google.Protobuf;
using Grpc.Core;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Rpc;

public static class Grid
{
    public static object Sink;
    public static bool Reverse;
    private static readonly byte[] Path = Encoding.UTF8.GetBytes("/armonik.ffi.Bench/Down");
    private static readonly byte[] Empty = Array.Empty<byte>();

    [ThreadStatic] private static byte[] _flat;

    private static unsafe byte[] Flatten(ak_bytes b)
    {
        int len = (int)b.len;
        if (_flat == null || _flat.Length < len) _flat = new byte[Math.Max(len, 1 << 20)];
        new ReadOnlySpan<byte>((void*)b.ptr, len).CopyTo(_flat);
        return _flat;
    }

    [ThreadStatic] private static CoreFfi_ListTasksDetailedResponse _dec;

    /// Cell C's codec: mirrors cell D, which flattens because `Dec` is over `byte[]`.
    private static object DecodeCore(ak_bytes b)
    {
        var c = _dec ??= new CoreFfi_ListTasksDetailedResponse();
        return c.Decode(Flatten(b), (int)b.len);
    }

    /// Cell B's codec: mirrors cell A, which parses the sequence in place.
    private static unsafe object DecodeGp(ak_bytes b) =>
        Gp.ListTasksDetailedResponse.Parser.ParseFrom(
            new ReadOnlySpan<byte>((void*)b.ptr, (int)b.len));

    // ---- the four cells, as one delegate each --------------------------------

    private sealed class Cell
    {
        public string Name, Codec, Transport, Delivery;
        public Func<int, Task> Run;
    }

    private static async Task GrpcCell<T>(CallInvoker inv, Marshaller<T> resp, int n) where T : class
    {
        var m = new Method<byte[], T>(MethodType.Unary, Bench.Name, "Down", Bench.Raw, resp);
        for (int i = 0; i < n; i++)
        {
            using var c = inv.AsyncUnaryCall(m, null, new CallOptions(), Empty);
            Sink = await c.ResponseAsync;
        }
    }

    private static async Task CoreCellCb(CoreChannel ch, Func<ak_bytes, object> dec, int n)
    {
        for (int i = 0; i < n; i++)
        {
            var b = await ch.CallCbAsync(Path, Empty);
            try { Sink = dec(b); }
            finally { CoreChannel.Release(ref b); }  // R-D9: a decode that throws still frees
        }
    }

    private static async Task CoreCellQ(CoreChannel ch, Func<ak_bytes, object> dec, int n)
    {
        for (int i = 0; i < n; i++)
        {
            var b = await ch.CallQAsync(Path, Empty);
            try { Sink = dec(b); }
            finally { CoreChannel.Release(ref b); }  // R-D9: a decode that throws still frees
        }
    }

    /// The blocking delivery parks a host thread inside the core for the whole
    /// call, so `inflight` of these is `inflight` thread-pool threads in a native
    /// frame. `Task.Run` is how that shape is expressed on .NET and it is the
    /// shape, not a harness artefact.
    private static Task CoreCellBlocking(CoreChannel ch, Func<ak_bytes, object> dec, int n) =>
        Task.Run(() =>
        {
            for (int i = 0; i < n; i++)
            {
                var b = ch.CallBlocking(Path, Empty);
                try { Sink = dec(b); }
                finally { CoreChannel.Release(ref b); }  // R-D9: a decode that throws still frees
            }
        });

    public static async Task Run(CallInvoker inv, CoreChannel ch, CoreChannel dflt,
                                 int rounds, int[] levels, int calls)
    {
        var cells = new List<Cell>
        {
            new Cell { Name = "A", Codec = "Google.Protobuf", Transport = "grpc-dotnet", Delivery = "-",
                       Run = n => GrpcCell(inv, Codecs.Incumbent, n) },
            new Cell { Name = "B", Codec = "Google.Protobuf", Transport = "core", Delivery = "callback",
                       Run = n => CoreCellCb(ch, DecodeGp, n) },
            new Cell { Name = "C", Codec = "core-ffi", Transport = "core", Delivery = "callback",
                       Run = n => CoreCellCb(ch, DecodeCore, n) },
            new Cell { Name = "D", Codec = "core-ffi", Transport = "grpc-dotnet", Delivery = "-",
                       Run = n => GrpcCell(inv, Codecs.Core, n) },
            // **The correction to stage 18, kept visible rather than silently
            // replacing it.** These two are the same cells against TONIC'S
            // DEFAULTS, which is what stage 18 measured, so the difference
            // between these rows and the pinned B and C above is how much of
            // that grid's transport gap was the settings.
            new Cell { Name = "B*", Codec = "Google.Protobuf", Transport = "core/dflt", Delivery = "callback",
                       Run = n => CoreCellCb(dflt, DecodeGp, n) },
            new Cell { Name = "C*", Codec = "core-ffi", Transport = "core/dflt", Delivery = "callback",
                       Run = n => CoreCellCb(dflt, DecodeCore, n) },
            new Cell { Name = "B", Codec = "Google.Protobuf", Transport = "core", Delivery = "blocking",
                       Run = n => CoreCellBlocking(ch, DecodeGp, n) },
            new Cell { Name = "C", Codec = "core-ffi", Transport = "core", Delivery = "blocking",
                       Run = n => CoreCellBlocking(ch, DecodeCore, n) },
            new Cell { Name = "B", Codec = "Google.Protobuf", Transport = "core", Delivery = "queue",
                       Run = n => CoreCellQ(ch, DecodeGp, n) },
            new Cell { Name = "C", Codec = "core-ffi", Transport = "core", Delivery = "queue",
                       Run = n => CoreCellQ(ch, DecodeCore, n) },
        };

        // stage 17 found an arm-position effect of 17 to 31 percent on one shape,
        // larger than every difference it was meant to rank. The grid is a table
        // of SUBTRACTIONS, so a position effect would land directly in B-A and
        // C-B. `--reverse-arms` runs the cells back to front against the same
        // cell-A base, so it shows up as the subtraction changing sign or size.
        if (Reverse) cells.Reverse();
        Gate(cells, inv, ch);

        Console.WriteLine("cell  codec             transport    delivery   inflight   calls    us/call   CPU us/call   alloc B/call   ratio   CPU spread");
        Console.WriteLine(new string('-', 128));
        // cell name + delivery -> CPU per level, for the subtraction below
        var cpu = new Dictionary<string, Dictionary<int, double>>();
        foreach (int inflight in levels)
        {
            double bas = 0;
            var rows = new List<(Cell C, double Wall, double Cpu, long Alloc, double Lo, double Hi, int Total)>();
            foreach (var cell in cells)
            {
                int per = Math.Max(1, calls / inflight);
                await cell.Run(Math.Max(4, per / 8));            // warm
                var best = (wall: double.MaxValue, c: double.MaxValue, alloc: 0L);
                double lo = double.MaxValue, hi = 0;
                for (int r = 0; r < rounds; r++)
                {
                    GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
                    long a0 = GC.GetTotalAllocatedBytes(true);
                    var t0 = Process.GetCurrentProcess().TotalProcessorTime;
                    var sw = Stopwatch.StartNew();
                    var tasks = new Task[inflight];
                    for (int k = 0; k < inflight; k++) tasks[k] = cell.Run(per);
                    await Task.WhenAll(tasks);
                    sw.Stop();
                    var t1 = Process.GetCurrentProcess().TotalProcessorTime;
                    long a1 = GC.GetTotalAllocatedBytes(true);
                    int total = per * inflight;
                    double c = (t1 - t0).TotalMicroseconds / total;
                    if (c < lo) lo = c;
                    if (c > hi) hi = c;
                    if (c < best.c) best = (sw.Elapsed.TotalMicroseconds / total, c, (a1 - a0) / total);
                }
                rows.Add((cell, best.wall, best.c, best.alloc, lo, hi, per * inflight));
                string key = cell.Name + "/" + cell.Delivery;
                if (!cpu.TryGetValue(key, out var d)) cpu[key] = d = new Dictionary<int, double>();
                d[inflight] = best.c;
                if (cell.Name == "A") bas = best.c;
            }
            foreach (var x in rows)
                Console.WriteLine("{0,-5} {1,-17} {2,-12} {3,-10} {4,8} {5,7} {6,10:F1} {7,13:F1} {8,14} {9,7:F3}   {10,9}",
                    x.C.Name, x.C.Codec, x.C.Transport, x.C.Delivery, inflight, x.Total,
                    x.Wall, x.Cpu, x.Alloc, x.Cpu / bas,
                    string.Format("+{0:F0}%", 100.0 * (x.Hi - x.Lo) / x.Lo));
            Console.WriteLine();
        }

        Console.WriteLine("THE SUBTRACTION. B-A is the transport, C-B and D-A are both the codec, one");
        Console.WriteLine("under each transport. Callback delivery for B and C. CPU us per call:");
        Console.WriteLine();
        Console.WriteLine("inflight    B-A (transport)   C-B (codec, core)   D-A (codec, grpc)   do they agree?");
        Console.WriteLine(new string('-', 88));
        foreach (int i in levels)
        {
            double a = cpu["A/-"][i], b = cpu["B/callback"][i], c = cpu["C/callback"][i], d = cpu["D/-"][i];
            double cb = c - b, da = d - a;
            string verdict = Math.Abs(cb - da) <= 0.10 * Math.Max(Math.Abs(cb), Math.Abs(da))
                ? "within 10%"
                : string.Format("differ by {0:F0}%", 100.0 * Math.Abs(cb - da) / Math.Max(1e-9, Math.Abs(da)));
            Console.WriteLine("{0,8} {1,17:F1} {2,19:F1} {3,19:F1}   {4}", i, b - a, cb, da, verdict);
        }
        Console.WriteLine();
    }

    /// Correctness before timing, in every cell. Each cell decodes one response
    /// and the decoded value is re-encoded and compared with the exact wire the
    /// server holds. The facade cells are re-encoded by the managed codec, the
    /// incumbent cells by `ToByteArray`.
    private static void Gate(List<Cell> cells, CallInvoker inv, CoreChannel ch)
    {
        Console.WriteLine("gate: one call per cell, decoded value re-encoded and compared with the");
        Console.WriteLine("server's wire ({0:N0} bytes).", Bench.Wire.Length);
        foreach (var cell in cells)
        {
            cell.Run(1).GetAwaiter().GetResult();
            byte[] got = Sink switch
            {
                Gp.ListTasksDetailedResponse g => g.ToByteArray(),
                ListTasksDetailedResponse f => ReFac(f),
                _ => throw new InvalidOperationException("cell " + cell.Name + " produced nothing"),
            };
            if (got.Length != Bench.Wire.Length)
                throw new InvalidOperationException(cell.Name + "/" + cell.Delivery
                    + ": re-encoded " + got.Length + ", wire " + Bench.Wire.Length);
            for (int i = 0; i < got.Length; i++)
                if (got[i] != Bench.Wire[i])
                    throw new InvalidOperationException(cell.Name + "/" + cell.Delivery
                        + ": byte " + i + " differs");
            Console.WriteLine("  cell {0} {1,-9} {2,-15} over {3,-11} byte-identical",
                cell.Name, cell.Delivery, cell.Codec, cell.Transport);
        }
        Console.WriteLine();
    }

    private static byte[] ReFac(ListTasksDetailedResponse m)
    {
        var e = Enc.New(Codec.Sites, Codec.SizeOfListTasksDetailedResponse(m) + 64);
        Codec.WriteListTasksDetailedResponse(ref e, m);
        return e.ToArray();
    }
}
