use std::collections::HashMap;

use crate::{
    api::v3,
    objects::tasks::{
        cancel, count_status, filter, get, list, list_detailed, result_ids, submit, Raw, Summary,
    },
    objects::{StatusCount, TaskOptions},
};

use super::GrpcCall;

/// Service for handling tasks.
#[derive(Clone)]
pub struct TasksClient<T> {
    inner: v3::tasks::tasks_client::TasksClient<T>,
}

impl<T> TasksClient<T>
where
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
    pub async fn list(&mut self, request: list::Request) -> Result<list::Response, tonic::Status> {
        self.call(request).await
    }

    /// Get a tasks list using pagination, filters and sorting.
    pub async fn list_detailed(
        &mut self,
        request: list_detailed::Request,
    ) -> Result<list_detailed::Response, tonic::Status> {
        self.call(request).await
    }

    /// Get a task by its id.
    pub async fn get(&mut self, task_id: String) -> Result<Raw, tonic::Status> {
        Ok(self.call(get::Request { task_id }).await?.task)
    }

    /// Cancel tasks using ids.
    pub async fn cancel(&mut self, task_ids: Vec<String>) -> Result<Vec<Summary>, tonic::Status> {
        Ok(self.call(cancel::Request { task_ids }).await?.tasks)
    }

    /// Get ids of the result that tasks should produce.
    pub async fn get_result_ids(
        &mut self,
        task_ids: Vec<String>,
    ) -> Result<HashMap<String, Vec<String>>, tonic::Status> {
        Ok(self
            .call(result_ids::Request { task_ids })
            .await?
            .task_results)
    }

    /// Get count from tasks status.
    pub async fn count_status(
        &mut self,
        filters: filter::Or,
    ) -> Result<Vec<StatusCount>, tonic::Status> {
        Ok(self.call(count_status::Request { filters }).await?.status)
    }

    /// Create tasks metadata and submit task for processing.
    pub async fn submit(
        &mut self,
        session_id: String,
        task_options: Option<TaskOptions>,
        items: Vec<submit::RequestItem>,
    ) -> Result<Vec<submit::ResponseItem>, tonic::Status> {
        Ok(self
            .call(submit::Request {
                session_id,
                task_options,
                items,
            })
            .await?
            .items)
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
    TasksClient {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            Ok(self
                .inner
                .list_tasks(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: list_detailed::Request) -> Result<list_detailed::Response> {
            Ok(self
                .inner
                .list_tasks_detailed(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            Ok(self
                .inner
                .get_task(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel::Request) -> Result<cancel::Response> {
            Ok(self
                .inner
                .cancel_tasks(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: result_ids::Request) -> Result<result_ids::Response> {
            Ok(self
                .inner
                .get_result_ids(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: count_status::Request) -> Result<count_status::Response> {
            Ok(self
                .inner
                .count_tasks_by_status(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: submit::Request) -> Result<submit::Response> {
            Ok(self
                .inner
                .submit_tasks(request)
                .await?
                .into_inner()
                .into())
        }
    }
}
