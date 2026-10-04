// D7 (2026-10-04): `akrpc --delivery-semantics --sock EP`. Each core delivery the timed
// delivery cells use (callback with asynchronous continuations, callback with inline
// continuations, completion queue with one drainer) must report a call's outcome as the
// BLOCKING delivery does: status and gRPC status on success, on a non-OK gRPC status and on a
// cancel, for unary calls (copy and move paths) and client streams (copy and move sends),
// against the campaign server (poc/rust/SERVER.md: Fetch, Push, UploadStream answer; StatusU13
// and StatusS13 answer gRPC 13; SleepU and SleepS answer after 3 s, the cancel target).
// The blocking unary call has no handle to cancel (ak_call_unary), so the unary cancel row
// compares the asynchronous deliveries with each other; the blocking stream is cancelled from
// a second thread while ak_call_recv blocks. Every completion's bytes are freed.

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Armonik.Ffi.Facade;
using Armonik.Ffi.Harness;
using Armonik.Ffi.Rpc;

namespace Armonik.Ffi.Campaign;

internal static class DeliverySemantics
{
    private static byte[] P(string m) => Encoding.UTF8.GetBytes("/armonik.ffi.campaign.v1.Grid/" + m);

    private sealed class Res
    {
        public int Rc, Grpc;
        public long Len = -1;
        public override string ToString() => Rc + "/" + Grpc + (Len >= 0 ? " len " + Len : "");
        public bool Same(Res o) => o != null && Rc == o.Rc && Grpc == o.Grpc && Len == o.Len;
    }

    public static async Task<int> Run(string sock)
    {
        RpcInit.Ensure();
        var rt = AkRpc.ak_runtime_new(2);
        var f22 = BuildFacade.P2_2();
        var up = Uploads.Streamed(4);
        var cores = new CoreFfi_ListTasksDetailedResponse();
        var coreu = new CoreFfi_UploadResultDataMessage();
        var cases = new[] { "unary ok", "unary error 13", "unary cancel", "unary_enc ok", "unary_enc error 13", "stream ok", "stream error 13", "stream cancel", "stream send_enc ok" };
        var deliveries = new[] { "blocking", "callback", "callback-inline", "queue" };
        var res = new Dictionary<(string, string), Res>();
        var owned = new List<IDisposable>();
        CoreChannel NewCh()
        {
            var ch = new CoreChannel(rt, CampaignMain.CoreUri(sock), CampaignMain.CoreOpts(false));
            if (AkRpc.ak_client_set_framed(ch.Client, 1) != AkRpc.AK_OK) throw new InvalidOperationException("ak_client_set_framed");
            owned.Add(ch);
            return ch;
        }
        foreach (var dname in deliveries)
        {
            var ch = NewCh();
            DeliveryChannel dc = dname == "blocking" ? null
                : new DeliveryChannel(ch, dname == "callback" ? Delivery.Callback : dname == "callback-inline" ? Delivery.CallbackInline : Delivery.Queue);
            if (dc != null) owned.Insert(0, dc);
            foreach (var c in cases)
            {
                Res r;
                try { r = await One(c, ch, dc, f22, up, cores, coreu); }
                catch (Exception e) { Console.WriteLine("  {0} / {1}: THREW {2}: {3}", dname, c, e.GetType().Name, e.Message); r = new Res { Rc = int.MinValue }; }
                res[(dname, c)] = r;
                // Leave the core's thread: an inline continuation resumes here ON a tokio worker,
                // where any blocking core entry (the next channel's ak_client_new_opts, a
                // blocking call) panics the runtime ("Cannot start a runtime from within a
                // runtime") and aborts the process: observed, and the reason the inline mode is
                // only safe on paths that start calls without blocking (the timed cells).
                await Task.Yield();
            }
        }
        foreach (var o in owned) o.Dispose();
        AkRpc.ak_runtime_destroy(rt);

        Console.WriteLine("# akrpc --delivery-semantics (D7): rc/grpc status [len] per case and delivery; the reference is blocking, or callback where blocking has no form (unary cancel)");
        Console.WriteLine("{0,-20} {1}", "case", string.Join(" ", deliveries.Select(d => d.PadRight(20))));
        int fails = 0;
        foreach (var c in cases)
        {
            var refd = res[("blocking", c)] == null ? "callback" : "blocking";
            var line = new StringBuilder();
            bool ok = true;
            foreach (var d in deliveries)
            {
                var r = res[(d, c)];
                line.Append((r == null ? "n/a" : r.ToString()).PadRight(21));
                if (r != null && !r.Same(res[(refd, c)])) ok = false;
            }
            // what each case must be, whatever the delivery
            var x = res[(refd, c)];
            bool exp = c switch
            {
                "unary ok" => x.Rc == 0 && x.Grpc == 0 && x.Len == 540422,
                "unary_enc ok" or "stream ok" or "stream send_enc ok" => x.Rc == 0 && x.Grpc == 0,
                "unary error 13" or "unary_enc error 13" or "stream error 13" => x.Rc == Armonik.Ffi.Harness.Abi.AK_ERR_RPC_STATUS && x.Grpc == 13,
                _ => x.Rc == Armonik.Ffi.Harness.Abi.AK_ERR_RPC_STATUS && x.Grpc == 1,   // the cancels: CANCELLED
            };
            if (!ok || !exp) fails++;
            Console.WriteLine("{0,-20} {1} {2}", c, line, ok && exp ? "PASS" : ok ? "FAIL (unexpected outcome)" : "FAIL (deliveries differ)");
        }
        Console.WriteLine("delivery-semantics: {0} failure(s)", fails);
        return fails == 0 ? 0 : 1;
    }

