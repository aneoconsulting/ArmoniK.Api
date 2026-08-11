//! TLS and mTLS: the client's own identity, the server's CA, and the two options that change how
//! verification behaves rather than what is verified.
//!
//! Unlike the TCP and HTTP/2 units, these fields share no common prefix in the environment
//! (`CertPem`, `CaCertPath`, `AllowUnsafeConnection`, `OverrideTargetName`, ...), so the embedding
//! composes them with a plain [`serde(flatten)`](serde::Deserialize) and no prefix at all.
//!
//! Every option naming a file loads it while the configuration is read: [`TlsConfig`] holds the
//! material itself, so a programmatic caller hands certificates over as content (from a secret
//! store, say) without staging them on disk, and a mistyped path fails where the error can name
//! the option. Only the override target is left for connection time, and only because it is
//! resolved against the endpoint.

use std::path::Path;

use http::Uri;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use secrecy::ExposeSecret;
use snafu::{OptionExt, ResultExt};

use crate::config::{
    ConfigError, EmptyPkcs12Snafu, HttpSnafu, IncompatibleOptionsSnafu, IoSnafu, Pkcs12Snafu,
    TlsSnafu, UriSnafu,
};

/// The client's TLS identity for mTLS: the certificate chain it presents, and the chain's key.
///
/// Loaded material rather than paths, so it can be built from content directly; reading it from
/// files is [`Identity::from_pem_files`] or [`Identity::from_pkcs12`], which is also how the
/// `CertPem`/`KeyPem` and `CertP12` options are read.
#[derive(Debug, PartialEq, Eq)]
pub struct Identity {
    /// The certificate chain presented to the server, leaf first.
    pub certs: Vec<CertificateDer<'static>>,
    /// The private key of the leaf certificate.
    pub key: PrivateKeyDer<'static>,
}

impl Clone for Identity {
    /// Hand written: [`PrivateKeyDer`] offers [`clone_key`](PrivateKeyDer::clone_key) instead of
    /// `Clone`, so a key is only ever duplicated on purpose.
    fn clone(&self) -> Self {
        Self {
            certs: self.certs.clone(),
            key: self.key.clone_key(),
        }
    }
}

impl Identity {
    /// Load an identity from PEM files: the whole chain found in `cert_pem`, in the order the
    /// file writes it (leaf first), and the key in `key_pem`.
    pub fn from_pem_files(
        cert_pem: impl AsRef<Path>,
        key_pem: impl AsRef<Path>,
    ) -> Result<Self, ConfigError> {
        let cert_path = cert_pem.as_ref();
        let pem = std::fs::read(cert_path).context(IoSnafu {
            path: cert_path.display().to_string(),
        })?;
        let certs = CertificateDer::pem_slice_iter(&pem)
            .collect::<Result<Vec<_>, _>>()
            .context(TlsSnafu {})?;
        // The iterator yields nothing for a file with no certificate at all, where the single-item
        // reader would have said `no items found`; the check keeps that mistake loud.
        snafu::ensure!(
            !certs.is_empty(),
            IncompatibleOptionsSnafu {
                msg: format!("`{}` contains no certificate", cert_path.display()),
            }
        );

        let key_path = key_pem.as_ref();
        let key = std::fs::read(key_path).context(IoSnafu {
            path: key_path.display().to_string(),
        })?;
        let key = PrivateKeyDer::from_pem_slice(&key).context(TlsSnafu {})?;

        Ok(Self { certs, key })
    }

    /// Load an identity from a PKCS#12 bundle, the form Windows and most certificate authorities
    /// hand out: the chain it carries and the key that chain belongs to. `password` is `None` for
    /// a bundle that carries no password, which is also how an empty one reads.
    pub fn from_pkcs12(
        cert_p12: impl AsRef<Path>,
        password: Option<&secrecy::SecretString>,
    ) -> Result<Self, ConfigError> {
        let path = cert_p12.as_ref().display().to_string();
        let data = std::fs::read(cert_p12.as_ref()).context(IoSnafu { path: path.clone() })?;
        // An unprotected bundle is written with the empty password, so an absent option opens it.
        let password = password.map_or("", |secret| secret.expose_secret());
        // `Strict` rather than the lenient policies: a bundle whose chain cannot be rebuilt is a
        // mistake to report, not one to paper over with a partial identity.
        let keystore = p12_keystore::KeyStore::from_pkcs12(
            &data,
            password,
            p12_keystore::Pkcs12ImportPolicy::Strict,
        )
        .context(Pkcs12Snafu { path: path.clone() })?;
        let (_, chain) = keystore
            .private_key_chain()
            .context(EmptyPkcs12Snafu { path })?;

        // The whole chain, in the leaf-first order `p12-keystore` rebuilds it in (the certificate
        // the key names, then each issuer upward), which is the order rustls sends it in.
        // Re-encoded into the DER shapes the PEM pair produces, so everything past this point sees
        // one identity format.
        let certs = chain
            .certs()
            .iter()
            .map(|cert| CertificateDer::from(cert.as_der().to_vec()))
            .collect();
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(chain.key().as_der().to_vec()));

