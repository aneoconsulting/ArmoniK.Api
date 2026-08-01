//! Turning a [`ClientConfig`] into a connected `tonic` channel.
//!
//! TLS, mTLS, and every timeout, keepalive and identity setting come together here.

use std::sync::Arc;

use hyper::Uri;
use hyper_rustls::{ConfigBuilderExt, FixedServerNameResolver, HttpsConnector};
use rustls::pki_types::ServerName;
use snafu::{ResultExt, Snafu};

use crate::config::ConfigError;
use crate::proxy::ProxyConnector;
use crate::tcp::TcpConnector;
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
    config: ClientConfig,
) -> Result<HttpsConnector<ProxyConnector<TcpConnector>>, ConnectionError> {
    // Built first, while `config` is whole: the fields below are moved out of it.
    let tcp = TcpConnector::new(&config);

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
            .with_custom_certificate_verifier(Arc::new(InsecureCertVerifier))
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

    // Tunnelling sits below TLS, so the handshake above still targets the real server. With no proxy
    // configured this delegates straight to the connector it wraps, settings and all.
    let http = ProxyConnector::new(tcp, config.proxy);

    Ok(https.enable_http1().enable_http2().wrap_connector(http))
}

/// Accepts any certificate, which is what `allow_unsafe_connection` asks for.
#[derive(Debug)]
pub(crate) struct InsecureCertVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

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
