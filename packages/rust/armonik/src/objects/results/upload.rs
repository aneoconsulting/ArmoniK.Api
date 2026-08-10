use super::Raw;

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.UploadResultDataRequest")]
pub enum Request {
    #[armonik(rename = "id")]
    Identifier {
        session_id: String,
        result_id: String,
    },
    DataChunk(bytes::Bytes),
}

impl Default for Request {
    fn default() -> Self {
        Self::Identifier {
            session_id: Default::default(),
            result_id: Default::default(),
        }
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.UploadResultDataResponse")]
pub struct Response {
    pub result: Raw,
}
