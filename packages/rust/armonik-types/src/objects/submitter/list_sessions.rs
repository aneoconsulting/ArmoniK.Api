/// Request for listing sessions; stands in for the `SessionFilter` message at
/// the Submitter.ListSessions RPC.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.submitter.SessionFilter")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.ListSessionsRequest",
    service = "Submitter",
    method = "ListSessions",
    input,
))]
pub struct Request {
    pub filter: super::SessionFilter,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.SessionIdList")]
pub struct Response {
    pub session_ids: Vec<String>,
}
