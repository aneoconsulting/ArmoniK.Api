//! ArmoniK's own environment vocabulary: `GrpcClient__*`.
//!
//! A deliberately small reader: every `GrpcClient__*` variable is handed to serde as the raw text
//! it holds, and `armonik-transport`'s option readers do the interpreting. Reading the
//! environment is integration work, ArmoniK's own vocabulary, so it lives in this crate rather
//! than in the transport.

use armonik_transport::reexports::serde;
use snafu::ResultExt;

use super::{ConnectionError, HttpConfig};

/// The prefix every `GrpcClient` option is read under.
pub const ARMONIK_PREFIX: &str = "GrpcClient__";

/// Read every option from the `GrpcClient__*` variables.
pub(super) fn config_from_env() -> Result<HttpConfig, NewClientError> {
    config_from(std::env::vars_os())
}

/// [`config_from_env`] over any set of variables, so the prefix rule and the decoding can be
/// exercised without mutating the process environment, which every other test shares.
fn config_from(
    variables: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
) -> Result<HttpConfig, NewClientError> {
    use serde::Deserialize as _;

    // Raw text, verbatim: each option's own reader parses what it needs, so nothing guesses a
    // type here and a numeric-looking password survives byte for byte.
    //
    // A name is decoded lossily, because the plain `vars` iterator panics on any non-Unicode
    // variable in the process, even one naming no option here, and a mangled name just fails to
    // match the prefix. A value is not: it is the option's content, and a byte replaced by U+FFFD
    // would name a different file or a different password, which fails later and somewhere else.
    //
    // The prefix is matched exactly. Windows resolves variable names case-insensitively, so
    // `GRPCCLIENT__ENDPOINT` reaches the same variable there and no option here; the spelling
    // every ArmoniK client documents is the one that is read, on every platform alike.
    let mut options = Vec::new();
    for (name, value) in variables {
        let name = name.to_string_lossy();
        let Some(option) = name.strip_prefix(ARMONIK_PREFIX) else {
            continue;
        };
        let Some(value) = value.to_str() else {
            return NotUnicodeSnafu {
                option: format!("{ARMONIK_PREFIX}{option}"),
            }
            .fail();
        };
        options.push((option.to_owned(), value.to_owned()));
    }
    HttpConfig::deserialize(serde::de::value::MapDeserializer::new(options.into_iter()))
        .context(EnvSnafu)
}

