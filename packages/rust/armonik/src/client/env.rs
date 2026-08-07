//! ArmoniK's own environment vocabulary: `GrpcClient__*`.
//!
//! A deliberately small reader: every `GrpcClient__*` variable is handed to serde as the raw text
//! it holds, and `armonik-transport`'s option readers do the interpreting. Reading the
//! environment is integration work, ArmoniK's own vocabulary, so it lives in this crate rather
//! than in the transport.

use armonik_transport::reexports::serde;
use snafu::ResultExt;

use super::{ConnectionError, HttpConfig};

/// The prefix every `GrpcClient` option is read under.
pub const ARMONIK_PREFIX: &str = "GrpcClient__";

/// Read every option from the `GrpcClient__*` variables.
pub(super) fn config_from_env() -> Result<HttpConfig, NewClientError> {
    use serde::Deserialize as _;

    // Raw text, verbatim: each option's own reader parses what it needs, so nothing guesses a
    // type here and a numeric-looking password survives byte for byte. Through `vars_os` and a
    // lossy decode, because the plain iterator panics on any non-Unicode variable in the
    // process, even one naming no option here.
    let options = std::env::vars_os().filter_map(|(name, value)| {
        let name = name.to_string_lossy();
        let option = name.strip_prefix(ARMONIK_PREFIX)?;
        Some((option.to_owned(), value.to_string_lossy().into_owned()))
    });
    HttpConfig::deserialize(serde::de::value::MapDeserializer::new(options)).context(EnvSnafu)
}

/// Creating a client from the environment.
#[derive(Debug, snafu::Snafu)]
#[non_exhaustive]
#[snafu(visibility(pub))]
pub enum NewClientError {
    #[snafu(display("Could not read the client configuration from the environment [{location}]"))]
    #[non_exhaustive]
    Env {
        #[snafu(source(from(serde::de::value::Error, Box::new)))]
        source: Box<serde::de::value::Error>,
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
