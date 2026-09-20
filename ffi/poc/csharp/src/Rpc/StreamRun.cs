// Driving the streaming arm, and the gate that runs before any of it is timed.
//
// **Correctness before timing, in both directions.** The downstream gate
// re-encodes every decoded message and compares it with the exact bytes the
// server streamed; for the two `core-ffi` arms the re-encoder is the MANAGED
// codec, so the check is the core's decode against a different implementation's
// encode rather than against itself. The upstream gate is on the server, which
// compares every message the client's codec produced with the same wire, byte
// for byte, and returns the mismatch count. A non-zero count fails the run.
//
// **The total message count is held constant across concurrency levels**, as in
// the unary table, so a row at 16 streams moves the same bytes as a row at 1 and
// the columns are per message throughout.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Google.Protobuf;
using Grpc.Core;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Rpc;

public static class StreamRun
{
    public static object Sink;

    private sealed class SArm
    {
        public string Name;
        /// (invoker, messages, verify) -> messages received
        public Func<CallInvoker, int, bool, Task<int>> Down;
        /// (invoker, messages) -> (received, mismatched)
        public Func<CallInvoker, int, Task<(long N, long Bad)>> Up;
    }

    private static async Task<int> DriveDown<T>(CallInvoker inv, string mth, Marshaller<T> resp,
        int n, bool verify, Func<T, byte[]> reenc, byte[] wire) where T : class
    {
        var m = new Method<byte[], T>(MethodType.ServerStreaming, Streamer.Name, mth,
            Bench.Raw, resp);
        using var c = inv.AsyncServerStreamingCall(m, null, new CallOptions(),
            BitConverter.GetBytes(n));
        int got = 0;
        while (await c.ResponseStream.MoveNext(default))
        {
            got++;
            if (verify)
            {
                var b = reenc(c.ResponseStream.Current);
                if (b.Length != wire.Length)
                    throw new InvalidOperationException(mth + ": re-encoded " + b.Length
                        + " bytes, streamed " + wire.Length);
                for (int i = 0; i < b.Length; i++)
                    if (b[i] != wire[i])
                        throw new InvalidOperationException(mth + ": byte " + i + " differs");
            }
            Sink = c.ResponseStream.Current;
        }
        return got;
    }

    private static async Task<(long, long)> DriveUp<T>(CallInvoker inv, string mth,
        Marshaller<T> req, int n, T msg) where T : class
    {
        var m = new Method<T, byte[]>(MethodType.ClientStreaming, Streamer.Name, mth,
            req, Bench.Raw);
        using var c = inv.AsyncClientStreamingCall(m, null, new CallOptions());
        for (int i = 0; i < n; i++) await c.RequestStream.WriteAsync(msg);
        await c.RequestStream.CompleteAsync();
        var r = await c.ResponseAsync;
        return (BitConverter.ToInt64(r, 0), BitConverter.ToInt64(r, 8));
    }

    /// Re-encoders. Two per family, by the type the decode produced -- so the
    /// four arms of a family share them and a core decode is checked against the
    /// managed encode.
    private static byte[] Re22Gp(Gp.ListTasksDetailedResponse m) => m.ToByteArray();

    private static byte[] Re22Fac(ListTasksDetailedResponse m)
    {
        var e = Enc.New(Codec.Sites, Codec.SizeOfListTasksDetailedResponse(m) + 64);
        Codec.WriteListTasksDetailedResponse(ref e, m);
        return e.ToArray();
    }

    private static byte[] Re53Gp(Gp.UploadResultDataMessage m) => m.ToByteArray();

    private static byte[] Re53Fac(UploadResultDataMessage m)
    {
        var e = Enc.New(Codec.Sites, Codec.SizeOfUploadResultDataMessage(m) + 64);
        Codec.WriteUploadResultDataMessage(ref e, m);
        return e.ToArray();
    }

    /// Arms run in a fixed order and the FIRST one is the ratio base, so a
    /// systematic advantage to running early would bias every ratio in the
    /// table. `--reverse-arms` runs them back to front against the same base, so
    /// a position effect shows up as the floor and the incumbent trading places.
    public static bool Reverse;

