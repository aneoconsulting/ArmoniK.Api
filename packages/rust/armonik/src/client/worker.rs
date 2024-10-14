use crate::{
    api::v3,
    objects::worker::{health_check, process},
    objects::Output,
};

use super::GrpcCall;

#[derive(Clone)]
pub struct WorkerClient<T> {
    inner: v3::worker::worker_client::WorkerClient<T>,
}

impl<T> WorkerClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::worker::worker_client::WorkerClient::new(channel),
        }
    }

    pub async fn health_check(&mut self) -> Result<health_check::Response, tonic::Status> {
        self.call(health_check::Request {}).await
    }

    pub async fn process(&mut self, request: process::Request) -> Result<Output, tonic::Status> {
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
    WorkerClient {
        async fn call(self, request: health_check::Request) -> Result<health_check::Response> {
            Ok(self
                .inner
                .health_check(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: process::Request) -> Result<process::Response> {
            Ok(self
                .inner
                .process(request)
                .await?
                .into_inner()
                .into())
        }
    }
}
