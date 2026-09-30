//! CAMPAIGN.md 4.2: the RPC grid's client cells (requirement 12), shared by `benches/rpc_suite.rs` (the
//! timed grid) and by the counting build's per-call counts (`crossings`, requirement 19).
//!
//!   A  prost (the incumbent)                    tonic, the idiomatic async call
//!   B  prost                                    the core's transport, BLOCKING (`ak_call_unary`)
//!   C  core-ffi (the generated binding, C ABI)  the core's transport, blocking
//!   D  core-ffi                                 tonic, async (raw-bytes codec)
//!   E  core-native (host-gen)                   the core's transport, blocking
//!   F  core-native (host-gen)                   tonic, async (raw-bytes codec)
//! C, D, E and F carry the unknown-field mode as a suffix (`-retain`, `-drop` in the full
//! build, `-nounk` in the no-unknown build).
//!
//! Optimisation T1, option 3 (the owner: optional, measured beside the reference): a cell
//! name with `f` after its letter (`Bf`, `Cf-drop`, `Df-retain`, ...) is the same cell on the
//! FRAMED send path, `rpc::unary_framed` (tonic's Channel; the request message sent as the
//! 5-byte prefix and the caller's Bytes, two body frames, no copy into tonic's buffer). B, C
//! and E set the core's client with `ak_client_set_framed(client, framed)` at `Conn::open` (the
//! core's default is the framed path since 2026-09-28, each message ONE frame); D
//! and F call `rpc::unary_framed` in the harness. The response is taken as tonic takes it.
//! A has no framed twin (prost encodes into tonic's buffer: it has no copy to remove).
//!
//! Delivery (requirement 16 as amended 2026-09-28, owner): the REFERENCE core cells are the
//! callback cells `B-cb`, `C-cb-*`, `E-cb-*` (and framed twins): the core's callback delivery,
//! each completion sent into a tokio oneshot awaited by one of k async tasks on the cell's own
//! runtime (Rust's idiomatic core delivery); the blocking B, C and E cells stay as the
//! labelled row: the core's blocking call from
//! k host threads (a pool, created before the warm-up and reused, R-H2); A, D and F use the
//! host stack's idiomatic call, which for packages/rust is tonic's async client: k tokio
//! tasks on a multi-thread runtime of TOKIO_WORKERS workers, each making its calls back to
//! back. Directions (requirement 14): `a` = Fetch and decode, `a+read` = Fetch, decode and
//! read every field, `b` = encode and Push. Every call is checked (requirement 18).
//!
//! Connections (requirement 13 as amended, R-H33): ONE channel per cell per launch, opened
//! by `Conn::open` and shared by every direction and in-flight count of the cell; the
//! socket is a Unix domain socket (requirement 17, R-H28).

use crate::generated::roots::R_ListTasksDetailedResponse as M2;
use crate::generated::roots::R_UploadResultDataMessage as M5;
use crate::Ops;
use ak_abi::*;
use core::ffi::c_void;
use bytes::Bytes;
use harness::arms_m2 as m2;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

pub const FETCH: &str = "/armonik.ffi.campaign.v1.Grid/Fetch";
pub const PUSH: &str = "/armonik.ffi.campaign.v1.Grid/Push";
/// U1-unary (the owner, labelled extra direction `c`): a result upload, M5 (P5.3, P5.4) as
/// the request, an empty response; the server decodes the request with prost.
pub const UPLOAD: &str = "/armonik.ffi.campaign.v1.Grid/Upload";
/// Direction `c`'s payloads and in-flight counts (k = 1 and 8 only, the owner's budget).
pub const C_PAYLOADS: &[&str] = &["P5.3", "P5.4"];
pub const C_INFLIGHT: &[usize] = &[1, 8];
/// U2-stream (the owner, labelled extra direction `d`): CAMPAIGN req 14's streamed upload
/// in 2 MiB chunks (FIX-PLAN D5), ArmoniK's UploadResultData shape: M5 per chunk, the ids on
/// the first message only, the chunk as data_chunk. The server (a tonic client-streaming
/// handler) decodes each message with prost and answers the data byte count (u64 LE);
/// STREAM_CHECK also answers the SHA-256 of every message's bytes as received (the gate's
/// byte check, bin upload_check).
pub const STREAM: &str = "/armonik.ffi.campaign.v1.Grid/UploadStream";
pub const STREAM_CHECK: &str = "/armonik.ffi.campaign.v1.Grid/UploadStreamCheck";
pub const CHUNK: usize = 2 * 1024 * 1024;
/// (label, chunks): 4 MiB in 2 chunks, 16 MiB in 8.
pub const D_PAYLOADS: &[(&str, usize)] = &[("4MiB", 2), ("16MiB", 8)];
pub const D_INFLIGHT: &[usize] = &[1, 8];
pub const WIN: u32 = 4 * 1024 * 1024;
/// Worker threads of every tokio runtime the client makes (cells A, D, F), and of the
/// core's runtime (`ak_runtime_new`, cells B, C, E). Recorded in every header (req 4).
pub const TOKIO_WORKERS: usize = 2;
pub const CORE_WORKERS: u32 = 2;

/// The runtime probe (container instrumentation, 2026-09-28): the host runtime's workers
/// (`AK_HOST_WORKERS`: N >= 1 workers of a multi-thread runtime, `ct` a current-thread
/// runtime) and the core runtime's (`AK_CORE_WORKERS`, passed to `ak_runtime_new`), read
/// once; unset = the defaults above. Recorded in every header (req 4).
pub fn host_workers() -> Option<usize> {
    static W: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    *W.get_or_init(|| match std::env::var("AK_HOST_WORKERS").ok().as_deref() {
        None | Some("") => Some(TOKIO_WORKERS),
        Some("ct") => None,
        Some(v) => Some(v.parse().ok().filter(|n| *n >= 1).expect("AK_HOST_WORKERS: N >= 1 or ct")),
    })
}
pub fn core_workers() -> u32 {
    static W: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *W.get_or_init(|| match std::env::var("AK_CORE_WORKERS").ok().as_deref() {
        None | Some("") => CORE_WORKERS,
        Some(v) => v.parse().ok().filter(|n| *n >= 1).expect("AK_CORE_WORKERS: N >= 1"),
    })
}
/// The host runtime's shape as printed in headers: `mt2`, `mt1`, `ct`.
pub fn host_rt_label() -> String {
    host_workers().map_or("ct".into(), |n| format!("mt{n}"))
}
/// A cell's own host runtime (A, D, F, the -cb cells), threads named `cell-rt`.
pub fn host_runtime() -> Arc<tokio::runtime::Runtime> {
    let mut b = match host_workers() {
        Some(n) => {
            let mut b = tokio::runtime::Builder::new_multi_thread();
            b.worker_threads(n);
            b
        }
        None => tokio::runtime::Builder::new_current_thread(),
    };
    Arc::new(b.thread_name("cell-rt").enable_all().build().unwrap())
}

pub type Fut = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// One call of a cell at a given in-flight slot.
#[derive(Clone)]
pub enum Call {
    Blocking(Arc<dyn Fn(usize) -> Result<(), String> + Send + Sync>),
    Async(Arc<tokio::runtime::Runtime>, Arc<dyn Fn(usize) -> Fut + Send + Sync>),
}

