use http::Uri;
use snafu::{ResultExt, Snafu};
use tonic::transport::{Certificate, ClientTlsConfig, Identity};

/// Options for creating a gRPC Client
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct ClientConfig {
    /// Endpoint for sending requests
    pub endpoint: Uri,
    /// Allow unsafe connections to the endpoint (without SSL), defaults to false
    pub allow_unsafe_connection: bool,
    /// TLS identity of the client: key + cert
    pub identity: Option<Identity>,
    /// CA certificate to authenticate the server
    pub cacert: Option<Certificate>,
    /// Override the endpoint name during SSL verification
    pub override_target_name: String,
}

impl ClientConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let ctx = EnvSnafu {};
        let endpoint = crate::utils::read_env("GrpcClient__Endpoint").context(ctx)?;

        let cert_path = crate::utils::read_env("GrpcClient__CertPem").context(ctx)?;
        let key_path = crate::utils::read_env("GrpcClient__KeyPem").context(ctx)?;
        let cacert_path = crate::utils::read_env("GrpcClient__CaCert").context(ctx)?;

        let cacert = if !cacert_path.is_empty() {
            let cacert_pem = std::fs::read_to_string(cacert_path.clone())
                .context(IoSnafu { path: cacert_path })?;
            Some(Certificate::from_pem(cacert_pem))
        } else {
            None
        };

        let identity = match (cert_path.as_str(), key_path.as_str()) {
            ("", "") => None,
            ("", _) | (_, "") => return IncompatibleOptionsSnafu{msg: format!("`GrpcClient__CertPem={cert_path}` and `GrpcClient__KeyPem={key_path}` must be either both empty or both set")}.fail(),
            (cert_path, key_path) => {
                let cert_pem =
                    std::fs::read_to_string(cert_path).context(IoSnafu { path: cert_path })?;
                let key = std::fs::read(key_path).context(IoSnafu { path: key_path })?;
                let cert = Certificate::from_pem(cert_pem);
                Some(Identity::from_pem(cert, key))
            }
        };

        Ok(Self {
            endpoint: Uri::try_from(endpoint.clone()).context(UriSnafu { uri: endpoint })?,
            allow_unsafe_connection: crate::utils::read_env_bool(
                "GrpcClient__AllowUnsafeConnection",
            )
            .context(ctx)?,
            identity,
            cacert,
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
        let mut endpoint = Self::from(value.endpoint);

        if endpoint.uri().scheme_str().unwrap_or("http") == "https" {
            let mut tls_config = ClientTlsConfig::new();
            if let Some(cacert) = value.cacert {
                tls_config = tls_config.ca_certificate(cacert);
            }
            if let Some(identity) = value.identity {
                tls_config = tls_config.identity(identity);
            }

            if !value.override_target_name.is_empty() {
                tls_config = tls_config.domain_name(value.override_target_name.clone());
            }

            endpoint = endpoint.tls_config(tls_config).context(TlsSnafu {})?;
        }

        Ok(endpoint)
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
    #[snafu(display("Invalid TLS configuration [{location}]"))]
    #[non_exhaustive]
    Tls {
        #[snafu(source(from(tonic::transport::Error, Box::new)))]
        source: Box<tonic::transport::Error>,
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
    #[snafu(display("Could not read file `{path}` [{location}]"))]
    #[non_exhaustive]
    Io {
        #[snafu(source(from(std::io::Error, Box::new)))]
        source: Box<std::io::Error>,
        path: String,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("{msg} [{location}]"))]
    #[non_exhaustive]
    IncompatibleOptions {
        msg: String,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}
