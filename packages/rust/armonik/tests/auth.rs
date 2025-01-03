use std::sync::Arc;

use armonik::{auth, server::AuthServiceExt};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::AuthService for Service {
    async fn current_user(
        self: Arc<Self>,
        _request: auth::current_user::Request,
    ) -> std::result::Result<auth::current_user::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(auth::current_user::Response {
                user: auth::User {
                    username: String::from("rpc-current-user-output"),
                    ..Default::default()
                },
            })
        })
        .await
    }
}

#[tokio::test]
async fn current_user() {
    let mut client = armonik::Client::with_channel(Service::default().auth_server()).into_auth();

    let response = client.current_user().await.unwrap();

    assert_eq!(response.username, "rpc-current-user-output");
}
