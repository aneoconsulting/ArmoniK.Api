use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.sessions.CloseSessionRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub session_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.sessions.CloseSessionResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub session: Raw,
}
