use crate::{
    api::v3,
    objects::sessions::{
        SessionCreateRequest, SessionCreateResponse, SessionListRequest, SessionListResponse,
        SessionRaw,
    },
};

/// Service for handling sessions
#[derive(Clone)]
pub struct SessionsClient<T> {
    inner: v3::sessions::sessions_client::SessionsClient<T>,
}

impl<T> SessionsClient<T>
where
    T: Clone,
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
    pub async fn list(
        &mut self,
        request: SessionListRequest,
    ) -> Result<SessionListResponse, tonic::Status> {
        Ok(self.inner.list_sessions(request).await?.into_inner().into())
    }

    /// Get a session by its id.
    pub async fn get(&mut self, session_id: String) -> Result<SessionRaw, tonic::Status> {
        Ok(self
            .inner
            .get_session(v3::sessions::GetSessionRequest { session_id })
            .await?
            .into_inner()
            .session
            .into())
    }

    /// Cancel a session by its id.
    pub async fn cancel(&mut self, session_id: String) -> Result<SessionRaw, tonic::Status> {
        Ok(self
            .inner
            .cancel_session(v3::sessions::CancelSessionRequest { session_id })
            .await?
            .into_inner()
            .session
            .into())
    }

    /// Create a session.
    pub async fn create(
        &mut self,
        request: SessionCreateRequest,
    ) -> Result<SessionCreateResponse, tonic::Status> {
        Ok(self
            .inner
            .create_session(request)
            .await?
            .into_inner()
            .into())
    }
}
