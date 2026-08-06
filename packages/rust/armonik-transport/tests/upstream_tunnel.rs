//! Tripwires for what `hyper_util`'s `Tunnel`, which the proxy connector drives, gets wrong.
//!
//! Each test asserts that a known upstream defect is still there; the crate README's "Known
//! issues" section describes them all. A `hyper-util` release that fixes one turns its tripwire
//! red: read the failure as the notice that the fix has landed, delete the tripwire, and update
//! the README. A dependency bump turning CI red is the point; read the failure before assuming it
//! is a regression.

use std::time::Duration;

use armonik_transport::{ClientConfig, ProxyConfig};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

mod common;

use common::{error_chain, ProxyAuth, SlowService};

/// A client reaching `endpoint` through `proxy`.
fn through_proxy(endpoint: &str, proxy: std::net::SocketAddr) -> ClientConfig {
    let mut config = common::config(endpoint, |_| {});
    config.proxy = ProxyConfig::explicit(
        hyper::Uri::try_from(format!("http://{proxy}")).expect("a valid proxy URI"),
    );
    config
}

/// Connect and make one call, returning what the server answered.
async fn call_through(config: ClientConfig) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
    let channel = armonik_transport::connect(config).await?;
    Ok(common::call(channel).await?)
}

/// The control. Without it, a broken fixture would read as upstream still being broken.
#[tokio::test]
async fn control_a_plain_200_opens_the_tunnel() {
    let server = common::serve(SlowService::new(Duration::ZERO)).await;
    let (proxy, tunnels) = common::spawn_proxy(ProxyAuth::None, 200).await;

    let answer = call_through(through_proxy(&server, proxy))
        .await
        .expect("the fixture itself must let an ordinary 200 through");

    assert_eq!(answer, common::REPLY);
    assert_eq!(tunnels.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn known_issue_a_success_other_than_200_does_not_open_the_tunnel() {
    // RFC 9110: any 2xx switches the connection to tunnel mode. `Tunnel` checks for exactly `200`,
    // so a proxy answering 201 is a tunnel that should open and does not.
    //
    // Not asserted on the fixture's tunnel counter: the fake proxy counts a tunnel as soon as it
    // has written its own response, before learning whether the client accepted it, so that counter
    // answers a different question from the one this test asks.
    let server = common::serve(SlowService::new(Duration::ZERO)).await;
    let (proxy, _tunnels) = common::spawn_proxy(ProxyAuth::None, 201).await;

    let error = call_through(through_proxy(&server, proxy))
        .await
        .expect_err(
            "201 unexpectedly opened the tunnel: hyper-util now accepts any 2xx. \
             Delete this tripwire and update the README's Known issues section.",
        );

    assert!(
        error_chain(error.as_ref()).contains("did not open the tunnel"),
        "unexpected error: {}",
        error_chain(error.as_ref())
    );
}

#[tokio::test]
async fn known_issue_an_http_1_0_407_is_not_recognised_as_authentication_required() {
    // `Tunnel` only special-cases `HTTP/1.1 407`, so a proxy that answers in `HTTP/1.0` (legal, and
    // how an old or minimal proxy might reply) falls into its generic refusal, and this crate's
    // error translation never sees the "proxy authorization required" text it looks for. The
    // message naming which two options to set is not shown for it.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let proxy = listener.local_addr().expect("proxy address");
    tokio::spawn(async move {
        let Ok((mut client, _)) = listener.accept().await else {
            return;
        };
        let _ = common::read_head(&mut client).await;
        let _ = client
            .write_all(b"HTTP/1.0 407 Proxy Authentication Required\r\n\r\n")
            .await;
        let _ = client.flush().await;
    });

    let server = common::serve(SlowService::new(Duration::ZERO)).await;
    let error = call_through(through_proxy(&server, proxy))
        .await
        .expect_err("the proxy should have refused the tunnel");

    let rendered = error_chain(error.as_ref());
    assert!(
        !rendered.contains("requires authentication"),
        "hyper-util now recognises an HTTP/1.0 407 too: delete this tripwire and update the \
         README's Known issues section. Got: {rendered}"
    );
    assert!(
        rendered.contains("did not open the tunnel"),
        "unexpected error: {rendered}"
    );
}

#[tokio::test]
async fn known_issue_a_portless_http_target_is_dialled_on_443_not_80() {
    // `Tunnel::call` defaults to 443 unconditionally when the target carries no port, regardless of
    // scheme, rather than the scheme deciding between 80 and 443. ArmoniK deployments always name a
    // port, so this is unlikely to bite in practice; still worth a tripwire.
    //
    // This listener records the `CONNECT` authority and closes without ever trying to reach it: the
    // recorded target, not a connection, is what the test asserts on.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let proxy = listener.local_addr().expect("proxy address");
    let requested_target = std::sync::Arc::new(std::sync::Mutex::new(None));
    let captured = std::sync::Arc::clone(&requested_target);
    tokio::spawn(async move {
        let Ok((mut client, _)) = listener.accept().await else {
            return;
        };
        let Ok(head) = common::read_head(&mut client).await else {
            return;
        };
        *captured.lock().expect("lock") = Some(common::request_target(&head));
        // No response: the client only needs to have sent the request to be observed here, and this
        // address would never answer regardless.
    });

    // Not a real server, and not dialled: 192.0.2.0/24 is reserved for documentation by RFC 5737,
    // so it needs no resolver.
    let _ = call_through(through_proxy("http://192.0.2.1", proxy)).await;

    assert_eq!(
        requested_target.lock().expect("lock").as_deref(),
        Some("192.0.2.1:443"),
        "if this now reads :80, hyper-util has fixed its default: delete this tripwire and update \
         the README's Known issues section"
    );
}
