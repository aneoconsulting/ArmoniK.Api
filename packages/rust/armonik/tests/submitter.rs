#![allow(deprecated)]

use std::sync::{Arc, Mutex};

use armonik::{
    reexports::tokio_stream::StreamExt,
    server::{RequestContext, SubmitterServiceExt},
    submitter,
};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    expected: Option<String>,
    called: Arc<Mutex<Option<String>>>,
}

impl armonik::server::SubmitterService for Service {
    async fn get_service_configuration(
        self: Arc<Self>,
        _request: submitter::get_service_configuration::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::get_service_configuration::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("get-service-configuration"));
        Ok(armonik::Configuration {
            data_chunk_max_size: 1337,
        })
    }

    async fn create_session(
        self: Arc<Self>,
        request: submitter::create_session::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::create_session::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("create-session"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.partition_ids[0], expected.as_str());
        }
        Ok(submitter::create_session::Response {
            session_id: String::from("create-session-output"),
        })
    }

    async fn cancel_session(
        self: Arc<Self>,
        request: submitter::cancel_session::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::cancel_session::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("cancel-session"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.session_id, expected.as_str());
        }
        Ok(submitter::cancel_session::Response {})
    }

    async fn list_tasks(
        self: Arc<Self>,
        request: submitter::list_tasks::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::list_tasks::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("list-tasks"));
        if let Some(expected) = &self.expected {
            match request.filter.ids {
                submitter::TaskFilterIds::Sessions(vec) => assert_eq!(vec[0], expected.as_str()),
                submitter::TaskFilterIds::Tasks(_) => panic!("Expected a session"),
            }
        }
        Ok(submitter::list_tasks::Response {
            task_ids: vec![String::from("list-tasks-output")],
        })
    }

    async fn list_sessions(
        self: Arc<Self>,
        request: submitter::list_sessions::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::list_sessions::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("list-sessions"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.filter.ids[0], expected.as_str());
        }
        Ok(submitter::list_sessions::Response {
            session_ids: vec![String::from("list-sessions-output")],
        })
    }

    async fn count_tasks(
        self: Arc<Self>,
        request: submitter::count_tasks::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::count_tasks::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("count-tasks"));
        if let Some(expected) = &self.expected {
            match request.filter.ids {
                submitter::TaskFilterIds::Sessions(vec) => assert_eq!(vec[0], expected.as_str()),
                submitter::TaskFilterIds::Tasks(_) => panic!("Expected a session"),
            }
        }
        Ok(armonik::Count {
            values: [(armonik::TaskStatus::Creating, 1337)]
                .into_iter()
                .collect(),
        })
    }

    async fn try_get_task_output(
        self: Arc<Self>,
        request: submitter::try_get_task_output::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::try_get_task_output::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("try-get-task-output"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.session_id, expected.as_str());
        }
        Ok(armonik::Output::Ok)
    }

    async fn wait_for_availability(
        self: Arc<Self>,
        request: submitter::wait_for_availability::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::wait_for_availability::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("wait-for-availability"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.session_id, expected.as_str());
        }
        Ok(submitter::wait_for_availability::Response::NotCompleted(
            String::from("wait-for-availability-output"),
        ))
    }

    async fn wait_for_completion(
        self: Arc<Self>,
        request: submitter::wait_for_completion::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::wait_for_completion::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("wait-for-completion"));
        if let Some(expected) = &self.expected {
            match request.filter.ids {
                submitter::TaskFilterIds::Sessions(vec) => assert_eq!(vec[0], expected.as_str()),
                submitter::TaskFilterIds::Tasks(_) => panic!("Expected a session"),
            }
        }
        Ok(armonik::Count {
            values: [(armonik::TaskStatus::Creating, 1337)]
                .into_iter()
                .collect(),
        })
    }

    async fn cancel_tasks(
        self: Arc<Self>,
        request: submitter::cancel_tasks::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::cancel_tasks::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("cancel-tasks"));
        if let Some(expected) = &self.expected {
            match request.filter.ids {
                submitter::TaskFilterIds::Sessions(vec) => assert_eq!(vec[0], expected.as_str()),
                submitter::TaskFilterIds::Tasks(_) => panic!("Expected a session"),
            }
        }
        Ok(submitter::cancel_tasks::Response {})
    }

    async fn task_status(
        self: Arc<Self>,
        request: submitter::task_status::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::task_status::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("task-status"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.task_ids[0], expected.as_str());
        }
        Ok(submitter::task_status::Response {
            statuses: [(
                String::from("task-status-output"),
                armonik::TaskStatus::Creating,
            )]
            .into_iter()
            .collect(),
        })
    }

    async fn result_status(
        self: Arc<Self>,
        request: submitter::result_status::Request,
        _context: RequestContext,
    ) -> std::result::Result<submitter::result_status::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("result-status"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.session_id, expected.as_str());
        }
        Ok(submitter::result_status::Response {
            statuses: [(
                String::from("result-status-output"),
                armonik::ResultStatus::Created,
            )]
            .into_iter()
            .collect(),
        })
    }

    async fn try_get_result(
        self: Arc<Self>,
        request: submitter::try_get_result::Request,
        _context: RequestContext,
    ) -> Result<
        impl tonic::codegen::tokio_stream::Stream<
                Item = Result<submitter::try_get_result::Response, tonic::Status>,
            > + Send,
        tonic::Status,
    > {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("try-get-result"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.session_id, expected.as_str());
        }
        Ok(futures::stream::iter([Ok(
            submitter::try_get_result::Response::default(),
        )]))
    }

    async fn create_small_tasks(
        self: Arc<Self>,
        request: submitter::create_tasks::SmallRequest,
        _context: RequestContext,
    ) -> Result<submitter::create_tasks::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("create-small-tasks"));
        if let Some(expected) = &self.expected {
            assert_eq!(request.session_id, expected.as_str());
        }

        Ok(submitter::create_tasks::Response::Status(vec![
            submitter::create_tasks::Status::TaskInfo {
                task_id: String::from("create-small-tasks-output"),
                expected_output_keys: Default::default(),
                data_dependencies: Default::default(),
                payload_id: Default::default(),
            },
        ]))
    }

    async fn create_large_tasks(
        self: Arc<Self>,
        request: impl tonic::codegen::tokio_stream::Stream<
                Item = Result<submitter::create_tasks::LargeRequest, tonic::Status>,
            > + Send
            + 'static,
        _context: RequestContext,
    ) -> Result<submitter::create_tasks::Response, tonic::Status> {
        _ = self
            .called
            .lock()
            .unwrap()
            .insert(String::from("create-large-tasks"));

        let mut request = std::pin::pin!(request);

        if let Some(expected) = &self.expected {
            match request.next().await {
                Some(Ok(submitter::create_tasks::LargeRequest::InitRequest(
                    submitter::create_tasks::InitRequest { session_id, .. },
                ))) => assert_eq!(session_id, expected.as_str()),
                _ => panic!("Expected an InitRequest message"),
            }
        }

        while let Some(Ok(_)) = request.next().await {}

        Ok(submitter::create_tasks::Response::Status(vec![
            submitter::create_tasks::Status::TaskInfo {
                task_id: String::from("create-large-tasks-output"),
                expected_output_keys: Default::default(),
                data_dependencies: Default::default(),
                payload_id: Default::default(),
            },
        ]))
    }
}

