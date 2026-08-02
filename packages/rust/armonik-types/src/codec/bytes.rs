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

use super::{FieldKind, ProtoField, Shape};

impl ProtoField for Bytes {
    const SHAPE: Shape = Shape::scalar(FieldKind::Bytes);

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

    // Repeated forms: the trait's unpacked defaults (bytes never pack).
}
