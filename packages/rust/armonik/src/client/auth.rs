use crate::auth::{current_user, User};
use crate::rpc::services;

/// Service for authentication management.
pub type Auth<T = tonic::transport::Channel> = super::ServiceClient<services::Auth, T>;

impl<T: super::Channel> super::ServiceClient<services::Auth, T> {
    /// Get current user
    pub async fn current_user(&mut self) -> Result<User, super::RequestError> {
        Ok(self.call(current_user::Request {}).await?.user)
    }
}

#[cfg(test)]
#[serial_test::serial(auth)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn current_user() {
        let before = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        let mut client = Client::new().await.unwrap().into_auth();
        client.current_user().await.unwrap();
        let after = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn current_user_call() {
        let before = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        let mut client = Client::new().await.unwrap().into_auth();
        client
            .call(crate::auth::current_user::Request {})
            .await
            .unwrap();
        let after = Client::get_nb_request("Authentication", "GetCurrentUser").await;
        assert_eq!(after - before, 1);
    }
}
