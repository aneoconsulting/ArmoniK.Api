//! Generic gRPC server: one [`Router`] type for every service, dispatching
//! into `tonic::server::Grpc` through a per-service routing table.
//!
//! `service!` emits one [`Routes`] impl per service (a const table, one entry
//! per RPC); the `tower::Service` plumbing below exists once. Handler futures
//! are boxed, exactly as in the tonic-generated servers this replaces.

use std::marker::PhantomData;
use std::sync::Arc;

use tonic::codegen::http;
use tonic::codegen::{BoxFuture, CompressionEncoding, EnabledCompressionEncodings};

use crate::rpc::{Rpc, Service};

use super::RequestContext;

/// How much of an unrouted path the UNIMPLEMENTED status repeats back.
const MAX_REPORTED_PATH: usize = 128;

/// Compression and message-size configuration, applied per call.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ServerConfig {
    accept_compression_encodings: EnabledCompressionEncodings,
    send_compression_encodings: EnabledCompressionEncodings,
    max_decoding_message_size: Option<usize>,
    max_encoding_message_size: Option<usize>,
}

/// An erased request handler: one per RPC, stored in [`Routes::ROUTES`].
///
/// Generic over the request body, like the tonic-generated servers this replaces
/// (`tonic-build-0.14.6/src/server.rs:151`). Fixing it at `tonic::body::Body` went unnoticed
/// because `Server::add_service` asks only for `Service<Request<tonic::body::Body>>`
/// (`transport/server/mod.rs:511`), and it is exactly what a router mounted on plain hyper
/// (`hyper::body::Incoming`), nested in an `axum::Router` through `route_service`, or sitting under
/// a layer that changes the body type, needs.
pub(crate) type RouteFn<S, B> =
    fn(
        Arc<S>,
        http::Request<B>,
        ServerConfig,
    ) -> BoxFuture<http::Response<tonic::body::Body>, std::convert::Infallible>;

/// The request stream handed to client-streaming trait methods.
pub(crate) type RequestStream<R> = futures::stream::BoxStream<'static, Result<R, tonic::Status>>;

/// The routing table of the service marker `Self` over a service
/// implementation `S`.
pub(crate) trait Routes<S: 'static, B: 'static>: Service {
    /// `(path, handler)`, one entry per RPC.
    const ROUTES: &'static [(&'static str, RouteFn<S, B>)];
}

/// A gRPC server for the service marker `Svc`, routing requests to the
/// service implementation `S`.
///
/// Mount it with `tonic::transport::Server::add_service`, nest it in an `axum::Router` with
/// `route_service`, or drive it straight from hyper: the `Service` impl takes any request body a
/// tonic-generated server would. There is no `with_interceptor` constructor; wrap the router in
/// `tonic::service::interceptor::InterceptedService::new` for that.
pub struct Router<Svc, S> {
    inner: Arc<S>,
    config: ServerConfig,
    _svc: PhantomData<fn() -> Svc>,
}

/// Hand-written, because the derive would demand `Svc: Debug` and the service markers `service!`
/// emits are unit structs implementing nothing. `Svc` is a `PhantomData<fn() -> Svc>` here, so it
/// holds no value to print and the bound was never about this type's contents: a downstream
/// `#[derive(Debug)]` over a struct with a `Router` field compiled on main and could not be made to
/// compile here.
impl<Svc, S: std::fmt::Debug> std::fmt::Debug for Router<Svc, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("inner", &self.inner)
            .field("config", &self.config)
            .finish()
    }
}

impl<Svc, S> Router<Svc, S> {
    /// Wrap a service implementation.
    pub fn new(inner: S) -> Self {
        Self::from_arc(Arc::new(inner))
    }

    /// Wrap an already-shared service implementation.
    pub fn from_arc(inner: Arc<S>) -> Self {
        Self {
            inner,
            config: ServerConfig::default(),
            _svc: PhantomData,
        }
    }

