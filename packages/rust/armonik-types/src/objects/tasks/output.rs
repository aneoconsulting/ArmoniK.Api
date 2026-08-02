use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use crate::codec::{FieldKind, ProtoAdapter, ProtoField};

/// Represents the task output.
///
/// Stands for the `TaskDetailed.Output` message, whose two plain fields
/// (`bool success = 1`, `string error = 2`) do not form a proto oneof: the
/// wire implementation is hand-written. `success = true` wins over any
/// error message; an absent or empty message is an error with an empty
/// message — which is also the `Default` (the proto zero value, like every
/// armonik type).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Output {
    /// To know if a task have failed or succeed.
    Success,
    /// The error message. Only set if task have failed.
    Error(String),
}

impl Default for Output {
    fn default() -> Self {
        Self::Error(String::new())
    }
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
        *self = Output::default();
    }
}

#[cfg(feature = "_differential")]
impl crate::differential::Normalize for Output {
    /// `success = true` wins over any error message: the enum keeps one of
    /// the two plain fields, so the losing `error` carries no information.
    fn normalize(message: &mut crate::differential::prost_reflect::DynamicMessage) {
        use crate::differential::prost_reflect::{ReflectMessage, Value};
        let descriptor = message.descriptor();
        let (Some(success), Some(error)) = (
            descriptor.get_field(SUCCESS_TAG),
            descriptor.get_field(ERROR_TAG),
        ) else {
            return;
        };
        if matches!(message.get_field(&success).as_ref(), Value::Bool(true)) {
            message.clear_field(&error);
        }
    }
}

// Hand-written `Message` impls register through the same `register!` macro the
// derive emits, so they carry their round-trip/`Normalize` hooks and their
// extern-map entry. `armonik` only externs top-level messages (this one is
// nested in the extern'd `TaskDetailed`), but keeping it in the registry holds
// the "every impl is harvested" invariant.
crate::register!(message: Output, "armonik.api.grpc.v1.tasks.TaskDetailed.Output");

impl ProtoField for Output {
    const KIND: FieldKind = FieldKind::Message;
    const NAMES: &'static [&'static str] = &["armonik.api.grpc.v1.tasks.TaskDetailed.Output"];

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        ::prost::encoding::message::encode(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        ::prost::encoding::message::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        ::prost::encoding::message::encoded_len(tag, value)
    }

    fn is_default(value: &Self) -> bool {
        crate::codec::message_is_default(value)
    }

    fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        ::prost::encoding::message::encode_repeated(tag, values, buf);
    }

    fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
        ::prost::encoding::message::encoded_len_repeated(tag, values)
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
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::Output;

    /// prost-derived reference of `TaskDetailed.Output` (extern'd, so no
    /// generated type exists). An independent codec — the fixtures are built
    /// and encoded through it, then decoded through our hand-written `Output`,
    /// so a bug in `merge_field`'s cross-field rule cannot hide behind a
    /// matching `Normalize` (the field-information ratchet, which probes one
    /// field at a time, never produces the `{ success, error }` combination).
    #[derive(Clone, PartialEq, Message)]
    struct RefOutput {
        #[prost(bool, tag = "1")]
        success: bool,
        #[prost(string, tag = "2")]
        error: String,
    }

    fn decode(reference: RefOutput) -> Output {
        Output::decode(reference.encode_to_vec().as_slice()).expect("decodes")
    }

    #[test]
    fn success_wins_over_a_set_error() {
        // Both fields set on the wire — the adversarial case the ratchet can't
        // reach. `TaskDetailed.Output` collapses to success.
        assert_eq!(
            decode(RefOutput {
                success: true,
                error: "boom".to_owned(),
            }),
            Output::Success,
        );
    }

    #[test]
    fn error_is_kept_when_not_successful() {
        assert_eq!(
            decode(RefOutput {
                success: false,
                error: "boom".to_owned(),
            }),
            Output::Error("boom".to_owned()),
        );
    }

    #[test]
    fn absent_output_is_the_empty_error() {
        // Both a `{ success: false, error: "" }` message and a wholly empty
        // one decode to the zero-default, an empty error.
        assert_eq!(
            decode(RefOutput {
                success: false,
                error: String::new(),
            }),
            Output::Error(String::new()),
        );
        assert_eq!(Output::decode([].as_slice()).expect("decodes"), Output::default());
        assert_eq!(Output::default(), Output::Error(String::new()));
    }
}
