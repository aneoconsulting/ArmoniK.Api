//! End-to-end tests for HTTP `CONNECT` tunnelling.
//!
//! A real client, through a real proxy, to a real gRPC server over loopback sockets. The proxy is a
//! few dozen lines below rather than an external binary, so the tests run wherever CI does.

use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::time::Duration;

use armonik_transport::{HttpConfig, ProxyConfig};
use tokio::net::TcpListener;

mod common;

use common::{ProxyAuth, SlowService, Tunnels};

/// Serve the gRPC service the tests call, on an ephemeral loopback port.
async fn spawn_server() -> String {
    common::serve(SlowService::new(Duration::ZERO)).await
}

/// A minimal HTTP proxy that only implements `CONNECT`, answering 200.
async fn spawn_proxy(auth: ProxyAuth) -> (SocketAddr, Tunnels) {
    common::spawn_proxy(auth, 200).await
}

/// A client reaching `endpoint` through `proxy`, configured through the API.
fn through_proxy(
    endpoint: &str,
    proxy: SocketAddr,
    credentials: Option<(&str, &str)>,
) -> HttpConfig {
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
fn direct(endpoint: &str) -> HttpConfig {
    common::config(endpoint, |_| {})
}

/// The same client, following the environment.
fn following_the_environment(endpoint: &str) -> HttpConfig {
    let mut config = common::config(endpoint, |_| {});
    config.proxy = ProxyConfig::system();
    config
}

/// Connect and make one call, returning what the server answered.
async fn call_through(config: HttpConfig) -> Result<bytes::Bytes, Box<dyn std::error::Error>> {
    let channel = armonik_transport::connect(config).await?;
    Ok(common::call(channel).await?)
}

use common::error_chain;

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

    // The message has to name the options to set, otherwise a 407 is a dead end for whoever hits
    // it.
    let rendered = error_chain(error.as_ref());
    assert!(
        rendered.contains("requires authentication"),
        "unexpected error: {rendered}"
    );
    assert!(
        rendered.contains("ProxyUsername"),
        "the error should say which options to set: {rendered}"
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

// --- what the environment decides, through the real connector ---
//
// `hyper_util`'s matcher does the reading; these pin how this crate maps its configuration onto it,
// which is ours to get right. They set process-wide variables, hence `serial`; and the host may
// already export proxy variables of its own, hence [`ProxyEnvironment`].

/// Every variable `Matcher::from_env` reads, in both spellings.
const PROXY_VARIABLES: [&str; 8] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

/// The proxy-related environment, cleared for the duration of a test.
///
/// `serial` keeps these tests from racing each other, but does nothing about what the host already
/// exports: a runner with `NO_PROXY=127.0.0.1` would bypass the proxy a test just configured. The
/// guard saves and clears [`PROXY_VARIABLES`] on creation and restores them on drop - also when the
/// test panics, so one failure cannot contaminate the next.
struct ProxyEnvironment {
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl ProxyEnvironment {
    fn cleared() -> Self {
        let saved = PROXY_VARIABLES
            .iter()
            .map(|&name| {
                let value = std::env::var_os(name);
                std::env::remove_var(name);
                (name, value)
            })
            .collect();
        Self { saved }
    }

    /// Set a variable the guard restores on drop.
    fn set(&self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
        assert!(
            PROXY_VARIABLES.contains(&name),
            "`{name}` is not a variable this guard restores"
        );
        std::env::set_var(name, value);
    }
}

impl Drop for ProxyEnvironment {
    fn drop(&mut self) {
        for (name, value) in self.saved.drain(..) {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

#[tokio::test]
#[serial_test::serial(env)]
async fn system_mode_takes_the_proxy_from_the_environment() {
    let environment = ProxyEnvironment::cleared();
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::None).await;

    // The variable has to still be set when `connect` runs, not merely when the configuration is
    // built: `system` resolves the environment as the connection is made.
    environment.set("HTTP_PROXY", format!("http://{proxy}"));
    let outcome = call_through(following_the_environment(&server)).await;

    assert_eq!(
        outcome.expect("the call should go through the proxy the environment names"),
        common::REPLY
    );
    assert_eq!(tunnels.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial_test::serial(env)]
async fn no_proxy_bypasses_the_proxy_in_system_mode() {
    let environment = ProxyEnvironment::cleared();
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::None).await;

    environment.set("HTTP_PROXY", format!("http://{proxy}"));
    environment.set("NO_PROXY", "127.0.0.1");
    let outcome = call_through(following_the_environment(&server)).await;

    assert_eq!(outcome.expect("a direct call succeeds"), common::REPLY);
    assert_eq!(
        tunnels.load(Ordering::SeqCst),
        0,
        "NO_PROXY named the target, so the proxy must not have been used"
    );
}

#[tokio::test]
#[serial_test::serial(env)]
async fn no_proxy_does_not_apply_to_an_explicitly_configured_proxy() {
    let environment = ProxyEnvironment::cleared();
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::None).await;

    // `NO_PROXY` belongs to the same environment convention as `HTTP_PROXY`, so it governs `system`
    // only. ArmoniK's other clients give an explicitly named proxy an empty bypass list, and diverging
    // would mean a request skipping the proxy here while using it there.
    environment.set("NO_PROXY", "127.0.0.1");
    let outcome = call_through(through_proxy(&server, proxy, None)).await;

    assert_eq!(
        outcome.expect("an explicit proxy is used whatever NO_PROXY says"),
        common::REPLY
    );
    assert_eq!(tunnels.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial_test::serial(env)]
async fn a_dedicated_credential_in_system_mode_keeps_the_other_half_the_url_carried() {
    // A dedicated half alone must not discard the other half of whatever the intercepted proxy
    // URL carried: setting only the password while `HTTP_PROXY` names a username must still send
    // that username, paired with the new password, not an empty one.
    let environment = ProxyEnvironment::cleared();
    let server = spawn_server().await;
    // The base64 of `url-user:new`, what the client is expected to send once the two are merged.
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::Required("dXJsLXVzZXI6bmV3")).await;

    environment.set("HTTP_PROXY", format!("http://url-user:old@{proxy}"));
    let mut config = direct(&server);
    config.proxy = ProxyConfig::system().with_credentials("", "new");
    let outcome = call_through(config).await;

    assert_eq!(
        outcome.expect("the merged credentials should satisfy the proxy"),
        common::REPLY
    );
    assert_eq!(
        tunnels.load(Ordering::SeqCst),
        1,
        "a rejection means the URL's username was dropped instead of kept"
    );
}

#[tokio::test]
#[serial_test::serial(env)]
async fn an_https_proxy_from_the_environment_is_refused_before_dialling() {
    // `system` resolves at connect time, so that is the only place its scheme can be checked. Dialling
    // it would write the handshake in the clear to a proxy expecting TLS.
    let environment = ProxyEnvironment::cleared();
    let server = spawn_server().await;

    environment.set("HTTP_PROXY", "https://proxy.corp:3128");
    let outcome = call_through(following_the_environment(&server)).await;

    let error = error_chain(
        outcome
            .expect_err("an https proxy cannot be reached")
            .as_ref(),
    );
    assert!(error.contains("only an `http` proxy"), "{error}");
}

// --- the string-form options, from the words to the tunnel ---

#[cfg(feature = "serde")]
#[tokio::test]
async fn the_proxy_options_reach_the_tunnel() {
    // Through serde on purpose, so the parsing is under test along with the tunnelling: accepting
    // the options and then not proxying is the defect the whole feature exists to fix.
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::Required("dXNlcjpzZWNyZXQ=")).await;

    let config: HttpConfig = serde_json::from_value(serde_json::json!({
        "Endpoint": server,
        "AllowUnsafeConnection": "true",
        "Proxy": format!("http://{proxy}"),
        "ProxyUsername": "user",
        "ProxyPassword": "secret",
    }))
    .expect("a valid configuration");

    let answer = call_through(config)
        .await
        .expect("the call should succeed through the proxy the options name");

    assert_eq!(answer, common::REPLY);
    assert_eq!(tunnels.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "serde")]
#[tokio::test]
#[serial_test::serial(env)]
async fn the_system_option_takes_the_proxy_from_the_environment() {
    let environment = ProxyEnvironment::cleared();
    let server = spawn_server().await;
    let (proxy, tunnels) = spawn_proxy(ProxyAuth::None).await;

    let config: HttpConfig = serde_json::from_value(serde_json::json!({
        "Endpoint": server,
        "AllowUnsafeConnection": "true",
        "Proxy": "system",
    }))
    .expect("a valid configuration");

    environment.set("HTTP_PROXY", format!("http://{proxy}"));
    let outcome = call_through(config).await;

    assert_eq!(
        outcome.expect("the call should go through the proxy the environment names"),
        common::REPLY
    );
    assert_eq!(tunnels.load(Ordering::SeqCst), 1);
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
