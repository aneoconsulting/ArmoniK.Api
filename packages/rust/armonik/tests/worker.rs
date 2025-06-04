use std::sync::Arc;

use armonik::{
    server::{RequestContext, WorkerServiceExt},
    worker,
};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::WorkerService for Service {
    async fn health_check(
        self: Arc<Self>,
        _request: worker::health_check::Request,
        _context: RequestContext,
    ) -> std::result::Result<worker::health_check::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(worker::health_check::Response::Serving)
        })
        .await
    }

    async fn process(
        self: Arc<Self>,
        _request: worker::process::Request,
        _context: RequestContext,
    ) -> std::result::Result<worker::process::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(worker::process::Response {
                output: armonik::Output::Error {
                    details: String::from("rpc-process-output"),
                },
            })
        })
        .await
    }
}

#[tokio::test]
async fn health_check() {
    let mut client =
        armonik::Client::with_channel(Service::default().worker_server()).into_worker();

    let response = client.health_check().await.unwrap();

    assert_eq!(response, worker::health_check::Response::Serving);
}

#[tokio::test]
async fn process() {
    let mut client =
        armonik::Client::with_channel(Service::default().worker_server()).into_worker();

    let response = client
        .process(worker::process::Request {
            communication_token: String::from("rpc-process-input"),
            session_id: Default::default(),
            task_id: Default::default(),
            task_options: Default::default(),
            expected_output_keys: Default::default(),
            payload_id: Default::default(),
            data_dependencies: Default::default(),
            data_folder: Default::default(),
            configuration: Default::default(),
        })
        .await
        .unwrap();

    match response {
        armonik::Output::Ok => panic!("Unexpected ok"),
        armonik::Output::Error { details } => assert_eq!(details, "rpc-process-output"),
    }
}
