//! Spike validation (DESIGN-rpc.md §8 step 0): the table-driven `Router` and
//! the generic `ServiceClient` against the same scenarios as `tests/results.rs`,
//! in all three old/new combinations:
//!
//! - the battle-tested convenience client against the new `Router`;
//! - the new `ServiceClient` against the old generated server;
//! - the new `ServiceClient` against the new `Router`.
//!
//! The `Service` mock is a copy of the one in `tests/results.rs`; this file is
//! deleted once the cutover makes it redundant.

use std::sync::Arc;

use armonik::{
    client::ServiceClient,
    reexports::tokio_stream::StreamExt,
    results::{self, create, create_metadata},
    rpc::services,
    server::{RequestContext, ResultsServiceExt, Router},
};

mod common;

#[derive(Debug, Clone, Default)]
struct Service {
    failure: Option<tonic::Status>,
    wait: Option<tokio::time::Duration>,
    early: bool,
    dropped: tokio_util::sync::CancellationToken,
}

impl armonik::server::ResultsService for Service {
    async fn list(
        self: Arc<Self>,
        request: results::list::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::list::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(results::list::Response {
                results: vec![results::Raw {
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

    async fn get(
        self: Arc<Self>,
        request: results::get::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::get::Response, tonic::Status> {
        let drop_guard = self.dropped.clone().drop_guard();
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            std::mem::drop(drop_guard);
            Ok(results::get::Response {
                result: results::Raw {
                    result_id: request.id,
                    name: String::from("rpc-get-output"),
                    ..Default::default()
                },
            })
        })
        .await
    }

    async fn get_owner_task_id(
        self: Arc<Self>,
        request: results::get_owner_task_id::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::get_owner_task_id::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(results::get_owner_task_id::Response {
                session_id: request.session_id,
                result_task: request
                    .result_ids
                    .into_iter()
                    .map(|result_id| (result_id, String::from("rpc-get-owner-task-id-output")))
                    .collect(),
            })
        })
        .await
    }

    async fn create_metadata(
        self: Arc<Self>,
        request: results::create_metadata::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::create_metadata::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(results::create_metadata::Response {
                results: request
                    .results
                    .into_iter()
                    .map(|item| results::Raw {
                        session_id: request.session_id.clone(),
                        result_id: String::from("rpc-create-metadata-output"),
                        name: item.name,
                        ..Default::default()
                    })
                    .collect(),
            })
        })
        .await
    }

