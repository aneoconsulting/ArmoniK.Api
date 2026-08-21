use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.results.UploadResultDataRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Request {
    /// No member set, which an `Identifier` of two empty ids is not.
    #[default]
    Invalid,
    #[armonik(rename = "id", inlined)]
    Identifier {
        session_id: String,
        result_id: String,
    },
    DataChunk(bytes::Bytes),
}

#[armonik_macros::message("armonik.api.grpc.v1.results.UploadResultDataResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub result: Raw,
}