    /// Accept requests compressed with the given encoding.
    ///
    /// Every `CompressionEncoding` variant sits behind one of tonic's compression features; turn on
    /// this crate's `gzip` to have one to pass. Cargo features being additive, a dependent
    /// can also enable tonic's directly; these exist so it is discoverable from here.
    #[must_use]
    pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
        self.config.accept_compression_encodings.enable(encoding);
        self
    }

    /// Compress responses with the given encoding, if the client supports it.
    #[must_use]
    pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
        self.config.send_compression_encodings.enable(encoding);
        self
    }

    /// Limits the maximum size of a decoded message. Default: `4MB`.
    #[must_use]
    pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
        self.config.max_decoding_message_size = Some(limit);
        self
    }

    /// Limits the maximum size of an encoded message. Default: `usize::MAX`.
    #[must_use]
    pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
        self.config.max_encoding_message_size = Some(limit);
        self
    }

    /// Whether the router can accept a request. It always can: it holds a shared service
    /// implementation and dispatches into `tonic::server::Grpc`, with no queue to fill.
    ///
    /// Inherent, and therefore what `router.poll_ready(cx)` resolves to, because the trait method
    /// it shadows is generic over the request body: calling that one with no `call` nearby to pin
    /// the body type is ambiguous, and rustc's suggested fix names a type parameter that is not in
    /// scope at the call site. The two answer identically, for every body type.
    ///
    /// `tower::ServiceExt::ready` goes through the trait and has the same ambiguity with no
    /// inherent counterpart; name the body type there, as
    /// `ServiceExt::<http::Request<tonic::body::Body>>::ready(&mut router)`.
    pub fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::convert::Infallible>> {
        std::task::Poll::Ready(Ok(()))
    }
}

impl<Svc, S> Clone for Router<Svc, S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            config: self.config,
            _svc: PhantomData,
        }
    }
}

impl<Svc, S: 'static, B: 'static> tonic::codegen::Service<http::Request<B>> for Router<Svc, S>
where
    Svc: Routes<S, B>,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Router::poll_ready(self, cx)
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        for (path, handler) in Svc::ROUTES {
            if *path == req.uri().path() {
                return handler(Arc::clone(&self.inner), req, self.config);
            }
        }
        // No route: a method of another service, or one this crate does not expose. The path names
        // it, so the client's error says which, truncated because it is client-supplied and the
        // `grpc-message` header percent-encodes it: a maximal path would expand to roughly three
        // times its length in a response header. Tonic's own codegen answers with no message at
        // all, so this is new surface.
        let mut path = req.uri().path();
        let truncated = path.len() > MAX_REPORTED_PATH;
        if truncated {
            // Back to a character boundary, because `http` accepts non-ASCII UTF-8 in a path and
            // byte 128 of a client-supplied one lands mid-character often enough: slicing there
            // panics inside this synchronous body, which unwinds the hyper connection task and
            // every concurrent stream on it.
            let end = (0..=MAX_REPORTED_PATH)
                .rev()
                .find(|index| path.is_char_boundary(*index))
                .unwrap_or(0);
            path = &path[..end];
        }
        let status = tonic::Status::unimplemented(if truncated {
            format!("{path}... is not implemented")
        } else {
            format!("{path} is not implemented")
        });
        Box::pin(async move { Ok(status.into_http()) })
    }
}

impl<Svc: Service, S> tonic::server::NamedService for Router<Svc, S> {
    const NAME: &'static str = Svc::NAME;
}

/// One handler passed to `tonic::server::Grpc`: the service implementation,
/// the trait-method closure from the routing table, and the per-RPC span. The
/// four `tonic::server::*Service` impls below give it the four call shapes.
struct Handler<S, F> {
    inner: Arc<S>,
    handler: F,
    span: tracing::Span,
}

