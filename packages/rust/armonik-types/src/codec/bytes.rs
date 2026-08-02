//! [`ProtoField`] implementation for [`bytes::Bytes`].
//!
//! `Vec<u8>` deliberately has no implementation: it would conflict with the
//! generic `Vec<T: ProtoField>` implementation, and all bytes payloads of
//! the API use `Bytes` so that decoding borrows the network buffer instead
//! of copying it.

use ::bytes::Bytes;
use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use super::{FieldKind, ProtoField};

impl ProtoField for Bytes {
    const KIND: FieldKind = FieldKind::Bytes;

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        encoding::bytes::encode(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::bytes::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        encoding::bytes::encoded_len(tag, value)
    }

    fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        encoding::bytes::encode_repeated(tag, values, buf);
    }

    fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
        encoding::bytes::encoded_len_repeated(tag, values)
    }

    fn merge_repeated(
        wire_type: WireType,
        values: &mut Vec<Self>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::bytes::merge_repeated(wire_type, values, buf, ctx)
    }
}
