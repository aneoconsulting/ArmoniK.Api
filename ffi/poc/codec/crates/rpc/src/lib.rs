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
/// For the core's streamed call (ak-core does not depend on tokio-stream itself).
pub use tokio_stream::wrappers::ReceiverStream;
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

// ---- the client calls: reference and FRAMED send paths, one call configuration --------
//
// The REFERENCE path is tonic's Grpc with the raw-bytes codec: RawEncoder copies every
// message into tonic's buffer. The FRAMED path keeps tonic's Channel (a tower service over
// `http::Request<Body>`, which adds the origin and `user-agent` itself) and sends every
// message as TWO body frames, the 5-byte gRPC prefix and the caller's Bytes, so the message
// is never copied on the host (optimisation T1 option 3; ABI v1 section 9). Its request is
// built as tonic 0.14.6's `GrpcConfig::prepare_request` builds it for `Grpc::new(channel)`:
// the call's metadata (reserved headers dropped, as tonic's sanitising does; grpc-timeout
// among them when a deadline is set), then POST, HTTP/2, the path, `te: trailers`,
// `content-type: application/grpc`. Its response is taken as `Grpc::create_response` +
// `client_streaming` take it: trailers-only status from the headers, else
// `Streaming::new_response(RawDecoder, ...)`, the one message, the trailers. Compression is
// OFF on both paths.
//
// `CallCfg` is the same on both paths (ABI v1 section 9, streaming as built, and D44):
//   max_send  None = no limit (tonic's default); a message above it is refused BEFORE
//             anything is sent, as `CallErr::Limit` (AK_ERR_LIMIT at the ABI);
//   max_recv  None = tonic's 4 MiB; a response above it fails the call with
//             RESOURCE_EXHAUSTED (tonic's own check, whose OUT_OF_RANGE is translated);
//   metadata  the call's metadata, grpc-timeout included when a deadline is set;
//   deadline  enforced here too (tokio's timer): DEADLINE_EXCEEDED.

/// One call's configuration (see the section comment).
#[derive(Clone, Default)]
pub struct CallCfg {
    pub max_send: Option<usize>,
    pub max_recv: Option<usize>,
    pub metadata: tonic::metadata::MetadataMap,
    pub deadline: Option<std::time::Duration>,
}

impl CallCfg {
    /// Limits only (the unary deliveries).
    pub fn limits(max_send: Option<usize>, max_recv: Option<usize>) -> Self {
        CallCfg { max_send, max_recv, ..Default::default() }
    }
    /// Set the deadline: enforced by the caller's timer, and sent as grpc-timeout exactly as
    /// tonic's `Request::set_timeout` writes it.
    pub fn set_deadline(&mut self, d: std::time::Duration) {
        let mut r = tonic::Request::new(());
        r.set_timeout(d);
        if let Some(v) = r.metadata().get("grpc-timeout") {
            self.metadata.insert("grpc-timeout", v.clone());
        }
        self.deadline = Some(d);
    }
}

/// Why a call failed: a message refused by the send limit before anything was sent, or the
/// call's gRPC status (the receive limit's is RESOURCE_EXHAUSTED).
#[derive(Debug)]
pub enum CallErr {
    Limit(usize, usize),
    Status(tonic::Status),
}

impl std::fmt::Display for CallErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallErr::Limit(n, l) => write!(f, "a message of {n} bytes is above the send limit of {l} bytes; nothing sent"),
            CallErr::Status(s) => write!(f, "{s}"),
        }
    }
}

impl From<tonic::Status> for CallErr {
    fn from(s: tonic::Status) -> Self {
        CallErr::Status(s)
    }
}

impl CallErr {
    /// The gRPC status code (the send-limit refusal has none: it is not a call outcome).
    pub fn code(&self) -> Option<tonic::Code> {
        match self {
            CallErr::Limit(..) => None,
            CallErr::Status(s) => Some(s.code()),
        }
    }
    /// A limit of this client refused it (send before sending, or receive).
    pub fn is_limit(&self) -> bool {
        match self {
            CallErr::Limit(..) => true,
            CallErr::Status(s) => s.code() == tonic::Code::ResourceExhausted && s.message().starts_with(RECV_LIMIT),
        }
    }
}

const RECV_LIMIT: &str = "Error, decoded message length too large";

/// tonic reports its receive limit as OUT_OF_RANGE; the ABI says RESOURCE_EXHAUSTED (as gRPC
/// does). Only tonic's own local check is translated (by its message).
fn recv_limit(s: tonic::Status) -> tonic::Status {
    if s.code() == tonic::Code::OutOfRange && s.message().starts_with(RECV_LIMIT) {
        tonic::Status::resource_exhausted(s.message().to_string())
    } else {
        s
    }
}

