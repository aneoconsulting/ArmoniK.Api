use super::Raw;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.sessions.StopSubmissionRequest")]
pub struct Request {
    pub session_id: String,
    pub client: bool,
    pub worker: bool,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.sessions.StopSubmissionResponse")]
pub struct Response {
    pub session: Raw,
}
