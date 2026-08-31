# Versioning

## Proto version

The `.proto` files live under a single `V1` package (`armonik.api.grpc.v1.*`). All services documented in the
[API Reference](../api-reference/overview.md) are part of this one protocol version — there is currently no `V2`
to select between.

## Package versions

Every generated package (C#, Python, TypeScript, Angular, C++) is released with the same version number, following
[Semantic Versioning](https://semver.org/): a release bumps every package together, so `ArmoniK.Api.Client 3.19.0`
and `armonik 3.19.0` are built from the same proto definitions. `Versions.ListVersions` (see the
[Quickstart](quickstart.md)) reports the running control plane's (`core`) and API (`api`) versions so you can check
compatibility at runtime.

## Release channels

- **Stable** — tagged releases on GitHub, published to NuGet/PyPI/npm.
- **Edge** — built from every commit on `main`, for C#, TypeScript and Angular (not yet for Python). Useful to test
  unreleased fixes, at the cost of a small risk of regressions.

See [Releases](../releases/index.md) for the full release process and per-language edge-channel links.

## Connecting to a specific version

The endpoint you connect to (`host:port`, e.g. `armonik.local:5001`) is just the address of the control plane —
there's no version segment in the URL or channel target. The proto/package version you compile against determines
which methods and fields are available; the control plane's own version (reported by `ListVersions`) determines
what it actually understands.
