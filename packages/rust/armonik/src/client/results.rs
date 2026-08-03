use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};

use crate::results::{
    create, create_metadata, delete_data, download, filter, get, get_owner_task_id,
    get_service_configuration, import, list, upload, Raw, Sort,
};
use crate::rpc::services;
use crate::utils::IntoCollection;

/// The ResultsService provides methods for interacting with results.
pub type Results<T = tonic::transport::Channel> = super::ServiceClient<services::Results, T>;

impl<T: super::Channel> super::ServiceClient<services::Results, T> {
    /// Get a results list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: crate::utils::into_filters(filters),
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
        results: impl IntoIterator<Item = create_metadata::RequestItem>,
    ) -> Result<Vec<Raw>, super::RequestError> {
        Ok(self
            .call(create_metadata::Request {
                results: results.into_collect(),
                session_id: session_id.into(),
            })
            .await?
            .results)
    }

    /// Create one result with data included in the request.
    pub async fn create(
        &mut self,
        session_id: impl Into<String>,
        results: impl IntoIterator<Item = create::RequestItem>,
    ) -> Result<Vec<Raw>, super::RequestError> {
        Ok(self
            .call(create::Request {
                results: results.into_collect(),
                session_id: session_id.into(),
            })
            .await?
            .results)
    }

    /// Import existing data from the object storage into existing results
    pub async fn import(
        &mut self,
        session_id: impl Into<String>,
        results: impl std::iter::IntoIterator<Item = (impl Into<String>, impl Into<bytes::Bytes>)>,
    ) -> Result<HashMap<String, Raw>, super::RequestError> {
        Ok(self
            .call(import::Request {
                results: results
                    .into_iter()
                    .map(|(result_id, opaque_id)| (result_id.into(), opaque_id.into()))
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
        <S as futures::Stream>::Item: Into<bytes::Bytes>,
    {
        let request = futures::stream::iter([upload::Request::Identifier {
            session_id: session_id.into(),
            result_id: result_id.into(),
        }]);
        let request = request.chain(data.map(|chunk| upload::Request::DataChunk(chunk.into())));

        Ok(self.call_streaming(request).await?.result)
    }

    /// Retrieve data.
    pub async fn download(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
    ) -> Result<
        impl futures::Stream<Item = Result<bytes::Bytes, super::RequestError>> + 'static,
        super::RequestError,
    > {
        Ok(self
            .call(download::Request {
                session_id: session_id.into(),
                result_id: result_id.into(),
            })
            .await?
            .map_ok(|response| response.data_chunk))
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
}

#[cfg(test)]
#[serial_test::serial(results)]
mod tests {
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
            .create_metadata(
                "session-id",
                [crate::results::create_metadata::RequestItem {
                    name: "result".into(),
                    manual_deletion: false,
                }],
            )
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
                [
                    crate::results::create::RequestItem {
                        name: "result1".into(),
                        data: bytes::Bytes::from_static(b"data1"),
                        manual_deletion: false,
                    },
                    crate::results::create::RequestItem {
                        name: "result2".into(),
                        data: bytes::Bytes::from_static(b"data2"),
                        manual_deletion: false,
                    },
                ],
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
            .upload(
                "session-id",
                "result-id",
                futures::stream::iter([bytes::Bytes::new()]),
            )
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
    async fn import() {
        let before = Client::get_nb_request("Results", "ImportResultsData").await;
        let mut client = Client::new().await.unwrap().into_results();
        client
            .import(
                "session-id",
                [("result", bytes::Bytes::from_static(b"opaque-id"))],
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Results", "ImportResultsData").await;
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
                results: Vec::new(),
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
                results: Vec::new(),
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
            .call_streaming(Box::pin(futures::stream::iter([
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
