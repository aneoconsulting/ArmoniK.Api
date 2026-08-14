//! [`ProtoAdapter`] implementations for fields whose Rust representation
//! differs structurally from their proto counterpart
//! (`#[armonik(with = "...")]`).

use std::collections::HashMap;
use std::hash::Hash;

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use super::{ProtoAdapter, ProtoField};

/// `repeated Pair { key = KT; value = VT }` exposed as a `HashMap`.
///
/// Entry order is not preserved and duplicate keys collapse (last wins),
/// exactly like the historical conversions.
///
/// The three wire methods are pure forwards to the [`HashMap`] `ProtoField`
/// implementation and change no bytes: that codec frames entries as tags 1 and
/// 2, which is exactly what the pair messages use. What this adapter is
/// actually for is suppressing the shape assert (the proto side is a repeated
/// message, the Rust side a map), carrying `normalize_dynamic`, and hosting
/// `absorbs`.
///
/// Key and value tags are hardcoded rather than parameters: any other pair would need the framing
/// hand-rolled again, and a `PairMap<KT, VT>` spelling only ever produced unsatisfied-bound errors
/// pointing into expanded tokens.
pub(crate) struct PairMap;

impl<K, V> ProtoAdapter<HashMap<K, V>> for PairMap
where
    K: ProtoField + Eq + Hash,
    V: ProtoField,
{
    fn encode_field(tag: u32, value: &HashMap<K, V>, buf: &mut impl BufMut) {
        <HashMap<K, V> as ProtoField>::encode_field(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut HashMap<K, V>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        <HashMap<K, V> as ProtoField>::merge_field(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &HashMap<K, V>) -> usize {
        <HashMap<K, V> as ProtoField>::encoded_len_field(tag, value)
    }

    /// The `HashMap` loses entry order and collapses duplicate keys.
    #[cfg(test)]
    fn normalize_dynamic(message: &mut ::prost_reflect::DynamicMessage, tag: u32) {
        crate::differential::fold_pairs_by_tag(message, tag, 1);
    }
}

/// `Wrapper { V inner = 1 }` exposed as the bare `V` (a `String` or a
/// `Vec<T>`): the single-field wrapper message is flattened away.
///
/// The inner tag is hardcoded, not a parameter. It read `Wrapper<TAG>` while
/// all ten of its sites were `Wrapper`, which advertised a generality
/// nothing exercised; `PairMap`'s const generics were collapsed for the same
/// reason, one file over. A wrapper at another tag is a one-line change here
/// and a `Wrapper<N>` again the day two of them coexist.
pub(crate) struct Wrapper;

/// The tag the wrapper message carries its single field at.
const TAG: u32 = 1;

impl<V: ProtoField> ProtoAdapter<V> for Wrapper {
    fn encode_field(tag: u32, value: &V, buf: &mut impl BufMut) {
        let body = V::encoded_len_field(TAG, value);
        encoding::encode_key(tag, WireType::LengthDelimited, buf);
        encoding::encode_varint(body as u64, buf);
        V::encode_field(TAG, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut V,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        encoding::merge_loop(value, buf, ctx, |value, buf, ctx| {
            let (tag, wire_type) = encoding::decode_key(buf)?;
            if tag == TAG {
                V::merge_field(wire_type, value, buf, ctx)
            } else {
                encoding::skip_field(wire_type, tag, buf, ctx)
            }
        })
    }

    fn encoded_len_field(tag: u32, value: &V) -> usize {
        let body = V::encoded_len_field(TAG, value);
        encoding::key_len(tag) + encoding::encoded_len_varint(body as u64) + body
    }
}