impl Call {
    /// One call, from the calling thread (warm-ups and the counting build).
    pub fn once(&self, slot: usize) -> Result<(), String> {
        match self {
            Call::Blocking(f) => f(slot),
            Call::Async(rt, f) => rt.block_on(f(slot)),
        }
    }
}

// ---- cell A's codec: tonic-prost's encoder and decoder bodies, with the request held in an
// Arc (tonic takes the request by value; cloning a P2.2 graph per call would be work
// production does not do) and the response's wire length recorded for requirement 18.
struct PCodec<Q, S> {
    len: Arc<AtomicU64>,
    _p: std::marker::PhantomData<(Q, S)>,
}
struct PEnc<Q>(std::marker::PhantomData<Q>);
struct PDec<S>(Arc<AtomicU64>, std::marker::PhantomData<S>);
impl<Q: prost::Message + Send + Sync + 'static> Encoder for PEnc<Q> {
    type Item = Arc<Q>;
    type Error = tonic::Status;
    fn encode(&mut self, item: Arc<Q>, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(dst).expect("Message only errors if not enough space");
        Ok(())
    }
}
impl<S: prost::Message + Default + Send + 'static> Decoder for PDec<S> {
    type Item = S;
    type Error = tonic::Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<S>, Self::Error> {
        use bytes::Buf;
        self.0.store(src.remaining() as u64, Ordering::Relaxed);
        S::decode(src).map(Some).map_err(|e| tonic::Status::internal(e.to_string()))
    }
}
impl<Q, S> Codec for PCodec<Q, S>
where
    Q: prost::Message + Send + Sync + 'static,
    S: prost::Message + Default + Send + 'static,
{
    type Encode = Arc<Q>;
    type Decode = S;
    type Encoder = PEnc<Q>;
    type Decoder = PDec<S>;
    fn encoder(&mut self) -> PEnc<Q> {
        PEnc(std::marker::PhantomData)
    }
    fn decoder(&mut self) -> PDec<S> {
        PDec(self.len.clone(), std::marker::PhantomData)
    }
}

pub struct CoreClient {
    rt: *mut ak_runtime,
    client: *mut ak_client,
    /// The host runtime a HOSTED core runtime hands its tasks to (patch p3-exec-slot); None
    /// for the core's own runtime.
    host: Option<Arc<tokio::runtime::Runtime>>,
}
unsafe impl Send for CoreClient {}
unsafe impl Sync for CoreClient {}
impl CoreClient {
    /// The raw client handle (the stream probe's split-timed cells).
    pub fn raw(&self) -> *mut ak_client {
        self.client
    }
    pub fn new(target: &str, pinned: bool) -> Self {
        // AK_CORE_HOSTED=1 (experiment p3-exec-slot, a patched core only): every core client
        // of the process on a hosted runtime whose tasks run on one shared host runtime, so
        // upload_check and rpc_semantics can be run against that mode.
        if std::env::var("AK_CORE_HOSTED").map_or(false, |v| v == "1") {
            static H: std::sync::OnceLock<Arc<tokio::runtime::Runtime>> = std::sync::OnceLock::new();
            return Self::new_hosted(target, pinned, H.get_or_init(host_runtime).clone());
        }
        let rt = unsafe { ak_runtime_new(core_workers()) };
        Self::with_runtime(rt, target, pinned, None)
    }
    /// A client on a HOSTED core runtime (patch p3-exec-slot: `ak_runtime_new_hosted`, found
    /// with dlsym; panics on a core without it): the core's tasks run on `host`, the core keeps
    /// one `ak-reactor` thread for its I/O and timer drivers.
    pub fn new_hosted(target: &str, pinned: bool, host: Arc<tokio::runtime::Runtime>) -> Self {
        let rt = unsafe { hosted::runtime(&host) };
        Self::with_runtime(rt, target, pinned, Some(host))
    }
    fn with_runtime(rt: *mut ak_runtime, target: &str, pinned: bool, host: Option<Arc<tokio::runtime::Runtime>>) -> Self {
        unsafe {
            let client = if pinned {
                let o = ak_client_opts {
                    stream_window: WIN,
                    connection_window: WIN,
                    adaptive_window: 0,
                    max_recv_message: 0,
                    max_send_message: 0,
                    tcp_nagle: 0,
                };
                ak_client_new_opts(rt, target.as_ptr(), target.len(), &o)
            } else {
                ak_client_new(rt, target.as_ptr(), target.len())
            };
            assert!(!client.is_null(), "core client for {target}");
            CoreClient { rt, client, host }
        }
    }
    /// Optimisation R2: one blocking call whose request is `enc`'s encoded output, moved
    /// into the core (`ak_call_unary_enc`), not copied; the response as `call`.
    pub fn call_enc<T>(&self, path: &str, enc: *mut ak_enc_ctx, f: impl FnOnce(&[u8]) -> Result<T, String>) -> Result<T, String> {
        unsafe {
            let mut out = ak_bytes::default();
            let mut gs = -1i32;
            let rc = ak_call_unary_enc(self.client, path.as_ptr(), path.len(), enc, &mut out, &mut gs);
            if rc != AK_OK {
                return Err(format!("ak_call_unary_enc rc {rc}, grpc status {gs}"));
            }
            let r = f(if out.len == 0 { &[] } else { std::slice::from_raw_parts(out.ptr, out.len) });
            ak_bytes_free(&mut out);
            r
        }
    }
    /// One blocking call; the response bytes are handed to `f` and freed after.
    pub fn call<T>(&self, path: &str, req: &[u8], f: impl FnOnce(&[u8]) -> Result<T, String>) -> Result<T, String> {
        unsafe {
            let mut out = ak_bytes::default();
            let mut gs = -1i32;
            let rc = ak_call_unary(self.client, path.as_ptr(), path.len(), req.as_ptr(), req.len(), &mut out, &mut gs);
            if rc != AK_OK {
                return Err(format!("ak_call_unary rc {rc}, grpc status {gs}"));
            }
            let r = f(if out.len == 0 { &[] } else { std::slice::from_raw_parts(out.ptr, out.len) });
            ak_bytes_free(&mut out);
            r
        }
    }
}
/// The callback bridge (CAMPAIGN req 16 as amended: Rust's core delivery is the callback
/// bridged to async with a oneshot). The core calls `on_complete` ONCE per completion, on
/// one of its threads; it sends the completion into the oneshot the awaiting task holds.
struct Comp(ak_completion);
unsafe impl Send for Comp {}
type CompTx = tokio::sync::oneshot::Sender<Comp>;

extern "C" fn on_complete(user: *mut c_void, comp: *mut ak_completion) {
    let tx = unsafe { Box::from_raw(user as *mut CompTx) };
    if let Err(Comp(c)) = tx.send(Comp(unsafe { *comp })) {
        drop(CbResp(c)); // the task is gone: free what the completion carries
    }
}

/// A oneshot and its sender as the callback's `user_data` (an address, so the future that
/// holds it stays `Send`).
fn bridge() -> (usize, tokio::sync::oneshot::Receiver<Comp>) {
    let (tx, rx) = tokio::sync::oneshot::channel::<Comp>();
    (Box::into_raw(Box::new(tx)) as usize, rx)
}