        Ok(Self { certs, key })
    }
}

/// TLS and mTLS: the client's own identity, the server's CA, and SSL verification behaviour.
#[derive(Debug, Clone, Default)]
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
    /// TLS identity of the client, `None` for no client authentication. Read from
    /// `CertPem`/`KeyPem`, or from `CertP12`/`CertP12Password`, whose files are loaded as the
    /// configuration is read.
    pub identity: Option<Identity>,
    /// The Certificate Authority the server is verified against, `None` for the system CAs. Read
    /// from `CaCertPath`, whose PEM file is loaded as the configuration is read.
    pub ca_cert: Option<CertificateDer<'static>>,
    /// Override the endpoint name during SSL verification. `OverrideTargetName`.
    pub override_target_name: Option<String>,
}

/// The flat string options [`TlsConfig`] is read from, one per field, all optional.
///
/// Every field tolerates the eager typing a `serde` source may apply (a bare `true` arriving as a
/// real boolean), and an empty string means unset, the same as an absent key: a deployment that
/// declares a variable with an empty default must not differ from one that leaves it out.
#[cfg(feature = "serde")]
#[derive(Debug, serde::Deserialize)]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(transform = crate::config::strip_rust_details)
)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawTls {
    #[serde(flatten)]
    identity: RawIdentity,
    /// Path to the Certificate Authority file, in PEM format; empty for the system CAs.
    #[serde(default, deserialize_with = "crate::config_utils::text")]
    ca_cert_path: String,
    /// Accept any server certificate instead of verifying it: `1`, `true`, `yes`, `enable`,
    /// `allow` or `authorize`, and their negatives; empty for false.
    #[serde(default, deserialize_with = "crate::config_utils::text")]
    allow_unsafe_connection: String,
    /// Override the endpoint name during SSL verification; empty for no override.
    #[serde(default, deserialize_with = "crate::config_utils::text")]
    override_target_name: String,
}

