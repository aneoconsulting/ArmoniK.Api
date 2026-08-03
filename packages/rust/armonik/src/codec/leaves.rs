//! [`ProtoField`] leaves: protobuf scalars, `String`, [`bytes::Bytes`] and
//! the well-known types, delegating to the [`prost::encoding`] building
//! blocks (the same ones `prost-derive` expands to).
//!
//! `Vec<u8>` deliberately has no implementation: it would conflict with the
//! generic `Vec<T: ProtoField>` implementation, and all bytes payloads of
//! the API use [`bytes::Bytes`] so that decoding borrows the network buffer
//! instead of copying it.

use ::bytes::Bytes;
use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use super::{FieldKind, ProtoField, Shape};

/// Scalar whose repeated form is packed (proto3 default for numeric kinds).
macro_rules! packed_scalar {
    ($ty:ty, $kind:ident, $module:ident) => {
        impl ProtoField for $ty {
            const SHAPE: Shape = Shape::scalar(FieldKind::$kind);

            fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
                encoding::$module::encode(tag, value, buf);
            }

            fn merge_field(
                wire_type: WireType,
                value: &mut Self,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                encoding::$module::merge(wire_type, value, buf, ctx)
            }

            fn encoded_len_field(tag: u32, value: &Self) -> usize {
                encoding::$module::encoded_len(tag, value)
            }

            fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
                encoding::$module::encode_packed(tag, values, buf);
            }

            fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
                encoding::$module::encoded_len_packed(tag, values)
            }

            fn merge_repeated(
                wire_type: WireType,
                values: &mut Vec<Self>,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                encoding::$module::merge_repeated(wire_type, values, buf, ctx)
            }
        }
    };
}

packed_scalar!(f64, Double, double);
packed_scalar!(f32, Float, float);
packed_scalar!(i32, Int32, int32);
packed_scalar!(i64, Int64, int64);
packed_scalar!(u32, UInt32, uint32);
packed_scalar!(u64, UInt64, uint64);
packed_scalar!(bool, Bool, bool);

/// Length-delimited leaf: singular forms only, the repeated trait defaults
/// (unpacked) are already right.
macro_rules! delimited_leaf {
    ($ty:ty, $kind:ident, $module:ident) => {
        impl ProtoField for $ty {
            const SHAPE: Shape = Shape::scalar(FieldKind::$kind);

            fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
                encoding::$module::encode(tag, value, buf);
            }

            fn merge_field(
                wire_type: WireType,
                value: &mut Self,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                encoding::$module::merge(wire_type, value, buf, ctx)
            }

            fn encoded_len_field(tag: u32, value: &Self) -> usize {
                encoding::$module::encoded_len(tag, value)
            }
        }
    };
}

delimited_leaf!(String, String, string);
delimited_leaf!(Bytes, Bytes, bytes);

// The well-known types, gated into the blanket message-kind impl.

impl super::Msg for prost_types::Timestamp {
    const NAMES: &'static [&'static str] = &["google.protobuf.Timestamp"];
}

impl super::Msg for prost_types::Duration {
    const NAMES: &'static [&'static str] = &["google.protobuf.Duration"];
}
