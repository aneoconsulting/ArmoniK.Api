use super::super::ResultStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.WatchResultRequest")]
pub struct Request {
    pub fetch_statuses: Vec<ResultStatus>,
    pub watch_statuses: Vec<ResultStatus>,
    pub result_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.WatchResultResponse")]
pub struct Response {
    pub status: ResultStatus,
    pub result_ids: Vec<String>,
}
