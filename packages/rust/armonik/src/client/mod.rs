use snafu::{ResultExt, Snafu};

mod agent;
mod applications;
mod auth;
mod config;
mod partitions;
mod results;
mod sessions;
mod submitter;
mod tasks;
mod versions;
mod worker;

pub use crate::utils::ReadEnvError;
pub use agent::AgentClient;
pub use applications::ApplicationsClient;
pub use auth::AuthClient;
pub use config::{ClientConfig, ConfigError};
pub use partitions::PartitionsClient;
pub use results::ResultsClient;
pub use sessions::SessionsClient;
#[allow(deprecated)]
pub use submitter::SubmitterClient;
pub use tasks::TasksClient;
pub use versions::VersionsClient;
pub use worker::WorkerClient;

#[derive(Clone)]
pub struct Client<T> {
    channel: T,
}

impl Client<tonic::transport::Channel> {
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<tonic::codegen::StdError>,
    {
        let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
        Ok(Self::with_channel(conn))
    }
    pub async fn new() -> Result<Self, ConnectionError> {
        let config = ClientConfig::from_env().context(ConfigSnafu {})?;
        let endpoint = config.endpoint.clone();
        Self::connect(config)
            .await
            .context(TransportSnafu { endpoint })
    }
}

impl<T> Client<T>
where
    T: Clone,
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    pub fn with_channel(channel: T) -> Self {
        Self { channel }
    }

    pub fn agent(&self) -> AgentClient<T> {
        AgentClient::with_channel(self.channel.clone())
    }

    pub fn applications(&self) -> ApplicationsClient<T> {
        ApplicationsClient::with_channel(self.channel.clone())
    }

    pub fn auth(&self) -> AuthClient<T> {
        AuthClient::with_channel(self.channel.clone())
    }

    pub fn partitions(&self) -> PartitionsClient<T> {
        PartitionsClient::with_channel(self.channel.clone())
    }

    pub fn results(&self) -> ResultsClient<T> {
        ResultsClient::with_channel(self.channel.clone())
    }

    pub fn sessions(&self) -> SessionsClient<T> {
        SessionsClient::with_channel(self.channel.clone())
    }

    #[deprecated]
    #[allow(deprecated)]
    pub fn submitter(&self) -> SubmitterClient<T> {
        SubmitterClient::with_channel(self.channel.clone())
    }

    pub fn tasks(&self) -> TasksClient<T> {
        TasksClient::with_channel(self.channel.clone())
    }

    pub fn versions(&self) -> VersionsClient<T> {
        VersionsClient::with_channel(self.channel.clone())
    }

    pub fn worker(&self) -> WorkerClient<T> {
        WorkerClient::with_channel(self.channel.clone())
    }
}

/// Perform a gRPC call from a raw request.
#[async_trait::async_trait(?Send)]
pub trait GrpcCall<Request> {
    type Response;
    type Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Request) -> Result<Self::Response, Self::Error>;
}

/// Perform a gRPC call from a raw request.
#[async_trait::async_trait(?Send)]
pub trait GrpcCallStream<Request, Stream>
where
    Stream: futures::Stream<Item = Request> + Send + 'static,
{
    type Response;
    type Error;

    /// Perform a gRPC call from a raw request.
    async fn call(self, request: Stream) -> Result<Self::Response, Self::Error>;
}

#[async_trait::async_trait(?Send)]
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
pub enum ConnectionError {
    #[snafu(display("Unable to read the client config [{location}]"))]
    #[non_exhaustive]
    Config {
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Error connecting to the remote {endpoint} [{location}]"))]
    #[non_exhaustive]
    Transport {
        endpoint: http::Uri,
        #[snafu(source(from(tonic::transport::Error, Box::new)))]
        source: Box<tonic::transport::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
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
        #[async_trait::async_trait(?Send)]
        impl<T> $crate::client::GrpcCall<$Request> for &'_ mut $Client<T>
        where
            T: tonic::client::GrpcService<tonic::body::BoxBody>,
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
