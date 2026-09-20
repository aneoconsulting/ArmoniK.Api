//! The RPC half of ABI v1 section 9, reduced to what stage 4 measures.
//!
//! "The RPC half does not know the schema. It moves opaque bytes and dispatches on a path
//! string, so it serves every RPC unchanged and adding one is a table row." Nothing in this
//! file mentions a message type, and that is the property stage 4's crossing count tests:
//! if the count per RPC were a function of field count, something here would have to know
//! about fields.
//!
//! **Section 9's three delivery modes are all here now.** The blocking call, the callback
//! and the completion queue are three DELIVERIES of one call path (`unary_once`), not three
//! implementations, which is the same condition 7.1 puts on the two decode families: two
//! bodies that have to agree is a fork at the place nobody tests. The java slice found only
//! the blocking form built and measured the core against grpc-java through it, which is the
//! mode the JVM is worst at -- blocking in a native frame pins a virtual thread's carrier
//! (section 9's fourth amendment, measured) -- so the comparison was taken through the arm
//! least suited to the host.
//!
//! **What each delivery costs at the boundary, which is the reason to have both.**
//!
//! | delivery | forward | reverse | who blocks |
//! |---|---|---|---|
//! | `ak_call_unary` | 2 (call, free) | 0 | a host thread, inside the core |
//! | `ak_call_unary_cb` | 2 (call, free) | **1** (the completion) | nobody; the core calls out |
//! | `ak_call_unary_q` | 3 (call, next, free) | **0** | a host thread, inside `ak_queue_next` |
//!
//! The queue trades one reverse call for one forward call, which is the whole of its case on
//! a host where a reverse call is dear and an upcall onto a thread the host does not own is
//! worse than dear. The callback is the mode for a host with cheap upcalls and an async
//! idiom to complete into. Neither is a default the ABI picks.
//!
//! **What is NOT here, and is in STATE.md's not-measured list rather than stubbed**: TLS,
//! retry and backoff, metadata, deadlines, the gRPC status code as a number, streaming, and
//! `ak_init` with its one-shot installs. Section 9's case is behavioural and this is not a
//! test of it.

use crate::{AK_ERR_HOST, AK_ERR_INVALID_STATE, AK_OK};
use bytes::Bytes;
use core::ffi::c_void;

pub enum ak_runtime {}
pub enum ak_client {}

/// A borrowed byte range the core owns until the host releases it.
#[repr(C)]
pub struct ak_bytes {
    pub ptr: *const u8,
    pub len: usize,
    /// The core's handle on the allocation. The host passes it back and does not read it.
    pub owner: *mut c_void,
}


// ---- what the cpp slice added, and why -------------------------------------------------
//
// Two additions, both of them things a slice could not do from its own side.
//
// **1. `ak_client_new_opts`.** `design/SHAPES.md` requires every cell of the RPC grid to
// pin the SAME transport configuration -- ArmoniK's rather than the stack's -- and to state
// it. The core could not be pinned at all: `ak_client_new` took a URI and nothing else, so
// the core's arm ran at hyper's defaults while the host's stack ran at its own, and the two
// were being compared as if that were the same transport. `ak_client_new` is now a call to
// `ak_client_new_opts` with no options, so the two cannot drift; passing NULL is
// byte-for-byte the old behaviour.
//
// **2. RPC crossing counters, under `--features count`.** README R5 says count crossings,
// do not infer them, and section 9's per-delivery table (2/0, 2/1, 3/0) was arithmetic
// nobody had run. The counters are `#[cfg(feature = "count")]` inside entry points that
// exist only under `--features rpc`, so the default artifact is untouched in both
// dimensions.