/// The traced, boxed response future shared by the unary and client-streaming
/// call shapes.
fn respond<T, Fut>(fut: Fut, span: tracing::Span) -> BoxFuture<tonic::Response<T>, tonic::Status>
where
    T: std::fmt::Debug,
    Fut: std::future::Future<Output = Result<T, tonic::Status>> + Send + 'static,
{
    Box::pin(tracing_futures::Instrument::instrument(
        async move {
            let res = fut.await;
            match &res {
                Ok(res) => tracing::trace!("Response: {res:?}"),
                Err(err) => tracing::trace!("Response: {err:?}"),
            }
            res.map(tonic::Response::new)
        },
        span,
    ))
}

/// The request stream, traced item by item, under a child span: it is polled while the handler
/// runs, so its events belong under the call.
fn traced_requests<R: Rpc>(
    streaming: tonic::Streaming<R>,
    span: &tracing::Span,
) -> RequestStream<R> {
    let stream = futures::StreamExt::map(streaming, |item| {
        match &item {
            Ok(item) => tracing::trace!("Request item: {item:?}"),
            Err(err) => tracing::trace!("Request item: {err:?}"),
        }
        item
    });
    futures::StreamExt::boxed(tracing_futures::Instrument::instrument(
        stream,
        tracing::trace_span!(parent: span, "stream"),
    ))
}

/// The traced, boxed response *stream* shared by the two server-streaming call shapes, mirroring
/// [`respond`] for the two that answer with one message.
fn respond_stream<R, Fut, St>(
    fut: Fut,
    span: tracing::Span,
) -> BoxFuture<tonic::Response<super::ServerStream<R::Response>>, tonic::Status>
where
    R: Rpc,
    Fut: std::future::Future<Output = Result<St, tonic::Status>> + Send + 'static,
    St: futures::Stream<Item = Result<R::Response, tonic::Status>> + Send + 'static,
{
    Box::pin(tracing_futures::Instrument::instrument(
        async move {
            match fut.await {
                Ok(stream) => {
                    let stream = futures::StreamExt::map(stream, |item| {
                        match &item {
                            Ok(item) => tracing::trace!("Response item: {item:?}"),
                            Err(err) => tracing::trace!("Response item: {err:?}"),
                        }
                        item
                    });
                    let stream = tracing_futures::Instrument::instrument(
                        futures::StreamExt::boxed(stream),
                        tracing::trace_span!("stream"),
                    );
                    Ok(tonic::Response::new(super::ServerStream {
                        receiver: stream,
                    }))
                }
                Err(err) => {
                    tracing::trace!("Response: {err:?}");
                    Err(err)
                }
            }
        },
        span,
    ))
}

impl<S, R, F, Fut> tonic::server::UnaryService<R> for Handler<S, F>
where
    R: Rpc,
    F: Fn(Arc<S>, R, RequestContext) -> Fut,
    Fut: std::future::Future<Output = Result<R::Response, tonic::Status>> + Send + 'static,
{
    type Response = R::Response;
    type Future = BoxFuture<tonic::Response<R::Response>, tonic::Status>;

    fn call(&mut self, request: tonic::Request<R>) -> Self::Future {
        let (metadata, extensions, request) = request.into_parts();
        tracing::trace!("Request: {request:?}");
        let context = RequestContext::new(metadata.into_headers(), extensions);
        respond(
            (self.handler)(Arc::clone(&self.inner), request, context),
            self.span.clone(),
        )
    }
}

impl<S, R, F, Fut> tonic::server::ClientStreamingService<R> for Handler<S, F>
where
    R: Rpc,
    F: Fn(Arc<S>, RequestStream<R>, RequestContext) -> Fut,
    Fut: std::future::Future<Output = Result<R::Response, tonic::Status>> + Send + 'static,
{
    type Response = R::Response;
    type Future = BoxFuture<tonic::Response<R::Response>, tonic::Status>;

    fn call(&mut self, request: tonic::Request<tonic::Streaming<R>>) -> Self::Future {
        let (metadata, extensions, streaming) = request.into_parts();
        let context = RequestContext::new(metadata.into_headers(), extensions);
        let span = self.span.clone();
        let stream = traced_requests(streaming, &span);
        respond(
            (self.handler)(Arc::clone(&self.inner), stream, context),
            span,
        )
    }
}

