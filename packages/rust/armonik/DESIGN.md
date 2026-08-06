# Design: direct wire implementation for the ergonomic API types

Status: **implemented** (branch `rust/direct-message-impls`)
Target: big-bang branch, released as a breaking beta bump.

> This document has two parts: Part I (this design) made the *messages*
> descriptor-driven; Part II (below, formerly `DESIGN-rpc.md`) did the same to
> the *calls*, dropping the stub codegen entirely. Part II supersedes Part I
> wherever they disagree (crate layout, build pipeline, stub surface).

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
| Message-field presence | Non-`Option` message fields: decode absent as default, **always written on encode** (a default nested message goes out as an empty one; no default is ever skipped); `Option` fields keep exact presence |
| `bytes` fields | `bytes::Bytes` everywhere (zero-copy decode from tonic buffers) |
| Migration | Big-bang branch; `main` untouched until the branch lands |
| Validation | Compile-time (derive vs descriptor) + generic differential round-trip harness (`prost-reflect` `DynamicMessage`) |
| protoc | Dropped; `protox` makes the build pure Rust |

## 3. Architecture

### 3.1 Crate layout

> **Superseded by Part II §3.1**: `armonik-types` merged into `armonik` once
> the extern-map harvest (its reason to exist) was deleted along with the stub
> codegen. The final layout is `armonik-macros` / `armonik` /
> `armonik-transport`, one build script, one `=` pin
> (`armonik` → `armonik-macros`). The historical rationale below is kept for
> the record.

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

> **Superseded by Part II §3.2**: there is no stub generation any more —
> `armonik/build.rs` only compiles the descriptor (protox) and writes
> `descriptor.bin` + the fingerprint anchor. Everything below describing the
> pruning/extern pipeline is historical.

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

