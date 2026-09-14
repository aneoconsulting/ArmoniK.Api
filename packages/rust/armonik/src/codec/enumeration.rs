//! Helpers for native-enum fields, which are `int32` varints on the wire.
//!
//! The `#[armonik_macros::enumeration]` expansion emits a [`ProtoField`](super::ProtoField)
//! implementation delegating to these functions, with `T` converting to and
//! from the raw `i32` through its (normalizing) `From` implementations.

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

fn to_varint<T: Copy + Into<i32>>(value: &T) -> u64 {
    // Sign-extension matches the 10-byte varint encoding of negative int32.
    i64::from((*value).into()) as u64
}

pub(crate) fn encode<T: Copy + Into<i32>>(tag: u32, value: &T, buf: &mut impl BufMut) {
    let raw: i32 = (*value).into();
    encoding::int32::encode(tag, &raw, buf);
}

pub(crate) fn merge<T: Copy + Into<i32> + From<i32>>(
    wire_type: WireType,
    value: &mut T,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    let mut raw: i32 = (*value).into();
    encoding::int32::merge(wire_type, &mut raw, buf, ctx)?;
    *value = T::from(raw);
    Ok(())
}

pub(crate) fn encoded_len<T: Copy + Into<i32>>(tag: u32, value: &T) -> usize {
    let raw: i32 = (*value).into();
    encoding::int32::encoded_len(tag, &raw)
}

fn packed_body_len<T: Copy + Into<i32>>(values: &[T]) -> usize {
    values
        .iter()
        .map(|value| encoding::encoded_len_varint(to_varint(value)))
        .sum()
}

pub(crate) fn encode_repeated<T: Copy + Into<i32>>(tag: u32, values: &[T], buf: &mut impl BufMut) {
    if values.is_empty() {
        return;
    }
    encoding::encode_key(tag, WireType::LengthDelimited, buf);
    encoding::encode_varint(packed_body_len(values) as u64, buf);
    for value in values {
        encoding::encode_varint(to_varint(value), buf);
    }
}

pub(crate) fn encoded_len_repeated<T: Copy + Into<i32>>(tag: u32, values: &[T]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let body = packed_body_len(values);
    encoding::key_len(tag) + encoding::encoded_len_varint(body as u64) + body
}

pub(crate) fn merge_repeated<T: Copy + Into<i32> + From<i32>>(
    wire_type: WireType,
    values: &mut Vec<T>,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    if wire_type == WireType::LengthDelimited {
        // Packed.
        encoding::merge_loop(values, buf, ctx, |values, buf, _ctx| {
            let raw = encoding::decode_varint(buf)? as i32;
            values.push(T::from(raw));
            Ok(())
        })
    } else {
        // Unpacked.
        encoding::check_wire_type(WireType::Varint, wire_type)?;
        let raw = encoding::decode_varint(buf)? as i32;
        values.push(T::from(raw));
        Ok(())
    }
}
