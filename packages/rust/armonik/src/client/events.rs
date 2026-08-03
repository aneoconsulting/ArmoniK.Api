use crate::rpc::services;

/// Service for subscribing to events representing modifications to ArmoniK
/// result and task data.
pub type Events<T = tonic::transport::Channel> = super::ServiceClient<services::Events, T>;

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
