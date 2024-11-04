use std::collections::HashMap;

use futures::{Stream, StreamExt};
use snafu::ResultExt;

use crate::agent::{
    create_results, create_results_metadata, create_tasks, get_common_data, get_direct_data,
    get_resource_data, notify_result_data, submit_tasks, ResultMetaData,
};
use crate::api::v3;
use crate::utils::IntoCollection;
use crate::TaskOptions;

use super::{GrpcCall, GrpcCallStream};

/// The ResultsService provides methods for interacting with results.
#[derive(Clone)]
pub struct AgentClient<T> {
    inner: v3::agent::agent_client::AgentClient<T>,
}

impl<T> AgentClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::agent::agent_client::AgentClient::new(channel),
        }
    }

    /// Create the metadata of multiple results at once.
    /// Data have to be uploaded separately.
    pub async fn create_results_metadata(
        &mut self,
        token: impl Into<String>,
        session_id: impl Into<String>,
        names: impl std::iter::IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, ResultMetaData>, super::RequestError> {
        Ok(self
            .call(create_results_metadata::Request {
                communication_token: token.into(),
                names: names.into_collect(),
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
    ) -> Result<HashMap<String, ResultMetaData>, super::RequestError> {
        Ok(self
            .call(create_results::Request {
                communication_token: token.into(),
                results: results
                    .into_iter()
                    .map(|(name, data)| (name.into(), data.into()))
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

    /// Perform a gRPC call from a raw request.
    pub async fn call<Request>(
        &mut self,
        request: Request,
    ) -> Result<<&mut Self as GrpcCall<Request>>::Response, <&mut Self as GrpcCall<Request>>::Error>
    where
        for<'a> &'a mut Self: GrpcCall<Request>,
    {
        <&mut Self as GrpcCall<Request>>::call(self, request).await
    }
}

super::impl_call! {
    AgentClient {
        async fn call(self, request: create_results_metadata::Request) -> Result<create_results_metadata::Response> {
            Ok(self
                .inner
                .create_results_meta_data(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: create_results::Request) -> Result<create_results::Response> {
            Ok(self
                .inner
                .create_results(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: notify_result_data::Request) -> Result<notify_result_data::Response> {
            Ok(self
                .inner
                .notify_result_data(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: submit_tasks::Request) -> Result<submit_tasks::Response> {
            Ok(self
                .inner
                .submit_tasks(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get_resource_data::Request) -> Result<get_resource_data::Response> {
            Ok(self
                .inner
                .get_resource_data(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner().into())
        }

        async fn call(self, request: get_common_data::Request) -> Result<get_common_data::Response> {
            Ok(self
                .inner
                .get_common_data(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner().into())
        }

        async fn call(self, request: get_direct_data::Request) -> Result<get_direct_data::Response> {
            Ok(self
                .inner
                .get_direct_data(request)
                .await
                .context(super::GrpcSnafu{})?
                .into_inner().into())
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<T, S> GrpcCallStream<create_tasks::Request, S> for &'_ mut AgentClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    S: Stream<Item = create_tasks::Request> + Send + 'static,
{
    type Response = create_tasks::Response;
    type Error = super::RequestError;

    async fn call(self, request: S) -> Result<Self::Response, Self::Error> {
        Ok(self
            .inner
            .create_task(request.map(Into::into))
            .await
            .context(super::GrpcSnafu {})?
            .into_inner()
            .into())
    }
}

#[cfg(test)]
#[serial_test::serial(agent)]
mod tests {
    use std::collections::HashMap;

    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn create_results_metadata() {
        let before = Client::get_nb_request("Agent", "CreateResultsMetaData").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .create_results_metadata("token", "session-id", ["result1", "result2"])
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "CreateResultsMetaData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_results() {
        let before = Client::get_nb_request("Agent", "CreateResults").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .create_results("token", "session-id", [("result1", "payload")])
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "CreateResults").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn notify_result_data() {
        let before = Client::get_nb_request("Agent", "NotifyResultData").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .notify_result_data("token", "session-id", ["result1", "result2"])
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "NotifyResultData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn submit() {
        let before = Client::get_nb_request("Agent", "SubmitTasks").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .submit_tasks("token", "session-id", None, [])
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "SubmitTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_tasks() {
        let before = Client::get_nb_request("Agent", "CreateTask").await;
        let mut client = Client::new().await.unwrap().agent();

        client
            .create_tasks(futures::stream::iter([
                crate::agent::create_tasks::Request::Invalid,
            ]))
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "CreateTask").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn create_results_metadata_call() {
        let before = Client::get_nb_request("Agent", "CreateResultsMetaData").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .call(crate::agent::create_results_metadata::Request {
                communication_token: String::from("token"),
                session_id: String::from("session-id"),
                names: Vec::new(),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "CreateResultsMetaData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_results_call() {
        let before = Client::get_nb_request("Agent", "CreateResults").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .call(crate::agent::create_results::Request {
                communication_token: String::from("token"),
                session_id: String::from("session-id"),
                results: HashMap::new(),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "CreateResults").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn notify_result_data_call() {
        let before = Client::get_nb_request("Agent", "NotifyResultData").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .call(crate::agent::notify_result_data::Request {
                communication_token: String::from("token"),
                session_id: String::from("session-id"),
                result_ids: vec![],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "NotifyResultData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn submit_tasks_call() {
        let before = Client::get_nb_request("Agent", "SubmitTasks").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .call(crate::agent::submit_tasks::Request {
                communication_token: String::from("token"),
                session_id: String::from("session-id"),
                task_options: None,
                items: vec![],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "SubmitTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_resource_data_call() {
        let before = Client::get_nb_request("Agent", "GetResourceData").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .call(crate::agent::get_resource_data::Request {
                communication_token: String::from("token"),
                result_id: String::from("result-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "GetResourceData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_common_data_call() {
        let before = Client::get_nb_request("Agent", "GetCommonData").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .call(crate::agent::get_common_data::Request {
                communication_token: String::from("token"),
                result_id: String::from("result-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "GetCommonData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_direct_data_call() {
        let before = Client::get_nb_request("Agent", "GetDirectData").await;
        let mut client = Client::new().await.unwrap().agent();
        client
            .call(crate::agent::get_direct_data::Request {
                communication_token: String::from("token"),
                result_id: String::from("result-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "GetDirectData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_tasks_call() {
        let before = Client::get_nb_request("Agent", "CreateTask").await;
        let mut client = Client::new().await.unwrap().agent();

        client
            .call(futures::stream::iter([
                crate::agent::create_tasks::Request::Invalid,
            ]))
            .await
            .unwrap();
        let after = Client::get_nb_request("Agent", "CreateTask").await;
        assert_eq!(after - before, 1);
    }
}
