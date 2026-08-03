use crate::health_checks::check;
use crate::rpc::services;

/// The HealthChecksService provides methods to verify the health of the
/// cluster.
pub type HealthChecks<T = tonic::transport::Channel> = super::ServiceClient<services::HealthChecks, T>;

impl<T: super::Channel> super::ServiceClient<services::HealthChecks, T> {
    /// Checks the health of the cluster. This can be used to verify that the cluster is up and running.
    pub async fn check(
        &mut self,
    ) -> Result<Vec<crate::health_checks::ServiceHealth>, super::RequestError> {
        Ok(self.call(check::Request {}).await?.services)
    }
}

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
