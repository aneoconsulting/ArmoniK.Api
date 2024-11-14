use std::sync::Arc;

use armonik::{server::VersionsServiceExt, versions};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::VersionsService for Service {
    async fn list(
        self: Arc<Self>,
        _request: versions::list::Request,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> std::result::Result<versions::list::Response, tonic::Status> {
        common::unary_rpc_impl(
            self.wait.clone(),
            self.failure.clone(),
            cancellation_token,
            || {
                Ok(versions::list::Response {
                    core: String::from("rpc-list-output"),
                    ..Default::default()
                })
            },
        )
        .await
    }
}

#[tokio::test]
async fn list() {
    let mut client = armonik::Client::with_channel(Service::default().versions_server()).versions();

    let response = client.list().await.unwrap();

    assert_eq!(response.core, "rpc-list-output");
}
