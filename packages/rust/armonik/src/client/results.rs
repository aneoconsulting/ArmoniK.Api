use std::collections::HashMap;

use futures::{Stream, StreamExt};
use snafu::ResultExt;

use crate::objects::results::{
    create, create_metadata, delete, download, get, list, owner, service_configuration, upload, Raw,
};
use crate::utils::IntoCollection;

use crate::api::v3;

use super::{GrpcCall, GrpcCallStream};

/// The ResultsService provides methods for interacting with results.
#[derive(Clone)]
pub struct ResultsClient<T> {
    inner: v3::results::results_client::ResultsClient<T>,
}

impl<T> ResultsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::results::results_client::ResultsClient::new(channel),
        }
    }

    /// Get a results list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        request: list::Request,
    ) -> Result<list::Response, super::RequestError> {
        self.call(request).await
    }

    /// Get the id of the task that should produce the result.
    pub async fn get(&mut self, result_id: impl Into<String>) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(get::Request {
                id: result_id.into(),
            })
            .await?
            .result)
    }

    /// Get the id of the task that should produce the result.
    pub async fn get_owner_task_id(
        &mut self,
        session_id: impl Into<String>,
        result_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, String>, super::RequestError> {
        Ok(self
            .call(owner::Request {
                session_id: session_id.into(),
                result_ids: result_ids.into_collect(),
            })
            .await?
            .result_task)
    }

    /// Create the metadata of multiple results at once.
    /// Data have to be uploaded separately.
    pub async fn create_metadata(
        &mut self,
        session_id: impl Into<String>,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, Raw>, super::RequestError> {
        Ok(self
            .call(create_metadata::Request {
                results: names.into_collect(),
                session_id: session_id.into(),
            })
            .await?
            .results)
    }

    /// Create one result with data included in the request.
    pub async fn create(
        &mut self,
        session_id: impl Into<String>,
        results: impl std::iter::IntoIterator<Item = (impl Into<String>, impl Into<Vec<u8>>)>,
    ) -> Result<HashMap<String, Raw>, super::RequestError> {
        Ok(self
            .call(create::Request {
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
    pub async fn upload<S>(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
        data: S,
    ) -> Result<Raw, super::RequestError>
    where
        S: futures::Stream + Send + Unpin + 'static,
        <S as futures::Stream>::Item: Into<Vec<u8>>,
    {
        let session_id: String = session_id.into();
        let result_id: String = result_id.into();
        let request = async_stream::stream! {
            yield v3::results::UploadResultDataRequest::from(
                upload::Request::Identifier {
                    session: session_id,
                    result_id,
                }
            );
        };
        let request = request.chain(data.map(|chunk| {
            v3::results::UploadResultDataRequest::from(upload::Request::DataChunk(chunk.into()))
        }));

        Ok(self
            .inner
            .upload_result_data(request)
            .await
            .context(super::GrpcSnafu {})?
            .into_inner()
            .result
            .map_or_else(Default::default, Into::into))
    }

    /// Retrieve data.
    pub async fn download(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
    ) -> Result<
        impl futures::Stream<Item = Result<Vec<u8>, super::RequestError>>,
        super::RequestError,
    > {
        Ok(self
            .inner
            .download_result_data(download::Request {
                session_id: session_id.into(),
                result_id: result_id.into(),
            })
            .await
            .context(super::GrpcSnafu {})?
            .into_inner()
            .map(|response| {
                response
                    .map(|response| response.data_chunk)
                    .context(super::GrpcSnafu {})
            }))
    }

    /// Delete data from multiple results.
    pub async fn delete_data(
        &mut self,
        session_id: impl Into<String>,
        result_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<String>, super::RequestError> {
        Ok(self
            .call(delete::Request {
                session_id: session_id.into(),
                result_ids: result_ids.into_collect(),
            })
            .await?
            .result_ids)
    }

    /// Get the configuration of the service.
    pub async fn get_service_configuration(
        &mut self,
    ) -> Result<service_configuration::Response, super::RequestError> {
        self.call(service_configuration::Request {}).await
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
    ResultsClient {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            Ok(self
                .inner
                .list_results(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            Ok(self
                .inner
                .get_result(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: owner::Request) -> Result<owner::Response> {
            Ok(self
                .inner
                .get_owner_task_id(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: create_metadata::Request) -> Result<create_metadata::Response> {
            Ok(self
                .inner
                .create_results_meta_data(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: create::Request) -> Result<create::Response> {
            Ok(self
                .inner
                .create_results(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: delete::Request) -> Result<delete::Response> {
            Ok(self
                .inner
                .delete_results_data(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: service_configuration::Request) -> Result<service_configuration::Response> {
            Ok(self
                .inner
                .get_service_configuration(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<T> GrpcCall<download::Request> for &'_ mut ResultsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    type Response = Box<dyn Stream<Item = Result<download::Response, super::RequestError>>>;
    type Error = super::RequestError;

    async fn call(self, request: download::Request) -> Result<Self::Response, Self::Error> {
        Ok(Box::new(
            self.inner
                .download_result_data(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .map(|response| response.map(Into::into).context(super::GrpcSnafu {})),
        ))
    }
}

#[async_trait::async_trait(?Send)]
impl<T, S> GrpcCallStream<upload::Request, S> for &'_ mut ResultsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    S: Stream<Item = upload::Request> + Send + 'static,
{
    type Response = upload::Response;
    type Error = super::RequestError;

    async fn call(self, request: S) -> Result<Self::Response, Self::Error> {
        Ok(self
            .inner
            .upload_result_data(request.map(Into::into))
            .await
            .context(super::GrpcSnafu {})?
            .into_inner()
            .into())
    }
}
