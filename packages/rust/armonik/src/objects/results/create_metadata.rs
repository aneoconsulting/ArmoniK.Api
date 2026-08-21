use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.results.CreateResultsMetaDataRequest.ResultCreate")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

#[armonik_macros::message("armonik.api.grpc.v1.results.CreateResultsMetaDataRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub session_id: String,
    pub results: Vec<RequestItem>,
}

#[armonik_macros::message("armonik.api.grpc.v1.results.CreateResultsMetaDataResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub results: Vec<Raw>,
}