    async fn create(
        self: Arc<Self>,
        request: results::create::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::create::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(results::create::Response {
                results: request
                    .results
                    .into_iter()
                    .map(|item| results::Raw {
                        session_id: request.session_id.clone(),
                        result_id: String::from("rpc-create-output"),
                        name: item.name,
                        ..Default::default()
                    })
                    .collect(),
            })
        })
        .await
    }

    async fn import(
        self: Arc<Self>,
        request: results::import::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::import::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(results::import::Response {
                results: request
                    .results
                    .into_iter()
                    .map(|(name, opaque_id)| {
                        (
                            name.clone(),
                            results::Raw {
                                session_id: request.session_id.clone(),
                                result_id: String::from("rpc-create-output"),
                                name,
                                opaque_id,
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
            })
        })
        .await
    }

    async fn delete_data(
        self: Arc<Self>,
        mut request: results::delete_data::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::delete_data::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(results::delete_data::Response {
                session_id: request.session_id,
                result_ids: vec![
                    std::mem::take(&mut request.result_ids[0]),
                    String::from("rpc-delete-data-output"),
                ],
            })
        })
        .await
    }

    async fn get_service_configuration(
        self: Arc<Self>,
        _request: results::get_service_configuration::Request,
        _context: RequestContext,
    ) -> std::result::Result<results::get_service_configuration::Response, tonic::Status> {
        common::unary_rpc_impl(self.wait, self.failure.clone(), || {
            Ok(results::get_service_configuration::Response {
                data_chunk_max_size: 1337,
            })
        })
        .await
    }

    async fn download(
        self: Arc<Self>,
        request: results::download::Request,
        _context: RequestContext,
    ) -> Result<
        impl tonic::codegen::tokio_stream::Stream<
                Item = Result<results::download::Response, tonic::Status>,
            > + Send,
        tonic::Status,
    > {
        let drop_guard = self.dropped.clone().drop_guard();

        if self.early {
            if let Some(duration) = self.wait {
                tokio::time::sleep(duration).await;
            }

            if let Some(failure) = self.failure.clone() {
                Err(failure)?
            }
        }

        Ok(async_stream::try_stream! {
            let _drop_guard = drop_guard;

            for chunk in [request.result_id.as_bytes(),  b"rpc-download-output-0", b"rpc-download-output-1"] {
                if let Some(duration) = self.wait {
                    tokio::time::sleep(duration)
                        .await;
                }

                if let Some(failure) = self.failure.clone() {
                    Err(failure)?
                }

                yield results::download::Response{
                    data_chunk: bytes::Bytes::copy_from_slice(chunk)
                };
            }
        })
    }

    async fn upload(
        self: Arc<Self>,
        request: impl tonic::codegen::tokio_stream::Stream<
                Item = Result<results::upload::Request, tonic::Status>,
            > + Send
            + 'static,
        _context: RequestContext,
    ) -> Result<results::upload::Response, tonic::Status> {
        let _drop_guard = self.dropped.clone().drop_guard();
        let mut request = std::pin::pin!(request);
        let mut session = None;
        let mut size = 0usize;

        loop {
            if self.early || session.is_some() {
                if let Some(duration) = self.wait {
                    tokio::time::sleep(duration).await;
                }
                if let Some(failure) = self.failure.clone() {
                    Err(failure)?
                }
            }

            match request.next().await {
                Some(Ok(results::upload::Request::Identifier { session_id, .. })) => {
                    session = Some(session_id);
                }
                Some(Ok(results::upload::Request::DataChunk(chunk))) => {
                    size += chunk.len();
                }
                Some(Err(err)) => {
                    return Err(err);
                }
                None => {
                    break;
                }
            }
        }

        if let Some(failure) = self.failure.clone() {
            Err(failure)?
        }

        Ok(results::upload::Response {
            result: results::Raw {
                session_id: session.unwrap_or_default(),
                result_id: String::from("rpc-upload-output"),
                size: size as i64,
                ..Default::default()
            },
        })
    }
}

/// The old convenience client, connected to the new router.
fn old_client_new_router(
    service: Service,
) -> armonik::client::Results<armonik::Client<Router<services::Results, Service>>> {
    armonik::Client::with_channel(Router::new(service)).into_results()
}

/// The new generic client, connected to the new router.
fn new_client_new_router(service: Service) -> ServiceClient<services::Results, Router<services::Results, Service>> {
    ServiceClient::with_channel(Router::new(service))
}

// Old client against the new router: the full scenario matrix of
// tests/results.rs, validating the router's protocol behavior.

#[tokio::test]
async fn list() {
    let mut client = old_client_new_router(Service::default());

    let response = client
        .list(
            armonik::results::filter::Or::default(),
            armonik::results::Sort::default(),
            3,
            12,
        )
        .await
        .unwrap();

    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 12);
    assert_eq!(response.total, 1337);
    assert_eq!(response.results[0].name, "rpc-list-output");
}

#[tokio::test]
async fn get() {
    let mut client = old_client_new_router(Service::default());

    let response = client.get("rpc-get-input").await.unwrap();

    assert_eq!(response.result_id, "rpc-get-input");
    assert_eq!(response.name, "rpc-get-output");
}

#[tokio::test]
async fn get_owner_task_id() {
    let mut client = old_client_new_router(Service::default());

    let response = client
        .get_owner_task_id("session-id", ["rpc-get-owner-task-id-input"])
        .await
        .unwrap();

    assert_eq!(
        response["rpc-get-owner-task-id-input"],
        "rpc-get-owner-task-id-output"
    );
}

#[tokio::test]
async fn create_metadata() {
    let mut client = old_client_new_router(Service::default());

    let response = client
        .create_metadata(
            "session-id",
            [create_metadata::RequestItem {
                name: String::from("rpc-create-metadata-input"),
                ..Default::default()
            }],
        )
        .await
        .unwrap();

    assert_eq!(response[0].result_id, "rpc-create-metadata-output");
}

#[tokio::test]
async fn create() {
    let mut client = old_client_new_router(Service::default());

    let response = client
        .create(
            "session-id",
            [create::RequestItem {
                name: String::from("rpc-create-input"),
                data: bytes::Bytes::from_static(b"payload"),
                ..Default::default()
            }],
        )
        .await
        .unwrap();

    assert_eq!(response[0].result_id, "rpc-create-output");
}

