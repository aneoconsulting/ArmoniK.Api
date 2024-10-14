use snafu::ResultExt;

use crate::{
    api::v3,
    objects::{
        sessions::{
            cancel, close, create, delete, get, list, pause, purge, resume, stop_submission, Raw,
        },
        TaskOptions,
    },
    utils::IntoCollection,
};

use super::GrpcCall;

/// Service for handling sessions
#[derive(Clone)]
pub struct SessionsClient<T> {
    inner: v3::sessions::sessions_client::SessionsClient<T>,
}

impl<T> SessionsClient<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::sessions::sessions_client::SessionsClient::new(channel),
        }
    }

    /// Get a sessions list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        request: list::Request,
    ) -> Result<list::Response, super::RequestError> {
        self.call(request).await
    }

    /// Get a session by its id.
    pub async fn get(&mut self, session_id: impl Into<String>) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(get::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Cancel a session by its id.
    pub async fn cancel(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(cancel::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Create a session.
    pub async fn create(
        &mut self,
        partitions: impl IntoIterator<Item = impl Into<String>>,
        task_options: TaskOptions,
    ) -> Result<String, super::RequestError> {
        Ok(self
            .call(create::Request {
                default_task_option: task_options,
                partition_ids: partitions.into_collect(),
            })
            .await?
            .session_id)
    }

    /// Pause a session by its id.
    pub async fn pause(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(pause::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Resume a paused session by its id.
    pub async fn resume(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(resume::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Close a session by its id.
    pub async fn close(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(close::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Purge a session by its id. Removes Results data.
    pub async fn purge(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(purge::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Delete a session by its id. Removes metadata from Results, Sessions and Tasks associated to the session.
    pub async fn delete(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(delete::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Stops clients and/or workers from submitting new tasks in the given session.
    pub async fn stop_submission(
        &mut self,
        session_id: impl Into<String>,
        stop_client: bool,
        stop_worker: bool,
    ) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(stop_submission::Request {
                id: session_id.into(),
                client: stop_client,
                worker: stop_worker,
            })
            .await?
            .session)
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
    SessionsClient {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            Ok(self
                .inner
                .list_sessions(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            Ok(self
                .inner
                .get_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel::Request) -> Result<cancel::Response> {
            Ok(self
                .inner
                .cancel_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: create::Request) -> Result<create::Response> {
            Ok(self
                .inner
                .create_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: pause::Request) -> Result<pause::Response> {
            Ok(self
                .inner
                .pause_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }


        async fn call(self, request: resume::Request) -> Result<resume::Response> {
            Ok(self
                .inner
                .resume_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: close::Request) -> Result<close::Response> {
            Ok(self
                .inner
                .close_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: purge::Request) -> Result<purge::Response> {
            Ok(self
                .inner
                .purge_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: delete::Request) -> Result<delete::Response> {
            Ok(self
                .inner
                .delete_session(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }

        async fn call(self, request: stop_submission::Request) -> Result<stop_submission::Response> {
            Ok(self
                .inner
                .stop_submission(request)
                .await
                .context(super::GrpcSnafu {})?
                .into_inner()
                .into())
        }
    }
}
