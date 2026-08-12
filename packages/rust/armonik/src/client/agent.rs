use futures::Stream;
use snafu::ResultExt;

use crate::agent::create_tasks;
use crate::rpc::services;

/// The Agent gRPC service, exposed to workers for spawning subtasks and
/// exchanging data. (The proto documents the service with nothing at all, so this
/// sentence is the crate's own, and stays here rather than being harvested.)
pub use crate::rpc::agent::Client as Agent;

impl<T: super::Channel> super::ServiceClient<services::Agent, T> {
    pub async fn create_tasks(
        &mut self,
        request: impl Stream<Item = create_tasks::Request> + Send + 'static,
    ) -> Result<Vec<create_tasks::Status>, super::RequestError> {
        let response = self.call(request).await?;

        match response {
            create_tasks::Response::Status {
                communication_token: _,
                statuses,
            } => Ok(statuses),
            create_tasks::Response::Error {
                communication_token: _,
                error,
            } => Err(tonic::Status::internal(error)).context(super::GrpcSnafu {}),
        }
    }
}
