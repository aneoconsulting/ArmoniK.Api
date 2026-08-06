//! Reaching the endpoint through an HTTP proxy.
//!
//! A `CONNECT` tunnel rather than an absolute-form request, so TLS stays end to end with the real
//! server: [`ProxyConnector`] sits below the TLS connector and hands back the stream a direct
//! connection would. The handshake itself is `hyper_util`'s own [`Tunnel`], not one written here.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use base64::Engine;
use hyper::http::uri::Scheme;
use hyper::http::HeaderValue;
use hyper::Uri;
use hyper_util::client::legacy::connect::proxy::Tunnel;
use hyper_util::rt::TokioIo;
use secrecy::{ExposeSecret, SecretString};
use snafu::{IntoError, Snafu};
use tokio::net::TcpStream;
use tower_service::Service;

/// What a connector reports when it fails, which every layer here has to be able to carry.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Upper bound on the tunnel handshake when no connect timeout is configured, so a proxy that
/// accepts the connection and then goes quiet fails instead of hanging. `Tunnel` has no timeout of
/// its own.
const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// Where to find the HTTP proxy used to reach the endpoint.
#[derive(Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProxySource {
    /// Connect directly.
    ///
    /// The default: a client that asks for nothing connects directly.
    #[default]
    Disabled,
    /// Use this specific proxy.
    Explicit(Uri),
}

impl std::fmt::Debug for ProxySource {
    /// Hand written because the variant is public: a caller can put a credential-bearing URI into
    /// `Explicit` directly, and `Debug` output reaches logs, so the password half of any userinfo
    /// is redacted here rather than trusted to have been stripped.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => f.write_str("Disabled"),
            Self::Explicit(uri) => f
                .debug_tuple("Explicit")
                .field(&format_args!("{}", RedactedUri(uri)))
                .finish(),
        }
    }
}

/// Renders a URI with the password half of any userinfo replaced by `<redacted>`.
///
/// Purely textual, so it cannot fail and holds even for authorities that would not survive a
/// round trip through the URI builder.
struct RedactedUri<'a>(&'a Uri);

impl std::fmt::Display for RedactedUri<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uri = self.0;
        // The last `@`, because a password may contain a literal one.
        let userinfo = uri
            .authority()
            .and_then(|authority| authority.as_str().rsplit_once('@'));
        let Some((userinfo, host)) = userinfo else {
            return write!(f, "{uri}");
        };
        // A userinfo without `:` carries no password, and a username is not a secret.
        let Some((username, _password)) = userinfo.split_once(':') else {
            return write!(f, "{uri}");
        };

        if let Some(scheme) = uri.scheme_str() {
            write!(f, "{scheme}://")?;
        }
        let path = uri.path_and_query().map_or("", |path| path.as_str());
        write!(f, "{username}:<redacted>@{host}{path}")
    }
}

/// Configuration of the HTTP proxy used to reach the endpoint.
///
/// Proxying uses a `CONNECT` tunnel, so TLS, mutual TLS included, is negotiated end to end with the
/// real server and the proxy never sees the plaintext.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ProxyConfig {
    /// Where to find the proxy.
    pub source: ProxySource,
    /// Username for proxy authentication, empty for none.
    pub username: String,
    /// Password for proxy authentication, empty for none. Redacted by `Debug` and zeroized on
    /// drop.
    pub password: SecretString,
}

impl ProxyConfig {
    /// Use this specific proxy.
    ///
    /// Credentials written into the URL are taken out of it and kept in the dedicated fields, so
    /// the URI carries none wherever it is rendered. A URI installed into `source` by hand gets the
    /// same treatment when the connection is made; this constructor only does it earlier, where a
    /// caller can still read the parts back.
    pub fn explicit(uri: Uri) -> Self {
        let (uri, credentials) = split_credentials(uri);
        let (username, password) = credentials.unwrap_or_default();
        Self {
            // On a URI whose sanitized form cannot be rebuilt, the placeholder goes in: the
            // credentials stay in the fields, and the route refuses the placeholder, so the bad
            // URL surfaces as an error that cannot name the password.
            source: ProxySource::Explicit(uri.unwrap_or_else(|elided| elided)),
            username,
            password: password.into(),
        }
    }

