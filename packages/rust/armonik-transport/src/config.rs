use std::time::Duration;

use hyper::{http::HeaderValue, Uri};
use snafu::{ResultExt, Snafu};

#[cfg(feature = "serde")]
use crate::http2_config::prefix_http2;
use crate::http2_config::{Http2Config, Http2ConfigArgs};
#[cfg(feature = "serde")]
use crate::proxy_config::prefix_proxy;
use crate::proxy_config::{parse_proxy_source, HttpProxyConfig, HttpProxyConfigArgs, ProxySource};
#[cfg(feature = "serde")]
use crate::secret::Secret;
#[cfg(feature = "serde")]
use crate::tcp_config::prefix_tcp;
use crate::tcp_config::{TcpConfig, TcpConfigArgs};
use crate::tls_config::{HttpTlsConfig, HttpTlsConfigArgs};

/// Options for the HTTP/2 transport: TLS, proxy, and everything else `connect` needs to reach the
/// endpoint.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct HttpConfig {
    /// Endpoint for sending requests
    pub endpoint: Uri,
    /// TLS and mTLS: the client's own identity, the server's CA, and SSL verification behaviour.
    pub tls: HttpTlsConfig,
    /// Timeout for establishing a connection to the server, defaults to 60s
    pub connect_timeout: Option<Duration>,
    /// Timeout for each request, defaults to no timeout
    pub timeout: Option<Duration>,
    /// Rate limit for requests, defaults to no rate limit
    pub rate_limit: Option<(u64, Duration)>,
    /// TCP-level socket options.
    pub tcp: TcpConfig,
    /// HTTP/2-level transport options.
    pub http2: Http2Config,
    /// User-Agent header value sent with each request
    pub user_agent: Option<HeaderValue>,
    /// HTTP proxy used to reach the endpoint, defaults to a direct connection
    pub proxy: HttpProxyConfig,
}

/// Options for creating a gRPC Client, in the string form a caller supplies them in
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct HttpConfigArgs {
    /// Endpoint for sending requests
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub endpoint: String,
    /// The client's TLS identity and the server's CA.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub tls: HttpTlsConfigArgs,
    /// Timeout for establishing a connection to the server, defaults to no timeout
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub connect_timeout: String,
    /// Timeout for each request, defaults to no timeout
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub timeout: String,
    /// Rate limit for requests, defaults to no rate limit
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub rate_limit: String,
    /// TCP-level socket options.
    #[cfg_attr(feature = "serde", serde(flatten, with = "prefix_tcp"))]
    pub tcp: TcpConfigArgs,
    /// HTTP/2-level transport options.
    #[cfg_attr(feature = "serde", serde(flatten, with = "prefix_http2"))]
    pub http2: Http2ConfigArgs,
    /// User-Agent header value sent with each request
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub user_agent: String,
    /// HTTP proxy to reach the endpoint through.
    ///
    /// Empty for a direct connection, `none` to disable proxying explicitly, `system` to read the
    /// environment (see [`ProxySource::System`]), otherwise the proxy URL, whose scheme has to be
    /// `http`: the `CONNECT` handshake is written in the clear.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub proxy: String,
    /// The dedicated proxy credentials.
    #[cfg_attr(feature = "serde", serde(flatten, with = "prefix_proxy"))]
    pub proxy_config: HttpProxyConfigArgs,
}

/// Boxes any error as a trait object, for a [`ConfigError`] variant shared by more than one
/// underlying error type.
pub(crate) fn boxed(
    error: impl std::error::Error + Send + Sync + 'static,
) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(error)
}

/// Reads a duration option, empty for `None`.
pub(crate) fn parse_optional_duration(
    option: &'static str,
    value: String,
) -> Result<Option<Duration>, ConfigError> {
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        value
            .parse::<humantime::Duration>()
            .context(InvalidDurationSnafu { option, value })?
            .into(),
    ))
}

