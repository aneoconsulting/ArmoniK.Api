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

/// Everything one embedding of a grouped unit needs: the prefix its options are read under, a
/// reader that names the option a source got wrong, and the unit's schema under the same prefix.
///
/// The prefix is this embedding's to choose, not the unit's to declare: a unit is a plain
/// collection of fields, and another embedding may compose the same one under a prefix of its own.
/// Declaring the three together is what keeps them from drifting apart.
///
/// `$schema` is separate from `$ty` because a unit built through `TryFrom` describes itself to a
/// schema in the shape a document writes, not the shape the program keeps.
///
/// Emits a module rather than free functions because `macro_rules!` cannot build an identifier out
/// of pieces on stable, and takes its types as full paths because names in the body resolve inside
/// that module rather than at the call site.
#[cfg(feature = "serde")]
macro_rules! embed_prefixed {
    ($name:ident, $ty:ty, $schema:ty, $prefix:literal) => {
        mod $name {
            serde_with::with_prefix!(prefix $prefix);

            /// The unit's schema, with this embedding's prefix on every property it declares.
            #[cfg(feature = "schema")]
            pub fn schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                crate::config_utils::schema_with_prefix::<$schema>(generator, $prefix)
            }

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

/// Rewrites a rustdoc intra-doc link into plain text: ``[`a::b::c`]`` becomes ```c```.
///
/// Only the last segment survives: the module path names types that exist on this side of code
/// generation and nowhere else.
#[cfg(feature = "schema")]
fn plain_text(description: &str) -> String {
    let mut out = String::with_capacity(description.len());
    let mut rest = description;
    while let Some(start) = rest.find("[`") {
        let (before, from) = rest.split_at(start);
        out.push_str(before);
        let body = &from[2..];
        // An opening delimiter with no closing one is not a link: the rest is prose, and rewriting
        // it would eat text a reader needs.
        let Some(end) = body.find("`]") else {
            out.push_str(from);
            return out;
        };
        let path = &body[..end];
        out.push('`');
        out.push_str(path.rsplit_once("::").map_or(path, |(_, last)| last));
        out.push('`');
        rest = &body[end + 2..];
    }
    out.push_str(rest);
    out
}

/// Take out of the generated schema what belongs to this crate rather than to the vocabulary:
/// every `default`, and the rustdoc links inside every `description`. Recursive, because
/// `schemars` is free to nest.
///
/// A `default` here is a Rust field's `Default`, serialised in the field's own type rather than in
/// the option's text form: `false` on an option whose schema type is string. Every option's real
/// contract is already stated once, as text: an empty or absent option reads as its default.
///
/// A `description` is the doc comment verbatim, and reaches a generated options class the same
/// way. A Rust path resolves to nothing there, and the brackets around it read as broken markup.
#[cfg(feature = "schema")]
pub(crate) fn strip_rust_details(schema: &mut schemars::Schema) {
    fn clean(object: &mut serde_json::Map<String, serde_json::Value>) {
        object.remove("default");
        if let Some(serde_json::Value::String(description)) = object.get_mut("description") {
            *description = plain_text(description);
        }
    }
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                clean(object);
                for child in object.values_mut() {
                    strip(child);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    strip(item);
                }
            }
            _ => {}
        }
    }
    let object = schema.ensure_object();
    clean(object);
    for child in object.values_mut() {
        strip(child);
    }
}

