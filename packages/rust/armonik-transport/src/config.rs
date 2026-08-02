use std::time::Duration;

use hyper::{http::HeaderValue, Uri};
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use snafu::{ResultExt, Snafu};

use crate::secret::Secret;

/// Where to find the HTTP proxy used to reach the endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProxySource {
    /// Connect directly, ignoring any proxy configured in the environment.
    ///
    /// The default: a client that asks for nothing connects directly.
    #[default]
    Disabled,
    /// Read the proxy from the environment, on `hyper_util`'s rules: `ALL_PROXY`, `HTTPS_PROXY`,
    /// `HTTP_PROXY` and `NO_PROXY`, in either case, with `NO_PROXY` matched as curl matches it.
    ///
    /// Read once, when `connect` builds the channel, so one that reconnects keeps the values it
    /// started with. This is the one value this crate goes looking for; every other is handed to it.
    System,
    /// Use this specific proxy.
    Explicit(Uri),
}

/// Configuration of the HTTP proxy used to reach the endpoint.
///
/// Proxying uses a `CONNECT` tunnel, so TLS, mutual TLS included, is negotiated end to end with the
/// real server and the proxy never sees the plaintext.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProxyConfig {
    /// Where to find the proxy.
    pub source: ProxySource,
    /// Username for proxy authentication, empty for none.
    pub username: String,
    /// Password for proxy authentication, empty for none.
    pub password: Secret,
}

impl ProxyConfig {
    /// Use this specific proxy.
    ///
    /// Credentials written into the URL are taken out of it and kept here, so the URI carries none
    /// wherever it is rendered. The type is `#[non_exhaustive]`, so this is the way in.
    pub fn explicit(uri: Uri) -> Self {
        let (uri, credentials) = crate::proxy::split_credentials(uri);
        let (username, password) = credentials.unwrap_or_default();
        Self {
            source: ProxySource::Explicit(uri),
            username,
            password: password.into(),
        }
    }

    /// Read the proxy from the environment.
    pub fn system() -> Self {
        Self {
            source: ProxySource::System,
            ..Default::default()
        }
    }

    /// Attach credentials for proxy authentication.
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<Secret>,
    ) -> Self {
        self.username = username.into();
        self.password = password.into();
        self
    }

    /// Credentials to present to the proxy, if any were configured.
    pub fn credentials(&self) -> Option<(&str, &str)> {
        if self.username.is_empty() && self.password.is_empty() {
            None
        } else {
            Some((&self.username, self.password.expose_secret()))
        }
    }
}

/// Options for creating a gRPC Client
#[derive(Debug)]
#[non_exhaustive]
pub struct ClientConfig {
    /// Endpoint for sending requests
    pub endpoint: Uri,
    /// Allow unsafe connections to the endpoint (without SSL), defaults to false
    pub allow_unsafe_connection: bool,
    /// TLS identity of the client: key + cert
    pub identity: Option<(CertificateDer<'static>, PrivateKeyDer<'static>)>,
    /// CA certificate to authenticate the server
    pub cacert: Option<CertificateDer<'static>>,
    /// Override the endpoint name during SSL verification
    pub override_target: Option<Uri>,
    /// Timeout for establishing a connection to the server, defaults to 60s
    pub connect_timeout: Option<Duration>,
    /// Timeout for each request, defaults to no timeout
    pub timeout: Option<Duration>,
    /// Rate limit for requests, defaults to no rate limit
    pub rate_limit: Option<(u64, Duration)>,
    /// TCP keepalive duration, defaults to no keepalive
    pub tcp_keepalive: Option<Duration>,
    /// Interval between TCP keepalive probes, defaults to OS default
    pub tcp_keepalive_interval: Option<Duration>,
    /// Number of TCP keepalive retries, defaults to OS default
    pub tcp_keepalive_retries: Option<u32>,
    /// Enable Nagle's algorithm (disable TCP_NODELAY), defaults to false
    pub tcp_nagle_algorithm: bool,
    /// HTTP/2 PING frame interval, defaults to no keepalive
    pub http2_keep_alive_interval: Option<Duration>,
    /// HTTP/2 PING timeout, defaults to no timeout
    pub http2_keep_alive_timeout: Option<Duration>,
    /// Send HTTP/2 keepalive PINGs even when idle, defaults to false
    pub http2_keep_alive_while_idle: bool,
    /// HTTP/2 max header list size in bytes, defaults to no limit
    pub http2_max_header_list_size: Option<u32>,
    /// User-Agent header value sent with each request
    pub user_agent: Option<HeaderValue>,
    /// HTTP proxy used to reach the endpoint, defaults to a direct connection
    pub proxy: ProxyConfig,
    /// Let the OS reuse local ports for outgoing connections, defaults to true.
    ///
    /// Windows only, where it sets `SO_REUSE_UNICASTPORT` so that opening many connections in a short
    /// window does not exhaust the ephemeral port range. On any other platform it is ignored, down to
    /// which connector is used.
    pub reuse_ports: bool,
}

