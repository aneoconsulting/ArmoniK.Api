//! The connection pool a request goes out on, and the options every request on it inherits.
//!
//! An `ak_client` is a `hyper_util` legacy client over the connector `armonik-transport` assembles:
//! the endpoint, TLS, mTLS, the proxy and every socket setting come from there, and none of them is
//! decided again here. What this module adds is the HTTP/2 engine on top of that connector, and the
//! options the connector has no say over.
//!
//! Creation is synchronous and opens nothing. The configuration is read, the certificates it names
//! are loaded, and the connector is assembled; the first socket is opened by the first request. A
//! host application may therefore create a client from a UI thread, and a mistyped option is
//! reported there rather than surfacing much later as a failed request.

use std::sync::OnceLock;
use std::time::Duration;

use armonik_transport::reexports::http::{HeaderValue, Uri};
use armonik_transport::reexports::http_body_util::combinators::BoxBody;
use armonik_transport::reexports::hyper_util::client::legacy::Client;
use armonik_transport::reexports::hyper_util::rt::{TokioExecutor, TokioTimer};
use armonik_transport::{https_connector, Connector, HttpConfig};
use bytes::Bytes;

use crate::error::{ak_bytes, FfiError};
use crate::handle::Registry;
use crate::rate_limit::RateLimiter;
use crate::status::ak_status;

/// How a request body reports that it cannot produce the rest of itself.
///
/// The boxed trait object rather than a type of this crate's own: `hyper` requires only that a body
/// error converts into one, and nothing here inspects an error it did not raise.
pub(crate) type BodyError = Box<dyn std::error::Error + Send + Sync>;

/// The body every request on a client is sent with.
///
/// Erased, because a client is built long before any request exists and `hyper` fixes one body type
/// per client: the same pool carries a request with no payload at all and one that streams for
/// minutes, and those are not the same type. The cost is one allocation per request body, against a
/// pool whose type would otherwise be decided by whichever request shape was written first.
pub(crate) type RequestBody = BoxBody<Bytes, BodyError>;

/// A connection pool, and the options a request on it draws from.
///
/// Handed to the caller as an opaque pointer. The registry owns it, and a request takes a counted
/// reference for as long as it runs, so a pool outlives an [`ak_client_release`] that lands while
/// work is still on it.
///
/// The last three fields are read by whoever sends a request, and nothing here does: they are
/// parsed and held, which changes no behaviour on its own. `tests/schema.rs` says as much, and lists
/// them among the options this library does not apply.
// `dead_code` measures reachability from this crate's Rust API, which is not the surface this crate
// offers.
#[allow(dead_code)]
pub struct ak_client {
    /// The pool, over the connector the configuration assembles.
    pub(crate) pool: Client<Connector, RequestBody>,
    /// The URI a request is addressed to: the endpoint, or the name `OverrideTargetName` moves it
    /// to. Resolved once here, because it is the same for every request on the pool.
    pub(crate) origin: Uri,
    /// `Timeout`: the whole-request bound, `None` for none.
    ///
    /// Held rather than set on the pool, because `hyper` has no notion of a request taking too long:
    /// nothing below the sender of a request can time one out.
    pub(crate) timeout: Option<Duration>,
    /// `UserAgent`: the header value a request carries when it has none of its own, `None` for none.
    pub(crate) user_agent: Option<HeaderValue>,
    /// `RateLimit`: how many requests a window admits, `None` for no limit.
    ///
    /// State the pool carries rather than a layer wrapped around it, because a permit belongs to one
    /// request: it is taken by the sender, and one limiter serves every request on the client, which
    /// is what "per client" means.
    pub(crate) rate_limit: Option<RateLimiter>,
}

/// The live clients, by the address the caller holds.
fn live() -> &'static Registry<ak_client> {
    static LIVE: OnceLock<Registry<ak_client>> = OnceLock::new();
    LIVE.get_or_init(Registry::new)
}

