//! ArmoniK clients for all the services

use snafu::{ResultExt, Snafu};

// Re-exported here, so a caller reaches them through the client rather than through the transport
// crate.
#[cfg(feature = "_gen-client")]
pub use armonik_transport::{
    ClientConfig, ClientConfigArgs, ConfigError, ConnectionError, ProxyConfig, ProxyError,
    ProxySource, Secret,
};

/// Reading the `GrpcClient__*` variables, which is this crate's business rather than the transport's.
#[cfg(feature = "_gen-client")]
mod env;
#[cfg(feature = "_gen-client")]
pub use env::{EnvConfigError, FromEnv, NewClientError, ReadEnvError};

#[cfg(feature = "worker")]
mod agent;
#[cfg(feature = "client")]
mod applications;
#[cfg(feature = "client")]
mod auth;
#[cfg(feature = "client")]
mod events;
#[cfg(feature = "client")]
mod health_checks;
#[cfg(feature = "client")]
mod partitions;
#[cfg(feature = "client")]
mod results;
#[cfg(feature = "client")]
mod sessions;
#[cfg(feature = "client")]
mod submitter;
#[cfg(feature = "client")]
mod tasks;
#[cfg(feature = "client")]
mod versions;
#[cfg(feature = "agent")]
mod worker;

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
    pub async fn new() -> Result<Self, NewClientError> {
        let config = ClientConfig::from_env().context(env::ConfigSnafu {})?;
        Self::with_config(config)
            .await
            .context(env::ConnectSnafu {})
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

    #[cfg(test)]
    async fn get_nb_request(service: &str, rpc: &str) -> usize {
        use std::collections::HashMap;

        use http_body_util::BodyExt;
        use hyper_util::rt::TokioExecutor;

        let mut config = <ClientConfig as FromEnv>::from_env().unwrap();

        match std::env::var("Http__Endpoint") {
            Ok(value) if !value.is_empty() => {
                config.endpoint = hyper::Uri::try_from(value).expect("HTTP endpoint");
            }
            Ok(_) | Err(std::env::VarError::NotPresent) => {}
            Err(std::env::VarError::NotUnicode(value)) => {
                panic!("{value:?} is not a valid unicode string")
            }
        }

        let request = hyper::Request::get(format!("{}calls.json", config.endpoint))
            .body(http_body_util::Empty::<&[u8]>::new())
            .expect("Request");

        let https = armonik_transport::https_connector(config)
            .await
            .expect("Build connection information");

        let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https);

        let response = client.request(request).await.expect("/calls.json");

        let body = response.collect().await.expect("Response").to_bytes();

        let calls =
            serde_json::from_slice::<HashMap<String, HashMap<String, usize>>>(body.as_ref())
                .expect("Invalid JSON request");

        calls[service][rpc]
    }
}

