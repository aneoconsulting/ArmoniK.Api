use futures::Stream;
use snafu::ResultExt;

use crate::agent::create_tasks;
use crate::client::client_method;
use crate::rpc::services;

/// The Agent gRPC service, exposed to workers for spawning subtasks and
/// exchanging data. (The proto documents the service with nothing at all, so this
/// sentence is the crate's own, and stays here rather than being harvested.)
pub use crate::rpc::agent::Client as Agent;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.agent.Agent")]
impl<T: super::Channel> super::ServiceClient<services::Agent, T> {
    /// Submit tasks as a request stream, turning the reply's error member into a `Status`.
    #[armonik(rpc = "CreateTask")]
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

    client_method!(CreateResultsMetaData:
        create_results_metadata(communication_token: into<String>, session_id: into<String>, results: iter<crate::agent::create_results_metadata::RequestItem>)
        -> crate::agent::create_results_metadata::Request => results: Vec<crate::agent::ResultMetaData>);
    client_method!(CreateResults:
        create_results(communication_token: into<String>, session_id: into<String>, results: iter<crate::agent::create_results::RequestItem>)
        -> crate::agent::create_results::Request => results: Vec<crate::agent::ResultMetaData>);
    client_method!(NotifyResultData:
        notify_result_data(communication_token: into<String>, session_id: into<String>, result_ids: iter<String>)
        -> crate::agent::notify_result_data::Request => result_ids: Vec<String>);
    client_method!(SubmitTasks:
        submit_tasks(communication_token: into<String>, session_id: into<String>, task_options: plain<Option<crate::TaskOptions>>, items: iter<crate::agent::submit_tasks::RequestItem>)
        -> crate::agent::submit_tasks::Request => items: Vec<crate::agent::submit_tasks::ResponseItem>);
    client_method!(GetResourceData:
        get_resource_data(communication_token: into<String>, result_id: into<String>)
        -> crate::agent::get_resource_data::Request => result_id: String);
    client_method!(GetCommonData:
        get_common_data(communication_token: into<String>, result_id: into<String>)
        -> crate::agent::get_common_data::Request => result_id: String);
    client_method!(GetDirectData:
        get_direct_data(communication_token: into<String>, result_id: into<String>)
        -> crate::agent::get_direct_data::Request => result_id: String);
}
