//! ArmoniK's own environment vocabulary: `GrpcClient__*`.
//!
//! The prefix is all this crate contributes. `armonik-transport` reads the options under whichever
//! prefix it is given; naming that prefix is integration work, ArmoniK's own vocabulary, so it
//! lives here rather than in the transport.

use snafu::ResultExt;

use super::{ConnectionError, HttpConfig};

/// The prefix every `GrpcClient` option is read under.
pub const ARMONIK_PREFIX: &str = "GrpcClient__";

/// Read every option from the `GrpcClient__*` variables.
pub(super) fn config_from_env() -> Result<HttpConfig, NewClientError> {
    config_from(std::env::vars_os())
}

/// [`config_from_env`] over any set of variables, so the prefix rule can be exercised without
/// mutating the process environment, which every other test shares.
fn config_from(
    variables: impl IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
) -> Result<HttpConfig, NewClientError> {
    HttpConfig::from_env_vars(ARMONIK_PREFIX, variables).context(EnvSnafu)
}

/// Creating a client from the environment.
#[derive(Debug, snafu::Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum NewClientError {
    #[snafu(display("Could not read the client configuration from the environment [{location}]"))]
    #[non_exhaustive]
    Env {
        #[snafu(source(from(armonik_transport::EnvError, Box::new)))]
        source: Box<armonik_transport::EnvError>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An error and everything it was caused by: the outer message is generic, so an assertion
    /// has to look at the whole chain.
    fn chain(error: &dyn std::error::Error) -> String {
        let mut rendered = vec![error.to_string()];
        let mut current = error.source();
        while let Some(source) = current {
            rendered.push(source.to_string());
            current = source.source();
        }
        rendered.join(" -> ")
    }

    /// The variables a process would hold, as [`config_from`] receives them.
    fn variables<const N: usize>(
        pairs: [(&str, &str); N],
    ) -> impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)> {
        pairs
            .map(|(name, value)| (name.into(), value.into()))
            .into_iter()
    }

    #[test]
    fn an_option_is_read_from_the_variable_that_names_it() {
        let config = config_from(variables([
            ("GrpcClient__Endpoint", "http://localhost:5001"),
            ("GrpcClient__Timeout", "30s"),
        ]))
        .expect("a valid configuration");

        assert_eq!(config.endpoint.host(), Some("localhost"));
        assert_eq!(config.timeout, Some(std::time::Duration::from_secs(30)));
    }

    #[test]
    fn a_variable_without_the_prefix_names_no_option() {
        // The same word without the prefix belongs to something else entirely, and a process is
        // full of them.
        let config = config_from(variables([
            ("GrpcClient__Endpoint", "http://prefixed:5001"),
            ("Endpoint", "http://bare:5001"),
            ("Timeout", "1s"),
        ]))
        .expect("a valid configuration");

        assert_eq!(config.endpoint.host(), Some("prefixed"));
        assert_eq!(config.timeout, None);
    }

    #[test]
    fn a_variable_naming_no_option_is_ignored_rather_than_refused() {
        // `HttpConfig` flattens its units, which rules out denying unknown fields, so a prefixed
        // variable this crate reads no option from passes through silently.
        let config = config_from(variables([
            ("GrpcClient__Endpoint", "http://localhost:5001"),
            ("GrpcClient__NotAnOptionAnyoneReads", "value"),
        ]))
        .expect("an unknown option should not fail the read");

        assert_eq!(config.endpoint.host(), Some("localhost"));
    }

    #[test]
    fn an_option_that_cannot_be_read_names_the_variable_it_came_from() {
        let error = config_from(variables([
            ("GrpcClient__Endpoint", "http://localhost:5001"),
            ("GrpcClient__Timeout", "soon"),
        ]))
        .expect_err("`soon` is not a duration");

        // The full variable, so the message is the line of the deployment to go and fix rather
        // than a value to hunt for: eight options share the duration reader.
        let rendered = chain(&error);
        assert!(rendered.contains("`GrpcClient__Timeout`"), "{rendered}");
        assert!(rendered.contains("soon"), "{rendered}");
        assert!(rendered.contains("duration"), "{rendered}");
    }
}