/// Create a client from a UTF-8 JSON configuration document.
///
/// `config_json` names the flat options of the transport's vocabulary - `Endpoint`, `CaCertPath`,
/// `AllowUnsafeConnection`, `Tcp*`, `Http2*`, `Proxy*`, and the rest - as a single JSON object of
/// strings. `include/http_config.schema.json` is that vocabulary in full; an option the document
/// does not name reads as its own default, so a caller writes only what it changes.
///
/// Synchronous, and it opens no connection: this reads the options, loads whatever certificates they
/// name, and assembles the connector. A failure here is therefore a configuration failure, reported
/// straight away with its whole cause chain flattened into `out_err`.
///
/// # Safety
///
/// `config_json` must point to `len` readable bytes. `out` must be a writable `ak_client*`, and
/// receives a handle to be given up by exactly one [`ak_client_release`]. `out_err`, when non-null,
/// must be a writable [`ak_bytes`] and receives a message to give up with
/// [`crate::ak_bytes_release`].
#[no_mangle]
pub unsafe extern "C" fn ak_client_create(
    config_json: *const u8,
    len: usize,
    out: *mut *mut ak_client,
    out_err: *mut ak_bytes,
) -> i32 {
    crate::guard::catch_unwind_status(out_err, || {
        if out.is_null() {
            return FfiError::NullArgument("out").into_ffi_result(out_err);
        }
        // SAFETY: documented as writable by this function's contract. Cleared first, so a caller
        // that only checks the handle cannot mistake an uninitialised slot for a client.
        unsafe { *out = std::ptr::null_mut() };
        if config_json.is_null() {
            return FfiError::NullArgument("config_json").into_ffi_result(out_err);
        }

        // SAFETY: forwarded from this function's contract.
        let bytes = unsafe { std::slice::from_raw_parts(config_json, len) };
        let client = match build(bytes) {
            Ok(client) => client,
            Err(error) => return error.into_ffi_result(out_err),
        };

        // SAFETY: checked non-null above.
        unsafe { *out = live().insert(client).cast_mut() };
        ak_status::AK_OK.code()
    })
}

/// Read `config_json` and assemble the pool it describes.
fn build(config_json: &[u8]) -> Result<ak_client, FfiError> {
    let text = std::str::from_utf8(config_json).map_err(|_| FfiError::InvalidUtf8)?;
    let config: HttpConfig =
        serde_json::from_str(text).map_err(|error| FfiError::InvalidJson(error.to_string()))?;

    // Before the connector, which needs it: an endpoint that addresses nothing is refused here, and
    // an `OverrideTargetName` is resolved against the endpoint exactly once.
    let origin = Uri::try_from(&config)?;
    let timeout = config.timeout;
    let user_agent = config.user_agent.clone();
    let http2 = config.http2;
    let pool_idle_timeout = config.pool_idle_timeout;
    // The reader refuses a zero count and a zero window, so the limiter's own assertions are not
    // reachable through a configuration document.
    let rate_limit = config.rate_limit.map(RateLimiter::new);

    let connector = https_connector(config, origin.clone())?;

    let mut builder = Client::builder(TokioExecutor::new());
    // `http2_only` because gRPC is HTTP/2 and nothing else. An h2c endpoint offers no ALPN to
    // negotiate over, so without this the pool speaks HTTP/1.1 to it and the server closes; over
    // TLS it opens an HTTP/1.1 connection, learns from ALPN that the peer wants h2, and opens a
    // second one, which doubles every cold start.
    //
    // Both timers, because they are separate settings and neither has a default: the HTTP/2
    // keepalive panics for want of one the moment a connection is established, and the pool's idle
    // sweep needs the other. This is not a nicety.
    builder
        .http2_only(true)
        .timer(TokioTimer::new())
        .pool_timer(TokioTimer::new());

    // Each option is set only when the document names it, so an unset one leaves whatever `hyper`
    // defaults to rather than being written over with a `None` this crate invented.
    if let Some(interval) = http2.keep_alive_interval {
        builder.http2_keep_alive_interval(interval);
    }
    if let Some(timeout) = http2.keep_alive_timeout {
        builder.http2_keep_alive_timeout(timeout);
    }
    // Not optional, so no branch: `false` is what both the option and `hyper` default to, and
    // setting it either way says the same thing.
    builder.http2_keep_alive_while_idle(http2.keep_alive_while_idle);
    if let Some(max) = http2.max_header_list_size {
        builder.http2_max_header_list_size(max);
    }
    // `PoolIdleTimeout` exists in the vocabulary for a consumer that pools, which is what this is:
    // a channel is one connection and has none, so nothing that builds one reads it.
    if let Some(idle) = pool_idle_timeout {
        builder.pool_idle_timeout(idle);
    }

    Ok(ak_client {
        pool: builder.build(connector),
        origin,
        timeout,
        user_agent,
        rate_limit,
    })
}

