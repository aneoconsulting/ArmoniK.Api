//! The one option whose default is not the zero value.

/// Whether the OS may reuse local ports for outgoing connections.
///
/// A `bool` would do, except that this option defaults to **on**, matching ArmoniK's other clients,
/// and a derived `Default` says `false`. Writing `Default` by hand on the structs that hold it would
/// mean listing every other field there, which goes stale the first time someone adds one; the default
/// lives in the type instead, where it cannot be forgotten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ReusePorts(bool);

impl ReusePorts {
    /// Whether the option is on.
    pub fn is_enabled(self) -> bool {
        self.0
    }
}

impl Default for ReusePorts {
    fn default() -> Self {
        Self(true)
    }
}

impl From<bool> for ReusePorts {
    fn from(enabled: bool) -> Self {
        Self(enabled)
    }
}

impl From<ReusePorts> for bool {
    fn from(reuse: ReusePorts) -> Self {
        reuse.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_on() {
        // The point of the type. A `bool` field would default to `false` and disagree with what the
        // option documents.
        assert!(ReusePorts::default().is_enabled());
    }

    #[test]
    fn it_carries_whichever_answer_it_was_given() {
        assert!(ReusePorts::from(true).is_enabled());
        assert!(!ReusePorts::from(false).is_enabled());
        assert!(!bool::from(ReusePorts::from(false)));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn it_serialises_as_the_bare_boolean() {
        // Transparent, so a configuration file keeps writing `true` rather than a wrapper object.
        assert_eq!(
            serde_json::to_string(&ReusePorts::from(false)).expect("serialise"),
            "false"
        );
        assert!(serde_json::from_str::<ReusePorts>("true")
            .expect("deserialise")
            .is_enabled());
    }
}