/// The entry refused (no completion follows): reclaim the sender.
unsafe fn unbridge(ud: usize) {
    drop(Box::from_raw(ud as *mut CompTx));
}

/// A delivered completion; the bytes it owns (a response) freed on drop with
/// `ak_bytes_free`, as the blocking cells free theirs. A send completion owns none.
pub struct CbResp(ak_completion);
unsafe impl Send for CbResp {}
impl CbResp {
    pub fn bytes(&self) -> &[u8] {
        if self.0.bytes.len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(self.0.bytes.ptr, self.0.bytes.len) } }
    }
}
impl Drop for CbResp {
    fn drop(&mut self) {
        if !self.0.bytes.owner.is_null() {
            unsafe { ak_bytes_free(&mut self.0.bytes) };
        }
    }
}

/// Await the completion; a status other than AK_OK is the call's error.
async fn landed(what: &'static str, rx: tokio::sync::oneshot::Receiver<Comp>) -> Result<CbResp, String> {
    let r = CbResp(rx.await.map_err(|_| format!("{what}: no completion delivered"))?.0);
    if r.0.status != AK_OK {
        return Err(format!("{what} status {}, grpc status {}", r.0.status, r.0.grpc_status));
    }
    Ok(r)
}

impl CoreClient {
    /// A unary call through `ak_call_unary_cb` (the request copied by the core before it
    /// returns), awaited on a oneshot; the handle destroyed after the completion.
    pub fn call_cb(&self, path: &'static str, req: &[u8]) -> impl Future<Output = Result<CbResp, String>> + Send + 'static {
        let (ud, rx) = bridge();
        let h = unsafe { ak_call_unary_cb(self.client, path.as_ptr(), path.len(), req.as_ptr(), req.len(), on_complete, ud as *mut c_void, 0) } as usize;
        Self::cb_tail("ak_call_unary_cb", h, ud, rx)
    }
    /// `call_cb` whose request is `enc`'s output, MOVED (`ak_call_unary_enc_cb`, as the
    /// blocking C cell's ak_call_unary_enc).
    pub fn call_enc_cb(&self, path: &'static str, enc: *mut ak_enc_ctx) -> impl Future<Output = Result<CbResp, String>> + Send + 'static {
        let (ud, rx) = bridge();
        let h = unsafe { ak_call_unary_enc_cb(self.client, path.as_ptr(), path.len(), enc, on_complete, ud as *mut c_void, 0) } as usize;
        Self::cb_tail("ak_call_unary_enc_cb", h, ud, rx)
    }
    fn cb_tail(what: &'static str, h: usize, ud: usize, rx: tokio::sync::oneshot::Receiver<Comp>) -> impl Future<Output = Result<CbResp, String>> + Send + 'static {
        async move {
            if h == 0 {
                unsafe { unbridge(ud) };
                return Err(format!("{what} returned NULL"));
            }
            let r = landed(what, rx).await;
            unsafe { ak_call_destroy(h as *mut ak_call) };
            r
        }
    }
}

/// Client streaming through the core's callback deliveries (the -cb cells of direction d):
/// open, per chunk one send completion awaited on its own oneshot (`send(j, handle, last,
/// user_data)` calls ak_call_send_cb or ak_call_send_enc_cb and returns its rc), then the
/// response through ak_call_recv_cb on another; free, destroy.
fn core_stream_cb(cc: Arc<CoreClient>, path: &'static str, n: usize,
                  mut send: impl FnMut(usize, *mut ak_call, i32, *mut c_void) -> i32 + Send + 'static,
                  verdict: impl FnOnce(&[u8]) -> Result<(), String> + Send + 'static) -> Fut {
    Box::pin(async move {
        let h = unsafe { ak_call_open(cc.client, path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null()) } as usize;
        if h == 0 {
            return Err("ak_call_open NULL".into());
        }
        let hp = move || h as *mut ak_call;
        let mut r = Ok(());
        for j in 0..n {
            let (ud, rx) = bridge();
            let rc = send(j, hp(), (j + 1 == n) as i32, ud as *mut c_void);
            if rc != AK_OK {
                unsafe { unbridge(ud) };
                r = Err(format!("ak_call_send_cb chunk {j} rc {rc}"));
                unsafe { ak_call_cancel(hp()) };
                break;
            }
            if let Err(e) = landed("ak_call_send_cb", rx).await {
                r = Err(format!("chunk {j}: {e}"));
                unsafe { ak_call_cancel(hp()) };
                break;
            }
        }
        let (ud, rx) = bridge();
        let rc = unsafe { ak_call_recv_cb(hp(), on_complete, ud as *mut c_void, 0) };
        let resp = if rc != AK_OK {
            unsafe { unbridge(ud) };
            Err(format!("ak_call_recv_cb rc {rc}"))
        } else {
            landed("ak_call_recv_cb", rx).await
        };
        if r.is_ok() {
            r = resp.and_then(|b| verdict(b.bytes()));
        }
        unsafe { ak_call_destroy(hp()) };
        r
    })
}

impl Drop for CoreClient {
    fn drop(&mut self) {
        unsafe {
            ak_client_destroy(self.client);
            // A hosted core runtime is left to the process's end: host tasks may still hold
            // its handle (the executor slot's destroy contract).
            if self.host.is_none() {
                ak_runtime_destroy(self.rt);
            }
        }
    }
}

/// A core runtime for the semantics test: `ak_runtime_new(workers)`, or with AK_CORE_HOSTED=1
/// a hosted one (patch p3-exec-slot) on one shared host runtime; true when hosted (then it is
/// never destroyed: the executor slot's destroy contract).
pub fn test_runtime(workers: u32) -> (*mut ak_runtime, bool) {
    if std::env::var("AK_CORE_HOSTED").map_or(false, |v| v == "1") {
        static H: std::sync::OnceLock<Arc<tokio::runtime::Runtime>> = std::sync::OnceLock::new();
        let h = H.get_or_init(host_runtime).clone();
        (unsafe { hosted::runtime(&h) }, true)
    } else {
        (unsafe { ak_runtime_new(workers) }, false)
    }
}

/// The host side of the executor slot (experiment p3-exec-slot): the core hands every task to
/// `spawn`, which spawns a future on the host runtime that polls it through `ak_task_poll`
/// with the host's own waker behind the slot's waker vtable. The entries are looked up with
/// dlsym so this crate builds and runs against a core without them.
pub mod hosted {
    use super::*;
    type NewHosted = unsafe extern "C" fn(u32, unsafe extern "C" fn(*mut c_void, *mut c_void), *mut c_void) -> *mut ak_runtime;
    type Poll = unsafe extern "C" fn(*mut c_void, *mut c_void, *const WakerVt) -> i32;
    type Free = unsafe extern "C" fn(*mut c_void);
    #[repr(C)]
    pub struct WakerVt {
        wake: unsafe extern "C" fn(*mut c_void),
        clone: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        drop: unsafe extern "C" fn(*mut c_void),
    }
    unsafe extern "C" fn w_wake(d: *mut c_void) {
        (*(d as *const std::task::Waker)).wake_by_ref();
    }
    unsafe extern "C" fn w_clone(d: *mut c_void) -> *mut c_void {
        Box::into_raw(Box::new((*(d as *const std::task::Waker)).clone())) as *mut c_void
    }
    unsafe extern "C" fn w_drop(d: *mut c_void) {
        drop(Box::from_raw(d as *mut std::task::Waker));
    }
    static VT: WakerVt = WakerVt { wake: w_wake, clone: w_clone, drop: w_drop };
    fn sym<T>(name: &[u8]) -> T {
        let p = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr() as *const libc::c_char) };
        assert!(!p.is_null(), "{} missing: the hosted cells need a core built with patch p3-exec-slot", String::from_utf8_lossy(&name[..name.len() - 1]));
        unsafe { std::mem::transmute_copy::<*mut c_void, T>(&p) }
    }
    struct Task {
        t: usize,
        poll: Poll,
        free: Free,
        done: bool,
    }
    impl Future for Task {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
            let w = cx.waker() as *const std::task::Waker as *mut c_void;
            if unsafe { (self.poll)(self.t as *mut c_void, w, &VT) } == 0 {
                return std::task::Poll::Pending;
            }
            self.done = true;
            std::task::Poll::Ready(())
        }
    }
    impl Drop for Task {
        fn drop(&mut self) {
            unsafe { (self.free)(self.t as *mut c_void) };
        }
    }
    struct Ctx {
        handle: tokio::runtime::Handle,
        poll: Poll,
        free: Free,
    }
    unsafe extern "C" fn spawn(user: *mut c_void, task: *mut c_void) {
        let c = &*(user as *const Ctx);
        c.handle.spawn(Task { t: task as usize, poll: c.poll, free: c.free, done: false });
    }
    /// A hosted core runtime whose tasks run on `host` (one `ak-reactor` driver thread).
    pub unsafe fn runtime(host: &tokio::runtime::Runtime) -> *mut ak_runtime {
        let new: NewHosted = sym(b"ak_runtime_new_hosted\0");
        let ctx = Box::leak(Box::new(Ctx { handle: host.handle().clone(), poll: sym(b"ak_task_poll\0"), free: sym(b"ak_task_free\0") }));
        let rt = new(1, spawn, ctx as *mut Ctx as *mut c_void);
        assert!(!rt.is_null(), "ak_runtime_new_hosted");
        rt
    }
}

