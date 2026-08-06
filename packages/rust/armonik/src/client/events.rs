use crate::rpc::services;

/// Service for subscribing to events representing modifications to ArmoniK
/// result and task data.
pub type Events<T = tonic::transport::Channel> = super::ServiceClient<services::Events, T>;
