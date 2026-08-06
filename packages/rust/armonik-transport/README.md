# armonik-transport

Transport layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) Rust client:
configuration parsing, and TLS/mTLS connection setup.

Depend on it when you need the connection layer without generated protobuf types or a
`protoc`/`tonic-prost-build` build step. [`armonik`](../armonik) re-exports all of it, so a client that
wants the services as well needs only that one.

## Reading the configuration

`HttpConfig` is plain data: build it directly, or turn on a feature to read it from somewhere. The
`serde` feature makes it deserializable (`Deserialize` only: a configuration that cannot be written
out cannot leak its proxy password), from a flat document of PascalCase options (`Endpoint`,
`TcpKeepalive`, `Http2KeepAliveInterval`, ...). The `env` feature adds
`HttpConfig::from_env(prefix)`, reading each option from one environment variable under a prefix of
the caller's choosing; the `armonik` crate reads them under the `GrpcClient__` prefix. The `schema`
feature adds two JSON schemas (via `schemars`), each committed under [`schema/`](schema/) and
pinned by a test: [`http_config.flat.schema.json`](schema/http_config.flat.schema.json) is the flat
option vocabulary for generating an options class, and
[`http_config.schema.json`](schema/http_config.schema.json) is the config's real, structured shape.

## Reaching the endpoint through a proxy

`HttpConfig::proxy` decides how the connection goes out. The default is `ProxySource::System`, the
same as ArmoniK's C# client: `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY` are honoured and `NO_PROXY` is
matched the way curl matches it, so a client that configures nothing follows its environment and
connects directly when the environment names no proxy. `ProxyConfig::disabled()` forces a direct
connection; `ProxyConfig::explicit(uri)` names a proxy, which is then used whatever `NO_PROXY` says.
`with_credentials` authenticates to it, half by half: a half left empty keeps what the proxy URL
itself carried.

In the string form, the `Proxy` option takes the same four values as ArmoniK's C# client: empty
and `system` follow the environment, `none` forces a direct connection, anything else is the
proxy's URL, defaulting to the `http` scheme. Credentials come one of two ways: written into the
URL, or through `ProxyUsername` and `ProxyPassword` next to a clean URL; setting both is refused
rather than merged. A URL that cannot be dialled as written, no host, a port that is not one, or a
scheme other than `http`, is refused while the configuration is being read. The `armonik` crate
reads these options from the environment under the `GrpcClient__` prefix (`GrpcClient__Proxy`, ...).

Two divergences from the C# client are deliberate. Spell `none` and `system` exactly so, or
capitalised: this crate accepts any casing, C# treats `NONE` as a proxy URL. And an `https` proxy
URL, which C# passes to the runtime, is refused here: the `CONNECT` handshake would go out in the
clear to a proxy expecting TLS.

### Known issues

The tunnel handshake is `hyper_util::client::legacy::connect::proxy::Tunnel`, not code written in
this crate, and as shipped in `hyper-util` 0.1.20 it gets four cases wrong. None are ours to fix
directly; [hyperium/hyper-util#300](https://github.com/hyperium/hyper-util/pull/300) tracks the
first two upstream. All but the second are pinned by a tripwire in `tests/upstream_tunnel.rs`, so a
`hyper-util` release that fixes one turns that test red rather than letting the fix pass unnoticed.

- **Only an exact `200` opens the tunnel.** RFC 9110 says any `2xx` should. A proxy answering `201`
  or `204` is treated as a refusal.
- **A status line split across two reads is rejected**, even though the connection is fine. Legal
  HTTP, and likelier with a slow proxy or a small MSS. Not pinned by a tripwire: asserting on how
  reads land needs timing assumptions that make the test flaky on a loaded runner.
- **An `HTTP/1.0 407` is not recognised as a request for credentials.** Only `HTTP/1.1 407` is; the
  older version falls into the same generic refusal as any other failure, so the message naming
  which two options to set is not shown for it.
- **A target with no port is dialled on `443`, whatever its scheme**, rather than the scheme
  deciding between `80` and `443`. ArmoniK deployments always name a port, so this is unlikely to
  matter in practice.

The connection is an HTTP `CONNECT` tunnel: TLS, mutual TLS included, is negotiated end to end with
the real server, and the proxy only forwards opaque bytes. Because the `CONNECT` handshake itself goes
out in the clear, the proxy's own URL has to be `http`. The handshake is bounded by
`HttpConfig::connect_timeout`, 30 seconds when none is set.

## Publishing

**This crate has to be published before `armonik`.** `armonik` depends on it by `path`, and a `path`
dependency cannot be published: `cargo publish` on `armonik` rewrites it into a version requirement
against the registry, so the version it names must already be there.

```sh
cargo publish -p armonik-transport   # first, and wait for the index to pick it up
cargo publish -p armonik
```
