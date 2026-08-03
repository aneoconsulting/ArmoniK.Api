use futures::Stream;

use crate::events::subscribe;
use crate::rpc::services;
use crate::utils::IntoCollection;

/// Service for subscribing to events representing modifications to ArmoniK
/// result and task data.
pub type Events<T = tonic::transport::Channel> = super::ServiceClient<services::Events, T>;

impl<T: super::Channel> super::ServiceClient<services::Events, T> {
    /// Subscribe to the event stream of a session.
    pub async fn subscribe(
        &mut self,
        session_id: impl Into<String>,
        task_filters: impl IntoIterator<Item = impl IntoIterator<Item = crate::tasks::filter::Field>>,
        result_filters: impl IntoIterator<
            Item = impl IntoIterator<Item = crate::results::filter::Field>,
        >,
        returned_events: impl IntoIterator<Item = impl Into<crate::events::EventsEnum>>,
    ) -> Result<
        impl Stream<Item = Result<subscribe::Response, super::RequestError>> + 'static,
        super::RequestError,
    > {
        self.call(subscribe::Request {
            session_id: session_id.into(),
            task_filters: crate::utils::into_filters(task_filters),
            result_filters: crate::utils::into_filters(result_filters),
            returned_events: returned_events.into_collect(),
        })
        .await
    }
}

#[cfg(test)]
#[serial_test::serial(events)]
mod tests {
    use futures::TryStreamExt;

    use crate::Client;


    #[tokio::test]
    async fn subscribe() {
        let before = Client::get_nb_request("Events", "GetEvents").await;
        let mut client = Client::new().await.unwrap().into_events();
        client
            .subscribe(
                "session-id",
                crate::tasks::filter::Or { or: vec![] },
                crate::results::filter::Or { or: vec![] },
                vec![crate::events::EventsEnum::UNSPECIFIED],
            )
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Events", "GetEvents").await;
        assert_eq!(after - before, 1);
    }
}
