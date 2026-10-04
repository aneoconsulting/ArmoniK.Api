// Owner decision D7 (2026-10-04, optimisation step 4): every core completion delivery for the
// core cells Bf, Cf-retain and Ef-retain, each as its own labelled cell beside the blocking
// one (which is unchanged):
//
//   <cell>.callback         the core's completion callback ([UnmanagedCallersOnly] OnDone, on a
//                           core runtime (tokio) worker thread) completes a TaskCompletionSource
//                           created with RunContinuationsAsynchronously: the awaiting
//                           continuation is queued to the .NET thread pool; the callback copies
//                           three words and returns.
//   <cell>.callback-inline  a labelled extra: the same with TaskCreationOptions.None, so the
//                           awaiting continuation (the decode, the next call's encode and start)
//                           runs INLINE on the core's tokio worker inside the callback. Safe for
//                           these cells: nothing on that path blocks on the core (every start is
//                           a non-blocking _cb entry, ak_call_destroy only frees the handle,
//                           whose in-flight task holds its own references), and a send's
//                           completion explicitly allows the next send from inside it (rpc.rs).
//   <cell>.queue            ak_queue with ONE drainer thread: a blocking ak_queue_next (1 s
//                           timeout, only to notice shutdown) pops one completion at a time (the
//                           ABI has no batch pop) and completes the matching
//                           TaskCompletionSource (RunContinuationsAsynchronously, so the decode
//                           never runs on the drainer).
//
// The same k awaiting async loops as the Grpc.Net cells (CampaignMain.RunCell). Streaming
// direction d uses the streaming form of each delivery: ak_call_send_cb / _q (or
// ak_call_send_enc_cb / _q on the move path), each send awaited before the next (the core
// allows one pending send), then ak_call_recv_cb / _q. Per-call allocation, stated: a
// Pending (with its TaskCompletionSource) per completion, and a GCHandle per callback
// completion (allocated at the start, freed in the callback).

using System;
using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Armonik.Ffi.Rpc;

namespace Armonik.Ffi.Campaign;

internal enum Delivery { Callback, CallbackInline, Queue }

/// One completion: the core's status, the gRPC status and the bytes (owned by the receiver).
internal struct Done
{
    public int Status, Grpc;
    public ak_bytes Bytes;
}

internal sealed class Pending
{
    public readonly TaskCompletionSource<Done> Tcs;
    public Pending(bool inline) => Tcs = new TaskCompletionSource<Done>(inline ? TaskCreationOptions.None : TaskCreationOptions.RunContinuationsAsynchronously);
}

/// A core channel with one delivery's machinery (the queue and its drainer for Queue).
internal sealed unsafe class DeliveryChannel : IDisposable
{
    public readonly CoreChannel Ch;
    public readonly Delivery D;
    private IntPtr _q;
    private Thread _drainer;
    private readonly ConcurrentDictionary<ulong, Pending> _pending = new();
    private long _tag;
    /// Completions this channel's drainer popped (the counting run's per-cell figure).
    public long Pops;
    /// Which thread ran the last callback / continuation (the inline extra's statement).
    public static string LastCallbackThread = "";

    public DeliveryChannel(CoreChannel ch, Delivery d)
    {
        Ch = ch;
        D = d;
        if (d != Delivery.Queue) return;
        _q = AkRpc.ak_queue_new();
        if (_q == IntPtr.Zero) throw new InvalidOperationException("ak_queue_new");
        _drainer = new Thread(Drain) { IsBackground = true, Name = "ak-queue-drainer" };
        _drainer.Start();
    }

