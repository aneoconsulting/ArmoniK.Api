//! Differential harness: randomized `DynamicMessage`s generated from the real protobuf descriptors
//! are round-tripped through the armonik types (decode + re-encode) and compared semantically. Two
//! ratchets keep the quotient honest: every message of the descriptor pool must be registered or
//! tracked, and every field of every registered message must stay information-bearing under the
//! types' own `Normalize` projections.
//!
//! Every randomized failure prints the seed needed to replay the exact case.

mod arbitrary;
mod compare;
mod probe;
mod registry;
mod rng;

use armonik::reexports::prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};

static DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

const ITERATIONS: u64 = 64;
const RECURSION_DEPTH: u32 = 3;

fn pool() -> DescriptorPool {
    DescriptorPool::decode(DESCRIPTOR).expect("embedded descriptor set decodes")
}

/// Recursive `name: value` dump of the set fields, for failure messages; `DynamicMessage`'s `Debug`
/// impl prints whole descriptors.
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

/// The zero-default invariant: every type's `Default::default()` is the proto zero value. This is
/// what lets decoding seed from `Default` with no special wire semantics, and it is checked on the
/// encoding of `Default::default()` alone: decoding starts from `Default`, so anything that goes
/// through a round-trip agrees with whatever the default happens to be. A set oneof member is
/// allowed as long as its payload is itself zero, which is what a defaulted oneof encodes to.
#[test]
fn default_encoding_is_the_proto_zero() {
    let pool = pool();
    for entry in registry::entries() {
        let desc = pool
            .get_message_by_name(entry.proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", entry.proto));
        let message = DynamicMessage::decode(desc, (entry.default_encoding)().as_slice())
            .unwrap_or_else(|err| {
                panic!("the default encoding of `{}` decodes: {err}", entry.proto)
            });
        if let Some(field) = first_nonzero(&message) {
            panic!(
                "`{}`: Default::default() carries a non-zero `{field}`. Move the value to a named \
                 constructor (`recommended()`, `Sort::ascending`, ...) and leave Default at the \
                 proto zero.\n    default: {}",
                entry.proto,
                debug_fields(&message),
            );
        }
    }
}

/// The path of the first field holding something other than the proto zero, if any.
fn first_nonzero(message: &DynamicMessage) -> Option<String> {
    use prost_reflect::ReflectMessage;

    message.descriptor().fields().find_map(|field| {
        if !message.has_field(&field) {
            return None;
        }
        match message.get_field(&field).as_ref() {
            prost_reflect::Value::Message(inner) => {
                first_nonzero(inner).map(|path| format!("{}.{path}", field.name()))
            }
            value if is_nonzero(value) => Some(field.name().to_owned()),
            _ => None,
        }
    })
}

/// Whether a value differs from the proto zero of its kind. A message is zero when every field it
/// holds is, which is what [`first_nonzero`] recurses for; repeated and map fields are zero only
/// when empty.
fn is_nonzero(value: &prost_reflect::Value) -> bool {
    use prost_reflect::Value;

    match value {
        Value::Message(inner) => first_nonzero(inner).is_some(),
        Value::List(items) => !items.is_empty(),
        Value::Map(entries) => !entries.is_empty(),
        Value::Bool(value) => *value,
        Value::I32(value) => *value != 0,
        Value::I64(value) => *value != 0,
        Value::U32(value) => *value != 0,
        Value::U64(value) => *value != 0,
        Value::F32(value) => *value != 0.0,
        Value::F64(value) => *value != 0.0,
        Value::EnumNumber(value) => *value != 0,
        Value::String(value) => !value.is_empty(),
        Value::Bytes(value) => !value.is_empty(),
    }
}

/// Fields that collapse to "nothing" under the quotient by design: their only representations are
/// equivalent to the empty message (the default member of a oneof whose payload carries no data).
/// Every other field must stay information-bearing. The `Normalize` projections come from the same
/// attributes as the codecs, so this ratchet is what keeps a codec bug from hiding behind a
/// matching projection bug: a field erased by both shows up here and has to be justified by hand.
const UNINFORMATIVE_FIELDS: &[&str] = &[
    // The `Ok` member is the `Output` default and its payload is `Empty`: `{ ok: {} }` IS the zero
    // value, indistinguishable from an absent message by the zero-default invariant.
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

                // The candidate distinguishes, so it must survive the round-trip,
                // deterministically, one field at a time.
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

/// Messages present in the schema but referenced by nothing; see `wire::UNREFERENCED_MESSAGES`. The
/// messages of *unexposed RPCs* are not listed anywhere by hand: `service!` registers them from its
/// `unexposed(...)` declaration (`wire::unexposed()`), so that allowlist cannot drift from the RPC
/// one.
const PERMANENT_UNMAPPED: &[&str] = armonik::wire::UNREFERENCED_MESSAGES;

#[test]
fn descriptor_coverage_ratchet() {
    let pool = pool();
    let registered: Vec<&str> = registry::entries().map(|entry| entry.proto).collect();
    // Messages flattened into a parent type, harvested from the annotations, and messages of
    // unexposed RPCs, registered by `service!`.
    let absorbed = armonik::wire::absorbed();
    let unexposed = armonik::wire::unexposed();

    let mut missing = Vec::new();
    for message in pool.all_messages() {
        let name = message.full_name();
        if !name.starts_with("armonik.") || message.is_map_entry() {
            continue;
        }
        if registered.contains(&name)
            || absorbed.contains(&name)
            || unexposed.contains(&name)
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

    // A message with a Rust type cannot also be absorbed (an `absorbs` pointed at a real type).
    let conflicts: Vec<&str> = absorbed
        .iter()
        .copied()
        .filter(|name| registered.contains(name))
        .collect();
    assert!(
        conflicts.is_empty(),
        "these messages are both registered and absorbed:\n    {conflicts:?}"
    );

    // Every absorbed name must exist (a flattened message that was renamed or removed leaves a
    // stale `absorbs`).
    for name in &absorbed {
        assert!(
            pool.get_message_by_name(name).is_some(),
            "absorbed entry `{name}` does not exist in the descriptor"
        );
    }

    // Every tracked name must actually exist (a renamed or removed message leaves a stale allowlist
    // entry).
    for name in PERMANENT_UNMAPPED {
        assert!(
            pool.get_message_by_name(name).is_some(),
            "PERMANENT_UNMAPPED entry `{name}` does not exist in the descriptor"
        );
    }
}
