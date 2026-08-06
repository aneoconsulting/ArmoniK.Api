//! Configuration for reaching the endpoint through an HTTP proxy.
//!
//! Proxying uses a `CONNECT` tunnel rather than an absolute-form request, so TLS stays end to end
//! with the real server. These are the types that describe the proxy and the credential handling
//! they guarantee: whatever form credentials arrive in, they end up in dedicated fields, never in
//! a URI that a log line or an error display would render.

use base64::Engine;
use hyper::http::uri::Scheme;
use hyper::http::HeaderValue;
use hyper::Uri;
use secrecy::{ExposeSecret, SecretString};

/// Where to find the HTTP proxy used to reach the endpoint.
#[derive(Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProxySource {
    /// Connect directly.
    ///
    /// The default: a client that asks for nothing connects directly.
    #[default]
    Disabled,
    /// Use this specific proxy.
    Explicit(Uri),
}

impl std::fmt::Debug for ProxySource {
    /// Hand written because the variant is public: a caller can put a credential-bearing URI into
    /// `Explicit` directly, and `Debug` output reaches logs, so the password half of any userinfo
    /// is redacted here rather than trusted to have been stripped.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => f.write_str("Disabled"),
            Self::Explicit(uri) => f
                .debug_tuple("Explicit")
                .field(&format_args!("{}", RedactedUri(uri)))
                .finish(),
        }
    }
}

/// Renders a URI with the password half of any userinfo replaced by `<redacted>`.
///
/// Purely textual, so it cannot fail and holds even for authorities that would not survive a
/// round trip through the URI builder.
struct RedactedUri<'a>(&'a Uri);

impl std::fmt::Display for RedactedUri<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uri = self.0;
        // The last `@`, because a password may contain a literal one.
        let userinfo = uri
            .authority()
            .and_then(|authority| authority.as_str().rsplit_once('@'));
        let Some((userinfo, host)) = userinfo else {
            return write!(f, "{uri}");
        };
        // A userinfo without `:` carries no password, and a username is not a secret.
        let Some((username, _password)) = userinfo.split_once(':') else {
            return write!(f, "{uri}");
        };

        if let Some(scheme) = uri.scheme_str() {
            write!(f, "{scheme}://")?;
        }
        let path = uri.path_and_query().map_or("", |path| path.as_str());
        write!(f, "{username}:<redacted>@{host}{path}")
    }
}

/// Configuration of the HTTP proxy used to reach the endpoint.
///
/// Proxying uses a `CONNECT` tunnel, so TLS, mutual TLS included, is negotiated end to end with the
/// real server and the proxy never sees the plaintext.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ProxyConfig {
    /// Where to find the proxy.
    pub source: ProxySource,
    /// Username for proxy authentication, empty for none.
    pub username: String,
    /// Password for proxy authentication, empty for none. Redacted by `Debug` and zeroized on
    /// drop.
    pub password: SecretString,
}

impl ProxyConfig {
    /// Use this specific proxy.
    ///
    /// Credentials written into the URL are taken out of it and kept in the dedicated fields, so
    /// the URI carries none wherever it is rendered. A URI installed into `source` by hand gets the
    /// same treatment when the connection is made; this constructor only does it earlier, where a
    /// caller can still read the parts back.
    pub fn explicit(uri: Uri) -> Self {
        let (uri, credentials) = split_credentials(uri);
        let (username, password) = credentials.unwrap_or_default();
        Self {
            source: ProxySource::Explicit(uri),
            username,
            password: password.into(),
        }
    }

    /// Connect directly.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Attach credentials for proxy authentication.
    ///
    /// A half left empty keeps what the proxy URL itself carried: setting only the password keeps
    /// the username the URL named, so the pair does not degrade into an unexplained 407. To clear
    /// a credential, assign the field directly.
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<SecretString>,
    ) -> Self {
        let username = username.into();
        if !username.is_empty() {
            self.username = username;
        }
        let password = password.into();
        if !password.expose_secret().is_empty() {
            self.password = password;
        }
        self
    }

    /// The route every connection through this configuration takes.
    ///
    /// Resolved from the fields as they are, not as a constructor left them: an `Explicit` URI is
    /// stripped of any userinfo even when it bypassed [`ProxyConfig::explicit`], so a password
    /// never reaches a log line or an error display through the URI.
    #[allow(dead_code)]
    pub(crate) fn fixed_route(&self) -> RouteResult {
        match &self.source {
            ProxySource::Disabled => Ok(None),
            // Straight through, whatever the target looks like: the caller named this proxy, so
            // failing to reach it has to be an error rather than a quiet direct connection.
            ProxySource::Explicit(uri) => {
                let (uri, credentials) = split_credentials(uri.clone());
                let (username, password) = credentials.unwrap_or_default();
                self.finish_route(uri, username, password)
            }
        }
    }

    /// The scheme rule and the credential merge, shared by every source. `Err` carries the proxy
    /// whose scheme cannot carry the cleartext `CONNECT` handshake.
    fn finish_route(
        &self,
        proxy_uri: Uri,
        url_username: String,
        url_password: String,
    ) -> RouteResult {
        if proxy_uri.scheme() != Some(&Scheme::HTTP) {
            return Err(proxy_uri);
        }

        // A dedicated half wins; the other half falls back to the URL's, so setting only the
        // password keeps the username the URL carried.
        let username = prefer_dedicated(&self.username, &url_username);
        let password = prefer_dedicated(self.password.expose_secret(), &url_password);
        let auth = (!username.is_empty() || !password.is_empty())
            .then(|| basic_auth_header(username, password));

        Ok(Some((proxy_uri, auth)))
    }
}

