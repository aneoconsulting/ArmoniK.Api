//! The client configuration, and how it is read.
//!
//! [`HttpConfig`] is plain data: every field is typed, public, and buildable by hand. Under the
//! `serde` feature it also deserialises directly from a flat document of `PascalCase` options
//! (`Endpoint`, `TcpKeepalive`, `Http2KeepAliveInterval`, ...), the same vocabulary every ArmoniK
//! client reads; the thematic groups ([`TlsConfig`], [`TcpConfig`], [`Http2Config`],
//! [`ProxyConfig`]) flatten back into those names, so grouping the fields changes no option a
//! deployment already sets.

use std::time::Duration;

use hyper::{http::HeaderValue, Uri};
use snafu::Snafu;

#[cfg(feature = "serde")]
use crate::http2_config::prefix_http2;
use crate::http2_config::Http2Config;
use crate::proxy::ProxyConfig;
#[cfg(feature = "serde")]
use crate::tcp_config::prefix_tcp;
use crate::tcp_config::TcpConfig;
use crate::tls_config::TlsConfig;

/// Timeout for establishing a connection when the option is left unset.
pub(crate) const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// Options for creating a gRPC client.
///
/// Deserializable but deliberately not serializable: the proxy password is a secret, and a type
/// that cannot be written out cannot leak it through a configuration dump. `Debug` redacts it for
/// the same reason.
///
/// Every option is spelled the way its own documentation names it (`Endpoint`, `TcpKeepalive`,
/// ...), an empty string reads as the option's default, and a `serde` source may hand a bare
/// number or boolean over in its own type rather than as text.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct HttpConfig {
    /// Endpoint for sending requests. `Endpoint`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "endpoint"))]
    pub endpoint: Uri,
    /// TLS and mTLS: the client's own identity, the server's CA, and SSL verification behaviour.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub tls: TlsConfig,
    /// Timeout for establishing a connection to the server, defaults to 60s. `ConnectTimeout`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "connect_timeout"))]
    pub connect_timeout: Option<Duration>,
    /// Timeout for each request, defaults to no timeout. `Timeout`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "timeout"))]
    pub timeout: Option<Duration>,
    /// Rate limit for requests, written `count/duration` (e.g. `100/1s`), defaults to no rate
    /// limit. `RateLimit`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "rate_limit"))]
    pub rate_limit: Option<(u64, Duration)>,
    /// TCP-level socket options, read under the `Tcp` prefix (`TcpKeepalive`, ...).
    #[cfg_attr(feature = "serde", serde(flatten, with = "prefix_tcp"))]
    pub tcp: TcpConfig,
    /// HTTP/2-level transport options, read under the `Http2` prefix (`Http2KeepAliveInterval`,
    /// ...).
    #[cfg_attr(feature = "serde", serde(flatten, with = "prefix_http2"))]
    pub http2: Http2Config,
    /// User-Agent header value sent with each request. `UserAgent`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "user_agent"))]
    pub user_agent: Option<HeaderValue>,
    /// HTTP proxy used to reach the endpoint (`Proxy`, `ProxyUsername`, `ProxyPassword`), defaults
    /// to following the environment.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub proxy: ProxyConfig,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            endpoint: Uri::default(),
            tls: TlsConfig::default(),
            connect_timeout: Some(DEFAULT_CONNECT_TIMEOUT),
            timeout: None,
            rate_limit: None,
            tcp: TcpConfig::default(),
            http2: Http2Config::default(),
            user_agent: None,
            proxy: ProxyConfig::default(),
        }
    }
}

impl TryFrom<&HttpConfig> for tonic::transport::Endpoint {
    type Error = ConfigError;

    fn try_from(value: &HttpConfig) -> Result<Self, Self::Error> {
        Ok(Self::from(value.endpoint.clone()))
    }
}

/// Reads any option as text, whatever scalar shape a `serde` source gave it.
///
/// Every option is authoritatively text, in the spelling its own doc names, but a source is not
/// obliged to agree: `figment`'s `Env` provider parses a bare `3` or `true` into a real integer or
/// boolean before `serde` ever sees it, and a plain `String` field rejects those outright. The
/// same provider parses a value made entirely of a bracketed or braced list (`[::1]`, with nothing
/// before or after the brackets) into a list or object the same way, which a value's own option
/// cannot be, so that shape is refused with a message naming the escape hatch: a literal pair of
/// double quotes around the value (`"[::1]"`) forces it to be read as a string instead.
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

/// [`text`], for the secret-valued options this crate reads.
///
/// A numeric-looking password may arrive as a real number the same way any other option can, and
/// rejecting it would make some passwords unusable.
#[cfg(feature = "serde")]
pub(crate) fn secret_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<secrecy::SecretString, D::Error> {
    text(deserializer).map(secrecy::SecretString::from)
}

