// The campaign runner's measuring half (design/CAMPAIGN.md, FIX-PLAN WP3). Driven by
// ../../run_campaign.sh, which pins each process (taskset to AK_CPU_CLIENT / AK_CPU_SERVER),
// runs the correctness gate first and writes the machine header. This file measures and
// writes one JSON object per sample (section 7); it summarises nothing and ranks nothing.
//
//   (the codec suite moved to ../BenchDotNet, BenchmarkDotNet; CAMPAIGN.md req 22a)
//   (the RPC server is the Rust slice's rpc_server, poc/rust/serve.sh; FIX-PLAN WP10)
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
using System.Net;
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
using Grpc.Core;
using Grpc.Net.Client;
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
            case "rpc": return await Rpc(a);
            default: Console.Error.WriteLine("campaign: --suite calib|rpc (the RPC server is the Rust slice's, poc/rust/serve.sh)"); return 2;
        }
    }

    /// The process-level part of requirement 27's header; the runner writes the machine.
    internal static void Header(string suite, string extra)
    {
        Console.WriteLine("# campaign suite {0}: raw samples, one JSON object per line; nothing summarised here", suite);
        Console.WriteLine("# runtime:        {0}", RuntimeInformation.FrameworkDescription);
        Console.WriteLine("# incumbent:      Google.Protobuf {0}; Grpc.Net.Client {1}", Ver(typeof(Google.Protobuf.MessageParser)), Ver(typeof(GrpcChannel)));
        Console.WriteLine("# server:         the Rust slice's tonic rpc_server, the campaign's one server for every slice (FIX-PLAN WP10, CAMPAIGN req 13 as amended; poc/rust/SERVER.md): service armonik.ffi.campaign.v1.Grid, P2.2 pre-serialised with prost, requests decoded with prost, receive limit 8 MiB; shipped socket = tonic's server defaults, pinned socket = 4 MiB stream and connection windows, adaptive off; its configuration and worker count are in its own log (rpc-server.log)");
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

    /// FIX-PLAN WP10: the campaign's one server, the Rust slice's tonic rpc_server
    /// (poc/rust/SERVER.md): service and method paths.
    public const string Svc = "armonik.ffi.campaign.v1.Grid";
    private static readonly Marshaller<byte[]> Raw = Marshallers.Create<byte[]>(b => b, b => b);
    private static readonly Method<byte[], byte[]> MDown = new Method<byte[], byte[]>(MethodType.Unary, Svc, "Fetch", Raw, Raw);
    private static readonly Method<byte[], byte[]> MUp = new Method<byte[], byte[]>(MethodType.Unary, Svc, "Push", Raw, Raw);

    internal static byte[] P22Wire() => BuildGp.P2_2().ToByteArray();

    private const int Window = 4 * 1024 * 1024;

    /// CAMPAIGN req 4 (R-H34): every log header states each stack's worker thread counts.
    internal static string ThreadLine(string extra)
    {
        ThreadPool.GetMinThreads(out int minW, out int minIo);
        ThreadPool.GetMaxThreads(out int maxW, out int maxIo);
        return string.Format(CultureInfo.InvariantCulture, "# threads:        .NET thread pool min {0} worker / {1} IO, max {2} worker / {3} IO, {4} pool thread(s) now; {5} logical CPUs visible; {6}",
            minW, minIo, maxW, maxIo, ThreadPool.ThreadCount, Environment.ProcessorCount, extra);
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
        new Method<byte[], T>(MethodType.Unary, Svc, "Fetch", Raw, Marshallers.Create<T>((m, c) => throw new NotSupportedException(), de));
    private static Method<T, byte[]> UpMethod<T>(Action<T, SerializationContext> se) =>
        new Method<T, byte[]>(MethodType.Unary, Svc, "Push", Marshallers.Create<T>(se, c => throw new NotSupportedException()), Raw);

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

    /// CAMPAIGN req 17 as amended (D10, 2026-10-01): the endpoint is `tcp:127.0.0.1:PORT` (the
    /// timed transport: TCP over loopback, Nagle off on every client socket) or a Unix socket
    /// path (history, and the gate's checks).
    internal static bool IsTcp(string ep) => ep.StartsWith("tcp:", StringComparison.Ordinal);
    internal static string CoreUri(string ep) => IsTcp(ep) ? "http://" + ep.Substring(4) : "unix:" + ep;
    internal static int TcpPort(string ep) => IsTcp(ep) ? int.Parse(ep.Substring(ep.LastIndexOf(':') + 1), CultureInfo.InvariantCulture) : -1;

    /// CAMPAIGN section 4.0 as amended (b58543f7b): transport `armonik` = ArmoniK's C# channel,
    /// built by packages/csharp's own GrpcChannelFactory.CreateChannel with its package defaults
    /// (GrpcClient: endpoint only; HttpClientHandler wrapped in its logging DelegatingHandler,
    /// retry ServiceConfig MaxAttempts 5, 1 s / 5 s / 1.5 on Unavailable, Aborted, Unknown,
    /// DisposeHttpClient, its ServicePoint settings). The core cells keep the core's client
    /// configuration of `shipped` (windows at the stack's defaults, adaptive off, Nagle off).
    internal static bool ArmonikCh;
    internal static GrpcChannel ArmonikChannel(string sock)
    {
        if (!IsTcp(sock)) throw new InvalidOperationException("transport armonik needs TCP");
        return ArmoniK.Api.Client.Submitter.GrpcChannelFactory.CreateChannel(new ArmoniK.Api.Client.Options.GrpcClient { Endpoint = "http://127.0.0.1:" + TcpPort(sock) });
    }

    private static GrpcChannel NewGrpc(string sock, bool pinned)
    {
        var handler = new SocketsHttpHandler { EnableMultipleHttp2Connections = false, PooledConnectionIdleTimeout = Timeout.InfiniteTimeSpan };
        if (pinned) handler.InitialHttp2StreamWindowSize = Window;
        bool tcp = IsTcp(sock);
        handler.ConnectCallback = async (c, ct) =>
        {
            if (tcp)
            {
                // D10: Nagle off on the client socket (read back on the live sockets, NoDelay.cs).
                var t = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp) { NoDelay = !PlantNagle };
                await t.ConnectAsync(new IPEndPoint(IPAddress.Loopback, TcpPort(sock)), ct);
                return new NetworkStream(t, true);
            }
            var s = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
            await s.ConnectAsync(new UnixDomainSocketEndPoint(sock), ct);
            return new NetworkStream(s, true);
        };
        return GrpcChannel.ForAddress(tcp ? "http://127.0.0.1:" + TcpPort(sock) : "http://localhost", new GrpcChannelOptions
        {
            HttpHandler = handler, MaxReceiveMessageSize = 64 << 20, MaxSendMessageSize = 64 << 20, Credentials = ChannelCredentials.Insecure,
        });
    }

    /// The core's transport: the same switch (requirement 17). shipped = the stack's windows
    /// with adaptive sizing off (what packages/csharp sets); pinned = 4 MiB and 4 MiB.
    internal static ak_client_opts CoreOpts(bool pinned) => new ak_client_opts
    {
        stream_window = pinned ? (uint)Window : 0, connection_window = pinned ? (uint)Window : 0,
        adaptive_window = 0, max_recv_message = 64u << 20, max_send_message = 64u << 20,
        tcp_nagle = PlantNagle ? 1 : 0,   // D10: Nagle off on every client socket, shipped and pinned alike
    };
    /// The control of the NODELAY readback: AK_CAMPAIGN_PLANT=nagle leaves Nagle ON in both client
    /// transports; the readback must fail every case (gen/gate.sh, run_campaign.sh --plant).
    internal static bool PlantNagle => Environment.GetEnvironmentVariable("AK_CAMPAIGN_PLANT") == "nagle";

    /// One cell of the grid in one direction: a blocking call (B, C, E: the core's blocking
    /// delivery) or an async one (A, D, F: Grpc.Net's idiomatic `await` of the call; the core's
    /// callback and queue extras).
    internal sealed class Cell
    {
        public string Name, Dir, Mode, Channel, Payload = "P2.2";
        public Action One;
        public Func<Task> OneAsync;
        /// Directions c and d (req 14 as amended): run at 1 and 8 in flight only, with this many
        /// times fewer calls per sample; `Check` is the pre-timing check (d: count and digest
        /// through StreamCheck; c: one accepted call).
        public int CallDiv = 1;
        public Action Check;
        public Func<Task> CheckAsync;
        public bool Upload => Dir == "c" || Dir == "d";
    }

    /// Every cell of this build, each on ITS OWN channel (req 13 as amended): Grpc.Net cells a
    /// GrpcChannel with its own handler and connection, core cells a CoreChannel on the one
    /// shared runtime `rt`. Directions: a (empty request, P2.2 response, decoded), a+read (the
    /// same, then every field read), b (P2.2 request, empty response).
    internal static List<Cell> BuildCells(string sock, bool pinned, IntPtr rt, int want, List<IDisposable> owned, List<string> chans, bool extras, Func<string, bool> keep = null)
    {
        keep ??= _ => true;   // only the cells kept get channels (one BDN unit = one cell, WP9)
        var g22 = BuildGp.P2_2();
        var f22 = BuildFacade.P2_2();
        var downPath = Encoding.UTF8.GetBytes("/" + Svc + "/Fetch");
        var upPath = Encoding.UTF8.GetBytes("/" + Svc + "/Push");
        byte[] gUpBytes = g22.ToByteArray();
        var cells = new List<Cell>();
        string ModeOf(string name) => name.EndsWith("-retain", StringComparison.Ordinal) ? "retain"
            : name.EndsWith("-nounk", StringComparison.Ordinal) ? "no-unknown"
            : name.StartsWith("C", StringComparison.Ordinal) || name.StartsWith("D", StringComparison.Ordinal) || name.StartsWith("E", StringComparison.Ordinal) || name.StartsWith("F", StringComparison.Ordinal) ? "drop" : "default";

        CoreChannel CoreCh(string name, bool queue = false)
        {
            var ch = new CoreChannel(rt, CoreUri(sock), CoreOpts(pinned));
            if (queue) ch.StartQueue();
            owned.Add(ch);
            chans.Add(name + ": its own core channel" + (queue ? " + completion queue drainer" : ""));
            return ch;
        }
        CallInvoker GrpcCh(string name)
        {
            var ch = ArmonikCh ? ArmonikChannel(sock) : NewGrpc(sock, pinned);
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
            int gs = -1;   // ABI v1 section 9: the gRPC status code (a non-OK one is AK_ERR_RPC_STATUS)
            fixed (byte* p = downPath) rc = AkRpc.ak_call_unary(ch.Client, p, (nuint)downPath.Length, null, 0, &r, &gs);
            try
            {
                if (rc != AkRpc.AK_OK) throw new Abort(cell + ": status " + rc + " grpc " + gs);
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
                // C, the MOVE path (ABI v1 section 9, WP8 parity): the encode stays in the core's
                // context and ak_call_unary_enc moves that buffer into the request, no copy.
                int er = Core.EncodeInto(f22, retain);
                if (er < 0) throw new Abort(cell + ": core encode " + er);
                ak_bytes r = default;
                int rc, gs = -1;
                fixed (byte* p = upPath) rc = AkRpc.ak_call_unary_enc(ch.Client, p, (nuint)upPath.Length, Core.EncContext, &r, &gs);
                try { if (rc != AkRpc.AK_OK) throw new Abort(cell + " up: status " + rc + " grpc " + gs); CheckLen((int)r.len, 0, cell + " up"); }
                finally { AkRpc.ak_bytes_free(&r); }
                return;
            }
            else if (codec == 3)
            {
                // Cc, the COPY path (a labelled extra): the core's buffer taken, then copied into
                // the request by ak_call_unary.
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
                int gs = -1;
                fixed (byte* p = upPath) rc = AkRpc.ak_call_unary(ch.Client, p, (nuint)upPath.Length, body, (nuint)len, &r, &gs);
                try { if (rc != AkRpc.AK_OK) throw new Abort(cell + " up: status " + rc + " grpc " + gs); CheckLen((int)r.len, 0, cell + " up"); }
                finally { AkRpc.ak_bytes_free(&r); }
            }
        }
        void AddCore(string name, int codec, bool retain)
        {
            if (!keep(name)) return;
            var ch = CoreCh(name);
            cells.Add(new Cell { Name = name, Dir = "a", Mode = ModeOf(name), One = () => CoreDown(ch, name, codec, retain, false) });
            cells.Add(new Cell { Name = name, Dir = "a+read", Mode = ModeOf(name), One = () => CoreDown(ch, name, codec, retain, true) });
            cells.Add(new Cell { Name = name, Dir = "b", Mode = ModeOf(name), One = () => CoreUp(ch, name, codec, retain) });
        }
        // Direction b only: the framed twins (Bf, Cf-*, Ef-*: the core's second send path beside
        // its reference, req 14) and C's copy path (Cc-*); a has an empty request, so its send path
        // is the same on either.
        void AddCoreB(string name, int codec, bool retain, bool framed)
        {
            if (!keep(name)) return;
            var ch = CoreCh(name + " (b)");
            if (framed && AkRpc.ak_client_set_framed(ch.Client, 1) != AkRpc.AK_OK) throw new InvalidOperationException("ak_client_set_framed");
            cells.Add(new Cell { Name = name, Dir = "b", Mode = ModeOf(name), Channel = name + " (b)", One = () => CoreUp(ch, name, codec, retain) });
        }

        // --- A, D, F: Grpc.Net, the idiomatic `await` of an AsyncUnaryCall (requirement 16 as
        //     amended, R-H30: async is what .NET production code does), with the marshaller
        //     swapped: A = Grpc.Tools' generated shape; D = core-ffi; F = host-gen. The request
        //     serializers are the codec suite's (Ops_*.Ser*), so its encode-transport rows time
        //     this very code.
        void AddGrpc<T>(string name, Method<byte[], T> down, Func<T, long> touch, Method<T, byte[]> up, T req) where T : class
        {
            if (!keep(name)) return;
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
        AddCoreB("Bf", 0, false, true);
#if AK_NO_UNKNOWN_FIELDS
        // WP5 step 10, req 12: the NO-UNKNOWN client (unknown fields compiled out of the core,
        // the binding and host-gen): C, D, E and F in mode no-unknown; A and B as controls.
        AddCore("C-nounk", 1, false);
        AddCoreB("Cf-nounk", 1, false, true);
        AddCoreB("Cc-nounk", 3, false, false);
        AddCoreB("Ef-nounk", 2, false, true);
        AddGrpc("D-nounk", DDown(false), tf, DUp(false), f22);
        AddCore("E-nounk", 2, false);
        AddGrpc("F-nounk", FDown(false), tf, FUp(false), f22);
#else
        AddCore("C-retain", 1, true);
        AddCore("C-drop", 1, false);
        foreach (var r in new[] { true, false })
        {
            var m = r ? "-retain" : "-drop";
            AddCoreB("Cf" + m, 1, r, true);
            AddCoreB("Cc" + m, 3, r, false);
            AddCoreB("Ef" + m, 2, r, true);
        }
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
            if (!keep(name)) continue;
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
        AddUploadCells(cells, (n, q) => CoreCh(n, q), GrpcCh, ModeOf, keep);
        foreach (var c in cells) c.Channel ??= c.Name;
        GC.KeepAlive(gUpBytes);
        return cells;
    }

    // ====================================================== directions c and d (Upload.cs)

    private static UpData[] _ups;
    /// req 18's plants for the upload directions: AK_CAMPAIGN_PLANT=len with
    /// AK_CAMPAIGN_PLANT_DIR=c expects a 1-byte response, =d one data byte more than sent;
    /// AK_CAMPAIGN_PLANT=digest expects a wrong SHA-256 in the pre-timing check.
    private static string PlantDir => Environment.GetEnvironmentVariable("AK_CAMPAIGN_PLANT") == "len" ? (Environment.GetEnvironmentVariable("AK_CAMPAIGN_PLANT_DIR") ?? "a") : "";
    private static byte[] PlantSha(UpData u)
    {
        if (Environment.GetEnvironmentVariable("AK_CAMPAIGN_PLANT") != "digest") return null;
        var s = (byte[])u.Sha.Clone(); s[0] ^= 1; return s;
    }

    private static void AddUploadCells(List<Cell> cells, Func<string, bool, CoreChannel> coreCh, Func<string, CallInvoker> grpcCh, Func<string, string> modeOf, Func<string, bool> keep)
    {
        if (_ups == null)
        {
            _ups = new[] { Uploads.Unary("P5.3"), Uploads.Unary("P5.4"), Uploads.Streamed(4), Uploads.Streamed(16) };
            foreach (var u in _ups) Uploads.CheckWire(u);   // byte identity of every message, before any call
        }
        var upload = Encoding.UTF8.GetBytes("/" + Svc + "/Upload");
        var stream = Encoding.UTF8.GetBytes("/" + Svc + "/UploadStream");
        var streamCheck = Encoding.UTF8.GetBytes("/" + Svc + "/UploadStreamCheck");
        int cWant = PlantDir == "c" ? 1 : 0;
        long dPlant = PlantDir == "d" ? 1 : 0;
        const int CDiv = 4, DDiv = 8;

        // B, C, E and their framed twins Bf, Cf, Ef: the core's transport.
        void Core(string name, int codec, bool retain, bool framed)
        {
            if (!keep(name)) return;
            var ch = coreCh(name + " (c, d)", false);
            if (framed && AkRpc.ak_client_set_framed(ch.Client, 1) != AkRpc.AK_OK) throw new InvalidOperationException("ak_client_set_framed");
            string mode = modeOf(name.Replace("f-", "-").Replace("Bf", "B"));
            foreach (var u in _ups)
            {
                var uu = u;
                if (!u.Stream)
                    cells.Add(new Cell { Name = name, Dir = "c", Payload = u.Payload, Mode = mode, CallDiv = CDiv, Channel = name,
                        One = () => Uploads.CoreUnary(ch, upload, codec, retain, uu, cWant, name),
                        Check = () => Uploads.CoreUnary(ch, upload, codec, retain, uu, cWant, name) });
                else
                    cells.Add(new Cell { Name = name, Dir = "d", Payload = u.Payload, Mode = mode, CallDiv = DDiv, Channel = name,
                        One = () => Uploads.CheckStreamResponse(Uploads.CoreStream(ch, stream, codec, retain, uu, name), uu, false, dPlant, null, name),
                        Check = () => Uploads.CheckStreamResponse(Uploads.CoreStream(ch, streamCheck, codec, retain, uu, name), uu, true, dPlant, PlantSha(uu), name) });
            }
        }
        // A, D, F: Grpc.Net, AsyncUnaryCall and AsyncClientStreamingCall.
        void Grpc<T>(string name, Marshaller<T> mm, Func<UpData, T[]> msgs) where T : class
        {
            if (!keep(name)) return;
            var inv = grpcCh(name + " (c, d)");
            var mc = new Method<T, byte[]>(MethodType.Unary, Svc, "Upload", mm, Raw);
            var md = new Method<T, byte[]>(MethodType.ClientStreaming, Svc, "UploadStream", mm, Raw);
            var mk = new Method<T, byte[]>(MethodType.ClientStreaming, Svc, "UploadStreamCheck", mm, Raw);
            foreach (var u in _ups)
            {
                var uu = u;
                var ms = msgs(u);
                if (!u.Stream)
                    cells.Add(new Cell { Name = name, Dir = "c", Payload = u.Payload, Mode = modeOf(name), CallDiv = CDiv, Channel = name + " (c, d)",
                        OneAsync = () => Uploads.GrpcUnary(inv, mc, ms[0], cWant, name),
                        CheckAsync = () => Uploads.GrpcUnary(inv, mc, ms[0], cWant, name) });
                else
                    cells.Add(new Cell { Name = name, Dir = "d", Payload = u.Payload, Mode = modeOf(name), CallDiv = DDiv, Channel = name + " (c, d)",
                        OneAsync = async () => Uploads.CheckStreamResponse(await Uploads.GrpcStream(inv, md, ms), uu, false, dPlant, null, name),
                        CheckAsync = async () => Uploads.CheckStreamResponse(await Uploads.GrpcStream(inv, mk, ms), uu, true, dPlant, PlantSha(uu), name) });
            }
        }
        Grpc("A", Uploads.MInc, u => u.G);
        Core("B", 0, false, false);
        Core("Bf", 0, false, true);
#if AK_NO_UNKNOWN_FIELDS
        foreach (var (c, e) in new[] { ("C-nounk", 1), ("E-nounk", 2) }) { Core(c, e, false, false); Core(c.Replace("-", "f-"), e, false, true); }
        Core("Cc-nounk", 3, false, false);
        Grpc("D-nounk", Uploads.MFfi(false), u => u.F);
        Grpc("F-nounk", Uploads.MHost(false), u => u.F);
#else
        foreach (var r in new[] { true, false })
        {
            string m = r ? "-retain" : "-drop";
            Core("C" + m, 1, r, false); Core("Cf" + m, 1, r, true); Core("Cc" + m, 3, r, false);
            Core("E" + m, 2, r, false); Core("Ef" + m, 2, r, true);
            Grpc("D" + m, Uploads.MFfi(r), u => u.F);
            Grpc("F" + m, Uploads.MHost(r), u => u.F);
        }
#endif
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
    internal static async Task RunCell(Cell c, CallerPool pool, int n, int k)
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

    private static async Task<int> Rpc(string[] a)
    {
        Armonik.Ffi.Bdn.JitTiers.Start();   // the JIT tier read back (R-H2, req 24)
        // WP5 step 10: the client's binding variant must match the core it loaded.
        var vwhy = AbiVariant.CheckLoadedCore();
        if (vwhy != null) { Console.WriteLine("# ABORT: core variant mismatch: " + vwhy); Console.WriteLine("# no samples written"); return 1; }
        var sock = Opt(a, "--sock", null);
        _ep = sock;
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
            // The --plant controls (req 18) run one send path at a time: AK_CAMPAIGN_ONLY=cell,...
            var only = Environment.GetEnvironmentVariable("AK_CAMPAIGN_ONLY");
            if (!string.IsNullOrEmpty(only)) { var keep = new HashSet<string>(only.Split(','), StringComparer.Ordinal); cells = cells.Where(c => keep.Contains(c.Name)).ToList(); }
            if (counts) return CountCells(cells, a);
            if (a.Contains("--upload-check")) return await UploadCheck(cells);
            // WP9: the timed grid is `akrpc bench` (BenchmarkDotNet, RpcBench.cs); the hand-written
            // sampler that was here is removed.
            Console.Error.WriteLine("campaign --suite rpc takes --counts or --upload-check; the timed grid is `akrpc bench`");
            return 2;
        }
        finally
        {
            foreach (var d in owned) d.Dispose();
            AkRpc.ak_runtime_destroy(rt);
        }
    }

    /// The gate's upload check (req 18/26, WP8): every c cell once, every d cell once through
    /// StreamCheck (count and SHA-256), both send paths of the core's transport; nothing timed.
    private static string _ep;
    private static async Task<int> UploadCheck(List<Cell> cells)
    {
        int n = 0;
        try
        {
            foreach (var c in cells.Where(x => x.Upload))
            {
                if (c.Check != null) c.Check(); else await c.CheckAsync();
                Console.WriteLine("  ok  {0,-10} {1} {2,-13} {3}", c.Name, c.Dir, c.Payload, c.Mode);
                n++;
            }
        }
        catch (Exception e) { Console.WriteLine("# ABORT: {0}: {1}", e.GetType().Name, e.Message); return 1; }
        if (IsTcp(_ep))
        {
            // D10: Nagle off read back on every live socket of this process to the server.
            var (found, nd) = NoDelay.Check(TcpPort(_ep));
            Console.WriteLine("TCP_NODELAY read back: {0} of {1} sockets to the server", nd, found);
            if (found == 0 || nd != found) return 1;
        }
        Console.WriteLine("upload check ({0} build): {1} upload cells, every c call accepted, every d stream's count and SHA-256 as received equal to the client's", AbiVariant.Name, n);
        return n > 0 ? 0 : 1;
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
        UnkHost.Exact = Environment.GetEnvironmentVariable("AK_COUNT_GROW") == "exact";   // a gate control only, as in CountRun
#endif
        var path = Opt(a, "--counts", "rpc-counts.txt");
        var o = new List<string>
        {
            "# CAMPAIGN req 19 (R-H31): one call per RPC cell and direction (" + AbiVariant.Name + " build, P2.2), counted by name in the host (AK_HOST_COUNT), after one untimed call on the cell's own channel.",
            "# fields: RPC cell dir payload mode | fwd N (every exported entry point called: codec and transport, resets included) rev N (core->host codec callbacks, grow excluded) grow N (ak_grow_fn calls, geometric grow as timed) reset N (of fwd: ak_dec_reset_<Root>, one per decode, before it) | entry=count ...",
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
            o.Add(string.Format(CultureInfo.InvariantCulture, "RPC {0} {1} {8} {2} | fwd {3} rev {4} grow {5} reset {6} | {7}", c.Name, c.Dir, c.Mode, fwd, Core.ReverseCalls, Grows(), resets,
                string.Join(" ", e.Select(x => x.Name + "=" + x.Calls)), c.Payload));
        }
        File.WriteAllLines(path, o);
        Console.WriteLine("rpc counts: {0} rows written to {1}", o.Count - 3, path);
        return 0;
#endif
    }

    /// The core client handle, for the blocking calls made through the generated imports.
    private static IntPtr CoreClient(CoreChannel c) => c.Client;
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
