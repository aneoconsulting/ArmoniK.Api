//! A gRPC service that answers slowly, and a raw client to call it with.
//!
//! Hand-rolled rather than generated: this crate has no protos and deliberately no `protoc` in its
//! build, so the codec moves opaque bytes and the service is one method that sleeps before replying.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use armonik_transport::reexports::hyper;
use armonik_transport::reexports::tonic::body::Body;
use armonik_transport::reexports::tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use armonik_transport::reexports::tonic::server::NamedService;
use armonik_transport::reexports::tonic::{Request, Response, Status};
use bytes::{Buf, BufMut, Bytes};
use tower_service::Service;

/// The one method the test service answers to.
pub const METHOD_PATH: &str = "/armonik_transport.test.Slow/Call";

/// What the service answers once it has finished sleeping, so a test can tell a real reply from an
/// empty one.
pub const REPLY: &[u8] = b"served";

/// A codec whose wire representation *is* the message: no framing beyond what gRPC already adds.
#[derive(Debug, Clone, Copy, Default)]
pub struct BytesCodec;

impl Codec for BytesCodec {
    type Encode = Bytes;
    type Decode = Bytes;
    type Encoder = Self;
    type Decoder = Self;

    fn encoder(&mut self) -> Self::Encoder {
        *self
    }

    fn decoder(&mut self) -> Self::Decoder {
        *self
    }
}

impl Encoder for BytesCodec {
    type Item = Bytes;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.reserve(item.len());
        dst.put_slice(&item);
        Ok(())
    }
}

impl Decoder for BytesCodec {
    type Item = Bytes;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.remaining();
        Ok(Some(src.copy_to_bytes(len)))
    }
}

/// A service that waits `delay` before answering, whatever it was sent.
#[derive(Clone)]
pub struct SlowService {
    delay: Duration,
}

impl SlowService {
    pub fn new(delay: Duration) -> Self {
        Self { delay }
    }
}

impl NamedService for SlowService {
    const NAME: &'static str = "armonik_transport.test.Slow";
}

/// The gRPC handler. `tonic::server::UnaryService` is a blanket implementation over a `tower` service
/// of the right shape rather than a trait to implement, so this is where the handler goes.
impl Service<Request<Bytes>> for SlowService {
    type Response = Response<Bytes>;
    type Error = Status;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: Request<Bytes>) -> Self::Future {
        let delay = self.delay;
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            Ok(Response::new(Bytes::from_static(REPLY)))
        })
    }
}

impl Service<hyper::Request<Body>> for SlowService {
    type Response = hyper::Response<Body>;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: hyper::Request<Body>) -> Self::Future {
        let mut handler = self.clone();
        Box::pin(async move {
            Ok(
                armonik_transport::reexports::tonic::server::Grpc::new(BytesCodec)
                    .unary(&mut handler, request)
                    .await,
            )
        })
    }
}

/// Serve `service` on an ephemeral loopback port and return its `http://` endpoint.
pub async fn serve(service: SlowService) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind the test server");
    let address = listener.local_addr().expect("the test server's address");

    tokio::spawn(async move {
        let incoming =
            armonik_transport::reexports::tonic::transport::server::TcpIncoming::from(listener);
        armonik_transport::reexports::tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming(incoming)
            .await
            .expect("serve the test service");
    });

    format!("http://{address}")
}

/// Make one unary call over `channel`, returning whatever gRPC says about it.
pub async fn call(
    channel: armonik_transport::reexports::tonic::transport::Channel,
) -> Result<Bytes, Status> {
    let path = armonik_transport::reexports::tonic::codegen::http::uri::PathAndQuery::try_from(
        METHOD_PATH,
    )
    .expect("a valid method path");

    let mut grpc = armonik_transport::reexports::tonic::client::Grpc::new(channel);
    // Generated clients always do this first, and `Grpc::unary` does not do it implicitly: skipping it
    // trips `tower::Buffer`'s "send_item called without first calling poll_reserve" assertion.
    grpc.ready()
        .await
        .map_err(|error| Status::unknown(format!("the channel was not ready: {error}")))?;

    let response = grpc
        .unary(Request::new(Bytes::from_static(b"ping")), path, BytesCodec)
        .await?;
    Ok(response.into_inner())
}

/// Build a [`HttpConfig`] from the string form, applying `set` to the arguments first.
///
/// Going through `ClientConfigArgs` keeps the parsing inside what is under test. It is a helper at all
/// because both structs are `#[non_exhaustive]`: a test outside the crate cannot write either as a
/// struct expression, and `..Default::default()` is the form that is forbidden.
#[allow(clippy::field_reassign_with_default)]
pub fn config(
    endpoint: &str,
    set: impl FnOnce(&mut armonik_transport::ClientConfigArgs),
) -> armonik_transport::HttpConfig {
    let mut args = armonik_transport::ClientConfigArgs::default();
    args.endpoint = endpoint.to_owned();
    args.allow_unsafe_connection = true;
    set(&mut args);
    armonik_transport::HttpConfig::from_config_args(args)
        .expect("the configuration should be valid")
}
