#![allow(deprecated)]

use futures::Stream;
use snafu::ResultExt;

use crate::client::client_method;
use crate::rpc::services;
use crate::submitter::{create_tasks, try_get_task_output};
use crate::utils::IntoCollection;
use crate::{Output, TaskOptions, TaskRequest};

pub use crate::rpc::submitter::Client as Submitter;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.submitter.Submitter")]
impl<T: super::Channel> super::ServiceClient<services::Submitter, T> {
    #[deprecated]
    #[armonik(rpc = "CreateSmallTasks")]
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
            // A reply naming neither outcome. Not silently the empty success it used to be: the
            // tasks may well have been created, and the caller has no way to find out from here.
            create_tasks::Response::Invalid => Err(tonic::Status::internal(
                "the submitter's reply set neither creation_status_list nor error",
            ))
            .context(super::GrpcSnafu {}),
        }
    }

    #[deprecated]
    #[armonik(rpc = "CreateLargeTasks")]
    pub async fn create_large_tasks(
        &mut self,
        request: impl Stream<Item = create_tasks::LargeRequest> + Send + 'static,
    ) -> Result<Vec<create_tasks::Status>, super::RequestError> {
        let response = self.call(request).await?;

        match response {
            create_tasks::Response::Status(statuses) => Ok(statuses),
            create_tasks::Response::Error(msg) => {
                Err(tonic::Status::internal(msg)).context(super::GrpcSnafu {})
            }
            // A reply naming neither outcome. Not silently the empty success it used to be: the
            // tasks may well have been created, and the caller has no way to find out from here.
            create_tasks::Response::Invalid => Err(tonic::Status::internal(
                "the submitter's reply set neither creation_status_list nor error",
            ))
            .context(super::GrpcSnafu {}),
        }
    }

    #[deprecated]
    #[armonik(rpc = "TryGetTaskOutput")]
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
            // An output naming neither outcome, which is not the success it used to read as.
            Output::Invalid => Err(tonic::Status::internal(
                "the submitter's TryGetTaskOutput reply set neither ok nor error",
            ))
            .context(super::GrpcSnafu {}),
        }
    }

    client_method!(#[deprecated] GetServiceConfiguration:
        get_service_configuration()
        -> crate::submitter::get_service_configuration::Request => crate::submitter::get_service_configuration::Response);
    client_method!(#[deprecated] CreateSession:
        create_session(partition_ids: iter<String>, default_task_options: plain<crate::TaskOptions>)
        -> crate::submitter::create_session::Request => session_id: String);
    client_method!(#[deprecated] CancelSession:
        cancel_session(session_id: into<String>)
        -> crate::submitter::cancel_session::Request => ());
    client_method!(#[deprecated] ListTasks:
        list_tasks(filter: plain<crate::submitter::TaskFilter>)
        -> crate::submitter::list_tasks::Request => task_ids: Vec<String>);
    client_method!(#[deprecated] ListSessions:
        list_sessions(filter: plain<crate::submitter::SessionFilter>)
        -> crate::submitter::list_sessions::Request => session_ids: Vec<String>);
    client_method!(#[deprecated] CountTasks:
        count_tasks(filter: plain<crate::submitter::TaskFilter>)
        -> crate::submitter::count_tasks::Request => values: std::collections::HashMap<crate::TaskStatus, i32>);
    client_method!(#[deprecated] TryGetResultStream:
        try_get_result(session_id: into<String>, result_id: into<String>)
        -> stream crate::submitter::try_get_result::Request => crate::submitter::try_get_result::Response);
    client_method!(#[deprecated] WaitForAvailability:
        wait_for_availability(session_id: into<String>, result_id: into<String>)
        -> crate::submitter::wait_for_availability::Request => crate::submitter::wait_for_availability::Response);
    client_method!(#[deprecated] WaitForCompletion:
        wait_for_completion(filter: plain<crate::submitter::TaskFilter>, stop_on_first_task_error: plain<bool>, stop_on_first_task_cancellation: plain<bool>)
        -> crate::submitter::wait_for_completion::Request => values: std::collections::HashMap<crate::TaskStatus, i32>);
    client_method!(#[deprecated] CancelTasks:
        cancel_tasks(filter: plain<crate::submitter::TaskFilter>)
        -> crate::submitter::cancel_tasks::Request => ());
    client_method!(#[deprecated] GetTaskStatus:
        task_status(task_ids: iter<String>)
        -> crate::submitter::task_status::Request => statuses: std::collections::HashMap<String, crate::TaskStatus>);
    client_method!(#[deprecated] GetResultStatus:
        result_status(session_id: into<String>, result_ids: iter<String>)
        -> crate::submitter::result_status::Request => statuses: std::collections::HashMap<String, crate::ResultStatus>);
}
