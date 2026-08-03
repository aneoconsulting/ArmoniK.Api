use crate::rpc::services;

/// The HealthChecksService provides methods to verify the health of the
/// cluster.
pub type HealthChecks<T = tonic::transport::Channel> = super::ServiceClient<services::HealthChecks, T>;

#[cfg(test)]
#[serial_test::serial(health_checks)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn check() {
        let before = Client::get_nb_request("HealthChecks", "CheckHealth").await;
        let mut client = Client::new().await.unwrap().into_health_checks();
        client.check().await.unwrap();
        let after = Client::get_nb_request("HealthChecks", "CheckHealth").await;
        assert_eq!(after - before, 1);
    }
}
