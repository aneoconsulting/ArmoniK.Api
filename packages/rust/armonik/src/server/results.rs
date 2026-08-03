use std::sync::Arc;

use crate::results;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::results::results_server as stub;

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

// Spike: the routing table `service!` will emit for this service. One entry
// per RPC, dispatching into the generic `serve_*` helpers; the tower plumbing
// lives once in `crate::server::router`.
impl<S: ResultsService + Send + Sync + 'static> super::router::Routes<S>
    for crate::rpc::services::Results
{
    const ROUTES: &'static [(&'static str, super::router::RouteFn<S>)] = &[
        (
            <results::list::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.list(r, c),
                    tracing::debug_span!("ResultsService::list"),
                ))
            },
        ),
        (
            <results::get::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.get(r, c),
                    tracing::debug_span!("ResultsService::get"),
                ))
            },
        ),
        (
            <results::get_owner_task_id::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.get_owner_task_id(r, c),
                    tracing::debug_span!("ResultsService::get_owner_task_id"),
                ))
            },
        ),
        (
            <results::create_metadata::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.create_metadata(r, c),
                    tracing::debug_span!("ResultsService::create_metadata"),
                ))
            },
        ),
        (
            <results::create::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.create(r, c),
                    tracing::debug_span!("ResultsService::create"),
                ))
            },
        ),
        (
            <results::import::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.import(r, c),
                    tracing::debug_span!("ResultsService::import"),
                ))
            },
        ),
        (
            <results::delete_data::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.delete_data(r, c),
                    tracing::debug_span!("ResultsService::delete_data"),
                ))
            },
        ),
        (
            <results::get_service_configuration::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_unary(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.get_service_configuration(r, c),
                    tracing::debug_span!("ResultsService::get_service_configuration"),
                ))
            },
        ),
        (
            <results::download::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_server_stream(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.download(r, c),
                    tracing::debug_span!("ResultsService::download"),
                ))
            },
        ),
        (
            <results::upload::Request as crate::rpc::Rpc>::PATH,
            |svc, req, config| {
                Box::pin(super::router::serve_client_stream(
                    svc,
                    req,
                    config,
                    |s: Arc<S>, r, c| s.upload(r, c),
                    tracing::debug_span!("ResultsService::upload"),
                ))
            },
        ),
    ];
}

pub trait ResultsServiceExt {
    fn results_server(self) -> stub::ResultsServer<Self>
    where
        Self: Sized;
}

impl<T: ResultsService + Send + Sync + 'static> ResultsServiceExt for T {
    fn results_server(self) -> stub::ResultsServer<Self> {
        stub::ResultsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Results) for ResultsService {
        fn list_results(crate::results::list::Request) -> crate::results::list::Response { list }
        fn get_result(crate::results::get::Request) -> crate::results::get::Response { get }
        fn get_owner_task_id(crate::results::get_owner_task_id::Request) -> crate::results::get_owner_task_id::Response { get_owner_task_id }
        fn create_results_meta_data(crate::results::create_metadata::Request) -> crate::results::create_metadata::Response { create_metadata }
        fn create_results(crate::results::create::Request) -> crate::results::create::Response { create }
        fn import_results_data(crate::results::import::Request) -> crate::results::import::Response { import }
        fn delete_results_data(crate::results::delete_data::Request) -> crate::results::delete_data::Response { delete_data }
        fn get_service_configuration(crate::results::get_service_configuration::Request) -> crate::results::get_service_configuration::Response { get_service_configuration }

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
