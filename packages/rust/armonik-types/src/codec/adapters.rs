//! [`ProtoAdapter`] implementations for fields whose Rust representation
//! differs structurally from their proto counterpart
//! (`#[armonik(with = "...")]`).

use std::collections::HashMap;
use std::hash::Hash;

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use super::{key_len, ProtoAdapter, ProtoField};

/// `repeated Pair { key = KT; value = VT }` exposed as a `HashMap`.
///
/// Entry order is not preserved and duplicate keys collapse (last wins),
/// exactly like the historical conversions.
pub(crate) struct PairMap<const KT: u32 = 1, const VT: u32 = 2>;

impl<K, V, const KT: u32, const VT: u32> ProtoAdapter<HashMap<K, V>> for PairMap<KT, VT>
where
    K: ProtoField + Eq + Hash,
    V: ProtoField,
{
    fn encode_field(tag: u32, value: &HashMap<K, V>, buf: &mut impl BufMut) {
        for (key, entry_value) in value {
            let mut entry_len = 0;
            if !K::is_default(key) {
                entry_len += K::encoded_len_field(KT, key);
            }
            if !V::is_default(entry_value) {
                entry_len += V::encoded_len_field(VT, entry_value);
            }
            encoding::encode_key(tag, WireType::LengthDelimited, buf);
            encoding::encode_varint(entry_len as u64, buf);
            if !K::is_default(key) {
                K::encode_field(KT, key, buf);
            }
            if !V::is_default(entry_value) {
                V::encode_field(VT, entry_value, buf);
            }
        }
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut HashMap<K, V>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut entry = super::read_delimited(buf)?;
        let mut key = K::default();
        let mut entry_value = V::default();
        while entry.has_remaining() {
            let (tag, wire_type) = encoding::decode_key(&mut entry)?;
            if tag == KT {
                K::merge_field(wire_type, &mut key, &mut entry, ctx.clone())?;
            } else if tag == VT {
                V::merge_field(wire_type, &mut entry_value, &mut entry, ctx.clone())?;
            } else {
                encoding::skip_field(wire_type, tag, &mut entry, ctx.clone())?;
            }
        }
        value.insert(key, entry_value);
        Ok(())
    }

    fn encoded_len_field(tag: u32, value: &HashMap<K, V>) -> usize {
        value
            .iter()
            .map(|(key, entry_value)| {
                let mut entry_len = 0;
                if !K::is_default(key) {
                    entry_len += K::encoded_len_field(KT, key);
                }
                if !V::is_default(entry_value) {
                    entry_len += V::encoded_len_field(VT, entry_value);
                }
                key_len(tag) + encoding::encoded_len_varint(entry_len as u64) + entry_len
            })
            .sum()
    }

    fn is_default(value: &HashMap<K, V>) -> bool {
        value.is_empty()
    }

    fn clear_field(value: &mut HashMap<K, V>) {
        value.clear();
    }

    /// The `HashMap` loses entry order and collapses duplicate keys.
    #[cfg(feature = "_differential")]
    fn normalize_dynamic(
        message: &mut crate::differential::prost_reflect::DynamicMessage,
        tag: u32,
    ) {
        crate::differential::fold_pairs_by_tag(message, tag, KT);
    }
}

/// `Wrapper { string inner = TAG }` exposed as a `String`.
pub(crate) struct StringWrapper<const TAG: u32 = 1>;

impl<const TAG: u32> ProtoAdapter<String> for StringWrapper<TAG> {
    fn encode_field(tag: u32, value: &String, buf: &mut impl BufMut) {
        let body = if <String as ProtoField>::is_default(value) {
            0
        } else {
            String::encoded_len_field(TAG, value)
        };
        encoding::encode_key(tag, WireType::LengthDelimited, buf);
        encoding::encode_varint(body as u64, buf);
        if body != 0 {
            String::encode_field(TAG, value, buf);
        }
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut String,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut wrapper = super::read_delimited(buf)?;
        while wrapper.has_remaining() {
            let (tag, wire_type) = encoding::decode_key(&mut wrapper)?;
            if tag == TAG {
                String::merge_field(wire_type, value, &mut wrapper, ctx.clone())?;
            } else {
                encoding::skip_field(wire_type, tag, &mut wrapper, ctx.clone())?;
            }
        }
        Ok(())
    }

    fn encoded_len_field(tag: u32, value: &String) -> usize {
        let body = if <String as ProtoField>::is_default(value) {
            0
        } else {
            String::encoded_len_field(TAG, value)
        };
        key_len(tag) + encoding::encoded_len_varint(body as u64) + body
    }

    /// The wrapper itself carries oneof presence in its uses; an empty one
    /// still encodes (as an empty message).
    fn is_default(value: &String) -> bool {
        let _ = value;
        false
    }

    fn clear_field(value: &mut String) {
        value.clear();
    }
}

/// `Wrapper { repeated T inner = TAG }` exposed as a `Vec<T>`.
pub(crate) struct VecWrapper<const TAG: u32 = 1>;

impl<T: ProtoField, const TAG: u32> ProtoAdapter<Vec<T>> for VecWrapper<TAG> {
    fn encode_field(tag: u32, value: &Vec<T>, buf: &mut impl BufMut) {
        let body = T::encoded_len_repeated(TAG, value);
        encoding::encode_key(tag, WireType::LengthDelimited, buf);
        encoding::encode_varint(body as u64, buf);
        T::encode_repeated(TAG, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Vec<T>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut wrapper = super::read_delimited(buf)?;
        while wrapper.has_remaining() {
            let (tag, wire_type) = encoding::decode_key(&mut wrapper)?;
            if tag == TAG {
                T::merge_repeated(wire_type, value, &mut wrapper, ctx.clone())?;
            } else {
                encoding::skip_field(wire_type, tag, &mut wrapper, ctx.clone())?;
            }
        }
        Ok(())
    }

    fn encoded_len_field(tag: u32, value: &Vec<T>) -> usize {
        let body = T::encoded_len_repeated(TAG, value);
        key_len(tag) + encoding::encoded_len_varint(body as u64) + body
    }

    /// The wrapper itself carries oneof presence in its uses; an empty one
    /// still encodes (as an empty message).
    fn is_default(value: &Vec<T>) -> bool {
        let _ = value;
        false
    }

    fn clear_field(value: &mut Vec<T>) {
        value.clear();
    }
}
