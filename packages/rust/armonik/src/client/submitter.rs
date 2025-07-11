#![allow(deprecated)]

use std::collections::HashMap;

use futures::{Stream, StreamExt};
use snafu::ResultExt;

use crate::api::v3;
use crate::submitter::{
    cancel_session, cancel_tasks, count_tasks, create_session, create_tasks,
    get_service_configuration, list_sessions, list_tasks, result_status, task_status,
    try_get_result, try_get_task_output, wait_for_availability, wait_for_completion, SessionFilter,
    TaskFilter,
};
use crate::utils::IntoCollection;
use crate::{Configuration, Output, ResultStatus, TaskOptions, TaskRequest, TaskStatus};

use super::{GrpcCall, GrpcCallStream};

#[derive(Clone)]
#[deprecated]
pub struct Submitter<T> {
    inner: v3::submitter::submitter_client::SubmitterClient<T>,
}

#[allow(deprecated)]
impl<T> Submitter<T>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::submitter::submitter_client::SubmitterClient::new(channel),
        }
    }

    pub async fn get_service_configuration(
        &mut self,
    ) -> Result<Configuration, super::RequestError> {
        self.call(get_service_configuration::Request {}).await
    }

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
        }
    }

    pub async fn list_tasks(
        &mut self,
        filter: TaskFilter,
    ) -> Result<Vec<String>, super::RequestError> {
        Ok(self.call(list_tasks::Request { filter }).await?.task_ids)
    }

    pub async fn list_sessions(
        &mut self,
        filter: SessionFilter,
    ) -> Result<Vec<String>, super::RequestError> {
        Ok(self
            .call(list_sessions::Request { filter })
            .await?
            .session_ids)
    }

    pub async fn count_tasks(
        &mut self,
        filter: TaskFilter,
    ) -> Result<HashMap<TaskStatus, i32>, super::RequestError> {
        Ok(self.call(count_tasks::Request { filter }).await?.values)
    }

    pub async fn try_get_result(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
    ) -> Result<
        impl Stream<Item = Result<try_get_result::Response, super::RequestError>>,
        super::RequestError,
    > {
        let span = tracing::debug_span!("Submitter::try_get_result");
        let call = tracing_futures::Instrument::instrument(
            self.inner.try_get_result_stream(try_get_result::Request {
                session_id: session_id.into(),
                result_id: result_id.into(),
            }),
            tracing::trace_span!(parent: &span, "rpc"),
        );
        let stream = call
            .await
            .context(super::GrpcSnafu {})?
            .into_inner()
            .map(|item| item.map(Into::into).context(super::GrpcSnafu {}));
        Ok(tracing_futures::Instrument::instrument(
            stream,
            tracing::trace_span!(parent: &span, "stream"),
        ))
    }

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

    pub async fn cancel_tasks(&mut self, filter: TaskFilter) -> Result<(), super::RequestError> {
        self.call(cancel_tasks::Request { filter }).await?;
        Ok(())
    }

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
    Submitter {
        async fn call(self, request: get_service_configuration::Request) -> Result<get_service_configuration::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .get_service_configuration(request),
                tracing::debug_span!("Submitter::get_service_configuration")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: create_session::Request) -> Result<create_session::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .create_session(request),
                tracing::debug_span!("Submitter::create_session")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel_session::Request) -> Result<cancel_session::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .cancel_session(request),
                tracing::debug_span!("Submitter::cancel_session")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: create_tasks::SmallRequest) -> Result<create_tasks::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .create_small_tasks(request),
                tracing::debug_span!("Submitter::create_tasks")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: list_tasks::Request) -> Result<list_tasks::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .list_tasks(request),
                tracing::debug_span!("Submitter::list_tasks")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: list_sessions::Request) -> Result<list_sessions::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .list_sessions(request),
                tracing::debug_span!("Submitter::list_sessions")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: count_tasks::Request) -> Result<count_tasks::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .count_tasks(request),
                tracing::debug_span!("Submitter::count_tasks")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: try_get_result::Request) -> Result<futures::stream::BoxStream<'static, Result<try_get_result::Response, tonic::Status>>> {
            let span = tracing::debug_span!("Submitter::try_get_result");
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .try_get_result_stream(request),
                tracing::trace_span!(parent: &span, "rpc")
            );
            let stream = call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .map(|item| item.map(Into::into));
            Ok(futures::stream::StreamExt::boxed(
                tracing_futures::Instrument::instrument(
                    stream,
                    tracing::trace_span!(parent: &span, "stream")
                )
            ))
        }

        async fn call(self, request: try_get_task_output::Request) -> Result<try_get_task_output::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .try_get_task_output(request),
                tracing::debug_span!("Submitter::try_get_task_output")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: wait_for_availability::Request) -> Result<wait_for_availability::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .wait_for_availability(request),
                tracing::debug_span!("Submitter::wait_for_availability")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: wait_for_completion::Request) -> Result<wait_for_completion::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .wait_for_completion(request),
                tracing::debug_span!("Submitter::wait_for_completion")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel_tasks::Request) -> Result<cancel_tasks::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .cancel_tasks(request),
                tracing::debug_span!("Submitter::cancel_tasks")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: task_status::Request) -> Result<task_status::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .get_task_status(request),
                tracing::debug_span!("Submitter::task_status")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: result_status::Request) -> Result<result_status::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .get_result_status(request),
                tracing::debug_span!("Submitter::result_status")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }
    }
}

impl<T, S> GrpcCallStream<create_tasks::LargeRequest, S> for &'_ mut Submitter<T>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    S: Stream<Item = create_tasks::LargeRequest> + Send + 'static,
{
    type Response = create_tasks::Response;
    type Error = super::RequestError;

    async fn call(self, request: S) -> Result<Self::Response, Self::Error> {
        let span = tracing::debug_span!("Submitter::create_tasks");
        let stream = tracing_futures::Instrument::instrument(
            request.map(Into::into),
            tracing::trace_span!(parent: &span, "stream"),
        );
        let call = tracing_futures::Instrument::instrument(
            self.inner.create_large_tasks(stream),
            tracing::trace_span!(parent: &span, "rpc"),
        );
        Ok(call.await.context(super::GrpcSnafu {})?.into_inner().into())
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
            .call(futures::stream::iter([
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