/// Reads a boolean option, on the vocabulary every ArmoniK client accepts. `Err` carries the
/// message, naming `option`: a flattened `serde` source buffers values before handing them over,
/// so by the time one is interpreted the field's own name is no longer available, and each caller
/// has to supply it.
#[cfg(feature = "serde")]
pub(crate) fn parse_bool(option: &str, value: &str) -> Result<bool, String> {
    match value {
        "" | "0" | "false" | "no" | "disable" | "disallow" | "forbid" => Ok(false),
        "1" | "true" | "yes" | "enable" | "allow" | "authorize" => Ok(true),
        _ => Err(format!(
            "`{option}={value}` is not a valid boolean (e.g. `true`, `1`, `yes`, or `false`, `0`, \
             `no`)"
        )),
    }
}

/// [`parse_bool`], as a `deserialize_with` body.
#[cfg(feature = "serde")]
pub(crate) fn bool_option<'de, D: serde::Deserializer<'de>>(
    option: &'static str,
    deserializer: D,
) -> Result<bool, D::Error> {
    parse_bool(option, &text(deserializer)?).map_err(serde::de::Error::custom)
}

/// Reads a duration option, empty for `None`. `Err` names `option`, for the same reason
/// [`parse_bool`] takes it: the field's own name is gone by the time the value is read.
#[cfg(feature = "serde")]
pub(crate) fn optional_duration<'de, D: serde::Deserializer<'de>>(
    option: &'static str,
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(None);
    }
    match value.parse::<humantime::Duration>() {
        Ok(duration) => Ok(Some(duration.into())),
        Err(error) => Err(serde::de::Error::custom(format!(
            "`{option}={value}` is not a valid duration (e.g. `30s` or `1m`): {error}"
        ))),
    }
}

/// Reads an integer option, empty for `None`. `Err` names `option`, like [`optional_duration`].
#[cfg(feature = "serde")]
pub(crate) fn optional_u32<'de, D: serde::Deserializer<'de>>(
    option: &'static str,
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(None);
    }
    match value.parse::<u32>() {
        Ok(int) => Ok(Some(int)),
        Err(error) => Err(serde::de::Error::custom(format!(
            "`{option}={value}` is not a valid integer: {error}"
        ))),
    }
}

/// Reads the endpoint, empty for the default [`Uri`].
///
/// An absent or empty endpoint is `connect`'s problem to reject with a named error, not this
/// field's to refuse up front: a configuration file need only name what it changes.
#[cfg(feature = "serde")]
fn endpoint<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Uri, D::Error> {
    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(Uri::default());
    }
    Uri::try_from(&value).map_err(|error| {
        serde::de::Error::custom(format!("`Endpoint={value}` is not a valid URI: {error}"))
    })
}

/// Reads the connect timeout, empty for the field's own default of 60s rather than `None`: an
/// absent bound would let a connection attempt hang for as long as the OS allows.
#[cfg(feature = "serde")]
fn connect_timeout<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    Ok(optional_duration("ConnectTimeout", deserializer)?.or(Some(DEFAULT_CONNECT_TIMEOUT)))
}

/// [`optional_duration`], naming this field's own option.
#[cfg(feature = "serde")]
fn timeout<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    optional_duration("Timeout", deserializer)
}

/// Reads the User-Agent option, empty for `None`.
#[cfg(feature = "serde")]
fn user_agent<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<HeaderValue>, D::Error> {
    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(None);
    }
    match HeaderValue::from_str(&value) {
        Ok(header) => Ok(Some(header)),
        Err(error) => Err(serde::de::Error::custom(format!(
            "Invalid user agent `{value}`: {error}"
        ))),
    }
}

/// Reads the rate limit, `count/duration`, empty for `None`.
#[cfg(feature = "serde")]
fn rate_limit<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<(u64, Duration)>, D::Error> {
    use serde::de::Error;

    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(None);
    }
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 2 {
        return Err(Error::custom(format!(
            "Rate limit should be in the format `number/duration`, e.g. `100/1s`, but got \
             `{value}`"
        )));
    }
    let limit = parts[0].parse::<u64>().map_err(|error| {
        Error::custom(format!(
            "Rate limit count `{}` is not a valid integer: {error}",
            parts[0]
        ))
    })?;
    let duration: Duration = parts[1]
        .parse::<humantime::Duration>()
        .map_err(|error| {
            Error::custom(format!(
                "`{value}` is not a valid duration (e.g. `30s` or `1m`): {error}"
            ))
        })?
        .into();
    // `tower`'s rate limiter asserts both are non-zero, so leaving these to it turns a mistyped
    // option into a panic inside `connect` rather than an error the caller can read.
    if limit == 0 || duration.is_zero() {
        return Err(Error::custom(format!(
            "`RateLimit={value}` has a zero count or duration. Both have to be above zero, as in \
             `100/1s`; leave it empty for no rate limit"
        )));
    }
    Ok(Some((limit, duration)))
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
    #[snafu(display("`CertP12`'s file `{path}` is not a valid PKCS#12 bundle [{location}]"))]
    #[non_exhaustive]
    Pkcs12 {
        #[snafu(source(from(p12_keystore::error::Error, Box::new)))]
        source: Box<p12_keystore::error::Error>,
        path: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "`CertP12`'s file `{path}` carries no private key and certificate chain [{location}]"
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
}

