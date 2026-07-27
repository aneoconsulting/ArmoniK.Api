use std::sync::Arc;

use crate::tasks;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::tasks::tasks_server as stub;

super::define_trait_methods! {
    trait TasksService {
        /// Get a tasks list using pagination, filters and sorting.
        fn tasks::list;

        /// Get a tasks list using pagination, filters and sorting.
        fn tasks::list_detailed;

        /// Get a task by its id.
        fn tasks::get;

        /// Cancel tasks using ids.
        fn tasks::cancel;

        /// Get ids of the result that tasks should produce.
        fn tasks::get_result_ids;

        /// Get count from tasks status.
        fn tasks::count_status;

        /// Create tasks metadata and submit task for processing.
        fn tasks::submit;
    }
}

pub trait TasksServiceExt {
    fn tasks_server(self) -> stub::TasksServer<Self>
    where
        Self: Sized;
}

impl<T: TasksService + Send + Sync + 'static> TasksServiceExt for T {
    fn tasks_server(self) -> stub::TasksServer<Self> {
        stub::TasksServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Tasks) for TasksService {
        fn list_tasks(crate::tasks::list::Request) -> crate::tasks::list::Response { list }
        fn list_tasks_detailed(crate::tasks::list::Request) -> crate::tasks::list_detailed::Response { list_detailed }
        fn get_task(crate::tasks::get::Request) -> crate::tasks::get::Response { get }
        fn cancel_tasks(crate::tasks::cancel::Request) -> crate::tasks::cancel::Response { cancel }
        fn get_result_ids(crate::tasks::get_result_ids::Request) -> crate::tasks::get_result_ids::Response { get_result_ids }
        fn count_tasks_by_status(crate::tasks::count_status::Request) -> crate::tasks::count_status::Response { count_status }
        fn submit_tasks(crate::tasks::submit::Request) -> crate::tasks::submit::Response { submit }
    }
}