pub fn tonic_channel(rt: &tokio::runtime::Runtime, target: &str, pinned: bool) -> tonic::transport::Channel {
    let t = target.to_string();
    rt.block_on(async move {
        let mut e = tonic::transport::Endpoint::from_shared(t).unwrap();
        if pinned {
            e = e
                .initial_stream_window_size(Some(WIN))
                .initial_connection_window_size(Some(WIN))
                .http2_adaptive_window(false);
        }
        e.connect().await.unwrap()
    })
}

/// A cell's connection for the whole launch: the core's client (B, C, E) or a tonic channel
/// on the cell's own runtime (A, D, F).
pub enum Conn {
    Core(Arc<CoreClient>),
    /// The callback cells (`-cb`, CAMPAIGN req 16 as amended): the core's client and the
    /// host runtime whose tasks await the completions (TOKIO_WORKERS workers, as A/D/F).
    CoreCb(Arc<CoreClient>, Arc<tokio::runtime::Runtime>),
    Tonic(Arc<tokio::runtime::Runtime>, tonic::transport::Channel),
}

impl Conn {
    pub fn open(cell: &str, target: &str, pinned: bool) -> Conn {
        match base(cell) {
            'B' | 'C' | 'E' => {
                let cc = CoreClient::new(target, pinned);
                // The send path set explicitly both ways: the core's DEFAULT is the framed path
                // since 2026-09-28 (owner), so the reference cells (B, C, E) switch it off and
                // the framed twins (Bf, Cf, Ef) keep it (the call is idempotent).
                let rc = unsafe { ak_client_set_framed(cc.client, framed(cell) as i32) };
                assert_eq!(rc, AK_OK, "ak_client_set_framed");
                if cb(cell) {
                    let rt = host_runtime();
                    return Conn::CoreCb(Arc::new(cc), rt);
                }
                Conn::Core(Arc::new(cc))
            }
            _ => {
                let rt = host_runtime();
                let ch = tonic_channel(&rt, target, pinned);
                Conn::Tonic(rt, ch)
            }
        }
    }
}

pub fn base(cell: &str) -> char {
    cell.chars().next().unwrap()
}

/// T1 option 3: the cell is on the framed send path (`Bf`, `Cf-drop`, ...).
pub fn framed(cell: &str) -> bool {
    cell.as_bytes().get(1) == Some(&b'f')
}

/// The cell's unknown-field mode (`retain`, `drop`, `nounk`), if it has one.
pub fn mode_of(cell: &str) -> Option<&str> {
    ["retain", "drop", "nounk"].into_iter().find(|m| cell.strip_suffix(m).map_or(false, |r| r.ends_with('-')))
}

/// The cell's name without its mode: `A`, `B`, `Bf`, `C`, `Cf`, `C-cb`, `Cf-cb`, ... (the
/// crossings rows).
pub fn stem(cell: &str) -> &str {
    match mode_of(cell) {
        Some(m) => &cell[..cell.len() - m.len() - 1],
        None => cell,
    }
}

/// The cell is a CALLBACK cell (`B-cb`, `C-cb-drop`, `Cf-cb-retain`, ...): the core's
/// callback delivery bridged to async Rust with a tokio oneshot, its k callers async tasks
/// on the cell's runtime. CAMPAIGN req 16 as amended (owner, 2026-09-28): for Rust these
/// are the REFERENCE core-transport cells; the blocking B, C, E cells are the labelled row.
pub fn cb(cell: &str) -> bool {
    stem(cell).ends_with("-cb")
}

/// How the cell's calls are delivered: `tonic` (A, D, F), `callback` (-cb) or `blocking`.
pub fn delivery(cell: &str) -> &'static str {
    match base(cell) {
        'B' | 'C' | 'E' if cb(cell) => "callback",
        'B' | 'C' | 'E' => "blocking",
        _ => "tonic",
    }
}

/// `retain` for a cell name's mode suffix.
pub fn retain_of(cell: &str) -> bool {
    cell.ends_with("-retain")
}

/// The codec state of one in-flight slot: the binding's contexts (core-ffi) and a
/// core-native encoder. Used by one thread or one task at a time.
pub struct Slot {
    pub ctx: harness::arms::core_ffi_arm::Ctx,
    pub enc: std::cell::UnsafeCell<ak_rt::Enc>,
}
unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

pub fn slots(k: usize) -> &'static [Slot] {
    Box::leak((0..k).map(|_| Slot {
        ctx: harness::arms::core_ffi_arm::Ctx::new(),
        enc: std::cell::UnsafeCell::new(ak_rt::Enc::new(facade::generated::core_native::SITES)),
    }).collect::<Vec<_>>().into_boxed_slice())
}

