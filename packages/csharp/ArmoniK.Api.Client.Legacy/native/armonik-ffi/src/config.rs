//! The C-facing configuration, and turning it into a real [`HttpConfig`].
//!
//! Certificates arrive as PEM bytes here, not paths: `HttpConfig` itself already takes bytes
//! (`identity`, `cacert`), so this is the layer that owns reading a `.p12` or an OS certificate
//! store, before it ever reaches Rust. `ClientConfigArgs`, which only understands paths, is not
//! used at all.
//!
//! Covers endpoint, TLS/mTLS and the two connection timeouts; every other `HttpConfig` field is
//! left at its default.

use std::time::Duration;

use armonik_transport::reexports::rustls::pki_types::pem::PemObject;
use armonik_transport::reexports::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use armonik_transport::HttpConfig;

use crate::buffer::ak_bytes;
use crate::error::{AK_ERR_INVALID_CONFIG, AK_ERR_INVALID_UTF8};

/// The configuration a caller builds and hands to [`crate::client::ak_client_create`].
///
/// Every [`ak_bytes`] field is read once, synchronously, before this struct is converted; none of
/// them need to outlive the call. Empty (null `ptr`, `0` `len`) means "not set", the same
/// convention [`ak_bytes`] itself uses for "nothing to report".
#[repr(C)]
pub struct ak_client_config {
    /// Required: the endpoint URL, e.g. `https://localhost:5001`.
    pub endpoint: ak_bytes,
    /// `0` to verify the server certificate normally, any other value to accept any certificate.
    pub allow_unsafe_connection: u8,
    /// The client's own certificate, PEM-encoded. Matched with `key_pem`; both empty or both set.
    pub cert_pem: ak_bytes,
    /// The client's own private key, PEM-encoded. Matched with `cert_pem`; both empty or both set.
    pub key_pem: ak_bytes,
    /// The Certificate Authority, PEM-encoded.
    pub ca_cert_pem: ak_bytes,
    /// Overrides the name checked during TLS verification. Empty for none.
    pub override_target: ak_bytes,
    /// Timeout for establishing the connection, in milliseconds; `-1` for the 60-second default.
    pub connect_timeout_ms: i64,
    /// Timeout for each request, in milliseconds; `-1` for no timeout.
    pub timeout_ms: i64,
}

/// What went wrong converting an [`ak_client_config`], and the option responsible.
#[derive(Debug)]
pub(crate) struct InvalidConfig {
    option: &'static str,
    message: String,
    code: i32,
}

impl InvalidConfig {
    fn new(option: &'static str, message: impl Into<String>) -> Self {
        Self {
            option,
            message: message.into(),
            code: AK_ERR_INVALID_CONFIG,
        }
    }

    /// Bytes read as a `[u8]` were not valid UTF-8: a distinct code from [`AK_ERR_INVALID_CONFIG`],
    /// since it names a problem with the buffer itself rather than with the option it was meant to
    /// spell.
    fn invalid_utf8(option: &'static str, message: impl Into<String>) -> Self {
        Self {
            option,
            message: message.into(),
            code: AK_ERR_INVALID_UTF8,
        }
    }

    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn into_bytes(self) -> ak_bytes {
        ak_bytes::from_string(format!("`{}` is not valid: {}", self.option, self.message))
    }
}

/// Reads `bytes` as UTF-8, or `None` when it is [`ak_bytes::EMPTY`].
///
/// # Safety
///
/// `bytes.ptr`/`bytes.len`, when not empty, must describe a valid, initialised `[u8]` that lives
/// at least as long as this call: exactly what every `ak_client_config` field promises its caller
/// filled in before the call that reads it.
unsafe fn read_str<'a>(
    option: &'static str,
    bytes: &'a ak_bytes,
) -> Result<Option<&'a str>, InvalidConfig> {
    if bytes.ptr.is_null() || bytes.len == 0 {
        return Ok(None);
    }
    // SAFETY: per this function's own contract, `ptr`/`len` describe a valid initialised slice for
    // the duration of this call.
    let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
    std::str::from_utf8(slice)
        .map(Some)
        .map_err(|error| InvalidConfig::invalid_utf8(option, error.to_string()))
}

