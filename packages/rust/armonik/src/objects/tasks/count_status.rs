use super::{super::StatusCount, filter};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.tasks.CountTasksByStatusRequest")]
pub struct Request {
    pub filters: filter::Or,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.tasks.CountTasksByStatusResponse")]
pub struct Response {
    pub status: Vec<StatusCount>,
}
