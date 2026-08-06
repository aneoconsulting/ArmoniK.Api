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
use hyper_util::client::proxy::matcher::Matcher;
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
    /// Read the proxy from the environment, on `hyper_util`'s rules: `ALL_PROXY`, `HTTPS_PROXY`,
    /// `HTTP_PROXY` and `NO_PROXY`, in either case, with `NO_PROXY` matched as curl matches it.
    ///
    /// The default, as it is for ArmoniK's C# client: a client that asks for nothing follows the
    /// environment, and connects directly when the environment names no proxy.
    ///
    /// Read once, when `connect` builds the channel, so one that reconnects keeps the values it
    /// started with.
    #[default]
    System,
    /// Connect directly, ignoring any proxy configured in the environment.
    Disabled,
    /// Use this specific proxy, whatever the environment says: `NO_PROXY` does not apply to it.
    Explicit(Uri),
}

impl std::fmt::Debug for ProxySource {
    /// Hand written because the variant is public: a caller can put a credential-bearing URI into
    /// `Explicit` directly, and `Debug` output reaches logs, so the password half of any userinfo
    /// is redacted here rather than trusted to have been stripped.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => f.write_str("System"),
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
///
/// Deserialised from the flat `Proxy`/`ProxyUsername`/`ProxyPassword` options: empty or `system`
/// follows the environment, `none` forces a direct connection, anything else is the proxy URL.
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(try_from = "RawProxy")
)]
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
            source: ProxySource::Explicit(uri),
            username,
            password: password.into(),
        }
    }

    /// Read the proxy from the environment.
    pub fn system() -> Self {
        Self::default()
    }

    /// Connect directly, ignoring any proxy configured in the environment.
    pub fn disabled() -> Self {
        Self {
            source: ProxySource::Disabled,
            ..Self::default()
        }
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

    /// The route every connection takes when it does not depend on the target: everything but
    /// `System`, which reads the environment per connection and answers `None` here.
    ///
    /// Resolved from the fields as they are, not as a constructor left them: an `Explicit` URI is
    /// stripped of any userinfo even when it bypassed [`ProxyConfig::explicit`], so a password
    /// never reaches a log line or an error display through the URI.
    pub(crate) fn fixed_route(&self) -> Option<RouteResult> {
        match &self.source {
            ProxySource::System => None,
            ProxySource::Disabled => Some(Ok(None)),
            // Straight through, whatever the target looks like: the caller named this proxy, so
            // failing to reach it has to be an error rather than a quiet direct connection.
            ProxySource::Explicit(uri) => {
                let (uri, credentials) = split_credentials(uri.clone());
                let (username, password) = credentials.unwrap_or_default();
                Some(self.finish_route(uri, username, password, None))
            }
        }
    }

    /// The route to `target` when the environment decides, `Ok(None)` for a direct connection.
    fn env_route(&self, matcher: &Matcher, target: &Uri) -> RouteResult {
        let Some(intercept) = matcher.intercept(target) else {
            return Ok(None);
        };
        // The matcher takes credentials out of the proxy URL itself; anything it left behind (a
        // malformed URL with several `@`) is stripped here so it never renders.
        let (uri, _leftovers) = split_credentials(intercept.uri().clone());
        self.finish_route(
            uri,
            String::new(),
            String::new(),
            intercept.basic_auth().cloned(),
        )
    }

    /// The scheme rule and the credential merge, shared by every source. `Err` carries the proxy
    /// whose scheme cannot carry the cleartext `CONNECT` handshake.
    fn finish_route(
        &self,
        proxy_uri: Uri,
        url_username: String,
        url_password: String,
        ready_auth: Option<HeaderValue>,
    ) -> RouteResult {
        if proxy_uri.scheme() != Some(&Scheme::HTTP) {
            return Err(proxy_uri);
        }

        let auth = if self.username.is_empty() && self.password.expose_secret().is_empty() {
            // Nothing to merge: what the URL carried is used as is. For `system` that is the
            // header the matcher already built, sensitive flag included.
            match ready_auth {
                Some(header) => Some(header),
                None if url_username.is_empty() && url_password.is_empty() => None,
                None => Some(basic_auth_header(&url_username, &url_password)),
            }
        } else {
            // A dedicated half wins; the other half falls back to the URL's, so setting only the
            // password keeps the username the URL carried. `Basic` forbids `:` in the username
            // (RFC 7617), which is what makes decoding the matcher's header unambiguous.
            let (url_username, url_password) = match ready_auth.as_ref() {
                Some(header) => decode_basic_auth(header),
                None => (url_username, url_password),
            };
            let username = prefer_dedicated(&self.username, &url_username);
            let password = prefer_dedicated(self.password.expose_secret(), &url_password);
            Some(basic_auth_header(username, password))
        };

        Ok(Some((proxy_uri, auth)))
    }
}

