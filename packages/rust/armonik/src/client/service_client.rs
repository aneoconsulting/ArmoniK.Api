//! Generic gRPC client: one [`ServiceClient`] type for every service, with
//! the RPC deduced from the request type through [`Rpc`].
//!
//! One entry point, [`ServiceClient::call`], serves all four call kinds. The
//! *output* shape hangs off the call kind ([`Dispatch`], a GAT keyed on
//! [`Unary`] / [`ServerStream`] / [`ClientStream`] / [`BidiStream`]); the
//! *input* shape picks its own dispatch through [`IntoCall`], whose marker
//! parameter keeps "a message" and "a stream of messages" from being one
//! overlapping impl.

use std::marker::PhantomData;

use snafu::ResultExt;

use crate::rpc::{BidiStream, ClientStream, Rpc, ServerStream, Service, Unary};

use super::RequestError;

/// The channel bounds every client signature would otherwise spell out.
pub trait Channel:
    tonic::client::GrpcService<
    tonic::body::Body,
    Error: Into<tonic::codegen::StdError>,
    ResponseBody: tonic::codegen::Body<
        Data = tonic::codegen::Bytes,
        Error: Into<tonic::codegen::StdError> + Send,
    > + Send
                      + 'static,
>
{
}

impl<T> Channel for T where
    T: tonic::client::GrpcService<
        tonic::body::Body,
        Error: Into<tonic::codegen::StdError>,
        ResponseBody: tonic::codegen::Body<
            Data = tonic::codegen::Bytes,
            Error: Into<tonic::codegen::StdError> + Send,
        > + Send
                          + 'static,
    >
{
}

/// A gRPC client for the service marker `Svc`.
pub struct ServiceClient<Svc, T = tonic::transport::Channel> {
    inner: tonic::client::Grpc<T>,
    _svc: PhantomData<fn() -> Svc>,
}

impl<Svc, T: Clone> Clone for ServiceClient<Svc, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _svc: PhantomData,
        }
    }
}

impl<Svc: Service, T: Channel> ServiceClient<Svc, T> {
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: tonic::client::Grpc::new(channel),
            _svc: PhantomData,
        }
    }

    /// Perform a gRPC call. The RPC, and with it the call kind, is deduced from
    /// the request type, so all four kinds go through this one method:
    ///
    /// * a unary or server-streaming RPC takes its request message, and gives
    ///   back the response message or a stream of response items;
    /// * a client-streaming or bidirectional RPC takes a
    ///   [`Stream`](futures::Stream) of its request messages, and gives back
    ///   the response message or a stream of them.
    ///
    /// Either input may instead be a [`tonic::Request`] wrapping it, for
    /// per-call metadata and deadlines.
    ///
    /// Both `R` and the marker `M` (see [`IntoCall`]) are inferred from the
    /// input, including when it is a pre-built [`tonic::Request`]. `M` never
    /// needs naming, but a turbofish has to leave room for it:
    /// `call::<R, _>(..)`.
    pub async fn call<R, M>(
        &mut self,
        input: impl IntoCall<R, M>,
    ) -> Result<<R::Kind as Dispatch>::Output<R>, RequestError>
    where
        R: Rpc<Service = Svc>,
        R::Kind: Dispatch,
    {
        input.into_call(&mut self.inner).await
    }

    /// Compress requests with the given encoding.
    ///
    /// No negotiation: the encoding is applied to every request from here on, and a server that
    /// does not accept it fails every one of them with `UNIMPLEMENTED`. Enable it only against a
    /// peer known to accept it.
    ///
    /// Every `CompressionEncoding` variant is behind one of tonic's compression features; turn on
    /// this crate's `gzip` to have one to pass.
    #[must_use]
    pub fn send_compressed(mut self, encoding: tonic::codegen::CompressionEncoding) -> Self {
        self.inner = self.inner.send_compressed(encoding);
        self
    }

    /// Accept responses compressed with the given encoding.
    ///
    /// This half *is* advertised, in `grpc-accept-encoding`, so a server may use it or not. See
    /// [`send_compressed`](Self::send_compressed) for where the encodings come from.
    #[must_use]
    pub fn accept_compressed(mut self, encoding: tonic::codegen::CompressionEncoding) -> Self {
        self.inner = self.inner.accept_compressed(encoding);
        self
    }

    /// Limits the maximum size of a decoded message. Default: `4MB`.
    #[must_use]
    pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
        self.inner = self.inner.max_decoding_message_size(limit);
        self
    }

    /// Limits the maximum size of an encoded message. Default: `usize::MAX`.
    #[must_use]
    pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
        self.inner = self.inner.max_encoding_message_size(limit);
        self
    }
}

