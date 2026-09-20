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
        bool pinned = !argv.Contains("--stack-default");
        bool tcp = argv.Contains("--tcp");
        int rounds = Arg(argv, "--rounds", 3);
        int calls = Arg(argv, "--calls", 300);
        var levels = new[] { 1, 8, 16 };

        if (pinned)
            // Before the first handler exists, or it does not take. This is what
            // actually HOLDS a pinned window; the property alone is a floor.
            AppContext.SetSwitch(
                "System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing", true);

        var facade = BuildFacade.P2_2();
        var gp = BuildGp.P2_2();
        var e = Enc.New(Codec.Sites, 1 << 21);
        Codec.WriteListTasksDetailedResponse(ref e, facade);
        Bench.Wire = e.ToArray();

        string sock = "/tmp/ak-ffi-rpc-" + Environment.ProcessId + ".sock";
        if (File.Exists(sock)) File.Delete(sock);

        var b = WebApplication.CreateSlimBuilder();
        b.Logging.ClearProviders();
        b.Services.AddGrpc(o => { o.MaxReceiveMessageSize = 64 << 20; o.MaxSendMessageSize = 64 << 20; });
        b.Services.AddSingleton<Bench>();
        b.Services.AddSingleton<IServiceMethodProvider<Bench>, BenchProvider>();
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
        await app.StartAsync();

        var handler = new SocketsHttpHandler
        {
            EnableMultipleHttp2Connections = false,
            PooledConnectionIdleTimeout = Timeout.InfiniteTimeSpan,
            InitialHttp2StreamWindowSize = pinned ? StreamWindow : 65535,
        };
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

        await app.StopAsync();
        if (File.Exists(sock)) File.Delete(sock);
        return 0;
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
        Console.WriteLine("# stream window:       {0} bytes (client InitialHttp2StreamWindowSize)",
            h.InitialHttp2StreamWindowSize);
        Console.WriteLine("# dynamic sizing:      {0}", pinned
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

    private static int Arg(string[] a, string name, int dflt)
    {
        int i = Array.IndexOf(a, name);
        return i >= 0 && i + 1 < a.Length && int.TryParse(a[i + 1], out int v) ? v : dflt;
    }
}
