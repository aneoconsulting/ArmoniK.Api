//! Establishing the TCP connection under the tunnel and the TLS layer.
//!
//! [`TcpConnector::Standard`] is hyper's own, taken whenever port reuse is off, so the default path
//! stays the one this crate has always used. [`TcpConnector::ReusePorts`] owns socket creation, which
//! is the only way to set `SO_REUSE_UNICASTPORT` before connecting, and is reached only when the
//! option is set: the newer code stays on the opt-in path.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use hyper::http::uri::Scheme;
use hyper::Uri;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioIo;
use tokio::net::{TcpSocket, TcpStream};
use tower_service::Service;

use super::ClientConfig;

/// Boxed error, matching what the connectors below and hyper itself produce.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// How long to wait for one address before also trying the next, matching `hyper_util`'s default.
///
/// The standard connector races addresses this way for free. Without it, one address that accepts the
/// `SYN` and never answers costs the whole `connect_timeout` before the next is tried, which on a
/// dual-stack host with a blackholed record turns a 300ms connection into a minute.
const FALLBACK_DELAY: Duration = Duration::from_millis(300);

/// Try `addresses` in order, starting the next one `fallback` after the last without cancelling it.
///
/// The first success wins; the last error is reported if none succeeds. Attempts already in flight
/// keep running, which is the point: a slow address that would have answered is not abandoned, it is
/// merely overtaken.
///
/// Generic over how an address is reached so that the scheduling can be tested on its own. The real
/// thing is `connect_to`, and there is no way to make a socket blackhole a `SYN` from a test.
async fn race<T, A, F>(
    addresses: Vec<std::net::SocketAddr>,
    fallback: Duration,
    attempt: A,
) -> Result<T, BoxError>
where
    A: Fn(std::net::SocketAddr) -> F,
    F: Future<Output = Result<T, BoxError>>,
{
    use futures_util::stream::{FuturesUnordered, StreamExt};

    let mut remaining = addresses.into_iter();
    let Some(first) = remaining.next() else {
        return Err(BoxError::from("no address to connect to"));
    };

    let in_flight = FuturesUnordered::new();
    in_flight.push(attempt(first));
    let mut in_flight = in_flight;
    let mut last_error = None;

    loop {
        // Only armed while an address is left to start, so a settled race is not kept awake by it.
        let next_up = remaining.len() > 0;

        tokio::select! {
            biased;

            finished = in_flight.next(), if !in_flight.is_empty() => match finished {
                Some(Ok(value)) => return Ok(value),
                Some(Err(error)) => {
                    tracing::debug!(%error, "Connection attempt failed");
                    last_error = Some(error);
                    // Do not wait out the fallback when an attempt has already failed.
                    if let Some(address) = remaining.next() {
                        in_flight.push(attempt(address));
                    }
                }
                // `FuturesUnordered` is empty, which the guard above rules out.
                None => unreachable!("polled an empty set of attempts"),
            },

            _ = tokio::time::sleep(fallback), if next_up => {
                if let Some(address) = remaining.next() {
                    in_flight.push(attempt(address));
                }
            }
        }

        if in_flight.is_empty() && remaining.len() == 0 {
            return Err(last_error
                .unwrap_or_else(|| BoxError::from("every address refused the connection")));
        }
    }
}

/// Order the resolved addresses so the families alternate, keeping the resolver's preference first.
///
/// Racing them as they come would reach the second family only after every address of the first: three
/// records as `[v6, v6, v4]` put the IPv4 attempt two fallback delays in, long enough for a
/// `connect_timeout` shorter than that to expire while a reachable address was never tried.
/// `hyper_util` splits the two families and races the halves; interleaving is what RFC 8305 asks for
/// and gives the same first-alternate-at-one-delay guarantee.
fn interleave_families(addresses: Vec<std::net::SocketAddr>) -> Vec<std::net::SocketAddr> {
    let total = addresses.len();
    let preferred_is_ipv6 = matches!(addresses.first(), Some(address) if address.is_ipv6());
    let (mut preferred, mut other): (Vec<_>, Vec<_>) = addresses
        .into_iter()
        .partition(|address| address.is_ipv6() == preferred_is_ipv6);

    // Reversed so that popping walks each family in the order it was resolved in.
    preferred.reverse();
    other.reverse();

    let mut interleaved = Vec::with_capacity(total);
    while interleaved.len() < total {
        interleaved.extend(preferred.pop());
        interleaved.extend(other.pop());
    }
    interleaved
}

