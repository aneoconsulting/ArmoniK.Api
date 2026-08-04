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
/// The wire methods delegate to the [`HashMap`] `ProtoField` implementation
/// (prost's real-map codec), which hardcodes entry tags 1/2, the same tags
/// the pair messages use. The implementation therefore only exists for
/// `PairMap<1, 2>` (every use); other tag pairs would need the hand-rolled
/// framing back. prost's map codec skips `== default` key/value subfields,
/// which the pair messages read back as the same values (the zero-default
/// invariant); the differential harness guards it.
pub(crate) struct PairMap<const KT: u32, const VT: u32>;

impl<K, V> ProtoAdapter<HashMap<K, V>> for PairMap<1, 2>
where
    K: ProtoField + Eq + Hash + Ord,
    V: ProtoField + PartialEq,
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
    #[cfg(feature = "_differential")]
    fn normalize_dynamic(
        message: &mut crate::differential::prost_reflect::DynamicMessage,
        tag: u32,
    ) {
        crate::differential::fold_pairs_by_tag(message, tag, 1);
    }
}

/// `Wrapper { V inner = TAG }` exposed as the bare `V` (a `String` or a
/// `Vec<T>`): the single-field wrapper message is flattened away.
pub(crate) struct Wrapper<const TAG: u32>;

impl<V: ProtoField, const TAG: u32> ProtoAdapter<V> for Wrapper<TAG> {
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
        let mut wrapper = super::read_delimited(buf)?;
        while wrapper.has_remaining() {
            let (tag, wire_type) = encoding::decode_key(&mut wrapper)?;
            if tag == TAG {
                V::merge_field(wire_type, value, &mut wrapper, ctx.clone())?;
            } else {
                encoding::skip_field(wire_type, tag, &mut wrapper, ctx.clone())?;
            }
        }
        Ok(())
    }

    fn encoded_len_field(tag: u32, value: &V) -> usize {
        let body = V::encoded_len_field(TAG, value);
        encoding::key_len(tag) + encoding::encoded_len_varint(body as u64) + body
    }
}
