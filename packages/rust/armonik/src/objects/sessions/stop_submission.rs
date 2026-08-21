use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.sessions.StopSubmissionRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub session_id: String,
    pub client: bool,
    pub worker: bool,
}

#[armonik_macros::message("armonik.api.grpc.v1.sessions.StopSubmissionResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub session: Raw,
}
