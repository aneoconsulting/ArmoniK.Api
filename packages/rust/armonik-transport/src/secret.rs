//! A configuration value that must not be printed.

use std::fmt;
use std::ops::Deref;

/// The text a redacted secret renders as.
const REDACTED: &str = "[redacted]";

/// A string that redacts itself when printed or serialised.
///
/// Redacted by construction rather than by a hand-written `Debug` on each holder: a struct grows
/// fields, and a `Debug` listing them by hand goes stale the first time someone forgets one.
///
/// Serialising redacts as well, which is what output that might be logged needs. [`Secret::revealed`]
/// opts out for one serialisation, and only for that one.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Secret(String);

impl Secret {
    /// Hold `value`, to be redacted wherever it is written.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The value itself, for the code that has to use it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Borrow this secret for a single serialisation in clear.
    ///
    /// The borrow is what makes it safe: revealing is a property of one call site, not of the value,
    /// so it cannot be carried along by a clone or outlive the expression that asked for it.
    ///
    /// For output that is itself protected and has to be read back. `Debug` still redacts, because a
    /// log is never that output.
    pub fn revealed(&self) -> Revealed<'_> {
        Revealed(self)
    }
}

/// A [`Secret`] borrowed for one serialisation in clear. See [`Secret::revealed`].
///
/// Deliberately not `Copy` or `Clone`: serialising takes it by reference, so nothing needs to
/// duplicate it, and a handle that cannot be passed around keeps revealing where it was asked for.
pub struct Revealed<'a>(&'a Secret);

// Redacted here too: this type exists to widen serialisation, not printing.
impl fmt::Debug for Revealed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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

impl Deref for Secret {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
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
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.is_empty() {
            serializer.serialize_str("")
        } else {
            serializer.serialize_str(REDACTED)
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Revealed<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0 .0)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Secret {
    /// Refuses the redaction marker, so that reading back a redacted dump fails where it can be
    /// understood rather than later, as an unexplained rejection by whatever the secret authenticates
    /// against.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value == REDACTED {
            return Err(serde::de::Error::custom(format!(
                "`{REDACTED}` is what a secret serialises to unless `Secret::revealed` was used; \
                 this input cannot be a secret"
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
        // Including through the wrapper that widens serialisation.
        assert_eq!(format!("{:?}", Secret::new("hunter2").revealed()), REDACTED);
    }

    #[test]
    fn an_empty_secret_reads_as_empty_rather_than_as_a_redaction() {
        // Otherwise a configuration with no password looks like one whose password cannot be shown.
        assert_eq!(format!("{:?}", Secret::default()), "\"\"");
    }

    #[test]
    fn the_value_is_available_to_the_code_that_needs_it() {
        assert_eq!(Secret::new("hunter2").expose(), "hunter2");
        assert_eq!(&*Secret::new("hunter2"), "hunter2");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialisation_redacts_unless_the_call_site_asks_otherwise() {
        let secret = Secret::new("hunter2");

        assert_eq!(
            serde_json::to_string(&secret).expect("serialise"),
            format!("\"{REDACTED}\"")
        );
        assert_eq!(
            serde_json::to_string(&secret.revealed()).expect("serialise"),
            "\"hunter2\""
        );
        // Revealing one call site leaves the secret itself untouched, which is the whole point.
        assert_eq!(
            serde_json::to_string(&secret).expect("serialise"),
            format!("\"{REDACTED}\"")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn reading_back_a_redacted_dump_is_refused_rather_than_believed() {
        let error = serde_json::from_str::<Secret>(&format!("\"{REDACTED}\""))
            .expect_err("the marker must not be taken for a secret");

        assert!(
            error.to_string().contains("revealed"),
            "the message should say what to do: {error}"
        );
    }
}