impl Default for ClientConfig {
    /// Written out rather than derived because `reuse_ports` defaults to on, which the zero value
    /// cannot say. Adding a field to the struct without adding it here does not compile.
    fn default() -> Self {
        Self {
            reuse_ports: true,
            endpoint: Uri::default(),
            allow_unsafe_connection: false,
            identity: None,
            cacert: None,
            override_target: None,
            connect_timeout: None,
            timeout: None,
            rate_limit: None,
            tcp_keepalive: None,
            tcp_keepalive_interval: None,
            tcp_keepalive_retries: None,
            tcp_nagle_algorithm: false,
            http2_keep_alive_interval: None,
            http2_keep_alive_timeout: None,
            http2_keep_alive_while_idle: false,
            http2_max_header_list_size: None,
            user_agent: None,
            proxy: ProxyConfig::default(),
        }
    }
}

impl Clone for ClientConfig {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            allow_unsafe_connection: self.allow_unsafe_connection,
            identity: self
                .identity
                .as_ref()
                .map(|(cert, key)| (cert.clone(), key.clone_key())),
            cacert: self.cacert.clone(),
            override_target: self.override_target.clone(),
            connect_timeout: self.connect_timeout,
            timeout: self.timeout,
            rate_limit: self.rate_limit,
            tcp_keepalive: self.tcp_keepalive,
            tcp_keepalive_interval: self.tcp_keepalive_interval,
            tcp_keepalive_retries: self.tcp_keepalive_retries,
            tcp_nagle_algorithm: self.tcp_nagle_algorithm,
            http2_keep_alive_interval: self.http2_keep_alive_interval,
            http2_keep_alive_timeout: self.http2_keep_alive_timeout,
            http2_keep_alive_while_idle: self.http2_keep_alive_while_idle,
            http2_max_header_list_size: self.http2_max_header_list_size,
            user_agent: self.user_agent.clone(),
            proxy: self.proxy.clone(),
            reuse_ports: self.reuse_ports,
        }
    }
}

/// Options for creating a gRPC Client, in the string form a caller supplies them in
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// Deliberately exhaustive, unlike the configuration it becomes: this is what a caller fills in, and a
// caller that cannot name every field cannot be told by the compiler when a new one appears.
pub struct ClientConfigArgs {
    /// Endpoint for sending requests
    pub endpoint: String,
    /// The client certificate itself, in PEM. Not a path: opening files is the caller's business.
    #[cfg_attr(feature = "serde", serde(default))]
    pub cert_pem: String,
    /// The client key itself, in PEM. Redacted wherever it is written; see [`Secret`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub key_pem: Secret,
    /// The Certificate Authority itself, in PEM.
    #[cfg_attr(feature = "serde", serde(default))]
    pub ca_cert: String,
    /// Allow unsafe connections to the endpoint (without SSL), defaults to false
    #[cfg_attr(feature = "serde", serde(default))]
    pub allow_unsafe_connection: bool,
    /// Override the endpoint name during SSL verification
    #[cfg_attr(feature = "serde", serde(default))]
    pub override_target_name: String,
    /// Timeout for establishing a connection to the server, defaults to no timeout
    #[cfg_attr(feature = "serde", serde(default))]
    pub connect_timeout: String,
    /// Timeout for each request, defaults to no timeout
    #[cfg_attr(feature = "serde", serde(default))]
    pub timeout: String,
    /// Rate limit for requests, defaults to no rate limit
    #[cfg_attr(feature = "serde", serde(default))]
    pub rate_limit: String,
    /// TCP keepalive duration (e.g. `30s`), defaults to no keepalive
    #[cfg_attr(feature = "serde", serde(default))]
    pub tcp_keepalive: String,
    /// Interval between TCP keepalive probes (e.g. `5s`), defaults to OS default
    #[cfg_attr(feature = "serde", serde(default))]
    pub tcp_keepalive_interval: String,
    /// Number of TCP keepalive retries, defaults to OS default
    #[cfg_attr(feature = "serde", serde(default))]
    pub tcp_keepalive_retries: String,
    /// Enable Nagle's algorithm (disable TCP_NODELAY), defaults to false
    #[cfg_attr(feature = "serde", serde(default))]
    pub tcp_nagle_algorithm: bool,
    /// HTTP/2 PING frame interval (e.g. `20s`), defaults to no keepalive
    #[cfg_attr(feature = "serde", serde(default))]
    pub http2_keep_alive_interval: String,
    /// HTTP/2 PING timeout (e.g. `10s`), defaults to no timeout
    #[cfg_attr(feature = "serde", serde(default))]
    pub http2_keep_alive_timeout: String,
    /// Send HTTP/2 keepalive PINGs even when idle, defaults to false
    #[cfg_attr(feature = "serde", serde(default))]
    pub http2_keep_alive_while_idle: bool,
    /// HTTP/2 max header list size in bytes, defaults to no limit
    #[cfg_attr(feature = "serde", serde(default))]
    pub http2_max_header_list_size: String,
    /// User-Agent header value sent with each request
    #[cfg_attr(feature = "serde", serde(default))]
    pub user_agent: String,
    /// HTTP proxy to reach the endpoint through.
    ///
    /// Empty for a direct connection, `none` to disable proxying explicitly, `system` to read the
    /// environment (see [`ProxySource::System`]), otherwise the proxy URL, whose scheme has to be
    /// `http`: the `CONNECT` handshake is written in the clear.
    #[cfg_attr(feature = "serde", serde(default))]
    pub proxy: String,
    /// Username for proxy authentication.
    ///
    /// Empty falls back to the username the `proxy` URL carried, if any.
    #[cfg_attr(feature = "serde", serde(default))]
    pub proxy_username: String,
    /// Password for proxy authentication.
    ///
    /// Empty falls back to the password the `proxy` URL carried, independently of the username, so
    /// setting this one alone still uses that URL's username. Redacted wherever it is written; see
    /// [`Secret`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub proxy_password: Secret,
    /// Let the OS reuse local ports for outgoing connections, [`None`] for the default, which is true
    #[cfg_attr(feature = "serde", serde(default))]
    pub reuse_ports: Option<bool>,
}

