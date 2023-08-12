use crate::{
    api::v3,
    objects::applications::{ApplicationListRequest, ApplicationListResponse},
};

#[derive(Clone)]
pub struct ApplicationsClient<T> {
    inner: v3::applications::applications_client::ApplicationsClient<T>,
}

impl<T> ApplicationsClient<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::applications::applications_client::ApplicationsClient::new(channel),
        }
    }

    pub async fn list(
        &mut self,
        request: ApplicationListRequest,
    ) -> Result<ApplicationListResponse, tonic::Status> {
        Ok(self
            .inner
            .list_applications(v3::applications::ListApplicationsRequest::from(request))
            .await?
            .into_inner()
            .into())
    }
}