/// The JSON schemas of [`HttpConfig`], hand-written as mirror types rather than derived from the
/// config types: [`serde_with::with_prefix!`] has no `schemars` integration, so a derived schema
/// would list `Keepalive` where the flat option is `TcpKeepalive`.
///
/// Two schemas, for two consumers. The flat one ([`schemars::JsonSchema`] on [`HttpConfig`]
/// itself) is the option vocabulary exactly as deserialisation accepts it: every option text,
/// none required, no nesting, which is what a C# options-class generator consumes. The structured
/// one ([`HttpConfig::structured_schema`]) mirrors the config's own shape, thematic units and
/// identity variants included, for a consumer that needs the semantics rather than the wire
/// spelling; it corresponds to no `Deserialize` path.
#[cfg(feature = "schema")]
mod schema {
    use std::borrow::Cow;

    /// Options for creating a gRPC client.
    #[derive(schemars::JsonSchema)]
    #[schemars(rename = "HttpConfig", rename_all = "PascalCase")]
    #[allow(dead_code)]
    struct HttpConfigSchema {
        /// Endpoint for sending requests.
        endpoint: Option<String>,
        /// Path to the client certificate file, in PEM format; set together with `KeyPem`.
        cert_pem: Option<String>,
        /// Path to the client key file, in PEM format; set together with `CertPem`.
        key_pem: Option<String>,
        /// Path to the client certificate and key bundled together, in PKCS#12 format; mutually
        /// exclusive with `CertPem`/`KeyPem`.
        cert_p12: Option<String>,
        /// Password protecting `CertP12`, empty for none; meaningless, and rejected, without
        /// `CertP12`.
        cert_p12_password: Option<String>,
        /// Path to the Certificate Authority file, in PEM format; empty for the system CAs.
        ca_cert: Option<String>,
        /// Accept any server certificate instead of verifying it: `1`, `true`, `yes`, `enable`,
        /// `allow` or `authorize`, and their negatives; empty for false.
        allow_unsafe_connection: Option<String>,
        /// Override the endpoint name during SSL verification.
        override_target_name: Option<String>,
        /// Timeout for establishing a connection to the server (e.g. `30s`), empty for 60s.
        connect_timeout: Option<String>,
        /// Timeout for each request (e.g. `30s`), empty for no timeout.
        timeout: Option<String>,
        /// Rate limit for requests, written `count/duration` (e.g. `100/1s`), empty for no rate
        /// limit.
        rate_limit: Option<String>,
        /// TCP keepalive duration (e.g. `30s`), empty for no keepalive.
        tcp_keepalive: Option<String>,
        /// Interval between TCP keepalive probes (e.g. `5s`), empty for the OS default.
        tcp_keepalive_interval: Option<String>,
        /// Number of TCP keepalive retries, empty for the OS default.
        tcp_keepalive_retries: Option<String>,
        /// Enable Nagle's algorithm (disable TCP_NODELAY); spelled like `AllowUnsafeConnection`,
        /// empty for false.
        tcp_nagle_algorithm: Option<String>,
        /// HTTP/2 PING frame interval (e.g. `20s`), empty for no keepalive.
        http2_keep_alive_interval: Option<String>,
        /// HTTP/2 PING timeout (e.g. `10s`), empty for no timeout.
        http2_keep_alive_timeout: Option<String>,
        /// Send HTTP/2 keepalive PINGs even when idle; spelled like `AllowUnsafeConnection`,
        /// empty for false.
        http2_keep_alive_while_idle: Option<String>,
        /// HTTP/2 max header list size in bytes, empty for no limit.
        http2_max_header_list_size: Option<String>,
        /// User-Agent header value sent with each request.
        user_agent: Option<String>,
        /// HTTP proxy used to reach the endpoint: empty or `system` to follow the environment,
        /// `none` for a direct connection, otherwise the proxy URL, whose scheme has to be
        /// `http`.
        proxy: Option<String>,
        /// Username for proxy authentication; empty falls back to the username the `Proxy` URL
        /// carried, if any.
        proxy_username: Option<String>,
        /// Password for proxy authentication; empty falls back to the password the `Proxy` URL
        /// carried, independently of the username.
        proxy_password: Option<String>,
    }

