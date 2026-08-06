use crate::rpc::services;

/// The PartitionsService provides methods for interacting with partitions.
pub type Partitions<T = tonic::transport::Channel> = super::ServiceClient<services::Partitions, T>;
