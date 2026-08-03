//! Building a [`ClientConfigArgs`] from the environment.
//!
//! Reading the environment at all is still integration work, not transport, so nothing here decides
//! *that* a caller should do it: [`ClientConfigArgs::from_env`] and [`ClientConfig::from_env`] are
//! offered because a variable per option is by far the most common way a deployment supplies one, not
//! because this crate goes looking for one on its own. `connect` never calls either.
//!
//! One trait, [`FromEnv`], covers everything, the same way `serde`'s `Deserialize` covers structs,
//! enums and scalars alike: what differs between them is what each `impl` does with the [`EnvSource`]
//! it is handed, not a different trait per shape.
//!
//! - On a struct, `#[derive(FromEnv)]` expands to one [`EnvSource::field`] call per named field,
//!   taken straight from the struct's own definition. Adding a field is enough for it to gain
//!   environment support; there is no parallel list here to keep in sync. A field that cannot be
//!   described this way opts out with `#[env(skip)]`, which leaves it at `Default::default()`.
//! - On an enum with one variant marked `#[env(bare)]`, that single-field tuple variant is what a
//!   bare environment string becomes, delegating to its inner type's own `FromEnv`: what lets
//!   [`Certificate`] read as [`Certificate::Path`] without a hand-written impl repeating that rule.
//!   The mark lives on the variant itself, not naming it from the container, so renaming the variant
//!   cannot desynchronise it from what the attribute means.
//! - On an enum with none marked, every variant must carry no data: the environment string selects
//!   one by name, matched case-insensitively against its Rust identifier, the way a plain C-like enum
//!   reads.
//! - On a leaf type (`String`, `bool`, `Secret`, `Option<T>`), `FromEnv` is implemented by hand,
//!   below, since there is only ever one of each to write.
//!
//! What is generic and what is not stays deliberately split. The *shape* of the vocabulary, a variable
//! per field named `{prefix}{Suffix}`, is generic: [`EnvNaming`] spells the suffix in `PascalCase`,
//! `snake_case` or `SCREAMING_SNAKE_CASE`, and the prefix is any string a caller chooses.
//! [`ClientConfigArgs::from_env`]'s default, an empty prefix in `SCREAMING_SNAKE_CASE`, is the
//! ordinary Rust convention (`RUST_LOG`, `CARGO_HOME`), not ArmoniK's: this crate does not know
//! ArmoniK's vocabulary exists. `GrpcClient__` and `PascalCase` are for `armonik`, or a C# host, to
//! configure, the same way they own everything else about what a deployment looks like.

use std::ffi::OsString;

use snafu::{ResultExt, Snafu};

pub use armonik_transport_derive::FromEnv;

use crate::secret::Secret;
use crate::{ClientConfig, ClientConfigArgs, ConfigError};

/// How an option's name is spelled, beyond the prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum EnvNaming {
    /// `cert_pem`, the field name verbatim.
    SnakeCase,
    /// `CERT_PEM`: the ordinary Rust and Unix convention for an environment variable, and what
    /// [`ClientConfigArgs::from_env`] defaults to.
    #[default]
    ScreamingSnakeCase,
    /// `CertPem`: ArmoniK's own convention.
    PascalCase,
}

impl EnvNaming {
    /// Spell `field`, a Rust field name such as `"cert_pem"`, in this convention.
    fn spell(self, field: &str) -> String {
        match self {
            Self::SnakeCase => field.to_owned(),
            Self::ScreamingSnakeCase => field.to_ascii_uppercase(),
            Self::PascalCase => {
                let mut out = String::with_capacity(field.len());
                let mut start_of_word = true;
                for ch in field.chars() {
                    if ch == '_' {
                        start_of_word = true;
                    } else if start_of_word {
                        out.extend(ch.to_uppercase());
                        start_of_word = false;
                    } else {
                        out.push(ch);
                    }
                }
                out
            }
        }
    }
}