> **Superseded in one respect**: the two derives later became the attribute
> macros `#[armonik_macros::message]` / `#[armonik_macros::enumeration]`, so
> the item can be re-emitted with the proto documentation injected (type,
> fields, oneof variants, enum values — the same harvest `service!` does for
> services); the hand-transcribed doc comments were deleted from `objects/`.
> Everything else below still holds; the `#[armonik(...)]` grammar is
> unchanged (and now stripped from the re-emitted item).

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
3. pulls tag, wire kind and cardinality from the descriptor — nothing is
   duplicated in the source (packedness is decided by the Rust element
   type's `ProtoField` impl);
4. validates: unknown field, type/kind mismatch, missing proto field
   (completeness), proto3 explicit-`optional` scalar not mapped to `Option` —
   all **spanned compile errors** naming both sides;
5. emits the `prost::Message` impl (`encode_raw`, `merge_field`,
   `encoded_len`, `clear`) built from `prost::encoding::*` helpers, plus a
   one-line `Msg` impl — picked up by the codec's blanket message-kind
   `ProtoField` impl — so the type composes as a field of other messages;
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
pub(crate) trait ProtoField: Default {           // `Default` = the decode seed;
                                                 // nothing compares against it
    const SHAPE: Shape;                          // kind/cardinality/names/map — one
                                                 // derive-emitted assert per field
                                                 // checks it against the descriptor
    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut);
    fn merge_field(wire: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<…>;
    fn encoded_len_field(tag: u32, value: &Self) -> usize;
    // + repeated forms with unpacked defaults
}
```

Concrete implementations: scalars, `String`, `bytes::Bytes`, `Vec<T>`
(repeated, packed for the numeric kinds), `HashMap<K, V>`, `Option<T>`
(presence), and plain proto enums (emitted by `derive(Enum)`). Every
message-shaped type — derived messages, transparent wrapper enums, the
well-known types — instead carries a one-line `Msg` marker impl
(just `NAMES`), and a single blanket
`impl<T: Msg> ProtoField for T` frames it through `prost::encoding::message`.
The blanket is coherence-safe because `Msg` is crate-local: rustc knows the
concrete impls on foreign types can never overlap it. A type implements
`Msg` XOR a concrete `ProtoField` (a second blanket for enums would be
E0119, which is why plain enums keep concrete impls).

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
    #[armonik(with = "crate::codec::adapters::PairMap<1, 2>")]
    pub result_task: HashMap<String, String>,
    pub session_id: String,
}
```

The named type implements `ProtoAdapter<T>` (`encode_field`/`merge_field`/
`encoded_len_field`, tag still supplied from the descriptor; `ProtoField` and
`ProtoAdapter` share method names, so the emitter only switches the dispatch
prefix). The crate ships generic building blocks —
`PairMap` (delegating to prost's real-map codec) and `Wrapper<TAG>` (a
single-field wrapper message flattened to its `String`/`Vec` payload) — so
a new adapter is a few lines. `with` fields skip the shape check — the
adapter is asserting a non-standard mapping on purpose — and are covered by
the differential harness instead (each adapter also declares its value-level
loss through `normalize_dynamic`).

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
> pair-map fields lose entry order and collapse duplicate keys;
> `tasks::Output` folds `success = true` over any error message, and its
> `Default` is `Error("")` — the proto zero — so an absent output is an
> empty *error* (the old conversion said success; the wire semantics of
> `TaskDetailed.Output` agree with the new reading).

> **No default is ever skipped on encode (supersedes the presence gate).**
> The encode side has no notion of a default value: every field a type
> declares is written, whatever it holds. Zeros, empty strings and
> present-but-empty nested messages therefore go on the wire, where a proto3
> receiver reads them exactly like an absent implicit-presence field. This
> removed the whole `is_default` family (the `ProtoField`/`ProtoAdapter`
> methods and their `nondefault` encode forms, the message-kind
> "`encoded_len() == 0`" override, `Msg::ALWAYS_PRESENT` for the transparent
> wrapper enums' force-emit, the hand-rolled `if raw == 0` skips in the
> wrapper-enum helpers and the `is_empty()` guards in the two hand-written
> impls), along with the `PartialEq` supertrait it needed. Those pieces only
> existed to defend each other: skipping erased a zero wrapper enum's
> presence, so wrapper enums were force-emitted, which in turn made
> "all-default" and "encodes to zero bytes" disagree for the containing
> message. The trade is a slightly larger encoding (zero fields on the wire)
> for a codec with one rule instead of three, and it is what the derives now
> emit for singular fields, oneof members and adapters alike.

- Non-`Option` message field ("absent = default"): decode merges in place,
  absence leaves the default; encode always writes the field, so a default
  nested message goes out as an empty one. These fields were chosen precisely
  because absent and default are semantically indistinguishable, which is why
  the wire form is free to pick either.
- `Option<T>` fields (presence-meaningful, e.g. `TaskOptions` on task
  submission where `None` inherits session defaults): `None` omits,
  `Some(default)` is emitted. Exact presence preserved; this is the only
  presence knob left, and it lives in the Rust type.
- proto3 explicit `optional` scalars must be `Option` in Rust — validated by
  the derive.
- Empty containers still encode to nothing: a `Vec` writes one field per
  element and a `HashMap` one per entry, so zero elements is zero bytes with
  no default check involved. Map *entries* keep prost's map codec, which omits
  `== default` key/value subfields (the canonical entry encoding; decoders fill
  them back in), and that is where `HashMap`'s `PartialEq` bound on the value
  type comes from.
- Flattened oneofs: decode with no variant set → the default/`Invalid`
  variant; the active member is written even when its payload is the default,
  and an `Invalid`/no-member variant writes nothing (there is no member to
  write, not a skipped default).
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
  needed. Dropping the presence gate also dropped its extra `encoded_len()`
  walk per nested message, at the cost of a few more bytes on the wire.
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
     absent and present-but-default nested messages are interchangeable per
     §3.6, and the encoder picks the second form);
   - the equivalence classes are **computed, not restated**: each type's
     `default_encoding` (what it emits for "nothing") is folded into
     messages where those fields are absent, deriving the default-member
     projection from the implementation itself; the
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
  stream)`, separate from the unary `client.call(request)` (a client-streaming
  call takes a `Stream<Item = R>` rather than an `R`, which no single
  signature expresses well; the historical coherence argument from the crate
  split is moot since Part II merged the crates back). The named methods
  (`Agent::create_tasks`, `Results::upload`, `Submitter::create_large_tasks`)
  are unchanged — they delegate to it.
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

---

# Part II — RPC definitions, and dropping the stub codegen

Status: **implemented** (branch `rust/direct-message-impls`; decisions settled
in review 2026-08-03, landed 2026-08-04)
Prerequisite: the direct-message revamp (Part I) landed.

> **All of §8 has landed.** The spike validated the table-driven router and
> the client dispatch on `Results` first (the full `tests/results.rs` scenario
> matrix in all three old/new combinations, plus calls against the dotnet mock
> over a real HTTP/2 connection); every later step was subtraction, verified
> against the mock (every RPC through the new dispatch) and the differential
> harness at each commit.

## 1. Motivation

`DESIGN.md` made the *messages* descriptor-driven and deleted the ~6,700-line
mirror plus its `From` conversions. It stopped one level short: the *calls* are
still hand-transcribed. 53 RPCs, each carrying a hand-written client dispatch
arm and two hand-written server list entries, all restating facts already
present in the schema or in the row next to them.

Measured on `rust/direct-message-impls`:

| Site | Lines | Information content |
|---|---|---|
| `client/*.rs` `impl_call!` + manual `GrpcCall`/`GrpcCallStream` impls | ~870 | `Request type → stub method` |
| `client/*.rs` struct + `where` bounds + `with_channel` + `call` + `call_streaming` | ~330 | service name |
| `client/mod.rs` `Client::{svc, into_svc}` pairs | ~190 | 12 service names |
| `server/*.rs` `define_trait_methods!` lists | ~120 | method name + doc comment |
| `server/*.rs` `impl_trait_methods!` lists | ~90 | stub method name; the rest restates the line above |
| `server/*.rs` `XServiceExt` trait + impl | ~145 | service name |
| `armonik/build.rs` | 344 | descriptor surgery to make `tonic-prost-build` emit armonik types |
| `client/*.rs` `#[cfg(test)] mod tests` path assertions | ~1500 | "does this RPC hit the right path" |

The semantic payload of all of it is a 53-row table of `(service, method,
kind, request, response)` plus 12 service names.

The duplication had already drifted, which is the better argument than the
line count (every item below was fixed in the housekeeping pass, then the
duplicated code itself was deleted):

- `client/results.rs`, the `import::Request` arm is the only dispatch arm in the
  crate with no span and no `Instrument` wrapper, and it is indented two spaces
  instead of four.
- `client/submitter.rs`, the `create_tasks::SmallRequest` arm opens
  `debug_span!("Submitter::create_tasks")` while the method is
  `create_small_tasks`.
- `client/submitter.rs`, `try_get_result` carries a dead `.map(|item| item)`.
- `client/events.rs`, `GrpcCall<subscribe::Request>` destructures the request
  into four locals so that `subscribe()` can rebuild the identical struct.
- `client/results.rs`, `download()` and `GrpcCall<download::Request>` are two
  copies of one body that have already diverged (one projects `.data_chunk`).
- `client/events.rs` tests are tagged `#[serial_test::serial(auth)]`;
  `client/events.rs` and `client/health_checks.rs` both document their stub
  re-export as "Service for authentication management."
- Doc comments are transcribed by hand from the protos into both the server
  trait lists and the client convenience methods, and have drifted:
  `sessions_service.proto` has "Close a session by its id.." (two periods),
  `results_service.proto` has "sorting" with no period; the Rust copies
  normalised both.

### 1.1 Complexity ledger (honest accounting)

Unlike the message revamp, this round is net-negative in hand-written code, and
it removes a mechanism rather than relocating one.

Measured on the landed branch (`a00dcd90..510538f8`, `packages/rust`): **2,248
insertions, 5,702 deletions — net −3,454 lines**. Deleted: `armonik/build.rs`
(344), `stubs.rs`, the client dispatch arms and per-service boilerplate, the
`Client` accessor pairs, the two server lists and `ServiceExt` boilerplate, the
19 `#[armonik(replace(...))]` annotations, `ReplaceSpec` + its emitter, the
build-facing half of `wire.rs`, the hand-maintained `UNEXPOSED_RPC_MESSAGES`,
and the per-RPC path tests (13 round-trips remain, §4). Added: the `service!`
proc macro with validation and doc harvest (~500), the generic router plus the
three `serve_*` bodies (~380 with docs), `ServiceClient` + `Dispatch` (~230),
the `Rpc`/`Service`/kind markers and `Channel` (~100), and the 12 invocations
(~110).

More importantly, three mechanisms disappear outright: the stub-descriptor
surgery, the `replace` substitution machinery, and the `linkme` harvest across a
build-dependency edge. What replaces them is not new infrastructure but reuse of
the descriptor-reading proc-macro that already exists.

The one thing that gets worse: the server router becomes ours. See §7.

## 2. Decisions (settled)

| Topic | Decision |
|---|---|
| Stub codegen | `tonic-prost-build` dropped entirely; no generated clients, servers or routers |
| tonic runtime | **Kept.** `tonic::client::Grpc` and `tonic::server::Grpc` do the protocol work |
| Crate layout | `armonik-types` folds into `armonik`; three crates become `armonik-macros`, `armonik`, `armonik-transport` |
| Pure-types dependency | Dropped. The split was technical, not driven by a consumer; `tonic` stays a plain dependency |
| RPC definitions | One `service! { ... }` proc-macro invocation per service, in that service's own file |
| RPC identity source | Written in the invocation, validated against the descriptor at expansion |
| `#[armonik(replace(...))]` | Deleted; its only consumer was stub-signature rewriting |
| Registry (`linkme`) | Test-only; `_registry` folds into `_differential` |
| Client dispatch | One `ServiceClient<Svc, T>`; `call` deduces the RPC from the request type |
| `GrpcCall` / `GrpcCallStream` | Deleted, replaced by `Rpc` + kind dispatch |
| Server trait | Generated by `service!` from the same invocation |
| Router shape | One generic `tower::Service` + `NamedService` over per-service const route tables emitted by `service!`; match-emission is the spike fallback (§3.6) |
| Convenience methods | Kept hand-written, bodies unchanged; no `Request::new` emitter. Compression re-evaluated after the cutover, with numbers (§3.7, §9) |
| Doc comments | Harvested by `service!` from the descriptor's `SourceCodeInfo` (build retains it); the invocation carries no prose (§3.3) |
| `unexposed(...)` | Declared in the invocation; the macro emits the differential harness's message allowlist from it, retiring the hand-maintained `UNEXPOSED_RPC_MESSAGES` |
| `tonic-prost` | Kept: it provides `ProstCodec`. Only `tonic-prost-build` is dropped |
| Tests | Per-service round-trips through one convenience method (12) plus one per streaming kind (3) replace the per-RPC path tests (§4) |
| `tonic::Status` in public signatures | Kept for this branch; REST is not on the roadmap |
| Span names | Not load-bearing (confirmed): fixed client callsite with `rpc` / `otel.name` fields; server keeps per-RPC literals, free in the emitted route closures (§3.6, §7) |

## 3. Architecture

### 3.1 Crate layout

```
packages/rust/
  armonik-macros/     # proc macros: derive(Message), derive(Enum), service!
                      #   deps: syn, quote, prost (descriptor decode only)
  armonik/            # everything else: messages, codec, RPC definitions,
                      #   client, server, router. One build script.
  armonik-transport/  # unchanged: config parsing, TLS, the connection
```

`armonik-types` folds into `armonik` wholesale. Its stated reasons to exist
(Part I §3.1) were a pure-types dependency and, "the reason the split earns
its keep", the extern-map harvest across the build-dependency edge. The harvest
is deleted by §3.2, and no consumer of the pure-types build exists in the
repository. By Part I's own test the split stops earning its keep.

Consequences worth taking while merging:

- `objects/` goes back to private with the flat `pub use` re-exports as the only
  surface. It is currently `pub` + `#[doc(hidden)]` solely so the paths
  registered into `wire::REGISTRY` resolve from the `armonik` crate.
- `Msg` and the rest of `codec` stay `pub(crate)`; the const-asserts of §3.3 no
  longer need to cross a crate boundary.
- The `=` pin chain shortens from two links to one; `scripts/versions/` loses a
  crate. `armonik-transport` stays independent and version-unlocked.
- `armonik-macros` now emits `crate::` paths into exactly one consumer, so the
  "internal-use derives" invariant becomes uniformly true.

### 3.2 Build pipeline

One build script, `armonik/build.rs`, which is `armonik-types/build.rs` moved
verbatim:

1. `protox` compiles `protos/V1/*.proto` into a `FileDescriptorSet`, retaining
   `SourceCodeInfo` (the leading comments feed the doc harvest, §3.3).
2. Write `$OUT_DIR/descriptor.bin` (read by all three macros) and
   `$OUT_DIR/schema_meta.rs` with `DESCRIPTOR_FINGERPRINT`, pulled in by
   `include!` so rustc's dep-info tracks it.

The old `armonik/build.rs` is deleted in full: `PRUNED_METHODS`,
`referenced_by_rpc`, `prune_for_stubs`, `prune_methods`, `apply_replacements`,
`prune_messages`, `guard_all_messages_externed`, `guard_unique_extern`, the
extern-map assembly and the `tonic_prost_build` invocation. With no codegen
there is no descriptor to prune, no signature to rewrite, and no extern map to
harvest.

The asymmetry that justifies this: field tags and wire kinds are **not** in the
Rust source, so the message derives need the descriptor as *input*. RPC identity
**is** in the source, so `service!` needs the descriptor only as an *oracle*.
Input belongs in a build script; an oracle can be a compile-time check.

### 3.3 The `service!` macro

One invocation per service, in that service's own file, so RPC definitions are
scoped where they belong rather than collected in a central array:

```rust
// armonik/src/rpc/results.rs
crate::rpc::service! {
    Results in crate::results @ "armonik.api.grpc.v1.results.Results";
    unexposed(WatchResults);

    rpc ListResults(list::Request) -> list::Response;
    rpc GetOwnerTaskId(get_owner_task_id::Request) -> get_owner_task_id::Response;
    rpc DownloadResultData(download::Request) -> stream download::Response;
    rpc UploadResultData(stream upload::Request) -> upload::Response;
    rpc GetServiceConfiguration(get_service_configuration::Request)
        -> get_service_configuration::Response;
}
```

`stream` sits where proto puts it, on the request or response position. It is
schema syntax, not a config field, and a reader looks for it there.

**Doc comments are harvested, not transcribed.** The invocation carries no
prose: `service!` reads the leading comments from the descriptor's
`SourceCodeInfo` (retained by the build, §3.2) and emits `#[doc]` on the
service marker, the server trait and its methods. The two hand-transcribed
copies that drifted (§1) become uncopyable.

The ergonomic method name (server trait method, telemetry label) is the module
segment of the request path (`list`, `get_owner_task_id`, `download`), the same
convention `define_trait_methods!` already relies on. The one place that
convention collides is `submitter::create_tasks::{SmallRequest, LargeRequest}`,
which share a module; an `as` override names them:

```rust
rpc CreateSmallTasks(create_tasks::SmallRequest) -> create_tasks::Response as create_small_tasks;
rpc CreateLargeTasks(stream create_tasks::LargeRequest) -> create_tasks::Response as create_large_tasks;
```

Request and response are both written out, which handles the remaining
exceptions with no further syntax: both `create_tasks` requests naming
`create_tasks::Response`, and `tasks::list_detailed::Request` breaking the
module convention.

**Invariant: request types are injective over RPCs.** `impl Rpc for
<module>::Request` requires every RPC to have a globally unique Rust request
type — across services too, since `Rpc::Service` is part of the impl. This
already holds: where the proto shares one message across RPCs (`Empty`,
`TaskFilter`, `ListTasksRequest`, `DataRequest`), the crate already gives each
site a distinct wire-compatible struct (the former `replace` types, which
outlive the `replace` machinery). A future RPC reusing an existing request type
fails as E0119 on the duplicate impl; together with the completeness check
below, the invariant is compiler-enforced.

**What one invocation emits.** Feature gates go inside the macro, not around the
invocation:

- ungated: `pub struct Results;` and `impl Service for Results`
- ungated: one `impl Rpc for <module>::Request` per line
- `_gen-server`: the `ResultsService` trait with the harvested doc comments
  attached, `ResultsServiceExt`, and the `Routes` table the generic router
  consumes (§3.6)
- `_gen-client`: `pub type Results<T> = ServiceClient<services::Results, T>`

The `Rpc` impls must be **unconditional**. A server-only build (`server` enables
`_gen-server` without `_gen-client`) has no client module, and the router
resolves everything through `Rpc`.

The header string is the **full proto service name** (not just the package):
the marker ident is the Rust-facing name, and the two are not always equal
(`Auth` vs `Authentication`, `HealthChecks` vs `HealthChecksService`), so the
proto side is spelled out and the package falls out of it.

**What it validates at expansion**, as spanned errors against `descriptor.bin`:

- the named service exists in the descriptor;
- every `rpc` names a method that exists on it;
- the `stream` keywords agree with the descriptor's `client_streaming` and
  `server_streaming` flags;
- no method is declared twice;
- every method of the service is declared, except those on an explicit
  `unexposed(...)` list in the invocation (today: `WatchResults` on `Results`
  and on `Submitter`).

That last one replaces `guard_all_rpcs_claimed` and the per-RPC path tests: a
forgotten RPC is a compile error naming the method, not a test failure. The
macro also resolves the unexposed methods' input and output message names from
the descriptor and emits the differential harness's message allowlist (gated on
`_differential`), retiring the hand-maintained `UNEXPOSED_RPC_MESSAGES`: one
declaration, both allowlists derived from it, no drift possible.

**What it cannot validate at expansion**, because a proc macro sees tokens and
not types: that `list::Request` actually implements `ListResultsRequest`. That
check is emitted instead, as a const assert on the codec's existing `NAMES`:

```rust
const _: () = assert!(
    crate::codec::names_contain(
        <list::Request as crate::codec::Msg>::NAMES,
        "armonik.api.grpc.v1.results.ListResultsRequest",
    ),
    "request type does not implement this RPC's input message",
);
```

One assert per side: the response (or the stream item, for server-streaming
RPCs) is checked against the method's output message the same way. Same trick
as the per-field `SHAPE` asserts, and it closes the loop: expansion checks the
*schema* facts, the const asserts check the *type* facts, and together they
cover what the deleted build-time guards covered.

### 3.4 The `Rpc` trait

Hand-written and public, so it stays documented and stable. `service!` emits
impls, never the trait.

```rust
/// A proto service. One marker type per service, emitted by `service!`.
pub trait Service {
    /// Fully-qualified proto service name.
    const NAME: &'static str;
}

/// Streaming shape. Marker types; the dispatch impls live in `client`/`server`.
pub struct Unary;
pub struct ServerStream;
pub struct ClientStream;

/// A request type that identifies exactly one RPC.
pub trait Rpc: prost::Message + Default + std::fmt::Debug + 'static {
    type Service: Service;
    type Kind;
    /// The response message, or the stream *item* for server-streaming RPCs.
    type Response: prost::Message + Default + std::fmt::Debug + 'static;

    const METHOD: &'static str;
    /// `concat!("/", package, ".", Service, "/", Method)`.
    const PATH: &'static str;
    /// Telemetry label, e.g. `"Results::list"`.
    const LABEL: &'static str;
}
```

(`Debug` is spelled out because `prost::Message` no longer implies it in
prost 0.14; the request/response tracing needs it.)

```rust
```

Everything here is `concat!` or `stringify!` over the invocation, so no
descriptor read is needed to *emit* it. The descriptor read exists only for the
validation in §3.3.

### 3.5 Client

Bundle the channel bounds first, independently useful and it shrinks every
signature downstream (the four-line `where` block appears 30+ times today):

```rust
pub trait Channel:
    tonic::client::GrpcService<
        tonic::body::Body,
        Error: Into<StdError>,
        ResponseBody: Body<Data = Bytes, Error: Into<StdError> + Send> + Send + 'static,
    >
{
}
impl<T> Channel for T where /* the same bounds */ {}
```

Associated-type bounds (stable since 1.79) express all four constraints in the
supertrait, so no helper associated type is needed. Note there is no `Clone`
supertrait: the borrowed `&mut Client<T>` channels are not `Clone` and never
were required to be.

One client struct, twelve aliases:

```rust
pub struct ServiceClient<Svc, T = tonic::transport::Channel> {
    inner: tonic::client::Grpc<T>,
    _svc: PhantomData<fn() -> Svc>,  // fn() -> Svc: Clone/Send/Sync need no Svc bound
}

