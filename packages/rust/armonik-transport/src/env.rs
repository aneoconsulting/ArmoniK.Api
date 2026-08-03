//! Building a [`ClientConfigArgs`] from the environment.
//!
//! Reading the environment at all is still integration work, not transport, so nothing here decides
//! *that* a caller should do it: [`ClientConfigArgs::from_env`] is offered because a variable per
//! option is by far the most common way a deployment supplies one, not because this crate goes looking
//! for one on its own. `connect` never calls it.
//!
//! The prefix is the only thing a caller chooses: `ClientConfigArgs`'s own field names, spelled in
//! `PascalCase` (`#[serde(rename_all = "PascalCase")]`, ArmoniK's own convention, the same for the C#
//! and C++ clients), decide the rest. [`EnvSource`] is a `serde::Deserializer` that reads
//! `{prefix}{Field}` for each field `serde_derive` asks it for, so the field walk itself is `serde`'s,
//! not a parallel list kept here: adding a field to `ClientConfigArgs` is enough for it to gain
//! environment support.

use std::ffi::OsString;

use serde::de::{Error as _, IntoDeserializer, Visitor};
use serde::Deserialize;
use snafu::Snafu;

use crate::ClientConfigArgs;

impl ClientConfigArgs {
    /// Read every option from the environment, under a prefix of the caller's choosing.
    pub fn from_env(prefix: &str) -> Result<Self, EnvFieldError> {
        Self::deserialize(EnvSource { prefix })
    }
}

/// A `serde::Deserializer` reading a struct's fields from `{prefix}{Field}` environment variables,
/// `Field` being whatever `serde_derive` already renamed the field to.
struct EnvSource<'a> {
    prefix: &'a str,
}

impl<'de> serde::Deserializer<'de> for EnvSource<'_> {
    type Error = EnvFieldError;

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_map(FieldMap {
            prefix: self.prefix,
            fields: fields.iter(),
            current: None,
        })
    }

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(Self::Error::custom(
            "this source only reads a struct, at the top level",
        ))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}

/// Walks a struct's field names, in order, skipping one this source's environment does not carry: a
/// missing variable leaves the field to `#[serde(default)]`, not an error.
struct FieldMap<'a> {
    prefix: &'a str,
    fields: std::slice::Iter<'static, &'static str>,
    current: Option<&'static str>,
}

impl<'de> serde::de::MapAccess<'de> for FieldMap<'_> {
    type Error = EnvFieldError;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        for field in self.fields.by_ref() {
            if std::env::var_os(format!("{}{field}", self.prefix)).is_some() {
                self.current = Some(field);
                return seed.deserialize(field.into_deserializer()).map(Some);
            }
        }
        Ok(None)
    }

    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let field = self
            .current
            .take()
            .expect("next_value_seed called without a preceding next_key_seed");
        let name = format!("{}{field}", self.prefix);
        let value = std::env::var(&name).map_err(|_| match std::env::var_os(&name) {
            Some(value) => NotUnicodeSnafu {
                name: name.clone(),
                value,
            }
            .build(),
            // `next_key_seed` already checked the variable is set; it cannot have vanished since.
            None => unreachable!("checked present in next_key_seed"),
        })?;
        seed.deserialize(EnvValue { name, value })
    }
}

/// One environment variable's value, deserialised into whatever type its field declares.
///
/// Not `String`'s own `IntoDeserializer`: that one expects `true`/`false` verbatim for a `bool`,
/// where an environment variable accepts the wider vocabulary every other ArmoniK client does, and
/// names itself when it fails to parse, the way every other option in this crate already does.
struct EnvValue {
    name: String,
    value: String,
}