#[tokio::test]
async fn import() {
    let mut client = old_client_new_router(Service::default());

    let response = client
        .import("session-id", [("rpc-import-input", "opaque-id")])
        .await
        .unwrap();

    assert_eq!(response["rpc-import-input"].result_id, "rpc-create-output");
    assert_eq!(
        response["rpc-import-input"].opaque_id,
        "opaque-id".as_bytes()
    );
}

#[tokio::test]
async fn delete_data() {
    let mut client = old_client_new_router(Service::default());

    let response = client
        .delete_data("session-id", ["rpc-delete-data-input"])
        .await
        .unwrap();

    assert_eq!(response[0], "rpc-delete-data-input");
    assert_eq!(response[1], "rpc-delete-data-output");
}

#[tokio::test]
async fn get_service_configuration() {
    let mut client = old_client_new_router(Service::default());

    let response = client.get_service_configuration().await.unwrap();

    assert_eq!(response.data_chunk_max_size, 1337);
}

#[tokio::test]
async fn download() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = old_client_new_router(Service {
        dropped: cancellation_token.clone(),
        ..Default::default()
    });

    let mut response = client
        .download("session-id", "rpc-download-input")
        .await
        .unwrap();

    let chunk = response.next().await.unwrap().unwrap();
    assert_eq!(chunk, b"rpc-download-input"[..]);

    let chunk = response.next().await.unwrap().unwrap();
    assert_eq!(chunk, b"rpc-download-output-0"[..]);

    let chunk = response.next().await.unwrap().unwrap();
    assert_eq!(chunk, b"rpc-download-output-1"[..]);

    assert!(response.next().await.is_none());
}

#[tokio::test]
async fn upload() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = old_client_new_router(Service {
        dropped: cancellation_token.clone(),
        ..Default::default()
    });

    let response = client
        .upload(
            "rpc-upload-input",
            "result-id",
            async_stream::stream! {
                yield b"first chunk".as_slice();
                yield b"second chunk";
            },
        )
        .await
        .unwrap();

    assert_eq!(response.session_id, "rpc-upload-input");
    assert_eq!(response.result_id, "rpc-upload-output");
    assert_eq!(response.size, 23);
}

/// A request for another service hits the router's UNIMPLEMENTED fallback.
#[tokio::test]
async fn unimplemented_path() {
    let mut client =
        armonik::Client::with_channel(Router::<services::Results, _>::new(Service::default()))
            .into_sessions();

    match client
        .list(
            armonik::sessions::filter::Or::default(),
            armonik::sessions::Sort::default(),
            true,
            0,
            10,
        )
        .await
    {
        Ok(response) => panic!("Expected a failure, but got a response {response:?}"),
        Err(armonik::client::RequestError::Grpc { source, .. }) => {
            assert!(
                matches!(source.code(), tonic::Code::Unimplemented),
                "Expected Unimplemented, got {source:?}"
            );
        }
        Err(err) => panic!("Got an unexpected type of failure {err:?}"),
    }
}

// Cancellations

#[tokio::test]
async fn get_wait() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = old_client_new_router(Service {
        wait: Some(tokio::time::Duration::from_millis(10)),
        dropped: cancellation_token.clone(),
        ..Default::default()
    });

    if let Ok(response) = tokio::time::timeout(
        tokio::time::Duration::from_micros(10),
        client.get("result-id"),
    )
    .await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    if cancellation_token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(10)))
        .await
        .is_some()
    {
        panic!("Expected a cancellation, but got a timeout")
    }
}

#[tokio::test]
async fn download_wait_early() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = old_client_new_router(Service {
        early: true,
        wait: Some(tokio::time::Duration::from_millis(10)),
        dropped: cancellation_token.clone(),
        ..Default::default()
    });

    if tokio::time::timeout(
        tokio::time::Duration::from_micros(10),
        client.download("session_id", "result_id"),
    )
    .await
    .is_ok()
    {
        panic!("Expected a timeout, but got a response stream");
    }

    if cancellation_token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(10)))
        .await
        .is_some()
    {
        panic!("Expected a cancellation, but got a timeout")
    }
}