/// The shapes the client's identity arrives in, selected untagged by which options the document
/// names: every shape carries every identity option, and what tells them apart is which ones it
/// requires. `Pkcs12` requires `CertP12`, `PemFiles` requires both PEM halves, and `Bare`, which
/// requires none, is last, so a more specific shape always wins over it.
///
/// Carrying the other spelling's options rather than ignoring them is what lets [`Self::load`]
/// reject the two spellings set at once, and what keeps a PEM pair readable next to a `CertP12`
/// that is present but empty.
///
/// Selection is on key presence alone, so a present-but-empty option still selects a shape;
/// [`RawIdentity::load`] is what reads empty as unset, whichever shape matched.
///
/// The shapes say which options exist and which arrive together, not which combinations are legal:
/// half a PEM pair matches `Bare`, so the schema accepts it and [`RawIdentity::load`] refuses it.
/// A relationship between options is enforced once, where the message can name both, rather than
/// encoded a second time in the schema. `dependentRequired` could express this one, but a rule
/// written twice drifts, and the reader is the copy that decides. This paragraph does not reach
/// the schema - schemars keeps only variant docs for a flattened enum - so the part a consumer
/// needs is on `Bare` itself.
#[cfg(feature = "serde")]
#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
enum RawIdentity {
    /// A certificate chain and its key bundled together in one PKCS#12 file.
    #[serde(rename_all = "PascalCase")]
    Pkcs12 {
        /// Path to the PKCS#12 bundle; mutually exclusive with `CertPem`/`KeyPem`.
        #[serde(deserialize_with = "crate::config_utils::text")]
        cert_p12: String,
        /// Password protecting `CertP12`, empty for none; rejected without `CertP12`.
        #[serde(default, deserialize_with = "crate::config_utils::secret_text")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        cert_p12_password: secrecy::SecretString,
        /// Path to the certificate chain file, in PEM format; set together with `KeyPem`.
        #[serde(default, deserialize_with = "crate::config_utils::text")]
        cert_pem: String,
        /// Path to the key file, in PEM format; set together with `CertPem`.
        #[serde(default, deserialize_with = "crate::config_utils::text")]
        key_pem: String,
    },
    /// A certificate chain and its key, each in its own PEM file.
    #[serde(rename_all = "PascalCase")]
    PemFiles {
        /// Path to the certificate chain file, in PEM format; set together with `KeyPem`.
        #[serde(deserialize_with = "crate::config_utils::text")]
        cert_pem: String,
        /// Path to the key file, in PEM format; set together with `CertPem`.
        #[serde(deserialize_with = "crate::config_utils::text")]
        key_pem: String,
        /// Path to the PKCS#12 bundle; mutually exclusive with `CertPem`/`KeyPem`.
        #[serde(default, deserialize_with = "crate::config_utils::text")]
        cert_p12: String,
        /// Password protecting `CertP12`, empty for none; rejected without `CertP12`.
        #[serde(default, deserialize_with = "crate::config_utils::secret_text")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        cert_p12_password: secrecy::SecretString,
    },
    /// Neither identity option set, or an incomplete one: half a PEM pair, or a password naming no
    /// bundle. Each is refused when the configuration is read, naming the options involved; this
    /// alternative admits them so the vocabulary stays one list, and the reader is what decides
    /// which combinations make an identity.
    #[serde(rename_all = "PascalCase")]
    Bare {
        /// Path to the certificate chain file, in PEM format; set together with `KeyPem`.
        #[serde(default, deserialize_with = "crate::config_utils::text")]
        cert_pem: String,
        /// Path to the key file, in PEM format; set together with `CertPem`.
        #[serde(default, deserialize_with = "crate::config_utils::text")]
        key_pem: String,
        /// Path to the PKCS#12 bundle; mutually exclusive with `CertPem`/`KeyPem`.
        #[serde(default, deserialize_with = "crate::config_utils::text")]
        cert_p12: String,
        /// Password protecting `CertP12`, empty for none; rejected without `CertP12`.
        #[serde(default, deserialize_with = "crate::config_utils::secret_text")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        cert_p12_password: secrecy::SecretString,
    },
}

#[cfg(feature = "serde")]
impl RawIdentity {
    /// The four identity options, whatever shape matched, so the rules below are stated once.
    fn options(self) -> (String, String, String, secrecy::SecretString) {
        match self {
            Self::Pkcs12 {
                cert_p12,
                cert_p12_password,
                cert_pem,
                key_pem,
            }
            | Self::PemFiles {
                cert_pem,
                key_pem,
                cert_p12,
                cert_p12_password,
            }
            | Self::Bare {
                cert_pem,
                key_pem,
                cert_p12,
                cert_p12_password,
            } => (cert_pem, key_pem, cert_p12, cert_p12_password),
        }
    }

