use futures::{Stream, StreamExt};
use snafu::ResultExt;

use crate::api::v3;
use crate::events::subscribe;
use crate::utils::IntoCollection;

use super::GrpcCall;

/// Service for authentication management.
#[derive(Clone)]
pub struct EventsClient<T> {
    inner: v3::events::events_client::EventsClient<T>,
}

impl<T> EventsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::events::events_client::EventsClient::new(channel),
        }
    }

    /// Get current user
    pub async fn subscribe(
        &mut self,
        session_id: impl Into<String>,
        task_filters: impl IntoIterator<Item = impl IntoIterator<Item = crate::tasks::filter::Field>>,
        result_filters: impl IntoIterator<
            Item = impl IntoIterator<Item = crate::results::filter::Field>,
        >,
        returned_events: impl IntoIterator<Item = impl Into<crate::events::EventsEnum>>,
    ) -> Result<
        impl Stream<Item = Result<subscribe::Response, super::RequestError>>,
        super::RequestError,
    > {
        Ok(self
            .inner
            .get_events(subscribe::Request {
                session_id: session_id.into(),
                task_filters: task_filters
                    .into_iter()
                    .map(IntoCollection::into_collect)
                    .collect(),
                result_filters: result_filters
                    .into_iter()
                    .map(IntoCollection::into_collect)
                    .collect(),
                returned_events: returned_events.into_collect(),
            })
            .await
            .context(super::GrpcSnafu {})?
            .into_inner()
            .map(|response| response.map(Into::into).context(super::GrpcSnafu {})))
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

#[async_trait::async_trait(?Send)]
impl<T> GrpcCall<subscribe::Request> for &'_ mut EventsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    type Response =
        std::pin::Pin<Box<dyn Stream<Item = Result<subscribe::Response, super::RequestError>>>>;
    type Error = super::RequestError;

    async fn call(self, request: subscribe::Request) -> Result<Self::Response, Self::Error> {
        Ok(Box::pin(
            self.inner
                .get_events(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .map(|response| response.map(Into::into).context(super::GrpcSnafu {})),
        ))
    }
}

#[cfg(test)]
#[serial_test::serial(auth)]
mod tests {
    use futures::TryStreamExt;

    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn subscribe() {
        let before = Client::get_nb_request("Events", "GetEvents").await;
        let mut client = Client::new().await.unwrap().events();
        client
            .subscribe(
                "session-id",
                crate::tasks::filter::Or { or: vec![] },
                crate::results::filter::Or { or: vec![] },
                vec![crate::events::EventsEnum::Unspecified],
            )
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Events", "GetEvents").await;
        assert_eq!(after - before, 1);
    }
    // Explicit call request

    #[tokio::test]
    async fn subscribe_call() {
        let before = Client::get_nb_request("Events", "GetEvents").await;
        let mut client = Client::new().await.unwrap().events();
        client
            .call(crate::events::subscribe::Request {
                session_id: String::from("session-id"),
                task_filters: crate::tasks::filter::Or { or: vec![] },
                result_filters: crate::results::filter::Or { or: vec![] },
                returned_events: vec![],
            })
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Events", "GetEvents").await;
        assert_eq!(after - before, 1);
    }
}
