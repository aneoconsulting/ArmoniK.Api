//! A [`Router`](armonik::server::Router) behind a real socket.
//!
//! Nothing else in the repository puts one there: every in-process round trip drives a router as a
//! client channel, which reaches `Service::call` directly, and `server_mounting.rs`'s two
//! `add_service` assertions are compile-only. So this is the only cover for everything hyper does
//! between the wire and `Service::call` -- a client-supplied path reaching the synchronous body,
//! framing, flow control, and a half-close.
//!
//! Two of those are invisible in process by construction: a path is never routed through hyper
//! there, and dropping a bidirectional call's response stream drops the handler's future with it,
//! where over a socket the request half has to be closed for the handler to end. All four call
//! shapes run here for that reason: unary, server-streamed, client-streamed, and bidirectional
//! with its cancellation.

#![cfg(all(feature = "client", feature = "server"))]

use std::sync::Arc;

use armonik::reexports::tonic;
use armonik::reexports::tonic::codegen::http;
use armonik::server::{
    RequestContext, ResultsService, ResultsServiceExt, VersionsService, VersionsServiceExt,
};
use armonik::{results, versions};
use futures::StreamExt as _;

#[derive(Clone, Default)]
struct Versions;

impl VersionsService for Versions {
    async fn list(
        self: Arc<Self>,
        _request: versions::list::Request,
        _context: RequestContext,
    ) -> Result<versions::list::Response, tonic::Status> {
        Ok(versions::list::Response {
            core: String::from("core"),
            api: String::from("api"),
        })
    }
}

/// Serve the versions router, which is all most of these tests need.
async fn serve() -> (tonic::transport::Channel, tokio::task::JoinHandle<()>) {
    serve_router(tonic::transport::Server::builder().add_service(Versions.versions_server())).await
}

/// Serve the results router, and hand back the token its `watch` handler holds while it reads.
async fn serve_watcher() -> (
    tonic::transport::Channel,
    tokio::task::JoinHandle<()>,
    tokio_util::sync::CancellationToken,
) {
    let watcher = Watcher::default();
    let serving = watcher.serving.clone();
    let (channel, server) =
        serve_router(tonic::transport::Server::builder().add_service(watcher.results_server()))
            .await;
    (channel, server, serving)
}

/// Serve a router on an ephemeral port and hand back a channel to it, plus the task's handle so
/// the caller drops the server with the test.
async fn serve_router(
    router: tonic::transport::server::Router,
) -> (tonic::transport::Channel, tokio::task::JoinHandle<()>) {
    // The listener is bound here and handed to the server as an incoming stream, rather than
    // letting `Server::serve` bind it: the test needs the port before the server starts, and
    // binding twice would race the connect below against the server's own bind.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind an ephemeral port");
    let address = listener.local_addr().expect("the bound address");

    let serving = tokio::spawn(async move {
        let incoming = async_stream::stream! {
            loop {
                yield listener.accept().await.map(|(stream, _)| stream);
            }
        };
        let _ = router.serve_with_incoming(incoming).await;
    });

    let channel = tonic::transport::Endpoint::from_shared(format!("http://{address}"))
        .expect("a valid endpoint")
        .connect()
        .await
        .expect("connect to the server");

    (channel, serving)
}

/// One raw request, past the client machinery: the path is whatever we say it is.
///
/// Under the service prefix, always: `Server::add_service` puts the router behind tonic's own
/// routing, which matches on that prefix, so a path without it is refused there and never reaches
/// the router's unmatched-path arm at all.
async fn raw(channel: &mut tonic::transport::Channel, path: &str) -> tonic::Status {
    use tonic::codegen::Service as _;

    // A tonic `Channel` is a tower buffer, which panics on `call` without a reservation.
    std::future::poll_fn(|cx| channel.poll_ready(cx))
        .await
        .expect("the channel is ready");

    let request = http::Request::builder()
        .method(http::Method::POST)
        .uri(format!("http://localhost{path}"))
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(tonic::body::Body::empty())
        .expect("build the request");

    let response = channel.call(request).await.expect("the server answers");
    assert_eq!(response.status(), http::StatusCode::OK, "trailers-only 200");
    tonic::Status::from_header_map(response.headers()).expect("a grpc status in the headers")
}

#[tokio::test]
async fn a_valid_rpc_answers_over_a_real_connection() {
    let (channel, serving) = serve().await;

    let response = armonik::Client::with_channel(channel)
        .into_versions()
        .list()
        .await
        .expect("the call succeeds");
    assert_eq!(response.core, "core");
    assert_eq!(response.api, "api");

    serving.abort();
}

