use snafu::ResultExt;

use crate::{api::v3, objects::applications::list};

use super::GrpcCall;

#[derive(Clone)]
pub struct ApplicationsClient<T> {
    inner: v3::applications::applications_client::ApplicationsClient<T>,
}

impl<T> ApplicationsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::applications::applications_client::ApplicationsClient::new(channel),
        }
    }

    pub async fn list(
        &mut self,
        request: list::Request,
    ) -> Result<list::Response, super::RequestError> {
        self.call(request).await
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
    ApplicationsClient {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            Ok(self
                .inner
                .list_applications(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }
    }
}