    private void Drain()
    {
        while (true)
        {
            ak_completion c;
            int rc = AkRpc.ak_queue_next(_q, &c, 1000);
            if (rc == AkRpc.AK_QUEUE_SHUTDOWN) return;
            if (rc != AkRpc.AK_QUEUE_OK) continue;
            Interlocked.Increment(ref Pops);
            if (_pending.TryRemove(c.tag, out var p))
                p.Tcs.TrySetResult(new Done { Status = c.status, Grpc = c.grpc_status, Bytes = c.bytes });
            else { var b = c.bytes; AkRpc.ak_bytes_free(&b); }
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static void OnDone(IntPtr user, ak_completion* comp)
    {
        // A core runtime (tokio) worker thread. Three words, a free and a set.
        var h = GCHandle.FromIntPtr(user);
        var p = (Pending)h.Target;
        h.Free();
        p.Tcs.TrySetResult(new Done { Status = comp->status, Grpc = comp->grpc_status, Bytes = comp->bytes });
    }

    /// A completion slot: a GCHandle for the callback deliveries, a tag for the queue.
    public Pending Begin(out IntPtr user, out ulong tag)
    {
        var p = new Pending(D == Delivery.CallbackInline);
        if (D == Delivery.Queue)
        {
            tag = (ulong)Interlocked.Increment(ref _tag);
            _pending[tag] = p;
            user = IntPtr.Zero;
        }
        else
        {
            tag = 0;
            user = GCHandle.ToIntPtr(GCHandle.Alloc(p));
        }
        return p;
    }

    /// A start that failed delivers nothing: release its slot.
    public void Abandon(IntPtr user, ulong tag)
    {
        if (D == Delivery.Queue) _pending.TryRemove(tag, out _);
        else GCHandle.FromIntPtr(user).Free();
    }

    public IntPtr StartUnary(byte[] path, byte* body, int len, IntPtr user, ulong tag)
    {
        fixed (byte* p = path)
            return D == Delivery.Queue
                ? AkRpc.ak_call_unary_q(Ch.Client, p, (nuint)path.Length, body, (nuint)len, _q, tag)
                : AkRpc.ak_call_unary_cb(Ch.Client, p, (nuint)path.Length, body, (nuint)len, &OnDone, user, tag);
    }

    public IntPtr StartUnary(byte[] path, byte[] body, int len, IntPtr user, ulong tag)
    {
        fixed (byte* b = body) return StartUnary(path, b, len, user, tag);
    }

    /// The move path: the encode context's output moved into the request before this returns.
    public IntPtr StartUnaryEnc(byte[] path, IntPtr enc, IntPtr user, ulong tag)
    {
        fixed (byte* p = path)
            return D == Delivery.Queue
                ? AkRpc.ak_call_unary_enc_q(Ch.Client, p, (nuint)path.Length, enc, _q, tag)
                : AkRpc.ak_call_unary_enc_cb(Ch.Client, p, (nuint)path.Length, enc, &OnDone, user, tag);
    }

    public IntPtr Open(byte[] path)
    {
        fixed (byte* p = path) return AkRpc.ak_call_open(Ch.Client, p, (nuint)path.Length, AkRpc.AK_CALL_CLIENT_STREAM, null);
    }

    public int StartSend(IntPtr h, byte* msg, int len, int last, IntPtr user, ulong tag) =>
        D == Delivery.Queue ? AkRpc.ak_call_send_q(h, msg, (nuint)len, last, _q, tag)
                            : AkRpc.ak_call_send_cb(h, msg, (nuint)len, last, &OnDone, user, tag);

    public int StartSendEnc(IntPtr h, IntPtr enc, int last, IntPtr user, ulong tag) =>
        D == Delivery.Queue ? AkRpc.ak_call_send_enc_q(h, enc, last, _q, tag)
                            : AkRpc.ak_call_send_enc_cb(h, enc, last, &OnDone, user, tag);

    public int StartRecv(IntPtr h, IntPtr user, ulong tag) =>
        D == Delivery.Queue ? AkRpc.ak_call_recv_q(h, _q, tag) : AkRpc.ak_call_recv_cb(h, &OnDone, user, tag);

    public static void Destroy(IntPtr call) => AkRpc.ak_call_destroy(call);
    public static void Cancel(IntPtr call) => AkRpc.ak_call_cancel(call);
    public static void Free(ak_bytes b) => AkRpc.ak_bytes_free(&b);
    public static byte[] Copy(ak_bytes b) => new ReadOnlySpan<byte>((void*)b.ptr, (int)b.len).ToArray();

    public void Dispose()
    {
        if (_q == IntPtr.Zero) return;
        AkRpc.ak_queue_shutdown(_q);
        _drainer?.Join(5000);
        AkRpc.ak_queue_destroy(_q);
        _q = IntPtr.Zero;
    }
}

/// The async flows of the delivery cells. The pointer work is in the synchronous starts
/// (C# allows no pointer in an async method); every completion is awaited before its call
/// handle is destroyed.
internal static class DeliveryFlows
{
    public static async Task<Done> Unary(DeliveryChannel dc, Func<IntPtr, ulong, IntPtr> start, string cell)
    {
        var p = dc.Begin(out var user, out var tag);
        IntPtr call;
        try { call = start(user, tag); }
        catch { dc.Abandon(user, tag); throw; }
        if (call == IntPtr.Zero) { dc.Abandon(user, tag); throw new CampaignAbort(cell + ": the call's start returned NULL"); }
        var d = await p.Tcs.Task.ConfigureAwait(false);
        DeliveryChannel.Destroy(call);
        return d;
    }

    /// One client stream: `send(i, user, tag)` encodes message i and starts its send; each
    /// send's completion is awaited before the next; then the response.
    public static async Task<Done> Stream(DeliveryChannel dc, byte[] path, int n, Func<IntPtr, int, IntPtr, ulong, int> send, string cell)
    {
        IntPtr h = dc.Open(path);
        if (h == IntPtr.Zero) throw new CampaignAbort(cell + " d: ak_call_open returned NULL");
        try
        {
            for (int i = 0; i < n; i++)
            {
                var ps = dc.Begin(out var u1, out var t1);
                int rc;
                try { rc = send(h, i, u1, t1); }
                catch { dc.Abandon(u1, t1); throw; }
                if (rc != AkRpc.AK_OK) { dc.Abandon(u1, t1); throw new CampaignAbort(cell + " d: send start " + rc + " (message " + i + ")"); }
                var sd = await ps.Tcs.Task.ConfigureAwait(false);
                if (sd.Status != AkRpc.AK_OK) { DeliveryChannel.Free(sd.Bytes); throw new CampaignAbort(cell + " d: send " + sd.Status + " (message " + i + ")"); }
            }
            var pr = dc.Begin(out var u2, out var t2);
            int rr = dc.StartRecv(h, u2, t2);
            if (rr != AkRpc.AK_OK) { dc.Abandon(u2, t2); throw new CampaignAbort(cell + " d: recv start " + rr); }
            return await pr.Tcs.Task.ConfigureAwait(false);
        }
        finally { DeliveryChannel.Destroy(h); }
    }
}
