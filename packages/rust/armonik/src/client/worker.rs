use crate::rpc::services;
use crate::worker::{health_check, process};
use crate::Output;

/// The Worker gRPC service, called by the agent to run tasks.
pub type Worker<T = tonic::transport::Channel> = super::ServiceClient<services::Worker, T>;

impl<T: super::Channel> super::ServiceClient<services::Worker, T> {
    pub async fn health_check(&mut self) -> Result<health_check::Response, super::RequestError> {
        self.call(health_check::Request {}).await
    }

    pub async fn process(
        &mut self,
        request: process::Request,
    ) -> Result<Output, super::RequestError> {
        Ok(self.call(request).await?.output)
    }
}