/// Await `body`, bounded by an absolute `deadline` when one is set.
async fn with_optional_deadline<T>(
    deadline: Option<tokio::time::Instant>,
    body: impl std::future::Future<Output = T>,
) -> Result<T, tokio::time::error::Elapsed> {
    match deadline {
        Some(deadline) => tokio::time::timeout_at(deadline, body).await,
        None => Ok(body.await),
    }
}

/// How the underlying TCP connection is opened.
#[derive(Debug, Clone)]
pub enum TcpConnector {
    /// hyper's connector. The default, and unchanged.
    Standard(HttpConnector),
    /// Our own connector, which can defer ephemeral port allocation.
    ReusePorts(ReusePortsConnector),
}

impl TcpConnector {
    /// Pick the connector the configuration calls for.
    pub(crate) fn new(config: &ClientConfig) -> Self {
        let mut http = HttpConnector::new();
        http.enforce_http(false); // required for hyper-rustls to switch schemes
        http.set_nodelay(!config.tcp_nagle_algorithm);
        http.set_keepalive(config.tcp_keepalive);
        http.set_keepalive_interval(config.tcp_keepalive_interval);
        http.set_keepalive_retries(config.tcp_keepalive_retries);
        if let Some(timeout) = config.connect_timeout {
            http.set_connect_timeout(Some(timeout));
        }

        // Also standard where the option does not exist: the connector below is only worth taking for
        // the socket option it can set, so elsewhere `reuse_ports` really is the no-op it documents,
        // rather than a different connector with the same effect.
        if !config.reuse_ports || !cfg!(windows) {
            return Self::Standard(http);
        }

        Self::ReusePorts(ReusePortsConnector {
            nodelay: !config.tcp_nagle_algorithm,
            keepalive: config.tcp_keepalive,
            keepalive_interval: config.tcp_keepalive_interval,
            keepalive_retries: config.tcp_keepalive_retries,
            connect_timeout: config.connect_timeout,
        })
    }
}

impl Service<Uri> for TcpConnector {
    type Response = TokioIo<TcpStream>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self {
            Self::Standard(inner) => inner.poll_ready(cx).map_err(Into::into),
            Self::ReusePorts(inner) => inner.poll_ready(cx),
        }
    }

    fn call(&mut self, target: Uri) -> Self::Future {
        match self {
            Self::Standard(inner) => {
                let future = inner.call(target);
                Box::pin(async move { future.await.map_err(Into::into) })
            }
            Self::ReusePorts(inner) => inner.call(target),
        }
    }
}

/// A TCP connector that asks the OS to defer ephemeral port allocation.
///
/// Opening many short-lived connections on Windows exhausts the ephemeral port range;
/// `SO_REUSE_UNICASTPORT` lets outbound connections share a local port when their remote endpoints
/// differ. It exists from Windows 10 and Server 2016 on, and this transport targets older machines,
/// so a rejection is logged and ignored rather than failing the connection.
#[derive(Debug, Clone)]
pub struct ReusePortsConnector {
    nodelay: bool,
    keepalive: Option<Duration>,
    keepalive_interval: Option<Duration>,
    keepalive_retries: Option<u32>,
    connect_timeout: Option<Duration>,
}

impl Service<Uri> for ReusePortsConnector {
    type Response = TokioIo<TcpStream>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, target: Uri) -> Self::Future {
        let connector = self.clone();
        Box::pin(async move { connector.connect(target).await })
    }
}