impl<Svc: Service, T: Channel> ServiceClient<Svc, T> {
    pub fn with_channel(channel: T) -> Self { /* once */ }

    /// Perform a gRPC call. The RPC is deduced from the request type.
    pub async fn call<R>(&mut self, request: impl tonic::IntoRequest<R>)
        -> Result<<R::Kind as Dispatch>::Output<R>, RequestError>
    where R: Rpc<Service = Svc>, R::Kind: Dispatch
    { R::Kind::dispatch(&mut self.inner, request.into_request()).await }

    pub async fn call_streaming<S, R>(&mut self, stream: S) -> Result<R::Response, RequestError>
    where S: futures::Stream<Item = R> + Send + 'static,
          R: Rpc<Service = Svc, Kind = ClientStream>
    { /* once */ }

    // knobs that are unreachable today, because `inner` is private
    pub fn send_compressed(self, e: CompressionEncoding) -> Self { ... }
    pub fn max_decoding_message_size(self, n: usize) -> Self { ... }
}
```

Dispatch hangs off the **kind** type, with a GAT for the return shape:

```rust
pub trait Dispatch {
    type Output<R: Rpc<Kind = Self>>;
    fn dispatch<T, R>(grpc: &mut tonic::client::Grpc<T>, req: tonic::Request<R>)
        -> impl Future<Output = Result<Self::Output<R>, RequestError>> + Send
    where T: Channel + Send, R: Rpc<Kind = Self>;
}

