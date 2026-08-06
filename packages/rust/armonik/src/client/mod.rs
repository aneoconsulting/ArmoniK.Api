//! ArmoniK clients for all the services

use snafu::{ResultExt, Snafu};

// Re-exported here, so a caller reaches them through the client rather than through the transport
// crate.
#[cfg(feature = "_gen-client")]
use armonik_transport::ConfigSnafu;
#[cfg(feature = "_gen-client")]
pub use armonik_transport::{
    ClientConfig, ClientConfigArgs, ConfigError, ConnectionError, ReadEnvError,
};

mod service_client;
pub use service_client::{Channel, Dispatch, ServiceClient};

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
