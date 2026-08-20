//! What the differential harness structurally cannot see.
//!
//! The harness fuzzes every registered type against `prost-reflect` and compares semantically,
//! which covers the derived majority far better than a hand-copied prototype of the emitter's
//! output ever did. Four properties escape it, and they are what is left here, asserted on real
//! `objects/` types rather than on mirrors of them:
//!
//! * **a field spelled with the wrong wire type is rejected.** The mutation harness reorders,
//!   duplicates and unpacks records, but never re-spells one, so every wire-type check is invisible
//!   to it.
//! * **`Option` presence is exact.** The harness compares after `Normalize`, whose
//!   canonical-absence fold is precisely the distinction between `None` and `Some(default)`.
//! * **`clear()` resets to the proto zero.** Nothing on the round-trip path calls it.
//! * **which zeros reach the wire.** An implicit-presence leaf leaves its zero out; a message field
//!   is written whatever it holds. Two byte-level facts about values a reader cannot tell apart, so
//!   the semantic comparison is blind to both.
//!
//! Plus the `ProtoField` impls no API field instantiates, which therefore have no other coverage at
//! all.
//!
//! Two properties used to be here and are not any more, both measured out rather than argued out. A
//! oneof member holding its zero stays on the wire, which the round-trip *does* see: every oneof
//! enum carries a memberless variant, so dropping the payload decodes to a different Rust value.
//! And the unpacked form of a repeated field decodes, which `differential::mutate`'s `unpacked`
//! class produces for every packable field of every registered message; that reaches the enum codec
//! through the six repeated enum fields this schema declares, so a schema that stopped declaring
//! one would need the case back, next to the other codecs no field instantiates.
//!
//! Deliberately not a copy of the derive's output: the harness already compares against
//! `DynamicMessage`, and a snapshot of the emitter would only ever be updated after it.

use prost::bytes::BufMut;
use prost::encoding::{DecodeContext, WireType};
use prost::Message;

use super::ProtoField;

/// Encode and decode back, the way a peer would.
fn roundtrip<T: Message + Default>(value: &T) -> T {
    T::decode(value.encode_to_vec().as_slice()).expect("a self-encoded message decodes")
}

/// `None` and `Some(default)` are different on the wire and stay different through a round trip.
/// This is the one distinction the harness's canonical-absence fold deliberately erases.
#[test]
fn optional_presence_is_exact() {
    let absent = crate::sessions::Raw::default();
    assert_eq!(absent.created_at, None);
    assert_eq!(roundtrip(&absent).created_at, None);

    let present = crate::sessions::Raw {
        created_at: Some(prost_types::Timestamp::default()),
        ..Default::default()
    };
    assert_eq!(
        roundtrip(&present).created_at,
        Some(prost_types::Timestamp::default()),
        "an explicitly present zero timestamp survives as present",
    );
    assert!(
        present.encoded_len() > absent.encoded_len(),
        "and it costs bytes the absent one does not",
    );
}

/// `clear` is a whole-value reset: every derived type is `Default`, and the zero-default invariant
/// makes that the proto zero.
#[test]
fn clear_resets_to_the_proto_zero() {
    let mut raw = crate::sessions::Raw {
        session_id: String::from("session"),
        status: crate::SessionStatus::Running,
        partition_ids: vec![String::from("partition")],
        created_at: Some(prost_types::Timestamp::default()),
        ..Default::default()
    };
    raw.clear();
    assert_eq!(raw, crate::sessions::Raw::default());
}

/// A zero leaf is left out of an implicit-presence field, which is what a proto3 encoder does and
/// what the receiver cannot distinguish from an explicit zero.
#[test]
fn a_zero_leaf_is_left_off_the_wire() {
    use crate::submitter::wait_for_availability::Request;

    assert!(Request::default().encode_to_vec().is_empty());

    // Only the set one: key, length, byte.
    let one_set = Request {
        session_id: String::from("s"),
        result_id: String::new(),
    };
    assert_eq!(one_set.encode_to_vec(), [0x0a, 0x01, b's']);
    assert_eq!(roundtrip(&one_set), one_set);
}

