use armonik::reexports::tokio_stream::StreamExt;
use armonik::results;
use armonik::server::{RequestContext, ResultsServiceExt};

#[macro_use]
mod common;

/// The stream `UploadResultData` is driven with, in both halves of the pair. The
/// convenience method prepends the identifier itself, so the `call`
/// side has to spell it out to send the same thing.
fn upload_request() -> impl futures::Stream<Item = results::upload::Request> {
    futures::stream::iter([
        results::upload::Request::Identifier {
            session_id: String::from("rpc-upload-input"),
            result_id: String::from("result-id"),
        },
        results::upload::Request::DataChunk(bytes::Bytes::from_static(b"first chunk")),
        results::upload::Request::DataChunk(bytes::Bytes::from_static(b"second chunk")),
    ])
}

/// The stream `WatchResults` is driven with, in both halves of the pair. Bidirectional, so the
/// same stream goes in through `call` and through the client method.
fn watch_request() -> impl futures::Stream<Item = results::watch::Request> {
    futures::stream::iter([results::watch::Request {
        fetch_statuses: vec![armonik::ResultStatus::Created],
        watch_statuses: vec![armonik::ResultStatus::Completed],
        result_ids: vec![String::from("rpc-watch-input")],
    }])
}

rpc_tests! {
    client: into_results;
    server: ResultsService, results_server;
    mock: "Results";
    fake { early: bool, dropped: tokio_util::sync::CancellationToken }

    rpc unary list {
        request: results::list::Request {
            filters: results::filter::Or::default(),
            sort: results::Sort::default(),
            page: 3,
            page_size: 12,
        },
        respond: |request: results::list::Request| results::list::Response {
            results: vec![results::Raw {
                name: String::from("rpc-list-output"),
                ..Default::default()
            }],
            page: request.page,
            page_size: request.page_size,
            total: 1337,
        },
        convenience: list(
            results::filter::Or::default(),
            results::Sort::default(),
            3,
            12,
        ),
        check: |response| {
            assert_eq!(response.page, 3);
            assert_eq!(response.page_size, 12);
            assert_eq!(response.total, 1337);
            assert_eq!(response.results[0].name, "rpc-list-output");
        },
    }

    // The handler holds a drop guard, for `get_wait` below.
    rpc unary get {
        request: results::get::Request {
            id: String::from("rpc-get-input"),
        },
        convenience: get("rpc-get-input"),
        project: |response| response.result,
        check: |result| {
            assert_eq!(result.result_id, "rpc-get-input");
            assert_eq!(result.name, "rpc-get-output");
        },
    }

    rpc unary get_owner_task_id {
        request: results::get_owner_task_id::Request {
            session_id: String::from("session-id"),
            result_ids: vec![String::from("rpc-get-owner-task-id-input")],
        },
        respond: |request: results::get_owner_task_id::Request| {
            results::get_owner_task_id::Response {
                session_id: request.session_id,
                result_task: request
                    .result_ids
                    .into_iter()
                    .map(|result_id| (result_id, String::from("rpc-get-owner-task-id-output")))
                    .collect(),
            }
        },
        convenience: get_owner_task_id("session-id", ["rpc-get-owner-task-id-input"]),
        project: |response| response.result_task,
        check: |result_task| {
            assert_eq!(
                result_task["rpc-get-owner-task-id-input"],
                "rpc-get-owner-task-id-output"
            );
        },
    }

    rpc unary create_metadata {
        request: results::create_metadata::Request {
            session_id: String::from("session-id"),
            results: vec![results::create_metadata::RequestItem {
                name: String::from("rpc-create-metadata-input"),
                manual_deletion: false,
            }],
        },
        respond: |request: results::create_metadata::Request| {
            results::create_metadata::Response {
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
            }
        },
        convenience: create_metadata(
            "session-id",
            [results::create_metadata::RequestItem {
                name: String::from("rpc-create-metadata-input"),
                manual_deletion: false,
            }],
        ),
        project: |response| response.results,
        check: |results| {
            assert_eq!(results[0].session_id, "session-id");
            assert_eq!(results[0].name, "rpc-create-metadata-input");
            assert_eq!(results[0].result_id, "rpc-create-metadata-output");
        },
    }

    rpc unary create {
        request: results::create::Request {
            session_id: String::from("session-id"),
            results: vec![results::create::RequestItem {
                name: String::from("rpc-create-input"),
                data: bytes::Bytes::from_static(b"payload"),
                manual_deletion: false,
            }],
        },
        respond: |request: results::create::Request| results::create::Response {
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
        },
        convenience: create(
            "session-id",
            [results::create::RequestItem {
                name: String::from("rpc-create-input"),
                data: bytes::Bytes::from_static(b"payload"),
                manual_deletion: false,
            }],
        ),
        project: |response| response.results,
        check: |results| {
            assert_eq!(results[0].session_id, "session-id");
            assert_eq!(results[0].name, "rpc-create-input");
            assert_eq!(results[0].result_id, "rpc-create-output");
        },
    }

    rpc unary import {
        request: results::import::Request {
            session_id: String::from("session-id"),
            results: [(
                String::from("rpc-import-input"),
                bytes::Bytes::from_static(b"opaque-id"),
            )]
            .into_iter()
            .collect(),
        },
        respond: |request: results::import::Request| results::import::Response {
            results: request
                .results
                .into_iter()
                .map(|(name, opaque_id)| results::Raw {
                    session_id: request.session_id.clone(),
                    result_id: String::from("rpc-import-output"),
                    name,
                    opaque_id,
                    ..Default::default()
                })
                .collect(),
        },
        convenience: import("session-id", [("rpc-import-input", b"opaque-id".as_slice())]),
        project: |response| response.results,
        check: |results: Vec<results::Raw>| {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].name, "rpc-import-input");
            assert_eq!(results[0].result_id, "rpc-import-output");
            assert_eq!(results[0].opaque_id, "opaque-id".as_bytes());
        },
    }

    rpc unary delete_data {
        request: results::delete_data::Request {
            session_id: String::from("session-id"),
            result_ids: vec![String::from("rpc-delete-data-input")],
        },
        respond: |mut request: results::delete_data::Request| results::delete_data::Response {
            session_id: request.session_id,
            result_ids: vec![
                std::mem::take(&mut request.result_ids[0]),
                String::from("rpc-delete-data-output"),
            ],
        },
        convenience: delete_data("session-id", ["rpc-delete-data-input"]),
        project: |response| response.result_ids,
        check: |result_ids| {
            assert_eq!(result_ids[0], "rpc-delete-data-input");
            assert_eq!(result_ids[1], "rpc-delete-data-output");
        },
    }

    rpc unary get_service_configuration {
        request: results::get_service_configuration::Request {},
        respond: |_request| results::get_service_configuration::Response {
            data_chunk_max_size: 1337,
        },
        convenience: get_service_configuration(),
        check: |response| {
            assert_eq!(response.data_chunk_max_size, 1337);
        },
    }

    rpc server_stream download {
        request: results::download::Request {
            session_id: String::from("session-id"),
            result_id: String::from("rpc-download-input"),
        },
        convenience: download("session-id", "rpc-download-input"),
        project: |response| response.data_chunk,
        check: |mut stream| async move {
            let chunk = stream.next().await.unwrap().unwrap();
            assert_eq!(chunk, b"rpc-download-input"[..]);

            let chunk = stream.next().await.unwrap().unwrap();
            assert_eq!(chunk, b"rpc-download-output-0"[..]);

            let chunk = stream.next().await.unwrap().unwrap();
            assert_eq!(chunk, b"rpc-download-output-1"[..]);

            assert!(stream.next().await.is_none());
        },
    }

    rpc client_stream upload {
        request: upload_request(),
        convenience: upload(
            "rpc-upload-input",
            "result-id",
            futures::stream::iter([
                bytes::Bytes::from_static(b"first chunk"),
                bytes::Bytes::from_static(b"second chunk"),
            ]),
        ),
        project: |response| response.result,
        check: |result| {
            assert_eq!(result.session_id, "rpc-upload-input");
            assert_eq!(result.result_id, "rpc-upload-output");
            assert_eq!(result.size, 23);
        },
    }

    rpc bidi_stream watch {
        request: watch_request(),
        convenience: watch(watch_request()),
        // No `project:`: a bidirectional method hands back the response stream whole, so both
        // halves of the pair see the same thing and the check reads the response itself.
        check: |mut stream| async move {
            let response = stream.next().await.unwrap().unwrap();
            assert_eq!(response.status, armonik::ResultStatus::Created);
            assert_eq!(response.result_ids, ["rpc-watch-input"]);

            let response = stream.next().await.unwrap().unwrap();
            assert_eq!(response.status, armonik::ResultStatus::Completed);
            assert_eq!(response.result_ids, ["rpc-watch-output"]);

            assert!(stream.next().await.is_none());
        },
    }

    manual {
        // One response per request message, plus one of its own, so the test can tell that both
        // directions stayed open rather than the request being drained first.
        async fn watch(
            self: std::sync::Arc<Self>,
            request: impl armonik::reexports::tokio_stream::Stream<
                    Item = Result<results::watch::Request, tonic::Status>,
                > + Send
                + 'static,
            _context: RequestContext,
        ) -> Result<
            impl armonik::reexports::tokio_stream::Stream<
                    Item = Result<results::watch::Response, tonic::Status>,
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
                let mut request = std::pin::pin!(request);

                while let Some(item) = request.next().await {
                    if let Some(duration) = self.wait {
                        tokio::time::sleep(duration).await;
                    }
                    if let Some(failure) = self.failure.clone() {
                        Err(failure)?
                    }
                    yield results::watch::Response {
                        status: armonik::ResultStatus::Created,
                        result_ids: item?.result_ids,
                    };
                }

                yield results::watch::Response {
                    status: armonik::ResultStatus::Completed,
                    result_ids: vec![String::from("rpc-watch-output")],
                };
            })
        }

        // Drops its guard only once it has answered, so `get_wait` can tell a
        // cancelled call from a slow one.
        async fn get(
            self: std::sync::Arc<Self>,
            request: results::get::Request,
            _context: RequestContext,
        ) -> Result<results::get::Response, tonic::Status> {
            let drop_guard = self.dropped.clone().drop_guard();
            common::stub(self.wait, self.failure.clone(), || {
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

        // Honours the knobs per chunk, so a failure can land before the stream is
        // returned (`early`) or partway through it.
        async fn download(
            self: std::sync::Arc<Self>,
            request: results::download::Request,
            _context: RequestContext,
        ) -> Result<
            impl armonik::reexports::tokio_stream::Stream<
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

                for chunk in [
                    request.result_id.as_bytes(),
                    b"rpc-download-output-0",
                    b"rpc-download-output-1",
                ] {
                    if let Some(duration) = self.wait {
                        tokio::time::sleep(duration).await;
                    }
                    if let Some(failure) = self.failure.clone() {
                        Err(failure)?
                    }
                    yield results::download::Response {
                        data_chunk: bytes::Bytes::copy_from_slice(chunk),
                    };
                }
            })
        }

        // Sums the chunk sizes off the stream, and honours the knobs per message.
        async fn upload(
            self: std::sync::Arc<Self>,
            request: impl armonik::reexports::tokio_stream::Stream<
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
                    Some(Ok(results::upload::Request::Invalid)) => {
                        return Err(tonic::Status::invalid_argument("no member set"))
                    }
                    Some(Err(err)) => return Err(err),
                    None => break,
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
}

