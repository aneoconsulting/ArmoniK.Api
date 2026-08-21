/// Request for cancelling a session; stands in for the `Session` message at
/// the Submitter.CancelSession RPC.
#[armonik_macros::message("armonik.api.grpc.v1.Session")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    #[armonik(rename = "id")]
    pub session_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.Empty")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {}
