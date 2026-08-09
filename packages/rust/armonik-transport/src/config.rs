//! The client configuration, and how it is read.
//!
//! [`HttpConfig`] is plain data: every field is typed, public, and buildable by hand. Under the
//! `serde` feature it also deserialises directly from a flat document of `PascalCase` options
//! (`Endpoint`, `TcpKeepalive`, `Http2KeepAliveInterval`, ...), the same vocabulary every ArmoniK
//! client reads; the thematic groups ([`TlsConfig`], [`TcpConfig`], [`Http2Config`],
//! [`RetryConfig`], [`ProxyConfig`]) flatten back into those names, so grouping the fields changes
//! no option a deployment already sets.
//!
//! The `schema` feature describes that same vocabulary as a JSON schema, derived from the types
//! that define it rather than written out a second time, and the `env` feature reads it from
//! environment variables under a prefix the caller chooses.

use std::time::Duration;

use hyper::{http::HeaderValue, Uri};
use snafu::Snafu;

use crate::http2_config::Http2Config;
use crate::proxy::ProxyConfig;
use crate::retry_config::RetryConfig;
use crate::tcp_config::TcpConfig;
use crate::tls_config::TlsConfig;

/// Timeout for establishing a connection when the option is left unset.
pub(crate) const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(feature = "serde")]
use crate::config_utils::{embed_prefixed, optional_duration, text};

// Every flattened unit goes through the same mechanism, so no unit spells an option name of its
// own. An empty prefix strips nothing: it reads the flat vocabulary unchanged, and still gets the
// naming and a prefix to give the day the unit is embedded a second time.
#[cfg(feature = "serde")]
embed_prefixed!(
    tls,
    crate::tls_config::TlsConfig,
    crate::tls_config::RawTls,
    ""
);
#[cfg(feature = "serde")]
embed_prefixed!(
    tcp,
    crate::tcp_config::TcpConfig,
    crate::tcp_config::TcpConfig,
    "Tcp"
);
#[cfg(feature = "serde")]
embed_prefixed!(
    http2,
    crate::http2_config::Http2Config,
    crate::http2_config::Http2Config,
    "Http2"
);
#[cfg(feature = "serde")]
embed_prefixed!(
    retry,
    crate::retry_config::RetryConfig,
    crate::retry_config::RawRetry,
    ""
);
#[cfg(feature = "serde")]
embed_prefixed!(
    proxy,
    crate::proxy::ProxyConfig,
    crate::proxy::ProxyShape,
    "Proxy"
);

/// What an intra-doc link into a unit renders as: the name a doc comment links to that unit by,
/// the prefix this configuration reads it under, and the fields of it that name one option each.
///
/// Here, next to the embeddings that choose those prefixes, rather than in `config_utils`: the
/// same unit is a different set of names under a different prefix.
///
/// The fields are listed rather than derived because a rustdoc path says nothing about what it
/// landed on: `username` and `explicit` are spelled alike, and so are an option's field and one
/// that groups several options or that no option reads. A name built for either would be a name
/// no source spells - the very thing this rewriting exists to keep out of the schema.
#[cfg(feature = "schema")]
const OPTION_FIELDS: &[crate::config_utils::PrefixedUnit<'static>] = &[
    (
        // `ca_cert` is absent: it holds the certificate the reader loaded, where the option names
        // the path it was read from, so a link to the field resolves to no option at all.
        "TlsConfig",
        "",
        &["allow_unsafe_connection", "override_target_name"],
    ),
    (
        "TcpConfig",
        "Tcp",
        &[
            "keepalive",
            "keepalive_interval",
            "keepalive_retries",
            "nagle_algorithm",
        ],
    ),
    (
        "Http2Config",
        "Http2",
        &[
            "keep_alive_interval",
            "keep_alive_timeout",
            "keep_alive_while_idle",
            "max_header_list_size",
        ],
    ),
    (
        "RetryConfig",
        "",
        &[
            "max_attempts",
            "initial_back_off",
            "max_back_off",
            "back_off_multiplier",
        ],
    ),
    ("ProxyConfig", "Proxy", &["username", "password"]),
];

