//! TLS and mTLS: the client's own identity, the server's CA, and the two options that change how
//! verification behaves rather than what is verified.
//!
//! Unlike the `Tcp`/`Http2` units, these fields share no common prefix in the environment (`CertPem`,
//! `CaCert`, `AllowUnsafeConnection`, `OverrideTargetName`, ...), so grouping them is a plain
//! [`serde(flatten)`](serde::Deserialize), with no [`serde_with::with_prefix!`] needed: every name
//! here already matches today's flat one exactly.

use hyper::Uri;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

#[cfg(feature = "serde")]
use crate::config::text;
use crate::config::{boxed, ConfigError, IncompatibleOptionsSnafu, IoSnafu, TlsSnafu, UriSnafu};
use snafu::ResultExt;

/// The client's TLS identity and the server's CA, in the string form a caller supplies them in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct TlsConfigArgs {
    /// A file this crate reads: the client's own certificate, matching `key_pem`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub cert_pem: String,
    /// A file this crate reads: the client's own key, matching `cert_pem`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub key_pem: String,
    /// A file this crate reads: the Certificate Authority.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub ca_cert: String,
    /// Accept any server certificate instead of verifying it, empty for false.
    ///
    /// Spelled as any other ArmoniK client accepts it: `1`, `true`, `yes`, `enable`, `allow` or
    /// `authorize`, and their negatives. A `serde` source may also give a real boolean.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub allow_unsafe_connection: String,
    /// Override the endpoint name during SSL verification
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub override_target_name: String,
}

/// The resolved form of [`TlsConfigArgs`].
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct TlsConfig {
    /// Allow unsafe connections to the endpoint (without SSL), defaults to false.
    pub allow_unsafe_connection: bool,
    /// TLS identity of the client: key + cert, loaded from `cert_pem`/`key_pem`.
    pub identity: Option<(CertificateDer<'static>, PrivateKeyDer<'static>)>,
    /// CA certificate to authenticate the server.
    pub cacert: Option<CertificateDer<'static>>,
    /// Override the endpoint name during SSL verification.
    pub override_target: Option<Uri>,
}

impl Clone for TlsConfig {
    fn clone(&self) -> Self {
        Self {
            allow_unsafe_connection: self.allow_unsafe_connection,
            identity: self
                .identity
                .as_ref()
                .map(|(cert, key)| (cert.clone(), key.clone_key())),
            cacert: self.cacert.clone(),
            override_target: self.override_target.clone(),
        }
    }
}

/// Reads `path` and parses it as one PEM-encoded certificate.
fn read_cert_pem(option: &'static str, path: &str) -> Result<CertificateDer<'static>, ConfigError> {
    let pem = std::fs::read_to_string(path).context(IoSnafu { option, path })?;
    CertificateDer::from_pem_slice(pem.as_bytes()).context(TlsSnafu {})
}

/// Reads `path`, `key_pem`'s own file, whose loaded bytes are as sensitive as the key they carry
/// the moment they leave the filesystem.
fn read_key_pem(option: &'static str, path: &str) -> Result<PrivateKeyDer<'static>, ConfigError> {
    let pem = std::fs::read_to_string(path).context(IoSnafu { option, path })?;
    PrivateKeyDer::from_pem_slice(pem.as_bytes()).context(TlsSnafu {})
}

impl TlsConfigArgs {
    /// Resolves against `endpoint`: an override target given as a bare host keeps the endpoint's own
    /// scheme and path, and is otherwise a full URI whose authority and path replace the endpoint's,
    /// but never its scheme, since the connection is still made to the endpoint. Only the name it is
    /// verified against changes.
    pub(crate) fn resolve(self, endpoint: &Uri) -> Result<TlsConfig, ConfigError> {
        let Self {
            cert_pem,
            key_pem,
            ca_cert,
            allow_unsafe_connection,
            override_target_name,
        } = self;

        let cacert = if ca_cert.is_empty() {
            None
        } else {
            Some(read_cert_pem("ca_cert", &ca_cert)?)
        };

        let identity = match (cert_pem.is_empty(), key_pem.is_empty()) {
            (true, true) => None,
            (false, false) => Some((
                read_cert_pem("cert_pem", &cert_pem)?,
                read_key_pem("key_pem", &key_pem)?,
            )),
            _ => {
                return IncompatibleOptionsSnafu {
                    msg: String::from(
                        "`cert_pem` and `key_pem` must be either both empty or both set",
                    ),
                }
                .fail()
            }
        };

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
                } = Uri::try_from(override_target_name.as_str())
                    .map_err(boxed)
                    .context(UriSnafu {
                        option: "override_target_name",
                        uri: override_target_name.clone(),
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

            Some(uri.build().map_err(boxed).context(UriSnafu {
                option: "override_target_name",
                uri: override_target_name,
            })?)
        };

        Ok(TlsConfig {
            allow_unsafe_connection: crate::config::parse_bool(
                "allow_unsafe_connection",
                &allow_unsafe_connection,
            )?,
            identity,
            cacert,
            override_target,
        })
    }
}