/// `T`'s own schema with `prefix` glued onto every property, mirroring what
/// [`serde_with::with_prefix!`] does to the names a flattened group is read from: `serde_with` has
/// no `schemars` integration, so without this the schema would describe a field as `Field` where
/// the source spells `PrefixField`.
#[cfg(feature = "schema")]
pub(crate) fn schema_with_prefix<T: schemars::JsonSchema>(
    generator: &mut schemars::SchemaGenerator,
    prefix: &str,
) -> schemars::Schema {
    /// The names live one level down in each branch of a union: a unit whose shapes are selected
    /// untagged describes itself as an `anyOf`, and prefixing only the top level would rename
    /// nothing at all there.
    fn rename(object: &mut serde_json::Map<String, serde_json::Value>, prefix: &str) {
        if let Some(serde_json::Value::Object(properties)) = object.remove("properties") {
            let properties: serde_json::Map<String, serde_json::Value> = properties
                .into_iter()
                .map(|(name, subschema)| (format!("{prefix}{name}"), subschema))
                .collect();
            object.insert(String::from("properties"), properties.into());
        }
        if let Some(serde_json::Value::Array(required)) = object.get_mut("required") {
            for name in required {
                if let serde_json::Value::String(name) = name {
                    *name = format!("{prefix}{name}");
                }
            }
        }
        for keyword in ["anyOf", "oneOf", "allOf"] {
            if let Some(serde_json::Value::Array(branches)) = object.get_mut(keyword) {
                for branch in branches {
                    if let Some(branch) = branch.as_object_mut() {
                        rename(branch, prefix);
                    }
                }
            }
        }
    }

    let mut schema = T::json_schema(generator);
    rename(schema.ensure_object(), prefix);
    schema
}

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

/// [`text`], for an option whose value is a secret: a [`secrecy::SecretString`] is redacted by
/// `Debug` and zeroized on drop, which a plain `String` field is not.
///
/// A numeric-looking secret may arrive as a real number the same way any other option can, and
/// rejecting it would make some values unusable.
#[cfg(feature = "serde")]
pub(crate) fn secret_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<secrecy::SecretString, D::Error> {
    text(deserializer).map(secrecy::SecretString::from)
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

/// Reads a field whose own type states what it accepts - a real number, an integer that cannot be
/// zero - empty for `None`. The message quotes the value and the type's own complaint, like
/// [`bool_option`].
///
/// Preferred to a check made once the whole unit is built: the value is refused while the source's
/// key is still known, so the name comes from the document rather than from a string written next
/// to the check.
#[cfg(feature = "serde")]
pub(crate) fn optional_parsed<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = text(deserializer)?;
    if value.is_empty() {
        return Ok(None);
    }
    match value.parse() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error) => Err(serde::de::Error::custom(format!(
            "`{value}` is not valid: {error}"
        ))),
    }
}

#[cfg(all(test, feature = "schema"))]
mod tests {
    use super::schema_with_prefix;

    /// A unit whose shapes are selected untagged, so its schema is a union of branches and the
    /// names live one level down rather than at the top.
    ///
    /// Never constructed: the derive is what is under test, not the values.
    #[derive(schemars::JsonSchema)]
    #[schemars(untagged)]
    #[allow(dead_code)]
    enum Untagged {
        #[schemars(rename_all = "PascalCase")]
        Both { first: String, second: String },
        #[schemars(rename_all = "PascalCase")]
        One { first: String },
    }

    #[test]
    fn a_prefix_reaches_the_names_inside_a_union_branch() {
        // Renaming the top level alone would rename nothing at all here: both the properties and
        // the keys `required` names sit inside the branches, and a schema that kept the bare names
        // would describe options no source spells.
        let mut generator = schemars::SchemaGenerator::default();
        let schema = schema_with_prefix::<Untagged>(&mut generator, "Prefix");
        let value = serde_json::to_value(&schema).expect("a schema serialises to JSON");

        let branches = value["anyOf"]
            .as_array()
            .unwrap_or_else(|| panic!("an untagged union is an anyOf: {value:#}"));
        assert_eq!(branches.len(), 2, "{value:#}");
        for branch in branches {
            for name in branch["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("each branch is an object shape: {value:#}"))
                .keys()
            {
                assert!(name.starts_with("Prefix"), "`{name}` kept its bare name");
            }
            for name in branch["required"].as_array().into_iter().flatten() {
                let name = name.as_str().expect("a required key is a string");
                assert!(name.starts_with("Prefix"), "`{name}` kept its bare name");
            }
        }
    }
}