/// Reads an integer option, empty for `None`.
pub(crate) fn parse_optional_int<T: std::str::FromStr<Err = std::num::ParseIntError>>(
    option: &'static str,
    value: String,
) -> Result<Option<T>, ConfigError> {
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        value
            .parse::<T>()
            .context(InvalidIntegerSnafu { option, value })?,
    ))
}

/// Reads a boolean option, on the vocabulary every ArmoniK client accepts.
///
/// The parsing lives here rather than in a `Deserialize` impl so that the error can name the option
/// it came from: a `serde` source that flattens its fields buffers values before handing them over,
/// and by then the field's own name is no longer available.
pub(crate) fn parse_bool(option: &'static str, value: &str) -> Result<bool, ConfigError> {
    match value {
        "" | "0" | "false" | "no" | "disable" | "disallow" | "forbid" => Ok(false),
        "1" | "true" | "yes" | "enable" | "allow" | "authorize" => Ok(true),
        _ => InvalidBoolSnafu {
            option,
            value: value.to_owned(),
        }
        .fail(),
    }
}

/// Reads any field of [`HttpConfigArgs`] as text, whatever scalar shape a `serde` source gave it.
///
/// Every field of `HttpConfigArgs` is authoritatively text, in the string form its own doc names,
/// but a source is not obliged to agree: `figment`'s `Env` provider parses a bare `3` or `true` into
/// a real integer or boolean before `serde` ever sees it, and a plain `String` field rejects those
/// outright. The same provider parses a value made entirely of a bracketed or braced list (`[::1]`,
/// with nothing before or after the brackets) into a list or object the same way, which a value's own
/// option cannot be, so that shape is refused with a message naming the escape hatch: a literal pair
/// of double quotes around the value (`"[::1]"`) forces it to be read as a string instead.
#[cfg(feature = "serde")]
pub(crate) fn text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    struct AnyScalar;

    impl<'de> serde::de::Visitor<'de> for AnyScalar {
        type Value = String;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a string, or a number or boolean spelling one")
        }

        fn visit_bool<E>(self, value: bool) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E> {
            Ok(value.to_owned())
        }

        fn visit_string<E>(self, value: String) -> Result<String, E> {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_f64<E>(self, value: f64) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, _seq: A) -> Result<String, A::Error> {
            Err(serde::de::Error::custom(
                "a value made entirely of a bracketed list reads as one, not as text; wrap it in a \
                 literal pair of double quotes (e.g. `\"[::1]\"`) to read it as a string",
            ))
        }

        fn visit_map<A: serde::de::MapAccess<'de>>(self, _map: A) -> Result<String, A::Error> {
            Err(serde::de::Error::custom(
                "a value made entirely of a braced object reads as one, not as text; wrap it in a \
                 literal pair of double quotes to read it as a string",
            ))
        }
    }

    deserializer.deserialize_any(AnyScalar)
}

/// [`text`], for the two fields whose value is a [`Secret`] rather than a plain `String`.
#[cfg(feature = "serde")]
pub(crate) fn secret_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Secret, D::Error> {
    text(deserializer).and_then(Secret::from_decoded_text)
}