impl ClientConfig {
    pub fn from_config_args(args: ClientConfigArgs) -> Result<Self, ConfigError> {
        let _span = tracing::debug_span!(
            "ClientConfig",
            args.endpoint,
            // The material itself now, not a path, so only its presence is recorded: a private key
            // must never reach a log, and a certificate would bury the span in PEM.
            cert_pem_set = !args.cert_pem.is_empty(),
            key_pem_set = !args.key_pem.is_empty(),
            ca_cert_set = !args.ca_cert.is_empty(),
            args.allow_unsafe_connection,
            args.override_target_name,
            args.connect_timeout,
            args.timeout,
            args.rate_limit,
            args.tcp_keepalive,
            args.tcp_keepalive_interval,
            args.tcp_keepalive_retries,
            args.tcp_nagle_algorithm,
            args.http2_keep_alive_interval,
            args.http2_keep_alive_timeout,
            args.http2_keep_alive_while_idle,
            args.http2_max_header_list_size,
            args.user_agent,
            // Elided, and `proxy_password` left out entirely: this span is recorded at debug level, and
            // a proxy URL carries credentials as often as the dedicated option does. Do not complete
            // the list.
            proxy = %crate::proxy::elide_userinfo(&args.proxy),
            args.proxy_username,
            reuse_ports = ?args.reuse_ports,
        );

        let ClientConfigArgs {
            endpoint,
            cert_pem,
            key_pem,
            ca_cert,
            allow_unsafe_connection,
            override_target_name,
            connect_timeout,
            timeout,
            rate_limit,
            tcp_keepalive,
            tcp_keepalive_interval,
            tcp_keepalive_retries,
            tcp_nagle_algorithm,
            http2_keep_alive_interval,
            http2_keep_alive_timeout,
            http2_keep_alive_while_idle,
            http2_max_header_list_size,
            user_agent,
            proxy,
            proxy_username,
            proxy_password,
            reuse_ports,
        } = args;

        let cacert = if ca_cert.is_empty() {
            None
        } else {
            Some(CertificateDer::from_pem_slice(ca_cert.as_bytes()).context(TlsSnafu {})?)
        };

        let identity = match (cert_pem.is_empty(), key_pem.is_empty()) {
            (true, true) => None,
            (true, false) | (false, true) => {
                return IncompatibleOptionsSnafu {
                    msg: String::from(
                        "`cert_pem` and `key_pem` must be either both empty or both set",
                    ),
                }
                .fail()
            }
            (false, false) => {
                let cert =
                    CertificateDer::from_pem_slice(cert_pem.as_bytes()).context(TlsSnafu {})?;
                let key = PrivateKeyDer::from_pem_slice(key_pem.expose_secret().as_bytes())
                    .context(TlsSnafu {})?;

                Some((cert, key))
            }
        };

        let endpoint = Uri::try_from(endpoint.clone()).context(UriSnafu { uri: endpoint })?;

        let override_target = if override_target_name.is_empty() {
            None
        } else {
            let authority;
            let path_and_query;

            if let Ok(auth) = override_target_name.parse::<hyper::http::uri::Authority>() {
                authority = Some(auth);
                path_and_query = endpoint.path_and_query().cloned();
            } else {
                hyper::http::uri::Parts {
                    authority,
                    path_and_query,
                    ..
                } = Uri::try_from(override_target_name.clone())
                    .context(UriSnafu {
                        uri: endpoint.to_string(),
                    })?
                    .into_parts();
            }

            let mut uri = hyper::http::uri::Builder::new();

            if let Some(scheme) = endpoint.scheme() {
                uri = uri.scheme(scheme.clone());
            }
            if let Some(authority) = authority.or_else(|| endpoint.authority().cloned()) {
                uri = uri.authority(authority);
            }
            if let Some(path_and_query) = path_and_query {
                uri = uri.path_and_query(path_and_query);
            }

            Some(uri.build().context(HttpSnafu {
                uri: override_target_name,
            })?)
        };

        let connect_timeout = if connect_timeout.is_empty() {
            Some(Duration::from_secs(60))
        } else {
            Some(
                connect_timeout
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "connect_timeout",
                        value: connect_timeout,
                    })?
                    .into(),
            )
        };

        let timeout = if timeout.is_empty() {
            None
        } else {
            Some(
                timeout
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "timeout",
                        value: timeout,
                    })?
                    .into(),
            )
        };

        let rate_limit = if rate_limit.is_empty() {
            None
        } else {
            let parts: Vec<&str> = rate_limit.split('/').collect();
            if parts.len() != 2 {
                return IncompatibleOptionsSnafu {
                    msg: format!("Rate limit should be in the format `number/duration`, e.g. `100/1s`, but got `{rate_limit}`"),
                }.fail();
            }
            let limit = parts[0]
                .parse::<u64>()
                .context(InvalidRateLimitCountSnafu {
                    value: parts[0].to_string(),
                })?;
            let duration: Duration = parts[1]
                .parse::<humantime::Duration>()
                .context(InvalidDurationSnafu {
                    option: "rate_limit",
                    value: rate_limit.clone(),
                })?
                .into();
            // `tower`'s rate limiter asserts both are non-zero, so leaving these to it turns a mistyped
            // option into a panic inside `connect` rather than an error the caller can read.
            if limit == 0 || duration.is_zero() {
                return IncompatibleOptionsSnafu {
                    msg: format!(
                        "`rate_limit={rate_limit}` has a zero count or duration. Both have \
                         to be above zero, as in `100/1s`; leave it empty for no rate limit"
                    ),
                }
                .fail();
            }
            Some((limit, duration))
        };

        let tcp_keepalive = if tcp_keepalive.is_empty() {
            None
        } else {
            Some(
                tcp_keepalive
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "tcp_keepalive",
                        value: tcp_keepalive,
                    })?
                    .into(),
            )
        };

        let tcp_keepalive_interval = if tcp_keepalive_interval.is_empty() {
            None
        } else {
            Some(
                tcp_keepalive_interval
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "tcp_keepalive_interval",
                        value: tcp_keepalive_interval,
                    })?
                    .into(),
            )
        };

        let tcp_keepalive_retries = if tcp_keepalive_retries.is_empty() {
            None
        } else {
            Some(
                tcp_keepalive_retries
                    .parse::<u32>()
                    .context(InvalidIntegerSnafu {
                        value: tcp_keepalive_retries,
                    })?,
            )
        };

        let http2_keep_alive_interval = if http2_keep_alive_interval.is_empty() {
            None
        } else {
            Some(
                http2_keep_alive_interval
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "http2_keep_alive_interval",
                        value: http2_keep_alive_interval,
                    })?
                    .into(),
            )
        };

        let http2_keep_alive_timeout = if http2_keep_alive_timeout.is_empty() {
            None
        } else {
            Some(
                http2_keep_alive_timeout
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "http2_keep_alive_timeout",
                        value: http2_keep_alive_timeout,
                    })?
                    .into(),
            )
        };

        let http2_max_header_list_size = if http2_max_header_list_size.is_empty() {
            None
        } else {
            Some(
                http2_max_header_list_size
                    .parse::<u32>()
                    .context(InvalidIntegerSnafu {
                        value: http2_max_header_list_size,
                    })?,
            )
        };

        let user_agent = if user_agent.is_empty() {
            None
        } else {
            let header = HeaderValue::from_str(&user_agent)
                .context(InvalidUserAgentSnafu { value: user_agent })?;
            Some(header)
        };

        let mut proxy = match parse_proxy_source(&proxy)? {
            // Through the constructor, so credentials written into the URL are taken out of it.
            ProxySource::Explicit(uri) => ProxyConfig::explicit(uri),
            ProxySource::System => ProxyConfig::system(),
            ProxySource::Disabled => ProxyConfig::default(),
        };
        // Field by field, not pair by pair: setting only the password while the URL carries
        // `user:other@` has to keep that username, or the request fails as an unexplained 407.
        let username = if proxy_username.is_empty() {
            std::mem::take(&mut proxy.username)
        } else {
            proxy_username
        };
        let password = if proxy_password.is_empty() {
            std::mem::take(&mut proxy.password)
        } else {
            proxy_password
        };
        proxy = proxy.with_credentials(username, password);

        Ok(Self {
            endpoint,
            allow_unsafe_connection,
            identity,
            cacert,
            override_target,
            connect_timeout,
            timeout,
            rate_limit,
            tcp_keepalive,
            tcp_keepalive_interval,
            tcp_keepalive_retries,
            tcp_nagle_algorithm,
            http2_keep_alive_interval,
            http2_keep_alive_timeout,
            http2_keep_alive_while_idle,
            http2_max_header_list_size,
            user_agent,
            proxy,
            // Unset means on, as it does for the other clients, so an explicit `false` is the only
            // way to turn it off.
            reuse_ports: reuse_ports.unwrap_or(true),
        })
    }
}

