//! Building a [`HttpConfigArgs`] from the environment.
//!
//! Reading the environment at all is still integration work, not transport, so nothing here decides
//! *that* a caller should do it: [`HttpConfigArgs::from_env`] is offered because a variable per
//! option is by far the most common way a deployment supplies one, not because this crate goes looking
//! for one on its own. `connect` never calls it.
//!
//! The prefix is the only thing a caller chooses: `HttpConfigArgs`'s own field names, spelled in
//! `PascalCase` (`#[serde(rename_all = "PascalCase")]`, ArmoniK's own convention, the same for the C#
//! and C++ clients), decide the rest. [`figment::providers::Env`] is the reader: it supports
//! `#[serde(flatten)]`, which a `Deserializer` implementing only `deserialize_struct` cannot.
//!
//! The prefix a caller passes is matched case-insensitively (an incidental property of the `Uncased`
//! comparison this reader's own prefix-stripping happens to use), but each field name after it still
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

use crate::HttpConfigArgs;

impl HttpConfigArgs {
    /// Read every option from the environment, under a prefix of the caller's choosing.
    pub fn from_env(prefix: &str) -> Result<Self, EnvFieldError> {
        Figment::new()
            .merge(Env::prefixed(prefix).lowercase(false))
            .extract()
            .context(ReadSnafu)
    }
}

