use std::collections::HashMap;

use super::super::ResultStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetResultStatusRequest")]
pub struct Request {
    pub session_id: String,
    pub result_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetResultStatusReply")]
pub struct Response {
    /// The status of each result.
    #[armonik(rename = "id_statuses", with = "crate::codec::adapters::PairMap<1, 2>")]
    pub statuses: HashMap<String, ResultStatus>,
}