/// `WatchResults` is declared `unexposed`, so it has no route: the router answers UNIMPLEMENTED,
/// naming the path it refused.
#[tokio::test]
async fn an_unrouted_path_is_named_in_the_status() {
    use armonik::reexports::http;
    use armonik::reexports::tonic;
    use armonik::reexports::tonic::codegen::Service as _;

    // A name the proto does not declare, which is the other half of what the router answers
    // UNIMPLEMENTED for (the first being a method of another service).
    let mut router = Service::default().results_server();
    let request = http::Request::builder()
        .uri("/armonik.api.grpc.v1.results.Results/NoSuchMethod")
        .body(tonic::body::Body::default())
        .expect("request");

    let response = router.call(request).await.expect("infallible");
    let headers = response.headers();

    assert_eq!(headers["grpc-status"], "12");
    let message = headers["grpc-message"].to_str().expect("ascii");
    assert!(
        message.contains("armonik.api.grpc.v1.results.Results/NoSuchMethod"),
        "unexpected message: {message}"
    );
}

/// A client whose fake sleeps for `wait` before answering, and cancels `token` when its handler is
/// dropped. The clock is paused in these tests, so the timeouts below resolve instantly.
fn slow_client(
    token: &tokio_util::sync::CancellationToken,
    early: bool,
) -> armonik::client::Results<impl armonik::client::Channel> {
    armonik::Client::with_channel(
        Service {
            early,
            wait: Some(tokio::time::Duration::from_millis(10)),
            dropped: token.clone(),
            ..Default::default()
        }
        .results_server(),
    )
    .into_results()
}

