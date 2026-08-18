//! Byte-level mutations of the wire form the harness feeds each type, for the shapes a peer is
//! allowed to produce and this schema never does.
//!
//! [`super::harness::registered_types_roundtrip`] encodes each generated message the way
//! `prost-reflect` writes it: declared fields only, ascending tags, repeated scalars packed, no
//! duplicates. Every one of those is an encoder's choice, not a rule, and a peer that chooses
//! differently is still writing valid protobuf. This re-runs the same round-trip over bytes that
//! make the other choices.
//!
//! The assertion is the harness's own and it never changes: the mutated bytes decode, and what
//! comes back equals the **unmutated** original. Not "the mutation survives" — no armonik type
//! preserves unknown fields (every emitted `merge_field` ends in `skip_field`, and no type has an
//! unknown-field member), so surviving would be the bug.
//!
//! Counterfactual, measured: replacing that `skip_field` with `Ok(())` in `shape/plain.rs` leaves
//! the suite green without this layer, and fails most of its cases with it.

use prost::Message;
use prost_reflect::{DynamicMessage, Kind, MessageDescriptor};

use super::{arbitrary, compare, harness, registry, rng};

/// Tags the mutations file their synthetic unknown fields under.
///
/// A fixed block rather than `max(declared) + 1` per message, because a transparent wrapper
/// delegates every tag to its inner type: "unknown here" has to mean unknown all the way down.
/// `harness::descriptor_coverage_ratchet` keeps the schema's own tags below it.
pub(super) const UNKNOWN_TAGS: std::ops::RangeInclusive<u32> = 1000..=1005;

/// One top-level record of an encoded message: its tag, its wire type, and the bytes it occupies,
/// key included.
struct Record<'a> {
    tag: u32,
    wire_type: u8,
    bytes: &'a [u8],
}

/// Split an encoded message into its top-level records, or `None` if the bytes are not a
/// well-formed sequence of them.
fn split(bytes: &[u8]) -> Option<Vec<Record<'_>>> {
    let mut records = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let key = read_varint(bytes, &mut at)?;
        let tag = u32::try_from(key >> 3).ok()?;
        let wire_type = u8::try_from(key & 7).ok()?;
        match wire_type {
            0 => {
                read_varint(bytes, &mut at)?;
            }
            1 => at = at.checked_add(8)?,
            2 => {
                let len = usize::try_from(read_varint(bytes, &mut at)?).ok()?;
                at = at.checked_add(len)?;
            }
            5 => at = at.checked_add(4)?,
            // Groups: no armonik field uses them, and the generator never writes one, so a group
            // at top level would be a bug in this module rather than a case to split.
            _ => return None,
        }
        if at > bytes.len() {
            return None;
        }
        records.push(Record {
            tag,
            wire_type,
            bytes: &bytes[start..at],
        });
    }
    Some(records)
}