/// Something `Self` can be built from: a whole configuration, walking its own fields, or a single
/// value, reading the one variable `source` names.
///
/// Derive it rather than implementing it, unless `Self` is a primitive with only one instance ever
/// worth writing (see the impls below): see the module documentation.
pub trait FromEnv: Sized {
    /// Build `Self` from `source`.
    fn from_env(source: &EnvSource<'_>) -> Result<Self, EnvFieldError>;
}

impl FromEnv for String {
    fn from_env(source: &EnvSource<'_>) -> Result<Self, EnvFieldError> {
        source.read_text().map(|(_, text)| text)
    }
}

impl FromEnv for bool {
    fn from_env(source: &EnvSource<'_>) -> Result<Self, EnvFieldError> {
        let (name, text) = source.read_text()?;
        match text.as_str() {
            "" | "0" | "false" | "no" | "disable" | "disallow" | "forbid" => Ok(false),
            "1" | "true" | "yes" | "enable" | "allow" | "authorize" => Ok(true),
            _ => NotBooleanSnafu { name, value: text }.fail(),
        }
    }
}

impl FromEnv for Secret {
    fn from_env(source: &EnvSource<'_>) -> Result<Self, EnvFieldError> {
        String::from_env(source).map(Secret::from)
    }
}

impl<T: FromEnv> FromEnv for Option<T> {
    fn from_env(source: &EnvSource<'_>) -> Result<Self, EnvFieldError> {
        let (_, value) = source.read();
        match value {
            None => Ok(None),
            Some(_) => T::from_env(source).map(Some),
        }
    }
}

/// Where [`FromEnv`] reads one field, or a whole configuration, from.
///
/// The root, from [`ClientConfigArgs::from_env_with`], names a prefix and a naming convention but no
/// field yet; `#[derive(FromEnv)]`'s generated code calls [`Self::field`] once per member to get a
/// source naming that one variable, which a leaf `FromEnv` impl reads with [`Self::read`].
pub struct EnvSource<'a> {
    prefix: &'a str,
    naming: EnvNaming,
    field: Option<String>,
}

impl<'a> EnvSource<'a> {
    fn new(prefix: &'a str, naming: EnvNaming) -> Self {
        Self {
            prefix,
            naming,
            field: None,
        }
    }

    /// A source naming the field `field`, one level under this one.
    pub fn field(&self, field: &str) -> EnvSource<'a> {
        EnvSource {
            prefix: self.prefix,
            naming: self.naming,
            field: Some(field.to_owned()),
        }
    }

    /// This source's own variable name, and what it currently holds.
    fn read(&self) -> (String, Option<OsString>) {
        let name = format!(
            "{}{}",
            self.prefix,
            self.naming.spell(self.field.as_deref().unwrap_or_default()),
        );
        let value = std::env::var_os(&name);
        (name, value)
    }

    /// This source's own variable name, and what it currently holds as text: `""` when unset.
    ///
    /// `pub(crate)` rather than private: `#[derive(FromEnv)]`'s generated code, for a C-like enum,
    /// calls this from `armonik-transport-derive`'s expansion, which lands in whichever module the
    /// enum itself is defined in, not this one.
    pub(crate) fn read_text(&self) -> Result<(String, String), EnvFieldError> {
        let (name, value) = self.read();
        match value {
            None => Ok((name, String::new())),
            Some(value) => value
                .into_string()
                .map(|text| (name.clone(), text))
                .map_err(|value| NotUnicodeSnafu { name, value }.build()),
        }
    }
}

impl ClientConfigArgs {
    /// Read every option from the environment, the ordinary Rust convention: no prefix,
    /// `SCREAMING_SNAKE_CASE`.
    pub fn from_env() -> Result<Self, EnvConfigError> {
        Self::from_env_with("", EnvNaming::ScreamingSnakeCase)
    }

    /// Read every option from the environment, under a prefix and a naming of the caller's choosing.
    pub fn from_env_with(prefix: &str, naming: EnvNaming) -> Result<Self, EnvConfigError> {
        <Self as FromEnv>::from_env(&EnvSource::new(prefix, naming)).context(FieldSnafu {})
    }
}

impl ClientConfig {
    /// Read every option from the environment and resolve it, the ordinary Rust convention: no
    /// prefix, `SCREAMING_SNAKE_CASE`.
    pub fn from_env() -> Result<Self, EnvConfigError> {
        Self::from_config_args(ClientConfigArgs::from_env()?).context(InvalidSnafu {})
    }

