//! Turning an [`HttpConfig`] into a connected `tonic` channel.
//!
//! TLS, mTLS, and every timeout, keepalive and identity setting come together here. The CA file
//! the configuration names is read here too; the identity was already loaded when the
//! configuration was read, so only the trust side is left to resolve.

use std::sync::Arc;

use hyper::Uri;
use hyper_rustls::{ConfigBuilderExt, FixedServerNameResolver, HttpsConnector};
use hyper_util::client::legacy::connect::HttpConnector;
use rustls::pki_types::ServerName;
use snafu::{IntoError, ResultExt, Snafu};

use crate::config::ConfigError;
use crate::proxy::ProxyConnector;
use crate::tls_config::ResolvedTls;
use crate::HttpConfig;

/// Connect to the endpoint described by `config`, eagerly: this resolves once the connection is
/// established, not lazily on the first request.
pub async fn connect(config: HttpConfig) -> Result<tonic::transport::Channel, ConnectionError> {
    let tls = resolve(&config)?;
    let endpoint = config.endpoint.clone();
    let override_target = tls.override_target.clone();
    let http2 = config.http2;
    let user_agent = config.user_agent.clone();
    let timeout = config.timeout;
    let rate_limit = config.rate_limit;

    let https = build_connector(config, tls)?;

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

    if let Some(interval) = http2.keep_alive_interval {
        transport_endpoint = transport_endpoint.http2_keep_alive_interval(interval);
    }
    if let Some(timeout) = http2.keep_alive_timeout {
        transport_endpoint = transport_endpoint.keep_alive_timeout(timeout);
    }
    transport_endpoint = transport_endpoint.keep_alive_while_idle(http2.keep_alive_while_idle);
    if let Some(max) = http2.max_header_list_size {
        transport_endpoint = transport_endpoint.http2_max_header_list_size(max);
    }
    if let Some(ua) = user_agent {
        transport_endpoint = transport_endpoint
            .user_agent(ua)
            .expect("HeaderValue is already validated, conversion is infallible");
    }

    // Build the actual channel from the configuration
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
    config: HttpConfig,
) -> Result<HttpsConnector<ProxyConnector<HttpConnector>>, ConnectionError> {
    let tls = resolve(&config)?;
    build_connector(config, tls)
}

/// Reject an empty endpoint and read the CA file the TLS options name.
///
/// The endpoint check lives here rather than at deserialise time: an unset endpoint reads as the
/// default [`Uri`] so a configuration file need only name what it changes, and the option that is
/// actually missing gets named by the connection attempt that needed it.
fn resolve(config: &HttpConfig) -> Result<ResolvedTls, ConnectionError> {
    if config.endpoint == Uri::default() {
        let error = crate::config::IncompatibleOptionsSnafu {
            msg: String::from("`Endpoint` is not set, so there is nothing to connect to"),
        }
        .build();
        return Err(ConfigSnafu.into_error(error));
    }
    config.tls.resolve(&config.endpoint).context(ConfigSnafu)
}

/// The connector stack itself, from a configuration whose TLS half is already resolved.
fn build_connector(
    config: HttpConfig,
    tls: ResolvedTls,
) -> Result<HttpsConnector<ProxyConnector<HttpConnector>>, ConnectionError> {
    let endpoint = config.endpoint;

    // Get the default crypto provider or fallback to the ring crypto provider
    let crypto_provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));

    // Configure TLS with sane protocol defaults
    let tls_config = rustls::ClientConfig::builder_with_provider(crypto_provider)
        .with_safe_default_protocol_versions()
        .with_context(|_| TlsSnafu {
            endpoint: endpoint.clone(),
        })?;

    // Configure the server verification
    let tls_config = if tls.allow_unsafe_connection {
        // Do not verify the server
        tls_config
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(crate::utils::InsecureCertVerifier))
    } else if let Some(cacert) = tls.cacert {
        // Verify that the server certificate is signed with a specific CA cert
        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.add(cacert).with_context(|_| TlsSnafu {
            endpoint: endpoint.clone(),
        })?;
        tls_config.with_root_certificates(root_cert_store)
    } else {
        // Verify the server certificate using the system CAs
        tls_config
            .with_native_roots()
            .with_context(|_| IoSnafu {})?
    };

    // Configure client identity for mTLS
    let tls_config = if let Some(identity) = config.tls.identity {
        // Present the loaded chain, leaf first, and authenticate with its key
        tls_config
            .with_client_auth_cert(identity.certs, identity.key)
            .with_context(|_| TlsSnafu {
                endpoint: endpoint.clone(),
            })?
    } else {
        // No mTLS
        tls_config.with_no_client_auth()
    };

    // Configure the connector to use http or https depending on the URI scheme
    let mut https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls_config)
        .https_or_http();

    if let Some(hostname) = &tls.override_target {
        let server_name = ServerName::try_from(hostname.host().unwrap_or_default())
            .expect("A valid URI host should be a valid ServerName")
            .to_owned();
        https = https.with_server_name_resolver(FixedServerNameResolver::new(server_name));
    };

    let mut http = HttpConnector::new();
    http.enforce_http(false); // required for hyper-rustls to switch schemes
    http.set_nodelay(!config.tcp.nagle_algorithm);
    http.set_keepalive(config.tcp.keepalive);
    http.set_keepalive_interval(config.tcp.keepalive_interval);
    http.set_keepalive_retries(config.tcp.keepalive_retries);
    if let Some(timeout) = config.connect_timeout {
        http.set_connect_timeout(Some(timeout));
    }

    // Tunnelling sits below TLS, so the handshake above still targets the real server. With no proxy
    // configured this delegates straight to the connector it wraps, settings and all.
    let http = ProxyConnector::new(http, config.proxy, config.connect_timeout);

    Ok(https.enable_http1().enable_http2().wrap_connector(http))
}

/// Everything that can go wrong between an [`HttpConfig`] and a usable channel.
#[derive(Debug, Snafu)]
#[non_exhaustive]
// snafu keeps its generated context selectors module-private by default. Public so that a caller in
// another crate can build one of these errors with the location captured at its own call site.
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

    /// Every message in the chain, joined.
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

    #[tokio::test]
    async fn an_empty_endpoint_is_rejected_before_anything_is_dialled() {
        // An unset endpoint deserialises to the default URI rather than failing, so the rejection
        // has to happen here, where the error can name the option.
        let error = https_connector(HttpConfig::default())
            .await
            .expect_err("an empty endpoint cannot be connected to");

        assert!(matches!(error, ConnectionError::Config { .. }), "{error:?}");
        assert!(
            chain(&error).contains("`Endpoint` is not set"),
            "{}",
            chain(&error)
        );
    }
}
