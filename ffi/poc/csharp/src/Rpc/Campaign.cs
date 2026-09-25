// The campaign runner's measuring half (design/CAMPAIGN.md, FIX-PLAN WP3). Driven by
// ../../run_campaign.sh, which pins each process (taskset to AK_CPU_CLIENT / AK_CPU_SERVER),
// runs the correctness gate first and writes the machine header. This file measures and
// writes one JSON object per sample (section 7); it summarises nothing and ranks nothing.
//
//   (the codec suite moved to ../BenchDotNet, BenchmarkDotNet; CAMPAIGN.md req 22a)
//   akrpc campaign --suite rpc-server --sock PATH --transport shipped|pinned
//   akrpc campaign --suite rpc        --sock PATH --transport shipped|pinned --launch N
//                  --rounds R [--calls C] [--inflight 1,8,16]
//   akrpc campaign --suite calib      --launch N --rounds R [--iters N]
//
// Clocks (requirement 21): the calib suite reads CLOCK_THREAD_CPUTIME_ID of the
// one measuring thread; the rpc suite reads getrusage(RUSAGE_SELF) of the client process
// (the server is another process) and wall time beside it. Process.TotalProcessorTime is not
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
    public string Suite, Arm, Cell, Payload, Content, Dir, Mode, Transport;
    public int Inflight = -1, Launch, Round;
    public long CpuNs, WallNs, Iters;

    public string Json()
    {
        var sb = new StringBuilder("{\"slice\":\"csharp\"");
        void S(string k, string v) { if (v != null) sb.Append(",\"").Append(k).Append("\":\"").Append(v).Append('"'); }
        void N(string k, long v) { sb.Append(",\"").Append(k).Append("\":").Append(v.ToString(CultureInfo.InvariantCulture)); }
        S("suite", Suite); S("arm", Arm); S("cell", Cell); S("payload", Payload); S("content", Content);
        S("dir", Dir); S("unknown_mode", Mode); S("transport", Transport);
        if (Inflight >= 0) N("inflight", Inflight);
        N("launch", Launch); N("round", Round); N("cpu_ns", CpuNs); N("wall_ns", WallNs); N("iters", Iters);
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
            default: Console.Error.WriteLine("campaign: --suite calib|rpc|rpc-server"); return 2;
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

    /// The server: a separate process (requirement 13), Kestrel over a Unix socket, returning
    /// PRE-SERIALISED P2.2 bytes for direction (a) and decoding the request with the incumbent
    /// for direction (b) -- identical work in every cell.
    private static async Task<int> Server(string[] a)
    {
        var sock = Opt(a, "--sock", null);
        bool pinned = Opt(a, "--transport", "shipped") == "pinned";
        CampaignService.Wire = P22Wire();
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
        Console.WriteLine("# campaign rpc-server listening on {0} ({1}); pid {2}", sock, pinned ? "pinned" : "shipped", Environment.ProcessId);
        await app.WaitForShutdownAsync();
        return 0;
    }

    private sealed class Abort : Exception { public Abort(string m) : base(m) { } }

    private static byte[] _wire;
    [ThreadStatic] private static CoreFfi_ListTasksDetailedResponse _core;
    [ThreadStatic] private static byte[] _buf;
    private static CoreFfi_ListTasksDetailedResponse Core => _core ??= new CoreFfi_ListTasksDetailedResponse();
    private static byte[] Buf(int n) => (_buf == null || _buf.Length < n) ? (_buf = new byte[Math.Max(n, 1 << 20)]) : _buf;

    private static byte[] Flatten(ReadOnlySequence<byte> s, out int n)
    {
        n = checked((int)s.Length);
        var b = Buf(n);
        s.CopyTo(b);
        return b;
    }

    private static unsafe Method<byte[], T> DownMethod<T>(Func<DeserializationContext, T> de) =>
        new Method<byte[], T>(MethodType.Unary, Svc, "Down", Raw, Marshallers.Create<T>((m, c) => throw new NotSupportedException(), de));
    private static unsafe Method<T, byte[]> UpMethod<T>(Action<T, SerializationContext> se) =>
        new Method<T, byte[]>(MethodType.Unary, Svc, "Up", Marshallers.Create<T>(se, c => throw new NotSupportedException()), Raw);

    private static void CheckLen(int got, int want, string cell)
    {
        if (got != want) throw new Abort(cell + ": response length " + got + ", expected " + want);
    }

    private static async Task<int> Rpc(string[] a)
    {
        var sock = Opt(a, "--sock", null);
        var transport = Opt(a, "--transport", "shipped");
        bool pinned = transport == "pinned";
        int launch = OptI(a, "--launch", 1), rounds = OptI(a, "--rounds", 5), calls = OptI(a, "--calls", 64);
        var levels = Opt(a, "--inflight", "1,8,16").Split(',').Select(x => int.Parse(x, CultureInfo.InvariantCulture)).ToArray();
        // packages/csharp's GrpcChannelProvider, on the Unix-socket path: dynamic window
        // sizing OFF and no window set. That is "shipped"; "pinned" adds the 4 MiB windows.
        AppContext.SetSwitch("System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing", true);
        _wire = P22Wire();
        int want = _wire.Length;
        // A CONTROL (run_campaign.sh --suite rpc --plant): a wrong expected length must abort
        // the run with no sample written (requirement 18).
        if (Environment.GetEnvironmentVariable("AK_CAMPAIGN_PLANT") == "len") want += 1;
        var g22 = BuildGp.P2_2();
        var f22 = BuildFacade.P2_2();

        var handler = new SocketsHttpHandler { EnableMultipleHttp2Connections = false, PooledConnectionIdleTimeout = Timeout.InfiniteTimeSpan };
        if (pinned) handler.InitialHttp2StreamWindowSize = Window;
        handler.ConnectCallback = async (c, ct) =>
        {
            var s = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
            await s.ConnectAsync(new UnixDomainSocketEndPoint(sock), ct);
            return new NetworkStream(s, true);
        };
        using var ch = GrpcChannel.ForAddress("http://localhost", new GrpcChannelOptions
        {
            HttpHandler = handler, MaxReceiveMessageSize = 64 << 20, MaxSendMessageSize = 64 << 20, Credentials = ChannelCredentials.Insecure,
        });
        var inv = ch.CreateCallInvoker();
        // The core's transport: the same switch (requirement 17). shipped = the stack's windows
        // with adaptive sizing off (what packages/csharp sets); pinned = 4 MiB and 4 MiB.
        var opts = new ak_client_opts
        {
            stream_window = pinned ? (uint)Window : 0, connection_window = pinned ? (uint)Window : 0,
            adaptive_window = 0, max_recv_message = 64u << 20, max_send_message = 64u << 20, tcp_nagle = pinned ? 0 : -1,
        };
        using var core = new CoreChannel("unix:" + sock, 2, opts);
        core.StartQueue();
        IntPtr cl = CoreClient(core);

        // grpc-dotnet marshallers. A: Grpc.Tools' production shape; D: core-ffi.
        var aDown = DownMethod(c => { CheckLen(c.PayloadLength, want, "A"); return Gp.ListTasksDetailedResponse.Parser.ParseFrom(c.PayloadAsReadOnlySequence()); });
        var aUp = UpMethod<Gp.ListTasksDetailedResponse>((m, c) => { c.SetPayloadLength(m.CalculateSize()); m.WriteTo(c.GetBufferWriter()); c.Complete(); });
        var dDown = DownMethod(c =>
        {
            CheckLen(c.PayloadLength, want, "D");
            var b = Flatten(c.PayloadAsReadOnlySequence(), out int n);
            int rc = Core.TryDecode(b, n, false, out var m);
            if (rc < 0) throw new Abort("D: core decode " + rc);
            return m;
        });
        var dUp = UpMethod<ListTasksDetailedResponse>((m, c) =>
        {
            unsafe
            {
                int rc = Core.TryEncode(m, false, out byte* p, out int n);
                if (rc < 0) throw new Abort("D: core encode " + rc);
                c.SetPayloadLength(n);
                var w = c.GetBufferWriter();
                new ReadOnlySpan<byte>(p, n).CopyTo(w.GetSpan(n));
                w.Advance(n);
                c.Complete();
            }
        });
        var downPath = Encoding.UTF8.GetBytes("/" + Svc + "/Down");
        var upPath = Encoding.UTF8.GetBytes("/" + Svc + "/Up");
        byte[] gUpBytes = g22.ToByteArray();

        async Task AsyncOp(Func<Task> one, int n, int inflight)
        {
            var ts = new Task[inflight];
            for (int t = 0; t < inflight; t++)
            {
                int mine = n / inflight + (t < n % inflight ? 1 : 0);
                ts[t] = Task.Run(async () => { for (int i = 0; i < mine; i++) await one(); });
            }
            await Task.WhenAll(ts);
        }
        Task BlockingOp(Action one, int n, int inflight)
        {
            var th = new Thread[inflight];
            Exception err = null;
            for (int t = 0; t < inflight; t++)
            {
                int mine = n / inflight + (t < n % inflight ? 1 : 0);
                th[t] = new Thread(() => { try { for (int i = 0; i < mine; i++) one(); } catch (Exception e) { err = e; } });
                th[t].Start();
            }
            foreach (var t in th) t.Join();
            if (err != null) throw err;
            return Task.CompletedTask;
        }

        // B/C: the core's BLOCKING delivery (requirement 16).
        unsafe void BDown()
        {
            ak_bytes r = default;
            int rc;
            fixed (byte* p = downPath) rc = AkRpc.ak_call_unary(cl, p, (nuint)downPath.Length, null, 0, &r);
            try
            {
                if (rc != AkRpc.AK_OK) throw new Abort("B: status " + rc);
                CheckLen((int)r.len, want, "B");
                Gp.ListTasksDetailedResponse.Parser.ParseFrom(new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len));
            }
            finally { AkRpc.ak_bytes_free(&r); }
        }
        unsafe void CDown()
        {
            ak_bytes r = default;
            int rc;
            fixed (byte* p = downPath) rc = AkRpc.ak_call_unary(cl, p, (nuint)downPath.Length, null, 0, &r);
            try
            {
                if (rc != AkRpc.AK_OK) throw new Abort("C: status " + rc);
                CheckLen((int)r.len, want, "C");
                // TryDecode takes a managed buffer; the response is copied once (charged to C).
                var b = Buf((int)r.len);
                new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len).CopyTo(b);
                int dr = Core.TryDecode(b, (int)r.len, false, out _);
                if (dr < 0) throw new Abort("C: core decode " + dr);
            }
            finally { AkRpc.ak_bytes_free(&r); }
        }
        unsafe void BUp()
        {
            int n = g22.CalculateSize();
            var b = Buf(n);
            g22.WriteTo(new Span<byte>(b, 0, n));
            ak_bytes r = default;
            int rc;
            fixed (byte* p = upPath) fixed (byte* q = b) rc = AkRpc.ak_call_unary(cl, p, (nuint)upPath.Length, q, (nuint)n, &r);
            try { if (rc != AkRpc.AK_OK) throw new Abort("B up: status " + rc); CheckLen((int)r.len, 0, "B up"); }
            finally { AkRpc.ak_bytes_free(&r); }
        }
        unsafe void CUp()
        {
            int er = Core.TryEncode(f22, false, out byte* q, out int n);
            if (er < 0) throw new Abort("C up: core encode " + er);
            ak_bytes r = default;
            int rc;
            fixed (byte* p = upPath) rc = AkRpc.ak_call_unary(cl, p, (nuint)upPath.Length, q, (nuint)n, &r);
            try { if (rc != AkRpc.AK_OK) throw new Abort("C up: status " + rc); CheckLen((int)r.len, 0, "C up"); }
            finally { AkRpc.ak_bytes_free(&r); }
        }
        async Task Grpc<TReq, TRes>(Method<TReq, TRes> m, TReq req) where TReq : class where TRes : class
        {
            using var c = inv.AsyncUnaryCall(m, null, new CallOptions(), req);
            await c.ResponseAsync;
            if (c.GetStatus().StatusCode != StatusCode.OK) throw new Abort(m.FullName + ": status " + c.GetStatus());
        }
        async Task UpRaw(Method<byte[], byte[]> m, byte[] req)
        {
            using var c = inv.AsyncUnaryCall(m, null, new CallOptions(), req);
            var r = await c.ResponseAsync;
            CheckLen(r.Length, 0, "up");
        }
        // The core's callback and queue deliveries: labelled extra rows (requirement 16).
        async Task CoreAsync(bool queue, byte[] path, byte[] req, int wantLen, bool decodeCore)
        {
            var r = queue ? await core.CallQAsync(path, req) : await core.CallCbAsync(path, req);
            try
            {
                CheckLen((int)r.len, wantLen, queue ? "queue" : "callback");
                if (wantLen > 0)
                {
                    unsafe
                    {
                        if (decodeCore)
                        {
                            var b = Buf((int)r.len);
                            new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len).CopyTo(b);
                            if (Core.TryDecode(b, (int)r.len, false, out _) < 0) throw new Abort("core decode");
                        }
                        else Gp.ListTasksDetailedResponse.Parser.ParseFrom(new ReadOnlySpan<byte>((void*)r.ptr, (int)r.len));
                    }
                }
            }
            finally { CoreChannel.Release(ref r); }
        }
        byte[] CoreUpBytes() => Core.EncodeToArray(f22);

        var ops = new List<(string Cell, string Dir, Func<int, int, Task> Run)>
        {
            ("A", "a", (n, k) => AsyncOp(() => Grpc(aDown, Array.Empty<byte>()), n, k)),
            ("B", "a", (n, k) => BlockingOp(BDown, n, k)),
            ("C", "a", (n, k) => BlockingOp(CDown, n, k)),
            ("D", "a", (n, k) => AsyncOp(() => Grpc(dDown, Array.Empty<byte>()), n, k)),
            ("A", "b", (n, k) => AsyncOp(() => Grpc(aUp, g22), n, k)),
            ("B", "b", (n, k) => BlockingOp(BUp, n, k)),
            ("C", "b", (n, k) => BlockingOp(CUp, n, k)),
            ("D", "b", (n, k) => AsyncOp(() => Grpc(dUp, f22), n, k)),
            ("B.callback", "a", (n, k) => AsyncOp(() => CoreAsync(false, downPath, Array.Empty<byte>(), want, false), n, k)),
            ("B.queue", "a", (n, k) => AsyncOp(() => CoreAsync(true, downPath, Array.Empty<byte>(), want, false), n, k)),
            ("C.callback", "a", (n, k) => AsyncOp(() => CoreAsync(false, downPath, Array.Empty<byte>(), want, true), n, k)),
            ("C.queue", "a", (n, k) => AsyncOp(() => CoreAsync(true, downPath, Array.Empty<byte>(), want, true), n, k)),
            ("B.callback", "b", (n, k) => AsyncOp(() => CoreAsync(false, upPath, gUpBytes, 0, false), n, k)),
            ("B.queue", "b", (n, k) => AsyncOp(() => CoreAsync(true, upPath, gUpBytes, 0, false), n, k)),
            ("C.callback", "b", (n, k) => AsyncOp(() => CoreAsync(false, upPath, CoreUpBytes(), 0, true), n, k)),
            ("C.queue", "b", (n, k) => AsyncOp(() => CoreAsync(true, upPath, CoreUpBytes(), 0, true), n, k)),
        };
        var cells = (from o in ops from k in levels select (o.Cell, o.Dir, k, o.Run)).ToList();
        Header("rpc", string.Format(CultureInfo.InvariantCulture,
            "launch {0}, rounds {1}, {2} calls per sample, in flight {3}; transport {4} (client: DisableDynamicWindowSizing{5}; Kestrel {6}; core: ak_client_opts stream {7} connection {8} adaptive 0 nagle {9}); Unix socket {10}; the server is a separate process; B/C blocking delivery, .callback/.queue extra rows; direction a: empty request, P2.2 response ({11} B); b: P2.2 request decoded by the server, empty response; warm-up {12} calls per cell; every call checked (status and length)",
            launch, rounds, calls, string.Join("/", levels), transport, pinned ? " + InitialHttp2StreamWindowSize 4 MiB" : ", no window set",
            pinned ? "stream/connection window 4 MiB" : "defaults", opts.stream_window, opts.connection_window, opts.tcp_nagle, sock, want, 2 * levels.Max()));

        var lines = new List<string>();
        try
        {
            foreach (var c in cells) await c.Run(2 * levels.Max(), c.k);          // warm-up
            for (int r = 1; r <= rounds; r++)
            {
                GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
                int start = (int)((long)(r - 1) * cells.Count / rounds);
                for (int j = 0; j < cells.Count; j++)
                {
                    var c = cells[(start + j) % cells.Count];
                    long w0 = Clock.WallNs(), c0 = Clock.ProcessCpuNs();
                    await c.Run(calls, c.k);
                    long c1 = Clock.ProcessCpuNs(), w1 = Clock.WallNs();
                    lines.Add(new Sample
                    {
                        Suite = "rpc", Cell = c.Cell, Payload = "P2.2", Dir = c.Dir, Transport = transport, Inflight = c.k,
                        Launch = launch, Round = r, CpuNs = c1 - c0, WallNs = w1 - w0, Iters = calls,
                    }.Json());
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
        foreach (var l in lines) Console.WriteLine(l);
        return 0;
    }

    /// The core client handle, for the blocking calls made through the generated imports.
    private static IntPtr CoreClient(CoreChannel c) => c.Client;
}

public sealed class CampaignService
{
    public static byte[] Wire;
    public Task<byte[]> DownH(byte[] req, ServerCallContext ctx) => Task.FromResult(Wire);
    public Task<byte[]> UpH(byte[] req, ServerCallContext ctx)
    {
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
