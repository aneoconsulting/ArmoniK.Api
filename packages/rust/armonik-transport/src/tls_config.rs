//! TLS and mTLS: the client's own identity, the server's CA, and the two options that change how
//! verification behaves rather than what is verified.
//!
//! Unlike the `Tcp`/`Http2` units, these fields share no common prefix in the environment
//! (`CertPem`, `CaCert`, `AllowUnsafeConnection`, `OverrideTargetName`, ...), so grouping them is a
//! plain [`serde(flatten)`](serde::Deserialize), with no [`serde_with::with_prefix!`] needed.
//!
//! The options name files; nothing here reads one. [`TlsConfig::resolve`] does, when the
//! connection is made, so a configuration can be built on one machine and used on another, and a
//! path that leads nowhere fails where the connection attempt can report it.

use std::path::PathBuf;

use hyper::Uri;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use snafu::ResultExt;

#[cfg(feature = "serde")]
use crate::config::IncompatibleOptionsSnafu;
use crate::config::{ConfigError, HttpSnafu, IoSnafu, TlsSnafu, UriSnafu};

/// Where the client's TLS identity comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdentitySource {
    /// A certificate and its key, each in its own PEM file.
    PemFiles {
        /// Path to the certificate file, in PEM format. `CertPem`.
        cert_pem: PathBuf,
        /// Path to the key file, in PEM format. `KeyPem`.
        key_pem: PathBuf,
    },
}

/// TLS and mTLS: the client's own identity, the server's CA, and SSL verification behaviour.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(try_from = "RawTls")
)]
#[non_exhaustive]
pub struct TlsConfig {
    /// Accept any server certificate instead of verifying it, defaults to false.
    /// `AllowUnsafeConnection`.
    ///
    /// Spelled as any other ArmoniK client accepts it: `1`, `true`, `yes`, `enable`, `allow` or
    /// `authorize`, and their negatives. A `serde` source may also give a real boolean.
    pub allow_unsafe_connection: bool,
    /// TLS identity of the client, `None` for no client authentication.
    pub identity: Option<IdentitySource>,
    /// Path to the Certificate Authority file in PEM format, `None` for the system CAs. `CaCert`.
    pub ca_cert: Option<PathBuf>,
    /// Override the endpoint name during SSL verification. `OverrideTargetName`.
    pub override_target_name: Option<String>,
}

/// The flat string options [`TlsConfig`] is read from, one per field, all optional.
///
/// Every field tolerates the eager typing a `serde` source may apply (a bare `true` arriving as a
/// real boolean), and an empty string means unset, the same as an absent key: a deployment that
/// declares a variable with an empty default must not differ from one that leaves it out.
#[cfg(feature = "serde")]
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "PascalCase", default)]
struct RawTls {
    #[serde(deserialize_with = "crate::config::text")]
    cert_pem: String,
    #[serde(deserialize_with = "crate::config::text")]
    key_pem: String,
    #[serde(deserialize_with = "crate::config::text")]
    ca_cert: String,
    #[serde(deserialize_with = "crate::config::text")]
    allow_unsafe_connection: String,
    #[serde(deserialize_with = "crate::config::text")]
    override_target_name: String,
}

#[cfg(feature = "serde")]
impl TryFrom<RawTls> for TlsConfig {
    type Error = ConfigError;

    fn try_from(raw: RawTls) -> Result<Self, Self::Error> {
        let RawTls {
            cert_pem,
            key_pem,
            ca_cert,
            allow_unsafe_connection,
            override_target_name,
        } = raw;

        // Half an identity is silent on a plain-TLS endpoint and only surfaces as a rejected
        // handshake on an mTLS one, so it is caught here, before either path is opened.
        let identity = match (cert_pem.is_empty(), key_pem.is_empty()) {
            (true, true) => None,
            (false, false) => Some(IdentitySource::PemFiles {
                cert_pem: PathBuf::from(cert_pem),
                key_pem: PathBuf::from(key_pem),
            }),
            _ => {
                return IncompatibleOptionsSnafu {
                    msg: String::from(
                        "`CertPem` and `KeyPem` must be either both empty or both set",
                    ),
                }
                .fail()
            }
        };

        Ok(Self {
            allow_unsafe_connection: crate::config::parse_bool(
                "AllowUnsafeConnection",
                &allow_unsafe_connection,
            )
            .map_err(|msg| IncompatibleOptionsSnafu { msg }.build())?,
            identity,
            ca_cert: if ca_cert.is_empty() {
                None
            } else {
                Some(PathBuf::from(ca_cert))
            },
            override_target_name: if override_target_name.is_empty() {
                None
            } else {
                Some(override_target_name)
            },
        })
    }
}

/// The resolved form of [`TlsConfig`]: files read, and the override target built against the
/// endpoint.
#[derive(Debug)]
pub(crate) struct ResolvedTls {
    pub(crate) allow_unsafe_connection: bool,
    pub(crate) identity: Option<(CertificateDer<'static>, PrivateKeyDer<'static>)>,
    pub(crate) cacert: Option<CertificateDer<'static>>,
    pub(crate) override_target: Option<Uri>,
}