#[tokio::test]
async fn download_wait_late() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = old_client_new_router(Service {
        wait: Some(tokio::time::Duration::from_millis(10)),
        dropped: cancellation_token.clone(),
        ..Default::default()
    });

    let mut stream = client.download("session_id", "result_id").await.unwrap();

    if let Ok(response) =
        tokio::time::timeout(tokio::time::Duration::from_micros(10), stream.next()).await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    std::mem::drop(stream);

    if cancellation_token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(10)))
        .await
        .is_some()
    {
        panic!("Expected a cancellation, but got a timeout")
    }
}

#[tokio::test]
async fn upload_wait_early() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = old_client_new_router(Service {
        early: true,
        wait: Some(tokio::time::Duration::from_millis(10)),
        dropped: cancellation_token.clone(),
        ..Default::default()
    });

    let future = client.call_streaming(async_stream::stream! {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        yield results::upload::Request::Identifier {
            session_id: String::from("session-id"),
            result_id: String::from("result-id")
        }
    });

    if let Ok(response) = tokio::time::timeout(tokio::time::Duration::from_micros(10), future).await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    if cancellation_token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(10)))
        .await
        .is_some()
    {
        panic!("Expected a cancellation, but got a timeout")
    }
}

#[tokio::test]
async fn upload_wait_late() {
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let mut client = old_client_new_router(Service {
        wait: Some(tokio::time::Duration::from_millis(10)),
        dropped: cancellation_token.clone(),
        ..Default::default()
    });

    let future = client.upload(
        "session-id",
        "result-id",
        async_stream::stream! {
            yield bytes::Bytes::new();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            yield bytes::Bytes::new();
        },
    );

    if let Ok(response) = tokio::time::timeout(tokio::time::Duration::from_micros(10), future).await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    if cancellation_token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(10)))
        .await
        .is_some()
    {
        panic!("Expected a cancellation, but got a timeout")
    }
}

// Failures

#[tokio::test]
async fn get_failure() {
    let mut client = old_client_new_router(Service {
        failure: Some(tonic::Status::invalid_argument("rpc-get-error")),
        ..Default::default()
    });

    match client.get("result-id").await {
        Ok(response) => panic!("Expected a failure, but got a response {response:?}"),
        Err(armonik::client::RequestError::Grpc { source, .. }) => {
            if !matches!(source.code(), tonic::Code::InvalidArgument) {
                panic!("Expected an InvalidArgument error, but got {source:?}");
            }

            assert_eq!(source.message(), "rpc-get-error");
        }
        Err(err) => {
            panic!("Got an unexpected type of failure {err:?}")
        }
    }
}

#[tokio::test]
async fn download_failure_early() {
    let mut client = old_client_new_router(Service {
        early: true,
        failure: Some(tonic::Status::invalid_argument("rpc-download-early-error")),
        ..Default::default()
    });

    match client.download("session-id", "result-id").await {
        Ok(_) => panic!("Expected a failure, but got a response stream"),
        Err(armonik::client::RequestError::Grpc { source, .. }) => {
            if !matches!(source.code(), tonic::Code::InvalidArgument) {
                panic!("Expected an InvalidArgument error, but got {source:?}");
            }

            assert_eq!(source.message(), "rpc-download-early-error");
        }
        Err(err) => {
            panic!("Got an unexpected type of failure {err:?}")
        }
    }
}

#[tokio::test]
async fn download_failure_late() {
    let mut client = old_client_new_router(Service {
        failure: Some(tonic::Status::invalid_argument("rpc-download-late-error")),
        ..Default::default()
    });

    let mut stream = client.download("session-id", "result-id").await.unwrap();

    match stream.next().await {
        Some(Ok(response)) => panic!("Expected a failure, but got a response {response:?}"),
        Some(Err(armonik::client::RequestError::Grpc { source, .. })) => {
            if !matches!(source.code(), tonic::Code::InvalidArgument) {
                panic!("Expected an InvalidArgument error, but got {source:?}");
            }

            assert_eq!(source.message(), "rpc-download-late-error");
        }
        Some(Err(err)) => {
            panic!("Got an unexpected type of failure {err:?}")
        }
        None => {
            panic!("Expected a failure, but got end of stream");
        }
    }
}

