use crate::rpc::services;

/// The Worker gRPC service, called by the agent to run tasks.
pub type Worker<T = tonic::transport::Channel> = super::ServiceClient<services::Worker, T>;

impl<T: super::Channel> super::ServiceClient<services::Worker, T> {
    /// Process a task and return its output.
    pub async fn process(
        &mut self,
        request: crate::worker::process::Request,
    ) -> Result<crate::Output, super::RequestError> {
        Ok(self.call(request).await?.output)
    }
}