/// A client whose fake answers `failure` instead of a response.
fn failing_client(
    message: &str,
    early: bool,
) -> armonik::client::Results<impl armonik::client::Channel> {
    armonik::Client::with_channel(
        Service {
            early,
            failure: Some(tonic::Status::invalid_argument(message)),
            ..Default::default()
        }
        .results_server(),
    )
    .into_results()
}

/// The handler must be dropped, which is what tears the server side down when the client stops
/// waiting.
async fn assert_cancelled(token: tokio_util::sync::CancellationToken) {
    if token
        .run_until_cancelled(tokio::time::sleep(tokio::time::Duration::from_millis(10)))
        .await
        .is_some()
    {
        panic!("Expected a cancellation, but got a timeout")
    }
}

/// The status a failing call must carry, whichever shape the call has.
fn assert_invalid_argument<T: std::fmt::Debug>(
    outcome: Result<T, armonik::client::RequestError>,
    message: &str,
) {
    match outcome {
        Ok(response) => panic!("Expected a failure, but got a response {response:?}"),
        Err(armonik::client::RequestError::Grpc { source, .. }) => {
            assert_eq!(source.code(), tonic::Code::InvalidArgument, "{source:?}");
            assert_eq!(source.message(), message);
        }
        Err(err) => panic!("Got an unexpected type of failure {err:?}"),
    }
}

