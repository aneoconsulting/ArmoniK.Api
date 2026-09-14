use crate::client::client_method;
use crate::rpc::services;

/// The Worker gRPC service, called by the agent to run tasks. (The proto documents
/// the service with nothing at all, so this sentence is the crate's own.)
pub use crate::rpc::worker::Client as Worker;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.worker.Worker")]
impl<T: super::Channel> super::ServiceClient<services::Worker, T> {
    /// Process a task and return its output.
    #[armonik(rpc = "Process")]
    pub async fn process(
        &mut self,
        request: crate::worker::process::Request,
    ) -> Result<crate::Output, super::RequestError> {
        Ok(self.call(request).await?.output)
    }

    client_method!(HealthCheck:
        health_check()
        -> crate::worker::health_check::Request => crate::worker::health_check::Response);
}
