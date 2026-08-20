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
/// actually for is carrying `normalize_dynamic` and the entry framing; the shape check is the
/// resolver's, synthesized from the pair's two fields.
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

/// `Wrapper { V inner = TAG }` exposed as the bare `V`: one length-delimited
/// framing layer around the codec `A`, which is how single-field wrapper
/// messages are flattened away. `#[armonik(inlined)]` emits `Wrapper<Own, N>`
/// with the tag read from the descriptor; a transparent enumeration composes
/// `Wrapper<..Wrapper<EnumLeaf, ..>..>` down its chain.
pub(crate) struct Wrapper<A, const TAG: u32>(::core::marker::PhantomData<A>);

/// The value's own [`ProtoField`], as a codec type: the bottom of a wrapper chain, and what
/// [`Wrapper`] wraps unless told otherwise.
pub(crate) struct Own;

impl<V: ProtoField> ProtoAdapter<V> for Own {
    fn encode_field(tag: u32, value: &V, buf: &mut impl BufMut) {
        V::encode_field(tag, value, buf)
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut V,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        V::merge_field(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &V) -> usize {
        V::encoded_len_field(tag, value)
    }
}

/// A `#[armonik(present)]` `bool` member, whose value is its own presence: encoding writes `true`,
/// and decoding consumes the bool *whatever it holds*, so an explicit `false` on the wire still
/// selects the variant. The value type is `()` because there is nothing to hold.
pub(crate) struct BoolPresence;

impl ProtoAdapter<()> for BoolPresence {
    fn encode_field(tag: u32, (): &(), buf: &mut impl BufMut) {
        <bool as ProtoField>::encode_field(tag, &true, buf);
    }

    fn merge_field(
        wire_type: WireType,
        (): &mut (),
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let mut marker = false;
        <bool as ProtoField>::merge_field(wire_type, &mut marker, buf, ctx)
    }

    fn encoded_len_field(tag: u32, (): &()) -> usize {
        <bool as ProtoField>::encoded_len_field(tag, &true)
    }

    /// Only presence survives: an explicit `false` still selects the variant.
    #[cfg(test)]
    fn normalize_dynamic(message: &mut ::prost_reflect::DynamicMessage, tag: u32) {
        crate::differential::bool_marker(message, tag);
    }
}

/// A `#[armonik(present)]` empty-message member: an empty length-delimited body is all the
/// information there is. Decoding rejects what prost's message codec rejects (the wire type, the
/// length framing, the keys inside); presence and emptiness are already equivalent on the dynamic
/// side, so the default identity projection is the right one.
pub(crate) struct EmptyPresence;

impl ProtoAdapter<()> for EmptyPresence {
    fn encode_field(tag: u32, (): &(), buf: &mut impl BufMut) {
        super::empty_body::encode(tag, buf);
    }

    fn merge_field(
        wire_type: WireType,
        (): &mut (),
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        super::empty_body::merge(wire_type, buf, ctx)
    }

    fn encoded_len_field(tag: u32, (): &()) -> usize {
        super::empty_body::encoded_len(tag)
    }
}

/// A proto enum as the `int32` it is on the wire: the bottom of a *transparent* enum's chain, whose
/// Rust type is the enum itself rather than a struct holding one.
pub(crate) struct EnumLeaf;

impl<T: Copy + Into<i32> + From<i32>> ProtoAdapter<T> for EnumLeaf {
    fn encode_field(tag: u32, value: &T, buf: &mut impl BufMut) {
        super::enumeration::encode(tag, value, buf)
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut T,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        super::enumeration::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &T) -> usize {
        super::enumeration::encoded_len(tag, value)
    }
}

impl<V, A: ProtoAdapter<V>, const TAG: u32> ProtoAdapter<V> for Wrapper<A, TAG> {
    fn encode_field(tag: u32, value: &V, buf: &mut impl BufMut) {
        let body = A::encoded_len_field(TAG, value);
        encoding::encode_key(tag, WireType::LengthDelimited, buf);
        encoding::encode_varint(body as u64, buf);
        A::encode_field(TAG, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut V,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        // Through `merge_loop`, which bounds the body by a byte count on the same buffer rather than
        // handing out a sub-buffer, so nesting stays monomorphic; and which brings the recursion and
        // length limits `ctx` carries plus the exact-length check.
        encoding::merge_loop(value, buf, ctx, |value, buf, ctx| {
            let (tag, wire_type) = encoding::decode_key(buf)?;
            if tag == TAG {
                A::merge_field(wire_type, value, buf, ctx)
            } else {
                encoding::skip_field(wire_type, tag, buf, ctx)
            }
        })
    }

    fn encoded_len_field(tag: u32, value: &V) -> usize {
        let body = A::encoded_len_field(TAG, value);
        encoding::key_len(tag) + encoding::encoded_len_varint(body as u64) + body
    }
}