// Cancellations: dropping the client future must tear the server handler down.

#[tokio::test(start_paused = true)]
async fn get_wait() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut client = slow_client(&token, false);

    if let Ok(response) = tokio::time::timeout(
        tokio::time::Duration::from_micros(10),
        client.get("result-id"),
    )
    .await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    assert_cancelled(token).await;
}

#[tokio::test(start_paused = true)]
async fn download_wait_early() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut client = slow_client(&token, true);

    if tokio::time::timeout(
        tokio::time::Duration::from_micros(10),
        client.download("session-id", "result-id"),
    )
    .await
    .is_ok()
    {
        panic!("Expected a timeout, but got a response stream");
    }

    assert_cancelled(token).await;
}

#[tokio::test(start_paused = true)]
async fn download_wait_late() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut client = slow_client(&token, false);

    let mut stream = client.download("session-id", "result-id").await.unwrap();

    if let Ok(response) =
        tokio::time::timeout(tokio::time::Duration::from_micros(10), stream.next()).await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    std::mem::drop(stream);

    assert_cancelled(token).await;
}

#[tokio::test(start_paused = true)]
async fn watch_wait_early() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut client = slow_client(&token, true);

    if tokio::time::timeout(
        tokio::time::Duration::from_micros(10),
        client.watch(watch_request()),
    )
    .await
    .is_ok()
    {
        panic!("Expected a timeout, but got a response stream");
    }

    assert_cancelled(token).await;
}

#[tokio::test(start_paused = true)]
async fn watch_wait_late() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut client = slow_client(&token, false);

    let mut stream = client.watch(watch_request()).await.unwrap();

    if let Ok(response) =
        tokio::time::timeout(tokio::time::Duration::from_micros(10), stream.next()).await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    std::mem::drop(stream);

    assert_cancelled(token).await;
}