/// Where a connection goes, and with which ready `Proxy-Authorization` value: `None` is a direct
/// connection. The error carries the proxy whose scheme cannot carry the cleartext handshake, so
/// whoever reports it names the offending URI once.
pub(crate) type RouteResult = Result<Option<(Uri, Option<HeaderValue>)>, Uri>;

/// The flat string options [`ProxyConfig`] is read from.
///
/// `username`/`password` share the `Proxy` prefix uniformly (`ProxyUsername`, `ProxyPassword`),
/// but `source` does not: it is `Proxy` alone, ArmoniK's own convention across every client.
/// [`serde_with::with_prefix!`] can only apply the same prefix to every field of a group, so each
/// field is renamed individually here instead.
#[cfg(feature = "serde")]
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct RawProxy {
    /// Where to find the proxy.
    ///
    /// Empty or `system` follows the environment (see [`ProxySource::System`]), `none` forces a
    /// direct connection, otherwise the proxy URL, whose scheme has to be `http`: the `CONNECT`
    /// handshake is written in the clear. `none` and `system` are the spellings every ArmoniK
    /// client understands; other casings are accepted here but not everywhere.
    #[serde(rename = "Proxy", deserialize_with = "crate::config::text")]
    source: String,
    /// Username for proxy authentication. Mutually exclusive with credentials written into the
    /// `Proxy` URL.
    #[serde(rename = "ProxyUsername", deserialize_with = "crate::config::text")]
    username: String,
    /// Password for proxy authentication. Mutually exclusive with credentials written into the
    /// `Proxy` URL. Redacted by `Debug` and zeroized on drop.
    #[serde(
        rename = "ProxyPassword",
        deserialize_with = "crate::config::secret_text"
    )]
    password: SecretString,
}

/// The two shapes proxy credentials arrive in, the same way a TLS identity has its own
/// [`crate::tls_config::IdentitySource`] shapes; the structured schema mirrors this as a `oneOf`.
///
/// A value that fits neither shape, a URL carrying credentials next to non-empty dedicated
/// fields, is refused rather than merged: guessing which half of which source wins would turn a
/// mixed configuration into a silent surprise.
#[cfg(feature = "serde")]
enum ProxyShape {
    /// A clean URL (or `system`/`none`/empty) next to dedicated credential fields.
    Fields {
        config: ProxyConfig,
        username: String,
        password: SecretString,
    },
    /// A URL that carries its credentials itself.
    Embedded { config: ProxyConfig },
}

#[cfg(feature = "serde")]
impl ProxyShape {
    /// Which shape `raw` fits, if any. `parse_proxy_source` moves any URL credentials into the
    /// config's fields, so a non-empty field there means the URL carried some.
    fn classify(raw: RawProxy) -> Result<Self, crate::config::ConfigError> {
        use crate::config::IncompatibleOptionsSnafu;

        let config = parse_proxy_source(&raw.source)?;
        let url_carried =
            !config.username.is_empty() || !config.password.expose_secret().is_empty();
        let dedicated = !raw.username.is_empty() || !raw.password.expose_secret().is_empty();
        match (url_carried, dedicated) {
            (true, true) => IncompatibleOptionsSnafu {
                msg: String::from(
                    "credentials are set both inside the `Proxy` URL and through \
                     `ProxyUsername`/`ProxyPassword`; set them one way",
                ),
            }
            .fail(),
            (true, false) => Ok(Self::Embedded { config }),
            (false, _) => Ok(Self::Fields {
                config,
                username: raw.username,
                password: raw.password,
            }),
        }
    }

