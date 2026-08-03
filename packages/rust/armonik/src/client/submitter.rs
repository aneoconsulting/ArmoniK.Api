#![allow(deprecated)]

use std::collections::HashMap;

use futures::Stream;
use snafu::ResultExt;

use crate::rpc::services;
use crate::submitter::{
    cancel_session, cancel_tasks, count_tasks, create_session, create_tasks,
    get_service_configuration, list_sessions, list_tasks, result_status, task_status,
    try_get_result, try_get_task_output, wait_for_availability, wait_for_completion, SessionFilter,
    TaskFilter,
};
use crate::utils::IntoCollection;
use crate::{Configuration, Output, ResultStatus, TaskOptions, TaskRequest, TaskStatus};

#[deprecated]
pub type Submitter<T = tonic::transport::Channel> = super::ServiceClient<services::Submitter, T>;

impl<T: super::Channel> super::ServiceClient<services::Submitter, T> {
    #[deprecated]
    pub async fn get_service_configuration(
        &mut self,
    ) -> Result<Configuration, super::RequestError> {
        self.call(get_service_configuration::Request {}).await
    }

    #[deprecated]
    pub async fn create_session(
        &mut self,
        partitions: impl IntoIterator<Item = impl Into<String>>,
        default_task_options: TaskOptions,
    ) -> Result<String, super::RequestError> {
        Ok(self
            .call(create_session::Request {
                default_task_options,
                partition_ids: partitions.into_collect(),
            })
            .await?
            .session_id)
    }

