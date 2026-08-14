use super::Raw;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.UploadResultDataRequest")]
pub enum Request {
    /// No member set.
    ///
    /// The absence used to decode to an `Identifier` with an empty session and result id, which
    /// names a result rather than saying that the message named none.
    #[default]
    Invalid,
    #[armonik(rename = "id", inline)]
    Identifier {
        session_id: String,
        result_id: String,
    },
    DataChunk(bytes::Bytes),
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.UploadResultDataResponse")]
pub struct Response {
    pub result: Raw,
}
