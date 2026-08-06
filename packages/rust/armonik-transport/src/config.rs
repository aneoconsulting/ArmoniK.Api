use std::time::Duration;

use hyper::{http::HeaderValue, Uri};
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use snafu::{ResultExt, Snafu};

use crate::proxy::{ProxyConfig, ProxySource};

/// Options for creating a gRPC Client
#[derive(Debug, Default)]
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
    /// HTTP proxy used to reach the endpoint, defaults to following the environment
    pub proxy: ProxyConfig,
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
        }
    }
}

/// Options for creating a gRPC Client (as given in the environment)
///
/// Deserializable but deliberately not serializable: the proxy password is a secret, and a type
/// that cannot be written out cannot leak it through a configuration dump. `Debug` redacts it for
/// the same reason.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[non_exhaustive]
pub struct ClientConfigArgs {
    /// Endpoint for sending requests
    pub endpoint: String,
    /// Path to the certificate file in pem format
    #[cfg_attr(feature = "serde", serde(default))]
    pub cert_pem: String,
    /// Path to the key file in pem format
    #[cfg_attr(feature = "serde", serde(default))]
    pub key_pem: String,
    /// Path to the Certificate Authority file in pem format
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
    /// Empty or `system` follows the environment (see [`ProxySource::System`]), `none` forces a
    /// direct connection, otherwise the proxy URL, whose scheme has to be `http`: the `CONNECT`
    /// handshake is written in the clear. `none` and `system` are the spellings every ArmoniK
    /// client understands; other casings are accepted here but not everywhere.
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
    /// setting this one alone still uses that URL's username. Redacted by `Debug` and zeroized on
    /// drop.
    #[cfg_attr(feature = "serde", serde(default))]
    pub proxy_password: secrecy::SecretString,
}

impl ClientConfigArgs {
    pub fn from_env() -> Result<Self, ConfigError> {
        use crate::utils::{read_env, read_env_bool};
        let ctx = EnvSnafu {};
        Ok(Self {
            endpoint: read_env("GrpcClient__Endpoint").context(ctx)?,
            cert_pem: read_env("GrpcClient__CertPem").context(ctx)?,
            key_pem: read_env("GrpcClient__KeyPem").context(ctx)?,
            ca_cert: read_env("GrpcClient__CaCert").context(ctx)?,
            allow_unsafe_connection: read_env_bool("GrpcClient__AllowUnsafeConnection")
                .context(ctx)?,
            override_target_name: read_env("GrpcClient__OverrideTargetName").context(ctx)?,
            connect_timeout: read_env("GrpcClient__ConnectTimeout").context(ctx)?,
            timeout: read_env("GrpcClient__Timeout").context(ctx)?,
            rate_limit: read_env("GrpcClient__RateLimit").context(ctx)?,
            tcp_keepalive: read_env("GrpcClient__TcpKeepalive").context(ctx)?,
            tcp_keepalive_interval: read_env("GrpcClient__TcpKeepaliveInterval").context(ctx)?,
            tcp_keepalive_retries: read_env("GrpcClient__TcpKeepaliveRetries").context(ctx)?,
            tcp_nagle_algorithm: read_env_bool("GrpcClient__TcpNagleAlgorithm").context(ctx)?,
            http2_keep_alive_interval: read_env("GrpcClient__Http2KeepAliveInterval")
                .context(ctx)?,
            http2_keep_alive_timeout: read_env("GrpcClient__Http2KeepAliveTimeout").context(ctx)?,
            http2_keep_alive_while_idle: read_env_bool("GrpcClient__Http2KeepAliveWhileIdle")
                .context(ctx)?,
            http2_max_header_list_size: read_env("GrpcClient__Http2MaxHeaderListSize")
                .context(ctx)?,
            user_agent: read_env("GrpcClient__UserAgent").context(ctx)?,
            proxy: read_env("GrpcClient__Proxy").context(ctx)?,
            proxy_username: read_env("GrpcClient__ProxyUsername").context(ctx)?,
            proxy_password: read_env("GrpcClient__ProxyPassword").context(ctx)?.into(),
        })
    }
}

