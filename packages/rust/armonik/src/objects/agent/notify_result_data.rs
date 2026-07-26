use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use crate::codec::ProtoField;

/// Request for notifying results data are available in files.
///
/// The proto message carries `repeated ResultIdentifier { session_id,
/// result_id }` pairs, flattened here into one session ID shared by all the
/// results: the wire implementation is hand-written. Encoding replicates the
/// session ID in every pair; decoding keeps the first non-empty one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Communication token received by the worker during task processing.
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
        let mut len = 0;
        if !self.session_id.is_empty() {
            len += <String as ProtoField>::encoded_len_field(PAIR_SESSION_TAG, &self.session_id);
        }
        if !result_id.is_empty() {
            len += <String as ProtoField>::encoded_len_field(PAIR_RESULT_TAG, result_id);
        }
        len
    }
}

// Hand-written rather than derived. The derive (and its `with` adapters)
// maps one Rust field to one proto field, but `ids` splits into TWO Rust
// fields: each encoded pair combines the shared `session_id` with one
// element of `result_ids`, and each decoded pair updates both — the
// first-non-empty-session rule reads the partially merged struct itself.
// Teaching the derive this one-off shape would cost more than the impl
// (see `tasks::Output` for the full tradeoff); the differential harness
// fuzzes it like any derived type, and by not implementing `ProtoField`
// the type cannot be nested in other messages.
impl prost::Message for Request {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        for result_id in &self.result_ids {
            let pair_len = self.pair_len(result_id);
            encoding::encode_key(IDS_TAG, WireType::LengthDelimited, buf);
            encoding::encode_varint(pair_len as u64, buf);
            if !self.session_id.is_empty() {
                <String as ProtoField>::encode_field(PAIR_SESSION_TAG, &self.session_id, buf);
            }
            if !result_id.is_empty() {
                <String as ProtoField>::encode_field(PAIR_RESULT_TAG, result_id, buf);
            }
        }
        if !self.communication_token.is_empty() {
            <String as ProtoField>::encode_field(TOKEN_TAG, &self.communication_token, buf);
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
            IDS_TAG => {
                encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = encoding::decode_varint(buf)? as usize;
                if buf.remaining() < len {
                    // prost offers no other public constructor.
                    #[allow(deprecated)]
                    return Err(DecodeError::new("buffer underflow"));
                }
                let mut pair = buf.take(len);
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
        let mut len = 0;
        for result_id in &self.result_ids {
            let pair_len = self.pair_len(result_id);
            len += encoding::encoded_len_varint(u64::from(IDS_TAG) << 3)
                + encoding::encoded_len_varint(pair_len as u64)
                + pair_len;
        }
        if !self.communication_token.is_empty() {
            len += <String as ProtoField>::encoded_len_field(TOKEN_TAG, &self.communication_token);
        }
        len
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(feature = "_differential")]
const _: () = {
    #[linkme::distributed_slice(crate::differential::REGISTRY)]
    static ENTRY: crate::differential::Entry = crate::differential::Entry {
        proto: "armonik.api.grpc.v1.agent.NotifyResultDataRequest",
        roundtrip: |bytes| {
            Ok(prost::Message::encode_to_vec(
                &<Request as prost::Message>::decode(bytes)?,
            ))
        },
        default_encoding: || prost::Message::encode_to_vec(&Request::default()),
    };
};

/// Response for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.NotifyResultDataResponse")]
pub struct Response {
    /// The list of ResultMetaData results that were created.
    pub result_ids: Vec<String>,
}
