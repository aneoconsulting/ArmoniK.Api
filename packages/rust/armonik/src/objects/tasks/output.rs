use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use crate::codec::{FieldKind, ProtoAdapter, ProtoField};

/// Represents the task output.
///
/// Stands for the `TaskDetailed.Output` message, whose two plain fields
/// (`bool success = 1`, `string error = 2`) do not form a proto oneof: the
/// wire implementation is hand-written. `success = true` wins over any
/// error message; a message without `success` is an error (possibly with
/// an empty message), while an absent output defaults to success.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Output {
    /// To know if a task have failed or succeed.
    #[default]
    Success,
    /// The error message. Only set if task have failed.
    Error(String),
}

const SUCCESS_TAG: u32 = 1;
const ERROR_TAG: u32 = 2;

// Hand-written rather than derived. Everything the derive emits (encode
// fragments, merge arms, decode seeds, descriptor asserts) is generated from
// a one-Rust-field-to-one-proto-field correspondence — Rust enums map to
// proto oneofs (or whole single-oneof messages), one variant per member, and
// `with` adapters can change how a single field is encoded, but not that
// arity. Here two *plain* fields project onto one enum, and the projection
// is cross-field: which variant a `success` occurrence produces depends on
// whether an `error` was merged, and vice versa. Teaching the derive this
// shape would cost more grammar and codegen than the two hand-written impls
// it would replace (this one and `agent::notify_result_data::Request`), each
// used exactly once. The differential harness fuzzes them against
// `DynamicMessage` ground truth exactly like the derived types.
impl prost::Message for Output {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        match self {
            Output::Success => <bool as ProtoField>::encode_field(SUCCESS_TAG, &true, buf),
            Output::Error(message) => {
                if !message.is_empty() {
                    <String as ProtoField>::encode_field(ERROR_TAG, message, buf);
                }
            }
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            SUCCESS_TAG => {
                let mut success = false;
                <bool as ProtoField>::merge_field(wire_type, &mut success, buf, ctx)?;
                if success {
                    *self = Output::Success;
                } else if !matches!(self, Output::Error(_)) {
                    *self = Output::Error(String::new());
                }
                Ok(())
            }
            ERROR_TAG => {
                // The message is kept only when the task did not succeed,
                // but the field must be consumed either way.
                let mut message = String::new();
                <String as ProtoField>::merge_field(wire_type, &mut message, buf, ctx)?;
                if !matches!(self, Output::Success) {
                    *self = Output::Error(message);
                }
                Ok(())
            }
            _ => encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        match self {
            Output::Success => <bool as ProtoField>::encoded_len_field(SUCCESS_TAG, &true),
            Output::Error(message) if !message.is_empty() => {
                <String as ProtoField>::encoded_len_field(ERROR_TAG, message)
            }
            Output::Error(_) => 0,
        }
    }

    fn clear(&mut self) {
        *self = Output::Success;
    }

    // Seed from `wire_default`: an empty message is an empty error, only
    // a truly absent output defaults to success.
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError>
    where
        Self: Default,
    {
        let mut message = <Self as ProtoField>::wire_default();
        prost::Message::merge(&mut message, &mut buf)?;
        Ok(message)
    }

    fn decode_length_delimited(buf: impl Buf) -> Result<Self, DecodeError>
    where
        Self: Default,
    {
        let mut message = <Self as ProtoField>::wire_default();
        prost::Message::merge_length_delimited(&mut message, buf)?;
        Ok(message)
    }
}

impl ProtoField for Output {
    const KIND: FieldKind = FieldKind::Message;
    const NAMES: &'static [&'static str] = &["armonik.api.grpc.v1.tasks.TaskDetailed.Output"];

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        crate::codec::message::encode(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        crate::codec::message::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        crate::codec::message::encoded_len(tag, value)
    }

    /// Always emitted: `Error("")` encodes as an empty message, keeping it
    /// distinct from an absent output (= success).
    fn is_default(value: &Self) -> bool {
        let _ = value;
        false
    }

    fn wire_default() -> Self {
        Output::Error(String::new())
    }

    fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        crate::codec::message::encode_repeated(tag, values, buf);
    }

    fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
        crate::codec::message::encoded_len_repeated(tag, values)
    }
}

/// `TaskSummary.error` (a plain string field) exposed as an [`Output`]: an
/// empty error stands for success, like the historical conversion.
pub(crate) struct ErrorAdapter;

impl ProtoAdapter<Output> for ErrorAdapter {
    fn encode_field(tag: u32, value: &Output, buf: &mut impl BufMut) {
        if let Output::Error(message) = value {
            <String as ProtoField>::encode_field(tag, message, buf);
        }
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Output,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let mut message = String::new();
        <String as ProtoField>::merge_field(wire_type, &mut message, buf, ctx)?;
        *value = if message.is_empty() {
            Output::Success
        } else {
            Output::Error(message)
        };
        Ok(())
    }

    fn encoded_len_field(tag: u32, value: &Output) -> usize {
        match value {
            Output::Error(message) => <String as ProtoField>::encoded_len_field(tag, message),
            Output::Success => 0,
        }
    }

    fn is_default(value: &Output) -> bool {
        match value {
            Output::Success => true,
            Output::Error(message) => message.is_empty(),
        }
    }

    fn clear_field(value: &mut Output) {
        *value = Output::Success;
    }
}
