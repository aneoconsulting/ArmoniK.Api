use http::Uri;
use snafu::{ResultExt, Snafu};

/// Options for creating a gRPC Client
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ClientConfig {
    /// Endpoint for sending requests
    pub endpoint: Uri,
    /// Allow unsafe connections to the endpoint (without SSL), defaults to false
    pub allow_unsafe_connection: bool,
    /// Path to the certificate file in pem format
    pub cert_pem: String,
    /// Path to the key file in pem format
    pub key_pem: String,
    /// Path to the Certificate Authority file in pem format
    pub cacert_pem: String,
    /// Override the endpoint name during SSL verification
    pub override_target_name: String,
}

impl ClientConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let ctx = EnvSnafu {};
        let endpoint = crate::utils::read_env("GrpcClient__Endpoint").context(ctx)?;
        Ok(Self {
            endpoint: Uri::try_from(endpoint.clone()).context(UriSnafu { uri: endpoint })?,
            allow_unsafe_connection: crate::utils::read_env_bool(
                "GrpcClient__AllowUnsafeConnection",
            )
            .context(ctx)?,
            cert_pem: crate::utils::read_env("GrpcClient__CertPem").context(ctx)?,
            key_pem: crate::utils::read_env("GrpcClient__KeyPem").context(ctx)?,
            cacert_pem: crate::utils::read_env("GrpcClient__CaCert").context(ctx)?,
            override_target_name: crate::utils::read_env("GrpcClient__OverrideTargetName")
                .context(ctx)?,
        })
    }
}

impl TryFrom<&ClientConfig> for tonic::transport::Endpoint {
    type Error = ConfigError;

    fn try_from(value: &ClientConfig) -> Result<Self, Self::Error> {
        Ok(Self::from(value.endpoint.clone()))
    }
}
impl TryFrom<ClientConfig> for tonic::transport::Endpoint {
    type Error = ConfigError;

    fn try_from(value: ClientConfig) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum ConfigError {
    #[snafu(display("Could not read environment variable [{location}]"))]
    #[non_exhaustive]
    Env {
        #[snafu(source(from(crate::utils::ReadEnvError, Box::new)))]
        source: Box<crate::utils::ReadEnvError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Endpoint URI is not valid: `{uri}` [{location}]"))]
    #[non_exhaustive]
    Uri {
        #[snafu(source(from(http::uri::InvalidUri, Box::new)))]
        source: Box<http::uri::InvalidUri>,
        uri: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