fn read_varint(bytes: &[u8], at: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in 0..10 {
        let byte = *bytes.get(*at)?;
        *at += 1;
        value |= u64::from(byte & 0x7f) << (shift * 7);
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

fn key(tag: u32, wire_type: u8) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(u64::from(tag) << 3 | u64::from(wire_type), &mut out);
    out
}

fn write_varint(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// One synthetic unknown field, whole. `which` picks the wire type, and with it the tag.
fn unknown_field(which: usize) -> Vec<u8> {
    let tag = UNKNOWN_TAGS.start() + which as u32;
    let mut out = Vec::new();
    match which {
        0 => {
            out.extend(key(tag, 0));
            write_varint(0x7fff_ffff_ffff_ffff, &mut out);
        }
        1 => {
            out.extend(key(tag, 1));
            out.extend([1, 2, 3, 4, 5, 6, 7, 8]);
        }
        2 => {
            out.extend(key(tag, 2));
            write_varint(5, &mut out);
            out.extend(b"bytes");
        }
        3 => {
            // A group: the one wire type no armonik field uses, so its handling is pure fallback.
            // `skip_field` has to find the matching end key and everything nested between them.
            out.extend(key(tag, 3));
            out.extend(key(tag + 100, 0));
            write_varint(42, &mut out);
            out.extend(key(tag, 4));
        }
        _ => {
            out.extend(key(tag, 5));
            out.extend([1, 2, 3, 4]);
        }
    }
    out
}

/// How many distinct unknown-field cases [`with_unknown_fields`] can produce.
const UNKNOWN_CASES: usize = 5;

/// The original, with one synthetic unknown field interleaved before every declared record and
/// after the last: a peer on a newer schema writes its fields wherever their tags fall, not in a
/// block at the end.
fn with_unknown_fields(records: &[Record<'_>], which: usize) -> Vec<u8> {
    let filler = unknown_field(which);
    let mut out = Vec::new();
    for record in records {
        out.extend_from_slice(&filler);
        out.extend_from_slice(record.bytes);
    }
    out.extend_from_slice(&filler);
    out
}

/// The original with its records in descending tag order.
///
/// Stably within a tag, which is mandatory rather than tidy: records sharing a tag are the elements
/// of a repeated field, and reversing them reverses the field, which `compare::value_equal`
/// compares positionally. This mutation is about field order, not element order.
///
/// The mirror of `codec::tests::fields_are_emitted_in_ascending_tag_order`, which pins what the
/// crate writes; this pins what it accepts.
fn descending(records: &[Record<'_>]) -> Vec<u8> {
    let mut ordered: Vec<&Record<'_>> = records.iter().collect();
    ordered.sort_by_key(|record| std::cmp::Reverse(record.tag));
    ordered
        .iter()
        .flat_map(|record| record.bytes.iter().copied())
        .collect()
}

/// The original with every packed repeated field spread into one record per element, which is the
/// other encoding proto3 allows and requires every reader to accept.
///
/// Generalises `codec::tests::unpacked_repeated_enums_are_accepted` from one hand-picked field to
/// every packable field of every registered message, inside real messages rather than a fixture.
fn unpacked(desc: &MessageDescriptor, records: &[Record<'_>]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut spread = false;
    for record in records {
        let Some(wire_type) = desc
            .get_field(record.tag)
            .filter(|field| record.wire_type == 2 && field.is_list())
            .and_then(|field| element_wire_type(&field.kind()))
        else {
            out.extend_from_slice(record.bytes);
            continue;
        };
        // Past the key and the length prefix, the payload is the elements back to back.
        let mut at = 0;
        read_varint(record.bytes, &mut at)?;
        let len = usize::try_from(read_varint(record.bytes, &mut at)?).ok()?;
        let payload = record.bytes.get(at..at + len)?;
        let mut element = 0;
        while element < payload.len() {
            let start = element;
            match wire_type {
                0 => {
                    read_varint(payload, &mut element)?;
                }
                1 => element += 8,
                _ => element += 4,
            }
            out.extend(key(record.tag, wire_type));
            out.extend_from_slice(payload.get(start..element)?);
            spread = true;
        }
    }
    spread.then_some(out)
}

/// The wire type each element of a packed repeated field takes once spelled one key per element, or
/// `None` for a kind that is never packed. The fixed-width kinds keep their width: re-spelling them
/// as varints would emit a different value under the same tag.
fn element_wire_type(kind: &Kind) -> Option<u8> {
    match kind {
        Kind::String | Kind::Bytes | Kind::Message(_) => None,
        Kind::Double | Kind::Fixed64 | Kind::Sfixed64 => Some(1),
        Kind::Float | Kind::Fixed32 | Kind::Sfixed32 => Some(5),
        _ => Some(0),
    }
}

/// The original with every singular field written twice, which protobuf resolves and this schema
/// never produces.
///
/// Two flavours, because the rule differs by kind: a repeated occurrence of a non-message field is
/// last-wins, so the duplicate is the record verbatim; repeated occurrences of a message field
/// *merge*, so the duplicate is the same tag with an empty body. Both are no-ops by construction,
/// which is what makes the harness's own assertion the right one here too.
fn duplicated(desc: &MessageDescriptor, records: &[Record<'_>]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut duplicated = false;
    for record in records {
        out.extend_from_slice(record.bytes);
        let Some(field) = desc.get_field(record.tag) else {
            continue;
        };
        if field.is_list() || field.is_map() {
            continue;
        }
        match field.kind() {
            Kind::Message(_) => {
                out.extend(key(record.tag, 2));
                write_varint(0, &mut out);
            }
            _ => out.extend_from_slice(record.bytes),
        }
        duplicated = true;
    }
    duplicated.then_some(out)
}

/// Every mutation of one encoded message, each with the name its failure is reported under.
///
/// The unknown-field wire type rotates with the iteration rather than all five running on each:
/// every wire type still lands on every type about a dozen times over `ITERATIONS`, and the five
/// were four fifths of the layer's whole cost.
fn cases(desc: &MessageDescriptor, bytes: &[u8], iteration: u64) -> Vec<(String, Vec<u8>)> {
    let Some(records) = split(bytes) else {
        panic!(
            "the generated encoding of `{}` does not split",
            desc.full_name()
        );
    };
    let mut cases = Vec::new();
    let which = (iteration % UNKNOWN_CASES as u64) as usize;
    let tag = UNKNOWN_TAGS.start() + which as u32;
    cases.push((
        format!("an unknown field at tag {tag}"),
        with_unknown_fields(&records, which),
    ));
    cases.push((String::from("descending tag order"), descending(&records)));
    if let Some(mutated) = unpacked(desc, &records) {
        cases.push((String::from("unpacked repeated fields"), mutated));
    }
    if let Some(mutated) = duplicated(desc, &records) {
        cases.push((String::from("duplicated singular fields"), mutated));
    }
    cases
}

/// This schema has no repeated field of a fixed-width kind, so `unpacked` never spreads one and no
/// message can show a wrong width. The mapping is pinned here instead.
#[test]
fn packed_elements_keep_their_width() {
    assert_eq!(element_wire_type(&Kind::Int32), Some(0));
    assert_eq!(element_wire_type(&Kind::Bool), Some(0));
    assert_eq!(element_wire_type(&Kind::Double), Some(1));
    assert_eq!(element_wire_type(&Kind::Sfixed64), Some(1));
    assert_eq!(element_wire_type(&Kind::Float), Some(5));
    assert_eq!(element_wire_type(&Kind::Fixed32), Some(5));
    assert_eq!(element_wire_type(&Kind::String), None);
    assert_eq!(element_wire_type(&Kind::Bytes), None);
}

#[test]
fn mutated_encodings_decode_to_the_same_value() {
    let pool = harness::pool();
    for (proto, hooks) in registry::entries() {
        let desc = pool
            .get_message_by_name(proto)
            .unwrap_or_else(|| panic!("registry entry `{}` is not in the descriptor", proto));
        for iteration in 0..harness::ITERATIONS {
            let seed = rng::seed(proto, iteration);
            let mut rng = rng::SplitMix64::new(seed);
            let mut original = arbitrary::message(&desc, &mut rng, harness::RECURSION_DEPTH);
            let bytes = original.encode_to_vec();
            registry::normalize(&mut original);

            for (case, mutated) in cases(&desc, &bytes, iteration) {
                let reencoded = (hooks.roundtrip)(&mutated).unwrap_or_else(|err| {
                    panic!(
                        "`{proto}` fails to decode {case} (seed {seed:#018x}): {err}\n\
                         original: {}",
                        harness::debug_fields(&original),
                    )
                });
                let mut back = DynamicMessage::decode(desc.clone(), reencoded.as_slice())
                    .unwrap_or_else(|err| {
                        panic!(
                            "`{proto}` re-encodes {case} into bytes that do not decode \
                             (seed {seed:#018x}): {err}"
                        )
                    });
                registry::normalize(&mut back);

                assert!(
                    compare::messages(&original, &back),
                    "`{proto}` reads {case} as a different value (seed {seed:#018x})\n\
                     original:   {}\n\
                     round-trip: {}",
                    harness::debug_fields(&original),
                    harness::debug_fields(&back),
                );
            }
        }
    }
}