/// A nested message holding only defaults is written too: absent and default are the same value for
/// the fields modelled without `Option`, and skipping it would cost a transparent wrapper its
/// presence.
#[test]
fn a_default_nested_message_is_still_written() {
    // `results::get::Response` is a single message-typed field, `result`, at tag 1.
    let response = crate::results::get::Response::default();
    let bytes = response.encode_to_vec();
    assert!(
        !bytes.is_empty(),
        "the default response is not the empty encoding",
    );

    let mut rest = bytes.as_slice();
    let (tag, wire_type) = prost::encoding::decode_key(&mut rest).expect("the key decodes");
    assert_eq!((tag, wire_type), (1, WireType::LengthDelimited));

    assert_eq!(roundtrip(&response), response);
}

/// The `ProtoField` impls no API field instantiates: every `Option` field in the schema holds a
/// message, and no field is a `double` or a `uint64`.
#[test]
fn the_leaf_impls_no_api_field_reaches() {
    fn roundtrip_field<T: ProtoField + PartialEq + std::fmt::Debug>(value: T) {
        let mut buf = Vec::new();
        T::encode_field(1, &value, &mut buf);
        assert_eq!(T::encoded_len_field(1, &value), buf.len());

        let mut rest = buf.as_slice();
        let mut decoded = T::default();
        if !rest.is_empty() {
            let (tag, wire_type) = prost::encoding::decode_key(&mut rest).expect("the key decodes");
            assert_eq!(tag, 1);
            T::merge_field(wire_type, &mut decoded, &mut rest, DecodeContext::default())
                .expect("the value merges");
        }
        assert_eq!(decoded, value);
    }

    roundtrip_field(1.5f64);
    roundtrip_field(u64::MAX);
    // The negative zero survives, which a `==` zero test in `is_zero` would not do: it would leave
    // the field off the wire, and an absent float reads back as `+0.0`. One case per codec, since
    // each has its own override, and the differential harness reaches neither: no field of this
    // schema is a float or a double.
    let mut buf = Vec::new();
    <f64 as ProtoField>::encode_implicit(1, &-0.0f64, &mut buf);
    assert!(!buf.is_empty(), "a negative zero double is written out");
    roundtrip_field(-0.0f64);

    let mut buf = Vec::new();
    <f32 as ProtoField>::encode_implicit(1, &-0.0f32, &mut buf);
    assert!(!buf.is_empty(), "a negative zero float is written out");
    roundtrip_field(-0.0f32);

    let mut buf = Vec::new();
    <f64 as ProtoField>::encode_implicit(1, &0.0, &mut buf);
    assert!(buf.is_empty(), "a positive zero is not");
    // `None` writes nothing, so nothing is what decodes back to it.
    roundtrip_field(Option::<u64>::None);
    roundtrip_field(Some(7u64));
}

