use crate::rpc::services;

/// Service for handling applications.
pub type Applications<T = tonic::transport::Channel> =
    super::ServiceClient<services::Applications, T>;