/// A method of the right service that the router does not serve. Refused in the router's own arm,
/// which is new surface: tonic's codegen answers with no message at all.
#[tokio::test]
async fn an_unknown_method_is_unimplemented_and_names_the_path() {
    let (mut channel, serving) = serve().await;

    let status = raw(
        &mut channel,
        "/armonik.api.grpc.v1.versions.Versions/NoSuchMethod",
    )
    .await;
    assert_eq!(status.code(), tonic::Code::Unimplemented);
    assert!(
        status.message().contains("NoSuchMethod"),
        "the status names the path: {}",
        status.message()
    );

    serving.abort();
}

/// A panic in `Service::call` takes the hyper connection task with it, so this asserts the
/// connection still serves afterwards. `http` accepts non-ASCII UTF-8 in a path, and the
/// truncation is in bytes.
#[tokio::test]
async fn a_long_non_ascii_path_does_not_take_the_connection_down() {
    let (mut channel, serving) = serve().await;

    let status = raw(
        &mut channel,
        &format!("/armonik.api.grpc.v1.versions.Versions/{}", "€".repeat(60)),
    )
    .await;
    assert_eq!(status.code(), tonic::Code::Unimplemented);
    assert!(
        status.message().contains("€€€"),
        "the status repeats the path back: {}",
        status.message()
    );

    let response = armonik::Client::with_channel(channel)
        .into_versions()
        .list()
        .await
        .expect("the connection still serves");
    assert_eq!(response.core, "core");

    serving.abort();
}

/// A request compressed with an encoding the server never enabled. The knobs exist
/// (`Router::accept_compressed`), this router calls none of them, and tonic's contract is that the
/// call fails rather than the body being read as-is.
#[tokio::test]
async fn an_unnegotiated_encoding_is_refused() {
    let (channel, serving) = serve().await;

    let mut grpc =
        tonic::client::Grpc::new(channel).send_compressed(tonic::codec::CompressionEncoding::Gzip);
    grpc.ready().await.expect("the channel is ready");

    let outcome: Result<tonic::Response<versions::list::Response>, tonic::Status> = grpc
        .unary(
            tonic::Request::new(versions::list::Request {}),
            http::uri::PathAndQuery::from_static(
                "/armonik.api.grpc.v1.versions.Versions/ListVersions",
            ),
            tonic_prost::ProstCodec::default(),
        )
        .await;

    let status = outcome.expect_err("an encoding the server did not accept is refused");
    assert_eq!(status.code(), tonic::Code::Unimplemented);

    serving.abort();
}

/// A defaulting handler per named RPC. The handler name is its request module's name, which is what
/// lets one identifier stand for the whole method; `ResultsService` has no default bodies, so a fake
/// has to spell every RPC whether or not the test drives it.
macro_rules! unexercised {
    ($($name:ident),* $(,)?) => {
        $(
            async fn $name(
                self: Arc<Self>,
                _request: results::$name::Request,
                _context: RequestContext,
            ) -> Result<results::$name::Response, tonic::Status> {
                Ok(Default::default())
            }
        )*
    };
}

/// A `Results` fake whose `WatchResults` handler holds a cancellation guard for as long as it is
/// reading requests, and answers one response per request. Every other method answers with a
/// default, so the service can be mounted whole.
#[derive(Clone, Default)]
struct Watcher {
    serving: tokio_util::sync::CancellationToken,
}

impl ResultsService for Watcher {
    async fn watch(
        self: Arc<Self>,
        request: impl futures::Stream<Item = Result<results::watch::Request, tonic::Status>>
            + Send
            + 'static,
        _context: RequestContext,
    ) -> Result<
        impl futures::Stream<Item = Result<results::watch::Response, tonic::Status>> + Send,
        tonic::Status,
    > {
        let guard = self.serving.clone().drop_guard();
        Ok(async_stream::stream! {
            // Dropped when this stream ends or is dropped, which is either way the handler being
            // over. A request half that never closes is what keeps it alive.
            let _guard = guard;
            let mut request = std::pin::pin!(request);

            while let Some(item) = request.next().await {
                yield item.map(|request| results::watch::Response {
                    status: armonik::ResultStatus::Created,
                    result_ids: request.result_ids,
                });
            }
        })
    }

