//! [`ProtoField`] implementations for `Option<T>`, `Vec<T>` and
//! `HashMap<K, V>`.

use std::collections::HashMap;
use std::hash::Hash;

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use super::{Cardinality, FieldKind, ProtoField};

/// Explicit presence: `None` is absent, `Some` is encoded unconditionally
/// (even when equal to the default).
impl<T: ProtoField> ProtoField for Option<T> {
    const KIND: FieldKind = T::KIND;
    const CARDINALITY: Cardinality = Cardinality::Optional;
    const NAMES: &'static [&'static str] = T::NAMES;

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

    fn is_default(value: &Self) -> bool {
        value.is_none()
    }
}

impl<T: ProtoField> ProtoField for Vec<T> {
    const KIND: FieldKind = T::KIND;
    const CARDINALITY: Cardinality = Cardinality::Repeated;
    const NAMES: &'static [&'static str] = T::NAMES;

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

    fn is_default(value: &Self) -> bool {
        value.is_empty()
    }

    fn clear_field(value: &mut Self) {
        value.clear();
    }
}

impl<K, V> ProtoField for HashMap<K, V>
where
    K: ProtoField + Eq + Hash + Ord,
    V: ProtoField + PartialEq,
{
    const KIND: FieldKind = FieldKind::Message;
    const CARDINALITY: Cardinality = Cardinality::Map;
    const NAMES: &'static [&'static str] = V::NAMES;
    const MAP_KEY_KIND: FieldKind = K::KIND;
    const MAP_VALUE_KIND: FieldKind = V::KIND;

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

    fn is_default(value: &Self) -> bool {
        value.is_empty()
    }

    fn clear_field(value: &mut Self) {
        value.clear();
    }
}
