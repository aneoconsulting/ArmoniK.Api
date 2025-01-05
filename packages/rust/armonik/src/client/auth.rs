use snafu::ResultExt;

use crate::api::v3;
use crate::auth::{current_user, User};

use super::GrpcCall;

/// Service for authentication management.
#[derive(Clone)]
pub struct Auth<T> {
    inner: v3::auth::authentication_client::AuthenticationClient<T>,
}

impl<T> Auth<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::auth::authentication_client::AuthenticationClient::new(channel),
        }
    }

    /// Get current user
    pub async fn current_user(&mut self) -> Result<User, super::RequestError> {
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
    Auth {
        async fn call(self, request: current_user::Request) -> Result<current_user::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .get_current_user(request),
                tracing::debug_span!("Auth::current_user")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }
    }
}

#[cfg(test)]
#[serial_test::serial(auth)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn current_user() {
        let before = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        let mut client = Client::new().await.unwrap().into_auth();
        client.current_user().await.unwrap();
        let after = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn current_user_call() {
        let before = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        let mut client = Client::new().await.unwrap().into_auth();
        client
            .call(crate::auth::current_user::Request {})
            .await
            .unwrap();
        let after = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        assert_eq!(after - before, 1);
    }
}
