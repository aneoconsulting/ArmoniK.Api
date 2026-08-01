//! Building a client configuration out of the `GrpcClient__*` environment variables.
//!
//! This is integration with a deployment, not transport: `armonik-transport` takes the configuration
//! it is handed and never goes looking for one, so the vocabulary of ArmoniK's options and the reading
//! of them live here, in the crate that knows what a deployment looks like.

use snafu::{ResultExt, Snafu};

use super::{ClientConfig, ClientConfigArgs, ConfigError, ConnectionError};

/// Loading a value from the environment.
///
/// An extension trait because the types belong to `armonik-transport`, where an inherent method would
/// have to live and does not belong.
pub trait FromEnv: Sized {
    /// Read every `GrpcClient__*` variable this type understands.
    fn from_env() -> Result<Self, EnvConfigError>;
}

impl FromEnv for ClientConfigArgs {
    fn from_env() -> Result<Self, EnvConfigError> {
        let ctx = ReadSnafu {};
        Ok(Self {
            endpoint: read_env("GrpcClient__Endpoint").context(ctx)?,
            cert_pem: read_env("GrpcClient__CertPem").context(ctx)?,
            key_pem: read_env("GrpcClient__KeyPem").context(ctx)?,
            ca_cert: read_env("GrpcClient__CaCert").context(ctx)?,
            allow_unsafe_connection: read_env_bool("GrpcClient__AllowUnsafeConnection")
                .context(ctx)?,
            override_target_name: read_env("GrpcClient__OverrideTargetName").context(ctx)?,
            connect_timeout: read_env("GrpcClient__ConnectTimeout").context(ctx)?,
            timeout: read_env("GrpcClient__Timeout").context(ctx)?,
            rate_limit: read_env("GrpcClient__RateLimit").context(ctx)?,
            tcp_keepalive: read_env("GrpcClient__TcpKeepalive").context(ctx)?,
            tcp_keepalive_interval: read_env("GrpcClient__TcpKeepaliveInterval").context(ctx)?,
            tcp_keepalive_retries: read_env("GrpcClient__TcpKeepaliveRetries").context(ctx)?,
            tcp_nagle_algorithm: read_env_bool("GrpcClient__TcpNagleAlgorithm").context(ctx)?,
            http2_keep_alive_interval: read_env("GrpcClient__Http2KeepAliveInterval")
                .context(ctx)?,
            http2_keep_alive_timeout: read_env("GrpcClient__Http2KeepAliveTimeout").context(ctx)?,
            http2_keep_alive_while_idle: read_env_bool("GrpcClient__Http2KeepAliveWhileIdle")
                .context(ctx)?,
            http2_max_header_list_size: read_env("GrpcClient__Http2MaxHeaderListSize")
                .context(ctx)?,
            user_agent: read_env("GrpcClient__UserAgent").context(ctx)?,
            proxy: read_env("GrpcClient__Proxy").context(ctx)?,
            proxy_username: read_env("GrpcClient__ProxyUsername").context(ctx)?,
            proxy_password: read_env("GrpcClient__ProxyPassword").context(ctx)?.into(),
            reuse_ports: read_env_bool_opt("GrpcClient__ReusePorts").context(ctx)?,
        })
    }
}

impl FromEnv for ClientConfig {
    fn from_env() -> Result<Self, EnvConfigError> {
        Self::from_config_args(ClientConfigArgs::from_env()?).context(InvalidSnafu {})
    }
}

pub(crate) fn read_env(name: &str) -> Result<String, ReadEnvError> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(String::new()),
        Err(std::env::VarError::NotUnicode(value)) => NotUnicodeSnafu {
            name: name.to_owned(),
            value,
        }
        .fail(),
    }
}

pub(crate) fn read_env_bool(name: &str) -> Result<bool, ReadEnvError> {
    let value = read_env(name)?;
    match value.as_ref() {
        "0" | "false" | "no" | "disable" | "disallow" | "forbid" | "" => Ok(false),
        "1" | "true" | "yes" | "enable" | "allow" | "authorize" => Ok(true),
        _ => NotBooleanSnafu {
            name: name.to_owned(),
            value,
        }
        .fail(),
    }
}

/// Like [`read_env_bool`], but for an option whose default is not `false`, so that the caller can
/// tell an absent variable from one set to `false`.
///
/// Set but empty stays what it is for every other boolean here, namely `false`: `read_env` maps both
/// to the empty string, so this has to ask the environment itself rather than go through it.
pub(crate) fn read_env_bool_opt(name: &str) -> Result<Option<bool>, ReadEnvError> {
    match std::env::var(name) {
        Err(std::env::VarError::NotPresent) => Ok(None),
        _ => read_env_bool(name).map(Some),
    }
}