impl Dispatch for Unary {
    type Output<R: Rpc<Kind = Self>> = R::Response;
    // one body: ready, ProstCodec, PathAndQuery::from_static(R::PATH),
    //           GrpcMethod extension, unary, into_inner, GrpcSnafu
}
impl Dispatch for ServerStream {
    type Output<R: Rpc<Kind = Self>> =
        futures::stream::BoxStream<'static, Result<R::Response, RequestError>>;
}
```

`GrpcCall` and `GrpcCallStream` are deleted. `client.call(request)` keeps its
contract and gains one: `R: Rpc<Service = Svc>` makes "you cannot call a
Sessions RPC on a Tasks client" an explicit bound rather than a property of
which impls happened to be written. `call` taking `impl IntoRequest<R>` also
makes per-call metadata and deadlines expressible, which they are not today.

Per-service files then hold only the convenience methods, as inherent impls on
the concrete alias (legal: local generic type, concrete type argument, so
`Sessions::with_channel` still resolves).

`Client::{svc, into_svc}` (24 methods, ~190 lines) becomes a twelve-line
`services! { agent => Agent, ... }` macro.

### 3.6 Server: the router

This is the only new code, and the reason for the spike in §8.

The resolution of an apparent contradiction earlier in the discussion: what
earns its keep is **`tonic::server::Grpc`**, which does the protocol work, not
the *generated* `SessionsServer`, which is twelve copies of a path match. We
delete the latter and keep the former.

One generic router, parameterised by the service marker:

```rust
pub struct Router<Svc, S> { inner: Arc<S>, _svc: PhantomData<fn() -> Svc>, config: ServerConfig }
```

Its `tower::Service` impl exists **once**, generically: it looks the path up in
a per-service route table and calls an erased handler. `service!` emits only
the table, one entry per RPC:

```rust
pub type RouteFn<S> = fn(Arc<S>, http::Request<tonic::body::Body>, ServerConfig)
    -> BoxFuture<'static, http::Response<tonic::body::Body>>;

