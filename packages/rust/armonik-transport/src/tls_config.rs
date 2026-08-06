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
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use secrecy::ExposeSecret;
use snafu::{OptionExt, ResultExt};

#[cfg(feature = "serde")]
use crate::config::IncompatibleOptionsSnafu;
use crate::config::{
    ConfigError, EmptyPkcs12Snafu, HttpSnafu, IoSnafu, Pkcs12Snafu, TlsSnafu, UriSnafu,
};

/// Where the client's TLS identity comes from.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum IdentitySource {
    /// A certificate and its key, each in its own PEM file.
    PemFiles {
        /// Path to the certificate file, in PEM format: the leaf first, then any intermediates,
        /// as PEM chains are conventionally laid out. `CertPem`.
        cert_pem: PathBuf,
        /// Path to the key file, in PEM format. `KeyPem`.
        key_pem: PathBuf,
    },
    /// A certificate and its key bundled together in one PKCS#12 file, the form Windows and most
    /// certificate authorities hand out.
    Pkcs12 {
        /// Path to the PKCS#12 bundle. Any intermediates it carries are kept, leaf first.
        /// `CertP12`.
        cert_p12: PathBuf,
        /// The password protecting the bundle, `None` for none. `CertP12Password`. Redacted by
        /// `Debug` and zeroized on drop.
        cert_p12_password: Option<secrecy::SecretString>,
    },
}

/// Not derived: [`secrecy::SecretString`] deliberately implements no `PartialEq`. What is compared
/// here is two configurations, not a credential against a guess, so exposing the passwords for the
/// comparison is fine.
impl PartialEq for IdentitySource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::PemFiles { cert_pem, key_pem },
                Self::PemFiles {
                    cert_pem: other_cert_pem,
                    key_pem: other_key_pem,
                },
            ) => cert_pem == other_cert_pem && key_pem == other_key_pem,
            (
                Self::Pkcs12 {
                    cert_p12,
                    cert_p12_password,
                },
                Self::Pkcs12 {
                    cert_p12: other_cert_p12,
                    cert_p12_password: other_password,
                },
            ) => {
                cert_p12 == other_cert_p12
                    && cert_p12_password.as_ref().map(ExposeSecret::expose_secret)
                        == other_password.as_ref().map(ExposeSecret::expose_secret)
            }
            _ => false,
        }
    }
}

