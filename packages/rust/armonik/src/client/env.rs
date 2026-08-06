//! ArmoniK's own environment vocabulary: `GrpcClient__*`.
//!
//! `armonik-transport` reads any prefix, `PascalCase` always; `ARMONIK_PREFIX` is what tells it
//! which, passed directly to `from_env` at the call site rather than read by this crate
//! variable by variable.

use super::{ConfigError, ConnectionError, EnvFieldError};

/// The prefix every `GrpcClient` option is read under.
pub const ARMONIK_PREFIX: &str = "GrpcClient__";

/// Creating a client from the environment.
#[derive(Debug, snafu::Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum NewClientError {
    #[snafu(display("Could not read the client configuration from the environment [{location}]"))]
    #[non_exhaustive]
    Env {
        #[snafu(source(from(EnvFieldError, Box::new)))]
        source: Box<EnvFieldError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display(
        "The environment does not describe a valid client configuration [{location}]"
    ))]
    #[non_exhaustive]
    Config {
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
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
    use crate::client::HttpConfig;

    /// Puts back what the variable held, rather than removing it: these tests run in a process whose
    /// environment may already carry a `GrpcClient__*` that other tests need. On drop, so that a
    /// failing test does not take it away from them either.
    struct Restore {
        name: String,
        previous: Option<std::ffi::OsString>,
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(previous) => std::env::set_var(&self.name, previous),
                None => std::env::remove_var(&self.name),
            }
        }
    }

    fn with_var<T>(name: &str, value: Option<&str>, body: impl FnOnce() -> T) -> T {
        let _restore = Restore {
            name: name.to_owned(),
            previous: std::env::var_os(name),
        };
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
        body()
    }

    #[test]
    #[serial_test::serial]
    fn a_variable_reaches_the_field_that_carries_it() {
        // The drift-prone half of ArmoniK's own vocabulary is the prefix and the naming, so that is
        // what is checked; `armonik-transport`'s own tests cover the mapping from field to variable
        // name in full. `UserAgent` on purpose: the tests that build a real client share this process
        // and are not serialised against this one, so borrowing a variable any of them depends on, the
        // endpoint above all, would send them somewhere else while this runs.
        let config = with_var(
            "GrpcClient__Endpoint",
            Some("http://localhost:5001"),
            || {
                with_var("GrpcClient__UserAgent", Some("armonik-test/1"), || {
                    HttpConfig::from_env(ARMONIK_PREFIX)
                })
            },
        )
        .expect("reading the environment must not fail");

        assert_eq!(
            config.user_agent,
            Some(
                armonik_transport::reexports::hyper::http::HeaderValue::from_static(
                    "armonik-test/1"
                )
            )
        );
    }
}
