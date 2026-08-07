//! Building an [`HttpConfig`] from the environment.
//!
//! Reading the environment at all is still integration work, not transport, so nothing here decides
//! *that* a caller should do it: [`HttpConfig::from_env`] is offered because a variable per option
//! is by far the most common way a deployment supplies one, not because this crate goes looking for
//! one on its own. `connect` never calls it.
//!
//! The prefix is the only thing a caller chooses: [`HttpConfig`]'s own option names, spelled in
//! `PascalCase` (`#[serde(rename_all = "PascalCase")]`, ArmoniK's own convention, the same for the
//! C# and C++ clients), decide the rest. Reading goes through `figment`, which supports
//! `#[serde(flatten)]` where a `Deserializer` implementing only `deserialize_struct` cannot, but
//! the values come from [`RawEnv`], a provider over [`figment::providers::Env`]'s enumeration
//! that keeps each value as the raw text the variable holds: every value reaches its option
//! byte for byte, brackets and quotes included, and an option that really is a number or a
//! boolean parses the text itself.
//!
//! The prefix a caller passes is matched case-insensitively (an incidental property of the `Uncased`
//! comparison this reader's own prefix-stripping happens to use), but each option name after it still
//! has to be spelled `PascalCase` exactly: `serde_derive`'s generated field matcher is a literal
//! string comparison, and nothing here folds case before it runs. This comparison is uniform across
//! platforms, unlike a lookup keyed by the host's own environment-variable case rules.
//!
//! A value that is not valid Unicode is read anyway, replacing what does not decode with the Unicode
//! replacement character, rather than reported as [`EnvFieldError`]: this reader enumerates every
//! variable through [`std::env::vars_os`] and decodes each one leniently, rather than looking up one
//! variable at a time and treating a decoding failure as this variable's own.

use figment::providers::Env;
use figment::Figment;
use snafu::{ResultExt, Snafu};

use crate::HttpConfig;

impl HttpConfig {
    /// Read every option from the environment, under a prefix of the caller's choosing.
    pub fn from_env(prefix: &str) -> Result<Self, EnvFieldError> {
        Figment::new()
            .merge(RawEnv(Env::prefixed(prefix).lowercase(false)))
            .extract()
            .context(ReadSnafu)
    }
}

/// [`figment::providers::Env`]'s enumeration - prefix stripping, case handling and profile
/// included - with every value kept as the raw text the variable holds.
///
/// `Env`'s own `Provider` impl runs each value through `figment`'s scalar grammar before `serde`
/// sees it: a bare `1.0` becomes a float, whose default rendering is `1`, which silently corrupts
/// a numeric-looking password. Text is what a variable holds, so text is what goes in; an option
/// that really is a number or a boolean parses the text itself.
struct RawEnv(Env);

impl figment::Provider for RawEnv {
    fn metadata(&self) -> figment::Metadata {
        self.0.metadata()
    }

    fn data(
        &self,
    ) -> Result<figment::value::Map<figment::Profile, figment::value::Dict>, figment::Error> {
        let mut dict = figment::value::Dict::new();
        for (key, value) in self.0.iter() {
            // `Value::from(String)` is the string verbatim: no grammar runs. The key goes in
            // whole as well: the option names contain no dots, so `Env`'s dotted-key nesting has
            // nothing to describe here, and an unrecognised name is ignored either way.
            dict.insert(key.as_str().to_owned(), value.into());
        }
        Ok(self.0.profile.collect(dict))
    }
}

