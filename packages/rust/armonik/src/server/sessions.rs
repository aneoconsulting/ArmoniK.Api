use std::sync::Arc;

use crate::api::v3;
use crate::sessions;

super::define_trait_methods! {
    trait SessionsService {
        /// Get a sessions list using pagination, filters and sorting.
        fn sessions::list;

        /// Get a session by its id.
        fn sessions::get;

        /// Cancel a session by its id.
        fn sessions::cancel;

        /// Create a session
        fn sessions::create;

        /// Pause a session by its id.
        fn sessions::pause;

        /// Resume a paused session by its id.
        fn sessions::resume;

        /// Close a session by its id.
        fn sessions::close;

        /// Purge a session by its id. Removes Results data.
        fn sessions::purge;

        /// Delete a session by its id. Removes metadata from Results, Sessions and Tasks associated to the session.
        fn sessions::delete;

        /// Stops clients and/or workers from submitting new tasks in the given session.
        fn sessions::stop_submission;
    }
}

pub trait SessionsServiceExt {
    fn sessions_server(self) -> v3::sessions::sessions_server::SessionsServer<Self>
    where
        Self: Sized;
}

impl<T: SessionsService + Send + Sync + 'static> SessionsServiceExt for T {
    fn sessions_server(self) -> v3::sessions::sessions_server::SessionsServer<Self> {
        v3::sessions::sessions_server::SessionsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::sessions::sessions_server::Sessions) for SessionsService {
        fn list_sessions(v3::sessions::ListSessionsRequest) -> v3::sessions::ListSessionsResponse { list }
        fn get_session(v3::sessions::GetSessionRequest) -> v3::sessions::GetSessionResponse { get }
        fn cancel_session(v3::sessions::CancelSessionRequest) -> v3::sessions::CancelSessionResponse { cancel }
        fn create_session(v3::sessions::CreateSessionRequest) -> v3::sessions::CreateSessionReply { create }
        fn pause_session(v3::sessions::PauseSessionRequest) -> v3::sessions::PauseSessionResponse { pause }
        fn resume_session(v3::sessions::ResumeSessionRequest) -> v3::sessions::ResumeSessionResponse { resume }
        fn close_session(v3::sessions::CloseSessionRequest) -> v3::sessions::CloseSessionResponse { close }
        fn purge_session(v3::sessions::PurgeSessionRequest) -> v3::sessions::PurgeSessionResponse { purge }
        fn delete_session(v3::sessions::DeleteSessionRequest) -> v3::sessions::DeleteSessionResponse { delete }
        fn stop_submission(v3::sessions::StopSubmissionRequest) -> v3::sessions::StopSubmissionResponse { stop_submission }
    }
}
