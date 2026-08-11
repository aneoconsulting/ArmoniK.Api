//! Reading options that arrive as text, and embedding a group of them under a prefix.
//!
//! Everything here exists because a configuration source spells its values as strings: an
//! environment variable has no other shape, and a typed source may still spell a number or a
//! boolean as itself. So every scalar is read through [`text`] and interpreted by the option's own
//! reader, rather than by a `Deserialize` that would reject the spelling outright.
//!
//! Nothing in this module knows what an option means. It names no option, carries no default, and
//! mentions no endpoint, proxy or certificate: the vocabulary belongs to whoever declares the
//! fields. That is what keeps the module liftable into a crate of its own the day a second one
//! needs it.

#[cfg(feature = "serde")]
use std::time::Duration;

/// Everything one embedding of a grouped unit needs: the prefix its options are read under, and a
/// reader that names the option a source got wrong.
///
/// The prefix is this embedding's to choose, not the unit's to declare: a unit is a plain
/// collection of fields, and another embedding may compose the same one under a prefix of its own.
///
/// Emits a module rather than free functions because `macro_rules!` cannot build an identifier out
/// of pieces on stable, and takes `$ty` as a full path because names in the body resolve inside
/// that module rather than at the call site.
#[cfg(feature = "serde")]
macro_rules! embed_prefixed {
    ($name:ident, $ty:ty, $prefix:literal) => {
        mod $name {
            serde_with::with_prefix!(prefix $prefix);

            /// The unit, read under this embedding's prefix, naming the option a source got wrong.
            ///
            /// The tracker wraps the deserializer the prefix module reads through, so the key it
            /// records is the one the source actually spelled, prefix included. That is the only
            /// place the name survives: `#[serde(flatten)]` buffers a value before handing it over,
            /// and by the time a reader interprets it the field it came from is gone.
            ///
            /// A conversion of the whole unit fails once every key has been read and popped, so
            /// the tracker has no key to offer; such a failure speaks about a relationship between
            /// options and names them itself, and its message goes through untouched.
            pub fn deserialize<'de, D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<$ty, D::Error> {
                use serde::de::Error as _;

                let mut track = serde_path_to_error::Track::new();
                let tracked = serde_path_to_error::Deserializer::new(deserializer, &mut track);
                prefix::deserialize(tracked).map_err(|error| {
                    let path = track.path().to_string();
                    if path.is_empty() || path == "." {
                        error
                    } else {
                        D::Error::custom(format!("`{path}`: {error}"))
                    }
                })
            }
        }
    };
}

#[cfg(feature = "serde")]
pub(crate) use embed_prefixed;

/// Reads any option as text, whatever scalar shape a `serde` source gave it.
///
/// Every option is authoritatively text, in the spelling its own doc names. The environment
/// reader hands every value over as a string verbatim, but a typed source is not obliged to: a
/// JSON file can spell a number or boolean as itself, and a plain `String` field would reject
/// those outright, so every scalar shape is accepted and rendered back to text. A list or an
/// object is refused: no option is one.
#[cfg(feature = "serde")]
pub(crate) fn text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    struct AnyScalar;

    impl<'de> serde::de::Visitor<'de> for AnyScalar {
        type Value = String;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a string, or a number or boolean spelling one")
        }

        fn visit_bool<E>(self, value: bool) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E> {
            Ok(value.to_owned())
        }

        fn visit_string<E>(self, value: String) -> Result<String, E> {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E> {
            Ok(value.to_string())
        }

        // A number a typed source hands over as a float renders in Rust's default float
        // formatting, not necessarily in the source's own spelling: acceptable for a real
        // number, and the environment reader never produces one.
        fn visit_f64<E>(self, value: f64) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, _seq: A) -> Result<String, A::Error> {
            Err(serde::de::Error::custom(
                "a list is not a single text value; spell the option as a string",
            ))
        }

        fn visit_map<A: serde::de::MapAccess<'de>>(self, _map: A) -> Result<String, A::Error> {
            Err(serde::de::Error::custom(
                "an object is not a single text value; spell the option as a string",
            ))
        }
    }

    deserializer.deserialize_any(AnyScalar)
}

/// The spellings a boolean option accepts, as an error message shows them.
#[cfg(feature = "serde")]
const BOOLEAN_SPELLINGS: &str = "e.g. `true`, `1`, `yes`, or `false`, `0`, `no`";

/// The boolean `value` spells, `None` for a spelling this vocabulary does not cover.
#[cfg(feature = "serde")]
fn boolean(value: &str) -> Option<bool> {
    match value {
        "" | "0" | "false" | "no" | "disable" | "disallow" | "forbid" => Some(false),
        "1" | "true" | "yes" | "enable" | "allow" | "authorize" => Some(true),
        _ => None,
    }
}

/// Reads a boolean option, `Err` carrying a message that names `option`.
///
/// Used by a conversion that states something about several options at once, where the
/// relationship is what has to be named and no single key identifies it. A per-field reader is
/// [`bool_option`], which gets its name from the source instead.
#[cfg(feature = "serde")]
pub(crate) fn parse_bool(option: &str, value: &str) -> Result<bool, String> {
    boolean(value)
        .ok_or_else(|| format!("`{option}={value}` is not a valid boolean ({BOOLEAN_SPELLINGS})"))
}

/// Reads a boolean field.
///
/// The message quotes the value alone: the option's name is the source's own key, which the
/// embedding's reader prepends, so writing one here would be inventing a second answer to a
/// question already answered.
#[cfg(feature = "serde")]
pub(crate) fn bool_option<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<bool, D::Error> {
    let value = text(deserializer)?;
    boolean(&value).ok_or_else(|| {
        serde::de::Error::custom(format!(
            "`{value}` is not a valid boolean ({BOOLEAN_SPELLINGS})"
        ))
    })
}

/// Reads a duration field, empty for `None`. The message quotes the value alone, like
/// [`bool_option`].
#[cfg(feature = "serde")]
pub(crate) fn optional_duration<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(None);
    }
    match value.parse::<humantime::Duration>() {
        Ok(duration) => Ok(Some(duration.into())),
        Err(error) => Err(serde::de::Error::custom(format!(
            "`{value}` is not a valid duration (e.g. `30s` or `1m`): {error}"
        ))),
    }
}

/// Reads an integer field, empty for `None`. The message quotes the value alone, like
/// [`bool_option`].
#[cfg(feature = "serde")]
pub(crate) fn optional_u32<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(None);
    }
    match value.parse::<u32>() {
        Ok(int) => Ok(Some(int)),
        Err(error) => Err(serde::de::Error::custom(format!(
            "`{value}` is not a valid integer: {error}"
        ))),
    }
}
