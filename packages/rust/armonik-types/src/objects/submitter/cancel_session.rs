/// Request for cancelling a session; stands in for the `Session` message at
/// the Submitter.CancelSession RPC.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Session")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.CancelSessionRequest",
    service = "Submitter",
    method = "CancelSession",
    input,
))]
pub struct Request {
    #[armonik(rename = "id")]
    pub session_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Empty")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.CancelSessionResponse",
    service = "Submitter",
    method = "CancelSession",
    output,
))]
pub struct Response {}
