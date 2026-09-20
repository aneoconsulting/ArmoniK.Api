//! Stage 4's transport: one unary RPC over loopback, and a codec that moves opaque bytes.
//!
//! ABI v1 section 9: "The RPC half does not know the schema. It moves opaque bytes and
//! dispatches on a path string, so it serves every RPC unchanged and adding one is a table
//! row." This crate is that half, and the point of it is that **nothing here is generated
//! and nothing here mentions a message type**. If the crossing count per RPC turns out to be
//! a function of field count, it is because something in this file knows about fields, and
//! nothing in it does.
//!
//! Deliberately small (`design/SHAPES.md`: "The RPC arm. Smaller, and deliberately so").
//! No TLS, no retry, no metadata, no deadlines, no streaming, no failure injection, no
//! server-side measurement.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};

/// A gRPC codec that does not know the schema: it hands the framing layer the bytes it was
/// given and hands back the bytes it received. Both arms of stage 4 use it, so the
/// comparison is not confounded by two different framing paths.
#[derive(Default, Clone)]
pub struct RawCodec;

#[derive(Default, Clone)]
pub struct RawEncoder;

#[derive(Default, Clone)]
pub struct RawDecoder;

impl Encoder for RawEncoder {
    type Item = Bytes;
    type Error = tonic::Status;
    fn encode(&mut self, item: Bytes, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.put_slice(&item);
        Ok(())
    }
}

impl Decoder for RawDecoder {
    type Item = Bytes;
    type Error = tonic::Status;
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Bytes>, Self::Error> {
        let n = src.remaining();
        let mut b = BytesMut::with_capacity(n);
        b.put_slice(&src.copy_to_bytes(n));
        Ok(Some(b.freeze()))
    }
}

impl Codec for RawCodec {
    type Encode = Bytes;
    type Decode = Bytes;
    type Encoder = RawEncoder;
    type Decoder = RawDecoder;
    fn encoder(&mut self) -> Self::Encoder {
        RawEncoder
    }
    fn decoder(&mut self) -> Self::Decoder {
        RawDecoder
    }
}

pub const PATH: &str = "/armonik.ffi.shapes.v1.Bench/Unary";

// ---- the server ---------------------------------------------------------------------

use http_body_util::BodyExt;
use std::sync::Arc;

/// A server that answers `PATH` with a fixed body. It exists to be the other end of a
/// loopback call and nothing else: no schema, no work, so that what the client arms measure
/// is the client and the loopback and not a server implementation.
pub struct Server {
    pub addr: std::net::SocketAddr,
    _handle: tokio::task::JoinHandle<()>,
}

/// **TCP_NODELAY is set on the accepted socket, and the default was changed to set it.**
///
/// The rust slice found the defect and reported it rather than flipping the default, which
/// was the right call under R0; the flip is the aggregating session's and it is taken.
/// `Server::builder()` defaults `tcp_nodelay` to true, but tonic documents that
/// `tcp_nodelay` and `tcp_keepalive` are **ignored when the server is driven by
/// `serve_with_incoming`** (`transport/server/mod.rs:701`), and `TcpIncoming::from(listener)`
/// leaves its own `nodelay` at `None` (`incoming.rs:120`), so `set_accepted_socket_options`
/// never touched the socket. The server end kept Nagle ON while tonic's client had it off by
/// default. A gRPC response is HEADERS, then DATA, then TRAILERS; with Nagle on the writer
/// the second small write waits for the peer's ACK of the first, and Linux's delayed-ACK
/// timer is 40 ms.
///
/// **What it was worth: 32,416 us of wall clock per call became 2,145, and on a 1 KB
/// response 44,041 became 149.** With it set, loopback TCP and a Unix socket agree on both
/// payloads and both columns, so **the transport gap this branch has been reporting was a
/// harness defect and not a transport.**
///
/// The default is flipped rather than left, because a benchmark server with Nagle on is a
/// defect every future arm would re-measure, and because every production gRPC stack
/// disables Nagle. `serve_nagle` keeps the defective form so the artifact stays
/// reproducible; nothing should measure against it except a run that is demonstrating it.
pub async fn serve(response: Bytes) -> Server {
    serve_opts(response, Some(true)).await
}

/// The defective form, kept only so the artifact above can be reproduced on demand.
/// **Not a transport row.** A figure taken against this server is measuring Nagle.
pub async fn serve_nagle(response: Bytes) -> Server {
    serve_opts(response, None).await
}

/// Deprecated spelling of `serve`, kept so the rust slice's stage 6 harness still builds.
pub async fn serve_nodelay(response: Bytes) -> Server {
    serve(response).await
}

async fn serve_opts(response: Bytes, nodelay: Option<bool>) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let response = Arc::new(response);
    let handle = tokio::spawn(async move {
        let svc = BenchService { response };
        let incoming =
            tonic::transport::server::TcpIncoming::from(listener).with_nodelay(nodelay);
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });
    // Give the acceptor a moment to be ready; the first connect retries anyway.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Server { addr, _handle: handle }
}

/// The same server over a **Unix domain socket**, which is what `design/SHAPES.md` makes the
/// primary transport row and what ArmoniK's client actually dials. It exists so the core's
/// own UDS path has something to dial in a test rather than only in a slice's harness.
pub struct UdsServer {
    pub path: std::path::PathBuf,
    _handle: tokio::task::JoinHandle<()>,
}

impl Drop for UdsServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub async fn serve_uds(response: Bytes, path: std::path::PathBuf) -> UdsServer {
    let _ = std::fs::remove_file(&path);
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let response = Arc::new(response);
    let handle = tokio::spawn(async move {
        let svc = BenchService { response };
        let incoming = tokio_stream::wrappers::UnixListenerStream::new(listener);
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    UdsServer { path, _handle: handle }
}

#[derive(Clone)]
struct BenchService {
    response: Arc<Bytes>,
}

impl tonic::server::NamedService for BenchService {
    const NAME: &'static str = "armonik.ffi.shapes.v1.Bench";
}

impl<B> tower_service::Service<http::Request<B>> for BenchService
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        let response = self.response.clone();
        Box::pin(async move {
            let mut grpc = tonic::server::Grpc::new(RawCodec);
            let svc = Answer { response };
            let res = grpc.unary(svc, req.map(tonic::body::Body::new)).await;
            Ok(res)
        })
    }
}

#[derive(Clone)]
struct Answer {
    response: Arc<Bytes>,
}

impl tonic::server::UnaryService<Bytes> for Answer {
    type Response = Bytes;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<tonic::Response<Bytes>, tonic::Status>> + Send>,
    >;
    fn call(&mut self, _req: tonic::Request<Bytes>) -> Self::Future {
        let r = self.response.clone();
        Box::pin(async move { Ok(tonic::Response::new((*r).clone())) })
    }
}

/// Keeps `BodyExt` in use; the trait is needed for the body bounds above.
#[allow(dead_code)]
fn _body_ext_used<B: http_body::Body>(b: B) -> http_body_util::combinators::UnsyncBoxBody<B::Data, B::Error>
where
    B: Send + 'static,
    B::Data: Send,
    B::Error: Send,
{
    b.boxed_unsync()
}
