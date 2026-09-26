// The campaign runner's measuring half (design/CAMPAIGN.md, FIX-PLAN WP3). Driven by
// ../../run_campaign.sh, which pins each process (taskset to AK_CPU_CLIENT / AK_CPU_SERVER),
// runs the correctness gate first and writes the machine header. This file measures and
// writes one JSON object per sample (section 7); it summarises nothing and ranks nothing.
//
//   (the codec suite moved to ../BenchDotNet, BenchmarkDotNet; CAMPAIGN.md req 22a)
//   akrpc campaign --suite rpc-server --sock-shipped PATH --sock-pinned PATH   (one per launch)
//   akrpc campaign --suite rpc-warm   --sock-shipped PATH --sock-pinned PATH --calls W
//   akrpc campaign --suite rpc        --sock PATH --transport shipped|pinned --launch N
//                  --rounds R [--calls C] [--inflight 1,8,16] [--core-workers 2]
//   akrpc campaign --suite rpc        --sock PATH --transport shipped --counts FILE
//                  (the counting build: req 19's per-call counts of every cell)
//   akrpc campaign --suite calib      --launch N --rounds R [--iters N]
//
// Clocks (requirement 21): the calib suite reads CLOCK_THREAD_CPUTIME_ID of the
// one measuring thread (a crossing benchmark, req 20); the rpc suite reads process CPU,
// getrusage(RUSAGE_SELF), of the client process per sample (the server is another process)
// and wall time beside it (req 21). Process.TotalProcessorTime is not
// used anywhere in this file.

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Net.Sockets;
using System.Runtime;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Armonik.Ffi.Rpc;
using Google.Protobuf;
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

namespace Armonik.Ffi.Campaign;

internal static unsafe class Clock
{
    [StructLayout(LayoutKind.Sequential)] private struct Timespec { public long Sec, Nsec; }
    [StructLayout(LayoutKind.Sequential)] private struct Timeval { public long Sec, Usec; }
    [StructLayout(LayoutKind.Sequential)]
    private struct Rusage
    {
        public Timeval Utime, Stime;
        public long Maxrss, Ixrss, Idrss, Isrss, Minflt, Majflt, Nswap, Inblock, Oublock, Msgsnd, Msgrcv, Nsignals, Nvcsw, Nivcsw;
    }
    [DllImport("libc", SetLastError = false)] private static extern int clock_gettime(int clk, Timespec* ts);
    [DllImport("libc", SetLastError = false)] private static extern int getrusage(int who, Rusage* ru);
    private const int CLOCK_THREAD_CPUTIME_ID = 3, RUSAGE_SELF = 0;

    /// The calling thread's CPU time, nanoseconds (1 ns resolution on Linux).
    public static long ThreadCpuNs()
    {
        Timespec t;
        if (clock_gettime(CLOCK_THREAD_CPUTIME_ID, &t) != 0) throw new InvalidOperationException("clock_gettime");
        return t.Sec * 1_000_000_000L + t.Nsec;
    }

    /// The whole process's user + system CPU time (getrusage, microsecond resolution).
    public static long ProcessCpuNs()
    {
        Rusage r;
        if (getrusage(RUSAGE_SELF, &r) != 0) throw new InvalidOperationException("getrusage");
        return (r.Utime.Sec + r.Stime.Sec) * 1_000_000_000L + (r.Utime.Usec + r.Stime.Usec) * 1000L;
    }

    public static long WallNs() => (long)(Stopwatch.GetTimestamp() * (1e9 / Stopwatch.Frequency));
}

/// One sample, one JSON line (requirement 28). Fields not applicable are left out.
internal sealed class Sample
{
    public string Suite, Arm, Cell, Payload, Content, Dir, Mode, Transport, Build, Extra;
    public int Inflight = -1, Launch, Round;
    public long CpuNs, WallNs, Iters;

    public string Json()
    {
        var sb = new StringBuilder("{\"slice\":\"csharp\"");
        void S(string k, string v) { if (v != null) sb.Append(",\"").Append(k).Append("\":\"").Append(v).Append('"'); }
        void N(string k, long v) { sb.Append(",\"").Append(k).Append("\":").Append(v.ToString(CultureInfo.InvariantCulture)); }
        S("suite", Suite); S("arm", Arm); S("cell", Cell); S("payload", Payload); S("content", Content);
        S("dir", Dir); S("unknown_mode", Mode); S("transport", Transport); S("build", Build);
        if (Inflight >= 0) N("inflight", Inflight);
        N("launch", Launch); N("round", Round); N("cpu_ns", CpuNs); N("wall_ns", WallNs); N("iters", Iters);
        if (Extra != null) sb.Append(',').Append(Extra);
        return sb.Append('}').ToString();
    }
}

public static class CampaignMain
{
    private static string Opt(string[] a, string k, string d) { int i = Array.IndexOf(a, k); return i >= 0 && i + 1 < a.Length ? a[i + 1] : d; }
    private static int OptI(string[] a, string k, int d) => int.Parse(Opt(a, k, d.ToString(CultureInfo.InvariantCulture)), CultureInfo.InvariantCulture);
    private static double OptD(string[] a, string k, double d) => double.Parse(Opt(a, k, d.ToString(CultureInfo.InvariantCulture)), CultureInfo.InvariantCulture);

    public static async Task<int> Run(string[] a)
    {
        var suite = Opt(a, "--suite", "");
        switch (suite)
        {
            case "calib": return Calib(a);
            case "rpc-server": return await Server(a);
            case "rpc": return await Rpc(a);
            case "rpc-warm": return await WarmServer(a);
            default: Console.Error.WriteLine("campaign: --suite calib|rpc|rpc-server|rpc-warm"); return 2;
        }
    }

