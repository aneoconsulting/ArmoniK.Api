//! The differential harness and the registry it discovers types through.
//!
//! Randomized `DynamicMessage`s generated from the real protobuf descriptors are round-tripped
//! through the armonik types and compared semantically; two ratchets keep the quotient honest. The
//! derives register each descriptor-validated type into [`registrations::REGISTRY`] (hand-written
//! impls register themselves), so the harness discovers the proto-to-type mapping instead of
//! maintaining it.
//!
//! Each registration also carries the type's [`Normalize`] projection: the value-level equivalence
//! classes its Rust representation defines (map adapters losing order and duplicates, presence-only
//! markers, transparent wrapper chains, the cross-field rules of the hand-written impls). The same
//! constructs that cause a projection declare it (adapters, attributes, hand-written impls), so the
//! harness never restates them. What it checks independently is that every field stays
//! information-bearing through the quotient (the field-information ratchet) and that round-trips
//! are lossless up to it.
//!
//! # Why the whole module is `#[cfg(test)]`
//!
//! It used to be a `_differential` feature turned on through a dev-dependency of the crate on
//! itself, which meant every integration suite linked a lib built with the harness compiled in:
//! the tested artifact was not the shipped one, and the shipped configuration was built by CI but
//! never tested. It also forced `Normalize`, the registry and its hooks to be `pub`. Under
//! `cfg(test)` the harness is a unit-test module like any other, `linkme` and `prost-reflect` are
//! plain dev-dependencies, and none of it is public.

use std::collections::BTreeMap;

use prost_reflect::{DynamicMessage, ReflectMessage, Value};

mod arbitrary;
mod compare;
mod harness;
mod probe;
pub(crate) mod registrations;
mod registry;
mod rng;

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
///
/// One caller, `PairMap`, whose key is a subfield of a pair message the proto declares for exactly
/// that purpose. There used to be a second, locating the key by name, for a repeated field re-keyed
/// on a field of its own element type: that shape lost entries whenever the field was not unique,
/// and `results::import::Response` no longer takes it.
pub fn fold_pairs_by_tag(message: &mut DynamicMessage, tag: u32, key_tag: u32) {
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
        let Some(key_desc) = pair.descriptor().get_field(key_tag) else {
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

/// Total order over the pair-key values the adapters accept (`Eq + Hash` scalars); the order itself
/// is arbitrary, it only has to be deterministic on both sides of a round-trip.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum MapKey {
    Bool(bool),
    Int(i64),
    Uint(u64),
    Str(String),
}
