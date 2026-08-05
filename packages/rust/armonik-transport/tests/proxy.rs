//! End-to-end tests for HTTP `CONNECT` tunnelling.
//!
//! A real client, through a real proxy, to a real gRPC server over loopback sockets. The proxy is a
//! few dozen lines below rather than an external binary, so the tests run wherever CI does.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use armonik_transport::{HttpConfig, ProxyConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

/// Observations a test can make about what the proxy did.
#[derive(Debug, Default)]
struct ProxyStats {
    /// How many `CONNECT` requests were accepted and tunnelled.
    tunnels: AtomicUsize,
    /// How many `CONNECT` requests were rejected for missing or wrong credentials.
    rejected: AtomicUsize,
}

/// A minimal HTTP proxy that only implements `CONNECT`, answering 200.
async fn spawn_proxy(auth: ProxyAuth) -> (SocketAddr, Arc<ProxyStats>) {
    spawn_proxy_answering(auth, 200).await
}

/// The same, answering `success` to a `CONNECT` it accepts.
async fn spawn_proxy_answering(auth: ProxyAuth, success: u16) -> (SocketAddr, Arc<ProxyStats>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let address = listener.local_addr().expect("proxy address");
    let stats = Arc::new(ProxyStats::default());

    let accepted = Arc::clone(&stats);
    tokio::spawn(async move {
        loop {
            let Ok((client, _)) = listener.accept().await else {
                return;
            };
            let stats = Arc::clone(&accepted);
            tokio::spawn(async move {
                // A failing tunnel is a normal outcome in these tests; the client asserts on it.
                let _ = serve_tunnel(client, auth, success, stats).await;
            });
        }
    });

    (address, stats)
}

async fn serve_tunnel(
    mut client: TcpStream,
    auth: ProxyAuth,
    success: u16,
    stats: Arc<ProxyStats>,
) -> std::io::Result<()> {
    let head = read_head(&mut client).await?;
    let target = request_target(&head);

    if let ProxyAuth::Required(expected) = auth {
        let presented = head.lines().find_map(|line| {
            line.strip_prefix("Proxy-Authorization: Basic ")
                .or_else(|| line.strip_prefix("proxy-authorization: Basic "))
        });

        if presented != Some(expected) {
            stats.rejected.fetch_add(1, Ordering::SeqCst);
            client
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await?;
            return client.flush().await;
        }
    }

    let mut upstream = TcpStream::connect(&target).await?;
    client
        .write_all(format!("HTTP/1.1 {success} Connection established\r\n\r\n").as_bytes())
        .await?;
    client.flush().await?;
    stats.tunnels.fetch_add(1, Ordering::SeqCst);

    tokio::io::copy_bidirectional(&mut client, &mut upstream)
        .await
        .map(|_| ())
}

/// The request-target of a `CONNECT` request's start line, e.g. `proxy.corp:3128`.
fn request_target(head: &str) -> String {
    head.lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or_default()
        .to_owned()
}

/// Read up to the blank line that ends an HTTP head.
async fn read_head(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut head = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).await?;
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            return Ok(String::from_utf8_lossy(&head).into_owned());
        }
        if head.len() > 8 * 1024 {
            return Err(std::io::Error::other("head too large"));
        }
    }
}

/// A client reaching `endpoint` through `proxy`, spelled as an environment would.
///
/// Through `HttpConfigArgs` on purpose, so the parsing is under test along with the tunnelling.
fn through_proxy(
    endpoint: &str,
    proxy: SocketAddr,
    credentials: Option<(&str, &str)>,
) -> HttpConfig {
    common::config(endpoint, |args| {
        args.proxy.source = format!("http://{proxy}");
        if let Some((username, password)) = credentials {
            args.proxy.username = String::from(username);
            args.proxy.password = password.into();
        }
    })
}

/// A client that connects directly, whatever proxy the tests happen to be running.
fn direct(endpoint: &str) -> HttpConfig {
    common::config(endpoint, |_| {})
}