    public static async Task Run(CallInvoker inv, int rounds, int[] levels, int msgs22, int msgs53)
    {
        var fac22 = BuildFacade.P2_2();
        var gp22 = BuildGp.P2_2();
        var fac53 = BuildFacade.P5_3();
        var gp53 = BuildGp.P5_3();

        var a22 = new[]
        {
            new SArm { Name = "gp-marshaller",
                Down = (i, n, v) => DriveDown(i, "Down22", Codecs.Incumbent, n, v, Re22Gp, Streamer.Wire22),
                Up   = (i, n)    => DriveUp(i, "Up22", Codecs.Incumbent, n, gp22) },
            new SArm { Name = "managed",
                Down = (i, n, v) => DriveDown(i, "Down22", Codecs.Managed, n, v, Re22Fac, Streamer.Wire22),
                Up   = (i, n)    => DriveUp(i, "Up22", Codecs.Managed, n, fac22) },
            new SArm { Name = "core-ffi",
                Down = (i, n, v) => DriveDown(i, "Down22", Codecs.Core, n, v, Re22Fac, Streamer.Wire22),
                Up   = (i, n)    => DriveUp(i, "Up22", Codecs.Core, n, fac22) },
            new SArm { Name = "core-ffi pull",
                Down = (i, n, v) => DriveDown(i, "Down22", Codecs.CorePull, n, v, Re22Fac, Streamer.Wire22),
                Up   = (i, n)    => DriveUp(i, "Up22", Codecs.Core, n, fac22) },
            // **The transport floor, and without it none of the rows above can
            // be read.** The same streamed messages with NO CODEC AT ALL: the
            // client's marshaller is the same `byte[]` passthrough the server
            // uses, so this row is the transport, the framing, the flow control
            // and the scheduling, and nothing else. Its ratio is the fraction of
            // the incumbent's CPU that no codec choice can touch.
            new SArm { Name = "raw (no codec)",
                Down = (i, n, v) => DriveDown(i, "Down22", RawCtx.M, n, v, b => b, Streamer.Wire22),
                Up   = (i, n)    => DriveUp(i, "Up22", RawCtx.M, n, Streamer.Wire22) },
        };
        var a53 = new[]
        {
            new SArm { Name = "gp-marshaller",
                Down = (i, n, v) => DriveDown(i, "Down53", ChunkCodecs.Incumbent, n, v, Re53Gp, Streamer.Wire53),
                Up   = (i, n)    => DriveUp(i, "Up53", ChunkCodecs.Incumbent, n, gp53) },
            new SArm { Name = "managed",
                Down = (i, n, v) => DriveDown(i, "Down53", ChunkCodecs.Managed, n, v, Re53Fac, Streamer.Wire53),
                Up   = (i, n)    => DriveUp(i, "Up53", ChunkCodecs.Managed, n, fac53) },
            new SArm { Name = "core-ffi",
                Down = (i, n, v) => DriveDown(i, "Down53", ChunkCodecs.Core, n, v, Re53Fac, Streamer.Wire53),
                Up   = (i, n)    => DriveUp(i, "Up53", ChunkCodecs.Core, n, fac53) },
            new SArm { Name = "core-ffi pull",
                Down = (i, n, v) => DriveDown(i, "Down53", ChunkCodecs.CorePull, n, v, Re53Fac, Streamer.Wire53),
                Up   = (i, n)    => DriveUp(i, "Up53", ChunkCodecs.Core, n, fac53) },
            new SArm { Name = "raw (no codec)",
                Down = (i, n, v) => DriveDown(i, "Down53", RawCtx.M, n, v, b => b, Streamer.Wire53),
                Up   = (i, n)    => DriveUp(i, "Up53", RawCtx.M, n, Streamer.Wire53) },
        };

        if (Reverse) { Array.Reverse(a22); Array.Reverse(a53); }
        Gate("P2.2", a22, inv);
        Gate("P5.3", a53, inv);

        Console.WriteLine("P2.2 ({0:N0} B a message), the field-heavy shape the rest of this slice",
            Streamer.Wire22.Length);
        Console.WriteLine("measures. Not an ArmoniK streaming shape; it is here as the control that says");
        Console.WriteLine("what streaming does to a codec that has real work to do.");
        Console.WriteLine();
        await Table(inv, a22, levels, rounds, msgs22);

        Console.WriteLine("P5.3 ({0:N0} B a message), ARMONIK'S ACTUAL STREAMING SHAPE: one chunk of a",
            Streamer.Wire53.Length);
        Console.WriteLine("result, the size its 2 MiB chunking produces.");
        Console.WriteLine();
        await Table(inv, a53, levels, rounds, msgs53);

        Console.WriteLine("The concurrency contract, priced. An encode context is not thread safe (ABI v1");
        Console.WriteLine("section 3) and the rust slice's positive control found that sharing one ABORTS");
        Console.WriteLine("the process rather than producing wrong bytes. `[ThreadStatic]` is how a managed");
        Console.WriteLine("host satisfies that without a lock, and what it costs is one context per thread");
        Console.WriteLine("the POOL decides to use, not per call:");
        Console.WriteLine();
        Console.WriteLine("  encode contexts created:          {0}", ChunkCodecs.EncContexts);
        Console.WriteLine("  decode contexts created:          {0}", ChunkCodecs.DecContexts);
        Console.WriteLine("  native staging they hold:         {0:N0} bytes", ChunkCodecs.StagingBytes);
        Console.WriteLine("  concurrency levels driven:        {0}", string.Join(", ", levels));
        Console.WriteLine("  processors:                       {0}", Environment.ProcessorCount);
        Console.WriteLine();
        Console.WriteLine("The count is a property of the THREAD POOL, not of the call rate: a context");
        Console.WriteLine("is created the first time the pool runs a marshaller on a thread that has not");
        Console.WriteLine("run one, and it is never freed. An ArmoniK worker with a large pool therefore");
        Console.WriteLine("holds a staging buffer per pool thread, sized from the LARGEST payload that");
        Console.WriteLine("thread ever encoded. That is the cost of the lock-free answer, and it is the");
        Console.WriteLine("argument for a pooled context with a try-lock rather than a thread-local one.");
        Console.WriteLine();
        Console.WriteLine("Sequence shape on the M5 arms: {0:N0} SEGMENTED bodies and {1:N0} single-segment,",
            ChunkCodecs.Segmented, ChunkCodecs.Single);
        Console.WriteLine("{0:N0} bytes copied to flatten them.", ChunkCodecs.Copied);
        Console.WriteLine();
    }

