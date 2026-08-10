use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use crate::codec::{ProtoAdapter, ProtoField};

/// Stands for the `TaskDetailed.Output` message, whose two plain fields (`bool success = 1`,
/// `string error = 2`) do not form a proto oneof, so the wire implementation is hand-written.
/// `success = true` wins over any error message; an absent or empty message is an error with an
/// empty message, which is also the `Default` (the proto zero value, like every armonik type).
///
/// One enum keeps one of the two fields, so the merge keeps one too: a `success` occurrence that
/// selects `Success` drops any error message merged before it, and a later `success = false`
/// leaves the empty error rather than that message. Repeating a singular field is the only way to
/// reach this, since either field is read once per message otherwise.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Output {
    Success,
    Error(String),
}

impl Default for Output {
    fn default() -> Self {
        Self::Error(String::new())
    }
}

const SUCCESS_TAG: u32 = 1;
const ERROR_TAG: u32 = 2;

// Hand-written rather than derived. Everything the derive emits (encode fragments, merge arms,
// decode seeds, descriptor asserts) is generated from a one-Rust-field-to-one-proto-field
// correspondence: Rust enums map to proto oneofs (or whole single-oneof messages), one variant per
// member, and `with` adapters can change how a single field is encoded, but not that arity. Here
// two plain fields project onto one enum, and the projection is cross-field: which variant a
// `success` occurrence produces depends on whether an `error` was merged, and vice versa. Teaching
// the derive this shape would cost more grammar and codegen than the two hand-written impls it
// would replace (this one and `agent::notify_result_data::Request`), each used exactly once. The
// differential harness fuzzes them against `DynamicMessage` ground truth exactly like the derived
// types.
impl prost::Message for Output {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        match self {
            Output::Success => <bool as ProtoField>::encode_field(SUCCESS_TAG, &true, buf),
            Output::Error(message) => <String as ProtoField>::encode_field(ERROR_TAG, message, buf),
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
                // The message is kept only when the task did not succeed, but the field must be
                // consumed either way.
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
            Output::Error(message) => <String as ProtoField>::encoded_len_field(ERROR_TAG, message),
        }
    }

    fn clear(&mut self) {
        *self = Output::default();
    }
}

#[cfg(test)]
impl crate::differential::Normalize for Output {
    /// `success = true` wins over any error message: the enum keeps one of the two plain fields, so
    /// the losing `error` carries no information.
    fn normalize(message: &mut ::prost_reflect::DynamicMessage) {
        use ::prost_reflect::{ReflectMessage, Value};
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

// Hand-written `Message` impls register through the same `register!` macro the expansions emit, so
// they carry their round-trip/`Normalize` hooks. This one is nested in `TaskDetailed` rather than a
// message of its own, and registering it anyway holds the "every impl is harvested" invariant.
crate::register!(message: Output, "armonik.api.grpc.v1.tasks.TaskDetailed.Output");

// As a field, `Output` is an ordinary message: the blanket message-kind `ProtoField` impl frames
// the hand-written `prost::Message` impl above.
impl crate::codec::Msg for Output {
    const NAMES: &'static [&'static str] = &["armonik.api.grpc.v1.tasks.TaskDetailed.Output"];
}

/// `TaskSummary.error` (a plain string field) exposed as an [`Output`]: an empty error stands for
/// success. The two sides carry different amounts of information, so the map is not injective:
/// `Success` and `Error(String::new())` are the same wire value there, and decoding it yields
/// `Success`.
pub(crate) struct ErrorAdapter;

impl ProtoAdapter<Output> for ErrorAdapter {
    fn encode_field(tag: u32, value: &Output, buf: &mut impl BufMut) {
        match value {
            Output::Error(message) => <String as ProtoField>::encode_field(tag, message, buf),
            // Success *is* the empty error message.
            Output::Success => crate::codec::empty_body::encode(tag, buf),
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
            Output::Success => crate::codec::empty_body::encoded_len(tag),
        }
    }
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::{Output, SUCCESS_TAG};

    /// prost-derived reference of `TaskDetailed.Output`, an independent codec: fixtures are built
    /// and encoded through it, then decoded through
    /// our hand-written `Output`, so a bug in `merge_field`'s cross-field rule cannot hide behind a
    /// matching `Normalize`. The field-information ratchet probes one field at a time and never
    /// produces the `{ success, error }` combination.
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
        // Both fields set on the wire, the adversarial case the ratchet cannot reach.
        // `TaskDetailed.Output` collapses to success.
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

    /// The merge keeps the last `success` occurrence, and the error message merged while it was
    /// false only for as long as no occurrence selects `Success`.
    #[test]
    fn a_repeated_success_field_keeps_the_last_occurrence() {
        let mut bytes = RefOutput {
            success: true,
            error: "boom".to_owned(),
        }
        .encode_to_vec();
        // A second `success`, false this time. `error` is not repeated, so nothing restores it.
        bytes.extend(
            RefOutput {
                success: false,
                error: String::new(),
            }
            .encode_to_vec(),
        );
        // prost writes nothing for a false bool, so spell the occurrence out.
        bytes.extend([(SUCCESS_TAG << 3) as u8, 0]);

        assert_eq!(
            Output::decode(bytes.as_slice()).expect("decodes"),
            Output::Error(String::new()),
        );
    }

    /// `TaskSummary` carries the output as a plain `error` string, where an empty message is
    /// success: `Success` and `Error("")` are the same wire value, which reads back as `Success`.
    #[test]
    fn an_empty_task_summary_error_is_success() {
        use crate::tasks::Summary;

        let encoded = |output| {
            Summary {
                output,
                ..Default::default()
            }
            .encode_to_vec()
        };

        assert_eq!(
            encoded(Output::Error(String::new())),
            encoded(Output::Success)
        );
        assert_eq!(
            Summary::decode(encoded(Output::Error(String::new())).as_slice())
                .expect("decodes")
                .output,
            Output::Success,
        );
        assert_eq!(
            Summary::decode(encoded(Output::Error(String::from("boom"))).as_slice())
                .expect("decodes")
                .output,
            Output::Error(String::from("boom")),
        );
    }

    #[test]
    fn absent_output_is_the_empty_error() {
        // Both a `{ success: false, error: "" }` message and a wholly empty one decode to the
        // zero-default, an empty error.
        assert_eq!(
            decode(RefOutput {
                success: false,
                error: String::new(),
            }),
            Output::Error(String::new()),
        );
        assert_eq!(
            Output::decode([].as_slice()).expect("decodes"),
            Output::default()
        );
        assert_eq!(Output::default(), Output::Error(String::new()));
    }
}
