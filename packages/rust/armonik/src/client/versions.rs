use crate::{api::v3, objects::versions::list};

use super::GrpcCall;

#[derive(Clone)]
pub struct VersionsClient<T> {
    inner: v3::versions::versions_client::VersionsClient<T>,
}

impl<T> VersionsClient<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::versions::versions_client::VersionsClient::new(channel),
        }
    }

    pub async fn list(&mut self) -> Result<list::Response, tonic::Status> {
        self.call(list::Request {}).await
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
    VersionsClient {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            Ok(self
                .inner
                .list_versions(request)
                .await?
                .into_inner()
                .into())
        }
    }
}
