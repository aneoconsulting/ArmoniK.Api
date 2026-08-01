//! Differential harness: randomized `DynamicMessage`s generated from the
//! real protobuf descriptors are round-tripped through the armonik types
//! (decode + re-encode) and compared semantically. Two ratchets keep the
//! quotient honest: every message of the descriptor pool must be registered
//! or tracked, and every field of every registered message must stay
//! information-bearing under the types' own `Normalize` projections.
//!
//! Every randomized failure prints the seed needed to replay the exact case.

mod arbitrary;
mod compare;
mod probe;
mod registry;
mod rng;

use armonik_types::reexports::prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};

static DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

const ITERATIONS: u64 = 64;
const RECURSION_DEPTH: u32 = 3;

fn pool() -> DescriptorPool {
    DescriptorPool::decode(DESCRIPTOR).expect("embedded descriptor set decodes")
}

/// Compact `name: value` dump of the set fields (recursive), for failure
/// messages — the `Debug` impl of `DynamicMessage` prints whole descriptors.
fn debug_fields(message: &DynamicMessage) -> String {
    use prost_reflect::ReflectMessage;
    use std::fmt::Write;

    let mut out = String::from("{ ");
    for field in message.descriptor().fields() {
        if !message.has_field(&field) {
            continue;
        }
        let value = message.get_field(&field);
        let _ = write!(out, "{}: {}, ", field.name(), debug_value(value.as_ref()));
    }
    out.push('}');
    out
}

fn debug_value(value: &prost_reflect::Value) -> String {
    match value {
        prost_reflect::Value::Message(inner) => debug_fields(inner),
        prost_reflect::Value::List(items) => {
            let items: Vec<String> = items.iter().map(debug_value).collect();
            format!("[{}]", items.join(", "))
        }
        other => format!("{other:?}"),
    }
}