fn check_send(len: usize, max: Option<usize>) -> Result<(), CallErr> {
    match max {
        Some(l) if len > l => Err(CallErr::Limit(len, l)),
        _ if len > u32::MAX as usize => Err(CallErr::Status(tonic::Status::resource_exhausted(format!(
            "Cannot return body with more than 4GB of data but got {len} bytes")))),
        _ => Ok(()),
    }
}

/// The deadline, enforced by this side's timer (grpc-timeout was sent with the request).
///
/// The server enforces the same grpc-timeout, and tonic's server reports its expiry as
/// CANCELLED "Timeout expired"; when that reply arrives before this side's timer (measured:
/// it does, on a Unix socket), it is the same deadline and is reported as the gRPC client
/// contract says, DEADLINE_EXCEEDED.
async fn with_deadline<T>(d: Option<std::time::Duration>, f: impl std::future::Future<Output = Result<T, CallErr>>) -> Result<T, CallErr> {
    let expired = || CallErr::Status(tonic::Status::deadline_exceeded("Deadline expired before operation could complete"));
    match d {
        None => f.await,
        Some(d) => match tokio::time::timeout(d, f).await {
            Ok(Err(CallErr::Status(s))) if s.code() == tonic::Code::Cancelled && s.message() == "Timeout expired" => Err(expired()),
            Ok(r) => r,
            Err(_) => Err(expired()),
        },
    }
}

fn grpc(chan: tonic::transport::Channel, cfg: &CallCfg) -> tonic::client::Grpc<tonic::transport::Channel> {
    let mut g = tonic::client::Grpc::new(chan);
    if let Some(l) = cfg.max_send {
        g = g.max_encoding_message_size(l);
    }
    if let Some(l) = cfg.max_recv {
        g = g.max_decoding_message_size(l);
    }
    g
}

/// Reference unary call (tonic's Grpc::unary with the raw-bytes codec).
pub async fn unary_raw(chan: tonic::transport::Channel, path: http::uri::PathAndQuery, body: Bytes, cfg: &CallCfg) -> Result<Bytes, CallErr> {
    check_send(body.len(), cfg.max_send)?;
    let md = cfg.metadata.clone();
    with_deadline(cfg.deadline, async move {
        let mut g = grpc(chan, cfg);
        g.ready().await.map_err(|e| tonic::Status::unavailable(format!("ready: {e}")))?;
        let req = tonic::Request::from_parts(md, Default::default(), body);
        g.unary(req, path, RawCodec).await.map(|r| r.into_inner()).map_err(|s| CallErr::Status(recv_limit(s)))
    })
    .await
}

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

fn prefix(len: usize) -> Bytes {
    let mut hdr = [0u8; 5];
    hdr[1..].copy_from_slice(&(len as u32).to_be_bytes());
    Bytes::copy_from_slice(&hdr)
}

/// Framed unary call (the section comment), with the defaults of `CallCfg` but `max_send`.
pub async fn unary_framed<T>(svc: T, path: http::uri::PathAndQuery, msg: Bytes, max_send: Option<usize>) -> Result<Bytes, tonic::Status>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    unary_framed_cfg(svc, path, msg, &CallCfg::limits(max_send, None)).await.map_err(|e| match e {
        CallErr::Status(s) => s,
        CallErr::Limit(n, l) => tonic::Status::out_of_range(format!(
            "Error, encoded message length too large: found {n} bytes, the limit is: {l} bytes")),
    })
}

/// Framed unary call whose body ALREADY carries its 5-byte gRPC prefix (the core's
/// encoder headroom, ak_rt::Enc::take_framed, or a copy made with the prefix): sent as ONE
/// body frame, as tonic's EncodeBody sends prefix and message in one buffer (owner,
/// 2026-09-28). The send limit applies to the message, not the prefix.
pub async fn unary_preframed_cfg<T>(svc: T, path: http::uri::PathAndQuery, framed: Bytes, cfg: &CallCfg) -> Result<Bytes, CallErr>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    check_send(framed.len().saturating_sub(5), cfg.max_send)?;
    let body = tonic::body::Body::new(Framed { hdr: Some(framed), msg: None });
    let req = framed_request(path, body, &cfg.metadata)?;
    with_deadline(cfg.deadline, framed_call(svc, req, cfg.max_recv)).await
}

