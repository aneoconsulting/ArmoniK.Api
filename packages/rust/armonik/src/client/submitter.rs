use std::collections::HashMap;

use futures::{Stream, StreamExt};

use crate::api::v3;
use crate::objects::submitter::{
    cancel_session, cancel_tasks, count_tasks, create_session, create_tasks, list_sessions,
    list_tasks, result_status, service_configuration, task_status, try_get_result,
    try_get_task_output, wait_for_availability, wait_for_completion, SessionFilter, TaskFilter,
};
use crate::objects::{Configuration, Output, ResultStatus, TaskOptions, TaskRequest, TaskStatus};

use super::{GrpcCall, GrpcCallStream};

#[derive(Clone)]
pub struct SubmitterClient<T> {
    inner: v3::submitter::submitter_client::SubmitterClient<T>,
}

impl<T> SubmitterClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::submitter::submitter_client::SubmitterClient::new(channel),
        }
    }

    pub async fn service_configuration(&mut self) -> Result<Configuration, tonic::Status> {
        self.call(service_configuration::Request {}).await
    }

    pub async fn create_session(
        &mut self,
        partitions: impl IntoIterator<Item = impl Into<String>>,
        task_options: TaskOptions,
    ) -> Result<String, tonic::Status> {
        Ok(self
            .call(create_session::Request {
                default_task_option: task_options,
                partition_ids: partitions.into_iter().map(Into::into).collect(),
            })
            .await?
            .session_id)
    }

    pub async fn cancel_session(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<(), tonic::Status> {
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
    ) -> Result<Vec<create_tasks::Status>, tonic::Status> {
        let response = self
            .call(create_tasks::SmallRequest {
                session_id: session_id.into(),
                task_options,
                task_requests: tasks.into_iter().map(Into::into).collect(),
            })
            .await?;

        match response {
            create_tasks::Response::Status(statuses) => Ok(statuses),
            create_tasks::Response::Error(msg) => Err(tonic::Status::internal(msg)),
        }
    }

    pub async fn create_large_tasks(
        &mut self,
        request: impl Stream<Item = create_tasks::LargeRequest> + Send + 'static,
    ) -> Result<Vec<create_tasks::Status>, tonic::Status> {
        let response = self.call(request).await?;

        match response {
            create_tasks::Response::Status(statuses) => Ok(statuses),
            create_tasks::Response::Error(msg) => Err(tonic::Status::internal(msg)),
        }
    }

    pub async fn list_tasks(&mut self, filter: TaskFilter) -> Result<Vec<String>, tonic::Status> {
        Ok(self.call(list_tasks::Request { filter }).await?.task_ids)
    }

    pub async fn list_sessions(
        &mut self,
        filter: SessionFilter,
    ) -> Result<Vec<String>, tonic::Status> {
        Ok(self
            .call(list_sessions::Request { filter })
            .await?
            .session_ids)
    }

    pub async fn count_tasks(
        &mut self,
        filter: TaskFilter,
    ) -> Result<HashMap<TaskStatus, i32>, tonic::Status> {
        Ok(self.call(count_tasks::Request { filter }).await?.values)
    }

    pub async fn try_get_result(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
    ) -> Result<impl Stream<Item = Result<try_get_result::Response, tonic::Status>>, tonic::Status>
    {
        Ok(self
            .inner
            .try_get_result_stream(try_get_result::Request {
                session_id: session_id.into(),
                result_id: result_id.into(),
            })
            .await?
            .into_inner()
            .map(|item| item.map(Into::into)))
    }

    pub async fn try_get_task_output(
        &mut self,
        session_id: impl Into<String>,
        task_id: impl Into<String>,
    ) -> Result<(), tonic::Status> {
        let response = self
            .call(try_get_task_output::Request {
                session_id: session_id.into(),
                task_id: task_id.into(),
            })
            .await?;

        match response {
            Output::Ok => Ok(()),
            Output::Error { details } => Err(tonic::Status::internal(details)),
        }
    }

    pub async fn wait_for_availability(
        &mut self,
        session_id: impl Into<String>,
        result_id: impl Into<String>,
    ) -> Result<wait_for_availability::Response, tonic::Status> {
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
    ) -> Result<HashMap<TaskStatus, i32>, tonic::Status> {
        Ok(self
            .call(wait_for_completion::Request {
                filter,
                stop_on_first_task_error,
                stop_on_first_task_cancellation,
            })
            .await?
            .values)
    }

    pub async fn cancel_tasks(&mut self, filter: TaskFilter) -> Result<(), tonic::Status> {
        self.call(cancel_tasks::Request { filter }).await?;
        Ok(())
    }

    pub async fn task_status(
        &mut self,
        task_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, TaskStatus>, tonic::Status> {
        Ok(self
            .call(task_status::Request {
                task_ids: task_ids.into_iter().map(Into::into).collect(),
            })
            .await?
            .statuses)
    }

    pub async fn result_status(
        &mut self,
        session_id: impl Into<String>,
        result_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<HashMap<String, ResultStatus>, tonic::Status> {
        Ok(self
            .call(result_status::Request {
                session_id: session_id.into(),
                result_ids: result_ids.into_iter().map(Into::into).collect(),
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
    SubmitterClient {
        async fn call(self, request: service_configuration::Request) -> Result<service_configuration::Response> {
            Ok(self
                .inner
                .get_service_configuration(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: create_session::Request) -> Result<create_session::Response> {
            Ok(self
                .inner
                .create_session(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel_session::Request) -> Result<cancel_session::Response> {
            Ok(self
                .inner
                .cancel_session(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: create_tasks::SmallRequest) -> Result<create_tasks::Response> {
            Ok(self
                .inner
                .create_small_tasks(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: list_tasks::Request) -> Result<list_tasks::Response> {
            Ok(self
                .inner
                .list_tasks(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: list_sessions::Request) -> Result<list_sessions::Response> {
            Ok(self
                .inner
                .list_sessions(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: count_tasks::Request) -> Result<count_tasks::Response> {
            Ok(self
                .inner
                .count_tasks(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: try_get_result::Request) -> Result<Box<dyn Stream<Item = Result<try_get_result::Response, tonic::Status>>>> {
            Ok(Box::new(self
                .inner
                .try_get_result_stream(request)
                .await?
                .into_inner()
                .map(|item| item.map(Into::into))))
        }

        async fn call(self, request: try_get_task_output::Request) -> Result<try_get_task_output::Response> {
            Ok(self
                .inner
                .try_get_task_output(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: wait_for_availability::Request) -> Result<wait_for_availability::Response> {
            Ok(self
                .inner
                .wait_for_availability(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: wait_for_completion::Request) -> Result<wait_for_completion::Response> {
            Ok(self
                .inner
                .wait_for_completion(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel_tasks::Request) -> Result<cancel_tasks::Response> {
            Ok(self
                .inner
                .cancel_tasks(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: task_status::Request) -> Result<task_status::Response> {
            Ok(self
                .inner
                .get_task_status(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: result_status::Request) -> Result<result_status::Response> {
            Ok(self
                .inner
                .get_result_status(request)
                .await?
                .into_inner()
                .into())
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<T, S> GrpcCallStream<create_tasks::LargeRequest, S> for &'_ mut SubmitterClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    S: Stream<Item = create_tasks::LargeRequest> + Send + 'static,
{
    type Response = create_tasks::Response;
    type Error = tonic::Status;

    async fn call(self, request: S) -> Result<Self::Response, Self::Error> {
        Ok(self
            .inner
            .create_large_tasks(request.map(Into::into))
            .await?
            .into_inner()
            .into())
    }
}
