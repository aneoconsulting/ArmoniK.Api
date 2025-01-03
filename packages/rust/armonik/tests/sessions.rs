use std::sync::Arc;

use armonik::{server::SessionsServiceExt, sessions};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
}

impl armonik::server::SessionsService for Service {
    async fn list(
        self: Arc<Self>,
        request: sessions::list::Request,
    ) -> std::result::Result<sessions::list::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::list::Response {
                sessions: vec![sessions::Raw {
                    session_id: String::from("rpc-list-output"),
                    ..Default::default()
                }],
                page: request.page,
                page_size: request.page_size,
                total: 1337,
            })
        })
        .await
    }

    async fn get(
        self: Arc<Self>,
        request: sessions::get::Request,
    ) -> std::result::Result<sessions::get::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::get::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-get-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn cancel(
        self: Arc<Self>,
        request: sessions::cancel::Request,
    ) -> std::result::Result<sessions::cancel::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::cancel::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-cancel-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn create(
        self: Arc<Self>,
        _request: sessions::create::Request,
    ) -> std::result::Result<sessions::create::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::create::Response {
                session_id: String::from("rpc-create-output"),
            })
        })
        .await
    }

    async fn pause(
        self: Arc<Self>,
        request: sessions::pause::Request,
    ) -> std::result::Result<sessions::pause::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::pause::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-pause-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn resume(
        self: Arc<Self>,
        request: sessions::resume::Request,
    ) -> std::result::Result<sessions::resume::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::resume::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-resume-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn close(
        self: Arc<Self>,
        request: sessions::close::Request,
    ) -> std::result::Result<sessions::close::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::close::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-close-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn purge(
        self: Arc<Self>,
        request: sessions::purge::Request,
    ) -> std::result::Result<sessions::purge::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::purge::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-purge-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn delete(
        self: Arc<Self>,
        request: sessions::delete::Request,
    ) -> std::result::Result<sessions::delete::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::delete::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-delete-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn stop_submission(
        self: Arc<Self>,
        request: sessions::stop_submission::Request,
    ) -> std::result::Result<sessions::stop_submission::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait.clone(), self.failure.clone(), || {
            Ok(sessions::stop_submission::Response {
                session: sessions::Raw {
                    session_id: request.session_id,
                    partition_ids: vec![String::from("rpc-stop-output")],
                    ..Default::default()
                },
            })
        })
        .await
    }
}

#[tokio::test]
async fn list() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client
        .list(
            armonik::sessions::filter::Or::default(),
            armonik::sessions::Sort::default(),
            true,
            3,
            12,
        )
        .await
        .unwrap();

    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 12);
    assert_eq!(response.total, 1337);
    assert_eq!(response.sessions[0].session_id, "rpc-list-output");
}

#[tokio::test]
async fn get() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client.get("rpc-get-input").await.unwrap();

    assert_eq!(response.session_id, "rpc-get-input");
    assert_eq!(response.partition_ids[0], "rpc-get-output");
}

#[tokio::test]
async fn cancel() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client.cancel("rpc-cancel-input").await.unwrap();

    assert_eq!(response.session_id, "rpc-cancel-input");
    assert_eq!(response.partition_ids[0], "rpc-cancel-output");
}

#[tokio::test]
async fn create() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client
        .create(vec![String::from("rpc-create-input")], Default::default())
        .await
        .unwrap();

    assert_eq!(response, "rpc-create-output");
}

#[tokio::test]
async fn pause() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client.pause("rpc-pause-input").await.unwrap();

    assert_eq!(response.session_id, "rpc-pause-input");
    assert_eq!(response.partition_ids[0], "rpc-pause-output");
}

#[tokio::test]
async fn resume() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client.resume("rpc-resume-input").await.unwrap();

    assert_eq!(response.session_id, "rpc-resume-input");
    assert_eq!(response.partition_ids[0], "rpc-resume-output");
}

#[tokio::test]
async fn close() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client.close("rpc-close-input").await.unwrap();

    assert_eq!(response.session_id, "rpc-close-input");
    assert_eq!(response.partition_ids[0], "rpc-close-output");
}

#[tokio::test]
async fn purge() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client.purge("rpc-purge-input").await.unwrap();

    assert_eq!(response.session_id, "rpc-purge-input");
    assert_eq!(response.partition_ids[0], "rpc-purge-output");
}

#[tokio::test]
async fn delete() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client.delete("rpc-delete-input").await.unwrap();

    assert_eq!(response.session_id, "rpc-delete-input");
    assert_eq!(response.partition_ids[0], "rpc-delete-output");
}

#[tokio::test]
async fn stop_submission() {
    let mut client =
        armonik::Client::with_channel(Service::default().sessions_server()).into_sessions();

    let response = client
        .stop_submission("rpc-stop-input", true, true)
        .await
        .unwrap();

    assert_eq!(response.session_id, "rpc-stop-input");
    assert_eq!(response.partition_ids[0], "rpc-stop-output");
}