/// Where a connection goes, and with which ready `Proxy-Authorization` value: `None` is a direct
/// connection. The error carries the proxy whose scheme cannot carry the cleartext handshake, so
/// whoever reports it names the offending URI once.
#[allow(dead_code)]
pub(crate) type RouteResult = Result<Option<(Uri, Option<HeaderValue>)>, Uri>;

/// A whole `Proxy-Authorization` value for the `Basic` scheme, marked sensitive so it is never
/// logged.
///
/// The output of base64 encoding is always valid header-value bytes, so building the `HeaderValue`
/// cannot fail on any input this crate constructs it from.
fn basic_auth_header(username: &str, password: &str) -> HeaderValue {
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    let mut value = HeaderValue::from_str(&format!("Basic {encoded}"))
        .expect("base64 output is always a valid header value");
    value.set_sensitive(true);
    value
}

/// Prefer `dedicated` unless it is empty, in which case fall back to `from_url`.
///
/// A dedicated username or password set alone must not discard the other half of whatever the
/// proxy URL carried, or the request fails as an unexplained 407.
fn prefer_dedicated<'a>(dedicated: &'a str, from_url: &'a str) -> &'a str {
    if dedicated.is_empty() {
        from_url
    } else {
        dedicated
    }
}

/// Split any `user:password@` prefix out of a proxy URI, returning the URI without it.
///
/// Credentials in the URL are how `HTTPS_PROXY` conventionally carries them, so they are honoured. They
/// must not stay in the URI: it is rendered in errors, in log lines, and in `ProxyConfig`'s
/// `Debug`. Percent-escapes are decoded, which is the only way to write a password containing `@`.
pub(crate) fn split_credentials(uri: Uri) -> (Uri, Option<(String, String)>) {
    let Some(authority) = uri.authority() else {
        return (uri, None);
    };
    // The last `@`, so a literal one inside the password does not split in the wrong place.
    let Some((userinfo, host)) = authority.as_str().rsplit_once('@') else {
        return (uri, None);
    };

    let (username, password) = match userinfo.split_once(':') {
        Some((username, password)) => (percent_decode(username), percent_decode(password)),
        None => (percent_decode(userinfo), String::new()),
    };

    let stripped = Uri::builder()
        .scheme(uri.scheme_str().unwrap_or("http"))
        .authority(host)
        .path_and_query(uri.path_and_query().map_or("/", |path| path.as_str()))
        .build();

    match stripped {
        Ok(stripped) if username.is_empty() && password.is_empty() => (stripped, None),
        Ok(stripped) => (stripped, Some((username, password))),
        // Unreachable: the parts come from a URI that already parsed. Keeping the original is the only
        // thing left to do, and dropping the credentials is the safer half of it.
        Err(_) => (uri, None),
    }
}