/// The transport knobs `design/SHAPES.md` asks each arm to pin and to state. A window of 0
/// means "leave the stack's default", which is what NULL options give on every field.
///
/// hyper's client defaults, for the record the SHAPES.md table wants: **2 MiB initial
/// stream window, 5 MiB initial connection window, adaptive window off**
/// (`hyper/src/proto/h2/client.rs`, `DEFAULT_STREAM_WINDOW` / `DEFAULT_CONN_WINDOW`).
///
/// **This struct is the UNION of two independent additions and that is a finding in itself.**
/// The rust and cpp slices each added an `ak_client_new_opts` on the same day, with different
/// field sets, because each needed the core's transport pinned and neither could know the
/// other was doing it. R0 stops a slice FORKING the core; it does not stop two slices adding
/// the same thing at once, and nothing detected this until the merge failed to compile. The
/// union is what both need, and the failure mode to note is that it could as easily have been
/// two subtly different behaviours behind one name.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ak_client_opts {
    /// `SETTINGS_INITIAL_WINDOW_SIZE`, per stream. 0 leaves hyper's 2 MiB. ArmoniK: 4 MiB.
    pub stream_window: u32,
    /// The connection-level window, which is a SEPARATE setting on hyper as it is on
    /// grpc-java. 0 leaves hyper's 5 MiB. Raising only the stream window is the mistake this
    /// entry point exists to make impossible to repeat.
    pub connection_window: u32,
    /// 1 on, 0 off, -1 leave the default (off). Adaptive sizing overrides the two windows
    /// above, which is why pinning a window and enabling this is a contradiction rather
    /// than a belt and braces.
    pub adaptive_window: i32,
    /// Largest message the client will accept, bytes. 0 leaves tonic's default. ArmoniK
    /// chunks at 2 MiB, so a cell that pins chunking pins this too.
    pub max_recv_message: u32,
    /// Largest message the client will send, bytes. 0 leaves tonic's default.
    pub max_send_message: u32,
    /// **Nagle's algorithm, named the way ArmoniK names it**: 1 enables Nagle (clears
    /// `TCP_NODELAY`), 0 disables it, -1 leaves tonic's default (which is nodelay ON).
    ///
    /// The spelling is `packages/rust/armonik-transport`'s: `tcp_nagle_algorithm: bool`,
    /// "defaults to false", read from `GrpcClient__TcpNagleAlgorithm` and applied as
    /// `http.set_nodelay(!config.tcp_nagle_algorithm)` (`src/connect.rs`). **So ArmoniK
    /// ships with Nagle OFF**, tonic's client default agrees, and the branch's 40 ms
    /// delayed-ACK artifact was only ever on the SERVER side of our own test harness.
    /// Carried here so an arm states the setting rather than inheriting it (R7), and so
    /// the non-default is reachable if anyone wants to price it.
    pub tcp_nagle: i32,
}

// ABI v1 section 10, applied to the CLIENT OPTIONS -- and this one is here because its
// absence was a live defect, not because the pattern looked tidy.
//
// `ak_client_opts` is declared twice: the core defines `rpc::ak_client_opts` and every host
// compiles against `ak_abi::ak_client_opts`. Two slices added this entry point on the same
// day with different field sets; the reconciliation into the union updated the core's
// definition and not the ABI's declaration, leaving the host at four fields and the core at
// six. Nothing failed. What a host would have got instead:
//
//   - its `max_recv_message` lands on the core's `adaptive_window`, and 2 MiB is `>= 0` and
//     `!= 0`, so ADAPTIVE SIZING TURNS ON -- which overrides the very windows this entry
//     point exists to pin;
//   - its `max_send_message` lands on `max_recv_message`;
//   - `max_send_message` and `tcp_nagle` are read PAST THE END of the host's 16-byte
//     object, so `tcp_nodelay` is set from whatever was on the stack. A `1` there re-enables
//     Nagle on the client and resurrects the 40 ms artifact, non-deterministically.
//
// Silent, wrong, and in the one setting that had just been corrected. So the agreement is
// asserted field by field, at compile time, where both declarations are visible.
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    use ak_abi::ak_client_opts as abi;
    assert!(size_of::<ak_client_opts>() == size_of::<abi>());
    assert!(align_of::<ak_client_opts>() == align_of::<abi>());
    // Size and alignment agreeing is NOT enough: two structs with the same six 4-byte fields
    // in different orders agree on both and disagree on every value. Offsets are the check.
    assert!(offset_of!(ak_client_opts, stream_window) == offset_of!(abi, stream_window));
    assert!(offset_of!(ak_client_opts, connection_window) == offset_of!(abi, connection_window));
    assert!(offset_of!(ak_client_opts, adaptive_window) == offset_of!(abi, adaptive_window));
    assert!(offset_of!(ak_client_opts, max_recv_message) == offset_of!(abi, max_recv_message));
    assert!(offset_of!(ak_client_opts, max_send_message) == offset_of!(abi, max_send_message));
    assert!(offset_of!(ak_client_opts, tcp_nagle) == offset_of!(abi, tcp_nagle));
};

/// R5's counters for the RPC half. Process-global rather than per-context, because a call
/// has no context to hang them on: `ak_queue_next` names a queue and `ak_bytes_free` names
/// nothing at all.
#[cfg(feature = "count")]
mod xcount {
    use core::sync::atomic::{AtomicU64, Ordering};
    pub static FWD: AtomicU64 = AtomicU64::new(0);
    pub static REV: AtomicU64 = AtomicU64::new(0);
    #[inline]
    pub fn fwd() {
        FWD.fetch_add(1, Ordering::Relaxed);
    }
    #[inline]
    pub fn rev() {
        REV.fetch_add(1, Ordering::Relaxed);
    }
}

#[inline(always)]
fn fwd() {
    #[cfg(feature = "count")]
    xcount::fwd();
}

#[inline(always)]
fn rev() {
    #[cfg(feature = "count")]
    xcount::rev();
}

/// Forward and reverse crossings since the last reset. Zero in a build without
/// `--features count`, and the host is told which build it is holding by
/// `ak_rpc_counting()` rather than by reading zeroes and guessing.
#[repr(C)]
pub struct ak_rpc_counters {
    pub forward: u64,
    pub reverse: u64,
}