impl ReusePortsConnector {
    async fn connect(&self, target: Uri) -> Result<TokioIo<TcpStream>, BoxError> {
        let host = target
            .host()
            .ok_or_else(|| BoxError::from(format!("`{target}` has no host to connect to")))?
            // An IPv6 literal is bracketed in a URI but must not be when resolving.
            .trim_matches(['[', ']']);
        let port = target.port_u16().unwrap_or_else(|| {
            if target.scheme() == Some(&Scheme::HTTPS) {
                443
            } else {
                80
            }
        });

        // Anchored before resolution, so name lookup and every connection attempt draw from the one
        // budget the caller asked for. Bounding each attempt separately instead would let a host
        // with several addresses take a multiple of `connect_timeout` to fail.
        let deadline = self
            .connect_timeout
            .map(|timeout| tokio::time::Instant::now() + timeout);

        let addresses = with_optional_deadline(deadline, tokio::net::lookup_host((host, port)))
            .await
            .map_err(|_| BoxError::from(format!("resolving `{host}` timed out")))?
            .map_err(|source| BoxError::from(format!("could not resolve `{host}`: {source}")))?
            .collect::<Vec<_>>();

        if addresses.is_empty() {
            return Err(BoxError::from(format!("`{host}` resolved to no address")));
        }

        let attempts = race(interleave_families(addresses), FALLBACK_DELAY, |address| {
            self.connect_to(address)
        });

        match with_optional_deadline(deadline, attempts).await {
            Ok(Ok(stream)) => Ok(TokioIo::new(stream)),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(BoxError::from(format!("connecting to `{host}` timed out"))),
        }
    }

    async fn connect_to(&self, address: std::net::SocketAddr) -> Result<TcpStream, BoxError> {
        let socket = if address.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }?;

        // Must happen before `connect`: the option governs how the local port is picked.
        if let Err(error) = enable_port_reuse(&socket) {
            tracing::debug!(
                %error,
                "The OS refused SO_REUSE_UNICASTPORT; connecting without deferred port allocation"
            );
        }

        socket.set_nodelay(self.nodelay)?;
        socket.set_keepalive(self.keepalive.is_some())?;

        // No timeout here: `connect` above bounds the whole operation against one deadline, and a
        // second per-attempt timeout would be the very thing that let the total overrun it.
        let stream = socket.connect(address).await?;

        self.apply_keepalive(&stream)?;

        Ok(stream)
    }

    /// Apply the fine-grained keepalive settings, so turning port reuse on does not quietly drop
    /// them compared with the standard connector.
    fn apply_keepalive(&self, stream: &TcpStream) -> Result<(), BoxError> {
        let Some(time) = self.keepalive else {
            return Ok(());
        };

        let mut keepalive = socket2::TcpKeepalive::new().with_time(time);
        if let Some(interval) = self.keepalive_interval {
            keepalive = keepalive.with_interval(interval);
        }
        if let Some(retries) = self.keepalive_retries {
            keepalive = with_retries(keepalive, retries);
        }

        socket2::SockRef::from(stream).set_tcp_keepalive(&keepalive)?;
        Ok(())
    }
}

/// Set the keepalive retry count where the platform allows it.
///
/// Mirrors what hyper's own connector does, so turning port reuse on does not change which settings
/// take effect: Windows and Apple platforms do not expose this knob, and both connectors ignore it
/// there rather than failing.
#[cfg(not(any(target_os = "windows", target_vendor = "apple")))]
fn with_retries(keepalive: socket2::TcpKeepalive, retries: u32) -> socket2::TcpKeepalive {
    keepalive.with_retries(retries)
}

#[cfg(any(target_os = "windows", target_vendor = "apple"))]
fn with_retries(keepalive: socket2::TcpKeepalive, _retries: u32) -> socket2::TcpKeepalive {
    keepalive
}