/// Creating a client from the environment.
#[derive(Debug, snafu::Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum NewClientError {
    #[snafu(display("Could not read the client configuration from the environment [{location}]"))]
    #[non_exhaustive]
    Env {
        #[snafu(source(from(serde::de::value::Error, Box::new)))]
        source: Box<serde::de::value::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("`{option}` is not valid Unicode [{location}]"))]
    #[non_exhaustive]
    NotUnicode {
        option: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not connect with that configuration [{location}]"))]
    #[non_exhaustive]
    Connect {
        #[snafu(source(from(ConnectionError, Box::new)))]
        source: Box<ConnectionError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An error and everything it was caused by: the outer message is generic, so an assertion
    /// has to look at the whole chain.
    fn chain(error: &dyn std::error::Error) -> String {
        let mut rendered = vec![error.to_string()];
        let mut current = error.source();
        while let Some(source) = current {
            rendered.push(source.to_string());
            current = source.source();
        }
        rendered.join(" -> ")
    }

    /// The variables a process would hold, as [`config_from`] receives them.
    fn variables<const N: usize>(
        pairs: [(&str, &str); N],
    ) -> impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)> {
        pairs
            .map(|(name, value)| (name.into(), value.into()))
            .into_iter()
    }

    #[test]
    fn an_option_is_read_from_the_variable_that_names_it() {
        let config = config_from(variables([
            ("GrpcClient__Endpoint", "http://localhost:5001"),
            ("GrpcClient__Timeout", "30s"),
        ]))
        .expect("a valid configuration");

        assert_eq!(config.endpoint.host(), Some("localhost"));
        assert_eq!(config.timeout, Some(std::time::Duration::from_secs(30)));
    }

    #[test]
    fn a_variable_without_the_prefix_names_no_option() {
        // The same word without the prefix belongs to something else entirely, and a process is
        // full of them.
        let config = config_from(variables([
            ("GrpcClient__Endpoint", "http://prefixed:5001"),
            ("Endpoint", "http://bare:5001"),
            ("Timeout", "1s"),
        ]))
        .expect("a valid configuration");

        assert_eq!(config.endpoint.host(), Some("prefixed"));
        assert_eq!(config.timeout, None);
    }

    #[test]
    fn a_variable_naming_no_option_is_ignored_rather_than_refused() {
        // `HttpConfig` flattens its units, which rules out denying unknown fields, so a prefixed
        // variable this crate reads no option from passes through silently.
        let config = config_from(variables([
            ("GrpcClient__Endpoint", "http://localhost:5001"),
            ("GrpcClient__NotAnOptionAnyoneReads", "value"),
        ]))
        .expect("an unknown option should not fail the read");

        assert_eq!(config.endpoint.host(), Some("localhost"));
    }

    #[test]
    fn an_option_that_cannot_be_read_reports_the_value_it_was_given() {
        let error = config_from(variables([
            ("GrpcClient__Endpoint", "http://localhost:5001"),
            ("GrpcClient__Timeout", "soon"),
        ]))
        .expect_err("`soon` is not a duration");

        // The value and what was expected of it, but not the variable it came from: a plain
        // `MapDeserializer` keeps no path, so the key is gone by the time a reader rejects the
        // value. Wrapping the read in a path tracker is what puts the name back.
        let rendered = chain(&error);
        assert!(rendered.contains("soon"), "{rendered}");
        assert!(rendered.contains("duration"), "{rendered}");
    }

    #[test]
    fn a_non_unicode_value_is_refused_rather_than_rewritten() {
        // Lossy decoding would put U+FFFD inside a path or a password, so the option would name a
        // different file, or authenticate as a different secret, and fail somewhere that cannot
        // say why.
        let error = config_from(
            [
                (
                    std::ffi::OsString::from("GrpcClient__Endpoint"),
                    std::ffi::OsString::from("http://localhost:5001"),
                ),
                (
                    std::ffi::OsString::from("GrpcClient__CertPem"),
                    lossy_value(),
                ),
            ]
            .into_iter(),
        )
        .expect_err("a non-Unicode value must be refused");

        let rendered = chain(&error);
        assert!(rendered.contains("GrpcClient__CertPem"), "{rendered}");
        assert!(!rendered.contains('\u{FFFD}'), "{rendered}");
    }

    #[test]
    fn a_variable_that_is_not_unicode_does_not_bring_the_read_down() {
        // `std::env::vars` panics on one of these, wherever in the process it came from, so the
        // read goes through `OsString` and decodes lossily instead.
        let odd = lossy_name();
        let config = config_from(
            [
                (
                    std::ffi::OsString::from("GrpcClient__Endpoint"),
                    std::ffi::OsString::from("http://localhost:5001"),
                ),
                (odd, std::ffi::OsString::from("whatever")),
            ]
            .into_iter(),
        )
        .expect("a non-Unicode variable names no option and must not fail the read");

        assert_eq!(config.endpoint.host(), Some("localhost"));
    }

    /// A variable value that is not valid Unicode, built the way each platform allows.
    #[cfg(windows)]
    fn lossy_value() -> std::ffi::OsString {
        use std::os::windows::ffi::OsStringExt as _;
        std::ffi::OsString::from_wide(&[0x0063, 0xD800, 0x002E, 0x0070, 0x0065, 0x006D])
    }

    #[cfg(not(windows))]
    fn lossy_value() -> std::ffi::OsString {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(vec![0x63, 0xFF, 0x2E, 0x70, 0x65, 0x6D])
    }

    /// A variable name that is not valid Unicode, built the way each platform allows.
    #[cfg(windows)]
    fn lossy_name() -> std::ffi::OsString {
        use std::os::windows::ffi::OsStringExt as _;
        // An unpaired surrogate: representable in a Windows environment block, not in a `str`.
        std::ffi::OsString::from_wide(&[0xD800, 0x0041])
    }

    #[cfg(not(windows))]
    fn lossy_name() -> std::ffi::OsString {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(vec![0xFF, 0x41])
    }
}
