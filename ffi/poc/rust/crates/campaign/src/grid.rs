//! CAMPAIGN.md 4.2: the RPC grid's client cells (requirement 12), shared by `rpc_client` (the
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
//! Delivery (requirement 16 as amended, R-H30): B, C and E use the core's blocking call from
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
use crate::Ops;
use ak_abi::*;
use bytes::Bytes;
use harness::arms_m2 as m2;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

pub const FETCH: &str = "/armonik.ffi.campaign.v1.Grid/Fetch";
pub const PUSH: &str = "/armonik.ffi.campaign.v1.Grid/Push";
pub const WIN: u32 = 4 * 1024 * 1024;
/// Worker threads of every tokio runtime the client makes (cells A, D, F), and of the
/// core's runtime (`ak_runtime_new`, cells B, C, E). Recorded in every header (req 4).
pub const TOKIO_WORKERS: usize = 2;
pub const CORE_WORKERS: u32 = 2;

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
}
unsafe impl Send for CoreClient {}
unsafe impl Sync for CoreClient {}
impl CoreClient {
    pub fn new(target: &str, pinned: bool) -> Self {
        unsafe {
            let rt = ak_runtime_new(CORE_WORKERS);
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
            CoreClient { rt, client }
        }
    }
    /// One blocking call; the response bytes are handed to `f` and freed after.
    pub fn call<T>(&self, path: &str, req: &[u8], f: impl FnOnce(&[u8]) -> Result<T, String>) -> Result<T, String> {
        unsafe {
            let mut out = ak_bytes::default();
            let rc = ak_call_unary(self.client, path.as_ptr(), path.len(), req.as_ptr(), req.len(), &mut out);
            if rc != AK_OK {
                return Err(format!("ak_call_unary rc {rc}"));
            }
            let r = f(if out.len == 0 { &[] } else { std::slice::from_raw_parts(out.ptr, out.len) });
            ak_bytes_free(&mut out);
            r
        }
    }
}
impl Drop for CoreClient {
    fn drop(&mut self) {
        unsafe {
            ak_client_destroy(self.client);
            ak_runtime_destroy(self.rt);
        }
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
    Tonic(Arc<tokio::runtime::Runtime>, tonic::transport::Channel),
}

impl Conn {
    pub fn open(cell: &str, target: &str, pinned: bool) -> Conn {
        match base(cell) {
            'B' | 'C' | 'E' => Conn::Core(Arc::new(CoreClient::new(target, pinned))),
            _ => {
                let rt = Arc::new(tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(TOKIO_WORKERS).enable_all().build().unwrap());
                let ch = tonic_channel(&rt, target, pinned);
                Conn::Tonic(rt, ch)
            }
        }
    }
}

pub fn base(cell: &str) -> char {
    cell.chars().next().unwrap()
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
    let check = move |got: usize| -> Result<(), String> {
        if got as u64 != want { Err(format!("cell {c} response {got} B, expected {want}")) } else { Ok(()) }
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
                let body: &[u8] = if fetch {
                    &[]
                } else {
                    M2::f_encode(ctx, f_val, retain).map_err(|e| format!("core-ffi encode {e}"))?;
                    unsafe { harness::generated::binding::encoded(ctx.enc) }
                };
                cc.call(path, body, |resp| {
                    check(resp.len())?;
                    if fetch {
                        let v = M2::f_decode(ctx, resp, retain).map_err(|e| format!("core-ffi decode {e}"))?;
                        if read { std::hint::black_box(M2::touch_f(&v)); } else { std::hint::black_box(&v); }
                    }
                    Ok(())
                })
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
            Call::Async(rt.clone(), Arc::new(move |i| {
                let ch = ch.clone();
                let slot = &sl[i];
                // The request body, encoded before the call as the idiomatic client does:
                // the core's buffer (D) or core-native's (F), copied into the `Bytes` tonic
                // takes (the transport-ready form of requirement 11).
                let body = if fetch {
                    Ok(Bytes::new())
                } else if ffi {
                    M2::f_encode(&slot.ctx, f_val, retain)
                        .map(|_| Bytes::copy_from_slice(unsafe { harness::generated::binding::encoded(slot.ctx.enc) }))
                        .map_err(|e| format!("core-ffi encode {e}"))
                } else {
                    let e = unsafe { &mut *slot.enc.get() };
                    M2::n_encode(f_val, e, retain);
                    Ok(Bytes::copy_from_slice(&e.buf))
                };
                Box::pin(async move {
                    let body = body?;
                    let mut g = tonic::client::Grpc::new(ch);
                    g.ready().await.map_err(|e| e.to_string())?;
                    let pq = http::uri::PathAndQuery::from_static(path);
                    let resp: Bytes = g.unary(tonic::Request::new(body), pq, rpc::RawCodec).await.map_err(|s| s.to_string())?.into_inner();
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
        (c, _) => panic!("cell {c} with the wrong connection"),
    }
}

/// The cells of THIS build (requirement 12): A and B once, C, D, E, F per unknown-field mode.
#[cfg(feature = "unknown-fields")]
pub const CELLS: &[&str] = &["A", "B", "C-retain", "C-drop", "D-retain", "D-drop", "E-retain", "E-drop", "F-retain", "F-drop"];
/// The no-unknown build: A and B again as its in-process controls.
#[cfg(not(feature = "unknown-fields"))]
pub const CELLS: &[&str] = &["A", "B", "C-nounk", "D-nounk", "E-nounk", "F-nounk"];

pub const DIRS: &[&str] = &["a", "a+read", "b"];

/// Server warm-up (requirement 13 as amended): `n` Fetch calls from each client transport
/// (tonic, the core's), checked, before round 1 of the first cell.
pub fn warm_server(target: &str, pinned: bool, n: usize, want_a: u64) -> Result<(), String> {
    for cell in ["A", "B"] {
        let conn = Conn::open(cell, target, pinned);
        let call = call_of(cell, &conn, "a", slots(1), want_a);
        for _ in 0..n {
            call.once(0)?;
        }
    }
    Ok(())
}
