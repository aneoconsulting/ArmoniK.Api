use tokio_stream::StreamExt;

use crate::objects::results::{
    CreateResultsMetadataRequest, CreateResultsMetadataResponse, CreateResultsRequest,
    CreateResultsResponse, DeleteResultsDataRequest, DeleteResultsDataResponse,
    DownloadResultDataRequest, GetOwnerTaskIdRequest, GetOwnerTaskIdResponse, ResultListRequest,
    ResultListResponse, ResultRaw, ResultsServiceConfiguration, UploadResultDataRequest,
};

use crate::api::v3;

/// The ResultsService provides methods for interacting with results.
#[derive(Clone)]
pub struct ResultsClient<T> {
    inner: v3::results::results_client::ResultsClient<T>,
}

impl<T> ResultsClient<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::results::results_client::ResultsClient::new(channel),
        }
    }

    /// Get a results list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        request: ResultListRequest,
    ) -> Result<ResultListResponse, tonic::Status> {
        Ok(self.inner.list_results(request).await?.into_inner().into())
    }

    /// Get the id of the task that should produce the result.
    pub async fn get(&mut self, result_id: String) -> Result<ResultRaw, tonic::Status> {
        Ok(self
            .inner
            .get_result(v3::results::GetResultRequest { result_id })
            .await?
            .into_inner()
            .result
            .into())
    }

    /// Get the id of the task that should produce the result.
    pub async fn get_owner_task_id(
        &mut self,
        request: GetOwnerTaskIdRequest,
    ) -> Result<GetOwnerTaskIdResponse, tonic::Status> {
        Ok(self
            .inner
            .get_owner_task_id(request)
            .await?
            .into_inner()
            .into())
    }

    /// Create the metadata of multiple results at once.
    /// Data have to be uploaded separately.
    pub async fn create_metadata(
        &mut self,
        request: CreateResultsMetadataRequest,
    ) -> Result<CreateResultsMetadataResponse, tonic::Status> {
        Ok(self
            .inner
            .create_results_meta_data(request)
            .await?
            .into_inner()
            .into())
    }

    /// Create one result with data included in the request.
    pub async fn create(
        &mut self,
        request: CreateResultsRequest,
    ) -> Result<CreateResultsResponse, tonic::Status> {
        Ok(self
            .inner
            .create_results(request)
            .await?
            .into_inner()
            .into())
    }

    /// Upload data for result with stream.
    pub async fn upload<S>(
        &mut self,
        session_id: String,
        result_id: String,
        data: S,
    ) -> Result<ResultRaw, tonic::Status>
    where
        S: futures::Stream + Send + Unpin + 'static,
        <S as futures::Stream>::Item: Into<Vec<u8>>,
    {
        let request = async_stream::stream! {
            yield v3::results::UploadResultDataRequest::from(
                UploadResultDataRequest::Identifier {
                    session: session_id,
                    result_id,
                }
            );
        };
        let request = request.chain(data.map(|chunk| {
            v3::results::UploadResultDataRequest::from(UploadResultDataRequest::DataChunk(
                chunk.into(),
            ))
        }));

        Ok(self
            .inner
            .upload_result_data(request)
            .await?
            .into_inner()
            .result
            .into())
    }

    /// Retrieve data.
    pub async fn download(
        &mut self,
        request: DownloadResultDataRequest,
    ) -> Result<impl futures::Stream<Item = Result<Vec<u8>, tonic::Status>>, tonic::Status> {
        Ok(self
            .inner
            .download_result_data(request)
            .await?
            .into_inner()
            .map(|response| response.map(|response| response.data_chunk)))
    }

    /// Delete data from multiple results.
    pub async fn delete_data(
        &mut self,
        request: DeleteResultsDataRequest,
    ) -> Result<DeleteResultsDataResponse, tonic::Status> {
        Ok(self
            .inner
            .delete_results_data(request)
            .await?
            .into_inner()
            .into())
    }

    /// Get the configuration of the service.
    pub async fn get_service_configuration(
        &mut self,
    ) -> Result<ResultsServiceConfiguration, tonic::Status> {
        Ok(self
            .inner
            .get_service_configuration(v3::Empty {})
            .await?
            .into_inner()
            .into())
    }
}
