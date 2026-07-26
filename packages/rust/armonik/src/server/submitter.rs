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
            context: crate::server::RequestContext,
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
            context: crate::server::RequestContext,
        ) -> impl std::future::Future<
            Output = Result<submitter::create_tasks::Response, tonic::Status>
        > + Send;

        fn create_large_tasks(
            self: Arc<Self>,
            request: impl tonic::codegen::tokio_stream::Stream<Item = Result<submitter::create_tasks::LargeRequest, tonic::Status>> + Send + 'static,
            context: crate::server::RequestContext,
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
        fn get_service_configuration(crate::submitter::get_service_configuration::Request) -> crate::Configuration { get_service_configuration }
        fn create_session(crate::submitter::create_session::Request) -> crate::submitter::create_session::Response { create_session }
        fn cancel_session(crate::Session) -> crate::submitter::cancel_session::Response { cancel_session }
        fn create_small_tasks(crate::submitter::create_tasks::SmallRequest) -> crate::submitter::create_tasks::Response { create_small_tasks }
        fn list_tasks(crate::submitter::TaskFilter) -> crate::TaskIdList { list_tasks }
        fn list_sessions(crate::submitter::SessionFilter) -> crate::submitter::list_sessions::Response { list_sessions }
        fn count_tasks(crate::submitter::TaskFilter) -> crate::Count { count_tasks }
        fn try_get_task_output(crate::TaskOutputRequest) -> crate::Output { try_get_task_output }
        fn wait_for_availability(crate::ResultRequest) -> crate::submitter::wait_for_availability::Response { wait_for_availability }
        fn wait_for_completion(crate::submitter::wait_for_completion::Request) -> crate::Count { wait_for_completion }
        fn cancel_tasks(crate::submitter::TaskFilter) -> crate::submitter::cancel_tasks::Response { cancel_tasks }
        fn get_task_status(crate::submitter::task_status::Request) -> crate::submitter::task_status::Response { task_status }
        fn get_result_status(crate::submitter::result_status::Request) -> crate::submitter::result_status::Response { result_status }

        ---

        async fn create_large_tasks(
            self: std::sync::Arc<Self>,
            request: tonic::Request<tonic::Streaming<crate::submitter::create_tasks::LargeRequest>>,
        ) -> std::result::Result<tonic::Response<crate::submitter::create_tasks::Response>, tonic::Status> {
            crate::server::impl_trait_methods!(stream client (self, request) {SubmitterService::create_large_tasks})
        }


        type TryGetResultStreamStream = crate::server::ServerStream<crate::submitter::try_get_result::Response>;
        async fn try_get_result_stream(
            self: std::sync::Arc<Self>,
            request: tonic::Request<crate::ResultRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::TryGetResultStreamStream>,
            tonic::Status,
        > {
            super::impl_trait_methods!(stream server (self, request) {SubmitterService::try_get_result})
        }    }
}
