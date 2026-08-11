//! End-to-end tests for HTTP `CONNECT` tunnelling.
//!
//! A real client, through a real proxy, to a real gRPC server over loopback sockets. The proxy is a
//! few dozen lines below rather than an external binary, so the tests run wherever CI does.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use armonik_transport::{ClientConfig, ProxyConfig};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

mod common;

use common::SlowService;

/// Serve the gRPC service the tests call, on an ephemeral loopback port.
async fn spawn_server() -> String {
    common::serve(SlowService::new(Duration::ZERO)).await
}

/// What a test proxy should demand of its clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProxyAuth {
    /// Accept every tunnel request.
    None,
    /// Reject with `407` unless the expected `Proxy-Authorization` header is present.
    Required(&'static str),
}

/// How many `CONNECT` requests a test proxy accepted and tunnelled.
type Tunnels = Arc<AtomicUsize>;

/// A minimal HTTP proxy that only implements `CONNECT`, answering 200.
async fn spawn_proxy(auth: ProxyAuth) -> (SocketAddr, Tunnels) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let address = listener.local_addr().expect("proxy address");
    let tunnels = Tunnels::default();

    let accepted = Arc::clone(&tunnels);
    tokio::spawn(async move {
        loop {
            let Ok((client, _)) = listener.accept().await else {
                return;
            };
            let tunnels = Arc::clone(&accepted);
            tokio::spawn(async move {
                // A failing tunnel is a normal outcome in these tests; the client asserts on it.
                let _ = serve_tunnel(client, auth, tunnels).await;
            });
        }
    });

    (address, tunnels)
}

async fn serve_tunnel(
    mut client: TcpStream,
    auth: ProxyAuth,
    tunnels: Tunnels,
) -> std::io::Result<()> {
    let head = common::read_head(&mut client).await?;
    let target = common::request_target(&head);

    if let ProxyAuth::Required(expected) = auth {
        let presented = head.lines().find_map(|line| {
            line.strip_prefix("Proxy-Authorization: Basic ")
                .or_else(|| line.strip_prefix("proxy-authorization: Basic "))
        });

        if presented != Some(expected) {
            client
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await?;
            return client.flush().await;
        }
    }

    let mut upstream = TcpStream::connect(&target).await?;
    client
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .await?;
    client.flush().await?;
    tunnels.fetch_add(1, Ordering::SeqCst);

    tokio::io::copy_bidirectional(&mut client, &mut upstream)
        .await
        .map(|_| ())
}

/// A client reaching `endpoint` through `proxy`, configured through the API.
fn through_proxy(
    endpoint: &str,
    proxy: SocketAddr,
    credentials: Option<(&str, &str)>,
) -> ClientConfig {
    let mut config = common::config(endpoint, |_| {});
    let mut proxy = ProxyConfig::explicit(
        hyper::Uri::try_from(format!("http://{proxy}")).expect("a valid proxy URI"),
    );
    if let Some((username, password)) = credentials {
        proxy = proxy.with_credentials(username, password);
    }
    config.proxy = proxy;
    config
}

/// A client that connects directly, whatever proxy the tests happen to be running.
fn direct(endpoint: &str) -> ClientConfig {
    common::config(endpoint, |_| {})
}

/// Connect and make one call, returning what the server answered.
async fn call_through(config: ClientConfig) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
    let channel = armonik_transport::connect(config).await?;
    Ok(common::call(channel).await?)
}

/// Render an error and everything it was caused by.
///
/// The transport error that wraps a proxy failure has a generic message, so an assertion has to look
/// at the whole chain.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut rendered = vec![error.to_string()];
    let mut current = error.source();
    while let Some(source) = current {
        rendered.push(source.to_string());
        current = source.source();
    }
    rendered.join(" -> ")
}

#[tokio::test]
async fn request_reaches_the_server_through_the_tunnel() {
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::None).await;

    let answer = call_through(through_proxy(&server, proxy, None))
        .await
        .expect("the call should succeed through the proxy");

    assert_eq!(answer, common::REPLY);
    assert_eq!(
        tunnels.load(Ordering::SeqCst),
        1,
        "the request must have gone through the proxy, not around it"
    );
}

