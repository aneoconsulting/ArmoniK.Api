use snafu::ResultExt;

use crate::api::v3;
use crate::worker::{health_check, process};
use crate::Output;

use super::GrpcCall;

#[derive(Clone)]
pub struct Worker<T> {
    inner: v3::worker::worker_client::WorkerClient<T>,
}

impl<T> Worker<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::worker::worker_client::WorkerClient::new(channel),
        }
    }

    pub async fn health_check(&mut self) -> Result<health_check::Response, super::RequestError> {
        self.call(health_check::Request {}).await
    }

    pub async fn process(
        &mut self,
        request: process::Request,
    ) -> Result<Output, super::RequestError> {
        Ok(self.call(request).await?.output)
    }

    /// Perform a gRPC call from a raw request.
    pub async fn call<Request>(
        &mut self,
        request: Request,
    ) -> Result<<&mut Self as GrpcCall<Request>>::Response, <&mut Self as GrpcCall<Request>>::Error>
    where
        for<'a> &'a mut Self: GrpcCall<Request>,
    {
        <&mut Self as GrpcCall<Request>>::call(self, request).await
    }
}

super::impl_call! {
    Worker {
        async fn call(self, request: health_check::Request) -> Result<health_check::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .health_check(request),
                tracing::debug_span!("Worker::health_check")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: process::Request) -> Result<process::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .process(request),
                tracing::debug_span!("Worker::process")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }
    }
}