/// [`crate::config_utils::strip_rust_details`] over this configuration's own units: every type of
/// the vocabulary transforms its schema through here, so all of them render a link the same way.
#[cfg(feature = "schema")]
pub(crate) fn strip_rust_details(schema: &mut schemars::Schema) {
    crate::config_utils::strip_rust_details(OPTION_FIELDS, schema);
}

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
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(transform = crate::config::strip_rust_details)
)]
#[non_exhaustive]
pub struct HttpConfig {
    /// Endpoint for sending requests. `Endpoint`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "endpoint"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub endpoint: Uri,
    /// TLS and mTLS: the client's own identity, the server's CA, and SSL verification behaviour,
    /// read under no prefix (`CertPem`, `CaCertPath`, `AllowUnsafeConnection`, ...).
    #[cfg_attr(
        feature = "serde",
        serde(flatten, deserialize_with = "tls::deserialize")
    )]
    #[cfg_attr(feature = "schema", schemars(schema_with = "tls::schema"))]
    pub tls: TlsConfig,
    /// Timeout for establishing a connection to the server, defaults to 60s. `ConnectTimeout`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "connect_timeout"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub connect_timeout: Option<Duration>,
    /// Timeout for each request, defaults to no timeout. `Timeout`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "optional_duration"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub timeout: Option<Duration>,
    /// How long an idle connection is kept in a pool before it is closed. `PoolIdleTimeout`.
    ///
    /// Applied by whoever drives the pool: a channel is one connection and has none, so nothing
    /// in `connect` reads it.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "optional_duration"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub pool_idle_timeout: Option<Duration>,
    /// Rate limit for requests, written `count/duration` (e.g. `100/1s`), defaults to no rate
    /// limit. `RateLimit`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "rate_limit"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub rate_limit: Option<(u64, Duration)>,
    /// TCP-level socket options, read under the `Tcp` prefix (`TcpKeepalive`, ...).
    #[cfg_attr(
        feature = "serde",
        serde(flatten, deserialize_with = "tcp::deserialize")
    )]
    #[cfg_attr(feature = "schema", schemars(schema_with = "tcp::schema"))]
    pub tcp: TcpConfig,
    /// HTTP/2-level transport options, read under the `Http2` prefix (`Http2KeepAliveInterval`,
    /// ...).
    #[cfg_attr(
        feature = "serde",
        serde(flatten, deserialize_with = "http2::deserialize")
    )]
    #[cfg_attr(feature = "schema", schemars(schema_with = "http2::schema"))]
    pub http2: Http2Config,
    /// How a failed request is replayed, read under no prefix (`MaxAttempts`, `InitialBackOff`,
    /// ...).
    ///
    /// Applied by whoever makes the calls: a channel carries no notion of a call, so nothing in
    /// `connect` reads it.
    #[cfg_attr(
        feature = "serde",
        serde(flatten, deserialize_with = "retry::deserialize")
    )]
    #[cfg_attr(feature = "schema", schemars(schema_with = "retry::schema"))]
    pub retry: RetryConfig,
    /// User-Agent header value sent with each request. `UserAgent`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "user_agent"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub user_agent: Option<HeaderValue>,
    /// HTTP proxy used to reach the endpoint, read under the `Proxy` prefix (`ProxyAddress`,
    /// `ProxyUsername`, `ProxyPassword`), defaults to following the environment.
    #[cfg_attr(
        feature = "serde",
        serde(flatten, deserialize_with = "proxy::deserialize")
    )]
    #[cfg_attr(feature = "schema", schemars(schema_with = "proxy::schema"))]
    pub proxy: ProxyConfig,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            endpoint: Uri::default(),
            tls: TlsConfig::default(),
            connect_timeout: Some(DEFAULT_CONNECT_TIMEOUT),
            timeout: None,
            pool_idle_timeout: None,
            rate_limit: None,
            tcp: TcpConfig::default(),
            http2: Http2Config::default(),
            retry: RetryConfig::default(),
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
    Ok(optional_duration(deserializer)?.or(Some(DEFAULT_CONNECT_TIMEOUT)))
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

