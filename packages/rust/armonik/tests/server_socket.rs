//! A [`Router`](armonik::server::Router) behind a real socket.
//!
//! Nothing else in the repository puts one there: every in-process round trip drives a router as a
//! client channel, which reaches `Service::call` directly, and `server_mounting.rs`'s two
//! `add_service` assertions are compile-only. So the paths that only a real connection exercises,
//! everything hyper does between the wire and `Service::call`, had never been executed.
//!
//! That gap is not hypothetical: the unrouted-path truncation panicked on any non-ASCII path, in
//! the synchronous body of `Service::call`, and its dedicated test only ever fed it ASCII.

#![cfg(all(feature = "client", feature = "server"))]

use std::sync::Arc;

use armonik::reexports::tonic;
use armonik::reexports::tonic::codegen::http;
use armonik::server::{RequestContext, VersionsService, VersionsServiceExt};
use armonik::versions;

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

/// Serve a router on an ephemeral port and hand back a channel to it, plus the task's handle so
/// the caller drops the server with the test.
async fn serve() -> (tonic::transport::Channel, tokio::task::JoinHandle<()>) {
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
        let _ = tonic::transport::Server::builder()
            .add_service(Versions.versions_server())
            .serve_with_incoming(incoming)
            .await;
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

/// The case that used to abort the connection: `http` accepts non-ASCII UTF-8 in a path, and the
/// truncation sliced it at byte 128 whatever sat there. Over a real socket the panic took the
/// hyper connection task with it, so this asserts the connection is still usable afterwards.
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
