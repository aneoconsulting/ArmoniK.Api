use crate::rpc::services;

/// Service for handling tasks.
pub type Tasks<T = tonic::transport::Channel> = super::ServiceClient<services::Tasks, T>;