/// Framed unary call with a full call configuration.
pub async fn unary_framed_cfg<T>(svc: T, path: http::uri::PathAndQuery, msg: Bytes, cfg: &CallCfg) -> Result<Bytes, CallErr>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    check_send(msg.len(), cfg.max_send)?;
    let hdr = prefix(msg.len());
    // An empty message is the prefix alone (no empty DATA frame after it).
    let msg = if msg.is_empty() { None } else { Some(msg) };
    let body = tonic::body::Body::new(Framed { hdr: Some(hdr), msg });
    let req = framed_request(path, body, &cfg.metadata)?;
    with_deadline(cfg.deadline, framed_call(svc, req, cfg.max_recv)).await
}

/// The request as tonic's GrpcConfig::prepare_request builds it (see the section comment).
fn framed_request(path: http::uri::PathAndQuery, body: tonic::body::Body, md: &tonic::metadata::MetadataMap)
    -> Result<http::Request<tonic::body::Body>, tonic::Status> {
    let uri = http::Uri::from_parts({
        let mut p = http::uri::Parts::default();
        p.path_and_query = Some(path);
        p
    })
    .map_err(|e| tonic::Status::internal(format!("uri: {e}")))?;
    let mut req = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .version(http::Version::HTTP_2)
        .body(body)
        .map_err(|e| tonic::Status::internal(format!("request: {e}")))?;
    let mut h = md.clone().into_headers();
    // tonic's MetadataMap::into_sanitized_headers: the reserved headers are dropped.
    for r in ["te", "content-type", "grpc-message", "grpc-message-type", "grpc-status"] {
        h.remove(r);
    }
    *req.headers_mut() = h;
    req.headers_mut().insert(http::header::TE, http::HeaderValue::from_static("trailers"));
    req.headers_mut().insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("application/grpc"));
    Ok(req)
}

/// Send `req` over `svc` and take the one response message as Grpc::create_response and
/// client_streaming do (trailers-only status, Streaming, the message, the trailers).
async fn framed_call<T>(mut svc: T, req: http::Request<tonic::body::Body>, max_recv: Option<usize>) -> Result<Bytes, CallErr>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    std::future::poll_fn(|cx| svc.poll_ready(cx)).await.map_err(|e| tonic::Status::from_error(e.into()))?;
    let resp = svc.call(req).await.map_err(|e| tonic::Status::from_error(e.into()))?;
    // Grpc::create_response, without compression.
    let status_code = resp.status();
    let mut stream = match tonic::Status::from_header_map(resp.headers()) {
        Some(st) if st.code() != tonic::Code::Ok => return Err(CallErr::Status(st)),
        Some(_) => tonic::codec::Streaming::new_empty(RawDecoder, resp.into_body()),
        None => tonic::codec::Streaming::new_response(RawDecoder, resp.into_body(), status_code, None, max_recv),
    };
    // Grpc::client_streaming: the one message, then the trailers.
    let m = stream.message().await.map_err(recv_limit)?.ok_or_else(|| tonic::Status::internal("Missing response message."))?;
    stream.trailers().await.map_err(recv_limit)?;
    Ok(m)
}

// ---- client streaming (U2-stream: ABI v1 section 9's streamed call) -----------------------

/// Reference: tonic's client streaming with the raw-bytes codec, messages from a channel.
pub async fn client_streaming_raw(
    chan: tonic::transport::Channel,
    path: http::uri::PathAndQuery,
    rx: tokio::sync::mpsc::Receiver<Bytes>,
    cfg: &CallCfg,
) -> Result<Bytes, CallErr> {
    client_streaming_raw_cfg(chan, path, tokio_stream::wrappers::ReceiverStream::new(rx), cfg).await
}

/// Reference, from any stream of messages, default configuration (the harness's D and F).
pub async fn client_streaming_raw_from<S>(chan: tonic::transport::Channel, path: http::uri::PathAndQuery, msgs: S) -> Result<Bytes, tonic::Status>
where
    S: tokio_stream::Stream<Item = Bytes> + Send + 'static,
{
    client_streaming_raw_cfg(chan, path, msgs, &CallCfg::default()).await.map_err(|e| match e {
        CallErr::Status(s) => s,
        e => tonic::Status::internal(e.to_string()),
    })
}

/// Reference client streaming with a call configuration. Per message the send limit is the
/// caller's to check (ak_call_send refuses before queueing); tonic's own encoding limit is set
/// to the same value.
pub async fn client_streaming_raw_cfg<S>(chan: tonic::transport::Channel, path: http::uri::PathAndQuery, msgs: S, cfg: &CallCfg) -> Result<Bytes, CallErr>
where
    S: tokio_stream::Stream<Item = Bytes> + Send + 'static,
{
    let md = cfg.metadata.clone();
    with_deadline(cfg.deadline, async move {
        let mut g = grpc(chan, cfg);
        g.ready().await.map_err(|e| tonic::Status::unavailable(format!("ready: {e}")))?;
        let req = tonic::Request::from_parts(md, Default::default(), msgs);
        g.client_streaming(req, path, RawCodec).await.map(|r| r.into_inner()).map_err(|s| CallErr::Status(recv_limit(s)))
    })
    .await
}

