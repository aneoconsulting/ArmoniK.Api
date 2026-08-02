//! [`ProtoField`] implementations for the well-known types used by the API.

use prost::bytes::{Buf, BufMut};
use prost::encoding::{message, DecodeContext, WireType};
use prost::DecodeError;

use super::{message_is_default, FieldKind, ProtoField};

macro_rules! well_known_message {
    ($ty:ty, $name:literal) => {
        impl ProtoField for $ty {
            const KIND: FieldKind = FieldKind::Message;
            const NAMES: &'static [&'static str] = &[$name];

            fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
                message::encode(tag, value, buf);
            }

            fn merge_field(
                wire_type: WireType,
                value: &mut Self,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                message::merge(wire_type, value, buf, ctx)
            }

            fn encoded_len_field(tag: u32, value: &Self) -> usize {
                message::encoded_len(tag, value)
            }

            fn is_default(value: &Self) -> bool {
                message_is_default(value)
            }

            fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
                message::encode_repeated(tag, values, buf);
            }

            fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
                message::encoded_len_repeated(tag, values)
            }

            fn merge_repeated(
                wire_type: WireType,
                values: &mut Vec<Self>,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                message::merge_repeated(wire_type, values, buf, ctx)
            }
        }
    };
}

well_known_message!(prost_types::Timestamp, "google.protobuf.Timestamp");
well_known_message!(prost_types::Duration, "google.protobuf.Duration");