/// What [`ServiceClient::call`] hands back for one call kind.
///
/// Split from [`DispatchMessage`] because every kind has an output shape, but
/// only the two whose request is a single message share a dispatch signature.
pub trait Dispatch: Sized {
    /// What `call` returns: the response message for unary and client-streaming
    /// RPCs, a stream of response items for server-streaming ones.
    type Output<R: Rpc<Kind = Self>>;
}

/// The call kinds whose request is a single message: [`Unary`] and
/// [`ServerStream`]. [`ClientStream`] is deliberately absent, which is what
/// makes "you cannot call a client-streaming RPC with one message" a compile
/// error.
pub trait DispatchMessage: Dispatch {
    /// Perform the call.
    #[allow(async_fn_in_trait)]
    async fn dispatch<T, R>(
        grpc: &mut tonic::client::Grpc<T>,
        request: tonic::Request<R>,
    ) -> Result<Self::Output<R>, RequestError>
    where
        T: Channel,
        R: Rpc<Kind = Self>;
}

/// The call kinds whose *request* is a stream: [`ClientStream`] and [`BidiStream`].
///
/// The mirror of [`DispatchMessage`], and the pairing is the point: the two traits partition the
/// four kinds by input shape, so "you cannot upload with one message" and "you cannot call a unary
/// RPC with a stream" are each a missing impl rather than a runtime surprise.
pub trait DispatchStream: Dispatch {
    /// Perform the call.
    #[allow(async_fn_in_trait)]
    async fn dispatch<T, R, S>(
        grpc: &mut tonic::client::Grpc<T>,
        request: tonic::Request<S>,
    ) -> Result<Self::Output<R>, RequestError>
    where
        T: Channel,
        R: Rpc<Kind = Self>,
        S: futures::Stream<Item = R> + Send + 'static;
}

impl Dispatch for Unary {
    type Output<R: Rpc<Kind = Self>> = R::Response;
}

impl DispatchMessage for Unary {
    async fn dispatch<T, R>(
        grpc: &mut tonic::client::Grpc<T>,
        mut request: tonic::Request<R>,
    ) -> Result<R::Response, RequestError>
    where
        T: Channel,
        R: Rpc<Kind = Self>,
    {
        let fut = async move {
            ready(grpc).await?;
            request.extensions_mut().insert(grpc_method::<R>());
            Ok(grpc
                .unary(request, path::<R>(), codec())
                .await
                .context(super::GrpcSnafu {})?
                .into_inner())
        };
        tracing_futures::Instrument::instrument(fut, span_for::<R>()).await
    }
}

impl Dispatch for ServerStream {
    type Output<R: Rpc<Kind = Self>> =
        futures::stream::BoxStream<'static, Result<R::Response, RequestError>>;
}

impl Dispatch for ClientStream {
    type Output<R: Rpc<Kind = Self>> = R::Response;
}

impl Dispatch for BidiStream {
    type Output<R: Rpc<Kind = Self>> =
        futures::stream::BoxStream<'static, Result<R::Response, RequestError>>;
}

impl DispatchMessage for ServerStream {
    async fn dispatch<T, R>(
        grpc: &mut tonic::client::Grpc<T>,
        mut request: tonic::Request<R>,
    ) -> Result<Self::Output<R>, RequestError>
    where
        T: Channel,
        R: Rpc<Kind = Self>,
    {
        let span = span_for::<R>();
        let stream_span = span.clone();
        let fut = async move {
            ready(grpc).await?;
            request.extensions_mut().insert(grpc_method::<R>());
            let stream = grpc
                .server_streaming(request, path::<R>(), codec())
                .await
                .context(super::GrpcSnafu {})?
                .into_inner();
            let stream = futures::StreamExt::map(stream, |item| item.context(super::GrpcSnafu {}));
            Ok(futures::StreamExt::boxed(
                tracing_futures::Instrument::instrument(stream, stream_span),
            ))
        };
        tracing_futures::Instrument::instrument(fut, span).await
    }
}

