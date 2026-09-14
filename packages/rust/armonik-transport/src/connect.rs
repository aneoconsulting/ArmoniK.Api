//! Turning a [`ClientConfig`] into a connected `tonic` channel.
//!
//! TLS, mTLS, and every timeout, keepalive and identity setting come together here.

use std::sync::Arc;

use hyper::Uri;
use hyper_rustls::{ConfigBuilderExt, FixedServerNameResolver, HttpsConnector};
use hyper_util::client::legacy::connect::HttpConnector;
use rustls::pki_types::{IpAddr, ServerName};
use snafu::{ResultExt, Snafu};

use crate::config::{ConfigError, IncompatibleOptionsSnafu};
use crate::ClientConfig;

/// Connect to the endpoint described by `config`, eagerly: this resolves once the connection is
/// established, not lazily on the first request.
pub async fn connect(config: ClientConfig) -> Result<tonic::transport::Channel, ConnectionError> {
    let endpoint = config.endpoint.clone();
    let override_target = config.override_target.clone();
    let http2_keep_alive_interval = config.http2_keep_alive_interval;
    let http2_keep_alive_timeout = config.http2_keep_alive_timeout;
    let http2_keep_alive_while_idle = config.http2_keep_alive_while_idle;
    let http2_max_header_list_size = config.http2_max_header_list_size;
    let user_agent = config.user_agent.clone();
    let timeout = config.timeout;
    let rate_limit = config.rate_limit;

    let https = https_connector(config).await?;

    let mut transport_endpoint = tonic::transport::Endpoint::from(endpoint.clone());
    if let Some(target) = override_target {
        transport_endpoint = transport_endpoint.origin(target);
    }

    if let Some(timeout) = timeout {
        transport_endpoint = transport_endpoint.timeout(timeout);
    }
    if let Some((limit, duration)) = rate_limit {
        transport_endpoint = transport_endpoint.rate_limit(limit, duration);
    }

    if let Some(interval) = http2_keep_alive_interval {
        transport_endpoint = transport_endpoint.http2_keep_alive_interval(interval);
    }
    if let Some(timeout) = http2_keep_alive_timeout {
        transport_endpoint = transport_endpoint.keep_alive_timeout(timeout);
    }
    transport_endpoint = transport_endpoint.keep_alive_while_idle(http2_keep_alive_while_idle);
    if let Some(max) = http2_max_header_list_size {
        transport_endpoint = transport_endpoint.http2_max_header_list_size(max);
    }
    if let Some(ua) = user_agent {
        transport_endpoint = transport_endpoint
            .user_agent(ua)
            .expect("HeaderValue is already validated, conversion is infallible");
    }

    transport_endpoint
        .connect_with_connector(https)
        .await
        .context(TransportSnafu { endpoint })
}

/// Build the connector stack, TCP then TLS or mTLS, that [`connect`] wraps in a channel.
///
/// Most callers want [`connect`]. This exists so a caller can drive plain HTTP through the same
/// connection configuration a channel would use, which is what it takes to reach a mock server's
/// diagnostic endpoint from a test. Hidden because its return type names this crate's dependencies
/// rather than its own; `pub` only so the signature is expressible.
#[doc(hidden)]
pub async fn https_connector(
    config: ClientConfig,
) -> Result<HttpsConnector<HttpConnector>, ConnectionError> {
    let endpoint = config.endpoint;

    let crypto_provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));

    let tls_config = rustls::ClientConfig::builder_with_provider(crypto_provider)
        .with_safe_default_protocol_versions()
        .with_context(|_| TlsSnafu {
            endpoint: endpoint.clone(),
        })?;

    // Server verification: no verification at all, a pinned CA, or the system trust store.
    let tls_config = if config.allow_unsafe_connection {
        tls_config
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(crate::utils::InsecureCertVerifier))
    } else if let Some(cacert) = config.cacert {
        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.add(cacert).with_context(|_| TlsSnafu {
            endpoint: endpoint.clone(),
        })?;
        tls_config.with_root_certificates(root_cert_store)
    } else {
        tls_config
            .with_native_roots()
            .with_context(|_| IoSnafu {})?
    };

    // A client identity turns this into mTLS.
    let tls_config = if let Some((cert, key)) = config.identity {
        tls_config
            .with_client_auth_cert(vec![cert], key)
            .with_context(|_| TlsSnafu {
                endpoint: endpoint.clone(),
            })?
    } else {
        tls_config.with_no_client_auth()
    };

    // `https_or_http` picks the scheme off the URI.
    let mut https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls_config)
        .https_or_http();

    if let Some(hostname) = &config.override_target {
        let server_name = override_server_name(hostname)?;
        https = https.with_server_name_resolver(FixedServerNameResolver::new(server_name));
    };

    let mut http = HttpConnector::new();
    http.enforce_http(false); // required for hyper-rustls to switch schemes
    http.set_nodelay(!config.tcp_nagle_algorithm);
    http.set_keepalive(config.tcp_keepalive);
    http.set_keepalive_interval(config.tcp_keepalive_interval);
    http.set_keepalive_retries(config.tcp_keepalive_retries);
    if let Some(timeout) = config.connect_timeout {
        http.set_connect_timeout(Some(timeout));
    }

    Ok(https.enable_http1().enable_http2().wrap_connector(http))
}

