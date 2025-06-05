use snafu::ResultExt;

use crate::api::v3;
use crate::health_checks::check;

use super::GrpcCall;

/// Service for authentication management.
#[derive(Clone)]
pub struct HealthChecks<T> {
    inner: v3::health_checks::health_checks_service_client::HealthChecksServiceClient<T>,
}

impl<T> HealthChecks<T>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::health_checks::health_checks_service_client::HealthChecksServiceClient::new(
                channel,
            ),
        }
    }

    /// Checks the health of the cluster. This can be used to verify that the cluster is up and running.
    pub async fn check(
        &mut self,
    ) -> Result<Vec<crate::health_checks::ServiceHealth>, super::RequestError> {
        Ok(self.call(check::Request {}).await?.services)
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
    HealthChecks {
        async fn call(self, request: check::Request) -> Result<check::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .check_health(request),
                tracing::debug_span!("HealthChecks::check")
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
#[serial_test::serial(health_checks)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn check() {
        let before = Client::get_nb_request("HealthChecks", "CheckHealth").await;
        let mut client = Client::new().await.unwrap().into_health_checks();
        client.check().await.unwrap();
        let after = Client::get_nb_request("HealthChecks", "CheckHealth").await;
        assert_eq!(after - before, 1);
    }
    // Explicit call request

    #[tokio::test]
    async fn check_call() {
        let before = Client::get_nb_request("HealthChecks", "CheckHealth").await;
        let mut client = Client::new().await.unwrap().into_health_checks();
        client
            .call(crate::health_checks::check::Request {})
            .await
            .unwrap();
        let after = Client::get_nb_request("HealthChecks", "CheckHealth").await;
        assert_eq!(after - before, 1);
    }
}