impl HttpConfig {
    pub fn from_config_args(args: HttpConfigArgs) -> Result<Self, ConfigError> {
        let _span = tracing::debug_span!(
            "HttpConfig",
            args.endpoint,
            args.tls.cert_pem,
            args.tls.key_pem,
            args.tls.ca_cert,
            args.tls.cert_p12,
            // `cert_p12_password` left out entirely, the same as `proxy_password` below: a path
            // names no secret, but a password does.
            args.tls.allow_unsafe_connection,
            args.tls.override_target_name,
            args.connect_timeout,
            args.timeout,
            args.rate_limit,
            args.tcp.keepalive,
            args.tcp.keepalive_interval,
            args.tcp.keepalive_retries,
            args.tcp.nagle_algorithm,
            args.http2.keep_alive_interval,
            args.http2.keep_alive_timeout,
            args.http2.keep_alive_while_idle,
            args.http2.max_header_list_size,
            args.user_agent,
            // Elided, and `proxy_password` left out entirely: this span is recorded at debug level, and
            // a proxy URL carries credentials as often as the dedicated option does. Do not complete
            // the list.
            proxy = %crate::proxy::elide_userinfo(&args.proxy),
            args.proxy_config.username,
        );

        let HttpConfigArgs {
            endpoint,
            tls,
            connect_timeout,
            timeout,
            rate_limit,
            tcp,
            http2,
            user_agent,
            proxy,
            proxy_config,
        } = args;

        let endpoint = Uri::try_from(endpoint.clone())
            .map_err(boxed)
            .context(UriSnafu {
                option: "endpoint",
                uri: endpoint,
            })?;
        let tls = tls.resolve(&endpoint)?;

        let connect_timeout = Some(
            parse_optional_duration("connect_timeout", connect_timeout)?
                .unwrap_or(Duration::from_secs(60)),
        );

        let timeout = parse_optional_duration("timeout", timeout)?;

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

        let tcp = tcp.resolve()?;
        let http2 = http2.resolve()?;

        let user_agent = if user_agent.is_empty() {
            None
        } else {
            let header = HeaderValue::from_str(&user_agent)
                .context(InvalidUserAgentSnafu { value: user_agent })?;
            Some(header)
        };

        let proxy = match parse_proxy_source(&proxy)? {
            // Through the constructor, so credentials written into the URL are taken out of it.
            ProxySource::Explicit(uri) => HttpProxyConfig::explicit(uri),
            ProxySource::System => HttpProxyConfig::system(),
            ProxySource::Disabled => HttpProxyConfig::default(),
        };
        let proxy = proxy_config.merge_into(proxy);

        Ok(Self {
            endpoint,
            tls,
            connect_timeout,
            timeout,
            rate_limit,
            tcp,
            http2,
            user_agent,
            proxy,
        })
    }
}

impl TryFrom<&HttpConfig> for tonic::transport::Endpoint {
    type Error = ConfigError;