    /// Both directions, one stream each, before anything is timed.
    private static void Gate(string id, SArm[] arms, CallInvoker inv)
    {
        Console.WriteLine("gate {0}: 4 messages a stream, downstream re-encoded and compared with the", id);
        Console.WriteLine("streamed bytes, upstream compared on the server.");
        foreach (var a in arms)
        {
            int got = a.Down(inv, 4, true).GetAwaiter().GetResult();
            if (got != 4) throw new InvalidOperationException(id + " " + a.Name + ": received " + got);
            var (n, bad) = a.Up(inv, 4).GetAwaiter().GetResult();
            if (n != 4 || bad != 0)
                throw new InvalidOperationException(
                    id + " " + a.Name + ": server saw " + n + " messages, " + bad + " wrong");
            Console.WriteLine("  {0,-16} down 4/4 byte-identical, up 4/4 byte-identical", a.Name);
        }
        Console.WriteLine();
    }

    private static async Task Table(CallInvoker inv, SArm[] arms, int[] levels, int rounds, int msgs)
    {
        Console.WriteLine("dir   arm              streams   msgs/stream    us/msg   CPU us/msg   alloc B/msg   ratio   CPU spread");
        Console.WriteLine(new string('-', 108));
        foreach (var dir in new[] { "down", "up" })
        {
            foreach (int streams in levels)
            {
                int per = Math.Max(1, msgs / streams);
                var rows = new List<(string Arm, double Wall, double Cpu, long Alloc, double Lo, double Hi)>();
                foreach (var a in arms)
                {
                    Func<int, Task> one = dir == "down"
                        ? (n => a.Down(inv, n, false))
                        : (n => a.Up(inv, n));
                    await one(Math.Max(2, per / 8));                 // warm
                    var best = (wall: double.MaxValue, cpu: double.MaxValue, alloc: 0L);
                    // The SPREAD, not just the best. `ffi/CLAUDE.md`: ranges and
                    // spreads, never a single digit dressed up as precision. On
                    // this container a gap smaller than the spread is not a
                    // result, and several of the rows below are exactly that.
                    double lo = double.MaxValue, hi = 0;
                    for (int r = 0; r < rounds; r++)
                    {
                        GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
                        long b0 = GC.GetTotalAllocatedBytes(true);
                        var t0 = Process.GetCurrentProcess().TotalProcessorTime;
                        var sw = Stopwatch.StartNew();
                        var tasks = new Task[streams];
                        for (int k = 0; k < streams; k++) tasks[k] = one(per);
                        await Task.WhenAll(tasks);
                        sw.Stop();
                        var t1 = Process.GetCurrentProcess().TotalProcessorTime;
                        long b1 = GC.GetTotalAllocatedBytes(true);
                        int total = per * streams;
                        double cpu = (t1 - t0).TotalMicroseconds / total;
                        if (cpu < lo) lo = cpu;
                        if (cpu > hi) hi = cpu;
                        if (cpu < best.cpu)
                            best = (sw.Elapsed.TotalMicroseconds / total, cpu, (b1 - b0) / total);
                    }
                    rows.Add((a.Name, best.wall, best.cpu, best.alloc, lo, hi));
                }
                // The base is `gp-marshaller` (R14) wherever in the order it ran,
                // so `--reverse-arms` is comparable row for row with the forward
                // run rather than being a different table.
                double bas = rows.Find(x => x.Arm == "gp-marshaller").Cpu;
                foreach (var x in rows)
                    Console.WriteLine("{0,-5} {1,-16} {2,7} {3,13} {4,9:F1} {5,12:F1} {6,13} {7,7:F3}   {8,9}",
                        dir, x.Arm, streams, per, x.Wall, x.Cpu, x.Alloc, x.Cpu / bas,
                        string.Format("+{0:F0}%", 100.0 * (x.Hi - x.Lo) / x.Lo));
                Console.WriteLine();
            }
        }
    }
}
