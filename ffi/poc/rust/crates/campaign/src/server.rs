//! CAMPAIGN.md 4.2: the RPC grid's server (requirement 13), shared by the `rpc_server`
//! process and by the counting build's per-call RPC counts (`crossings`, requirement 19).
//!
//! Two methods, both raw bytes, so the server's work is identical whichever cell calls:
//!   /armonik.ffi.campaign.v1.Grid/Fetch   direction (a): the request is ignored, the
//!        response is P2.2, PRE-SERIALISED once at start-up (prost's encoding, checked
//!        against the payload manifest's hash before the server accepts anything);
//!   /armonik.ffi.campaign.v1.Grid/Push    direction (b): the request is DECODED with prost
//!        (the incumbent, in every cell) and must be a non-empty ListTasksDetailedResponse;
//!        the response is empty. A request that fails to decode is answered with an error
//!        status, which the client counts as a failed call (requirement 18).
//! The socket is a Unix domain socket (requirement 17 as amended 2026-09-26, R-H28).
//! Transport: `shipped` = tonic's server defaults; `pinned` = 4 MiB stream and connection
//! windows, adaptive windows off (Nagle does not apply to a Unix socket).

use bytes::{Buf, BufMut, Bytes};
use std::sync::Arc;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

pub const FETCH: &str = "/armonik.ffi.campaign.v1.Grid/Fetch";
pub const PUSH: &str = "/armonik.ffi.campaign.v1.Grid/Push";
/// FIX-PLAN WP10 (SERVER.md): direction (a)'s PLANTED twin for other slices' requirement 18
/// controls: P2.2 with its last byte cut, so a client checking the length must fail.
pub const FETCH_SHORT: &str = "/armonik.ffi.campaign.v1.Grid/FetchShort";

#[derive(Default, Clone)]
struct Raw;
#[derive(Default, Clone)]
struct RawE;
#[derive(Default, Clone)]
struct RawD;
impl Encoder for RawE {
    type Item = Bytes;
    type Error = tonic::Status;
    fn encode(&mut self, item: Bytes, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.put_slice(&item);
        Ok(())
    }
}
impl Decoder for RawD {
    type Item = Bytes;
    type Error = tonic::Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Bytes>, Self::Error> {
        let n = src.remaining();
        // Optimisation R1 (as in the rpc crate): copy_to_bytes already yields an owned Bytes.
        Ok(Some(src.copy_to_bytes(n)))
    }
}
impl Codec for Raw {
    type Encode = Bytes;
    type Decode = Bytes;
    type Encoder = RawE;
    type Decoder = RawD;
    fn encoder(&mut self) -> RawE {
        RawE
    }
    fn decoder(&mut self) -> RawD {
        RawD
    }
}

/// T1 option 3's header evidence (`bin/header_diff`): when armed, every request the server
/// receives is recorded as it arrives -- method, URI, version, then each header in the order
/// the server's HeaderMap yields it. Off (None) in every timed run.
pub static CAPTURE: std::sync::Mutex<Option<Vec<Vec<String>>>> = std::sync::Mutex::new(None);

/// With `CAPTURE` armed: the sizes of the DATA frames each request's body arrived in, as the
/// server's body yields them (one entry per request, in arrival order).
pub static FRAMES: std::sync::Mutex<Vec<std::sync::Arc<std::sync::Mutex<Vec<usize>>>>> = std::sync::Mutex::new(Vec::new());

/// A request body that records its data frames' sizes (header_diff only).
struct CountFrames<B> {
    inner: B,
    sizes: std::sync::Arc<std::sync::Mutex<Vec<usize>>>,
}
impl<B: http_body::Body<Data = Bytes> + Unpin> http_body::Body for CountFrames<B> {
    type Data = Bytes;
    type Error = B::Error;
    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Bytes>, B::Error>>> {
        let r = std::pin::Pin::new(&mut self.inner).poll_frame(cx);
        if let std::task::Poll::Ready(Some(Ok(f))) = &r {
            if let Some(d) = f.data_ref() {
                self.sizes.lock().unwrap().push(d.len());
            }
        }
        r
    }
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}