    impl schemars::JsonSchema for super::HttpConfig {
        fn schema_name() -> Cow<'static, str> {
            HttpConfigSchema::schema_name()
        }

        fn schema_id() -> Cow<'static, str> {
            Cow::Borrowed(concat!(module_path!(), "::HttpConfig"))
        }

        fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            HttpConfigSchema::json_schema(generator)
        }

        fn inline_schema() -> bool {
            HttpConfigSchema::inline_schema()
        }
    }

    /// Options for creating a gRPC client.
    #[derive(schemars::JsonSchema)]
    #[schemars(rename = "HttpConfig", rename_all = "PascalCase")]
    #[allow(dead_code)]
    pub(super) struct HttpConfigStructured {
        /// Endpoint for sending requests.
        endpoint: Option<String>,
        /// TLS and mTLS: the client's own identity, the server's CA, and SSL verification
        /// behaviour.
        tls: Option<TlsConfigSchema>,
        /// Timeout for establishing a connection to the server (e.g. `30s`), defaults to 60s.
        connect_timeout: Option<String>,
        /// Timeout for each request (e.g. `30s`), defaults to no timeout.
        timeout: Option<String>,
        /// Rate limit for requests, written `count/duration` (e.g. `100/1s`), defaults to no rate
        /// limit.
        rate_limit: Option<String>,
        /// TCP-level socket options.
        tcp: Option<TcpConfigSchema>,
        /// HTTP/2-level transport options.
        http2: Option<Http2ConfigSchema>,
        /// User-Agent header value sent with each request.
        user_agent: Option<String>,
        /// HTTP proxy used to reach the endpoint, defaults to following the environment.
        proxy: Option<ProxyConfigSchema>,
    }

    /// TLS and mTLS: the client's own identity, the server's CA, and SSL verification behaviour.
    #[derive(schemars::JsonSchema)]
    #[schemars(rename = "TlsConfig", rename_all = "PascalCase")]
    #[allow(dead_code)]
    struct TlsConfigSchema {
        /// Accept any server certificate instead of verifying it: `1`, `true`, `yes`, `enable`,
        /// `allow` or `authorize`, and their negatives; empty for false.
        allow_unsafe_connection: Option<String>,
        /// TLS identity of the client, absent for no client authentication.
        identity: Option<IdentitySourceSchema>,
        /// Path to the Certificate Authority file, in PEM format; absent for the system CAs.
        ca_cert: Option<String>,
        /// Override the endpoint name during SSL verification.
        override_target_name: Option<String>,
    }

    /// Where the client's TLS identity comes from.
    #[derive(schemars::JsonSchema)]
    #[schemars(rename = "IdentitySource", rename_all = "PascalCase")]
    #[allow(dead_code)]
    enum IdentitySourceSchema {
        /// A certificate and its key, each in its own PEM file.
        #[schemars(rename_all = "PascalCase")]
        PemFiles {
            /// Path to the certificate file, in PEM format.
            cert_pem: String,
            /// Path to the key file, in PEM format.
            key_pem: String,
        },
        /// A certificate and its key bundled together in one PKCS#12 file.
        #[schemars(rename_all = "PascalCase")]
        Pkcs12 {
            /// Path to the PKCS#12 bundle.
            cert_p12: String,
            /// The password protecting the bundle, absent for none.
            cert_p12_password: Option<String>,
        },
    }

    /// TCP-level socket options.
    #[derive(schemars::JsonSchema)]
    #[schemars(rename = "TcpConfig", rename_all = "PascalCase")]
    #[allow(dead_code)]
    struct TcpConfigSchema {
        /// TCP keepalive duration (e.g. `30s`), empty for no keepalive.
        keepalive: Option<String>,
        /// Interval between TCP keepalive probes (e.g. `5s`), empty for the OS default.
        keepalive_interval: Option<String>,
        /// Number of TCP keepalive retries, empty for the OS default.
        keepalive_retries: Option<String>,
        /// Enable Nagle's algorithm (disable TCP_NODELAY); spelled like `AllowUnsafeConnection`,
        /// empty for false.
        nagle_algorithm: Option<String>,
    }

    /// HTTP/2-level transport options.
    #[derive(schemars::JsonSchema)]
    #[schemars(rename = "Http2Config", rename_all = "PascalCase")]
    #[allow(dead_code)]
    struct Http2ConfigSchema {
        /// HTTP/2 PING frame interval (e.g. `20s`), empty for no keepalive.
        keep_alive_interval: Option<String>,
        /// HTTP/2 PING timeout (e.g. `10s`), empty for no timeout.
        keep_alive_timeout: Option<String>,
        /// Send HTTP/2 keepalive PINGs even when idle; spelled like `AllowUnsafeConnection`,
        /// empty for false.
        keep_alive_while_idle: Option<String>,
        /// HTTP/2 max header list size in bytes, empty for no limit.
        max_header_list_size: Option<String>,
    }

    /// Configuration of the HTTP proxy used to reach the endpoint.
    #[derive(schemars::JsonSchema)]
    #[schemars(rename = "ProxyConfig", rename_all = "PascalCase")]
    #[allow(dead_code)]
    struct ProxyConfigSchema {
        /// Where to find the proxy: empty or `system` to follow the environment, `none` for a
        /// direct connection, otherwise the proxy URL, whose scheme has to be `http`.
        source: Option<String>,
        /// Username for proxy authentication; empty falls back to the username the proxy URL
        /// carried, if any.
        username: Option<String>,
        /// Password for proxy authentication; empty falls back to the password the proxy URL
        /// carried, independently of the username.
        password: Option<String>,
    }
}

