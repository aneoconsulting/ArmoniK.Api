//! Reading an [`HttpConfig`] from environment variables, under a prefix the caller chooses.
//!
//! The prefix is the only thing a caller decides: [`HttpConfig`]'s own `PascalCase` option names
//! spell the rest of each variable. Which prefix a deployment sets is that deployment's
//! vocabulary, not this crate's, so no prefix is named here and nothing calls this on its own.
//!
//! Every value reaches `serde` as the raw text the variable holds, and each option's own reader
//! interprets it. So nothing guesses a type on the way in, and a value that happens to look like a
//! number or a list arrives byte for byte.

use serde::de::value::MapDeserializer;
use serde::Deserialize as _;
use snafu::{IntoError as _, Snafu};

use crate::HttpConfig;

impl HttpConfig {
    /// Read every option from the process environment, under `prefix`.
    pub fn from_env(prefix: &str) -> Result<Self, EnvError> {
        Self::from_env_vars(prefix, std::env::vars_os())
    }

    /// [`HttpConfig::from_env`] over any set of variables, for a caller holding an environment it
    /// did not get from its own process: a container specification it is about to submit, or a
    /// test that must leave the one every other test shares alone.
    pub fn from_env_vars(
        prefix: &str,
        variables: impl IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
    ) -> Result<Self, EnvError> {
        // A name is decoded lossily, because the plain `vars` iterator panics on any non-Unicode
        // variable in the process, even one naming no option here, and a mangled name just fails
        // to match the prefix. A value is not: it is the option's content, and a byte replaced by
        // U+FFFD would name a different file or a different password, failing later and elsewhere.
        //
        // The prefix is matched exactly. Windows resolves variable names case-insensitively, so a
        // differently cased spelling reaches the same variable there and no option here; one
        // spelling reads the same on every platform.
        let mut read = Vec::new();
        for (name, value) in variables {
            let name = name.to_string_lossy();
            let Some(option) = name.strip_prefix(prefix) else {
                continue;
            };
            let Some(value) = value.to_str() else {
                return NotUnicodeSnafu {
                    variable: format!("{prefix}{option}"),
                }
                .fail();
            };
            read.push((option.to_owned(), value.to_owned()));
        }
        let options = read.into_iter();

        let mut track = serde_path_to_error::Track::new();
        let tracked =
            serde_path_to_error::Deserializer::new(MapDeserializer::new(options), &mut track);
        Self::deserialize(tracked).map_err(|source| {
            let path = track.path().to_string();
            // An option behind `#[serde(flatten)]` is buffered before its reader sees it, so the
            // tracker here is handed the whole unit and has no key to offer. The unit's own
            // tracker has already written the option name into the message.
            if path.is_empty() || path == "." {
                ReadSnafu.into_error(source)
            } else {
                VariableSnafu {
                    variable: format!("{prefix}{path}"),
                }
                .into_error(source)
            }
        })
    }
}

