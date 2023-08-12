use crate::{api::v3, objects::auth::User};

#[derive(Clone)]
pub struct AuthClient<T> {
    inner: v3::auth::authentication_client::AuthenticationClient<T>,
}

impl<T> AuthClient<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::auth::authentication_client::AuthenticationClient::new(channel),
        }
    }

    pub async fn current_user(&mut self) -> Result<User, tonic::Status> {
        Ok(self
            .inner
            .get_current_user(v3::auth::GetCurrentUserRequest {})
            .await?
            .into_inner()
            .into())
    }
}
