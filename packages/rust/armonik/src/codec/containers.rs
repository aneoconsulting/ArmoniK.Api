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
impl<K, V> ProtoField for HashMap<K, V>
where
    K: ProtoField + Eq + Hash + Ord,
    V: ProtoField + PartialEq,
{
    const SHAPE: Shape = Shape {
        kind: FieldKind::Message,
        cardinality: Cardinality::Map,
        names: V::SHAPE.names,
        map: Some((K::SHAPE.kind, V::SHAPE.kind)),
    };

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        encoding::hash_map::encode(
            |tag, key, buf| K::encode_field(tag, key, buf),
            |tag, key| K::encoded_len_field(tag, key),
            |tag, value, buf| V::encode_field(tag, value, buf),
            |tag, value| V::encoded_len_field(tag, value),
            tag,
            value,
            buf,
        );
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        encoding::hash_map::merge(
            |wire_type, key: &mut K, buf: &mut _, ctx| K::merge_field(wire_type, key, buf, ctx),
            |wire_type, value: &mut V, buf: &mut _, ctx| V::merge_field(wire_type, value, buf, ctx),
            value,
            buf,
            ctx,
        )
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        encoding::hash_map::encoded_len(
            |tag, key| K::encoded_len_field(tag, key),
            |tag, value| V::encoded_len_field(tag, value),
            tag,
            value,
        )
    }
}