/// Anything [`ServiceClient::call`] accepts, together with the RPC it starts.
///
/// The request type alone determines the RPC, so `call` does not fork on the
/// call kind; this trait is what lets one signature take a message, a
/// [`tonic::Request`] around one, a stream of messages, or a `tonic::Request`
/// around a stream. Those four input shapes cannot be four impls of a
/// one-parameter trait — `impl IntoCall<R> for S where S: Stream<Item = R>`
/// and `impl IntoCall<R> for R` overlap as far as coherence is concerned,
/// since nothing stops an upstream crate implementing `Stream` for a request
/// type. `M` is an inert marker, distinct per impl, that makes the impls
/// disjoint by their *trait arguments* rather than by their self types (the
/// trick `axum::handler::Handler` uses). It is inferred at every call site
/// alongside `R` and never named.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid input for `call`",
    label = "invalid `call` input",
    note = "`call` takes the RPC's request message, a `tonic::Request` wrapping it, or, for an RPC \
            whose request is a stream, a `Stream` of its request messages",
    note = "which of the two depends on the RPC: a client-streaming or bidirectional one takes a \
            `Stream` of its request messages (`futures::stream::iter([..])`, \
            `async_stream::stream!`, ...), and a unary or server-streaming one takes the message \
            itself, not a stream"
)]
pub trait IntoCall<R, M>
where
    R: Rpc,
    R::Kind: Dispatch,
{
    /// Perform the call this input describes.
    #[allow(async_fn_in_trait)]
    async fn into_call<T: Channel>(
        self,
        grpc: &mut tonic::client::Grpc<T>,
    ) -> Result<<R::Kind as Dispatch>::Output<R>, RequestError>;
}

/// [`IntoCall`] marker: the input is the request message itself.
pub struct ByMessage;
/// [`IntoCall`] marker: the input is a [`tonic::Request`] around the message.
pub struct ByRequest;
/// [`IntoCall`] marker: the input is a stream of request messages.
pub struct ByStream;
/// [`IntoCall`] marker: the input is a [`tonic::Request`] around such a stream.
pub struct ByStreamRequest;

impl<R> IntoCall<R, ByMessage> for R
where
    R: Rpc,
    R::Kind: DispatchMessage,
{
    async fn into_call<T: Channel>(
        self,
        grpc: &mut tonic::client::Grpc<T>,
    ) -> Result<<R::Kind as Dispatch>::Output<R>, RequestError> {
        <R::Kind as DispatchMessage>::dispatch(grpc, tonic::Request::new(self)).await
    }
}

// `do_not_recommend`: when no impl applies, listing the `tonic::Request` ones as
// "other types that implement `IntoCall`" is noise; the `on_unimplemented` note
// says the same thing in words.
#[diagnostic::do_not_recommend]
impl<R> IntoCall<R, ByRequest> for tonic::Request<R>
where
    R: Rpc,
    R::Kind: DispatchMessage,
{
    async fn into_call<T: Channel>(
        self,
        grpc: &mut tonic::client::Grpc<T>,
    ) -> Result<<R::Kind as Dispatch>::Output<R>, RequestError> {
        <R::Kind as DispatchMessage>::dispatch(grpc, self).await
    }
}

impl<R, S> IntoCall<R, ByStream> for S
where
    R: Rpc,
    R::Kind: DispatchStream,
    S: futures::Stream<Item = R> + Send + 'static,
{
    async fn into_call<T: Channel>(
        self,
        grpc: &mut tonic::client::Grpc<T>,
    ) -> Result<<R::Kind as Dispatch>::Output<R>, RequestError> {
        <R::Kind as DispatchStream>::dispatch(grpc, tonic::Request::new(self)).await
    }
}

#[diagnostic::do_not_recommend]
impl<R, S> IntoCall<R, ByStreamRequest> for tonic::Request<S>
where
    R: Rpc,
    R::Kind: DispatchStream,
    S: futures::Stream<Item = R> + Send + 'static,
{
    async fn into_call<T: Channel>(
        self,
        grpc: &mut tonic::client::Grpc<T>,
    ) -> Result<<R::Kind as Dispatch>::Output<R>, RequestError> {
        <R::Kind as DispatchStream>::dispatch(grpc, self).await
    }
}

impl DispatchStream for ClientStream {
    async fn dispatch<T, R, S>(
        grpc: &mut tonic::client::Grpc<T>,
        request: tonic::Request<S>,
    ) -> Result<R::Response, RequestError>
    where
        T: Channel,
        R: Rpc<Kind = Self>,
        S: futures::Stream<Item = R> + Send + 'static,
    {
        let span = span_for::<R>();
        let request = tagged::<R, S>(request, &span);
        let fut = async move {
            ready(grpc).await?;
            Ok(grpc
                .client_streaming(request, path::<R>(), codec())
                .await
                .context(super::GrpcSnafu {})?
                .into_inner())
        };
        tracing_futures::Instrument::instrument(fut, span).await
    }
}

