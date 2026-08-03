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

    use crate::Client;


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
}
