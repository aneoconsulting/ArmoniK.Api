//! How a [`Router`] can be mounted, and the per-call knobs it carries.
//!
//! Nothing else in the suite exercises either: all the in-process round trips drive a router as a
//! client `GrpcService` channel, which reaches `Service::call` without ever going through
//! `Server::add_service` or a body type other than `tonic::body::Body`. That is why the router's
//! `Service` impl could be narrowed to one body type without anything failing: `add_service` asks
//! only for `Service<Request<tonic::body::Body>>`, so it accepts either shape.

#![cfg(all(feature = "client", feature = "server"))]

use std::sync::Arc;

use armonik::reexports::prost::bytes::Bytes;
use armonik::rpc::services::Versions as VersionsMarker;
use armonik::server::{RequestContext, VersionsService, VersionsServiceExt};
use armonik::versions;
use http_body_util::Full;
use tonic::codegen::http;

/// The smallest service that can answer something.
#[derive(Clone, Default)]
struct Versions;

impl VersionsService for Versions {
    fn list(
        self: Arc<Self>,
        _request: versions::list::Request,
        _context: RequestContext,
    ) -> impl std::future::Future<Output = Result<versions::list::Response, tonic::Status>> + Send
    {
        std::future::ready(Ok(versions::list::Response {
            core: String::from("core"),
            api: String::from("api"),
        }))
    }
}

/// Compile-only: the router routes any body a tonic-generated server would, not just
/// `tonic::body::Body`.
///
/// `Full<Bytes>` stands in for the ones that matter in practice: `hyper::body::Incoming` when the
/// router is wired straight onto hyper, and whatever a `tower` layer or an `axum::Router` hands
/// down when it is nested with `route_service`. This assertion failed with E0277 before the
/// `Service` impl was parameterized.
#[test]
fn routes_any_request_body() {
    fn assert_routes<B>()
    where
        armonik::server::Router<VersionsMarker, Versions>:
            tonic::codegen::Service<http::Request<B>>,
    {
    }

    assert_routes::<Full<Bytes>>();
    assert_routes::<tonic::body::Body>();
}

/// Compile-only: a router still satisfies what `tonic::transport::Server::add_service` asks of a
/// service, which is the supported way to serve one.
#[test]
fn mounts_on_a_tonic_server() {
    fn assert_mountable<S>(_service: S)
    where
        S: tonic::codegen::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<tonic::body::Body>,
                Error = std::convert::Infallible,
            > + tonic::server::NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
    {
    }

    assert_mountable(Versions.versions_server());
}

/// The four size and compression knobs are `Router` methods with no other coverage: each returns
/// the router so they chain, and a call still succeeds with all four set.
#[tokio::test]
async fn the_per_call_knobs_are_applied() {
    use armonik::reexports::tonic::codegen::CompressionEncoding;

    let channel = Versions
        .versions_server()
        .accept_compressed(CompressionEncoding::Gzip)
        .send_compressed(CompressionEncoding::Gzip)
        .max_decoding_message_size(64 * 1024)
        .max_encoding_message_size(64 * 1024);

    let mut client = armonik::Client::with_channel(channel).into_versions();
    let response = client
        .call(versions::list::Request {})
        .await
        .expect("the call succeeds with every knob set");
    assert_eq!(response.core, "core");
}

/// A path the service does not serve answers UNIMPLEMENTED and names the path, rather than routing
/// it somewhere else.
#[tokio::test]
async fn an_unrouted_path_is_unimplemented() {
    use tonic::codegen::Service as _;

    let mut router = Versions.versions_server();
    let request = http::Request::builder()
        .uri("/armonik.api.grpc.v1.versions.Versions/NoSuchMethod")
        .body(tonic::body::Body::empty())
        .expect("build the request");

    let response = router.call(request).await.expect("infallible");
    let status = tonic::Status::from_header_map(response.headers()).expect("a grpc status");
    assert_eq!(status.code(), tonic::Code::Unimplemented);
    assert!(
        status.message().contains("NoSuchMethod"),
        "the status names the path: {}",
        status.message()
    );
}

/// The unrouted path is repeated back to the client, but bounded: it is client-supplied, and the
/// `grpc-message` header percent-encodes it.
#[tokio::test]
async fn a_long_unrouted_path_is_truncated() {
    use tonic::codegen::Service as _;

    let long = "x".repeat(4096);
    let mut router = Versions.versions_server();
    let request = http::Request::builder()
        .uri(format!("/{long}"))
        .body(tonic::body::Body::empty())
        .expect("build the request");

    let response = router.call(request).await.expect("infallible");
    let status = tonic::Status::from_header_map(response.headers()).expect("a grpc status");
    assert!(
        status.message().len() < 200,
        "the message is bounded, got {} bytes",
        status.message().len()
    );
    assert!(status.message().starts_with("/xxx"));
}