/// Interpret the `proxy` value.
///
/// As ArmoniK's other clients spell it: empty is a direct connection, `none` disables proxying,
/// `system` reads the environment, anything else is a proxy URL, defaulting to the `http` scheme.
fn parse_proxy_source(proxy: &str) -> Result<ProxySource, ConfigError> {
    match proxy {
        "" => Ok(ProxySource::Disabled),
        _ if proxy.eq_ignore_ascii_case("none") => Ok(ProxySource::Disabled),
        _ if proxy.eq_ignore_ascii_case("system") => Ok(ProxySource::System),
        _ => {
            let with_scheme = if proxy.contains("://") {
                proxy.to_owned()
            } else {
                format!("http://{proxy}")
            };
            // Not reported through `UriSnafu`: its message names the endpoint, which would send
            // whoever reads it looking at the wrong option.
            let uri = Uri::try_from(&with_scheme).ok().filter(|uri| {
                uri.authority()
                    .is_some_and(|authority| !authority.host().is_empty())
            });
            match uri {
                // Caught here rather than at connect time, so a mistyped option fails while the
                // configuration is being read and names itself.
                Some(uri) if uri.scheme_str().is_some_and(|scheme| scheme != "http") => {
                    IncompatibleOptionsSnafu {
                        msg: format!(
                            "The `CONNECT` handshake is written in the clear, so only an `http` \
                             proxy can be reached, and `proxy={}` names another scheme",
                            crate::proxy::elide_userinfo(proxy)
                        ),
                    }
                    .fail()
                }
                Some(uri) => Ok(ProxySource::Explicit(uri)),
                None => IncompatibleOptionsSnafu {
                    // Elided: a URL rejected for having no host can still have carried a password.
                    msg: format!(
                        "`proxy={}` is not a valid proxy URL. Expected `none`, \
                         `system`, or a URL such as `http://proxy.example.com:3128`",
                        crate::proxy::elide_userinfo(proxy)
                    ),
                }
                .fail(),
            }
        }
    }
}

