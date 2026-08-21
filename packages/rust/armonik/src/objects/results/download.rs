#[armonik_macros::message("armonik.api.grpc.v1.results.DownloadResultDataRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub session_id: String,
    pub result_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.results.DownloadResultDataResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub data_chunk: bytes::Bytes,
}
