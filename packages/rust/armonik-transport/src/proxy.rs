//! Reaching the endpoint through an HTTP proxy.
//!
//! A `CONNECT` tunnel rather than an absolute-form request, so TLS stays end to end with the real
//! server: [`ProxyConnector`] sits below the TLS connector and hands back the stream a direct
//! connection would.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use base64::Engine;
use hyper::http::uri::{Authority, Scheme};
use hyper::Uri;
use hyper_util::client::proxy::matcher::Matcher;
use hyper_util::rt::TokioIo;
use snafu::{IntoError, ResultExt, Snafu};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tower_service::Service;

use super::{ProxyConfig, ProxySource};

/// What a connector reports when it fails, which every layer here has to be able to carry.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Upper bound on the response head a proxy may send, to stop a hostile or broken proxy from
/// making us buffer without end.
const MAX_RESPONSE_HEAD: usize = 8 * 1024;

/// Upper bound on the whole tunnel handshake, so a proxy that accepts the connection and then goes
/// quiet fails instead of hanging.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// Wraps a TCP connector so that it tunnels through an HTTP proxy when one is configured.
///
/// A request that must not be proxied goes straight to the inner connector, leaving that path as it
/// was. Generic over the connector because all this layer needs is something that turns a [`Uri`]
/// into a TCP stream; which one is knowledge it has no use for.
#[derive(Debug, Clone)]
pub struct ProxyConnector<S> {
    inner: S,
    proxy: ProxyConfig,
    /// The environment's proxy rules: `Some` for [`ProxySource::System`] alone, since a matcher
    /// answers `None` for a scheme other than `http`/`https` and would skip an explicit proxy.
    ///
    /// Behind an `Arc` because a `tower` connector is cloned per connection and `Matcher` is not
    /// `Clone`.
    matcher: Option<std::sync::Arc<Matcher>>,
}

impl<S> ProxyConnector<S> {
    /// Wrap a TCP connector with the given proxy configuration.
    pub(crate) fn new(inner: S, proxy: ProxyConfig) -> Self {
        // Read once, here: a channel that reconnects keeps the values it started with.
        let matcher = matches!(proxy.source, ProxySource::System)
            .then(|| std::sync::Arc::new(Matcher::from_env()));
        Self {
            inner,
            proxy,
            matcher,
        }
    }
}

impl<S> Service<Uri> for ProxyConnector<S>
where
    S: Service<Uri, Response = TokioIo<TcpStream>> + Send + 'static,
    S::Error: Into<BoxError>,
    S::Future: Send + 'static,
{
    type Response = TokioIo<TcpStream>;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, target: Uri) -> Self::Future {
        let route = match &self.proxy.source {
            ProxySource::Disabled => None,
            // Straight through, whatever the target looks like: the caller named this proxy, so failing
            // to reach it has to be an error rather than a quiet direct connection. Credentials were
            // taken out of the URL when the configuration was built.
            ProxySource::Explicit(uri) => Some((uri.clone(), None)),
            ProxySource::System => self
                .matcher
                .as_ref()
                .and_then(|matcher| matcher.intercept(&target))
                .map(|intercept| {
                    let from_url = intercept
                        .basic_auth()
                        .and_then(|value| value.to_str().ok())
                        .map(String::from);
                    (intercept.uri().clone(), from_url)
                }),
        };

        // Nothing to tunnel through: keep the original behaviour untouched.
        let Some((proxy_uri, from_url)) = route else {
            let future = self.inner.call(target);
            return Box::pin(async move { future.await.map_err(Into::into) });
        };

        if proxy_uri.scheme() != Some(&Scheme::HTTP) {
            let error = UnsupportedProxySnafu {
                proxy: proxy_uri.clone(),
            }
            .build();
            return Box::pin(std::future::ready(Err(error.into())));
        }

        let authority = match target_authority(&target) {
            Ok(authority) => authority,
            Err(error) => return Box::pin(std::future::ready(Err(error.into()))),
        };
        // The configured options first; whatever the proxy URL carried is the fallback, already encoded
        // by the matcher.
        let credentials = self
            .proxy
            .credentials()
            .map(|(user, password)| basic_auth(user, password))
            .or(from_url);

        let connect_to_proxy = self.inner.call(proxy_uri.clone());

        Box::pin(async move {
            let stream = connect_to_proxy.await.map_err(|source| {
                ConnectSnafu {
                    proxy: proxy_uri.clone(),
                }
                .into_error(source.into())
            })?;
            let mut stream = stream.into_inner();

            let handshake = tunnel(&mut stream, &authority, credentials.as_deref());
            tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake)
                .await
                .map_err(|_| {
                    HandshakeTimeoutSnafu {
                        proxy: proxy_uri.clone(),
                        timeout: HANDSHAKE_TIMEOUT,
                    }
                    .build()
                })??;

            tracing::debug!(proxy = %proxy_uri, target = %authority, "Established proxy tunnel");

            Ok(TokioIo::new(stream))
        })
    }
}

