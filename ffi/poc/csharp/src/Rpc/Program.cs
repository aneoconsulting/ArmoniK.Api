// The RPC arm: a real grpc-dotnet client against a real grpc-dotnet server,
// over a Unix domain socket, with only the CODEC differing between arms.
//
// design/SHAPES.md: "A marshaller arm is NOT an RPC arm: calling ProtoLiteUtils
// or a SerializationContext measures the codec path gRPC drives, which R14 asks
// for separately, and says nothing about the transport." ../Harness already has
// the marshaller arm as `gp-marshaller`. This is the other one.
//
// Both ends are grpc-dotnet proper. The server registers its methods through
// `IServiceMethodProvider<T>`, which is grpc-dotnet's own extensibility point,
// rather than through a code-generated service base -- because four arms need
// four marshallers on one method and a generated base fixes the marshaller at
// build time. Everything else is the shipped pipeline: Kestrel HTTP/2, the
// server call handler, the client's `CallInvoker`, flow control, trailers.
//
// **What is isolated, and how.** The SERVER's marshaller is a `byte[]`
// passthrough in every arm, so the server does no codec work at all and the
// only codec in the process is the client's. `Down` has the client decode a
// P2.2 response; `Up` has it encode a P2.2 request. Same transport, same
// bytes, same everything above the marshaller.
//
// **R7, and the part the transport makes non-obvious.** The pinned arm pins
// ARMONIK's configuration, not .NET's default -- R14 applied to the transport.
// On .NET that needs TWO settings: `InitialHttp2StreamWindowSize` says where the
// stream window STARTS and does not cap it (`Http2StreamWindowManager` doubles
// from there to a 16 MB default), and the
// `Http2FlowControl.DisableDynamicWindowSizing` AppContext switch is what holds
// it. The CONNECTION window needs nothing here and that is NOT true of the other
// stacks: `Http2Connection` hardcodes 64 MiB and raises it at setup, where tonic
// and grpc-java take the two separately.
//
// CPU per call is the headline. A loopback round trip is dominated by scheduling
// that neither codec is responsible for, so wall clock is reported beside it.

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Net.Sockets;
using System.Threading;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Grpc.AspNetCore.Server.Model;
using Grpc.Core;
using Grpc.Net.Client;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Server.Kestrel.Core;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Gp = Armonik.Ffi.Shapes.V1;

namespace Armonik.Ffi.Rpc;

/// The service the server hosts. Both directions carry `byte[]` on the SERVER
/// side, so the server does no codec work whatever the client does.
public sealed class Bench
{
    public const string Name = "armonik.ffi.Bench";
    public static byte[] Wire;

    public static readonly Marshaller<byte[]> Raw =
        Marshallers.Create<byte[]>(b => b, b => b);

    public static readonly Method<byte[], byte[]> Down = new Method<byte[], byte[]>(
        MethodType.Unary, Name, "Down", Raw, Raw);
    public static readonly Method<byte[], byte[]> Up = new Method<byte[], byte[]>(
        MethodType.Unary, Name, "Up", Raw, Raw);

    public Task<byte[]> DownH(byte[] req, ServerCallContext ctx) => Task.FromResult(Wire);
    public Task<byte[]> UpH(byte[] req, ServerCallContext ctx) => Task.FromResult(Array.Empty<byte>());
}

/// grpc-dotnet's own registration seam. A generated service base would fix the
/// marshaller at build time; four arms need four.
public sealed class BenchProvider : IServiceMethodProvider<Bench>
{
    public void OnServiceMethodDiscovery(ServiceMethodProviderContext<Bench> ctx)
    {
        ctx.AddUnaryMethod<byte[], byte[]>(Bench.Down, Array.Empty<object>(),
            (s, r, c) => s.DownH(r, c));
        ctx.AddUnaryMethod<byte[], byte[]>(Bench.Up, Array.Empty<object>(),
            (s, r, c) => s.UpH(r, c));
    }
}

public static class Program
{
    private const int StreamWindow = 4 * 1024 * 1024;   // ArmoniK's
    private const int Chunking = 2 * 1024 * 1024;       // ArmoniK's