    /// The process-level part of requirement 27's header; the runner writes the machine.
    private static void Header(string suite, string extra)
    {
        Console.WriteLine("# campaign suite {0}: raw samples, one JSON object per line; nothing summarised here", suite);
        Console.WriteLine("# runtime:        {0}", RuntimeInformation.FrameworkDescription);
        Console.WriteLine("# incumbent:      Google.Protobuf {0}; Grpc.Net.Client {1}; Grpc.AspNetCore.Server {2}", Ver(typeof(Google.Protobuf.MessageParser)),
            Ver(typeof(GrpcChannel)), Ver(typeof(Grpc.AspNetCore.Server.GrpcServiceOptions)));
        Console.WriteLine("# JIT:            TieredCompilation {0}, TieredPGO {1}, ReadyToRun {2} (net8.0 defaults unless set)",
            Env("DOTNET_TieredCompilation", "default(on)"), Env("DOTNET_TieredPGO", "default(on)"), Env("DOTNET_ReadyToRun", "default(on)"));
        Console.WriteLine("# GC:             server={0}, concurrent={1}, latency={2}", GCSettings.IsServerGC,
            AppContext.TryGetSwitch("System.GC.Concurrent", out var c) ? c.ToString() : "default(on)", GCSettings.LatencyMode);
        Console.WriteLine("# managed codec:  plan utf8={0} unknown={1} limit={2}", Armonik.Ffi.Facade.Codec.Utf8Policy,
            Armonik.Ffi.Facade.Codec.UnknownMode, Armonik.Ffi.Facade.Codec.Limit);
        Console.WriteLine("# process CPUs:   affinity mask 0x{0:x} ({1} logical CPUs visible)", (long)Process.GetCurrentProcess().ProcessorAffinity, Environment.ProcessorCount);
        Console.WriteLine("# {0}", extra);
    }

    private static string Ver(Type t) =>
        (t.Assembly.GetCustomAttributes(typeof(System.Reflection.AssemblyInformationalVersionAttribute), false).FirstOrDefault()
         as System.Reflection.AssemblyInformationalVersionAttribute)?.InformationalVersion?.Split('+')[0] ?? t.Assembly.GetName().Version.ToString();

    private static string Env(string k, string d) { var v = Environment.GetEnvironmentVariable(k); return string.IsNullOrEmpty(v) ? d : v; }

    // ============================================================== calib suite

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static ulong Back(ulong x) => x + 1;

    private static unsafe int Calib(string[] a)
    {
        int launch = OptI(a, "--launch", 1), rounds = OptI(a, "--rounds", 5), iters = OptI(a, "--iters", 10_000_000);
        AbiInit.Ensure();
        Header("calib", string.Format(CultureInfo.InvariantCulture,
            "launch {0}, rounds {1}, {2} calls per sample; forward = ak_noop through the generated P/Invoke (LibraryImport on net8.0); reverse = ak_noop_reverse into an [UnmanagedCallersOnly] callback (one forward AND one reverse crossing per call); warm-up {2} calls each", launch, rounds, iters));
        Console.WriteLine(ThreadLine("the crossing loop runs on the main thread; no core runtime is created"));
        ulong s = 0;
        delegate* unmanaged[Cdecl]<ulong, ulong> cb = &Back;
        for (int i = 0; i < iters; i++) { s += Abi.ak_noop((ulong)i); s += Abi.ak_noop_reverse(cb, (ulong)i); }
        for (int r = 1; r <= rounds; r++)
        {
            foreach (var arm in (r % 2 == 1) ? new[] { "crossing-forward", "crossing-forward-reverse" } : new[] { "crossing-forward-reverse", "crossing-forward" })
            {
                long w0 = Clock.WallNs(), c0 = Clock.ThreadCpuNs();
                if (arm == "crossing-forward") for (int i = 0; i < iters; i++) s += Abi.ak_noop((ulong)i);
                else for (int i = 0; i < iters; i++) s += Abi.ak_noop_reverse(cb, (ulong)i);
                long c1 = Clock.ThreadCpuNs(), w1 = Clock.WallNs();
                Console.WriteLine(new Sample { Suite = "calib", Arm = arm, Launch = launch, Round = r, CpuNs = c1 - c0, WallNs = w1 - w0, Iters = iters }.Json());
            }
        }
        Console.WriteLine("# end (sink {0})", s & 1);
        return 0;
    }

    // ============================================================== rpc suite

    public const string Svc = "armonik.ffi.Campaign";
    private static readonly Marshaller<byte[]> Raw = Marshallers.Create<byte[]>(b => b, b => b);
    private static readonly Method<byte[], byte[]> MDown = new Method<byte[], byte[]>(MethodType.Unary, Svc, "Down", Raw, Raw);
    private static readonly Method<byte[], byte[]> MUp = new Method<byte[], byte[]>(MethodType.Unary, Svc, "Up", Raw, Raw);

    private static byte[] P22Wire() => BuildGp.P2_2().ToByteArray();

    private const int Window = 4 * 1024 * 1024;

    /// CAMPAIGN req 4 (R-H34): every log header states each stack's worker thread counts.
    internal static string ThreadLine(string extra)
    {
        ThreadPool.GetMinThreads(out int minW, out int minIo);
        ThreadPool.GetMaxThreads(out int maxW, out int maxIo);
        return string.Format(CultureInfo.InvariantCulture, "# threads:        .NET thread pool min {0} worker / {1} IO, max {2} worker / {3} IO, {4} pool thread(s) now; {5} logical CPUs visible; {6}",
            minW, minIo, maxW, maxIo, ThreadPool.ThreadCount, Environment.ProcessorCount, extra);
    }

