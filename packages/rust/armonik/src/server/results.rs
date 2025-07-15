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
        fn list_results(v3::results::ListResultsRequest) -> v3::results::ListResultsResponse { list }
        fn get_result(v3::results::GetResultRequest) -> v3::results::GetResultResponse { get }
        fn get_owner_task_id(v3::results::GetOwnerTaskIdRequest) -> v3::results::GetOwnerTaskIdResponse { get_owner_task_id }
        fn create_results_meta_data(v3::results::CreateResultsMetaDataRequest) -> v3::results::CreateResultsMetaDataResponse { create_metadata }
        fn create_results(v3::results::CreateResultsRequest) -> v3::results::CreateResultsResponse { create }
        fn import_results_data(v3::results::ImportResultsDataRequest) -> v3::results::ImportResultsDataResponse { import }
        fn delete_results_data(v3::results::DeleteResultsDataRequest) -> v3::results::DeleteResultsDataResponse { delete_data }
        fn get_service_configuration(v3::Empty) -> v3::results::ResultsServiceConfigurationResponse { get_service_configuration }

        ---

        async fn upload_result_data(
            self: std::sync::Arc<Self>,
            request: tonic::Request<tonic::Streaming<v3::results::UploadResultDataRequest>>,
        ) -> std::result::Result<
            tonic::Response<v3::results::UploadResultDataResponse>,
            tonic::Status,
        > {
            crate::server::impl_trait_methods!(stream client (self, request) {ResultsService::upload})
        }

        type DownloadResultDataStream = crate::server::ServerStream<v3::results::DownloadResultDataResponse>;
        async fn download_result_data(
            self: std::sync::Arc<Self>,
            request: tonic::Request<v3::results::DownloadResultDataRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::DownloadResultDataStream>,
            tonic::Status,
        > {
            super::impl_trait_methods!(stream server (self, request) {ResultsService::download})
        }

        type WatchResultsStream = crate::server::ServerStream<v3::results::WatchResultResponse>;
        async fn watch_results(
            self: std::sync::Arc<Self>,
            _request: tonic::Request<tonic::Streaming<v3::results::WatchResultRequest>>,
        ) -> std::result::Result<
            tonic::Response<Self::WatchResultsStream>,
            tonic::Status,
        > {
            let span = tracing::debug_span!("ResultsService::watch_results");
            let _entered = span.enter();
            Err(tonic::Status::unimplemented("Results::WatchResults is not implemented"))
        }
    }
}
