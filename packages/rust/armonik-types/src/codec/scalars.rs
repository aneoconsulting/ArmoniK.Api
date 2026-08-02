//! [`ProtoField`] implementations for protobuf scalar types, delegating to
//! the [`prost::encoding`] building blocks (the same ones `prost-derive`
//! expands to).

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use super::{FieldKind, ProtoField};

/// Scalar whose repeated form is packed (proto3 default for numeric kinds).
macro_rules! packed_scalar {
    ($ty:ty, $kind:ident, $module:ident) => {
        impl ProtoField for $ty {
            const KIND: FieldKind = FieldKind::$kind;

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

impl ProtoField for String {
    const KIND: FieldKind = FieldKind::String;

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        encoding::string::encode(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::string::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        encoding::string::encoded_len(tag, value)
    }

    // Repeated forms: the trait's unpacked defaults (strings never pack).
}