    #[deprecated]
    pub async fn cancel_session(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<(), super::RequestError> {
        self.call(cancel_session::Request {
            session_id: session_id.into(),
        })
        .await?;
        Ok(())
    }

    #[deprecated]
    pub async fn create_small_tasks(
        &mut self,
        session_id: impl Into<String>,
        task_options: Option<TaskOptions>,
        tasks: impl IntoIterator<Item = TaskRequest>,
    ) -> Result<Vec<create_tasks::Status>, super::RequestError> {
        let response = self
            .call(create_tasks::SmallRequest {
                session_id: session_id.into(),
                task_options,
                task_requests: tasks.into_collect(),
            })
            .await?;

        match response {
            create_tasks::Response::Status(statuses) => Ok(statuses),
            create_tasks::Response::Error(msg) => {
                Err(tonic::Status::internal(msg)).context(super::GrpcSnafu {})
            }
        }
    }

    #[deprecated]
    pub async fn create_large_tasks(
        &mut self,
        request: impl Stream<Item = create_tasks::LargeRequest> + Send + 'static,
    ) -> Result<Vec<create_tasks::Status>, super::RequestError> {
        let response = self.call_streaming(request).await?;

        match response {
            create_tasks::Response::Status(statuses) => Ok(statuses),
            create_tasks::Response::Error(msg) => {
                Err(tonic::Status::internal(msg)).context(super::GrpcSnafu {})
            }
        }
    }

    #[deprecated]
    pub async fn list_tasks(
        &mut self,
        filter: TaskFilter,
    ) -> Result<Vec<String>, super::RequestError> {
        Ok(self.call(list_tasks::Request { filter }).await?.task_ids)
    }

    #[deprecated]
    pub async fn list_sessions(
        &mut self,
        filter: SessionFilter,
    ) -> Result<Vec<String>, super::RequestError> {
        Ok(self
            .call(list_sessions::Request { filter })
            .await?
            .session_ids)
    }

    #[deprecated]
    pub async fn count_tasks(
        &mut self,
        filter: TaskFilter,
    ) -> Result<HashMap<TaskStatus, i32>, super::RequestError> {
        Ok(self.call(count_tasks::Request { filter }).await?.values)
    }

    #[deprecated]
    pub async fn try_get_result(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
    ) -> Result<
        impl Stream<Item = Result<try_get_result::Response, super::RequestError>>,
        super::RequestError,
    > {
        self.call(try_get_result::Request {
            session_id: session_id.into(),
            result_id: result_id.into(),
        })
        .await
    }

    #[deprecated]
    pub async fn try_get_task_output(
        &mut self,
        session_id: impl Into<String>,
        task_id: impl Into<String>,
    ) -> Result<(), super::RequestError> {
        let response = self
            .call(try_get_task_output::Request {
                session_id: session_id.into(),
                task_id: task_id.into(),
            })
            .await?;

        match response {
            Output::Ok => Ok(()),
            Output::Error { details } => {
                Err(tonic::Status::internal(details)).context(super::GrpcSnafu {})
            }
        }
    }

    #[deprecated]
    pub async fn wait_for_availability(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
    ) -> Result<wait_for_availability::Response, super::RequestError> {
        self.call(wait_for_availability::Request {
            session_id: session_id.into(),
            result_id: result_id.into(),
        })
        .await
    }

    #[deprecated]
    pub async fn wait_for_completion(
        &mut self,
        filter: TaskFilter,
        stop_on_first_task_error: bool,
        stop_on_first_task_cancellation: bool,
    ) -> Result<HashMap<TaskStatus, i32>, super::RequestError> {
        Ok(self
            .call(wait_for_completion::Request {
                filter,
                stop_on_first_task_error,
                stop_on_first_task_cancellation,
            })
            .await?
            .values)
    }

    #[deprecated]
    pub async fn cancel_tasks(&mut self, filter: TaskFilter) -> Result<(), super::RequestError> {
        self.call(cancel_tasks::Request { filter }).await?;
        Ok(())
    }

    #[deprecated]
    pub async fn task_status(
        &mut self,
        task_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, TaskStatus>, super::RequestError> {
        Ok(self
            .call(task_status::Request {
                task_ids: task_ids.into_collect(),
            })
            .await?
            .statuses)
    }

    #[deprecated]
    pub async fn result_status(
        &mut self,
        session_id: impl Into<String>,
        result_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, ResultStatus>, super::RequestError> {
        Ok(self
            .call(result_status::Request {
                session_id: session_id.into(),
                result_ids: result_ids.into_collect(),
            })
            .await?
            .statuses)
    }
}

#[cfg(test)]
#[serial_test::serial(submitter)]
mod tests {
    use futures::TryStreamExt;

    use crate::Client;

    // Named methods

    #[tokio::test]
    async fn get_service_configuration() {
        let before = Client::get_nb_request("Submitter", "GetServiceConfiguration").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client.get_service_configuration().await.unwrap();
        let after = Client::get_nb_request("Submitter", "GetServiceConfiguration").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_session() {
        let before = Client::get_nb_request("Submitter", "CreateSession").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .create_session(
                ["part1", "part2"],
                crate::TaskOptions {
                    partition_id: String::from("part1"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "CreateSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel_session() {
        let before = Client::get_nb_request("Submitter", "CancelSession").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client.cancel_session("session-id").await.unwrap();
        let after = Client::get_nb_request("Submitter", "CancelSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_small_tasks() {
        let before = Client::get_nb_request("Submitter", "CreateSmallTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        match client.create_small_tasks("session-id", None, []).await {
            Ok(_) => (),
            Err(crate::client::RequestError::Grpc { source, .. }) => {
                if source.code() != tonic::Code::Internal || !source.message().is_empty() {
                    panic!("{source:?}")
                }
            }
        }
        let after = Client::get_nb_request("Submitter", "CreateSmallTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_large_tasks() {
        let before = Client::get_nb_request("Submitter", "CreateLargeTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        match client
            .create_large_tasks(futures::stream::iter([
                crate::submitter::create_tasks::LargeRequest::Invalid,
            ]))
            .await
        {
            Ok(_) => (),
            Err(crate::client::RequestError::Grpc { source, .. }) => {
                if source.code() != tonic::Code::Internal || !source.message().is_empty() {
                    panic!("{source:?}")
                }
            }
        }
        let after = Client::get_nb_request("Submitter", "CreateLargeTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn list_tasks() {
        let before = Client::get_nb_request("Submitter", "ListTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .list_tasks(crate::submitter::TaskFilter {
                ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from("session-id")]),
                statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "ListTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn list_sessions() {
        let before = Client::get_nb_request("Submitter", "ListSessions").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .list_sessions(crate::submitter::SessionFilter {
                ids: vec![String::from("session-id")],
                statuses: crate::submitter::SessionFilterStatuses::Exclude(vec![]),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "ListSessions").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn count_tasks() {
        let before = Client::get_nb_request("Submitter", "CountTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .count_tasks(crate::submitter::TaskFilter {
                ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from("session-id")]),
                statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "CountTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn try_get_result() {
        let before = Client::get_nb_request("Submitter", "TryGetResultStream").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .try_get_result("session-id", "result-id")
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "TryGetResultStream").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn try_get_task_output() {
        let before = Client::get_nb_request("Submitter", "TryGetTaskOutput").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .try_get_task_output("session-id", "task_id")
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "TryGetTaskOutput").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn wait_for_availability() {
        let before = Client::get_nb_request("Submitter", "WaitForAvailability").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .wait_for_availability("session-id", "result-id")
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "WaitForAvailability").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn wait_for_completion() {
        let before = Client::get_nb_request("Submitter", "WaitForCompletion").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .wait_for_completion(
                crate::submitter::TaskFilter {
                    ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from(
                        "session-id",
                    )]),
                    statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
                },
                true,
                true,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "WaitForCompletion").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel_tasks() {
        let before = Client::get_nb_request("Submitter", "CancelTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .cancel_tasks(crate::submitter::TaskFilter {
                ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from("session-id")]),
                statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "CancelTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn task_status() {
        let before = Client::get_nb_request("Submitter", "GetTaskStatus").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client.task_status(["task1", "task2"]).await.unwrap();
        let after = Client::get_nb_request("Submitter", "GetTaskStatus").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn result_status() {
        let before = Client::get_nb_request("Submitter", "GetResultStatus").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .result_status("session-id", ["result1", "result2"])
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "GetResultStatus").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn get_service_configuration_call() {
        let before = Client::get_nb_request("Submitter", "GetServiceConfiguration").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::get_service_configuration::Request {})
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "GetServiceConfiguration").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_session_call() {
        let before = Client::get_nb_request("Submitter", "CreateSession").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::create_session::Request {
                partition_ids: vec![String::from("part1"), String::from("part2")],
                default_task_options: crate::TaskOptions {
                    partition_id: String::from("part1"),
                    ..Default::default()
                },
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "CreateSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel_session_call() {
        let before = Client::get_nb_request("Submitter", "CancelSession").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::cancel_session::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "CancelSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_small_tasks_call() {
        let before = Client::get_nb_request("Submitter", "CreateSmallTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        match client
            .call(crate::submitter::create_tasks::SmallRequest {
                session_id: String::from("session-id"),
                task_options: None,
                task_requests: vec![],
            })
            .await
        {
            Ok(_) => (),
            Err(crate::client::RequestError::Grpc { source, .. }) => {
                if source.code() != tonic::Code::Internal || !source.message().is_empty() {
                    panic!("{source:?}")
                }
            }
        }
        let after = Client::get_nb_request("Submitter", "CreateSmallTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_large_tasks_call() {
        let before = Client::get_nb_request("Submitter", "CreateLargeTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        match client
            .call_streaming(futures::stream::iter([
                crate::submitter::create_tasks::LargeRequest::Invalid,
            ]))
            .await
        {
            Ok(_) => (),
            Err(crate::client::RequestError::Grpc { source, .. }) => {
                if source.code() != tonic::Code::Internal || !source.message().is_empty() {
                    panic!("{source:?}")
                }
            }
        }
        let after = Client::get_nb_request("Submitter", "CreateLargeTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn list_tasks_call() {
        let before = Client::get_nb_request("Submitter", "ListTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::list_tasks::Request {
                filter: crate::submitter::TaskFilter {
                    ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from(
                        "session-id",
                    )]),
                    statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
                },
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "ListTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn list_sessions_call() {
        let before = Client::get_nb_request("Submitter", "ListSessions").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::list_sessions::Request {
                filter: crate::submitter::SessionFilter {
                    ids: vec![String::from("session-id")],
                    statuses: crate::submitter::SessionFilterStatuses::Exclude(vec![]),
                },
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "ListSessions").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn count_tasks_call() {
        let before = Client::get_nb_request("Submitter", "CountTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::count_tasks::Request {
                filter: crate::submitter::TaskFilter {
                    ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from(
                        "session-id",
                    )]),
                    statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
                },
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "CountTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn try_get_result_call() {
        let before = Client::get_nb_request("Submitter", "TryGetResultStream").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::try_get_result::Request {
                session_id: String::from("session-id"),
                result_id: String::from("result-id"),
            })
            .await
            .unwrap()
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "TryGetResultStream").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn try_get_task_output_call() {
        let before = Client::get_nb_request("Submitter", "TryGetTaskOutput").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::try_get_task_output::Request {
                session_id: String::from("session-id"),
                task_id: String::from("task-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "TryGetTaskOutput").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn wait_for_availability_call() {
        let before = Client::get_nb_request("Submitter", "WaitForAvailability").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::wait_for_availability::Request {
                session_id: String::from("session-id"),
                result_id: String::from("result-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "WaitForAvailability").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn wait_for_completion_call() {
        let before = Client::get_nb_request("Submitter", "WaitForCompletion").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::wait_for_completion::Request {
                filter: crate::submitter::TaskFilter {
                    ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from(
                        "session-id",
                    )]),
                    statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
                },
                stop_on_first_task_cancellation: true,
                stop_on_first_task_error: true,
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "WaitForCompletion").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel_tasks_call() {
        let before = Client::get_nb_request("Submitter", "CancelTasks").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::cancel_tasks::Request {
                filter: crate::submitter::TaskFilter {
                    ids: crate::submitter::TaskFilterIds::Sessions(vec![String::from(
                        "session-id",
                    )]),
                    statuses: crate::submitter::TaskFilterStatuses::Exclude(vec![]),
                },
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "CancelTasks").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn task_status_call() {
        let before = Client::get_nb_request("Submitter", "GetTaskStatus").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::task_status::Request {
                task_ids: Vec::new(),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "GetTaskStatus").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn result_status_call() {
        let before = Client::get_nb_request("Submitter", "GetResultStatus").await;
        let mut client = Client::new().await.unwrap().into_submitter();
        client
            .call(crate::submitter::result_status::Request {
                session_id: String::from("session-id"),
                result_ids: Vec::new(),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Submitter", "GetResultStatus").await;
        assert_eq!(after - before, 1);
    }
}