/// The call of `cell` in direction `dir` ("a", "a+read", "b") over the in-flight slots `sl`
/// (caller i uses slot i). `want_a` is the expected response length of direction (a) (the
/// runner's plant makes it wrong).
pub fn call_of(cell: &str, conn: &Conn, dir: &'static str, sl: &'static [Slot], want_a: u64) -> Call {
    let retain = retain_of(cell);
    let fetch = dir != "b";
    let read = dir == "a+read";
    let want = if fetch { want_a } else { 0 };
    let path = if fetch { FETCH } else { PUSH };
    let p_val = Arc::new(m2::prost_arm::value(m2::P2_2));
    let f_val: &'static _ = Box::leak(Box::new(m2::armonik_arm::value(m2::P2_2)));
    let c = base(cell);
    let name: &'static str = cell_of(cell);
    let check = move |got: usize| -> Result<(), String> {
        if got as u64 != want { Err(format!("cell {name} response {got} B, expected {want}")) } else { Ok(()) }
    };
    match (c, conn) {
        ('A', Conn::Tonic(rt, ch)) => {
            let (ch, empty) = (ch.clone(), Arc::new(shapes_prost::shapes::Empty {}));
            Call::Async(rt.clone(), Arc::new(move |_i| {
                let (ch, empty, p_val) = (ch.clone(), empty.clone(), p_val.clone());
                Box::pin(async move {
                    let len = Arc::new(AtomicU64::new(u64::MAX));
                    let mut g = tonic::client::Grpc::new(ch);
                    g.ready().await.map_err(|e| e.to_string())?;
                    let pq = http::uri::PathAndQuery::from_static(path);
                    if fetch {
                        let codec = PCodec::<shapes_prost::shapes::Empty, shapes_prost::shapes::ListTasksDetailedResponse> { len: len.clone(), _p: Default::default() };
                        let v = g.unary(tonic::Request::new(empty), pq, codec).await.map_err(|s| s.to_string())?.into_inner();
                        if read { std::hint::black_box(M2::touch_p(&v)); } else { std::hint::black_box(&v); }
                    } else {
                        let codec = PCodec::<shapes_prost::shapes::ListTasksDetailedResponse, shapes_prost::shapes::Empty> { len: len.clone(), _p: Default::default() };
                        std::hint::black_box(g.unary(tonic::Request::new(p_val), pq, codec).await.map_err(|s| s.to_string())?.into_inner());
                    }
                    check(len.load(Ordering::Relaxed) as usize)
                }) as Fut
            }))
        }
        ('B', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |_i| {
                let body = if fetch { Vec::new() } else { prost::Message::encode_to_vec(&*p_val) };
                cc.call(path, &body, |resp| {
                    check(resp.len())?;
                    if fetch {
                        let v = <shapes_prost::shapes::ListTasksDetailedResponse as prost::Message>::decode(resp).map_err(|e| e.to_string())?;
                        if read { std::hint::black_box(M2::touch_p(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                })
            }))
        }
        ('C', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |i| {
                let ctx = &sl[i].ctx;
                let on_resp = |resp: &[u8]| {
                    check(resp.len())?;
                    if fetch {
                        let v = M2::f_decode(ctx, resp, retain).map_err(|e| format!("core-ffi decode {e}"))?;
                        if read { std::hint::black_box(M2::touch_f(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                };
                if fetch {
                    cc.call(path, &[], on_resp)
                } else {
                    // Optimisation R2: the encoded request is MOVED into the call
                    // (ak_call_unary_enc), not read out with ak_enc_take and copied.
                    M2::f_encode(ctx, f_val, retain).map_err(|e| format!("core-ffi encode {e}"))?;
                    cc.call_enc(path, ctx.enc, on_resp)
                }
            }))
        }
        ('E', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |i| {
                let e = unsafe { &mut *sl[i].enc.get() };
                let body: &[u8] = if fetch {
                    &[]
                } else {
                    M2::n_encode(f_val, e, retain);
                    &e.buf
                };
                cc.call(path, body, |resp| {
                    check(resp.len())?;
                    if fetch {
                        let v = M2::n_decode(resp, retain).map_err(|e| format!("core-native decode {e}"))?;
                        if read { std::hint::black_box(M2::touch_f(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                })
            }))
        }
        ('D', Conn::Tonic(rt, ch)) | ('F', Conn::Tonic(rt, ch)) => {
            let ch = ch.clone();
            let ffi = c == 'D';
            let framed = framed(cell);
            Call::Async(rt.clone(), Arc::new(move |i| {
                let ch = ch.clone();
                let slot = &sl[i];
                // The request body, encoded before the call as the idiomatic client does:
                // the core's buffer moved to the host as an owned `Bytes` (D, T1 ffi),
                // core-native's moved out by Enc::take (F, T1) (requirement 11 row
                // transport-ready-tonic).
                let body = if fetch {
                    Ok(Bytes::new())
                } else if ffi {
                    // T1 (ffi): the core's buffer moved to the host, not copied
                    M2::f_encode(&slot.ctx, f_val, retain)
                        .map_err(|e| format!("core-ffi encode {e}"))
                        .and_then(|_| crate::ffi_owned_body(slot.ctx.enc).map_err(|rc| format!("ak_enc_take_owned rc {rc}")))
                } else {
                    let e = unsafe { &mut *slot.enc.get() };
                    M2::n_encode(f_val, e, retain);
                    // T1: moved out (Enc::take), not copied
                    Ok(e.take())
                };
                Box::pin(async move {
                    let body = body?;
                    let pq = http::uri::PathAndQuery::from_static(path);
                    let resp: Bytes = if framed {
                        // T1 option 3: two body frames, no copy into tonic's buffer.
                        rpc::unary_framed(ch, pq, body, None).await.map_err(|s| s.to_string())?
                    } else {
                        let mut g = tonic::client::Grpc::new(ch);
                        g.ready().await.map_err(|e| e.to_string())?;
                        g.unary(tonic::Request::new(body), pq, rpc::RawCodec).await.map_err(|s| s.to_string())?.into_inner()
                    };
                    check(resp.len())?;
                    if fetch {
                        let v = if ffi {
                            M2::f_decode(&slot.ctx, &resp, retain).map_err(|e| format!("core-ffi decode {e}"))?
                        } else {
                            M2::n_decode(&resp, retain).map_err(|e| format!("core-native decode {e}"))?
                        };
                        if read { std::hint::black_box(M2::touch_f(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                }) as Fut
            }))
        }
        // The callback cells (-cb): the same work as B, C, E, the call through the core's
        // callback delivery awaited on a oneshot by an async task (CAMPAIGN req 16 as amended).
        ('B', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |_i| {
                let body = if fetch { Vec::new() } else { prost::Message::encode_to_vec(&*p_val) };
                let fut = cc.call_cb(path, &body);
                Box::pin(async move {
                    let r = fut.await?;
                    let resp = r.bytes();
                    check(resp.len())?;
                    if fetch {
                        let v = <shapes_prost::shapes::ListTasksDetailedResponse as prost::Message>::decode(resp).map_err(|e| e.to_string())?;
                        if read { std::hint::black_box(M2::touch_p(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                }) as Fut
            }))
        }
        ('C', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |i| {
                let slot: &'static Slot = &sl[i];
                type F = Pin<Box<dyn Future<Output = Result<CbResp, String>> + Send>>;
                let fut: Result<F, String> = if fetch {
                    Ok(Box::pin(cc.call_cb(path, &[])))
                } else {
                    // As cell C: the encoded request MOVED into the call (ak_call_unary_enc_cb).
                    M2::f_encode(&slot.ctx, f_val, retain).map_err(|e| format!("core-ffi encode {e}"))
                        .map(|_| Box::pin(cc.call_enc_cb(path, slot.ctx.enc)) as F)
                };
                Box::pin(async move {
                    let r = fut?.await?;
                    let resp = r.bytes();
                    check(resp.len())?;
                    if fetch {
                        let v = M2::f_decode(&slot.ctx, resp, retain).map_err(|e| format!("core-ffi decode {e}"))?;
                        if read { std::hint::black_box(M2::touch_f(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                }) as Fut
            }))
        }
        ('E', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |i| {
                let slot: &'static Slot = &sl[i];
                let fut = if fetch {
                    cc.call_cb(path, &[])
                } else {
                    let e = unsafe { &mut *slot.enc.get() };
                    M2::n_encode(f_val, e, retain);
                    cc.call_cb(path, &e.buf)
                };
                Box::pin(async move {
                    let r = fut.await?;
                    let resp = r.bytes();
                    check(resp.len())?;
                    if fetch {
                        let v = M2::n_decode(resp, retain).map_err(|e| format!("core-native decode {e}"))?;
                        if read { std::hint::black_box(M2::touch_f(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                }) as Fut
            }))
        }
        (c, _) => panic!("cell {c} with the wrong connection"),
    }
}

/// U1-unary: M5 payload `pid` as the facade value (core-native, core-ffi) and as prost's
/// (A, B), both checked against the validated manifest before any call (requirement 13).
pub fn m5_values(pid: &str) -> (&'static facade::UploadResultDataMessage, Arc<shapes_prost::shapes::UploadResultDataMessage>, usize) {
    let f = M5::build(pid).unwrap_or_else(|| panic!("no payload {pid}"));
    let wire = prost::Message::encode_to_vec(&f);
    let man = harness::manifest::Manifest::load();
    assert_eq!(harness::manifest::sha(&wire), man.row(pid).sha256, "{pid} bytes differ from the manifest");
    let p = <shapes_prost::shapes::UploadResultDataMessage as prost::Message>::decode(&wire[..]).expect("prost decodes the payload");
    assert_eq!(prost::Message::encode_to_vec(&p), wire, "{pid}: prost re-encodes the same bytes");
    (Box::leak(Box::new(f)), Arc::new(p), wire.len())
}

/// The call of `cell` in direction `c` (U1-unary): encode payload `pid` (M5) and upload it
/// to `UPLOAD`; the response must be empty (requirement 18: status OK, length 0 -- or
/// `want` under the runner's plant). Every cell that has direction `b`, framed twins included.
pub fn call_of_c(cell: &str, conn: &Conn, pid: &str, sl: &'static [Slot], want: u64) -> Call {
    let retain = retain_of(cell);
    let (f_val, p_val, _) = m5_values(pid);
    let name: &'static str = cell_of(cell);
    let check = move |got: usize| -> Result<(), String> {
        if got as u64 != want { Err(format!("cell {name} upload response {got} B, expected {want}")) } else { Ok(()) }
    };
    let path = UPLOAD;
    match (base(cell), conn) {
        ('A', Conn::Tonic(rt, ch)) => {
            let ch = ch.clone();
            Call::Async(rt.clone(), Arc::new(move |_i| {
                let (ch, p_val) = (ch.clone(), p_val.clone());
                Box::pin(async move {
                    let len = Arc::new(AtomicU64::new(u64::MAX));
                    let mut g = tonic::client::Grpc::new(ch);
                    g.ready().await.map_err(|e| e.to_string())?;
                    let codec = PCodec::<shapes_prost::shapes::UploadResultDataMessage, shapes_prost::shapes::Empty> { len: len.clone(), _p: Default::default() };
                    std::hint::black_box(g.unary(tonic::Request::new(p_val), http::uri::PathAndQuery::from_static(path), codec).await.map_err(|s| s.to_string())?.into_inner());
                    check(len.load(Ordering::Relaxed) as usize)
                }) as Fut
            }))
        }
        ('B', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |_i| {
                let body = prost::Message::encode_to_vec(&*p_val);
                cc.call(path, &body, |resp| check(resp.len()))
            }))
        }
        ('C', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |i| {
                let ctx = &sl[i].ctx;
                M5::f_encode(ctx, f_val, retain).map_err(|e| format!("core-ffi encode {e}"))?;
                cc.call_enc(path, ctx.enc, |resp| check(resp.len()))
            }))
        }
        ('E', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |i| {
                let e = unsafe { &mut *sl[i].enc.get() };
                M5::n_encode(f_val, e, retain);
                cc.call(path, &e.buf, |resp| check(resp.len()))
            }))
        }
        ('D', Conn::Tonic(rt, ch)) | ('F', Conn::Tonic(rt, ch)) => {
            let ch = ch.clone();
            let ffi = base(cell) == 'D';
            let framed = framed(cell);
            Call::Async(rt.clone(), Arc::new(move |i| {
                let ch = ch.clone();
                let slot = &sl[i];
                let body = if ffi {
                    M5::f_encode(&slot.ctx, f_val, retain)
                        .map_err(|e| format!("core-ffi encode {e}"))
                        .and_then(|_| crate::ffi_owned_body(slot.ctx.enc).map_err(|rc| format!("ak_enc_take_owned rc {rc}")))
                } else {
                    let e = unsafe { &mut *slot.enc.get() };
                    M5::n_encode(f_val, e, retain);
                    Ok(e.take())
                };
                Box::pin(async move {
                    let body = body?;
                    let pq = http::uri::PathAndQuery::from_static(path);
                    let resp: Bytes = if framed {
                        rpc::unary_framed(ch, pq, body, None).await.map_err(|s| s.to_string())?
                    } else {
                        let mut g = tonic::client::Grpc::new(ch);
                        g.ready().await.map_err(|e| e.to_string())?;
                        g.unary(tonic::Request::new(body), pq, rpc::RawCodec).await.map_err(|s| s.to_string())?.into_inner()
                    };
                    check(resp.len())
                }) as Fut
            }))
        }
        ('B', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |_i| {
                let body = prost::Message::encode_to_vec(&*p_val);
                let fut = cc.call_cb(path, &body);
                Box::pin(async move { check(fut.await?.bytes().len()) }) as Fut
            }))
        }
        ('C', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |i| {
                let ctx = &sl[i].ctx;
                let fut = M5::f_encode(ctx, f_val, retain).map_err(|e| format!("core-ffi encode {e}"))
                    .map(|_| cc.call_enc_cb(path, ctx.enc));
                Box::pin(async move { check(fut?.await?.bytes().len()) }) as Fut
            }))
        }
        ('E', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |i| {
                let e = unsafe { &mut *sl[i].enc.get() };
                M5::n_encode(f_val, e, retain);
                let fut = cc.call_cb(path, &e.buf);
                Box::pin(async move { check(fut.await?.bytes().len()) }) as Fut
            }))
        }
        (c, _) => panic!("cell {c} with the wrong connection"),
    }
}

/// U2-stream: one upload of `chunks` x 2 MiB, as the facade values and as prost's, the total
/// data bytes, and the SHA-256 of the messages' wire bytes (all encoders byte-identical, so
/// the one expectation serves every cell). Deterministic data (splitmix64).
pub struct StreamPayload {
    pub f: Vec<facade::UploadResultDataMessage>,
    pub p: Vec<Arc<shapes_prost::shapes::UploadResultDataMessage>>,
    pub data_bytes: u64,
    pub sha256: [u8; 32],
}

pub fn stream_payload(chunks: usize) -> &'static StreamPayload {
    use sha2::Digest;
    let mut seed = 0x5EED_0000u64 + chunks as u64;
    let mut next = || {
        seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let (mut f, mut p) = (Vec::new(), Vec::new());
    let mut h = sha2::Sha256::new();
    for i in 0..chunks {
        let mut data = Vec::with_capacity(CHUNK);
        while data.len() < CHUNK {
            data.extend_from_slice(&next().to_le_bytes());
        }
        let first = i == 0;
        let v = facade::UploadResultDataMessage {
            upload: Some(facade::UploadResultData {
                session_id: if first { "session-u2".into() } else { String::new() },
                result_id: if first { "result-u2".into() } else { String::new() },
                data_chunk: bytes::Bytes::from(data),
                ..Default::default()
            }),
            ..Default::default()
        };
        let wire = prost::Message::encode_to_vec(&v);
        h.update(&wire);
        let pv = <shapes_prost::shapes::UploadResultDataMessage as prost::Message>::decode(&wire[..]).expect("prost decodes the chunk");
        assert_eq!(prost::Message::encode_to_vec(&pv), wire, "prost re-encodes the chunk");
        let mut e = ak_rt::Enc::new(facade::generated::core_native::SITES);
        M5::n_encode(&v, &mut e, false);
        assert_eq!(&e.buf[..], &wire[..], "core-native encodes the chunk as prost does");
        f.push(v);
        p.push(Arc::new(pv));
    }
    Box::leak(Box::new(StreamPayload { f, p, data_bytes: (chunks * CHUNK) as u64, sha256: h.finalize().into() }))
}

/// The response of an upload: the data byte count (u64 LE), then (STREAM_CHECK) 32 bytes.
pub fn stream_response(resp: &[u8], want_bytes: u64, want_sha: Option<&[u8; 32]>) -> Result<(), String> {
    let want_len = if want_sha.is_some() { 40 } else { 8 };
    if resp.len() != want_len {
        return Err(format!("upload response {} B, expected {want_len}", resp.len()));
    }
    let got = u64::from_le_bytes(resp[..8].try_into().unwrap());
    if got != want_bytes {
        return Err(format!("the server received {got} data bytes, expected {want_bytes}"));
    }
    if let Some(sha) = want_sha {
        if &resp[8..] != &sha[..] {
            return Err("the server received other bytes than the uploaded ones (SHA-256 differs)".into());
        }
    }
    Ok(())
}

/// Blocking client streaming through the core (cells B, C, E): open, one `send` per chunk
/// (`send(i, handle, last)` returns the entry's rc), recv, free, destroy.
fn core_stream(cc: &CoreClient, path: &str, n: usize, mut send: impl FnMut(usize, *mut ak_call, i32) -> i32,
               on_resp: impl FnOnce(&[u8]) -> Result<(), String>) -> Result<(), String> {
    unsafe {
        let h = ak_call_open(cc.client, path.as_ptr(), path.len(), AK_CALL_CLIENT_STREAM, std::ptr::null());
        if h.is_null() {
            return Err("ak_call_open NULL".into());
        }
        let mut r = Ok(());
        for i in 0..n {
            let rc = send(i, h, (i + 1 == n) as i32);
            if rc != AK_OK {
                r = Err(format!("ak_call_send chunk {i} rc {rc}"));
                ak_call_cancel(h);
                break;
            }
        }
        let mut out = ak_bytes::default();
        let mut gs = -1i32;
        let rc = ak_call_recv(h, &mut out, &mut gs);
        if r.is_ok() {
            r = if rc != AK_OK {
                Err(format!("ak_call_recv rc {rc}, grpc status {gs}"))
            } else {
                on_resp(if out.len == 0 { &[] } else { std::slice::from_raw_parts(out.ptr, out.len) })
            };
        }
        ak_bytes_free(&mut out);
        ak_call_destroy(h);
        r
    }
}

/// A prost request stream, raw response (cell A of direction d).
struct PRaw<Q>(std::marker::PhantomData<Q>);
impl<Q: prost::Message + Send + Sync + 'static> Codec for PRaw<Q> {
    type Encode = Arc<Q>;
    type Decode = Bytes;
    type Encoder = PEnc<Q>;
    type Decoder = rpc::RawDecoder;
    fn encoder(&mut self) -> PEnc<Q> {
        PEnc(std::marker::PhantomData)
    }
    fn decoder(&mut self) -> rpc::RawDecoder {
        rpc::RawDecoder
    }
}

/// The call of `cell` in direction `d` (U2-stream): upload `chunks` x 2 MiB as a client
/// stream to `path` (STREAM, or STREAM_CHECK for the gate's byte check); requirement 18:
/// status OK and the server's data byte count (`want`, the plant makes it wrong), plus the
/// SHA-256 on STREAM_CHECK.
pub fn call_of_d(cell: &str, conn: &Conn, chunks: usize, sl: &'static [Slot], want: u64, check: bool) -> Call {
    let sha: Option<&'static [u8; 32]> = if check { Some(&stream_payload(chunks).sha256) } else { None };
    call_of_d_with(cell, conn, chunks, sl, want, sha)
}

/// `call_of_d` with the expected SHA-256 given (upload_check's control plants a wrong one).
pub fn call_of_d_with(cell: &str, conn: &Conn, chunks: usize, sl: &'static [Slot], want: u64, sha: Option<&'static [u8; 32]>) -> Call {
    let retain = retain_of(cell);
    let pl = stream_payload(chunks);
    let name: &'static str = cell_of(cell);
    let path: &'static str = if sha.is_some() { STREAM_CHECK } else { STREAM };
    let verdict = move |resp: &[u8]| stream_response(resp, want, sha).map_err(|e| format!("cell {name}: {e}"));
    let n = chunks;
    match (base(cell), conn) {
        ('A', Conn::Tonic(rt, ch)) => {
            let ch = ch.clone();
            Call::Async(rt.clone(), Arc::new(move |_i| {
                let ch = ch.clone();
                Box::pin(async move {
                    let mut g = tonic::client::Grpc::new(ch);
                    g.ready().await.map_err(|e| e.to_string())?;
                    let msgs = tokio_stream::iter(pl.p.iter().cloned());
                    let resp = g.client_streaming(tonic::Request::new(msgs), http::uri::PathAndQuery::from_static(path), PRaw(std::marker::PhantomData))
                        .await.map_err(|s| s.to_string())?.into_inner();
                    verdict(&resp)
                }) as Fut
            }))
        }
        ('B', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |_i| {
                core_stream(&cc, path, n, |i, h, last| unsafe {
                    let body = prost::Message::encode_to_vec(&*pl.p[i]);
                    ak_call_send(h, body.as_ptr(), body.len(), last)
                }, verdict)
            }))
        }
        ('C', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |i| {
                let ctx = &sl[i].ctx;
                core_stream(&cc, path, n, |j, h, last| unsafe {
                    if let Err(e) = M5::f_encode(ctx, &pl.f[j], retain) { return e; }
                    ak_call_send_enc(h, ctx.enc, last)
                }, verdict)
            }))
        }
        ('E', Conn::Core(cc)) => {
            let cc = cc.clone();
            Call::Blocking(Arc::new(move |i| {
                let e = unsafe { &mut *sl[i].enc.get() };
                core_stream(&cc, path, n, |j, h, last| unsafe {
                    M5::n_encode(&pl.f[j], e, retain);
                    ak_call_send(h, e.buf.as_ptr(), e.buf.len(), last)
                }, verdict)
            }))
        }
        ('D', Conn::Tonic(rt, ch)) | ('F', Conn::Tonic(rt, ch)) => {
            let ch = ch.clone();
            let ffi = base(cell) == 'D';
            let framed = framed(cell);
            Call::Async(rt.clone(), Arc::new(move |i| {
                let ch = ch.clone();
                let slot: &'static Slot = &sl[i];
                // The messages encoded lazily, one as the transport asks for it (on the
                // connection's thread). The COUNTING build encodes them all first, on this
                // thread: the binding's tallies (ak_enc_reset) are thread-local, and the
                // entries crossed per call are the same either way.
                let enc = move |j: usize| {
                    if ffi {
                        match M5::f_encode(&slot.ctx, &pl.f[j], retain) {
                            Ok(_) => crate::ffi_owned_body(slot.ctx.enc).unwrap_or_default(),
                            Err(_) => Bytes::new(),
                        }
                    } else {
                        let e = unsafe { &mut *slot.enc.get() };
                        M5::n_encode(&pl.f[j], e, retain);
                        e.take()
                    }
                };
                #[cfg(not(feature = "count"))]
                let msgs = tokio_stream::StreamExt::map(tokio_stream::iter(0..n), enc);
                #[cfg(feature = "count")]
                let msgs = tokio_stream::iter((0..n).map(enc).collect::<Vec<_>>());
                Box::pin(async move {
                    let pq = http::uri::PathAndQuery::from_static(path);
                    let resp = if framed {
                        rpc::client_streaming_framed(ch, pq, msgs, None).await.map_err(|s| s.to_string())?
                    } else {
                        rpc::client_streaming_raw_from(ch, pq, msgs).await.map_err(|s| s.to_string())?
                    };
                    verdict(&resp)
                }) as Fut
            }))
        }
        ('B', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |_i| {
                core_stream_cb(cc.clone(), path, n, move |j, h, last, ud| unsafe {
                    let body = prost::Message::encode_to_vec(&*pl.p[j]);
                    ak_call_send_cb(h, body.as_ptr(), body.len(), last, on_complete, ud, 0)
                }, verdict)
            }))
        }
        ('C', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |i| {
                let slot: &'static Slot = &sl[i];
                core_stream_cb(cc.clone(), path, n, move |j, h, last, ud| unsafe {
                    if let Err(e) = M5::f_encode(&slot.ctx, &pl.f[j], retain) { return e; }
                    ak_call_send_enc_cb(h, slot.ctx.enc, last, on_complete, ud, 0)
                }, verdict)
            }))
        }
        ('E', Conn::CoreCb(cc, rt)) => {
            let cc = cc.clone();
            Call::Async(rt.clone(), Arc::new(move |i| {
                let slot: &'static Slot = &sl[i];
                core_stream_cb(cc.clone(), path, n, move |j, h, last, ud| unsafe {
                    let e = &mut *slot.enc.get();
                    M5::n_encode(&pl.f[j], e, retain);
                    ak_call_send_cb(h, e.buf.as_ptr(), e.buf.len(), last, on_complete, ud, 0)
                }, verdict)
            }))
        }
        (c, _) => panic!("cell {c} with the wrong connection"),
    }
}

