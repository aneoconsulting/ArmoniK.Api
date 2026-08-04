//! `ak_client_create`/`ak_client_free` driven directly through the C ABI, as a C caller would.

use std::ptr;

use armonik_ffi::buffer::ak_bytes;
use armonik_ffi::client::{ak_client_create, ak_client_free};
use armonik_ffi::config::ak_client_config;
use armonik_ffi::error::{
    AK_ERR_CONNECTION_FAILED, AK_ERR_INVALID_CONFIG, AK_ERR_INVALID_UTF8, AK_ERR_NULL_POINTER,
};

fn bytes_of(text: &str) -> ak_bytes {
    ak_bytes {
        ptr: text.as_ptr(),
        len: text.len(),
    }
}

// A lone continuation byte: never valid UTF-8 on its own. `static`, not `const`, so its address is
// stable for as long as a test holds a pointer into it.
static INVALID_UTF8: [u8; 1] = [0x80];

fn invalid_utf8_bytes() -> ak_bytes {
    ak_bytes {
        ptr: INVALID_UTF8.as_ptr(),
        len: INVALID_UTF8.len(),
    }
}

fn empty_config() -> ak_client_config {
    ak_client_config {
        endpoint: ak_bytes::EMPTY,
        allow_unsafe_connection: 0,
        cert_pem: ak_bytes::EMPTY,
        key_pem: ak_bytes::EMPTY,
        ca_cert_pem: ak_bytes::EMPTY,
        override_target: ak_bytes::EMPTY,
        connect_timeout_ms: -1,
        timeout_ms: -1,
    }
}

/// Answers every request with an empty response: enough to complete the HTTP/2 handshake `connect`
/// performs.
#[derive(Clone)]
struct Nothing;

impl
    armonik_transport::reexports::tonic::codegen::Service<
        armonik_transport::reexports::hyper::Request<tonic::body::Body>,
    > for Nothing
{
    type Response = armonik_transport::reexports::hyper::Response<tonic::body::Body>;
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(
        &mut self,
        _request: armonik_transport::reexports::hyper::Request<tonic::body::Body>,
    ) -> Self::Future {
        std::future::ready(Ok(armonik_transport::reexports::hyper::Response::new(
            tonic::body::Body::default(),
        )))
    }
}

/// Serve nothing on an ephemeral loopback port and return its `http://` endpoint.
async fn serve_nothing() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind the test server");
    let address = listener.local_addr().expect("the test server's address");

    tokio::spawn(async move {
        let incoming =
            armonik_transport::reexports::tonic::transport::server::TcpIncoming::from(listener);
        armonik_transport::reexports::tonic::transport::Server::builder()
            .serve_with_incoming(Nothing, incoming)
            .await
            .expect("serve the empty test router");
    });

    format!("http://{address}")
}

#[test]
fn a_null_config_is_rejected() {
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(ptr::null(), &mut client, &mut err) };

    assert_eq!(code, AK_ERR_NULL_POINTER);
    assert!(client.is_null());
}

#[test]
fn a_null_out_pointer_is_rejected() {
    let config = empty_config();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, ptr::null_mut(), &mut err) };

    assert_eq!(code, AK_ERR_NULL_POINTER);
}

#[test]
fn invalid_utf8_is_its_own_error_code_not_the_generic_one() {
    let mut config = empty_config();
    config.endpoint = invalid_utf8_bytes();
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, &mut client, &mut err) };

    assert_eq!(code, AK_ERR_INVALID_UTF8);
    assert!(client.is_null());
    unsafe { armonik_ffi::buffer::ak_bytes_free(err) };
}

#[test]
fn a_missing_endpoint_is_rejected() {
    let config = empty_config();
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, &mut client, &mut err) };

    assert_eq!(code, AK_ERR_INVALID_CONFIG);
    assert!(client.is_null());
    let message = unsafe { std::slice::from_raw_parts(err.ptr, err.len) };
    assert!(String::from_utf8_lossy(message).contains("endpoint"));
    unsafe { armonik_ffi::buffer::ak_bytes_free(err) };
}

