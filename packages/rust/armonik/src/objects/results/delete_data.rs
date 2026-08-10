#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.results.DeleteResultsDataRequest")]
pub struct Request {
    pub session_id: String,
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.results.DeleteResultsDataResponse")]
pub struct Response {
    pub session_id: String,
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}
