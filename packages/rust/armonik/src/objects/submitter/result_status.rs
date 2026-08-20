use std::collections::HashMap;

use super::super::ResultStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetResultStatusRequest")]
pub struct Request {
    pub session_id: String,
    pub result_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetResultStatusReply")]
pub struct Response {
    /// The status of each result.
    #[armonik(rename = "id_statuses", inlined)]
    pub statuses: HashMap<String, ResultStatus>,
}
