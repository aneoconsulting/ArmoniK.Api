#![allow(deprecated)]

use armonik::reexports::tokio_stream::StreamExt;
use armonik::server::{RequestContext, SubmitterServiceExt};
use armonik::submitter;

#[macro_use]
mod common;

/// The `TaskFilter` the id-taking RPCs are driven with, in both halves of the
/// pair.
fn task_filter(session_id: &str) -> submitter::TaskFilter {
    submitter::TaskFilter {
        ids: submitter::TaskFilterIds::Sessions(vec![String::from(session_id)]),
        statuses: Default::default(),
    }
}

/// The session id a filter-taking RPC is asserted against.
fn filter_session(filter: &submitter::TaskFilter) -> &str {
    match &filter.ids {
        submitter::TaskFilterIds::Sessions(ids) => &ids[0],
        submitter::TaskFilterIds::Tasks(_) => panic!("Expected a session filter"),
    }
}

fn counts() -> armonik::Count {
    armonik::Count {
        values: [(armonik::TaskStatus::Creating, 1337)]
            .into_iter()
            .collect(),
    }
}

fn statuses(task_id: &str) -> submitter::create_tasks::Response {
    submitter::create_tasks::Response::Status(vec![submitter::create_tasks::Status::TaskInfo {
        task_id: String::from(task_id),
        expected_output_keys: Default::default(),
        data_dependencies: Default::default(),
        payload_id: Default::default(),
    }])
}

fn task_infos(response: submitter::create_tasks::Response) -> Vec<submitter::create_tasks::Status> {
    match response {
        submitter::create_tasks::Response::Status(statuses) => statuses,
        submitter::create_tasks::Response::Error(err) => panic!("Unexpected error {err:?}"),
    }
}

fn task_id(status: &submitter::create_tasks::Status) -> &str {
    match status {
        submitter::create_tasks::Status::TaskInfo { task_id, .. } => task_id,
        submitter::create_tasks::Status::Error(err) => panic!("Unexpected error {err:?}"),
    }
}

/// The stream `CreateLargeTasks` is driven with, in both halves of the pair.
fn create_large_tasks_request() -> impl futures::Stream<Item = submitter::create_tasks::LargeRequest>
{
    futures::stream::iter([
        submitter::create_tasks::LargeRequest::InitRequest(submitter::create_tasks::InitRequest {
            session_id: String::from("create-large-tasks-input"),
            task_options: None,
        }),
        submitter::create_tasks::LargeRequest::Invalid,
    ])
}