#[cfg(feature = "schema")]
impl HttpConfig {
    /// The JSON schema mirroring this type's own structure: thematic units as nested objects, the
    /// TLS identity as a `oneOf` of its variants.
    ///
    /// The flat option vocabulary, what deserialisation actually accepts, is this type's own
    /// [`schemars::JsonSchema`] implementation.
    pub fn structured_schema() -> schemars::Schema {
        schemars::schema_for!(schema::HttpConfigStructured)
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use secrecy::ExposeSecret as _;
    use serde_json::json;

    use super::*;
    use crate::proxy::ProxySource;
    use crate::tls_config::IdentitySource;

    /// The configuration `value` describes, expected to be valid.
    fn config(value: serde_json::Value) -> HttpConfig {
        serde_json::from_value(value).expect("a valid configuration")
    }

    /// The error message `value` produces, expected to be a rejection.
    fn error(value: serde_json::Value) -> String {
        serde_json::from_value::<HttpConfig>(value)
            .expect_err("the configuration should be rejected")
            .to_string()
    }

    #[test]
    fn the_minimum_is_an_endpoint() {
        let config = config(json!({"Endpoint": "http://localhost:5001"}));

        assert_eq!(config.endpoint.to_string(), "http://localhost:5001/");
        assert!(config.tls.identity.is_none());
        assert!(config.tls.ca_cert.is_none());
        assert_eq!(config.tls.override_target_name, None);
        assert_eq!(config.rate_limit, None);
    }

    #[test]
    fn an_absent_endpoint_reads_as_the_default_uri_rather_than_failing() {
        // A configuration file need only name what it changes; rejecting the empty endpoint is
        // `connect`'s job, where the error can name the option.
        for value in [json!({}), json!({"Endpoint": ""})] {
            let config = config(value);
            assert_eq!(config.endpoint, Uri::default());
        }
    }

    #[test]
    fn an_endpoint_that_is_not_a_uri_is_reported_with_the_value() {
        let rendered = error(json!({"Endpoint": "http://exa mple:5001"}));

        assert!(rendered.contains("not a valid URI"), "{rendered}");
        assert!(rendered.contains("exa mple"), "{rendered}");
    }

    #[test]
    fn absent_fields_default() {
        // Deserialise only: the type deliberately has no `Serialize`, so a proxy password cannot
        // leak through a configuration dump.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "Timeout": "30s",
            "ProxyPassword": "s3cr3t",
        }));

        assert_eq!(config.timeout, Some(Duration::from_secs(30)));
        assert!(config.tls.identity.is_none(), "an absent field defaults");
        assert!(!config.tls.allow_unsafe_connection);
        assert_eq!(config.proxy.password.expose_secret(), "s3cr3t");
    }

    #[test]
    fn an_empty_option_reads_as_its_default() {
        // A deployment that declares every variable with an empty default must behave exactly like
        // one that declares none.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "ConnectTimeout": "",
            "Timeout": "",
            "RateLimit": "",
            "TcpKeepalive": "",
            "TcpKeepaliveRetries": "",
            "UserAgent": "",
        }));

        assert_eq!(config.connect_timeout, Some(DEFAULT_CONNECT_TIMEOUT));
        assert_eq!(config.timeout, None);
        assert_eq!(config.rate_limit, None);
        assert_eq!(config.tcp.keepalive, None);
        assert_eq!(config.tcp.keepalive_retries, None);
        assert_eq!(config.user_agent, None);
    }

    // --- durations and numbers ---

    #[test]
    fn durations_are_read_in_the_units_they_are_written_in() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "ConnectTimeout": "500ms",
            "TcpKeepalive": "30s",
            "TcpKeepaliveInterval": "2m",
            "Http2KeepAliveInterval": "1h",
        }));

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
    fn a_duration_that_cannot_be_parsed_names_the_option_and_the_value() {
        // Eight options share the duration and integer readers, so a message that does not name
        // its option leaves the reader guessing which of them is mistyped.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "TcpKeepalive": "soon",
        }));

        assert!(rendered.contains("not a valid duration"), "{rendered}");
        assert!(rendered.contains("soon"), "{rendered}");
        assert!(rendered.contains("TcpKeepalive"), "{rendered}");
    }

    #[test]
    fn integers_are_read_and_a_bad_one_names_the_value() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "TcpKeepaliveRetries": "3",
            "Http2MaxHeaderListSize": "16384",
        }));
        assert_eq!(config.tcp.keepalive_retries, Some(3));
        assert_eq!(config.http2.max_header_list_size, Some(16384));

        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "TcpKeepaliveRetries": "many",
        }));
        assert!(rendered.contains("not a valid integer"), "{rendered}");
        assert!(rendered.contains("many"), "{rendered}");
        assert!(rendered.contains("TcpKeepaliveRetries"), "{rendered}");
    }

    #[test]
    fn an_integer_that_does_not_fit_is_rejected_rather_than_wrapped() {
        // These are `u32`; a value past the top must fail rather than silently become something
        // else.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "Http2MaxHeaderListSize": "4294967296",
        }));

        assert!(rendered.contains("not a valid integer"), "{rendered}");
    }

    #[test]
    fn a_number_a_source_typed_eagerly_is_still_read() {
        // A JSON document, like `figment`'s own reading of the environment, may hand a bare
        // number over as one rather than as text.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "TcpKeepaliveRetries": 3,
        }));

        assert_eq!(config.tcp.keepalive_retries, Some(3));
    }

    // --- the user agent ---

    #[test]
    fn a_user_agent_is_read_into_a_header_value() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "UserAgent": "armonik-test/1",
        }));

        assert_eq!(
            config.user_agent,
            Some(HeaderValue::from_static("armonik-test/1"))
        );
    }

    #[test]
    fn a_user_agent_that_is_not_a_header_value_is_reported() {
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "UserAgent": "line\nbreak",
        }));

        assert!(rendered.contains("Invalid user agent"), "{rendered}");
    }

    // --- rate limit ---

    #[test]
    fn a_rate_limit_is_a_count_and_a_duration() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "RateLimit": "100/1s",
        }));

        assert_eq!(config.rate_limit, Some((100, Duration::from_secs(1))));
    }

    #[test]
    fn a_zero_rate_limit_is_rejected_rather_than_left_to_panic() {
        // `tower`'s `Rate::new` asserts both halves are above zero, so a zero has to be refused
        // here rather than reaching it: a panic inside `connect` tells the caller nothing.
        for value in ["0/1s", "1/0s", "0/0s"] {
            let rendered = error(json!({
                "Endpoint": "http://localhost:5001",
                "RateLimit": value,
            }));

            assert!(
                rendered.contains("zero count or duration"),
                "{value}: {rendered}"
            );
            assert!(
                rendered.contains(value),
                "the message should quote it: {rendered}"
            );
        }
    }

    #[test]
    fn a_rate_limit_missing_its_duration_is_reported_with_the_expected_shape() {
        // The message has to show the format, since `100` on its own looks perfectly reasonable to
        // whoever wrote it.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "RateLimit": "100",
        }));

        assert!(rendered.contains("number/duration"), "{rendered}");
        assert!(rendered.contains("100"), "{rendered}");
    }

    #[test]
    fn each_half_of_a_rate_limit_is_validated_separately() {
        let count = error(json!({
            "Endpoint": "http://localhost:5001",
            "RateLimit": "plenty/1s",
        }));
        assert!(count.contains("Rate limit count"), "{count}");
        assert!(count.contains("plenty"), "{count}");

        let duration = error(json!({
            "Endpoint": "http://localhost:5001",
            "RateLimit": "100/soon",
        }));
        assert!(duration.contains("not a valid duration"), "{duration}");
        assert!(duration.contains("soon"), "{duration}");
    }

    // --- booleans ---

    #[test]
    fn a_boolean_option_accepts_the_spelling_its_source_writes_naturally() {
        // A document may spell a boolean option the way its own format does, rather than the way
        // an environment variable has to.
        for (written, expected) in [
            (json!(true), true),
            (json!(false), false),
            (json!("yes"), true),
            (json!(1), true),
            (json!(0), false),
            (json!(""), false),
        ] {
            let config = config(json!({
                "Endpoint": "http://localhost:5001",
                "AllowUnsafeConnection": written,
            }));

            assert_eq!(
                config.tls.allow_unsafe_connection, expected,
                "{written} should resolve to {expected}"
            );
        }
    }

    #[test]
    fn an_unusable_boolean_names_the_option_and_the_vocabulary() {
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "AllowUnsafeConnection": "perhaps",
        }));

        assert!(
            rendered.contains("`AllowUnsafeConnection=perhaps`"),
            "{rendered}"
        );
        assert!(rendered.contains("not a valid boolean"), "{rendered}");
        assert!(rendered.contains("`yes`"), "{rendered}");
    }

    // --- certificates ---

    #[test]
    fn half_an_identity_is_rejected_and_names_both_options() {
        // Half an identity is silent on a plain-TLS endpoint and only surfaces as a rejected
        // handshake on an mTLS one. Neither path is read from disk here, so this needs no fixture.
        for (cert, key) in [("cert.pem", ""), ("", "key.pem")] {
            let rendered = error(json!({
                "Endpoint": "http://localhost:5001",
                "CertPem": cert,
                "KeyPem": key,
            }));

            assert!(rendered.contains("CertPem"), "{rendered}");
            assert!(rendered.contains("KeyPem"), "{rendered}");
        }
    }

    #[test]
    fn neither_half_is_no_identity_rather_than_an_error() {
        let config = config(json!({"Endpoint": "http://localhost:5001"}));
        assert!(config.tls.identity.is_none());
    }

    #[test]
    fn a_certificate_option_names_a_path_without_reading_it() {
        // Whether the path leads anywhere is `connect`'s question to ask: a configuration can be
        // read on one machine and used on another.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "CertPem": "no/such/cert.pem",
            "KeyPem": "no/such/key.pem",
        }));

        assert_eq!(
            config.tls.identity,
            Some(IdentitySource::PemFiles {
                cert_pem: "no/such/cert.pem".into(),
                key_pem: "no/such/key.pem".into(),
            })
        );
    }

    #[test]
    fn cert_p12_and_the_pem_pair_are_mutually_exclusive() {
        // Two spellings of the identity at once is a contradiction whichever half of the PEM pair
        // is set, and the message has to name both spellings so either one can be removed.
        for (cert, key) in [("cert.pem", "key.pem"), ("cert.pem", ""), ("", "key.pem")] {
            let rendered = error(json!({
                "Endpoint": "http://localhost:5001",
                "CertPem": cert,
                "KeyPem": key,
                "CertP12": "identity.p12",
            }));

            assert!(rendered.contains("CertP12"), "{rendered}");
            assert!(rendered.contains("CertPem"), "{rendered}");
        }
    }

    #[test]
    fn a_p12_password_without_a_p12_is_rejected_without_echoing_it() {
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "CertP12Password": "s3cr3t",
        }));

        assert!(rendered.contains("CertP12Password"), "{rendered}");
        assert!(rendered.contains("CertP12"), "{rendered}");
        assert!(!rendered.contains("s3cr3t"), "{rendered}");
    }

    #[test]
    fn empty_p12_options_read_as_unset() {
        // Including the password: an empty password next to no bundle is not "a password without
        // a bundle", it is the shape a deployment with empty defaults declares.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "CertP12": "",
            "CertP12Password": "",
        }));

        assert!(config.tls.identity.is_none());
    }

    #[test]
    fn a_p12_option_names_a_path_without_reading_it() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "CertP12": "no/such/identity.p12",
            "CertP12Password": "s3cr3t",
        }));

        assert_eq!(
            config.tls.identity,
            Some(IdentitySource::Pkcs12 {
                cert_p12: "no/such/identity.p12".into(),
                cert_p12_password: Some(secrecy::SecretString::from("s3cr3t")),
            })
        );
        let rendered = format!("{:?}", config.tls);
        assert!(!rendered.contains("s3cr3t"), "{rendered}");
    }

    // --- the proxy ---

    /// The configuration `Proxy=value` produces, expected to be valid.
    fn proxy_config(value: &str) -> HttpConfig {
        config(json!({"Endpoint": "http://localhost:5001", "Proxy": value}))
    }

    /// The error message `Proxy=value` produces, expected to be a rejection.
    fn proxy_error(value: &str) -> String {
        error(json!({"Endpoint": "http://localhost:5001", "Proxy": value}))
    }

    #[test]
    fn proxy_empty_and_system_follow_the_environment() {
        // The same reading as ArmoniK's C# client, where an unset option leaves the handler
        // following the environment.
        for value in ["", "system", "System", "SYSTEM"] {
            let config = proxy_config(value);
            assert_eq!(config.proxy.source, ProxySource::System, "{value:?}");
        }
    }

    #[test]
    fn proxy_none_forces_a_direct_connection() {
        for value in ["none", "None", "NONE"] {
            let config = proxy_config(value);
            assert_eq!(config.proxy.source, ProxySource::Disabled, "{value:?}");
        }
    }

    #[test]
    fn proxy_url_defaults_to_the_http_scheme() {
        let with_scheme = proxy_config("http://proxy.corp:3128");
        let without_scheme = proxy_config("proxy.corp:3128");

        let ProxySource::Explicit(with_scheme) = with_scheme.proxy.source else {
            panic!("expected an explicit proxy");
        };
        let ProxySource::Explicit(without_scheme) = without_scheme.proxy.source else {
            panic!("expected an explicit proxy");
        };
        assert_eq!(with_scheme, without_scheme);
        assert_eq!(with_scheme.host(), Some("proxy.corp"));
        assert_eq!(with_scheme.port_u16(), Some(3128));
    }

    #[test]
    fn proxy_credentials_in_the_url_are_honoured_and_removed_from_it() {
        let config = proxy_config("http://user:secret@proxy.corp:3128");

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
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "Proxy": "http://url-user:url-secret@proxy.corp:3128",
            "ProxyUsername": "option-user",
            "ProxyPassword": "option-secret",
        }));

        assert_eq!(config.proxy.username, "option-user");
        assert_eq!(config.proxy.password.expose_secret(), "option-secret");
    }

    #[test]
    fn a_dedicated_password_alone_keeps_the_username_the_url_carried() {
        // Replacing the pair rather than each half would leave an empty username here, and the
        // proxy would answer 407 with nothing to explain it.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "Proxy": "http://url-user:url-secret@proxy.corp:3128",
            "ProxyPassword": "option-secret",
        }));

        assert_eq!(config.proxy.username, "url-user");
        assert_eq!(config.proxy.password.expose_secret(), "option-secret");
    }

    #[test]
    fn the_configuration_keeps_the_password_out_of_its_debug_output() {
        // `HttpConfig` is `Debug` and holds the password, so a careless `Debug` would put it
        // anywhere a configuration gets printed.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "Proxy": "http://user:url-secret@proxy.corp:3128",
            "ProxyPassword": "option-secret",
        }));

        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("option-secret") && !rendered.contains("url-secret"),
            "password rendered: {rendered}"
        );
        assert!(
            rendered.contains("user"),
            "the username is not a secret and stays useful: {rendered}"
        );
    }

    #[test]
    fn a_rejected_proxy_url_does_not_echo_its_password() {
        // A URL can be rejected for having no host, or for a "port" that is really a password
        // whose `@host` part went missing, and still have carried a credential.
        for value in ["http://user:s3cr3t@", "http://admin:hunter2"] {
            let rendered = proxy_error(value);
            assert!(
                !rendered.contains("s3cr3t") && !rendered.contains("hunter2"),
                "password echoed for {value:?}: {rendered}"
            );
        }
    }

    #[test]
    fn a_proxy_that_is_not_http_is_rejected_rather_than_reached_in_the_clear() {
        // The `CONNECT` handshake goes out unencrypted, so a proxy expecting TLS would see
        // gibberish. Accepting the URL and failing at connect time would report it as an
        // unreachable proxy.
        for value in ["https://proxy.corp:3128", "socks5://proxy.corp:1080"] {
            let rendered = proxy_error(value);
            assert!(
                rendered.contains("only an `http` proxy"),
                "{value}: {rendered}"
            );
        }
    }

    #[test]
    fn proxy_without_a_host_is_rejected_and_names_its_own_option() {
        // Reporting these through the endpoint's URI error would send whoever reads it looking at
        // the wrong option.
        for value in ["http:///no-host", "http://", "http://:3128", "://"] {
            let rendered = proxy_error(value);
            assert!(
                rendered.contains("is not a valid proxy URL"),
                "unexpected error for {value:?}: {rendered}"
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
            let rendered = proxy_error(value);
            assert!(
                rendered.contains("does not name a valid port"),
                "unexpected error for {value:?}: {rendered}"
            );
        }
    }

    #[test]
    fn an_ipv6_proxy_is_accepted_with_and_without_a_port() {
        // The port check must not mistake the colons of an IPv6 literal for an invalid port.
        for (value, port) in [("http://[::1]:3128", Some(3128)), ("http://[::1]", None)] {
            let config = proxy_config(value);
            let ProxySource::Explicit(uri) = &config.proxy.source else {
                panic!("expected an explicit proxy for {value:?}");
            };
            assert_eq!(uri.port_u16(), port, "{value:?}");
        }
    }
}
