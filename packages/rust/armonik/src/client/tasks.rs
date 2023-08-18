use crate::{
    api::v3,
    objects::tasks::{
        CancelTasksRequest, CancelTasksResponse, CountTasksByStatusRequest,
        CountTasksByStatusResponse, GetResultIdsRequest, GetResultIdsResponse, GetTaskRequest,
        GetTaskResponse, SubmitTasksRequest, SubmitTasksResponse, TaskListDetailedResponse,
        TaskListRequest, TaskListResponse,
    },
};

/// Service for handling tasks.
#[derive(Clone)]
pub struct TasksClient<T> {
    inner: v3::tasks::tasks_client::TasksClient<T>,
}

impl<T> TasksClient<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::tasks::tasks_client::TasksClient::new(channel),
        }
    }

    /// Get a tasks list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        request: TaskListRequest,
    ) -> Result<TaskListResponse, tonic::Status> {
        Ok(self.inner.list_tasks(request).await?.into_inner().into())
    }

    /// Get a tasks list using pagination, filters and sorting.
    pub async fn list_detailed(
        &mut self,
        request: TaskListRequest,
    ) -> Result<TaskListDetailedResponse, tonic::Status> {
        Ok(self
            .inner
            .list_tasks_detailed(request)
            .await?
            .into_inner()
            .into())
    }

    /// Get a task by its id.
    pub async fn get(&mut self, request: GetTaskRequest) -> Result<GetTaskResponse, tonic::Status> {
        Ok(self.inner.get_task(request).await?.into_inner().into())
    }

    /// Cancel tasks using ids.
    pub async fn cancel(
        &mut self,
        request: CancelTasksRequest,
    ) -> Result<CancelTasksResponse, tonic::Status> {
        Ok(self.inner.cancel_tasks(request).await?.into_inner().into())
    }

    /// Get ids of the result that tasks should produce.
    pub async fn get_result_ids(
        &mut self,
        request: GetResultIdsRequest,
    ) -> Result<GetResultIdsResponse, tonic::Status> {
        Ok(self
            .inner
            .get_result_ids(request)
            .await?
            .into_inner()
            .into())
    }

    /// Get count from tasks status.
    pub async fn count(
        &mut self,
        request: CountTasksByStatusRequest,
    ) -> Result<CountTasksByStatusResponse, tonic::Status> {
        Ok(self
            .inner
            .count_tasks_by_status(request)
            .await?
            .into_inner()
            .into())
    }

    /// Create tasks metadata and submit task for processing.
    pub async fn submit(
        &mut self,
        request: SubmitTasksRequest,
    ) -> Result<SubmitTasksResponse, tonic::Status> {
        Ok(self.inner.submit_tasks(request).await?.into_inner().into())
    }
}
