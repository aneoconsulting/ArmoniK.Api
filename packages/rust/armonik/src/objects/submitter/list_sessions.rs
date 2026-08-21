/// Request for listing sessions; stands in for the `SessionFilter` message at
/// the Submitter.ListSessions RPC.
#[armonik_macros::message("armonik.api.grpc.v1.submitter.SessionFilter")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(transparent)]
pub struct Request {
    pub filter: super::SessionFilter,
}

#[armonik_macros::message("armonik.api.grpc.v1.submitter.SessionIdList")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub session_ids: Vec<String>,
}
