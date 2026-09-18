//! A second protobuf implementation on the same descriptor, so that stage 1's finding does
//! not rest on prost's derive alone.
//!
//! prost-reflect encodes a `DynamicMessage` from the descriptor at runtime; it shares no
//! encoding code with the `#[derive(Message)]` expansion. If both omit an implicit-presence
//! leaf holding the proto zero, that is protobuf's rule and not a prost quirk.

use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, Value};

fn pool() -> DescriptorPool {
    DescriptorPool::decode(shapes_prost::DESCRIPTOR).unwrap()
}

#[test]
fn an_implicit_presence_zero_leaf_is_omitted_by_both_implementations() {
    // Timestamp { seconds: 1700000000, nanos: 0 } is what `timestamp(idx=0)` produces, and
    // it is the exact value the schema emitters write a two-byte `nanos = 0` for.
    let derived = shapes_prost::shapes::Timestamp {
        seconds: 1_700_000_000,
        nanos: 0,
    }
    .encode_to_vec();

    let pool = pool();
    let desc = pool
        .get_message_by_name("armonik.ffi.shapes.v1.Timestamp")
        .unwrap();
    let mut dynamic = DynamicMessage::new(desc);
    dynamic.set_field_by_name("seconds", Value::I64(1_700_000_000));
    dynamic.set_field_by_name("nanos", Value::I32(0));
    let reflected = dynamic.encode_to_vec();

    // Both implementations write field 1 as a varint and stop. No tag 2 anywhere.
    assert_eq!(derived, reflected, "prost derive against prost-reflect");
    assert_eq!(derived[0], 0x08, "field 1, varint");
    assert_eq!(derived.len(), 6, "a 5-byte varint and its key, and nothing else: {derived:02x?}");
    assert!(!derived.windows(2).any(|w| w == [0x10, 0x00]), "no `nanos = 0`");

    // What emit/payloads.py writes today: the same, plus a two-byte `nanos = 0`.
    let mut emitters = derived.clone();
    emitters.extend_from_slice(&[0x10, 0x00]);
    assert_ne!(derived, emitters);

    // And the extra bytes are accepted on the way back in, decoding to the same value,
    // which is why nothing before this caught it: it is legal wire, just not canonical.
    assert_eq!(
        shapes_prost::shapes::Timestamp::decode(&emitters[..]).unwrap(),
        shapes_prost::shapes::Timestamp { seconds: 1_700_000_000, nanos: 0 }
    );
}

#[test]
fn a_duration_that_is_all_zero_encodes_to_an_empty_message() {
    // `duration(idx=0)` is (0, 0): every leaf at its proto zero, so the message body is empty
    // but the field is still present. That is P2.x element 0's `options.max_duration`.
    let derived = shapes_prost::shapes::Duration { seconds: 0, nanos: 0 }.encode_to_vec();
    assert!(derived.is_empty());
}