/// Reading [`HttpConfigArgs`] from the environment failed.
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

    use super::*;
    use crate::{ConfigError, HttpConfig};

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
        let args = with_var(
            "ARMONIK_TEST__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var("ARMONIK_TEST__UserAgent", Some("armonik-test/1"), || {
                    HttpConfigArgs::from_env("ARMONIK_TEST__")
                })
            },
        )
        .expect("reading the environment must not fail");

        assert_eq!(args.user_agent, "armonik-test/1");
        assert_eq!(args.endpoint, "http://localhost:5001");
    }

    #[test]
    #[serial_test::serial]
    fn an_absent_variable_is_empty_rather_than_an_error_endpoint_included() {
        // Matches every other field, and what this crate did before it grew its own environment
        // reading: a missing option is this crate's problem to reject with a named error
        // (`ConfigError::Uri` for an empty endpoint), not this module's to refuse up front.
        let args = HttpConfigArgs::from_env("ARMONIK_TEST_ABSENT__")
            .expect("an absent variable must not fail the read");

        assert_eq!(args.endpoint, "");
    }

    #[test]
    #[serial_test::serial]
    fn every_accepted_boolean_spelling_is_accepted() {
        // The vocabulary is wider than `true`/`false` and there is no other record of it: the list is
        // the specification, so it is written out here rather than sampled. Read through to the
        // resolved configuration, since that is the layer that interprets the spelling.
        fn resolve(spelling: &str) -> Result<HttpConfig, ConfigError> {
            let args = with_var(
                "ARMONIK_TEST_BOOL__AllowUnsafeConnection",
                Some(spelling),
                || {
                    with_var(
                        "ARMONIK_TEST_BOOL__Endpoint",
                        Some("http://localhost:5001"),
                        || HttpConfigArgs::from_env("ARMONIK_TEST_BOOL__"),
                    )
                },
            )
            .expect("a spelling is text, whatever it spells");
            HttpConfig::from_config_args(args)
        }

        for spelling in ["1", "true", "yes", "enable", "allow", "authorize"] {
            let config = resolve(spelling).expect(spelling);
            assert!(
                config.tls.allow_unsafe_connection,
                "`{spelling}` should read as true"
            );
        }

        for spelling in ["0", "false", "no", "disable", "disallow", "forbid", ""] {
            let config = resolve(spelling).expect(spelling);
            assert!(
                !config.tls.allow_unsafe_connection,
                "`{spelling}` should read as false"
            );
        }

        // An unusable spelling is the configuration's to reject, naming the option: reading it is
        // just reading text, and a flattened `serde` source no longer knows which variable it came
        // from by the time a value is interpreted.
        let rendered = resolve("perhaps")
            .expect_err("`perhaps` is not a boolean")
            .to_string();
        assert!(rendered.contains("perhaps"), "{rendered}");
        assert!(
            rendered.contains("allow_unsafe_connection"),
            "the message should name the option: {rendered}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn a_certificate_variable_names_a_path_without_reading_it() {
        // Whether that path leads anywhere is `HttpConfig::from_config_args`'s question to ask, not
        // this one's: nothing here touches a filesystem.
        let args = with_var(
            "ARMONIK_TEST_CERT__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_CERT__CertPem",
                    Some("no/such/cert.pem"),
                    || HttpConfigArgs::from_env("ARMONIK_TEST_CERT__"),
                )
            },
        )
        .expect("reading the variable itself must not fail");

        assert_eq!(args.tls.cert_pem, "no/such/cert.pem");
    }

    #[test]
    #[serial_test::serial]
    fn an_unset_certificate_variable_is_no_certificate_rather_than_an_error() {
        let args = HttpConfigArgs::from_env("ARMONIK_TEST_NOCERT__")
            .expect("an unset variable names no path");

        assert_eq!(args.tls.cert_pem, "");
    }

    #[test]
    #[serial_test::serial]
    fn a_variable_that_looks_like_a_number_is_still_read_as_text() {
        // This reader's own `Env` provider parses a bare `3` into a real integer before `serde` ever
        // sees it. A number, spelled as any other option is: text, is what every field here expects.
        let args = with_var(
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
                            || HttpConfigArgs::from_env("ARMONIK_TEST_NUMERIC__"),
                        )
                    },
                )
            },
        )
        .expect("a value that parses as a number must still be read");

        assert_eq!(args.tcp.keepalive_retries, "3");
        assert_eq!(args.http2.max_header_list_size, "16384");
    }

    #[test]
    #[serial_test::serial]
    fn a_secret_that_looks_like_a_number_is_still_read_as_text() {
        // This reader's own `Env` provider can hand `Secret::deserialize` a real integer rather than a
        // string; `secret_text` has to tolerate that the same way `text` does for a plain `String`.
        let args = with_var(
            "ARMONIK_TEST_NUMERIC_SECRET__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_NUMERIC_SECRET__ProxyPassword",
                    Some("1234"),
                    || HttpConfigArgs::from_env("ARMONIK_TEST_NUMERIC_SECRET__"),
                )
            },
        )
        .expect("a numeric-looking secret must still be read");

        assert_eq!(args.proxy_config.password.expose_secret(), "1234");
    }

    #[test]
    #[serial_test::serial]
    fn a_variable_made_entirely_of_brackets_names_its_own_escape_hatch() {
        // A trailing or leading character defeats the reader's own list grammar (`[::1]:5001` stays
        // text, since the brackets do not span the whole value), so only a value that is *nothing but*
        // a bracketed list hits this at all: a bare `[::1]`, not the usual `[::1]:5001`.
        let rendered = with_var(
            "ARMONIK_TEST_BRACKETS__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_BRACKETS__OverrideTargetName",
                    Some("[::1]"),
                    || HttpConfigArgs::from_env("ARMONIK_TEST_BRACKETS__"),
                )
            },
        )
        .expect_err("a bracketed list is not this option's own text")
        .to_string();

        assert!(
            rendered.contains("double quotes"),
            "the message should name the escape hatch: {rendered}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn wrapping_a_bracketed_value_in_quotes_reads_it_as_text() {
        let args = with_var(
            "ARMONIK_TEST_QUOTED_BRACKETS__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_QUOTED_BRACKETS__OverrideTargetName",
                    Some("\"[::1]\""),
                    || HttpConfigArgs::from_env("ARMONIK_TEST_QUOTED_BRACKETS__"),
                )
            },
        )
        .expect("the escape hatch this reader documents must work");

        assert_eq!(args.tls.override_target_name, "[::1]");
    }

    #[test]
    #[serial_test::serial]
    fn a_certificate_variable_present_but_empty_is_no_certificate_either() {
        // A deployment that declares the variable with an empty default must not be told to open an
        // empty path: a plain `String` has only one absent representation, the empty string, so
        // there is no separate empty-but-present case to collapse into by mistake.
        let args = with_var("ARMONIK_TEST_EMPTYCERT__CertPem", Some(""), || {
            HttpConfigArgs::from_env("ARMONIK_TEST_EMPTYCERT__")
        })
        .expect("an empty variable must not fail the read");

        assert_eq!(args.tls.cert_pem, "");
        let config = HttpConfig::from_config_args(HttpConfigArgs {
            endpoint: String::from("http://localhost:5001"),
            ..args
        })
        .expect("an empty cert_pem must not be treated as half an identity");
        assert!(config.tls.identity.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn the_prefix_matches_regardless_of_its_own_case() {
        // Verified rather than assumed: an incidental property of the `Uncased` comparison this
        // reader's prefix-stripping happens to use, not something the field-name match below shares.
        let args = with_var(
            "armonik_test_case__Endpoint",
            Some("http://localhost:5001"),
            || HttpConfigArgs::from_env("ARMONIK_TEST_CASE__"),
        )
        .expect("the prefix's own case must not matter");

        assert_eq!(args.endpoint, "http://localhost:5001");
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
        let args = with_var(
            "ARMONIK_TEST_CASE_FIELD__endpoint",
            Some("http://localhost:5001"),
            || HttpConfigArgs::from_env("ARMONIK_TEST_CASE_FIELD__"),
        )
        .expect("an unrecognised suffix must not fail the read");

        assert_eq!(args.endpoint, "");
    }

    #[test]
    #[serial_test::serial]
    fn the_empty_prefix_still_reads_pascal_case() {
        let args = with_var("Endpoint", Some("http://localhost:5001"), || {
            HttpConfigArgs::from_env("")
        })
        .expect("reading the environment must not fail");

        assert_eq!(args.endpoint, "http://localhost:5001");
    }
}
