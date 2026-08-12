//! [`ProtoField`] implementations for `Option<T>`, `Vec<T>` and
//! `HashMap<K, V>`.

use std::collections::HashMap;
use std::hash::Hash;

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use super::{Cardinality, FieldKind, ProtoField, Shape};

/// Explicit presence: `None` is absent, `Some` is encoded unconditionally
/// (even when equal to the default).
impl<T: ProtoField> ProtoField for Option<T> {
    const SHAPE: Shape = Shape {
        cardinality: Cardinality::Optional,
        ..T::SHAPE
    };

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if let Some(value) = value {
            T::encode_field(tag, value, buf);
        }
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        T::merge_field(wire_type, value.get_or_insert_with(T::default), buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        value
            .as_ref()
            .map_or(0, |value| T::encoded_len_field(tag, value))
    }
}

impl<T: ProtoField> ProtoField for Vec<T> {
    const SHAPE: Shape = Shape {
        cardinality: Cardinality::Repeated,
        ..T::SHAPE
    };

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        T::encode_repeated(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        T::merge_repeated(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        T::encoded_len_repeated(tag, value)
    }
}

/// Map fields keep prost's map codec, which omits `== default` key and value
/// subfields inside each entry (the canonical map-entry encoding, and where the
/// `PartialEq` bound on the value comes from). Decoders fill those subfields
/// back in from the defaults, so nothing is lost; a `#[armonik(with)]` adapter
/// over pair messages inherits the same framing (see
/// [`PairMap`](super::adapters::PairMap)).
/// A proto `map<K, V>`, which is a repeated `{ K key = 1; V value = 2; }` entry message on the
/// wire.
///
/// Hand-written rather than delegated to `prost::encoding::hash_map`, for one reason: prost's
/// version skips a key or value equal to its default, and this crate's encode side skips nothing
/// (see the module docs). Delegating made map entries the single exception to that rule, which cost
/// a `V: PartialEq` bound to express and a line in DESIGN to record. Both are gone with it.
///
/// A receiver reads the two forms identically: proto3 seeds an absent implicit-presence field from
/// its default, which is exactly what the skipped subfield held.
impl<K, V> ProtoField for HashMap<K, V>
where
    K: ProtoField + Eq + Hash,
    V: ProtoField,
{
    const SHAPE: Shape = Shape {
        kind: FieldKind::Message,
        cardinality: Cardinality::Map,
        names: V::SHAPE.names,
        map: Some((K::SHAPE.kind, V::SHAPE.kind)),
    };

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        for (key, value) in value {
            let entry_len = entry_len::<K, V>(key, value);
            encoding::encode_key(tag, WireType::LengthDelimited, buf);
            encoding::encode_varint(entry_len as u64, buf);
            K::encode_field(KEY_TAG, key, buf);
            V::encode_field(VALUE_TAG, value, buf);
        }
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        // An entry omitting either subfield is legal and means its default, which is what the pair
        // is seeded with.
        let mut entry = (K::default(), V::default());
        encoding::merge_loop(&mut entry, buf, ctx, |entry, buf, ctx| {
            let (tag, wire_type) = encoding::decode_key(buf)?;
            match tag {
                KEY_TAG => K::merge_field(wire_type, &mut entry.0, buf, ctx),
                VALUE_TAG => V::merge_field(wire_type, &mut entry.1, buf, ctx),
                _ => encoding::skip_field(wire_type, tag, buf, ctx),
            }
        })?;
        value.insert(entry.0, entry.1);
        Ok(())
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        value
            .iter()
            .map(|(key, value)| {
                let entry_len = entry_len::<K, V>(key, value);
                encoding::key_len(tag) + encoding::encoded_len_varint(entry_len as u64) + entry_len
            })
            .sum()
    }
}

/// The tags a proto map entry's key and value always carry.
const KEY_TAG: u32 = 1;
const VALUE_TAG: u32 = 2;

fn entry_len<K: ProtoField, V: ProtoField>(key: &K, value: &V) -> usize {
    K::encoded_len_field(KEY_TAG, key) + V::encoded_len_field(VALUE_TAG, value)
}