pub trait Routes<S>: Service {
    const ROUTES: &'static [(&'static str, RouteFn<S>)];
}

// emitted by service! per service; one line per RPC:
impl<S: ResultsService> Routes<S> for services::Results {
    const ROUTES: &'static [(&'static str, RouteFn<S>)] = &[
        (<list::Request as Rpc>::PATH,
         |svc, req, cfg| Box::pin(serve_unary(
             svc, req, cfg, |s, r, ctx| s.list(r, ctx),
             tracing::debug_span!("Results::list")))),
        ...
    ];
}
```

The three `serve_*` helpers (unary, client-stream, server-stream) are the three
arms of today's `impl_trait_methods!` (~120 lines that expand once per RPC)
hoisted into three functions that exist once; they build the codec, apply the
config and the span, and dispatch into `tonic::server::Grpc::{unary,
client_streaming, server_streaming}`. The `.into()` calls in those arms are
deleted outright: since the extern'ing landed they are `Into<T> for T` no-ops.

Three facts make the table sound: non-capturing closures coerce to `fn`
pointers in const context; the `debug_span!` statics inside them name no
generic parameter, so per-RPC literal span names survive (§5.13 only forbids
*generic* names); and the `BoxFuture` costs nothing relative to today, because
tonic's generated servers already declare `type Future = BoxFuture<...>` and
box every handler. If the spike finds the table fighting the type system
anyway, the fallback is `service!` emitting a per-service `match` over the same
`serve_*` helpers: same deletions, ~30 more emitted lines per service.

What the one generic impl must reproduce from the generated code: `poll_ready`,
UNIMPLEMENTED for unmatched paths, content-type handling, `NamedService` (via
`Svc::NAME`, so `tonic::transport::Server::add_service` still accepts it), and
`accept_compressed` / `send_compressed` / `max_{de,en}coding_message_size`
passthrough.

Because `stream` tells `service!` the shape of each RPC, it can emit the
streaming trait signatures itself (`impl Stream<Item = Result<T, Status>>` in
argument or return position). The five hand-shaped signatures currently living
after the `---` in `define_trait_methods!`, and the `---` escape hatch itself,
go away.

### 3.7 The convenience layer is fully derived from the request structs

The convenience methods have **zero per-method source**: `service!` emits
them, and their *signatures* come from the request structs' own fields — the
one place the Rust types (not the proto types, §5.8) are visible. Parameters
mirror the fields in declaration order (field order is wire-irrelevant, so
structs are reordered where ergonomics demand), widened by a sugar class the
derive infers from each field's type:

| field type | parameter | conversion |
|---|---|---|
| `String`, `Bytes`, `Vec<u8>` | `impl Into<…>` | `.into()` |
| `Vec<T>` | `impl IntoIterator<Item = impl Into<T>>` | `.into_collect()` |
| `HashMap<K, V>` | `impl IntoIterator<Item = (impl Into<K>, impl Into<V>)>` | pair map |
| `filter::Or` | nested `impl IntoIterator` of `filter::Field` | `into_filters(…)` |
| anything else | itself | moved |

The mechanism is a cross-macro handshake: `#[derive(Message)]` emits, next to
each struct, a `__armonik_fields_*` callback macro (field names + sugar
classes, CPS-style) and flat `__armonik_ty_*` aliases (so another module can
name field and element types without transporting relative-path tokens);
`service!` emits an invocation of the request's callback continued into the
`__emit_convenience` proc macro, which builds the method. Method docs are the
harvested proto comments.

The *response* side follows one rule with per-RPC overrides on the rpc line:
a single-field response projects that field automatically, a multi-field one
returns whole; `=> field` forces a projection, `=> *` forces whole, `=> ()`
discards. For `auto`, `__emit_convenience` chains once through the *response*
type's callback to count its fields.

