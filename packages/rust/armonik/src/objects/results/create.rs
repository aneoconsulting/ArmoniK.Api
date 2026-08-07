use super::Raw;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsRequest.ResultCreate")]
pub struct RequestItem {
    pub name: String,
    pub data: bytes::Bytes,
    pub manual_deletion: bool,
}

impl<K: Into<String>, V: Into<bytes::Bytes>> From<(K, V)> for RequestItem {
    fn from((name, data): (K, V)) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
            manual_deletion: false,
        }
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsRequest")]
pub struct Request {
    pub session_id: String,
    pub results: Vec<RequestItem>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsResponse")]
pub struct Response {
    pub results: Vec<Raw>,
}
