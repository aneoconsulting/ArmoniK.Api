use super::Raw;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest.ResultCreate")]
pub struct RequestItem {
        pub name: String,
    pub manual_deletion: bool,
}

impl<T: Into<String>> From<T> for RequestItem {
    fn from(value: T) -> Self {
        Self {
            name: value.into(),
            manual_deletion: false,
        }
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest")]
pub struct Request {
    pub session_id: String,
    pub results: Vec<RequestItem>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsMetaDataResponse")]
pub struct Response {
    pub results: Vec<Raw>,
}
