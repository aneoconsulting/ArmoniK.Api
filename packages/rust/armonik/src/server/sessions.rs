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
        fn list_sessions(crate::sessions::list::Request) -> crate::sessions::list::Response { list }
        fn get_session(crate::sessions::get::Request) -> crate::sessions::get::Response { get }
        fn cancel_session(crate::sessions::cancel::Request) -> crate::sessions::cancel::Response { cancel }
        fn create_session(crate::sessions::create::Request) -> crate::sessions::create::Response { create }
        fn pause_session(crate::sessions::pause::Request) -> crate::sessions::pause::Response { pause }
        fn resume_session(crate::sessions::resume::Request) -> crate::sessions::resume::Response { resume }
        fn close_session(crate::sessions::close::Request) -> crate::sessions::close::Response { close }
        fn purge_session(crate::sessions::purge::Request) -> crate::sessions::purge::Response { purge }
        fn delete_session(crate::sessions::delete::Request) -> crate::sessions::delete::Response { delete }
        fn stop_submission(crate::sessions::stop_submission::Request) -> crate::sessions::stop_submission::Response { stop_submission }
    }
}