/// Reading an [`HttpConfig`] from the environment failed.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum EnvError {
    /// A refusal `serde` can attribute to one key, named with the caller's prefix so the message
    /// is the variable to go and fix.
    #[snafu(display("`{variable}`: {source} [{location}]"))]
    #[non_exhaustive]
    Variable {
        variable: String,
        #[snafu(source(from(serde::de::value::Error, Box::new)))]
        source: Box<serde::de::value::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    /// A value the OS accepts and Unicode does not. Refused rather than decoded lossily: a byte
    /// replaced by U+FFFD would name a different file, or authenticate as a different secret.
    #[snafu(display("`{variable}` is not valid Unicode [{location}]"))]
    #[non_exhaustive]
    NotUnicode {
        variable: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    /// A refusal no single key accounts for, whose message names what it is about itself.
    #[snafu(display("{source} [{location}]"))]
    #[non_exhaustive]
    Read {
        #[snafu(source(from(serde::de::value::Error, Box::new)))]
        source: Box<serde::de::value::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::time::Duration;

    use super::*;

    /// The variables a process would hold, as [`HttpConfig::from_env_vars`] receives them.
    fn variables<const N: usize>(pairs: [(&str, &str); N]) -> [(OsString, OsString); N] {
        pairs.map(|(name, value)| (name.into(), value.into()))
    }

    #[test]
    fn a_value_that_is_not_unicode_is_refused_rather_than_rewritten() {
        // A lossy decode would put U+FFFD inside the path, so the option would name a file the
        // deployment never wrote, and the failure would come from somewhere that cannot say why.
        let error = HttpConfig::from_env_vars(
            "GrpcClient__",
            [
                (
                    OsString::from("GrpcClient__Endpoint"),
                    OsString::from("http://localhost:5001"),
                ),
                (OsString::from("GrpcClient__CertPem"), not_unicode()),
            ],
        )
        .expect_err("a non-Unicode value must be refused");

        let rendered = error.to_string();
        assert!(rendered.contains("GrpcClient__CertPem"), "{rendered}");
        assert!(!rendered.contains('\u{FFFD}'), "{rendered}");
    }

    #[test]
    fn the_prefix_is_the_callers_own_and_a_variable_outside_it_names_no_option() {
        // Two deployments reading the same option under prefixes of their own is the whole point
        // of taking one: neither may see the other's variable.
        let config = HttpConfig::from_env_vars(
            "First__",
            variables([
                ("First__Endpoint", "http://first:5001"),
                ("Second__Endpoint", "http://second:5001"),
                ("Endpoint", "http://bare:5001"),
            ]),
        )
        .expect("a valid configuration");

        assert_eq!(config.endpoint.host(), Some("first"));
    }

    #[test]
    fn an_option_that_cannot_be_read_names_the_variable_to_go_and_fix() {
        // The value alone leaves a reader hunting through a deployment for whichever of eight
        // duration options is mistyped; the name has to carry the prefix to be that variable.
        let rendered = HttpConfig::from_env_vars(
            "Prefix__",
            variables([
                ("Prefix__Timeout", "soon"),
                ("Prefix__Endpoint", "http://h"),
            ]),
        )
        .expect_err("`soon` is not a duration")
        .to_string();

        assert!(rendered.contains("`Prefix__Timeout`"), "{rendered}");
        assert!(rendered.contains("soon"), "{rendered}");
    }

    #[test]
    fn a_flattened_option_is_named_by_its_own_unit_and_not_twice() {
        // `#[serde(flatten)]` buffers the unit's keys where a path tracker cannot follow, so this
        // reader has no key and contributes no name. The unit's own tracker supplies it, which
        // costs the prefix: the message names the option rather than the variable.
        let rendered =
            HttpConfig::from_env_vars("Prefix__", variables([("Prefix__TcpKeepalive", "soon")]))
                .expect_err("`soon` is not a duration")
                .to_string();

        assert!(rendered.contains("`TcpKeepalive`"), "{rendered}");
        assert_eq!(
            rendered.matches("TcpKeepalive").count(),
            1,
            "named once, by the unit alone: {rendered}"
        );
        assert!(
            !rendered.contains("Prefix__TcpKeepalive"),
            "the variable's own name is out of reach here: {rendered}"
        );
    }

    #[test]
    fn a_value_reaches_its_option_as_the_text_the_variable_holds() {
        // A source that ran values through a grammar of its own would read `[::1]` as a list and
        // `1.0` as a float, whose default rendering is `1`. Both are values a deployment writes.
        let config = HttpConfig::from_env_vars(
            "Prefix__",
            variables([
                ("Prefix__OverrideTargetName", "[::1]"),
                ("Prefix__UserAgent", "1.0"),
            ]),
        )
        .expect("a valid configuration");

        assert_eq!(config.tls.override_target_name.as_deref(), Some("[::1]"));
        assert_eq!(
            config.user_agent,
            Some(hyper::http::HeaderValue::from_static("1.0"))
        );
    }

    #[test]
    fn an_option_nothing_here_applies_is_read_all_the_same() {
        // `PoolIdleTimeout` is for whoever drives a pool, so the read is the only thing that
        // proves the option exists at all.
        let config =
            HttpConfig::from_env_vars("Prefix__", variables([("Prefix__PoolIdleTimeout", "90s")]))
                .expect("a valid configuration");

        assert_eq!(config.pool_idle_timeout, Some(Duration::from_secs(90)));
    }

    #[test]
    fn a_variable_declared_empty_reads_as_the_option_default_like_an_absent_one() {
        // A deployment that declares every variable with an empty default has to behave exactly
        // like one that declares none.
        let declared = HttpConfig::from_env_vars(
            "Prefix__",
            variables([
                ("Prefix__Endpoint", ""),
                ("Prefix__Timeout", ""),
                ("Prefix__TcpKeepalive", ""),
                ("Prefix__CertPem", ""),
            ]),
        )
        .expect("an empty variable must not fail the read");
        let absent = HttpConfig::from_env_vars("Prefix__", variables([]))
            .expect("an absent variable must not fail the read");

        for config in [&declared, &absent] {
            assert_eq!(config.endpoint, hyper::Uri::default());
            assert_eq!(config.timeout, None);
            assert_eq!(config.tcp.keepalive, None);
            assert_eq!(config.tls.identity, None);
        }
    }

    #[test]
    fn a_variable_that_is_not_unicode_does_not_bring_the_read_down() {
        // `std::env::vars` panics on one of these, wherever in the process it came from, so the
        // read goes through `OsString` and decodes lossily instead.
        let config = HttpConfig::from_env_vars(
            "Prefix__",
            [
                (OsString::from("Prefix__Timeout"), OsString::from("30s")),
                (not_unicode(), OsString::from("whatever")),
            ],
        )
        .expect("a non-Unicode variable names no option and must not fail the read");

        assert_eq!(config.timeout, Some(Duration::from_secs(30)));
    }

    /// A byte sequence the OS accepts and Unicode does not, for a name or a value alike.
    #[cfg(windows)]
    fn not_unicode() -> OsString {
        use std::os::windows::ffi::OsStringExt as _;
        // An unpaired surrogate: representable in a Windows environment block, not in a `str`.
        OsString::from_wide(&[0xD800, 0x0041])
    }

    #[cfg(not(windows))]
    fn not_unicode() -> OsString {
        use std::os::unix::ffi::OsStringExt as _;
        OsString::from_vec(vec![0xFF, 0x41])
    }
}