    /// Read every option from the environment and resolve it, under a prefix and a naming of the
    /// caller's choosing.
    pub fn from_env_with(prefix: &str, naming: EnvNaming) -> Result<Self, EnvConfigError> {
        Self::from_config_args(ClientConfigArgs::from_env_with(prefix, naming)?)
            .context(InvalidSnafu {})
    }
}

/// Reading a single environment variable into the type a field declares.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum EnvFieldError {
    #[snafu(display(
        "Environment variable `{name}={value:?}` is not a valid unicode string [{location}]"
    ))]
    #[non_exhaustive]
    NotUnicode {
        name: String,
        value: OsString,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Environment variable `{name}={value}` is not a valid boolean [{location}]"))]
    #[non_exhaustive]
    NotBoolean {
        name: String,
        value: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "Environment variable `{name}={value}` is not one of {allowed:?} [{location}]"
    ))]
    #[non_exhaustive]
    NotAVariant {
        name: String,
        value: String,
        allowed: &'static [&'static str],
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

impl EnvFieldError {
    /// Built rather than constructed through its `snafu` context selector directly:
    /// `#[derive(FromEnv)]`'s generated code for a C-like enum calls this from
    /// `armonik-transport-derive`'s expansion, which cannot spell `NotAVariantSnafu` (that name only
    /// exists as a macro-generated item, invisible to another crate's own macro expansion).
    pub fn not_a_variant(name: String, value: String, allowed: &'static [&'static str]) -> Self {
        NotAVariantSnafu {
            name,
            value,
            allowed,
        }
        .build()
    }
}