/// Builds a real [`HttpConfig`] from the bytes a caller supplied.
///
/// # Safety
///
/// Every [`ak_bytes`] field of `config` must satisfy [`read_str`]'s contract.
pub(crate) unsafe fn build(config: &ak_client_config) -> Result<HttpConfig, InvalidConfig> {
    let mut http = HttpConfig::default();

    // SAFETY: forwarding this function's own contract.
    let endpoint = unsafe { read_str("endpoint", &config.endpoint) }?
        .ok_or_else(|| InvalidConfig::new("endpoint", "required, and was not set"))?;
    http.endpoint = endpoint
        .parse()
        .map_err(|error| InvalidConfig::new("endpoint", format!("{error}")))?;

    http.allow_unsafe_connection = config.allow_unsafe_connection != 0;

    // SAFETY: forwarding this function's own contract.
    let cert_pem = unsafe { read_str("cert_pem", &config.cert_pem) }?;
    // SAFETY: forwarding this function's own contract.
    let key_pem = unsafe { read_str("key_pem", &config.key_pem) }?;
    http.identity = match (cert_pem, key_pem) {
        (None, None) => None,
        (Some(cert_pem), Some(key_pem)) => Some((
            CertificateDer::<'static>::from_pem_slice(cert_pem.as_bytes())
                .map_err(|error| InvalidConfig::new("cert_pem", error.to_string()))?,
            PrivateKeyDer::<'static>::from_pem_slice(key_pem.as_bytes())
                .map_err(|error| InvalidConfig::new("key_pem", error.to_string()))?,
        )),
        _ => {
            return Err(InvalidConfig::new(
                "cert_pem/key_pem",
                "must be either both set or both empty",
            ))
        }
    };

    // SAFETY: forwarding this function's own contract.
    if let Some(ca_cert_pem) = unsafe { read_str("ca_cert_pem", &config.ca_cert_pem) }? {
        http.cacert = Some(
            CertificateDer::<'static>::from_pem_slice(ca_cert_pem.as_bytes())
                .map_err(|error| InvalidConfig::new("ca_cert_pem", error.to_string()))?,
        );
    }

    // SAFETY: forwarding this function's own contract.
    if let Some(override_target) = unsafe { read_str("override_target", &config.override_target) }?
    {
        http.override_target = Some(build_override_target(&http.endpoint, override_target)?);
    }

    http.connect_timeout = match config.connect_timeout_ms {
        -1 => Some(Duration::from_secs(60)),
        millis @ 0.. => Some(Duration::from_millis(millis as u64)),
        _ => {
            return Err(InvalidConfig::new(
                "connect_timeout_ms",
                "only -1 (the default) or a value of 0 or more is valid",
            ))
        }
    };
    http.timeout = match config.timeout_ms {
        -1 => None,
        millis @ 0.. => Some(Duration::from_millis(millis as u64)),
        _ => {
            return Err(InvalidConfig::new(
                "timeout_ms",
                "only -1 (no timeout) or a value of 0 or more is valid",
            ))
        }
    };

    Ok(http)
}

/// Builds the URI [`build`] resolves `override_target` to: `endpoint`'s own scheme, and either the
/// name a bare host names or, if it does not parse as one, whatever a full URI's own authority and
/// path name instead.
///
/// A bare host is what this option's own doc promises ("the name checked during TLS verification"),
/// but `hyper::Uri` alone parses one into a scheme-less, path-less URI - which `connect`'s override
/// then silently fails every call over, since a tonic channel's origin requires a scheme. This
/// mirrors `armonik_transport::HttpConfig::from_config_args`'s own handling of the equivalent
/// string-form option for exactly that reason.
fn build_override_target(
    endpoint: &armonik_transport::reexports::hyper::Uri,
    override_target: &str,
) -> Result<armonik_transport::reexports::hyper::Uri, InvalidConfig> {
    use armonik_transport::reexports::hyper::http::uri::{Authority, Builder, Parts};
    use armonik_transport::reexports::hyper::Uri;

    let (authority, path_and_query) = match override_target.parse::<Authority>() {
        Ok(authority) => (Some(authority), endpoint.path_and_query().cloned()),
        Err(_) => {
            let Parts {
                authority,
                path_and_query,
                ..
            } = override_target
                .parse::<Uri>()
                .map_err(|error| InvalidConfig::new("override_target", format!("{error}")))?
                .into_parts();
            (authority, path_and_query)
        }
    };

    let mut uri = Builder::new();
    if let Some(scheme) = endpoint.scheme() {
        uri = uri.scheme(scheme.clone());
    }
    if let Some(authority) = authority.or_else(|| endpoint.authority().cloned()) {
        uri = uri.authority(authority);
    }
    if let Some(path_and_query) = path_and_query {
        uri = uri.path_and_query(path_and_query);
    }

    uri.build()
        .map_err(|error| InvalidConfig::new("override_target", format!("{error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes_of(text: &str) -> ak_bytes {
        ak_bytes {
            ptr: text.as_ptr(),
            len: text.len(),
        }
    }

    fn config_with_override_target(override_target: &str) -> ak_client_config {
        ak_client_config {
            endpoint: bytes_of("https://localhost:5001/base"),
            allow_unsafe_connection: 0,
            cert_pem: ak_bytes::EMPTY,
            key_pem: ak_bytes::EMPTY,
            ca_cert_pem: ak_bytes::EMPTY,
            override_target: bytes_of(override_target),
            connect_timeout_ms: -1,
            timeout_ms: -1,
        }
    }

    #[test]
    fn a_bare_host_keeps_the_endpoints_scheme_and_path() {
        let config = config_with_override_target("server.example.com");

        // SAFETY: every field above is a `&'static str`'s bytes, valid for the whole test.
        let http = unsafe { build(&config) }.expect("a valid configuration");
        let target = http.override_target.expect("an override target");

        assert_eq!(target.scheme_str(), Some("https"));
        assert_eq!(
            target.authority().map(|a| a.as_str()),
            Some("server.example.com")
        );
        assert_eq!(target.path(), "/base");
    }

    #[test]
    fn a_full_uri_replaces_the_authority_and_the_path_but_not_the_endpoints_scheme() {
        // A different scheme than the endpoint's "https", so this only passes if the endpoint's own
        // scheme really does win rather than whatever this URI names.
        let config = config_with_override_target("http://server.example.com/other");

        // SAFETY: every field above is a `&'static str`'s bytes, valid for the whole test.
        let http = unsafe { build(&config) }.expect("a valid configuration");
        let target = http.override_target.expect("an override target");

        assert_eq!(target.scheme_str(), Some("https"));
        assert_eq!(
            target.authority().map(|a| a.as_str()),
            Some("server.example.com")
        );
        assert_eq!(target.path(), "/other");
    }
}
