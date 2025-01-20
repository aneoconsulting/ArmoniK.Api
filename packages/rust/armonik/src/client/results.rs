use std::collections::HashMap;

use futures::{Stream, StreamExt};
use snafu::ResultExt;

use crate::api::v3;
use crate::results::{
    create, create_metadata, delete_data, download, filter, get, get_owner_task_id,
    get_service_configuration, list, upload, Raw, Sort,
};
use crate::utils::IntoCollection;

use super::{GrpcCall, GrpcCallStream};

/// The ResultsService provides methods for interacting with results.
#[derive(Clone)]
pub struct Results<T> {
    inner: v3::results::results_client::ResultsClient<T>,
}

impl<T> Results<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::results::results_client::ResultsClient::new(channel),
        }
    }

    /// Get a results list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: filters
                .into_iter()
                .map(IntoCollection::into_collect)
                .collect(),
            sort,
            page,
            page_size,
        })
        .await
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
            .call(get_owner_task_id::Request {
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
                names: names.into_collect(),
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
        S: futures::Stream + Send + 'static,
        <S as futures::Stream>::Item: Into<Vec<u8>>,
    {
        let session_id: String = session_id.into();
        let result_id: String = result_id.into();

        let request = futures::stream::iter([v3::results::UploadResultDataRequest::from(
            upload::Request::Identifier {
                session_id,
                result_id,
            },
        )]);
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
            .call(delete_data::Request {
                session_id: session_id.into(),
                result_ids: result_ids.into_collect(),
            })
            .await?
            .result_ids)
    }

    /// Get the configuration of the service.
    pub async fn get_service_configuration(
        &mut self,
    ) -> Result<get_service_configuration::Response, super::RequestError> {
        self.call(get_service_configuration::Request {}).await
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
    Results {
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

        async fn call(self, request: get_owner_task_id::Request) -> Result<get_owner_task_id::Response> {
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

        async fn call(self, request: delete_data::Request) -> Result<delete_data::Response> {
            Ok(self
                .inner
                .delete_results_data(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get_service_configuration::Request) -> Result<get_service_configuration::Response> {
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
impl<T> GrpcCall<download::Request> for &'_ mut Results<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    type Response =
        std::pin::Pin<Box<dyn Stream<Item = Result<download::Response, super::RequestError>>>>;
    type Error = super::RequestError;

    async fn call(self, request: download::Request) -> Result<Self::Response, Self::Error> {
        Ok(Box::pin(
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
impl<T, S> GrpcCallStream<upload::Request, S> for &'_ mut Results<T>
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

#[cfg(test)]
#[serial_test::serial(results)]
mod tests {
    use std::collections::HashMap;

    use crate::Client;
    use futures::TryStreamExt;

    // Named methods

    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Results", "ListResults").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .list(
                crate::results::filter::Or {
                    or: vec![crate::results::filter::And { and: vec![] }],
                },
                crate::results::Sort::default(),
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "ListResults").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get() {
        let before = Client::get_nb_request("Results", "GetResult").await;
        let mut client = Client::new().await.unwrap().into_results();
        client.get("result-id").await.unwrap();
        let after = Client::get_nb_request("Results", "GetResult").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_owner_task_id() {
        let before = Client::get_nb_request("Results", "GetOwnerTaskId").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .get_owner_task_id("session-id", ["result1", "result2"])
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "GetOwnerTaskId").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_metadata() {
        let before = Client::get_nb_request("Results", "CreateResultsMetaData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .create_metadata("session-id", ["result1", "result2"])
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "CreateResultsMetaData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create() {
        let before = Client::get_nb_request("Results", "CreateResults").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .create(
                "session-id",
                [("result1", "payload1"), ("result2", "payload2")],
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "CreateResults").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn upload() {
        let before = Client::get_nb_request("Results", "UploadResultData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .upload("session-id", "result-id", futures::stream::iter([b""]))
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "UploadResultData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn download() {
        let before = Client::get_nb_request("Results", "DownloadResultData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .download("session-id", "result-id")
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "DownloadResultData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn delete_data() {
        let before = Client::get_nb_request("Results", "DeleteResultsData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .delete_data("session-id", ["result1", "result2"])
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "DeleteResultsData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_service_configuration() {
        let before = Client::get_nb_request("Results", "GetServiceConfiguration").await;
        let mut client = Client::new().await.unwrap().into_results();
        client.get_service_configuration().await.unwrap();
        let after = Client::get_nb_request("Results", "GetServiceConfiguration").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Results", "ListResults").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::list::Request {
                page_size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "ListResults").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_call() {
        let before = Client::get_nb_request("Results", "GetResult").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::get::Request {
                id: String::from("result-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "GetResult").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_owner_task_id_call() {
        let before = Client::get_nb_request("Results", "GetOwnerTaskId").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::get_owner_task_id::Request {
                session_id: String::from("session-id"),
                result_ids: Vec::new(),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "GetOwnerTaskId").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_metadata_call() {
        let before = Client::get_nb_request("Results", "CreateResultsMetaData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::create_metadata::Request {
                session_id: String::from("session-id"),
                names: Vec::new(),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "CreateResultsMetaData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_call() {
        let before = Client::get_nb_request("Results", "CreateResults").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::create::Request {
                session_id: String::from("session-id"),
                results: HashMap::new(),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "CreateResults").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn delete_data_call() {
        let before = Client::get_nb_request("Results", "DeleteResultsData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::delete_data::Request {
                session_id: String::from("session-id"),
                result_ids: vec![String::from("result-id")],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "DeleteResultsData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_service_configuration_call() {
        let before = Client::get_nb_request("Results", "GetServiceConfiguration").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::get_service_configuration::Request {})
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "GetServiceConfiguration").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn download_call() {
        let before = Client::get_nb_request("Results", "DownloadResultData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(crate::results::download::Request {
                session_id: String::from("session-id"),
                result_id: String::from("result-id"),
            })
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "DownloadResultData").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn upload_call() {
        let before = Client::get_nb_request("Results", "UploadResultData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .call(Box::pin(futures::stream::iter([
                crate::results::upload::Request::Identifier {
                    session_id: String::from("session-id"),
                    result_id: String::from("result-id"),
                },
            ])))
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "UploadResultData").await;
        assert_eq!(after - before, 1);
    }
}
