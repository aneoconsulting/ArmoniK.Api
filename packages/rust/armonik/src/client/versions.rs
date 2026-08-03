use crate::rpc::services;
use crate::versions::list;

/// Service for getting versions of the components.
pub type Versions<T = tonic::transport::Channel> = super::ServiceClient<services::Versions, T>;

impl<T: super::Channel> super::ServiceClient<services::Versions, T> {
    pub async fn list(&mut self) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {}).await
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
