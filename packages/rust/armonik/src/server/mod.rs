#[cfg(feature = "agent")]
mod agent;
#[cfg(feature = "server")]
mod applications;
#[cfg(feature = "server")]
mod auth;
#[cfg(feature = "server")]
mod events;
#[cfg(feature = "server")]
mod health_checks;
#[cfg(feature = "server")]
mod partitions;
mod request_context;
#[cfg(feature = "server")]
mod results;
#[cfg(feature = "server")]
mod sessions;
#[cfg(feature = "server")]
mod submitter;
#[cfg(feature = "server")]
mod tasks;
#[cfg(feature = "server")]
mod versions;
#[cfg(feature = "worker")]
mod worker;

#[cfg(feature = "agent")]
pub use agent::{AgentService, AgentServiceExt};
#[cfg(feature = "server")]
pub use applications::{ApplicationsService, ApplicationsServiceExt};
#[cfg(feature = "server")]
pub use auth::{AuthService, AuthServiceExt};
#[cfg(feature = "server")]
pub use events::{EventsService, EventsServiceExt};
#[cfg(feature = "server")]
pub use health_checks::{HealthChecksService, HealthChecksServiceExt};
#[cfg(feature = "server")]
pub use partitions::{PartitionsService, PartitionsServiceExt};
pub use request_context::RequestContext;
#[cfg(feature = "server")]
pub use results::{ResultsService, ResultsServiceExt};
#[cfg(feature = "server")]
pub use sessions::{SessionsService, SessionsServiceExt};
#[cfg(feature = "server")]
pub use submitter::{SubmitterService, SubmitterServiceExt};
#[cfg(feature = "server")]
pub use tasks::{TasksService, TasksServiceExt};
#[cfg(feature = "server")]
pub use versions::{VersionsService, VersionsServiceExt};
#[cfg(feature = "worker")]
pub use worker::{WorkerService, WorkerServiceExt};

macro_rules! define_trait_methods {
    (trait $name:ident {$($(#[$attr:meta])* fn $service:ident::$method:ident ;)* $(--- $($body:tt)*)?}) => {
        pub trait $name {
            $(
                $(#[$attr])*
                fn $method(
                    self: Arc<Self>,
                    request: $service::$method::Request,
                    context: crate::server::RequestContext,
                ) -> impl std::future::Future<Output = std::result::Result<$service::$method::Response, tonic::Status>> + Send;
            )*
            $($($body)*)?
        }
    };
}

macro_rules! impl_trait_methods {
    (impl ($name:ty) for $type:ident {$(fn $method:ident($request:ty) -> $response:ty {$inner:ident})* $(--- $($body:tt)*)?}) => {
        #[crate::reexports::async_trait]
        impl<T: $type + Send + Sync + 'static> $name for T {
            $(
                async fn $method(
                    self: Arc<Self>,
                    request: tonic::Request<$request>,
                ) -> std::result::Result<tonic::Response<$response>, tonic::Status> {
                    crate::server::impl_trait_methods!(unary (self, request) {$type::$inner})
                }
            )*
            $($($body)*)?
        }
    };
    (unary ($self:ident, $request:ident) { $inner:path }) => {
        {
            let (metadata_map, extensions, request) = $request.into_parts();
            let context = crate::server::RequestContext::new(metadata_map.into_headers(), extensions);
            let fut = tracing_futures::Instrument::instrument(
                $inner($self, request.into(), context),
                tracing::debug_span!(stringify!($inner)),
            );
            match fut.await {
                Ok(res) => Ok(tonic::Response::new(res.into())),
                Err(err) => Err(err),
            }
        }
    };
    (stream client ($self:ident, $request:ident) { $inner:path }) => {
        {
            let (metadata_map, extensions, request) = $request.into_parts();
            let context = crate::server::RequestContext::new(metadata_map.into_headers(), extensions);
            let span = tracing::debug_span!(stringify!($inner));
            let stream = tracing_futures::Instrument::instrument(
                tonic::codegen::tokio_stream::StreamExt::map(request, |r| r.map(Into::into)),
                tracing::trace_span!(parent: &span, "stream"),
            );
            let fut = tracing_futures::Instrument::instrument(
                $inner(
                    $self,
                    stream,
                    context,
                ),
                tracing::trace_span!(parent: &span, "rpc"),
            );
            match fut.await {
                Ok(res) => Ok(tonic::Response::new(res.into())),
                Err(err) => Err(err),
            }
        }
    };
    (stream server ($self:ident, $request:ident) { $inner:path }) => {
        {
            let (metadata_map, extensions, request) = $request.into_parts();
            let context = crate::server::RequestContext::new(metadata_map.into_headers(), extensions);
            let span = tracing::debug_span!(stringify!($inner));
            let fut = tracing_futures::Instrument::instrument(
                $inner($self, request.into(), context),
                tracing::trace_span!(parent: &span, "rpc")
            );
            match fut.await {
                Ok(stream) => {
                    let stream = tonic::codegen::tokio_stream::StreamExt::map(stream, |res| res.map(Into::into));
                    let stream = tracing_futures::Instrument::instrument(
                        futures::stream::StreamExt::boxed(stream),
                        tracing::trace_span!(parent: &span, "stream")
                    );

                    Ok(tonic::Response::new(
                        crate::server::ServerStream{
                            receiver: stream,
                        },
                    ))
                }
                Err(err) => Err(err),
            }
        }
    };
}

pub struct ServerStream<T> {
    receiver: tracing_futures::Instrumented<
        futures::stream::BoxStream<'static, Result<T, tonic::Status>>,
    >,
}

impl<T> crate::reexports::tokio_stream::Stream for ServerStream<T> {
    type Item = Result<T, tonic::Status>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.receiver.inner_mut().as_mut().poll_next(cx)
    }
}

use define_trait_methods;
use impl_trait_methods;
