//! Turning a [`ClientConfig`] into a connected `tonic` channel.
//!
//! This is the whole point of the crate: TLS, mTLS and every timeout, keepalive and identity setting
//! come together here. Moved verbatim out of `armonik`'s `Client::with_config`, which now calls
//! [`connect`] and keeps only the part that is about a client rather than a connection.

use std::sync::Arc;

use hyper::Uri;
use hyper_rustls::{ConfigBuilderExt, FixedServerNameResolver, HttpsConnector};
use hyper_util::client::legacy::connect::HttpConnector;
use rustls::pki_types::ServerName;
use snafu::{IntoError, ResultExt, Snafu};

use crate::config::ConfigError;
use crate::ClientConfig;

/// Connect to the endpoint described by `config`, eagerly — this resolves once the connection is
/// actually established, not lazily on first request.
pub async fn connect(config: ClientConfig) -> Result<tonic::transport::Channel, ConnectionError> {
    let endpoint = config.endpoint.clone();
    let override_target = config.override_target.clone();
    let http2_keep_alive_interval = config.http2_keep_alive_interval;
    let http2_keep_alive_timeout = config.http2_keep_alive_timeout;
    let http2_keep_alive_while_idle = config.http2_keep_alive_while_idle;
    let http2_max_header_list_size = config.http2_max_header_list_size;
    let user_agent = config.user_agent.clone();

    let https = https_connector(config).await?;

    let mut transport_endpoint = tonic::transport::Endpoint::from(endpoint.clone());
    if let Some(target) = override_target {
        transport_endpoint = transport_endpoint.origin(target);
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

    // Build the actual channel from the configuration
    transport_endpoint
        .connect_with_connector(https)
        .await
        .context(TransportSnafu { endpoint })
}

/// Build the connector stack — TCP, then TLS or mTLS — that [`connect`] wraps in a channel.
///
/// Most callers want [`connect`] instead. This exists so that a caller can drive plain HTTP requests
/// through the exact same connection configuration a [`connect`]ed channel would use: `armonik`'s test
/// helper does that to reach the mock server's diagnostic `/calls.json` endpoint.
///
/// Hidden from the documented surface because its return type names this crate's dependencies rather
/// than its own types; it is `pub` so the signature is expressible, not as an API to build against.
#[doc(hidden)]
pub async fn https_connector(
    config: ClientConfig,
) -> Result<HttpsConnector<HttpConnector>, ConnectionError> {
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
    let tls_config = if config.allow_unsafe_connection {
        // Do not verify the server
        tls_config
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(crate::utils::InsecureCertVerifier))
    } else if let Some(cacert) = config.cacert {
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
    let tls_config = if let Some((cert, key)) = config.identity {
        // Use the the specified client certificate and key for the client authentication
        tls_config
            .with_client_auth_cert(vec![cert], key)
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

    if let Some(hostname) = &config.override_target {
        let server_name = ServerName::try_from(hostname.host().unwrap_or_default())
            .expect("A valid URI host should be a valid ServerName")
            .to_owned();
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

/// Everything that can go wrong between a [`ClientConfig`] and a usable channel.
#[derive(Debug, Snafu)]
#[non_exhaustive]
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

impl From<ConfigError> for ConnectionError {
    /// So a caller can write `ClientConfig::from_env()?` and get a [`ConnectionError`].
    ///
    /// `armonik`'s `Client::new` did that with snafu's generated context selector, which is no longer
    /// reachable from there now that this error lives in another crate.
    fn from(source: ConfigError) -> Self {
        ConfigSnafu {}.into_error(source)
    }
}
