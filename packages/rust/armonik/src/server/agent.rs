use std::sync::Arc;

use crate::agent;
use crate::api::v3;

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
        ) -> impl std::future::Future<
            Output = Result<agent::create_tasks::Response, tonic::Status>
        > + Send;

    }
}

pub trait AgentServiceExt {
    fn agent_server(self) -> v3::agent::agent_server::AgentServer<Self>
    where
        Self: Sized;
}

impl<T: AgentService + Send + Sync + 'static> AgentServiceExt for T {
    fn agent_server(self) -> v3::agent::agent_server::AgentServer<Self> {
        v3::agent::agent_server::AgentServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::agent::agent_server::Agent) for AgentService {
        fn create_results_meta_data(v3::agent::CreateResultsMetaDataRequest) -> v3::agent::CreateResultsMetaDataResponse { create_results_metadata }
        fn create_results(v3::agent::CreateResultsRequest) -> v3::agent::CreateResultsResponse { create_results }
        fn notify_result_data(v3::agent::NotifyResultDataRequest) -> v3::agent::NotifyResultDataResponse { notify_result_data }
        fn submit_tasks(v3::agent::SubmitTasksRequest) -> v3::agent::SubmitTasksResponse { submit_tasks }
        fn get_resource_data(v3::agent::DataRequest) -> v3::agent::DataResponse { get_resource_data }
        fn get_common_data(v3::agent::DataRequest) -> v3::agent::DataResponse { get_common_data }
        fn get_direct_data(v3::agent::DataRequest) -> v3::agent::DataResponse { get_direct_data }

        ---

        async fn create_task(
            self: std::sync::Arc<Self>,
            request: tonic::Request<tonic::Streaming<v3::agent::CreateTaskRequest>>,
        ) -> std::result::Result<tonic::Response<v3::agent::CreateTaskReply>, tonic::Status> {
            crate::server::impl_trait_methods!(stream client (self, request) {AgentService::create_tasks})
        }
    }
}