/// The warm-up through direction d (4 MiB), for the plant control of the streamed path.
pub fn warm_with_d(cells: &[&str], target: &str, pinned: bool, n: usize, want_plus: u64) -> Result<(), String> {
    for &cell in cells {
        let conn = Conn::open(cell, target, pinned);
        let call = call_of_d(cell, &conn, D_PAYLOADS[0].1, slots(1), (D_PAYLOADS[0].1 * CHUNK) as u64 + want_plus, false);
        for _ in 0..n {
            call.once(0)?;
        }
    }
    Ok(())
}

/// The cells of THIS build (requirement 12): A and B once, C, D, E, F per unknown-field mode.
#[cfg(feature = "unknown-fields")]
pub const CELLS: &[&str] = &["A", "B", "C-retain", "C-drop", "D-retain", "D-drop", "E-retain", "E-drop", "F-retain", "F-drop",
    "Bf", "Cf-retain", "Cf-drop", "Df-retain", "Df-drop", "Ef-retain", "Ef-drop", "Ff-retain", "Ff-drop",
    // the callback cells (CAMPAIGN req 16 as amended: Rust's reference core-transport cells)
    "B-cb", "C-cb-retain", "C-cb-drop", "E-cb-retain", "E-cb-drop",
    "Bf-cb", "Cf-cb-retain", "Cf-cb-drop", "Ef-cb-retain", "Ef-cb-drop"];
