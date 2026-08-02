use std::collections::HashMap;

use prost::bytes::{Buf, BufMut};
use prost::encoding::{self, DecodeContext, WireType};
use prost::DecodeError;

use crate::codec::{ProtoAdapter, ProtoField};

use super::Raw;

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.ImportResultsDataRequest")]
pub struct Request {
    /// The opaque ids associated to the results to import.
    #[armonik(
        with = "crate::codec::adapters::PairMap<1, 2>",
        absorbs = "armonik.api.grpc.v1.results.ImportResultsDataRequest.ResultOpaqueId"
    )]
    pub results: HashMap<String, bytes::Bytes>,
    /// The session in which create results.
    pub session_id: String,
}

/// Response for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.ImportResultsDataResponse")]
pub struct Response {
    /// The list of raw results that were created.
    #[armonik(with = "RawByName")]
    pub results: HashMap<String, Raw>,
}

/// `repeated ResultRaw` exposed as a `HashMap` keyed by the results' own
/// `name` field: entry order is not preserved and duplicate names collapse
/// (last wins), exactly like the historical conversion.
pub(crate) struct RawByName;

impl ProtoAdapter<HashMap<String, Raw>> for RawByName {
    fn encode_field(tag: u32, value: &HashMap<String, Raw>, buf: &mut impl BufMut) {
        for raw in value.values() {
            Raw::encode_field(tag, raw, buf);
        }
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut HashMap<String, Raw>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut raw = Raw::default();
        Raw::merge_field(wire_type, &mut raw, buf, ctx)?;
        value.insert(raw.name.clone(), raw);
        Ok(())
    }

    fn encoded_len_field(tag: u32, value: &HashMap<String, Raw>) -> usize {
        value
            .values()
            .map(|raw| Raw::encoded_len_field(tag, raw))
            .sum()
    }

    fn is_default(value: &HashMap<String, Raw>) -> bool {
        value.is_empty()
    }

    /// The `HashMap` loses entry order and collapses duplicate names.
    #[cfg(feature = "_differential")]
    fn normalize_dynamic(
        message: &mut crate::differential::prost_reflect::DynamicMessage,
        tag: u32,
    ) {
        crate::differential::fold_pairs_by_name(message, tag, "name");
    }
}
