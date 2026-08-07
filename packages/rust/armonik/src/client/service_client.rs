//! Generic gRPC client: one [`ServiceClient`] type for every service, with
//! the RPC deduced from the request type through [`Rpc`].

use std::marker::PhantomData;

use snafu::ResultExt;

use crate::rpc::{ClientStream, Rpc, ServerStream, Service, Unary};

use super::RequestError;

/// The channel bounds every client signature used to spell out, bundled.
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

    /// Perform a gRPC call. The RPC is deduced from the request type.
    pub async fn call<R>(
        &mut self,
        request: impl tonic::IntoRequest<R>,
    ) -> Result<<R::Kind as Dispatch>::Output<R>, RequestError>
    where
        R: Rpc<Service = Svc>,
        R::Kind: Dispatch,
    {
        <R::Kind as Dispatch>::dispatch(&mut self.inner, request.into_request()).await
    }

    /// Perform a client-streaming gRPC call from a raw request stream.
    pub async fn call_streaming<S>(
        &mut self,
        request: S,
    ) -> Result<<S::Item as Rpc>::Response, RequestError>
    where
        S: futures::Stream + Send + 'static,
        S::Item: Rpc<Service = Svc, Kind = ClientStream>,
    {
        let span = span_for::<S::Item>();
        let grpc = &mut self.inner;
        let fut = async move {
            ready(grpc).await?;
            let mut request = tonic::IntoStreamingRequest::into_streaming_request(request);
            request.extensions_mut().insert(grpc_method::<S::Item>());
            Ok(grpc
                .client_streaming(request, path::<S::Item>(), codec())
                .await
                .context(super::GrpcSnafu {})?
                .into_inner())
        };
        tracing_futures::Instrument::instrument(fut, span).await
    }

    /// Compress requests with the given encoding, if the server supports it.
    #[must_use]
    pub fn send_compressed(mut self, encoding: tonic::codegen::CompressionEncoding) -> Self {
        self.inner = self.inner.send_compressed(encoding);
        self
    }

    /// Enable decompressing responses.
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

/// Client-side dispatch for one call kind: how to drive `tonic::client::Grpc`
/// and what shape the output has.
pub trait Dispatch: Sized {
    /// What `call` returns: the response message for unary RPCs, a stream of
    /// response items for server-streaming ones.
    type Output<R: Rpc<Kind = Self>>;

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

impl Dispatch for Unary {
    type Output<R: Rpc<Kind = Self>> = R::Response;

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
