# armonik-transport

Transport layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) Rust client:
configuration parsing, and TLS/mTLS connection setup.

Depend on it when you need the connection layer without generated protobuf types or a
`protoc`/`tonic-prost-build` build step. [`armonik`](../armonik) re-exports all of it, so a client that
wants the services as well needs only that one.

## Reaching the endpoint through a proxy

`ClientConfig::proxy` decides how the connection goes out. The default is `ProxySource::System`, the
same as ArmoniK's C# client: `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY` are honoured and `NO_PROXY` is
matched the way curl matches it, so a client that configures nothing follows its environment and
connects directly when the environment names no proxy. `ProxyConfig::disabled()` forces a direct
connection; `ProxyConfig::explicit(uri)` names a proxy, which is then used whatever `NO_PROXY` says.
`with_credentials` authenticates to it, half by half: a half left empty keeps what the proxy URL
itself carried.

In the string form, `GrpcClient__Proxy` takes the same four values as ArmoniK's C# client: empty
and `system` follow the environment, `none` forces a direct connection, anything else is the
proxy's URL, defaulting to the `http` scheme. `GrpcClient__ProxyUsername` and
`GrpcClient__ProxyPassword` authenticate to it, falling back to whatever the URL itself carried. A
URL that cannot be dialled as written, no host, a port that is not one, or a scheme other than
`http`, is refused while the configuration is being read.

Two divergences from the C# client are deliberate. Spell `none` and `system` exactly so, or
capitalised: this crate accepts any casing, C# treats `NONE` as a proxy URL. And an `https` proxy
URL, which C# passes to the runtime, is refused here: the `CONNECT` handshake would go out in the
clear to a proxy expecting TLS.

The connection is an HTTP `CONNECT` tunnel: TLS, mutual TLS included, is negotiated end to end with
the real server, and the proxy only forwards opaque bytes. Because the `CONNECT` handshake itself goes
out in the clear, the proxy's own URL has to be `http`. The handshake is bounded by
`ClientConfig::connect_timeout`, 30 seconds when none is set.

## Publishing

**This crate has to be published before `armonik`.** `armonik` depends on it by `path`, and a `path`
dependency cannot be published: `cargo publish` on `armonik` rewrites it into a version requirement
against the registry, so the version it names must already be there.

```sh
cargo publish -p armonik-transport   # first, and wait for the index to pick it up
cargo publish -p armonik
```