/// Give up the caller's reference to a client.
///
/// Requests already under way keep the pool alive and run to completion: this gives back one
/// reference, not necessarily the last one. Null is accepted and does nothing.
///
/// # Safety
///
/// `client` must be a handle from [`ak_client_create`] that has not been released, or null.
#[no_mangle]
pub unsafe extern "C" fn ak_client_release(client: *mut ak_client) {
    crate::guard::catch_unwind_void(|| {
        if client.is_null() {
            return;
        }
        drop(live().remove(client));
    });
}

/// A counted reference to a live client, or `None` when that address is not one.
#[cfg(test)]
fn get(client: *const ak_client) -> Option<std::sync::Arc<ak_client>> {
    live().get(client)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A directory of generated PEM files, removed when the test that made it ends.
    struct FixtureDir(PathBuf);

    impl FixtureDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("armonik-ffi-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create the fixture directory");
            Self(path)
        }

        fn write(&self, name: &str, content: impl AsRef<[u8]>) -> String {
            let path = self.0.join(name);
            std::fs::write(&path, content).expect("write the fixture file");
            path.display().to_string().replace('\\', "/")
        }
    }

    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// What a caller sees from one `ak_client_create`: the status, the message, and the handle.
    struct Created {
        status: i32,
        message: String,
        handle: *mut ak_client,
    }

    impl Drop for Created {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                // SAFETY: produced by `ak_client_create` and released exactly once.
                unsafe { ak_client_release(self.handle) };
            }
        }
    }

    /// Create a client the way a caller does, reading back whatever it was told.
    fn create(config: &str) -> Created {
        let mut handle: *mut ak_client = std::ptr::null_mut();
        let mut err = ak_bytes::EMPTY;
        // SAFETY: both out-parameters are live locals, and the document outlives the call.
        let status = unsafe {
            ak_client_create(
                config.as_ptr(),
                config.len(),
                std::ptr::addr_of_mut!(handle),
                std::ptr::addr_of_mut!(err),
            )
        };
        let message = if err.ptr.is_null() {
            String::new()
        } else {
            // SAFETY: written by the call above, and released here exactly once.
            let seen = unsafe { std::slice::from_raw_parts(err.ptr, err.len) }.to_vec();
            unsafe { crate::ak_bytes_release(err) };
            String::from_utf8_lossy(&seen).into_owned()
        };
        Created {
            status,
            message,
            handle,
        }
    }

    // Every test that reaches `build` is out of Miri's reach: verifying against the system CAs reads
    // the platform's certificate store, and the crypto provider runs assembly Miri cannot interpret.

    #[test]
    #[cfg_attr(miri, ignore)]
    fn a_valid_configuration_produces_a_client_without_connecting() {
        // Nothing listens on this port, and creation still succeeds: no socket is opened here, which
        // is the whole reason a host application may call this from a thread it cannot block.
        let created = create(r#"{"Endpoint": "http://127.0.0.1:1/"}"#);

        assert_eq!(
            created.status,
            ak_status::AK_OK.code(),
            "{}",
            created.message
        );
        assert!(!created.handle.is_null());
        assert!(get(created.handle).is_some(), "the handle is live");
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn the_origin_a_request_is_addressed_to_follows_the_override() {
        // `OverrideTargetName` moves the name the certificate is verified against and the authority
        // a request carries, and leaves the address dialled alone.
        let created = create(
            r#"{"Endpoint": "https://127.0.0.1:5001/", "OverrideTargetName": "server.example.com"}"#,
        );
        assert_eq!(
            created.status,
            ak_status::AK_OK.code(),
            "{}",
            created.message
        );

        let client = get(created.handle).expect("a live client");
        assert_eq!(client.origin.host(), Some("server.example.com"));
    }

    /// Every option this pool carries onto the `hyper` builder, each with a value nothing defaults
    /// to, so an option that is dropped on the way cannot pass for one that was left alone.
    const EVERY_POOL_OPTION: &str = r#"{
        "Endpoint": "http://127.0.0.1:1/",
        "Http2KeepAliveInterval": "20s",
        "Http2KeepAliveTimeout": "10s",
        "Http2KeepAliveWhileIdle": "true",
        "Http2MaxHeaderListSize": "16384",
        "PoolIdleTimeout": "90s"
    }"#;

    #[test]
    fn the_pool_options_are_read_under_the_names_this_abi_documents() {
        // The names in a caller's document are part of this ABI, and the values behind them belong
        // to the transport's vocabulary. `hyper` offers no way to read a setting back off a built
        // client, so what a test can hold is the step before it: that these spellings really are
        // the ones the configuration reads, and that they arrive as the values written here.
        let config: HttpConfig =
            serde_json::from_str(EVERY_POOL_OPTION).expect("the option names the ABI documents");

        assert_eq!(
            config.http2.keep_alive_interval,
            Some(Duration::from_secs(20))
        );
        assert_eq!(
            config.http2.keep_alive_timeout,
            Some(Duration::from_secs(10))
        );
        assert!(config.http2.keep_alive_while_idle);
        assert_eq!(config.http2.max_header_list_size, Some(16384));
        assert_eq!(config.pool_idle_timeout, Some(Duration::from_secs(90)));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn a_document_naming_every_pool_option_produces_a_client() {
        let created = create(EVERY_POOL_OPTION);

        assert_eq!(
            created.status,
            ak_status::AK_OK.code(),
            "{}",
            created.message
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn a_rate_limit_becomes_a_limiter_the_client_holds() {
        let limited = create(r#"{"Endpoint": "http://127.0.0.1:1/", "RateLimit": "100/1s"}"#);
        assert_eq!(
            limited.status,
            ak_status::AK_OK.code(),
            "{}",
            limited.message
        );
        assert!(
            get(limited.handle)
                .expect("a live client")
                .rate_limit
                .is_some(),
            "the option becomes a limiter on the client rather than being read and dropped"
        );

        let unlimited = create(r#"{"Endpoint": "http://127.0.0.1:1/"}"#);
        assert!(
            get(unlimited.handle)
                .expect("a live client")
                .rate_limit
                .is_none(),
            "an absent option costs a request nothing"
        );
    }

    #[test]
    fn a_rate_limit_that_admits_nothing_is_refused_at_creation() {
        // The one shape the limiter itself asserts against. Refusing it while the option is read is
        // what keeps that assertion out of reach of any document a caller can write.
        for spelling in ["0/1s", "10/0s", "nonsense", "100"] {
            let created = create(&format!(
                r#"{{"Endpoint": "http://127.0.0.1:1/", "RateLimit": "{spelling}"}}"#
            ));

            assert_eq!(
                created.status,
                ak_status::AK_INVALID_CONFIG.code(),
                "`RateLimit={spelling}` was accepted"
            );
            assert!(
                created.message.contains("RateLimit") || created.message.contains("Rate limit"),
                "the option at fault has to be named: {}",
                created.message
            );
        }
    }

    #[test]
    fn a_document_that_is_not_json_is_refused_with_a_message() {
        let created = create("not json at all");

        assert_eq!(created.status, ak_status::AK_INVALID_CONFIG.code());
        assert!(created.handle.is_null(), "no handle on a failure");
        assert!(
            !created.message.is_empty(),
            "the caller has nothing else to go on"
        );
    }

    #[test]
    fn a_configuration_that_is_not_utf8_is_refused_before_it_is_parsed() {
        let mut handle: *mut ak_client = std::ptr::null_mut();
        let invalid = [0x7bu8, 0xff];
        // SAFETY: `out` is a live local, and the array outlives the call.
        let status = unsafe {
            ak_client_create(
                invalid.as_ptr(),
                invalid.len(),
                std::ptr::addr_of_mut!(handle),
                std::ptr::null_mut(),
            )
        };

        assert_eq!(status, ak_status::AK_INVALID_UTF8.code());
        assert!(handle.is_null());
    }

    #[test]
    fn an_unset_endpoint_is_refused_and_the_message_names_the_option() {
        let created = create("{}");

        assert_eq!(created.status, ak_status::AK_INVALID_CONFIG.code());
        assert!(
            created.message.contains("`Endpoint` is not set"),
            "the option at fault has to survive the flattening: {}",
            created.message
        );
    }

    #[test]
    fn an_endpoint_that_is_not_a_uri_is_refused_and_quoted_back() {
        let created = create(r#"{"Endpoint": "not a uri"}"#);

        assert_eq!(created.status, ak_status::AK_INVALID_CONFIG.code());
        assert!(
            created.message.contains("not a uri"),
            "the value at fault is what the caller has to correct: {}",
            created.message
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn an_unreadable_certificate_is_reported_with_its_cause_rather_than_a_summary() {
        let created =
            create(r#"{"Endpoint": "https://localhost:443/", "CaCertPath": "no/such/file.pem"}"#);

        assert_eq!(created.status, ak_status::AK_INVALID_CONFIG.code());
        assert!(
            created.message.contains("no/such/file.pem"),
            "a message that stopped at the outer error would not say which file: {}",
            created.message
        );
    }

    /// A configuration whose certificate and key belong to different identities.
    ///
    /// `rustls` refuses that pair while the client configuration is assembled, which makes it the
    /// one TLS failure reachable before a socket is opened.
    fn mismatched_identity(dir: &FixtureDir) -> String {
        let identity = rcgen::generate_simple_self_signed(["test".to_owned()])
            .expect("a self-signed certificate");
        let stranger = rcgen::KeyPair::generate().expect("a second key");
        let cert = dir.write("cert.pem", identity.cert.pem());
        let key = dir.write("key.pem", stranger.serialize_pem());

        format!(
            r#"{{"Endpoint": "https://localhost:443/", "CertPem": "{cert}", "KeyPem": "{key}"}}"#
        )
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn a_tls_failure_crosses_flattened_rather_than_as_its_outer_sentence() {
        // What the flattening is for: the outer error says a TLS connection could not be
        // established, and only its cause says why.
        let dir = FixtureDir::new("mismatched-identity");
        let created = create(&mismatched_identity(&dir));

        assert_eq!(created.status, ak_status::AK_INVALID_CONFIG.code());
        assert!(
            created
                .message
                .starts_with("Could not establish TLS connection to the remote"),
            "{}",
            created.message
        );
        assert!(
            created.message.contains("keys may not be consistent"),
            "the outer sentence alone says nothing about what to fix, so the cause has to survive \
             the flattening: {}",
            created.message
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn no_failure_here_is_reported_as_a_connection_failure() {
        // The invariant behind the code each test above asserts, stated once so that a path added
        // later cannot quietly break it. This entry point opens no connection, so it has none to
        // report failing: `AK_CONNECTION_FAILED` would send its reader to check network
        // reachability over a mistake in a document or a file.
        let dir = FixtureDir::new("no-connection-failure");
        for document in [
            String::from("not json at all"),
            String::from("{}"),
            String::from(r#"{"Endpoint": "not a uri"}"#),
            String::from(r#"{"Endpoint": "http://127.0.0.1:1/", "RateLimit": "0/1s"}"#),
            String::from(
                r#"{"Endpoint": "https://localhost:443/", "CaCertPath": "no/such/file.pem"}"#,
            ),
            mismatched_identity(&dir),
        ] {
            let created = create(&document);

            assert_ne!(
                created.status,
                ak_status::AK_CONNECTION_FAILED.code(),
                "`{document}` was refused as a connection failure: {}",
                created.message
            );
            assert_eq!(
                created.status,
                ak_status::AK_INVALID_CONFIG.code(),
                "`{document}`: {}",
                created.message
            );
        }
    }

    #[test]
    fn a_null_configuration_is_refused_rather_than_dereferenced() {
        let mut handle: *mut ak_client = std::ptr::null_mut();
        // SAFETY: `out` is a live local; the null `config_json` is the case under test.
        let status = unsafe {
            ak_client_create(
                std::ptr::null(),
                0,
                std::ptr::addr_of_mut!(handle),
                std::ptr::null_mut(),
            )
        };

        assert_eq!(status, ak_status::AK_NULL_ARGUMENT.code());
        assert!(handle.is_null());
    }

    #[test]
    fn a_null_out_parameter_is_refused_rather_than_written_through() {
        let config = "{}";
        // SAFETY: the null `out` is the case under test.
        let status = unsafe {
            ak_client_create(
                config.as_ptr(),
                config.len(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        assert_eq!(status, ak_status::AK_NULL_ARGUMENT.code());
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn releasing_a_client_twice_is_caught_rather_than_a_double_free() {
        let mut created = create(r#"{"Endpoint": "http://127.0.0.1:1/"}"#);
        assert_eq!(
            created.status,
            ak_status::AK_OK.code(),
            "{}",
            created.message
        );
        let handle = std::mem::replace(&mut created.handle, std::ptr::null_mut());

        // SAFETY: the first gives up the registry's reference; the second has to be caught by the
        // registry rather than reaching the allocation again.
        unsafe {
            ak_client_release(handle);
            ak_client_release(handle);
        }
        assert!(get(handle).is_none());
    }

    #[test]
    fn releasing_a_null_client_does_nothing() {
        // SAFETY: null is documented as accepted.
        unsafe { ak_client_release(std::ptr::null_mut()) };
    }
}