#[tokio::test]
async fn get_service_configuration() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            called: called.clone(),
            ..Default::default()
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client.get_service_configuration().await.unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "get-service-configuration"
    );

    assert_eq!(response.data_chunk_max_size, 1337);
}

#[tokio::test]
async fn create_session() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("create-session-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .create_session(["create-session-input"], Default::default())
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "create-session"
    );

    assert_eq!(response, "create-session-output");
}

#[tokio::test]
async fn cancel_session() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("cancel-session-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    client.cancel_session("cancel-session-input").await.unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "cancel-session"
    );
}

#[tokio::test]
async fn list_tasks() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("list-tasks-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .list_tasks(submitter::TaskFilter {
            ids: submitter::TaskFilterIds::Sessions(vec![String::from("list-tasks-input")]),
            statuses: Default::default(),
        })
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "list-tasks"
    );

    assert_eq!(response[0], "list-tasks-output");
}

#[tokio::test]
async fn list_sessions() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("list-sessions-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .list_sessions(submitter::SessionFilter {
            ids: vec![String::from("list-sessions-input")],
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "list-sessions"
    );

    assert_eq!(response[0], "list-sessions-output");
}

#[tokio::test]
async fn count_tasks() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("count-tasks-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .count_tasks(submitter::TaskFilter {
            ids: submitter::TaskFilterIds::Sessions(vec![String::from("count-tasks-input")]),
            statuses: Default::default(),
        })
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "count-tasks"
    );

    assert_eq!(response[&armonik::TaskStatus::Creating], 1337);
}

