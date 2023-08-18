use crate::api::v3;

/// Response for obtaining results service configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

impl From<Request> for v3::Empty {
    fn from(_value: Request) -> Self {
        Self {}
    }
}

impl From<v3::Empty> for Request {
    fn from(_value: v3::Empty) -> Self {
        Self {}
    }
}

super::super::impl_convert!(Request : Option<v3::Empty>);

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

impl From<Response> for v3::results::ResultsServiceConfigurationResponse {
    fn from(value: Response) -> Self {
        Self {
            data_chunk_max_size: value.data_chunk_max_size,
        }
    }
}

impl From<v3::results::ResultsServiceConfigurationResponse> for Response {
    fn from(value: v3::results::ResultsServiceConfigurationResponse) -> Self {
        Self {
            data_chunk_max_size: value.data_chunk_max_size,
        }
    }
}

super::super::impl_convert!(Response : Option<v3::results::ResultsServiceConfigurationResponse>);