Both sides mangle the reflection's names from the type path written on the rpc
line, which an **alias** of a message breaks: `pub type Response = Count;` has
no reflection of its own (the derive emitted it next to `Count`, under the
`count` stem). `#[armonik_macros::reflect]` on the alias carries it over, as
renaming re-exports of the callback and the field aliases under the alias's own
stem, the field names coming from one chain through the source callback into
`__emit_reflect`. Unlike the convenience emission, it expands *in the alias's
module*, so the alias's own relative right-hand side is all it needs; the
reflection is looked up in the module named after the aliased type
(`super::super::Count` in `super::super::count`, the one-object-per-file
convention). It carries `submitter::{count_tasks, wait_for_completion}`, whose
`Response` is `Count` and whose methods project its `values` map as they did
before the revamp. Generic aliases (the per-service `Sort`/`FilterStatus`
instantiations) have no reflection to carry and are rejected.

**Opt-out**: `manual` on the rpc line emits nothing — the escape for custom
wiring or a wrong mechanical default. Client-streaming RPCs are always manual
(their entry point is `call_streaming`). Today that leaves six hand-written
methods (`results::upload`, `worker::process` — nine exploded parameters is a
wrong default — `submitter::{create_small_tasks, create_large_tasks,
try_get_task_output}`, `agent::create_tasks`); the hand-written
`notify_result_data::Request` carries hand-written reflection to stay
generated.

Everything is type-checked post-expansion, so a wrong sugar inference is a
compile error, never a wire bug; behaviorally, every generated method is
covered per method by the in-process integration suites. Accepted DX shift:
parameter names and order are now mechanically the field names and order
(e.g. `results::get` takes `id`, the agent methods take `communication_token`).

An earlier draft had `#[derive(Message)]` emit a `Request::new(..)`
constructor instead, and a first implementation generated only the bodies
under hand-written signatures; both were dropped for this design, which is
what actually deletes the layer. The nested-filter collect shared by the
`list` methods survives as the `into_filters` helper the emission calls.

### 3.8 What the registry is for now

`linkme` and the `wire` module survive, test-only. `_registry` folds into
`_differential` and `linkme` becomes a dev-dependency.

- kept, because the differential coverage ratchet consumes them: `REGISTRY`,
  `Registration`, `Role::{Message, Absorbed}`, `absorbed()`, `DESCRIPTOR`,
  `Diff`; `UNEXPOSED_RPC_MESSAGES` survives as a name the harness consumes but
  is now emitted by `service!` from the `unexposed(...)` declarations (§3.3)
  rather than hand-maintained;
- deleted, because their only consumer was `armonik/build.rs`: `Direction`,
  `Replacement`, `Role::Replace`, `replacements()`, `extern_mapping()`.

The RPC side needs no `linkme` at all: completeness is checked at expansion
against the descriptor (§3.3), not by collecting a distributed slice.

## 4. Validation & testing

1. **Expansion time** (`service!` vs descriptor): unknown service or method,
   streaming-flag mismatch, duplicate declaration, missing RPC. Spanned.
2. **Compile time** (emitted const asserts): request and response types
   implement the descriptor's input and output messages; plus the existing
   per-field `SHAPE` asserts and the `DESCRIPTOR_FINGERPRINT` tripwire.
3. **Differential harness**: unchanged, now test-only.
4. **Integration tests** (`tests/*.rs`, client against in-process server):
   unchanged in spirit, and they become the primary evidence that the router
   behaves. `tests/results.rs` covers both streaming directions.
5. **Retired**: the ~1500 lines of per-RPC `#[cfg(test)] mod tests` path
   assertions in `client/*.rs`, subsumed by (1) and (2) — but only *after* the
   cutover, so they cover it for free while it happens. What replaces them: one
   round-trip per service through a representative convenience method (12
   tests, catching the one class the compiler cannot: a convenience method
   wired end-to-end to the wrong RPC), plus one test per streaming kind (3).

Note on features: `armonik`'s own tests need `DESCRIPTOR`, so `_differential`
must be enabled through the self dev-dependency. Cargo unifies that onto the
normal dependency in test builds, which pulls `prost-reflect` into the test
graph only.

## 5. Aborted alternatives

Recorded because several of them look obviously right until a specific fact
kills them.

### 5.1 `Rpc` on request types, in `armonik-types`, inferred from the descriptor

The first proposal: `#[derive(Message)]` scans the descriptor's services for an
RPC whose input is this message and emits `impl Rpc`.

Killed by cardinality. *Proto* request messages are not in bijection with RPCs:
`Empty` serves five, `TaskFilter` three, `ListTasksRequest` two, `DataRequest`
three. A descriptor scan cannot decide which RPC a message type stands for,
which is exactly why `replace` had to be invented in the first place. Writing
the relation down per RPC in the invocation sidesteps the inference entirely,
and the *Rust* request types are already distinct per RPC (the former `replace`
types), so the emitted impls stay coherent — see the injectivity invariant in
§3.3.

Secondary objection: a pure-types crate has no business knowing about services,
paths or streaming kinds.

### 5.2 The table in `armonik/rpcs.rs`, outside both `build.rs` and `src/`

Premised on `build.rs` needing to read the table, which is false. Every field of
a row is written down in the row; `PATH` is string concatenation; and with no
codegen there is no generated method identifier, so there is no PascalCase to
snake_case conversion either. Nothing needs the descriptor as *input* to define
an RPC, so nothing needs a build script, so the table can live in `src/`. This
realisation is what deletes `armonik/build.rs` entirely.

### 5.3 A fifth crate, `armonik-rpc`

Conceptually clean, and it would make the table a first-class typed artifact.
Rejected: a longer `=`-pinned lockstep release chain (already listed as a risk
in Part I §7) for no consumer. Superseded outright by §5.2 and by the merge.

### 5.4 A central `const RPCS: &[Def]` array

Rejected on two grounds: RPC definitions should be scoped per service, in the
service's file, and should read as Rust rather than as data fed to a generator.

### 5.5 Distributed `const RPCS` per module, plus a central `&[&[Def]]`

The obvious repair to §5.4. Rejected because the central list is a second place
to forget an entry, which is the failure mode this whole exercise removes.

### 5.6 `linkme` in `armonik` to collect distributed RPC registrations

The other repair to §5.5, and the one that follows the "registration and
implementation are the same act" principle `DESIGN.md` already states for
`Normalize`. Made unnecessary by expansion-time completeness checking (§3.3),
which is strictly better: a compile error at the invocation instead of a test
failure listing a name.

### 5.7 The convenience methods as the RPC definition

