use crate::rpc::services;

/// Service for authentication management.
pub type Auth<T = tonic::transport::Channel> = super::ServiceClient<services::Auth, T>;
