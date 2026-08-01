//! Reading configuration from the environment, and the one deliberately-insecure certificate
//! verifier that `allow_unsafe_connection` selects.

use snafu::Snafu;

pub(crate) fn read_env(name: &str) -> Result<String, ReadEnvError> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(String::new()),
        Err(std::env::VarError::NotUnicode(value)) => NotUnicodeSnafu {
            name: name.to_owned(),
            value,
        }
        .fail(),
    }
}

pub(crate) fn read_env_bool(name: &str) -> Result<bool, ReadEnvError> {
    let value = read_env(name)?;
    match value.as_ref() {
        "0" | "false" | "no" | "disable" | "disallow" | "forbid" | "" => Ok(false),
        "1" | "true" | "yes" | "enable" | "allow" | "authorize" => Ok(true),
        _ => NotBooleanSnafu {
            name: name.to_owned(),
            value,
        }
        .fail(),
    }
}

/// Like [`read_env_bool`], but for an option whose default is not `false`.
///
/// Only a variable that is absent takes `unset`. Set but empty stays what it is for every other
/// boolean here, namely `false`: `read_env` maps both to the empty string, so this has to ask the
/// environment itself rather than go through it.
pub(crate) fn read_env_bool_or(name: &str, unset: bool) -> Result<bool, ReadEnvError> {
    match std::env::var(name) {
        Err(std::env::VarError::NotPresent) => Ok(unset),
        _ => read_env_bool(name),
    }
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum ReadEnvError {
    #[snafu(display(
        "Environment variable `{name}={value:?}` is not a valid unicode string [{location}]"
    ))]
    #[non_exhaustive]
    NotUnicode {
        name: String,
        value: std::ffi::OsString,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Environment variable `{name}={value}` is not a valid boolean [{location}]"))]
    #[non_exhaustive]
    NotBoolean {
        name: String,
        value: String,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// A variable name of its own per test, so that a stray value cannot leak between them even though
    /// they are serialised.
    fn with_var<T>(name: &str, value: Option<&str>, body: impl FnOnce() -> T) -> T {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
        let outcome = body();
        std::env::remove_var(name);
        outcome
    }

    #[test]
    #[serial_test::serial]
    fn every_accepted_spelling_is_accepted() {
        // The vocabulary is wider than `true`/`false` and there is no other record of it: the list is the
        // specification, so it is written out here rather than sampled.
        for spelling in ["1", "true", "yes", "enable", "allow", "authorize"] {
            let read = with_var("ARMONIK_TEST_BOOL", Some(spelling), || {
                read_env_bool("ARMONIK_TEST_BOOL")
            });
            assert!(read.expect(spelling), "`{spelling}` should read as true");
        }

        for spelling in ["0", "false", "no", "disable", "disallow", "forbid", ""] {
            let read = with_var("ARMONIK_TEST_BOOL", Some(spelling), || {
                read_env_bool("ARMONIK_TEST_BOOL")
            });
            assert!(!read.expect(spelling), "`{spelling}` should read as false");
        }
    }

    #[test]
    #[serial_test::serial]
    fn an_absent_variable_takes_the_given_default_but_an_empty_one_does_not() {
        // The option that defaults to on has to tell absent from empty, which `read_env_bool` cannot:
        // it maps both to `false`. Empty stays `false` here, so it means the same thing as it does for
        // every other boolean.
        let absent = with_var("ARMONIK_TEST_BOOL_OR", None, || {
            read_env_bool_or("ARMONIK_TEST_BOOL_OR", true)
        });
        assert!(absent.expect("unset"), "absent should take the default");

        let empty = with_var("ARMONIK_TEST_BOOL_OR", Some(""), || {
            read_env_bool_or("ARMONIK_TEST_BOOL_OR", true)
        });
        assert!(!empty.expect("empty"), "set but empty should read as false");

        let explicit = with_var("ARMONIK_TEST_BOOL_OR", Some("true"), || {
            read_env_bool_or("ARMONIK_TEST_BOOL_OR", false)
        });
        assert!(explicit.expect("explicit"), "an explicit value wins");
    }

    #[test]
    #[serial_test::serial]
    fn an_unset_variable_reads_as_false() {
        // Absent and empty are the same thing here, which is what lets every boolean option default to
        // off without the caller having to set it.
        let read = with_var("ARMONIK_TEST_BOOL", None, || {
            read_env_bool("ARMONIK_TEST_BOOL")
        });
        assert!(!read.expect("an unset variable"));
    }

    #[test]
    #[serial_test::serial]
    fn an_unrecognised_value_is_reported_with_its_name_and_value() {
        // The message has to carry both, or the reader is left guessing which of a dozen `GrpcClient__*`
        // variables was the problem.
        let read = with_var("ARMONIK_TEST_BOOL", Some("perhaps"), || {
            read_env_bool("ARMONIK_TEST_BOOL")
        });

        let error = read.expect_err("`perhaps` is not a boolean");
        let rendered = error.to_string();
        assert!(rendered.contains("ARMONIK_TEST_BOOL"), "{rendered}");
        assert!(rendered.contains("perhaps"), "{rendered}");
    }

    #[test]
    #[serial_test::serial]
    fn reading_a_plain_string_passes_it_through_and_maps_absent_to_empty() {
        let value = with_var("ARMONIK_TEST_STRING", Some(" spaced "), || {
            read_env("ARMONIK_TEST_STRING")
        });
        assert_eq!(value.expect("set"), " spaced ", "no trimming, no rewriting");

        let absent = with_var("ARMONIK_TEST_STRING", None, || {
            read_env("ARMONIK_TEST_STRING")
        });
        assert_eq!(absent.expect("unset"), "", "absent reads as empty");
    }
}