impl ClientConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_config_args(ClientConfigArgs::from_env()?)
    }
    pub fn from_config_args(args: ClientConfigArgs) -> Result<Self, ConfigError> {
        let _span = tracing::debug_span!(
            "ClientConfig",
            args.endpoint,
            args.cert_pem,
            args.key_pem,
            args.ca_cert,
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
            // Elided, and `proxy_password` left out entirely: this span is recorded at debug level,
            // and a proxy URL carries credentials as often as the dedicated option does. Do not
            // complete the list.
            proxy = %crate::proxy::elide_userinfo(&args.proxy),
            args.proxy_username,
        );

        let ClientConfigArgs {
            endpoint,
            cert_pem: cert_path,
            key_pem: key_path,
            ca_cert: cacert_path,
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
        } = args;

        // Read CAcert file
        let cacert = if !cacert_path.is_empty() {
            let cacert_pem = std::fs::read_to_string(cacert_path.clone())
                .context(IoSnafu { path: cacert_path })?;
            Some(CertificateDer::from_pem_slice(cacert_pem.as_bytes()).context(TlsSnafu {})?)
        } else {
            None
        };

        // Read client cert and key files
        let identity = match (cert_path.as_str(), key_path.as_str()) {
            ("", "") => None,
            ("", _) | (_, "") => return IncompatibleOptionsSnafu{msg: format!("`GrpcClient__CertPem={cert_path}` and `GrpcClient__KeyPem={key_path}` must be either both empty or both set")}.fail(),
            (cert_path, key_path) => {
                let cert_pem =
                    std::fs::read_to_string(cert_path).context(IoSnafu { path: cert_path })?;
                let key_pem = std::fs::read(key_path).context(IoSnafu { path: key_path })?;
                let cert = CertificateDer::from_pem_slice(cert_pem.as_bytes()).context(TlsSnafu {})?;
                let key = PrivateKeyDer::from_pem_slice(key_pem.as_slice()).context(TlsSnafu{})?;

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
                    .context(InvalidDurationSnafu { value: timeout })?
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
                    value: rate_limit.clone(),
                })?
                .into();
            // `tower`'s rate limiter asserts both are non-zero, so leaving these to it turns a mistyped
            // option into a panic inside `connect` rather than an error the caller can read.
            if limit == 0 || duration.is_zero() {
                return IncompatibleOptionsSnafu {
                    msg: format!(
                        "`GrpcClient__RateLimit={rate_limit}` has a zero count or duration. Both have \
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
            // A dedicated credential half wins over the URL's; `with_credentials` leaves an empty
            // half to what the URL carried.
            proxy: parse_proxy_source(&proxy)?.with_credentials(proxy_username, proxy_password),
        })
    }
}

