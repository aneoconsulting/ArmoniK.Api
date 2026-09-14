/// Shares its wire form (`DataRequest`) with the other data RPCs; a type of
/// its own keeps the request types injective over RPCs, so `call` deduces this
/// one from its argument.
#[armonik_macros::message("armonik.api.grpc.v1.agent.DataRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub communication_token: String,
    pub result_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.agent.DataResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub result_id: String,
}
