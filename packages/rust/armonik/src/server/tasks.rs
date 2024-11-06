use std::sync::Arc;

use crate::api::v3;
use crate::tasks;

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
    fn tasks_server(self) -> v3::tasks::tasks_server::TasksServer<Self>
    where
        Self: Sized;
}

impl<T: TasksService + Send + Sync + 'static> TasksServiceExt for T {
    fn tasks_server(self) -> v3::tasks::tasks_server::TasksServer<Self> {
        v3::tasks::tasks_server::TasksServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::tasks::tasks_server::Tasks) for TasksService {
        fn list_tasks(v3::tasks::ListTasksRequest) -> v3::tasks::ListTasksResponse { list }
        fn list_tasks_detailed(v3::tasks::ListTasksRequest) -> v3::tasks::ListTasksDetailedResponse { list_detailed }
        fn get_task(v3::tasks::GetTaskRequest) -> v3::tasks::GetTaskResponse { get }
        fn cancel_tasks(v3::tasks::CancelTasksRequest) -> v3::tasks::CancelTasksResponse { cancel }
        fn get_result_ids(v3::tasks::GetResultIdsRequest) -> v3::tasks::GetResultIdsResponse { get_result_ids }
        fn count_tasks_by_status(v3::tasks::CountTasksByStatusRequest) -> v3::tasks::CountTasksByStatusResponse { count_status }
        fn submit_tasks(v3::tasks::SubmitTasksRequest) -> v3::tasks::SubmitTasksResponse { submit }
    }
}
