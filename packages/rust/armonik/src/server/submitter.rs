use std::sync::Arc;

use crate::api::v3;
use crate::submitter;

super::define_trait_methods! {
    trait SubmitterService {
        fn submitter::get_service_configuration;
        fn submitter::create_session;
        fn submitter::cancel_session;
        fn submitter::list_tasks;
        fn submitter::list_sessions;
        fn submitter::count_tasks;
        fn submitter::try_get_task_output;
        fn submitter::wait_for_availability;
        fn submitter::wait_for_completion;
        fn submitter::cancel_tasks;
        fn submitter::task_status;
        fn submitter::result_status;

        ---

        fn try_get_result(
            self: Arc<Self>,
            request: submitter::try_get_result::Request,
            cancellation_token: tokio_util::sync::CancellationToken,
        ) -> impl std::future::Future<
            Output = Result<
                impl tonic::codegen::tokio_stream::Stream<
                        Item = Result<submitter::try_get_result::Response, tonic::Status>,
                    > + Send,
                tonic::Status,
            >,
        > + Send;

        fn create_small_tasks(
            self: Arc<Self>,
            request: submitter::create_tasks::SmallRequest,
            cancellation_token: tokio_util::sync::CancellationToken
        ) -> impl std::future::Future<
            Output = Result<submitter::create_tasks::Response, tonic::Status>
        > + Send;

        fn create_large_tasks(
            self: Arc<Self>,
            request: impl tonic::codegen::tokio_stream::Stream<Item = Result<submitter::create_tasks::LargeRequest, tonic::Status>> + Send + 'static,
            cancellation_token: tokio_util::sync::CancellationToken
        ) -> impl std::future::Future<
            Output = Result<submitter::create_tasks::Response, tonic::Status>
        > + Send;
    }
}

pub trait SubmitterServiceExt {
    fn submitter_server(self) -> v3::submitter::submitter_server::SubmitterServer<Self>
    where
        Self: Sized;
}

impl<T: SubmitterService + Send + Sync + 'static> SubmitterServiceExt for T {
    fn submitter_server(self) -> v3::submitter::submitter_server::SubmitterServer<Self> {
        v3::submitter::submitter_server::SubmitterServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::submitter::submitter_server::Submitter) for SubmitterService {
        fn get_service_configuration(v3::Empty) -> v3::Configuration { get_service_configuration }
        fn create_session(v3::submitter::CreateSessionRequest) -> v3::submitter::CreateSessionReply { create_session }
        fn cancel_session(v3::Session) -> v3::Empty { cancel_session }
        fn create_small_tasks(v3::submitter::CreateSmallTaskRequest) -> v3::submitter::CreateTaskReply { create_small_tasks }
        fn list_tasks(v3::submitter::TaskFilter) -> v3::TaskIdList { list_tasks }
        fn list_sessions(v3::submitter::SessionFilter) -> v3::submitter::SessionIdList { list_sessions }
        fn count_tasks(v3::submitter::TaskFilter) -> v3::Count { count_tasks }
        fn try_get_task_output(v3::TaskOutputRequest) -> v3::Output { try_get_task_output }
        fn wait_for_availability(v3::ResultRequest) -> v3::submitter::AvailabilityReply { wait_for_availability }
        fn wait_for_completion(v3::submitter::WaitRequest) -> v3::Count { wait_for_completion }
        fn cancel_tasks(v3::submitter::TaskFilter) -> v3::Empty { cancel_tasks }
        fn get_task_status(v3::submitter::GetTaskStatusRequest) -> v3::submitter::GetTaskStatusReply { task_status }
        fn get_result_status(v3::submitter::GetResultStatusRequest) -> v3::submitter::GetResultStatusReply { result_status }

        ---

        async fn create_large_tasks(
            self: std::sync::Arc<Self>,
            request: tonic::Request<tonic::Streaming<v3::submitter::CreateLargeTaskRequest>>,
        ) -> std::result::Result<tonic::Response<v3::submitter::CreateTaskReply>, tonic::Status> {
            crate::server::impl_trait_methods!(stream client (self, request) {SubmitterService::create_large_tasks})
        }


        type TryGetResultStreamStream = crate::reexports::tokio_stream::wrappers::ReceiverStream<
            Result<v3::submitter::ResultReply, tonic::Status>,
        >;
        async fn try_get_result_stream(
            self: std::sync::Arc<Self>,
            request: tonic::Request<v3::ResultRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::TryGetResultStreamStream>,
            tonic::Status,
        > {
            super::impl_trait_methods!(stream server (self, request) {SubmitterService::try_get_result})
        }


        type WatchResultsStream = crate::reexports::tokio_stream::wrappers::ReceiverStream<
            Result<v3::submitter::WatchResultStream, tonic::Status>,
        >;
        async fn watch_results(
            self: std::sync::Arc<Self>,
            _request: tonic::Request<tonic::Streaming<v3::submitter::WatchResultRequest>>,
        ) -> std::result::Result<
            tonic::Response<Self::WatchResultsStream>,
            tonic::Status,
        > {
            todo!("Results::WatchResults is not implemented")
        }
    }
}
