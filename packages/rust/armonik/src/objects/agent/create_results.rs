use super::ResultMetaData;

#[armonik_macros::message("armonik.api.grpc.v1.agent.CreateResultsRequest.ResultCreate")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestItem {
    pub name: String,
    pub data: bytes::Bytes,
}

impl<K: Into<String>, V: Into<bytes::Bytes>> From<(K, V)> for RequestItem {
    fn from((name, data): (K, V)) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
        }
    }
}

#[armonik_macros::message("armonik.api.grpc.v1.agent.CreateResultsRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub communication_token: String,
    pub session_id: String,
    pub results: Vec<RequestItem>,
}

#[armonik_macros::message("armonik.api.grpc.v1.agent.CreateResultsResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub communication_token: String,
    pub results: Vec<ResultMetaData>,
}