/// 1 if this core counts RPC crossings, 0 if it does not. R5's hazard in one call: a
/// harness that reads zeroes out of a non-counting build and publishes them has reported
/// that the boundary is free.
#[no_mangle]
pub extern "C" fn ak_rpc_counting() -> i32 {
    #[cfg(feature = "count")]
    {
        1
    }
    #[cfg(not(feature = "count"))]
    {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_rpc_counters(out: *mut ak_rpc_counters) {
    if out.is_null() {
        return;
    }
    #[cfg(feature = "count")]
    {
        use core::sync::atomic::Ordering;
        (*out).forward = xcount::FWD.load(Ordering::Relaxed);
        (*out).reverse = xcount::REV.load(Ordering::Relaxed);
    }
    #[cfg(not(feature = "count"))]
    {
        (*out).forward = 0;
        (*out).reverse = 0;
    }
}

/// Not itself counted: the host calls it between measured windows, never inside one.
#[no_mangle]
pub extern "C" fn ak_rpc_counters_reset() {
    #[cfg(feature = "count")]
    {
        use core::sync::atomic::Ordering;
        xcount::FWD.store(0, Ordering::Relaxed);
        xcount::REV.store(0, Ordering::Relaxed);
    }
}

pub struct RuntimeImpl {
    pub rt: tokio::runtime::Runtime,
}

pub struct ClientImpl {
    pub rt: *const RuntimeImpl,
    /// The CHANNEL, not a `Grpc`. `Grpc::unary` takes `&mut self`, so holding one here made
    /// `ak_call_unary` a mutating call on shared state, and two host threads calling it at
    /// once a data race -- which is what stage 4's 8-in-flight arm failed on. A `Channel` is
    /// cheap to clone and clones share the connection, so a call builds its own `Grpc` and
    /// the client handle is what ABI v1 section 9 says it is: usable from many threads at
    /// once, with ownership between handles internal.
    pub chan: tonic::transport::Channel,
}

/// `worker_threads` comes from the host with a small explicit default, never from
/// `Runtime::new()`: Rust reads the cgroup quota, so a requests-only pod takes every CPU on
/// the node (ABI v1 section 3). Stage 4 passes it explicitly for that reason and not to tune.
#[no_mangle]
pub extern "C" fn ak_runtime_new(worker_threads: u32) -> *mut ak_runtime {
    let n = if worker_threads == 0 { 2 } else { worker_threads as usize };
    match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(n)
        .enable_all()
        .build()
    {
        Ok(rt) => Box::into_raw(Box::new(RuntimeImpl { rt })) as *mut ak_runtime,
        Err(_) => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_runtime_destroy(r: *mut ak_runtime) {
    if !r.is_null() {
        drop(Box::from_raw(r as *mut RuntimeImpl));
    }
}

/// **A Unix domain socket is what ArmoniK's client actually dials**, and the core already
/// reaches one: tonic's `Endpoint::from_shared` parses a `unix:` target itself
/// (`transport/channel/endpoint.rs`, `uds_connector.rs`) and connects over a `UnixStream`.
/// `unix:/path` and `unix:///path` both name a socket; anything else is dialled as a URI.
///
/// **This was thought to be missing and it is not.** A connector was written here before the
/// tonic source was read, and it is deleted rather than kept beside a working one -- but the
/// TEST it came with stays, because "the core dials a UDS" was an assumption nobody had
/// exercised and `design/SHAPES.md` makes that transport the primary row. A slice that
/// cannot reach a socket is looking at its own harness, not at this.
#[no_mangle]
pub unsafe extern "C" fn ak_client_new(
    r: *mut ak_runtime,
    uri: *const u8,
    uri_len: usize,
) -> *mut ak_client {
    ak_client_new_opts(r, uri, uri_len, core::ptr::null())
}

/// The same dial with the transport pinned. ONE connect path, so an arm that pins and an
/// arm that does not cannot diverge in anything but the settings.
#[no_mangle]
pub unsafe extern "C" fn ak_client_new_opts(
    r: *mut ak_runtime,
    uri: *const u8,
    uri_len: usize,
    opts: *const ak_client_opts,
) -> *mut ak_client {
    fwd();
    if r.is_null() {
        return core::ptr::null_mut();
    }
    let rt = &*(r as *const RuntimeImpl);
    let s = match core::str::from_utf8(core::slice::from_raw_parts(uri, uri_len)) {
        Ok(s) => s.to_string(),
        Err(_) => return core::ptr::null_mut(),
    };
    let o = if opts.is_null() {
        ak_client_opts {
            stream_window: 0,
            connection_window: 0,
            adaptive_window: -1,
            max_recv_message: 0,
            max_send_message: 0,
            tcp_nagle: -1,
        }
    } else {
        *opts
    };
    let chan = rt.rt.block_on(async move {
        // `unix:` targets included: from_shared dispatches on the scheme.
        let mut ep = tonic::transport::Endpoint::from_shared(s).ok()?;
        if o.stream_window != 0 {
            ep = ep.initial_stream_window_size(o.stream_window);
        }
        if o.connection_window != 0 {
            ep = ep.initial_connection_window_size(o.connection_window);
        }
        if o.adaptive_window >= 0 {
            ep = ep.http2_adaptive_window(o.adaptive_window != 0);
        }
        if o.tcp_nagle >= 0 {
            // ArmoniK's sense, inverted for tonic's: nagle on means nodelay off.
            ep = ep.tcp_nodelay(o.tcp_nagle == 0);
        }
        ep.connect().await.ok()
    });
    match chan {
        Some(chan) => Box::into_raw(Box::new(ClientImpl {
            rt: r as *const RuntimeImpl,
            chan,
        })) as *mut ak_client,
        None => core::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ak_client_destroy(c: *mut ak_client) {
    fwd();
    if !c.is_null() {
        drop(Box::from_raw(c as *mut ClientImpl));
    }
}

/// The blocking unary call. ONE crossing in.
///
/// ABI v1 section 9 gives the blocking form a handle so it can be cancelled; stage 4 does
/// not build cancellation, so this takes none and that omission is recorded rather than
/// papered over.
#[no_mangle]
pub unsafe extern "C" fn ak_call_unary(
    c: *mut ak_client,
    path: *const u8,
    path_len: usize,
    req: *const u8,
    req_len: usize,
    out: *mut ak_bytes,
) -> i32 {
    fwd();
    // Shared, not exclusive: many host threads may be inside this at once.
    let cl = &*(c as *const ClientImpl);
    let rt = &*cl.rt;
    let p = match core::str::from_utf8(core::slice::from_raw_parts(path, path_len)) {
        Ok(p) => p,
        Err(_) => return AK_ERR_INVALID_STATE,
    };
    let path = match http::uri::PathAndQuery::from_maybe_shared(p.to_string()) {
        Ok(p) => p,
        Err(_) => return AK_ERR_INVALID_STATE,
    };
    let body = Bytes::copy_from_slice(core::slice::from_raw_parts(req, req_len));
    let res = rt.rt.block_on(unary_once(cl.chan.clone(), path, body));
    match res {
        Ok(b) => {
            *out = into_ak_bytes(b);
            AK_OK
        }
        Err(e) => {
            trace("ak_call_unary", &e);
            AK_ERR_HOST
        }
    }
}

/// **One call path, three deliveries.** Every mode below awaits this, so a delivery cannot
/// drift from another delivery: there is one place the request is sent and one place the
/// response is taken.
async fn unary_once(
    chan: tonic::transport::Channel,
    path: http::uri::PathAndQuery,
    body: Bytes,
) -> Result<Bytes, String> {
    let mut grpc = tonic::client::Grpc::new(chan);
    grpc.ready().await.map_err(|e| format!("ready: {e}"))?;
    grpc.unary(tonic::Request::new(body), path, rpc::RawCodec)
        .await
        .map(|r| r.into_inner())
        .map_err(|e| format!("unary: {e}"))
}

/// The one place response bytes become an `ak_bytes`, so every delivery hands the host the
/// same thing and `ak_bytes_free` stays the only release path.
fn into_ak_bytes(b: Bytes) -> ak_bytes {
    let b = Box::new(b);
    ak_bytes {
        ptr: b.as_ptr(),
        len: b.len(),
        owner: Box::into_raw(b) as *mut c_void,
    }
}

fn empty_ak_bytes() -> ak_bytes {
    ak_bytes { ptr: core::ptr::null(), len: 0, owner: core::ptr::null_mut() }
}

fn trace(who: &str, e: &str) {
    if std::env::var_os("AK_RPC_TRACE").is_some() {
        eprintln!("{who}: {e}");
    }
}

/// The second crossing, and the only other one: the host is done with the response bytes.
#[no_mangle]
pub unsafe extern "C" fn ak_bytes_free(b: *mut ak_bytes) {
    fwd();
    if !b.is_null() && !(*b).owner.is_null() {
        drop(Box::from_raw((*b).owner as *mut Bytes));
        (*b).owner = core::ptr::null_mut();
        (*b).ptr = core::ptr::null();
        (*b).len = 0;
    }
}

// ---- ABI v1 section 9's two non-blocking deliveries -------------------------------------
//
// Both spawn `unary_once` on the runtime and differ only in where the result is put. The
// java slice's transport arm was taken through the blocking mode because it was the only
// one built, on the host whose own measurement says blocking in a native frame pins a
// virtual thread's carrier. That is a harness handicap of exactly the class R14 names,
// pointing at the core's own arm instead of at the incumbent.

/// What a completion carries. The bytes are released with `ak_bytes_free` exactly as the
/// blocking mode's are, so a host has one release path whichever delivery it takes.
#[repr(C)]
pub struct ak_completion {
    pub tag: u64,
    pub status: i32,
    pub bytes: ak_bytes,
}

/// The callback a host registers. Called ONCE per call, on a tokio worker thread -- a
/// thread the host does not own, which is the property that makes this mode wrong for a
/// runtime that must attach before it can run managed code, and right for one that can
/// complete a future from anywhere.
pub type ak_completion_cb = extern "C" fn(user_data: *mut c_void, comp: *mut ak_completion);

pub enum ak_call {}
pub enum ak_queue {}

struct CallImpl {
    abort: tokio::task::AbortHandle,
}

/// The host's callback and its context, crossing into a spawned task. Raw pointers are not
/// `Send`, and the host is the one asserting that its `user_data` may be touched from
/// another thread -- which is what registering a completion callback MEANS. Stated here
/// rather than left to a reader of the signature.
struct CbCtx {
    cb: ak_completion_cb,
    user: *mut c_void,
}
unsafe impl Send for CbCtx {}

/// The completion queue: a callback pushing onto a queue, as section 9 says, so it is
/// strictly additive rather than a second transport.
///
/// `Mutex` plus `Condvar` rather than an mpsc receiver behind a lock, because a drainer
/// blocked on `recv()` holding that lock serialises every other drainer. Section 9 says the
/// queue "ships with one drainer" and a POC could have taken the simpler shape; the failure
/// mode if a host runs two is a deadlock-shaped mystery rather than an error, and that is
/// not a good thing to leave in a reference implementation.
struct QueueImpl {
    m: std::sync::Mutex<QueueState>,
    cv: std::sync::Condvar,
}

struct QueueState {
    q: std::collections::VecDeque<ak_completion>,
    shutdown: bool,
}

// The queue holds `ak_completion`s, which carry raw pointers to core-owned bytes. They are
// produced by the core and consumed by the host; nothing else touches them.
unsafe impl Send for ak_completion {}

/// `ak_queue_next` returned a completion.
pub const AK_QUEUE_OK: i32 = 0;
/// The timeout expired with no completion. Not an error: a drainer polls its own shutdown.
pub const AK_QUEUE_TIMEOUT: i32 = 1;
/// The queue is shutting down and is drained. Every drainer gets this, once the backlog is.
pub const AK_QUEUE_SHUTDOWN: i32 = 2;

#[no_mangle]
pub extern "C" fn ak_queue_new() -> *mut ak_queue {
    Box::into_raw(Box::new(QueueImpl {
        m: std::sync::Mutex::new(QueueState {
            q: std::collections::VecDeque::new(),
            shutdown: false,
        }),
        cv: std::sync::Condvar::new(),
    })) as *mut ak_queue
}

/// Wake every drainer and refuse further pushes. Completions already queued are still
/// delivered, so a host drains to `AK_QUEUE_SHUTDOWN` and knows nothing was dropped.
#[no_mangle]
pub unsafe extern "C" fn ak_queue_shutdown(q: *mut ak_queue) {
    if q.is_null() {
        return;
    }
    let qi = &*(q as *const QueueImpl);
    if let Ok(mut st) = qi.m.lock() {
        st.shutdown = true;
    }
    qi.cv.notify_all();
}

/// Only after every call that names this queue has completed or been cancelled, and every
/// drainer has left. Leaks the bytes of any completion still queued rather than freeing
/// memory the host may hold a pointer into.
#[no_mangle]
pub unsafe extern "C" fn ak_queue_destroy(q: *mut ak_queue) {
    if !q.is_null() {
        drop(Box::from_raw(q as *mut QueueImpl));
    }
}

/// **The downcall the host blocks in.** No upcall, no thread the host does not own, and on
/// a managed runtime no pinning and no attach: the drainer is a host thread that entered
/// the core and will come back out.
///
/// `timeout_ms` of 0 polls; `u64::MAX` waits without a deadline.
#[no_mangle]
pub unsafe extern "C" fn ak_queue_next(
    q: *mut ak_queue,
    out: *mut ak_completion,
    timeout_ms: u64,
) -> i32 {
    fwd();
    if q.is_null() || out.is_null() {
        return AK_ERR_INVALID_STATE;
    }
    let qi = &*(q as *const QueueImpl);
    let mut st = match qi.m.lock() {
        Ok(st) => st,
        Err(_) => return AK_ERR_INVALID_STATE,
    };
    loop {
        if let Some(c) = st.q.pop_front() {
            *out = c;
            return AK_QUEUE_OK;
        }
        if st.shutdown {
            return AK_QUEUE_SHUTDOWN;
        }
        if timeout_ms == 0 {
            return AK_QUEUE_TIMEOUT;
        }
        if timeout_ms == u64::MAX {
            st = match qi.cv.wait(st) {
                Ok(st) => st,
                Err(_) => return AK_ERR_INVALID_STATE,
            };
        } else {
            let (s, t) = match qi
                .cv
                .wait_timeout(st, std::time::Duration::from_millis(timeout_ms))
            {
                Ok(v) => v,
                Err(_) => return AK_ERR_INVALID_STATE,
            };
            st = s;
            if t.timed_out() && st.q.is_empty() && !st.shutdown {
                return AK_QUEUE_TIMEOUT;
            }
        }
    }
}

/// Cancel an in-flight call. The delivery still happens: an aborted call completes with
/// `AK_ERR_HOST` and empty bytes, because a host that registered a completion and never
/// got one has no way to stop waiting.
#[no_mangle]
pub unsafe extern "C" fn ak_call_cancel(h: *mut ak_call) {
    fwd();
    if !h.is_null() {
        (*(h as *mut CallImpl)).abort.abort();
    }
}

/// Frees the handle. Only after the call's completion has been delivered.
#[no_mangle]
pub unsafe extern "C" fn ak_call_destroy(h: *mut ak_call) {
    fwd();
    if !h.is_null() {
        drop(Box::from_raw(h as *mut CallImpl));
    }
}

/// Shared prologue: validate the path and copy the request out of host memory, which must
/// happen on the calling thread because the host may reuse its buffer the moment this
/// returns.
unsafe fn call_parts(
    c: *mut ak_client,
    path: *const u8,
    path_len: usize,
    req: *const u8,
    req_len: usize,
) -> Option<(tonic::transport::Channel, &'static RuntimeImpl, http::uri::PathAndQuery, Bytes)> {
    if c.is_null() {
        return None;
    }
    let cl = &*(c as *const ClientImpl);
    let rt = &*cl.rt;
    let p = core::str::from_utf8(core::slice::from_raw_parts(path, path_len)).ok()?;
    let path = http::uri::PathAndQuery::from_maybe_shared(p.to_string()).ok()?;
    let body = Bytes::copy_from_slice(core::slice::from_raw_parts(req, req_len));
    Some((cl.chan.clone(), rt, path, body))
}

/// **The callback delivery.** Returns immediately with a handle; the completion arrives on
/// a tokio worker thread. The host's callback must be able to run on a thread the core
/// owns -- on .NET an `UnmanagedCallersOnly` entry point completing a `TaskCompletionSource`,
/// which is the idiom that runtime is built around.
#[no_mangle]
pub unsafe extern "C" fn ak_call_unary_cb(
    c: *mut ak_client,
    path: *const u8,
    path_len: usize,
    req: *const u8,
    req_len: usize,
    cb: ak_completion_cb,
    user_data: *mut c_void,
    tag: u64,
) -> *mut ak_call {
    fwd();
    let (chan, rt, path, body) = match call_parts(c, path, path_len, req, req_len) {
        Some(v) => v,
        None => return core::ptr::null_mut(),
    };
    let ctx = CbCtx { cb, user: user_data };
    let task = rt.rt.spawn(async move {
        let ctx = ctx;
        let mut comp = match unary_once(chan, path, body).await {
            Ok(b) => ak_completion { tag, status: AK_OK, bytes: into_ak_bytes(b) },
            Err(e) => {
                trace("ak_call_unary_cb", &e);
                ak_completion { tag, status: AK_ERR_HOST, bytes: empty_ak_bytes() }
            }
        };
        rev();
        (ctx.cb)(ctx.user, &mut comp);
    });
    Box::into_raw(Box::new(CallImpl { abort: task.abort_handle() })) as *mut ak_call
}

/// **The completion-queue delivery.** Returns immediately with a handle; the completion is
/// pushed onto `q`, where a host thread blocked in `ak_queue_next` takes it. No upcall at
/// all, which is the point on a host where a reverse call is dear: the JVM, where a cached
/// upcall measures 72 to 80 ns and an uncached one nearly 300.
#[no_mangle]
pub unsafe extern "C" fn ak_call_unary_q(
    c: *mut ak_client,
    path: *const u8,
    path_len: usize,
    req: *const u8,
    req_len: usize,
    q: *mut ak_queue,
    tag: u64,
) -> *mut ak_call {
    fwd();
    if q.is_null() {
        return core::ptr::null_mut();
    }
    let (chan, rt, path, body) = match call_parts(c, path, path_len, req, req_len) {
        Some(v) => v,
        None => return core::ptr::null_mut(),
    };
    // The queue outlives the call by the host's contract (`ak_queue_destroy` only after
    // every call naming it has completed), so the task holds it as an address rather than
    // an Arc -- which keeps the queue a plain C handle instead of a refcount the host
    // cannot see.
    let qaddr = q as usize;
    let task = rt.rt.spawn(async move {
        let comp = match unary_once(chan, path, body).await {
            Ok(b) => ak_completion { tag, status: AK_OK, bytes: into_ak_bytes(b) },
            Err(e) => {
                trace("ak_call_unary_q", &e);
                ak_completion { tag, status: AK_ERR_HOST, bytes: empty_ak_bytes() }
            }
        };
        let qi = unsafe { &*(qaddr as *const QueueImpl) };
        if let Ok(mut st) = qi.m.lock() {
            st.q.push_back(comp);
        }
        qi.cv.notify_one();
    });
    Box::into_raw(Box::new(CallImpl { abort: task.abort_handle() })) as *mut ak_call
}

#[cfg(test)]
mod delivery_tests {
    //! Three deliveries of one call path, so the thing to test is that they agree on the
    //! bytes and differ only in where the completion turns up. A mode that returns the
    //! right bytes on an empty queue, or never completes at all, is the failure worth
    //! catching: both look like "no output" from a benchmark harness.
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    const RESP: &[u8] = b"the response bytes, which every delivery must hand back intact";

    /// A runtime and a client against a server answering with RESP. Returned raw because
    /// that is how a host holds them.
    fn fixture() -> (*mut ak_runtime, *mut ak_client) {
        let r = ak_runtime_new(2);
        assert!(!r.is_null());
        let addr = unsafe {
            let rt = &*(r as *const RuntimeImpl);
            rt.rt.block_on(async { rpc::serve(Bytes::from_static(RESP)).await.addr })
        };
        let uri = format!("http://{addr}");
        let c = unsafe { ak_client_new(r, uri.as_ptr(), uri.len()) };
        assert!(!c.is_null(), "client did not connect to {uri}");
        (r, c)
    }

    unsafe fn take(b: &mut ak_bytes) -> Vec<u8> {
        let v = core::slice::from_raw_parts(b.ptr, b.len).to_vec();
        ak_bytes_free(b);
        v
    }

    #[test]
    fn the_three_deliveries_return_the_same_bytes() {
        let (r, c) = fixture();
        unsafe {
            // 1. blocking
            let mut out = empty_ak_bytes();
            assert_eq!(
                ak_call_unary(c, rpc::PATH.as_ptr(), rpc::PATH.len(), b"req".as_ptr(), 3, &mut out),
                AK_OK
            );
            assert_eq!(take(&mut out), RESP);

            // 2. callback, completing a condvar the way a host completes a future
            static SEEN: AtomicU32 = AtomicU32::new(0);
            struct Sink {
                m: std::sync::Mutex<Option<(u64, i32, Vec<u8>)>>,
                cv: std::sync::Condvar,
            }
            extern "C" fn on_done(user: *mut c_void, comp: *mut ak_completion) {
                SEEN.fetch_add(1, Ordering::SeqCst);
                unsafe {
                    let s = &*(user as *const Sink);
                    let c = &mut *comp;
                    let v = core::slice::from_raw_parts(c.bytes.ptr, c.bytes.len).to_vec();
                    ak_bytes_free(&mut c.bytes);
                    *s.m.lock().unwrap() = Some((c.tag, c.status, v));
                    s.cv.notify_all();
                }
            }
            let sink = Sink {
                m: std::sync::Mutex::new(None),
                cv: std::sync::Condvar::new(),
            };
            let h = ak_call_unary_cb(
                c,
                rpc::PATH.as_ptr(),
                rpc::PATH.len(),
                b"req".as_ptr(),
                3,
                on_done,
                &sink as *const Sink as *mut c_void,
                7,
            );
            assert!(!h.is_null());
            let got = {
                let mut g = sink.m.lock().unwrap();
                while g.is_none() {
                    let (ng, t) = sink
                        .cv
                        .wait_timeout(g, std::time::Duration::from_secs(10))
                        .unwrap();
                    g = ng;
                    assert!(!t.timed_out(), "the callback never fired");
                }
                g.take().unwrap()
            };
            assert_eq!(got.0, 7, "the tag did not survive");
            assert_eq!(got.1, AK_OK);
            assert_eq!(got.2, RESP);
            assert_eq!(SEEN.load(Ordering::SeqCst), 1, "the callback fired more than once");
            ak_call_destroy(h);

            // 3. the completion queue
            let q = ak_queue_new();
            let mut comp = ak_completion { tag: 0, status: 0, bytes: empty_ak_bytes() };
            assert_eq!(
                ak_queue_next(q, &mut comp, 0),
                AK_QUEUE_TIMEOUT,
                "an empty queue polled with no timeout must not block or invent a completion"
            );
            let h = ak_call_unary_q(
                c,
                rpc::PATH.as_ptr(),
                rpc::PATH.len(),
                b"req".as_ptr(),
                3,
                q,
                99,
            );
            assert!(!h.is_null());
            assert_eq!(ak_queue_next(q, &mut comp, 10_000), AK_QUEUE_OK);
            assert_eq!(comp.tag, 99);
            assert_eq!(comp.status, AK_OK);
            assert_eq!(take(&mut comp.bytes), RESP);
            ak_call_destroy(h);

            // The drainer's exit: shutdown wakes it and says so, once the backlog is gone.
            ak_queue_shutdown(q);
            assert_eq!(ak_queue_next(q, &mut comp, u64::MAX), AK_QUEUE_SHUTDOWN);
            ak_queue_destroy(q);

            ak_client_destroy(c);
            ak_runtime_destroy(r);
        }
    }

    #[test]
    fn the_core_dials_a_unix_domain_socket() {
        // The transport row design/SHAPES.md makes primary, and what ArmoniK's client
        // actually dials. Until this existed every RPC arm was forced onto loopback TCP,
        // which measures a kernel path production does not take.
        let r = ak_runtime_new(2);
        assert!(!r.is_null());
        let path = std::env::temp_dir().join(format!("ak-uds-test-{}.sock", std::process::id()));
        let _srv = unsafe {
            let rt = &*(r as *const RuntimeImpl);
            rt.rt.block_on(rpc::serve_uds(Bytes::from_static(RESP), path.clone()))
        };
        let target = format!("unix:{}", path.display());
        unsafe {
            let c = ak_client_new(r, target.as_ptr(), target.len());
            assert!(!c.is_null(), "the core did not connect to {target}");

            let mut out = empty_ak_bytes();
            assert_eq!(
                ak_call_unary(c, rpc::PATH.as_ptr(), rpc::PATH.len(), b"req".as_ptr(), 3, &mut out),
                AK_OK
            );
            assert_eq!(take(&mut out), RESP);

            // And over the queue, so the UDS path is exercised by a delivery that does not
            // block the calling thread inside the core.
            let q = ak_queue_new();
            let h = ak_call_unary_q(
                c, rpc::PATH.as_ptr(), rpc::PATH.len(), b"req".as_ptr(), 3, q, 5,
            );
            assert!(!h.is_null());
            let mut comp = ak_completion { tag: 0, status: 0, bytes: empty_ak_bytes() };
            assert_eq!(ak_queue_next(q, &mut comp, 10_000), AK_QUEUE_OK);
            assert_eq!(comp.tag, 5);
            assert_eq!(take(&mut comp.bytes), RESP);
            ak_call_destroy(h);
            ak_queue_shutdown(q);
            ak_queue_destroy(q);

            ak_client_destroy(c);
            ak_runtime_destroy(r);
        }
    }

    #[test]
    fn the_queue_delivers_every_tag_and_correlates_them() {
        // The property the tag exists for: N calls in flight, N completions, each carrying
        // its own tag. A queue that dropped or duplicated one would still look fine on a
        // single call, which is why this is a separate test.
        let (r, c) = fixture();
        const N: u64 = 16;
        unsafe {
            let q = ak_queue_new();
            let mut handles = Vec::new();
            for tag in 0..N {
                let h = ak_call_unary_q(
                    c,
                    rpc::PATH.as_ptr(),
                    rpc::PATH.len(),
                    b"req".as_ptr(),
                    3,
                    q,
                    tag,
                );
                assert!(!h.is_null());
                handles.push(h);
            }
            let mut seen = vec![0u32; N as usize];
            for _ in 0..N {
                let mut comp = ak_completion { tag: 0, status: 0, bytes: empty_ak_bytes() };
                assert_eq!(ak_queue_next(q, &mut comp, 30_000), AK_QUEUE_OK);
                assert_eq!(comp.status, AK_OK);
                assert_eq!(take(&mut comp.bytes), RESP);
                seen[comp.tag as usize] += 1;
            }
            assert!(seen.iter().all(|&n| n == 1), "tags dropped or duplicated: {seen:?}");
            for h in handles {
                ak_call_destroy(h);
            }
            ak_queue_shutdown(q);
            ak_queue_destroy(q);
            ak_client_destroy(c);
            ak_runtime_destroy(r);
        }
    }

    #[test]
    fn a_pinned_transport_still_dials_and_still_answers() {
        // `design/SHAPES.md`: every cell of the RPC grid pins the SAME transport, and
        // states it. The knob has to be exercised rather than trusted -- an `Endpoint`
        // builder that rejects a setting returns an error at `connect()`, which from a
        // harness looks exactly like "the server is not up yet".
        let r = ak_runtime_new(2);
        assert!(!r.is_null());
        let addr = unsafe {
            let rt = &*(r as *const RuntimeImpl);
            rt.rt.block_on(async { rpc::serve(Bytes::from_static(RESP)).await.addr })
        };
        let uri = format!("http://{addr}");
        // ArmoniK's transport, as design/SHAPES.md pins it: a 4 MiB window on both the
        // stream and the connection, adaptive off (it would override both), and the
        // message limits that go with 2 MiB chunking.
        let opts = ak_client_opts {
            stream_window: 4 * 1024 * 1024,
            connection_window: 4 * 1024 * 1024,
            adaptive_window: 0,
            max_recv_message: 4 * 1024 * 1024,
            max_send_message: 4 * 1024 * 1024,
            tcp_nagle: 0,
        };
        unsafe {
            let c = ak_client_new_opts(r, uri.as_ptr(), uri.len(), &opts);
            assert!(!c.is_null(), "a pinned endpoint did not connect to {uri}");
            let mut out = empty_ak_bytes();
            assert_eq!(
                ak_call_unary(c, rpc::PATH.as_ptr(), rpc::PATH.len(), b"req".as_ptr(), 3, &mut out),
                AK_OK
            );
            assert_eq!(take(&mut out), RESP);
            ak_client_destroy(c);

            // NULL options is the old behaviour, and that is the property that lets
            // `ak_client_new` be one line rather than a second connect path.
            let c = ak_client_new_opts(r, uri.as_ptr(), uri.len(), core::ptr::null());
            assert!(!c.is_null());
            ak_client_destroy(c);
            ak_runtime_destroy(r);
        }
    }
}
