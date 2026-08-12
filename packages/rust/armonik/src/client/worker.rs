use crate::rpc::services;

/// The Worker gRPC service, called by the agent to run tasks. (The proto documents
/// the service with nothing at all, so this sentence is the crate's own.)
pub use crate::rpc::worker::Client as Worker;

impl<T: super::Channel> super::ServiceClient<services::Worker, T> {
    /// Process a task and return its output.
    pub async fn process(
        &mut self,
        request: crate::worker::process::Request,
    ) -> Result<crate::Output, super::RequestError> {
        Ok(self.call(request).await?.output)
    }
}
