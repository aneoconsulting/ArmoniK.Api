use std::collections::HashMap;

use futures::{Stream, StreamExt};
use futures_util::TryStreamExt;

use crate::objects::agent::{
    create_results, create_results_metadata, create_tasks, get_common_data, get_direct_data,
    get_resource_data, send_result, submit_tasks, upload_result, ResultMetaData,
};
use crate::objects::{DataChunk, TaskOptions};
use crate::utils::IntoCollection;

use crate::api::v3;

use super::{GrpcCall, GrpcCallStream};

/// The ResultsService provides methods for interacting with results.
#[derive(Clone)]
pub struct AgentClient<T> {
    inner: v3::agent::agent_client::AgentClient<T>,
}

macro_rules! impl_get_data {
    ($name:ident) => {
        pub async fn $name(
            &mut self,
            token: impl Into<String>,
            key: impl Into<String>,
        ) -> Result<impl Stream<Item = Result<Vec<u8>, tonic::Status>>, tonic::Status> {
            let mut stream = self
                .inner
                .$name($name::Request {
                    communication_token: token.into(),
                    key: key.into(),
                })
                .await?
                .into_inner();

            Ok(async_stream::try_stream! {
                while let Some(response) = stream.try_next().await? {
                    let response: $name::Response = response.into();
                    match response {
                        $name::Response::DataChunk {
                            communication_token: _,
                            key: _,
                            chunk: DataChunk::Data(chunk),
                        } => yield chunk,
                        $name::Response::DataChunk {
                            communication_token: _,
                            key: _,
                            chunk: DataChunk::Complete,
                        } => (),
                        $name::Response::Error {
                            communication_token: _,
                            key: Some(_),
                            error,
                        } => Err(tonic::Status::not_found(error))?,
                        $name::Response::Error {
                            communication_token: _,
                            key: None,
                            error,
                        } => Err(tonic::Status::internal(error))?,
                    }
                }
            })
        }
    };
}

impl<T> AgentClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
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
    ) -> Result<HashMap<String, ResultMetaData>, tonic::Status> {
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
    ) -> Result<HashMap<String, ResultMetaData>, tonic::Status> {
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

    /// Upload data for result with stream.
    pub async fn upload_result<S>(
        &mut self,
        token: impl Into<String>,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
        data: impl futures::Stream<Item = impl Into<Vec<u8>>> + Send + Unpin + 'static,
    ) -> Result<(), tonic::Status> {
        let token: String = token.into();
        let session_id: String = session_id.into();
        let result_id: String = result_id.into();
        let request = {
            let token = token.clone();
            async_stream::stream! {
                yield
                    upload_result::Request::Identifier {
                        communication_token: token,
                        session: session_id,
                        result_id,
                    }
                ;
            }
        };

        let request = request.chain(data.map(move |chunk| upload_result::Request::DataChunk {
            communication_token: token.clone(),
            chunk: chunk.into(),
        }));

        self.call(request).await?;

        Ok(())
    }

    /// Create tasks metadata and submit task for processing.
    pub async fn submit(
        &mut self,
        token: impl Into<String>,
        session_id: impl Into<String>,
        task_options: Option<TaskOptions>,
        items: impl IntoIterator<Item = submit_tasks::RequestItem>,
    ) -> Result<Vec<submit_tasks::ResponseItem>, tonic::Status> {
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
    ) -> Result<Vec<create_tasks::Status>, tonic::Status> {
        let response = self.call(request).await?;

        match response {
            create_tasks::Response::Status {
                communication_token: _,
                statuses,
            } => Ok(statuses),
            create_tasks::Response::Error {
                communication_token: _,
                error,
            } => Err(tonic::Status::internal(error)),
        }
    }

    impl_get_data!(get_resource_data);
    impl_get_data!(get_common_data);
    impl_get_data!(get_direct_data);

    pub async fn send_result<Stream, Key, Chunk>(
        &mut self,
        token: impl Into<String>,
        mut data: Stream,
    ) -> Result<(), tonic::Status>
    where
        Key: Into<String> + Send,
        Chunk: Into<Vec<u8>> + Send,
        Stream: futures::Stream<Item = (Key, Chunk)> + Send + Unpin + 'static,
    {
        let token: String = token.into();
        let request = async_stream::stream! {
            let previous = String::new();
            while let Some((key, chunk)) = data.next().await {
                let key = key.into();
                if key != previous {
                    yield send_result::Request::Init { communication_token: token.clone(), key };
                }

                yield send_result::Request::DataChunk {
                    communication_token: token.clone(),
                    chunk: DataChunk::Data(chunk.into()),
                };
            }

            yield send_result::Request::LastResult { communication_token: token };
        };

        self.call(Box::pin(request)).await?;

        Ok(())
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
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: create_results::Request) -> Result<create_results::Response> {
            Ok(self
                .inner
                .create_results(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: submit_tasks::Request) -> Result<submit_tasks::Response> {
            Ok(self
                .inner
                .submit_tasks(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: get_resource_data::Request) -> Result<Box<dyn Stream<Item = Result<get_resource_data::Response, tonic::Status>>>> {
            Ok(Box::new(self
                .inner
                .get_resource_data(request)
                .await?
                .into_inner()
                .map(|item| item.map(Into::into))))
        }

        async fn call(self, request: get_common_data::Request) -> Result<Box<dyn Stream<Item = Result<get_common_data::Response, tonic::Status>>>> {
            Ok(Box::new(self
                .inner
                .get_common_data(request)
                .await?
                .into_inner()
                .map(|item| item.map(Into::into))))
        }

        async fn call(self, request: get_direct_data::Request) -> Result<Box<dyn Stream<Item = Result<get_direct_data::Response, tonic::Status>>>> {
            Ok(Box::new(self
                .inner
                .get_direct_data(request)
                .await?
                .into_inner()
                .map(|item| item.map(Into::into))))
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<T, S> GrpcCallStream<upload_result::Request, S> for &'_ mut AgentClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    S: Stream<Item = upload_result::Request> + Send + 'static,
{
    type Response = upload_result::Response;
    type Error = tonic::Status;

    async fn call(self, request: S) -> Result<Self::Response, Self::Error> {
        Ok(self
            .inner
            .upload_result_data(request.map(Into::into))
            .await?
            .into_inner()
            .into())
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
    type Error = tonic::Status;

    async fn call(self, request: S) -> Result<Self::Response, Self::Error> {
        Ok(self
            .inner
            .create_task(request.map(Into::into))
            .await?
            .into_inner()
            .into())
    }
}

#[async_trait::async_trait(?Send)]
impl<S, T> GrpcCallStream<send_result::Request, S> for &'_ mut AgentClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    S: Stream<Item = send_result::Request> + Send + 'static,
{
    type Response = send_result::Response;
    type Error = tonic::Status;

    async fn call(self, request: S) -> Result<Self::Response, Self::Error> {
        Ok(self
            .inner
            .send_result(request.map(Into::into))
            .await?
            .into_inner()
            .into())
    }
}
