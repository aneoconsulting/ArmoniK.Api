//! Registry of every message type for the differential harness: the derives register each
//! descriptor-validated type here (hand-written impls register themselves), so the harness
//! discovers the proto-to-type mapping instead of maintaining it. Test-only: the `_differential`
//! feature is enabled through the self dev-dependency.
//!
//! Each entry also carries the type's [`Normalize`] projection: the value-level equivalence classes
//! its Rust representation defines (map adapters losing order and duplicates, presence-only
//! markers, transparent wrapper chains, the cross-field rules of the hand-written impls). The same
//! constructs that cause a projection declare it (adapters, attributes, hand-written impls), so the
//! harness never restates them. What it checks independently is that every field stays
//! information-bearing through the quotient (the field-information ratchet) and that round-trips
//! are lossless up to it.

use std::collections::BTreeMap;

pub use prost_reflect;

use prost_reflect::{DynamicMessage, ReflectMessage, Value};

/// A registered type's round-trip and projection hooks, as the harness consumes them. Projected
/// from the `_differential`-gated [`crate::wire::Diff`] on each [`crate::wire::Registration`] (see
/// [`entries`]); the `default_encoding` doubles as the zero-default invariant and the harness's
/// canonical-absence fold.
#[derive(Clone, Copy)]
pub struct Entry {
    pub proto: &'static str,
    pub roundtrip: fn(&[u8]) -> Result<Vec<u8>, prost::DecodeError>,
    pub default_encoding: fn() -> Vec<u8>,
    pub normalize: fn(&mut DynamicMessage),
}

/// Every registered type carrying harness hooks, so all but the type-less entries, projected from
/// the single [`crate::wire::REGISTRY`].
pub fn entries() -> impl Iterator<Item = Entry> {
    crate::wire::REGISTRY.iter().filter_map(|registration| {
        registration.diff.as_ref().map(|diff| Entry {
            proto: registration.proto,
            roundtrip: diff.roundtrip,
            default_encoding: diff.default_encoding,
            normalize: diff.normalize,
        })
    })
}

/// Projection of a dynamic message onto the equivalence classes this type's wire implementation
/// defines: two messages are the same value exactly when their projections match (up to proto3
/// presence semantics and the canonical-absence fold, which the harness owns).
///
/// Derived types get an implementation generated from the same attributes that shape their codec;
/// hand-written `prost::Message` impls write it by hand next to the codec. Registration requires
/// it, so a hand-written type cannot forget its projection.
pub trait Normalize {
    fn normalize(message: &mut DynamicMessage);
}

/// `#[armonik(present)]` bool member: only its presence survives (an explicit `false` reads as
/// set).
pub fn bool_marker(message: &mut DynamicMessage, tag: u32) {
    let Some(field) = message.descriptor().get_field(tag) else {
        return;
    };
    if message.has_field(&field) {
        message.set_field(&field, Value::Bool(true));
    }
}

/// Transparent enum wrapper (chain) message: when the chained enum value is zero, every
/// representation (absent members, explicit zeros, empty inner wrappers) is equivalent to the empty
/// message.
pub fn wrapper_chain(message: &mut DynamicMessage) {
    let mut number = 0;
    let mut cursor = Value::Message(message.clone());
    loop {
        match cursor {
            Value::Message(wrapper) => {
                let Some(inner) = wrapper.descriptor().fields().next() else {
                    break;
                };
                cursor = wrapper.get_field(&inner).into_owned();
            }
            Value::EnumNumber(value) => {
                number = value;
                break;
            }
            _ => break,
        }
    }
    if number == 0 {
        let fields: Vec<_> = message.descriptor().fields().collect();
        for member in fields {
            message.clear_field(&member);
        }
    }
}

/// Fold a repeated message field exposed as a `HashMap` keyed by the pair subfield with number
/// `key_tag`: duplicates collapse (last wins) and order is lost, so entries are sorted by key.
pub fn fold_pairs_by_tag(message: &mut DynamicMessage, tag: u32, key_tag: u32) {
    fold_pairs(message, tag, |pair| pair.descriptor().get_field(key_tag));
}

/// [`fold_pairs_by_tag`], with the key subfield located by name (for adapters keyed on a field of
/// the entries' own message type).
pub fn fold_pairs_by_name(message: &mut DynamicMessage, tag: u32, key_name: &str) {
    fold_pairs(message, tag, |pair| {
        pair.descriptor().get_field_by_name(key_name)
    });
}

/// Total order over the pair-key values the adapters accept (`Eq + Hash` scalars); the order itself
/// is arbitrary, it only has to be deterministic on both sides of a round-trip.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum MapKey {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Str(String),
}

fn fold_pairs(
    message: &mut DynamicMessage,
    tag: u32,
    key_field: impl Fn(&DynamicMessage) -> Option<prost_reflect::FieldDescriptor>,
) {
    let Some(field) = message.descriptor().get_field(tag) else {
        return;
    };
    if !message.has_field(&field) {
        return;
    }
    let Value::List(entries) = message.get_field(&field).into_owned() else {
        return;
    };
    let mut by_key = BTreeMap::new();
    for entry in entries {
        let Value::Message(pair) = &entry else {
            continue;
        };
        let Some(key_desc) = key_field(pair) else {
            continue;
        };
        let key = match pair.get_field(&key_desc).as_ref() {
            Value::Bool(value) => MapKey::Bool(*value),
            Value::I32(value) => MapKey::Int(i64::from(*value)),
            Value::I64(value) => MapKey::Int(*value),
            Value::EnumNumber(value) => MapKey::Int(i64::from(*value)),
            Value::U32(value) => MapKey::Uint(u64::from(*value)),
            Value::U64(value) => MapKey::Uint(*value),
            Value::String(value) => MapKey::Str(value.clone()),
            other => MapKey::Str(format!("{other:?}")),
        };
        by_key.insert(key, entry);
    }
    message.set_field(&field, Value::List(by_key.into_values().collect()));
}
