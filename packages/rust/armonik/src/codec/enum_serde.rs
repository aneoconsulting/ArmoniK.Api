//! Serde for the proto enumerations: one representation out, three spellings in.
//!
//! A named value serializes as its Rust variant name; the catch-all as a plain integer, because
//! `{"Unknown": 9999}` spells a Rust implementation detail where the number is what every other
//! binding agrees on. Deserializing accepts all three shapes and normalizes through the enum's own
//! `From<i32>`, which is the point: `derive(Deserialize)` is generated in the module that owns the
//! payload's private field, so it builds the catch-all directly and skips that conversion.
//!
//! Reading three shapes means [`Deserializer::deserialize_any`], so this covers the self-describing
//! formats (JSON, YAML, TOML) and not the ones that need the type to drive the parse (bincode,
//! postcard).

use std::fmt;

use serde::de::{Error as _, IgnoredAny, MapAccess, Unexpected, Visitor};
use serde::{Deserializer, Serializer};

/// One entry per named value: the Rust variant's name and the proto number it stands for, in
/// declaration order. Emitted by `#[armonik_macros::enumeration]`, which is the only thing that
/// knows both halves.
pub(crate) type Values = &'static [(&'static str, i32)];

pub(crate) fn serialize<S: Serializer>(
    values: Values,
    value: i32,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match values.iter().find(|(_, number)| *number == value) {
        Some((name, _)) => serializer.serialize_str(name),
        None => serializer.serialize_i32(value),
    }
}

/// The proto value a document names, for the enum to normalize.
///
/// `name` is the Rust type's, and `unknown` its catch-all variant's; both only ever reach the error
/// message and the one map key this accepts.
pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    values: Values,
    name: &'static str,
    unknown: &'static str,
    deserializer: D,
) -> Result<i32, D::Error> {
    deserializer.deserialize_any(Raw {
        values,
        name,
        unknown,
    })
}

struct Raw {
    values: Values,
    name: &'static str,
    unknown: &'static str,
}

impl<'de> Visitor<'de> for Raw {
    type Value = i32;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a {}: one of ", self.name)?;
        for (index, (name, _)) in self.values.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "`{name}`")?;
        }
        write!(
            f,
            ", a proto value as an integer, or {{\"{}\": <integer>}}",
            self.unknown
        )
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        match self.values.iter().find(|(name, _)| *name == value) {
            Some((_, number)) => Ok(*number),
            // Deliberately not the catch-all: a name this crate version does not know is a
            // different mistake from a number it does not know, and only the number round-trips.
            None => Err(E::invalid_value(Unexpected::Str(value), &self)),
        }
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        i32::try_from(value).map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        i32::try_from(value).map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))
    }

    /// The externally-tagged spelling of the catch-all, accepted so a document written by
    /// `derive(Deserialize)` still reads. Normalized like every other numeric path.
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let Some(key) = map.next_key::<String>()? else {
            return Err(A::Error::invalid_length(0, &self));
        };
        if key != self.unknown {
            return Err(A::Error::invalid_value(Unexpected::Str(&key), &self));
        }
        let value = map.next_value::<i32>()?;
        if map.next_key::<IgnoredAny>()?.is_some() {
            return Err(A::Error::invalid_length(2, &self));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::TaskStatus;

    fn json(status: TaskStatus) -> String {
        serde_json::to_string(&status).expect("serialize")
    }

    fn read(text: &str) -> TaskStatus {
        serde_json::from_str(text).expect("deserialize")
    }

    #[test]
    fn a_named_value_is_its_variant_name_and_an_unknown_one_is_a_number() {
        assert_eq!(json(TaskStatus::Completed), "\"Completed\"");
        assert_eq!(json(TaskStatus::UNSPECIFIED), "0");
        assert_eq!(json(TaskStatus::from(9999)), "9999");
    }

    /// The three spellings one value can arrive as, all normalized through `From<i32>`. The last
    /// is what `derive(Deserialize)` writes, and reading it with that derive yields a catch-all
    /// holding 4: unequal to `Completed`, and the same bytes on the wire.
    #[test]
    fn every_spelling_of_a_known_value_reads_as_the_named_variant() {
        assert_eq!(read("\"Completed\""), TaskStatus::Completed);
        assert_eq!(read("4"), TaskStatus::Completed);
        assert_eq!(read("{\"Unknown\":4}"), TaskStatus::Completed);

        assert!(matches!(read("4"), TaskStatus::Completed));
        assert!(matches!(read("{\"Unknown\":4}"), TaskStatus::Completed));
    }

    #[test]
    fn an_unknown_value_round_trips_through_both_spellings() {
        assert_eq!(read("9999"), TaskStatus::from(9999));
        assert_eq!(read("{\"Unknown\":9999}"), TaskStatus::from(9999));
        assert_eq!(read(&json(TaskStatus::from(9999))), TaskStatus::from(9999));
    }

    /// A name this crate version does not know is a different mistake from a number it does not
    /// know: only the number means anything to a peer, so only the number is accepted.
    #[test]
    fn an_unknown_name_is_rejected_and_says_what_it_expected() {
        let error = serde_json::from_str::<TaskStatus>("\"Frobnicated\"").expect_err("rejected");
        let message = error.to_string();
        assert!(message.contains("Frobnicated"), "{message}");
        assert!(message.contains("`Completed`"), "{message}");
        assert!(message.contains("{\"Unknown\": <integer>}"), "{message}");
    }

    #[test]
    fn a_value_outside_i32_is_rejected() {
        serde_json::from_str::<TaskStatus>("2147483648").expect_err("rejected");
        serde_json::from_str::<TaskStatus>("-2147483649").expect_err("rejected");
    }

    /// Through a field of a message, which is how these are actually written.
    #[test]
    fn an_enumeration_field_of_a_message_reads_the_same_way() {
        let raw = crate::sessions::Raw {
            status: crate::SessionStatus::Running,
            ..Default::default()
        };
        let text = serde_json::to_string(&raw).expect("serialize");
        assert!(text.contains("\"status\":\"Running\""), "{text}");
        assert_eq!(
            serde_json::from_str::<crate::sessions::Raw>(&text).expect("deserialize"),
            raw
        );
    }
}
