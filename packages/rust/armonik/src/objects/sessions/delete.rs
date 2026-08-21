use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.sessions.DeleteSessionRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub session_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.sessions.DeleteSessionResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub session: Raw,
}
