//! Turning an [`HttpConfig`] into the connector a request goes out through.
//!
//! TLS, mTLS, the proxy tunnel and every socket setting come together here. The certificates were
//! loaded when the configuration was read.
//!
//! This is where the crate stops: whoever wants a channel wraps the connector in an HTTP/2 engine
//! of their own.

use std::sync::Arc;

use hyper::Uri;
use hyper_rustls::{ConfigBuilderExt, FixedServerNameResolver, HttpsConnector};
use hyper_util::client::legacy::connect::HttpConnector;
use rustls::pki_types::ServerName;
use snafu::{ResultExt, Snafu};

use crate::proxy::ProxyConnector;
use crate::HttpConfig;

/// Build the connector stack, TCP then the proxy tunnel then TLS or mTLS, that reaches the endpoint
/// `config` names.
///
/// `origin` is what `config` converts to: the name requests are addressed to, which is also the name
/// the server certificate is verified against when `OverrideTargetName` moves it off the endpoint.
/// Taken rather than resolved here, so a caller that needs it for the engine it builds on top
/// resolves it once.
///
/// Hidden because its return type names this crate's dependencies rather than its own; `pub` only
/// so the signature is expressible.
#[doc(hidden)]
pub async fn https_connector(
    config: HttpConfig,
    origin: Uri,
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
    let tls_config = if config.tls.allow_unsafe_connection {
        // Do not verify the server
        tls_config
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(crate::utils::InsecureCertVerifier))
    } else if let Some(cacert) = config.tls.ca_cert {
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

    // Only when the option moves the name off the endpoint: with no override the origin is the
    // endpoint, which is the name the default resolver derives anyway.
    if config.tls.override_target_name.is_some() {
        let server_name = ServerName::try_from(origin.host().unwrap_or_default())
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

/// Everything that can go wrong between an [`HttpConfig`] and a usable connector.
#[derive(Debug, Snafu)]
#[non_exhaustive]
// snafu keeps its generated context selectors module-private by default. Public so that a caller in
// another crate can build one of these errors with the location captured at its own call site.
#[snafu(visibility(pub))]
pub enum ConnectionError {
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