rpc_tests! {
    client: into_submitter;
    server: SubmitterService, submitter_server;
    mock: "Submitter";

    rpc unary get_service_configuration {
        request: submitter::get_service_configuration::Request {},
        respond: |_request| armonik::Configuration {
            data_chunk_max_size: 1337,
        },
        convenience: get_service_configuration(),
        check: |configuration| {
            assert_eq!(configuration.data_chunk_max_size, 1337);
        },
    }

    rpc unary create_session {
        request: submitter::create_session::Request {
            partition_ids: vec![String::from("create-session-input")],
            default_task_options: Default::default(),
        },
        respond: |request: submitter::create_session::Request| {
            assert_eq!(request.partition_ids[0], "create-session-input");
            submitter::create_session::Response {
                session_id: String::from("create-session-output"),
            }
        },
        convenience: create_session(["create-session-input"], Default::default()),
        project: |response| response.session_id,
        check: |session_id| {
            assert_eq!(session_id, "create-session-output");
        },
    }

    rpc unary cancel_session {
        request: submitter::cancel_session::Request {
            session_id: String::from("cancel-session-input"),
        },
        respond: |request: submitter::cancel_session::Request| {
            assert_eq!(request.session_id, "cancel-session-input");
            submitter::cancel_session::Response {}
        },
        convenience: cancel_session("cancel-session-input"),
        project: |_response| (),
        check: |_| {},
    }

    rpc unary create_small_tasks {
        request: submitter::create_tasks::SmallRequest {
            session_id: String::from("create-small-tasks-input"),
            task_options: None,
            task_requests: vec![armonik::TaskRequest::default()],
        },
        respond: |request: submitter::create_tasks::SmallRequest| {
            assert_eq!(request.session_id, "create-small-tasks-input");
            statuses("create-small-tasks-output")
        },
        convenience: create_small_tasks(
            "create-small-tasks-input",
            None,
            [armonik::TaskRequest::default()],
        ),
        project: task_infos,
        check: |statuses| {
            assert_eq!(task_id(&statuses[0]), "create-small-tasks-output");
        },
        mock_error: |status: &tonic::Status| {
            status.code() == tonic::Code::Internal && status.message().is_empty()
        },
    }

    rpc client_stream create_large_tasks {
        request: create_large_tasks_request(),
        convenience: create_large_tasks(create_large_tasks_request()),
        project: task_infos,
        check: |statuses| {
            assert_eq!(task_id(&statuses[0]), "create-large-tasks-output");
        },
        mock_error: |status: &tonic::Status| {
            status.code() == tonic::Code::Internal && status.message().is_empty()
        },
    }

    rpc unary list_tasks {
        request: submitter::list_tasks::Request {
            filter: task_filter("list-tasks-input"),
        },
        respond: |request: submitter::list_tasks::Request| {
            assert_eq!(filter_session(&request.filter), "list-tasks-input");
            submitter::list_tasks::Response {
                task_ids: vec![String::from("list-tasks-output")],
            }
        },
        convenience: list_tasks(task_filter("list-tasks-input")),
        project: |response| response.task_ids,
        check: |task_ids| {
            assert_eq!(task_ids[0], "list-tasks-output");
        },
    }

    rpc unary list_sessions {
        request: submitter::list_sessions::Request {
            filter: submitter::SessionFilter {
                ids: vec![String::from("list-sessions-input")],
                statuses: Default::default(),
            },
        },
        respond: |request: submitter::list_sessions::Request| {
            assert_eq!(request.filter.ids[0], "list-sessions-input");
            submitter::list_sessions::Response {
                session_ids: vec![String::from("list-sessions-output")],
            }
        },
        convenience: list_sessions(submitter::SessionFilter {
            ids: vec![String::from("list-sessions-input")],
            statuses: Default::default(),
        }),
        project: |response| response.session_ids,
        check: |session_ids| {
            assert_eq!(session_ids[0], "list-sessions-output");
        },
    }

    rpc unary count_tasks {
        request: submitter::count_tasks::Request {
            filter: task_filter("count-tasks-input"),
        },
        respond: |request: submitter::count_tasks::Request| {
            assert_eq!(filter_session(&request.filter), "count-tasks-input");
            counts()
        },
        convenience: count_tasks(task_filter("count-tasks-input")),
        project: |response| response.values,
        check: |values| {
            assert_eq!(values[&armonik::TaskStatus::Creating], 1337);
        },
    }

    rpc server_stream try_get_result {
        request: submitter::try_get_result::Request {
            session_id: String::from("try-get-result-input"),
            result_id: String::from("result-id"),
        },
        convenience: try_get_result("try-get-result-input", "result-id"),
        check: |mut stream| async move {
            let reply = stream.next().await.unwrap().unwrap();
            match reply {
                submitter::try_get_result::Response::NotCompleted(reason) => {
                    assert_eq!(reason, "try-get-result-output")
                }
                reply => panic!("Unexpected reply {reply:?}"),
            }
            assert!(stream.next().await.is_none());
        },
    }

    rpc unary try_get_task_output {
        request: submitter::try_get_task_output::Request {
            session_id: String::from("try-get-task-output-input"),
            task_id: String::from("task-id"),
        },
        respond: |request: submitter::try_get_task_output::Request| {
            assert_eq!(request.session_id, "try-get-task-output-input");
            armonik::Output::Ok
        },
        convenience: try_get_task_output("try-get-task-output-input", "task-id"),
        project: |output| match output {
            armonik::Output::Ok => (),
            armonik::Output::Error { details } => panic!("Unexpected error {details:?}"),
        },
        check: |_| {},
    }

    rpc unary wait_for_availability {
        request: submitter::wait_for_availability::Request {
            session_id: String::from("wait-for-availability-input"),
            result_id: String::from("result-id"),
        },
        respond: |request: submitter::wait_for_availability::Request| {
            assert_eq!(request.session_id, "wait-for-availability-input");
            submitter::wait_for_availability::Response::NotCompleted(String::from(
                "wait-for-availability-output",
            ))
        },
        convenience: wait_for_availability("wait-for-availability-input", "result-id"),
        check: |response| match response {
            submitter::wait_for_availability::Response::NotCompleted(reason) => {
                assert_eq!(reason, "wait-for-availability-output")
            }
            response => panic!("Unexpected response {response:?}"),
        },
    }

    rpc unary wait_for_completion {
        request: submitter::wait_for_completion::Request {
            filter: task_filter("wait-for-completion-input"),
            stop_on_first_task_error: false,
            stop_on_first_task_cancellation: false,
        },
        respond: |request: submitter::wait_for_completion::Request| {
            assert_eq!(filter_session(&request.filter), "wait-for-completion-input");
            counts()
        },
        convenience: wait_for_completion(
            task_filter("wait-for-completion-input"),
            false,
            false,
        ),
        project: |response| response.values,
        check: |values| {
            assert_eq!(values[&armonik::TaskStatus::Creating], 1337);
        },
    }

    rpc unary cancel_tasks {
        request: submitter::cancel_tasks::Request {
            filter: task_filter("cancel-tasks-input"),
        },
        respond: |request: submitter::cancel_tasks::Request| {
            assert_eq!(filter_session(&request.filter), "cancel-tasks-input");
            submitter::cancel_tasks::Response {}
        },
        convenience: cancel_tasks(task_filter("cancel-tasks-input")),
        project: |_response| (),
        check: |_| {},
    }

    rpc unary task_status {
        request: submitter::task_status::Request {
            task_ids: vec![String::from("task-status-input")],
        },
        respond: |request: submitter::task_status::Request| {
            assert_eq!(request.task_ids[0], "task-status-input");
            submitter::task_status::Response {
                statuses: [(
                    String::from("task-status-output"),
                    armonik::TaskStatus::Creating,
                )]
                .into_iter()
                .collect(),
            }
        },
        convenience: task_status(["task-status-input"]),
        project: |response| response.statuses,
        check: |statuses| {
            assert_eq!(
                statuses[&String::from("task-status-output")],
                armonik::TaskStatus::Creating
            );
        },
    }

    rpc unary result_status {
        request: submitter::result_status::Request {
            session_id: String::from("result-status-input"),
            result_ids: vec![String::from("result-id")],
        },
        respond: |request: submitter::result_status::Request| {
            assert_eq!(request.session_id, "result-status-input");
            submitter::result_status::Response {
                statuses: [(
                    String::from("result-status-output"),
                    armonik::ResultStatus::Created,
                )]
                .into_iter()
                .collect(),
            }
        },
        convenience: result_status("result-status-input", ["result-id"]),
        project: |response| response.statuses,
        check: |statuses| {
            assert_eq!(
                statuses[&String::from("result-status-output")],
                armonik::ResultStatus::Created
            );
        },
    }

    manual {
        // Reads the session id off the first message, so it cannot be a plain
        // function of one request.
        async fn create_large_tasks(
            self: std::sync::Arc<Self>,
            request: impl armonik::reexports::tokio_stream::Stream<
                    Item = Result<submitter::create_tasks::LargeRequest, tonic::Status>,
                > + Send
                + 'static,
            _context: RequestContext,
        ) -> Result<submitter::create_tasks::Response, tonic::Status> {
            let mut request = std::pin::pin!(request);

            match request.next().await {
                Some(Ok(submitter::create_tasks::LargeRequest::InitRequest(
                    submitter::create_tasks::InitRequest { session_id, .. },
                ))) => assert_eq!(session_id, "create-large-tasks-input"),
                message => panic!("Expected an InitRequest message, but got {message:?}"),
            }

            while let Some(Ok(_)) = request.next().await {}

            Ok(statuses("create-large-tasks-output"))
        }

        async fn try_get_result(
            self: std::sync::Arc<Self>,
            request: submitter::try_get_result::Request,
            _context: RequestContext,
        ) -> Result<
            impl armonik::reexports::tokio_stream::Stream<
                    Item = Result<submitter::try_get_result::Response, tonic::Status>,
                > + Send,
            tonic::Status,
        > {
            assert_eq!(request.session_id, "try-get-result-input");
            Ok(futures::stream::iter([Ok(
                submitter::try_get_result::Response::NotCompleted(String::from(
                    "try-get-result-output",
                )),
            )]))
        }
    }
}