    public static async Task<int> Main(string[] argv)
    {
        // The concurrency contract's POSITIVE CONTROL, and it runs in its own
        // process on purpose: the rust slice found that a shared encode context
        // ABORTS rather than returning an error, and an abort takes the whole
        // harness with it. Run it as a child and read the exit status.
        if (argv.Contains("--shared-ctx")) return SharedCtx(argv);
        // FIX-PLAN WP5 step 4: the RPC structs (generated from plan.rpc) against the
        // Rust declaration, by name both ways, with the harness's comparison.
        int li = Array.IndexOf(argv, "--layout");
        if (li >= 0)
        {
            var probe = Armonik.Ffi.Harness.Json.Parse(System.IO.File.ReadAllText(argv[li + 1]));
            RpcInit.Ensure();
            Console.WriteLine("# akrpc --layout: plan.rpc's structs (Generated/RpcAbi.cs) against the Rust declaration; ak_init() = {0}", RpcInit.Code);
            return Armonik.Ffi.Harness.LayoutCompare.Compare(probe, RpcLayout.Table(), argv[li + 1], null, 0, null);
        }

        // **Three transport configurations, and only one of them ships.**
        // `packages/rust/armonik-transport`'s `ClientConfig` has connect and
        // request timeouts, a rate limit, TCP keepalive and its interval and
        // retries, `tcp_nagle_algorithm`, the HTTP/2 PING interval, timeout and
        // while-idle flag, and a max header list size -- and NO stream or
        // connection window. `packages/csharp`'s `GrpcChannelProvider` sets no
        // window either, but on the UNIX SOCKET path it does set
        // `Http2FlowControl.DisableDynamicWindowSizing`, as a WORKAROUND for a
        // connectivity issue (grpc-dotnet #2361) rather than for throughput.
        //
        // So production is: no window pinned, .NET's 64 KB default, and the
        // auto-tuner that would otherwise grow it to 16 MB switched OFF. That is
        // a third configuration, it is the R14 baseline, and this arm had not
        // measured it -- `--pinned` is ArmoniK's INTENDED configuration and
        // `--stack-default` is .NET's, and neither is what ships.
        bool shipped = argv.Contains("--shipped");
        bool pinned = !shipped && !argv.Contains("--stack-default");
        bool tcp = argv.Contains("--tcp");
        // Streaming runs IN THE SAME PROCESS as the unary table, because the
        // claim it exists to test is "streaming moves the codec's share relative
        // to unary" and this slice has already found that two arms nothing
        // touched move up to 9 percent between sittings on this container.
        bool stream = argv.Contains("--stream");
        // ABI v1 section 9's transport, now that the core exports it. The grid is
        // A/B/C/D in ONE process against ONE server, because B-A and C-B are
        // subtractions and a subtraction across sittings is worth nothing here.
        bool grid = argv.Contains("--grid");
        // The blocking delivery stalled in WALL CLOCK twice while its CPU column
        // looked normal, never in the callback or queue delivery. Two
        // observations is an anecdote; `--park` is the measurement.
        bool park = argv.Contains("--park");
        // **The rust slice's Nagle diagnostic, reproduced here rather than
        // assumed away.** Their signature was that a 1 KB response cost MORE
        // than a 540 KB one over loopback TCP -- backwards for flow control,
        // exactly right for Nagle. `--tiny` swaps P2.2 for P1.1 so this arm can
        // be asked the same question. Both ends here are grpc-dotnet, not the
        // core's test server, so the defect should not be present -- which is a
        // prediction, and this is the measurement of it.
        bool nagle = argv.Contains("--nagle");
        // "Count crossings, do not infer them" (ffi/CLAUDE.md). The core now
        // exports `ak_rpc_counters`, so the transport's crossings per call are a
        // reading rather than an argument -- but only in a build with
        // `--features count`, and `ak_rpc_counting()` is what says which build
        // this is. R5's hazard is a harness that reads zeroes out of a
        // non-counting core and publishes "the boundary is free".
        bool crossings = argv.Contains("--crossings");
        // R-D9's gate: every delivery on a FAILED call must still reach
        // `ak_bytes_free` before it throws. Counted, not inferred: with the
        // counting core, a failed call's forward count includes the free, so
        // a binding that skips it reads one short.
        bool errorPath = argv.Contains("--error-path");
        int rounds = Arg(argv, "--rounds", 3);
        int calls = Arg(argv, "--calls", 300);
        var levels = new[] { 1, 8, 16 };

        if (pinned || shipped)
            // Before the first handler exists, or it does not take. This is what
            // actually HOLDS a pinned window; the property alone is a floor. The
            // SHIPPED arm sets exactly this and no window, which is
            // `GrpcChannelProvider.cs` line for line.
            AppContext.SetSwitch(
                "System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing", true);

        var e = Enc.New(Codec.Sites, 1 << 21);
        Codec.WriteListTasksDetailedResponse(ref e, BuildFacade.P2_2());
        Bench.Wire = e.ToArray();
        var es = Enc.New(Codec.Sites, 1 << 16);
        Codec.WriteListResultsResponse(ref es, BuildFacade.P1_1());
        byte[] wireSmall = es.ToArray();
        Streamer.Wire22 = Bench.Wire;
        var e53 = Enc.New(Codec.Sites, (1 << 20) + 4096);
        Codec.WriteUploadResultDataMessage(ref e53, BuildFacade.P5_3());
        Streamer.Wire53 = e53.ToArray();

        string sock = "/tmp/ak-ffi-rpc-" + Environment.ProcessId + ".sock";
        if (File.Exists(sock)) File.Delete(sock);

        var b = WebApplication.CreateSlimBuilder();
        b.Logging.ClearProviders();
        b.Services.AddGrpc(o => { o.MaxReceiveMessageSize = 64 << 20; o.MaxSendMessageSize = 64 << 20; });
        b.Services.AddSingleton<Bench>();
        b.Services.AddSingleton<IServiceMethodProvider<Bench>, BenchProvider>();
        b.Services.AddSingleton<Streamer>();
        b.Services.AddSingleton<IServiceMethodProvider<Streamer>, StreamerProvider>();
        int port = 0;
        b.WebHost.ConfigureKestrel(o =>
        {
            o.Limits.Http2.InitialStreamWindowSize = pinned ? StreamWindow : 98304;
            o.Limits.Http2.InitialConnectionWindowSize = pinned ? StreamWindow * 2 : 131072;
            if (tcp) o.ListenLocalhost(PickPort(out port), l => l.Protocols = HttpProtocols.Http2);
            else o.ListenUnixSocket(sock, l => l.Protocols = HttpProtocols.Http2);
        });
        var app = b.Build();
        app.MapGrpcService<Bench>();
        app.MapGrpcService<Streamer>();
        await app.StartAsync();

        // A correctness gate, so it runs before any timed table and prints none.
        if (errorPath)
        {
            int rc = await ErrorPath(sock, Arg(argv, "--calls", 50));
            await app.StopAsync();
            if (File.Exists(sock)) File.Delete(sock);
            return rc;
        }

        var handler = new SocketsHttpHandler
        {
            EnableMultipleHttp2Connections = false,
            PooledConnectionIdleTimeout = Timeout.InfiniteTimeSpan,
        };
        // The shipped arm does not touch the property at all, because
        // `GrpcChannelProvider` does not.
        if (!shipped) handler.InitialHttp2StreamWindowSize = pinned ? StreamWindow : 65535;
        if (!tcp)
            handler.ConnectCallback = async (c, ct) =>
            {
                var s = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
                await s.ConnectAsync(new UnixDomainSocketEndPoint(sock), ct);
                return new NetworkStream(s, true);
            };
        using var ch = GrpcChannel.ForAddress(
            tcp ? "http://127.0.0.1:" + port : "http://localhost",
            new GrpcChannelOptions
            {
                HttpHandler = handler,
                MaxReceiveMessageSize = 64 << 20,
                MaxSendMessageSize = 64 << 20,
                Credentials = ChannelCredentials.Insecure,
            });
        var inv = ch.CreateCallInvoker();

        Shipped = shipped;
        Header(tcp, pinned, Bench.Wire.Length, handler);

        // The four client-side codecs. The method NAME is the same in every arm,
        // so the server handler and the whole transport path are identical; only
        // the client's marshaller differs.
        var down = new (string Arm, Func<CallInvoker, int, Task> Run)[]
        {
            ("gp-marshaller", (i, n) => Drive(i, Bench.Name, "Down", Codecs.Incumbent, n)),
            ("managed",       (i, n) => Drive(i, Bench.Name, "Down", Codecs.Managed, n)),
            ("core-ffi",      (i, n) => Drive(i, Bench.Name, "Down", Codecs.Core, n)),
            ("core-ffi pull", (i, n) => Drive(i, Bench.Name, "Down", Codecs.CorePull, n)),
        };

        Console.WriteLine("DOWNSTREAM: the client DECODES a {0}-byte P2.2 response. The server's",
            Bench.Wire.Length);
        Console.WriteLine("marshaller is a byte[] passthrough in every arm, so the only codec work");
        Console.WriteLine("in the process is the client's.");
        Console.WriteLine();
        await Table(inv, down, levels, rounds, calls);

        Console.WriteLine("Sequence shape, which decides whether the facade's byte[] reader costs");
        Console.WriteLine("anything here: gRPC delivered {0} SEGMENTED bodies and {1} single-segment,",
            Codecs.Segmented, Codecs.Single);
        Console.WriteLine("copying {0:N0} bytes in total to flatten them. The copy is charged to the",
            Codecs.Copied);
        Console.WriteLine("facade arms; a ReadOnlySequence reader is what would remove it.");
        Console.WriteLine();

        if (stream)
        {
            Console.WriteLine(new string('=', 96));
            Console.WriteLine("STREAMING. Same process, same channel, same sitting as the unary table");
            Console.WriteLine("above -- which is the only way the comparison between them is readable,");
            Console.WriteLine("because two arms this slice did not touch moved 9 percent between sittings");
            Console.WriteLine("on this container.");
            Console.WriteLine(new string('=', 96));
            Console.WriteLine();
            StreamRun.Reverse = argv.Contains("--reverse-arms");
            await StreamRun.Run(inv, rounds, levels,
                Arg(argv, "--msgs22", 512), Arg(argv, "--msgs53", 128));
        }

        if (crossings)
        {
            using var cc = new CoreChannel("unix:" + sock, Arg(argv, "--core-workers", 2));
            cc.StartQueue();
            int n = Arg(argv, "--calls", 200);
            var path = System.Text.Encoding.UTF8.GetBytes("/armonik.ffi.Bench/Down");
            Console.WriteLine("# harness: rpc --crossings (ABI v1 section 9's transport, counted)");
            Console.WriteLine("# counting build: {0}", AkRpc.ak_rpc_counting() == 1 ? "YES" : "NO");
            if (AkRpc.ak_rpc_counting() != 1)
            {
                Console.WriteLine();
                Console.WriteLine("This core does NOT count. Every number below would be a zero and a");
                Console.WriteLine("zero here means 'not measured', not 'free'. Build the core with the");
                Console.WriteLine("count feature beside rpc and run this again.");
                await app.StopAsync();
                if (File.Exists(sock)) File.Delete(sock);
                return 2;
            }
            Console.WriteLine("# calls:         {0} per delivery", n);
            Console.WriteLine();
            Console.WriteLine("delivery     forward/call   reverse/call");
            Console.WriteLine(new string('-', 46));
            foreach (var d in new[] { "callback", "blocking", "queue" })
            {
                AkRpc.ak_rpc_counters_reset();
                for (int i = 0; i < n; i++)
                {
                    ak_bytes got;
                    if (d == "callback") got = await cc.CallCbAsync(path, Array.Empty<byte>());
                    else if (d == "queue") got = await cc.CallQAsync(path, Array.Empty<byte>());
                    else got = cc.CallBlocking(path, Array.Empty<byte>());
                    CoreChannel.Release(ref got);
                }
                ak_rpc_counters k;
                unsafe { AkRpc.ak_rpc_counters(&k); }
                Console.WriteLine("{0,-12} {1,12:F2} {2,14:F2}", d, (double)k.forward / n, (double)k.reverse / n);
            }
            Console.WriteLine();
            Console.WriteLine("ABI v1 section 9 says two crossings per call and none per field. A number");
            Console.WriteLine("that grows with the payload's field count would mean something in the");
            Console.WriteLine("transport knows about messages, and nothing in it does.");
            Console.WriteLine();
            await app.StopAsync();
            if (File.Exists(sock)) File.Delete(sock);
            return 0;
        }

        if (nagle)
        {
            // **No codec at all**: the client's marshaller is the same `byte[]`
            // passthrough the server uses, so this times the transport and
            // nothing else. The signature being looked for is the SMALL payload
            // costing MORE than the large one, which is backwards for flow
            // control and exactly right for Nagle.
            var raw = new Method<byte[], byte[]>(MethodType.Unary, Bench.Name, "Down",
                Bench.Raw, Bench.Raw);
            Console.WriteLine("# harness: rpc --nagle (the rust slice's diagnostic, on THIS stack)");
            Console.WriteLine("# transport:  {0}", tcp ? "loopback TCP" : "unix domain socket");
            Console.WriteLine("# both ends:  grpc-dotnet (Kestrel server, SocketsHttpHandler client),");
            Console.WriteLine("#             NOT the core's test server, which is where the defect was");
            Console.WriteLine("# codec:      none, byte[] passthrough both ways");
            Console.WriteLine();
            Console.WriteLine("payload bytes      calls     wall us/call");
            Console.WriteLine(new string('-', 48));
            byte[] big = Bench.Wire;
            foreach (var w in new[] { wireSmall, big })
            {
                Bench.Wire = w;
                int n = Arg(argv, "--calls", 300);
                for (int k = 0; k < Math.Max(8, n / 10); k++)
                {
                    using var warm = inv.AsyncUnaryCall(raw, null, new CallOptions(), Array.Empty<byte>());
                    Sink = await warm.ResponseAsync;
                }
                double best = double.MaxValue;
                for (int r = 0; r < Math.Max(3, rounds); r++)
                {
                    var sw = Stopwatch.StartNew();
                    for (int k = 0; k < n; k++)
                    {
                        using var c = inv.AsyncUnaryCall(raw, null, new CallOptions(), Array.Empty<byte>());
                        Sink = await c.ResponseAsync;
                    }
                    sw.Stop();
                    best = Math.Min(best, sw.Elapsed.TotalMicroseconds / n);
                }
                Console.WriteLine("{0,13:N0} {1,10} {2,16:F1}", w.Length, n, best);
            }
            Bench.Wire = big;
            Console.WriteLine();
            Console.WriteLine("If the small payload costs MORE than the large one, Nagle is on. If it");
            Console.WriteLine("costs less in proportion to its size, it is not.");
            Console.WriteLine();
            await app.StopAsync();
            if (File.Exists(sock)) File.Delete(sock);
            return 0;
        }

        if (park)
        {
            // **Does parking N host threads inside the core cost a managed host,
            // and is it the THREAD POOL that pays?**
            //
            // Section 9's case for a non-blocking delivery is that blocking in a
            // native frame pins a JVM virtual thread's carrier. .NET has no
            // carrier to pin, so the branch has no reason to expect anything
            // here -- but a host thread parked in `ak_call_unary` is a thread the
            // pool cannot reuse, and .NET's pool grows past its minimum at about
            // one or two threads a second. With 16 calls in flight and 4
            // processors that is a wait nothing in the CPU column can show.
            //
            // The experiment is one batch from a COLD pool, because the stall is
            // a growth cost and a warmed pool has already paid it. Three
            // configurations, each in the same process but ordered so the
            // blocking arm runs before anything can have grown the pool for it.
            using var pc = new CoreChannel("unix:" + sock, Arg(argv, "--core-workers", 2));
            int n = Arg(argv, "--park-inflight", 16);
            var path = System.Text.Encoding.UTF8.GetBytes("/armonik.ffi.Bench/Down");
            ThreadPool.GetMinThreads(out int wmin, out int cmin);
            Console.WriteLine("# harness: rpc --park (what a parked host thread costs a managed host)");
            Console.WriteLine("# processors:        {0}", Environment.ProcessorCount);
            Console.WriteLine("# pool minimum:      {0} worker threads", wmin);
            Console.WriteLine("# core workers:      {0}", Arg(argv, "--core-workers", 2));
            Console.WriteLine("# in flight:         {0}, ONE batch, from a cold pool", n);
            Console.WriteLine();

            async Task<double> Batch(bool blocking)
            {
                var sw = Stopwatch.StartNew();
                var ts = new Task[n];
                for (int k = 0; k < n; k++)
                    ts[k] = blocking
                        ? Task.Run(() => { var b = pc.CallBlocking(path, Array.Empty<byte>());
                                           CoreChannel.Release(ref b); })
                        : Go(pc, path);
                await Task.WhenAll(ts);
                sw.Stop();
                return sw.Elapsed.TotalMilliseconds;
            }

            double blockCold = await Batch(true);
            double cbCold = await Batch(false);
            double blockWarm = await Batch(true);
            ThreadPool.SetMinThreads(Math.Max(wmin, n + 8), cmin);
            double blockMin = await Batch(true);

            Console.WriteLine("batch                                        wall ms for {0} calls", n);
            Console.WriteLine(new string('-', 72));
            Console.WriteLine("blocking, COLD pool                          {0,10:F1}", blockCold);
            Console.WriteLine("callback, pool already grown by the above    {0,10:F1}", cbCold);
            Console.WriteLine("blocking, pool already grown                 {0,10:F1}", blockWarm);
            Console.WriteLine("blocking, SetMinThreads({0}), cold again   {1,10:F1}", n + 8, blockMin);
            Console.WriteLine();
            Console.WriteLine("The cost is the POOL GROWING to replace threads parked in a native frame:");
            Console.WriteLine("`SetMinThreads` removes it and nothing else does. **It is NOT a one-off a");
            Console.WriteLine("long-lived process pays once** -- the third row is the same batch again with");
            Console.WriteLine("the pool already grown, and it is slow too, because the pool retires idle");
            Console.WriteLine("threads between batches. A bursty caller pays it repeatedly. That claim was");
            Console.WriteLine("written here the other way round first and the measurement refuted it.");
            Console.WriteLine();
            await app.StopAsync();
            if (File.Exists(sock)) File.Delete(sock);
            return 0;
        }

        if (grid)
        {
            if (tcp) { Console.Error.WriteLine("--grid is UDS only"); return 1; }
            Console.WriteLine(new string('=', 128));
            Console.WriteLine("THE GRID. A and D above are the same two codecs over grpc-dotnet; B and C");
            Console.WriteLine("are the same two over the CORE's transport, dialling the same UNIX socket");
            Console.WriteLine("this process's Kestrel is listening on. Same process, same sitting.");
            Console.WriteLine(new string('=', 128));
            Console.WriteLine();
            // **Cells B and C are PINNED to what the .NET client does**, which
            // stage 18 did not do and which is an R7 defect in that grid: A and D
            // pinned ArmoniK's transport and B and C took tonic's defaults, so
            // part of what B-A measured may have been the settings rather than
            // the stack. The mirror is the CLIENT's configuration, because the
            // grid varies the client: a 4 MiB stream window, adaptive sizing OFF,
            // a 64 MiB connection window (which `Http2Connection` hardcodes and
            // this slice established from the runtime source), 64 MiB message
            // limits, and Nagle off, which is what `armonik-transport` ships.
            var pin = new ak_client_opts
            {
                stream_window = StreamWindow,
                connection_window = 64u << 20,
                adaptive_window = 0,
                max_recv_message = 64u << 20,
                max_send_message = 64u << 20,
                tcp_nagle = 0,
            };
            using var core = new CoreChannel("unix:" + sock, Arg(argv, "--core-workers", 2), pin);
            core.StartQueue();
            // The same client with tonic's defaults, kept as a LABELLED row so the
            // correction to stage 18 is visible rather than silently replacing it.
            using var coreDflt = new CoreChannel("unix:" + sock, Arg(argv, "--core-workers", 2));
            Console.WriteLine("# core transport:      tonic over unix:{0}", sock);
            Console.WriteLine("# core client pinned:  stream {0} B, connection {1} B, adaptive OFF, "
                + "msg limits {2} B, Nagle OFF", StreamWindow, 64 << 20, 64 << 20);
            Console.WriteLine("# counting build:      {0}", AkRpc.ak_rpc_counting() == 1
                ? "YES, ak_rpc_counters is live"
                : "no (this core is built without --features count), so the crossing "
                  + "columns are NOT read from it");
            Console.WriteLine("# core worker threads: {0} (ak_runtime_new, explicit -- ABI v1 section 3 "
                + "says never Runtime::new(), which reads the cgroup quota)",
                Arg(argv, "--core-workers", 2));
            Console.WriteLine("# deliveries:          callback (headline), blocking (labelled), "
                + "queue (second row, one drainer)");
            Console.WriteLine();
            Grid.Reverse = argv.Contains("--reverse-arms");
            await Grid.Run(inv, core, coreDflt, rounds, levels, calls);
        }

        await app.StopAsync();
        if (File.Exists(sock)) File.Delete(sock);
        return 0;
    }


