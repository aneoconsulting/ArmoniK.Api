/// Request for getting a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.DownloadResultDataRequest")]
pub struct Request {
    /// The session of the result.
    pub session_id: String,
    /// The ID of the result.
    pub result_id: String,
}

/// Response for getting a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.DownloadResultDataResponse")]
pub struct Response {
    /// A chunk of data.
    pub data_chunk: bytes::Bytes,
}
