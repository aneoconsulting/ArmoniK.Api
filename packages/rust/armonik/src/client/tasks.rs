use std::collections::HashMap;

use crate::rpc::services;
use crate::tasks::{
    cancel, count_status, filter, get, get_result_ids, list, list_detailed, submit, Raw, Sort,
    Summary,
};
use crate::utils::IntoCollection;
use crate::{StatusCount, TaskOptions};

/// Service for handling tasks.
pub type Tasks<T = tonic::transport::Channel> = super::ServiceClient<services::Tasks, T>;

impl<T: super::Channel> super::ServiceClient<services::Tasks, T> {
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
}

#[cfg(test)]
#[serial_test::serial(tasks)]
mod tests {
    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Tasks", "ListTasks").await;
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
        client.get("task-id").await.unwrap();
        let after = Client::get_nb_request("Tasks", "GetTask").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel() {
        let before = Client::get_nb_request("Tasks", "CancelTasks").await;
        let mut client = Client::new().await.unwrap().into_tasks();
        client.cancel(["task1", "task2"]).await.unwrap();
        let after = Client::get_nb_request("Tasks", "CancelTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_result_ids() {
        let before = Client::get_nb_request("Tasks", "GetResultIds").await;
        let mut client = Client::new().await.unwrap().into_tasks();
        client.get_result_ids(["task1", "task2"]).await.unwrap();
        let after = Client::get_nb_request("Tasks", "GetResultIds").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn count_status() {
        let before = Client::get_nb_request("Tasks", "CountTasksByStatus").await;
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
        client.submit("session-id", None, []).await.unwrap();
        let after = Client::get_nb_request("Tasks", "SubmitTasks").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Tasks", "ListTasks").await;
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
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
        let mut client = Client::new().await.unwrap().into_tasks();
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
