use crate::rpc::services;

/// Service for authentication management.
pub type Auth<T = tonic::transport::Channel> = super::ServiceClient<services::Auth, T>;

#[cfg(test)]
#[serial_test::serial(auth)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn current_user() {
        let before = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        let mut client = Client::new().await.unwrap().into_auth();
        client.current_user().await.unwrap();
        let after = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        assert_eq!(after - before, 1);
    }
}
