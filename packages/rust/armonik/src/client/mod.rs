//! ArmoniK clients for all the services.
//!
//! # Writing a client method
//!
//! One impl block per service, carrying `#[armonik_macros::client]` and the proto service name. The
//! attribute prepends each method the RPC's documentation, harvested from the proto rather than
//! copied, and registers it so `every_rpc_has_a_client_method` can prove no RPC was forgotten.
//!
//! Methods are written out rather than generated. That is deliberate: a signature that is spelled
//! here cannot move when a field is added to the proto message behind it, which is what generating
//! them from the request's fields used to do to every caller.
//!
//! The common shape -- widen a few arguments, build the request, call, project one field -- goes
//! through `client_method!`, which spells the signature just the same:
//!
//! ```ignore
//! #[armonik_macros::client]
//! #[armonik(service = "armonik.api.grpc.v1.sessions.Sessions")]
//! impl<T: super::Channel> super::ServiceClient<services::Sessions, T> {
//!     client_method!(GetSession:
//!         get(session_id: into<String>)
//!         -> crate::sessions::get::Request => session: crate::sessions::Raw);
//! }
//! ```
//!
//! `into` / `iter` / `pairs` / `filters` / `plain` say how each argument is widened and converted
//! back; the table is in `client/method.rs`. Anything that does not fit -- the bodies that turn a oneof
//! response into an error, or build a request stream -- is an ordinary `fn` in the same block, under
//! `#[armonik(rpc = "MethodName")]`.

use snafu::{ResultExt, Snafu};

// Re-exported here, so a caller reaches them through the client rather than through the transport
// crate.
#[cfg(feature = "_gen-client")]
use armonik_transport::ConfigSnafu;
#[cfg(feature = "_gen-client")]
pub use armonik_transport::{
    ClientConfig, ClientConfigArgs, ConfigError, ConnectionError, ReadEnvError,
};

#[cfg(feature = "_gen-client")]
pub(crate) mod method;
#[cfg(feature = "_gen-client")]
pub(crate) use method::client_method;

mod service_client;
pub use service_client::{
    ByMessage, ByRequest, ByStream, ByStreamRequest, Channel, Dispatch, DispatchMessage, IntoCall,
    ServiceClient,
};

// The four use-case features are four distinct use cases, and a user normally wants exactly one.
// `Agent` and `Worker` are therefore gated on the *other* one of the pair, which reads as a typo
// and is not: a worker is what calls an agent, and an agent is what calls a worker. The rest of
// the services are what a `client` calls.
#[cfg(feature = "worker")]
pub mod agent;
#[cfg(feature = "client")]
pub mod applications;
#[cfg(feature = "client")]
pub mod auth;
#[cfg(feature = "client")]
pub mod events;
#[cfg(feature = "client")]
pub mod health_checks;
#[cfg(feature = "client")]
pub mod partitions;
#[cfg(feature = "client")]
pub mod results;
#[cfg(feature = "client")]
pub mod sessions;
#[cfg(feature = "client")]
pub mod submitter;
#[cfg(feature = "client")]
pub mod tasks;
#[cfg(feature = "client")]
pub mod versions;
#[cfg(feature = "agent")]
pub mod worker;

#[cfg(feature = "worker")]
pub use agent::Agent;
#[cfg(feature = "client")]
pub use applications::Applications;
#[cfg(feature = "client")]
pub use auth::Auth;
#[cfg(feature = "client")]
pub use events::Events;
#[cfg(feature = "client")]
pub use health_checks::HealthChecks;
#[cfg(feature = "client")]
pub use partitions::Partitions;
#[cfg(feature = "client")]
pub use results::Results;
#[cfg(feature = "client")]
pub use sessions::Sessions;
#[cfg(feature = "client")]
#[allow(deprecated)]
pub use submitter::Submitter;
#[cfg(feature = "client")]
pub use tasks::Tasks;
#[cfg(feature = "client")]
pub use versions::Versions;
#[cfg(feature = "agent")]
pub use worker::Worker;

/// ArmoniK Client
#[derive(Clone)]
pub struct Client<T = tonic::transport::Channel> {
    channel: T,
}

impl Client<tonic::transport::Channel> {
    /// Create a new client using the configuration from the environment variables
    pub async fn new() -> Result<Self, ConnectionError> {
        Self::with_config(ClientConfig::from_env().context(ConfigSnafu {})?).await
    }

    /// Create a new client with the specified client configuration
    pub async fn with_config(config: ClientConfig) -> Result<Self, ConnectionError> {
        let endpoint = config.endpoint.to_string();
        tracing_futures::Instrument::instrument(
            async move {
                Ok(Self::with_channel(
                    armonik_transport::connect(config).await?,
                ))
            },
            tracing::debug_span!("Client", endpoint),
        )
        .await
    }
}

/// One borrowed + one owned accessor per service on [`Client`].
macro_rules! services {
    ($($(#[$attr:meta])* $borrow:ident, $into:ident => $Service:ident;)*) => {
        $(
            $(#[$attr])*
            #[doc = concat!("Create a borrowed [`", stringify!($Service), "`]")]
            pub fn $borrow(&mut self) -> $Service<&mut Self> {
                $Service::with_channel(self)
            }

            $(#[$attr])*
            #[doc = concat!("Create an owned [`", stringify!($Service), "`]")]
            pub fn $into(self) -> $Service<Self> {
                $Service::with_channel(self)
            }
        )*
    };
}

impl<T> Client<T>
where
    T: Clone,
    T: crate::client::Channel,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self { channel }
    }

    services! {
        #[cfg(feature = "worker")] agent, into_agent => Agent;
        #[cfg(feature = "client")] applications, into_applications => Applications;
        #[cfg(feature = "client")] auth, into_auth => Auth;
        #[cfg(feature = "client")] events, into_events => Events;
        #[cfg(feature = "client")] health_checks, into_health_checks => HealthChecks;
        #[cfg(feature = "client")] partitions, into_partitions => Partitions;
        #[cfg(feature = "client")] results, into_results => Results;
        #[cfg(feature = "client")] sessions, into_sessions => Sessions;
        #[cfg(feature = "client")] #[deprecated] #[allow(deprecated)] submitter, into_submitter => Submitter;
        #[cfg(feature = "client")] tasks, into_tasks => Tasks;
        #[cfg(feature = "client")] versions, into_versions => Versions;
        #[cfg(feature = "agent")] worker, into_worker => Worker;
    }
}

impl<T> tonic::client::GrpcService<tonic::body::Body> for Client<T>
where
    T: crate::client::Channel,
{
    type ResponseBody = T::ResponseBody;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.channel.poll_ready(cx)
    }

    fn call(&mut self, request: tonic::codegen::http::Request<tonic::body::Body>) -> Self::Future {
        self.channel.call(request)
    }
}

impl<T> tonic::client::GrpcService<tonic::body::Body> for &'_ mut Client<T>
where
    T: crate::client::Channel,
{
    type ResponseBody = T::ResponseBody;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.channel.poll_ready(cx)
    }

    fn call(&mut self, request: tonic::codegen::http::Request<tonic::body::Body>) -> Self::Future {
        self.channel.call(request)
    }
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum RequestError {
    #[snafu(display("Grpc request error [{location}]"))]
    #[non_exhaustive]
    Grpc {
        #[snafu(source(from(tonic::Status, Box::new)))]
        source: Box<tonic::Status>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
