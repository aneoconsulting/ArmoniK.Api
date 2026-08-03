use crate::rpc::services;

/// Service for handling sessions.
pub type Sessions<T = tonic::transport::Channel> = super::ServiceClient<services::Sessions, T>;

#[cfg(test)]
#[serial_test::serial(sessions)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Sessions", "ListSessions").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .list(
                crate::sessions::filter::Or {
                    or: vec![crate::sessions::filter::And { and: vec![] }],
                },
                crate::sessions::Sort::default(),
                true,
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "ListSessions").await;
        assert_eq!(after - before, 1);
    }
}
