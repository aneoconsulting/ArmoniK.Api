//! ArmoniK servers for all the services.
//!
//! Each service is a trait (emitted by `service!`, one method per RPC with
//! the doc comments harvested from the protos) plus an `Ext` extension trait
//! whose `*_server` method wraps an implementation into a [`Router`] accepted
//! by `tonic::transport::Server::add_service`.

mod request_context;
pub(crate) mod router;

pub use request_context::RequestContext;
pub use router::Router;

// Mirror of the crossing in `client/mod.rs`: an `agent` build is the one that *serves* the agent
// service, a `worker` build the one that serves the worker service, and each calls the other.
#[cfg(feature = "agent")]
pub use crate::rpc::agent::{AgentService, AgentServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::applications::{ApplicationsService, ApplicationsServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::auth::{AuthService, AuthServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::events::{EventsService, EventsServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::health_checks::{HealthChecksService, HealthChecksServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::partitions::{PartitionsService, PartitionsServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::results::{ResultsService, ResultsServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::sessions::{SessionsService, SessionsServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::submitter::{SubmitterService, SubmitterServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::tasks::{TasksService, TasksServiceExt};
#[cfg(feature = "server")]
pub use crate::rpc::versions::{VersionsService, VersionsServiceExt};
#[cfg(feature = "worker")]
pub use crate::rpc::worker::{WorkerService, WorkerServiceExt};

/// The response stream of a server-streaming RPC, as framed by the router.
pub(crate) struct ServerStream<T> {
    pub(crate) receiver: tracing_futures::Instrumented<
        futures::stream::BoxStream<'static, Result<T, tonic::Status>>,
    >,
}

impl<T> crate::reexports::tokio_stream::Stream for ServerStream<T> {
    type Item = Result<T, tonic::Status>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // Through the wrapper, which is what enters the span the router traces the response items
        // under.
        std::pin::Pin::new(&mut self.receiver).poll_next(cx)
    }
}