impl TryFrom<&ClientConfig> for tonic::transport::Endpoint {
    type Error = ConfigError;

    fn try_from(value: &ClientConfig) -> Result<Self, Self::Error> {
        Ok(Self::from(value.endpoint.clone()))
    }
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum ConfigError {
    #[snafu(display("Invalid TLS configuration [{location}]"))]
    #[non_exhaustive]
    Tls {
        #[snafu(source(from(rustls::pki_types::pem::Error, Box::new)))]
        source: Box<rustls::pki_types::pem::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Endpoint URI is not valid: `{uri}` [{location}]"))]
    #[non_exhaustive]
    Uri {
        #[snafu(source(from(hyper::http::uri::InvalidUri, Box::new)))]
        source: Box<hyper::http::uri::InvalidUri>,
        uri: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Override URI is not valid: `{uri}` [{location}]"))]
    #[non_exhaustive]
    Http {
        #[snafu(source(from(hyper::http::Error, Box::new)))]
        source: Box<hyper::http::Error>,
        uri: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("{msg} [{location}]"))]
    #[non_exhaustive]
    IncompatibleOptions {
        msg: String,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "`{option}={value}` is not a valid duration (e.g. `30s` or `1m`) [{location}]"
    ))]
    #[non_exhaustive]
    InvalidDuration {
        source: humantime::DurationError,
        option: &'static str,
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Rate limit count `{value}` is not a valid integer [{location}]"))]
    #[non_exhaustive]
    InvalidRateLimitCount {
        source: std::num::ParseIntError,
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("`{value}` is not a valid integer [{location}]"))]
    #[non_exhaustive]
    InvalidInteger {
        source: std::num::ParseIntError,
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Invalid user agent `{value}` [{location}]"))]
    #[non_exhaustive]
    InvalidUserAgent {
        source: hyper::http::header::InvalidHeaderValue,
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The minimum viable arguments: an endpoint, and nothing else set.
    fn args() -> ClientConfigArgs {
        ClientConfigArgs {
            endpoint: String::from("http://localhost:5001"),
            ..Default::default()
        }
    }

    /// Every message in the chain, joined. snafu keeps the detail in the source, so asserting on the
    /// outermost `Display` alone would pass whatever the cause turned out to be.
    fn chain(error: &ConfigError) -> String {
        let mut rendered = error.to_string();
        let mut source = std::error::Error::source(error);
        while let Some(cause) = source {
            rendered.push_str(" | ");
            rendered.push_str(&cause.to_string());
            source = cause.source();
        }
        rendered
    }

    #[test]
    fn the_minimum_is_an_endpoint() {
        let config = ClientConfig::from_config_args(args()).expect("an endpoint is enough");

        assert_eq!(config.endpoint.to_string(), "http://localhost:5001/");
        assert!(config.identity.is_none());
        assert!(config.cacert.is_none());
        assert_eq!(config.override_target, None);
        assert_eq!(config.rate_limit, None);
    }

    #[test]
    fn an_endpoint_that_is_not_a_uri_is_reported() {
        let error = ClientConfig::from_config_args(ClientConfigArgs {
            endpoint: String::new(),
            ..args()
        })
        .expect_err("an empty endpoint is not a URI");

        assert!(matches!(error, ConfigError::Uri { .. }), "{error:?}");
    }

    // --- durations and numbers ---

    #[test]
    fn durations_are_read_in_the_units_they_are_written_in() {
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            connect_timeout: String::from("500ms"),
            tcp_keepalive: String::from("30s"),
            tcp_keepalive_interval: String::from("2m"),
            http2_keep_alive_interval: String::from("1h"),
            ..args()
        })
        .expect("valid durations");

        assert_eq!(config.connect_timeout, Some(Duration::from_millis(500)));
        assert_eq!(config.tcp_keepalive, Some(Duration::from_secs(30)));
        assert_eq!(
            config.tcp_keepalive_interval,
            Some(Duration::from_secs(120))
        );
        assert_eq!(
            config.http2_keep_alive_interval,
            Some(Duration::from_secs(3600))
        );
    }