/// The name the server certificate is verified against, from the host of an override target.
///
/// `http` reports the host of an IPv6 authority with its brackets, as `[::1]`, while a [`ServerName`]
/// is the address alone. Brackets delimit an IP literal and nothing else, so what stands between them
/// has to parse as an address rather than fall back to being read as a name.
///
/// A host that is neither is a mistyped `GrpcClient__OverrideTargetName`, reported as the
/// configuration error it is: this runs inside a library, where a panic leaves the caller nothing to
/// read.
fn override_server_name(target: &Uri) -> Result<ServerName<'static>, ConnectionError> {
    let host = target.host().unwrap_or_default();

    let server_name = match host.strip_prefix('[').and_then(|ip| ip.strip_suffix(']')) {
        Some(literal) => IpAddr::try_from(literal).ok().map(ServerName::from),
        None => ServerName::try_from(host).ok().map(|name| name.to_owned()),
    };

    match server_name {
        Some(server_name) => Ok(server_name),
        None => IncompatibleOptionsSnafu {
            msg: format!(
                "`GrpcClient__OverrideTargetName` names the host `{host}`, which no certificate can \
                 be verified against. It has to be a DNS name or an IP address, as in \
                 `server.example.com`, `10.0.0.1` or `[::1]`"
            ),
        }
        .fail()
        .context(ConfigSnafu {}),
    }
}

/// Everything that can go wrong between a [`ClientConfig`] and a usable channel.
#[derive(Debug, Snafu)]
#[non_exhaustive]
// snafu keeps its context selectors module-private by default. Public so a caller in another crate
// can build one of these errors with the location captured at its own call site.
#[snafu(visibility(pub))]
pub enum ConnectionError {
    #[snafu(display("Could not read the client config [{location}]"))]
    #[non_exhaustive]
    Config {
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not connect to the remote {endpoint} [{location}]"))]
    #[non_exhaustive]
    Transport {
        endpoint: Uri,
        #[snafu(source(from(tonic::transport::Error, Box::new)))]
        source: Box<tonic::transport::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not establish TLS connection to the remote {endpoint} [{location}]"))]
    #[non_exhaustive]
    Tls {
        endpoint: Uri,
        #[snafu(source(from(rustls::Error, Box::new)))]
        source: Box<rustls::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not read system cert store [{location}]"))]
    #[non_exhaustive]
    Io {
        #[snafu(source(from(std::io::Error, Box::new)))]
        source: Box<std::io::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientConfigArgs;

    /// A configuration whose only interesting part is the override target. Unsafe connections so that
    /// building the connector reads no certificate store.
    fn config(override_target_name: &str) -> ClientConfig {
        ClientConfig::from_config_args(ClientConfigArgs {
            endpoint: String::from("https://10.0.0.1:5003"),
            override_target_name: String::from(override_target_name),
            allow_unsafe_connection: true,
            ..Default::default()
        })
        .expect("the override target should be a valid authority")
    }

    /// The name derived from an override target, for a value that yields one.
    fn server_name(override_target_name: &str) -> ServerName<'static> {
        let target = config(override_target_name)
            .override_target
            .expect("an override target");
        override_server_name(&target).expect("the host should name something verifiable")
    }

    /// Every message in the chain, joined: the option name is in the cause, not the outermost message.
    fn chain(error: &ConnectionError) -> String {
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
    fn an_override_written_as_a_bracketed_ipv6_literal_names_the_address() {
        // `[::1]` is how an IPv6 host is written in an authority, and `http` hands the brackets back
        // with it. The name a certificate is checked against is the address inside them.
        assert_eq!(
            server_name("[::1]"),
            ServerName::from(IpAddr::try_from("::1").expect("an address")),
        );
        assert_eq!(
            server_name("[2001:db8::1]:5003"),
            ServerName::from(IpAddr::try_from("2001:db8::1").expect("an address")),
        );
    }

    #[test]
    fn an_override_written_as_a_dns_name_or_an_ipv4_address_is_taken_as_it_stands() {
        assert_eq!(
            server_name("server.example.com"),
            ServerName::try_from("server.example.com").expect("a name"),
        );
        assert_eq!(
            server_name("10.0.0.1:5003"),
            ServerName::from(IpAddr::try_from("10.0.0.1").expect("an address")),
        );
    }

    #[test]
    fn brackets_around_something_that_is_not_an_address_are_not_read_as_a_name() {
        // `http` balances the brackets without looking inside them, so `[example.com]` reaches here.
        // Dropping the brackets and taking what is left would verify against a host nobody wrote.
        let target = config("[example.com]")
            .override_target
            .expect("an override target");

        let error =
            override_server_name(&target).expect_err("brackets are an IP literal or nothing");
        assert!(matches!(error, ConnectionError::Config { .. }), "{error:?}");
    }

    #[test]
    fn a_host_that_names_nothing_verifiable_is_reported_against_its_option() {
        // Whoever set it has a dozen `GrpcClient__*` variables to choose from, so the message has to
        // name the one at fault and quote what it read.
        let target = config("-nope-")
            .override_target
            .expect("an override target");

        let error = override_server_name(&target).expect_err("a leading hyphen is not a DNS label");
        let rendered = chain(&error);
        assert!(
            rendered.contains("GrpcClient__OverrideTargetName"),
            "{rendered}"
        );
        assert!(rendered.contains("-nope-"), "{rendered}");
    }

    #[tokio::test]
    async fn a_bracketed_ipv6_override_builds_a_connector() {
        // The whole path, since the name is only pinned onto the connector at the end of it.
        https_connector(config("[::1]"))
            .await
            .expect("a bracketed IPv6 override is a valid one");
    }

    #[tokio::test]
    async fn an_override_that_names_nothing_verifiable_fails_rather_than_panics() {
        let error = https_connector(config("-nope-"))
            .await
            .expect_err("the connector cannot be built without a name to verify against");

        assert!(matches!(error, ConnectionError::Config { .. }), "{error:?}");
    }
}