/// The no-unknown build: A and B again as its in-process controls.
#[cfg(not(feature = "unknown-fields"))]
pub const CELLS: &[&str] = &["A", "B", "C-nounk", "D-nounk", "E-nounk", "F-nounk", "Bf", "Cf-nounk", "Df-nounk", "Ef-nounk", "Ff-nounk",
    "B-cb", "C-cb-nounk", "E-cb-nounk", "Bf-cb", "Cf-cb-nounk", "Ef-cb-nounk"];

pub const DIRS: &[&str] = &["a", "a+read", "b"];

/// Server warm-up (requirement 13 as amended): `n` Fetch calls from each client transport
/// (tonic, the core's), checked, before round 1 of the first cell.
pub fn warm_server(target: &str, pinned: bool, n: usize, want_a: u64) -> Result<(), String> {
    warm_with(&["A", "B"], target, pinned, n, want_a)
}

/// A cell of this build named by its stem (`A`, `Bf`, `Df`, ...): the stem itself when it is a
/// cell, else the first cell `<stem>-<mode>`. `rpc_suite`'s AK_RPC_WARM_CELLS takes stems, so the
/// runner's plant control (requirement 18) can name ONE send path per run and each path
/// must abort on its own: A (tonic codec), B (core, reference), Bf (core, framed), Df (tonic
/// Channel, framed, the harness's).
pub fn cell_of(stem: &str) -> &'static str {
    CELLS.iter().copied().find(|c| *c == stem || c.strip_prefix(stem).map_or(false, |r| r.starts_with('-')))
        .unwrap_or_else(|| panic!("no cell {stem} in this build"))
}

/// The warm-up through direction c (P5.3), for the plant control of the upload path.
pub fn warm_with_c(cells: &[&str], target: &str, pinned: bool, n: usize, want_c: u64) -> Result<(), String> {
    for &cell in cells {
        let conn = Conn::open(cell, target, pinned);
        let call = call_of_c(cell, &conn, "P5.3", slots(1), want_c);
        for _ in 0..n {
            call.once(0)?;
        }
    }
    Ok(())
}

pub fn warm_with(cells: &[&str], target: &str, pinned: bool, n: usize, want_a: u64) -> Result<(), String> {
    for &cell in cells {
        let conn = Conn::open(cell, target, pinned);
        let call = call_of(cell, &conn, "a", slots(1), want_a);
        for _ in 0..n {
            call.once(0)?;
        }
    }
    Ok(())
}
