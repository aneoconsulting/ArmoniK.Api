# armonik-transport

Transport layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) Rust client:
configuration parsing, and TLS/mTLS connection setup.

Depend on it when you need the connection layer without generated protobuf types or a
`protoc`/`tonic-prost-build` build step. [`armonik`](../armonik) re-exports all of it, so a client that
wants the services as well needs only that one.

## What it does, and what it leaves to you

It turns a set of options into a connected `tonic::transport::Channel`. That is the whole job.

It does **not** go looking for those options on its own. Nothing here reads an environment variable
unless asked to: `ClientConfigArgs::from_env`, behind the `env` feature, is offered because a variable
per option is the common case, not because `connect` calls it itself. `armonik` configures that
reading with its own `GrpcClient__` prefix; a host application binding through a C ABI hands over a
JSON document instead. The one exception is `ProxySource::System`, which reads `HTTPS_PROXY` and
`NO_PROXY` because that is a convention every HTTP client obeys rather than a setting of ArmoniK's.

It also has no notion of a *call*. A channel carries requests; deadlines, cancellation and replay are
properties of a call, so they belong to the layer that makes one. What this crate offers there is
inert material, described under [Replaying a failed request](#replaying-a-failed-request).

## Connecting

`ClientConfigArgs` is the string form of every option, the shape a caller fills in.
`ClientConfig::from_config_args` parses it, and `connect` opens the channel.

```rust,no_run
use armonik_transport::{ClientConfig, ClientConfigArgs};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = ClientConfig::from_config_args(ClientConfigArgs {
    endpoint: String::from("https://localhost:5001"),
    ca_cert: String::from("ca.pem"),
    ..Default::default()
})?;

let channel = armonik_transport::connect(config).await?;
# Ok(())
# }
```

With the `serde` feature, `ClientConfigArgs` deserialises from any format serde supports, which is how
a caller that is not written in Rust supplies its options.

## TLS and mutual TLS

`ca_cert` authenticates the server; `cert_pem` and `key_pem` together are the client's own identity
for mutual TLS, and must be either both set or both empty. All three are files this crate reads
itself.

`cert_p12` is an alternative to `cert_pem`/`key_pem`: the client's certificate and key bundled
together in one PKCS#12 file, the form Windows and most certificate authorities hand out, optionally
protected by `cert_p12_password`. Mutually exclusive with `cert_pem`/`key_pem` - set one identity or
the other, never both.

`cert_p12_password` is `Secret`, redacted by `Debug` and by serialisation; the other paths are not
secrets themselves, only what they lead to is. `allow_unsafe_connection` accepts any server
certificate instead of verifying it, for a self-signed endpoint; it has no effect on a plain `http://`
endpoint, which never negotiates TLS at all. `override_target_name` overrides the name checked during
verification, for an endpoint reached by an address that does not match its certificate.

## Reaching the endpoint through a proxy

`proxy` takes four forms, the same ArmoniK uses everywhere else: empty for a direct connection,
`"none"` to refuse one explicitly, `"system"` to read `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY`/`NO_PROXY`,
or the proxy's own URL. `proxy_username` and `proxy_password` authenticate to it, falling back to
whatever the URL itself carried.

The connection is an HTTP `CONNECT` tunnel: TLS, mutual TLS included, is negotiated end to end with
the real server, and the proxy only forwards opaque bytes. Because the `CONNECT` handshake itself goes
out in the clear, the proxy's own URL has to be `http`. The tunnel handshake itself is
`hyper_util::client::legacy::connect::proxy::Tunnel`, not code written in this crate.

### Known issues

`Tunnel`, as shipped in `hyper-util` 0.1.20, gets four cases wrong. None are ours to fix directly, and
each is pinned by a test named `known_issue_*`, in `tests/proxy.rs` or `tests/upstream_tunnel.rs`, so a
`hyper-util` release that fixes one turns that test red rather than letting the fix pass unnoticed.

- **Only an exact `200` opens the tunnel.** RFC 9110 says any `2xx` should. A proxy answering `201` or
  `204` is treated as a refusal.
- **A status line split across two reads is rejected**, even though the connection is fine. Legal
  HTTP, and likelier with a slow proxy or a small MSS.
- **An `HTTP/1.0 407` is not recognised as a request for credentials.** Only `HTTP/1.1 407` is; the
  older version falls into the same generic refusal as any other failure, so the message naming which
  two options to set is not shown for it.
- **A target with no port is dialled on `443`, whatever its scheme**, rather than the scheme deciding
  between `80` and `443`. ArmoniK deployments always name a port, so this is unlikely to matter in
  practice.

Track fixing the first two upstream through
[hyperium/hyper-util#300](https://github.com/hyperium/hyper-util/pull/300) and the ArmoniK.Api issue
tracker.

## Replaying a failed request

The policy lives here; the loop runs in your code.

`RetryPolicy` carries what the other ArmoniK clients use by default: five attempts, one second growing
by 1.5 to a five-second ceiling, replaying on `Unavailable`, `Aborted` and `Unknown`. The waits follow
the gRPC specification, `random(0, min(initial * multiplier^n, max))`, drawn uniformly *below* the
computed delay rather than added to it.

Driving it is the `retry!` macro, which expands in your own function so that each attempt can borrow
the client it is made on:

```
use armonik_transport::{GrpcStatus, RetryPolicy};
use armonik_transport::reexports::tonic;

# let runtime = armonik_transport::reexports::tokio::runtime::Builder::new_current_thread()
#     .enable_time()
#     .build()
#     .unwrap();
# runtime.block_on(async {
// Waits of zero, so the example does not sleep. Left alone the policy waits about a second.
let mut policy = RetryPolicy::default();
policy.initial_backoff = std::time::Duration::ZERO;
policy.max_backoff = std::time::Duration::ZERO;

let mut attempts = 0;

let outcome: Result<u32, tonic::Status> = armonik_transport::retry! {
    policy = Some(policy),
    code = GrpcStatus::grpc_code,
    // Evaluated afresh each turn: sending consumes a request, so make a new one.
    attempt = {
        attempts += 1;
        if attempts < 3 {
            Err(tonic::Status::unavailable("not yet"))
        } else {
            Ok(attempts)
        }
    }
};

assert_eq!(outcome.unwrap(), 3);
# });
```

You decide what may be replayed at all, because only you know it: whether the method is unary, whether
anything has already reached your own caller, and whether the request can still be reproduced. A
`policy` of `None` runs the attempt once.

The wait is a plain `.await`, so dropping the future abandons it immediately. Wrap the whole expansion
in a deadline rather than checking between attempts, or the deadline will not cut a backoff short.

## Publishing

**This crate has to be published before `armonik`.** `armonik` depends on it by `path`, and a `path`
dependency cannot be published: `cargo publish` on `armonik` rewrites it into a version requirement
against the registry, so the version it names must already be there.

```sh
cargo publish -p armonik-transport   # first, and wait for the index to pick it up
cargo publish -p armonik
```
