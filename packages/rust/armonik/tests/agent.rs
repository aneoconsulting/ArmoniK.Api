use armonik::agent;
use armonik::reexports::tokio_stream::StreamExt;
use armonik::server::{AgentServiceExt, RequestContext};

#[macro_use]
mod common;

/// The stream `CreateTask` is driven with, in both halves of the pair.
fn create_tasks_request() -> impl futures::Stream<Item = agent::create_tasks::Request> {
    futures::stream::iter([agent::create_tasks::Request::InitRequest {
        communication_token: String::from("rpc-create-tasks-input"),
        request: agent::create_tasks::InitRequest { task_options: None },
    }])
}

rpc_tests! {
    client: into_agent;
    server: AgentService, agent_server;
    mock: "Agent";

    rpc unary create_results_metadata {
        request: agent::create_results_metadata::Request {
            communication_token: String::from("rpc-create-results-metadata-input"),
            session_id: String::new(),
            results: vec![agent::create_results_metadata::RequestItem {
                name: String::from("result-id"),
            }],
        },
        respond: |request: agent::create_results_metadata::Request| {
            agent::create_results_metadata::Response {
                communication_token: request.communication_token,
                results: request
                    .results
                    .into_iter()
                    .map(|item| agent::ResultMetaData {
                        session_id: String::from("rpc-create-results-metadata-output"),
                        name: item.name,
                        ..Default::default()
                    })
                    .collect(),
            }
        },
        convenience: create_results_metadata(
            "rpc-create-results-metadata-input",
            "",
            ["result-id"],
        ),
        project: |response| response.results,
        check: |results| {
            assert_eq!(results[0].name, "result-id");
            assert_eq!(results[0].session_id, "rpc-create-results-metadata-output");
        },
    }

    rpc unary create_results {
        request: agent::create_results::Request {
            communication_token: String::from("rpc-create-results-input"),
            session_id: String::new(),
            results: vec![agent::create_results::RequestItem {
                name: String::from("result-id"),
                data: bytes::Bytes::new(),
            }],
        },
        respond: |request: agent::create_results::Request| agent::create_results::Response {
            communication_token: request.communication_token,
            results: request
                .results
                .into_iter()
                .map(|item| agent::ResultMetaData {
                    name: item.name,
                    session_id: String::from("rpc-create-results-output"),
                    ..Default::default()
                })
                .collect(),
        },
        convenience: create_results(
            "rpc-create-results-input",
            "",
            [("result-id", b"".as_slice())],
        ),
        project: |response| response.results,
        check: |results| {
            assert_eq!(results[0].name, "result-id");
            assert_eq!(results[0].session_id, "rpc-create-results-output");
        },
    }

    rpc unary notify_result_data {
        request: agent::notify_result_data::Request {
            communication_token: String::from("rpc-notify-result-data-input"),
            session_id: String::new(),
            result_ids: vec![String::new()],
        },
        respond: |request: agent::notify_result_data::Request| {
            agent::notify_result_data::Response {
                result_ids: vec![
                    request.communication_token,
                    String::from("rpc-notify-result-data-output"),
                ],
            }
        },
        convenience: notify_result_data("rpc-notify-result-data-input", "", [""]),
        project: |response| response.result_ids,
        check: |result_ids| {
            assert_eq!(result_ids[0], "rpc-notify-result-data-input");
            assert_eq!(result_ids[1], "rpc-notify-result-data-output");
        },
    }

    rpc unary submit_tasks {
        request: agent::submit_tasks::Request {
            communication_token: String::from("rpc-submit-tasks-input"),
            session_id: String::new(),
            task_options: None,
            items: Vec::new(),
        },
        respond: |request: agent::submit_tasks::Request| agent::submit_tasks::Response {
            communication_token: request.communication_token,
            items: vec![agent::submit_tasks::ResponseItem {
                task_id: String::from("rpc-submit-tasks-output"),
                ..Default::default()
            }],
        },
        convenience: submit_tasks(
            "rpc-submit-tasks-input",
            "",
            None,
            Vec::<agent::submit_tasks::RequestItem>::new(),
        ),
        project: |response| response.items,
        check: |items| {
            assert_eq!(items[0].task_id, "rpc-submit-tasks-output");
        },
    }

    rpc unary get_resource_data {
        request: agent::get_resource_data::Request {
            communication_token: String::from("rpc-get-resource-data-input"),
            result_id: String::new(),
        },
        respond: |_request| agent::get_resource_data::Response {
            result_id: String::from("rpc-get-resource-data-output"),
        },
        convenience: get_resource_data("rpc-get-resource-data-input", ""),
        project: |response| response.result_id,
        check: |result_id| {
            assert_eq!(result_id, "rpc-get-resource-data-output");
        },
    }

    rpc unary get_common_data {
        request: agent::get_common_data::Request {
            communication_token: String::from("rpc-get-common-data-input"),
            result_id: String::new(),
        },
        respond: |_request| agent::get_common_data::Response {
            result_id: String::from("rpc-get-common-data-output"),
        },
        convenience: get_common_data("rpc-get-common-data-input", ""),
        project: |response| response.result_id,
        check: |result_id| {
            assert_eq!(result_id, "rpc-get-common-data-output");
        },
    }

    rpc unary get_direct_data {
        request: agent::get_direct_data::Request {
            communication_token: String::from("rpc-get-direct-data-input"),
            result_id: String::new(),
        },
        respond: |_request| agent::get_direct_data::Response {
            result_id: String::from("rpc-get-direct-data-output"),
        },
        convenience: get_direct_data("rpc-get-direct-data-input", ""),
        project: |response| response.result_id,
        check: |result_id| {
            assert_eq!(result_id, "rpc-get-direct-data-output");
        },
    }

    rpc client_stream create_tasks {
        request: create_tasks_request(),
        convenience: create_tasks(create_tasks_request()),
        project: |response| match response {
            agent::create_tasks::Response::Status { statuses, .. } => statuses,
            agent::create_tasks::Response::Error { error, .. } => {
                panic!("Expected a status list, but got Error({error})")
            }
            agent::create_tasks::Response::Invalid { .. } => {
                panic!("Expected a status list, but the reply set no member")
            }
        },
        check: |statuses| match &statuses[0] {
            agent::create_tasks::Status::TaskInfo { task_id, .. } => {
                assert_eq!(task_id, "rpc-create-tasks-output");
            }
            agent::create_tasks::Status::Error(err) => {
                panic!("Expected TaskInfo, but got Error({err})")
            }
            agent::create_tasks::Status::Invalid => {
                panic!("Expected TaskInfo, but the status set no member")
            }
        },
    }

    manual {
        // Reads the token off the first message, so it cannot be a plain
        // function of one request.
        async fn create_tasks(
            self: std::sync::Arc<Self>,
            request: impl armonik::reexports::tokio_stream::Stream<
                    Item = Result<agent::create_tasks::Request, tonic::Status>,
                > + Send
                + 'static,
            _context: RequestContext,
        ) -> Result<agent::create_tasks::Response, tonic::Status> {
            let mut request = std::pin::pin!(request);
            let mut token = None;
            loop {
                match request.next().await {
                    Some(Ok(agent::create_tasks::Request::InitRequest {
                        communication_token,
                        ..
                    })) => token = Some(communication_token),
                    Some(Ok(_)) => {}
                    Some(Err(err)) => return Err(err),
                    None => break,
                }
            }
            common::stub(self.wait, self.failure.clone(), || {
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
}
