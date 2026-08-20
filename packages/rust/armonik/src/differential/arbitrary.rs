//! Randomized `DynamicMessage` generation from a descriptor, probing the presence edges on purpose:
//! empty nested messages, default-valued scalars, unknown enum values, empty and populated
//! containers.

use std::collections::HashMap;

use bytes::Bytes;
use prost_reflect::{DynamicMessage, Kind, MapKey, MessageDescriptor, Value};

use super::rng::SplitMix64;

pub fn message(desc: &MessageDescriptor, rng: &mut SplitMix64, depth: u32) -> DynamicMessage {
    let mut message = DynamicMessage::new(desc.clone());

    // Real oneofs: set at most one member.
    for oneof in desc.oneofs() {
        if oneof.is_synthetic() {
            continue;
        }
        let members: Vec<_> = oneof.fields().collect();
        if members.is_empty() || !rng.chance(800) {
            continue;
        }
        let member = &members[rng.below(members.len() as u64) as usize];
        let value = field_value(&member.kind(), rng, depth);
        message.set_field(member, value);
    }

    for field in desc.fields() {
        if field
            .containing_oneof()
            .is_some_and(|oneof| !oneof.is_synthetic())
        {
            continue;
        }
        if !rng.chance(700) {
            continue;
        }
        let value = if field.is_map() {
            let Kind::Message(entry) = field.kind() else {
                unreachable!("map fields have message kind")
            };
            let key_kind = entry.map_entry_key_field().kind();
            let value_kind = entry.map_entry_value_field().kind();
            let mut map = HashMap::new();
            for _ in 0..rng.below(4) {
                map.insert(
                    map_key(&key_kind, rng),
                    field_value(&value_kind, rng, depth),
                );
            }
            Value::Map(map)
        } else if field.is_list() {
            let kind = field.kind();
            let values = (0..rng.below(4))
                .map(|_| field_value(&kind, rng, depth))
                .collect();
            Value::List(values)
        } else {
            field_value(&field.kind(), rng, depth)
        };
        message.set_field(&field, value);
    }

    message
}

fn field_value(kind: &Kind, rng: &mut SplitMix64, depth: u32) -> Value {
    match kind {
        Kind::Double => Value::F64(f64_sample(rng)),
        Kind::Float => Value::F32(f64_sample(rng) as f32),
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => Value::I32(i32_sample(rng)),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => Value::I64(i64_sample(rng)),
        Kind::Uint32 | Kind::Fixed32 => Value::U32(rng.next() as u32),
        Kind::Uint64 | Kind::Fixed64 => Value::U64(rng.next()),
        Kind::Bool => Value::Bool(rng.chance(500)),
        Kind::String => Value::String(string_sample(rng)),
        Kind::Bytes => Value::Bytes(bytes_sample(rng)),
        Kind::Message(desc) => {
            if depth == 0 {
                // Deliberately empty: probes unset-vs-empty equivalence.
                Value::Message(DynamicMessage::new(desc.clone()))
            } else {
                Value::Message(message(desc, rng, depth - 1))
            }
        }
        Kind::Enum(desc) => {
            let values: Vec<i32> = desc.values().map(|value| value.number()).collect();
            // Known values, zero, or a value unknown to the schema (proto3 open enums must
            // round-trip losslessly).
            let choice = rng.below(values.len() as u64 + 2);
            let number = match values.get(choice as usize) {
                Some(number) => *number,
                None if choice == values.len() as u64 => 0,
                None => 1000 + rng.below(1000) as i32,
            };
            Value::EnumNumber(number)
        }
    }
}

fn map_key(kind: &Kind, rng: &mut SplitMix64) -> MapKey {
    match kind {
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => MapKey::I32(i32_sample(rng)),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => MapKey::I64(i64_sample(rng)),
        Kind::Uint32 | Kind::Fixed32 => MapKey::U32(rng.next() as u32),
        Kind::Uint64 | Kind::Fixed64 => MapKey::U64(rng.next()),
        Kind::Bool => MapKey::Bool(rng.chance(500)),
        Kind::String => MapKey::String(string_sample(rng)),
        other => unreachable!("invalid map key kind {other:?}"),
    }
}

fn i32_sample(rng: &mut SplitMix64) -> i32 {
    match rng.below(6) {
        0 => 0,
        1 => 1,
        2 => -1,
        3 => i32::MAX,
        4 => i32::MIN,
        _ => rng.next() as i32,
    }
}

fn i64_sample(rng: &mut SplitMix64) -> i64 {
    match rng.below(6) {
        0 => 0,
        1 => 1,
        2 => -1,
        3 => i64::MAX,
        4 => i64::MIN,
        _ => rng.next() as i64,
    }
}

fn f64_sample(rng: &mut SplitMix64) -> f64 {
    match rng.below(5) {
        0 => 0.0,
        // The one value whose implicit-presence encoding a `==` zero test gets wrong.
        1 => -0.0,
        2 => 1.5,
        3 => -2.25,
        _ => (rng.next() as i32) as f64 / 16.0,
    }
}

fn string_sample(rng: &mut SplitMix64) -> String {
    // The last sample is multi-byte UTF-8 on purpose, spelled with escapes to keep this file ASCII.
    const POOL: &[&str] = &[
        "",
        "a",
        "value",
        "id-1234",
        "namespace/path",
        "\u{e9}\u{e0}\u{fc}-unicode",
    ];
    POOL[rng.below(POOL.len() as u64) as usize].to_owned()
}

fn bytes_sample(rng: &mut SplitMix64) -> Bytes {
    const POOL: &[&[u8]] = &[b"", b"\x00", b"payload-bytes", b"\xff\xfe\x00\x01"];
    Bytes::from_static(POOL[rng.below(POOL.len() as u64) as usize])
}
