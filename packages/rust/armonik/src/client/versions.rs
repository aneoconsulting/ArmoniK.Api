use crate::rpc::services;

/// Service for getting versions of the components.
pub type Versions<T = tonic::transport::Channel> = super::ServiceClient<services::Versions, T>;
