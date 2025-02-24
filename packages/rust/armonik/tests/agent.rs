use std::sync::Arc;

use armonik::{agent, reexports::tokio_stream::StreamExt, server::AgentServiceExt};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::AgentService for Service {
    async fn create_results_metadata(
        self: Arc<Self>,
        request: agent::create_results_metadata::Request,
    ) -> std::result::Result<agent::create_results_metadata::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::create_results_metadata::Response {
                communication_token: request.communication_token,
                results: request
                    .names
                    .into_iter()
                    .map(|name| {
                        (
                            name.clone(),
                            agent::ResultMetaData {
                                session_id: String::from("rpc-create-results-metadata-output"),
                                name,
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
            })
        })
        .await
    }

    async fn create_results(
        self: Arc<Self>,
        request: agent::create_results::Request,
    ) -> std::result::Result<agent::create_results::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::create_results::Response {
                communication_token: request.communication_token,
                results: request
                    .results
                    .into_keys()
                    .map(|name| {
                        eprintln!("NAME: {name}");
                        (
                            name.clone(),
                            agent::ResultMetaData {
                                name,
                                session_id: String::from("rpc-create-results-output"),
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
            })
        })
        .await
    }

    async fn notify_result_data(
        self: Arc<Self>,
        request: agent::notify_result_data::Request,
    ) -> std::result::Result<agent::notify_result_data::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::notify_result_data::Response {
                result_ids: vec![
                    request.communication_token,
                    String::from("rpc-notify-result-data-output"),
                ],
            })
        })
        .await
    }

    async fn submit_tasks(
        self: Arc<Self>,
        request: agent::submit_tasks::Request,
    ) -> std::result::Result<agent::submit_tasks::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::submit_tasks::Response {
                communication_token: request.communication_token,
                items: vec![agent::submit_tasks::ResponseItem {
                    task_id: String::from("rpc-submit-tasks-output"),
                    ..Default::default()
                }],
            })
        })
        .await
    }

    async fn get_resource_data(
        self: Arc<Self>,
        _request: agent::get_resource_data::Request,
    ) -> std::result::Result<agent::get_resource_data::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::get_resource_data::Response {
                result_id: String::from("rpc-get-resource-data-output"),
            })
        })
        .await
    }

    async fn get_common_data(
        self: Arc<Self>,
        _request: agent::get_common_data::Request,
    ) -> std::result::Result<agent::get_common_data::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::get_common_data::Response {
                result_id: String::from("rpc-get-common-data-output"),
            })
        })
        .await
    }

    async fn get_direct_data(
        self: Arc<Self>,
        _request: agent::get_direct_data::Request,
    ) -> std::result::Result<agent::get_direct_data::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::get_direct_data::Response {
                result_id: String::from("rpc-get-direct-data-output"),
            })
        })
        .await
    }

    async fn create_tasks(
        self: Arc<Self>,
        request: impl tonic::codegen::tokio_stream::Stream<
                Item = Result<agent::create_tasks::Request, tonic::Status>,
            > + Send
            + 'static,
    ) -> Result<agent::create_tasks::Response, tonic::Status> {
        let mut request = std::pin::pin!(request);
        let mut token = None;
        loop {
            match request.next().await {
                Some(Ok(agent::create_tasks::Request::InitTaskRequest {
                    communication_token,
                    ..
                })) => {
                    token = Some(communication_token);
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => return Err(err),
                None => break,
            }
        }
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(agent::create_tasks::Response::Status {
                communication_token: token.unwrap_or_default(),
                statuses: vec![agent::create_tasks::Status::TaskInfo {
                    task_id: String::from("rpc-create-tasks-output"),
                    expected_output_keys: vec![],
                    data_dependencies: vec![],
                    payload_id: String::new(),
                }],
            })
        })
        .await
    }
}

#[tokio::test]
async fn create_results_metadata() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .create_results_metadata("rpc-create-results-metadata-input", "", ["result-id"])
        .await
        .unwrap();

    assert_eq!(
        response["result-id"].session_id,
        "rpc-create-results-metadata-output"
    );
}

#[tokio::test]
async fn create_results() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .create_results("rpc-create-results-input", "", [("result-id", b"")])
        .await
        .unwrap();

    assert_eq!(
        response["result-id"].session_id,
        "rpc-create-results-output"
    );
}

#[tokio::test]
async fn notify_result_data() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .notify_result_data("rpc-notify-result-data-input", "", [""])
        .await
        .unwrap();

    assert_eq!(response[0], "rpc-notify-result-data-input");
    assert_eq!(response[1], "rpc-notify-result-data-output");
}

#[tokio::test]
async fn submit_tasks() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .submit_tasks("rpc-submit-tasks-input", "", None, [])
        .await
        .unwrap();

    assert_eq!(response[0].task_id, "rpc-submit-tasks-output");
}

#[tokio::test]
async fn get_resource_data() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .call(agent::get_resource_data::Request {
            communication_token: String::from("rpc-get-resource-data-input"),
            result_id: String::from(""),
        })
        .await
        .unwrap();

    assert_eq!(response.result_id, "rpc-get-resource-data-output");
}

#[tokio::test]
async fn get_common_data() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .call(agent::get_common_data::Request {
            communication_token: String::from("rpc-get-common-data-input"),
            result_id: String::from(""),
        })
        .await
        .unwrap();

    assert_eq!(response.result_id, "rpc-get-common-data-output");
}

#[tokio::test]
async fn get_direct_data() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .call(agent::get_direct_data::Request {
            communication_token: String::from("rpc-get-direct-data-input"),
            result_id: String::from(""),
        })
        .await
        .unwrap();

    assert_eq!(response.result_id, "rpc-get-direct-data-output");
}

#[tokio::test]
async fn create_tasks() {
    let mut client = armonik::Client::with_channel(Service::default().agent_server()).into_agent();

    let response = client
        .create_tasks(futures::stream::iter([
            agent::create_tasks::Request::InitRequest {
                communication_token: String::from("rpc-create-tasks-input"),
                request: agent::create_tasks::InitRequest { task_options: None },
            },
        ]))
        .await
        .unwrap();

    match &response[0] {
        agent::create_tasks::Status::TaskInfo { task_id, .. } => {
            assert_eq!(task_id, "rpc-create-tasks-output");
        }
        agent::create_tasks::Status::Error(err) => {
            panic!("Expected TaskInfo, but got Error({err})")
        }
    }
}
