use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.results.GetResultRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    #[armonik(rename = "result_id")]
    pub id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.results.GetResultResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub result: Raw,
}