#[test]
fn a_key_without_a_matching_certificate_is_rejected() {
    let mut config = empty_config();
    config.endpoint = bytes_of("http://127.0.0.1:1");
    config.key_pem = bytes_of("not even close to a key");
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, &mut client, &mut err) };

    assert_eq!(code, AK_ERR_INVALID_CONFIG);
    let message = unsafe { std::slice::from_raw_parts(err.ptr, err.len) };
    assert!(String::from_utf8_lossy(message).contains("cert_pem"));
    unsafe { armonik_ffi::buffer::ak_bytes_free(err) };
}

#[test]
fn garbage_pem_is_rejected_rather_than_silently_accepted() {
    let mut config = empty_config();
    config.endpoint = bytes_of("http://127.0.0.1:1");
    config.cert_pem = bytes_of("garbage");
    config.key_pem = bytes_of("garbage");
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, &mut client, &mut err) };

    assert_eq!(code, AK_ERR_INVALID_CONFIG);
    assert!(client.is_null());
    unsafe { armonik_ffi::buffer::ak_bytes_free(err) };
}

#[test]
fn a_timeout_below_the_sentinel_is_rejected_rather_than_defaulted() {
    let mut config = empty_config();
    config.endpoint = bytes_of("http://127.0.0.1:1");
    // Only -1 means "the default"; anything else negative is a mistake, not another way to spell it.
    config.connect_timeout_ms = -2;
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, &mut client, &mut err) };

    assert_eq!(code, AK_ERR_INVALID_CONFIG);
    assert!(client.is_null());
    let message = unsafe { std::slice::from_raw_parts(err.ptr, err.len) };
    assert!(String::from_utf8_lossy(message).contains("connect_timeout_ms"));
    unsafe { armonik_ffi::buffer::ak_bytes_free(err) };
}

#[test]
fn an_unreachable_endpoint_reports_connection_failed() {
    let mut config = empty_config();
    // A reserved, non-routable TEST-NET-1 address (RFC 5737): connecting to it fails, and quickly,
    // rather than the test hanging on a real timeout.
    config.endpoint = bytes_of("http://192.0.2.1:1");
    config.connect_timeout_ms = 200;
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, &mut client, &mut err) };

    assert_eq!(code, AK_ERR_CONNECTION_FAILED);
    assert!(client.is_null());
    let message = unsafe { std::slice::from_raw_parts(err.ptr, err.len) };
    let message = String::from_utf8_lossy(message);
    // More than just "Could not connect to the remote": the actual reason - a timeout, here - has
    // to reach the caller too, not only which step failed.
    assert!(message.contains(": "), "{message}");
    // Not a Rust source location either: the caller on the other side of this ABI cannot open the
    // file it would name.
    assert!(!message.contains(".rs:"), "{message}");
    unsafe { armonik_ffi::buffer::ak_bytes_free(err) };
}

#[test]
fn a_real_server_is_reached_and_the_client_is_freed_cleanly() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime to serve the test endpoint on");
    let endpoint = runtime.block_on(serve_nothing());

    let mut config = empty_config();
    config.endpoint = bytes_of(&endpoint);
    let mut client = ptr::null_mut();
    let mut err = ak_bytes::EMPTY;

    let code = unsafe { ak_client_create(&config, &mut client, &mut err) };

    assert_eq!(code, 0, "connect should succeed against a real listener");
    assert!(!client.is_null());

    unsafe { ak_client_free(client) };
}

#[test]
fn freeing_a_null_client_is_a_no_op() {
    unsafe { ak_client_free(ptr::null_mut()) };
}

#[test]
fn freeing_an_empty_bytes_buffer_is_a_no_op() {
    unsafe { armonik_ffi::buffer::ak_bytes_free(ak_bytes::EMPTY) };
}
