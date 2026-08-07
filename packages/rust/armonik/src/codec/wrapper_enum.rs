//! Helpers for "transparent" enums: Rust enums standing for proto message wrappers around an enum
//! field, either a single wrapper (`sessions.TaskOptionField`) or a chain of single-field wrappers
//! (`applications.ApplicationField` wrapping `ApplicationRawField` wrapping the enum). `path` holds
//! the field tags from the outermost wrapper down to the enum field.

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, key_len, DecodeContext, WireType};
use prost::DecodeError;

// The recursion is dynamic over `dyn Buf`: a generic recursion would monomorphize an unbounded
// tower of `Take<&mut Take<...>>` types.
fn merge_dyn<T: Copy + Into<i32> + From<i32>>(
    path: &[u32],
    wire_type: WireType,
    value: &mut T,
    buf: &mut dyn Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
    let (&inner_tag, rest) = path.split_first().expect("non-empty wrapper path");
    let mut wrapper = super::read_delimited(buf)?;
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

// Body forms of the outermost wrapper (no containing field key), for the `prost::Message` impls of
// transparent enums standing for RPC messages. The outermost wrapper itself is framed by the
// blanket message-kind `ProtoField` impl over those `prost::Message` impls.
//
// Nothing along the chain is ever skipped, so the shape is a function of `path` alone: the enum's
// `int32` field at the last tag, wrapped in one length-delimited message per tag above it. Both
// halves are therefore a fold from the enum field outwards, rather than a recursion that has to ask
// the value where it bottoms out.

pub(crate) fn encoded_len_raw<T: Copy + Into<i32>>(path: &[u32], value: &T) -> usize {
    let (&leaf, wrappers) = path.split_last().expect("non-empty wrapper path");
    let mut len = encoding::int32::encoded_len(leaf, &(*value).into());
    for &tag in wrappers.iter().rev() {
        len = key_len(tag) + encoding::encoded_len_varint(len as u64) + len;
    }
    len
}

pub(crate) fn encode_raw<T: Copy + Into<i32>>(path: &[u32], value: &T, buf: &mut impl BufMut) {
    let (&leaf, wrappers) = path.split_last().expect("non-empty wrapper path");
    for (depth, &tag) in wrappers.iter().enumerate() {
        // This wrapper's body is the rest of the chain below it.
        encoding::encode_key(tag, WireType::LengthDelimited, buf);
        encoding::encode_varint(encoded_len_raw(&path[depth + 1..], value) as u64, buf);
    }
    encoding::int32::encode(leaf, &(*value).into(), buf);
}

pub(crate) fn merge_root_field<T: Copy + Into<i32> + From<i32>>(
    path: &[u32],
    tag: u32,
    wire_type: WireType,
    value: &mut T,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    let (&inner_tag, rest) = path.split_first().expect("non-empty wrapper path");
    if tag != inner_tag {
        return encoding::skip_field(wire_type, tag, buf, ctx);
    }
    if rest.is_empty() {
        let mut raw: i32 = (*value).into();
        encoding::int32::merge(wire_type, &mut raw, buf, ctx)?;
        *value = T::from(raw);
        Ok(())
    } else {
        merge_dyn(rest, wire_type, value, buf, ctx)
    }
}
