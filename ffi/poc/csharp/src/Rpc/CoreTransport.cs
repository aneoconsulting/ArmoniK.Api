// ABI v1 section 9's transport, bound from .NET, with all three deliveries.
//
// The codec half of this ABI has been measured here since stage 8. The TRANSPORT
// half had not been reachable from C# at all: the shared core builds without the
// `rpc` feature by default, and until now the only delivery was the blocking
// call, which is the wrong shape for this runtime.
//
// **Why the callback is the mode .NET wants, stated before it is measured.** The
// completion fires once, on a tokio worker thread the host does not own. A
// runtime that must attach a thread before it can run managed code pays for that;
// .NET's reverse P/Invoke attaches it, and what the callback then has to do is
// complete a `TaskCompletionSource`, which is explicitly completable from any
// thread. So the upcall is one crossing -- 7.5 to 12 ns on this host by this
// slice's own R13 calibration -- and no thread of the host's is parked anywhere.
// On the JVM the same upcall is 72 to 80 ns and the queue wins instead. Two
// hosts, one interface, opposite best modes.
//
// **The delegate-rooting hazard this slice found on the floor does NOT apply
// here, and the reason is worth stating.** `[UnmanagedCallersOnly]` does not
// create a thunk: the method is compiled to a native entry point at build time
// and `&OnDone` is its address, so there is nothing for the collector to
// reclaim. The hazard is `Marshal.GetFunctionPointerForDelegate`, whose thunk
// lives exactly as long as the delegate object, and that is what a net48 binding
// would have to use. The floor binding would have to root the delegate for the
// lifetime of every in-flight call; the target binding has no such rule.
//
// **What the callback must NOT do**, and it is the one real trap: run managed
// work of consequence on the tokio worker. A continuation scheduled inline would
// decode 540 KB on a thread the core needs for its own I/O. The
// `TaskCompletionSource` is therefore created with
// `RunContinuationsAsynchronously`, and the callback copies three words and
// returns.

using System;
using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace Armonik.Ffi.Rpc;

// The transport's structs and prototypes (`ak_bytes`, `ak_completion`,
// `ak_client_opts`, every `ak_*` RPC entry point, the counting surface
// `ak_rpc_counting`/`ak_rpc_counters`/`ak_rpc_counters_reset` with `ak_rpc_counters`,
// and ak_init) are GENERATED from plan.rpc and plan.FIXED into Generated/RpcAbi.cs
// (FIX-PLAN WP5, R-G5, D40), LibraryImport under NET7_0_OR_GREATER and DllImport
// otherwise. Nothing of the ABI is declared by hand here.

/// What one in-flight call is waiting on. Pinned by a `GCHandle` for as long as
/// the core can complete it, and freed by whichever side finishes it.
internal sealed class CallState
{
    /// **`RunContinuationsAsynchronously` is load bearing.** Without it the
    /// awaiting continuation -- which decodes half a megabyte -- runs INLINE on
    /// the tokio worker thread that delivered the completion, inside the core's
    /// own I/O pool.
    public readonly TaskCompletionSource<bool> Tcs =
        new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
    public ak_bytes Bytes;
    public int Status;
}

/// A client over the core's transport. `ak_client_new` takes a `unix:` target and
/// tonic dials the socket itself, so every cell of the grid runs over the same
/// UDS the shipped C# client uses.
public sealed class CoreChannel : IDisposable
{
    private readonly IntPtr _rt, _cl;
    /// The core client handle, for callers that drive the generated imports directly
    /// (the campaign runner's blocking cells, which hand the core pointers, not arrays).
    public IntPtr Client => _cl;
    private IntPtr _q;
    private Thread _drainer;
    private readonly ConcurrentDictionary<ulong, CallState> _pending = new();
    private long _tag;

    private readonly bool _ownsRt = true;

    /// CAMPAIGN req 13 (R-H33): a client on a runtime the caller owns, so every cell of a
    /// process gets ITS OWN channel (its own connection) while all of them share one runtime
    /// whose worker count the log header states. The caller destroys the runtime after the
    /// last channel.
    public unsafe CoreChannel(IntPtr runtime, string uri, ak_client_opts opts)
    {
        RpcInit.Ensure();
        _rt = runtime;
        _ownsRt = false;
        var u = Encoding.UTF8.GetBytes(uri);
        fixed (byte* p = u) _cl = AkRpc.ak_client_new_opts(_rt, p, (nuint)u.Length, &opts);
        if (_cl == IntPtr.Zero) throw new InvalidOperationException("ak_client_new failed for " + uri + " (set AK_RPC_TRACE=1 for the core's reason)");
    }

    public unsafe CoreChannel(string uri, int workerThreads, ak_client_opts? opts = null)
    {
        // ABI v1 section 3: ak_init before any other entry point. The generated
        // RPC binding renders it (plan.lifecycle, R-G7) in AkRpc's static
        // constructor; the codec binding does the same, and the second call
        // returns AK_ALREADY_INITIALIZED, a success.
        RpcInit.Ensure();
        _rt = AkRpc.ak_runtime_new((uint)workerThreads);
        if (_rt == IntPtr.Zero) throw new InvalidOperationException("ak_runtime_new");
        var u = Encoding.UTF8.GetBytes(uri);
        if (opts is ak_client_opts o)
            fixed (byte* p = u) _cl = AkRpc.ak_client_new_opts(_rt, p, (nuint)u.Length, &o);
        else
            fixed (byte* p = u) _cl = AkRpc.ak_client_new(_rt, p, (nuint)u.Length);
        if (_cl == IntPtr.Zero)
        {
            AkRpc.ak_runtime_destroy(_rt);
            throw new InvalidOperationException("ak_client_new failed for " + uri
                + " (set AK_RPC_TRACE=1 for the core's reason)");
        }
    }

