use super::{super::StatusCount, filter};

#[armonik_macros::message("armonik.api.grpc.v1.tasks.CountTasksByStatusRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub filters: filter::Or,
}

#[armonik_macros::message("armonik.api.grpc.v1.tasks.CountTasksByStatusResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub status: Vec<StatusCount>,
}
