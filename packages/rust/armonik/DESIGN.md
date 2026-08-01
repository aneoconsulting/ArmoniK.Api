# Design: direct wire implementation for the ergonomic API types

Status: **implemented** (branch `rust/direct-message-impls`)
Target: big-bang branch, released as a breaking beta bump.

## 1. Motivation

The crate currently maintains two parallel representations of every API
message:

- `src/api/v3.rs`: prost/tonic-generated types (~218 messages, 21 enums,
  27 oneofs), straight from the protos.
- `src/objects/`: ~6,700 hand-written lines of ergonomic mirror types, plus
  bidirectional `From` conversions (`impl_convert!`), which the `client/` and
  `server/` layers apply on every request and response.

The ergonomic layer exists for good reasons and its shape is kept:

- proto enums are native Rust enums;
- message fields that are semantically "absent = default" are plain fields,
  not `Option`;
- messages consisting of a single oneof are flattened into a Rust enum
  (`events::Update`, `DataChunk`, …);
- per-rpc module layout (`sessions::list::Request`), field renames
  (`Session.id` → `session_id`), unified types shared across services
  (`TaskOptionField`).

The goal of this revamp: **the ergonomic types implement `prost::Message`
directly and become the only representation.** The generated message structs,
the conversion layer, and the double decode/convert pass disappear.
`api::v3` is removed from the public API entirely.

### 1.1 Complexity ledger (honest accounting)

Total system complexity did not shrink — it **relocated**, and this section
says so plainly so the trade is reviewed on its merits. The ~6,700-line
hand-written mirror plus its `From` conversions are gone, but hand-maintained
*production* code across the three crates is now larger in aggregate (the
descriptor-reading proc-macro, the codec, the build pipeline, and the annotated
`objects/`), and it moved from local `From` impls a maintainer edits *next to
the type* into three places that carry complexity less locally: a proc-macro
that reads `descriptor.bin` at expansion, a two-crate build pipeline that
rewrites a `FileDescriptorSet`, and a `linkme` slice harvested across a
build-dependency edge with three `=`-pinned crates released in lockstep.

