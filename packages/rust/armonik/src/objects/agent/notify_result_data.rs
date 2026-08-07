use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use crate::codec::ProtoField;

/// The proto message carries `repeated ResultIdentifier { session_id, result_id }` pairs, flattened
/// here into one session ID shared by all the results: the wire implementation is hand-written.
/// Encoding replicates the session ID in every pair; decoding keeps the first non-empty one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub communication_token: String,
    /// The identifier of the session where all the results to be notified are.
    pub session_id: String,
    /// The identifiers of the results to be notified.
    pub result_ids: Vec<String>,
}

const IDS_TAG: u32 = 1;
const TOKEN_TAG: u32 = 4;
/// Fields of the `ResultIdentifier` pairs.
const PAIR_SESSION_TAG: u32 = 1;
const PAIR_RESULT_TAG: u32 = 2;

impl Request {
    fn pair_len(&self, result_id: &String) -> usize {
        <String as ProtoField>::encoded_len_field(PAIR_SESSION_TAG, &self.session_id)
            + <String as ProtoField>::encoded_len_field(PAIR_RESULT_TAG, result_id)
    }
}

// Hand-written rather than derived. The derive, and its `with` adapters, maps one Rust field to one
// proto field, but `ids` splits into two Rust fields: each encoded pair combines the shared
// `session_id` with one element of `result_ids`, and each decoded pair updates both, since the
// first-non-empty-session rule reads the partially merged struct itself. Teaching the derive this
// one-off shape would cost more than the impl (see `tasks::Output` for the full tradeoff). The
// differential harness fuzzes it like any derived type.
impl prost::Message for Request {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        for result_id in &self.result_ids {
            let pair_len = self.pair_len(result_id);
            encoding::encode_key(IDS_TAG, WireType::LengthDelimited, buf);
            encoding::encode_varint(pair_len as u64, buf);
            <String as ProtoField>::encode_field(PAIR_SESSION_TAG, &self.session_id, buf);
            <String as ProtoField>::encode_field(PAIR_RESULT_TAG, result_id, buf);
        }
        <String as ProtoField>::encode_field(TOKEN_TAG, &self.communication_token, buf);
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        match tag {
            IDS_TAG => {
                encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
                let mut pair = crate::codec::read_delimited(buf)?;
                let mut session_id = String::new();
                let mut result_id = String::new();
                while pair.has_remaining() {
                    let (tag, wire_type) = encoding::decode_key(&mut pair)?;
                    match tag {
                        PAIR_SESSION_TAG => <String as ProtoField>::merge_field(
                            wire_type,
                            &mut session_id,
                            &mut pair,
                            ctx.clone(),
                        )?,
                        PAIR_RESULT_TAG => <String as ProtoField>::merge_field(
                            wire_type,
                            &mut result_id,
                            &mut pair,
                            ctx.clone(),
                        )?,
                        _ => encoding::skip_field(wire_type, tag, &mut pair, ctx.clone())?,
                    }
                }
                if self.session_id.is_empty() {
                    self.session_id = session_id;
                }
                self.result_ids.push(result_id);
                Ok(())
            }
            TOKEN_TAG => <String as ProtoField>::merge_field(
                wire_type,
                &mut self.communication_token,
                buf,
                ctx,
            ),
            _ => encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len =
            <String as ProtoField>::encoded_len_field(TOKEN_TAG, &self.communication_token);
        for result_id in &self.result_ids {
            let pair_len = self.pair_len(result_id);
            len += encoding::key_len(IDS_TAG)
                + encoding::encoded_len_varint(pair_len as u64)
                + pair_len;
        }
        len
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(feature = "_differential")]
impl crate::differential::Normalize for Request {
    /// The `ResultIdentifier` pairs are flattened into one shared session ID (the first non-empty
    /// one) plus the result IDs: every pair's `session_id` is equivalent to that shared one.
    fn normalize(message: &mut crate::differential::prost_reflect::DynamicMessage) {
        use crate::differential::prost_reflect::{ReflectMessage, Value};
        let Some(ids) = message.descriptor().get_field(IDS_TAG) else {
            return;
        };
        if !message.has_field(&ids) {
            return;
        }
        let Value::List(mut entries) = message.get_field(&ids).into_owned() else {
            return;
        };
        let session_id = entries
            .iter()
            .find_map(|entry| {
                let Value::Message(pair) = entry else {
                    return None;
                };
                let session = pair.descriptor().get_field(PAIR_SESSION_TAG)?;
                match pair.get_field(&session).as_ref() {
                    Value::String(session) if !session.is_empty() => Some(session.clone()),
                    _ => None,
                }
            })
            .unwrap_or_default();
        for entry in &mut entries {
            let Value::Message(pair) = entry else {
                continue;
            };
            let Some(session) = pair.descriptor().get_field(PAIR_SESSION_TAG) else {
                continue;
            };
            pair.set_field(&session, Value::String(session_id.clone()));
        }
        message.set_field(&ids, Value::List(entries));
    }
}

// Hand-written `Message` impls register through the same `register!` macro the derive emits
// (round-trip/`Normalize` hooks + extern-map entry).
crate::register!(message: Request, "armonik.api.grpc.v1.agent.NotifyResultDataRequest");

// The field reflection a derive would emit (the `prost::Message` impl above is hand-written),
// consumed by the `service!` convenience emission.
#[doc(hidden)]
macro_rules! __armonik_fields_request {
    ($($cont:tt)::* ! { $($ctx:tt)* }) => {
        $($cont)::* ! { $($ctx)*
            fields { [communication_token into] [session_id into] [result_ids iter] }
        }
    };
}
#[doc(hidden)]
#[cfg(feature = "_gen-client")]
pub(crate) use __armonik_fields_request;

#[doc(hidden)]
#[allow(non_camel_case_types, dead_code)]
pub(crate) type __armonik_ty_request_communication_token = String;
#[doc(hidden)]
#[allow(non_camel_case_types, dead_code)]
pub(crate) type __armonik_ty_request_session_id = String;
#[doc(hidden)]
#[allow(non_camel_case_types, dead_code)]
pub(crate) type __armonik_ty_request_result_ids = Vec<String>;
#[doc(hidden)]
#[allow(non_camel_case_types, dead_code)]
pub(crate) type __armonik_ty_request_result_ids_elem = String;

// The one-line `Msg` marker every message-shaped type carries: `service!`'s const asserts read
// `NAMES` from it. It also grants the blanket `ProtoField` (nesting as a field), which is moot: no
// proto message has a field of this type, and the derives validate field shapes against the
// descriptor.
impl crate::codec::Msg for Request {
    const NAMES: &'static [&'static str] = &["armonik.api.grpc.v1.agent.NotifyResultDataRequest"];
}