    /// The server: ONE separate process per launch (requirement 13 as amended, R-H33), serving
    /// every cell of both builds over both transport configurations: two Kestrel hosts in this
    /// process, `shipped` on one Unix socket and `pinned` (4 MiB windows) on the other, since
    /// Kestrel's HTTP/2 limits are per host. It returns PRE-SERIALISED P2.2 bytes for direction
    /// (a) and decodes the request with the incumbent for direction (b) -- identical work in
    /// every cell.
    private static async Task<int> Server(string[] a)
    {
        CampaignService.Wire = P22Wire();
        var apps = new List<WebApplication>();
        foreach (var t in new[] { "shipped", "pinned" })
        {
            var sock = Opt(a, "--sock-" + t, null);
            if (sock == null) continue;
            bool pinned = t == "pinned";
            if (File.Exists(sock)) File.Delete(sock);
            var b = WebApplication.CreateSlimBuilder();
            b.Logging.ClearProviders();
            b.Services.AddGrpc(o => { o.MaxReceiveMessageSize = 64 << 20; o.MaxSendMessageSize = 64 << 20; });
            b.Services.AddSingleton<CampaignService>();
            b.Services.AddSingleton<IServiceMethodProvider<CampaignService>, CampaignProvider>();
            b.WebHost.ConfigureKestrel(o =>
            {
                if (pinned)
                {
                    o.Limits.Http2.InitialStreamWindowSize = Window;
                    o.Limits.Http2.InitialConnectionWindowSize = Window;
                }
                o.ListenUnixSocket(sock, l => l.Protocols = HttpProtocols.Http2);
            });
            var app = b.Build();
            app.MapGrpcService<CampaignService>();
            await app.StartAsync();
            apps.Add(app);
            Console.WriteLine("# campaign rpc-server: {0} on {1} (Kestrel {2}); pid {3}", t, sock, pinned ? "stream/connection window 4 MiB" : "defaults", Environment.ProcessId);
        }
        if (apps.Count == 0) { Console.Error.WriteLine("rpc-server: --sock-shipped and/or --sock-pinned"); return 2; }
        Console.WriteLine("# runtime:        {0}; Grpc.AspNetCore.Server {1}; GC server={2}", RuntimeInformation.FrameworkDescription, Ver(typeof(Grpc.AspNetCore.Server.GrpcServiceOptions)), GCSettings.IsServerGC);
        Console.WriteLine(ThreadLine("one process, " + apps.Count + " Kestrel host(s) sharing the thread pool"));
        await Task.WhenAll(apps.Select(x => x.WaitForShutdownAsync()));
        Console.WriteLine("# served: {0} Down, {1} Up (every client of this launch, the server warm-up included)", CampaignService.Downs, CampaignService.Ups);
        return 0;
    }

    private sealed class Abort : Exception { public Abort(string m) : base(m) { } }

    private static byte[] _wire;
    private static long _sink;
    [ThreadStatic] private static CoreFfi_ListTasksDetailedResponse _core;
    [ThreadStatic] private static byte[] _buf;
    [ThreadStatic] private static Enc _he;
    private static CoreFfi_ListTasksDetailedResponse Core => _core ??= new CoreFfi_ListTasksDetailedResponse();
    /// Cell D's marshaller runs on whichever thread-pool thread Grpc.Net uses, so it takes a
    /// core-ffi context from this pool and gives it back (no per-thread context to create when
    /// a new pool thread appears; the pool grows to the calls in flight during the warm-up).
    private static readonly System.Collections.Concurrent.ConcurrentBag<CoreFfi_ListTasksDetailedResponse> _cores = new System.Collections.Concurrent.ConcurrentBag<CoreFfi_ListTasksDetailedResponse>();
    private static CoreFfi_ListTasksDetailedResponse RentCore() => _cores.TryTake(out var c) ? c : new CoreFfi_ListTasksDetailedResponse();
    private static byte[] Buf(int n) => (_buf == null || _buf.Length < n) ? (_buf = new byte[Math.Max(n, 1 << 20)]) : _buf;

    private static byte[] Flatten(ReadOnlySequence<byte> s, out int n)
    {
        n = checked((int)s.Length);
        var b = Buf(n);
        s.CopyTo(b);
        return b;
    }

    private static Method<byte[], T> DownMethod<T>(Func<DeserializationContext, T> de) =>
        new Method<byte[], T>(MethodType.Unary, Svc, "Down", Raw, Marshallers.Create<T>((m, c) => throw new NotSupportedException(), de));
    private static Method<T, byte[]> UpMethod<T>(Action<T, SerializationContext> se) =>
        new Method<T, byte[]>(MethodType.Unary, Svc, "Up", Marshallers.Create<T>(se, c => throw new NotSupportedException()), Raw);

    private static void CheckLen(int got, int want, string cell)
    {
        if (got != want) throw new Abort(cell + ": response length " + got + ", expected " + want);
    }

    /// host-gen's decode of the response (cells E and F): the generated managed codec.
    private static ListTasksDetailedResponse HostDecode(byte[] b, int n, bool retain, string cell)
    {
        var d = new Dec { Buf = b, Pos = 0, End = n, Err = 0 };
        var m = new ListTasksDetailedResponse();
        if (retain) HostR.ReadListTasksDetailedResponse(ref d, m, 0); else Armonik.Ffi.Facade.Codec.ReadListTasksDetailedResponse(ref d, m, 0);
        if (d.Err != 0) throw new Abort(cell + ": managed decode " + d.Err);
        return m;
    }

    private static GrpcChannel NewGrpc(string sock, bool pinned)
    {
        var handler = new SocketsHttpHandler { EnableMultipleHttp2Connections = false, PooledConnectionIdleTimeout = Timeout.InfiniteTimeSpan };
        if (pinned) handler.InitialHttp2StreamWindowSize = Window;
        handler.ConnectCallback = async (c, ct) =>
        {
            var s = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
            await s.ConnectAsync(new UnixDomainSocketEndPoint(sock), ct);
            return new NetworkStream(s, true);
        };
        return GrpcChannel.ForAddress("http://localhost", new GrpcChannelOptions
        {
            HttpHandler = handler, MaxReceiveMessageSize = 64 << 20, MaxSendMessageSize = 64 << 20, Credentials = ChannelCredentials.Insecure,
        });
    }

    /// The core's transport: the same switch (requirement 17). shipped = the stack's windows
    /// with adaptive sizing off (what packages/csharp sets); pinned = 4 MiB and 4 MiB.
    private static ak_client_opts CoreOpts(bool pinned) => new ak_client_opts
    {
        stream_window = pinned ? (uint)Window : 0, connection_window = pinned ? (uint)Window : 0,
        adaptive_window = 0, max_recv_message = 64u << 20, max_send_message = 64u << 20, tcp_nagle = pinned ? 0 : -1,
    };

    /// One cell of the grid in one direction: a blocking call (B, C, E: the core's blocking
    /// delivery) or an async one (A, D, F: Grpc.Net's idiomatic `await` of the call; the core's
    /// callback and queue extras).
    internal sealed class Cell
    {
        public string Name, Dir, Mode, Channel;
        public Action One;
        public Func<Task> OneAsync;
    }