/// The framed body of a message stream: per message its 5-byte prefix, then the message
/// (an empty message is the prefix alone); the send limit per message.
struct FramedStream<S> {
    msgs: S,
    pending: Option<Bytes>,
    max_send: usize,
    done: bool,
    /// Every message already carries its 5-byte prefix: yielded as ONE frame.
    preframed: bool,
}

impl<S> http_body::Body for FramedStream<S>
where
    S: tokio_stream::Stream<Item = Bytes> + Unpin,
{
    type Data = Bytes;
    type Error = tonic::Status;
    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Bytes>, tonic::Status>>> {
        use std::task::Poll;
        if let Some(m) = self.pending.take() {
            return Poll::Ready(Some(Ok(http_body::Frame::data(m))));
        }
        if self.done {
            return Poll::Ready(None);
        }
        match std::pin::Pin::new(&mut self.msgs).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Some(m)) if self.preframed => {
                let len = m.len().saturating_sub(5);
                if len > self.max_send {
                    return Poll::Ready(Some(Err(tonic::Status::out_of_range(format!(
                        "Error, encoded message length too large: found {len} bytes, the limit is: {} bytes", self.max_send)))));
                }
                Poll::Ready(Some(Ok(http_body::Frame::data(m))))
            }
            Poll::Ready(Some(m)) => {
                let len = m.len();
                if len > self.max_send || len > u32::MAX as usize {
                    return Poll::Ready(Some(Err(tonic::Status::out_of_range(format!(
                        "Error, encoded message length too large: found {len} bytes, the limit is: {} bytes", self.max_send)))));
                }
                if len > 0 {
                    self.pending = Some(m);
                }
                Poll::Ready(Some(Ok(http_body::Frame::data(prefix(len)))))
            }
        }
    }
    fn is_end_stream(&self) -> bool {
        self.done && self.pending.is_none()
    }
}

/// Framed client streaming over `svc`, default configuration but `max_send` (the harness).
pub async fn client_streaming_framed<T, S>(svc: T, path: http::uri::PathAndQuery, msgs: S, max_send: Option<usize>) -> Result<Bytes, tonic::Status>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S: tokio_stream::Stream<Item = Bytes> + Unpin + Send + 'static,
{
    client_streaming_framed_cfg(svc, path, msgs, &CallCfg::limits(max_send, None)).await.map_err(|e| match e {
        CallErr::Status(s) => s,
        e => tonic::Status::internal(e.to_string()),
    })
}

/// Framed client streaming with a call configuration.
pub async fn client_streaming_framed_cfg<T, S>(svc: T, path: http::uri::PathAndQuery, msgs: S, cfg: &CallCfg) -> Result<Bytes, CallErr>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S: tokio_stream::Stream<Item = Bytes> + Unpin + Send + 'static,
{
    let body = tonic::body::Body::new(FramedStream { msgs, pending: None, max_send: cfg.max_send.unwrap_or(usize::MAX), done: false, preframed: false });
    let req = framed_request(path, body, &cfg.metadata)?;
    with_deadline(cfg.deadline, framed_call(svc, req, cfg.max_recv)).await
}

/// Framed client streaming whose messages ALREADY carry their 5-byte prefix (the core's
/// framed default): each message ONE body frame.
pub async fn client_streaming_preframed_cfg<T, S>(svc: T, path: http::uri::PathAndQuery, msgs: S, cfg: &CallCfg) -> Result<Bytes, CallErr>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as http_body::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S: tokio_stream::Stream<Item = Bytes> + Unpin + Send + 'static,
{
    let body = tonic::body::Body::new(FramedStream { msgs, pending: None, max_send: cfg.max_send.unwrap_or(usize::MAX), done: false, preframed: true });
    let req = framed_request(path, body, &cfg.metadata)?;
    with_deadline(cfg.deadline, framed_call(svc, req, cfg.max_recv)).await
}

/// A message copied with its 5-byte gRPC prefix in front: ONE buffer, for the preframed
/// paths (the copy a copying entry makes anyway, not a second one).
pub fn framed_copy(msg: &[u8]) -> Bytes {
    let mut v = Vec::with_capacity(msg.len() + 5);
    v.push(0);
    v.extend_from_slice(&(msg.len() as u32).to_be_bytes());
    v.extend_from_slice(msg);
    Bytes::from(v)
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