    /// Connect directly.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Attach credentials for proxy authentication.
    ///
    /// A half left empty keeps what the proxy URL itself carried: setting only the password keeps
    /// the username the URL named, so the pair does not degrade into an unexplained 407. To clear
    /// a credential, assign the field directly.
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<SecretString>,
    ) -> Self {
        let username = username.into();
        if !username.is_empty() {
            self.username = username;
        }
        let password = password.into();
        if !password.expose_secret().is_empty() {
            self.password = password;
        }
        self
    }

    /// The route every connection through this configuration takes.
    ///
    /// Resolved from the fields as they are, not as a constructor left them: an `Explicit` URI is
    /// stripped of any userinfo even when it bypassed [`ProxyConfig::explicit`], so a password
    /// never reaches a log line or an error display through the URI.
    pub(crate) fn fixed_route(&self) -> RouteResult {
        match &self.source {
            ProxySource::Disabled => Ok(None),
            // Straight through, whatever the target looks like: the caller named this proxy, so
            // failing to reach it has to be an error rather than a quiet direct connection.
            ProxySource::Explicit(uri) => {
                let (uri, credentials) = split_credentials(uri.clone());
                let (username, password) = credentials.unwrap_or_default();
                match uri {
                    Ok(uri) => self.finish_route(uri, username, password),
                    // No sanitized form exists, and the original must not come back: it is the
                    // one URI that still holds the password.
                    Err(elided) => Err(elided),
                }
            }
        }
    }

    /// The scheme rule and the credential merge, shared by every source. `Err` carries the proxy
    /// whose scheme cannot carry the cleartext `CONNECT` handshake.
    fn finish_route(
        &self,
        proxy_uri: Uri,
        url_username: String,
        url_password: String,
    ) -> RouteResult {
        if proxy_uri.scheme() != Some(&Scheme::HTTP) {
            return Err(proxy_uri);
        }

        // A dedicated half wins; the other half falls back to the URL's, so setting only the
        // password keeps the username the URL carried.
        let username = prefer_dedicated(&self.username, &url_username);
        let password = prefer_dedicated(self.password.expose_secret(), &url_password);
        let auth = (!username.is_empty() || !password.is_empty())
            .then(|| basic_auth_header(username, password));

        Ok(Some((proxy_uri, auth)))
    }
}

/// Where a connection goes, and with which ready `Proxy-Authorization` value: `None` is a direct
/// connection. The error carries the proxy that cannot be routed through - a scheme that cannot
/// carry the cleartext handshake, or the placeholder for a URI with no sanitized form - so
/// whoever reports it names the offending URI once, and that URI never holds a password.
pub(crate) type RouteResult = Result<Option<(Uri, Option<HeaderValue>)>, Uri>;

/// Wraps a TCP connector so that it tunnels through an HTTP proxy when one is configured.
///
/// A request that must not be proxied goes straight to the inner connector, leaving that path as it
/// was. Generic over the connector because all this layer needs is something that turns a [`Uri`]
/// into a TCP stream; which one is knowledge it has no use for.
#[derive(Debug, Clone)]
pub struct ProxyConnector<S> {
    inner: S,
    prepared: Prepared,
    /// Upper bound on the tunnel handshake alone; the dial to the proxy is the inner connector's,
    /// bounded by its own connect timeout.
    handshake_timeout: Duration,
}

/// What [`ProxyConnector::new`] resolves once, so a connection pays no route work when the route
/// cannot change.
#[derive(Debug, Clone)]
enum Prepared {
    /// Connect directly.
    Direct,
    /// Tunnel through this proxy, presenting this ready `Proxy-Authorization` value.
    Via {
        proxy: Uri,
        auth: Option<HeaderValue>,
    },
    /// Refuse every connection: this proxy's scheme cannot carry the cleartext handshake.
    Unsupported(Uri),
}

impl<S> ProxyConnector<S> {
    /// Wrap a TCP connector with the given proxy configuration.
    ///
    /// `connect_timeout` bounds the tunnel handshake, so the knob that governs how long connecting
    /// may take governs the proxied path too; without one, a 30-second default keeps a proxy that
    /// accepts the connection and then goes quiet from hanging the client.
    pub(crate) fn new(inner: S, proxy: ProxyConfig, connect_timeout: Option<Duration>) -> Self {
        let prepared = match proxy.fixed_route() {
            Ok(None) => Prepared::Direct,
            Ok(Some((uri, auth))) => Prepared::Via { proxy: uri, auth },
            Err(unsupported) => Prepared::Unsupported(unsupported),
        };
        Self {
            inner,
            prepared,
            handshake_timeout: connect_timeout.unwrap_or(DEFAULT_HANDSHAKE_TIMEOUT),
        }
    }
}