    /// Every cell of this build, each on ITS OWN channel (req 13 as amended): Grpc.Net cells a
    /// GrpcChannel with its own handler and connection, core cells a CoreChannel on the one
    /// shared runtime `rt`. Directions: a (empty request, P2.2 response, decoded), a+read (the
    /// same, then every field read), b (P2.2 request, empty response).
    private static List<Cell> BuildCells(string sock, bool pinned, IntPtr rt, int want, List<IDisposable> owned, List<string> chans, bool extras)
    {
        var g22 = BuildGp.P2_2();
        var f22 = BuildFacade.P2_2();
        var downPath = Encoding.UTF8.GetBytes("/" + Svc + "/Down");
        var upPath = Encoding.UTF8.GetBytes("/" + Svc + "/Up");
        byte[] gUpBytes = g22.ToByteArray();
        var cells = new List<Cell>();
        string ModeOf(string name) => name.EndsWith("-retain", StringComparison.Ordinal) ? "retain"
            : name.EndsWith("-nounk", StringComparison.Ordinal) ? "no-unknown"
            : name.StartsWith("C", StringComparison.Ordinal) || name.StartsWith("D", StringComparison.Ordinal) || name.StartsWith("E", StringComparison.Ordinal) || name.StartsWith("F", StringComparison.Ordinal) ? "drop" : "default";

        CoreChannel CoreCh(string name, bool queue = false)
        {
            var ch = new CoreChannel(rt, "unix:" + sock, CoreOpts(pinned));
            if (queue) ch.StartQueue();
            owned.Add(ch);
            chans.Add(name + ": its own core channel" + (queue ? " + completion queue drainer" : ""));
            return ch;
        }
        CallInvoker GrpcCh(string name)
        {
            var ch = NewGrpc(sock, pinned);
            owned.Add(ch);
            chans.Add(name + ": its own GrpcChannel (SocketsHttpHandler, one HTTP/2 connection)");
            return ch.CreateCallInvoker();
        }

        // --- B, C, E: the core's BLOCKING delivery (requirement 16); codec 0 = incumbent,
        //     1 = core-ffi, 2 = host-gen.
        unsafe void CoreDown(CoreChannel ch, string cell, int codec, bool retain, bool read)
        {
            ak_bytes r = default;
            int rc;
            fixed (byte* p = downPath) rc = AkRpc.ak_call_unary(ch.Client, p, (nuint)downPath.Length, null, 0, &r);
            try
            {
                if (rc != AkRpc.AK_OK) throw new Abort(cell + ": status " + rc);
                CheckLen((int)r.len, want, cell);
                if (codec == 0)
                {
                    var m = Gp.ListTasksDetailedResponse.Parser.ParseFrom(new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len));
                    if (read) _sink += Touch.G_ListTasksDetailedResponse(m);
                    return;
                }
                // The managed decoders take a managed buffer: the response is copied once
                // (charged to the cell).
                var b = Buf((int)r.len);
                new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len).CopyTo(b);
                ListTasksDetailedResponse fm;
                if (codec == 1)
                {
                    int dr = Core.TryDecode(b, (int)r.len, retain, out fm);
                    if (dr < 0) throw new Abort(cell + ": core decode " + dr + (dr == CoreFfi_ListTasksDetailedResponse.UNDELIVERED ? " (UNDELIVERED)" : ""));
                }
                else fm = HostDecode(b, (int)r.len, retain, cell);
                if (read) _sink += Touch.F_ListTasksDetailedResponse(fm);
            }
            finally { AkRpc.ak_bytes_free(&r); }
        }
        unsafe void CoreUp(CoreChannel ch, string cell, int codec, bool retain)
        {
            byte* q;
            int n;
            byte[] pinnedArr = null;
            if (codec == 0)
            {
                n = g22.CalculateSize();
                pinnedArr = Buf(n);
                g22.WriteTo(new Span<byte>(pinnedArr, 0, n));
            }
            else if (codec == 1)
            {
                int er = Core.TryEncode(f22, retain, out q, out n);
                if (er < 0) throw new Abort(cell + ": core encode " + er);
                Send(q, n);
                return;
            }
            else
            {
                if (_he.Buf == null) _he = Enc.New(Armonik.Ffi.Facade.Codec.Sites, 1 << 16);
                _he.Reset();
                if (retain) HostR.WriteListTasksDetailedResponse(ref _he, f22); else Armonik.Ffi.Facade.Codec.WriteListTasksDetailedResponse(ref _he, f22);
                if (_he.Err != 0) throw new Abort(cell + ": managed encode " + _he.Err);
                n = _he.Pos;
                pinnedArr = _he.Buf;
            }
            fixed (byte* pp = pinnedArr) Send(pp, n);

            void Send(byte* body, int len)
            {
                ak_bytes r = default;
                int rc;
                fixed (byte* p = upPath) rc = AkRpc.ak_call_unary(ch.Client, p, (nuint)upPath.Length, body, (nuint)len, &r);
                try { if (rc != AkRpc.AK_OK) throw new Abort(cell + " up: status " + rc); CheckLen((int)r.len, 0, cell + " up"); }
                finally { AkRpc.ak_bytes_free(&r); }
            }
        }
        void AddCore(string name, int codec, bool retain)
        {
            var ch = CoreCh(name);
            cells.Add(new Cell { Name = name, Dir = "a", Mode = ModeOf(name), One = () => CoreDown(ch, name, codec, retain, false) });
            cells.Add(new Cell { Name = name, Dir = "a+read", Mode = ModeOf(name), One = () => CoreDown(ch, name, codec, retain, true) });
            cells.Add(new Cell { Name = name, Dir = "b", Mode = ModeOf(name), One = () => CoreUp(ch, name, codec, retain) });
        }

        // --- A, D, F: Grpc.Net, the idiomatic `await` of an AsyncUnaryCall (requirement 16 as
        //     amended, R-H30: async is what .NET production code does), with the marshaller
        //     swapped: A = Grpc.Tools' generated shape; D = core-ffi; F = host-gen. The request
        //     serializers are the codec suite's (Ops_*.Ser*), so its encode-transport rows time
        //     this very code.
        void AddGrpc<T>(string name, Method<byte[], T> down, Func<T, long> touch, Method<T, byte[]> up, T req) where T : class
        {
            var inv = GrpcCh(name);
            cells.Add(new Cell { Name = name, Dir = "a", Mode = ModeOf(name), OneAsync = async () => { await inv.AsyncUnaryCall(down, null, new CallOptions(), Array.Empty<byte>()); } });
            cells.Add(new Cell { Name = name, Dir = "a+read", Mode = ModeOf(name), OneAsync = async () => { var m = await inv.AsyncUnaryCall(down, null, new CallOptions(), Array.Empty<byte>()); _sink += touch(m); } });
            cells.Add(new Cell { Name = name, Dir = "b", Mode = ModeOf(name), OneAsync = async () => { var r = await inv.AsyncUnaryCall(up, null, new CallOptions(), req); CheckLen(r.Length, 0, name + " up"); } });
        }
        var aDown = DownMethod(c => { CheckLen(c.PayloadLength, want, "A"); return Gp.ListTasksDetailedResponse.Parser.ParseFrom(c.PayloadAsReadOnlySequence()); });
        var aUp = UpMethod<Gp.ListTasksDetailedResponse>(Ops_ListTasksDetailedResponse.SerInc);
        Method<byte[], ListTasksDetailedResponse> DDown(bool retain) => DownMethod(c =>
        {
            CheckLen(c.PayloadLength, want, "D");
            var b = Flatten(c.PayloadAsReadOnlySequence(), out int n);
            var core = RentCore();
            int rc = core.TryDecode(b, n, retain, out var m);
            _cores.Add(core);
            if (rc < 0) throw new Abort("D: core decode " + rc + (rc == CoreFfi_ListTasksDetailedResponse.UNDELIVERED ? " (UNDELIVERED)" : ""));
            return m;
        });
        Method<ListTasksDetailedResponse, byte[]> DUp(bool retain) => UpMethod<ListTasksDetailedResponse>((m, c) =>
        {
            var core = RentCore();
            try { Ops_ListTasksDetailedResponse.SerFfi(core, m, retain, c); }
            finally { _cores.Add(core); }
        });
        Method<byte[], ListTasksDetailedResponse> FDown(bool retain) => DownMethod(c =>
        {
            CheckLen(c.PayloadLength, want, "F");
            var b = Flatten(c.PayloadAsReadOnlySequence(), out int n);
            return HostDecode(b, n, retain, "F");
        });
        Method<ListTasksDetailedResponse, byte[]> FUp(bool retain) => UpMethod<ListTasksDetailedResponse>((m, c) => Ops_ListTasksDetailedResponse.SerHost(m, retain, c));
        Func<Gp.ListTasksDetailedResponse, long> tg = Touch.G_ListTasksDetailedResponse;
        Func<ListTasksDetailedResponse, long> tf = Touch.F_ListTasksDetailedResponse;

        AddGrpc("A", aDown, tg, aUp, g22);
        AddCore("B", 0, false);
