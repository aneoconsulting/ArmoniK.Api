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
                    cancellation_token: tokio_util::sync::CancellationToken,
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
                    crate::server::impl_trait_methods!(unary (self, request) {T::$inner})
                }
            )*
            $($($body)*)?
        }
    };
    (unary ($self:ident, $request:ident) { $inner:path }) => {
        {
            let ct = tokio_util::sync::CancellationToken::new();
            let _drop_guard = ct.clone().drop_guard();
            let fut = tokio::spawn(async move { $inner($self, $request.into_inner().into(), ct).await});
            match fut.await {
                Ok(Ok(res)) => Ok(tonic::Response::new(res.into())),
                Ok(Err(err)) => Err(err),
                Err(err) => Err(tonic::Status::internal(err.to_string())),
            }
        }
    };
    (stream client ($self:ident, $request:ident) { $inner:path }) => {
        {
            let ct = tokio_util::sync::CancellationToken::new();
            let _drop_guard = ct.clone().drop_guard();
            let fut = tokio::spawn(async move {
                $inner(
                    $self,
                    tonic::codegen::tokio_stream::StreamExt::map($request.into_inner(), |r| r.map(Into::into)),
                    ct)
                .await
            });
            match fut.await {
                Ok(Ok(res)) => Ok(tonic::Response::new(res.into())),
                Ok(Err(err)) => Err(err),
                Err(err) => Err(tonic::Status::internal(err.to_string())),
            }
        }
    };
    (stream server ($self:ident, $request:ident) { $inner:path }) => {
        {
            let ct = tokio_util::sync::CancellationToken::new();
            let drop_guard = ct.clone().drop_guard();
            let fut = tokio::spawn(async move { $inner($self, $request.into_inner().into(), ct).await });
            match fut.await {
                Ok(Ok(stream)) => {
                    let (tx, rx) = tokio::sync::mpsc::channel(1);
                    tokio::spawn(async move {
                        let mut stream = std::pin::pin!(stream);

                        while let Some(res) = tonic::codegen::tokio_stream::StreamExt::next(&mut stream).await {
                            _ = tx.send(res.map(Into::into)).await;
                        }
                    });

                    Ok(tonic::Response::new(
                        crate::server::ServerStream{
                            receiver: rx,
                            drop_guard,
                        },
                    ))
                }
                Ok(Err(err)) => Err(err),
                Err(err) => Err(tonic::Status::internal(err.to_string())),
            }
        }
    };
}

pub struct ServerStream<T> {
    receiver: tokio::sync::mpsc::Receiver<Result<T, tonic::Status>>,
    #[allow(unused)]
    drop_guard: tokio_util::sync::DropGuard,
}

impl<T> crate::reexports::tokio_stream::Stream for ServerStream<T> {
    type Item = Result<T, tonic::Status>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

use define_trait_methods;
use impl_trait_methods;