/// Reading [`HttpConfig`] from the environment failed.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum EnvFieldError {
    #[snafu(display("{source} [{location}]"))]
    #[non_exhaustive]
    Read {
        #[snafu(source(from(figment::Error, Box::new)))]
        source: Box<figment::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use secrecy::ExposeSecret as _;

    use super::*;

    /// Puts back what the variable held, rather than removing it: these tests run in a process whose
    /// environment may already carry a variable another test needs. On drop, so that a failing test
    /// does not take it away from them either.
    struct Restore {
        name: String,
        previous: Option<OsString>,
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(previous) => std::env::set_var(&self.name, previous),
                None => std::env::remove_var(&self.name),
            }
        }
    }

    fn with_var<T>(name: &str, value: Option<&str>, body: impl FnOnce() -> T) -> T {
        let _restore = Restore {
            name: name.to_owned(),
            previous: std::env::var_os(name),
        };
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
        body()
    }

    #[test]
    #[serial_test::serial]
    fn every_plain_field_is_populated_without_a_line_written_for_it_here() {
        // `user_agent` on purpose: the tests that build a real client share this process and are not
        // serialised against this one, so borrowing a variable any of them depends on, the endpoint
        // above all, would send them somewhere else while this runs.
        let config = with_var(
            "ARMONIK_TEST__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var("ARMONIK_TEST__UserAgent", Some("armonik-test/1"), || {
                    HttpConfig::from_env("ARMONIK_TEST__")
                })
            },
        )
        .expect("reading the environment must not fail");

        assert_eq!(
            config.user_agent,
            Some(hyper::http::HeaderValue::from_static("armonik-test/1"))
        );
        assert_eq!(config.endpoint.to_string(), "http://localhost:5001/");
    }

    #[test]
    #[serial_test::serial]
    fn an_absent_variable_is_the_default_rather_than_an_error_endpoint_included() {
        // Matches every other option: a missing endpoint is `connect`'s problem to reject with a
        // named error, not this module's to refuse up front.
        let config = HttpConfig::from_env("ARMONIK_TEST_ABSENT__")
            .expect("an absent variable must not fail the read");

        assert_eq!(config.endpoint, hyper::Uri::default());
    }

    #[test]
    #[serial_test::serial]
    fn every_accepted_boolean_spelling_is_accepted() {
        // The vocabulary is wider than `true`/`false` and there is no other record of it: the list is
        // the specification, so it is written out here rather than sampled.
        fn read(spelling: &str) -> Result<HttpConfig, EnvFieldError> {
            with_var(
                "ARMONIK_TEST_BOOL__AllowUnsafeConnection",
                Some(spelling),
                || {
                    with_var(
                        "ARMONIK_TEST_BOOL__Endpoint",
                        Some("http://localhost:5001"),
                        || HttpConfig::from_env("ARMONIK_TEST_BOOL__"),
                    )
                },
            )
        }

        for spelling in ["1", "true", "yes", "enable", "allow", "authorize"] {
            let config = read(spelling).expect(spelling);
            assert!(
                config.tls.allow_unsafe_connection,
                "`{spelling}` should read as true"
            );
        }

        for spelling in ["0", "false", "no", "disable", "disallow", "forbid", ""] {
            let config = read(spelling).expect(spelling);
            assert!(
                !config.tls.allow_unsafe_connection,
                "`{spelling}` should read as false"
            );
        }

        // An unusable spelling is rejected naming the option: reading it is just reading text, and
        // a flattened `serde` source no longer knows which variable it came from by the time a
        // value is interpreted, so the option name is written into the message itself.
        let rendered = read("perhaps")
            .expect_err("`perhaps` is not a boolean")
            .to_string();
        assert!(rendered.contains("perhaps"), "{rendered}");
        assert!(
            rendered.contains("AllowUnsafeConnection"),
            "the message should name the option: {rendered}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn a_certificate_variable_that_leads_nowhere_fails_the_read_and_names_it() {
        // The identity loads its files as the configuration is read, so a mistyped path fails
        // here, where the error can name the file, rather than at connect time.
        let rendered = with_var(
            "ARMONIK_TEST_CERT__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_CERT__CertPem",
                    Some("no/such/cert.pem"),
                    || {
                        with_var("ARMONIK_TEST_CERT__KeyPem", Some("no/such/key.pem"), || {
                            HttpConfig::from_env("ARMONIK_TEST_CERT__")
                        })
                    },
                )
            },
        )
        .expect_err("a missing certificate file must fail the read")
        .to_string();

        assert!(rendered.contains("no/such/cert.pem"), "{rendered}");
    }

    #[test]
    #[serial_test::serial]
    fn an_unset_certificate_variable_is_no_certificate_rather_than_an_error() {
        let config =
            HttpConfig::from_env("ARMONIK_TEST_NOCERT__").expect("an unset variable names no path");

        assert_eq!(config.tls.identity, None);
    }

    #[test]
    #[serial_test::serial]
    fn a_variable_that_looks_like_a_number_is_still_read() {
        // The raw reader hands a bare `3` over as text; the option's own reader does the parsing,
        // so a numeric option keeps working without the provider guessing types.
        let config = with_var(
            "ARMONIK_TEST_NUMERIC__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_NUMERIC__TcpKeepaliveRetries",
                    Some("3"),
                    || {
                        with_var(
                            "ARMONIK_TEST_NUMERIC__Http2MaxHeaderListSize",
                            Some("16384"),
                            || HttpConfig::from_env("ARMONIK_TEST_NUMERIC__"),
                        )
                    },
                )
            },
        )
        .expect("a value that parses as a number must still be read");

        assert_eq!(config.tcp.keepalive_retries, Some(3));
        assert_eq!(config.http2.max_header_list_size, Some(16384));
    }

    #[test]
    #[serial_test::serial]
    fn a_secret_that_looks_like_a_number_is_still_read_as_text() {
        // The raw reader hands the password over as text whatever it looks like, and the secret's
        // reader keeps it as written.
        let config = with_var(
            "ARMONIK_TEST_NUMERIC_SECRET__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_NUMERIC_SECRET__ProxyPassword",
                    Some("1234"),
                    || HttpConfig::from_env("ARMONIK_TEST_NUMERIC_SECRET__"),
                )
            },
        )
        .expect("a numeric-looking secret must still be read");

        assert_eq!(config.proxy.password.expose_secret(), "1234");
    }

    #[test]
    #[serial_test::serial]
    fn a_float_looking_password_reaches_the_config_byte_exact() {
        // `figment`'s own `Env` provider parses `1.0` into a float, whose default rendering is
        // `1`: a silently corrupted credential. The raw reader never parses, so the spelling
        // survives byte for byte.
        for written in ["1.0", "2.50"] {
            let config = with_var(
                "ARMONIK_TEST_FLOAT_SECRET__Endpoint",
                Some("http://localhost:5001"),
                || {
                    with_var(
                        "ARMONIK_TEST_FLOAT_SECRET__ProxyPassword",
                        Some(written),
                        || HttpConfig::from_env("ARMONIK_TEST_FLOAT_SECRET__"),
                    )
                },
            )
            .expect(written);

            assert_eq!(config.proxy.password.expose_secret(), written, "{written}");
        }
    }

    #[test]
    #[serial_test::serial]
    fn a_bracketed_value_is_read_verbatim() {
        // `[::1]` is exactly what a target-name override for an IPv6 literal looks like. The raw
        // reader hands the text over as written: no list grammar, and no quote stripping either,
        // so brackets need no escape hatch and quote characters are part of the value.
        for written in ["[::1]", "\"[::1]\""] {
            let config = with_var(
                "ARMONIK_TEST_BRACKETS__Endpoint",
                Some("http://localhost:5001"),
                || {
                    with_var(
                        "ARMONIK_TEST_BRACKETS__OverrideTargetName",
                        Some(written),
                        || HttpConfig::from_env("ARMONIK_TEST_BRACKETS__"),
                    )
                },
            )
            .expect(written);

            assert_eq!(
                config.tls.override_target_name,
                Some(String::from(written)),
                "{written}"
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn a_certificate_variable_present_but_empty_is_no_certificate_either() {
        // A deployment that declares the variable with an empty default must not be told to open an
        // empty path: empty and absent collapse into the same unset identity.
        let config = with_var("ARMONIK_TEST_EMPTYCERT__CertPem", Some(""), || {
            HttpConfig::from_env("ARMONIK_TEST_EMPTYCERT__")
        })
        .expect("an empty variable must not fail the read");

        assert_eq!(
            config.tls.identity, None,
            "an empty cert_pem must not be treated as half an identity"
        );
    }

    #[test]
    #[serial_test::serial]
    fn the_prefix_matches_regardless_of_its_own_case() {
        // Verified rather than assumed: an incidental property of the `Uncased` comparison this
        // reader's prefix-stripping happens to use, not something the field-name match below shares.
        let config = with_var(
            "armonik_test_case__Endpoint",
            Some("http://localhost:5001"),
            || HttpConfig::from_env("ARMONIK_TEST_CASE__"),
        )
        .expect("the prefix's own case must not matter");

        assert_eq!(config.endpoint.to_string(), "http://localhost:5001/");
    }

    #[test]
    #[serial_test::serial]
    fn a_field_name_spelled_in_the_wrong_case_is_left_alone() {
        // Not itself a discriminator against a `.lowercase(true)` reader (a lower-case suffix like
        // this one would be left alone under either setting): its own value is that an unrecognised
        // suffix behaves like any other variable naming no field, rather than an error. Setting both
        // spellings to tell the two settings apart is not reliable across platforms: Windows folds an
        // environment variable's own name by case at the OS level, so `Endpoint` and `endpoint` are
        // the same variable there before this reader ever sees either one.
        let config = with_var(
            "ARMONIK_TEST_CASE_FIELD__endpoint",
            Some("http://localhost:5001"),
            || HttpConfig::from_env("ARMONIK_TEST_CASE_FIELD__"),
        )
        .expect("an unrecognised suffix must not fail the read");

        assert_eq!(config.endpoint, hyper::Uri::default());
    }

    #[test]
    #[serial_test::serial]
    fn the_empty_prefix_still_reads_pascal_case() {
        let config = with_var("Endpoint", Some("http://localhost:5001"), || {
            HttpConfig::from_env("")
        })
        .expect("reading the environment must not fail");

        assert_eq!(config.endpoint.to_string(), "http://localhost:5001/");
    }
}
