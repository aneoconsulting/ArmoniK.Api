use std::sync::Arc;

use armonik::{applications, server::ApplicationsServiceExt};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::ApplicationsService for Service {
    async fn list(
        self: Arc<Self>,
        request: applications::list::Request,
    ) -> std::result::Result<applications::list::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(applications::list::Response {
                applications: vec![applications::Raw {
                    name: String::from("rpc-list-output"),
                    ..Default::default()
                }],
                page: request.page,
                page_size: request.page_size,
                total: 1337,
            })
        })
        .await
    }
}

#[tokio::test]
async fn list() {
    let mut client =
        armonik::Client::with_channel(Service::default().applications_server()).into_applications();

    let response = client
        .list(
            armonik::applications::filter::Or::default(),
            armonik::applications::Sort::default(),
            3,
            12,
        )
        .await
        .unwrap();

    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 12);
    assert_eq!(response.total, 1337);
    assert_eq!(response.applications[0].name, "rpc-list-output");
}