    // ---- the callback delivery, the headline on this host --------------------

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static unsafe void OnDone(IntPtr user, ak_completion* comp)
    {
        // Runs on a TOKIO WORKER THREAD. Three words and a set; nothing else.
        var h = GCHandle.FromIntPtr(user);
        var st = (CallState)h.Target;
        st.Status = comp->status;
        st.Bytes = comp->bytes;
        h.Free();
        st.Tcs.TrySetResult(true);
    }

    /// One call. The response bytes stay the CORE's until `Release` is called, so
    /// a codec can read them in place and the second crossing is the free.
    /// The pointer half, split out because C# forbids `await` in an unsafe
    /// context and the call has to be STARTED with pointers and AWAITED without.
    private unsafe IntPtr StartCb(byte[] path, byte[] req, IntPtr user, ulong tag)
    {
        fixed (byte* p = path)
        fixed (byte* r = req)
            return AkRpc.ak_call_unary_cb(_cl, p, (nuint)path.Length,
                r, (nuint)req.Length, &OnDone, user, tag);
    }

    public async Task<ak_bytes> CallCbAsync(byte[] path, byte[] req)
    {
        var st = new CallState();
        var h = GCHandle.Alloc(st);
        IntPtr call = StartCb(path, req, GCHandle.ToIntPtr(h),
            (ulong)Interlocked.Increment(ref _tag));
        if (call == IntPtr.Zero) { h.Free(); throw new InvalidOperationException("ak_call_unary_cb"); }
        await st.Tcs.Task.ConfigureAwait(false);
        AkRpc.ak_call_destroy(call);
        return TakeOrThrow(st);
    }

    // ---- the blocking call, the labelled row ---------------------------------

    /// **Blocks the calling thread INSIDE the core.** With N in flight that is N
    /// host threads parked in a native frame, which is the shape section 9 says
    /// pins a virtual thread's carrier on the JVM. .NET has no carrier to pin,
    /// but a parked thread-pool thread is still a thread-pool thread.
    public unsafe ak_bytes CallBlocking(byte[] path, byte[] req)
    {
        ak_bytes b = default;
        int rc;
        fixed (byte* p = path)
        fixed (byte* r = req)
            rc = AkRpc.ak_call_unary(_cl, p, (nuint)path.Length, r, (nuint)req.Length, &b);
        if (rc != AkRpc.AK_OK)
        {
            // Same rule as TakeOrThrow (R-D9): `out` starts zeroed, so this is a
            // no-op unless the core wrote an owned body before failing.
            Release(ref b);
            throw new InvalidOperationException("ak_call_unary rc " + rc);
        }
        return b;
    }

    // ---- the completion queue, the second row --------------------------------

    /// One drainer, as section 9 specifies. The core's own comment says running
    /// two is a deadlock-shaped mystery rather than an error, so this runs one
    /// and says so.
    public unsafe void StartQueue()
    {
        _q = AkRpc.ak_queue_new();
        _drainer = new Thread(() =>
        {
            while (true)
            {
                ak_completion c;
                int rc = AkRpc.ak_queue_next(_q, &c, 200);
                if (rc == AkRpc.AK_QUEUE_SHUTDOWN) return;
                if (rc != AkRpc.AK_QUEUE_OK) continue;
                if (_pending.TryRemove(c.tag, out var st))
                {
                    st.Status = c.status;
                    st.Bytes = c.bytes;
                    st.Tcs.TrySetResult(true);
                }
            }
        }) { IsBackground = true, Name = "ak-queue-drainer" };
        _drainer.Start();
    }

    private unsafe IntPtr StartQ(byte[] path, byte[] req, ulong tag)
    {
        fixed (byte* p = path)
        fixed (byte* r = req)
            return AkRpc.ak_call_unary_q(_cl, p, (nuint)path.Length, r, (nuint)req.Length, _q, tag);
    }

    public async Task<ak_bytes> CallQAsync(byte[] path, byte[] req)
    {
        ulong tag = (ulong)Interlocked.Increment(ref _tag);
        var st = new CallState();
        _pending[tag] = st;
        IntPtr call = StartQ(path, req, tag);
        if (call == IntPtr.Zero) { _pending.TryRemove(tag, out _); throw new InvalidOperationException("ak_call_unary_q"); }
        await st.Tcs.Task.ConfigureAwait(false);
        AkRpc.ak_call_destroy(call);
        return TakeOrThrow(st);
    }

    /// R-D9: a completion owns its `bytes` whatever its status, and
    /// `ak_bytes_free` is the one release path (ABI v1 section 9). Today's core
    /// hands back an empty `ak_bytes` on failure, so the free is a no-op there,
    /// but the binding must not depend on that: a core that attaches an error
    /// body would otherwise leak it on every failed call. Freed BEFORE the
    /// throw, so no exception path can skip it.
    private static ak_bytes TakeOrThrow(CallState st)
    {
        if (st.Status == AkRpc.AK_OK) return st.Bytes;
        var b = st.Bytes;
        st.Bytes = default;
        Release(ref b);
        throw new InvalidOperationException("core call status " + st.Status);
    }

    /// The second crossing, and the only other one.
    public static unsafe void Release(ref ak_bytes b)
    {
        fixed (ak_bytes* p = &b) AkRpc.ak_bytes_free(p);
    }

    public unsafe void Dispose()
    {
        if (_q != IntPtr.Zero)
        {
            AkRpc.ak_queue_shutdown(_q);
            _drainer?.Join(2000);
            AkRpc.ak_queue_destroy(_q);
            _q = IntPtr.Zero;
        }
        AkRpc.ak_client_destroy(_cl);
        if (_ownsRt) AkRpc.ak_runtime_destroy(_rt);
    }
}