/// Decode `%XX` escapes, leaving anything malformed exactly as it was written, which is how
/// `percent_encoding` behaves for a `%` not followed by two hex digits.
fn percent_decode(value: &str) -> String {
    percent_encoding::percent_decode_str(value)
        .decode_utf8_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_direct_connection() {
        assert_eq!(ProxySource::default(), ProxySource::Disabled);
        assert_eq!(ProxyConfig::default().source, ProxySource::Disabled);
    }

    #[test]
    fn a_url_without_credentials_is_left_alone() {
        for value in ["http://proxy.corp:3128/", "http://proxy.corp/"] {
            let (uri, credentials) = split_credentials(Uri::try_from(value).expect("a valid URI"));
            assert_eq!(uri.to_string(), value, "{value} should be unchanged");
            assert_eq!(credentials, None);
        }
    }

    #[test]
    fn credentials_are_taken_out_of_the_url() {
        let config = ProxyConfig::explicit(
            Uri::try_from("http://user:secret@proxy.corp:3128").expect("uri"),
        );

        assert_eq!(config.username, "user");
        assert_eq!(config.password.expose_secret(), "secret");
        let ProxySource::Explicit(uri) = &config.source else {
            panic!("expected an explicit proxy");
        };
        assert!(
            !uri.to_string().contains("secret"),
            "the password must not survive in the URI, which gets logged: {uri}"
        );
    }

    #[test]
    fn a_username_without_a_password_is_accepted() {
        let (uri, credentials) =
            split_credentials(Uri::try_from("http://user@proxy.corp:3128").expect("uri"));

        assert_eq!(uri.to_string(), "http://proxy.corp:3128/");
        assert_eq!(credentials, Some((String::from("user"), String::new())));
    }

    #[test]
    fn percent_escapes_in_the_userinfo_are_decoded() {
        // The only way to write a password containing `@` or `:`, so sending it encoded would send the
        // wrong password.
        let (uri, credentials) = split_credentials(
            Uri::try_from("http://a%40b:p%3Ass%40word@proxy.corp:3128").expect("uri"),
        );

        assert_eq!(uri.to_string(), "http://proxy.corp:3128/");
        assert_eq!(
            credentials,
            Some((String::from("a@b"), String::from("p:ss@word")))
        );
    }

    #[test]
    fn a_malformed_escape_is_left_as_written() {
        // Better a password that is wrong in a visible way than one silently mangled.
        for (written, expected) in [("100%", "100%"), ("9%ZZ", "9%ZZ"), ("%2", "%2")] {
            assert_eq!(percent_decode(written), expected, "{written}");
        }
    }

    #[test]
    fn basic_auth_header_encodes_user_and_password() {
        // The canonical example from RFC 7617.
        let value = basic_auth_header("Aladdin", "open sesame");
        assert_eq!(value, "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==");
        assert!(value.is_sensitive(), "must not be logged");
    }

    #[test]
    fn the_password_is_kept_out_of_the_debug_output() {
        let config = ProxyConfig::explicit(Uri::try_from("http://proxy.corp:3128").expect("uri"))
            .with_credentials("user", "s3cr3t");

        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("s3cr3t"),
            "password rendered: {rendered}"
        );
        assert!(
            rendered.contains("user"),
            "the username is not a secret and stays useful: {rendered}"
        );
    }

    #[test]
    fn a_credentialed_uri_installed_by_hand_is_redacted_in_debug_output() {
        // `#[non_exhaustive]` does not prevent building the variant directly, so `Debug` cannot
        // assume the constructor's stripping ran before the value is logged.
        let source = ProxySource::Explicit(
            Uri::try_from("http://user:s3cr3t@proxy.corp:3128").expect("uri"),
        );

        let rendered = format!("{source:?}");
        assert!(
            !rendered.contains("s3cr3t"),
            "password rendered: {rendered}"
        );
        assert!(
            rendered.contains("user") && rendered.contains("proxy.corp:3128"),
            "the username and the host are not secrets and stay useful: {rendered}"
        );
    }

    #[test]
    fn a_uri_without_a_password_is_rendered_verbatim_in_debug_output() {
        for value in ["http://proxy.corp:3128/", "http://user@proxy.corp:3128/"] {
            let source = ProxySource::Explicit(Uri::try_from(value).expect("uri"));
            let rendered = format!("{source:?}");
            assert!(
                rendered.contains(value),
                "{value} carries no password and should be rendered as written: {rendered}"
            );
        }
    }

    #[test]
    fn a_password_set_alone_keeps_the_username_the_url_carried() {
        // Replacing the pair rather than each half would leave an empty username here, and the proxy
        // would answer 407 with nothing to explain it.
        let config = ProxyConfig::explicit(
            Uri::try_from("http://url-user:url-secret@proxy.corp:3128").expect("uri"),
        )
        .with_credentials("", "option-secret");

        assert_eq!(config.username, "url-user");
        assert_eq!(config.password.expose_secret(), "option-secret");
    }

    // --- routing ---

    #[test]
    fn disabled_routes_directly() {
        assert_eq!(ProxyConfig::disabled().fixed_route(), Ok(None));
    }

    #[test]
    fn a_credentialed_uri_installed_by_hand_is_stripped_and_still_authenticates() {
        // `#[non_exhaustive]` does not prevent building the variant directly, so the constructor's
        // stripping cannot be assumed to have run. The route is where the URI is used, so it is
        // where the guarantee has to hold: no password in the URI it returns, and the credentials
        // honoured rather than dropped.
        let config = ProxyConfig {
            source: ProxySource::Explicit(
                Uri::try_from("http://user:s3cr3t@proxy.corp:3128").expect("uri"),
            ),
            ..ProxyConfig::default()
        };

        let (uri, auth) = config
            .fixed_route()
            .expect("routing should succeed")
            .expect("an explicit proxy routes through it");

        assert!(!uri.to_string().contains("s3cr3t"), "leaked: {uri}");
        assert_eq!(auth, Some(basic_auth_header("user", "s3cr3t")));
    }

    #[test]
    fn a_proxy_that_is_not_http_is_refused() {
        // The `CONNECT` handshake goes out unencrypted, so a proxy expecting TLS would see gibberish.
        let uri = Uri::try_from("https://proxy.corp:3128").expect("uri");
        let config = ProxyConfig::explicit(uri.clone());

        assert_eq!(
            config.fixed_route(),
            Err(uri),
            "the route should carry the refused proxy back"
        );
    }

    #[test]
    fn no_credentials_means_no_authorization_header() {
        let config = ProxyConfig::explicit(Uri::try_from("http://proxy.corp:3128").expect("uri"));
        let (_, auth) = config
            .fixed_route()
            .expect("routing should succeed")
            .expect("an explicit proxy routes through it");
        assert_eq!(auth, None);
    }
}