impl<S> Service<Uri> for ProxyConnector<S>
where
    S: Service<Uri, Response = TokioIo<TcpStream>> + Clone + Send + 'static,
    S::Error: Into<BoxError> + Send + Sync + 'static,
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
        let (proxy_uri, auth) = match &self.prepared {
            // Nothing to tunnel through: keep the original behaviour untouched.
            Prepared::Direct => {
                let future = self.inner.call(target);
                return Box::pin(async move { future.await.map_err(Into::into) });
            }
            Prepared::Via { proxy, auth } => (proxy.clone(), auth.clone()),
            Prepared::Unsupported(proxy) => {
                let error = UnsupportedProxySnafu {
                    proxy: proxy.clone(),
                }
                .build();
                return Box::pin(std::future::ready(Err(error.into())));
            }
        };

        // Dial the proxy with the inner connector, so a failure to reach it is classified by
        // construction rather than by matching upstream error text; `Tunnel` then handshakes over
        // the stream it is handed and only the handshake is upstream's.
        let connect = self.inner.call(proxy_uri.clone());
        let timeout = self.handshake_timeout;
        Box::pin(async move {
            let stream = connect.await.map_err(|error| {
                ConnectSnafu {
                    proxy: proxy_uri.clone(),
                }
                .into_error(error.into())
            })?;

            let mut tunnel = Tunnel::new(proxy_uri.clone(), Connected(Some(stream)));
            if let Some(auth) = auth {
                tunnel = tunnel.with_auth(auth);
            }

            let handshake = tunnel.call(target.clone());
            let stream = tokio::time::timeout(timeout, handshake)
                .await
                .map_err(|_| {
                    HandshakeTimeoutSnafu {
                        proxy: proxy_uri.clone(),
                        timeout,
                    }
                    .build()
                })?
                .map_err(|error| translate(proxy_uri.clone(), error))?;

            tracing::debug!(proxy = %proxy_uri, %target, "Established proxy tunnel");

            Ok(stream)
        })
    }
}

/// A connector that hands out one stream, already connected.
///
/// What [`Tunnel`] dials through: the proxy was dialled by the real connector before the tunnel
/// was built, so the errors of reaching it and of handshaking over it stay distinguishable without
/// looking at either one's text.
struct Connected(Option<TokioIo<TcpStream>>);

impl Service<Uri> for Connected {
    type Response = TokioIo<TcpStream>;
    type Error = BoxError;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _proxy: Uri) -> Self::Future {
        std::future::ready(
            self.0
                .take()
                .ok_or_else(|| BoxError::from("the tunnel opens once per connection")),
        )
    }
}

/// Turn what `Tunnel` reports into an error naming the proxy.
///
/// The proxy was dialled before the tunnel was built, so everything arriving here is the handshake
/// itself. One case still gets a better message than "did not open the tunnel", detected by text:
/// `hyper_util`'s `TunnelError` lives in a private module of that crate with no public path, only
/// trait bounds reach it, so its 407 variant cannot be matched. Brittle in principle; pinned by an
/// integration test, so a wording change upstream breaks loudly instead of silently losing the
/// hint.
fn translate(proxy: Uri, error: impl std::error::Error + Send + Sync + 'static) -> ProxyError {
    if error.to_string().contains("proxy authorization required") {
        return AuthenticationRequiredSnafu {}.build();
    }
    TunnelFailedSnafu { proxy }.into_error(BoxError::from(error))
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
    #[snafu(display("The proxy requires authentication; configure proxy credentials [{location}]"))]
    #[non_exhaustive]
    AuthenticationRequired {
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("The proxy {proxy} did not open the tunnel [{location}]"))]
    #[non_exhaustive]
    TunnelFailed {
        proxy: Uri,
        source: BoxError,
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
}

