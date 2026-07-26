use crate::api::v3;

/// Response for obtaining results service configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {}

// The generated `Empty` stays: several `{}` request/response types across
// services share it, so it cannot be extern'd to a single armonik type.
impl From<Request> for v3::Empty {
    fn from(_: Request) -> Self {
        Self {}
    }
}

impl From<v3::Empty> for Request {
    fn from(_: v3::Empty) -> Self {
        Self {}
    }
}

/// Response for obtaining results service configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.ResultsServiceConfigurationResponse")]
pub struct Response {
    /// Maximum size supported by a data chunk for the result service.
    pub data_chunk_max_size: i32,
}