/// The grid server's receive limit (U1-unary's P5.4 needs more than tonic's 4 MiB default).
pub const SERVER_MAX_RECV: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct Svc {
    pub fetch: Arc<Bytes>,
}

impl tonic::server::NamedService for Svc {
    const NAME: &'static str = "armonik.ffi.campaign.v1.Grid";
}

impl<B> tower_service::Service<http::Request<B>> for Svc
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        let mut sizes = None;
        if let Ok(mut g) = CAPTURE.lock() {
            if let Some(v) = g.as_mut() {
                let sz = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
                FRAMES.lock().unwrap().push(sz.clone());
                sizes = Some(sz);
                let mut r = vec![format!("{} {} {:?}", req.method(), req.uri(), req.version())];
                for (k, val) in req.headers() {
                    r.push(format!("{}: {}", k, val.to_str().unwrap_or("<non-ascii>")));
                }
                v.push(r);
            }
        }
        let fetch = self.fetch.clone();
        let push = req.uri().path() == PUSH;
        let short = req.uri().path() == FETCH_SHORT;
        let req_path = req.uri().path().to_string();
        // U2-stream: the client-streaming upload (and its byte-checking twin).
        let stream = match req.uri().path() {
            p if p == crate::grid::STREAM => Some(false),
            p if p == crate::grid::STREAM_CHECK => Some(true),
            _ => None,
        };
        let upload = req.uri().path() == crate::grid::UPLOAD;
        Box::pin(async move {
            // U1-unary: P5.4 is 4,194,390 B, over tonic's default 4 MiB decode limit, so the
            // grid's server accepts up to 8 MiB (every path; nothing else here comes near).
            let mut grpc = tonic::server::Grpc::new(Raw).max_decoding_message_size(SERVER_MAX_RECV);
            // The RPC semantics test's paths (bin rpc_semantics, gate step 11f): a chosen
            // status (unary and stream), a server that sleeps past the caller's deadline, and
            // a metadata echo. Never timed.
            if let Some(t) = test_path(&req_path) {
                let req = req.map(tonic::body::Body::new);
                return Ok(match t {
                    TestPath::StatusU(c) => grpc.unary(TestUnary { status: Some(c), sleep: false }, req).await,
                    TestPath::SleepU => grpc.unary(TestUnary { status: None, sleep: true }, req).await,
                    TestPath::StatusS(c) => grpc.client_streaming(TestStream { status: Some(c), sleep: false, echo: false, stall: false }, req).await,
                    TestPath::SleepS => grpc.client_streaming(TestStream { status: None, sleep: true, echo: false, stall: false }, req).await,
                    TestPath::StallS => grpc.client_streaming(TestStream { status: None, sleep: true, echo: false, stall: true }, req).await,
                    TestPath::EchoS => grpc.client_streaming(TestStream { status: None, sleep: false, echo: true, stall: false }, req).await,
                });
            }
            if let Some(check) = stream {
                return match sizes {
                    Some(sizes) => {
                        let req = req.map(|b| tonic::body::Body::new(CountFrames { inner: Box::pin(b), sizes }));
                        Ok(grpc.client_streaming(StreamAnswer { check }, req).await)
                    }
                    None => Ok(grpc.client_streaming(StreamAnswer { check }, req.map(tonic::body::Body::new)).await),
                };
            }
            match sizes {
                Some(sizes) => {
                    let req = req.map(|b| tonic::body::Body::new(CountFrames { inner: Box::pin(b), sizes }));
                    Ok(grpc.unary(Answer { fetch, push, upload, short }, req).await)
                }
                None => Ok(grpc.unary(Answer { fetch, push, upload, short }, req.map(tonic::body::Body::new)).await),
            }
        })
    }
}

/// The test paths: `<GRID>/StatusU<n>` and `<GRID>/StatusS<n>` answer status code n (unary,
/// stream), `SleepU` / `SleepS` answer OK after 3 s (past a test's deadline), `EchoS`
/// answers the request's `ak-echo` value, a '|' and its `ak-echo-bin` bytes; `StallS` sleeps 3 s
/// BEFORE reading the request stream (so a client's send can be left pending), then as SleepS.
pub const TEST_PREFIX: &str = "/armonik.ffi.campaign.v1.Grid/";

