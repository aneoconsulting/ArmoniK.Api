use std::sync::Arc;

use hyper::Uri;
use hyper_rustls::{ConfigBuilderExt, FixedServerNameResolver};
use rustls::pki_types::ServerName;
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
pub use config::{ClientConfig, ClientConfigArgs, ConfigError};
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
    /// Create a new client using the configuration from the environment variables
    pub async fn new() -> Result<Self, ConnectionError> {
        Self::with_config(ClientConfig::from_env().context(ConfigSnafu {})?).await
    }

    /// Create a new client with the specified client configuration
    pub async fn with_config(config: ClientConfig) -> Result<Self, ConnectionError> {
        let endpoint = config.endpoint.clone();
        let override_target = config.override_target.clone();

        let https = Self::https_connector_builder(config).await?.build();

        let mut transport_endpoint = tonic::transport::Endpoint::from(endpoint.clone());
        if let Some(target) = override_target {
            transport_endpoint = transport_endpoint.origin(target);
        }

        // Build the actual channel from the configuration
        let channel = transport_endpoint
            .connect_with_connector(https)
            .await
            .context(TransportSnafu { endpoint })?;

        Ok(Self::with_channel(channel))
    }

    async fn https_connector_builder(
        config: ClientConfig,
    ) -> Result<
        hyper_rustls::HttpsConnectorBuilder<hyper_rustls::builderstates::WantsProtocols3>,
        ConnectionError,
    > {
        let endpoint = config.endpoint;

        // Get the default crypto provider or fallback to the ring crypto provider
        let crypto_provider = rustls::crypto::CryptoProvider::get_default()
            .cloned()
            .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));

        // Configure TLS with sane protocol defaults
        let tls_config = rustls::ClientConfig::builder_with_provider(crypto_provider)
            .with_safe_default_protocol_versions()
            .with_context(|_| TlsSnafu {
                endpoint: endpoint.clone(),
            })?;

        // Configure the server verification
        let tls_config = if config.allow_unsafe_connection {
            // Do not verify the server
            tls_config
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(crate::utils::InsecureCertVerifier))
        } else if let Some(cacert) = config.cacert {
            // Verify that the server certificate is signed with a specific CA cert
            let mut root_cert_store = rustls::RootCertStore::empty();
            root_cert_store.add(cacert).with_context(|_| TlsSnafu {
                endpoint: endpoint.clone(),
            })?;
            tls_config.with_root_certificates(root_cert_store)
        } else {
            // Verify the server certificate using the system CAs
            tls_config
                .with_native_roots()
                .with_context(|_| IoSnafu {})?
        };

        // Configure client identity for mTLS
        let tls_config = if let Some((cert, key)) = config.identity {
            // Use the the specified client certificate and key for the client authentication
            tls_config
                .with_client_auth_cert(vec![cert], key)
                .with_context(|_| TlsSnafu {
                    endpoint: endpoint.clone(),
                })?
        } else {
            // No mTLS
            tls_config.with_no_client_auth()
        };

        // Configure the connector to use http or https depending on the URI scheme
        let mut https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_or_http();

        if let Some(hostname) = &config.override_target {
            let server_name = ServerName::try_from(hostname.host().unwrap_or_default())
                .expect("A valid URI host should be a valid ServerName")
                .to_owned();
            https = https.with_server_name_resolver(FixedServerNameResolver::new(server_name));
        };

        Ok(https.enable_http1().enable_http2())
    }

    #[cfg(test)]
    async fn get_nb_request(service: &str, rpc: &str) -> usize {
        use std::collections::HashMap;

        use http_body_util::BodyExt;
        use hyper_util::rt::TokioExecutor;

        let mut config = ClientConfig::from_env().unwrap();

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

        let https = Self::https_connector_builder(config)
            .await
            .expect("Build connection information")
            .build();

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
    #[snafu(display("Could not read the client config [{location}]"))]
    #[non_exhaustive]
    Config {
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not connect to the remote {endpoint} [{location}]"))]
    #[non_exhaustive]
    Transport {
        endpoint: Uri,
        #[snafu(source(from(tonic::transport::Error, Box::new)))]
        source: Box<tonic::transport::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not establish TLS connection to the remote {endpoint} [{location}]"))]
    #[non_exhaustive]
    Tls {
        endpoint: Uri,
        #[snafu(source(from(rustls::Error, Box::new)))]
        source: Box<rustls::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not read system cert store [{location}]"))]
    #[non_exhaustive]
    Io {
        #[snafu(source(from(std::io::Error, Box::new)))]
        source: Box<std::io::Error>,
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