impl<S, R, F, Fut, St> tonic::server::ServerStreamingService<R> for Handler<S, F>
where
    R: Rpc,
    F: Fn(Arc<S>, R, RequestContext) -> Fut,
    Fut: std::future::Future<Output = Result<St, tonic::Status>> + Send + 'static,
    St: futures::Stream<Item = Result<R::Response, tonic::Status>> + Send + 'static,
{
    type Response = R::Response;
    type ResponseStream = super::ServerStream<R::Response>;
    type Future = BoxFuture<tonic::Response<Self::ResponseStream>, tonic::Status>;

    fn call(&mut self, request: tonic::Request<R>) -> Self::Future {
        let (metadata, extensions, request) = request.into_parts();
        tracing::trace!("Request: {request:?}");
        let context = RequestContext::new(metadata.into_headers(), extensions);
        let span = self.span.clone();
        respond_stream::<R, _, _>(
            (self.handler)(Arc::clone(&self.inner), request, context),
            span,
        )
    }
}

impl<S, R, F, Fut, St> tonic::server::StreamingService<R> for Handler<S, F>
where
    R: Rpc,
    F: Fn(Arc<S>, RequestStream<R>, RequestContext) -> Fut,
    Fut: std::future::Future<Output = Result<St, tonic::Status>> + Send + 'static,
    St: futures::Stream<Item = Result<R::Response, tonic::Status>> + Send + 'static,
{
    type Response = R::Response;
    type ResponseStream = super::ServerStream<R::Response>;
    type Future = BoxFuture<tonic::Response<Self::ResponseStream>, tonic::Status>;

    fn call(&mut self, request: tonic::Request<tonic::Streaming<R>>) -> Self::Future {
        let (metadata, extensions, streaming) = request.into_parts();
        let context = RequestContext::new(metadata.into_headers(), extensions);
        let span = self.span.clone();
        let stream = traced_requests(streaming, &span);
        respond_stream::<R, _, _>(
            (self.handler)(Arc::clone(&self.inner), stream, context),
            span,
        )
    }
}

/// Serve one unary request: decode through `tonic::server::Grpc`, hand the
/// message and its [`RequestContext`] to `handler`, encode the result.
pub(crate) async fn serve_unary<S, R, F, Fut, B>(
    svc: Arc<S>,
    req: http::Request<B>,
    config: ServerConfig,
    handler: F,
    span: tracing::Span,
) -> Result<http::Response<tonic::body::Body>, std::convert::Infallible>
where
    R: Rpc,
    S: Send + Sync + 'static,
    F: Fn(Arc<S>, R, RequestContext) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<R::Response, tonic::Status>> + Send + 'static,
    B: tonic::codegen::Body + Send + 'static,
    B::Error: Into<tonic::codegen::StdError> + Send + 'static,
{
    let mut grpc = grpc(config);
    Ok(grpc
        .unary(
            Handler {
                inner: svc,
                handler,
                span,
            },
            req,
        )
        .await)
}

/// Serve one client-streaming request; `handler` receives the request stream.
pub(crate) async fn serve_client_stream<S, R, F, Fut, B>(
    svc: Arc<S>,
    req: http::Request<B>,
    config: ServerConfig,
    handler: F,
    span: tracing::Span,
) -> Result<http::Response<tonic::body::Body>, std::convert::Infallible>
where
    R: Rpc,
    S: Send + Sync + 'static,
    F: Fn(Arc<S>, RequestStream<R>, RequestContext) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<R::Response, tonic::Status>> + Send + 'static,
    B: tonic::codegen::Body + Send + 'static,
    B::Error: Into<tonic::codegen::StdError> + Send + 'static,
{
    let mut grpc = grpc(config);
    Ok(grpc
        .client_streaming(
            Handler {
                inner: svc,
                handler,
                span,
            },
            req,
        )
        .await)
}