#if AK_NO_UNKNOWN_FIELDS
        // WP5 step 10, req 12: the NO-UNKNOWN client (unknown fields compiled out of the core,
        // the binding and host-gen): C, D, E and F in mode no-unknown; A and B as controls.
        AddCore("C-nounk", 1, false);
        AddGrpc("D-nounk", DDown(false), tf, DUp(false), f22);
        AddCore("E-nounk", 2, false);
        AddGrpc("F-nounk", FDown(false), tf, FUp(false), f22);
#else
        AddCore("C-retain", 1, true);
        AddCore("C-drop", 1, false);
        AddGrpc("D-retain", DDown(true), tf, DUp(true), f22);
        AddGrpc("D-drop", DDown(false), tf, DUp(false), f22);
        AddCore("E-retain", 2, true);
        AddCore("E-drop", 2, false);
        AddGrpc("F-retain", FDown(true), tf, FUp(true), f22);
        AddGrpc("F-drop", FDown(false), tf, FUp(false), f22);
        // The core's callback and queue deliveries: labelled extra rows (requirement 16),
        // awaited; C.* in drop mode.
        foreach (var (name, queue, core) in new[] { ("B.callback", false, false), ("B.queue", true, false), ("C.callback", false, true), ("C.queue", true, true) })
        {
            if (!extras) break;
            var ch = CoreCh(name, queue);
            async Task Down(bool read)
            {
                var r = queue ? await ch.CallQAsync(downPath, Array.Empty<byte>()) : await ch.CallCbAsync(downPath, Array.Empty<byte>());
                try
                {
                    CheckLen((int)r.len, want, name);
                    long h = Decode(r, core, read);
                    if (read) _sink += h;
                }
                finally { CoreChannel.Release(ref r); }
            }
            async Task Up()
            {
                var body = core ? Core.EncodeToArray(f22) : gUpBytes;
                var r = queue ? await ch.CallQAsync(upPath, body) : await ch.CallCbAsync(upPath, body);
                try { CheckLen((int)r.len, 0, name + " up"); }
                finally { CoreChannel.Release(ref r); }
            }
            cells.Add(new Cell { Name = name, Dir = "a", Mode = core ? "drop" : "default", OneAsync = () => Down(false) });
            cells.Add(new Cell { Name = name, Dir = "a+read", Mode = core ? "drop" : "default", OneAsync = () => Down(true) });
            cells.Add(new Cell { Name = name, Dir = "b", Mode = core ? "drop" : "default", OneAsync = Up });
        }
