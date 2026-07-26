use std::sync::Arc;

use crate::api::v3;
use crate::results;

super::define_trait_methods! {
    trait ResultsService {
        /// Get a results list using pagination, filters and sorting.
        fn results::list;

        /// Get the id of the task that should produce the result.
        fn results::get;

        /// Get the id of the task that should produce the result.
        fn results::get_owner_task_id;

        /// Create the metadata of multiple results at once.
        /// Data have to be uploaded separately.
        fn results::create_metadata;

        /// Create one result with data included in the request.
        fn results::create;

        /// Import existing data from the object storage into existing results.
        fn results::import;

        /// Delete data from multiple results.
        fn results::delete_data;

        /// Get the configuration of the service.
        fn results::get_service_configuration;

        ---
        /// Retrieve data.
        fn download(
            self: Arc<Self>,
            request: results::download::Request,
            context: crate::server::RequestContext,
        ) -> impl std::future::Future<
            Output = Result<
                impl tonic::codegen::tokio_stream::Stream<
                        Item = Result<results::download::Response, tonic::Status>,
                    > + Send,
                tonic::Status,
            >,
        > + Send;

        /// Upload data for result with stream.
        fn upload(
            self: Arc<Self>,
            request: impl tonic::codegen::tokio_stream::Stream<Item = Result<results::upload::Request, tonic::Status>> + Send + 'static,
            context: crate::server::RequestContext,
        ) -> impl std::future::Future<
            Output = Result<results::upload::Response, tonic::Status>
        > + Send;
    }
}

pub trait ResultsServiceExt {
    fn results_server(self) -> v3::results::results_server::ResultsServer<Self>
    where
        Self: Sized;
}

impl<T: ResultsService + Send + Sync + 'static> ResultsServiceExt for T {
    fn results_server(self) -> v3::results::results_server::ResultsServer<Self> {
        v3::results::results_server::ResultsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::results::results_server::Results) for ResultsService {
        fn list_results(crate::results::list::Request) -> crate::results::list::Response { list }
        fn get_result(crate::results::get::Request) -> crate::results::get::Response { get }
        fn get_owner_task_id(crate::results::get_owner_task_id::Request) -> crate::results::get_owner_task_id::Response { get_owner_task_id }
        fn create_results_meta_data(crate::results::create_metadata::Request) -> crate::results::create_metadata::Response { create_metadata }
        fn create_results(crate::results::create::Request) -> crate::results::create::Response { create }
        fn import_results_data(crate::results::import::Request) -> crate::results::import::Response { import }
        fn delete_results_data(crate::results::delete_data::Request) -> crate::results::delete_data::Response { delete_data }
        fn get_service_configuration(crate::Empty) -> crate::results::get_service_configuration::Response { get_service_configuration }

        ---

        async fn upload_result_data(
            self: std::sync::Arc<Self>,
            request: tonic::Request<tonic::Streaming<crate::results::upload::Request>>,
        ) -> std::result::Result<
            tonic::Response<crate::results::upload::Response>,
            tonic::Status,
        > {
            crate::server::impl_trait_methods!(stream client (self, request) {ResultsService::upload})
        }

        type DownloadResultDataStream = crate::server::ServerStream<crate::results::download::Response>;
        async fn download_result_data(
            self: std::sync::Arc<Self>,
            request: tonic::Request<crate::results::download::Request>,
        ) -> std::result::Result<
            tonic::Response<Self::DownloadResultDataStream>,
            tonic::Status,
        > {
            super::impl_trait_methods!(stream server (self, request) {ResultsService::download})
        }
    }
}
