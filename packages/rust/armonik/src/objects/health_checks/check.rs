use crate::api::v3;

use super::ServiceHealth;

/// Request to check if all services are healthy.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

super::super::impl_convert!(
    struct Request = v3::health_checks::CheckHealthRequest {
    }
);

/// Response to check if all services are healthy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub services: Vec<ServiceHealth>,
}

super::super::impl_convert!(
    struct Response = v3::health_checks::CheckHealthResponse {
        list services,
    }
);