impl TlsConfig {
    /// Read the files the options name, and resolve the override target against `endpoint`: an
    /// override given as a bare host keeps the endpoint's own scheme and path, and is otherwise a
    /// full URI whose authority and path replace the endpoint's, but never its scheme, since the
    /// connection is still made to the endpoint. Only the name it is verified against changes.
    pub(crate) fn resolve(&self, endpoint: &Uri) -> Result<ResolvedTls, ConfigError> {
        let cacert = match &self.ca_cert {
            None => None,
            Some(path) => {
                let pem = std::fs::read_to_string(path).context(IoSnafu {
                    path: path.display().to_string(),
                })?;
                Some(CertificateDer::from_pem_slice(pem.as_bytes()).context(TlsSnafu {})?)
            }
        };

        let identity = match &self.identity {
            None => None,
            Some(IdentitySource::PemFiles { cert_pem, key_pem }) => {
                let cert = std::fs::read_to_string(cert_pem).context(IoSnafu {
                    path: cert_pem.display().to_string(),
                })?;
                let key = std::fs::read(key_pem).context(IoSnafu {
                    path: key_pem.display().to_string(),
                })?;
                Some((
                    CertificateDer::from_pem_slice(cert.as_bytes()).context(TlsSnafu {})?,
                    PrivateKeyDer::from_pem_slice(key.as_slice()).context(TlsSnafu {})?,
                ))
            }
        };

        let override_target = match &self.override_target_name {
            None => None,
            Some(name) => {
                let authority;
                let path_and_query;

                if let Ok(auth) = name.parse::<hyper::http::uri::Authority>() {
                    authority = Some(auth);
                    path_and_query = endpoint.path_and_query().cloned();
                } else {
                    hyper::http::uri::Parts {
                        authority,
                        path_and_query,
                        ..
                    } = Uri::try_from(name.as_str())
                        .context(UriSnafu { uri: name.clone() })?
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

                Some(uri.build().context(HttpSnafu { uri: name.clone() })?)
            }
        };

        Ok(ResolvedTls {
            allow_unsafe_connection: self.allow_unsafe_connection,
            identity,
            cacert,
            override_target,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every message in the chain, joined. snafu keeps the detail in the source, so asserting on
    /// the outermost `Display` alone would pass whatever the cause turned out to be.
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

    fn endpoint() -> Uri {
        Uri::try_from("http://localhost:5001").expect("a valid endpoint")
    }

    #[test]
    fn an_empty_configuration_resolves_to_nothing() {
        let resolved = TlsConfig::default()
            .resolve(&endpoint())
            .expect("nothing to read");

        assert!(resolved.identity.is_none());
        assert!(resolved.cacert.is_none());
        assert_eq!(resolved.override_target, None);
        assert!(!resolved.allow_unsafe_connection);
    }

    #[test]
    fn a_certificate_path_that_does_not_exist_is_reported_with_the_path() {
        // These options are paths, not contents. A typo in one has to name the file rather than
        // surface later as a TLS failure.
        let config = TlsConfig {
            identity: Some(IdentitySource::PemFiles {
                cert_pem: PathBuf::from("no/such/cert.pem"),
                key_pem: PathBuf::from("no/such/key.pem"),
            }),
            ..TlsConfig::default()
        };

        let error = config
            .resolve(&endpoint())
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
        let config = TlsConfig {
            ca_cert: Some(PathBuf::from("no/such/ca.pem")),
            ..TlsConfig::default()
        };

        let error = config
            .resolve(&endpoint())
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
        // The common case: the certificate names one host, the endpoint is reached at another. Only
        // the authority is being overridden, so everything else has to come from the endpoint.
        let config = TlsConfig {
            override_target_name: Some(String::from("server.example.com")),
            ..TlsConfig::default()
        };

        let resolved = config
            .resolve(&Uri::try_from("https://10.0.0.1:5003/base").expect("a valid endpoint"))
            .expect("valid");

        let override_target = resolved.override_target.expect("an override target");
        assert_eq!(override_target.scheme_str(), Some("https"));
        assert_eq!(
            override_target.authority().map(|a| a.as_str()),
            Some("server.example.com")
        );
        assert_eq!(override_target.path(), "/base");
    }

    #[test]
    fn an_override_target_given_as_a_uri_replaces_the_authority_and_the_path() {
        let config = TlsConfig {
            override_target_name: Some(String::from("https://server.example.com/other")),
            ..TlsConfig::default()
        };

        let resolved = config
            .resolve(&Uri::try_from("https://10.0.0.1:5003/base").expect("a valid endpoint"))
            .expect("valid");

        let override_target = resolved.override_target.expect("an override target");
        assert_eq!(
            override_target.authority().map(|a| a.as_str()),
            Some("server.example.com")
        );
        assert_eq!(override_target.path(), "/other");
        // The scheme still comes from the endpoint: the connection is made to the endpoint, and
        // this only changes the name it is verified against.
        assert_eq!(override_target.scheme_str(), Some("https"));
    }

    #[test]
    fn no_override_target_leaves_it_unset() {
        let resolved = TlsConfig::default().resolve(&endpoint()).expect("valid");
        assert_eq!(resolved.override_target, None);
    }
}
