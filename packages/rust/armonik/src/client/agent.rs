use std::collections::HashMap;

use futures::{Stream, StreamExt};
use snafu::ResultExt;

use crate::objects::agent::{
    create_results, create_results_metadata, create_tasks, get_common_data, get_direct_data,
    get_resource_data, notify_result_data, submit_tasks, ResultMetaData,
};
use crate::objects::TaskOptions;
use crate::utils::IntoCollection;

use crate::api::v3;

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
                results: names.into_collect(),
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

    /// Create tasks metadata and submit task for processing.
    pub async fn submit(
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