#[test]
fn registered_types_roundtrip() {
    let pool = pool();
    for entry in registry::entries() {
        let desc = pool
            .get_message_by_name(entry.proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", entry.proto));
        for iteration in 0..ITERATIONS {
            let seed = rng::seed(entry.proto, iteration);
            let mut rng = rng::SplitMix64::new(seed);
            let mut original = arbitrary::message(&desc, &mut rng, RECURSION_DEPTH);
            let bytes = original.encode_to_vec();

            let reencoded = (entry.roundtrip)(&bytes).unwrap_or_else(|err| {
                panic!(
                    "armonik type failed to decode `{}` (seed {seed:#018x}): {err}\n\
                     original: {original:#?}",
                    entry.proto
                )
            });
            let mut back = DynamicMessage::decode(desc.clone(), reencoded.as_slice())
                .unwrap_or_else(|err| {
                    panic!(
                        "re-encoded bytes of `{}` do not decode (seed {seed:#018x}): {err}\n\
                         original: {original:#?}",
                        entry.proto
                    )
                });

            registry::normalize(&mut original);
            registry::normalize(&mut back);

            assert!(
                compare::messages(&original, &back),
                "semantic mismatch for `{}` (seed {seed:#018x})\n\
                 original:   {}\n\
                 round-trip: {}",
                entry.proto,
                debug_fields(&original),
                debug_fields(&back),
            );
        }
    }
}

/// The zero-default invariant: every type's `Default::default()` is the
/// proto zero value, so decoding an empty message yields it. This is what
/// lets decoding seed from `Default` with no special wire semantics.
#[test]
fn empty_message_decodes_to_default() {
    for entry in registry::entries() {
        let reencoded = (entry.roundtrip)(&[]).expect("an empty message decodes");
        assert_eq!(
            reencoded,
            (entry.default_encoding)(),
            "`{}`: decoding an empty message must yield Default::default()",
            entry.proto,
        );
    }
}

/// Fields that collapse to "nothing" under the quotient by design: their
/// only representations are equivalent to the empty message (the default
/// member of a oneof whose payload carries no data). Every other field must
/// stay information-bearing — since the `Normalize` projections are
/// generated from the same attributes as the codecs, this ratchet is what
/// keeps a codec bug from hiding behind a matching projection bug: a field
/// erased by both shows up here and must be justified by hand.
const UNINFORMATIVE_FIELDS: &[&str] = &[
    // The `Ok` member is the `Output` default and its payload is `Empty`:
    // `{ ok: {} }` IS the zero value, indistinguishable from an absent
    // message by the zero-default invariant.
    "armonik.api.grpc.v1.Output.ok",
];

const PROBE_DEPTH: u32 = 3;

#[test]
fn field_information_ratchet() {
    let pool = pool();
    for entry in registry::entries() {
        let desc = pool
            .get_message_by_name(entry.proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", entry.proto));
        let mut empty = DynamicMessage::new(desc.clone());
        registry::normalize(&mut empty);

        for field in desc.fields() {
            let qualified = format!("{}.{}", entry.proto, field.name());
            let mut informative = false;
            for candidate in probe::candidates(&field, PROBE_DEPTH) {
                let mut probe = DynamicMessage::new(desc.clone());
                probe.set_field(&field, candidate);
                let bytes = probe.encode_to_vec();
                let mut normalized = probe;
                registry::normalize(&mut normalized);
                if compare::messages(&normalized, &empty) {
                    // This candidate collapses by design; try the others.
                    continue;
                }
                informative = true;

                // The candidate distinguishes, so it must survive the
                // round-trip — deterministically, one field at a time.
                let reencoded = (entry.roundtrip)(&bytes).unwrap_or_else(|err| {
                    panic!("armonik type failed to decode a probe of `{qualified}`: {err}")
                });
                let mut back = DynamicMessage::decode(desc.clone(), reencoded.as_slice())
                    .unwrap_or_else(|err| {
                        panic!("re-encoded probe of `{qualified}` does not decode: {err}")
                    });
                registry::normalize(&mut back);
                assert!(
                    compare::messages(&normalized, &back),
                    "probe of `{qualified}` does not survive the round-trip\n\
                     probe:      {}\n\
                     round-trip: {}",
                    debug_fields(&normalized),
                    debug_fields(&back),
                );
            }
            if informative {
                assert!(
                    !UNINFORMATIVE_FIELDS.contains(&qualified.as_str()),
                    "`{qualified}` is information-bearing; remove it from UNINFORMATIVE_FIELDS"
                );
            } else {
                assert!(
                    UNINFORMATIVE_FIELDS.contains(&qualified.as_str()),
                    "`{qualified}` carries no information under the quotient: every probe \
                     normalizes to the empty message. Fix the codec or its projection, or \
                     add the field to UNINFORMATIVE_FIELDS with a justification."
                );
            }
        }
    }
}

/// Messages of RPCs the crate does not expose. Unlike the many messages that
/// are *flattened into a parent type* — which self-register as absorbed through
/// `wire::absorbed()` (a `with` adapter's `absorbs`, a transparent chain's
/// middles, an inline struct variant) — these are not absorbed by any type, so
/// they are tracked here. Shared with the build script's stub pruning (one
/// list, `wire::UNEXPOSED_RPC_MESSAGES`) so the two allowlists cannot drift.
const PERMANENT_UNMAPPED: &[&str] = armonik_types::wire::UNEXPOSED_RPC_MESSAGES;

#[test]
fn descriptor_coverage_ratchet() {
    let pool = pool();
    let registered: Vec<&str> = registry::entries().map(|entry| entry.proto).collect();
    // Messages flattened into a parent type, harvested from the annotations.
    let absorbed = armonik_types::wire::absorbed();

    let mut missing = Vec::new();
    for message in pool.all_messages() {
        let name = message.full_name();
        if !name.starts_with("armonik.") || message.is_map_entry() {
            continue;
        }
        if registered.contains(&name)
            || absorbed.contains(&name)
            || PERMANENT_UNMAPPED.contains(&name)
        {
            continue;
        }
        missing.push(name.to_owned());
    }
    missing.sort();
    assert!(
        missing.is_empty(),
        "messages neither mapped nor tracked; add them to the registry:\n    \"{}\"",
        missing.join("\",\n    \"")
    );

    // A message with a Rust type cannot also be absorbed (an `absorbs`
    // pointed at a real type).
    let conflicts: Vec<&str> = absorbed
        .iter()
        .copied()
        .filter(|name| registered.contains(name))
        .collect();
    assert!(
        conflicts.is_empty(),
        "these messages are both registered and absorbed:\n    {conflicts:?}"
    );

    // Every absorbed name must exist (a flattened message that was renamed or
    // removed leaves a stale `absorbs`).
    for name in &absorbed {
        assert!(
            pool.get_message_by_name(name).is_some(),
            "absorbed entry `{name}` does not exist in the descriptor"
        );
    }

    // Every tracked name must actually exist (a renamed or removed message
    // leaves a stale allowlist entry).
    for name in PERMANENT_UNMAPPED {
        assert!(
            pool.get_message_by_name(name).is_some(),
            "PERMANENT_UNMAPPED entry `{name}` does not exist in the descriptor"
        );
    }
}
