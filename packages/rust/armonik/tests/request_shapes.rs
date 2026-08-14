//! The `tonic::Request` input shapes of [`ServiceClient::call`](armonik::client::ServiceClient).
//!
//! `IntoCall` exists so that one `call` takes four inputs: a request message, a `tonic::Request`
//! around one, a stream of messages, and a `tonic::Request` around a stream. The two that carry a
//! `tonic::Request` are the reason the trait needs its marker parameter at all, and neither had any
//! coverage: every case in the suite passes a bare message or a bare stream.
//!
//! What that leaves unpinned is what the wrapper is *for*. Metadata and `grpc-timeout` ride on the
//! request, and the stream-input path is the one that takes a request apart and puts it back
//! together (`service_client::tagged`), so a refactor to `Request::new(stream)` there would drop
//! the caller's metadata on `UploadResultData` and `CreateLargeTasks` with nothing to notice.

#![cfg(all(feature = "client", feature = "server"))]

use std::sync::Arc;

use armonik::reexports::tonic;
use armonik::server::{RequestContext, ResultsService, ResultsServiceExt};
use armonik::{results, Client};

/// Echoes back what the request carried, so the assertions are about the metadata rather than
/// about the RPC.
#[derive(Clone, Default)]
struct Echo;

/// The metadata key the tests send and the fake reads back.
const KEY: &str = "x-armonik-probe";

/// What the fake saw, encoded into a result name so it can travel back through a real response.
fn seen(context: &RequestContext) -> String {
    let probe = context
        .headers()
        .get(KEY)
        .map(|value| value.to_str().expect("ascii").to_owned())
        .unwrap_or_else(|| String::from("<none>"));
    let timeout = context
        .headers()
        .get("grpc-timeout")
        .map(|value| value.to_str().expect("ascii").to_owned())
        .unwrap_or_else(|| String::from("<none>"));
    format!("{probe}/{timeout}")
}

impl ResultsService for Echo {
    async fn get(
        self: Arc<Self>,
        _request: results::get::Request,
        context: RequestContext,
    ) -> Result<results::get::Response, tonic::Status> {
        Ok(results::get::Response {
            result: results::Raw {
                name: seen(&context),
                ..Default::default()
            },
        })
    }

    async fn upload(
        self: Arc<Self>,
        _request: impl futures::Stream<Item = Result<results::upload::Request, tonic::Status>>
            + Send
            + 'static,
        context: RequestContext,
    ) -> Result<results::upload::Response, tonic::Status> {
        Ok(results::upload::Response {
            result: results::Raw {
                name: seen(&context),
                ..Default::default()
            },
        })
    }

    async fn list(
        self: Arc<Self>,
        _request: results::list::Request,
        _context: RequestContext,
    ) -> Result<results::list::Response, tonic::Status> {
        Err(unimplemented())
    }

    async fn get_owner_task_id(
        self: Arc<Self>,
        _request: results::get_owner_task_id::Request,
        _context: RequestContext,
    ) -> Result<results::get_owner_task_id::Response, tonic::Status> {
        Err(unimplemented())
    }

    async fn create_metadata(
        self: Arc<Self>,
        _request: results::create_metadata::Request,
        _context: RequestContext,
    ) -> Result<results::create_metadata::Response, tonic::Status> {
        Err(unimplemented())
    }

    async fn create(
        self: Arc<Self>,
        _request: results::create::Request,
        _context: RequestContext,
    ) -> Result<results::create::Response, tonic::Status> {
        Err(unimplemented())
    }

    async fn import(
        self: Arc<Self>,
        _request: results::import::Request,
        _context: RequestContext,
    ) -> Result<results::import::Response, tonic::Status> {
        Err(unimplemented())
    }

    async fn delete_data(
        self: Arc<Self>,
        _request: results::delete_data::Request,
        _context: RequestContext,
    ) -> Result<results::delete_data::Response, tonic::Status> {
        Err(unimplemented())
    }

    async fn get_service_configuration(
        self: Arc<Self>,
        _request: results::get_service_configuration::Request,
        _context: RequestContext,
    ) -> Result<results::get_service_configuration::Response, tonic::Status> {
        Err(unimplemented())
    }

    async fn download(
        self: Arc<Self>,
        _request: results::download::Request,
        _context: RequestContext,
    ) -> Result<
        impl futures::Stream<Item = Result<results::download::Response, tonic::Status>> + Send,
        tonic::Status,
    > {
        Err::<futures::stream::Empty<_>, _>(unimplemented())
    }

    async fn watch(
        self: Arc<Self>,
        _request: impl futures::Stream<Item = Result<results::watch::Request, tonic::Status>>
            + Send
            + 'static,
        _context: RequestContext,
    ) -> Result<
        impl futures::Stream<Item = Result<results::watch::Response, tonic::Status>> + Send,
        tonic::Status,
    > {
        Err::<futures::stream::Empty<_>, _>(unimplemented())
    }
}

fn unimplemented() -> tonic::Status {
    tonic::Status::unimplemented("not part of this suite")
}

fn client() -> armonik::client::Results<impl armonik::client::Channel> {
    Client::with_channel(Echo.results_server()).into_results()
}

/// Wrap `request`, attach the probe metadata and a timeout.
fn wrapped<T>(request: T) -> tonic::Request<T> {
    let mut request = tonic::Request::new(request);
    request.metadata_mut().insert(
        KEY,
        tonic::metadata::MetadataValue::from_static("by-request"),
    );
    request.set_timeout(std::time::Duration::from_secs(30));
    request
}

#[tokio::test]
async fn a_wrapped_message_carries_its_metadata_and_timeout() {
    let response = client()
        .call(wrapped(results::get::Request {
            id: String::from("id"),
        }))
        .await
        .expect("the call succeeds");

    assert_eq!(response.result.name, "by-request/30000000u");
}

/// The path that takes the request apart and rebuilds it around the instrumented stream. Both
/// halves have to survive that: the metadata the caller set, and the deadline.
#[tokio::test]
async fn a_wrapped_stream_carries_its_metadata_and_timeout() {
    let response = client()
        .call(wrapped(futures::stream::iter([
            results::upload::Request::Identifier {
                session_id: String::from("session"),
                result_id: String::from("result"),
            },
        ])))
        .await
        .expect("the call succeeds");

    assert_eq!(response.result.name, "by-request/30000000u");
}

/// The bare inputs, for contrast: no metadata, no deadline, same RPC.
#[tokio::test]
async fn a_bare_input_carries_neither() {
    let response = client()
        .call(results::get::Request {
            id: String::from("id"),
        })
        .await
        .expect("the call succeeds");
    assert_eq!(response.result.name, "<none>/<none>");

    let response = client()
        .call(futures::stream::iter([
            results::upload::Request::Identifier {
                session_id: String::from("session"),
                result_id: String::from("result"),
            },
        ]))
        .await
        .expect("the call succeeds");
    assert_eq!(response.result.name, "<none>/<none>");
}
