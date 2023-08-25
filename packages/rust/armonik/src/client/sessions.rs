use crate::{
    api::v3,
    objects::{
        sessions::{cancel, create, get, list, Raw},
        TaskOptions,
    },
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
    pub fn new(channel: T) -> Self {
        Self {
            inner: v3::sessions::sessions_client::SessionsClient::new(channel),
        }
    }

    /// Get a sessions list using pagination, filters and sorting.
    pub async fn list(&mut self, request: list::Request) -> Result<list::Response, tonic::Status> {
        self.call(request).await
    }

    /// Get a session by its id.
    pub async fn get(&mut self, session_id: impl Into<String>) -> Result<Raw, tonic::Status> {
        Ok(self
            .call(get::Request {
                id: session_id.into(),
            })
            .await?
            .session)
    }

    /// Cancel a session by its id.
    pub async fn cancel(&mut self, session_id: impl Into<String>) -> Result<Raw, tonic::Status> {
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
    ) -> Result<String, tonic::Status> {
        Ok(self
            .call(create::Request {
                default_task_option: task_options,
                partition_ids: partitions.into_iter().map(Into::into).collect(),
            })
            .await?
            .session_id)
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
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: get::Request) -> Result<get::Response> {
            Ok(self
                .inner
                .get_session(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: cancel::Request) -> Result<cancel::Response> {
            Ok(self
                .inner
                .cancel_session(request)
                .await?
                .into_inner()
                .into())
        }

        async fn call(self, request: create::Request) -> Result<create::Response> {
            Ok(self
                .inner
                .create_session(request)
                .await?
                .into_inner()
                .into())
        }
    }
}