    /// R-D9. A path the server does not serve comes back UNIMPLEMENTED, which
    /// the core reports as a non-OK status on every delivery. Each call must
    /// throw, and each must have crossed into `ak_bytes_free` first: the
    /// expected forward counts are the success path's (blocking 2, callback 3,
    /// queue 4, stage 19), because the free is on both paths. One short means
    /// the binding threw without freeing.
    private static async Task<int> ErrorPath(string sock, int n)
    {
        using var cc = new CoreChannel("unix:" + sock, 2);
        // The queue's drainer polls `ak_queue_next` with a 200 ms timeout, and
        // every poll is a forward crossing, so it is started only for the queue
        // rows (a running drainer put 2.02 on a blocking row), and the check
        // below is on the whole part of the per-call count: idle polls add a
        // few per row, a missing free subtracts one per call.
        var bad = System.Text.Encoding.UTF8.GetBytes("/armonik.ffi.Bench/NoSuchMethod");
        var good = System.Text.Encoding.UTF8.GetBytes("/armonik.ffi.Bench/Down");
        bool counting = AkRpc.ak_rpc_counting() == 1;
        Console.WriteLine("# harness: rpc --error-path (R-D9: free on a non-OK status)");
        Console.WriteLine("# counting build: {0}", counting ? "YES" : "NO (throw check only)");
        Console.WriteLine("# calls:          {0} per delivery per path", n);
        Console.WriteLine();
        Console.WriteLine("path    delivery     threw/calls   forward/call   reverse/call   expected fwd");
        Console.WriteLine(new string('-', 82));
        var expect = new Dictionary<string, int> { ["blocking"] = 2, ["callback"] = 3, ["queue"] = 4 };
        int fails = 0;
        foreach (var d in new[] { "blocking", "callback", "queue" })
        foreach (var (label, path) in new[] { ("ok", good), ("error", bad) })
        {
            if (d == "queue" && label == "ok") cc.StartQueue();
            if (counting) AkRpc.ak_rpc_counters_reset();
            int threw = 0;
            for (int i = 0; i < n; i++)
            {
                try
                {
                    ak_bytes got;
                    if (d == "callback") got = await cc.CallCbAsync(path, Array.Empty<byte>());
                    else if (d == "queue") got = await cc.CallQAsync(path, Array.Empty<byte>());
                    else got = cc.CallBlocking(path, Array.Empty<byte>());
                    CoreChannel.Release(ref got);
                }
                catch (InvalidOperationException) { threw++; }
            }
            double fwd = double.NaN, rev = double.NaN;
            if (counting)
            {
                ak_rpc_counters k;
                unsafe { AkRpc.ak_rpc_counters(&k); }
                fwd = (double)k.forward / n; rev = (double)k.reverse / n;
            }
            bool okThrow = label == "ok" ? threw == 0 : threw == n;
            bool okFwd = !counting || Math.Floor(fwd + 1e-9) == expect[d];
            if (!okThrow || !okFwd) fails++;
            Console.WriteLine("{0,-7} {1,-12} {2,5}/{3,-7} {4,12:F2} {5,14:F2} {6,14}  {7}",
                label, d, threw, n, fwd, rev, expect[d], okThrow && okFwd ? "PASS" : "FAIL");
        }
        Console.WriteLine();
        Console.WriteLine("error-path: {0} failure(s)", fails);
        return fails == 0 ? 0 : 1;
    }