    /// The configuration either shape describes.
    fn resolve(self) -> ProxyConfig {
        match self {
            Self::Embedded { config } => config,
            Self::Fields {
                config,
                username,
                password,
            } => config.with_credentials(username, password),
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<RawProxy> for ProxyConfig {
    type Error = crate::config::ConfigError;

    fn try_from(raw: RawProxy) -> Result<Self, Self::Error> {
        ProxyShape::classify(raw).map(ProxyShape::resolve)
    }
}

/// Interpret the `Proxy` value.
///
/// Empty and `system` follow the environment, `none` forces a direct connection, anything else is a
/// proxy URL, defaulting to the `http` scheme. A URL that cannot be dialled as written, a missing
/// host, a port that is not one, or a scheme the `CONNECT` handshake cannot use, fails here, while
/// the configuration is being read and can name itself, rather than at connect time.
#[cfg(feature = "serde")]
fn parse_proxy_source(proxy: &str) -> Result<ProxyConfig, crate::config::ConfigError> {
    use crate::config::IncompatibleOptionsSnafu;
    use snafu::ensure;

    if proxy.is_empty() || proxy.eq_ignore_ascii_case("system") {
        return Ok(ProxyConfig::system());
    }
    if proxy.eq_ignore_ascii_case("none") {
        return Ok(ProxyConfig::disabled());
    }

    let with_scheme = if proxy.contains("://") {
        proxy.to_owned()
    } else {
        format!("http://{proxy}")
    };
    // Not reported through the endpoint's URI error: its message names the endpoint, which would
    // send whoever reads it looking at the wrong option.
    let uri = Uri::try_from(&with_scheme).ok().filter(|uri| {
        uri.authority()
            .is_some_and(|authority| !authority.host().is_empty())
    });
    let Some(uri) = uri else {
        return IncompatibleOptionsSnafu {
            // Elided: a URL rejected for having no host can still have carried a password.
            msg: format!(
                "`Proxy={}` is not a valid proxy URL. Expected `none`, \
                 `system`, or a URL such as `http://proxy.example.com:3128`",
                elide_userinfo(proxy)
            ),
        }
        .fail();
    };

    ensure!(
        uri.scheme_str().is_none_or(|scheme| scheme == "http"),
        IncompatibleOptionsSnafu {
            msg: format!(
                "{}, and `Proxy={}` names another scheme",
                HTTP_ONLY_RULE,
                elide_userinfo(proxy)
            ),
        }
    );

    let config = ProxyConfig::explicit(uri);
    let ProxySource::Explicit(stripped) = &config.source else {
        return IncompatibleOptionsSnafu {
            msg: String::from("`Proxy`: an explicit proxy should stay explicit"),
        }
        .fail();
    };

    // `http::Uri` keeps the port as text and parses it lazily, so `proxy.corp:99999` or
    // `proxy.corp:31z8` would otherwise be accepted here and silently dialled on port 80. Checked
    // on the credential-stripped form, where the only `:` left outside an IPv6 literal is a port's.
    let authority = stripped.authority().expect("checked above to have a host");
    let written_port = authority
        .as_str()
        .rsplit_once(':')
        .map(|(_, tail)| tail)
        .filter(|tail| !tail.contains(']'));
    ensure!(
        written_port.is_none() || stripped.port_u16().is_some(),
        IncompatibleOptionsSnafu {
            msg: format!(
                "`Proxy={}` does not name a valid port",
                elide_userinfo(proxy)
            ),
        }
    );

    Ok(config)
}

/// Wraps a TCP connector so that it tunnels through an HTTP proxy when one is configured.
///
/// A request that must not be proxied goes straight to the inner connector, leaving that path as it
/// was. Generic over the connector because all this layer needs is something that turns a [`Uri`]
/// into a TCP stream; which one is knowledge it has no use for.
#[derive(Debug, Clone)]
pub struct ProxyConnector<S> {
    inner: S,
    proxy: ProxyConfig,
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
    /// Resolve per connection: the environment's rules, read once here so a channel that
    /// reconnects keeps the values it started with. Behind an `Arc` because a `tower` connector is
    /// cloned per connection and `Matcher` is not `Clone`.
    PerCall(std::sync::Arc<Matcher>),
}

impl<S> ProxyConnector<S> {
    /// Wrap a TCP connector with the given proxy configuration.
    ///
    /// `connect_timeout` bounds the tunnel handshake, so the knob that governs how long connecting
    /// may take governs the proxied path too; without one, a 30-second default keeps a proxy that
    /// accepts the connection and then goes quiet from hanging the client.
    pub(crate) fn new(inner: S, proxy: ProxyConfig, connect_timeout: Option<Duration>) -> Self {
        let prepared = match proxy.fixed_route() {
            None => Prepared::PerCall(std::sync::Arc::new(Matcher::from_env())),
            Some(Ok(None)) => Prepared::Direct,
            Some(Ok(Some((uri, auth)))) => Prepared::Via { proxy: uri, auth },
            Some(Err(unsupported)) => Prepared::Unsupported(unsupported),
        };
        Self {
            inner,
            proxy,
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
        let route = match &self.prepared {
            Prepared::Direct => Ok(None),
            Prepared::Via { proxy, auth } => Ok(Some((proxy.clone(), auth.clone()))),
            Prepared::Unsupported(proxy) => Err(proxy.clone()),
            Prepared::PerCall(matcher) => self.proxy.env_route(matcher, &target),
        };
        let route = match route {
            Ok(route) => route,
            Err(proxy) => {
                let error = UnsupportedProxySnafu { proxy }.build();
                return Box::pin(std::future::ready(Err(error.into())));
            }
        };

        // Nothing to tunnel through: keep the original behaviour untouched.
        let Some((proxy_uri, auth)) = route else {
            let future = self.inner.call(target);
            return Box::pin(async move { future.await.map_err(Into::into) });
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

/// The one statement of the scheme rule, shared between the connect-time check and the
/// configuration-time one so the two messages cannot drift apart.
pub(crate) const HTTP_ONLY_RULE: &str =
    "The `CONNECT` handshake is written in the clear, so only an `http` proxy can be reached";

/// Render a proxy URL with any credentials elided, for a message or a log line.
///
/// Textual rather than through [`Uri`], because the values that most need eliding are the ones that
/// failed to parse in the first place. Every ambiguity elides rather than shows: over-eliding loses
/// a detail from a message, under-eliding prints a password. Gated with the code that renders
/// proxy values: only the string-form reading does.
#[cfg(any(feature = "serde", test))]
pub(crate) fn elide_userinfo(value: &str) -> String {
    let (scheme, rest) = match value.split_once("://") {
        Some((scheme, rest)) => (Some(scheme), rest),
        None => (None, value),
    };
    let elided = |body: String| match scheme {
        Some(scheme) => format!("{scheme}://{body}"),
        None => body,
    };

    // The last `@` in the whole of the rest, not just up to the first `/`. A password containing an
    // unescaped `/` is malformed, which is exactly the input that reaches this function, and looking
    // only at the authority would leave such a password rendered in full.
    if let Some((_, remainder)) = rest.rsplit_once('@') {
        return elided(format!("***@{remainder}"));
    }

    // No `@`, but a `:` whose tail is not a port: userinfo written without a host
    // (`http://admin:hunter2`), and the tail is the password. An IPv6 literal also lands here and
    // is over-elided; nothing leaks.
    if let Some((head, tail)) = rest.split_once(':') {
        if !tail.is_empty() && tail.parse::<u16>().is_err() {
            return elided(format!("{head}:***"));
        }
    }

    String::from(value)
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
    #[snafu(display(
        "The proxy requires authentication; set the `ProxyUsername` and `ProxyPassword` options \
         [{location}]"
    ))]
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
    #[snafu(display("{HTTP_ONLY_RULE}, not {proxy} [{location}]"))]
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

/// The username and password a `Basic` `Proxy-Authorization` value carries.
///
/// The inverse of [`basic_auth_header`], needed only when a dedicated credential half has to merge
/// with what a proxy URL carried. The split is at the first `:`, which RFC 7617 makes unambiguous
/// by forbidding `:` in the username. Malformed input, which `hyper_util` itself never produces,
/// decodes as an empty pair rather than panicking: worst case a proxy that needed credentials
/// rejects the tunnel, which is what an absent value already does.
fn decode_basic_auth(value: &HeaderValue) -> (String, String) {
    let encoded = value
        .to_str()
        .unwrap_or_default()
        .trim_start_matches("Basic ");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap_or_default();
    let decoded = String::from_utf8_lossy(&decoded).into_owned();
    match decoded.split_once(':') {
        Some((user, password)) => (user.to_owned(), password.to_owned()),
        None => (decoded, String::new()),
    }
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
    fn the_default_source_follows_the_environment() {
        // The same default as ArmoniK's C# client: unset means the environment decides.
        assert_eq!(ProxySource::default(), ProxySource::System);
        assert_eq!(ProxyConfig::default().source, ProxySource::System);
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
            // Credentials with the `@host` part missing entirely: the tail is not a port, so it is
            // a password.
            ("http://admin:hunter2", "http://admin:***"),
            ("admin:hunter2", "admin:***"),
        ] {
            let elided = elide_userinfo(written);
            assert_eq!(elided, expected, "{written}");
            assert!(
                !elided.contains("secret")
                    && !elided.contains("pass")
                    && !elided.contains("hunter"),
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
        assert_eq!(ProxyConfig::disabled().fixed_route(), Some(Ok(None)));
    }

    #[test]
    fn system_resolves_per_connection_rather_than_at_construction() {
        assert_eq!(ProxyConfig::system().fixed_route(), None);
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
            .expect("a source fixed at construction")
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
            Some(Err(uri)),
            "the route should carry the refused proxy back"
        );
    }

    #[test]
    fn no_credentials_means_no_authorization_header() {
        let config = ProxyConfig::explicit(Uri::try_from("http://proxy.corp:3128").expect("uri"));
        let (_, auth) = config
            .fixed_route()
            .expect("a source fixed at construction")
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