#endif
        foreach (var c in cells) c.Channel = c.Name;
        GC.KeepAlive(gUpBytes);
        return cells;
    }

    private static unsafe long Decode(ak_bytes r, bool core, bool read)
    {
        if (core)
        {
            var b = Buf((int)r.len);
            new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len).CopyTo(b);
            if (Core.TryDecode(b, (int)r.len, false, out var fm) < 0) throw new Abort("core decode");
            return read ? Touch.F_ListTasksDetailedResponse(fm) : 0;
        }
        var m = Gp.ListTasksDetailedResponse.Parser.ParseFrom(new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len));
        return read ? Touch.G_ListTasksDetailedResponse(m) : 0;
    }

    /// `n` calls over `k` in flight: blocking cells on the caller threads (CallerPool), async
    /// cells as `k` concurrent async loops on the thread pool, each awaiting its calls back to
    /// back.
    private static async Task RunCell(Cell c, CallerPool pool, int n, int k)
    {
        if (c.One != null) { pool.Run(c.One, n, k); return; }
        var ts = new Task[k];
        for (int i = 0; i < k; i++)
        {
            int cnt = n / k + (i < n % k ? 1 : 0);
            ts[i] = Task.Run(async () => { for (int j = 0; j < cnt; j++) await c.OneAsync(); });
        }
        await Task.WhenAll(ts);
    }

    /// CAMPAIGN req 13 (R-H33): the server's warm-up, before any client of the launch starts
    /// its round 1: `--calls` calls per direction from EACH client transport (Grpc.Net, and the
    /// core's transport) to each of the server's sockets, every call checked.
    private static async Task<int> WarmServer(string[] a)
    {
        int calls = OptI(a, "--calls", 2000);
        var wire = P22Wire();
        var down = Encoding.UTF8.GetBytes("/" + Svc + "/Down");
        var up = Encoding.UTF8.GetBytes("/" + Svc + "/Up");
        AppContext.SetSwitch("System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing", true);
        IntPtr rt = AkRpc.ak_runtime_new(2);
        try
        {
            foreach (var t in new[] { "shipped", "pinned" })
            {
                var sock = Opt(a, "--sock-" + t, null);
                if (sock == null) continue;
                bool pinned = t == "pinned";
                using (var ch = NewGrpc(sock, pinned))
                {
                    var inv = ch.CreateCallInvoker();
                    for (int i = 0; i < calls; i++)
                    {
                        CheckLen((await inv.AsyncUnaryCall(MDown, null, new CallOptions(), Array.Empty<byte>())).Length, wire.Length, "warm Grpc.Net a");
                        CheckLen((await inv.AsyncUnaryCall(MUp, null, new CallOptions(), wire)).Length, 0, "warm Grpc.Net b");
                    }
                }
                using (var cc = new CoreChannel(rt, "unix:" + sock, CoreOpts(pinned)))
                {
                    for (int i = 0; i < calls; i++)
                    {
                        var r = cc.CallBlocking(down, Array.Empty<byte>());
                        try { CheckLen((int)r.len, wire.Length, "warm core a"); } finally { CoreChannel.Release(ref r); }
                        r = cc.CallBlocking(up, wire);
                        try { CheckLen((int)r.len, 0, "warm core b"); } finally { CoreChannel.Release(ref r); }
                    }
                }
                Console.WriteLine("# server warm-up: {0} ({1}): {2} calls per direction (a, b) from Grpc.Net and {2} from the core's transport, every call checked", t, sock, calls);
            }
        }
        catch (Exception e)
        {
            Console.WriteLine("# ABORT: server warm-up: {0}: {1}", e.GetType().Name, e.Message);
            return 1;
        }
        finally { AkRpc.ak_runtime_destroy(rt); }
        return 0;
    }

    private static async Task<int> Rpc(string[] a)
    {
        Armonik.Ffi.Bdn.JitTiers.Start();   // the JIT tier read back (R-H2, req 24)
        // WP5 step 10: the client's binding variant must match the core it loaded.
        var vwhy = AbiVariant.CheckLoadedCore();
        if (vwhy != null) { Console.WriteLine("# ABORT: core variant mismatch: " + vwhy); Console.WriteLine("# no samples written"); return 1; }
        var sock = Opt(a, "--sock", null);
        var transport = Opt(a, "--transport", "shipped");
        bool pinned = transport == "pinned";
        bool counts = a.Contains("--counts");
        int launch = OptI(a, "--launch", 1), rounds = OptI(a, "--rounds", 5), calls = OptI(a, "--calls", 64), workers = OptI(a, "--core-workers", 2);
        var levels = Opt(a, "--inflight", "1,8,16").Split(',').Select(x => int.Parse(x, CultureInfo.InvariantCulture)).ToArray();
        // packages/csharp's GrpcChannelProvider, on the Unix-socket path: dynamic window
        // sizing OFF and no window set. That is "shipped"; "pinned" adds the 4 MiB windows.
        AppContext.SetSwitch("System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing", true);
        _wire = P22Wire();
        int want = _wire.Length;
        // A CONTROL (run_campaign.sh --suite rpc --plant): a wrong expected length must abort
        // the run with no sample written (requirement 18).
        if (Environment.GetEnvironmentVariable("AK_CAMPAIGN_PLANT") == "len") want += 1;

        var owned = new List<IDisposable>();
        var chans = new List<string>();
        IntPtr rt = AkRpc.ak_runtime_new((uint)workers);
        if (rt == IntPtr.Zero) { Console.WriteLine("# ABORT: ak_runtime_new"); return 1; }
        try
        {
            // The counting run builds no extra rows: their queue drainer calls the core on its
            // own thread, which a per-call count must not see.
            var cells = BuildCells(sock, pinned, rt, want, owned, chans, extras: !counts);
            if (counts) return CountCells(cells, a);
            return await Grid(cells, chans, a, sock, transport, pinned, launch, rounds, calls, workers, levels, want);
        }
        finally
        {
            foreach (var d in owned) d.Dispose();
            AkRpc.ak_runtime_destroy(rt);
        }
    }

    private static async Task<int> Grid(List<Cell> cellList, List<string> chans, string[] a, string sock, string transport, bool pinned, int launch, int rounds, int calls, int workers, int[] levels, int want)
    {
        var cells = (from c in cellList from k in levels select (C: c, k)).ToList();
        var o = CoreOpts(pinned);
        Header("rpc", string.Format(CultureInfo.InvariantCulture,
            "build " + AbiVariant.Name + " (WP5 step 10); launch {0}, rounds {1}, {2} calls per sample, in flight {3}; transport {4} (client: DisableDynamicWindowSizing{5}; Kestrel {6}; core: ak_client_opts stream {7} connection {8} adaptive 0 nagle {9}); Unix socket {10} (req 17: UDS); the server is ONE separate process for this launch, serving both builds and both transports, warmed before any client (its log states the calls); cells (req 12 as amended): A incumbent over Grpc.Net, B incumbent over the core's transport, C core-ffi over the core's transport, D core-ffi over Grpc.Net, E host-gen over the core's transport, F host-gen over Grpc.Net; C, D, E, F in each unknown-field mode of this build (full: -retain = decision 11's options armed at every position and ak_uencode_* / CodecRetain, -drop = reset with NULL and ak_encode_* / Codec; no-unknown build: -nounk, A and B its controls); a retained decode that leaves a grown buffer undelivered fails its call; directions (req 14 as amended): a = empty request, P2.2 response ({11} B) decoded, a+read = the same then every field read (Touch), b = P2.2 request decoded by the server, empty response; delivery (req 16 as amended): B, C, E the core's BLOCKING call on caller threads ({12}, created before the warm-up and shared by every cell); A, D, F Grpc.Net's idiomatic async call (`await CallInvoker.AsyncUnaryCall`, as Grpc.Tools' generated client does), k in flight = k concurrent async loops on the thread pool; the core's callback/queue rows are labelled extras, awaited the same way; one channel per cell for the whole launch (req 13 as amended), opened and warmed before round 1; the core's cells share ONE core runtime with {13} worker thread(s); cell order per round: a seeded shuffle of launch and round; every call checked (status and length); ratios, where the aggregation forms them, from per-launch medians (req 30)",
            launch, rounds, calls, string.Join("/", levels), transport, pinned ? " + InitialHttp2StreamWindowSize 4 MiB" : ", no window set",
            pinned ? "stream/connection window 4 MiB" : "defaults", o.stream_window, o.connection_window, o.tcp_nagle, sock, want, levels.Max(), workers));
        foreach (var ch in chans) Console.WriteLine("# channel:        " + ch);
        Console.WriteLine(ThreadLine("caller threads " + levels.Max() + " (blocking cells); core runtime: 1, " + workers + " worker thread(s) (ak_runtime_new), shared by every core channel of this process"));

        var samples = new List<(Sample S, DateTime T0, DateTime T1)>();
        int warmRounds = 0; long lastWarmJits = -1;
        using var pool = new CallerPool(levels.Max());
        try
        {
            // R-H2 / req 24: warm-up rounds over every cell (64 calls each, 0.5 s apart) until a
            // round compiles nothing of the measured code (runtime JIT events), at most 10.
            while (warmRounds < 10)
            {
                warmRounds++;
                long before = Armonik.Ffi.Bdn.JitTiers.Received;
                foreach (var c in cells) await RunCell(c.C, pool, 64, c.k);
                long seen;
                do { seen = Armonik.Ffi.Bdn.JitTiers.Received; Thread.Sleep(500); } while (seen != Armonik.Ffi.Bdn.JitTiers.Received);
                lastWarmJits = Armonik.Ffi.Bdn.JitTiers.Received - before;
                if (lastWarmJits == 0) break;
            }
            Console.WriteLine("# warm-up:        {0} round(s) of 64 calls per cell, direction and in-flight level (so every channel is opened and warmed before round 1), 0.5 s apart; the last round compiled {1} method(s) of measured code (JIT events read back)", warmRounds, lastWarmJits);
            Console.WriteLine(ThreadLine("after the warm-up"));
            for (int r = 1; r <= rounds; r++)
            {
                GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
                // R-H18 / R-H23: the cell order of each round is a seeded shuffle of launch and
                // round, so no two launches share a schedule; the order is the file's row order.
                var rng = new Random(launch * 1009 + r);
                var order = cells.OrderBy(_ => rng.Next()).ToList();
                foreach (var c in order)
                {
                    var t0 = DateTime.UtcNow;
                    long w0 = Clock.WallNs(), c0 = Clock.ProcessCpuNs();
                    await RunCell(c.C, pool, calls, c.k);
                    long c1 = Clock.ProcessCpuNs(), w1 = Clock.WallNs();
                    var t1 = DateTime.UtcNow;
                    samples.Add((new Sample
                    {
                        Suite = "rpc", Cell = c.C.Name, Payload = "P2.2", Dir = c.C.Dir, Transport = transport, Inflight = c.k,
                        Mode = c.C.Mode, Launch = launch, Round = r, CpuNs = c1 - c0, WallNs = w1 - w0, Iters = calls, Build = AbiVariant.Name,
                    }, t0, t1));
                }
            }
        }
        catch (Exception e)
        {
            // Requirement 18: one failed call aborts the run and produces no figure.
            Console.WriteLine("# ABORT: {0}: {1}", e.GetType().Name, e.Message);
            Console.WriteLine("# no samples written");
            return 1;
        }
        // The JIT tier read back per sample: compilations of measured code inside its window.
        { long seen; do { seen = Armonik.Ffi.Bdn.JitTiers.Received; Thread.Sleep(300); } while (seen != Armonik.Ffi.Bdn.JitTiers.Received); }
        var jits = Armonik.Ffi.Bdn.JitTiers.Events.ToArray();
        int quiet = 0;
        foreach (var (smp, t0, t1) in samples)
        {
            var inw = jits.Where(j => j.T >= t0 && j.T <= t1).ToList();
            if (inw.Count == 0) quiet++;
            smp.Extra = "\"jit_in_window\":{" + string.Join(",", inw.GroupBy(j => j.Tier).OrderBy(g => g.Key)
                .Select(g => "\"" + Armonik.Ffi.Bdn.JitTiers.TierName[g.Key] + "\":" + g.Count())) + "}";
            Console.WriteLine(smp.Json());
        }
        Console.WriteLine("# jit read back:  {0} of {1} samples compiled nothing of the measured code inside their window (sink {2})", quiet, samples.Count, _sink & 1);
        return 0;
    }

    /// CAMPAIGN req 19 as amended (R-H31): the crossings of ONE call of every cell B to E (and
    /// A, F, which make none), from the counting build (AK_HOST_COUNT: every generated import,
    /// codec and RPC, counts its own calls by name). Each cell's call is made once untimed,
    /// then counted once, on one thread. Written to --counts FILE.
    private static int CountCells(List<Cell> cells, string[] a)
    {
#if !AK_HOST_COUNT
        Console.Error.WriteLine("--counts needs the counting build (dotnet build -p:AkHostCount=true)");
        return 2;
#else
#if !AK_NO_UNKNOWN_FIELDS
        UnkHost.Exact = Environment.GetEnvironmentVariable("AK_COUNT_GROW") != "doubling";   // a gate control, as in CountRun
#endif
        var path = Opt(a, "--counts", "rpc-counts.txt");
        var o = new List<string>
        {
            "# CAMPAIGN req 19 (R-H31): one call per RPC cell and direction (" + AbiVariant.Name + " build, P2.2), counted by name in the host (AK_HOST_COUNT), after one untimed call on the cell's own channel.",
            "# fields: RPC cell dir mode | fwd N (every exported entry point called: codec and transport, resets included) rev N (core->host codec callbacks, grow excluded) grow N (ak_grow_fn calls, exact-size) reset N (of fwd: ak_dec_reset_<Root>, one before and one after each decode) | entry=count ...",
            "# the extras (.callback, .queue) are not counted here (labelled extra rows; not built in this run, so their queue drainer cannot enter a count); D's marshaller takes a core-ffi context from a pool, so no context is created inside a counted call",
        };
        foreach (var c in cells)
        {
            if (c.Name.Contains('.')) continue;
            void Call() { if (c.One != null) c.One(); else c.OneAsync().GetAwaiter().GetResult(); }
            Call();
            _ = Core;   // the counting thread's own context exists before the counters are zeroed
            Abi.EntryReset(); AkRpc.EntryReset(); Core.CallsReset();
#if !AK_NO_UNKNOWN_FIELDS
            UnkHost.Grows = 0;
            long Grows() => UnkHost.Grows;
#else
            long Grows() => 0;
#endif
            Call();
            var e = Abi.EntryCounts().Concat(AkRpc.EntryCounts()).OrderBy(x => x.Name, StringComparer.Ordinal).ToList();
            long fwd = e.Sum(x => x.Calls), resets = e.Where(x => x.Name.StartsWith("ak_dec_reset_", StringComparison.Ordinal)).Sum(x => x.Calls);
            if (resets != Core.ResetCalls) { Console.Error.WriteLine("reset tally mismatch on " + c.Name + " " + c.Dir); return 1; }
            o.Add(string.Format(CultureInfo.InvariantCulture, "RPC {0} {1} {2} | fwd {3} rev {4} grow {5} reset {6} | {7}", c.Name, c.Dir, c.Mode, fwd, Core.ReverseCalls, Grows(), resets,
                string.Join(" ", e.Select(x => x.Name + "=" + x.Calls))));
        }
        File.WriteAllLines(path, o);
        Console.WriteLine("rpc counts: {0} rows written to {1}", o.Count - 3, path);
        return 0;
#endif
    }

    /// The core client handle, for the blocking calls made through the generated imports.
    private static IntPtr CoreClient(CoreChannel c) => c.Client;
}