    private static async Task Go(CoreChannel c, byte[] path)
    {
        var b = await c.CallCbAsync(path, Array.Empty<byte>());
        CoreChannel.Release(ref b);
    }

    private static int PickPort(out int port)
    {
        var l = new TcpListener(IPAddress.Loopback, 0);
        l.Start();
        port = ((IPEndPoint)l.LocalEndpoint).Port;
        l.Stop();
        return port;
    }

    /// One arm's calls, `n` of them with `inflight` outstanding.
    private static async Task Drive<T>(CallInvoker inv, string svc, string mth,
                                       Marshaller<T> resp, int n) where T : class
    {
        var m = new Method<byte[], T>(MethodType.Unary, svc, mth, Bench.Raw, resp);
        for (int i = 0; i < n; i++)
        {
            using var c = inv.AsyncUnaryCall(m, null, new CallOptions(), Array.Empty<byte>());
            Sink = await c.ResponseAsync;
        }
    }

    public static object Sink;

    private static async Task Table(CallInvoker inv,
        (string Arm, Func<CallInvoker, int, Task> Run)[] arms, int[] levels, int rounds, int calls)
    {
        Console.WriteLine("arm              inflight   calls    us/call   CPU us/call   alloc B/call   ratio");
        Console.WriteLine(new string('-', 88));
        var baseCpu = new Dictionary<int, double>();
        foreach (int inflight in levels)
        {
            foreach (var (arm, run) in arms)
            {
                await run(inv, Math.Max(8, calls / 10));       // warm
                var best = (wall: double.MaxValue, cpu: double.MaxValue, alloc: 0L);
                for (int r = 0; r < rounds; r++)
                {
                    GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
                    long a0 = GC.GetTotalAllocatedBytes(true);
                    var c0 = Process.GetCurrentProcess().TotalProcessorTime;
                    var sw = Stopwatch.StartNew();
                    var tasks = new Task[inflight];
                    int per = Math.Max(1, calls / inflight);
                    for (int k = 0; k < inflight; k++) tasks[k] = run(inv, per);
                    await Task.WhenAll(tasks);
                    sw.Stop();
                    var c1 = Process.GetCurrentProcess().TotalProcessorTime;
                    long a1 = GC.GetTotalAllocatedBytes(true);
                    int total = per * inflight;
                    double wall = sw.Elapsed.TotalMicroseconds / total;
                    double cpu = (c1 - c0).TotalMicroseconds / total;
                    if (cpu < best.cpu) best = (wall, cpu, (a1 - a0) / total);
                }
                if (arm == arms[0].Arm) baseCpu[inflight] = best.cpu;
                Console.WriteLine("{0,-16} {1,8} {2,7} {3,10:F1} {4,13:F1} {5,14} {6,7:F3}",
                    arm, inflight, Math.Max(1, calls / inflight) * inflight,
                    best.wall, best.cpu, best.alloc, best.cpu / baseCpu[inflight]);
            }
            Console.WriteLine();
        }
    }

