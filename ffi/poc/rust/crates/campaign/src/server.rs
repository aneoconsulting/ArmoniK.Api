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

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::sync::Arc;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

pub const FETCH: &str = "/armonik.ffi.campaign.v1.Grid/Fetch";
pub const PUSH: &str = "/armonik.ffi.campaign.v1.Grid/Push";

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
        let mut b = BytesMut::with_capacity(n);
        b.put_slice(&src.copy_to_bytes(n));
        Ok(Some(b.freeze()))
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
        let fetch = self.fetch.clone();
        let push = req.uri().path() == PUSH;
        Box::pin(async move {
            let mut grpc = tonic::server::Grpc::new(Raw);
            Ok(grpc.unary(Answer { fetch, push }, req.map(tonic::body::Body::new)).await)
        })
    }
}

#[derive(Clone)]
struct Answer {
    fetch: Arc<Bytes>,
    push: bool,
}

impl tonic::server::UnaryService<Bytes> for Answer {
    type Response = Bytes;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<tonic::Response<Bytes>, tonic::Status>> + Send>>;
    fn call(&mut self, req: tonic::Request<Bytes>) -> Self::Future {
        let (fetch, push) = (self.fetch.clone(), self.push);
        Box::pin(async move {
            if push {
                use prost::Message;
                let b = req.into_inner();
                match shapes_prost::shapes::ListTasksDetailedResponse::decode(b) {
                    Ok(v) if !v.tasks.is_empty() => Ok(tonic::Response::new(Bytes::new())),
                    Ok(_) => Err(tonic::Status::invalid_argument("empty ListTasksDetailedResponse")),
                    Err(e) => Err(tonic::Status::invalid_argument(e.to_string())),
                }
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
