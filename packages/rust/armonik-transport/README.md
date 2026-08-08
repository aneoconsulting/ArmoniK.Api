# armonik-transport

Transport layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) Rust client:
configuration parsing, and TLS/mTLS connection setup.

`https_connector` is where the crate stops: it hands back the connector a request goes out through,
TCP then the proxy tunnel then TLS, and wrapping that in an HTTP/2 engine belongs to whoever
consumes it. [`armonik`](../armonik) wraps it in a `tonic` channel.

Depend on it when you need the connection layer without generated protobuf types or a
`protoc`/`tonic-prost-build` build step. [`armonik`](../armonik) re-exports all of it, so a client that
wants the services as well needs only that one.

## Driving the connection yourself

`Connector` is the stack `https_connector` returns, TCP then the proxy tunnel then TLS, so a
consumer that wraps it in an engine of its own can name what it holds: `Client<Connector, B>` rather
than a trait object erasing the connector. `ProxyConnector`, one of its layers, is exported for the
same reason; it is assembled from an `HttpConfig`, never by hand.

`armonik_transport::reexports` carries the crates the connector is built from and spoken to: `http`,
`hyper`, `hyper_util`, `hyper_rustls`, `rustls`, `h2`, `http_body_util` and `tokio`. Take them from
there rather than declaring your own requirement for the same crates, and no version of a shared
type can drift from the one the connection was built with. `h2` and `http_body_util` are re-exported
for that alone; nothing in this crate uses them.

Only the versions are guaranteed, not the features. Which parts of a re-exported crate exist is
decided by what every crate in the build asks for together, and this one asks only for the features
it needs itself. So a consumer that needs a runtime still declares `tokio` for that: the `tokio`
re-exported here is the one whose stream types the connector hands back, not a runtime.
`http_body_util` is likewise re-exported without `channel`, so a consumer that feeds a request body
through `http_body_util::channel::Channel` turns that feature on itself, where the `tokio` edge it
adds reads as that consumer's need rather than as this crate's.

## Reading the options from the environment

The `env` feature adds `HttpConfig::from_env(prefix)`: one variable per option, named `prefix` plus
the option's own PascalCase spelling, with the prefix entirely the caller's to choose. An absent
variable and one declared empty both read as the option's default. A value an option refuses is
reported as the variable to go and fix, `prefix` included; an option belonging to one of the
grouped units (`Tcp*`, `Http2*`, and the TLS options) is named by the unit instead, without the
prefix. `armonik` sets the prefix ArmoniK deployments use, `GrpcClient__`.

`HttpConfig::from_env_vars(prefix, variables)` reads the same options out of any set of variables,
for a caller holding an environment other than its own process's.

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
## Replaying a failed request

`HttpConfig::retry` holds the policy: `MaxAttempts`, `InitialBackOff`, `MaxBackOff` and
`BackOffMultiplier`, defaulting to what ArmoniK's other clients hand grpc-dotnet, so a deployment
that sets nothing behaves the same whichever client talks to it. It is applied by whoever makes the
calls: a channel carries no notion of a call, so nothing that builds one reads it.

`BackOffMultiplier` is spelled `BackoffMultiplier`, with a lowercase o, by the C# client. Until that
client is renamed to match, a deployment setting `GrpcClient__BackoffMultiplier` is read by C# and
not by this crate, which reads `GrpcClient__BackOffMultiplier`. `InitialBackOff` and `MaxBackOff`
are spelled identically on both sides.

## Reaching the endpoint through a proxy

`HttpConfig::proxy` decides how the connection goes out. The default is `ProxySource::System`, the
same as ArmoniK's C# client: `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY` are honoured and `NO_PROXY` is
matched the way curl matches it, so a client that configures nothing follows its environment and
connects directly when the environment names no proxy. `ProxyConfig::disabled()` forces a direct
connection; `ProxyConfig::explicit(uri)` names a proxy, which is then used whatever `NO_PROXY` says.
`with_credentials` authenticates to it, half by half: a half left empty keeps what the proxy URL
itself carried.

As options, `ProxyAddress` takes the same four values as ArmoniK's C# client: empty and `system`
follow the environment, `none` forces a direct connection, anything else is the proxy's URL,
defaulting to the `http` scheme. `ProxyUsername` and `ProxyPassword` authenticate to it. A URL that
cannot be dialled as written, no host, a port that is not one, or a scheme other than `http`, is
refused while the configuration is being read.

Credentials go in one place or the other, never both: a URL carrying `user:password@` next to a
non-empty `ProxyUsername` or `ProxyPassword` is refused rather than merged, since guessing which
half of which source wins would turn a mixed configuration into a silent surprise.

Three divergences from the C# client are deliberate. The option is `ProxyAddress`, where C# spells
it `Proxy`; the matching C# rename is filed separately, and until it lands a deployment setting
both clients names the proxy twice. Spell `none` and `system` exactly so, or capitalised: this
crate accepts any casing, C# treats `NONE` as a proxy URL. And an `https` proxy URL, which C#
passes to the runtime, is refused here: the `CONNECT` handshake would go out in the clear to a
proxy expecting TLS.

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
