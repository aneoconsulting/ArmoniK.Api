//! The differential tests: randomized `DynamicMessage`s generated from the real protobuf
//! descriptors are round-tripped through the armonik types (decode + re-encode) and compared
//! semantically. Two ratchets keep the quotient honest: every message of the descriptor pool must
//! be registered or tracked, and every field of every registered message must stay
//! information-bearing under the types' own `Normalize` projections.
//!
//! [`super::mutate`] re-runs the round-trip over the same messages encoded the other legal ways: a
//! peer's choices about field order, packing, duplicates and fields this schema does not declare.
//!
//! Every randomized failure prints the seed needed to replay the exact case.

use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};

use super::{arbitrary, compare, probe, registrations, registry, rng};

static DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

pub(super) const ITERATIONS: u64 = 64;
pub(super) const RECURSION_DEPTH: u32 = 3;

pub(super) fn pool() -> DescriptorPool {
    DescriptorPool::decode(DESCRIPTOR).expect("embedded descriptor set decodes")
}

/// Recursive `name: value` dump of the set fields, for failure messages; `DynamicMessage`'s `Debug`
/// impl prints whole descriptors.
pub(super) fn debug_fields(message: &DynamicMessage) -> String {
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
    for (proto, hooks) in registry::entries() {
        let desc = pool
            .get_message_by_name(proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", proto));
        for iteration in 0..ITERATIONS {
            let seed = rng::seed(proto, iteration);
            let mut rng = rng::SplitMix64::new(seed);
            let mut original = arbitrary::message(&desc, &mut rng, RECURSION_DEPTH);
            let bytes = original.encode_to_vec();

            let reencoded = (hooks.roundtrip)(&bytes).unwrap_or_else(|err| {
                panic!(
                    "armonik type failed to decode `{}` (seed {seed:#018x}): {err}\n\
                     original: {original:#?}",
                    proto
                )
            });
            let mut back = DynamicMessage::decode(desc.clone(), reencoded.as_slice())
                .unwrap_or_else(|err| {
                    panic!(
                        "re-encoded bytes of `{}` do not decode (seed {seed:#018x}): {err}\n\
                         original: {original:#?}",
                        proto
                    )
                });

            registry::normalize(&mut original);
            registry::normalize(&mut back);

            assert!(
                compare::messages(&original, &back),
                "semantic mismatch for `{}` (seed {seed:#018x})\n\
                 original:   {}\n\
                 round-trip: {}",
                proto,
                debug_fields(&original),
                debug_fields(&back),
            );
        }
    }
}

/// Several Rust types can stand for one proto message: a request type per RPC that shares a wire
/// message, one stand-in for `Empty` per RPC using it. [`registry::normalize`] keys on the proto
/// name, since a nested message is a name with no Rust type attached, so it applies one member of
/// each group to every message of that name, and which member is whichever the linker put first.
/// Sound only while a group agrees, which is what this pins.
#[test]
fn types_sharing_a_proto_name_agree() {
    use std::collections::HashMap;

    let pool = pool();
    let mut groups: HashMap<&str, Vec<registrations::Hooks>> = HashMap::new();
    for (proto, hooks) in registry::entries() {
        groups.entry(proto).or_default().push(hooks);
    }

    for (proto, group) in groups.iter().filter(|(_, group)| group.len() > 1) {
        let desc = pool
            .get_message_by_name(proto)
            .unwrap_or_else(|| panic!("registry entry `{proto}` is not in the descriptor"));
        let (first, rest) = group.split_first().expect("more than one registration");
        for other in rest {
            let (left_name, right_name) = ((first.type_name)(), (other.type_name)());
            assert_eq!(
                (first.default_encoding)(),
                (other.default_encoding)(),
                "`{proto}`: {left_name} and {right_name} encode different defaults, so the \
                 canonical-absence fold depends on which one registered first",
            );
            for iteration in 0..ITERATIONS {
                let seed = rng::seed(proto, iteration);
                let mut rng = rng::SplitMix64::new(seed);
                let message = arbitrary::message(&desc, &mut rng, RECURSION_DEPTH);
                let (mut left, mut right) = (message.clone(), message);
                (first.normalize)(&mut left);
                (other.normalize)(&mut right);
                assert!(
                    compare::messages(&left, &right),
                    "`{proto}`: {left_name} and {right_name} project it differently \
                     (seed {seed:#018x})\n\
                     {left_name}: {}\n{right_name}: {}",
                    debug_fields(&left),
                    debug_fields(&right),
                );
            }
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
    for (proto, hooks) in registry::entries() {
        let desc = pool
            .get_message_by_name(proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", proto));
        let message = DynamicMessage::decode(desc, (hooks.default_encoding)().as_slice())
            .unwrap_or_else(|err| panic!("the default encoding of `{}` decodes: {err}", proto));
        if let Some(field) = first_nonzero(&message) {
            panic!(
                "`{}`: Default::default() carries a non-zero `{field}`. Move the value to a named \
                 constructor (`recommended()`, `Sort::ascending`, ...) and leave Default at the \
                 proto zero.\n    default: {}",
                proto,
                debug_fields(&message),
            );
        }
    }
}

/// A message with no oneof member set decodes to a value with no oneof member set.
///
/// Stricter than the invariant above, which lets a defaulted oneof through as long as its payload
/// is itself zero, and that slack is exactly what hid the bug: 15 of the flattened oneofs had no
/// "no member set" variant, so an absent oneof decoded to whichever member the hand-written
/// `Default` happened to pick, and re-encoded with that member set. A peer that left
/// `value_condition` unset was read as `task_id string-equals ""`, which selects a different set of
/// tasks and cannot be rejected.
///
/// Synthetic oneofs are skipped: proto3 `optional` is one, and `Option<T>` models it directly.
#[test]
fn an_absent_oneof_decodes_to_no_member() {
    let pool = pool();
    let mut wrong = Vec::new();
    for (proto, hooks) in registry::entries() {
        let desc = pool
            .get_message_by_name(proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", proto));

        let reencoded = (hooks.roundtrip)(&[])
            .unwrap_or_else(|err| panic!("`{proto}` fails to decode the empty message: {err}"));
        let decoded = DynamicMessage::decode(desc.clone(), reencoded.as_slice())
            .unwrap_or_else(|err| panic!("`{proto}` re-encodes the empty message as: {err}"));

        for oneof in desc.oneofs() {
            if oneof.is_synthetic() || FLATTENED_ONEOFS.contains(&proto) {
                continue;
            }
            if let Some(field) = oneof.fields().find(|field| decoded.has_field(field)) {
                wrong.push(format!(
                    "`{proto}`: an empty message decodes with `{}.{}` set ({})",
                    oneof.name(),
                    field.name(),
                    debug_fields(&decoded),
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "these types read an absent oneof as one of its members. Give each an attribute-less \
         variant for \"no member set\", so the absence is a value rather than whichever member \
         `Default` picks:\n    {}",
        wrong.join("\n    "),
    );
}

/// Messages whose oneof a [`transparent`](armonik_macros::enumeration#transparent) enumeration
/// flattens away, so there is no Rust variant left to hold "no member set".
///
/// Justified by the shape rather than allowed by fiat: each of these is a wrapper whose oneof has a
/// single member, itself a wrapper around one proto enum, and the flattened enum's unspecified
/// value already means "names no field". The absent oneof and the present-but-unspecified member
/// say the same thing, so nothing is lost by conflating them. Contrast
/// `applications.FilterField.value_condition`, also a one-member oneof, where they say different
/// things (no condition at all, versus a string condition on the empty string) and the Rust type
/// therefore carries the variant.
const FLATTENED_ONEOFS: &[&str] = &[
    "armonik.api.grpc.v1.applications.ApplicationField",
    "armonik.api.grpc.v1.partitions.PartitionField",
    "armonik.api.grpc.v1.results.ResultField",
];

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
/// equivalent to the empty message. Every other field must stay information-bearing. The
/// `Normalize` projections come from the same attributes as the codecs, so this ratchet is what
/// keeps a codec bug from hiding behind a matching projection bug: a field erased by both shows up
/// here and has to be justified by hand.
///
/// Empty, and that is the statement. It held `armonik.api.grpc.v1.Output.ok` for as long as `Ok`
/// was the `Output` default, which made `{ ok: {} }` the zero value and so indistinguishable from
/// an absent message. Giving `Output` a "no member set" variant separated the two, and took the
/// last exception with it.
const UNINFORMATIVE_FIELDS: &[&str] = &[];

const PROBE_DEPTH: u32 = 3;

#[test]
fn field_information_ratchet() {
    let pool = pool();
    for (proto, hooks) in registry::entries() {
        let desc = pool
            .get_message_by_name(proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", proto));
        let mut empty = DynamicMessage::new(desc.clone());
        registry::normalize(&mut empty);

        for field in desc.fields() {
            let qualified = format!("{}.{}", proto, field.name());
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
                let reencoded = (hooks.roundtrip)(&bytes).unwrap_or_else(|err| {
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

/// Messages present in the schema but referenced by nothing; see `registrations::UNREFERENCED_MESSAGES`. The
/// messages of *unexposed RPCs* are not listed anywhere by hand: `service!` registers them from its
/// `unexposed(...)` declaration (`registrations::unexposed()`), so that allowlist cannot drift from the RPC
/// one.
const PERMANENT_UNMAPPED: &[&str] = registrations::UNREFERENCED_MESSAGES;

#[test]
fn descriptor_coverage_ratchet() {
    let pool = pool();
    let registered: Vec<&str> = registry::entries().map(|(proto, _)| proto).collect();
    // Messages flattened into a parent type, harvested from the annotations, and messages of
    // unexposed RPCs, registered by `service!`.
    let absorbed = registrations::absorbed();
    let unexposed = registrations::unexposed();

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

    // The block `mutate` files its synthetic unknown fields under has to stay unknown. Derived
    // rather than restated: a schema that ever declares a tag up there would make those fields
    // known to whichever message declared it, and the mutation would stop being one.
    let highest = pool
        .all_messages()
        .flat_map(|message| {
            message
                .fields()
                .map(|field| field.number())
                .collect::<Vec<_>>()
        })
        .max()
        .unwrap_or_default();
    assert!(
        highest < *super::mutate::UNKNOWN_TAGS.start(),
        "the schema declares tag {highest}, which reaches into the block `mutate` uses for its \
         synthetic unknown fields ({:?}). Move that block up.",
        super::mutate::UNKNOWN_TAGS,
    );
}

/// Every declared RPC has a client method, and every client method names a declared RPC.
///
/// The two sides are filled from opposite ends and never see each other: `service!` records what
/// `rpc/*.rs` declares, `#[armonik_macros::client]` records what `client/*.rs` implements. Nothing
/// else connects them, which is the point -- the client methods are written by hand, so their
/// signatures cannot drift with the schema, and this is what replaces the guarantee that generating
/// them gives for free: that one exists at all.
///
/// `unexposed(...)` RPCs are not declared, which is what exempts them.
#[test]
fn every_rpc_has_a_client_method() {
    use super::registrations::{CLIENT_METHODS, DECLARED_RPCS};

    let key = |rpc: &super::registrations::Rpc| format!("{}/{}", rpc.service, rpc.method);
    let declared: std::collections::BTreeSet<String> = DECLARED_RPCS.iter().map(key).collect();
    let implemented: std::collections::BTreeSet<String> = CLIENT_METHODS.iter().map(key).collect();

    let uncovered: Vec<&String> = declared.difference(&implemented).collect();
    assert!(
        uncovered.is_empty(),
        "these RPCs have no client method; write one in `client/<svc>.rs` under \
         #[armonik(rpc = \"...\")], or list the RPC in `unexposed(...)`:\n    {uncovered:#?}"
    );

    let unclaimed: Vec<&String> = implemented.difference(&declared).collect();
    assert!(
        unclaimed.is_empty(),
        "these client methods name an RPC that no `service!` declares:\n    {unclaimed:#?}"
    );

    // A guard on the guard: an empty pair of slices would satisfy both assertions above.
    assert_eq!(
        declared.len(),
        60,
        "the RPC count moved; update this number once the change is deliberate"
    );
}