#[tokio::test]
async fn credentials_are_presented_when_the_proxy_demands_them() {
    let server = spawn_server().await;
    // The base64 of `user:secret`, which is what the client is expected to send.
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    let answer = call_through(through_proxy(&server, proxy, Some(("user", "secret"))))
        .await
        .expect("the call should succeed once credentials are supplied");

    assert_eq!(answer, common::REPLY);
    assert_eq!(tunnels.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn credentials_written_into_the_proxy_url_authenticate_the_tunnel() {
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    // The conventional `HTTPS_PROXY` form. Accepting the URL and then not authenticating with it is
    // the failure this pins.
    let mut config = direct(&server);
    config.proxy = ProxyConfig::explicit(
        hyper::Uri::try_from(format!("http://user:secret@{proxy}")).expect("a valid proxy URI"),
    );

    let answer = call_through(config)
        .await
        .expect("the credentials in the URL should have been used");

    assert_eq!(answer, common::REPLY);
    assert_eq!(tunnels.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_missing_credential_is_reported_as_such() {
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    let error = call_through(through_proxy(&server, proxy, None))
        .await
        .expect_err("the proxy should have refused the tunnel");

    // The message has to say credentials are what is missing, otherwise a 407 is a dead end for
    // whoever hits it.
    let rendered = error_chain(error.as_ref());
    assert!(
        rendered.contains("requires authentication"),
        "unexpected error: {rendered}"
    );
    // And the failure has to be matchable by type, not only by text: `find_in` is how a caller
    // reacts to this case in particular, e.g. by prompting for credentials.
    let typed = armonik_transport::ProxyError::find_in(error.as_ref());
    assert!(
        matches!(
            typed,
            Some(armonik_transport::ProxyError::AuthenticationRequired { .. })
        ),
        "expected AuthenticationRequired in the chain, got {typed:?}"
    );
    assert_eq!(tunnels.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn wrong_credentials_are_rejected() {
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    let error = call_through(through_proxy(&server, proxy, Some(("user", "wrong"))))
        .await
        .expect_err("the proxy should have refused the tunnel");

    let rendered = error_chain(error.as_ref());
    assert!(
        rendered.contains("requires authentication"),
        "unexpected error: {rendered}"
    );
    assert_eq!(tunnels.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_dead_proxy_fails_instead_of_bypassing_it() {
    let server = spawn_server().await;

    // Bind a port and drop it, so nothing is listening there.
    let dead = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind")
        .local_addr()
        .expect("address");
    drop(TcpListener::bind(dead).await);

    let error = call_through(through_proxy(&server, dead, None))
        .await
        .expect_err("an unreachable proxy must fail the call");

    // Silently falling back to a direct connection would defeat the point of configuring a proxy.
    let rendered = error_chain(error.as_ref());
    assert!(
        rendered.contains("Could not connect to the proxy"),
        "the error should name the proxy: {rendered}"
    );
}

#[tokio::test]
async fn no_proxy_is_used_when_proxying_is_disabled() {
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::None).await;

    let answer = call_through(direct(&server))
        .await
        .expect("a direct call should succeed");

    assert_eq!(answer, common::REPLY);
    assert_eq!(
        tunnels.load(Ordering::SeqCst),
        0,
        "the proxy must not be involved when it is disabled"
    );
    let _ = proxy;
}

// --- the handshake is bounded in time ---

#[tokio::test]
async fn a_proxy_that_goes_quiet_fails_within_the_connect_timeout() {
    // `Tunnel` has no timeout of its own, so without this bound a proxy that accepts the connection
    // and never answers would hang the client. The configured connect timeout is the knob that
    // governs how long connecting may take, so it governs the proxied path too.
    let server = spawn_server().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let proxy = listener.local_addr().expect("proxy address");
    tokio::spawn(async move {
        // Accept and hold the socket without ever answering.
        let Ok((client, _)) = listener.accept().await else {
            return;
        };
        tokio::time::sleep(Duration::from_secs(30)).await;
        drop(client);
    });

    let mut config = through_proxy(&server, proxy, None);
    config.connect_timeout = Some(Duration::from_millis(200));

    let started = std::time::Instant::now();
    let error = call_through(config)
        .await
        .expect_err("a quiet proxy must time out");

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the timeout should be the configured one, not a hang"
    );
    let rendered = error_chain(error.as_ref());
    assert!(
        rendered.contains("did not complete the tunnel"),
        "unexpected error: {rendered}"
    );
}