#[cfg(all(test, feature = "serde"))]
mod tests {
    use secrecy::ExposeSecret as _;
    use serde_json::json;

    use super::*;
    use crate::proxy::ProxySource;

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

    /// Every property name the schema declares, wherever `schemars` nested it.
    #[cfg(feature = "schema")]
    fn property_names(value: &serde_json::Value, names: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(object) => {
                for (key, child) in object {
                    if key == "properties" {
                        if let serde_json::Value::Object(properties) = child {
                            names.extend(properties.keys().cloned());
                        }
                    }
                    property_names(child, names);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    property_names(item, names);
                }
            }
            _ => {}
        }
    }

    #[cfg(feature = "schema")]
    #[test]
    fn every_field_a_link_resolves_through_names_an_option_the_schema_declares() {
        // The table is written by hand, so this is what keeps it from drifting: a field listed
        // there under the wrong prefix, or one that stopped being an option, would put a name no
        // deployment can set into a description a consumer generates its class from.
        let schema = serde_json::to_value(schemars::schema_for!(HttpConfig))
            .expect("a schema serialises to JSON");
        let mut names = Vec::new();
        property_names(&schema, &mut names);

        for (unit, _, fields) in OPTION_FIELDS {
            for field in *fields {
                let path = format!("{unit}::{field}");
                let flat = crate::config_utils::flat_name(&path, OPTION_FIELDS)
                    .unwrap_or_else(|| panic!("`{path}` is listed and has to resolve"));
                assert!(
                    names.contains(&flat),
                    "`{path}` renders `{flat}`, which no option spells"
                );
            }
        }
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
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "Timeout": "30s",
        }));

        assert_eq!(config.timeout, Some(Duration::from_secs(30)));
        assert!(config.tls.identity.is_none(), "an absent field defaults");
        assert!(!config.tls.allow_unsafe_connection);
    }

    #[test]
    fn a_proxy_option_is_read_rather_than_ignored() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "ProxyAddress": "http://proxy.corp:3128",
            "ProxyUsername": "user",
        }));

        let ProxySource::Explicit(uri) = &config.proxy.source else {
            panic!("expected an explicit proxy");
        };
        assert_eq!(uri.host(), Some("proxy.corp"));
        assert_eq!(config.proxy.username, "user");
    }

    #[test]
    fn an_empty_option_reads_as_its_default() {
        // A deployment that declares every variable with an empty default must behave exactly like
        // one that declares none.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "ConnectTimeout": "",
            "Timeout": "",
            "PoolIdleTimeout": "",
            "RateLimit": "",
            "TcpKeepalive": "",
            "TcpKeepaliveRetries": "",
            "UserAgent": "",
            "MaxAttempts": "",
            "InitialBackOff": "",
            "MaxBackOff": "",
            "BackOffMultiplier": "",
        }));

        assert_eq!(config.connect_timeout, Some(DEFAULT_CONNECT_TIMEOUT));
        assert_eq!(config.timeout, None);
        assert_eq!(config.pool_idle_timeout, None);
        assert_eq!(config.rate_limit, None);
        assert_eq!(config.tcp.keepalive, None);
        assert_eq!(config.tcp.keepalive_retries, None);
        assert_eq!(config.user_agent, None);
        assert_eq!(config.retry, RetryConfig::default());
    }

    // --- durations and numbers ---

    #[test]
    fn durations_are_read_in_the_units_they_are_written_in() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "ConnectTimeout": "500ms",
            "PoolIdleTimeout": "90s",
            "TcpKeepalive": "30s",
            "TcpKeepaliveInterval": "2m",
            "Http2KeepAliveInterval": "1h",
        }));

        assert_eq!(config.connect_timeout, Some(Duration::from_millis(500)));
        assert_eq!(config.pool_idle_timeout, Some(Duration::from_secs(90)));
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
        // its option leaves the reader guessing which of them is mistyped. The name is the key the
        // document spelled, prefix included, and no reader is told what it is called.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "TcpKeepalive": "soon",
        }));

        assert!(rendered.contains("not a valid duration"), "{rendered}");
        assert!(rendered.contains("soon"), "{rendered}");
        assert!(rendered.contains("TcpKeepalive"), "{rendered}");
    }

    #[test]
    fn the_named_option_carries_the_prefix_the_embedding_reads_it_under() {
        // The unit knows nothing of the prefix, so the name can only come from the document, and
        // it has to be the full option a deployment would set rather than the bare field.
        for (option, value) in [
            ("Http2KeepAliveInterval", "soon"),
            ("TcpKeepaliveInterval", "soon"),
            ("TcpKeepaliveRetries", "many"),
            ("Http2MaxHeaderListSize", "many"),
            ("TcpNagleAlgorithm", "perhaps"),
            ("Http2KeepAliveWhileIdle", "perhaps"),
            ("MaxAttempts", "many"),
            ("InitialBackOff", "a while"),
            ("BackOffMultiplier", "twice"),
        ] {
            let rendered = error(json!({
                "Endpoint": "http://localhost:5001",
                option: value,
            }));

            assert!(rendered.contains(&format!("`{option}`")), "{rendered}");
            assert!(rendered.contains(value), "{rendered}");
        }
    }

    #[test]
    fn a_relationship_between_options_is_reported_without_a_key_in_front_of_it() {
        // A conversion of a whole unit fails once every key has been read, so the tracker has no
        // key left to offer. Decorating it with the empty path would put a bare `.` where the
        // reader expects an option name, and the message already names what it relates.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "CertPem": "cert.pem",
        }));

        assert!(!rendered.contains("`.`"), "{rendered}");
        assert!(!rendered.starts_with('.'), "{rendered}");
    }

    #[test]
    fn every_unit_reads_its_options_under_the_flat_names_a_deployment_sets() {
        // The units are read through prefix modules, so this pins the whole vocabulary: a
        // prefix written on the wrong embedding would silently move a group of options.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "AllowUnsafeConnection": "true",
            "OverrideTargetName": "server.example.com",
            "TcpKeepalive": "30s",
            "TcpNagleAlgorithm": "true",
            "Http2KeepAliveTimeout": "10s",
            "Http2MaxHeaderListSize": "16384",
        }));

        assert!(config.tls.allow_unsafe_connection);
        assert_eq!(
            config.tls.override_target_name.as_deref(),
            Some("server.example.com")
        );
        assert_eq!(config.tcp.keepalive, Some(Duration::from_secs(30)));
        assert!(config.tcp.nagle_algorithm);
        assert_eq!(
            config.http2.keep_alive_timeout,
            Some(Duration::from_secs(10))
        );
        assert_eq!(config.http2.max_header_list_size, Some(16384));
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
        // A JSON document may hand a bare number over as one rather than as text.
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
    fn a_certificate_path_that_leads_nowhere_fails_while_reading_and_names_it() {
        // The identity's files are loaded as the configuration is read, so a mistyped path fails
        // here, where the error can name the file, rather than at connect time.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "CertPem": "no/such/cert.pem",
            "KeyPem": "no/such/key.pem",
        }));

        assert!(rendered.contains("no/such/cert.pem"), "{rendered}");
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
        // Both shapes the orphan takes: alone, and next to a PEM identity that is legal on its own.
        for extra in [
            json!({}),
            json!({"CertPem": "cert.pem", "KeyPem": "key.pem"}),
        ] {
            let mut value = json!({
                "Endpoint": "http://localhost:5001",
                "CertP12Password": "s3cr3t",
            });
            value
                .as_object_mut()
                .expect("a JSON object")
                .extend(extra.as_object().expect("a JSON object").clone());

            let rendered = error(value);
            assert!(rendered.contains("CertP12Password"), "{rendered}");
            assert!(rendered.contains("CertP12"), "{rendered}");
            assert!(!rendered.contains("s3cr3t"), "{rendered}");
        }
    }

    #[test]
    fn empty_p12_options_read_as_unset() {
        // Including the password: an empty password next to no bundle is not "a password without a
        // bundle", it is the shape a deployment with empty defaults declares.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "CertP12": "",
            "CertP12Password": "",
        }));

        assert!(config.tls.identity.is_none());
    }

    #[test]
    fn an_empty_cert_p12_leaves_the_pem_pair_next_to_it_alone() {
        // A present-but-empty `CertP12` names no bundle, so it is neither an identity of its own
        // nor a contradiction with the PEM pair that is really set.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "CertP12": "",
            "CertPem": "no/such/cert.pem",
            "KeyPem": "no/such/key.pem",
        }));

        assert!(rendered.contains("no/such/cert.pem"), "{rendered}");
    }

    #[test]
    fn a_p12_path_that_leads_nowhere_fails_while_reading_and_names_it() {
        // The bundle is loaded as the configuration is read, like the PEM pair, and the password
        // it was opened with stays out of the message.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "CertP12": "no/such/identity.p12",
            "CertP12Password": "s3cr3t",
        }));

        assert!(rendered.contains("no/such/identity.p12"), "{rendered}");
        assert!(!rendered.contains("s3cr3t"), "{rendered}");
    }

    // --- retry ---

    #[test]
    fn the_retry_options_default_to_what_the_other_clients_do() {
        let config = config(json!({"Endpoint": "http://localhost:5001"}));

        assert_eq!(config.retry, RetryConfig::default());
    }

    #[test]
    fn each_retry_option_is_read() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "MaxAttempts": "3",
            "InitialBackOff": "250ms",
            "MaxBackOff": "2s",
            "BackOffMultiplier": "3",
        }));

        assert_eq!(config.retry.max_attempts, 3);
        assert_eq!(config.retry.initial_back_off, Duration::from_millis(250));
        assert_eq!(config.retry.max_back_off, Duration::from_secs(2));
        assert_eq!(config.retry.back_off_multiplier, 3.0);
    }

    #[test]
    fn a_fractional_multiplier_survives_the_trip_through_text() {
        // The one option that is a real number: reading it as an integer would round the growth
        // the other clients use down to no growth at all.
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "BackOffMultiplier": "1.5",
        }));

        assert_eq!(config.retry.back_off_multiplier, 1.5);
    }

    #[test]
    fn zero_attempts_is_refused_by_the_option_rather_than_read_as_one() {
        // `0` and `1` would both mean one try and no replay, so a source that spells `0` means
        // something the client cannot do and has to hear about it.
        let rendered = error(json!({
            "Endpoint": "http://localhost:5001",
            "MaxAttempts": "0",
        }));

        assert!(rendered.contains("`MaxAttempts`"), "{rendered}");
        assert!(rendered.contains("zero"), "{rendered}");
    }

    #[test]
    fn a_ceiling_below_the_initial_back_off_is_refused_and_names_both_options() {
        // Setting one of the two is enough: the other keeps its default, and the pair is what is
        // wrong, so the message cannot lean on the key a source spelled.
        for value in [
            json!({"Endpoint": "http://localhost:5001", "InitialBackOff": "10s"}),
            json!({"Endpoint": "http://localhost:5001", "MaxBackOff": "500ms"}),
        ] {
            let rendered = error(value);

            assert!(rendered.contains("`MaxBackOff`"), "{rendered}");
            assert!(rendered.contains("`InitialBackOff`"), "{rendered}");
        }
    }

    #[test]
    fn a_ceiling_equal_to_the_initial_back_off_is_a_constant_wait_rather_than_an_error() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "InitialBackOff": "2s",
            "MaxBackOff": "2s",
            "MaxAttempts": "3",
        }));

        assert_eq!(
            config.retry.bounds().collect::<Vec<_>>(),
            [Duration::from_secs(2); 2]
        );
    }

    // --- the proxy ---

    /// The configuration `ProxyAddress=value` produces, expected to be valid.
    fn proxy_config(value: &str) -> HttpConfig {
        config(json!({"Endpoint": "http://localhost:5001", "ProxyAddress": value}))
    }

    /// The error message `ProxyAddress=value` produces, expected to be a rejection.
    fn proxy_error(value: &str) -> String {
        error(json!({"Endpoint": "http://localhost:5001", "ProxyAddress": value}))
    }

    #[test]
    fn an_empty_or_system_address_follows_the_environment() {
        // The same reading as ArmoniK's C# client, where an unset option leaves the handler
        // following the environment.
        for value in ["", "system", "System", "SYSTEM"] {
            let config = proxy_config(value);
            assert_eq!(config.proxy.source, ProxySource::System, "{value:?}");
        }
    }

    #[test]
    fn none_forces_a_direct_connection() {
        for value in ["none", "None", "NONE"] {
            let config = proxy_config(value);
            assert_eq!(config.proxy.source, ProxySource::Disabled, "{value:?}");
        }
    }

    #[test]
    fn a_proxy_url_defaults_to_the_http_scheme() {
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
    fn credentials_in_the_url_are_honoured_and_removed_from_it() {
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
    fn the_dedicated_credential_options_are_read() {
        let config = config(json!({
            "Endpoint": "http://localhost:5001",
            "ProxyAddress": "http://proxy.corp:3128",
            "ProxyUsername": "user",
            "ProxyPassword": "secret",
        }));

        assert_eq!(config.proxy.username, "user");
        assert_eq!(config.proxy.password.expose_secret(), "secret");
    }

    #[test]
    fn credentials_in_both_the_url_and_the_options_are_rejected() {
        // Two ways in, not a merge: guessing which half of which source wins turns a mixed
        // configuration into a silent surprise, so it is refused while it is being read. The
        // message names both sources and echoes neither value.
        for dedicated in [
            json!({"ProxyUsername": "option-user", "ProxyPassword": "option-secret"}),
            json!({"ProxyPassword": "option-secret"}),
            json!({"ProxyUsername": "option-user"}),
        ] {
            let mut value = json!({
                "Endpoint": "http://localhost:5001",
                "ProxyAddress": "http://url-user:url-secret@proxy.corp:3128",
            });
            value
                .as_object_mut()
                .expect("a JSON object")
                .extend(dedicated.as_object().expect("a JSON object").clone());

            let rendered = error(value);
            assert!(rendered.contains("set them one way"), "{rendered}");
            assert!(rendered.contains("ProxyUsername"), "{rendered}");
            assert!(
                !rendered.contains("url-secret") && !rendered.contains("option-secret"),
                "a password is echoed: {rendered}"
            );
        }
    }

    #[test]
    fn the_configuration_keeps_the_password_out_of_its_debug_output() {
        // `HttpConfig` is `Debug` and holds the password, so a careless `Debug` would put it
        // anywhere a configuration gets printed. Both shapes are covered: a URL-carried password
        // and a dedicated one.
        for value in [
            json!({
                "Endpoint": "http://localhost:5001",
                "ProxyAddress": "http://user:url-secret@proxy.corp:3128",
            }),
            json!({
                "Endpoint": "http://localhost:5001",
                "ProxyAddress": "http://proxy.corp:3128",
                "ProxyUsername": "user",
                "ProxyPassword": "option-secret",
            }),
        ] {
            let rendered = format!("{:?}", config(value));
            assert!(
                !rendered.contains("option-secret") && !rendered.contains("url-secret"),
                "password rendered: {rendered}"
            );
            assert!(
                rendered.contains("user"),
                "the username is not a secret and stays useful: {rendered}"
            );
        }
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
    fn a_proxy_url_whose_credentials_cannot_be_stripped_is_rejected_while_reading() {
        // `@` is legal inside a bracketed authority, so this parses, yet what follows the
        // credentials (`proxy]`) is not a host on its own. Accepting it would store the
        // placeholder and fail only at connect time, against this option's fail-on-read contract;
        // the rejection must not echo the password either.
        let rendered = proxy_error("http://[user:s3cr3t@proxy]");

        assert!(
            rendered.contains("once its credentials are taken out"),
            "{rendered}"
        );
        assert!(!rendered.contains("s3cr3t"), "password echoed: {rendered}");
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
    fn a_proxy_without_a_host_is_rejected_and_names_its_own_option() {
        // Reporting these through the endpoint's URI error would send whoever reads it looking at
        // the wrong option.
        for value in ["http:///no-host", "http://", "http://:3128", "://"] {
            let rendered = proxy_error(value);
            assert!(
                rendered.contains("`ProxyAddress"),
                "unexpected error for {value:?}: {rendered}"
            );
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
