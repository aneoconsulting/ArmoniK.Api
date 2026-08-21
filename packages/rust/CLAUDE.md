# Rust packages

Bindings for the ArmoniK API: ergonomic Rust types implementing `prost::Message`
directly against the protobuf schema, with no generated intermediate types and no
conversion layer, plus the gRPC clients and servers speaking them natively.

## Layout

| Crate | Holds |
|---|---|
| `armonik` | the object types, the codec, `service!` invocations (`src/rpc/`), the generic `ServiceClient` and `Router`, the tests |
| `armonik-macros` | `#[armonik_macros::message]`, `#[armonik_macros::enumeration]`, `#[armonik_macros::alias]`, `#[armonik_macros::client]` and `service!`; resolution against the descriptor, then codegen |
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

Run from `packages/rust`. These are what CI runs (`format-rust` in
`.github/workflows/ci.yml`), in this order:

```bash
cargo build --workspace --locked
cargo fmt --all --check
RUSTDOCFLAGS="-Dwarnings" cargo doc --workspace --no-deps --all-features --document-private-items
cargo clippy --workspace --all-features --no-deps -- -Dwarnings -Dunused-crate-dependencies
cargo clippy --workspace --all-features --all-targets --no-deps -- -Dwarnings
```

Both clippy invocations, and in that order: `-Dunused-crate-dependencies` is per
target and a test target links the whole package's dev-dependencies, so it only
runs over the shipped targets; `--all-targets` is what covers the tests and the
benches, and it is the one that catches a bench-only lint. If a scratch file under
`armonik/benches/` does not build, rename it out of `*.rs` rather than dropping the
invocation that would have found the real error under it: every `benches/*.rs` is a
target, and `include = ["**/*.rs"]` would publish it too.

`features-rust` covers the seven feature configurations someone can actually
select, and `msrv-rust` pins the MSRV. Both are worth running by hand after
touching feature-gated code or reaching for a newer language feature:

```bash
for f in "" serde client server agent worker client,server; do
  cargo clippy -p armonik --locked --no-default-features --features "$f" \
    --no-deps -- -Dwarnings -Dunused-crate-dependencies
  cargo test -p armonik --no-default-features --features "$f" --lib
done
```

The `cargo test` half matters as much as the lint: clippy on the default targets
does not compile `cfg(test)`, so a test that only holds with every feature on
passes CI's `--all-features` run and fails the first thing a contributor types.

## Tests

```bash
cargo test -p armonik --lib --all-features               # codec and object unit tests
cargo test -p armonik --lib --all-features differential  # the ratchets, see below
cargo test -p armonik --all-features -- --skip mock      # integration, no server needed
```

Twelve of the sixteen suites in `armonik/tests/*.rs` are one `rpc_tests!` block,
one case per RPC, emitting an `in_process::{call, convenience}` pair against a
generated fake and a `mock::{call, convenience}` pair against the .NET mock. The
`mock` halves skip themselves and report `ok` when `GrpcClient__Endpoint` is
unset, so running them is opt-in by environment (below) rather than by flag; under
`CI` an unset endpoint is an assertion failure instead, since there the whole
cross-implementation half silently evaporating is the failure mode. The other four
suites (`server_socket`, `server_mounting`, `call_inputs`, `ui`) are written
out.

The differential harness has nine tests, all of which must keep passing:
round-trip against randomized `DynamicMessage`s, decoding of *mutated* encodings
(the byte-level layer: an interleaved unknown field, descending tag order,
repeated fields spread unpacked, duplicated singular fields), per-field
information (nothing the quotient erases without a justified allowlist
entry), descriptor coverage (every message mapped or tracked),
`default_encoding_is_the_proto_zero`, an absent oneof decoding to the memberless
variant, packed elements keeping their width, the types sharing one proto name
agreeing on their projection and default encoding, and every declared RPC having
a client method.

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

## Simplifying

Less is more, and the target is a mechanism general enough to need no special
cases -- not fewer capabilities. The rules are:

- **A degenerate case falls out of the general code.** An empty list
  interpolates to nothing, so the shape carrying none of something is the shape
  carrying some of it. A branch that exists only because one arm has zero of what
  another has is the defect, not the shape.
- **Two orthogonal axes are not a product of named cases.** A sum type carrying
  the coordinates it already encodes, or a mode inferred from a flag plus an
  emptiness test, gives one fact two homes that can disagree. Read it where it is
  decided.
- **A shared helper should be total over what its callers reach.** Where a
  sibling helper covers a case this one refuses, and the refusal is what pushed
  that case into a bespoke path, the hole is the defect. A helper that is simply
  never asked about a case -- `emit::slot_dispatch`, over the `Group` and
  `Poisoned` codecs, which frame themselves or are never dispatched -- is not.
- **Generalize toward the common case.** A struct is not an enum with one
  variant: unifying that way pessimizes every struct to subsume the two enums
  that carry shared fields.
- **Prefer removing a restriction to adding a guard.** If the emitter cannot
  express a shape, ask whether ordering or composing differently would, before
  writing a check that rejects it.

Before proposing an architectural alternative, read the registers of rejected
options in the pull request that introduced this crate: each entry is recorded
with the fact that refutes it, so the work is to refute the fact rather than to
re-derive the option.