// The `ResultIdentifier` pair message is flattened into the request's shared session ID and result
// IDs, so no Rust type stands for it. Declared absorbed the same way a `with` adapter would.
crate::register!(absorbed: "armonik.api.grpc.v1.agent.NotifyResultDataRequest.ResultIdentifier");

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.NotifyResultDataResponse")]
pub struct Response {
    pub result_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::Request;

    /// prost-derived reference of `NotifyResultDataRequest` and its `ResultIdentifier` pairs (both
    /// absorbed, so no generated type exists). An independent codec: the multi-pair fixture is
    /// encoded through it and decoded through our hand-written flattening `Request`, exercising the
    /// cross-pair session-collapse the field-information ratchet cannot probe.
    #[derive(Clone, PartialEq, Message)]
    struct RefIdentifier {
        #[prost(string, tag = "1")]
        session_id: String,
        #[prost(string, tag = "2")]
        result_id: String,
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefRequest {
        #[prost(message, repeated, tag = "1")]
        ids: Vec<RefIdentifier>,
        #[prost(string, tag = "4")]
        communication_token: String,
    }

    #[test]
    fn pairs_collapse_to_the_first_nonempty_session() {
        let reference = RefRequest {
            ids: vec![
                RefIdentifier {
                    session_id: String::new(),
                    result_id: "r0".to_owned(),
                },
                RefIdentifier {
                    session_id: "s1".to_owned(),
                    result_id: "r1".to_owned(),
                },
                RefIdentifier {
                    // A differing session on a later pair is dropped: the request carries one
                    // shared session id.
                    session_id: "s2".to_owned(),
                    result_id: "r2".to_owned(),
                },
            ],
            communication_token: "tok".to_owned(),
        };
        let request = Request::decode(reference.encode_to_vec().as_slice()).expect("decodes");
        assert_eq!(request.session_id, "s1");
        assert_eq!(request.result_ids, ["r0", "r1", "r2"]);
        assert_eq!(request.communication_token, "tok");
    }
}
