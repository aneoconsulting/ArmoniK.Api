use crate::rpc::services;

/// Service for handling sessions.
pub type Sessions<T = tonic::transport::Channel> = super::ServiceClient<services::Sessions, T>;