/// Serve one server-streaming request; `handler` returns the response stream.
pub(crate) async fn serve_server_stream<S, R, F, Fut, St, B>(
    svc: Arc<S>,
    req: http::Request<B>,
    config: ServerConfig,
    handler: F,
    span: tracing::Span,
) -> Result<http::Response<tonic::body::Body>, std::convert::Infallible>
where
    R: Rpc,
    S: Send + Sync + 'static,
    F: Fn(Arc<S>, R, RequestContext) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<St, tonic::Status>> + Send + 'static,
    St: futures::Stream<Item = Result<R::Response, tonic::Status>> + Send + 'static,
    B: tonic::codegen::Body + Send + 'static,
    B::Error: Into<tonic::codegen::StdError> + Send + 'static,
{
    let mut grpc = grpc(config);
    Ok(grpc
        .server_streaming(
            Handler {
                inner: svc,
                handler,
                span,
            },
            req,
        )
        .await)
}

/// Serve one bidirectional-streaming request; `handler` receives the request stream and returns
/// the response stream.
///
/// `S: Send` on top of what the other three ask for: `tonic::server::Grpc::streaming` is the only
/// one of the four entry points that requires it, and it is the primitive the other three delegate
/// to.
pub(crate) async fn serve_bidi_stream<S, R, F, Fut, St, B>(
    svc: Arc<S>,
    req: http::Request<B>,
    config: ServerConfig,
    handler: F,
    span: tracing::Span,
) -> Result<http::Response<tonic::body::Body>, std::convert::Infallible>
where
    R: Rpc,
    S: Send + Sync + 'static,
    F: Fn(Arc<S>, RequestStream<R>, RequestContext) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<St, tonic::Status>> + Send + 'static,
    St: futures::Stream<Item = Result<R::Response, tonic::Status>> + Send + 'static,
    B: tonic::codegen::Body + Send + 'static,
    B::Error: Into<tonic::codegen::StdError> + Send + 'static,
{
    let mut grpc = grpc(config);
    Ok(grpc
        .streaming(
            Handler {
                inner: svc,
                handler,
                span,
            },
            req,
        )
        .await)
}

/// Apply the per-call configuration through tonic's *public* builders.
///
/// `apply_compression_config` and `apply_max_message_size_config` do the same in one call each, and
/// are what tonic's own codegen uses, but both are `#[doc(hidden)]`: a 0.14.x patch may reshape
/// them without considering it semver-relevant, and this is a library. `EnabledCompressionEncodings`
/// has no public iterator, so the sets are drained with the public `pop` and re-applied in the
/// order they were enabled in, which is the order the `grpc-accept-encoding` header lists.
fn grpc<R: Rpc>(
    config: ServerConfig,
) -> tonic::server::Grpc<tonic_prost::ProstCodec<R::Response, R>> {
    fn drain(mut encodings: EnabledCompressionEncodings) -> Vec<CompressionEncoding> {
        let mut out = Vec::new();
        while let Some(encoding) = encodings.pop() {
            out.push(encoding);
        }
        out.reverse();
        out
    }

    let mut grpc = tonic::server::Grpc::new(tonic_prost::ProstCodec::default());
    for encoding in drain(config.accept_compression_encodings) {
        grpc = grpc.accept_compressed(encoding);
    }
    for encoding in drain(config.send_compression_encodings) {
        grpc = grpc.send_compressed(encoding);
    }
    if let Some(limit) = config.max_decoding_message_size {
        grpc = grpc.max_decoding_message_size(limit);
    }
    if let Some(limit) = config.max_encoding_message_size {
        grpc = grpc.max_encoding_message_size(limit);
    }
    grpc
}
