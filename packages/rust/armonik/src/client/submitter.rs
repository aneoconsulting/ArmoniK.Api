#![allow(deprecated)]

use futures::Stream;
use snafu::ResultExt;

use crate::rpc::services;
use crate::submitter::{create_tasks, try_get_task_output};
use crate::utils::IntoCollection;
use crate::{Output, TaskOptions, TaskRequest};

#[deprecated]
pub type Submitter<T = tonic::transport::Channel> = super::ServiceClient<services::Submitter, T>;

impl<T: super::Channel> super::ServiceClient<services::Submitter, T> {
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
}