Attractive, and once §5.2 removed the build-script visibility objection it
looked viable. Killed by feature gating: the `Rpc` impls must be unconditional
because a server-only build has no client module, and convenience methods are
`_gen-client` by construction. Two supporting objections: a proc macro cannot
read another file's AST, so an annotation on a convenience method cannot see the
request struct's fields; and the server needs the raw request and response types
regardless, so deriving server glue from a client ergonomics choice is
backwards.

### 5.8 Auto-generating the convenience methods from field information

The stated motivation for merging the crates. It does not work, and merging does
not help it.

Proc macros share no state across invocations: what `#[derive(Message)]` learns
about `list::Request` is gone by the time `service!` expands, and there is no
compiler-mediated channel between them. `service!` could read the descriptor,
but the descriptor gives *proto* types, and the gap between proto types and the
Rust types is precisely what this crate exists to bridge: `repeated FiltersAnd
filters` is `filter::Or`, `TaskOptionField` is a transparent enum,
`GetOwnerTaskIdResponse.result_task` is a `PairMap` adapter. Descriptor-driven
convenience generation would be wrong exactly on the types that were hand-shaped.

Resolved by not generating the convenience layer at all (§3.7): the methods
stay hand-written, and the merge is justified by §3.1 alone.

### 5.9 Dropping the tonic runtime for pure prost / tower / hyper

