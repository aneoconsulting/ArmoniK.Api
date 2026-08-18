# Rust packages

Bindings for the ArmoniK API: ergonomic Rust types implementing `prost::Message`
directly against the protobuf schema, with no generated intermediate types and no
conversion layer, plus the gRPC clients and servers speaking them natively.

`armonik/DESIGN.md` is the design record. It is meant to describe what the code
does now, so a change to the mechanisms below changes it too.

## Layout

| Crate | Holds |
|---|---|
| `armonik` | the object types, the codec, `service!` invocations (`src/rpc/`), the generic `ServiceClient` and `Router`, the tests |
| `armonik-macros` | `#[armonik_macros::message]`, `#[armonik_macros::enumeration]`, `#[armonik_macros::alias]` and `service!`; resolution against the descriptor, then codegen |
| `armonik-transport` | endpoint configuration, TLS/mTLS, connectors. Independent, so depending on it alone keeps protobuf codegen out of a build |

The protos in `../../Protos/V1` are the source of truth. `armonik/build.rs`
compiles them with `protox` into `$OUT_DIR/descriptor.bin`; the macros read that
file at expansion time and take tags, wire kinds, cardinalities and
documentation from it, never from the Rust source. Every expansion const-asserts
a descriptor fingerprint, so a stale expansion cannot survive a proto change.

## Invariants

Breaking one of these is a wire bug, not a style issue. They are enforced by the
differential harness (`armonik/src/differential/`, a `cfg(test)` module) rather
than by review.

- **`Default::default()` is the proto zero value** for every type, so decoding
  seeds from `Default` with no special rules. Non-zero starting points live in
  named constructors (`TaskOptions::recommended()`,
  `<service>::list::Request::recommended()`, `Sort::ascending`), never in
  `Default`.
- **Only an implicit-presence leaf skips its zero.** A scalar, `String`, `Bytes`
  or enum field holding the proto zero is left out, like any proto3 encoder;
  everything else is written whatever it holds, message fields and oneof members
  included (a member being there is what selects its variant). The skip lives in
  `ProtoField::encode_implicit` over an `is_zero` that is `false` by default, so
  a codec opts into skipping rather than out of it; the derives only pick which
  of the two entry points each slot is written through.
- **Enums are open**: one dataful `Unknown(UnknownX)` catch-all carries the zero
  value and anything this crate version does not know, losslessly. The
  comparison traits are emitted in terms of the proto value rather than derived
  (the resolver rejects a site derive), so `Ord` follows the proto values and an
  unknown value sorts by the number it carries.
- **Request types are injective over RPCs**: two RPCs sharing a wire message
  still get a Rust type each, which is what lets `client.call(request)` deduce
  the RPC from its argument.
- Types whose fields allow it derive `Eq`; the object types are uniform on this.

## Commands

Run from `packages/rust`. The first four are what CI runs (`format-rust` in
`.github/workflows/ci.yml`), in this order:

```bash
cargo build --workspace --locked
cargo fmt --all --check
RUSTDOCFLAGS="-Dwarnings" cargo doc --workspace --no-deps
cargo clippy --workspace --all-features --no-deps -- -Dwarnings -Dunused-crate-dependencies
```

CI only ever lints `--all-features`, so a client-less or server-less build can
rot unnoticed. Check the combinations by hand after touching feature-gated code:

```bash
for f in "" "server" "serde" "server,serde" "client" "agent" "worker"; do
  cargo clippy -p armonik --no-default-features ${f:+--features $f} --no-deps --lib -- -Dwarnings
done
```

`--all-targets` picks up untracked scratch files under `armonik/benches/`, which
do not build. Lint with `--lib --tests` instead.

## Tests

```bash
cargo test -p armonik --lib --all-features               # codec and object unit tests
cargo test -p armonik --lib --all-features differential  # the ratchets, see below
cargo test -p armonik --all-features -- --skip mock      # integration, no server needed
```

Each suite in `armonik/tests/*.rs` is one `rpc_tests!` block, one case per RPC,
emitting an `in_process::{call, convenience}` pair against a generated fake and a
`mock::{call, convenience}` pair against the .NET mock. The `mock` halves need
that server, so they fail with a URI error unless it is running (below).

The differential harness has five ratchets, all of which must keep passing:
round-trip against randomized `DynamicMessage`s, per-field information (nothing
the quotient erases without a justified allowlist entry), descriptor coverage
(every message mapped or tracked), `default_encoding_is_the_proto_zero`, and the
types sharing one proto name agreeing on their projection and default encoding.

### Running the mock

```bash
cd ../csharp/out
( Grpc__Port=5000 Http__Port=4999 dotnet ArmoniK.Api.Mock.dll > /tmp/mock.log 2>&1 & )
curl -s --retry 30 --retry-connrefused --retry-delay 1 http://localhost:4999/calls.json
cd ../../rust
GrpcClient__Endpoint=http://localhost:5000 \
Http__Endpoint=http://localhost:4999 \
GrpcClient__AllowUnsafeConnection=true \
  cargo test -p armonik --all-features
```

Build it first with `dotnet publish -o ../out` from
`packages/csharp/ArmoniK.Api.Mock` if `out/ArmoniK.Api.Mock.dll` is missing. To
stop it, match on the process name
rather than the command line: `pkill -f ArmoniK.Api.Mock.dll` also matches the
shell running it.

```bash
ps -eo pid=,comm=,args= | awk '$2=="dotnet" && /Mock\.dll/ {print $1}' | xargs -r kill
```

## Conventions

- Comments and rustdoc describe the current state. Not what the code used to do,
  not what a previous implementation did; that is what the history is for.
- A wire-behaviour change needs a test that pins the direction, not just a
  passing round-trip: the harness compares re-encoded bytes, so a mapping that
  is wrong symmetrically survives it.
- Keep `armonik/DESIGN.md` in step with the mechanisms; it is the only place the
  reasoning behind them is written down.