impl ProxyError {
    /// The `ProxyError` buried in `error`'s source chain, if any.
    ///
    /// [`crate::connect`] returns the transport's own error type, which carries this one as an
    /// anonymous boxed cause. This walk is how a caller reacts to a proxy failure in particular,
    /// e.g. prompting for credentials on [`ProxyError::AuthenticationRequired`], without matching
    /// on rendered text.
    pub fn find_in<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<&'a ProxyError> {
        let mut current = Some(error);
        while let Some(error) = current {
            if let Some(proxy) = error.downcast_ref::<ProxyError>() {
                return Some(proxy);
            }
            current = error.source();
        }
        None
    }
}

/// A whole `Proxy-Authorization` value for the `Basic` scheme, marked sensitive so it is never
/// logged.
///
/// The output of base64 encoding is always valid header-value bytes, so building the `HeaderValue`
/// cannot fail on any input this crate constructs it from.
fn basic_auth_header(username: &str, password: &str) -> HeaderValue {
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    let mut value = HeaderValue::from_str(&format!("Basic {encoded}"))
        .expect("base64 output is always a valid header value");
    value.set_sensitive(true);
    value
}

/// Prefer `dedicated` unless it is empty, in which case fall back to `from_url`.
///
/// A dedicated username or password set alone must not discard the other half of whatever the
/// proxy URL carried, or the request fails as an unexplained 407.
fn prefer_dedicated<'a>(dedicated: &'a str, from_url: &'a str) -> &'a str {
    if dedicated.is_empty() {
        from_url
    } else {
        dedicated
    }
}

/// Split any `user:password@` prefix out of a proxy URI, returning the URI without it.
///
/// Credentials in the URL are how `HTTPS_PROXY` conventionally carries them, so they are honoured. They
/// must not stay in the URI: it is rendered in errors, in log lines, and in `ProxyConfig`'s
/// `Debug`. Percent-escapes are decoded, which is the only way to write a password containing `@`.
///
/// The split is textual, so it always happens; only the URI half can fail. `Err` means the
/// post-`@` text is not an authority on its own (`@` is legal inside a bracketed authority, so
/// `http://[user:password@proxy]` parses yet `proxy]` does not rebuild) and carries
/// [`elided_proxy`], never anything from the input: the input is the one URI that still holds the
/// password.
pub(crate) fn split_credentials(uri: Uri) -> (Result<Uri, Uri>, Option<(String, String)>) {
    let Some(authority) = uri.authority() else {
        return (Ok(uri), None);
    };
    // The last `@`, so a literal one inside the password does not split in the wrong place.
    let Some((userinfo, host)) = authority.as_str().rsplit_once('@') else {
        return (Ok(uri), None);
    };

    let (username, password) = match userinfo.split_once(':') {
        Some((username, password)) => (percent_decode(username), percent_decode(password)),
        None => (percent_decode(userinfo), String::new()),
    };
    let credentials =
        (!username.is_empty() || !password.is_empty()).then_some((username, password));

    let stripped = Uri::builder()
        .scheme(uri.scheme_str().unwrap_or("http"))
        .authority(host)
        .path_and_query(uri.path_and_query().map_or("/", |path| path.as_str()))
        .build();

    (stripped.map_err(|_| elided_proxy()), credentials)
}

/// Stands in for a proxy URI whose sanitized form cannot be rebuilt.
///
/// Authority-form on purpose: carrying no scheme means `finish_route` refuses it, so a proxy URL
/// that cannot be sanitized surfaces as a route error naming this placeholder, wherever the URI
/// came from.
fn elided_proxy() -> Uri {
    Uri::from_static("unrepresentable-proxy.invalid")
}

