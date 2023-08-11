use crate::{api::v3, objects::versions::Versions};

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

    pub async fn list(&mut self) -> Result<Versions, tonic::Status> {
        Ok(self
            .inner
            .list_versions(v3::versions::ListVersionsRequest {})
            .await?
            .into_inner()
            .into())
    }
}
