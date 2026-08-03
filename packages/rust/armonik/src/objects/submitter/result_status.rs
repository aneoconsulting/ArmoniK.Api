use std::collections::HashMap;

use super::super::ResultStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetResultStatusRequest")]
pub struct Request {
    pub session_id: String,
    pub result_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetResultStatusReply")]
pub struct Response {
    /// The status of each result.
    #[armonik(
        rename = "id_statuses",
        with = "crate::codec::adapters::PairMap<1, 2>",
        absorbs = "armonik.api.grpc.v1.submitter.GetResultStatusReply.IdStatus"
    )]
    pub statuses: HashMap<String, ResultStatus>,
}
