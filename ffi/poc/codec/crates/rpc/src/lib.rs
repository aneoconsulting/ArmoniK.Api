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

use bytes::{Buf, BufMut, Bytes};
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
        // Optimisation R1: `copy_to_bytes` already yields an owned `Bytes` (zero-copy
        // when the frame is one contiguous chunk); copying it into a second buffer
        // bought nothing.
        let n = src.remaining();
        Ok(Some(src.copy_to_bytes(n)))
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

// ---- the FRAMED send path (optimisation T1, option 3; owner: optional, labelled) ----------
//
// `Grpc::unary` with `RawCodec` copies the request once: tonic's `Encoder` API only offers
// `&mut EncodeBuf`, so `RawEncoder` puts the caller's `Bytes` into tonic's buffer. The
// framed path keeps tonic's `Channel` (a tower service over `http::Request<Body>`, which
// adds the origin and `user-agent` itself) and sends the gRPC message as TWO body frames,
// the 5-byte length prefix and the caller's `Bytes`, so the message is never copied on the
// host. The request is built as tonic 0.14.6's `GrpcConfig::prepare_request` builds it for
// a `Grpc::new(channel)` (default origin, no compression, no metadata, no deadline): POST,
// HTTP/2, the path as the URI, `te: trailers`, `content-type: application/grpc`, nothing
// else (`grpc-accept-encoding` only when accept-compression is enabled, which it is not
// here; `grpc-timeout` only from a deadline, never set here). The response is taken as
// `Grpc::streaming` + `client_streaming` take it: trailers-only status from the headers
// (`Status::from_header_map`), then `Streaming::new_response(RawDecoder, ...)`, the one
// message, the trailers (their grpc-status checked by `Streaming`). Compression stays OFF
// on both paths (no send/accept encodings are configured on either). The send size limit
// is `encode_item`'s: `max_send` (tonic's default is no limit, `usize::MAX`, which is what
// every caller here passes -- the core does not apply `ak_client_opts.max_send_message` on
// either path) and the 4 GiB prefix limit.

/// The two frames of one length-prefixed gRPC message, uncompressed.
struct Framed {
    hdr: Option<Bytes>,
    msg: Option<Bytes>,
}

impl http_body::Body for Framed {
    type Data = Bytes;
    type Error = tonic::Status;
    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Bytes>, tonic::Status>>> {
        let next = match self.hdr.take() {
            Some(h) => Some(h),
            None => self.msg.take(),
        };
        std::task::Poll::Ready(next.map(|b| Ok(http_body::Frame::data(b))))
    }
    fn is_end_stream(&self) -> bool {
        self.hdr.is_none() && self.msg.is_none()
    }
    // size_hint left at its default (unknown), as tonic's EncodeBody leaves it, so hyper adds
    // no content-length the codec path does not send.
}

/// One unary call over `svc` (a tonic `Channel`) whose request is `msg`, sent without a copy
/// (see the section comment). `max_send`: the largest message sent, `None` = tonic's default.
pub async fn unary_framed<T>(
    mut svc: T,
    path: http::uri::PathAndQuery,
    msg: Bytes,
    max_send: Option<usize>,
) -> Result<Bytes, tonic::Status>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    // encode_item's limits, with its messages.
    let len = msg.len();
    let limit = max_send.unwrap_or(usize::MAX);
    if len > limit {
        return Err(tonic::Status::out_of_range(format!(
            "Error, encoded message length too large: found {len} bytes, the limit is: {limit} bytes"
        )));
    }
    if len > u32::MAX as usize {
        return Err(tonic::Status::resource_exhausted(format!(
            "Cannot return body with more than 4GB of data but got {len} bytes"
        )));
    }
    let mut hdr = [0u8; 5];
    hdr[1..].copy_from_slice(&(len as u32).to_be_bytes());
    // An empty message is the prefix alone (no empty DATA frame after it).
    let msg = if msg.is_empty() { None } else { Some(msg) };
    let body = tonic::body::Body::new(Framed { hdr: Some(Bytes::copy_from_slice(&hdr)), msg });
    let uri = http::Uri::from_parts({
        let mut p = http::uri::Parts::default();
        p.path_and_query = Some(path);
        p
    })
    .map_err(|e| tonic::Status::internal(format!("uri: {e}")))?;
    let req = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .version(http::Version::HTTP_2)
        .header(http::header::TE, http::HeaderValue::from_static("trailers"))
        .header(http::header::CONTENT_TYPE, http::HeaderValue::from_static("application/grpc"))
        .body(body)
        .map_err(|e| tonic::Status::internal(format!("request: {e}")))?;
    std::future::poll_fn(|cx| svc.poll_ready(cx)).await.map_err(|e| tonic::Status::unknown(format!("ready: {}", e.into())))?;
    let resp = svc.call(req).await.map_err(|e| tonic::Status::unknown(format!("call: {}", e.into())))?;
    // Grpc::create_response, without compression.
    let status_code = resp.status();
    let mut stream = match tonic::Status::from_header_map(resp.headers()) {
        Some(st) if st.code() != tonic::Code::Ok => return Err(st),
        Some(_) => tonic::codec::Streaming::new_empty(RawDecoder, resp.into_body()),
        None => tonic::codec::Streaming::new_response(RawDecoder, resp.into_body(), status_code, None, None),
    };
    // Grpc::client_streaming: the one message, then the trailers.
    let m = stream.message().await?.ok_or_else(|| tonic::Status::internal("Missing response message."))?;
    stream.trailers().await?;
    Ok(m)
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
