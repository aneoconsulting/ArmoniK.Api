/// Request to retrieve data.
///
/// Shares its wire form (`DataRequest`) with the other data RPCs; the build
/// script gives this RPC a distinct synthetic stub message so the calls stay
/// fully distinct types.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.DataRequest")]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// Id of the result that will be retrieved.
    pub result_id: String,
}

/// Response when data is available in the shared folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.DataResponse")]
pub struct Response {
    /// Id of the result that will be retrieved.
    pub result_id: String,
}
