/// Shares its wire form (`DataRequest`) with the other data RPCs; the build
/// script gives this RPC a distinct synthetic stub message so the calls stay
/// fully distinct types.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.DataRequest")]
pub struct Request {
    pub communication_token: String,
    pub result_id: String,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.DataResponse")]
pub struct Response {
    pub result_id: String,
}
