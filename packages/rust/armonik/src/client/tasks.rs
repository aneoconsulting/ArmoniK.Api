use std::collections::HashMap;

use snafu::ResultExt;

use crate::api::v3;
use crate::tasks::{
    cancel, count_status, filter, get, get_result_ids, list, list_detailed, submit, Raw, Sort,
    Summary,
};
use crate::utils::IntoCollection;
use crate::{StatusCount, TaskOptions};

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
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::tasks::tasks_client::TasksClient::new(channel),
        }
    }

    /// Get a tasks list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
        with_errors: bool,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: filters
                .into_iter()
                .map(crate::utils::IntoCollection::into_collect)
                .collect(),
            sort,
            with_errors,
            page,
            page_size,
        })
        .await
    }

    /// Get a tasks list using pagination, filters and sorting.
    pub async fn list_detailed(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
        with_errors: bool,
        page: i32,
        page_size: i32,
    ) -> Result<list_detailed::Response, super::RequestError> {
        self.call(list_detailed::Request {
            filters: filters
                .into_iter()
                .map(crate::utils::IntoCollection::into_collect)
                .collect(),
            sort,
            with_errors,
            page,
            page_size,
        })
        .await
    }

    /// Get a task by its id.
    pub async fn get(&mut self, task_id: impl Into<String>) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(get::Request {
                task_id: task_id.into(),
            })
            .await?
            .task)
    }

    /// Cancel tasks using ids.
    pub async fn cancel(
        &mut self,
        task_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Vec<Summary>, super::RequestError> {
        Ok(self
            .call(cancel::Request {
                task_ids: task_ids.into_collect(),
            })
            .await?
            .tasks)
    }

    /// Get ids of the result that tasks should produce.
    pub async fn get_result_ids(
        &mut self,
        task_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, Vec<String>>, super::RequestError> {
        Ok(self
            .call(get_result_ids::Request {
                task_ids: task_ids.into_collect(),
            })
            .await?
            .task_results)
    }

    /// Get count from tasks status.
    pub async fn count_status(
        &mut self,
        filters: filter::Or,
    ) -> Result<Vec<StatusCount>, super::RequestError> {
        Ok(self.call(count_status::Request { filters }).await?.status)
    }

    /// Create tasks metadata and submit task for processing.
    pub async fn submit(
        &mut self,
        session_id: impl Into<String>,
        task_options: Option<TaskOptions>,
        items: impl IntoIterator<Item = submit::RequestItem>,
    ) -> Result<Vec<submit::ResponseItem>, super::RequestError> {
        Ok(self
            .call(submit::Request {
                session_id: session_id.into(),
                task_options,
                items: items.into_collect(),
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
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: list_detailed::Request) -> Result<list_detailed::Response> {
            Ok(self
                .inner
                .list_tasks_detailed(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            Ok(self
                .inner
                .get_task(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel::Request) -> Result<cancel::Response> {
            Ok(self
                .inner
                .cancel_tasks(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get_result_ids::Request) -> Result<get_result_ids::Response> {
            Ok(self
                .inner
                .get_result_ids(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: count_status::Request) -> Result<count_status::Response> {
            Ok(self
                .inner
                .count_tasks_by_status(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: submit::Request) -> Result<submit::Response> {
            Ok(self
                .inner
                .submit_tasks(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }
    }
}

#[cfg(test)]
#[serial_test::serial(tasks)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Tasks", "ListTasks").await;
        let mut client = Client::singleton().await.tasks();
        client
            .list(
                crate::tasks::filter::Or {
                    or: vec![crate::tasks::filter::And { and: vec![] }],
                },
                crate::tasks::Sort::default(),
                true,
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "ListTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn list_detailed() {
        let before = Client::get_nb_request("Tasks", "ListTasksDetailed").await;
        let mut client = Client::singleton().await.tasks();
        client
            .list_detailed(
                crate::tasks::filter::Or {
                    or: vec![crate::tasks::filter::And { and: vec![] }],
                },
                crate::tasks::Sort::default(),
                true,
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "ListTasksDetailed").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get() {
        let before = Client::get_nb_request("Tasks", "GetTask").await;
        let mut client = Client::singleton().await.tasks();
        client.get("task-id").await.unwrap();
        let after = Client::get_nb_request("Tasks", "GetTask").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel() {
        let before = Client::get_nb_request("Tasks", "CancelTasks").await;
        let mut client = Client::singleton().await.tasks();
        client.cancel(["task1", "task2"]).await.unwrap();
        let after = Client::get_nb_request("Tasks", "CancelTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_result_ids() {
        let before = Client::get_nb_request("Tasks", "GetResultIds").await;
        let mut client = Client::singleton().await.tasks();
        client.get_result_ids(["task1", "task2"]).await.unwrap();
        let after = Client::get_nb_request("Tasks", "GetResultIds").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn count_status() {
        let before = Client::get_nb_request("Tasks", "CountTasksByStatus").await;
        let mut client = Client::singleton().await.tasks();
        client
            .count_status(crate::tasks::filter::Or {
                or: vec![crate::tasks::filter::And { and: vec![] }],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "CountTasksByStatus").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn submit() {
        let before = Client::get_nb_request("Tasks", "SubmitTasks").await;
        let mut client = Client::singleton().await.tasks();
        client.submit("session-id", None, []).await.unwrap();
        let after = Client::get_nb_request("Tasks", "SubmitTasks").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Tasks", "ListTasks").await;
        let mut client = Client::singleton().await.tasks();
        client
            .call(crate::tasks::list::Request {
                page_size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "ListTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn list_detailed_call() {
        let before = Client::get_nb_request("Tasks", "ListTasksDetailed").await;
        let mut client = Client::singleton().await.tasks();
        client
            .call(crate::tasks::list_detailed::Request {
                page_size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "ListTasksDetailed").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_call() {
        let before = Client::get_nb_request("Tasks", "GetTask").await;
        let mut client = Client::singleton().await.tasks();
        client
            .call(crate::tasks::get::Request {
                task_id: String::from("task-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "GetTask").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel_call() {
        let before = Client::get_nb_request("Tasks", "CancelTasks").await;
        let mut client = Client::singleton().await.tasks();
        client
            .call(crate::tasks::cancel::Request {
                task_ids: vec![String::from("task1"), String::from("task2")],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "CancelTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_result_ids_call() {
        let before = Client::get_nb_request("Tasks", "GetResultIds").await;
        let mut client = Client::singleton().await.tasks();
        client
            .call(crate::tasks::get_result_ids::Request {
                task_ids: vec![String::from("task1"), String::from("task2")],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "GetResultIds").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn count_status_call() {
        let before = Client::get_nb_request("Tasks", "CountTasksByStatus").await;
        let mut client = Client::singleton().await.tasks();
        client
            .call(crate::tasks::count_status::Request {
                filters: crate::tasks::filter::Or {
                    or: vec![crate::tasks::filter::And { and: vec![] }],
                },
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "CountTasksByStatus").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn submit_call() {
        let before = Client::get_nb_request("Tasks", "SubmitTasks").await;
        let mut client = Client::singleton().await.tasks();
        client
            .call(crate::tasks::submit::Request {
                session_id: String::from("session-id"),
                task_options: None,
                items: vec![],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Tasks", "SubmitTasks").await;
        assert_eq!(after - before, 1);
    }
}
