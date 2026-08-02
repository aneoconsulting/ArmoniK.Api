---
navigation.icon: vscode-icons:file-type-rust
---

<!-- @case-police-ignore Api -->

# Rust packages

Two crates, and which one to take depends on what you need.

`armonik` is the client: the generated protobuf types and one wrapper per ArmoniK service. Building it
runs `protoc` through a build script.

`armonik-transport` is the connection layer alone: reading a configuration, negotiating TLS or mutual
TLS, tunnelling through a proxy, and handing back a connected gRPC channel. No protobuf, no `protoc`.
Take it when you speak to ArmoniK over an ABI, from another language, or with your own generated code.
`armonik` re-exports all of it, so a client that wants the services as well needs only that one.

## Where the options come from

Every option is a `GrpcClient__*` environment variable, spelled as it is for the C# and C++ clients:
`GrpcClient__Endpoint`, `GrpcClient__CaCert`, `GrpcClient__Proxy`, and the rest.

Reading them is the transport's own mechanism, `ClientConfigArgs::from_env`, generic over the prefix
only: the `PascalCase` naming is fixed at compile time. `armonik` configures it with its own
`GrpcClient__` prefix, in one line, rather than reading each variable itself. A host application that
keeps its settings elsewhere, in a JSON document or a command line, deserialises a `ClientConfigArgs`
directly instead.

Certificates are files this crate reads itself. `GrpcClient__CertPem` and `GrpcClient__KeyPem` name
the client's own certificate and key; `GrpcClient__CaCert` names the Certificate Authority.
`GrpcClient__CertP12`, with `GrpcClient__CertP12Password`, is an alternative for a certificate and key
bundled together in one PKCS#12 file, the form Windows and most certificate authorities hand out.

## Connecting

```rust
let mut client = armonik::Client::new().await?;
let versions = client.versions().list().await?;
```

`Client::new()` reads every `GrpcClient__*` variable and connects in one step. To build a
configuration by hand rather than from the environment, fill in a `ClientConfigArgs` and pass it to
`ClientConfig::from_config_args`, then `Client::with_config`.

## Reaching the endpoint through a proxy

`GrpcClient__Proxy` takes the same four forms as elsewhere in ArmoniK: empty for a direct connection,
`none` to refuse one explicitly, `system` to follow `HTTPS_PROXY` and `NO_PROXY`, or the proxy's URL.
`GrpcClient__ProxyUsername` and `GrpcClient__ProxyPassword` authenticate to it.

The connection is an HTTP `CONNECT` tunnel, so TLS is negotiated end to end with the real server and
the proxy forwards opaque bytes. It never sees the plaintext, and mutual TLS keeps working through it.
Because the handshake goes out in the clear, the proxy's own URL has to be `http`.

## Replaying a failed request

The policy travels with the configuration: `GrpcClient__MaxAttempts`, `GrpcClient__InitialBackoff`,
`GrpcClient__MaxBackoff`, `GrpcClient__BackoffMultiplier`, `GrpcClient__RetryableStatusCodes`,
`GrpcClient__MaxRetryBufferPerCall` and `GrpcClient__MaxRetryUnarySize`. Left alone they are what the
C# client uses, so a deployment behaves the same whichever client talks to it: five attempts, one
second growing by 1.5 to a five-second ceiling, replaying on `Unavailable`, `Aborted` and `Unknown`.
`GrpcClient__MaxAttempts=1` never replays.

The waits follow the gRPC specification, `random(0, min(initial * multiplier^n, max))`, drawn
uniformly below the computed delay so that clients which failed together do not come back together.

What the transport does **not** decide is whether a given request may be sent twice. A channel has no
notion of a call, so that judgement belongs to whoever makes one: only they know whether the method is
unary, whether anything has already been handed to their own caller, and whether the request can still
be reproduced. The `armonik_transport::retry!` macro carries the loop into that layer.