impl<T> Client<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Build a client from a gRPC channel
    pub fn with_channel(channel: T) -> Self {
        Self { channel }
    }

    #[cfg(feature = "worker")]
    /// Create a borrowed [`Agent`]
    pub fn agent(&mut self) -> Agent<&mut Self> {
        Agent::with_channel(self)
    }
    #[cfg(feature = "worker")]
    /// Create an owned [`Agent`]
    pub fn into_agent(self) -> Agent<Self> {
        Agent::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Applications`]
    pub fn applications(&mut self) -> Applications<&mut Self> {
        Applications::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Applications`]
    pub fn into_applications(self) -> Applications<Self> {
        Applications::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Auth`]
    pub fn auth(&mut self) -> Auth<&mut Self> {
        Auth::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Auth`]
    pub fn into_auth(self) -> Auth<Self> {
        Auth::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Events`]
    pub fn events(&mut self) -> Events<&mut Self> {
        Events::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Events`]
    pub fn into_events(self) -> Events<Self> {
        Events::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`HealthChecks`]
    pub fn health_checks(&mut self) -> HealthChecks<&mut Self> {
        HealthChecks::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`HealthChecks`]
    pub fn into_health_checks(self) -> HealthChecks<Self> {
        HealthChecks::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Partitions`]
    pub fn partitions(&mut self) -> Partitions<&mut Self> {
        Partitions::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Partitions`]
    pub fn into_partitions(self) -> Partitions<Self> {
        Partitions::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Results`]
    pub fn results(&mut self) -> Results<&mut Self> {
        Results::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Results`]
    pub fn into_results(self) -> Results<Self> {
        Results::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Sessions`]
    pub fn sessions(&mut self) -> Sessions<&mut Self> {
        Sessions::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Sessions`]
    pub fn into_sessions(self) -> Sessions<Self> {
        Sessions::with_channel(self)
    }

    /// Create a borrowed [`Submitter`]
    #[cfg(feature = "client")]
    #[deprecated]
    #[allow(deprecated)]
    pub fn submitter(&mut self) -> Submitter<&mut Self> {
        Submitter::with_channel(self)
    }
    #[cfg(feature = "client")]
    #[deprecated]
    #[allow(deprecated)]
    /// Create an owned [`Submitter`]
    pub fn into_submitter(self) -> Submitter<Self> {
        Submitter::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Tasks`]
    pub fn tasks(&mut self) -> Tasks<&mut Self> {
        Tasks::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Tasks`]
    pub fn into_tasks(self) -> Tasks<Self> {
        Tasks::with_channel(self)
    }

    #[cfg(feature = "client")]
    /// Create a borrowed [`Versions`]
    pub fn versions(&mut self) -> Versions<&mut Self> {
        Versions::with_channel(self)
    }
    #[cfg(feature = "client")]
    /// Create an owned [`Versions`]
    pub fn into_versions(self) -> Versions<Self> {
        Versions::with_channel(self)
    }

    #[cfg(feature = "agent")]
    /// Create a borrowed [`Worker`]
    pub fn worker(&mut self) -> Worker<&mut Self> {
        Worker::with_channel(self)
    }
    #[cfg(feature = "agent")]
    /// Create an owned [`Worker`]
    pub fn into_worker(self) -> Worker<Self> {
        Worker::with_channel(self)
    }
}

impl<T> tonic::client::GrpcService<tonic::body::Body> for Client<T>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
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
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
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

/// Perform a gRPC call from a raw request.
#[allow(async_fn_in_trait)]
pub trait GrpcCall<Request> {
    type Response;
    type Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Request) -> Result<Self::Response, Self::Error>;
}

/// Perform a gRPC call from a raw request.
#[allow(async_fn_in_trait)]
pub trait GrpcCallStream<Request, Stream>
where
    Stream: futures::Stream<Item = Request> + Send + 'static,
{
    type Response;
    type Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Stream) -> Result<Self::Response, Self::Error>;
}

impl<Stream, Request, T> GrpcCall<Stream> for T
where
    Stream: futures::Stream<Item = Request> + Send + 'static,
    T: GrpcCallStream<Request, Stream>,
{
    type Response = <T as GrpcCallStream<Request, Stream>>::Response;
    type Error = <T as GrpcCallStream<Request, Stream>>::Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Stream) -> Result<Self::Response, Self::Error> {
        <T as GrpcCallStream<Request, Stream>>::call(self, request).await
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

macro_rules! impl_call {
    (@one $Client:ident($self:ident, $request:ident: $Request:ty) -> Result<$Response:ty> $block:block) => {
        crate::client::impl_call! {
            @one $Client($self, $request: $Request) -> Result<$Response, crate::client::RequestError> $block
        }
    };
    (@one $Client:ident($self:ident, $request:ident: $Request:ty) -> Result<$Response:ty, $Error:ty> $block:block) => {
        impl<T> $crate::client::GrpcCall<$Request> for &'_ mut $Client<T>
        where
            T: tonic::client::GrpcService<tonic::body::Body>,
            T::Error: Into<tonic::codegen::StdError>,
            T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
            <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
        {
            type Response = $Response;
            type Error = $Error;

            async fn call($self, $request: $Request) -> Result<Self::Response, Self::Error> $block
        }
    };
    ($Client:ident {$(async fn call($self:ident, $request:ident: $Request:ty) -> Result<$($Result:ty),*> $block:block)*}) => {
        $(
            crate::client::impl_call! {
                @one $Client($self, $request: $Request) -> Result<$($Result),*> $block
            }
        )*
    };
}

pub(crate) use impl_call;
