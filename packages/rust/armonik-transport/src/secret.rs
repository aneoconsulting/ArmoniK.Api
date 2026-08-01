//! A configuration value that must not be printed.

use std::fmt;

/// What a secret renders as instead of its value.
const REDACTED: &str = "[redacted]";

/// A string that redacts itself when printed or serialised.
///
/// The value is reachable only through [`Secret::expose_secret`], named so that reading it is a visible act.
/// There is deliberately no `Deref` or `AsRef`, which would let it out silently; `rustls` guards a
/// private key the same way, with `secret_der` as the only way in.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Secret(String);

impl Secret {
    /// Hold `value`, to be redacted wherever it is written.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The value itself, for the code that has to use it.
    ///
    /// Named in full so that a call site reads as the deliberate act it is, following the convention
    /// the `secrecy` crate set.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Whether no secret was given, which a caller may ask without reading one.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            f.write_str("\"\"")
        } else {
            f.write_str(REDACTED)
        }
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Secret {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Secret {
    /// Redacts, so that a configuration dumped for diagnosis carries no credential.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.is_empty() {
            serializer.serialize_str("")
        } else {
            serializer.serialize_str(REDACTED)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Secret {
    /// Refuses the redaction marker, so that reading back a dump fails where it can be understood
    /// rather than later, as an unexplained rejection by whatever the secret authenticates against.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value == REDACTED {
            return Err(serde::de::Error::custom(format!(
                "`{REDACTED}` is what a secret serialises to, so this input cannot be one"
            )));
        }
        Ok(Self::new(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_shows_the_value() {
        assert_eq!(format!("{:?}", Secret::new("hunter2")), REDACTED);
    }

    #[test]
    fn an_empty_secret_reads_as_empty_rather_than_as_a_redaction() {
        // Otherwise a configuration with no password looks like one whose password cannot be shown.
        assert_eq!(format!("{:?}", Secret::default()), "\"\"");
    }

    #[test]
    fn the_value_is_available_only_to_the_code_that_asks_for_it() {
        assert_eq!(Secret::new("hunter2").expose_secret(), "hunter2");
        // Emptiness is answerable without reading the value.
        assert!(Secret::default().is_empty());
        assert!(!Secret::new("hunter2").is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialisation_redacts() {
        assert_eq!(
            serde_json::to_string(&Secret::new("hunter2")).expect("serialise"),
            format!("\"{REDACTED}\"")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn reading_back_a_redacted_dump_is_refused_rather_than_believed() {
        let error = serde_json::from_str::<Secret>(&format!("\"{REDACTED}\""))
            .expect_err("the marker must not be taken for a secret");

        assert!(
            error.to_string().contains("cannot be one"),
            "the message should say why: {error}"
        );
    }
}
