use crate::rpc::services;

/// Service for getting versions of the components.
pub type Versions<T = tonic::transport::Channel> = super::ServiceClient<services::Versions, T>;

#[cfg(test)]
#[serial_test::serial(versions)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Versions", "ListVersions").await;
        let mut client = Client::new().await.unwrap().into_versions();
        client.list().await.unwrap();
        let after = Client::get_nb_request("Versions", "ListVersions").await;
        assert_eq!(after - before, 1);
    }
}