enum TestPath {
    StatusU(i32),
    StatusS(i32),
    SleepU,
    SleepS,
    StallS,
    EchoS,
}

fn test_path(p: &str) -> Option<TestPath> {
    let t = p.strip_prefix(TEST_PREFIX)?;
    if let Some(n) = t.strip_prefix("StatusU") {
        return n.parse().ok().map(TestPath::StatusU);
    }
    if let Some(n) = t.strip_prefix("StatusS") {
        return n.parse().ok().map(TestPath::StatusS);
    }
    match t {
        "SleepU" => Some(TestPath::SleepU),
        "SleepS" => Some(TestPath::SleepS),
        "StallS" => Some(TestPath::StallS),
        "EchoS" => Some(TestPath::EchoS),
        _ => None,
    }
}

#[derive(Clone)]
struct TestUnary {
    status: Option<i32>,
    sleep: bool,
}

impl tonic::server::UnaryService<Bytes> for TestUnary {
    type Response = Bytes;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<tonic::Response<Bytes>, tonic::Status>> + Send>>;
    fn call(&mut self, _req: tonic::Request<Bytes>) -> Self::Future {
        let (status, sleep) = (self.status, self.sleep);
        Box::pin(async move {
            if sleep {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            match status {
                Some(c) => Err(tonic::Status::new(tonic::Code::from(c), format!("the test path's status {c}"))),
                None => Ok(tonic::Response::new(Bytes::from_static(b"slept"))),
            }
        })
    }
}

#[derive(Clone)]
struct TestStream {
    status: Option<i32>,
    sleep: bool,
    echo: bool,
    stall: bool,
}

impl tonic::server::ClientStreamingService<Bytes> for TestStream {
    type Response = Bytes;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<tonic::Response<Bytes>, tonic::Status>> + Send>>;
    fn call(&mut self, req: tonic::Request<tonic::Streaming<Bytes>>) -> Self::Future {
        let (status, sleep, echo, stall) = (self.status, self.sleep, self.echo, self.stall);
        Box::pin(async move {
            let mut out = Vec::new();
            if echo {
                if let Some(v) = req.metadata().get("ak-echo") {
                    out.extend_from_slice(v.as_bytes());
                }
                out.push(b'|');
                if let Some(v) = req.metadata().get_bin("ak-echo-bin") {
                    out.extend_from_slice(&v.to_bytes().map_err(|e| tonic::Status::invalid_argument(e.to_string()))?);
                }
            }
            if stall {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            let mut s = req.into_inner();
            while s.message().await?.is_some() {}
            if sleep && !stall {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            match status {
                Some(c) => Err(tonic::Status::new(tonic::Code::from(c), format!("the test path's status {c}"))),
                None => Ok(tonic::Response::new(Bytes::from(out))),
            }
        })
    }
}

/// U2-stream's handler: every message decoded with prost as M5; the ids required on the
/// first message; the answer is the data byte count (u64 LE), plus, with `check`, the
/// SHA-256 of every message's bytes as received.
#[derive(Clone)]
struct StreamAnswer {
    check: bool,
}

impl tonic::server::ClientStreamingService<Bytes> for StreamAnswer {
    type Response = Bytes;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<tonic::Response<Bytes>, tonic::Status>> + Send>>;
    fn call(&mut self, req: tonic::Request<tonic::Streaming<Bytes>>) -> Self::Future {
        let check = self.check;
        Box::pin(async move {
            use prost::Message;
            use sha2::Digest;
            let mut s = req.into_inner();
            let (mut total, mut first) = (0u64, true);
            let mut h = sha2::Sha256::new();
            while let Some(m) = s.message().await? {
                if check {
                    h.update(&m);
                }
                let v = shapes_prost::shapes::UploadResultDataMessage::decode(m)
                    .map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
                let u = v.upload.ok_or_else(|| tonic::Status::invalid_argument("a message without upload"))?;
                if first && (u.session_id.is_empty() || u.result_id.is_empty()) {
                    return Err(tonic::Status::invalid_argument("the first message carries no ids"));
                }
                first = false;
                total += u.data_chunk.len() as u64;
            }
            let mut out = total.to_le_bytes().to_vec();
            if check {
                out.extend_from_slice(&h.finalize());
            }
            Ok(tonic::Response::new(Bytes::from(out)))
        })
    }
}

#[derive(Clone)]
struct Answer {
    fetch: Arc<Bytes>,
    push: bool,
    /// U1-unary (direction `c`): decode the request as M5 with prost, answer empty.
    upload: bool,
    /// FETCH_SHORT: P2.2 one byte short (a planted wrong response).
    short: bool,
}

impl tonic::server::UnaryService<Bytes> for Answer {
    type Response = Bytes;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<tonic::Response<Bytes>, tonic::Status>> + Send>>;
    fn call(&mut self, req: tonic::Request<Bytes>) -> Self::Future {
        let (fetch, push, upload, short) = (self.fetch.clone(), self.push, self.upload, self.short);
        Box::pin(async move {
            if upload {
                use prost::Message;
                let b = req.into_inner();
                return match shapes_prost::shapes::UploadResultDataMessage::decode(b) {
                    Ok(v) if v.upload.as_ref().map_or(false, |u| !u.data_chunk.is_empty()) => Ok(tonic::Response::new(Bytes::new())),
                    Ok(_) => Err(tonic::Status::invalid_argument("empty UploadResultDataMessage")),
                    Err(e) => Err(tonic::Status::invalid_argument(e.to_string())),
                };
            }
            if push {
                use prost::Message;
                let b = req.into_inner();
                match shapes_prost::shapes::ListTasksDetailedResponse::decode(b) {
                    Ok(v) if !v.tasks.is_empty() => Ok(tonic::Response::new(Bytes::new())),
                    Ok(_) => Err(tonic::Status::invalid_argument("empty ListTasksDetailedResponse")),
                    Err(e) => Err(tonic::Status::invalid_argument(e.to_string())),
                }
            } else if short {
                Ok(tonic::Response::new(fetch.slice(..fetch.len() - 1)))
            } else {
                Ok(tonic::Response::new((*fetch).clone()))
            }
        })
    }
}


/// The pre-serialised P2.2 response, checked against the manifest (requirement 13).
pub fn p22_response() -> Vec<u8> {
    let v = harness::arms_m2::prost_arm::value(harness::arms_m2::P2_2);
    let bytes = prost::Message::encode_to_vec(&v);
    let man = harness::manifest::Manifest::load();
    assert_eq!(harness::manifest::sha(&bytes), man.row("P2.2").sha256, "P2.2 bytes differ from the manifest");
    bytes
}

/// Serve the grid on a Unix socket at `path` (removed first if a stale one is there) until
/// the future is dropped. `ready` is called once the socket is bound.
pub async fn serve_uds(path: std::path::PathBuf, pinned: bool, fetch: Vec<u8>, ready: impl FnOnce()) {
    let _ = std::fs::remove_file(&path);
    let listener = tokio::net::UnixListener::bind(&path).expect("bind the grid's unix socket");
    let incoming = tokio_stream::wrappers::UnixListenerStream::new(listener);
    let mut b = tonic::transport::Server::builder();
    if pinned {
        b = b
            .initial_stream_window_size(Some(4 * 1024 * 1024))
            .initial_connection_window_size(Some(4 * 1024 * 1024))
            .http2_adaptive_window(Some(false));
    }
    let svc = Svc { fetch: Arc::new(Bytes::from(fetch)) };
    ready();
    b.add_service(svc).serve_with_incoming(incoming).await.unwrap();
}

/// A server in THIS process, on its own runtime (the counting build's per-call counts).
pub fn spawn_in_process(path: std::path::PathBuf, pinned: bool) -> tokio::runtime::Runtime {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let fetch = p22_response();
    rt.spawn(serve_uds(path, pinned, fetch, move || { let _ = tx.send(()); }));
    rx.recv().expect("in-process server");
    rt
}
