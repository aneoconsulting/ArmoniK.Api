use super::super::ResultStatus;

#[armonik_macros::message("armonik.api.grpc.v1.results.WatchResultRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub fetch_statuses: Vec<ResultStatus>,
    pub watch_statuses: Vec<ResultStatus>,
    pub result_ids: Vec<String>,
}

#[armonik_macros::message("armonik.api.grpc.v1.results.WatchResultResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub status: ResultStatus,
    pub result_ids: Vec<String>,
}