    private static async Task<Res> One(string c, CoreChannel ch, DeliveryChannel dc, ListTasksDetailedResponse f22, UpData up, CoreFfi_ListTasksDetailedResponse cores, CoreFfi_UploadResultDataMessage coreu)
    {
        switch (c)
        {
            case "unary ok": return dc == null ? BlockUnary(ch, P("Fetch")) : await AsyncUnary(dc, (u, t) => dc.StartUnary(P("Fetch"), Array.Empty<byte>(), 0, u, t), false);
            case "unary error 13": return dc == null ? BlockUnary(ch, P("StatusU13")) : await AsyncUnary(dc, (u, t) => dc.StartUnary(P("StatusU13"), Array.Empty<byte>(), 0, u, t), false);
            case "unary cancel": return dc == null ? null : await AsyncUnary(dc, (u, t) => dc.StartUnary(P("SleepU"), Array.Empty<byte>(), 0, u, t), true);
            case "unary_enc ok":
            case "unary_enc error 13":
            {
                var path = P(c == "unary_enc ok" ? "Push" : "StatusU13");
                if (cores.EncodeInto(f22, false) < 0) throw new InvalidOperationException("encode");
                return dc == null ? BlockUnaryEnc(ch, path, cores.EncContext) : await AsyncUnary(dc, (u, t) => dc.StartUnaryEnc(path, cores.EncContext, u, t), false);
            }
            default:
            {
                var path = P(c == "stream error 13" ? "StatusS13" : c == "stream cancel" ? "SleepS" : "UploadStream");
                bool enc = c == "stream send_enc ok", cancel = c == "stream cancel";
                return dc == null ? BlockStream(ch, path, up, enc, cancel, coreu) : await AsyncStream(dc, path, up, enc, cancel, coreu);
            }
        }
    }

    private static unsafe Res BlockUnary(CoreChannel ch, byte[] path)
    {
        ak_bytes b = default; int gs = -1, rc;
        fixed (byte* p = path) rc = AkRpc.ak_call_unary(ch.Client, p, (nuint)path.Length, null, 0, &b, &gs);
        var r = new Res { Rc = rc, Grpc = gs, Len = rc == 0 ? (long)b.len : -1 };
        AkRpc.ak_bytes_free(&b);
        return r;
    }

    private static unsafe Res BlockUnaryEnc(CoreChannel ch, byte[] path, IntPtr enc)
    {
        ak_bytes b = default; int gs = -1, rc;
        fixed (byte* p = path) rc = AkRpc.ak_call_unary_enc(ch.Client, p, (nuint)path.Length, enc, &b, &gs);
        var r = new Res { Rc = rc, Grpc = gs, Len = rc == 0 ? (long)b.len : -1 };
        AkRpc.ak_bytes_free(&b);
        return r;
    }

