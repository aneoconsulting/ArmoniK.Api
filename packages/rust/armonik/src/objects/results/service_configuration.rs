use crate::api::v3;

/// Response for obtaining results service configuration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultsServiceConfiguration {
    /// Maximum size supported by a data chunk for the result service.
    pub data_chunk_max_size: i32,
}

impl Default for ResultsServiceConfiguration {
    fn default() -> Self {
        Self {
            data_chunk_max_size: 64 * 1024,
        }
    }
}

impl From<ResultsServiceConfiguration> for v3::results::ResultsServiceConfigurationResponse {
    fn from(value: ResultsServiceConfiguration) -> Self {
        Self {
            data_chunk_max_size: value.data_chunk_max_size,
        }
    }
}

impl From<v3::results::ResultsServiceConfigurationResponse> for ResultsServiceConfiguration {
    fn from(value: v3::results::ResultsServiceConfigurationResponse) -> Self {
        Self {
            data_chunk_max_size: value.data_chunk_max_size,
        }
    }
}

super::super::impl_convert!(ResultsServiceConfiguration : Option<v3::results::ResultsServiceConfigurationResponse>);