#[tokio::test]
async fn try_get_task_output() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("try-get-task-output-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    client
        .try_get_task_output("try-get-task-output-input", "task-id")
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "try-get-task-output"
    );
}

#[tokio::test]
async fn wait_for_availability() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("wait-for-availability-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .wait_for_availability("wait-for-availability-input", "result-id")
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "wait-for-availability"
    );

    match response {
        submitter::wait_for_availability::Response::NotCompleted(res) => {
            assert_eq!(res, "wait-for-availability-output")
        }
        _ => panic!("Unexpected output"),
    }
}

#[tokio::test]
async fn wait_for_completion() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("wait-for-completion-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .wait_for_completion(
            submitter::TaskFilter {
                ids: submitter::TaskFilterIds::Sessions(vec![String::from(
                    "wait-for-completion-input",
                )]),
                statuses: Default::default(),
            },
            false,
            false,
        )
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "wait-for-completion"
    );

    assert_eq!(response[&armonik::TaskStatus::Creating], 1337);
}

#[tokio::test]
async fn cancel_tasks() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("cancel-tasks-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    client
        .cancel_tasks(submitter::TaskFilter {
            ids: submitter::TaskFilterIds::Sessions(vec![String::from("cancel-tasks-input")]),
            statuses: Default::default(),
        })
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "cancel-tasks"
    );
}

#[tokio::test]
async fn task_status() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("task-status-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client.task_status(["task-status-input"]).await.unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "task-status"
    );

    assert_eq!(response.into_keys().next().unwrap(), "task-status-output");
}

#[tokio::test]
async fn result_status() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("result-status-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .result_status("result-status-input", ["result-id"])
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "result-status"
    );

    assert_eq!(response.into_keys().next().unwrap(), "result-status-output");
}

#[tokio::test]
async fn try_get_result() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("try-get-result-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    _ = client
        .try_get_result("try-get-result-input", "result-id")
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "try-get-result"
    );
}

#[tokio::test]
async fn create_small_tasks() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("create-small-tasks-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .create_small_tasks(
            "create-small-tasks-input",
            None,
            [armonik::TaskRequest::default()],
        )
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "create-small-tasks"
    );

    match &response[0] {
        submitter::create_tasks::Status::TaskInfo { task_id, .. } => {
            assert_eq!(task_id.as_str(), "create-small-tasks-output")
        }
        submitter::create_tasks::Status::Error(err) => panic!("Unexpected error {err:?}"),
    }
}

#[tokio::test]
async fn create_large_tasks() {
    let called = Arc::new(Mutex::default());
    let mut client = armonik::Client::with_channel(
        Service {
            expected: Some(String::from("create-large-tasks-input")),
            called: called.clone(),
        }
        .submitter_server(),
    )
    .into_submitter();

    let response = client
        .create_large_tasks(futures::stream::iter([
            submitter::create_tasks::LargeRequest::InitRequest(
                submitter::create_tasks::InitRequest {
                    session_id: String::from("create-large-tasks-input"),
                    ..Default::default()
                },
            ),
            submitter::create_tasks::LargeRequest::Invalid,
            submitter::create_tasks::LargeRequest::Invalid,
        ]))
        .await
        .unwrap();

    assert_eq!(
        called.lock().as_ref().unwrap().as_ref().unwrap(),
        "create-large-tasks"
    );

    match &response[0] {
        submitter::create_tasks::Status::TaskInfo { task_id, .. } => {
            assert_eq!(task_id.as_str(), "create-large-tasks-output")
        }
        submitter::create_tasks::Status::Error(err) => panic!("Unexpected error {err:?}"),
    }
}
