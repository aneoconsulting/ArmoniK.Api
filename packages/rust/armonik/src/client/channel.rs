//! Assembling a `tonic` channel from an [`HttpConfig`].
//!
//! `armonik-transport` hands back a connector: TCP, the proxy tunnel, TLS and mTLS. The HTTP/2
//! engine on top of it is this crate's choice, so the options only an engine can apply - the request
//! timeout, the rate limit, the HTTP/2 keepalives, the user agent, and the origin the transport
//! resolves - are applied here.

use armonik_transport::{ConfigError, ConnectionError, HttpConfig};
use snafu::{ResultExt, Snafu};
use tonic::codegen::http::Uri;
use tonic::transport::{Channel, Endpoint};

/// Connect to the endpoint described by `config`, eagerly: this resolves once the connection is
/// established, not lazily on the first request.
pub async fn connect(config: HttpConfig) -> Result<Channel, ChannelError> {
    // The one resolution of the configuration's origin: the connector verifies the certificate
    // against it, the channel addresses its requests to it.
    let origin = Uri::try_from(&config).context(ConfigSnafu)?;
    let endpoint = config.endpoint.clone();
    let http2 = config.http2;
    let user_agent = config.user_agent.clone();
    let timeout = config.timeout;
    let rate_limit = config.rate_limit;

    let https =
        armonik_transport::https_connector(config, origin.clone()).context(ConnectorSnafu)?;

    // The endpoint is what gets dialled, the origin what requests are addressed to. No condition on
    // the two being the same: with no override they are, which is what a channel does by default.
    let mut transport_endpoint = Endpoint::from(endpoint.clone()).origin(origin);

    if let Some(timeout) = timeout {
        transport_endpoint = transport_endpoint.timeout(timeout);
    }
    if let Some((limit, duration)) = rate_limit {
        transport_endpoint = transport_endpoint.rate_limit(limit, duration);
    }

    if let Some(interval) = http2.keep_alive_interval {
        transport_endpoint = transport_endpoint.http2_keep_alive_interval(interval);
    }
    if let Some(timeout) = http2.keep_alive_timeout {
        transport_endpoint = transport_endpoint.keep_alive_timeout(timeout);
    }
    transport_endpoint = transport_endpoint.keep_alive_while_idle(http2.keep_alive_while_idle);
    if let Some(max) = http2.max_header_list_size {
        transport_endpoint = transport_endpoint.http2_max_header_list_size(max);
    }
    if let Some(ua) = user_agent {
        transport_endpoint = transport_endpoint
            .user_agent(ua)
            .expect("HeaderValue is already validated, conversion is infallible");
    }

    transport_endpoint
        .connect_with_connector(https)
        .await
        .context(TransportSnafu { endpoint })
}

/// Everything that can go wrong between an [`HttpConfig`] and a usable channel.
#[derive(Debug, Snafu)]
#[non_exhaustive]
// snafu keeps its generated context selectors module-private by default. Public so that a caller in
// another crate can build one of these errors with the location captured at its own call site.
#[snafu(visibility(pub))]
pub enum ChannelError {
    #[snafu(display("Could not read the client config [{location}]"))]
    #[non_exhaustive]
    Config {
        #[snafu(source(from(ConfigError, Box::new)))]
        source: Box<ConfigError>,
        #[snafu(implicit)]
        location: snafu::Location,
    },
    #[snafu(display("Could not build the connector [{location}]"))]
    #[non_exhaustive]
    Connector {
        #[snafu(source(from(ConnectionError, Box::new)))]
        source: Box<ConnectionError>,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every message in the chain, joined.
    fn chain(error: &ChannelError) -> String {
        let mut rendered = error.to_string();
        let mut source = std::error::Error::source(error);
        while let Some(cause) = source {
            rendered.push_str(" | ");
            rendered.push_str(&cause.to_string());
            source = cause.source();
        }
        rendered
    }

    #[tokio::test]
    async fn an_empty_endpoint_is_rejected_before_anything_is_dialled() {
        // An unset endpoint deserialises to the default URI rather than failing, so the rejection
        // has to happen here, where the error can name the option.
        let error = connect(HttpConfig::default())
            .await
            .expect_err("an empty endpoint cannot be connected to");

        assert!(matches!(error, ChannelError::Config { .. }), "{error:?}");
        assert!(
            chain(&error).contains("`Endpoint` is not set"),
            "{}",
            chain(&error)
        );
    }
}