/// Interpret the `GrpcClient__Proxy` value.
///
/// Empty and `system` follow the environment, `none` forces a direct connection, anything else is a
/// proxy URL, defaulting to the `http` scheme. A URL that cannot be dialled as written, a missing
/// host, a port that is not one, or a scheme the `CONNECT` handshake cannot use, fails here, while
/// the configuration is being read and can name itself, rather than at connect time.
fn parse_proxy_source(proxy: &str) -> Result<ProxyConfig, ConfigError> {
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
    // Not reported through `UriSnafu`: its message names the endpoint, which would send
    // whoever reads it looking at the wrong option.
    let uri = Uri::try_from(&with_scheme).ok().filter(|uri| {
        uri.authority()
            .is_some_and(|authority| !authority.host().is_empty())
    });
    let Some(uri) = uri else {
        return IncompatibleOptionsSnafu {
            // Elided: a URL rejected for having no host can still have carried a password.
            msg: format!(
                "`GrpcClient__Proxy={}` is not a valid proxy URL. Expected `none`, \
                 `system`, or a URL such as `http://proxy.example.com:3128`",
                crate::proxy::elide_userinfo(proxy)
            ),
        }
        .fail();
    };

    if uri.scheme_str().is_some_and(|scheme| scheme != "http") {
        return IncompatibleOptionsSnafu {
            msg: format!(
                "{}, and `GrpcClient__Proxy={}` names another scheme",
                crate::proxy::HTTP_ONLY_RULE,
                crate::proxy::elide_userinfo(proxy)
            ),
        }
        .fail();
    }

    let config = ProxyConfig::explicit(uri);
    let ProxySource::Explicit(stripped) = &config.source else {
        return IncompatibleOptionsSnafu {
            msg: String::from("`GrpcClient__Proxy`: an explicit proxy should stay explicit"),
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
    if written_port.is_some() && stripped.port_u16().is_none() {
        return IncompatibleOptionsSnafu {
            msg: format!(
                "`GrpcClient__Proxy={}` does not name a valid port",
                crate::proxy::elide_userinfo(proxy)
            ),
        }
        .fail();
    }

    Ok(config)
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
    #[snafu(display("Could not read environment variable [{location}]"))]
    #[non_exhaustive]
    Env {
        #[snafu(source(from(crate::utils::ReadEnvError, Box::new)))]
        source: Box<crate::utils::ReadEnvError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
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
    #[snafu(display("Could not read file `{path}` [{location}]"))]
    #[non_exhaustive]
    Io {
        #[snafu(source(from(std::io::Error, Box::new)))]
        source: Box<std::io::Error>,
        path: String,
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
    #[snafu(display("`GrpcClient__ConnectTimeout={value}` is not a valid duration (e.g. `30s` or `1m`) [{location}]"))]
    #[non_exhaustive]
    InvalidDuration {
        source: humantime::DurationError,
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
    use secrecy::ExposeSecret as _;

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
        // on an mTLS one. Neither path is read from disk before the check, so this needs no fixture.
        for (cert, key) in [("cert.pem", ""), ("", "key.pem")] {
            let error = ClientConfig::from_config_args(ClientConfigArgs {
                cert_pem: String::from(cert),
                key_pem: String::from(key),
                ..args()
            })
            .expect_err("half an identity must be rejected");

            assert!(
                matches!(error, ConfigError::IncompatibleOptions { .. }),
                "{error:?}"
            );
            let rendered = chain(&error);
            assert!(rendered.contains("GrpcClient__CertPem"), "{rendered}");
            assert!(rendered.contains("GrpcClient__KeyPem"), "{rendered}");
        }
    }

    #[test]
    fn neither_half_is_no_identity_rather_than_an_error() {
        let config = ClientConfig::from_config_args(args()).expect("valid");
        assert!(config.identity.is_none());
    }

    #[test]
    fn a_certificate_path_that_does_not_exist_is_reported_with_the_path() {
        // These options are paths, not contents. A typo in one has to name the file rather than surface
        // later as a TLS failure.
        let error = ClientConfig::from_config_args(ClientConfigArgs {
            cert_pem: String::from("no/such/cert.pem"),
            key_pem: String::from("no/such/key.pem"),
            ..args()
        })
        .expect_err("a missing file must be reported");

        assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
        assert!(
            chain(&error).contains("no/such/cert.pem"),
            "{}",
            chain(&error)
        );
    }

    #[test]
    fn a_missing_ca_certificate_is_reported_with_the_path() {
        let error = ClientConfig::from_config_args(ClientConfigArgs {
            ca_cert: String::from("no/such/ca.pem"),
            ..args()
        })
        .expect_err("a missing file must be reported");

        assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
        assert!(
            chain(&error).contains("no/such/ca.pem"),
            "{}",
            chain(&error)
        );
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
    fn arguments_deserialise_with_absent_fields_defaulted() {
        // Every field but the endpoint carries `serde(default)`, so a configuration file need only name
        // what it changes. The feature is off by default, so nothing else here would notice it breaking.
        // Deserialise only: the type deliberately has no `Serialize`, so a proxy password cannot leak
        // through a configuration dump.
        let deserialised: ClientConfigArgs = serde_json::from_str(
            r#"{"endpoint":"http://localhost:5001","timeout":"30s","proxy_password":"s3cr3t"}"#,
        )
        .expect("absent fields should default");

        assert_eq!(deserialised.endpoint, "http://localhost:5001");
        assert_eq!(deserialised.timeout, "30s");
        assert_eq!(deserialised.cert_pem, "", "an absent field defaults");
        assert!(!deserialised.allow_unsafe_connection);
        assert_eq!(deserialised.proxy_password.expose_secret(), "s3cr3t");
    }

    // --- the proxy ---

    /// The configuration `set` produces, expected to be valid.
    fn proxy_config(set: impl FnOnce(&mut ClientConfigArgs)) -> ClientConfig {
        let mut arguments = args();
        set(&mut arguments);
        ClientConfig::from_config_args(arguments).expect("a valid configuration")
    }

    /// The error message `set` produces, expected to be a rejection.
    fn proxy_error(set: impl FnOnce(&mut ClientConfigArgs)) -> String {
        let mut arguments = args();
        set(&mut arguments);
        ClientConfig::from_config_args(arguments)
            .expect_err("the configuration should be rejected")
            .to_string()
    }

    #[test]
    fn proxy_empty_and_system_follow_the_environment() {
        // The same reading as ArmoniK's C# client, where an unset option leaves the handler
        // following the environment.
        for value in ["", "system", "System", "SYSTEM"] {
            let config = proxy_config(|args| args.proxy = String::from(value));
            assert_eq!(config.proxy.source, ProxySource::System, "{value:?}");
        }
    }

    #[test]
    fn proxy_none_forces_a_direct_connection() {
        for value in ["none", "None", "NONE"] {
            let config = proxy_config(|args| args.proxy = String::from(value));
            assert_eq!(config.proxy.source, ProxySource::Disabled, "{value:?}");
        }
    }

    #[test]
    fn proxy_url_defaults_to_the_http_scheme() {
        let with_scheme = proxy_config(|args| args.proxy = String::from("http://proxy.corp:3128"));
        let without_scheme = proxy_config(|args| args.proxy = String::from("proxy.corp:3128"));

        assert_eq!(with_scheme.proxy.source, without_scheme.proxy.source);
        let ProxySource::Explicit(uri) = with_scheme.proxy.source else {
            panic!("expected an explicit proxy");
        };
        assert_eq!(uri.host(), Some("proxy.corp"));
        assert_eq!(uri.port_u16(), Some(3128));
    }

    #[test]
    fn proxy_credentials_in_the_url_are_honoured_and_removed_from_it() {
        let config =
            proxy_config(|args| args.proxy = String::from("http://user:secret@proxy.corp:3128"));

        assert_eq!(config.proxy.username, "user");
        assert_eq!(config.proxy.password.expose_secret(), "secret");
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
        let config = proxy_config(|args| {
            args.proxy = String::from("http://url-user:url-secret@proxy.corp:3128");
            args.proxy_username = String::from("option-user");
            args.proxy_password = "option-secret".into();
        });

        assert_eq!(config.proxy.username, "option-user");
        assert_eq!(config.proxy.password.expose_secret(), "option-secret");
    }

    #[test]
    fn a_dedicated_password_alone_keeps_the_username_the_url_carried() {
        // Replacing the pair rather than each half would leave an empty username here, and the proxy
        // would answer 407 with nothing to explain it.
        let config = proxy_config(|args| {
            args.proxy = String::from("http://url-user:url-secret@proxy.corp:3128");
            args.proxy_password = "option-secret".into();
        });

        assert_eq!(config.proxy.username, "url-user");
        assert_eq!(config.proxy.password.expose_secret(), "option-secret");
    }

    #[test]
    fn the_arguments_keep_the_password_out_of_their_debug_output() {
        // These are what a caller inspects before handing them over, and what a panic message or a
        // log line would render.
        let arguments = ClientConfigArgs {
            proxy: String::from("http://user:url-secret@proxy.corp:3128"),
            proxy_password: "option-secret".into(),
            ..args()
        };

        let rendered = format!("{arguments:?}");
        assert!(
            !rendered.contains("option-secret"),
            "password rendered: {rendered}"
        );
    }

    #[test]
    fn a_rejected_proxy_url_does_not_echo_its_password() {
        // A URL can be rejected for having no host, or for a "port" that is really a password
        // whose `@host` part went missing, and still have carried a credential.
        for value in ["http://user:s3cr3t@", "http://admin:hunter2"] {
            let error = proxy_error(|args| args.proxy = String::from(value));
            assert!(
                !error.contains("s3cr3t") && !error.contains("hunter2"),
                "password echoed for {value:?}: {error}"
            );
        }
    }

    #[test]
    fn a_proxy_that_is_not_http_is_rejected_rather_than_reached_in_the_clear() {
        // The `CONNECT` handshake goes out unencrypted, so a proxy expecting TLS would see gibberish.
        // Accepting the URL and failing at connect time would report it as an unreachable proxy.
        for value in ["https://proxy.corp:3128", "socks5://proxy.corp:1080"] {
            let error = proxy_error(|args| args.proxy = String::from(value));
            assert!(error.contains("only an `http` proxy"), "{value}: {error}");
        }
    }

    #[test]
    fn proxy_without_a_host_is_rejected_and_names_its_own_option() {
        // Reporting these through the endpoint's URI error would send whoever reads it looking at the
        // wrong setting.
        for value in ["http:///no-host", "http://", "http://:3128", "://"] {
            let error = proxy_error(|args| args.proxy = String::from(value));
            assert!(
                error.contains("is not a valid proxy URL"),
                "unexpected error for {value:?}: {error}"
            );
        }
    }

    #[test]
    fn a_port_that_is_not_one_is_rejected_rather_than_dialled_on_80() {
        // `http::Uri` keeps the port as text and parses it lazily, so without the explicit check a
        // typo would pass validation and the connector would quietly fall back to the scheme
        // default.
        for value in [
            "proxy.corp:99999",
            "proxy.corp:31z8",
            "http://admin:hunter2",
        ] {
            let error = proxy_error(|args| args.proxy = String::from(value));
            assert!(
                error.contains("does not name a valid port"),
                "unexpected error for {value:?}: {error}"
            );
        }
    }

    #[test]
    fn an_ipv6_proxy_is_accepted_with_and_without_a_port() {
        // The port check must not mistake the colons of an IPv6 literal for an invalid port.
        for (value, port) in [("http://[::1]:3128", Some(3128)), ("http://[::1]", None)] {
            let config = proxy_config(|args| args.proxy = String::from(value));
            let ProxySource::Explicit(uri) = &config.proxy.source else {
                panic!("expected an explicit proxy for {value:?}");
            };
            assert_eq!(uri.port_u16(), port, "{value:?}");
        }
    }
}
