use snafu::ResultExt;

use crate::versions::list;

use super::GrpcCall;

/// The raw tonic client stub, speaking the armonik types natively.
pub use crate::stubs::versions::versions_client as stub;

#[derive(Clone)]
pub struct Versions<T> {
    inner: stub::VersionsClient<T>,
}

impl<T> Versions<T>
where
    T: crate::client::Channel,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: stub::VersionsClient::new(channel),
        }
    }

    pub async fn list(&mut self) -> Result<list::Response, super::RequestError> {
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
    Versions {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .list_versions(request),
                tracing::debug_span!("Versions::list")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner())
        }
    }
}

#[cfg(test)]
#[serial_test::serial(versions)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Versions", "ListVersions").await;
        let mut client = Client::new().await.unwrap().into_versions();
        client.list().await.unwrap();
        let after = Client::get_nb_request("Versions", "ListVersions").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Versions", "ListVersions").await;
        let mut client = Client::new().await.unwrap().into_versions();
        client
            .call(crate::versions::list::Request {})
            .await
            .unwrap();
        let after = Client::get_nb_request("Versions", "ListVersions").await;
        assert_eq!(after - before, 1);
    }
}