    public static bool Shipped;

    private static void Header(bool tcp, bool pinned, int bytes, SocketsHttpHandler h)
    {
        Console.WriteLine("# harness: rpc (end to end, grpc-dotnet both ends)");
        Console.WriteLine("# utc:                 {0:yyyy-MM-ddTHH:mm:ssZ}", DateTime.UtcNow);
        Console.WriteLine("# runtime:             {0}",
            System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription);
        Console.WriteLine("# processors:          {0}", Environment.ProcessorCount);
        Console.WriteLine("# transport:           {0}", tcp
            ? "loopback TCP (the labelled second row)"
            : "UNIX DOMAIN SOCKET -- what packages/csharp defaults its worker and agent channels to");
        Console.WriteLine("# stream window:       {0} bytes (client InitialHttp2StreamWindowSize){1}",
            h.InitialHttp2StreamWindowSize,
            Shipped ? " -- .NET's DEFAULT, untouched, which is what packages/csharp does" : "");
        Console.WriteLine("# configuration:       {0}", Shipped
            ? "SHIPPED -- what packages/csharp's GrpcChannelProvider actually does on a UDS: "
              + "no window pinned and dynamic sizing OFF (a connectivity workaround, grpc-dotnet #2361)"
            : (pinned ? "ArmoniK's INTENDED configuration, which no ArmoniK client ships"
                      : ".NET's stack default"));
        Console.WriteLine("# dynamic sizing:      {0}", pinned || Shipped
            ? "OFF (Http2FlowControl.DisableDynamicWindowSizing). Without this the window "
              + "STARTS at the pinned value and doubles to a 16 MB cap"
            : "ON, the .NET default");
        Console.WriteLine("# connection window:   client 64 MiB, HARDCODED in Http2Connection and not "
            + "configurable on .NET;");
        Console.WriteLine("#                      server {0} (Kestrel). tonic and grpc-java take the two "
            + "separately", pinned ? (StreamWindow * 2) + " bytes" : "131072 bytes");
        Console.WriteLine("# chunking:            {0} bytes, ArmoniK's", Chunking);
        Console.WriteLine("# payload:             P2.2, {0} bytes", bytes);
        Console.WriteLine("# server codec:        byte[] passthrough in EVERY arm, so the only codec "
            + "work in the process is the client's");
        Console.WriteLine();
    }


