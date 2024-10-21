use hyper::Uri;
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use snafu::{ResultExt, Snafu};

/// Options for creating a gRPC Client
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ClientConfig {
    /// Endpoint for sending requests
    pub endpoint: Uri,
    /// Allow unsafe connections to the endpoint (without SSL), defaults to false
    pub allow_unsafe_connection: bool,
    /// TLS identity of the client: key + cert
    pub identity: Option<(CertificateDer<'static>, PrivateKeyDer<'static>)>,
    /// CA certificate to authenticate the server
    pub cacert: Option<CertificateDer<'static>>,
    /// Override the endpoint name during SSL verification
    pub override_target_name: String,
}

impl Clone for ClientConfig {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            allow_unsafe_connection: self.allow_unsafe_connection,
            identity: self
                .identity
                .as_ref()
                .map(|(cert, key)| (cert.clone(), key.clone_key())),
            cacert: self.cacert.clone(),
            override_target_name: self.override_target_name.clone(),
        }
    }
}

/// Options for creating a gRPC Client (as given in the environment)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ClientConfigArgs {
    /// Endpoint for sending requests
    pub endpoint: String,
    /// Path to the certificate file in pem format
    pub cert_pem: String,
    /// Path to the key file in pem format
    pub key_pem: String,
    /// Path to the Certificate Authority file in pem format
    pub ca_cert: String,
    /// Allow unsafe connections to the endpoint (without SSL), defaults to false
    pub allow_unsafe_connection: bool,
    /// Override the endpoint name during SSL verification
    pub override_target_name: String,
}

impl ClientConfigArgs {
    pub fn from_env() -> Result<Self, super::ConfigError> {
        use crate::utils::{read_env, read_env_bool};
        let ctx = EnvSnafu {};
        Ok(Self {
            endpoint: read_env("GrpcClient__Endpoint").context(ctx)?,
            cert_pem: read_env("GrpcClient__CertPem").context(ctx)?,
            key_pem: read_env("GrpcClient__KeyPem").context(ctx)?,
            ca_cert: read_env("GrpcClient__CaCert").context(ctx)?,
            allow_unsafe_connection: read_env_bool("GrpcClient__AllowUnsafeConnection")
                .context(ctx)?,
            override_target_name: read_env("GrpcClient__OverrideTargetName").context(ctx)?,
        })
    }
}

impl ClientConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_config_args(ClientConfigArgs::from_env()?)
    }
    pub fn from_config_args(args: ClientConfigArgs) -> Result<Self, ConfigError> {
        tracing::debug!("GrpcClientConfig: {args:?}");

        let ClientConfigArgs {
            endpoint,
            cert_pem: cert_path,
            key_pem: key_path,
            ca_cert: cacert_path,
            allow_unsafe_connection,
            override_target_name,
        } = args;

        // Read CAcert file
        let cacert = if !cacert_path.is_empty() {
            let cacert_pem = std::fs::read_to_string(cacert_path.clone())
                .context(IoSnafu { path: cacert_path })?;
            Some(CertificateDer::from_pem_slice(cacert_pem.as_bytes()).context(TlsSnafu {})?)
        } else {
            None
        };

        // Read client cert and key files
        let identity = match (cert_path.as_str(), key_path.as_str()) {
            ("", "") => None,
            ("", _) | (_, "") => return IncompatibleOptionsSnafu{msg: format!("`GrpcClient__CertPem={cert_path}` and `GrpcClient__KeyPem={key_path}` must be either both empty or both set")}.fail(),
            (cert_path, key_path) => {
                let cert_pem =
                    std::fs::read_to_string(cert_path).context(IoSnafu { path: cert_path })?;
                let key_pem = std::fs::read(key_path).context(IoSnafu { path: key_path })?;
                let cert = CertificateDer::from_pem_slice(cert_pem.as_bytes()).context(TlsSnafu {})?;
                let key = PrivateKeyDer::from_pem_slice(key_pem.as_slice()).context(TlsSnafu{})?;

                Some((cert, key))
            }
        };

        Ok(Self {
            endpoint: Uri::try_from(endpoint.clone()).context(UriSnafu { uri: endpoint })?,
            allow_unsafe_connection,
            identity,
            cacert,
            override_target_name,
        })
    }
}

impl TryFrom<&ClientConfig> for tonic::transport::Endpoint {
    type Error = ConfigError;

    fn try_from(value: &ClientConfig) -> Result<Self, Self::Error> {
        Ok(Self::from(value.endpoint.clone()))
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
        #[snafu(source(from(rustls::pki_types::pem::Error, Box::new)))]
        source: Box<rustls::pki_types::pem::Error>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Endpoint URI is not valid: `{uri}` [{location}]"))]
    #[non_exhaustive]
    Uri {
        #[snafu(source(from(hyper::http::uri::InvalidUri, Box::new)))]
        source: Box<hyper::http::uri::InvalidUri>,
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
