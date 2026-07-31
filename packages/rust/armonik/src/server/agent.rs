use std::sync::Arc;

use crate::agent;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::agent::agent_server as stub;

super::define_trait_methods! {
    trait AgentService {
        /// Create the metadata of multiple results at once.
        /// Data have to be uploaded separately.
        fn agent::create_results_metadata;

        /// Create multiple results with data included in the request.
        fn agent::create_results;

        /// Notify results data are available in files.
        fn agent::notify_result_data;

        /// Create tasks metadata and submit task for processing.
        fn agent::submit_tasks;

        /// Retrieve Resource Data from the Agent
        /// Data is stored in the shared folder between Agent and Worker as a file with the result id as name
        /// Blocks until data are available in the shared folder
        fn agent::get_resource_data;

        /// Retrieve Resource Data from the Agent
        /// Data is stored in the shared folder between Agent and Worker as a file with the result id as name
        /// Blocks until data are available in the shared folder
        fn agent::get_common_data;

        /// Retrieve Resource Data from the Agent
        /// Data is stored in the shared folder between Agent and Worker as a file with the result id as name
        /// Blocks until data are available in the shared folder
        fn agent::get_direct_data;

        ---

        fn create_tasks(
            self: Arc<Self>,
            request: impl tonic::codegen::tokio_stream::Stream<Item = Result<agent::create_tasks::Request, tonic::Status>> + Send + 'static,
            context: crate::server::RequestContext,
        ) -> impl std::future::Future<
            Output = Result<agent::create_tasks::Response, tonic::Status>
        > + Send;

    }
}

pub trait AgentServiceExt {
    fn agent_server(self) -> stub::AgentServer<Self>
    where
        Self: Sized;
}

impl<T: AgentService + Send + Sync + 'static> AgentServiceExt for T {
    fn agent_server(self) -> stub::AgentServer<Self> {
        stub::AgentServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Agent) for AgentService {
        fn create_results_meta_data(crate::agent::create_results_metadata::Request) -> crate::agent::create_results_metadata::Response { create_results_metadata }
        fn create_results(crate::agent::create_results::Request) -> crate::agent::create_results::Response { create_results }
        fn notify_result_data(crate::agent::notify_result_data::Request) -> crate::agent::notify_result_data::Response { notify_result_data }
        fn submit_tasks(crate::agent::submit_tasks::Request) -> crate::agent::submit_tasks::Response { submit_tasks }
        fn get_resource_data(crate::agent::get_resource_data::Request) -> crate::agent::get_resource_data::Response { get_resource_data }
        fn get_common_data(crate::agent::get_common_data::Request) -> crate::agent::get_common_data::Response { get_common_data }
        fn get_direct_data(crate::agent::get_direct_data::Request) -> crate::agent::get_direct_data::Response { get_direct_data }

        ---

        async fn create_task(
            self: std::sync::Arc<Self>,
            // Extern'd types: the generated stub speaks the armonik types
            // directly, no conversion left on this path.
            request: tonic::Request<tonic::Streaming<agent::create_tasks::Request>>,
        ) -> std::result::Result<tonic::Response<agent::create_tasks::Response>, tonic::Status> {
            crate::server::impl_trait_methods!(stream client (self, request) {AgentService::create_tasks})
        }
    }
}