/// Connect and make one call, returning what the server answered.
async fn call_through(config: HttpConfig) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
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
    let (proxy, stats) = spawn_proxy(ProxyAuth::None).await;

    let answer = call_through(through_proxy(&server, proxy, None))
        .await
        .expect("the call should succeed through the proxy");

    assert_eq!(answer, common::REPLY);
    assert_eq!(
        stats.tunnels.load(Ordering::SeqCst),
        1,
        "the request must have gone through the proxy, not around it"
    );
}

#[tokio::test]
async fn credentials_are_presented_when_the_proxy_demands_them() {
    let server = spawn_server().await;
    // The base64 of `user:secret`, which is what the client is expected to send.
    let (proxy, stats) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    let answer = call_through(through_proxy(&server, proxy, Some(("user", "secret"))))
        .await
        .expect("the call should succeed once credentials are supplied");

    assert_eq!(answer, common::REPLY);
    assert_eq!(stats.tunnels.load(Ordering::SeqCst), 1);
    assert_eq!(stats.rejected.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_proxy_can_be_configured_without_going_through_the_environment() {
    let server = spawn_server().await;
    let (proxy, stats) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    // `ProxyConfig` is `#[non_exhaustive]`, so these constructors are the only way another crate can
    // build one. Worth its own test: nothing else here exercises them.
    let mut config = direct(&server);
    config.proxy = ProxyConfig::explicit(
        hyper::Uri::try_from(format!("http://{proxy}")).expect("a valid proxy URI"),
    )
    .with_credentials("user", "secret");

    let answer = call_through(config)
        .await
        .expect("the call should succeed through the proxy");

    assert_eq!(answer, common::REPLY);
    assert_eq!(stats.tunnels.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn credentials_written_into_the_proxy_url_authenticate_the_tunnel() {
    let server = spawn_server().await;
    let (proxy, stats) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    // The conventional `HTTPS_PROXY` form. Accepting the URL and then not authenticating with it is
    // the failure this pins.
    let config = common::config(&server, |args| {
        args.proxy.source = format!("http://user:secret@{proxy}");
    });

    let answer = call_through(config)
        .await
        .expect("the credentials in the URL should have been used");

    assert_eq!(answer, common::REPLY);
    assert_eq!(stats.tunnels.load(Ordering::SeqCst), 1);
    assert_eq!(stats.rejected.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_missing_credential_is_reported_as_such() {
    let server = spawn_server().await;
    let (proxy, stats) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    let error = call_through(through_proxy(&server, proxy, None))
        .await
        .expect_err("the proxy should have refused the tunnel");

    // The message has to name the options to set, otherwise a 407 is a dead end for whoever hits
    // it.
    let rendered = error_chain(error.as_ref());
    assert!(
        rendered.contains("requires authentication"),
        "unexpected error: {rendered}"
    );
    assert!(
        rendered.contains("proxy_username"),
        "the error should say which options to set: {rendered}"
    );
    assert_eq!(stats.rejected.load(Ordering::SeqCst), 1);
    assert_eq!(stats.tunnels.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn wrong_credentials_are_rejected() {
    let server = spawn_server().await;
    let (proxy, stats) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    let error = call_through(through_proxy(&server, proxy, Some(("user", "wrong"))))
        .await
        .expect_err("the proxy should have refused the tunnel");

    let rendered = error_chain(error.as_ref());
    assert!(
        rendered.contains("requires authentication"),
        "unexpected error: {rendered}"
    );
    assert_eq!(stats.rejected.load(Ordering::SeqCst), 1);
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
    let (proxy, stats) = spawn_proxy(ProxyAuth::None).await;

    let answer = call_through(direct(&server))
        .await
        .expect("a direct call should succeed");

    assert_eq!(answer, common::REPLY);
    assert_eq!(
        stats.tunnels.load(Ordering::SeqCst),
        0,
        "the proxy must not be involved when it is disabled"
    );
    let _ = proxy;
}

// --- what the environment decides, through the real connector ---
//
// `hyper_util`'s matcher does the reading; these pin how this crate maps ArmoniK's option vocabulary
// onto it, which is ours to get right. They set process-wide variables, hence `serial`.

#[tokio::test]
#[serial_test::serial(env)]
async fn system_mode_takes_the_proxy_from_the_environment() {
    let server = spawn_server().await;
    let (proxy, stats) = spawn_proxy(ProxyAuth::None).await;

    // The variable has to still be set when `connect` runs, not merely when the configuration is
    // built: `system` resolves the environment as the connection is made. Every other option is read
    // once, in `HttpConfigArgs::from_env`.
    let _http_proxy = common::EnvGuard::set("HTTP_PROXY", &format!("http://{proxy}"));
    let outcome = call_through(common::config(&server, |args| {
        args.proxy.source = String::from("system")
    }))
    .await;

    assert_eq!(
        outcome.expect("the call should go through the proxy the environment names"),
        common::REPLY
    );
    assert_eq!(stats.tunnels.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial_test::serial(env)]
async fn no_proxy_bypasses_the_proxy_in_system_mode() {
    let server = spawn_server().await;
    let (proxy, stats) = spawn_proxy(ProxyAuth::None).await;

    let _http_proxy = common::EnvGuard::set("HTTP_PROXY", &format!("http://{proxy}"));
    let _no_proxy = common::EnvGuard::set("NO_PROXY", "127.0.0.1");
    let outcome = call_through(common::config(&server, |args| {
        args.proxy.source = String::from("system")
    }))
    .await;

    assert_eq!(outcome.expect("a direct call succeeds"), common::REPLY);
    assert_eq!(
        stats.tunnels.load(Ordering::SeqCst),
        0,
        "NO_PROXY named the target, so the proxy must not have been used"
    );
}

#[tokio::test]
#[serial_test::serial(env)]
async fn no_proxy_does_not_apply_to_an_explicitly_configured_proxy() {
    let server = spawn_server().await;
    let (proxy, stats) = spawn_proxy(ProxyAuth::None).await;

    // `NO_PROXY` belongs to the same environment convention as `HTTP_PROXY`, so it governs `system`
    // only. ArmoniK's other clients give an explicitly named proxy an empty bypass list, and diverging
    // would mean a request skipping the proxy here while using it there.
    let _no_proxy = common::EnvGuard::set("NO_PROXY", "127.0.0.1");
    let outcome = call_through(through_proxy(&server, proxy, None)).await;

    assert_eq!(
        outcome.expect("an explicit proxy is used whatever NO_PROXY says"),
        common::REPLY
    );
    assert_eq!(stats.tunnels.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial_test::serial(env)]
async fn a_dedicated_credential_in_system_mode_keeps_the_other_half_the_url_carried() {
    // A dedicated option alone must not discard the other half of whatever the intercepted proxy
    // URL carried: setting only `ProxyPassword` while `HTTP_PROXY` names a username must still send
    // that username, paired with the new password, not an empty one.
    let server = spawn_server().await;
    // The base64 of `url-user:new`, what the client is expected to send once the two are merged.
    let (proxy, stats) = spawn_proxy(ProxyAuth::Required("dXJsLXVzZXI6bmV3")).await;

    let _http_proxy = common::EnvGuard::set("HTTP_PROXY", &format!("http://url-user:old@{proxy}"));
    let outcome = call_through(common::config(&server, |args| {
        args.proxy.source = String::from("system");
        args.proxy.password = String::from("new").into();
    }))
    .await;

    assert_eq!(
        outcome.expect("the merged credentials should satisfy the proxy"),
        common::REPLY
    );
    assert_eq!(stats.tunnels.load(Ordering::SeqCst), 1);
    assert_eq!(
        stats.rejected.load(Ordering::SeqCst),
        0,
        "a rejection means the URL's username was dropped instead of kept"
    );
}

#[tokio::test]
async fn known_issue_a_success_other_than_200_does_not_open_the_tunnel() {
    // RFC 9110: any 2xx switches the connection to tunnel mode. `hyper_util`'s `Tunnel`, which this
    // crate delegates the handshake to, checks for exactly `200`, so a proxy answering 201 is a tunnel
    // that should open and does not. See the crate README's "Known issues".
    //
    // A tripwire, not a preference: the day `hyper_util` accepts any 2xx, this starts failing, which
    // is the signal to loosen it back to asserting success and to update the README.
    //
    // Not asserted on `ProxyStats::tunnels`: the fake proxy counts a tunnel as soon as it has written
    // its own response, before learning whether the client accepted it, so that counter answers a
    // different question from the one this test asks.
    let server = spawn_server().await;
    let (proxy, _stats) = spawn_proxy_answering(ProxyAuth::None, 201).await;

    let error = call_through(through_proxy(&server, proxy, None))
        .await
        .expect_err("201 unexpectedly opened the tunnel");

    assert!(
        error_chain(error.as_ref()).contains("did not open the tunnel"),
        "unexpected error: {}",
        error_chain(error.as_ref())
    );
}

#[tokio::test]
async fn known_issue_an_http_1_0_407_is_not_recognised_as_authentication_required() {
    // `hyper_util`'s `Tunnel` only special-cases `HTTP/1.1 407`, so a proxy that answers in `HTTP/1.0`
    // (legal, and how an old or minimal proxy might reply) falls into its generic refusal, and this
    // crate's `translate` never sees the "proxy authorization required" text it looks for. A tripwire:
    // the day the check widens to either version, the message assertion below starts failing.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let proxy = listener.local_addr().expect("proxy address");
    tokio::spawn(async move {
        let Ok((mut client, _)) = listener.accept().await else {
            return;
        };
        let _ = read_head(&mut client).await;
        let _ = client
            .write_all(b"HTTP/1.0 407 Proxy Authentication Required\r\n\r\n")
            .await;
        let _ = client.flush().await;
    });

    let server = spawn_server().await;
    let error = call_through(through_proxy(&server, proxy, None))
        .await
        .expect_err("the proxy should have refused the tunnel");

    let rendered = error_chain(error.as_ref());
    assert!(
        !rendered.contains("requires authentication"),
        "hyper_util now recognises an HTTP/1.0 407 too: update `translate` and this test. \
         Got: {rendered}"
    );
    assert!(
        rendered.contains("did not open the tunnel"),
        "unexpected error: {rendered}"
    );
}

#[tokio::test]
async fn known_issue_a_portless_http_target_is_dialled_on_443_not_80() {
    // `hyper_util`'s `Tunnel::call` defaults to 443 unconditionally when the target carries no port,
    // regardless of scheme, rather than the scheme deciding between 80 and 443. ArmoniK deployments
    // always name a port, so this is unlikely to bite in practice; still worth a tripwire; see the
    // crate README's "Known issues".
    //
    // Not through `spawn_proxy`/`serve_tunnel`: those dial the named target for real once the tunnel
    // is accepted, and a documentation-space address such as this one does not refuse a connection so
    // much as go quiet, which is a real wait rather than a fast local failure. This listener records
    // the `CONNECT` authority and closes without ever trying to reach it.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let proxy = listener.local_addr().expect("proxy address");
    let requested_target = std::sync::Arc::new(std::sync::Mutex::new(None));
    let captured = std::sync::Arc::clone(&requested_target);
    tokio::spawn(async move {
        let Ok((mut client, _)) = listener.accept().await else {
            return;
        };
        let Ok(head) = read_head(&mut client).await else {
            return;
        };
        *captured.lock().expect("lock") = Some(request_target(&head));
        // No response: the client only needs to have sent the request to be observed here, and this
        // address would never answer regardless.
    });

    // Not a real server either, and not dialled: 192.0.2.0/24 is reserved for documentation by
    // RFC 5737, so it needs no resolver and this test asserts on the request, not on a connection.
    let _ = call_through(through_proxy("http://192.0.2.1", proxy, None)).await;

    assert_eq!(
        requested_target.lock().expect("lock").as_deref(),
        Some("192.0.2.1:443"),
        "if this now reads :80, hyper_util has fixed its default: update this test and the README"
    );
}

#[tokio::test]
#[serial_test::serial(env)]
async fn an_https_proxy_from_the_environment_is_refused_before_dialling() {
    // `system` resolves at connect time, so that is the only place its scheme can be checked. Dialling
    // it would write the handshake in the clear to a proxy expecting TLS.
    let server = spawn_server().await;

    let _http_proxy = common::EnvGuard::set("HTTP_PROXY", "https://proxy.corp:3128");
    let outcome = call_through(common::config(&server, |args| {
        args.proxy.source = String::from("system")
    }))
    .await;

    let error = error_chain(
        outcome
            .expect_err("an https proxy cannot be reached")
            .as_ref(),
    );
    assert!(error.contains("only an `http` proxy"), "{error}");
}