#[tokio::test]
async fn upload_failure_early() {
    let mut client = old_client_new_router(Service {
        early: true,
        failure: Some(tonic::Status::invalid_argument("rpc-download-late-error")),
        ..Default::default()
    });

    let future = client.call_streaming(async_stream::stream! {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        yield results::upload::Request::Identifier {
            session_id: String::from("session-id"),
            result_id: String::from("result-id")
        }
    });

    match tokio::time::timeout(tokio::time::Duration::from_millis(10), future).await {
        Ok(Ok(response)) => panic!("Expected a failure, but got a response {response:?}"),
        Ok(Err(armonik::client::RequestError::Grpc { source, .. })) => {
            if !matches!(source.code(), tonic::Code::InvalidArgument) {
                panic!("Expected an InvalidArgument error, but got {source:?}");
            }

            assert_eq!(source.message(), "rpc-download-late-error");
        }
        Ok(Err(err)) => panic!("Got an unexpected type of failure {err:?}"),
        Err(err) => panic!("Expected a failure, but got a timeout {err:?}"),
    }
}

#[tokio::test]
async fn upload_failure_late() {
    let mut client = old_client_new_router(Service {
        failure: Some(tonic::Status::invalid_argument("rpc-download-late-error")),
        ..Default::default()
    });

    let future = client.call_streaming(async_stream::stream! {
        yield results::upload::Request::Identifier {
            session_id: String::from("session-id"),
            result_id: String::from("result-id")
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        yield results::upload::Request::DataChunk(bytes::Bytes::new());
    });

    match tokio::time::timeout(tokio::time::Duration::from_millis(10), future).await {
        Ok(Ok(response)) => panic!("Expected a failure, but got a response {response:?}"),
        Ok(Err(armonik::client::RequestError::Grpc { source, .. })) => {
            if !matches!(source.code(), tonic::Code::InvalidArgument) {
                panic!("Expected an InvalidArgument error, but got {source:?}");
            }

            assert_eq!(source.message(), "rpc-download-late-error");
        }
        Ok(Err(err)) => panic!("Got an unexpected type of failure {err:?}"),
        Err(err) => panic!("Expected a failure, but got a timeout {err:?}"),
    }
}

#[tokio::test]
async fn upload_failure_end() {
    let mut client = old_client_new_router(Service {
        failure: Some(tonic::Status::invalid_argument("rpc-download-late-error")),
        ..Default::default()
    });

    match client
        .call_streaming(futures::stream::iter::<[results::upload::Request; 0]>([]))
        .await
    {
        Ok(response) => panic!("Expected a failure, but got a response {response:?}"),
        Err(armonik::client::RequestError::Grpc { source, .. }) => {
            if !matches!(source.code(), tonic::Code::InvalidArgument) {
                panic!("Expected an InvalidArgument error, but got {source:?}");
            }

            assert_eq!(source.message(), "rpc-download-late-error");
        }
        Err(err) => panic!("Got an unexpected type of failure {err:?}"),
    }
}

// New client against the old generated server: validates the Dispatch
// machinery independently of the router.

#[tokio::test]
async fn new_client_unary_old_server() {
    let mut client: ServiceClient<services::Results, _> =
        ServiceClient::with_channel(Service::default().results_server());

    let response = client
        .call(results::get::Request {
            id: String::from("rpc-get-input"),
        })
        .await
        .unwrap();

    assert_eq!(response.result.result_id, "rpc-get-input");
    assert_eq!(response.result.name, "rpc-get-output");

    // Per-call metadata through `impl IntoRequest<R>`, impossible with the
    // old dispatch.
    let mut request = tonic::Request::new(results::get_service_configuration::Request {});
    request
        .metadata_mut()
        .insert("x-spike", "1".parse().unwrap());
    let response = client
        .call::<results::get_service_configuration::Request>(request)
        .await
        .unwrap();
    assert_eq!(response.data_chunk_max_size, 1337);
}

#[tokio::test]
async fn new_client_download_old_server() {
    let mut client: ServiceClient<services::Results, _> =
        ServiceClient::with_channel(Service::default().results_server());

    let mut stream = client
        .call(results::download::Request {
            session_id: String::from("session-id"),
            result_id: String::from("rpc-download-input"),
        })
        .await
        .unwrap();

    let chunk = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk.data_chunk, b"rpc-download-input"[..]);
    let chunk = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk.data_chunk, b"rpc-download-output-0"[..]);
    let chunk = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk.data_chunk, b"rpc-download-output-1"[..]);
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn new_client_upload_old_server() {
    let mut client: ServiceClient<services::Results, _> =
        ServiceClient::with_channel(Service::default().results_server());

    let response = client
        .call_streaming(futures::stream::iter([
            results::upload::Request::Identifier {
                session_id: String::from("session-id"),
                result_id: String::from("result-id"),
            },
            results::upload::Request::DataChunk(bytes::Bytes::from_static(b"first chunk")),
            results::upload::Request::DataChunk(bytes::Bytes::from_static(b"second chunk")),
        ]))
        .await
        .unwrap();

    assert_eq!(response.result.session_id, "session-id");
    assert_eq!(response.result.result_id, "rpc-upload-output");
    assert_eq!(response.result.size, 23);
}