public sealed class CampaignService
{
    public static byte[] Wire;
    public static long Downs, Ups;
    public Task<byte[]> DownH(byte[] req, ServerCallContext ctx) { Interlocked.Increment(ref Downs); return Task.FromResult(Wire); }
    public Task<byte[]> UpH(byte[] req, ServerCallContext ctx)
    {
        Interlocked.Increment(ref Ups);
        // The server DECODES the request, with the incumbent, identically in every cell.
        if (req.Length != Wire.Length) throw new RpcException(new Status(StatusCode.InvalidArgument, "request length " + req.Length));
        Gp.ListTasksDetailedResponse.Parser.ParseFrom(req);
        return Task.FromResult(Array.Empty<byte>());
    }
}

public sealed class CampaignProvider : IServiceMethodProvider<CampaignService>
{
    private static readonly Marshaller<byte[]> Raw = Marshallers.Create<byte[]>(b => b, b => b);
    public void OnServiceMethodDiscovery(ServiceMethodProviderContext<CampaignService> ctx)
    {
        ctx.AddUnaryMethod<byte[], byte[]>(new Method<byte[], byte[]>(MethodType.Unary, CampaignMain.Svc, "Down", Raw, Raw), Array.Empty<object>(), (s, r, c) => s.DownH(r, c));
        ctx.AddUnaryMethod<byte[], byte[]>(new Method<byte[], byte[]>(MethodType.Unary, CampaignMain.Svc, "Up", Raw, Raw), Array.Empty<object>(), (s, r, c) => s.UpH(r, c));
    }
}