/// Decode `%XX` escapes, leaving anything malformed exactly as it was written, which is how
/// `percent_encoding` behaves for a `%` not followed by two hex digits.
fn percent_decode(value: &str) -> String {
    percent_encoding::percent_decode_str(value)
        .decode_utf8_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_direct_connection() {
        assert_eq!(ProxySource::default(), ProxySource::Disabled);
        assert_eq!(ProxyConfig::default().source, ProxySource::Disabled);
    }

    #[test]
    fn a_url_without_credentials_is_left_alone() {
        for value in ["http://proxy.corp:3128/", "http://proxy.corp/"] {
            let (uri, credentials) = split_credentials(Uri::try_from(value).expect("a valid URI"));
            let uri = uri.expect("nothing to strip");
            assert_eq!(uri.to_string(), value, "{value} should be unchanged");
            assert_eq!(credentials, None);
        }
    }

    #[test]
    fn credentials_are_taken_out_of_the_url() {
        let config = ProxyConfig::explicit(
            Uri::try_from("http://user:secret@proxy.corp:3128").expect("uri"),
        );

        assert_eq!(config.username, "user");
        assert_eq!(config.password.expose_secret(), "secret");
        let ProxySource::Explicit(uri) = &config.source else {
            panic!("expected an explicit proxy");
        };
        assert!(
            !uri.to_string().contains("secret"),
            "the password must not survive in the URI, which gets logged: {uri}"
        );
    }

    #[test]
    fn a_username_without_a_password_is_accepted() {
        let (uri, credentials) =
            split_credentials(Uri::try_from("http://user@proxy.corp:3128").expect("uri"));

        assert_eq!(
            uri.expect("rebuildable").to_string(),
            "http://proxy.corp:3128/"
        );
        assert_eq!(credentials, Some((String::from("user"), String::new())));
    }

    #[test]
    fn percent_escapes_in_the_userinfo_are_decoded() {
        // The only way to write a password containing `@` or `:`, so sending it encoded would send the
        // wrong password.
        let (uri, credentials) = split_credentials(
            Uri::try_from("http://a%40b:p%3Ass%40word@proxy.corp:3128").expect("uri"),
        );

        assert_eq!(
            uri.expect("rebuildable").to_string(),
            "http://proxy.corp:3128/"
        );
        assert_eq!(
            credentials,
            Some((String::from("a@b"), String::from("p:ss@word")))
        );
    }

    #[test]
    fn a_uri_with_no_sanitized_form_is_still_split() {
        // `@` is legal inside a bracketed authority, so this parses, yet the post-`@` text
        // `proxy]` is not an authority on its own and the stripped URI cannot be rebuilt. The
        // split still happens: handing back the original would keep the password in a URI that
        // errors and logs render.
        let (uri, credentials) =
            split_credentials(Uri::try_from("http://[user:s3cr3t@proxy]").expect("uri"));

        let elided = uri.expect_err("`proxy]` alone is not a valid authority");
        assert!(!elided.to_string().contains("s3cr3t"), "leaked: {elided}");
        assert_eq!(
            credentials,
            Some((String::from("[user"), String::from("s3cr3t")))
        );
    }

    #[test]
    fn a_malformed_escape_is_left_as_written() {
        // Better a password that is wrong in a visible way than one silently mangled.
        for (written, expected) in [("100%", "100%"), ("9%ZZ", "9%ZZ"), ("%2", "%2")] {
            assert_eq!(percent_decode(written), expected, "{written}");
        }
    }

    #[test]
    fn basic_auth_header_encodes_user_and_password() {
        // The canonical example from RFC 7617.
        let value = basic_auth_header("Aladdin", "open sesame");
        assert_eq!(value, "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==");
        assert!(value.is_sensitive(), "must not be logged");
    }

    #[test]
    fn the_password_is_kept_out_of_the_debug_output() {
        let config = ProxyConfig::explicit(Uri::try_from("http://proxy.corp:3128").expect("uri"))
            .with_credentials("user", "s3cr3t");

        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("s3cr3t"),
            "password rendered: {rendered}"
        );
        assert!(
            rendered.contains("user"),
            "the username is not a secret and stays useful: {rendered}"
        );
    }

    #[test]
    fn a_credentialed_uri_installed_by_hand_is_redacted_in_debug_output() {
        // `#[non_exhaustive]` does not prevent building the variant directly, so `Debug` cannot
        // assume the constructor's stripping ran before the value is logged.
        let source = ProxySource::Explicit(
            Uri::try_from("http://user:s3cr3t@proxy.corp:3128").expect("uri"),
        );

        let rendered = format!("{source:?}");
        assert!(
            !rendered.contains("s3cr3t"),
            "password rendered: {rendered}"
        );
        assert!(
            rendered.contains("user") && rendered.contains("proxy.corp:3128"),
            "the username and the host are not secrets and stay useful: {rendered}"
        );
    }

    #[test]
    fn a_uri_without_a_password_is_rendered_verbatim_in_debug_output() {
        for value in ["http://proxy.corp:3128/", "http://user@proxy.corp:3128/"] {
            let source = ProxySource::Explicit(Uri::try_from(value).expect("uri"));
            let rendered = format!("{source:?}");
            assert!(
                rendered.contains(value),
                "{value} carries no password and should be rendered as written: {rendered}"
            );
        }
    }

    #[test]
    fn a_password_set_alone_keeps_the_username_the_url_carried() {
        // Replacing the pair rather than each half would leave an empty username here, and the proxy
        // would answer 407 with nothing to explain it.
        let config = ProxyConfig::explicit(
            Uri::try_from("http://url-user:url-secret@proxy.corp:3128").expect("uri"),
        )
        .with_credentials("", "option-secret");

        assert_eq!(config.username, "url-user");
        assert_eq!(config.password.expose_secret(), "option-secret");
    }

    // --- routing ---

    #[test]
    fn disabled_routes_directly() {
        assert_eq!(ProxyConfig::disabled().fixed_route(), Ok(None));
    }

    #[test]
    fn a_credentialed_uri_installed_by_hand_is_stripped_and_still_authenticates() {
        // `#[non_exhaustive]` does not prevent building the variant directly, so the constructor's
        // stripping cannot be assumed to have run. The route is where the URI is used, so it is
        // where the guarantee has to hold: no password in the URI it returns, and the credentials
        // honoured rather than dropped.
        let config = ProxyConfig {
            source: ProxySource::Explicit(
                Uri::try_from("http://user:s3cr3t@proxy.corp:3128").expect("uri"),
            ),
            ..ProxyConfig::default()
        };

        let (uri, auth) = config
            .fixed_route()
            .expect("routing should succeed")
            .expect("an explicit proxy routes through it");

        assert!(!uri.to_string().contains("s3cr3t"), "leaked: {uri}");
        assert_eq!(auth, Some(basic_auth_header("user", "s3cr3t")));
    }

    #[test]
    fn a_proxy_that_is_not_http_is_refused() {
        // The `CONNECT` handshake goes out unencrypted, so a proxy expecting TLS would see gibberish.
        let uri = Uri::try_from("https://proxy.corp:3128").expect("uri");
        let config = ProxyConfig::explicit(uri.clone());

        assert_eq!(
            config.fixed_route(),
            Err(uri),
            "the route should carry the refused proxy back"
        );
    }

    #[test]
    fn a_uri_with_no_sanitized_form_becomes_a_route_error_without_the_password() {
        // Whether the URI went through the constructor or straight into the field, no output may
        // carry the password: the fields keep the credentials, `Debug` stays clean, and the route
        // error names the placeholder instead of the original.
        let constructed =
            ProxyConfig::explicit(Uri::try_from("http://[user:s3cr3t@proxy]").expect("uri"));
        let by_hand = ProxyConfig {
            source: ProxySource::Explicit(
                Uri::try_from("http://[user:s3cr3t@proxy]").expect("uri"),
            ),
            ..ProxyConfig::default()
        };

        assert_eq!(constructed.username, "[user");
        assert_eq!(constructed.password.expose_secret(), "s3cr3t");

        for config in [constructed, by_hand] {
            let rendered = format!("{config:?}");
            assert!(
                !rendered.contains("s3cr3t"),
                "password rendered: {rendered}"
            );
            let refused = config
                .fixed_route()
                .expect_err("nothing sane to route through");
            assert!(!refused.to_string().contains("s3cr3t"), "leaked: {refused}");
        }
    }

    #[test]
    fn no_credentials_means_no_authorization_header() {
        let config = ProxyConfig::explicit(Uri::try_from("http://proxy.corp:3128").expect("uri"));
        let (_, auth) = config
            .fixed_route()
            .expect("routing should succeed")
            .expect("an explicit proxy routes through it");
        assert_eq!(auth, None);
    }

    // --- the connector dials the proxy, not the target ---

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
    async fn an_explicit_proxy_is_dialled_whatever_the_target_looks_like() {
        // The caller named this proxy, so it is dialled even for a target scheme no environment
        // convention would proxy; connecting straight to the target instead would silently ignore
        // the configuration.
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let proxy = Uri::try_from("http://proxy.corp:3128").expect("a valid proxy URI");
        let mut connector = ProxyConnector::new(
            Recorder(std::sync::Arc::clone(&seen)),
            ProxyConfig::explicit(proxy.clone()),
            None,
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