    fn try_from(value: &HttpConfig) -> Result<Self, Self::Error> {
        Ok(Self::from(value.endpoint.clone()))
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
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
    #[snafu(display("`{option}={uri}` is not a valid URI [{location}]"))]
    #[non_exhaustive]
    Uri {
        source: Box<dyn std::error::Error + Send + Sync>,
        option: &'static str,
        uri: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not read `{option}`'s file `{path}` [{location}]"))]
    #[non_exhaustive]
    Io {
        #[snafu(source(from(std::io::Error, Box::new)))]
        source: Box<std::io::Error>,
        option: &'static str,
        path: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("`cert_p12`'s file `{path}` is not a valid PKCS#12 bundle [{location}]"))]
    #[non_exhaustive]
    Pkcs12 {
        #[snafu(source(from(p12_keystore::error::Error, Box::new)))]
        source: Box<p12_keystore::error::Error>,
        path: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "`cert_p12`'s file `{path}` carries no private key and certificate chain [{location}]"
    ))]
    #[non_exhaustive]
    EmptyPkcs12 {
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
    #[snafu(display(
        "`{option}={value}` is not a valid boolean (e.g. `true`, `1`, `yes`, or `false`, `0`, `no`) [{location}]"
    ))]
    #[non_exhaustive]
    InvalidBool {
        option: &'static str,
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("`{option}={value}` is not a valid integer [{location}]"))]
    #[non_exhaustive]
    InvalidInteger {
        source: std::num::ParseIntError,
        option: &'static str,
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
    use rustls::pki_types::PrivateKeyDer;

    use super::*;
    use crate::secret::Secret;

    /// The minimum viable arguments: an endpoint, and nothing else set.
    fn args() -> HttpConfigArgs {
        HttpConfigArgs {
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
        let config = HttpConfig::from_config_args(args()).expect("an endpoint is enough");

        assert_eq!(config.endpoint.to_string(), "http://localhost:5001/");
        assert!(config.tls.identity.is_none());
        assert!(config.tls.cacert.is_none());
        assert_eq!(config.tls.override_target, None);
        assert_eq!(config.rate_limit, None);
    }

    #[test]
    fn an_endpoint_that_is_not_a_uri_is_reported() {
        let error = HttpConfig::from_config_args(HttpConfigArgs {
            endpoint: String::new(),
            ..args()
        })
        .expect_err("an empty endpoint is not a URI");

        assert!(matches!(error, ConfigError::Uri { .. }), "{error:?}");
    }

    // --- durations and numbers ---

    #[test]
    fn durations_are_read_in_the_units_they_are_written_in() {
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            connect_timeout: String::from("500ms"),
            tcp: TcpConfigArgs {
                keepalive: String::from("30s"),
                keepalive_interval: String::from("2m"),
                ..Default::default()
            },
            http2: Http2ConfigArgs {
                keep_alive_interval: String::from("1h"),
                ..Default::default()
            },
            ..args()
        })
        .expect("valid durations");

        assert_eq!(config.connect_timeout, Some(Duration::from_millis(500)));
        assert_eq!(config.tcp.keepalive, Some(Duration::from_secs(30)));
        assert_eq!(
            config.tcp.keepalive_interval,
            Some(Duration::from_secs(120))
        );
        assert_eq!(
            config.http2.keep_alive_interval,
            Some(Duration::from_secs(3600))
        );
    }

    #[test]
    fn a_duration_that_cannot_be_parsed_names_the_value() {
        let error = HttpConfig::from_config_args(HttpConfigArgs {
            tcp: TcpConfigArgs {
                keepalive: String::from("soon"),
                ..Default::default()
            },
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
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            tcp: TcpConfigArgs {
                keepalive_retries: String::from("3"),
                ..Default::default()
            },
            http2: Http2ConfigArgs {
                max_header_list_size: String::from("16384"),
                ..Default::default()
            },
            ..args()
        })
        .expect("valid integers");
        assert_eq!(config.tcp.keepalive_retries, Some(3));
        assert_eq!(config.http2.max_header_list_size, Some(16384));

        let error = HttpConfig::from_config_args(HttpConfigArgs {
            tcp: TcpConfigArgs {
                keepalive_retries: String::from("many"),
                ..Default::default()
            },
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
        let error = HttpConfig::from_config_args(HttpConfigArgs {
            http2: Http2ConfigArgs {
                max_header_list_size: String::from("4294967296"),
                ..Default::default()
            },
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
        let config = HttpConfig::from_config_args(HttpConfigArgs {
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
            let error = HttpConfig::from_config_args(HttpConfigArgs {
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
        let error = HttpConfig::from_config_args(HttpConfigArgs {
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
        let count = HttpConfig::from_config_args(HttpConfigArgs {
            rate_limit: String::from("plenty/1s"),
            ..args()
        })
        .expect_err("`plenty` is not a count");
        assert!(
            matches!(count, ConfigError::InvalidRateLimitCount { .. }),
            "{count:?}"
        );

        let duration = HttpConfig::from_config_args(HttpConfigArgs {
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
        for (cert, key) in [
            (String::from("cert.pem"), String::new()),
            (String::new(), String::from("key.pem")),
        ] {
            let error = HttpConfig::from_config_args(HttpConfigArgs {
                tls: HttpTlsConfigArgs {
                    cert_pem: cert,
                    key_pem: key,
                    ..Default::default()
                },
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
        let config = HttpConfig::from_config_args(args()).expect("valid");
        assert!(config.tls.identity.is_none());
    }

    #[test]
    fn content_that_is_not_pem_fails_as_such() {
        // Garbage content gets a PEM error naming what was wrong, rather than this crate silently
        // accepting it.
        let mut cert = tempfile::NamedTempFile::new().expect("temp file");
        std::io::Write::write_all(&mut cert, b"not a certificate").expect("write");
        let mut key = tempfile::NamedTempFile::new().expect("temp file");
        std::io::Write::write_all(&mut key, b"not a key").expect("write");

        for args in [
            HttpConfigArgs {
                tls: HttpTlsConfigArgs {
                    cert_pem: cert.path().to_str().expect("utf8 path").to_owned(),
                    key_pem: key.path().to_str().expect("utf8 path").to_owned(),
                    ..Default::default()
                },
                ..args()
            },
            HttpConfigArgs {
                tls: HttpTlsConfigArgs {
                    ca_cert: cert.path().to_str().expect("utf8 path").to_owned(),
                    ..Default::default()
                },
                ..args()
            },
        ] {
            let error = HttpConfig::from_config_args(args)
                .expect_err("garbage content is not a certificate");

            assert!(matches!(error, ConfigError::Tls { .. }), "{error:?}");
        }
    }

    #[test]
    fn a_path_that_leads_nowhere_names_the_option_and_the_path() {
        for args in [
            HttpConfigArgs {
                tls: HttpTlsConfigArgs {
                    cert_pem: String::from("no/such/cert.pem"),
                    key_pem: String::from("no/such/key.pem"),
                    ..Default::default()
                },
                ..args()
            },
            HttpConfigArgs {
                tls: HttpTlsConfigArgs {
                    ca_cert: String::from("no/such/ca.pem"),
                    ..Default::default()
                },
                ..args()
            },
        ] {
            let error =
                HttpConfig::from_config_args(args).expect_err("a missing file must be reported");

            assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
            let rendered = chain(&error);
            assert!(rendered.contains("no/such/"), "{rendered}");
        }
    }

    #[test]
    fn a_path_to_a_real_file_is_actually_read() {
        // Proven by the error changing kind, not by a successful parse: generating a real certificate
        // is more than this needs. A file that exists but holds no certificate must fail as `Tls`, not
        // as `Io`, or the path was never opened at all.
        let mut ca = tempfile::NamedTempFile::new().expect("temp file");
        std::io::Write::write_all(&mut ca, b"clearly not a certificate").expect("write");

        let error = HttpConfig::from_config_args(HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                ca_cert: ca.path().to_str().expect("utf8 path").to_owned(),
                ..Default::default()
            },
            ..args()
        })
        .expect_err("this file holds no certificate");

        assert!(matches!(error, ConfigError::Tls { .. }), "{error:?}");
    }

    #[test]
    fn cert_p12_and_the_pem_pair_are_mutually_exclusive() {
        let error = HttpConfig::from_config_args(HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                cert_pem: String::from("cert.pem"),
                key_pem: String::from("key.pem"),
                cert_p12: String::from("identity.p12"),
                ..Default::default()
            },
            ..args()
        })
        .expect_err("both forms of identity must be rejected");

        assert!(
            matches!(error, ConfigError::IncompatibleOptions { .. }),
            "{error:?}"
        );
        let rendered = chain(&error);
        assert!(rendered.contains("cert_p12"), "{rendered}");
    }

    #[test]
    fn a_p12_password_without_a_p12_is_rejected() {
        let error = HttpConfig::from_config_args(HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                cert_p12_password: Secret::from("s3cr3t"),
                ..Default::default()
            },
            ..args()
        })
        .expect_err("a password naming no file must be rejected");

        assert!(
            matches!(error, ConfigError::IncompatibleOptions { .. }),
            "{error:?}"
        );
        let rendered = chain(&error);
        assert!(rendered.contains("cert_p12_password"), "{rendered}");
        assert!(!rendered.contains("s3cr3t"), "{rendered}");
    }

    #[test]
    fn a_p12_file_is_read_into_the_same_identity_a_pem_pair_would_be() {
        // A self-signed certificate and its PKCS#8 key, generated fresh here rather than read from
        // a fixture, then bundled into a PKCS#12 file with `p12-keystore`'s own writer.
        const PASSWORD: &str = "s3cr3t";

        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(["test".to_owned()]).expect("a self-signed cert");
        let cert_der = cert.der().to_vec();
        let key_der = signing_key.serialize_der();

        let chain = p12_keystore::PrivateKeyChain::new(
            [1u8].as_slice(),
            p12_keystore::PrivateKey::from_der(&key_der).expect("a valid PKCS#8 key"),
            [p12_keystore::Certificate::from_der(&cert_der).expect("a valid X.509 certificate")],
        );
        let mut keystore = p12_keystore::KeyStore::new();
        keystore.add_entry(
            "identity",
            p12_keystore::KeyStoreEntry::PrivateKeyChain(chain),
        );
        let pfx = keystore.writer(PASSWORD).write().expect("write the bundle");

        let mut p12 = tempfile::NamedTempFile::new().expect("temp file");
        std::io::Write::write_all(&mut p12, &pfx).expect("write");

        let config = HttpConfig::from_config_args(HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                cert_p12: p12.path().to_str().expect("utf8 path").to_owned(),
                cert_p12_password: Secret::from(PASSWORD),
                ..Default::default()
            },
            ..args()
        })
        .expect("a valid PKCS#12 bundle");

        let (cert, key) = config.tls.identity.expect("an identity was bundled");
        assert_eq!(cert.as_ref(), cert_der, "the leaf certificate round-trips");
        let PrivateKeyDer::Pkcs8(key) = key else {
            panic!("expected the PKCS#8 variant, since that is what the bundle carried");
        };
        assert_eq!(
            key.secret_pkcs8_der(),
            key_der.as_slice(),
            "the key round-trips"
        );
    }

    #[test]
    fn a_p12_that_leads_nowhere_names_the_path() {
        let error = HttpConfig::from_config_args(HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                cert_p12: String::from("no/such/identity.p12"),
                ..Default::default()
            },
            ..args()
        })
        .expect_err("a missing file must be reported");

        assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
        assert!(chain(&error).contains("no/such/"), "{}", chain(&error));
    }

    #[test]
    fn a_p12_file_that_is_not_pkcs12_is_rejected_as_such() {
        let mut p12 = tempfile::NamedTempFile::new().expect("temp file");
        std::io::Write::write_all(&mut p12, b"clearly not a pkcs12 bundle").expect("write");

        let error = HttpConfig::from_config_args(HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                cert_p12: p12.path().to_str().expect("utf8 path").to_owned(),
                ..Default::default()
            },
            ..args()
        })
        .expect_err("garbage is not a pkcs12 bundle");

        assert!(matches!(error, ConfigError::Pkcs12 { .. }), "{error:?}");
    }

    // --- override target ---

    #[test]
    fn an_override_target_given_as_a_host_keeps_the_endpoints_scheme_and_path() {
        // The common case: the certificate names one host, the endpoint is reached at another. Only the
        // authority is being overridden, so everything else has to come from the endpoint.
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            endpoint: String::from("https://10.0.0.1:5003/base"),
            tls: HttpTlsConfigArgs {
                override_target_name: String::from("server.example.com"),
                ..Default::default()
            },
            ..args()
        })
        .expect("valid");

        let override_target = config.tls.override_target.expect("an override target");
        assert_eq!(override_target.scheme_str(), Some("https"));
        assert_eq!(
            override_target.authority().map(|a| a.as_str()),
            Some("server.example.com")
        );
        assert_eq!(override_target.path(), "/base");
    }

    #[test]
    fn an_override_target_given_as_a_uri_replaces_the_authority_and_the_path() {
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            endpoint: String::from("https://10.0.0.1:5003/base"),
            tls: HttpTlsConfigArgs {
                override_target_name: String::from("https://server.example.com/other"),
                ..Default::default()
            },
            ..args()
        })
        .expect("valid");

        let override_target = config.tls.override_target.expect("an override target");
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
        let config = HttpConfig::from_config_args(args()).expect("valid");
        assert_eq!(config.tls.override_target, None);
    }

    // --- the serde feature ---

    /// The rendered message is what a user actually reads, so assert it whole rather than by
    /// fragments: a stray space or a missing spelling only shows up here.
    #[test]
    fn an_unusable_boolean_names_the_option_and_the_vocabulary() {
        let rendered = HttpConfig::from_config_args(HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                allow_unsafe_connection: String::from("perhaps"),
                ..Default::default()
            },
            ..args()
        })
        .expect_err("`perhaps` spells no boolean")
        .to_string();

        let (message, location) = rendered
            .rsplit_once(" [")
            .expect("every error in this crate carries its location");
        // `concat!` rather than a `\`-continued literal: that one keeps the source indentation in
        // the string, which is exactly the defect this test exists to catch.
        assert_eq!(
            message,
            concat!(
                "`allow_unsafe_connection=perhaps` is not a valid boolean ",
                "(e.g. `true`, `1`, `yes`, or `false`, `0`, `no`)"
            )
        );
        assert!(location.ends_with(']'), "{rendered}");
    }

    /// A document may spell a boolean option the way its own format does, rather than the way an
    /// environment variable has to.
    #[cfg(feature = "serde")]
    #[test]
    fn a_boolean_option_accepts_the_spelling_its_source_writes_naturally() {
        for (written, expected) in [
            (r#"{"AllowUnsafeConnection":true}"#, true),
            (r#"{"AllowUnsafeConnection":false}"#, false),
            (r#"{"AllowUnsafeConnection":"yes"}"#, true),
            (r#"{"AllowUnsafeConnection":1}"#, true),
            (r#"{"AllowUnsafeConnection":0}"#, false),
        ] {
            let mut args: HttpConfigArgs =
                serde_json::from_str(written).unwrap_or_else(|error| panic!("{written}: {error}"));
            args.endpoint = String::from("http://localhost:5001");

            let config = HttpConfig::from_config_args(args)
                .unwrap_or_else(|error| panic!("{written}: {error}"));

            assert_eq!(
                config.tls.allow_unsafe_connection, expected,
                "{written} should resolve to {expected}"
            );
        }
    }

    /// What this crate writes, this crate reads: the options are textual, so a boolean comes back as
    /// the word it was resolved from rather than as the source's own boolean.
    #[cfg(feature = "serde")]
    #[test]
    fn a_boolean_option_survives_being_written_and_read_again() {
        let written = serde_json::to_string(&HttpConfigArgs {
            tls: HttpTlsConfigArgs {
                allow_unsafe_connection: String::from("yes"),
                ..Default::default()
            },
            ..args()
        })
        .expect("serialise");

        assert!(
            written.contains(r#""AllowUnsafeConnection":"yes""#),
            "kept as written: {written}"
        );

        let read: HttpConfigArgs = serde_json::from_str(&written).expect("read back");
        let config = HttpConfig::from_config_args(read).expect("resolve");

        assert!(config.tls.allow_unsafe_connection);
    }

    /// The point of grouping each thematic unit's own fields with `with_prefix!` rather than a
    /// plain nested object: not one JSON key changes, so an existing document, or an existing
    /// environment variable, still reads.
    #[cfg(feature = "serde")]
    #[test]
    fn a_thematic_units_own_fields_still_serialise_as_flat_top_level_keys() {
        let args = HttpConfigArgs {
            tcp: TcpConfigArgs {
                keepalive: String::from("30s"),
                ..Default::default()
            },
            http2: Http2ConfigArgs {
                max_header_list_size: String::from("16384"),
                ..Default::default()
            },
            ..Default::default()
        };

        let written = serde_json::to_string(&args).expect("serialise");
        assert!(
            written.contains(r#""TcpKeepalive":"30s""#),
            "flat, not nested under \"Tcp\": {written}"
        );
        assert!(!written.contains(r#""Tcp":"#), "{written}");
        assert!(
            written.contains(r#""Http2MaxHeaderListSize":"16384""#),
            "flat, not nested under \"Http2\": {written}"
        );
        assert!(!written.contains(r#""Http2":"#), "{written}");

        let read: HttpConfigArgs =
            serde_json::from_str(r#"{"TcpKeepalive":"5s","Http2MaxHeaderListSize":"8192"}"#)
                .expect("read the flat keys back");
        assert_eq!(read.tcp.keepalive, "5s");
        assert_eq!(read.http2.max_header_list_size, "8192");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn arguments_round_trip_through_serde_with_absent_fields_defaulted() {
        // The struct carries `serde(default)`, so a configuration file need only name what it
        // changes; an absent `endpoint` fails later, as `ConfigError::Uri`, not here. The feature is
        // off by default, so nothing else here would notice it breaking.
        let deserialised: HttpConfigArgs =
            serde_json::from_str(r#"{"Timeout":"30s"}"#).expect("absent fields should default");

        assert_eq!(deserialised.endpoint, "", "an absent field defaults");
        assert_eq!(deserialised.timeout, "30s");
        assert_eq!(deserialised.tls.cert_pem, "", "an absent field defaults");
        assert_eq!(
            deserialised.tls.allow_unsafe_connection, "",
            "an absent boolean defaults to empty, which reads as false"
        );

        let round_tripped: HttpConfigArgs =
            serde_json::from_str(&serde_json::to_string(&deserialised).expect("serialise"))
                .expect("deserialise");
        assert_eq!(round_tripped, deserialised);
    }

    // --- the proxy ---

    #[test]
    fn proxy_none_and_empty_disable_proxying() {
        for value in ["", "none", "None", "NONE"] {
            let config = HttpConfig::from_config_args(HttpConfigArgs {
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
            let config = HttpConfig::from_config_args(HttpConfigArgs {
                proxy: String::from(value),
                ..args()
            })
            .expect("a valid configuration");
            assert_eq!(config.proxy.source, ProxySource::System, "{value:?}");
        }
    }

    #[test]
    fn proxy_url_defaults_to_the_http_scheme() {
        let with_scheme = HttpConfig::from_config_args(HttpConfigArgs {
            proxy: String::from("http://proxy.corp:3128"),
            ..args()
        })
        .expect("a valid configuration");
        let without_scheme = HttpConfig::from_config_args(HttpConfigArgs {
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
        let none = HttpConfig::from_config_args(HttpConfigArgs {
            proxy: String::from("proxy.corp:3128"),
            ..args()
        })
        .expect("a valid configuration");
        assert_eq!(none.proxy.credentials(), None);

        let some = HttpConfig::from_config_args(HttpConfigArgs {
            proxy: String::from("proxy.corp:3128"),
            proxy_config: HttpProxyConfigArgs {
                username: String::from("user"),
                password: "secret".into(),
            },
            ..args()
        })
        .expect("a valid configuration");
        assert_eq!(some.proxy.credentials(), Some(("user", "secret")));
    }

    #[test]
    fn proxy_credentials_in_the_url_are_honoured_and_removed_from_it() {
        let config = HttpConfig::from_config_args(HttpConfigArgs {
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
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            proxy: String::from("http://url-user:url-secret@proxy.corp:3128"),
            proxy_config: HttpProxyConfigArgs {
                username: String::from("option-user"),
                password: "option-secret".into(),
            },
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
        // `HttpConfig` is `Debug` and holds a `HttpProxyConfig`, so a derived `Debug` would put the
        // password anywhere a configuration gets printed.
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            proxy: String::from("proxy.corp:3128"),
            proxy_config: HttpProxyConfigArgs {
                username: String::from("user"),
                password: "s3cr3t".into(),
            },
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
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            proxy: String::from("http://url-user:url-secret@proxy.corp:3128"),
            proxy_config: HttpProxyConfigArgs {
                password: "option-secret".into(),
                ..Default::default()
            },
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
        let error = HttpConfig::from_config_args(HttpConfigArgs {
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
        // `HttpConfig` is not the only type that holds it: these are what a caller inspects before
        // handing them over.
        let args = HttpConfigArgs {
            proxy: String::from("http://user:url-secret@proxy.corp:3128"),
            proxy_config: HttpProxyConfigArgs {
                password: "option-secret".into(),
                ..Default::default()
            },
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
        let args = HttpConfigArgs {
            proxy_config: HttpProxyConfigArgs {
                password: "s3cr3t".into(),
                ..Default::default()
            },
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
            let error = HttpConfig::from_config_args(HttpConfigArgs {
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
            let error = HttpConfig::from_config_args(HttpConfigArgs {
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
}
