//! What `hyper_util`'s `Tunnel`, which [`crate::proxy`] drives, still gets wrong: a status line
//! split across two reads is rejected even though the connection is fine.
//!
//! This test is meant to fail: when a `hyper-util` release fixes it, the failure is the notice that
//! the crate README's "Known issues" section is out of date. Fixing it upstream is tracked by #702.
//! The unrelated "only an exact 200 opens the tunnel" defect is pinned once, in `tests/proxy.rs`,
//! through the full stack rather than here against the raw connector.
//!
//! A dependency bump turning CI red is the point. Read the failure before assuming it is a regression.

use hyper::Uri;
use hyper_util::client::legacy::connect::proxy::Tunnel;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower_service::Service;

/// How long the split case waits between the two halves of its status line.
///
/// Long enough that the reader, which is already waiting, takes the first half on its own rather than
/// finding both halves in one read. Nagle is off on that socket for the same reason.
const SPLIT_DELAY: std::time::Duration = std::time::Duration::from_millis(50);

/// A proxy that answers any `CONNECT` with `head`, in one write or in two.
///
/// It never dials the target: what is under test is how the response is read, so there is nothing to
/// tunnel to.
async fn spawn_proxy(head: &'static str, split_at: Option<usize>) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let address = listener.local_addr().expect("proxy address");

    tokio::spawn(async move {
        let Ok((mut client, _)) = listener.accept().await else {
            return;
        };
        // So the first half of a split status line leaves on its own segment rather than waiting for
        // the second and arriving as one read, which would make the split case pass for the wrong
        // reason.
        let _ = client.set_nodelay(true);

        // Read to the blank line that ends the request head.
        let mut seen = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            if client.read_exact(&mut byte).await.is_err() {
                return;
            }
            seen.push(byte[0]);
            if seen.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        // A closed connection is a different failure from a rejected status line, so the writes are
        // best effort and the socket is held open afterwards.
        match split_at {
            None => {
                let _ = client.write_all(head.as_bytes()).await;
            }
            Some(at) => {
                let _ = client.write_all(&head.as_bytes()[..at]).await;
                let _ = client.flush().await;
                tokio::time::sleep(SPLIT_DELAY).await;
                let _ = client.write_all(&head.as_bytes()[at..]).await;
            }
        }
        let _ = client.flush().await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    });

    address
}

/// Open a tunnel with `hyper_util`'s connector, which is what this crate would use if it could.
async fn tunnel_through(proxy: std::net::SocketAddr) -> Result<TcpStream, String> {
    let proxy_uri = Uri::try_from(format!("http://{proxy}")).expect("proxy uri");
    let mut connector = HttpConnector::new();
    connector.enforce_http(false);

    Tunnel::new(proxy_uri, connector)
        .call(Uri::try_from("https://example.invalid:443").expect("target uri"))
        .await
        .map(TokioIo::into_inner)
        .map_err(|error| error.to_string())
}

/// The control. Without it, a broken fixture would read as upstream still being broken.
#[tokio::test]
async fn a_plain_200_opens_the_tunnel() {
    let proxy = spawn_proxy("HTTP/1.1 200 Connection established\r\n\r\n", None).await;

    tunnel_through(proxy)
        .await
        .expect("the fixture itself must let an ordinary 200 through");
}

#[tokio::test]
async fn hyper_util_still_refuses_a_status_line_split_across_reads() {
    // Cut inside `HTTP/1.1 200`, so the first read carries `HTTP/1.1 2`, which matches neither prefix
    // `Tunnel` looks for and falls into its catch-all refusal. Legal, and likelier with a slow proxy
    // or a small MSS.
    let proxy = spawn_proxy("HTTP/1.1 200 Connection established\r\n\r\n", Some(10)).await;

    let error = tunnel_through(proxy).await.err().unwrap_or_else(|| {
        panic!(
            "hyper-util now reads a split status line correctly, or the two writes reached it as \
             one read. Rule the second out before believing the first: the halves are 50ms apart \
             on a socket with Nagle off. If it really is fixed, drop this test and update the \
             README's \"Known issues\" section."
        )
    });

    assert!(
        error.contains("unsuccessful"),
        "unexpected failure: {error}"
    );
}