/// Turning the environment into a client configuration.
#[derive(Debug, Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum EnvConfigError {
    #[snafu(display("Could not read the environment [{location}]"))]
    #[non_exhaustive]
    Field {
        #[snafu(source(from(EnvFieldError, Box::new)))]
        source: Box<EnvFieldError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "The environment does not describe a valid client configuration [{location}]"
    ))]
    #[non_exhaustive]
    Invalid {
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Certificate;

    /// A plain C-like enum, proving `#[derive(FromEnv)]` handles that shape too, without
    /// `#[env(bare = ..)]`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, crate::env::FromEnv)]
    enum LogLevel {
        Debug,
        Info,
        Warn,
    }

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
    fn pascal_case_and_screaming_snake_case_spell_every_field_it_needs_to() {
        for (field, pascal, screaming) in [
            ("endpoint", "Endpoint", "ENDPOINT"),
            ("cert_pem", "CertPem", "CERT_PEM"),
            (
                "tcp_keepalive_retries",
                "TcpKeepaliveRetries",
                "TCP_KEEPALIVE_RETRIES",
            ),
            (
                "http2_keep_alive_while_idle",
                "Http2KeepAliveWhileIdle",
                "HTTP2_KEEP_ALIVE_WHILE_IDLE",
            ),
            ("reuse_ports", "ReusePorts", "REUSE_PORTS"),
        ] {
            assert_eq!(EnvNaming::PascalCase.spell(field), pascal, "{field}");
            assert_eq!(
                EnvNaming::ScreamingSnakeCase.spell(field),
                screaming,
                "{field}"
            );
            assert_eq!(EnvNaming::SnakeCase.spell(field), field);
        }
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
                    ClientConfigArgs::from_env_with("ARMONIK_TEST__", EnvNaming::PascalCase)
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
        let args = ClientConfigArgs::from_env_with("ARMONIK_TEST_ABSENT__", EnvNaming::PascalCase)
            .expect("an absent variable must not fail the read");

        assert_eq!(args.endpoint, "");
    }

    #[test]
    #[serial_test::serial]
    fn every_accepted_boolean_spelling_is_accepted() {
        // The vocabulary is wider than `true`/`false` and there is no other record of it: the list is
        // the specification, so it is written out here rather than sampled.
        let source = EnvSource::new("", EnvNaming::ScreamingSnakeCase).field("armonik_test_bool");

        for spelling in ["1", "true", "yes", "enable", "allow", "authorize"] {
            let read = with_var("ARMONIK_TEST_BOOL", Some(spelling), || {
                bool::from_env(&source)
            });
            assert!(read.expect(spelling), "`{spelling}` should read as true");
        }

        for spelling in ["0", "false", "no", "disable", "disallow", "forbid", ""] {
            let read = with_var("ARMONIK_TEST_BOOL", Some(spelling), || {
                bool::from_env(&source)
            });
            assert!(!read.expect(spelling), "`{spelling}` should read as false");
        }

        let unrecognised = with_var("ARMONIK_TEST_BOOL", Some("perhaps"), || {
            bool::from_env(&source)
        });
        let error = unrecognised.expect_err("`perhaps` is not a boolean");
        let rendered = error.to_string();
        assert!(rendered.contains("ARMONIK_TEST_BOOL"), "{rendered}");
        assert!(rendered.contains("perhaps"), "{rendered}");
    }

    #[test]
    #[serial_test::serial]
    fn an_unset_optional_boolean_is_none_rather_than_false() {
        let source =
            EnvSource::new("", EnvNaming::ScreamingSnakeCase).field("armonik_test_bool_opt");

        let absent = with_var("ARMONIK_TEST_BOOL_OPT", None, || {
            Option::<bool>::from_env(&source)
        });
        assert_eq!(absent.expect("unset"), None);

        let set = with_var("ARMONIK_TEST_BOOL_OPT", Some("false"), || {
            Option::<bool>::from_env(&source)
        });
        assert_eq!(set.expect("set"), Some(false));
    }

    #[test]
    #[serial_test::serial]
    fn a_c_like_enum_is_selected_by_name_case_insensitively() {
        let source = EnvSource::new("", EnvNaming::ScreamingSnakeCase).field("armonik_test_level");

        for (spelling, expected) in [
            ("Debug", LogLevel::Debug),
            ("debug", LogLevel::Debug),
            ("WARN", LogLevel::Warn),
            ("Info", LogLevel::Info),
        ] {
            let read = with_var("ARMONIK_TEST_LEVEL", Some(spelling), || {
                LogLevel::from_env(&source)
            });
            assert_eq!(read.expect(spelling), expected, "{spelling}");
        }
    }

    #[test]
    #[serial_test::serial]
    fn an_unrecognised_variant_names_the_variable_the_value_and_what_was_allowed() {
        let source = EnvSource::new("", EnvNaming::ScreamingSnakeCase).field("armonik_test_level");

        let error = with_var("ARMONIK_TEST_LEVEL", Some("Verbose"), || {
            LogLevel::from_env(&source)
        })
        .expect_err("`Verbose` is not a `LogLevel`");

        assert!(
            matches!(error, EnvFieldError::NotAVariant { .. }),
            "{error:?}"
        );
        let rendered = error.to_string();
        assert!(rendered.contains("ARMONIK_TEST_LEVEL"), "{rendered}");
        assert!(rendered.contains("Verbose"), "{rendered}");
        assert!(rendered.contains("Debug"), "{rendered}");
    }

    #[test]
    #[serial_test::serial]
    fn a_certificate_variable_names_a_path_without_reading_it() {
        // Whether that path leads anywhere is `ClientConfig::from_config_args`'s question to ask, not
        // this one's: nothing here touches a filesystem.
        let args = with_var(
            "ARMONIK_TEST_CERT__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var(
                    "ARMONIK_TEST_CERT__CertPem",
                    Some("no/such/cert.pem"),
                    || {
                        ClientConfigArgs::from_env_with(
                            "ARMONIK_TEST_CERT__",
                            EnvNaming::PascalCase,
                        )
                    },
                )
            },
        )
        .expect("reading the variable itself must not fail");

        assert_eq!(
            args.cert_pem,
            Some(Certificate::Path(String::from("no/such/cert.pem")))
        );
    }

    #[test]
    #[serial_test::serial]
    fn an_unset_certificate_variable_is_no_certificate_rather_than_an_error() {
        let args = ClientConfigArgs::from_env_with("ARMONIK_TEST_NOCERT__", EnvNaming::PascalCase)
            .expect("an unset variable names no path");

        assert_eq!(args.cert_pem, None);
    }

    #[test]
    #[serial_test::serial]
    fn the_default_reads_the_ordinary_rust_convention() {
        let args = with_var(
            "ENDPOINT",
            Some("http://localhost:5001"),
            ClientConfigArgs::from_env,
        )
        .expect("reading the environment must not fail");

        assert_eq!(args.endpoint, "http://localhost:5001");
    }
}
