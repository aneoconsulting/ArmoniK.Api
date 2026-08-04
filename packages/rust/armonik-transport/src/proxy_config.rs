//! Everything about the HTTP proxy: where to find it, its resolved configuration, and the dedicated
//! credentials in the string form a caller supplies them in.
//!
//! [`HttpProxyConfigArgs`] holds only `username`/`password`: grouped because their names in the
//! environment already share the `Proxy` prefix (`ProxyUsername`, `ProxyPassword`).
//! [`serde_with::with_prefix!`] reproduces that prefix from this struct's own field names composed
//! with `#[serde(flatten)]`, so grouping these two fields changes no environment variable a
//! deployment already sets. The `proxy` URL field itself stays on [`crate::HttpConfigArgs`]
//! directly: a prefix cannot produce an empty suffix, so there is no `Proxy`-prefixed name for it to
//! take.

use hyper::Uri;

#[cfg(feature = "serde")]
use crate::config::{secret_text, text};
use crate::config::{ConfigError, IncompatibleOptionsSnafu};
use crate::secret::Secret;

#[cfg(feature = "serde")]
serde_with::with_prefix!(pub(crate) prefix_proxy "Proxy");

/// Where to find the HTTP proxy used to reach the endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProxySource {
    /// Connect directly, ignoring any proxy configured in the environment.
    ///
    /// The default: a client that asks for nothing connects directly.
    #[default]
    Disabled,
    /// Read the proxy from the environment, on `hyper_util`'s rules: `ALL_PROXY`, `HTTPS_PROXY`,
    /// `HTTP_PROXY` and `NO_PROXY`, in either case, with `NO_PROXY` matched as curl matches it.
    ///
    /// Read once, when `connect` builds the channel, so one that reconnects keeps the values it
    /// started with. Every other option is read in [`crate::HttpConfigArgs::from_env`].
    System,
    /// Use this specific proxy.
    Explicit(Uri),
}

/// Configuration of the HTTP proxy used to reach the endpoint.
///
/// Proxying uses a `CONNECT` tunnel, so TLS, mutual TLS included, is negotiated end to end with the
/// real server and the proxy never sees the plaintext.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct HttpProxyConfig {
    /// Where to find the proxy.
    pub source: ProxySource,
    /// Username for proxy authentication, empty for none.
    pub username: String,
    /// Password for proxy authentication, empty for none.
    pub password: Secret,
}

impl HttpProxyConfig {
    /// Use this specific proxy.
    ///
    /// Credentials written into the URL are taken out of it and kept here, so the URI carries none
    /// wherever it is rendered. The type is `#[non_exhaustive]`, so this is the way in.
    pub fn explicit(uri: Uri) -> Self {
        let (uri, credentials) = crate::proxy::split_credentials(uri);
        let (username, password) = credentials.unwrap_or_default();
        Self {
            source: ProxySource::Explicit(uri),
            username,
            password: password.into(),
        }
    }

    /// Read the proxy from the environment.
    pub fn system() -> Self {
        Self {
            source: ProxySource::System,
            ..Default::default()
        }
    }

    /// Attach credentials for proxy authentication.
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<Secret>,
    ) -> Self {
        self.username = username.into();
        self.password = password.into();
        self
    }

    /// Credentials to present to the proxy, if any were configured.
    pub fn credentials(&self) -> Option<(&str, &str)> {
        if self.username.is_empty() && self.password.is_empty() {
            None
        } else {
            Some((&self.username, self.password.expose_secret()))
        }
    }
}

/// The dedicated proxy credentials, in the string form a caller supplies them in.
///
/// Read from a `Proxy`-prefixed variable or JSON key, e.g. [`Self::username`] is `ProxyUsername`:
/// see the module documentation for why.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct HttpProxyConfigArgs {
    /// Username for proxy authentication. `ProxyUsername`.
    ///
    /// Empty falls back to the username the `proxy` URL carried, if any.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub username: String,
    /// Password for proxy authentication. `ProxyPassword`.
    ///
    /// Empty falls back to the password the `proxy` URL carried, independently of the username, so
    /// setting this one alone still uses that URL's username. Redacted wherever it is written; see
    /// [`Secret`].
    #[cfg_attr(feature = "serde", serde(deserialize_with = "secret_text"))]
    pub password: Secret,
}

impl HttpProxyConfigArgs {
    /// Merges these dedicated credentials into `proxy`, field by field rather than pair by pair: a
    /// dedicated option set alone must not discard the other half of whatever credentials the
    /// `proxy` URL carried. See [`crate::proxy::prefer_dedicated`].
    pub(crate) fn merge_into(self, proxy: HttpProxyConfig) -> HttpProxyConfig {
        let username = crate::proxy::prefer_dedicated(&self.username, &proxy.username).to_owned();
        let password = Secret::from(crate::proxy::prefer_dedicated(
            self.password.expose_secret(),
            proxy.password.expose_secret(),
        ));
        proxy.with_credentials(username, password)
    }
}

/// As ArmoniK's other clients spell it: empty is a direct connection, `none` disables proxying,
/// `system` reads the environment, anything else is a proxy URL, defaulting to the `http` scheme.
pub(crate) fn parse_proxy_source(proxy: &str) -> Result<ProxySource, ConfigError> {
    match proxy {
        "" => Ok(ProxySource::Disabled),
        _ if proxy.eq_ignore_ascii_case("none") => Ok(ProxySource::Disabled),
        _ if proxy.eq_ignore_ascii_case("system") => Ok(ProxySource::System),
        _ => {
            let with_scheme = if proxy.contains("://") {
                proxy.to_owned()
            } else {
                format!("http://{proxy}")
            };
            // Not reported through `UriSnafu`: its message names the endpoint, which would send
            // whoever reads it looking at the wrong option.
            let uri = Uri::try_from(&with_scheme).ok().filter(|uri| {
                uri.authority()
                    .is_some_and(|authority| !authority.host().is_empty())
            });
            match uri {
                // Caught here rather than at connect time, so a mistyped option fails while the
                // configuration is being read and names itself.
                Some(uri) if uri.scheme_str().is_some_and(|scheme| scheme != "http") => {
                    IncompatibleOptionsSnafu {
                        msg: format!(
                            "The `CONNECT` handshake is written in the clear, so only an `http` \
                             proxy can be reached, and `proxy={}` names another scheme",
                            crate::proxy::elide_userinfo(proxy)
                        ),
                    }
                    .fail()
                }
                Some(uri) => Ok(ProxySource::Explicit(uri)),
                None => IncompatibleOptionsSnafu {
                    // Elided: a URL rejected for having no host can still have carried a password.
                    msg: format!(
                        "`proxy={}` is not a valid proxy URL. Expected `none`, \
                         `system`, or a URL such as `http://proxy.example.com:3128`",
                        crate::proxy::elide_userinfo(proxy)
                    ),
                }
                .fail(),
            }
        }
    }
}
