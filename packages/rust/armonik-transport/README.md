# armonik-transport

Transport layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) Rust client:
configuration parsing, and TLS/mTLS connection setup.

Depend on it when you need the connection layer without generated protobuf types or a
`protoc`/`tonic-prost-build` build step. [`armonik`](../armonik) re-exports all of it, so a client that
wants the services as well needs only that one.

## Describing the options to another language

The `schema` feature derives a JSON schema of the flat PascalCase option vocabulary (`Endpoint`,
`TcpKeepalive`, `Http2KeepAliveInterval`, ...) from the types that define it, for a consumer that
generates an options class of its own. Nothing is committed; print it with:

```sh
cargo run -p armonik-transport --features schema --example generate_schema
```

The Certificate Authority is named `CaCertPath`, where ArmoniK's C# client spells it `CaCert`. The
option is a path to a PEM file, not the certificate, and the name says so. Until the C# client is
renamed to match, a deployment serving both names the file twice.

## The client's identity for mutual TLS

The identity is loaded while the configuration is read, so a mistyped path fails there, naming the
option, rather than later as a refused handshake. It comes one of two ways.

`CertPem` and `KeyPem` are the certificate chain and its key, each in its own PEM file; set both or
neither. `CertP12` is the alternative: the two bundled together in one PKCS#12 file, the form
Windows and most certificate authorities hand out, optionally protected by `CertP12Password`.
Whichever way it is spelled, the whole chain the file carries is presented, leaf first, so a server
that trusts only the root can still build its path. `CertP12` and `CertPem`/`KeyPem` are mutually
exclusive - set one identity or the other, never both - and a `CertP12Password` naming no bundle is
refused rather than ignored.

ArmoniK's C# client reads the same `CertP12` option but has no `CertP12Password` counterpart, so a
password-protected bundle is not portable to it.

## Reaching the endpoint through a proxy

`HttpConfig::proxy` decides how the connection goes out. The default is `ProxySource::System`, the
same as ArmoniK's C# client: `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY` are honoured and `NO_PROXY` is
matched the way curl matches it, so a client that configures nothing follows its environment and
connects directly when the environment names no proxy. `ProxyConfig::disabled()` forces a direct
connection; `ProxyConfig::explicit(uri)` names a proxy, which is then used whatever `NO_PROXY` says.
`with_credentials` authenticates to it, half by half: a half left empty keeps what the proxy URL
itself carried.

The connection is an HTTP `CONNECT` tunnel: TLS, mutual TLS included, is negotiated end to end with
the real server, and the proxy only forwards opaque bytes. Because the `CONNECT` handshake itself goes
out in the clear, the proxy's own URL has to be `http`. The handshake is bounded by
`HttpConfig::connect_timeout`, 60 seconds when none is set.

## Publishing

**This crate has to be published before `armonik`.** `armonik` depends on it by `path`, and a `path`
dependency cannot be published: `cargo publish` on `armonik` rewrites it into a version requirement
against the registry, so the version it names must already be there.

```sh
cargo publish -p armonik-transport   # first, and wait for the index to pick it up
cargo publish -p armonik
```
