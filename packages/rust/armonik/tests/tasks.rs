use std::sync::Arc;

use armonik::{
    server::{RequestContext, TasksServiceExt},
    tasks,
};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::TasksService for Service {
    async fn list(
        self: Arc<Self>,
        request: tasks::list::Request,
        _context: RequestContext,
    ) -> std::result::Result<tasks::list::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(tasks::list::Response {
                tasks: vec![tasks::Summary {
                    task_id: String::from("rpc-list-output"),
                    ..Default::default()
                }],
                page: request.page,
                page_size: request.page_size,
                total: 1337,
            })
        })
        .await
    }

    async fn list_detailed(
        self: Arc<Self>,
        request: tasks::list_detailed::Request,
        _context: RequestContext,
    ) -> std::result::Result<tasks::list_detailed::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(tasks::list_detailed::Response {
                tasks: vec![tasks::Raw {
                    task_id: String::from("rpc-list-detailed-output"),
                    ..Default::default()
                }],
                page: request.page,
                page_size: request.page_size,
                total: 1338,
            })
        })
        .await
    }

    async fn get(
        self: Arc<Self>,
        request: tasks::get::Request,
        _context: RequestContext,
    ) -> std::result::Result<tasks::get::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(tasks::get::Response {
                task: tasks::Raw {
                    session_id: String::from("rpc-get-output"),
                    task_id: request.task_id,
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn cancel(
        self: Arc<Self>,
        request: tasks::cancel::Request,
        _context: RequestContext,
    ) -> std::result::Result<tasks::cancel::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(tasks::cancel::Response {
                tasks: request
                    .task_ids
                    .into_iter()
                    .map(|task_id| tasks::Summary {
                        session_id: String::from("rpc-cancel-output"),
                        task_id,
                        ..Default::default()
                    })
                    .collect(),
            })
        })
        .await
    }

    async fn get_result_ids(
        self: Arc<Self>,
        request: tasks::get_result_ids::Request,
        _context: RequestContext,
    ) -> std::result::Result<tasks::get_result_ids::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(tasks::get_result_ids::Response {
                task_results: request
                    .task_ids
                    .into_iter()
                    .map(|task_id| (task_id, vec![String::from("rpc-get-result-ids-output")]))
                    .collect(),
            })
        })
        .await
    }

    async fn count_status(
        self: Arc<Self>,
        _request: tasks::count_status::Request,
        _context: RequestContext,
    ) -> std::result::Result<tasks::count_status::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(tasks::count_status::Response {
                status: vec![armonik::StatusCount {
                    status: armonik::TaskStatus::Creating,
                    count: 1337,
                }],
            })
        })
        .await
    }

    async fn submit(
        self: Arc<Self>,
        request: tasks::submit::Request,
        _context: RequestContext,
    ) -> std::result::Result<tasks::submit::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(tasks::submit::Response {
                items: request
                    .items
                    .into_iter()
                    .map(|item| tasks::submit::ResponseItem {
                        task_id: String::from("rpc-submit-output"),
                        payload_id: item.payload_id,
                        ..Default::default()
                    })
                    .collect(),
            })
        })
        .await
    }
}

#[tokio::test]
async fn list() {
    let mut client = armonik::Client::with_channel(Service::default().tasks_server()).into_tasks();

    let response = client
        .list(
            armonik::tasks::filter::Or::default(),
            armonik::tasks::Sort::default(),
            false,
            3,
            12,
        )
        .await
        .unwrap();

    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 12);
    assert_eq!(response.total, 1337);
    assert_eq!(response.tasks[0].task_id, "rpc-list-output");
}

#[tokio::test]
async fn list_detailed() {
    let mut client = armonik::Client::with_channel(Service::default().tasks_server()).into_tasks();

    let response = client
        .list_detailed(
            armonik::tasks::filter::Or::default(),
            armonik::tasks::Sort::default(),
            false,
            3,
            12,
        )
        .await
        .unwrap();

    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 12);
    assert_eq!(response.total, 1338);
    assert_eq!(response.tasks[0].task_id, "rpc-list-detailed-output");
}

#[tokio::test]
async fn get() {
    let mut client = armonik::Client::with_channel(Service::default().tasks_server()).into_tasks();

    let response = client.get("rpc-get-input").await.unwrap();

    assert_eq!(response.task_id, "rpc-get-input");
    assert_eq!(response.session_id, "rpc-get-output");
}

#[tokio::test]
async fn cancel() {
    let mut client = armonik::Client::with_channel(Service::default().tasks_server()).into_tasks();

    let response = client.cancel(["rpc-cancel-input"]).await.unwrap();

    assert_eq!(response[0].task_id, "rpc-cancel-input");
    assert_eq!(response[0].session_id, "rpc-cancel-output");
}

#[tokio::test]
async fn get_result_ids() {
    let mut client = armonik::Client::with_channel(Service::default().tasks_server()).into_tasks();

    let response = client
        .get_result_ids(["rpc-get-result-ids-input"])
        .await
        .unwrap();

    assert_eq!(
        response["rpc-get-result-ids-input"][0],
        "rpc-get-result-ids-output"
    );
}

#[tokio::test]
async fn count_status() {
    let mut client = armonik::Client::with_channel(Service::default().tasks_server()).into_tasks();

    let response = client
        .count_status(tasks::filter::Or::default())
        .await
        .unwrap();

    assert_eq!(response[0].count, 1337);
}

#[tokio::test]
async fn submit() {
    let mut client = armonik::Client::with_channel(Service::default().tasks_server()).into_tasks();

    let response = client
        .submit(
            "session-id",
            None,
            [tasks::submit::RequestItem {
                payload_id: String::from("rpc-submit-input"),
                ..Default::default()
            }],
        )
        .await
        .unwrap();

    assert_eq!(response[0].payload_id, "rpc-submit-input");
    assert_eq!(response[0].task_id, "rpc-submit-output");
}
