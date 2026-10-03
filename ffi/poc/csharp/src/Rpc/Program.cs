// akrpc: the csharp slice's RPC client process (design/CAMPAIGN.md section 4.2).
//
//   akrpc bench ...                     the timed RPC grid on BenchmarkDotNet (RpcBench.cs, WP9)
//   akrpc campaign --suite calib|rpc    the crossing benchmark (calib); the counting run and the
//                                       gate's upload check (rpc --counts / --upload-check)
//   akrpc --layout PROBE.json           plan.rpc's structs against the Rust declaration (the gate)
//   akrpc --error-path --sock PATH      R-D9: every delivery frees a failed call's bytes (the gate)
//   akrpc --shared-ctx [--per-thread]   the concurrency contract's control
//
// FIX-PLAN WP10: the RPC SERVER is the Rust slice's tonic rpc_server for every slice
// (poc/rust/SERVER.md, started by poc/rust/serve.sh); this project has no server of its own
// any more. Its earlier in-process Kestrel server and the pre-campaign timing modes (--grid,
// --stream, Bench.cs; D42) are removed with it.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;

namespace Armonik.Ffi.Rpc;

public static class Program
{
    public static async Task<int> Main(string[] argv)
    {
        Armonik.Ffi.Bdn.Alloc.Probe();   // req 25 / D9: the allocator readback, before the runtime frees any large block
        // The concurrency contract's POSITIVE CONTROL, and it runs in its own
        // process on purpose: the rust slice found that a shared encode context
        // ABORTS rather than returning an error, and an abort takes the whole
        // harness with it. Run it as a child and read the exit status.
        if (argv.Contains("--shared-ctx")) return SharedCtx(argv);
        // design/CAMPAIGN.md: the campaign runner's measuring process (run_campaign.sh).
        if (argv.Length > 0 && argv[0] == "campaign") return await Armonik.Ffi.Campaign.CampaignMain.Run(argv.Skip(1).ToArray());
        // FIX-PLAN WP9: the RPC grid on BenchmarkDotNet (RpcBench.cs), one process per cell.
        if (argv.Length > 0 && argv[0] == "bench") return Armonik.Ffi.Campaign.RpcBenchMain.Run(argv.Skip(1).ToArray());
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
        // R-D9's gate: every delivery on a FAILED call must still reach `ak_bytes_free` before
        // it throws. Counted, not inferred: with the counting core, a failed call's forward
        // count includes the free, so a binding that skips it reads one short.
        if (argv.Contains("--error-path"))
        {
            int si = Array.IndexOf(argv, "--sock");
            if (si < 0 || si + 1 >= argv.Length) { Console.Error.WriteLine("--error-path needs --sock PATH (the campaign server, poc/rust/serve.sh)"); return 2; }
            return await ErrorPath(argv[si + 1], Arg(argv, "--calls", 50));
        }
        Console.Error.WriteLine("akrpc: bench | campaign --suite calib|rpc | --layout PROBE | --error-path --sock PATH | --shared-ctx");
        return 2;
    }

    private static async Task<int> ErrorPath(string sock, int n)
    {
        using var cc = new CoreChannel(Armonik.Ffi.Campaign.CampaignMain.CoreUri(sock), 2);
        // The queue's drainer polls `ak_queue_next` with a 200 ms timeout, and
        // every poll is a forward crossing, so it is started only for the queue
        // rows (a running drainer put 2.02 on a blocking row), and the check
        // below is on the whole part of the per-call count: idle polls add a
        // few per row, a missing free subtracts one per call.
        // WP10: against the campaign's one server (poc/rust/SERVER.md): `Fetch` answers P2.2
        // with OK; `StatusU13` answers gRPC status 13 (INTERNAL), a non-OK completion.
        var bad = System.Text.Encoding.UTF8.GetBytes("/armonik.ffi.campaign.v1.Grid/StatusU13");
        var good = System.Text.Encoding.UTF8.GetBytes("/armonik.ffi.campaign.v1.Grid/Fetch");
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