/// The `host:port` the tunnel should be opened to, defaulting the port from the scheme.
fn target_authority(target: &Uri) -> Result<Authority, ProxyError> {
    let host = target.host().ok_or_else(|| {
        UnsupportedTargetSnafu {
            target: target.clone(),
            reason: String::from("no host"),
        }
        .build()
    })?;

    let port = target.port_u16().unwrap_or_else(|| {
        if target.scheme() == Some(&Scheme::HTTPS) {
            443
        } else {
            80
        }
    });

    // Bracket IPv6 literals so the authority stays parseable.
    let authority = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };

    Authority::try_from(authority.as_str()).ok().ok_or_else(|| {
        UnsupportedTargetSnafu {
            target: target.clone(),
            reason: format!("`{authority}` is not a valid authority"),
        }
        .build()
    })
}

/// Perform the `CONNECT` handshake on an already-open connection to the proxy.
async fn tunnel(
    stream: &mut TcpStream,
    target: &Authority,
    authorization: Option<&str>,
) -> Result<(), ProxyError> {
    let mut request = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
    // The whole header value, scheme included, so that what the matcher hands back and what
    // `basic_auth` builds are interchangeable here.
    if let Some(authorization) = authorization {
        request.push_str("Proxy-Authorization: ");
        request.push_str(authorization);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .await
        .context(HandshakeIoSnafu {})?;
    stream.flush().await.context(HandshakeIoSnafu {})?;

    // One byte at a time, stopping on the blank line that ends the head: a buffered read could
    // swallow bytes belonging to the tunnel. Once per connection, over about a hundred bytes.
    let mut head = Vec::with_capacity(128);
    loop {
        let mut byte = [0u8; 1];
        if stream
            .read_exact(&mut byte)
            .await
            .context(HandshakeIoSnafu {})
            .is_err()
        {
            return TruncatedResponseSnafu {}.fail();
        }
        head.push(byte[0]);

        if head.ends_with(b"\r\n\r\n") {
            break;
        }
        if head.len() >= MAX_RESPONSE_HEAD {
            return ResponseTooLargeSnafu {
                limit: MAX_RESPONSE_HEAD,
            }
            .fail();
        }
    }

    match status_code(&head) {
        // Any 2xx switches the connection to tunnel mode, not 200 alone.
        Some(status) if (200..300).contains(&status) => Ok(()),
        Some(407) => AuthenticationRequiredSnafu {}.fail(),
        Some(status) => RejectedSnafu { status }.fail(),
        None => MalformedResponseSnafu {}.fail(),
    }
}

/// Extract the status code from an HTTP response head.
fn status_code(head: &[u8]) -> Option<u16> {
    let line = head.split(|byte| *byte == b'\n').next()?;
    let line = std::str::from_utf8(line).ok()?;
    let mut parts = line.split_whitespace();

    let version = parts.next()?;
    if !version.starts_with("HTTP/") {
        return None;
    }

    parts.next()?.parse().ok()
}

/// Split any `user:password@` prefix out of a proxy URI, returning the URI without it.
///
/// Credentials in the URL are how `HTTPS_PROXY` conventionally carries them, so they are honoured. They
/// must not stay in the URI: it is rendered in errors, in the tunnel log line, and in `ProxyConfig`'s
/// `Debug`. Percent-escapes are decoded, which is the only way to write a password containing `@`.
pub(crate) fn split_credentials(uri: Uri) -> (Uri, Option<(String, String)>) {
    let Some(authority) = uri.authority() else {
        return (uri, None);
    };
    // The last `@`, so a literal one inside the password does not split in the wrong place.
    let Some((userinfo, host)) = authority.as_str().rsplit_once('@') else {
        return (uri, None);
    };

    let (username, password) = match userinfo.split_once(':') {
        Some((username, password)) => (percent_decode(username), percent_decode(password)),
        None => (percent_decode(userinfo), String::new()),
    };

    let stripped = Uri::builder()
        .scheme(uri.scheme_str().unwrap_or("http"))
        .authority(host)
        .path_and_query(uri.path_and_query().map_or("/", |path| path.as_str()))
        .build();

    match stripped {
        Ok(stripped) if username.is_empty() && password.is_empty() => (stripped, None),
        Ok(stripped) => (stripped, Some((username, password))),
        // Unreachable: the parts come from a URI that already parsed. Keeping the original is the only
        // thing left to do, and dropping the credentials is the safer half of it.
        Err(_) => (uri, None),
    }
}

/// Render a proxy URL with any credentials elided, for a message or a log line.
///
/// Textual rather than through [`Uri`], because the values that most need eliding are the ones that
/// failed to parse in the first place.
pub(crate) fn elide_userinfo(value: &str) -> String {
    let (scheme, rest) = match value.split_once("://") {
        Some((scheme, rest)) => (Some(scheme), rest),
        None => (None, value),
    };

    // The last `@` in the whole of the rest, not just up to the first `/`. A password containing an
    // unescaped `/` is malformed, which is exactly the input that reaches this function, and looking
    // only at the authority would leave such a password rendered in full. The cost is over-eliding a
    // URL whose *path* contains an `@`, which loses a host name from a message and leaks nothing.
    let Some((_, remainder)) = rest.rsplit_once('@') else {
        return String::from(value);
    };

    match scheme {
        Some(scheme) => format!("{scheme}://***@{remainder}"),
        None => format!("***@{remainder}"),
    }
}

/// Decode `%XX` escapes, leaving anything malformed exactly as it was written.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        let decoded = if bytes[index] == b'%' && index + 2 < bytes.len() {
            std::str::from_utf8(&bytes[index + 1..index + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        } else {
            None
        };

        match decoded {
            Some(byte) => {
                out.push(byte);
                index += 3;
            }
            None => {
                out.push(bytes[index]);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// A whole `Proxy-Authorization` value for the `Basic` scheme.
///
/// The scheme is included so that this and [`Matcher`]'s own encoding are interchangeable at the call
/// site.
fn basic_auth(username: &str, password: &str) -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
    )
}

/// Failure to reach the endpoint through the proxy.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum ProxyError {
    #[snafu(display("Could not connect to the proxy {proxy} [{location}]"))]
    #[non_exhaustive]
    Connect {
        proxy: Uri,
        source: BoxError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "The proxy {proxy} did not complete the tunnel within {timeout:?} [{location}]"
    ))]
    #[non_exhaustive]
    HandshakeTimeout {
        proxy: Uri,
        timeout: Duration,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not exchange the CONNECT handshake with the proxy [{location}]"))]
    #[non_exhaustive]
    HandshakeIo {
        source: std::io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("The proxy closed the connection during the CONNECT handshake [{location}]"))]
    #[non_exhaustive]
    TruncatedResponse {
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "The proxy sent more than {limit} bytes of response head to CONNECT [{location}]"
    ))]
    #[non_exhaustive]
    ResponseTooLarge {
        limit: usize,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("The proxy sent a malformed response to CONNECT [{location}]"))]
    #[non_exhaustive]
    MalformedResponse {
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "The proxy requires authentication, and the configuration carries no credentials \
         [{location}]"
    ))]
    #[non_exhaustive]
    AuthenticationRequired {
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("The proxy refused to open the tunnel, HTTP status {status} [{location}]"))]
    #[non_exhaustive]
    Rejected {
        status: u16,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "The `CONNECT` handshake is written in the clear, so only an `http` proxy can be reached, \
         not {proxy} [{location}]"
    ))]
    #[non_exhaustive]
    UnsupportedProxy {
        proxy: Uri,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Cannot tunnel to {target}: {reason} [{location}]"))]
    #[non_exhaustive]
    UnsupportedTarget {
        target: Uri,
        reason: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_auth_encodes_user_and_password() {
        // The canonical example from RFC 7617.
        assert_eq!(
            basic_auth("Aladdin", "open sesame"),
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
    }

    #[test]
    fn status_code_is_read_from_the_first_line() {
        assert_eq!(
            status_code(b"HTTP/1.1 200 Connection established\r\n\r\n"),
            Some(200)
        );
        assert_eq!(
            status_code(b"HTTP/1.0 407 Proxy Authentication Required\r\n\r\n"),
            Some(407)
        );
        // No reason phrase is still valid.
        assert_eq!(status_code(b"HTTP/1.1 502\r\n\r\n"), Some(502));
    }

    #[test]
    fn status_code_rejects_non_http_responses() {
        assert_eq!(status_code(b"\r\n\r\n"), None);
        assert_eq!(status_code(b"NOTHTTP 200 OK\r\n\r\n"), None);
        assert_eq!(status_code(b"HTTP/1.1 nonsense\r\n\r\n"), None);
        // A raw TLS record must not be mistaken for a response.
        assert_eq!(status_code(&[0x16, 0x03, 0x01, 0x00, 0x05]), None);
    }

    #[test]
    fn target_authority_defaults_the_port_from_the_scheme() {
        let authority = |uri: &str| {
            target_authority(&Uri::try_from(uri).unwrap())
                .unwrap()
                .to_string()
        };

        assert_eq!(
            authority("https://armonik.example.com/"),
            "armonik.example.com:443"
        );
        assert_eq!(
            authority("http://armonik.example.com/"),
            "armonik.example.com:80"
        );
        assert_eq!(
            authority("https://armonik.example.com:5001/"),
            "armonik.example.com:5001"
        );
    }

    #[test]
    fn target_authority_brackets_ipv6_literals() {
        let uri = Uri::try_from("https://[::1]:5001/").unwrap();
        assert_eq!(target_authority(&uri).unwrap().to_string(), "[::1]:5001");
    }

    #[test]
    fn a_url_without_credentials_is_left_alone() {
        for value in ["http://proxy.corp:3128/", "http://proxy.corp/"] {
            let (uri, credentials) = split_credentials(Uri::try_from(value).expect("a valid URI"));
            assert_eq!(uri.to_string(), value, "{value} should be unchanged");
            assert_eq!(credentials, None);
        }
    }

    #[test]
    fn credentials_are_taken_out_of_the_url() {
        let (uri, credentials) =
            split_credentials(Uri::try_from("http://user:secret@proxy.corp:3128").expect("uri"));

        assert_eq!(uri.to_string(), "http://proxy.corp:3128/");
        assert_eq!(
            credentials,
            Some((String::from("user"), String::from("secret")))
        );
        assert!(
            !uri.to_string().contains("secret"),
            "the password must not survive in the URI, which gets logged"
        );
    }

    #[test]
    fn a_username_without_a_password_is_accepted() {
        let (uri, credentials) =
            split_credentials(Uri::try_from("http://user@proxy.corp:3128").expect("uri"));

        assert_eq!(uri.to_string(), "http://proxy.corp:3128/");
        assert_eq!(credentials, Some((String::from("user"), String::new())));
    }

    #[test]
    fn percent_escapes_in_the_userinfo_are_decoded() {
        // The only way to write a password containing `@` or `:`, so sending it encoded would send the
        // wrong password.
        let (uri, credentials) = split_credentials(
            Uri::try_from("http://a%40b:p%3Ass%40word@proxy.corp:3128").expect("uri"),
        );

        assert_eq!(uri.to_string(), "http://proxy.corp:3128/");
        assert_eq!(
            credentials,
            Some((String::from("a@b"), String::from("p:ss@word")))
        );
    }

    #[test]
    fn eliding_hides_the_credentials_and_keeps_the_rest() {
        for (written, expected) in [
            (
                "http://user:secret@proxy.corp:3128",
                "http://***@proxy.corp:3128",
            ),
            ("http://user@proxy.corp", "http://***@proxy.corp"),
            // Rejected for having no host, and still worth eliding.
            ("http://user:secret@", "http://***@"),
            ("user:secret@proxy.corp", "***@proxy.corp"),
            (
                "http://user:secret@proxy.corp/path",
                "http://***@proxy.corp/path",
            ),
            // A `/` inside the password is malformed, which is how such a value reaches this function
            // at all, and is why the split is on the last `@` rather than on the authority.
            (
                "http://user:my/pass@proxy.corp:3128",
                "http://***@proxy.corp:3128",
            ),
            // Several `@`: the last one wins, so nothing before it survives.
            ("http://user:p@ss@proxy.corp", "http://***@proxy.corp"),
        ] {
            let elided = elide_userinfo(written);
            assert_eq!(elided, expected, "{written}");
            assert!(
                !elided.contains("secret") && !elided.contains("pass"),
                "{written} still shows its password as {elided}"
            );
        }
    }

    #[test]
    fn eliding_leaves_a_url_without_credentials_untouched() {
        for value in ["http://proxy.corp:3128", "proxy.corp:3128", "", "none"] {
            assert_eq!(elide_userinfo(value), value, "{value}");
        }
    }

    #[test]
    fn a_malformed_escape_is_left_as_written() {
        // Better a password that is wrong in a visible way than one silently mangled.
        for (written, expected) in [("100%", "100%"), ("9%ZZ", "9%ZZ"), ("%2", "%2")] {
            assert_eq!(percent_decode(written), expected, "{written}");
        }
    }

    // --- an explicitly named proxy is not subject to the matcher ---

    /// A connector that records what it was asked to reach, and refuses.
    #[derive(Clone)]
    struct Recorder(std::sync::Arc<std::sync::Mutex<Vec<Uri>>>);

    impl Service<Uri> for Recorder {
        type Response = TokioIo<TcpStream>;
        type Error = BoxError;
        type Future =
            Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, target: Uri) -> Self::Future {
            self.0.lock().expect("lock").push(target);
            Box::pin(std::future::ready(Err(BoxError::from("recorder"))))
        }
    }

    #[tokio::test]
    async fn an_explicit_proxy_is_used_even_for_a_target_a_matcher_would_not_match() {
        // `Matcher::intercept` answers `None` for any scheme other than `http`/`https`, so routing an
        // explicit proxy through it would connect straight to the target instead, silently ignoring the
        // proxy the caller configured.
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let proxy = Uri::try_from("http://proxy.corp:3128").expect("a valid proxy URI");
        let mut connector = ProxyConnector::new(
            Recorder(std::sync::Arc::clone(&seen)),
            ProxyConfig::explicit(proxy.clone()),
        );

        let target = Uri::try_from("grpc://armonik.example.com:5001").expect("a valid target");
        let _ = connector.call(target.clone()).await;

        let seen = seen.lock().expect("lock");
        assert_eq!(
            seen.as_slice(),
            &[proxy],
            "the proxy should have been dialled, not {target}"
        );
    }
}
