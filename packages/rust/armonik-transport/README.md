# armonik-transport

Transport layer for the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) Rust client:
configuration parsing, and TLS/mTLS connection setup.

This crate is factored out of [`armonik`](../armonik), which re-exports everything here at the same
paths it always used (`armonik::ClientConfig`, `armonik::client::ConfigError`, ...), so depending on
`armonik` directly is unaffected by the split. Depend on `armonik-transport` instead when you need the
connection layer without generated protobuf types or a `protoc`/`tonic-prost-build` build step.

## Publishing

**This crate has to be published before `armonik`.** `armonik` depends on it by `path`, and a `path`
dependency cannot be published: `cargo publish` on `armonik` rewrites it into a version requirement
against the registry, so the version it names must already be there.

```sh
cargo publish -p armonik-transport   # first, and wait for the index to pick it up
cargo publish -p armonik
```
