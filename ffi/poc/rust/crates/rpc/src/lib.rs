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

pub async fn serve(response: Bytes) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let response = Arc::new(response);
    let handle = tokio::spawn(async move {
        let svc = BenchService { response };
        let incoming = tonic::transport::server::TcpIncoming::from(listener);
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