impl Eq for IdentitySource {}

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
    cert_p12: String,
    #[serde(deserialize_with = "crate::config::secret_text")]
    cert_p12_password: secrecy::SecretString,
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
            cert_p12,
            cert_p12_password,
            ca_cert,
            allow_unsafe_connection,
            override_target_name,
        } = raw;

        // Both spellings of the identity at once is a contradiction to reject, not a preference
        // to resolve silently.
        if !cert_p12.is_empty() && (!cert_pem.is_empty() || !key_pem.is_empty()) {
            return IncompatibleOptionsSnafu {
                msg: String::from(
                    "`CertP12` and `CertPem`/`KeyPem` name the client identity two different \
                     ways; set only one",
                ),
            }
            .fail();
        }
        // A password naming no bundle is a typo somewhere; honouring half of it would hide it.
        if cert_p12.is_empty() && !cert_p12_password.expose_secret().is_empty() {
            return IncompatibleOptionsSnafu {
                msg: String::from("`CertP12Password` is set without `CertP12`"),
            }
            .fail();
        }

        let identity = if !cert_p12.is_empty() {
            Some(IdentitySource::Pkcs12 {
                cert_p12: PathBuf::from(cert_p12),
                cert_p12_password: if cert_p12_password.expose_secret().is_empty() {
                    None
                } else {
                    Some(cert_p12_password)
                },
            })
        } else {
            // Half an identity is silent on a plain-TLS endpoint and only surfaces as a rejected
            // handshake on an mTLS one, so it is caught here, before either path is opened.
            match (cert_pem.is_empty(), key_pem.is_empty()) {
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
    /// The client's certificate chain, leaf first, and its key. The whole chain is kept: a server
    /// that trusts only the root needs the intermediates to build its path.
    pub(crate) identity: Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
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
                // Every certificate in the file, not just the first: a PEM file conventionally
                // carries the leaf followed by its intermediates, and dropping those breaks
                // against a server that trusts only the root. A file with none is still an error.
                let certs = CertificateDer::pem_slice_iter(cert.as_bytes())
                    .collect::<Result<Vec<_>, _>>()
                    .and_then(|certs| {
                        if certs.is_empty() {
                            Err(rustls::pki_types::pem::Error::NoItemsFound)
                        } else {
                            Ok(certs)
                        }
                    })
                    .context(TlsSnafu {})?;
                Some((
                    certs,
                    PrivateKeyDer::from_pem_slice(key.as_slice()).context(TlsSnafu {})?,
                ))
            }
            Some(IdentitySource::Pkcs12 {
                cert_p12,
                cert_p12_password,
            }) => {
                let path = cert_p12.display().to_string();
                let data = std::fs::read(cert_p12).context(IoSnafu { path: path.clone() })?;
                // An absent password opens an unprotected bundle the same way an empty one does.
                let password = cert_p12_password
                    .as_ref()
                    .map_or("", ExposeSecret::expose_secret);
                let keystore = p12_keystore::KeyStore::from_pkcs12(
                    &data,
                    password,
                    p12_keystore::Pkcs12ImportPolicy::Strict,
                )
                .context(Pkcs12Snafu { path: path.clone() })?;
                let (_, chain) = keystore
                    .private_key_chain()
                    .context(EmptyPkcs12Snafu { path: path.clone() })?;
                // The whole chain, in the leaf-first order `p12-keystore` rebuilds it in (the
                // certificate the key names, then each issuer upward), which is the order rustls
                // sends it in; under the `Strict` import policy the chain carries at least that
                // leaf. Re-encoded into the same DER shapes the PEM pair produces, so everything
                // past this point sees one identity format.
                let certs: Vec<CertificateDer<'static>> = chain
                    .certs()
                    .iter()
                    .map(|cert| CertificateDer::from(cert.as_der().to_vec()))
                    .collect();
                let key = chain.key().as_der().to_vec();
                Some((certs, PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key))))
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

    // --- PKCS#12 ---

    /// A fresh self-signed identity written into PKCS#12 bytes with `p12-keystore`'s own writer,
    /// generated rather than committed so no fixture can expire. Returns the bundle and the DER
    /// forms the resolve is expected to hand back.
    fn p12_bundle(password: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
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
        let pfx = keystore.writer(password).write().expect("write the bundle");
        (pfx, cert_der, key_der)
    }

    /// A CA-signed identity, generated fresh like the self-signed one: the CA signs itself, and
    /// the leaf is signed by it. The chain tests use it to assert that both certificates survive
    /// the resolve, in order.
    fn ca_signed_identity() -> (rcgen::Certificate, rcgen::KeyPair, rcgen::Certificate) {
        let ca_key = rcgen::KeyPair::generate().expect("a CA key");
        let mut ca_params = rcgen::CertificateParams::new(Vec::new()).expect("CA params");
        ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        ca_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "test ca");
        let ca_cert = ca_params.self_signed(&ca_key).expect("a CA certificate");

        let leaf_key = rcgen::KeyPair::generate().expect("a leaf key");
        let leaf_params =
            rcgen::CertificateParams::new(vec!["test".to_owned()]).expect("leaf params");
        let issuer = rcgen::Issuer::new(ca_params, ca_key);
        let leaf_cert = leaf_params
            .signed_by(&leaf_key, &issuer)
            .expect("a leaf certificate");

        (leaf_cert, leaf_key, ca_cert)
    }

    /// `bytes`, written to a file the test owns, so the option under test names a real path.
    fn temp_file(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        std::io::Write::write_all(&mut file, bytes).expect("write");
        file
    }

    /// A configuration whose identity is the PKCS#12 bundle at `path`.
    fn p12_config(path: &std::path::Path, password: Option<&str>) -> TlsConfig {
        TlsConfig {
            identity: Some(IdentitySource::Pkcs12 {
                cert_p12: path.to_path_buf(),
                cert_p12_password: password.map(secrecy::SecretString::from),
            }),
            ..TlsConfig::default()
        }
    }

    #[test]
    fn a_p12_bundle_is_read_into_the_same_identity_a_pem_pair_would_be() {
        let (pfx, cert_der, key_der) = p12_bundle("s3cr3t");
        let file = temp_file(&pfx);

        let resolved = p12_config(file.path(), Some("s3cr3t"))
            .resolve(&endpoint())
            .expect("a valid PKCS#12 bundle");

        let (certs, key) = resolved.identity.expect("an identity was bundled");
        assert_eq!(certs.len(), 1, "one certificate went in, one comes out");
        assert_eq!(
            certs[0].as_ref(),
            cert_der,
            "the leaf certificate round-trips"
        );
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
    fn a_p12_bundle_keeps_its_intermediates_leaf_first() {
        // A server that trusts only the root rebuilds its path through the intermediates the
        // client sends; a resolve that kept only the leaf would fail that handshake.
        let (leaf, leaf_key, ca) = ca_signed_identity();
        let chain = p12_keystore::PrivateKeyChain::new(
            [1u8].as_slice(),
            p12_keystore::PrivateKey::from_der(&leaf_key.serialize_der())
                .expect("a valid PKCS#8 key"),
            [
                p12_keystore::Certificate::from_der(leaf.der().as_ref()).expect("a valid leaf"),
                p12_keystore::Certificate::from_der(ca.der().as_ref()).expect("a valid CA"),
            ],
        );
        let mut keystore = p12_keystore::KeyStore::new();
        keystore.add_entry(
            "identity",
            p12_keystore::KeyStoreEntry::PrivateKeyChain(chain),
        );
        let pfx = keystore.writer("s3cr3t").write().expect("write the bundle");
        let file = temp_file(&pfx);

        let resolved = p12_config(file.path(), Some("s3cr3t"))
            .resolve(&endpoint())
            .expect("a valid PKCS#12 bundle");

        let (certs, _) = resolved.identity.expect("an identity was bundled");
        let ders: Vec<&[u8]> = certs.iter().map(AsRef::as_ref).collect();
        assert_eq!(
            ders,
            vec![leaf.der().as_ref(), ca.der().as_ref()],
            "the whole chain survives, leaf first"
        );
    }

    #[test]
    fn no_password_opens_a_bundle_protected_by_the_empty_one() {
        // The empty password is what an "unprotected" PKCS#12 bundle is actually written with, so
        // an absent option has to open it.
        let (pfx, _, _) = p12_bundle("");
        let file = temp_file(&pfx);

        let resolved = p12_config(file.path(), None)
            .resolve(&endpoint())
            .expect("an unprotected bundle needs no password");

        assert!(resolved.identity.is_some());
    }

    #[test]
    fn a_wrong_p12_password_is_reported_with_the_path() {
        // The path, not the password: whoever reads the error must learn which file refused to
        // open, and nothing about what was tried.
        let (pfx, _, _) = p12_bundle("right");
        let file = temp_file(&pfx);

        let error = p12_config(file.path(), Some("wrong"))
            .resolve(&endpoint())
            .expect_err("the wrong password must be rejected");

        assert!(matches!(error, ConfigError::Pkcs12 { .. }), "{error:?}");
        let rendered = chain(&error);
        assert!(
            rendered.contains(&file.path().display().to_string()),
            "{rendered}"
        );
        assert!(!rendered.contains("wrong"), "{rendered}");
    }

    #[test]
    fn a_p12_bundle_with_no_identity_is_rejected_as_empty() {
        // A bundle can be perfectly valid PKCS#12 and still carry no private key chain, which is
        // its own story: nothing is malformed, there is just no identity inside.
        let pfx = p12_keystore::KeyStore::new()
            .writer("s3cr3t")
            .write()
            .expect("write the bundle");
        let file = temp_file(&pfx);

        let error = p12_config(file.path(), Some("s3cr3t"))
            .resolve(&endpoint())
            .expect_err("an identity-less bundle must be rejected");

        assert!(
            matches!(error, ConfigError::EmptyPkcs12 { .. }),
            "{error:?}"
        );
        assert!(
            chain(&error).contains(&file.path().display().to_string()),
            "{}",
            chain(&error)
        );
    }

    #[test]
    fn a_file_that_is_not_pkcs12_is_rejected_as_such() {
        let file = temp_file(b"clearly not a pkcs12 bundle");

        let error = p12_config(file.path(), None)
            .resolve(&endpoint())
            .expect_err("garbage is not a pkcs12 bundle");

        assert!(matches!(error, ConfigError::Pkcs12 { .. }), "{error:?}");
    }

    #[test]
    fn a_p12_path_that_does_not_exist_is_reported_with_the_path() {
        let error = p12_config(std::path::Path::new("no/such/identity.p12"), None)
            .resolve(&endpoint())
            .expect_err("a missing file must be reported");

        assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
        assert!(
            chain(&error).contains("no/such/identity.p12"),
            "{}",
            chain(&error)
        );
    }

    // --- PEM chains ---

    /// A configuration whose identity is the PEM pair at `cert_pem`/`key_pem`.
    fn pem_config(cert_pem: &std::path::Path, key_pem: &std::path::Path) -> TlsConfig {
        TlsConfig {
            identity: Some(IdentitySource::PemFiles {
                cert_pem: cert_pem.to_path_buf(),
                key_pem: key_pem.to_path_buf(),
            }),
            ..TlsConfig::default()
        }
    }

    #[test]
    fn a_single_certificate_pem_pair_resolves_to_a_one_certificate_chain() {
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(["test".to_owned()]).expect("a self-signed cert");
        let cert_file = temp_file(cert.pem().as_bytes());
        let key_file = temp_file(signing_key.serialize_pem().as_bytes());

        let resolved = pem_config(cert_file.path(), key_file.path())
            .resolve(&endpoint())
            .expect("a valid PEM pair");

        let (certs, _) = resolved.identity.expect("an identity was configured");
        assert_eq!(certs.len(), 1, "one certificate went in, one comes out");
        assert_eq!(certs[0].as_ref(), cert.der().as_ref());
    }

    #[test]
    fn a_pem_file_carrying_a_chain_keeps_every_certificate_in_order() {
        // The same defect the PKCS#12 chain test guards against, in the PEM spelling: the file
        // conventionally holds the leaf followed by its intermediates.
        let (leaf, leaf_key, ca) = ca_signed_identity();
        let cert_file = temp_file(format!("{}{}", leaf.pem(), ca.pem()).as_bytes());
        let key_file = temp_file(leaf_key.serialize_pem().as_bytes());

        let resolved = pem_config(cert_file.path(), key_file.path())
            .resolve(&endpoint())
            .expect("a valid PEM chain");

        let (certs, _) = resolved.identity.expect("an identity was configured");
        let ders: Vec<&[u8]> = certs.iter().map(AsRef::as_ref).collect();
        assert_eq!(
            ders,
            vec![leaf.der().as_ref(), ca.der().as_ref()],
            "the whole chain survives, leaf first"
        );
    }

    #[test]
    fn a_certificate_file_with_no_certificate_is_rejected() {
        // Reading the whole file instead of its first item must not turn emptiness into an empty
        // chain that only fails at the handshake.
        let cert_file = temp_file(b"");
        let key_file = temp_file(b"");

        let error = pem_config(cert_file.path(), key_file.path())
            .resolve(&endpoint())
            .expect_err("an empty certificate file must be rejected");

        assert!(matches!(error, ConfigError::Tls { .. }), "{error:?}");
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