/// Fields go out in ascending tag order, including across a whole-message oneof's members and the
/// non-oneof fields that surround them.
///
/// The harness compares `DynamicMessage`s after `Normalize`, so it is blind to order.
/// `agent::create_tasks::Request` is the shape that exercises this: a oneof at tags 1 to 3 plus a
/// `communication_token` sibling at tag 4.
#[test]
fn fields_are_emitted_in_ascending_tag_order() {
    let request = crate::agent::create_tasks::Request::InitRequest {
        communication_token: String::from("tok"),
        request: crate::agent::create_tasks::InitRequest { task_options: None },
    };
    let bytes = request.encode_to_vec();

    let mut rest = bytes.as_slice();
    let mut tags = Vec::new();
    while !rest.is_empty() {
        let (tag, wire_type) = prost::encoding::decode_key(&mut rest).expect("the key decodes");
        tags.push(tag);
        prost::encoding::skip_field(wire_type, tag, &mut rest, DecodeContext::default())
            .expect("the field is skippable");
    }
    // The member at tag 1, then the sibling at tag 4: the member's own tag places it, not the fact
    // that it is the member.
    assert_eq!(tags, [1, 4]);
    assert!(tags.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(roundtrip(&request), request);
}

/// A map entry holding the zero key and the zero value is written out whole, and reads back as
/// itself.
///
/// prost's `hash_map` skips a subfield equal to its default, so it writes `{"": ""}` as an empty
/// entry. Both forms decode to the same map, which is why the harness cannot see the difference.
#[test]
fn a_map_entry_of_defaults_is_written_out() {
    use std::collections::HashMap;

    let mut values = HashMap::new();
    values.insert(String::new(), String::new());
    let mut buf = Vec::new();
    <HashMap<String, String> as ProtoField>::encode_field(1, &values, &mut buf);
    assert_eq!(
        <HashMap<String, String> as ProtoField>::encoded_len_field(1, &values),
        buf.len(),
    );
    // key + length, then the two zero-length subfields: not the empty entry prost would write.
    assert_eq!(buf, [0x0a, 0x04, 0x0a, 0x00, 0x12, 0x00]);

    let mut rest = buf.as_slice();
    let (tag, wire_type) = prost::encoding::decode_key(&mut rest).expect("the key decodes");
    assert_eq!(tag, 1);
    let mut decoded = HashMap::new();
    <HashMap<String, String> as ProtoField>::merge_field(
        wire_type,
        &mut decoded,
        &mut rest,
        DecodeContext::default(),
    )
    .expect("the entry merges");
    assert_eq!(decoded, values);

    // And prost's form still decodes to the same thing.
    let empty_entry: &[u8] = &[0x0a, 0x00];
    let mut rest = empty_entry;
    let (_, wire_type) = prost::encoding::decode_key(&mut rest).expect("the key decodes");
    let mut decoded = HashMap::new();
    <HashMap<String, String> as ProtoField>::merge_field(
        wire_type,
        &mut decoded,
        &mut rest,
        DecodeContext::default(),
    )
    .expect("an entry omitting both subfields merges");
    assert_eq!(decoded, values);
}

/// The `PairMap` delegation must keep rejecting a mis-typed field key: the wire-type check lives in
/// prost's map codec, and forwarding to it is the whole implementation.
#[test]
fn pair_map_rejects_non_delimited_wire_type() {
    use std::collections::HashMap;

    use super::adapters::PairMap;
    use super::ProtoAdapter;

    let mut buf = Vec::new();
    buf.put_u8(0);

    let mut map = HashMap::<String, String>::new();
    let err = <PairMap as ProtoAdapter<HashMap<String, String>>>::merge_field(
        WireType::Varint,
        &mut map,
        &mut buf.as_slice(),
        DecodeContext::default(),
    )
    .expect_err("a varint where a length-delimited entry belongs is a decode error");
    assert!(format!("{err}").contains("invalid wire type"), "{err}");
}

/// A `#[armonik(present)]` marker stands for an `Empty` member, so it is a message field on the
/// wire: only a length-delimited body sets it. Presence alone carries the value, which is exactly
/// what makes it tempting to accept any spelling of it.
#[test]
fn a_present_marker_rejects_a_non_delimited_wire_type() {
    /// `Output.ok`, an `Empty`.
    const OK_TAG: u32 = 2;

    let mut varint = Vec::new();
    prost::encoding::encode_key(OK_TAG, WireType::Varint, &mut varint);
    prost::encoding::encode_varint(1, &mut varint);
    let err = crate::Output::decode(varint.as_slice())
        .expect_err("a varint where an `Empty` member belongs is a decode error");
    assert!(format!("{err}").contains("invalid wire type"), "{err}");

    let mut delimited = Vec::new();
    prost::encoding::encode_key(OK_TAG, WireType::LengthDelimited, &mut delimited);
    prost::encoding::encode_varint(0, &mut delimited);
    assert_eq!(
        crate::Output::decode(delimited.as_slice()).expect("an empty body sets the marker"),
        crate::Output::Ok,
    );
}

/// The empty list is unchecked, which is what makes the salvage stub's marker safe: a type whose
/// expansion failed already has a `compile_error!` next to it, and this assert firing too would be
/// the cascade the stub exists to prevent.
#[test]
fn an_empty_oneof_list_is_unchecked() {
    use super::oneof_matches;

    assert!(oneof_matches(&[], "anything.at.all"));
    assert!(oneof_matches(&["a.b.c"], "a.b.c"));
    assert!(!oneof_matches(&["a.b.c"], "a.b.d"));
    // Several, as a unified type declares.
    assert!(oneof_matches(&["a.b.c", "x.y.z"], "x.y.z"));
    // A prefix is not a match: `names_contain` compares lengths first.
    assert!(!oneof_matches(&["a.b.condition"], "a.b.cond"));
}
