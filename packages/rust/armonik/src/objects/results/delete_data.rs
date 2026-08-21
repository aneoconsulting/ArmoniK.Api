#[armonik_macros::message("armonik.api.grpc.v1.results.DeleteResultsDataRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub session_id: String,
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}

#[armonik_macros::message("armonik.api.grpc.v1.results.DeleteResultsDataResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub session_id: String,
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}