#[tokio::test(start_paused = true)]
async fn upload_wait_early() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut client = slow_client(&token, true);

    let future = client.call(async_stream::stream! {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        yield results::upload::Request::Identifier {
            session_id: String::from("session-id"),
            result_id: String::from("result-id"),
        }
    });

    if let Ok(response) = tokio::time::timeout(tokio::time::Duration::from_micros(10), future).await
    {
        panic!("Expected a timeout, but got a response: {response:?}");
    }

    assert_cancelled(token).await;
}

#[tokio::test(start_paused = true)]
async fn upload_wait_late() {
    let token = tokio_util::sync::CancellationToken::new();
    let mut client = slow_client(&token, false);

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

    assert_cancelled(token).await;
}

// Failures: a `Status` must reach the caller, wherever in the call it is raised.

#[tokio::test]
async fn get_failure() {
    let mut client = failing_client("rpc-get-error", false);

    assert_invalid_argument(client.get("result-id").await, "rpc-get-error");
}

#[tokio::test]
async fn download_failure_early() {
    let mut client = failing_client("rpc-download-early-error", true);

    match client.download("session-id", "result-id").await {
        Ok(_) => panic!("Expected a failure, but got a response stream"),
        outcome => assert_invalid_argument(outcome.map(|_| ()), "rpc-download-early-error"),
    }
}

#[tokio::test]
async fn download_failure_late() {
    let mut client = failing_client("rpc-download-late-error", false);

    let mut stream = client.download("session-id", "result-id").await.unwrap();

    match stream.next().await {
        Some(outcome) => assert_invalid_argument(outcome, "rpc-download-late-error"),
        None => panic!("Expected a failure, but got end of stream"),
    }
}

#[tokio::test]
async fn watch_failure_early() {
    let mut client = failing_client("rpc-watch-early-error", true);

    match client.watch(watch_request()).await {
        Ok(_) => panic!("Expected a failure, but got a response stream"),
        outcome => assert_invalid_argument(outcome.map(|_| ()), "rpc-watch-early-error"),
    }
}

#[tokio::test]
async fn watch_failure_late() {
    let mut client = failing_client("rpc-watch-late-error", false);

    let mut stream = client.watch(watch_request()).await.unwrap();

    match stream.next().await {
        Some(outcome) => assert_invalid_argument(outcome, "rpc-watch-late-error"),
        None => panic!("Expected a failure, but got end of stream"),
    }
}

#[tokio::test]
async fn upload_failure_early() {
    let mut client = failing_client("rpc-upload-early-error", true);

    let future = client.call(async_stream::stream! {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        yield results::upload::Request::Identifier {
            session_id: String::from("session-id"),
            result_id: String::from("result-id"),
        }
    });

    match tokio::time::timeout(tokio::time::Duration::from_millis(10), future).await {
        Ok(outcome) => assert_invalid_argument(outcome, "rpc-upload-early-error"),
        Err(err) => panic!("Expected a failure, but got a timeout {err:?}"),
    }
}

#[tokio::test]
async fn upload_failure_late() {
    let mut client = failing_client("rpc-upload-late-error", false);

    let future = client.call(async_stream::stream! {
        yield results::upload::Request::Identifier {
            session_id: String::from("session-id"),
            result_id: String::from("result-id"),
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        yield results::upload::Request::DataChunk(bytes::Bytes::new());
    });

    match tokio::time::timeout(tokio::time::Duration::from_millis(10), future).await {
        Ok(outcome) => assert_invalid_argument(outcome, "rpc-upload-late-error"),
        Err(err) => panic!("Expected a failure, but got a timeout {err:?}"),
    }
}

#[tokio::test]
async fn upload_failure_end() {
    let mut client = failing_client("rpc-upload-end-error", false);

    assert_invalid_argument(
        client
            .call(futures::stream::iter::<[results::upload::Request; 0]>([]))
            .await,
        "rpc-upload-end-error",
    );
}
