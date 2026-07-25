//! Helpers for message-kind fields, shared by the derive-emitted code, the
//! well-known types and the hand-written implementations.

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

pub(crate) fn encode<M: prost::Message>(tag: u32, message: &M, buf: &mut impl BufMut) {
    encoding::message::encode(tag, message, buf);
}

pub(crate) fn merge<M: prost::Message>(
    wire_type: WireType,
    message: &mut M,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    encoding::message::merge(wire_type, message, buf, ctx)
}

pub(crate) fn encoded_len<M: prost::Message>(tag: u32, message: &M) -> usize {
    encoding::message::encoded_len(tag, message)
}

pub(crate) fn encode_repeated<M: prost::Message>(tag: u32, messages: &[M], buf: &mut impl BufMut) {
    encoding::message::encode_repeated(tag, messages, buf);
}

pub(crate) fn merge_repeated<M: prost::Message + Default>(
    wire_type: WireType,
    messages: &mut Vec<M>,
    buf: &mut impl Buf,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    encoding::message::merge_repeated(wire_type, messages, buf, ctx)
}

pub(crate) fn encoded_len_repeated<M: prost::Message>(tag: u32, messages: &[M]) -> usize {
    encoding::message::encoded_len_repeated(tag, messages)
}

/// A message that encodes to zero bytes is indistinguishable from an absent
/// one for the "absent = default" fields of the API.
pub(crate) fn is_default<M: prost::Message>(message: &M) -> bool {
    message.encoded_len() == 0
}
