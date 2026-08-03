//! RPC identity: service markers, call-kind markers, and the [`Rpc`] trait
//! tying each request type to the RPC it initiates.
//!
//! One [`service!`] invocation per service, in that service's own file,
//! declares every RPC and is validated against the protobuf descriptor at
//! expansion time.

/// Declare the RPCs of one proto service. See [`armonik_macros::service!`].
pub use armonik_macros::service;

mod agent;
mod applications;
mod auth;
mod events;
mod health_checks;
mod partitions;
mod results;
mod sessions;
mod submitter;
mod tasks;
mod versions;
mod worker;

/// Service marker types, one per proto service.
pub mod services {
    pub use super::agent::Agent;
    pub use super::applications::Applications;
    pub use super::auth::Auth;
    pub use super::events::Events;
    pub use super::health_checks::HealthChecks;
    pub use super::partitions::Partitions;
    pub use super::results::Results;
    pub use super::sessions::Sessions;
    pub use super::submitter::Submitter;
    pub use super::tasks::Tasks;
    pub use super::versions::Versions;
    pub use super::worker::Worker;
}

/// A proto service. One marker type per service, emitted by [`service!`].
pub trait Service {
    /// Fully-qualified proto service name.
    const NAME: &'static str;
}

/// Marker: unary RPC.
pub struct Unary;

/// Marker: server-streaming RPC.
pub struct ServerStream;

/// Marker: client-streaming RPC.
pub struct ClientStream;

/// A request type that identifies exactly one RPC. Implementations are
/// emitted by [`service!`].
///
/// Every RPC has a globally unique Rust request type (where the proto shares
/// one message across RPCs, the crate defines a distinct wire-compatible
/// struct per site), so the request type alone determines the service, the
/// method, the path and the response type.
pub trait Rpc: prost::Message + Default + std::fmt::Debug + 'static {
    /// The service this RPC belongs to.
    type Service: Service;
    /// [`Unary`], [`ServerStream`] or [`ClientStream`].
    type Kind;
    /// The response message, or the stream *item* for server-streaming RPCs.
    type Response: prost::Message + Default + std::fmt::Debug + 'static;

    /// Method name, as in the proto (`"ListResults"`).
    const METHOD: &'static str;
    /// Request path: `/package.Service/Method`.
    const PATH: &'static str;
    /// Telemetry label (`"Results::list"`).
    const LABEL: &'static str;
}