    #[test]
    fn a_duration_that_cannot_be_parsed_names_the_value() {
        let error = ClientConfig::from_config_args(ClientConfigArgs {
            tcp_keepalive: String::from("soon"),
            ..args()
        })
        .expect_err("`soon` is not a duration");

        assert!(
            matches!(error, ConfigError::InvalidDuration { .. }),
            "{error:?}"
        );
        assert!(chain(&error).contains("soon"), "{}", chain(&error));
    }

    #[test]
    fn integers_are_read_and_a_bad_one_names_the_value() {
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            tcp_keepalive_retries: String::from("3"),
            http2_max_header_list_size: String::from("16384"),
            ..args()
        })
        .expect("valid integers");
        assert_eq!(config.tcp_keepalive_retries, Some(3));
        assert_eq!(config.http2_max_header_list_size, Some(16384));

        let error = ClientConfig::from_config_args(ClientConfigArgs {
            tcp_keepalive_retries: String::from("many"),
            ..args()
        })
        .expect_err("`many` is not an integer");
        assert!(
            matches!(error, ConfigError::InvalidInteger { .. }),
            "{error:?}"
        );
        assert!(chain(&error).contains("many"), "{}", chain(&error));
    }

    #[test]
    fn an_integer_that_does_not_fit_is_rejected_rather_than_wrapped() {
        // These are `u32`; a value past the top must fail rather than silently become something else.
        let error = ClientConfig::from_config_args(ClientConfigArgs {
            http2_max_header_list_size: String::from("4294967296"),
            ..args()
        })
        .expect_err("2^32 does not fit in a u32");

        assert!(
            matches!(error, ConfigError::InvalidInteger { .. }),
            "{error:?}"
        );
    }

    // --- rate limit ---

    #[test]
    fn a_rate_limit_is_a_count_and_a_duration() {
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            rate_limit: String::from("100/1s"),
            ..args()
        })
        .expect("valid");

        assert_eq!(config.rate_limit, Some((100, Duration::from_secs(1))));
    }

    #[test]
    fn a_zero_rate_limit_is_rejected_rather_than_left_to_panic() {
        // `tower`'s `Rate::new` asserts both halves are above zero, so a zero has to be refused here
        // rather than reaching it: a panic inside `connect` tells the caller nothing.
        for value in ["0/1s", "1/0s", "0/0s"] {
            let error = ClientConfig::from_config_args(ClientConfigArgs {
                rate_limit: String::from(value),
                ..args()
            })
            .expect_err("a zero rate limit must be rejected")
            .to_string();

            assert!(error.contains("zero count or duration"), "{value}: {error}");
            assert!(
                error.contains(value),
                "the message should quote it: {error}"
            );
        }
    }

    #[test]
    fn a_rate_limit_missing_its_duration_is_reported_with_the_expected_shape() {
        // The message has to show the format, since `100` on its own looks perfectly reasonable to whoever
        // wrote it.
        let error = ClientConfig::from_config_args(ClientConfigArgs {
            rate_limit: String::from("100"),
            ..args()
        })
        .expect_err("a rate limit needs both halves");

        assert!(
            matches!(error, ConfigError::IncompatibleOptions { .. }),
            "{error:?}"
        );
        let rendered = chain(&error);
        assert!(rendered.contains("number/duration"), "{rendered}");
        assert!(rendered.contains("100"), "{rendered}");
    }

    #[test]
    fn each_half_of_a_rate_limit_is_validated_separately() {
        let count = ClientConfig::from_config_args(ClientConfigArgs {
            rate_limit: String::from("plenty/1s"),
            ..args()
        })
        .expect_err("`plenty` is not a count");
        assert!(
            matches!(count, ConfigError::InvalidRateLimitCount { .. }),
            "{count:?}"
        );

        let duration = ClientConfig::from_config_args(ClientConfigArgs {
            rate_limit: String::from("100/soon"),
            ..args()
        })
        .expect_err("`soon` is not a duration");
        assert!(
            matches!(duration, ConfigError::InvalidDuration { .. }),
            "{duration:?}"
        );
    }

    // --- certificates ---

    #[test]
    fn half_an_identity_is_rejected_and_names_both_variables() {
        // Half an identity is silent on a plain-TLS endpoint and only surfaces as a rejected handshake
        // on an mTLS one, so it is caught before either half is parsed.
        for (cert, key) in [("a certificate", ""), ("", "a key")] {
            let error = ClientConfig::from_config_args(ClientConfigArgs {
                cert_pem: String::from(cert),
                key_pem: key.into(),
                ..args()
            })
            .expect_err("half an identity must be rejected");

            assert!(
                matches!(error, ConfigError::IncompatibleOptions { .. }),
                "{error:?}"
            );
            let rendered = chain(&error);
            assert!(rendered.contains("cert_pem"), "{rendered}");
            assert!(rendered.contains("key_pem"), "{rendered}");
        }
    }

    #[test]
    fn neither_half_is_no_identity_rather_than_an_error() {
        let config = ClientConfig::from_config_args(args()).expect("valid");
        assert!(config.identity.is_none());
    }

    #[test]
    fn a_path_where_the_material_is_expected_fails_as_pem() {
        // These options carry the certificate itself. Someone still passing a path gets a PEM error
        // naming what was wrong, rather than this crate quietly opening whatever it points at.
        for args in [
            ClientConfigArgs {
                cert_pem: String::from("/etc/ssl/certs/client.pem"),
                key_pem: "/etc/ssl/private/client.key".into(),
                ..args()
            },
            ClientConfigArgs {
                ca_cert: String::from("/etc/ssl/certs/ca.pem"),
                ..args()
            },
        ] {
            let error =
                ClientConfig::from_config_args(args).expect_err("a path is not a certificate");

            assert!(matches!(error, ConfigError::Tls { .. }), "{error:?}");
        }
    }

    // --- override target ---

    #[test]
    fn an_override_target_given_as_a_host_keeps_the_endpoints_scheme_and_path() {
        // The common case: the certificate names one host, the endpoint is reached at another. Only the
        // authority is being overridden, so everything else has to come from the endpoint.
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            endpoint: String::from("https://10.0.0.1:5003/base"),
            override_target_name: String::from("server.example.com"),
            ..args()
        })
        .expect("valid");

        let override_target = config.override_target.expect("an override target");
        assert_eq!(override_target.scheme_str(), Some("https"));
        assert_eq!(
            override_target.authority().map(|a| a.as_str()),
            Some("server.example.com")
        );
        assert_eq!(override_target.path(), "/base");
    }

    #[test]
    fn an_override_target_given_as_a_uri_replaces_the_authority_and_the_path() {
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            endpoint: String::from("https://10.0.0.1:5003/base"),
            override_target_name: String::from("https://server.example.com/other"),
            ..args()
        })
        .expect("valid");

        let override_target = config.override_target.expect("an override target");
        assert_eq!(
            override_target.authority().map(|a| a.as_str()),
            Some("server.example.com")
        );
        assert_eq!(override_target.path(), "/other");
        // The scheme still comes from the endpoint: the connection is made to the endpoint, and this only
        // changes the name it is verified against.
        assert_eq!(override_target.scheme_str(), Some("https"));
    }

    #[test]
    fn no_override_target_leaves_it_unset() {
        let config = ClientConfig::from_config_args(args()).expect("valid");
        assert_eq!(config.override_target, None);
    }

    // --- the serde feature ---

    #[cfg(feature = "serde")]
    #[test]
    fn arguments_round_trip_through_serde_with_absent_fields_defaulted() {
        // Every field but the endpoint carries `serde(default)`, so a configuration file need only name
        // what it changes. The feature is off by default, so nothing else here would notice it breaking.
        let deserialised: ClientConfigArgs =
            serde_json::from_str(r#"{"endpoint":"http://localhost:5001","timeout":"30s"}"#)
                .expect("absent fields should default");

        assert_eq!(deserialised.endpoint, "http://localhost:5001");
        assert_eq!(deserialised.timeout, "30s");
        assert_eq!(deserialised.cert_pem, "", "an absent field defaults");
        assert!(!deserialised.allow_unsafe_connection);

        let round_tripped: ClientConfigArgs =
            serde_json::from_str(&serde_json::to_string(&deserialised).expect("serialise"))
                .expect("deserialise");
        assert_eq!(round_tripped, deserialised);
    }

    // --- the proxy ---

    #[test]
    fn proxy_none_and_empty_disable_proxying() {
        for value in ["", "none", "None", "NONE"] {
            let config = ClientConfig::from_config_args(ClientConfigArgs {
                proxy: String::from(value),
                ..args()
            })
            .expect("a valid configuration");
            assert_eq!(
                config.proxy.source,
                ProxySource::Disabled,
                "{value:?} should disable proxying"
            );
        }
    }

    #[test]
    fn proxy_system_reads_the_environment() {
        for value in ["system", "System", "SYSTEM"] {
            let config = ClientConfig::from_config_args(ClientConfigArgs {
                proxy: String::from(value),
                ..args()
            })
            .expect("a valid configuration");
            assert_eq!(config.proxy.source, ProxySource::System, "{value:?}");
        }
    }

    #[test]
    fn proxy_url_defaults_to_the_http_scheme() {
        let with_scheme = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("http://proxy.corp:3128"),
            ..args()
        })
        .expect("a valid configuration");
        let without_scheme = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("proxy.corp:3128"),
            ..args()
        })
        .expect("a valid configuration");

        assert_eq!(with_scheme.proxy.source, without_scheme.proxy.source);
        let ProxySource::Explicit(uri) = with_scheme.proxy.source else {
            panic!("expected an explicit proxy");
        };
        assert_eq!(uri.host(), Some("proxy.corp"));
        assert_eq!(uri.port_u16(), Some(3128));
    }

    #[test]
    fn proxy_credentials_are_optional() {
        let none = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("proxy.corp:3128"),
            ..args()
        })
        .expect("a valid configuration");
        assert_eq!(none.proxy.credentials(), None);

        let some = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("proxy.corp:3128"),
            proxy_username: String::from("user"),
            proxy_password: "secret".into(),
            ..args()
        })
        .expect("a valid configuration");
        assert_eq!(some.proxy.credentials(), Some(("user", "secret")));
    }

    #[test]
    fn proxy_credentials_in_the_url_are_honoured_and_removed_from_it() {
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("http://user:secret@proxy.corp:3128"),
            ..args()
        })
        .expect("a valid configuration");

        assert_eq!(config.proxy.credentials(), Some(("user", "secret")));
        let ProxySource::Explicit(uri) = &config.proxy.source else {
            panic!("expected an explicit proxy");
        };
        assert!(
            !uri.to_string().contains("secret"),
            "the URI is rendered in errors and logs: {uri}"
        );
    }

    #[test]
    fn the_dedicated_proxy_options_win_over_the_url() {
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("http://url-user:url-secret@proxy.corp:3128"),
            proxy_username: String::from("option-user"),
            proxy_password: "option-secret".into(),
            ..args()
        })
        .expect("a valid configuration");

        assert_eq!(
            config.proxy.credentials(),
            Some(("option-user", "option-secret"))
        );
    }

    #[test]
    fn the_proxy_password_is_kept_out_of_the_debug_output() {
        // `ClientConfig` is `Debug` and holds a `ProxyConfig`, so a derived `Debug` would put the
        // password anywhere a configuration gets printed.
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("proxy.corp:3128"),
            proxy_username: String::from("user"),
            proxy_password: "s3cr3t".into(),
            ..args()
        })
        .expect("a valid configuration");

        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("s3cr3t"),
            "password rendered: {rendered}"
        );
        assert!(
            rendered.contains("[redacted]"),
            "the field should still be visible as redacted: {rendered}"
        );
        assert!(
            rendered.contains("user"),
            "the username is not a secret and stays useful: {rendered}"
        );
    }

    #[test]
    fn a_dedicated_password_alone_keeps_the_username_the_url_carried() {
        // Replacing the pair rather than each field would leave an empty username here, and the proxy
        // would answer 407 with nothing to explain it.
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("http://url-user:url-secret@proxy.corp:3128"),
            proxy_password: "option-secret".into(),
            ..args()
        })
        .expect("a valid configuration");

        assert_eq!(
            config.proxy.credentials(),
            Some(("url-user", "option-secret"))
        );
    }

    #[test]
    fn a_rejected_proxy_url_does_not_echo_its_password() {
        // A URL can be rejected for having no host and still have carried a credential.
        let error = ClientConfig::from_config_args(ClientConfigArgs {
            proxy: String::from("http://user:s3cr3t@"),
            ..args()
        })
        .expect_err("a proxy without a host must be rejected")
        .to_string();

        assert!(!error.contains("s3cr3t"), "password echoed: {error}");
        assert!(error.contains("is not a valid proxy URL"), "{error}");
    }

    #[test]
    fn the_arguments_keep_the_password_out_of_their_debug_output() {
        // `ClientConfig` is not the only type that holds it: these are what a caller inspects before
        // handing them over.
        let args = ClientConfigArgs {
            proxy: String::from("http://user:url-secret@proxy.corp:3128"),
            proxy_password: "option-secret".into(),
            ..args()
        };

        let rendered = format!("{args:?}");
        assert!(
            !rendered.contains("option-secret"),
            "password rendered: {rendered}"
        );
        assert!(
            rendered.contains("[redacted]"),
            "the field should still show as redacted: {rendered}"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn an_ordinary_serialisation_redacts_the_password() {
        let args = ClientConfigArgs {
            proxy_password: "s3cr3t".into(),
            ..args()
        };

        let json = serde_json::to_string(&args).expect("serialise");
        assert!(!json.contains("s3cr3t"), "password written out: {json}");
    }

    #[test]
    fn a_proxy_that_is_not_http_is_rejected_rather_than_reached_in_the_clear() {
        // The `CONNECT` handshake goes out unencrypted, so a proxy expecting TLS would see gibberish.
        // Accepting the URL and failing at connect time would report it as an unreachable proxy.
        for value in ["https://proxy.corp:3128", "socks5://proxy.corp:1080"] {
            let error = ClientConfig::from_config_args(ClientConfigArgs {
                proxy: String::from(value),
                ..args()
            })
            .expect_err("only an http proxy can be reached")
            .to_string();

            assert!(error.contains("only an `http` proxy"), "{value}: {error}");
        }
    }

    #[test]
    fn proxy_without_a_host_is_rejected_and_names_its_own_option() {
        // Reporting these through the endpoint's URI error would send whoever reads it looking at the
        // wrong setting.
        for value in ["http:///no-host", "http://", "http://:3128", "://"] {
            let error = ClientConfig::from_config_args(ClientConfigArgs {
                proxy: String::from(value),
                ..args()
            })
            .expect_err("a proxy without a host must be rejected")
            .to_string();
            assert!(
                error.contains("is not a valid proxy URL"),
                "unexpected error for {value:?}: {error}"
            );
        }
    }

    #[test]
    fn port_reuse_is_on_unless_it_is_turned_off() {
        // The one option whose default is not the zero value, checked in both places that decide it:
        // a configuration built by hand, and one converted from arguments.
        assert!(ClientConfig::default().reuse_ports);

        for (given, expected) in [(None, true), (Some(true), true), (Some(false), false)] {
            let config = ClientConfig::from_config_args(ClientConfigArgs {
                reuse_ports: given,
                ..args()
            })
            .expect("configuration");

            assert_eq!(config.reuse_ports, expected, "given {given:?}");
        }
    }
}
