# armonik-transport

Transport layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) Rust client:
configuration parsing, and TLS/mTLS connection setup.

Depend on it when you need the connection layer without generated protobuf types or a
`protoc`/`tonic-prost-build` build step. [`armonik`](../armonik) re-exports all of it, so a client that
wants the services as well needs only that one.

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

`Tunnel`, as shipped in `hyper-util` 0.1.20, gets four cases wrong. None are ours to fix directly; each
is pinned by a test in `tests/proxy.rs` or `tests/upstream_tunnel.rs`, so a `hyper-util` release that
fixes one turns that test red rather than letting the fix pass unnoticed.

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

## Publishing

**This crate has to be published before `armonik`.** `armonik` depends on it by `path`, and a `path`
dependency cannot be published: `cargo publish` on `armonik` rewrites it into a version requirement
against the registry, so the version it names must already be there.

```sh
cargo publish -p armonik-transport   # first, and wait for the index to pick it up
cargo publish -p armonik
```
