//! Deterministic distinguishing probes for the field-information ratchet:
//! per-field candidate values that should survive the quotient.

use std::collections::HashMap;

use armonik_types::reexports::bytes::Bytes;
use prost_reflect::{DynamicMessage, FieldDescriptor, Kind, MapKey, MessageDescriptor, Value};

/// Candidate distinguishing values for a field. The field carries
/// information iff at least one candidate normalizes differently from the
/// absent field. Enums get one candidate per declared nonzero value plus an
/// unknown one, because a single value can legitimately coincide with the
/// type's canonical "absent" form (an explicit ascending sort direction is
/// indistinguishable from no direction — a descending one is not).
pub fn candidates(field: &FieldDescriptor, depth: u32) -> Vec<Value> {
    if field.is_map() {
        let Kind::Message(entry) = field.kind() else {
            unreachable!("map fields have message kind")
        };
        let key = map_key(&entry.map_entry_key_field().kind());
        return elements(&entry.map_entry_value_field().kind(), depth)
            .into_iter()
            .map(|value| Value::Map(HashMap::from([(key.clone(), value)])))
            .collect();
    }
    if field.is_list() {
        return elements(&field.kind(), depth)
            .into_iter()
            .map(|value| Value::List(vec![value]))
            .collect();
    }
    elements(&field.kind(), depth)
}

fn elements(kind: &Kind, depth: u32) -> Vec<Value> {
    match kind {
        Kind::Double => vec![Value::F64(1.5)],
        Kind::Float => vec![Value::F32(1.5)],
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => vec![Value::I32(1)],
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => vec![Value::I64(1)],
        Kind::Uint32 | Kind::Fixed32 => vec![Value::U32(1)],
        Kind::Uint64 | Kind::Fixed64 => vec![Value::U64(1)],
        Kind::Bool => vec![Value::Bool(true)],
        Kind::String => vec![Value::String("probe".to_owned())],
        Kind::Bytes => vec![Value::Bytes(Bytes::from_static(b"probe"))],
        Kind::Message(desc) => rich_variants(desc, depth)
            .into_iter()
            .map(Value::Message)
            .collect(),
        Kind::Enum(desc) => {
            let declared: Vec<i32> = desc.values().map(|value| value.number()).collect();
            let mut unknown = 1000;
            while declared.contains(&unknown) {
                unknown += 1;
            }
            declared
                .into_iter()
                .filter(|&number| number != 0)
                .chain([unknown])
                .map(Value::EnumNumber)
                .collect()
        }
    }
}

/// [`rich`], plus one variant per further member of each real oneof (a
/// member may carry no data — the `Ok` of a status oneof is equivalent to
/// the whole message being absent — so no single choice can witness the
/// message's information).
fn rich_variants(desc: &MessageDescriptor, depth: u32) -> Vec<DynamicMessage> {
    let base = rich(desc, depth);
    let mut variants = vec![base.clone()];
    if depth == 0 {
        return variants;
    }
    for oneof in desc.oneofs() {
        if oneof.is_synthetic() {
            continue;
        }
        for member in oneof.fields().skip(1) {
            if let Some(value) = candidates(&member, depth - 1).into_iter().next() {
                // Setting a oneof member clears the previously set one.
                let mut variant = base.clone();
                variant.set_field(&member, value);
                variants.push(variant);
            }
        }
    }
    variants
}

/// A message with every field set to a distinguishing value (the first
/// candidate), and the first member of every real oneof set. Depth-limited:
/// at zero it degrades to the empty message, whose distinguishing power then
/// rests on the fields above it.
fn rich(desc: &MessageDescriptor, depth: u32) -> DynamicMessage {
    let mut message = DynamicMessage::new(desc.clone());
    if depth == 0 {
        return message;
    }
    for oneof in desc.oneofs() {
        if oneof.is_synthetic() {
            continue;
        }
        if let Some(member) = oneof.fields().next() {
            if let Some(value) = candidates(&member, depth - 1).into_iter().next() {
                message.set_field(&member, value);
            }
        }
    }
    for field in desc.fields() {
        if field
            .containing_oneof()
            .is_some_and(|oneof| !oneof.is_synthetic())
        {
            continue;
        }
        if let Some(value) = candidates(&field, depth - 1).into_iter().next() {
            message.set_field(&field, value);
        }
    }
    message
}

fn map_key(kind: &Kind) -> MapKey {
    match kind {
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => MapKey::I32(1),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => MapKey::I64(1),
        Kind::Uint32 | Kind::Fixed32 => MapKey::U32(1),
        Kind::Uint64 | Kind::Fixed64 => MapKey::U64(1),
        Kind::Bool => MapKey::Bool(true),
        Kind::String => MapKey::String("probe".to_owned()),
        other => unreachable!("invalid map key kind {other:?}"),
    }
}