    /// The loaded identity this shape names, `None` when every identity option is unset. An empty
    /// value is the option left unset, whatever shape the presence of its key selected.
    fn load(self) -> Result<Option<Identity>, ConfigError> {
        let (cert_pem, key_pem, cert_p12, cert_p12_password) = self.options();

        // Both spellings of the identity at once is a contradiction to reject, not a preference to
        // resolve silently.
        snafu::ensure!(
            cert_p12.is_empty() || (cert_pem.is_empty() && key_pem.is_empty()),
            IncompatibleOptionsSnafu {
                msg: String::from(
                    "`CertP12` and `CertPem`/`KeyPem` name the client identity two different \
                     ways; set only one",
                ),
            }
        );
        // A password naming no bundle is a typo somewhere; honouring half of it would hide it.
        snafu::ensure!(
            !cert_p12.is_empty() || cert_p12_password.expose_secret().is_empty(),
            IncompatibleOptionsSnafu {
                msg: String::from("`CertP12Password` is set without `CertP12`"),
            }
        );

        if !cert_p12.is_empty() {
            // The empty password and no password open the same bundle, so the two need no telling
            // apart here.
            return Identity::from_pkcs12(cert_p12, Some(&cert_p12_password)).map(Some);
        }

        match (cert_pem.is_empty(), key_pem.is_empty()) {
            (true, true) => Ok(None),
            (false, false) => Identity::from_pem_files(cert_pem, key_pem).map(Some),
            // Half an identity is silent on a plain-TLS endpoint and only surfaces as a rejected
            // handshake on an mTLS one, so it is caught here, before either path is opened.
            _ => IncompatibleOptionsSnafu {
                msg: String::from("`CertPem` and `KeyPem` must be either both empty or both set"),
            }
            .fail(),
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<RawTls> for TlsConfig {
    type Error = ConfigError;

    fn try_from(raw: RawTls) -> Result<Self, Self::Error> {
        let RawTls {
            identity,
            ca_cert_path,
            allow_unsafe_connection,
            override_target_name,
        } = raw;

        Ok(Self {
            allow_unsafe_connection: crate::config_utils::parse_bool(
                "AllowUnsafeConnection",
                &allow_unsafe_connection,
            )
            .map_err(|msg| IncompatibleOptionsSnafu { msg }.build())?,
            identity: identity.load()?,
            ca_cert: if ca_cert_path.is_empty() {
                None
            } else {
                let pem = std::fs::read_to_string(&ca_cert_path).context(IoSnafu {
                    path: ca_cert_path.clone(),
                })?;
                Some(CertificateDer::from_pem_slice(pem.as_bytes()).context(TlsSnafu {})?)
            },
            override_target_name: if override_target_name.is_empty() {
                None
            } else {
                Some(override_target_name)
            },
        })
    }
}

impl TlsConfig {
    /// The name the server certificate is verified against, resolved against `endpoint`: an
    /// override given as a bare host keeps the endpoint's own scheme and path, and is otherwise a
    /// full URI whose authority and path replace the endpoint's, but never its scheme, since the
    /// connection is still made to the endpoint. Only the name it is verified against changes.
    pub(crate) fn override_target(&self, endpoint: &Uri) -> Result<Option<Uri>, ConfigError> {
        let Some(name) = &self.override_target_name else {
            return Ok(None);
        };

        let authority;
        let path_and_query;

        if let Ok(auth) = name.parse::<http::uri::Authority>() {
            authority = Some(auth);
            path_and_query = endpoint.path_and_query().cloned();
        } else {
            http::uri::Parts {
                authority,
                path_and_query,
                ..
            } = Uri::try_from(name.as_str())
                .context(UriSnafu { uri: name.clone() })?
                .into_parts();
        }

        let mut uri = http::uri::Builder::new();

        if let Some(scheme) = endpoint.scheme() {
            uri = uri.scheme(scheme.clone());
        }
        if let Some(authority) = authority.or_else(|| endpoint.authority().cloned()) {
            uri = uri.authority(authority);
        }
        if let Some(path_and_query) = path_and_query {
            uri = uri.path_and_query(path_and_query);
        }

        Ok(Some(uri.build().context(HttpSnafu { uri: name.clone() })?))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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

    /// A throwaway self-signed certificate, in PEM. A constant rather than generated material:
    /// these tests assert on the file's own bytes, and a constant is those bytes written out.
    const LEAF_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIBTDCB/6ADAgECAhRyMWWAJ+yfjk7RGKDLxakVhcQ0yTAFBgMrZXAwHDEaMBgG
A1UEAwwRYXJtb25pay10ZXN0LWxlYWYwHhcNMjYwODA3MDAxMjE0WhcNMzYwODA0
MDAxMjE0WjAcMRowGAYDVQQDDBFhcm1vbmlrLXRlc3QtbGVhZjAqMAUGAytlcAMh
ANpxqjoKlnD4as5IH/H694WLFqqR+3FFy3Luct6xHos6o1MwUTAdBgNVHQ4EFgQU
wBlGnaZ16gJ3FMNszbUSNAmS100wHwYDVR0jBBgwFoAUwBlGnaZ16gJ3FMNszbUS
NAmS100wDwYDVR0TAQH/BAUwAwEB/zAFBgMrZXADQQAVxIzL4W9Brb44NaC7WbQF
r1lL1Bm4R9X1EGGbfcpdqJZh2r43Te2SgvAEKRagQrJQezmZ+ZKOx519ulCpXa8G
-----END CERTIFICATE-----
";

    /// A second certificate, standing in for the leaf's chain.
    const CHAIN_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIBXTCCAQ+gAwIBAgIUHwDgogJUfux1AArhbpjKtIcEml8wBQYDK2VwMCQxIjAg
BgNVBAMMGWFybW9uaWstdGVzdC1pbnRlcm1lZGlhdGUwHhcNMjYwODA3MDAxMjE0
WhcNMzYwODA0MDAxMjE0WjAkMSIwIAYDVQQDDBlhcm1vbmlrLXRlc3QtaW50ZXJt
ZWRpYXRlMCowBQYDK2VwAyEAXWWWsSuVzwTdkDK/FTiTLtW1x34pvcKyU9PE5o+i
b96jUzBRMB0GA1UdDgQWBBSGsdzF3Wx49uPI1LVCoaH0B3Pb5jAfBgNVHSMEGDAW
gBSGsdzF3Wx49uPI1LVCoaH0B3Pb5jAPBgNVHRMBAf8EBTADAQH/MAUGAytlcANB
AD1iaefMRLn1MblPJgkZ3UoVp6RszYRmGSwrtl7beH+au7s8oCQA9NaM/Acz2sxZ
Y9NMEug15pmfQhxOmtFVgw0=
-----END CERTIFICATE-----
";

    /// The key of [`LEAF_CERT`], PKCS#8.
    const LEAF_KEY: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIAKZ5vS5lxuHsHFDHPJmgDlI5D43nIUJ6Woni24zaHSM
-----END PRIVATE KEY-----
";

    /// A directory of certificate fixtures for one test, removed on drop.
    struct FixtureDir(PathBuf);

    impl FixtureDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("armonik-tls-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create the fixture directory");
            Self(path)
        }

        fn write(&self, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, content).expect("write the fixture file");
            path
        }
    }

    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_identity_is_loaded_with_its_whole_chain_in_file_order() {
        // `rustls` sends the chain as given and expects the leaf first, so the file's own order
        // has to survive loading.
        let dir = FixtureDir::new("chain");
        let cert = dir.write("cert.pem", format!("{LEAF_CERT}{CHAIN_CERT}"));
        let key = dir.write("key.pem", LEAF_KEY);

        let identity = Identity::from_pem_files(&cert, &key).expect("valid PEM files");

        assert_eq!(identity.certs.len(), 2, "both certificates load");
        let leaf = CertificateDer::from_pem_slice(LEAF_CERT.as_bytes()).expect("valid PEM");
        assert_eq!(identity.certs[0], leaf, "the leaf comes first");
    }

    #[test]
    fn a_certificate_path_that_does_not_exist_is_reported_with_the_path() {
        // A typo in the option has to name the file rather than surface later as a TLS failure.
        let error = Identity::from_pem_files("no/such/cert.pem", "no/such/key.pem")
            .expect_err("a missing file must be reported");

        assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
        assert!(
            chain(&error).contains("no/such/cert.pem"),
            "{}",
            chain(&error)
        );
    }

    #[test]
    fn a_missing_key_file_is_reported_with_its_own_path() {
        let dir = FixtureDir::new("missing-key");
        let cert = dir.write("cert.pem", LEAF_CERT);

        let error = Identity::from_pem_files(&cert, dir.0.join("no-key.pem"))
            .expect_err("a missing key file must be reported");

        assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
        assert!(chain(&error).contains("no-key.pem"), "{}", chain(&error));
    }

    #[test]
    fn a_certificate_file_with_no_certificate_is_rejected_with_the_path() {
        // Iterating an empty file yields an empty chain rather than an error, and `rustls` would
        // only refuse it at handshake time.
        let dir = FixtureDir::new("empty");
        let cert = dir.write("cert.pem", "not a certificate\n");
        let key = dir.write("key.pem", LEAF_KEY);

        let error = Identity::from_pem_files(&cert, &key)
            .expect_err("a file with no certificate must be rejected");

        assert!(
            chain(&error).contains("contains no certificate"),
            "{}",
            chain(&error)
        );
        assert!(chain(&error).contains("cert.pem"), "{}", chain(&error));
    }

    #[test]
    fn an_identity_clone_carries_the_same_material() {
        let dir = FixtureDir::new("clone");
        let cert = dir.write("cert.pem", LEAF_CERT);
        let key = dir.write("key.pem", LEAF_KEY);

        let identity = Identity::from_pem_files(&cert, &key).expect("valid PEM files");

        assert_eq!(identity.clone(), identity);
    }

    // --- PKCS#12 ---

    /// A CA-signed identity, generated fresh rather than committed so no fixture can expire: the
    /// CA signs itself, and the leaf is signed by it. `p12-keystore` rebuilds a bundle's chain by
    /// issuer, so only material that really signed the leaf proves the chain survives.
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

    /// A PKCS#12 bundle carrying `key` and `certs`, written with `p12-keystore`'s own writer.
    fn p12_bundle(key: &rcgen::KeyPair, certs: &[&rcgen::Certificate], password: &str) -> Vec<u8> {
        let chain = p12_keystore::PrivateKeyChain::new(
            [1u8].as_slice(),
            p12_keystore::PrivateKey::from_der(&key.serialize_der()).expect("a valid PKCS#8 key"),
            certs.iter().map(|cert| {
                p12_keystore::Certificate::from_der(cert.der().as_ref())
                    .expect("a valid X.509 certificate")
            }),
        );
        let mut keystore = p12_keystore::KeyStore::new();
        keystore.add_entry(
            "identity",
            p12_keystore::KeyStoreEntry::PrivateKeyChain(chain),
        );
        keystore.writer(password).write().expect("write the bundle")
    }

    #[test]
    fn a_p12_bundle_is_read_into_the_same_identity_a_pem_pair_would_be() {
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(["test".to_owned()]).expect("a self-signed cert");
        let dir = FixtureDir::new("p12");
        let path = dir.write("identity.p12", p12_bundle(&signing_key, &[&cert], "s3cr3t"));

        let identity = Identity::from_pkcs12(&path, Some(&secrecy::SecretString::from("s3cr3t")))
            .expect("a valid PKCS#12 bundle");

        assert_eq!(identity.certs.len(), 1, "one certificate in, one out");
        assert_eq!(identity.certs[0].as_ref(), cert.der().as_ref());
        let PrivateKeyDer::Pkcs8(key) = &identity.key else {
            panic!("expected the PKCS#8 variant, since that is what the bundle carried");
        };
        assert_eq!(key.secret_pkcs8_der(), signing_key.serialize_der());
    }

    #[test]
    fn a_p12_bundle_keeps_its_intermediates_leaf_first() {
        // A server that trusts only the root rebuilds its path through the intermediates the
        // client sends; a load that kept only the leaf would fail that handshake.
        let (leaf, leaf_key, ca) = ca_signed_identity();
        let dir = FixtureDir::new("p12-chain");
        let path = dir.write(
            "identity.p12",
            p12_bundle(&leaf_key, &[&leaf, &ca], "s3cr3t"),
        );

        let identity = Identity::from_pkcs12(&path, Some(&secrecy::SecretString::from("s3cr3t")))
            .expect("a valid PKCS#12 bundle");

        let ders: Vec<&[u8]> = identity.certs.iter().map(AsRef::as_ref).collect();
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
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(["test".to_owned()]).expect("a self-signed cert");
        let dir = FixtureDir::new("p12-nopass");
        let path = dir.write("identity.p12", p12_bundle(&signing_key, &[&cert], ""));

        Identity::from_pkcs12(&path, None).expect("an unprotected bundle needs no password");
    }

    #[test]
    fn a_wrong_p12_password_is_reported_with_the_path() {
        // The path, not the password: whoever reads the error must learn which file refused to
        // open, and nothing about what was tried.
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(["test".to_owned()]).expect("a self-signed cert");
        let dir = FixtureDir::new("p12-refused");
        let path = dir.write("identity.p12", p12_bundle(&signing_key, &[&cert], "s3cr3t"));

        let error = Identity::from_pkcs12(&path, Some(&secrecy::SecretString::from("hunter2")))
            .expect_err("the wrong password must be rejected");

        assert!(matches!(error, ConfigError::Pkcs12 { .. }), "{error:?}");
        let rendered = chain(&error);
        assert!(rendered.contains("identity.p12"), "{rendered}");
        assert!(!rendered.contains("hunter2"), "{rendered}");
    }

    #[test]
    fn a_p12_bundle_with_no_identity_is_rejected_as_empty() {
        // A bundle can be perfectly valid PKCS#12 and still carry no private key chain, which is
        // its own story: nothing is malformed, there is just no identity inside.
        let dir = FixtureDir::new("p12-empty");
        let path = dir.write(
            "identity.p12",
            p12_keystore::KeyStore::new()
                .writer("s3cr3t")
                .write()
                .expect("write the bundle"),
        );

        let error = Identity::from_pkcs12(&path, Some(&secrecy::SecretString::from("s3cr3t")))
            .expect_err("an identity-less bundle must be rejected");

        assert!(
            matches!(error, ConfigError::EmptyPkcs12 { .. }),
            "{error:?}"
        );
        assert!(chain(&error).contains("identity.p12"), "{}", chain(&error));
    }

    #[test]
    fn a_file_that_is_not_pkcs12_is_rejected_as_such() {
        let dir = FixtureDir::new("p12-garbage");
        let path = dir.write("identity.p12", "clearly not a pkcs12 bundle");

        let error = Identity::from_pkcs12(&path, None).expect_err("garbage is not a bundle");

        assert!(matches!(error, ConfigError::Pkcs12 { .. }), "{error:?}");
    }

    #[test]
    fn a_p12_path_that_does_not_exist_is_reported_with_the_path() {
        let error = Identity::from_pkcs12("no/such/identity.p12", None)
            .expect_err("a missing file must be reported");

        assert!(matches!(error, ConfigError::Io { .. }), "{error:?}");
        assert!(
            chain(&error).contains("no/such/identity.p12"),
            "{}",
            chain(&error)
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn the_cert_p12_option_loads_the_bundle_it_names() {
        // The whole path a deployment takes, from the option to the loaded chain: `config.rs`
        // covers which combinations are legal, this covers that the legal one reads the file.
        let (leaf, leaf_key, ca) = ca_signed_identity();
        let dir = FixtureDir::new("p12-option");
        let path = dir.write(
            "identity.p12",
            p12_bundle(&leaf_key, &[&leaf, &ca], "s3cr3t"),
        );

        let config: TlsConfig = serde_json::from_value(serde_json::json!({
            "CertP12": path.display().to_string(),
            "CertP12Password": "s3cr3t",
        }))
        .expect("a valid configuration");

        let identity = config.identity.expect("an identity was configured");
        let ders: Vec<&[u8]> = identity.certs.iter().map(AsRef::as_ref).collect();
        assert_eq!(ders, vec![leaf.der().as_ref(), ca.der().as_ref()]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_missing_ca_certificate_is_reported_with_the_path() {
        // The CA is loaded with the rest of the configuration, so a typo in the option names the
        // file instead of surfacing as a failed handshake.
        let error = serde_json::from_value::<TlsConfig>(serde_json::json!({
            "CaCertPath": "no/such/ca.pem",
        }))
        .expect_err("a missing file must be reported");

        assert!(error.to_string().contains("no/such/ca.pem"), "{error}");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_ca_certificate_is_loaded_while_the_configuration_is_read() {
        let dir = FixtureDir::new("ca");
        let ca = dir.write("ca.pem", LEAF_CERT);

        let config = serde_json::from_value::<TlsConfig>(serde_json::json!({
            "CaCertPath": ca.display().to_string(),
        }))
        .expect("a readable CA file");

        let expected = CertificateDer::from_pem_slice(LEAF_CERT.as_bytes()).expect("valid PEM");
        assert_eq!(config.ca_cert, Some(expected));
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

        let override_target = config
            .override_target(
                &Uri::try_from("https://10.0.0.1:5003/base").expect("a valid endpoint"),
            )
            .expect("valid")
            .expect("an override target");

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

        let override_target = config
            .override_target(
                &Uri::try_from("https://10.0.0.1:5003/base").expect("a valid endpoint"),
            )
            .expect("valid")
            .expect("an override target");

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
        let config = TlsConfig::default();

        assert_eq!(config.override_target(&endpoint()).expect("valid"), None);
        assert!(config.ca_cert.is_none());
        assert!(!config.allow_unsafe_connection);
    }
}