    private static async Task<Res> AsyncUnary(DeliveryChannel dc, Func<IntPtr, ulong, IntPtr> start, bool cancel)
    {
        var p = dc.Begin(out var user, out var tag);
        IntPtr call = start(user, tag);
        if (call == IntPtr.Zero) { dc.Abandon(user, tag); throw new InvalidOperationException("start returned NULL"); }
        if (cancel) { await Task.Delay(200); DeliveryChannel.Cancel(call); }
        var d = await p.Tcs.Task;
        DeliveryChannel.Destroy(call);
        var r = new Res { Rc = d.Status, Grpc = d.Grpc, Len = d.Status == 0 ? (long)d.Bytes.len : -1 };
        DeliveryChannel.Free(d.Bytes);
        return r;
    }

    private static unsafe int SendMsg(IntPtr h, UpData up, int i, bool enc, CoreFfi_UploadResultDataMessage coreu, Func<IntPtr, int, int> sendEnc, Func<IntPtr, int, int, int> send)
    {
        int last = i == up.G.Length - 1 ? 1 : 0;
        if (enc)
        {
            if (coreu.EncodeInto(up.F[i], false) < 0) throw new InvalidOperationException("encode");
            return sendEnc(coreu.EncContext, last);
        }
        var a = Google.Protobuf.MessageExtensions.ToByteArray(up.G[i]);
        fixed (byte* q = a) return send((IntPtr)q, a.Length, last);
    }

    private static unsafe Res BlockStream(CoreChannel ch, byte[] path, UpData up, bool enc, bool cancel, CoreFfi_UploadResultDataMessage coreu)
    {
        IntPtr h;
        fixed (byte* p = path) h = AkRpc.ak_call_open(ch.Client, p, (nuint)path.Length, AkRpc.AK_CALL_CLIENT_STREAM, null);
        if (h == IntPtr.Zero) throw new InvalidOperationException("ak_call_open");
        try
        {
            int firstSend = 0;
            for (int i = 0; i < up.G.Length; i++)
            {
                int rc = SendMsg(h, up, i, enc, coreu, (e, last) => AkRpc.ak_call_send_enc(h, e, last), (q, n, last) => AkRpc.ak_call_send(h, (byte*)q, (nuint)n, last));
                if (rc != 0 && firstSend == 0) firstSend = rc;
                if (rc != 0) break;
            }
            Timer t = cancel ? new Timer(_ => AkRpc.ak_call_cancel(h), null, 200, Timeout.Infinite) : null;
            ak_bytes b = default; int gs = -1;
            int rr = AkRpc.ak_call_recv(h, &b, &gs);
            t?.Dispose();
            var r = new Res { Rc = rr, Grpc = gs, Len = rr == 0 ? (long)b.len : -1 };
            AkRpc.ak_bytes_free(&b);
            return r;
        }
        finally { AkRpc.ak_call_destroy(h); }
    }

    private static unsafe int SendCopy(DeliveryChannel dc, IntPtr h, IntPtr q, int n, int last, IntPtr u, ulong t) => dc.StartSend(h, (byte*)q, n, last, u, t);

    private static async Task<Res> AsyncStream(DeliveryChannel dc, byte[] path, UpData up, bool enc, bool cancel, CoreFfi_UploadResultDataMessage coreu)
    {
        IntPtr h = dc.Open(path);
        if (h == IntPtr.Zero) throw new InvalidOperationException("ak_call_open");
        try
        {
            for (int i = 0; i < up.G.Length; i++)
            {
                var ps = dc.Begin(out var u1, out var t1);
                int rc = SendMsg(h, up, i, enc, coreu, (e, last) => dc.StartSendEnc(h, e, last, u1, t1), (q, n, last) => SendCopy(dc, h, q, n, last, u1, t1));
                if (rc != 0) { dc.Abandon(u1, t1); break; }
                var sd = await ps.Tcs.Task;
                DeliveryChannel.Free(sd.Bytes);
                if (sd.Status != 0) break;
            }
            var pr = dc.Begin(out var u2, out var t2);
            int rr = dc.StartRecv(h, u2, t2);
            if (rr != 0) { dc.Abandon(u2, t2); return new Res { Rc = rr, Grpc = -1 }; }
            if (cancel) { await Task.Delay(200); DeliveryChannel.Cancel(h); }
            var d = await pr.Tcs.Task;
            var r = new Res { Rc = d.Status, Grpc = d.Grpc, Len = d.Status == 0 ? (long)d.Bytes.len : -1 };
            DeliveryChannel.Free(d.Bytes);
            return r;
        }
        finally { DeliveryChannel.Destroy(h); }
    }
}