impl DispatchStream for BidiStream {
    async fn dispatch<T, R, S>(
        grpc: &mut tonic::client::Grpc<T>,
        request: tonic::Request<S>,
    ) -> Result<Self::Output<R>, RequestError>
    where
        T: Channel,
        R: Rpc<Kind = Self>,
        S: futures::Stream<Item = R> + Send + 'static,
    {
        let span = span_for::<R>();
        let stream_span = span.clone();
        // The two halves are tied together here, before tonic sees either: the request stream runs
        // until `cancelled` resolves, and the only sender lives in the returned response stream.
        let (cancel, cancelled) = futures::channel::oneshot::channel::<()>();
        let (metadata, extensions, stream) = tagged::<R, S>(request, &span).into_parts();
        let request = tonic::Request::from_parts(
            metadata,
            extensions,
            futures::StreamExt::take_until(stream, cancelled),
        );
        let fut = async move {
            ready(grpc).await?;
            // `streaming` is the primitive the other three tonic entry points delegate to.
            let stream = grpc
                .streaming(request, path::<R>(), codec())
                .await
                .context(super::GrpcSnafu {})?
                .into_inner();
            let stream = futures::StreamExt::map(stream, |item| item.context(super::GrpcSnafu {}));
            Ok(futures::StreamExt::boxed(Cancelling {
                stream: futures::StreamExt::boxed(tracing_futures::Instrument::instrument(
                    stream,
                    stream_span,
                )),
                _cancel: cancel,
            }))
        };
        tracing_futures::Instrument::instrument(fut, span).await
    }
}

/// A bidirectional call's response stream, which closes the request half when it is dropped.
///
/// The request stream was moved into `call`, so this is the caller's only handle on the RPC and
/// dropping it has to end the call. Dropping tonic's `Streaming` on its own does not: the request
/// body is still open, so the server's handler stays parked on the next request message with its
/// h2 stream and its task, for as long as the caller holds the sending end. Dropping the sender
/// below resolves the future the request stream is `take_until`'d by, which half-closes the body,
/// which is what a handler reading `while let Some(..)` waits for.
///
/// The other three kinds need none of this: their `Output` owns the whole call, so dropping it
/// drops the request half with it.
struct Cancelling<T> {
    stream: futures::stream::BoxStream<'static, T>,
    _cancel: futures::channel::oneshot::Sender<()>,
}

impl<T> futures::Stream for Cancelling<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<T>> {
        // Both fields are `Unpin`, so there is nothing to project.
        futures::Stream::poll_next(self.get_mut().stream.as_mut(), cx)
    }
}

/// Tag a stream-input request with its RPC's `GrpcMethod`, and put the caller's stream under a
/// child span: it is polled while the call runs, so its events belong under the call rather than
/// wherever the caller built it.
fn tagged<R: Rpc, S>(
    request: tonic::Request<S>,
    span: &tracing::Span,
) -> tonic::Request<tracing_futures::Instrumented<S>> {
    let (metadata, mut extensions, stream) = request.into_parts();
    extensions.insert(grpc_method::<R>());
    let stream = tracing_futures::Instrument::instrument(
        stream,
        tracing::trace_span!(parent: span, "stream"),
    );
    tonic::Request::from_parts(metadata, extensions, stream)
}

fn span_for<R: Rpc>() -> tracing::Span {
    tracing::debug_span!("armonik.rpc", rpc = R::LABEL, otel.name = R::LABEL)
}

fn grpc_method<R: Rpc>() -> tonic::codegen::GrpcMethod<'static> {
    tonic::codegen::GrpcMethod::new(<R::Service as Service>::NAME, R::METHOD)
}

fn path<R: Rpc>() -> tonic::codegen::http::uri::PathAndQuery {
    tonic::codegen::http::uri::PathAndQuery::from_static(R::PATH)
}

fn codec<R: Rpc>() -> tonic_prost::ProstCodec<R, R::Response> {
    tonic_prost::ProstCodec::default()
}

async fn ready<T: Channel>(grpc: &mut tonic::client::Grpc<T>) -> Result<(), RequestError> {
    grpc.ready()
        .await
        .map_err(|e| tonic::Status::unknown(format!("Service was not ready: {}", e.into())))
        .context(super::GrpcSnafu {})
}