// New client against the new router: the full new path.

#[tokio::test]
async fn new_client_unary_new_router() {
    let mut client = new_client_new_router(Service::default());

    let response = client
        .call(results::list::Request {
            page: 3,
            page_size: 12,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(response.page, 3);
    assert_eq!(response.page_size, 12);
    assert_eq!(response.total, 1337);
    assert_eq!(response.results[0].name, "rpc-list-output");
}

#[tokio::test]
async fn new_client_download_new_router() {
    let mut client = new_client_new_router(Service::default());

    let mut stream = client
        .call(results::download::Request {
            session_id: String::from("session-id"),
            result_id: String::from("rpc-download-input"),
        })
        .await
        .unwrap();

    let chunk = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk.data_chunk, b"rpc-download-input"[..]);
    let chunk = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk.data_chunk, b"rpc-download-output-0"[..]);
    let chunk = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk.data_chunk, b"rpc-download-output-1"[..]);
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn new_client_upload_new_router() {
    let mut client = new_client_new_router(Service::default());

    let response = client
        .call_streaming(futures::stream::iter([
            results::upload::Request::Identifier {
                session_id: String::from("session-id"),
                result_id: String::from("result-id"),
            },
            results::upload::Request::DataChunk(bytes::Bytes::from_static(b"first chunk")),
            results::upload::Request::DataChunk(bytes::Bytes::from_static(b"second chunk")),
        ]))
        .await
        .unwrap();

    assert_eq!(response.result.session_id, "session-id");
    assert_eq!(response.result.result_id, "rpc-upload-output");
    assert_eq!(response.result.size, 23);
}

// New client against the dotnet mock over a real HTTP/2 connection: run with
// `cargo test --test results_spike -- --ignored` after starting the mock
// (`Grpc__Port=5000 Http__Port=4999 dotnet ArmoniK.Api.Mock.dll`) with
// `GrpcClient__Endpoint=http://localhost:5000` and
// `GrpcClient__AllowUnsafeConnection=true` in the test environment.

async fn mock_client() -> ServiceClient<services::Results, tonic::transport::Channel> {
    let config = armonik::ClientConfig::from_env().unwrap();
    ServiceClient::with_channel(armonik::transport::connect(config).await.unwrap())
}

#[tokio::test]
#[ignore = "needs the dotnet mock server"]
async fn mock_unary() {
    let mut client = mock_client().await;
    client
        .call(results::get::Request {
            id: String::from("result-id"),
        })
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "needs the dotnet mock server"]
async fn mock_download() {
    let mut client = mock_client().await;
    let mut stream = client
        .call(results::download::Request {
            session_id: String::from("session-id"),
            result_id: String::from("result-id"),
        })
        .await
        .unwrap();
    while let Some(chunk) = stream.next().await {
        chunk.unwrap();
    }
}

#[tokio::test]
#[ignore = "needs the dotnet mock server"]
async fn mock_upload() {
    let mut client = mock_client().await;
    client
        .call_streaming(futures::stream::iter([
            results::upload::Request::Identifier {
                session_id: String::from("session-id"),
                result_id: String::from("result-id"),
            },
            results::upload::Request::DataChunk(bytes::Bytes::from_static(b"chunk")),
        ]))
        .await
        .unwrap();
}

#[tokio::test]
async fn new_client_failure_new_router() {
    let mut client = new_client_new_router(Service {
        failure: Some(tonic::Status::invalid_argument("rpc-get-error")),
        ..Default::default()
    });

    match client
        .call(results::get::Request {
            id: String::from("result-id"),
        })
        .await
    {
        Ok(response) => panic!("Expected a failure, but got a response {response:?}"),
        Err(armonik::client::RequestError::Grpc { source, .. }) => {
            assert!(matches!(source.code(), tonic::Code::InvalidArgument));
            assert_eq!(source.message(), "rpc-get-error");
        }
        Err(err) => panic!("Got an unexpected type of failure {err:?}"),
    }
}
