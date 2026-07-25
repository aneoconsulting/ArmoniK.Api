//! Helpers for "transparent" enums: Rust enums standing for a proto message
//! that wraps a single enum field (e.g. `sessions.TaskOptionField` /
//! `tasks.TaskOptionField`). On the wire the value is a length-delimited
//! message `{ inner_tag: enum_value }`; in Rust it is the enum itself.

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

fn key_len(tag: u32) -> usize {
    encoding::encoded_len_varint(u64::from(tag) << 3)
}

fn inner_len<T: Copy + Into<i32>>(inner_tag: u32, value: &T) -> usize {
    let raw: i32 = (*value).into();
    if raw == 0 {
        0
    } else {
        encoding::int32::encoded_len(inner_tag, &raw)
    }
}

pub(crate) fn encode<T: Copy + Into<i32>>(
    tag: u32,
    inner_tag: u32,
    value: &T,
    buf: &mut impl BufMut,
) {
    let raw: i32 = (*value).into();
    let len = inner_len(inner_tag, value);
    encoding::encode_key(tag, WireType::LengthDelimited, buf);
    encoding::encode_varint(len as u64, buf);
    if raw != 0 {
        encoding::int32::encode(inner_tag, &raw, buf);
    }
}

pub(crate) fn encoded_len<T: Copy + Into<i32>>(tag: u32, inner_tag: u32, value: &T) -> usize {
    let len = inner_len(inner_tag, value);
    key_len(tag) + encoding::encoded_len_varint(len as u64) + len
}

pub(crate) fn merge<T: Copy + Into<i32> + From<i32>>(
    inner_tag: u32,
    wire_type: WireType,
    value: &mut T,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
    let len = encoding::decode_varint(buf)? as usize;
    if buf.remaining() < len {
        // prost offers no other public constructor; pinned to prost 0.14.
        #[allow(deprecated)]
        return Err(DecodeError::new("buffer underflow"));
    }
    let mut wrapper = buf.take(len);
    while wrapper.has_remaining() {
        let (tag, wire_type) = encoding::decode_key(&mut wrapper)?;
        if tag == inner_tag {
            let mut raw: i32 = (*value).into();
            encoding::int32::merge(wire_type, &mut raw, &mut wrapper, ctx.clone())?;
            *value = T::from(raw);
        } else {
            encoding::skip_field(wire_type, tag, &mut wrapper, ctx.clone())?;
        }
    }
    Ok(())
}

/// A zero value encodes to an empty wrapper, indistinguishable from an
/// absent one.
pub(crate) fn is_default<T: Copy + Into<i32>>(value: &T) -> bool {
    (*value).into() == 0
}
