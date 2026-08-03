use crate::rpc::services;

/// Service for handling applications.
pub type Applications<T = tonic::transport::Channel> = super::ServiceClient<services::Applications, T>;

#[cfg(test)]
#[serial_test::serial(applications)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Applications", "ListApplications").await;
        let mut client = Client::new().await.unwrap().into_applications();
        client
            .list(
                crate::applications::filter::Or {
                    or: vec![crate::applications::filter::And { and: vec![] }],
                },
                crate::applications::Sort::default(),
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Applications", "ListApplications").await;
        assert_eq!(after - before, 1);
    }
}
