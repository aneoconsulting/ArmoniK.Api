use futures::Stream;
use snafu::ResultExt;

use crate::agent::{
    create_results, create_results_metadata, create_tasks, notify_result_data, submit_tasks,
    ResultMetaData,
};
use crate::rpc::services;
use crate::utils::IntoCollection;
use crate::TaskOptions;

/// The Agent gRPC service, exposed to workers for spawning subtasks and
/// exchanging data.
pub type Agent<T = tonic::transport::Channel> = super::ServiceClient<services::Agent, T>;

impl<T: super::Channel> super::ServiceClient<services::Agent, T> {
    /// Create the metadata of multiple results at once.
    /// Data have to be uploaded separately.
    pub async fn create_results_metadata(
        &mut self,
        token: impl Into<String>,
        session_id: impl Into<String>,
        names: impl std::iter::IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<ResultMetaData>, super::RequestError> {
        Ok(self
            .call(create_results_metadata::Request {
                communication_token: token.into(),
                results: names.into_iter().map(|name| name.into().into()).collect(),
                session_id: session_id.into(),
            })
            .await?
            .results)
    }

    /// Create multiple results with data included in the request.
    pub async fn create_results(
        &mut self,
        token: impl Into<String>,
        session_id: impl Into<String>,
        results: impl std::iter::IntoIterator<Item = (impl Into<String>, impl Into<Vec<u8>>)>,
    ) -> Result<Vec<ResultMetaData>, super::RequestError> {
        Ok(self
            .call(create_results::Request {
                communication_token: token.into(),
                results: results
                    .into_iter()
                    .map(|(name, data)| (name.into(), data.into()).into())
                    .collect(),
                session_id: session_id.into(),
            })
            .await?
            .results)
    }

    /// Notify results data are available in files.
    pub async fn notify_result_data(
        &mut self,
        token: impl Into<String>,
        session_id: impl Into<String>,
        result_ids: impl std::iter::IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<String>, super::RequestError> {
        Ok(self
            .call(notify_result_data::Request {
                communication_token: token.into(),
                session_id: session_id.into(),
                result_ids: result_ids.into_collect(),
            })
            .await?
            .result_ids)
    }

    /// Create tasks metadata and submit task for processing.
    pub async fn submit_tasks(
        &mut self,
        token: impl Into<String>,
        session_id: impl Into<String>,
        task_options: Option<TaskOptions>,
        items: impl IntoIterator<Item = submit_tasks::RequestItem>,
    ) -> Result<Vec<submit_tasks::ResponseItem>, super::RequestError> {
        Ok(self
            .call(submit_tasks::Request {
                communication_token: token.into(),
                session_id: session_id.into(),
                task_options,
                items: items.into_collect(),
            })
            .await?
            .items)
    }

    pub async fn create_tasks(
        &mut self,
        request: impl Stream<Item = create_tasks::Request> + Send + 'static,
    ) -> Result<Vec<create_tasks::Status>, super::RequestError> {
        let response = self.call_streaming(request).await?;

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

#[cfg(test)]
#[serial_test::serial(agent)]
mod tests {
    use crate::Client;


    #[tokio::test]
    async fn submit() {
        let before = Client::get_nb_request("Agent", "SubmitTasks").await;
        let mut client = Client::new().await.unwrap().into_agent();
        client
            .submit_tasks("token", "session-id", None, [])
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "SubmitTasks").await;
        assert_eq!(after - before, 1);
    }
}