/// Turning the environment into a client configuration.
#[derive(Debug, Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum EnvConfigError {
    #[snafu(display("Could not read a `GrpcClient__*` environment variable [{location}]"))]
    #[non_exhaustive]
    Read {
        #[snafu(source(from(ReadEnvError, Box::new)))]
        source: Box<ReadEnvError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "The environment does not describe a valid client configuration [{location}]"
    ))]
    #[non_exhaustive]
    Invalid {
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

/// Creating a client from the environment.
#[derive(Debug, Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum NewClientError {
    #[snafu(display("Could not read the client configuration [{location}]"))]
    #[non_exhaustive]
    Config {
        #[snafu(source(from(EnvConfigError, Box::new)))]
        source: Box<EnvConfigError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not connect with that configuration [{location}]"))]
    #[non_exhaustive]
    Connect {
        #[snafu(source(from(ConnectionError, Box::new)))]
        source: Box<ConnectionError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum ReadEnvError {
    #[snafu(display(
        "Environment variable `{name}={value:?}` is not a valid unicode string [{location}]"
    ))]
    #[non_exhaustive]
    NotUnicode {
        name: String,
        value: std::ffi::OsString,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Environment variable `{name}={value}` is not a valid boolean [{location}]"))]
    #[non_exhaustive]
    NotBoolean {
        name: String,
        value: String,
        backtrace: snafu::Backtrace,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A variable name of its own per test, so that a stray value cannot leak between them even though
    /// they are serialised.
    fn with_var<T>(name: &str, value: Option<&str>, body: impl FnOnce() -> T) -> T {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
        let outcome = body();
        std::env::remove_var(name);
        outcome
    }

    #[test]
    #[serial_test::serial]
    fn every_accepted_spelling_is_accepted() {
        // The vocabulary is wider than `true`/`false` and there is no other record of it: the list is the
        // specification, so it is written out here rather than sampled.
        for spelling in ["1", "true", "yes", "enable", "allow", "authorize"] {
            let read = with_var("ARMONIK_TEST_BOOL", Some(spelling), || {
                read_env_bool("ARMONIK_TEST_BOOL")
            });
            assert!(read.expect(spelling), "`{spelling}` should read as true");
        }

        for spelling in ["0", "false", "no", "disable", "disallow", "forbid", ""] {
            let read = with_var("ARMONIK_TEST_BOOL", Some(spelling), || {
                read_env_bool("ARMONIK_TEST_BOOL")
            });
            assert!(!read.expect(spelling), "`{spelling}` should read as false");
        }
    }

    #[test]
    #[serial_test::serial]
    fn an_absent_variable_reads_as_none_but_an_empty_one_does_not() {
        // The option that defaults to on has to tell absent from empty, which `read_env_bool` cannot:
        // it maps both to `false`. Empty stays `false` here, so it means the same thing as it does for
        // every other boolean.
        let absent = with_var("ARMONIK_TEST_BOOL_OPT", None, || {
            read_env_bool_opt("ARMONIK_TEST_BOOL_OPT")
        });
        assert_eq!(absent.expect("unset"), None, "absent means unset");

        let empty = with_var("ARMONIK_TEST_BOOL_OPT", Some(""), || {
            read_env_bool_opt("ARMONIK_TEST_BOOL_OPT")
        });
        assert_eq!(
            empty.expect("empty"),
            Some(false),
            "set but empty should read as false"
        );

        let explicit = with_var("ARMONIK_TEST_BOOL_OPT", Some("true"), || {
            read_env_bool_opt("ARMONIK_TEST_BOOL_OPT")
        });
        assert_eq!(explicit.expect("explicit"), Some(true));
    }

    #[test]
    #[serial_test::serial]
    fn an_unset_variable_reads_as_false() {
        // Absent and empty are the same thing here, which is what lets every boolean option default to
        // off without the caller having to set it.
        let read = with_var("ARMONIK_TEST_BOOL", None, || {
            read_env_bool("ARMONIK_TEST_BOOL")
        });
        assert!(!read.expect("an unset variable"));
    }

    #[test]
    #[serial_test::serial]
    fn an_unrecognised_value_is_reported_with_its_name_and_value() {
        // The message has to carry both, or the reader is left guessing which of a dozen `GrpcClient__*`
        // variables was the problem.
        let read = with_var("ARMONIK_TEST_BOOL", Some("perhaps"), || {
            read_env_bool("ARMONIK_TEST_BOOL")
        });

        let error = read.expect_err("`perhaps` is not a boolean");
        let rendered = error.to_string();
        assert!(rendered.contains("ARMONIK_TEST_BOOL"), "{rendered}");
        assert!(rendered.contains("perhaps"), "{rendered}");
    }

    #[test]
    #[serial_test::serial]
    fn reading_a_plain_string_passes_it_through_and_maps_absent_to_empty() {
        let value = with_var("ARMONIK_TEST_STRING", Some(" spaced "), || {
            read_env("ARMONIK_TEST_STRING")
        });
        assert_eq!(value.expect("set"), " spaced ", "no trimming, no rewriting");

        let absent = with_var("ARMONIK_TEST_STRING", None, || {
            read_env("ARMONIK_TEST_STRING")
        });
        assert_eq!(absent.expect("unset"), "", "absent reads as empty");
    }

    #[test]
    #[serial_test::serial]
    fn the_endpoint_reaches_the_configuration() {
        // The one test that the whole chain is wired: a variable set here comes out of
        // `from_config_args` as a parsed endpoint.
        let config = with_var(
            "GrpcClient__Endpoint",
            Some("http://localhost:5001"),
            ClientConfig::from_env,
        )
        .expect("a lone endpoint is a valid configuration");

        assert_eq!(config.endpoint.to_string(), "http://localhost:5001/");
    }
}
