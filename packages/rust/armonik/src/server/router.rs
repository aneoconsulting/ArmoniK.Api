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

/// Compression and message-size configuration, applied per call.
#[derive(Clone, Copy, Default)]
pub(crate) struct ServerConfig {
    accept_compression_encodings: EnabledCompressionEncodings,
    send_compression_encodings: EnabledCompressionEncodings,
    max_decoding_message_size: Option<usize>,
    max_encoding_message_size: Option<usize>,
}

/// An erased request handler: one per RPC, stored in [`Routes::ROUTES`].
pub(crate) type RouteFn<S> = fn(
    Arc<S>,
    http::Request<tonic::body::Body>,
    ServerConfig,
) -> BoxFuture<http::Response<tonic::body::Body>, std::convert::Infallible>;

/// The request stream handed to client-streaming trait methods.
pub(crate) type RequestStream<R> = futures::stream::BoxStream<'static, Result<R, tonic::Status>>;

/// The routing table of the service marker `Self` over a service
/// implementation `S`.
pub(crate) trait Routes<S: 'static>: Service {
    /// `(path, handler)`, one entry per RPC.
    const ROUTES: &'static [(&'static str, RouteFn<S>)];
}

/// A gRPC server for the service marker `Svc`, routing requests to the
/// service implementation `S`.
pub struct Router<Svc, S> {
    inner: Arc<S>,
    config: ServerConfig,
    _svc: PhantomData<fn() -> Svc>,
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

    /// Enable decompressing requests with the given encoding.
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

impl<Svc, S: 'static> tonic::codegen::Service<http::Request<tonic::body::Body>> for Router<Svc, S>
where
    Svc: Routes<S>,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<tonic::body::Body>) -> Self::Future {
        for (path, handler) in Svc::ROUTES {
            if *path == req.uri().path() {
                return handler(Arc::clone(&self.inner), req, self.config);
            }
        }
        Box::pin(async move {
            let mut response = http::Response::new(tonic::body::Body::default());
            let headers = response.headers_mut();
            headers.insert(
                tonic::Status::GRPC_STATUS,
                (tonic::Code::Unimplemented as i32).into(),
            );
            headers.insert(
                http::header::CONTENT_TYPE,
                tonic::metadata::GRPC_CONTENT_TYPE,
            );
            Ok(response)
        })
    }
}

impl<Svc: Service, S> tonic::server::NamedService for Router<Svc, S> {
    const NAME: &'static str = Svc::NAME;
}

/// One handler passed to `tonic::server::Grpc`: the service implementation,
/// the trait-method closure from the routing table, and the per-RPC span. The
/// three `tonic::server::*Service` impls below give it the three call shapes.
struct Handler<S, F> {
    inner: Arc<S>,
    handler: F,
    span: tracing::Span,
}

/// The traced, boxed response future shared by the unary and client-streaming
/// call shapes.
fn respond<T, Fut>(
    fut: Fut,
    span: tracing::Span,
) -> BoxFuture<tonic::Response<T>, tonic::Status>
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
        let stream = futures::StreamExt::map(streaming, |item| {
            match &item {
                Ok(item) => tracing::trace!("Request item: {item:?}"),
                Err(err) => tracing::trace!("Request item: {err:?}"),
            }
            item
        });
        let stream = futures::StreamExt::boxed(tracing_futures::Instrument::instrument(
            stream,
            tracing::trace_span!(parent: &span, "stream"),
        ));
        respond((self.handler)(Arc::clone(&self.inner), stream, context), span)
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
        let fut = (self.handler)(Arc::clone(&self.inner), request, context);
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
}

/// Serve one unary request: decode through `tonic::server::Grpc`, hand the
/// message and its [`RequestContext`] to `handler`, encode the result.
pub(crate) async fn serve_unary<S, R, F, Fut>(
    svc: Arc<S>,
    req: http::Request<tonic::body::Body>,
    config: ServerConfig,
    handler: F,
    span: tracing::Span,
) -> Result<http::Response<tonic::body::Body>, std::convert::Infallible>
where
    R: Rpc,
    S: Send + Sync + 'static,
    F: Fn(Arc<S>, R, RequestContext) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<R::Response, tonic::Status>> + Send + 'static,
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
pub(crate) async fn serve_client_stream<S, R, F, Fut>(
    svc: Arc<S>,
    req: http::Request<tonic::body::Body>,
    config: ServerConfig,
    handler: F,
    span: tracing::Span,
) -> Result<http::Response<tonic::body::Body>, std::convert::Infallible>
where
    R: Rpc,
    S: Send + Sync + 'static,
    F: Fn(Arc<S>, RequestStream<R>, RequestContext) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<R::Response, tonic::Status>> + Send + 'static,
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
pub(crate) async fn serve_server_stream<S, R, F, Fut, St>(
    svc: Arc<S>,
    req: http::Request<tonic::body::Body>,
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

fn grpc<R: Rpc>(
    config: ServerConfig,
) -> tonic::server::Grpc<tonic_prost::ProstCodec<R::Response, R>> {
    tonic::server::Grpc::new(tonic_prost::ProstCodec::default())
        .apply_compression_config(
            config.accept_compression_encodings,
            config.send_compression_encodings,
        )
        .apply_max_message_size_config(
            config.max_decoding_message_size,
            config.max_encoding_message_size,
        )
}
