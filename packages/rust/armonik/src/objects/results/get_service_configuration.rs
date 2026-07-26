/// Response for obtaining results service configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {}

impl From<Request> for crate::Empty {
    fn from(_: Request) -> Self {
        Self {}
    }
}

impl From<crate::Empty> for Request {
    fn from(_: crate::Empty) -> Self {
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
