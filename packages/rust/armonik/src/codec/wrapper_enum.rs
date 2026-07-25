//! Helpers for "transparent" enums: Rust enums standing for proto message
//! wrappers around an enum field — either a single wrapper (e.g.
//! `sessions.TaskOptionField`) or a chain of single-field wrappers (e.g.
//! `applications.ApplicationField` wrapping `ApplicationRawField` wrapping
//! the enum). `path` holds the field tags from the outermost wrapper down to
//! the enum field.

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

fn key_len(tag: u32) -> usize {
    encoding::encoded_len_varint(u64::from(tag) << 3)
}

fn body_len<T: Copy + Into<i32>>(path: &[u32], value: &T) -> usize {
    let raw: i32 = (*value).into();
    if raw == 0 {
        // A zero value carries no information at any wrapper depth.
        return 0;
    }
    let (&inner_tag, rest) = path.split_first().expect("non-empty wrapper path");
    if rest.is_empty() {
        encoding::int32::encoded_len(inner_tag, &raw)
    } else {
        encoded_len(inner_tag, rest, value)
    }
}

pub(crate) fn encode<T: Copy + Into<i32>>(
    tag: u32,
    path: &[u32],
    value: &T,
    buf: &mut impl BufMut,
) {
    let len = body_len(path, value);
    encoding::encode_key(tag, WireType::LengthDelimited, buf);
    encoding::encode_varint(len as u64, buf);
    let raw: i32 = (*value).into();
    if raw == 0 {
        return;
    }
    let (&inner_tag, rest) = path.split_first().expect("non-empty wrapper path");
    if rest.is_empty() {
        encoding::int32::encode(inner_tag, &raw, buf);
    } else {
        encode(inner_tag, rest, value, buf);
    }
}

pub(crate) fn encoded_len<T: Copy + Into<i32>>(tag: u32, path: &[u32], value: &T) -> usize {
    let len = body_len(path, value);
    key_len(tag) + encoding::encoded_len_varint(len as u64) + len
}

pub(crate) fn merge<T: Copy + Into<i32> + From<i32>>(
    path: &[u32],
    wire_type: WireType,
    value: &mut T,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    // The recursion is dynamic over `dyn Buf`: a generic recursion would
    // monomorphize an unbounded tower of `Take<&mut Take<...>>` types.
    merge_dyn(path, wire_type, value, buf, ctx)
}

fn merge_dyn<T: Copy + Into<i32> + From<i32>>(
    path: &[u32],
    wire_type: WireType,
    value: &mut T,
    buf: &mut dyn Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
    let len = encoding::decode_varint(&mut &mut *buf)? as usize;
    if buf.remaining() < len {
        // prost offers no other public constructor; pinned to prost 0.14.
        #[allow(deprecated)]
        return Err(DecodeError::new("buffer underflow"));
    }
    let (&inner_tag, rest) = path.split_first().expect("non-empty wrapper path");
    let mut wrapper = buf.take(len);
    while wrapper.has_remaining() {
        let (tag, wire_type) = encoding::decode_key(&mut wrapper)?;
        if tag != inner_tag {
            encoding::skip_field(wire_type, tag, &mut wrapper, ctx.clone())?;
        } else if rest.is_empty() {
            let mut raw: i32 = (*value).into();
            encoding::int32::merge(wire_type, &mut raw, &mut wrapper, ctx.clone())?;
            *value = T::from(raw);
        } else {
            let inner: &mut dyn Buf = &mut wrapper;
            merge_dyn(rest, wire_type, value, inner, ctx.clone())?;
        }
    }
    Ok(())
}

/// Wrapper enums are always emitted (a zero value encodes as an empty
/// wrapper), preserving the absent-vs-explicit-zero distinction that fields
/// with a non-zero API default rely on — exactly like the historical
/// `Some(wrapper)` conversions.
pub(crate) fn is_default<T: Copy + Into<i32>>(value: &T) -> bool {
    let _ = value;
    false
}
