use snafu::ResultExt;

use crate::api::v3;
use crate::sessions::{
    cancel, close, create, delete, filter, get, list, pause, purge, resume, stop_submission, Raw,
    Sort,
};
use crate::utils::IntoCollection;
use crate::TaskOptions;

use super::GrpcCall;

/// Service for handling sessions
#[derive(Clone)]
pub struct Sessions<T> {
    inner: v3::sessions::sessions_client::SessionsClient<T>,
}

impl<T> Sessions<T>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self {
            inner: v3::sessions::sessions_client::SessionsClient::new(channel),
        }
    }

    /// Get a sessions list using pagination, filters and sorting.
    pub async fn list(
        &mut self,
        filters: impl IntoIterator<Item = impl IntoIterator<Item = filter::Field>>,
        sort: Sort,
        with_task_options: bool,
        page: i32,
        page_size: i32,
    ) -> Result<list::Response, super::RequestError> {
        self.call(list::Request {
            filters: filters
                .into_iter()
                .map(crate::utils::IntoCollection::into_collect)
                .collect(),
            sort,
            with_task_options,
            page,
            page_size,
        })
        .await
    }

    /// Get a session by its id.
    pub async fn get(&mut self, session_id: impl Into<String>) -> Result<Raw, super::RequestError> {
        Ok(self
            .call(get::Request {
                session_id: session_id.into(),
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
                session_id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Create a session.
    pub async fn create(
        &mut self,
        partitions: impl IntoIterator<Item = impl Into<String>>,
        default_task_options: TaskOptions,
    ) -> Result<String, super::RequestError> {
        Ok(self
            .call(create::Request {
                default_task_options,
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
                session_id: session_id.into(),
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
                session_id: session_id.into(),
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
                session_id: session_id.into(),
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
                session_id: session_id.into(),
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
                session_id: session_id.into(),
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
                session_id: session_id.into(),
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
    Sessions {
        async fn call(self, request: list::Request) -> Result<list::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .list_sessions(request),
                tracing::debug_span!("Sessions::list")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .get_session(request),
                tracing::debug_span!("Sessions::get")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel::Request) -> Result<cancel::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .cancel_session(request),
                tracing::debug_span!("Sessions::cancel")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: create::Request) -> Result<create::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .create_session(request),
                tracing::debug_span!("Sessions::create")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: pause::Request) -> Result<pause::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .pause_session(request),
                tracing::debug_span!("Sessions::pause")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }


        async fn call(self, request: resume::Request) -> Result<resume::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .resume_session(request),
                tracing::debug_span!("Sessions::resume")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: close::Request) -> Result<close::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .close_session(request),
                tracing::debug_span!("Sessions::close")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: purge::Request) -> Result<purge::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .purge_session(request),
                tracing::debug_span!("Sessions::purge")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: delete::Request) -> Result<delete::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .delete_session(request),
                tracing::debug_span!("Sessions::delete")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }

        async fn call(self, request: stop_submission::Request) -> Result<stop_submission::Response> {
            let call = tracing_futures::Instrument::instrument(
                self
                    .inner
                    .stop_submission(request),
                tracing::debug_span!("Sessions::stop_submission")
            );
            Ok(call
                .await
                .context(super::GrpcSnafu{})?
                .into_inner()
                .into())
        }
    }
}

#[cfg(test)]
#[serial_test::serial(sessions)]
mod tests {
    use crate::{Client, TaskOptions};

    // Named methods

    #[tokio::test]
    async fn list() {
        let before = Client::get_nb_request("Sessions", "ListSessions").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .list(
                crate::sessions::filter::Or {
                    or: vec![crate::sessions::filter::And { and: vec![] }],
                },
                crate::sessions::Sort::default(),
                true,
                0,
                10,
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "ListSessions").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get() {
        let before = Client::get_nb_request("Sessions", "GetSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client.get("session-id").await.unwrap();
        let after = Client::get_nb_request("Sessions", "GetSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel() {
        let before = Client::get_nb_request("Sessions", "CancelSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client.cancel("session-id").await.unwrap();
        let after = Client::get_nb_request("Sessions", "CancelSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create() {
        let before = Client::get_nb_request("Sessions", "CreateSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .create(
                ["part1", "part2"],
                TaskOptions {
                    partition_id: String::from("part1"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "CreateSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn pause() {
        let before = Client::get_nb_request("Sessions", "PauseSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client.pause("session-id").await.unwrap();
        let after = Client::get_nb_request("Sessions", "PauseSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn resume() {
        let before = Client::get_nb_request("Sessions", "ResumeSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client.resume("session-id").await.unwrap();
        let after = Client::get_nb_request("Sessions", "ResumeSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn close() {
        let before = Client::get_nb_request("Sessions", "CloseSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client.close("session-id").await.unwrap();
        let after = Client::get_nb_request("Sessions", "CloseSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn purge() {
        let before = Client::get_nb_request("Sessions", "PurgeSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client.purge("session-id").await.unwrap();
        let after = Client::get_nb_request("Sessions", "PurgeSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn delete() {
        let before = Client::get_nb_request("Sessions", "DeleteSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client.delete("session-id").await.unwrap();
        let after = Client::get_nb_request("Sessions", "DeleteSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn stop_submission() {
        let before = Client::get_nb_request("Sessions", "StopSubmission").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .stop_submission("session-id", true, true)
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "StopSubmission").await;
        assert_eq!(after - before, 1);
    }

    // Explicit call request

    #[tokio::test]
    async fn list_call() {
        let before = Client::get_nb_request("Sessions", "ListSessions").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::list::Request {
                page_size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "ListSessions").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn get_call() {
        let before = Client::get_nb_request("Sessions", "GetSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::get::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "GetSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn cancel_call() {
        let before = Client::get_nb_request("Sessions", "CancelSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::cancel::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "CancelSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn create_call() {
        let before = Client::get_nb_request("Sessions", "CreateSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::create::Request {
                default_task_options: TaskOptions {
                    partition_id: String::from("part1"),
                    ..Default::default()
                },
                partition_ids: vec![String::from("part1"), String::from("part2")],
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "CreateSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn pause_call() {
        let before = Client::get_nb_request("Sessions", "PauseSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::pause::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "PauseSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn resume_call() {
        let before = Client::get_nb_request("Sessions", "ResumeSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::resume::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "ResumeSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn close_call() {
        let before = Client::get_nb_request("Sessions", "CloseSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::close::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "CloseSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn purge_call() {
        let before = Client::get_nb_request("Sessions", "PurgeSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::purge::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "PurgeSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn delete_call() {
        let before = Client::get_nb_request("Sessions", "DeleteSession").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::delete::Request {
                session_id: String::from("session-id"),
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "DeleteSession").await;
        assert_eq!(after - before, 1);
    }

    #[tokio::test]
    async fn stop_submission_call() {
        let before = Client::get_nb_request("Sessions", "StopSubmission").await;
        let mut client = Client::new().await.unwrap().into_sessions();
        client
            .call(crate::sessions::stop_submission::Request {
                session_id: String::from("session-id"),
                client: true,
                worker: true,
            })
            .await
            .unwrap();
        let after = Client::get_nb_request("Sessions", "StopSubmission").await;
        assert_eq!(after - before, 1);
    }
}