impl<'de> serde::Deserializer<'de> for EnvValue {
    type Error = EnvFieldError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value.as_str() {
            "" | "0" | "false" | "no" | "disable" | "disallow" | "forbid" => {
                visitor.visit_bool(false)
            }
            "1" | "true" | "yes" | "enable" | "allow" | "authorize" => visitor.visit_bool(true),
            _ => Err(Self::Error::custom(format!(
                "`{}={}` is not a valid boolean",
                self.name, self.value
            ))),
        }
    }

    serde::forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
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
    #[snafu(display("{message} [{location}]"))]
    #[non_exhaustive]
    Custom {
        message: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

impl serde::de::Error for EnvFieldError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        CustomSnafu {
            message: msg.to_string(),
        }
        .build()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::ClientConfig;

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
                    ClientConfigArgs::from_env("ARMONIK_TEST__")
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
        let args = ClientConfigArgs::from_env("ARMONIK_TEST_ABSENT__")
            .expect("an absent variable must not fail the read");

        assert_eq!(args.endpoint, "");
    }

    #[test]
    #[serial_test::serial]
    fn every_accepted_boolean_spelling_is_accepted() {
        // The vocabulary is wider than `true`/`false` and there is no other record of it: the list is
        // the specification, so it is written out here rather than sampled.
        for spelling in ["1", "true", "yes", "enable", "allow", "authorize"] {
            let args = with_var(
                "ARMONIK_TEST_BOOL__AllowUnsafeConnection",
                Some(spelling),
                || ClientConfigArgs::from_env("ARMONIK_TEST_BOOL__"),
            )
            .expect(spelling);
            assert!(
                args.allow_unsafe_connection,
                "`{spelling}` should read as true"
            );
        }

        for spelling in ["0", "false", "no", "disable", "disallow", "forbid", ""] {
            let args = with_var(
                "ARMONIK_TEST_BOOL__AllowUnsafeConnection",
                Some(spelling),
                || ClientConfigArgs::from_env("ARMONIK_TEST_BOOL__"),
            )
            .expect(spelling);
            assert!(
                !args.allow_unsafe_connection,
                "`{spelling}` should read as false"
            );
        }

        let unrecognised = with_var(
            "ARMONIK_TEST_BOOL__AllowUnsafeConnection",
            Some("perhaps"),
            || ClientConfigArgs::from_env("ARMONIK_TEST_BOOL__"),
        );
        let error = unrecognised.expect_err("`perhaps` is not a boolean");
        let rendered = error.to_string();
        assert!(rendered.contains("perhaps"), "{rendered}");
        assert!(
            rendered.contains("ARMONIK_TEST_BOOL__AllowUnsafeConnection"),
            "the message should name the variable: {rendered}"
        );
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
                    || ClientConfigArgs::from_env("ARMONIK_TEST_CERT__"),
                )
            },
        )
        .expect("reading the variable itself must not fail");

        assert_eq!(args.cert_pem, "no/such/cert.pem");
    }

    #[test]
    #[serial_test::serial]
    fn an_unset_certificate_variable_is_no_certificate_rather_than_an_error() {
        let args = ClientConfigArgs::from_env("ARMONIK_TEST_NOCERT__")
            .expect("an unset variable names no path");

        assert_eq!(args.cert_pem, "");
    }

    #[test]
    #[serial_test::serial]
    fn a_certificate_variable_present_but_empty_is_no_certificate_either() {
        // A deployment that declares the variable with an empty default must not be told to open an
        // empty path: a plain `String` has only one absent representation, the empty string, so
        // there is no separate empty-but-present case to collapse into by mistake.
        let args = with_var("ARMONIK_TEST_EMPTYCERT__CertPem", Some(""), || {
            ClientConfigArgs::from_env("ARMONIK_TEST_EMPTYCERT__")
        })
        .expect("an empty variable must not fail the read");

        assert_eq!(args.cert_pem, "");
        let config = ClientConfig::from_config_args(ClientConfigArgs {
            endpoint: String::from("http://localhost:5001"),
            ..args
        })
        .expect("an empty cert_pem must not be treated as half an identity");
        assert!(config.identity.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn the_empty_prefix_still_reads_pascal_case() {
        let args = with_var("Endpoint", Some("http://localhost:5001"), || {
            ClientConfigArgs::from_env("")
        })
        .expect("reading the environment must not fail");

        assert_eq!(args.endpoint, "http://localhost:5001");
    }
}
