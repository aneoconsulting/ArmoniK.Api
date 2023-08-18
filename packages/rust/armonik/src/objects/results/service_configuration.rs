use crate::api::v3;

/// Response for obtaining results service configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

super::super::impl_convert!(
    struct Request = v3::Empty {
    }
);

/// Response for obtaining results service configuration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// Maximum size supported by a data chunk for the result service.
    pub data_chunk_max_size: i32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            data_chunk_max_size: 64 * 1024,
        }
    }
}

super::super::impl_convert!(
    struct Response = v3::results::ResultsServiceConfigurationResponse {
        data_chunk_max_size,
    }
);
