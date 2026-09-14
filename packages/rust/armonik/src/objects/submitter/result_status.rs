use std::collections::HashMap;

use super::super::ResultStatus;

#[armonik_macros::message("armonik.api.grpc.v1.submitter.GetResultStatusRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub session_id: String,
    pub result_ids: Vec<String>,
}

#[armonik_macros::message("armonik.api.grpc.v1.submitter.GetResultStatusReply")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    /// The status of each result.
    #[armonik(rename = "id_statuses")]
    pub statuses: HashMap<String, ResultStatus>,
}
