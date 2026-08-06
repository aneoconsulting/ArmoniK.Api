use crate::rpc::services;

/// The HealthChecksService provides methods to verify the health of the
/// cluster.
pub type HealthChecks<T = tonic::transport::Channel> =
    super::ServiceClient<services::HealthChecks, T>;