/// R-H2: a fixed set of caller threads, created once and reused by every sample, so no thread
/// is created inside a timed window. Run(one, n, k) splits n calls over the first k threads,
/// each making its calls back to back, and returns when all are done; the first exception is
/// rethrown (requirement 18 aborts the run on it).
internal sealed class CallerPool : IDisposable
{
    private readonly Thread[] _t;
    private readonly SemaphoreSlim[] _go;
    private readonly int[] _count;
    private readonly CountdownEvent _done = new CountdownEvent(1);
    private Action _work;
    private Exception _err;
    private volatile bool _stop;

    public CallerPool(int n)
    {
        _t = new Thread[n];
        _go = new SemaphoreSlim[n];
        _count = new int[n];
        for (int i = 0; i < n; i++)
        {
            _go[i] = new SemaphoreSlim(0);
            _t[i] = new Thread(Loop) { IsBackground = true, Name = "caller-" + i };
            _t[i].Start(i);
        }
    }

    private void Loop(object o)
    {
        int i = (int)o;
        while (true)
        {
            _go[i].Wait();
            if (_stop) return;
            try { var w = _work; for (int k = 0; k < _count[i]; k++) w(); }
            catch (Exception e) { Interlocked.CompareExchange(ref _err, e, null); }
            _done.Signal();
        }
    }

    public void Run(Action one, int n, int k)
    {
        if (k > _t.Length) throw new ArgumentOutOfRangeException(nameof(k));
        _work = one;
        _err = null;
        _done.Reset(k);
        for (int i = 0; i < k; i++) _count[i] = n / k + (i < n % k ? 1 : 0);
        for (int i = 0; i < k; i++) _go[i].Release();
        _done.Wait();
        if (_err != null) throw _err;
    }

    public void Dispose()
    {
        _stop = true;
        foreach (var g in _go) g.Release();
    }
}