This is the usual, and here the correct, infra-vs-boilerplate trade: the old
lines were *O(messages)* and drifted silently with no automated net, while the
new machinery is fixed-cost and the marginal message is near-free (the derive
self-registers; a plain field's `Normalize` is the identity). But the payoff is
"complexity amortized and made drift-proof", not "complexity removed" — approve
the beta break on that basis, not on a per-message line count.

## 2. Decisions (settled)

| Topic | Decision |
|---|---|
| Implementation strategy | Derive proc-macro (`armonik-macros` crate), descriptor-driven |
| Descriptor access | Hybrid: `protox` in `build.rs` compiles descriptors; derive reads the compiled `descriptor.bin`; fingerprint const-assert guards staleness |
| Public API | Same shapes as today; minor breaks allowed (crate is beta on purpose) |
| Raw generated types | Removed entirely, including from tonic stub signatures (`extern_path`) |
| Unknown enum values | Single merged catch-all `Other(Raw…)` dataful variant covering unspecified (0) and unknown values; opaque payload; lossless round-trip |
| Message-field presence | Non-`Option` message fields: decode absent as default, **omit on encode when the nested encoding is empty** (free check via `encoded_len() == 0`); `Option` fields keep exact presence |
| `bytes` fields | `bytes::Bytes` everywhere (zero-copy decode from tonic buffers) |
| Migration | Big-bang branch; `main` untouched until the branch lands |
| Validation | Compile-time (derive vs descriptor) + generic differential round-trip harness (`prost-reflect` `DynamicMessage`) |
| protoc | Dropped; `protox` makes the build pure Rust |

## 3. Architecture

### 3.1 Crate layout

```
packages/rust/
  armonik/            # tonic client/server stubs + ergonomic client/server
                      #   wrappers; re-exports armonik-types wholesale
  armonik-types/      # the message types: ergonomic structs/enums implementing
                      #   prost::Message directly (objects + codec + the
                      #   differential harness); its build.rs compiles the
                      #   descriptor. A pure-types dependency — no tonic graph.
  armonik-macros/     # proc-macro crate: derive(Message), derive(Enum)
                      #   deps: syn, quote, prost (descriptor decode only)
```

The three crates are version-locked with `=` pins (`armonik` → `armonik-types`
→ `armonik-macros`) and published in that order. The derives are internal-use
— they emit `crate::codec::…` paths, so they only expand inside
`armonik-types` — and `#[doc(hidden)]`: the attribute grammar is not a
supported public API.

`armonik-types` exists so downstream can depend on the wire types without the
client/server stubs and their tonic/hyper/rustls graph, and — the reason the
split earns its keep — so `armonik`'s build script can **harvest** the
proto-name → Rust-path extern map, the per-RPC substitutions and the absorbed
(flattened-away) messages from the `#[armonik(...)]` annotations instead of
hand-maintaining them. `armonik-types` is a build-dependency of `armonik`;
every derive and the two hand-written impls register — through one `register!`
macro (the single home of the registry's layout) — one `Registration` per
proto name into a single `linkme` slice (`armonik_types::wire::REGISTRY`, gated
behind the base `_registry` feature the build-dependency enables). Each entry's
`Role` is `Message { rust_path }`, `Replace(Replacement)`, or `Absorbed`;
`armonik`'s `build.rs` reads `extern_mapping()`, `replacements()` and
`absorbed()`, which filter the one slice. The map is fully harvested — no
hand-maintained residue: entries that cannot be a plain extern mapping (the
shared `Empty`, `TaskFilter`, … sites) carry `#[armonik(replace(...))]` and
register a `Replacement` instead. Two build-time guards keep it honest:
`guard_all_messages_externed` fails the build if any top-level message survives
stub pruning without an extern entry, and `guard_unique_extern` fails it if two
Rust types claim one proto name (see the 1-of-N convention on `wire::Role`).

The `_registry` feature is the base; the differential harness's
`_differential` feature *extends* it (`_differential = ["_registry",
"dep:prost-reflect"]`), adding the round-trip/`Normalize` hooks as a
feature-gated field of `Registration`. So `armonik`'s build-dependency, which
enables only `_registry`, pulls `linkme` but never `prost-reflect`, and a
downstream pure-types build of `armonik-types` pulls neither.

### 3.2 Build pipeline (`build.rs`)

> **Stubs-only generation.** The descriptor set is compiled once (protox)
> and used twice: the full set becomes `descriptor.bin` for the derives and
> the differential harness, while the tonic stub generation receives a
> *pruned* copy — the never-exposed WatchResults methods are dropped
> (tonic answers UNIMPLEMENTED for unrouted paths), every message that
> nothing generated references (field wrappers, watch messages, legacy) is
> removed alongside all file-level enums, and every RPC slot carrying a
> `#[armonik(replace(...))]` type (the `Empty` sites, the shared `TaskFilter`
> / request wrappers, …) is rewritten to a distinct synthetic message
> extern'd to its API type (message type names never appear on the wire,
> so wire-compatible signature rewrites are free). Combined with the
> harvested extern map, the generated module (`crate::stubs`, private)
> contains exactly the 12 client and 12 server stubs and nothing else; every
> service's stub is re-exported as the public `stub` module of its
> client/server module (`armonik::client::sessions::stub::SessionsClient`,
> `armonik::server::sessions::stub::{Sessions, SessionsServer}`). The
> message-only packages produce no file at all, and without the
> client/server features the module is not even compiled (the crate then
> works as a pure-types dependency).

The pipeline is split across the two crates. **`armonik-types/build.rs`**:

1. `protox` compiles `protos/V1/*.proto` → `FileDescriptorSet`
   (pure Rust; `protoc` no longer required; `cargo:rerun-if-changed` per
   proto file).
2. Writes `$OUT_DIR/descriptor.bin` (input of the derive) and
   `$OUT_DIR/schema_meta.rs` containing
   `pub(crate) const DESCRIPTOR_FINGERPRINT: u128 = …;`
   (hash of the descriptor bytes). `lib.rs` pulls the latter in via
   `include!`, which puts it in rustc's dep-info — tracked by cargo, sccache,
   and any hermetic build system. The descriptor bytes are also re-exported as
   `armonik_types::wire::DESCRIPTOR`.

**`armonik/build.rs`** (with `armonik-types` as a build-dependency, so it is
compiled first and its derives have run):

3. Decodes `armonik_types::wire::DESCRIPTOR` (no proto compilation here), prunes
   it, and feeds it to `tonic-prost-build` via `compile_fds` to generate
   **service stubs only**:
   - the `extern_path` entries come from `armonik_types::wire::extern_mapping()`
     — the annotations, harvested — mapping each RPC type to its ergonomic type
     (`.armonik.api.grpc.v1.sessions.ListSessionsRequest`
     → `::armonik_types::sessions::list::Request`), plus one entry per
     `replacements()` substitution mapping its synthetic target message to the
     standing-in type; there is no hand-maintained extern list;
   - extern'd messages are not generated at all, and since nested types never
     appear in stub signatures, **only the top-level RPC types need entries**;
   - `guard_all_messages_externed` fails the build if any top-level message
     survives pruning without an extern entry, and `guard_unique_extern` fails
     it if two Rust types claim one proto name — the ratchets that keep the
     harvested map honest as the schema evolves.

### 3.3 The derive

`#[derive(armonik::Message)]` on structs and oneof-shaped enums,
`#[derive(armonik::Enum)]` on proto enums. An enum with `message = ...`
alone stands for the *whole* message: its single oneof is inferred, and any
non-oneof field of the message is a *sibling*, declared in every variant
(including the attribute-less "no member set" one) so the per-field merge
stays stateless and order-independent. `oneof = "..."` declares an enum for
one oneof of a larger message, embedded in a struct — and is rejected when
the oneof covers the whole message, keeping the two shapes visually
distinct. At expansion time the macro:

1. reads `$OUT_DIR/descriptor.bin` (env `OUT_DIR`; one prost decode, cached
   in a `OnceLock` for the whole rustc/rust-analyzer process; clear error if
   the file is missing, i.e. build scripts have not run);
2. resolves the message named by `#[armonik(message = "…")]` and matches Rust
   fields against proto fields **by name** (renames via attribute);
3. pulls tag, wire kind, cardinality, packedness from the descriptor —
   nothing is duplicated in the source;
4. validates: unknown field, type/kind mismatch, missing proto field
   (completeness), proto3 explicit-`optional` scalar not mapped to `Option` —
   all **spanned compile errors** naming both sides;
5. emits the `prost::Message` impl (`encode_raw`, `merge_field`,
   `encoded_len`, `clear`) built from `prost::encoding::*` helpers, plus a
   `ProtoField` impl so the type composes as a field of other messages;
6. emits the staleness tripwire:
   `const _: () = assert!(crate::__schema::DESCRIPTOR_FINGERPRINT == <seen>);`
   — if any caching layer ever replays an expansion against a newer
   descriptor, compilation fails instead of silently drifting.

Point 6 is what makes the hybrid sound: descriptor *reads* happen in the
macro (best diagnostics, real inference), descriptor *invalidation* is
anchored in build.rs + `include!` (rustc-native tracking), and the
fingerprint proves the two agree.

#### Field dispatch: the `ProtoField` trait

Wire representation is chosen by the type system, not guessed from syntax:

```rust
pub trait ProtoField {
    const KIND: FieldKind;                       // checked against descriptor
    fn encode(tag: u32, value: &Self, buf: &mut impl BufMut);
    fn merge(wire: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<…>;
    fn encoded_len(tag: u32, value: &Self) -> usize;
}
```

Implementations: scalars, `String`, `bytes::Bytes`, `Vec<T>` (repeated,
packed where the proto says so), `HashMap<K, V>`, `Option<T>` (presence),
`prost_types::{Timestamp, Duration}` (message fields, kept as public API
types), and — emitted by the derives themselves — every armonik message and
enum type. No blanket impls, so no coherence hazards.

#### Attribute vocabulary

Everything not listed here is inferred from the descriptor.

| Attribute | On | Meaning |
|---|---|---|
| `message = "full.proto.Name"` | type | proto message to validate/encode against; repeatable for unified types (`TaskOptionField` validates against both `sessions.` and `tasks.` variants) |
| `enum = "full.proto.Name"` | type | proto enum for `derive(Enum)`; variants matched by name (prefix-stripped PascalCase, as prost does) |
| `rename = "id"` | field / variant | proto name differs from Rust name |
| `oneof = "type"` | type (enum) | the enum stands for one oneof of a larger message, embedded in a struct (without it, an enum stands for the whole single-oneof message); variants matched against oneof members |
| `present` | variant | oneof variant carried by presence alone (e.g. `DataChunk::Complete` ⇔ `dataComplete: true`) |
| `transparent` | type | single-field message flattened into its field's type (message `TaskOptionField { field: enum }` ⇔ Rust enum) |
| `with = "path::to::module"` | field | custom codec — see §3.5 |
| `tag = N` | field | optional; validated against the descriptor rather than trusted |

### 3.4 Enums: the merged `Other` variant

proto3 enums are open; today unknown wire values collapse to
`Unspecified` (lossy on re-serialization). New shape, generated by
`derive(Enum)`:

```rust
#[derive(armonik::Enum, …)]
#[armonik(enum = "armonik.api.grpc.v1.task_status.TaskStatus")]
pub enum TaskStatus {
    Creating,              // = 1, names matched against the proto
    Submitted,             // = 2
    …
    Retried,               // = 11
    /// Unspecified (0) or a value unknown to this crate version.
    Other(OtherTaskStatus),
}

impl TaskStatus {
    /// Matchable in patterns: `TaskStatus::UNSPECIFIED => …`
    pub const UNSPECIFIED: Self = Self::Other(OtherTaskStatus(0));
}
```

- **One catch-all arm** covers "I don't know this" — unspecified and unknown
  are handled identically by application code anyway.
- **Lossless**: the raw value round-trips through decode/encode.
- **Opaque payload**: `OtherTaskStatus` has a private field, so `Other` can
  only be constructed by `From<i32>`/decoding, which normalizes known values
  to their named variants. The invariant "no known value ever hides inside
  `Other`" is compiler-enforced, keeping derived `PartialEq`/`Hash`
  semantically correct. Raw access via `.value() -> i32`.
- `UNSPECIFIED` is generated only when the proto defines a value 0 with no
  more specific name in the Rust enum; if 0 is a named value (e.g. an
  operator enum whose 0 is `Equal`), `Other` simply never holds 0.
- `Default` is implemented manually (`Self::UNSPECIFIED`, or the proto's 0
  value).
- Size is 8 bytes (discriminant + payload) instead of 4. Same-layout-as-`i32`
  with a payload variant is not expressible on stable Rust (niche-range
  control / pattern types are unstable); since decode now constructs enum
  values directly — no `Vec<i32>` → `Vec<enum>` passes remain — the only cost
  is memory density in small repeated fields. Revisit if pattern types
  (RFC 3628) stabilize.
- Casts: `as i32` no longer works on a dataful enum; `From<TaskStatus> for
  i32` and `From<i32> for TaskStatus` are generated.

### 3.5 Non-standard formats: `with` adapters

serde-style escape hatch for fields whose Rust representation is not the
proto shape. Concrete case: `GetOwnerTaskIdResponse.result_task` is
`repeated MapResultTask { result_id = 1, task_id = 2 }` on the wire but
`HashMap<String, String>` in the API.

```rust
#[derive(armonik::Message)]
#[armonik(message = "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse")]
pub struct Response {
    #[armonik(with = "adapters::pair_map")]
    pub result_task: HashMap<String, String>,
    pub session_id: String,
}
```

The module provides `encode`/`merge`/`encoded_len` (tag still supplied from
the descriptor). The crate ships generic building blocks (`pair_map` over
key-tag/value-tag) so a new adapter is a few lines. `with` fields skip the
kind check — the adapter is asserting a non-standard mapping on purpose —
and are covered by the differential harness instead.

### 3.6 Presence semantics

> **Zero-default invariant (supersedes the earlier addendum).** Every
> type's `Default::default()` IS the proto zero value, so decoding simply
> seeds from `Default` and "absent = default" holds with no further rules.
> The harness enforces it: decoding an empty message must yield
> `Default::default()` for every registered type
> (`empty_message_decodes_to_default`). The historical non-zero defaults
> (infinite `max_duration`, `priority = 1`, 80 KiB chunks, `page_size =
> 100`, the `SessionId`/`TaskId`/`ResultId`/`Asc` sort defaults) are gone
> from `Default` — `TaskOptions::recommended()` and the exported
> `INFINITE_DURATION` carry the useful ones. This removed the
> `wire_default()` trait method, the reset-if-seed merge guards, the
> keeps-api-default rule and the generic-mode runtime seed decisions, plus
> most harness projections.
>
> The projections that remain are *representation* facts, not defaults:
> marker oneof members forget their payload (explicit `false` re-encodes
> as `true`); oneofs whose Rust default is a member variant re-encode an
> absent oneof with that member present (there is no `None` state);
> transparent wrapper enums and multi-member field oneofs always emit, so
> values containing them are never wire-empty; pair-map fields lose entry
> order and collapse duplicate keys; `tasks::Output` folds `success =
> true` over any error message, and its `Default` is `Error("")` — the
> proto zero — so an absent output is an empty *error* (the old
> conversion said success; the wire semantics of `TaskDetailed.Output`
> agree with the new reading).

- Non-`Option` message field ("absent = default"): decode merges in place,
  absence leaves the default; encode **skips the field when the nested
  `encoded_len() == 0`**. This presence gate costs one extra `encoded_len()`
  walk of the nested message on encode (the field is written only when
  non-empty; when written, prost recomputes the length for the varint prefix).
  The gated fields are shallow top-level wrappers, so the cost is a handful of
  extra traversals per call, not an asymptotic one. An all-default message
  encodes to zero bytes, making "empty encoding" ⇔ "default value". These
  fields were chosen
  precisely because absent and default are semantically indistinguishable.
- `Option<T>` fields (presence-meaningful, e.g. `TaskOptions` on task
  submission where `None` inherits session defaults): `None` omits,
  `Some(default)` **is** emitted. Exact presence preserved.
- proto3 explicit `optional` scalars must be `Option` in Rust — validated by
  the derive.
- Flattened oneofs: decode with no variant set → the default/`Invalid`
  variant; encoding the default variant emits nothing (unchanged behavior).
- Unknown *fields* are skipped on decode and not preserved on re-encode —
  same as prost and as today. Only unknown enum *values* become lossless.

### 3.7 Performance

- The decode-then-convert double pass disappears on both directions of every
  call, and with it the re-collection of vectors (`Vec<i32>` ⇔ `Vec<enum>`,
  nested filter vecs) and map rebuilds.
- All proto `bytes` fields become `bytes::Bytes`: decoding slices the tonic
  receive buffer instead of copying (`DataChunk::Data`, result
  upload/download chunks, task payloads); encoding clones cheaply.
  (`bytes` needs its `serde` feature when armonik's `serde` feature is on.)
- `encoded_len` recomputation for nested messages matches prost's own
  behavior (recomputed per level); filter trees are shallow, no caching
  needed.
- `benches/` are kept and run before/after on the branch; results recorded in
  the PR description.

## 4. Validation & testing

Two layers — independent for the derived majority, with one caveat called out
below for the two hand-written cross-field impls:

1. **Compile time** (derive vs descriptor): wrong/missing/extra fields, kind
   mismatches, presence-rule violations, enum variant name/value mismatches —
   all spanned errors; fingerprint tripwire against stale expansions.
2. **Differential round-trip harness** (test-time, generic — no per-type test
   code):
   - `protox` compiles the protos in-test; `prost-reflect` generates
     randomized `DynamicMessage`s per descriptor;
   - round-trip both directions: dynamic → bytes → armonik type → bytes →
     dynamic, compared semantically (field-set equality, not byte equality —
     empty nested messages may legitimately disappear per §3.6);
   - the equivalence classes are **computed, not restated**: each type's
     `default_encoding` (what it emits for "nothing") is folded into
     messages where those fields are absent, deriving the default-member
     and always-emit projections from the implementation itself; the
     value-level projections live in a `Normalize` impl per type, generated
     from the same constructs that shape the codec — each `with` adapter
     declares its own loss (`PairMap` order/duplicates), `present` markers
     and `transparent` chains emit theirs. For the two hand-written
     cross-field impls (`tasks::Output`, `agent::notify_result_data`) the
     `Normalize` projection is co-authored with the codec, so the harness is
     **not** an independent oracle there: a shared wrong belief about the wire
     contract would pass both sides. Those two are pinned instead by
     checked-in unit fixtures built and encoded through prost-derived
     reference messages (an independent codec), covering the cross-field
     combinations the field-information ratchet cannot reach probing one
     field at a time — `{ success, error }` both set, multi-pair differing
     session ids. Registration still requires the impl, so a projection is
     never forgotten;
   - the proto-to-type mapping is **self-registering**: each derive (and
     hand-written impl) pushes its entry into a `linkme` distributed slice
     under the private `_differential` feature, enabled only through the
     self dev-dependency — new messages are covered with zero harness
     changes, and only the generic instantiations are hand-listed;
   - a coverage test iterates *all* messages in the descriptor pool and
     fails on any message with no registered Rust type (explicit allowlist
     for intentionally flattened ones);
   - because the projections are generated from the implementation's own
     declarations, a **field-information ratchet** guards the quotient
     itself: every field of every registered message must have a probe
     value that survives normalization distinguishably (all declared enum
     values are tried, plus an unknown one) and round-trips one field at a
     time; a field the quotient erases entirely must be justified in an
     explicit allowlist — one entry exists today (`v1.Output.ok`, the zero
     value itself);
   - covers `with` adapters, generics (`FilterStatus<T>` instantiations
     checked against each service's concrete proto), and unified types.
3. Existing integration tests (`tests/*.rs`, client vs in-process server)
   continue to pass unchanged in spirit — they exercise the public API, which
   keeps its shape.

## 5. Migration plan (big-bang branch)

Order of work on the branch; each step compiles and is separately reviewable
even though the branch lands as one unit:

1. **`armonik-macros` crate**: `ProtoField` trait + impls, `derive(Enum)`,
   `derive(Message)` (structs, oneof-flattened enums, `transparent`,
   `with`), descriptor loading + fingerprint. Unit tests with a small fixture
   proto.
2. **Build pipeline**: switch `build.rs` to `protox`; emit
   `descriptor.bin` + `schema_meta.rs`; stubs via `compile_fds`. (`protoc`
   removed from CI images and docs.)
3. **Differential harness** landed early, running against whatever is
   already derived — it is the safety net for everything after.
4. **Annotate `objects/`** service by service (enums and shared objects
   first, then per-service modules): add derives/attributes, delete the
   type's `From` impls as it becomes self-encoding.
5. **`extern_path` the RPC types**; simplify `client/*` (drop conversion in
   `GrpcCall` paths) and `server/*` (`impl_trait_methods!` loses its
   conversion layer — traits and stubs now speak the same types).
6. **Delete** `impl_convert!` and the manual `IntoRequest` impls (tonic's
   blanket impl covers the native types); internalize `api/v3.rs` (stubs +
   `Empty` remain, `pub(crate)`). `IntoCollection` stays: the client
   conveniences use it independently of the conversion layer.
7. **Polish**: serde feature audit (`bytes/serde`; `Other` variants change
   the serde shape of formerly-unit enums), README/docs, CHANGELOG, version
   bump, release pipeline extended to publish both crates version-locked.
8. **Benches** re-run; numbers in the PR.

## 6. Public API changes (breaking, accepted)

- `armonik::api::v3` removed entirely (goal of the revamp). What remains
  of the generation is exactly the tonic client/server stubs, exposed per
  service as `armonik::client::<service>::stub` and
  `armonik::server::<service>::stub` — presentable as public API because
  every message in their signatures is an armonik type (the five
  `Empty`-signature RPCs each speak their own wire-compatible `{}` type),
  and the leftovers are pruned from generation.
- The message types moved to the new `armonik-types` crate (see §3.1), which
  `armonik` re-exports wholesale: `armonik::applications::Raw`,
  `armonik::TaskOptions`, etc. keep resolving, so this is source-compatible.
  Downstream that wants only the types (no tonic graph) can depend on
  `armonik-types` directly.
- Client-streaming calls have their own entry point, `client.call_streaming(
  stream)`, separate from the unary `client.call(request)`. They must be
  separate methods: moving the request types to `armonik-types` made them
  foreign to the client crate, so the compiler can no longer prove a unary
  request type is not a `Stream`, and a single `call` accepting both would make
  the streaming and unary `GrpcCall` impls overlap. `call_streaming` dispatches
  to `GrpcCallStream` directly. The named methods (`Agent::create_tasks`,
  `Results::upload`, `Submitter::create_large_tasks`) are unchanged — they
  delegate to it.
- Rust types sharing one wire message stay distinct and convert at the
  stub boundary (`tasks::list_detailed`, the agent data RPCs, the
  submitter request wrappers), so `client.call(...)` dispatch is
  unchanged.
- `sessions::RawField` wire fix: the old crate encoded the Rust
  discriminants for ClientSubmission/WorkerSubmission/ClosedAt/PurgedAt/
  DeletedAt, which disagreed with `SessionRawEnumField`; values are now
  matched by name against the descriptor.
- Enums: dataful `Other(…)` variant replaces `Unspecified` unit variants
  (`UNSPECIFIED` const provided); `as i32` casts replaced by `From` impls;
  `#[repr(i32)]` gone; matches need an `Other`/catch-all arm.
- All `bytes` payload fields: `Vec<u8>` → `bytes::Bytes`.
- `Default::default()` is the proto zero value for every type (the
  zero-default invariant): `TaskOptions`, `Configuration`, the list
  requests and the sort/field enums lose their historical non-zero
  defaults (`TaskOptions::recommended()` and `INFINITE_DURATION` replace
  them); `tasks::Output::default()` is `Error("")`, and an absent task
  output now reads as an empty error rather than success.
- serde representation shifts for the affected enums and `Bytes` fields.
- `prost_types::{Duration, Timestamp}` remain the public time types
  (unchanged).
- Everything else keeps its current shape and module paths.

## 7. Risks & mitigations

| Risk | Mitigation |
|---|---|
| `prost::encoding` is public but evolves across prost majors | pinned prost 0.14; helpers are the same surface prost-derive expands to, so breakage tracks prost upgrades we'd do deliberately |
| Derive reads a file at expansion (rust-analyzer, exotic caches) | single build-produced artifact, `OnceLock`-cached; fingerprint const-assert turns any staleness into a compile error; clear diagnostic when build scripts haven't run |
| Descriptor/kind matching bugs in the macro | differential harness fuzzes every message both directions against `DynamicMessage` ground truth |
| Generic filter types can't name one proto message | explicit field attributes on those ~5 types; per-instantiation coverage in the harness |
| Two-crate release (`armonik-macros`) | `=version` pin; release CI publishes macros first, then armonik |
| Big-bang review size | branch structured in the step order above; harness green from step 3 onward |

## 8. Future work

- Pattern types (RFC 3628) would allow `Other(i32 is …)` niches → 4-byte
  enums; revisit when stable.
- `deprecated` submitter service could be dropped instead of migrated if its
  removal is scheduled anyway.
- Zero-copy strings (e.g. `Bytes`-backed) if profiling ever shows String
  allocation as a hotspot; not worth the API noise today.