    /// One encode context, several threads, ABI v1 section 3's "an encode
    /// context is not thread safe" taken at its word. `--per-thread` runs the
    /// same loop with a context each, which is the NEGATIVE control: it must
    /// report zero wrong, or the detector is not detecting anything.
    ///
    /// Three outcomes are possible and they are not equally bad. Wrong bytes
    /// with a zero exit status is the worst, because nothing tells the host.
    /// A managed exception is the best. An abort is what the rust slice saw,
    /// and on .NET it is worse than in rust: a .NET developer who shares an
    /// object expects an `InvalidOperationException` at the seam, not a
    /// SIGABRT with no stack in managed code.
    private static unsafe int SharedCtx(string[] argv)
    {
        int threads = Arg(argv, "--threads", 4);
        int iters = Arg(argv, "--iters", 20000);
        bool shared = !argv.Contains("--per-thread");
        var src = BuildFacade.P5_1();
        var w = Enc.New(Codec.Sites, 1 << 20);
        Codec.WriteUploadResultDataMessage(ref w, src);
        byte[] wire = w.ToArray();

        Console.WriteLine("# harness: rpc --shared-ctx (the concurrency contract's control)");
        Console.WriteLine("# utc:       {0:yyyy-MM-ddTHH:mm:ssZ}", DateTime.UtcNow);
        Console.WriteLine("# runtime:   {0}",
            System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription);
        Console.WriteLine("# mode:      {0}", shared
            ? "SHARED -- one ak_enc_ctx, " + threads + " threads encoding into it at once"
            : "per-thread -- one ak_enc_ctx each, the negative control");
        Console.WriteLine("# payload:   P5.1, {0} wire bytes, {1} encodes a thread", wire.Length, iters);
        Console.WriteLine("# expected:  zero wrong, or a managed exception, or the process does not "
            + "reach the last line");
        Console.Out.Flush();

        var one = new CoreFfi_UploadResultDataMessage();
        long wrong = 0, done = 0, threw = 0;
        var ts = new Thread[threads];
        for (int t = 0; t < threads; t++)
        {
            ts[t] = new Thread(() =>
            {
                var c = shared ? one : new CoreFfi_UploadResultDataMessage();
                for (int i = 0; i < iters; i++)
                {
                    try
                    {
                        c.Encode(src, out byte* q, out int len);
                        bool bad = len != wire.Length;
                        if (!bad)
                            for (int k = 0; k < len; k++)
                                if (q[k] != wire[k]) { bad = true; break; }
                        if (bad) Interlocked.Increment(ref wrong);
                    }
                    catch (Exception) { Interlocked.Increment(ref threw); }
                    Interlocked.Increment(ref done);
                }
            });
            ts[t].Start();
        }
        foreach (var th in ts) th.Join();

        Console.WriteLine();
        Console.WriteLine("encodes:   {0:N0}", done);
        Console.WriteLine("wrong:     {0:N0}", wrong);
        Console.WriteLine("threw:     {0:N0}", threw);
        Console.WriteLine("verdict:   {0}", wrong == 0 && threw == 0
            ? "survived, every byte correct"
            : (wrong > 0 ? "WRONG BYTES, and the process did not notice" : "threw, which is the good failure"));
        return wrong == 0 && threw == 0 ? 0 : 2;
    }

    private static int Arg(string[] a, string name, int dflt)
    {
        int i = Array.IndexOf(a, name);
        return i >= 0 && i + 1 < a.Length && int.TryParse(a[i + 1], out int v) ? v : dflt;
    }
}
