use crate::{
    api::v3,
    objects::auth::{current_user, User},
};

use super::GrpcCall;

/// Service for authentication management.
#[derive(Clone)]
pub struct AuthClient<T> {
    inner: v3::auth::authentication_client::AuthenticationClient<T>,
}

impl<T> AuthClient<T>
where
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

    /// Get current user
    pub async fn current_user(&mut self) -> Result<User, tonic::Status> {
        Ok(self.call(current_user::Request {}).await?.user)
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
    AuthClient {
        async fn call(self, request: current_user::Request) -> Result<current_user::Response> {
            Ok(self
                .inner
                .get_current_user(request)
                .await?
                .into_inner()
                .into())
        }
    }
}