/// Ask the OS to defer ephemeral port allocation for this socket.
///
/// Only Windows has such an option; elsewhere this is a no-op and `reuse_ports` has no effect, which
/// is what the option documents.
#[cfg(windows)]
fn enable_port_reuse(socket: &TcpSocket) -> std::io::Result<()> {
    use std::os::windows::io::AsRawSocket;

    use windows_sys::Win32::Networking::WinSock::{setsockopt, SOL_SOCKET, SO_REUSE_UNICASTPORT};

    let enabled: u32 = 1;

    // SAFETY: the borrow keeps the handle valid for the call, and the pointer and length describe the
    // 4-byte DWORD this option expects. The signature comes from `windows-sys` rather than being
    // written here, so it cannot disagree with the one `ws2_32` actually exports.
    let result = unsafe {
        setsockopt(
            socket.as_raw_socket() as _,
            SOL_SOCKET,
            SO_REUSE_UNICASTPORT,
            std::ptr::addr_of!(enabled).cast(),
            std::mem::size_of_val(&enabled) as i32,
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(windows))]
fn enable_port_reuse(_socket: &TcpSocket) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(reuse_ports: bool) -> ClientConfig {
        ClientConfig {
            reuse_ports,
            ..ClientConfig::default()
        }
    }

    #[test]
    fn the_standard_connector_is_used_unless_port_reuse_is_asked_for() {
        assert!(matches!(
            TcpConnector::new(&config(false)),
            TcpConnector::Standard(_)
        ));

        let asked_for = TcpConnector::new(&config(true));
        if cfg!(windows) {
            assert!(matches!(asked_for, TcpConnector::ReusePorts(_)));
        } else {
            // Nothing to gain from it here, so the option changes no code path at all.
            assert!(matches!(asked_for, TcpConnector::Standard(_)));
        }
    }

    #[test]
    #[cfg(windows)]
    fn the_reuse_connector_carries_the_tcp_settings_over() {
        let mut config = config(true);
        config.tcp_nagle_algorithm = true;
        config.tcp_keepalive = Some(Duration::from_secs(30));
        config.tcp_keepalive_interval = Some(Duration::from_secs(5));
        config.tcp_keepalive_retries = Some(3);
        config.connect_timeout = Some(Duration::from_secs(7));

        let TcpConnector::ReusePorts(connector) = TcpConnector::new(&config) else {
            panic!("expected the port-reuse connector");
        };

        // Turning port reuse on must not silently drop settings the standard connector honours.
        assert!(!connector.nodelay, "Nagle enabled means nodelay off");
        assert_eq!(connector.keepalive, Some(Duration::from_secs(30)));
        assert_eq!(connector.keepalive_interval, Some(Duration::from_secs(5)));
        assert_eq!(connector.keepalive_retries, Some(3));
        assert_eq!(connector.connect_timeout, Some(Duration::from_secs(7)));
    }

    /// Winsock must accept `SO_REUSE_UNICASTPORT` on a fresh socket.
    ///
    /// The option cannot be read back: `getsockopt` answers 0 whatever was set, so accepting the call
    /// is all there is to observe. The companion test below is what makes that mean something.
    #[cfg(windows)]
    #[test]
    fn winsock_accepts_the_port_reuse_option() {
        let socket = TcpSocket::new_v4().expect("socket");

        enable_port_reuse(&socket)
            .expect("Windows 10 and Server 2016 onwards should accept SO_REUSE_UNICASTPORT");
    }

    /// The control for the test above: an option number Winsock does not know must be rejected.
    ///
    /// Without this, a wrong constant or a mismatched calling convention would look exactly like
    /// success, and port reuse would silently never happen.
    #[cfg(windows)]
    #[test]
    fn winsock_rejects_an_unknown_option() {
        use std::os::windows::io::{AsRawSocket, RawSocket};

        const SOL_SOCKET: i32 = 0xffff;
        /// Deliberately not a real option.
        const NONSENSE_OPTION: i32 = 0x3fff;

        #[link(name = "ws2_32")]
        extern "system" {
            fn setsockopt(
                socket: RawSocket,
                level: i32,
                option: i32,
                value: *const u8,
                length: i32,
            ) -> i32;
        }

        let socket = TcpSocket::new_v4().expect("socket");
        let enabled: u32 = 1;

        // SAFETY: same contract as `enable_port_reuse`, with an option number chosen to fail.
        let result = unsafe {
            setsockopt(
                socket.as_raw_socket(),
                SOL_SOCKET,
                NONSENSE_OPTION,
                std::ptr::addr_of!(enabled).cast::<u8>(),
                std::mem::size_of_val(&enabled) as i32,
            )
        };

        assert_ne!(
            result, 0,
            "Winsock accepted a nonsense option, so accepting SO_REUSE_UNICASTPORT proves nothing"
        );
    }

    /// The option governs how the local port is chosen, so it only applies before the socket is
    /// bound. Setting it late fails, which is why the connector sets it on a fresh socket.
    #[cfg(windows)]
    #[tokio::test]
    async fn the_port_reuse_option_must_be_set_before_binding() {
        let socket = TcpSocket::new_v4().expect("socket");
        socket
            .bind("127.0.0.1:0".parse().expect("address"))
            .expect("bind");

        assert!(
            enable_port_reuse(&socket).is_err(),
            "a bound socket should reject the option"
        );
    }

    #[tokio::test]
    async fn a_host_that_resolves_to_nothing_is_reported() {
        let connector = ReusePortsConnector {
            nodelay: true,
            keepalive: None,
            keepalive_interval: None,
            keepalive_retries: None,
            connect_timeout: Some(Duration::from_secs(1)),
        };

        let target = Uri::try_from("http://invalid.invalid:1/").expect("uri");
        let error = connector
            .connect(target)
            .await
            .expect_err("an unresolvable host must fail");

        assert!(
            error.to_string().contains("invalid.invalid"),
            "the error should name the host: {error}"
        );
    }

    // --- how the addresses are raced ---
    //
    // The scheduling is what is tested, not the sockets. `race` is generic over reaching an address
    // precisely so a test can supply attempts whose timing it decides, on a paused clock: there is no
    // way to make a socket blackhole a `SYN` from inside a test, which is why hyper's own equivalent
    // test needs real unreachable addresses and is excluded from its CI.

    /// Three addresses, distinguishable by their last octet.
    fn addresses(count: u8) -> Vec<std::net::SocketAddr> {
        (1..=count)
            .map(|last| {
                std::net::SocketAddr::from((std::net::Ipv4Addr::new(203, 0, 113, last), 443))
            })
            .collect()
    }

    fn last_octet(address: std::net::SocketAddr) -> u8 {
        match address.ip() {
            std::net::IpAddr::V4(v4) => v4.octets()[3],
            std::net::IpAddr::V6(_) => unreachable!("the fixtures are IPv4"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn the_first_address_wins_when_it_answers() {
        let started = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let seen = std::sync::Arc::clone(&started);
        let winner = race(addresses(3), FALLBACK_DELAY, |address| {
            seen.lock().expect("lock").push(last_octet(address));
            async move { Ok::<_, BoxError>(last_octet(address)) }
        })
        .await
        .expect("the first address answers");

        assert_eq!(winner, 1);
        assert_eq!(
            *started.lock().expect("lock"),
            vec![1],
            "no other address should have been tried"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_silent_address_is_overtaken_rather_than_waited_out() {
        // The regression this exists for: address 1 accepts and never answers. Sequentially that costs
        // the whole connect timeout; here address 2 is started `FALLBACK_DELAY` later and wins.
        let start = tokio::time::Instant::now();

        let winner = race(addresses(2), FALLBACK_DELAY, |address| async move {
            if last_octet(address) == 1 {
                std::future::pending::<()>().await;
            }
            Ok::<_, BoxError>(last_octet(address))
        })
        .await
        .expect("the second address answers");

        assert_eq!(winner, 2);
        assert_eq!(
            start.elapsed(),
            FALLBACK_DELAY,
            "the second address should start exactly one fallback later"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_failure_starts_the_next_address_without_waiting() {
        // A refusal is an answer, so there is nothing to overtake: move on at once.
        let start = tokio::time::Instant::now();

        let winner = race(addresses(2), FALLBACK_DELAY, |address| async move {
            match last_octet(address) {
                1 => Err(BoxError::from("refused")),
                other => Ok(other),
            }
        })
        .await
        .expect("the second address answers");

        assert_eq!(winner, 2);
        assert_eq!(
            start.elapsed(),
            Duration::ZERO,
            "a refusal should not wait out the fallback"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_last_error_is_reported_when_every_address_fails() {
        let error = race(addresses(3), FALLBACK_DELAY, |address| async move {
            Err::<u8, _>(BoxError::from(format!("refused {}", last_octet(address))))
        })
        .await
        .expect_err("every address fails");

        assert_eq!(error.to_string(), "refused 3");
    }

    #[tokio::test(start_paused = true)]
    async fn a_slow_address_still_wins_if_nothing_overtakes_it() {
        // Started attempts are not cancelled, so an address slower than the fallback is not lost.
        let winner = race(addresses(1), FALLBACK_DELAY, |address| async move {
            tokio::time::sleep(FALLBACK_DELAY * 10).await;
            Ok::<_, BoxError>(last_octet(address))
        })
        .await
        .expect("the slow address answers");

        assert_eq!(winner, 1);
    }

    /// One address per family, distinguishable by their last octet.
    fn dual_stack(families: &str) -> Vec<std::net::SocketAddr> {
        families
            .bytes()
            .enumerate()
            .map(|(index, family)| {
                let last = index as u8 + 1;
                match family {
                    b'4' => std::net::SocketAddr::from((
                        std::net::Ipv4Addr::new(203, 0, 113, last),
                        443,
                    )),
                    b'6' => std::net::SocketAddr::from((
                        std::net::Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, last as u16),
                        443,
                    )),
                    other => unreachable!("`{other}` is not a family"),
                }
            })
            .collect()
    }

    /// How the ordering reads, one character per address.
    fn families_of(addresses: &[std::net::SocketAddr]) -> String {
        addresses
            .iter()
            .map(|address| if address.is_ipv6() { '6' } else { '4' })
            .collect()
    }

    #[test]
    fn the_other_family_comes_second_however_the_resolver_ordered_them() {
        // The regression: raw order puts the lone IPv4 address two fallback delays in, so a
        // `connect_timeout` of 500ms expires before it is ever tried.
        for (resolved, expected) in [
            ("664", "646"),
            ("446", "464"),
            ("6644", "6464"),
            ("66644", "64646"),
        ] {
            let ordered = interleave_families(dual_stack(resolved));
            assert_eq!(families_of(&ordered), expected, "resolved as {resolved}");
        }
    }

    #[test]
    fn one_family_keeps_its_order_and_no_address_is_no_work() {
        let resolved = dual_stack("666");
        assert_eq!(interleave_families(resolved.clone()), resolved);
        assert!(interleave_families(Vec::new()).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn the_other_family_is_reached_one_fallback_in_not_two() {
        // Two records of the preferred family that hang, then a reachable one of the other.
        let start = tokio::time::Instant::now();

        let winner = race(
            interleave_families(dual_stack("664")),
            FALLBACK_DELAY,
            |address| async move {
                if address.is_ipv6() {
                    std::future::pending::<()>().await;
                }
                Ok::<_, BoxError>(address)
            },
        )
        .await
        .expect("the IPv4 address answers");

        assert!(winner.is_ipv4());
        assert_eq!(
            start.elapsed(),
            FALLBACK_DELAY,
            "the other family should not wait out every address of the first"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn no_address_is_an_error_rather_than_a_hang() {
        let error = race(Vec::new(), FALLBACK_DELAY, |_| async {
            Ok::<u8, BoxError>(0)
        })
        .await
        .expect_err("nothing to connect to");

        assert!(error.to_string().contains("no address"), "{error}");
    }
}
