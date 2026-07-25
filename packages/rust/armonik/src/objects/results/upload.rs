use super::Raw;

/// The possible messages that constitute a UploadResultDataRequest
/// They should be sent in the following order:
/// - id
/// - data_chunk (stream can have multiple data_chunk messages that represent data divided in several parts)
///
/// Data chunk cannot exceed the size returned by the GetServiceConfiguration rpc method
#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.results.UploadResultDataRequest",
    oneof = "type"
)]
pub enum Request {
    /// The identifier of the result to which add data.
    #[armonik(rename = "id")]
    Identifier {
        /// The session of the result.
        session_id: String,
        /// The ID of the result.
        result_id: String,
    },
    /// A chunk of data.
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

#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.UploadResultDataResponse")]
pub struct Response {
    /// The metadata of the updated result that was updated.
    pub result: Raw,
}