Rejected. What would have to be reimplemented is not framing but the parts that
are discovered rather than designed: status in trailers including the
trailers-only case where it arrives in a HEADERS frame with END_STREAM,
`grpc-message` percent-encoding (gRPC's own variant, not RFC 3986),
`grpc-status-details-bin`, `grpc-timeout` unit-suffix encoding, compression
negotiation with its specific UNIMPLEMENTED plus `grpc-accept-encoding`
response, size limits mapping to RESOURCE_EXHAUSTED, `-bin` metadata base64,
content-type validation, HTTP/2 error code to status mapping (REFUSED_STREAM to
UNAVAILABLE and friends), and a streaming decoder that buffers partial frames
correctly.

The stakes are worse here than for a typical crate: this binding must
interoperate with the C#, Java, Python and C++ implementations against one
control plane. tonic is tested against gRPC's interop suite.

The dependency argument does not hold either. Removing tonic removes tonic;
hyper, h2, tower, tokio, prost and http are already direct dependencies, and
`armonik-transport` already owns hyper and hyper-rustls. Compile time is
dominated by `protox` and `tonic-prost-build` in build scripts, which is §3.2,
not the runtime.

### 5.10 Keeping the generated server stubs

The initial position, on the grounds that the router is non-trivial protocol
code. Refined rather than reversed: the protocol code lives in
`tonic::server::Grpc`, which we keep and call. The *generated* `XServer` is a
path match plus that call, twelve times. See §3.6.

### 5.11 Feature-gating tonic to preserve a pure-types build after the merge

Rejected. With no consumer, the no-feature configuration is an untested
configuration, and untested configurations rot. Preserving it would also mean
making `futures`, `snafu` and `tracing` optional for the same non-reason.

### 5.12 Blanket impls discriminated by `R::Kind`

```rust
impl<R: Rpc<Kind = Unary>> ClientCall for R { ... }
impl<R: Rpc<Kind = ServerStream>> ClientCall for R { ... }
```

E0119. Coherence does not reason about associated-type disequality. Hanging the
dispatch off the kind type (§3.5) has no coherence question at all: two impls on
two distinct concrete types.

### 5.13 Per-RPC span names from a generic dispatcher

`tracing::debug_span!(R::LABEL)` cannot compile: the macro expands to `static
META: Metadata` plus `static CALLSITE`, and statics cannot reference generic
parameters. Constructing `Metadata` by hand per monomorphization is possible in
principle (`Metadata::new` is const) but needs a `Callsite` and leans on
`tracing-core` internals. See §7 for the accepted consequence.

### 5.14 Inferring the streaming kind from the convenience method's return type

Rejected: a mistyped signature would silently change the wire behaviour. Written
in the invocation and checked against the descriptor, a mistake is a compile
error.

### 5.15 A terser `rpc ListResults(list);` form

Rejected: writing request and response explicitly costs one line each and
handles `create_tasks::{SmallRequest, LargeRequest}` and `list_detailed` with no
override syntax.

### 5.16 An `armonik::Status` abstraction boundary

**Deferred, not rejected.** `tonic::Status` in the server trait signatures makes
the user-facing API tonic-shaped, which would matter if a second transport
existed. REST is not on the roadmap, the crate is beta, and the RPC table leaves
exactly one conversion site per kind, so this stays a cheap follow-up. Revisit
if `feat-implements-rest-json` or `wk/feat/rust-proxy` becomes real.

## 6. Public API changes (breaking, accepted)

- `armonik::client::<service>::stub` and `armonik::server::<service>::stub` are
  removed. There are no generated stubs. This is the load-bearing break.
- `GrpcCall` and `GrpcCallStream` are removed. `client.call(request)` and
  `client.call_streaming(stream)` keep their behaviour; `call` now accepts
  `impl IntoRequest<R>`, so per-call metadata and deadlines become possible.
  `call_streaming` stays separate: the coherence argument for the split
  (Part I §6, request types foreign to the client crate) is moot after the
  merge, but a client-streaming call takes a `Stream<Item = R>` rather than an
  `R`, which no single signature expresses well. One inference wart: passing a
  bare message infers `R`, but passing a pre-built `tonic::Request` needs a
  turbofish (`call::<R>(request)`) because tonic's two blanket `IntoRequest`
  impls leave `R` ambiguous.
- `Sessions<T>` and friends become type aliases for `ServiceClient<Svc, T>`.
  `with_channel` and the convenience methods resolve unchanged; diagnostics
  mention the underlying type. `#[deprecated]` moves to the `Submitter` alias
  and should be repeated on its convenience methods for parity.
- `armonik-types` no longer exists as a crate. `armonik::results::list::Request`
  and every other path is unchanged, since `armonik` re-exported the whole
  surface already.
- `#[armonik(replace(...))]` is removed from the attribute grammar (which is
  `#[doc(hidden)]` and unsupported, so this is not a public break).
- The client's `tracing` span names change; see §7.
- `Rpc`, `Service`, `Unary`, `ServerStream`, `ClientStream`, `Dispatch` and
  `Channel` are new public API. `Rpc` should be supported and documented, unlike
  the attribute grammar.
- New public knobs on the clients: `send_compressed`, `accept_compressed`,
  `max_decoding_message_size`, `max_encoding_message_size`.

## 7. Risks & mitigations

| Risk | Mitigation |
|---|---|
| The router is ours now, and it is protocol-adjacent | Dispatch into `tonic::server::Grpc`, so framing and status handling stay tonic's. Spike the table-driven shape on `Results` against `tests/results.rs` before anything else (§8); match-emission is the in-step fallback |
| Client span names change | Resolved: confirmed not load-bearing (human debugging only; no dashboards or `EnvFilter` directives depend on them). Fixed callsite `"armonik.rpc"` plus `rpc = R::LABEL` and `otel.name = R::LABEL` fields |
| Server span names | Preserved: the emitted route closures are per-RPC callsites, so the literal names survive (§3.6) |
| `service!` reads a file at expansion | Same mechanism, same `OnceLock` cache and same fingerprint tripwire as the existing derives. Not a new class of coupling |
| Losing the two build-time extern guards | Replaced by expansion-time completeness (§3.3) and the const asserts. The duplicate-generated-struct failure `guard_all_messages_externed` prevented cannot occur without codegen |
| One larger crate, worse incrementality | Accepted. Touching a client file now recompiles the codec; ~16k lines total |
| Feature matrix consolidates into one crate | `_differential` now lives alongside the client. The self dev-dependency trick already in use keeps `prost-reflect` out of normal builds; verify under resolver v2 |
| tonic 0.14 internals (`client::Grpc`, `server::Grpc`, `GrpcMethod`) | These are public API, and they are exactly what the generated code calls. Breakage tracks tonic majors we would upgrade deliberately |

## 8. Migration plan (all landed)

Each step compiled, was separately reviewable, and was committed on its own.
One deviation from the written order: step 8 (the merge) had to follow step 7
for the reason recorded in step 8 itself.

0. **Spike** on one service, `Results` (all three call kinds), against the
   existing `tests/results.rs` and the mock server, with the generated stubs
   still in place alongside: the `Channel` bundle, hand-written
   `Rpc`/`Service`/kind markers, `Dispatch` for `Unary` and `ServerStream`,
   `ServiceClient::{call, call_streaming}`, and the table-driven router of
   §3.6. Everything after this step is subtraction; this step is the only
   addition that could fail. Do not proceed on estimate.
1. `Channel` supertrait bundle adopted across the existing code. Mechanical,
   no API change, shrinks every signature the later steps touch.
2. Housekeeping the duplication hid: the missing span on the `import` arm, the
   wrong span name on `create_small_tasks`, the dead `.map(|item| item)`, the
   `serial(auth)` tag on the events tests, the two copy-pasted stub doc
   comments, the stale `stubs.rs` header (`EXTERN_TYPES` and `PRUNED_MESSAGES`
   no longer exist), and the stale `armonik-macros/src/descriptor.rs` header
   (the descriptor is compiled by *`armonik-types`*'s build script, not
   `armonik`'s).
3. `Rpc` / `Service` / kind markers, hand-written (promoted from the spike).
   No consumer yet.
4. `service!`, emitting the marker, the `Rpc` impls and the harvested docs,
   validating against the descriptor. Lands alongside the existing client and
   server code so the two can be diffed against each other before either is
   deleted. Transition glue, deleted at step 8: `armonik/build.rs` copies
   `descriptor.bin` + the fingerprint anchor into `armonik`'s `OUT_DIR` so the
   invocations can expand there pre-merge, and `codec` is `pub` +
   `#[doc(hidden)]` so the emitted const asserts reach `Msg::NAMES` across the
   still-split crates.
5. Client: `ServiceClient` + `Dispatch`; the twelve aliases; convenience
   methods become inherent impls on the aliases with bodies unchanged; the
   `services!` accessor macro; the new compression/size knobs; `#[deprecated]`
   onto the `Submitter` alias and its methods; delete `impl_call!`, `GrpcCall`,
   `GrpcCallStream`.
6. Server: `service!` grows the trait, the one-line `Ext` and the `Routes`
   table; delete `define_trait_methods!`, `impl_trait_methods!` and the `---`
   blocks.
7. Delete `stubs.rs`, the stub-generation half of `armonik/build.rs`, the
   `tonic-prost-build` dependency (`tonic-prost` stays: it provides
   `ProstCodec`), and the `replace` machinery end to end (annotations,
   `ReplaceSpec`, emitter, `Role::Replace`, `Replacement`, `Direction`,
   `replacements()`, `extern_mapping()`).
8. Merge `armonik-types` into `armonik`: move the source and the build script,
   drop the build-dependency and the step-4 glue, fold `_registry` into
   `_differential`, make `linkme` dev-only, put `objects/` and `codec` back to
   private, retire the hand-maintained `UNEXPOSED_RPC_MESSAGES` (now emitted
   from `unexposed(...)`), update `scripts/versions/`. `armonik-transport`
   stays a separate crate. **This step must follow step 7**: until the stub
   generation dies, `armonik/build.rs` harvests the extern map from
   `armonik_types::wire::REGISTRY` across the build-dependency edge, and a
   crate cannot be its own build-dependency.
9. Extract the nested-filter helper; no other convenience-body changes.
10. Retire the per-RPC path tests; land the 12 per-service round-trips and the
    3 kind tests; run the suite against the mock before and after.
11. Docs: this document folded into `DESIGN.md` as Part II; Part I's crate
    layout, build pipeline and `call_streaming` rationale annotated as
    superseded; release pipeline down to `armonik-macros` + `armonik` pinned,
    `armonik-transport` unlocked (version bump with the branch release).

## 9. Future work

- **`armonik::Status`** (§5.16), if a second transport materialises.
- **Convenience method compression** (`macro_rules!` over the common shape), if
  the bodies turn out uniform enough to be worth its diagnostics cost. Evaluate
  after step 9, with real numbers, not before (§3.7).
- **Reflection service**, now cheap: the descriptor is embedded already, and
  nothing prunes it any more.

## 10. Resolved questions

1. **Grammar**: as in §3.3, with the `as` override for the two `create_tasks`
   methods and `stream` as a position prefix.
2. **`unexposed`**: in the invocation; the macro emits the harness allowlist
   from it (§3.3, §3.8), so the two allowlists cannot drift.
3. **Span names**: confirmed not load-bearing (human debugging only); free to
   change (§7).
4. **`armonik-transport`**: stays a separate crate. Unlike `armonik-types` the
   split costs nothing — no `=` pin in the chain, no build-dependency edge, no
   macro coupling — and it quarantines the hyper/rustls surface, with
   `wk/feat/rust-proxy` a plausible second consumer.