    /// One chunk per name, so the order of a server-streamed body is observable.
    async fn download(
        self: Arc<Self>,
        request: results::download::Request,
        _context: RequestContext,
    ) -> Result<
        impl futures::Stream<Item = Result<results::download::Response, tonic::Status>> + Send,
        tonic::Status,
    > {
        Ok(futures::stream::iter(["first-", "second-"].map(
            move |prefix| {
                Ok(results::download::Response {
                    data_chunk: format!("{prefix}{}", request.result_id).into(),
                })
            },
        )))
    }

    /// Sums the chunk sizes off the request stream, so a client-streamed body that arrives short
    /// is visible in the answer.
    async fn upload(
        self: Arc<Self>,
        request: impl futures::Stream<Item = Result<results::upload::Request, tonic::Status>>
            + Send
            + 'static,
        _context: RequestContext,
    ) -> Result<results::upload::Response, tonic::Status> {
        let mut request = std::pin::pin!(request);
        let mut size = 0i64;
        let mut result_id = String::new();
        while let Some(message) = request.next().await {
            match message? {
                results::upload::Request::Identifier { result_id: id, .. } => result_id = id,
                results::upload::Request::DataChunk(chunk) => size += chunk.len() as i64,
                results::upload::Request::Invalid => {
                    return Err(tonic::Status::invalid_argument("no member set"))
                }
            }
        }
        Ok(results::upload::Response {
            result: results::Raw {
                result_id,
                size,
                ..Default::default()
            },
        })
    }

    unexercised!(
        list,
        get,
        get_owner_task_id,
        create_metadata,
        create,
        import,
        delete_data,
        get_service_configuration
    );
}

/// Dropping a bidirectional call's response stream ends the call.
///
/// The request half was moved into `call`, so the response stream is the caller's only handle on
/// the RPC: if dropping it does not close the request half, the server handler stays parked on
/// `request.next()` for as long as the caller holds the sender, with its h2 stream and its task.
/// No in-process test can see this -- there, dropping the returned stream drops the handler's
/// future directly, whatever the request half is doing.
#[tokio::test]
async fn dropping_a_bidi_response_stream_ends_the_handler() {
    let (channel, server, serving) = serve_watcher().await;

    // Unbounded and kept alive past the drop below: the request half stays open unless the client
    // closes it, which is the whole point.
    let (requests, stream) = futures::channel::mpsc::unbounded();
    let mut client = armonik::Client::with_channel(channel).into_results();
    let mut responses = client.watch(stream).await.expect("the call starts");

    requests
        .unbounded_send(results::watch::Request {
            result_ids: vec![String::from("one")],
            ..Default::default()
        })
        .expect("the request half is open");
    let response = responses
        .next()
        .await
        .expect("a response")
        .expect("not a status");
    assert_eq!(response.result_ids, ["one"]);

    std::mem::drop(responses);

    tokio::time::timeout(std::time::Duration::from_secs(10), serving.cancelled())
        .await
        .expect("the server handler ends when the response stream is dropped");

    server.abort();
}

/// A server-streamed body over a real connection: several frames, in order, then a clean end.
///
/// In process this reaches `Service::call` directly and the stream is a `futures::Stream` all the
/// way; over a socket every item is a length-prefixed frame that hyper writes, h2 windows and the
/// client's decoder reassembles.
#[tokio::test]
async fn a_server_streamed_body_arrives_in_order() {
    let (channel, server, _serving) = serve_watcher().await;

    let mut client = armonik::Client::with_channel(channel).into_results();
    let chunks: Vec<String> = client
        .call(results::download::Request {
            session_id: String::from("session"),
            result_id: String::from("result"),
        })
        .await
        .expect("the call starts")
        .map(|item| {
            let response = item.expect("a chunk");
            String::from_utf8(response.data_chunk.to_vec()).expect("utf-8")
        })
        .collect()
        .await;

    assert_eq!(chunks, ["first-result", "second-result"]);

    server.abort();
}

/// A client-streamed body over a real connection: the handler reads every frame the caller sent
/// before answering, which is what its total says.
#[tokio::test]
async fn a_client_streamed_body_arrives_whole() {
    let (channel, server, _serving) = serve_watcher().await;

    let mut client = armonik::Client::with_channel(channel).into_results();
    let response = client
        .upload(
            "session",
            "result",
            futures::stream::iter((0..8).map(|index| vec![0xa5u8; 1024 * (index + 1)])),
        )
        .await
        .expect("the call succeeds");

    assert_eq!(response.result_id, "result");
    assert_eq!(response.size, (1..=8).map(|n| 1024 * n).sum::<i64>());

    server.abort();
}
