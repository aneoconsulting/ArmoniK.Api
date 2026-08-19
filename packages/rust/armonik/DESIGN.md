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
  (`events::Update`, `DataChunk`, ...);
- per-rpc module layout (`sessions::list::Request`), field renames
  (`Session.id` -> `session_id`), unified types shared across services
  (`TaskOptionField`).

The goal of this revamp: **the ergonomic types implement `prost::Message`
directly and become the only representation.** The generated message structs,
the conversion layer, and the double decode/convert pass disappear.
`api::v3` is removed from the public API entirely.

### 1.1 Complexity ledger (honest accounting)

Total system complexity did not shrink, it **relocated**, and this section
says so plainly so the trade is reviewed on its merits. The ~6,700-line
hand-written mirror plus its `From` conversions are gone, but hand-maintained
*production* code across the three crates is now larger in aggregate (the
descriptor-reading proc-macro, the codec, the build pipeline, and the annotated
`objects/`), and it moved from local `From` impls a maintainer edits *next to
the type* into three places that carry complexity less locally: a proc-macro
that reads `descriptor.bin` at expansion, a two-crate build pipeline that
rewrites a `FileDescriptorSet`, and a `linkme` slice harvested across a
build-dependency edge with three `=`-pinned crates released in lockstep.

**Measured, on checked-in Rust** (`git ls-tree` line counts, `main` against the
branch as it stands):

| area | main | branch | delta |
|---|---:|---:|---:|
| `objects/` | 6,699 | 4,306 | -2,393 |
| `api/` (the generated shim) | 53 | 0 | -53 |
| `client/` + `server/` + `rpc/` | 5,282 | 2,177 | -3,105 |
| `codec/` | 0 | 1,438 | +1,438 |
| `armonik-macros/src` | 0 | 6,985 | +6,985 |
| `armonik-transport/src` | 1,274 | 1,270 | -4 |
| **production total** | **13,308** | **16,176** | **+2,868 (+22%)** |
| tests | 3,110 | 4,465 | +1,355 (+44%) |

Two things that table corrects. "The ~6,700-line hand-written mirror is gone" is
not what happened: `objects/` fell by 2,393 lines, and `api/v3.rs` was a 53-line
`include_proto!` shim, so the generated code left the *build*, not the
repository. And the amortization argument above does not survive the count. The
schema has 206 messages and is frozen; there is no stream of new ones for a
fixed cost to amortize against, so "the marginal message is near-free" is true
and irrelevant.

The argument that does survive is about what the lines *buy*, and it is not a
line count at all: tags, wire kinds and cardinalities now come from the
descriptor and disagreeing with it is a compile error; there is a round-trip
oracle where there was none; there is one representation of each message instead
of two with conversions between them; and the build needs no `protoc`. Approve
the beta break on that, not on an amortization schedule.

## 2. Decisions (settled)

| Topic | Decision |
|---|---|
| Implementation strategy | Derive proc-macro (`armonik-macros` crate), descriptor-driven |
| Descriptor access | Hybrid: `protox` in `build.rs` compiles descriptors; derive reads the compiled `descriptor.bin`; fingerprint const-assert guards staleness |
| Public API | Same shapes as today; minor breaks allowed (crate is beta on purpose) |
| Raw generated types | Removed entirely, including from tonic stub signatures (`extern_path`) |
| Unknown enum values | Single merged catch-all `Unknown(Unknown...)` dataful variant covering unspecified (0) and unknown values; opaque payload; lossless round-trip |
| Message-field presence | Non-`Option` message fields: decode absent as default, **always written on encode** (a default nested message goes out as an empty one; no default is ever skipped); `Option` fields keep exact presence |
| `bytes` fields | `bytes::Bytes` everywhere (zero-copy decode from tonic buffers) |
| Migration | Big-bang branch; `main` untouched until the branch lands |
| Validation | Compile-time (derive vs descriptor) + generic differential round-trip harness (`prost-reflect` `DynamicMessage`) |
| protoc | Dropped; `protox` makes the build pure Rust |

## 3. Architecture

### 3.1 Crate layout

> **Superseded by Part II section 3.1**: `armonik-types` merged into `armonik` once
> the extern-map harvest (its reason to exist) was deleted along with the stub
> codegen. The final layout is `armonik-macros` / `armonik` /
> `armonik-transport`, one build script, one `=` pin
> (`armonik` -> `armonik-macros`). The historical rationale below is kept for
> the record.

```
packages/rust/
  armonik/            # tonic client/server stubs + ergonomic client/server
                      #   wrappers; re-exports armonik-types wholesale
  armonik-types/      # the message types: ergonomic structs/enums implementing
                      #   prost::Message directly (objects + codec + the
                      #   differential harness); its build.rs compiles the
                      #   descriptor. A pure-types dependency, no tonic graph.
  armonik-macros/     # proc-macro crate: #[message], #[enumeration]
                      #   deps: syn, quote, prost (descriptor decode only)
```

The three crates are version-locked with `=` pins (`armonik` -> `armonik-types`
-> `armonik-macros`) and published in that order. The derives are internal-use
(they emit `crate::codec::...` paths, so they only expand inside
`armonik-types`) and `#[doc(hidden)]`: the attribute grammar is not a
supported public API.

`armonik-types` exists so downstream can depend on the wire types without the
client/server stubs and their tonic/hyper/rustls graph, and (the reason the
split earns its keep) so `armonik`'s build script can **harvest** the
proto-name -> Rust-path extern map, the per-RPC substitutions and the absorbed
(flattened-away) messages from the `#[armonik(...)]` annotations instead of
hand-maintaining them. `armonik-types` is a build-dependency of `armonik`;
every derive and the one hand-written impl register, through one `register!`
macro (the single home of the registry's layout), one `Registration` per
proto name into a single `linkme` slice (`armonik_types::wire::REGISTRY`, gated
behind the base `_registry` feature the build-dependency enables). Each entry's
`Role` is `Message { rust_path }`, `Replace(Replacement)`, or `Absorbed`;
`armonik`'s `build.rs` reads `extern_mapping()`, `replacements()` and
`absorbed()`, which filter the one slice. The map is fully harvested, with no
hand-maintained residue: entries that cannot be a plain extern mapping (the
shared `Empty`, `TaskFilter`, ... sites) carry `#[armonik(replace(...))]` and
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

> **As shipped.** Neither feature exists. `armonik-types` was never split out,
> so there is no build-dependency to gate, and the registry now lives under
> `#[cfg(test)]` with `linkme` and `prost-reflect` as plain dev-dependencies
> (section 3.8, section 11.1). This subsection describes the shape the design
> reasoned through, not the tree.

### 3.2 Build pipeline (`build.rs`)

> **Superseded by Part II section 3.2**: there is no stub generation any more;
> `armonik/build.rs` only compiles the descriptor (protox) and writes
> `descriptor.bin` + the fingerprint anchor. Everything below describing the
> pruning/extern pipeline is historical.

> **Stubs-only generation.** The descriptor set is compiled once (protox)
> and used twice: the full set becomes `descriptor.bin` for the derives and
> the differential harness, while the tonic stub generation receives a
> *pruned* copy: the never-exposed Submitter.WatchResults method is dropped
> (tonic answers UNIMPLEMENTED for unrouted paths), every message that
> nothing generated references (field wrappers, watch messages, legacy) is
> removed alongside all file-level enums, and every RPC slot carrying a
> `#[armonik(replace(...))]` type (the `Empty` sites, the shared `TaskFilter`
> / request wrappers, ...) is rewritten to a distinct synthetic message
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

1. `protox` compiles `protos/V1/*.proto` -> `FileDescriptorSet`
   (pure Rust; `protoc` no longer required; `cargo:rerun-if-changed` per
   proto file).
2. Writes `$OUT_DIR/descriptor.bin` (input of the derive) and
   `$OUT_DIR/schema_meta.rs` containing
   `pub(crate) const DESCRIPTOR_FINGERPRINT: u128 = ...;`
   (hash of the descriptor bytes). `lib.rs` pulls the latter in via
   `include!`, which puts it in rustc's dep-info, tracked by cargo, sccache,
   and any hermetic build system. The descriptor bytes are also re-exported as
   `armonik_types::wire::DESCRIPTOR`.

**`armonik/build.rs`** (with `armonik-types` as a build-dependency, so it is
compiled first and its derives have run):

3. Decodes `armonik_types::wire::DESCRIPTOR` (no proto compilation here), prunes
   it, and feeds it to `tonic-prost-build` via `compile_fds` to generate
   **service stubs only**:
   - the `extern_path` entries come from `armonik_types::wire::extern_mapping()`
     (the annotations, harvested) mapping each RPC type to its ergonomic type
     (`.armonik.api.grpc.v1.sessions.ListSessionsRequest`
     -> `::armonik_types::sessions::list::Request`), plus one entry per
     `replacements()` substitution mapping its synthetic target message to the
     standing-in type; there is no hand-maintained extern list;
   - extern'd messages are not generated at all, and since nested types never
     appear in stub signatures, **only the top-level RPC types need entries**;
   - `guard_all_messages_externed` fails the build if any top-level message
     survives pruning without an extern entry, and `guard_unique_extern` fails
     it if two Rust types claim one proto name. These are the ratchets that
     keep the harvested map honest as the schema evolves.

### 3.3 The derive

> **Superseded in one respect**: the two derives later became the attribute
> macros `#[armonik_macros::message]` / `#[armonik_macros::enumeration]`, so
> the item can be re-emitted with the proto documentation injected (type,
> fields, oneof variants, enum values, the same harvest `service!` does for
> services); the hand-transcribed doc comments were deleted from `objects/`.
> Everything else below still holds; the `#[armonik(...)]` grammar is
> unchanged (and now stripped from the re-emitted item).

`#[armonik_macros::message]` on structs and oneof-shaped enums,
`#[armonik_macros::enumeration]` on proto enums. An enum with `message = ...`
alone stands for the *whole* message: its single oneof is inferred, and any
non-oneof field of the message is a *sibling*, declared in every variant
(including the attribute-less "no member set" one) so the per-field merge
stays stateless and order-independent. A sibling whose tag falls between two
member tags is rejected: encoding writes the siblings around the member, which
for such a message has no ascending-tag spelling. `oneof = "..."` declares an enum for
one oneof of a larger message, embedded in a struct, and is rejected when
the oneof covers the whole message, keeping the two shapes visually
distinct. At expansion time the macro:

1. reads `$OUT_DIR/descriptor.bin` (env `OUT_DIR`; one prost decode, cached
   in a `OnceLock` for the whole rustc/rust-analyzer process; clear error if
   the file is missing, i.e. build scripts have not run);
2. resolves the message named by `#[armonik(message = "...")]` and matches Rust
   fields against proto fields **by name** (renames via attribute);
3. pulls tag, wire kind and cardinality from the descriptor, so nothing is
   duplicated in the source (packedness is decided by the Rust element
   type's `ProtoField` impl);
4. validates: unknown field, type/kind mismatch, missing proto field
   (completeness), proto3 explicit-`optional` scalar not mapped to `Option`:
   all **spanned compile errors** naming both sides;
5. emits the `prost::Message` impl (`encode_raw`, `merge_field`,
   `encoded_len`, `clear`) built from `prost::encoding::*` helpers, plus a
   one-line `Msg` impl, picked up by the codec's blanket message-kind
   `ProtoField` impl, so the type composes as a field of other messages;
6. emits the staleness tripwire:
   `const _: () = assert!(crate::__schema::DESCRIPTOR_FINGERPRINT == <seen>);`
   If any caching layer ever replays an expansion against a newer
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
    const SHAPE: Shape;                          // kind/cardinality/names/map: one
                                                 // derive-emitted assert per field
                                                 // checks it against the descriptor
    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut);
    fn merge_field(wire: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<...>;
    fn encoded_len_field(tag: u32, value: &Self) -> usize;
    // + repeated forms with unpacked defaults
}
```

Concrete implementations: scalars, `String`, `bytes::Bytes`, `Vec<T>`
(repeated, packed for the numeric kinds), `HashMap<K, V>`, `Option<T>`
(presence), and plain proto enums (emitted by `#[armonik_macros::enumeration]`). Every
message-shaped type (derived messages, transparent wrapper enums, the
well-known types) instead carries a one-line `Msg` marker impl
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
| `enum = "full.proto.Name"` | type | proto enum for `#[armonik_macros::enumeration]`; variants matched by name (prefix-stripped PascalCase, as prost does) |
| `rename = "id"` | field / variant | proto name differs from Rust name |
| `oneof = "type"` | type (enum) | the enum stands for one oneof of a larger message, embedded in a struct (without it, an enum stands for the whole single-oneof message); variants matched against oneof members |
| `present` | variant | oneof variant carried by presence alone (e.g. `DataChunk::Complete` <-> `dataComplete: true`) |
| `transparent` | type | single-field message flattened into its field's type (message `TaskOptionField { field: enum }` <-> Rust enum) |
| `with = "path::to::Adapter"` | field | custom codec; names a *type* implementing `ProtoAdapter<T>`, not a module (section 3.5) |
| `tag = N` | field | `generic` mode only, where it is authoritative; a descriptor-validated field takes its tag from the descriptor and spelling one was rejected as restating it |

### 3.4 Enums: the merged `Unknown` variant

proto3 enums are open, so an unknown wire value must round-trip losslessly
rather than collapse to `Unspecified`. The shape
`#[armonik_macros::enumeration]` expands:

```rust
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(enum = "armonik.api.grpc.v1.task_status.TaskStatus")]
pub enum TaskStatus {
    Creating,              // = 1, names matched against the proto
    Submitted,             // = 2
    ...
    Paused,                // = 13
    /// Unspecified (0) or a value unknown to this crate version.
    Unknown(UnknownTaskStatus),
}

impl TaskStatus {
    /// Matchable in patterns: `TaskStatus::UNSPECIFIED => ...`
    pub const UNSPECIFIED: Self = Self::Unknown(UnknownTaskStatus(0));
}
```

- **One catch-all arm** covers "I don't know this": unspecified and unknown
  are handled identically by application code anyway.
- **Lossless**: the raw value round-trips through decode/encode.
- **Opaque payload**: `UnknownTaskStatus` has a private field, so `Unknown` can
  only be constructed by `From<i32>`/decoding, which normalizes known values
  to their named variants. The invariant "no known value ever hides inside
  `Unknown`" is compiler-enforced, which is what makes `matches!(x,
  Status::Completed)` agree with `x == Status::Completed`. Raw access via
  `.value() -> i32`. Under `serde` the payload is its bare number both ways: its
  `Deserialize` runs the same `From<i32>` and keeps only what lands on the
  catch-all. A derive cannot, being generated inside the module that owns the
  private field, where any number builds a payload.
- **Coverage**: every proto value needs a named variant, except the zero one,
  which the catch-all may cover instead. `UNSPECIFIED` is emitted for that
  case and names it whatever the proto calls the value (`worker::
  health_check::Response::UNSPECIFIED` is `ServingStatus::UNKNOWN`). Where 0
  *is* a named variant (an operator enum whose 0 is `Equal`, `SortDirection::
  Unspecified`), `Unknown` simply never holds 0.
- **Comparison**: `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash` are emitted
  in terms of `i32::from`, and deriving them at the site is rejected. One value
  has two spellings, the named variant and a catch-all holding its number, and
  only the proto value equates them; a derive would also order the catch-all by
  where it sits in the item. So the type orders by proto value throughout, an
  unknown value sorting by the number it carries rather than as a class.
  `Sort<T>`/`SortMany<T>` derive `Ord` and need this to be meaningful.
- `Default` is emitted (the zero value) unless a variant carries the std
  `#[default]` attribute.
- Size is 8 bytes (discriminant + payload) instead of 4. Same-layout-as-`i32`
  with a payload variant is not expressible on stable Rust (niche-range
  control / pattern types are unstable); since decode now constructs enum
  values directly (no `Vec<i32>` -> `Vec<enum>` passes remain), the only cost
  is memory density in small repeated fields. Revisit if pattern types
  (RFC 3628) stabilize.
- Casts: `as i32` no longer works on a dataful enum; `From<TaskStatus> for
  i32` and `From<i32> for TaskStatus` are generated.

### 3.5 Non-standard formats: `with` adapters

`#[armonik(with = "path::to::Adapter")]` is the escape hatch for fields whose
Rust representation is not the proto shape. It reads like serde's `with`, but it
names a *type* implementing `ProtoAdapter<T>` rather than a module of free
functions, which is what keeps the const generics and gives one trait-bound
error instead of two or three argument-type ones (section 5). Concrete case: `GetOwnerTaskIdResponse.result_task` is
`repeated MapResultTask { result_id = 1, task_id = 2 }` on the wire but
`HashMap<String, String>` in the API.

```rust
#[armonik_macros::message]
#[armonik(message = "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse")]
pub struct Response {
    #[armonik(with = "crate::codec::adapters::PairMap")]
    pub result_task: HashMap<String, String>,
    pub session_id: String,
}
```

The named type implements `ProtoAdapter<T>` (`encode_field`/`merge_field`/
`encoded_len_field`, tag still supplied from the descriptor; `ProtoField` and
`ProtoAdapter` share method names, so the emitter only switches the dispatch
prefix). The crate ships generic building blocks,
`PairMap` (delegating to prost's real-map codec) and `Wrapper<TAG>` (a
single-field wrapper message flattened to its `String`/`Vec` payload), so
a new adapter is a few lines. `with` fields skip the shape check (the
adapter is asserting a non-standard mapping on purpose) and are covered by
the differential harness instead (each adapter also declares its value-level
loss through `normalize_dynamic`).

### 3.6 Presence semantics

> **Zero-default invariant (supersedes the earlier addendum).** Every
> type's `Default::default()` IS the proto zero value, so decoding simply
> seeds from `Default` and "absent = default" holds with no further rules.
> The harness enforces it on the encoding of `Default::default()` itself
> (`default_encoding_is_the_proto_zero`): decoded dynamically, every field it
> carries must hold the proto zero, a set oneof member being allowed when its
> payload is zero. A round-trip cannot check this, because decoding seeds from
> `Default` and `registry::apply_rules` folds each type's `default_encoding`
> into both sides of the comparison, so any default at all agrees with itself.
> The non-zero defaults (infinite `max_duration`, `priority = 1`, 80 KiB
> chunks, `page_size = 100`, the `SessionId`/`TaskId`/`ResultId`/`Asc` sort
> defaults) are gone from `Default`; the named constructors of section 6.1
> (`TaskOptions::recommended()`, `<service>::list::Request::recommended()`,
> `Sort::ascending`) and the exported `INFINITE_DURATION` carry them. This
> removed the
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
> `Default` is `Error("")`, the proto zero, so an absent output is an
> empty *error*. Two consequences of that fold are pinned by unit tests
> rather than designed away: bound to `TaskSummary.error` (a plain string,
> no `success` field) through `ErrorAdapter`, `Success` and `Error("")` are
> the same wire value and decode to `Success`; and since one enum slot cannot
> hold both proto fields, a *repeated* `success` occurrence that selects
> `Success` drops an error message merged before it.

> **Only an implicit-presence leaf skips its zero (supersedes the presence gate
> and the encode-everything rule after it).** A field whose codec is a scalar
> leaf (the `ProtoField` primitives, `String`, `Bytes`, a proto enum) is left out
> when it holds the proto zero, which is what every other proto3 encoder does and
> what a receiver cannot tell from an absent field. Everything else is written
> whatever it holds: `ProtoField::is_zero` and `ProtoAdapter::is_zero` are `false`
> unless a leaf overrides them, so message fields, transparent wrappers, adapters
> and containers never skip.
>
> The skip is a defaulted `encode_implicit`/`encoded_len_implicit` pair on the two
> codec traits, not something the derives expand: which of a field's two entry
> points to name is the one positional fact an expansion has and a codec does not,
> since the same `String` is skippable as a struct field and not as a oneof
> member, where the field being present is what selects the variant. So the
> emitter picks a method name per slot, `Presence::Explicit` at that one, exactly
> as it already picks between the singular and repeated forms.
>
> That scoping is what lets the rule stand alone, where the old `is_default`
> family needed three rules to defend each other: skipping erased a zero wrapper
> enum's presence, so wrapper enums were force-emitted, which made "all-default"
> and "encodes to zero bytes" disagree for the containing message. A leaf-only
> skip cannot start that cascade, because the message framing around a wrapper is
> never conditional. So `Msg::ALWAYS_PRESENT`, the `nondefault` encode forms, the
> message-kind "`encoded_len() == 0`" override and the `PartialEq` supertrait stay
> gone; what came back is three defaulted methods per trait, sharing one
> predicate so the length walk cannot disagree with what is written.
>
> Measured on the bench fixtures: `results/get` 53 bytes against 59, and the
> 32-entry `results/list` response 1700 against 1894. What that buys in time, and
> what it leaves of the branch's unary regression, is in `benches/wire.rs`.

- Non-`Option` message field ("absent = default"): decode merges in place,
  absence leaves the default; encode always writes the field, so a default
  nested message goes out as an empty one. These fields were chosen precisely
  because absent and default are semantically indistinguishable, which is why
  the wire form is free to pick either.
- `Option<T>` fields (presence-meaningful, e.g. `TaskOptions` on task
  submission where `None` inherits session defaults): `None` omits,
  `Some(default)` is emitted. Exact presence preserved; this is the only
  presence knob left, and it lives in the Rust type.
- proto3 explicit `optional` scalars must be `Option` in Rust, validated by
  the derive.
- Empty containers still encode to nothing: a `Vec` writes one field per
  element and a `HashMap` one per entry, so zero elements is zero bytes with
  no default check involved. A map *entry* writes both its subfields whatever
  they hold, unlike a leaf field of a message: the entry codec is hand-written
  (`codec/containers.rs`) and does not consult `is_zero`, where prost's map codec
  omitted a `== default` key or value. Both forms decode identically, which is
  why the differential harness cannot see the difference and `codec/tests.rs`
  pins the bytes instead.
- Flattened oneofs: decode with no variant set -> the default/`Invalid`
  variant; the active member is written even when its payload is the default,
  and an `Invalid`/no-member variant writes nothing (there is no member to
  write, not a skipped default).
- Unknown *fields* are skipped on decode and not preserved on re-encode,
  same as prost and as today. Only unknown enum *values* become lossless.

### 3.7 Performance

- The decode-then-convert double pass disappears on both directions of every
  call, and with it the re-collection of vectors (`Vec<i32>` <-> `Vec<enum>`,
  nested filter vecs) and map rebuilds.
- All proto `bytes` fields become `bytes::Bytes`: decoding slices the tonic
  receive buffer instead of copying (`DataChunk::Data`, result
  upload/download chunks, task payloads); encoding clones cheaply.
  (`bytes` needs its `serde` feature when armonik's `serde` feature is on.)

  **What that costs, and it is not a regression**: an 80 KiB chunk decodes in
  about 30 ns, which is a refcount rather than a copy, so the decoded value
  *shares the tonic receive buffer*. One retained small chunk therefore pins
  whichever allocation it was sliced from, which can be far larger than the
  chunk. A caller that keeps a few bytes out of a large response for a long
  time should copy them out (`Bytes::copy_from_slice`, or `to_vec`). This is
  also why the 5 MiB download benched inconclusive: the work moved off decode
  and onto whoever drops the buffer.
- `encoded_len` recomputation for nested messages matches prost's own
  behavior (recomputed per level); filter trees are shallow, no caching
  needed. Dropping the presence gate also dropped its extra `encoded_len()`
  walk per nested message, at the cost of a few more bytes on the wire.
- `benches/` are kept and run before/after on the branch; results recorded in
  the PR description.

## 4. Validation & testing

Two layers, independent for the derived majority, with one caveat called out
below for the single hand-written cross-field impl:

1. **Compile time** (derive vs descriptor): wrong/missing/extra fields, kind
   mismatches, presence-rule violations, enum variant name/value mismatches:
   all spanned errors; fingerprint tripwire against stale expansions.
2. **Differential round-trip harness** (test-time, generic, no per-type test
   code):
   - `protox` compiles the protos in-test; `prost-reflect` generates
     randomized `DynamicMessage`s per descriptor;
   - round-trip both directions: dynamic -> bytes -> armonik type -> bytes ->
     dynamic, compared semantically (field-set equality, not byte equality:
     absent and present-but-default nested messages are interchangeable per
     section 3.6, and the encoder picks the second form);
   - the equivalence classes are **computed, not restated**: each type's
     `default_encoding` (what it emits for "nothing") is folded into
     messages where those fields are absent, deriving the default-member
     projection from the implementation itself; the
     value-level projections live in a `Normalize` impl per type, generated
     from the same constructs that shape the codec: each `with` adapter
     declares its own loss (`PairMap` order/duplicates), `present` markers
     and `transparent` chains emit theirs. For the one hand-written
     cross-field impl (`tasks::Output`) the `Normalize` projection is
     co-authored with the codec, so the harness is **not** an independent
     oracle there: a shared wrong belief about the wire contract would pass
     both sides. It is pinned instead by checked-in unit fixtures built and
     encoded through a prost-derived reference message (an independent
     codec), covering the cross-field combinations the field-information
     ratchet cannot reach probing one field at a time: `{ success, error }`
     both set. Registration still requires the impl, so a projection is
     never forgotten;
   - the proto-to-type mapping is **self-registering**: each derive (and
     hand-written impl) pushes its entry into a `linkme` distributed slice
     under `#[cfg(test)]` (this bullet said "under the private
     `_differential` feature, enabled only through the self dev-dependency";
     see section 11.1), so new messages are covered with zero harness
     changes, and only the generic instantiations are hand-listed;
   - several Rust types may register one proto name (a request type per RPC
     sharing a wire message, the `Empty` stand-ins) while the projection is
     keyed by the name alone, since a nested message is a name with no Rust type
     attached: a ratchet holds every such group to one projection and one
     default encoding, and names the pair that disagrees;
   - a coverage test iterates *all* messages in the descriptor pool and
     fails on any message with no registered Rust type (explicit allowlist
     for intentionally flattened ones);
   - because the projections are generated from the implementation's own
     declarations, a **field-information ratchet** guards the quotient
     itself: every field of every registered message must have a probe
     value that survives normalization distinguishably (all declared enum
     values are tried, plus an unknown one) and round-trips one field at a
     time; a field the quotient erases entirely must be justified in an
     explicit allowlist; one entry exists today (`v1.Output.ok`, the zero
     value itself);
   - covers `with` adapters, generics (`FilterStatus<T>` instantiations
     checked against each service's concrete proto), and unified types;
   - the **zero-default ratchet** is separate (section 3.6): it reads each
     type's `default_encoding` directly, since the canonical-absence fold makes
     the round-trip blind to what the default is.
3. **Integration tests** (`tests/*.rs`), one `rpc_tests!` case per RPC driving
   two pairs: `mock::{call, convenience}` against `ArmoniK.Api.Mock` over a
   real connection (checking the call landed on the RPC it was aimed at,
   through the mock's `/calls.json` tally) and `in_process::{call,
   convenience}` against the generated fake, asserting the response. Each pair
   covers both entry points, `ServiceClient::call` with a hand-built request
   (or request stream) and the method `service!` derives from that request's
   fields. Hand-written tests cover what a per-RPC case cannot: cancellation
   (a dropped client future must tear the server handler down, on a paused
   clock) and failure propagation early, mid-stream and at end of stream, plus
   the router's unrouted-path status. They compile against `armonik` from the
   outside, so they also prove the public API is usable.

## 5. Migration plan (big-bang branch)

Order of work on the branch; each step compiles and is separately reviewable
even though the branch lands as one unit:

1. **`armonik-macros` crate**: `ProtoField` trait + impls, `#[enumeration]`,
   `#[message]` (structs, oneof-flattened enums, `transparent`,
   `with`), descriptor loading + fingerprint. Unit tests with a small fixture
   proto.
2. **Build pipeline**: switch `build.rs` to `protox`; emit
   `descriptor.bin` + `schema_meta.rs`; stubs via `compile_fds`. (`protoc`
   removed from CI images and docs.)
3. **Differential harness** landed early, running against whatever is
   already derived; it is the safety net for everything after.
4. **Annotate `objects/`** service by service (enums and shared objects
   first, then per-service modules): add derives/attributes, delete the
   type's `From` impls as it becomes self-encoding.
5. **`extern_path` the RPC types**; simplify `client/*` (drop conversion in
   `GrpcCall` paths) and `server/*` (`impl_trait_methods!` loses its
   conversion layer: traits and stubs now speak the same types).
6. **Delete** `impl_convert!` and the manual `IntoRequest` impls (tonic's
   blanket impl covers the native types); internalize `api/v3.rs` (stubs +
   `Empty` remain, `pub(crate)`). `IntoCollection` stays: the client
   conveniences use it independently of the conversion layer.
7. **Polish**: serde feature audit (`bytes/serde`; `Unknown` variants change
   the serde shape of formerly-unit enums), README/docs, CHANGELOG, version
   bump. The release pipeline was *not* extended: `publish.yml` has no
   `cargo publish` step, and the `=`-pinned crates are released by hand, in
   the order `packages/rust/RELEASING.md` gives.
8. **Benches** re-run; numbers in the PR.

## 6. Public API changes (breaking, accepted)

- `armonik::api::v3` removed entirely (goal of the revamp). What remains
  of the generation is exactly the tonic client/server stubs, exposed per
  service as `armonik::client::<service>::stub` and
  `armonik::server::<service>::stub`, presentable as public API because
  every message in their signatures is an armonik type (the five
  `Empty`-signature RPCs each speak their own wire-compatible `{}` type),
  and the leftovers are pruned from generation.
- The message types stayed in `armonik`: `armonik::applications::Raw`,
  `armonik::TaskOptions` and the rest keep resolving from where they always
  did. This bullet used to send readers to a separate `armonik-types` crate;
  it was never split out (Part II section 3.1 records why), and this is the
  migration guide, so it says what shipped rather than carrying a marker.
- Client-streaming calls go through the same entry point as every other kind,
  `client.call(stream)` (superseded: this bullet used to describe a separate
  `call_streaming`; see Part II section 3.5 and 5.17). The named methods
  (`Agent::create_tasks`, `Results::upload`, `Submitter::create_large_tasks`)
  are unchanged; they delegate to it.
- Rust types sharing one wire message stay distinct and convert at the
  stub boundary (`tasks::list_detailed`, the agent data RPCs, the
  submitter request wrappers), so `client.call(...)` dispatch is
  unchanged.
- `sessions::RawField` wire fix: the old crate encoded the Rust
  discriminants for ClientSubmission/WorkerSubmission/ClosedAt/PurgedAt/
  DeletedAt, which disagreed with `SessionRawEnumField`; values are now
  matched by name against the descriptor.
- Enums: dataful `Unknown(...)` variant replaces `Unspecified` unit variants
  (`UNSPECIFIED` const provided); `as i32` casts replaced by `From` impls;
  matches need an `Unknown`/catch-all arm. The comparison traits are emitted
  rather than derived, in terms of the proto value, so `Ord` orders by it and an
  unknown value sorts by the number it carries (see 6.1).
  `worker::health_check::Response` loses its named `Unknown` variant to the
  catch-all with the rest of the zero values: the status is
  `Response::UNSPECIFIED`.
- Every object type derives `Eq` where its fields allow it, and the `Raw`,
  `Summary` and list `Response` types gain `PartialEq`.
- All `bytes` payload fields: `Vec<u8>` -> `bytes::Bytes`.
- `Default::default()` is the proto zero value for every type (the
  zero-default invariant); `tasks::Output::default()` is `Error("")`, and an
  absent task output reads as an empty error rather than success. Section 6.1
  lists the replacements.
- serde representation shifts for the affected enums and `Bytes` fields.
- `prost_types::{Duration, Timestamp}` remain the public time types
  (unchanged).
- Everything else keeps its current shape and module paths.

### 6.1 Defaults: what `Default::default()` yields, and what replaces it

`Default` is the proto zero value everywhere, which is what lets decoding seed
from it with no special wire semantics. Call sites written as
`Ty { field, ..Default::default() }` still compile and now send zeros, so every
type that carried a non-zero default has a constructor supplying the values by
name:

| Type | `Default::default()` | Named constructor |
|---|---|---|
| `TaskOptions` | zero duration, 0 retries, priority 0 | `TaskOptions::recommended()` (`INFINITE_DURATION`, 1 retry, priority 1) |
| `{applications,partitions,results,sessions,tasks}::list::Request`, `tasks::list_detailed::Request` | `page_size: 0`, unspecified sort direction | `Request::recommended()` (`page_size: 100`, ascending) |
| `Sort<T>` / `SortMany<T>` | unspecified direction | `Sort::ascending(field)` / `Sort::descending(field)` |
| `SortDirection` | `Unspecified` | `SortDirection::Asc` |
| `Configuration`, `results::get_service_configuration::Response` | `data_chunk_max_size: 0` | the value the server answers with |
| `tasks::Output` | `Error("")` | `Output::Success` |

Ordering follows the same rule: an enum's variants carry their proto values as
discriminants, so `Unknown(..)` (the zero value and any value unknown to this
crate version) sorts before every named variant, ordered among themselves by the
raw value.

## 7. Risks & mitigations

| Risk | Mitigation |
|---|---|
| `prost::encoding` is public but evolves across prost majors | pinned prost 0.14; helpers are the same surface prost-derive expands to, so breakage tracks prost upgrades we'd do deliberately |
| Derive reads a file at expansion (rust-analyzer, exotic caches) | single build-produced artifact, `OnceLock`-cached; fingerprint const-assert turns any staleness into a compile error; clear diagnostic when build scripts haven't run |
| Descriptor/kind matching bugs in the macro | differential harness fuzzes every message both directions against `DynamicMessage` ground truth |
| Generic filter types can't name one proto message | explicit field attributes on those ~5 types; per-instantiation coverage in the harness |
| Three-crate release (`armonik-transport`, `armonik-macros`, `armonik`) | `=version` pin on the macros, which is what makes a mismatched pair fail to compile rather than misbehave. There is **no** release CI for these: `publish.yml` has no `cargo publish` step, and the order and its reasons are written down in `packages/rust/RELEASING.md` |
| Big-bang review size | branch structured in the step order above; harness green from step 3 onward |

## 8. Future work

- Pattern types (RFC 3628) would allow `Unknown(i32 is ...)` niches -> 4-byte
  enums; revisit when stable.
- `deprecated` submitter service could be dropped instead of migrated if its
  removal is scheduled anyway.
- Zero-copy strings (e.g. `Bytes`-backed) if profiling ever shows String
  allocation as a hotspot; not worth the API noise today.

---

# Part II: RPC definitions, and dropping the stub codegen

Status: **implemented** (branch `rust/direct-message-impls`; decisions settled
in review 2026-08-03, landed 2026-08-04)
Prerequisite: the direct-message revamp (Part I) landed.

> **All of section 8 has landed.** The spike validated the table-driven router and
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
| `client/*.rs` `impl_call!` + manual `GrpcCall`/`GrpcCallStream` impls | ~870 | `Request type -> stub method` |
| `client/*.rs` struct + `where` bounds + `with_channel` + `call` | ~330 | service name |
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
insertions, 5,702 deletions, net -3,454 lines**. Deleted: `armonik/build.rs`
(344), `stubs.rs`, the client dispatch arms and per-service boilerplate, the
`Client` accessor pairs, the two server lists and `ServiceExt` boilerplate, the
19 `#[armonik(replace(...))]` annotations, `ReplaceSpec` + its emitter, the
build-facing half of `wire.rs`, the hand-maintained `UNEXPOSED_RPC_MESSAGES`,
and the per-RPC path tests (13 round-trips remain, section 4). Added: the `service!`
proc macro with validation and doc harvest (~500), the generic router plus the
three `serve_*` bodies (~380 with docs), `ServiceClient` + `Dispatch` (~230),
the `Rpc`/`Service`/kind markers and `Channel` (~100), and the 12 invocations
(~110).

More importantly, three mechanisms disappear outright: the stub-descriptor
surgery, the `replace` substitution machinery, and the `linkme` harvest across a
build-dependency edge. What replaces them is not new infrastructure but reuse of
the descriptor-reading proc-macro that already exists.

The one thing that gets worse: the server router becomes ours. See section 7.

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
| Registry (`linkme`) | Test-only, under `#[cfg(test)]`; no feature (section 11.1) |
| Client dispatch | One `ServiceClient<Svc, T>`; `call` deduces the RPC from the request type |
| `GrpcCall` / `GrpcCallStream` | Deleted, replaced by `Rpc` + kind dispatch |
| Server trait | Generated by `service!` from the same invocation |
| Router shape | One generic `tower::Service` + `NamedService` over per-service const route tables emitted by `service!`; match-emission is the spike fallback (section 3.6) |
| Convenience methods | **Generated** by `service!` from the request struct's fields, through the reflection handshake (section 3.7). The row below this one recorded the opposite for a while; what settled it is that spread parameters need the request's field list at a point in the source where only the rpc line is visible, and only a macro-to-macro channel can carry it there |
| Doc comments | Harvested by `service!` from the descriptor's `SourceCodeInfo` (build retains it); the invocation carries no prose (section 3.3) |
| `unexposed(...)` | Declared in the invocation; the macro emits the differential harness's message allowlist from it, retiring the hand-maintained `UNEXPOSED_RPC_MESSAGES` |
| `tonic-prost` | Kept: it provides `ProstCodec`. Only `tonic-prost-build` is dropped |
| Tests | One `rpc_tests!` case per RPC, driving both the in-process server and the C# mock: 59 RPCs x 2 pairs, not the 12 + 3 this row first planned |
| `tonic::Status` in public signatures | Kept for this branch; REST is not on the roadmap |
| Span names | Not load-bearing (confirmed): fixed client callsite with `rpc` / `otel.name` fields; server keeps per-RPC literals, free in the emitted route closures (section 3.6, section 7) |

## 3. Architecture

### 3.1 Crate layout

```
packages/rust/
  armonik-macros/     # proc macros: #[message], #[enumeration], service!
                      #   deps: syn, quote, prost (descriptor decode only)
  armonik/            # everything else: messages, codec, RPC definitions,
                      #   client, server, router. One build script.
  armonik-transport/  # unchanged: config parsing, TLS, the connection
```

`armonik-types` folds into `armonik` wholesale. Its stated reasons to exist
(Part I section 3.1) were a pure-types dependency and, "the reason the split earns
its keep", the extern-map harvest across the build-dependency edge. The harvest
is deleted by section 3.2, and no consumer of the pure-types build exists in the
repository. By Part I's own test the split stops earning its keep.

Consequences worth taking while merging:

- `objects/` goes back to private with the flat `pub use` re-exports as the only
  surface. It is currently `pub` + `#[doc(hidden)]` solely so the paths
  registered into `wire::REGISTRY` resolve from the `armonik` crate.
- `Msg` and the rest of `codec` stay `pub(crate)`; the const-asserts of section 3.3 no
  longer need to cross a crate boundary.
- The `=` pin chain shortens from two links to one; `scripts/versions/` loses a
  crate. `armonik-transport` stays independent and version-unlocked.
- `armonik-macros` now emits `crate::` paths into exactly one consumer, so the
  "internal-use derives" invariant becomes uniformly true.

### 3.2 Build pipeline

One build script, `armonik/build.rs`, which is `armonik-types/build.rs` moved
verbatim:

1. `protox` compiles `protos/V1/*.proto` into a `FileDescriptorSet`, retaining
   `SourceCodeInfo` (the leading comments feed the doc harvest, section 3.3).
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

    rpc ListResults(list::Request) -> list::Response;
    rpc WatchResults(stream watch::Request) -> stream watch::Response;
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
`SourceCodeInfo` (retained by the build, section 3.2) and emits `#[doc]` on the
service marker, the server trait and its methods. The two hand-transcribed
copies that drifted (section 1) become uncopyable.

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
type, across services too, since `Rpc::Service` is part of the impl. This
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
  consumes (section 3.6)
- `_gen-client`: one convenience method per non-`manual` rpc line (section 3.7),
  and `pub type Client<T> = ServiceClient<Results, T>` carrying the same
  harvested docs as the marker. `client/<svc>.rs` is a one-line re-export of it
  (`pub use crate::rpc::results::Client as Results;`). The twelve aliases used to
  be hand-written there, each with a hand-transcribed service doc comment, two of
  which had already drifted from the protos (`client/partitions.rs`,
  `client/versions.rs`) — the duplication class Part II section 1 names as
  motivation, reintroduced by hand. `client/agent.rs` and `client/worker.rs` keep
  a doc comment of their own, because those two protos document their service
  with nothing at all

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
  `unexposed(...)` list in the invocation (today: `WatchResults` on
  `Submitter`, whose copy the proto deprecates; the `Results` one is routed).

That last one replaces `guard_all_rpcs_claimed` and the per-RPC path tests: a
forgotten RPC is a compile error naming the method, not a test failure. The
macro also resolves the unexposed methods' input and output message names from
the descriptor and emits the differential harness's message allowlist (gated on
`cfg(test)`), retiring the hand-maintained `UNEXPOSED_RPC_MESSAGES`: one
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
validation in section 3.3.

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

    /// Perform a gRPC call. The RPC, and with it the kind, is deduced from the
    /// request type: one entry point for all four kinds.
    pub async fn call<R, M>(&mut self, input: impl IntoCall<R, M>)
        -> Result<<R::Kind as Dispatch>::Output<R>, RequestError>
    where R: Rpc<Service = Svc>, R::Kind: Dispatch
    { input.into_call(&mut self.inner).await }

    // knobs that are unreachable today, because `inner` is private
    pub fn send_compressed(self, e: CompressionEncoding) -> Self { ... }
    pub fn max_decoding_message_size(self, n: usize) -> Self { ... }
}
```

The **output shape** hangs off the kind type, with a GAT, and the **dispatch**
hangs off the *input* shape, through a marker-disambiguated trait:

```rust
pub trait Dispatch: Sized {
    type Output<R: Rpc<Kind = Self>>;
}
impl Dispatch for Unary        { type Output<R: Rpc<Kind = Self>> = R::Response; }
impl Dispatch for ClientStream { type Output<R: Rpc<Kind = Self>> = R::Response; }
impl Dispatch for ServerStream {
    type Output<R: Rpc<Kind = Self>> =
        futures::stream::BoxStream<'static, Result<R::Response, RequestError>>;
}

/// The kinds whose request is a single message. `ClientStream` is deliberately
/// absent: that absence is what rejects `call(one_message)` on an upload.
pub trait DispatchMessage: Dispatch {
    async fn dispatch<T, R>(grpc: &mut tonic::client::Grpc<T>, req: tonic::Request<R>)
        -> Result<Self::Output<R>, RequestError>
    where T: Channel, R: Rpc<Kind = Self>;
}
impl DispatchMessage for Unary { /* ready, ProstCodec, PathAndQuery::from_static(R::PATH),
                                    GrpcMethod extension, unary, into_inner, GrpcSnafu */ }
impl DispatchMessage for ServerStream { ... }

pub trait IntoCall<R, M> where R: Rpc, R::Kind: Dispatch {
    async fn into_call<T: Channel>(self, grpc: &mut tonic::client::Grpc<T>)
        -> Result<<R::Kind as Dispatch>::Output<R>, RequestError>;
}
pub struct ByMessage; pub struct ByRequest; pub struct ByStream; pub struct ByStreamRequest;

impl<R: Rpc<Kind: DispatchMessage>> IntoCall<R, ByMessage> for R { ... }
impl<R: Rpc<Kind: DispatchMessage>> IntoCall<R, ByRequest> for tonic::Request<R> { ... }
impl<R: Rpc<Kind = ClientStream>, S: Stream<Item = R> + Send + 'static>
    IntoCall<R, ByStream> for S { ... }
impl<R: Rpc<Kind = ClientStream>, S: Stream<Item = R> + Send + 'static>
    IntoCall<R, ByStreamRequest> for tonic::Request<S> { ... }
```

`M` is inert: it exists only so the four impls are disjoint by their *trait
arguments*, since they are not disjoint by their self types as far as coherence
is concerned (nothing stops an upstream crate implementing `Stream` for a
request type — see section 5.17). This is the trick `axum::handler::Handler`
uses. `M` is inferred at every call site alongside `R` and is never named; the
only trace it leaves is that an explicit turbofish has to leave room for it,
`call::<R, _>(..)`.

Writing the four input shapes out by hand, rather than reusing
`impl tonic::IntoRequest<R>`, is also what **removes** the inference wart: with
tonic's two blanket impls, `call(tonic::Request::new(msg))` could not infer `R`
and needed `call::<R>(..)`; with `R: Rpc` on our own impls, the `T`-for-`T` arm
is rejected (a `tonic::Request` is not an `Rpc`) and `R` is inferred. It also
buys per-call metadata and deadlines on *client-streaming* calls, which no
shape of `call_streaming` ever offered.

`GrpcCall`, `GrpcCallStream` and `call_streaming` are deleted. `client.call(x)`
keeps its contract and gains two: `R: Rpc<Service = Svc>` makes "you cannot call
a Sessions RPC on a Tasks client" an explicit bound rather than a property of
which impls happened to be written, and the entry point no longer forks on the
call kind, which is the point — the request type conveys all the information
about the RPC, so the caller should not have to restate it in the method name.

Per-service files then hold only the convenience methods, as inherent impls on
the concrete alias (legal: local generic type, concrete type argument, so
`Sessions::with_channel` still resolves).

`Client::{svc, into_svc}` (24 methods, ~190 lines) becomes a twelve-line
`services! { agent => Agent, ... }` macro.

### 3.6 Server: the router

This is the only new code, and the reason for the spike in section 8.

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
generic parameter, so per-RPC literal span names survive (section 5.13 only forbids
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

The unmatched-path arm answers
`Status::unimplemented("<path> is not implemented").into_http()`, so the
`grpc-message` names the method that was refused. That arm is where a method of
another service lands, and where the `unexposed(...)` RPCs land: they have no
route by construction, so `Submitter.WatchResults` is refused there rather
than by a generated stub.

Because `stream` tells `service!` the shape of each RPC, it can emit the
streaming trait signatures itself (`impl Stream<Item = Result<T, Status>>` in
argument or return position). The five hand-shaped signatures currently living
after the `---` in `define_trait_methods!`, and the `---` escape hatch itself,
go away.

### 3.7 The convenience layer is fully derived from the request structs

The convenience methods have **zero per-method source**: `service!` emits
them, and their *signatures* come from the request structs' own fields, the
one place the Rust types (not the proto types, section 5.8) are visible. Parameters
mirror the fields in declaration order (field order is wire-irrelevant, so
structs are reordered where ergonomics demand), widened by a sugar class the
derive infers from each field's type:

| field type | parameter | conversion |
|---|---|---|
| `String`, `Bytes`, `Vec<u8>` | `impl Into<...>` | `.into()` |
| `Vec<T>` | `impl IntoIterator<Item = impl Into<T>>` | `.into_collect()` |
| `HashMap<K, V>` | `impl IntoIterator<Item = (impl Into<K>, impl Into<V>)>` | pair map |
| `filter::Or` | nested `impl IntoIterator` of `filter::Field` | `into_filters(...)` |
| anything else | itself | moved |

The mechanism is a cross-macro handshake: `#[armonik_macros::message]` emits, next to
each struct, a `__armonik_fields_*` callback macro (field names + sugar
classes, CPS-style) and flat `__armonik_ty_*` aliases (so another module can
name field and element types without transporting relative-path tokens);
`service!` emits an invocation of the request's callback continued into the
`__emit_convenience` proc macro, which builds the method. Method docs are the
harvested proto comments.

The *response* side is spelled on the rpc line, not inferred: a bare line
returns the whole response, `=> field` projects that field, `=> ()` discards it.

It used to be inferred, with the same three forms plus `=> *` for "whole,
always": a bare line meant "project the single field, or return the whole
response if there are several", which `__emit_convenience` resolved by chaining
once through the *response* type's callback to count its fields. Two things were
wrong with it. The return type of 30 of the 59 methods was a function of a proto
message's field count, so adding a second field to any of those messages
silently changed the public API. And that second hop was the only reason a
*response* needed a reflection callback at all: a response with none, an enum or
an alias, failed to resolve a mangled name rather than saying what was missing.
Making the 30 say `=> field` deleted the hop, the `auto` form and `=> *` at
once.

The request side is untouched, and it is where the reflection earns its keep:
the field list is what turns `list::Request { filters, sort, with_task_options,
page, page_size }` into `client.list(filters, sort, with_task_options, page,
page_size)`, and only a macro-to-macro channel can carry it to the rpc line,
where the struct is not visible.

That mangling is by the type path written on the rpc line, which an **alias** of
a message breaks: `pub type Response = Count;` has no reflection of its own (the
derive emitted it next to `Count`, under the `count` stem). A second attribute
macro, `#[armonik_macros::reflect]`, used to carry it over for the two RPCs that
needed it, and produced the worst diagnostic in the system when forgotten: it
pointed at the *request*'s derive, in the wrong file, and suggested importing a
mangled alias. Both sites now declare the message themselves
(`submitter::{count_tasks, wait_for_completion}::Response`), which is what the
crate does everywhere else a proto message is shared across RPC sites, and the
second proc macro is gone. Two aliases remain and are fine, because neither
projects: `submitter::get_service_configuration::Response = Configuration` and
`submitter::try_get_task_output::Response = Output`.

**Opt-out**: `manual` on the rpc line emits nothing: the escape for custom
wiring or a wrong mechanical default. Client-streaming RPCs are required to
carry it: their request is a stream, so there is no message to spread into
parameters. (Since the `IntoCall` unification their *entry point* is `call`
like everyone else's, so a `stream in, projected response out` method would now
be derivable -- but all three client-streaming RPCs need a hand-written body
anyway, two to turn a oneof response into an error and one to build the request
stream, so deriving it would be an emission path with no consumer, which
section 5.11's rule rejects.) Today that leaves eight hand-written
methods (`results::{upload, watch}`, `worker::process`, where nine exploded
parameters is a wrong default, `submitter::{create_small_tasks,
create_large_tasks, try_get_task_output}`, `agent::create_tasks`, and
`agent::notify_result_data`, whose `Request::in_session` constructor spreads one
session across many result ids).

Everything is type-checked post-expansion, so a wrong sugar inference is a
compile error, never a wire bug; behaviorally, every generated method is
covered per method by the in-process integration suites. Accepted DX shift:
parameter names and order are now mechanically the field names and order
(e.g. `results::get` takes `id`, the agent methods take `communication_token`).

An earlier draft had `#[armonik_macros::message]` emit a `Request::new(..)`
constructor instead, and a first implementation generated only the bodies
under hand-written signatures; both were dropped for this design, which is
what actually deletes the layer. The nested-filter collect shared by the
`list` methods survives as the `into_filters` helper the emission calls.

### 3.8 What the registry is for now

`linkme` and the registry survive, test-only. They live in
`src/differential/registrations.rs` under `#[cfg(test)]`, and `linkme` is a plain
dev-dependency; there is no `_registry` feature and no `_differential` one (see
section 11.1).

- kept, because the differential harness consumes them: `REGISTRY`,
  `Registration`, `Role::{Message, Absorbed, Unexposed}`, `absorbed()`,
  `unexposed()`, and the per-type `Hooks` a `Role::Message` carries;
  `UNREFERENCED_MESSAGES` is the one hand-maintained allowlist left, and the
  unexposed RPCs' messages are emitted by `service!` from the `unexposed(...)`
  declarations (section 3.3) rather than listed;
- deleted, because their only consumer was `armonik/build.rs`: `Direction`,
  `Replacement`, `Role::Replace`, `replacements()`, `extern_mapping()`;
- deleted, because nothing consumed them: `DESCRIPTOR` (the harness embeds the
  descriptor set with its own `include_bytes!`) and the `Diff`/`Entry` pair,
  which were the same struct with a re-wrap between them.

The RPC side needs no `linkme` at all: completeness is checked at expansion
against the descriptor (section 3.3), not by collecting a distributed slice.

### 3.9 Spans

Every call runs under one span, plus a child `stream` span wherever a stream is
polled outside the call future, since the call future has already completed by
the time the peer polls a stream it returned:

| Side | Span | Child `stream` span |
|---|---|---|
| Client | `armonik.rpc` (fields `rpc`/`otel.name` = `R::LABEL`, section 5.13) | none of its own: the response stream of a server-streaming call is instrumented with `span.clone()`, so it continues the same `armonik.rpc` span rather than opening a child (`service_client.rs`) |
| Server | the per-RPC `debug_span!` from the route closure (section 3.6) | the inbound request stream of a client-streaming RPC, and the response stream, which carries the `Response item` traces |

The response-stream case is why `ServerStream` polls through its
`tracing_futures::Instrumented` wrapper rather than the stream inside it:
`Instrumented`'s own `poll_next` is what enters the span, so unwrapping it
would leave every server-streaming RPC tracing its request line and nothing
else under that span.

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
   assertions in `client/*.rs`, subsumed by (1) and (2), but only *after* the
   cutover, so they cover it for free while it happens. What replaces them: one
   round-trip per service through a representative convenience method (12
   tests, catching the one class the compiler cannot: a convenience method
   wired end-to-end to the wrong RPC), plus one test per streaming kind (3).
   **What shipped is larger**: `rpc_tests!` emits one case per RPC, each driving
   the in-process server and the C# mock through both `call` and the convenience
   method, so 59 RPCs rather than 12 + 3.

Note on features: the harness needs the registry, not `DESCRIPTOR`, which it
embeds itself. It was reached through a `_differential` feature enabled by a
dev-dependency of the crate on itself, which meant every integration suite
linked a lib with the harness compiled in and the *shipped* configuration was
built by CI but never tested. It is now a `#[cfg(test)]` module, so what the
suites link is the artifact the crate ships, and `prost-reflect` and `linkme`
are ordinary dev-dependencies.

## 5. Aborted alternatives

Recorded because several of them look obviously right until a specific fact
kills them.

### 5.1 `Rpc` on request types, in `armonik-types`, inferred from the descriptor

The first proposal: `#[armonik_macros::message]` scans the descriptor's services for an
RPC whose input is this message and emits `impl Rpc`.

Killed by cardinality. *Proto* request messages are not in bijection with RPCs:
`Empty` serves five, `TaskFilter` three, `ListTasksRequest` two, `DataRequest`
three. A descriptor scan cannot decide which RPC a message type stands for,
which is exactly why `replace` had to be invented in the first place. Writing
the relation down per RPC in the invocation sidesteps the inference entirely,
and the *Rust* request types are already distinct per RPC (the former `replace`
types), so the emitted impls stay coherent; see the injectivity invariant in
section 3.3.

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
in Part I section 7) for no consumer. Superseded outright by section 5.2 and by the merge.

### 5.4 A central `const RPCS: &[Def]` array

Rejected on two grounds: RPC definitions should be scoped per service, in the
service's file, and should read as Rust rather than as data fed to a generator.

### 5.5 Distributed `const RPCS` per module, plus a central `&[&[Def]]`

The obvious repair to section 5.4. Rejected because the central list is a second place
to forget an entry, which is the failure mode this whole exercise removes.

### 5.6 `linkme` in `armonik` to collect distributed RPC registrations

The other repair to section 5.5, and the one that follows the "registration and
implementation are the same act" principle `DESIGN.md` already states for
`Normalize`. Made unnecessary by expansion-time completeness checking (section 3.3),
which is strictly better: a compile error at the invocation instead of a test
failure listing a name.

### 5.7 The convenience methods as the RPC definition

Attractive, and once section 5.2 removed the build-script visibility objection it
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

Proc macros share no state across invocations: what `#[armonik_macros::message]` learns
about `list::Request` is gone by the time `service!` expands, and there is no
compiler-mediated channel between them. `service!` could read the descriptor,
but the descriptor gives *proto* types, and the gap between proto types and the
Rust types is precisely what this crate exists to bridge: `repeated FiltersAnd
filters` is `filter::Or`, `TaskOptionField` is a transparent enum,
`GetOwnerTaskIdResponse.result_task` is a `PairMap` adapter. Descriptor-driven
convenience generation would be wrong exactly on the types that were hand-shaped.

Resolved by not generating the convenience layer at all (section 3.7): the methods
stay hand-written, and the merge is justified by section 3.1 alone.

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
dominated by `protox` and `tonic-prost-build` in build scripts, which is section 3.2,
not the runtime.

### 5.10 Keeping the generated server stubs

The initial position, on the grounds that the router is non-trivial protocol
code. Refined rather than reversed: the protocol code lives in
`tonic::server::Grpc`, which we keep and call. The *generated* `XServer` is a
path match plus that call, twelve times. See section 3.6.

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
dispatch off the kind type (section 3.5) has no coherence question at all: two impls on
two distinct concrete types.

This is also why `IntoCall` needs its marker parameter: `impl IntoCall<R> for R`
and `impl IntoCall<R> for S where S: Stream<Item = R>` are the same collision in
another costume, and no amount of bound-writing separates them (section 5.17).

### 5.13 Per-RPC span names from a generic dispatcher

`tracing::debug_span!(R::LABEL)` cannot compile: the macro expands to `static
META: Metadata` plus `static CALLSITE`, and statics cannot reference generic
parameters. Constructing `Metadata` by hand per monomorphization is possible in
principle (`Metadata::new` is const) but needs a `Callsite` and leans on
`tracing-core` internals. See section 7 for the accepted consequence.

### 5.14 Inferring the streaming kind from the convenience method's return type

Rejected: a mistyped signature would silently change the wire behaviour. Written
in the invocation and checked against the descriptor, a mistake is a compile
error.

### 5.15 A terser `rpc ListResults(list);` form

Rejected, but not for the reason first recorded here. The claim was that
`create_tasks::{SmallRequest, LargeRequest}` and `list_detailed` would need
override syntax; `list_detailed` is perfectly canonical, and counting the rpc
lines, the terser form would in fact cover **57 of 59**, the two
`create_tasks::{Small,Large}Request` lines being the only exceptions.

What actually kills it: `-> stream` would need a bare-keyword form of its own,
and the response type has to stay visible for a `=> field` projection to be
reviewable against it (section 3.7).

### 5.16 An `armonik::Status` abstraction boundary

**Deferred, not rejected.** `tonic::Status` in the server trait signatures makes
the user-facing API tonic-shaped, which would matter if a second transport
existed. REST is not on the roadmap, the crate is beta, and the RPC table leaves
exactly one conversion site per kind, so this stays a cheap follow-up. Revisit
if `feat-implements-rest-json` or `wk/feat/rust-proxy` becomes real.

### 5.17 The shapes tried before `IntoCall`, for one `call` over every kind

The separate `call_streaming` was originally kept on the grounds that "a
client-streaming call takes a `Stream<Item = R>` rather than an `R`, which no
single signature expresses well". That is true of a signature written in terms
of `R` alone; it is not true once the *input* gets to pick the impl. Four shapes
were compiled against the real request types before section 3.5's landed. All
outputs below are rustc 1.95.0.

**(a) An `Input<R>` GAT on `Dispatch`** (`type Input<R>` = `R` for the message
kinds, `BoxStream<'static, R>` for `ClientStream`, `call(input: <R::Kind as
Dispatch>::Input<R>)`). Compiles, and does serve every kind — but the
argument is now behind a projection, and Rust cannot invert one, so *every* call
site loses inference, including the ones that have it today:

```
error[E0284]: type annotations needed
   = note: cannot satisfy `<_ as Rpc>::Kind == _`
help: consider specifying the generic argument
   |     let _ = client.call::<R>(sessions::list::Request::default());
```

A turbofish on every call, and a boxed request stream. Strictly worse than the
`call`/`call_streaming` split.

**(b) The input as a method parameter with an associated-type-equality bound**
(`call<R, I>(input: I) where R::Kind: Dispatch<Input<R> = I>`) instead of a
projection in argument position. Identical failure — the equality bound still
has to be solved right-to-left — and now the turbofish takes both parameters:
`call::<R, sessions::list::Request>(..)`.

**(c) The marker derived from the kind instead of free**
(`Dispatch::Marker`, with `call<R>(input: impl IntoCall<R, <R::Kind as
Dispatch>::Marker>)`). This one *works*, and is in one respect nicer: `call`
keeps a single type parameter, so an explicit turbofish stays `call::<R>(..)`.
It was rejected because it cannot accept a `tonic::Request` around a request
stream: with the marker pinned to the kind, that impl has to share `ByStream`
with the bare-stream impl, and then

```
error[E0119]: conflicting implementations of trait `IntoCall<_, ByStream>` for type `tonic::Request<_>`
   = note: upstream crates may add a new impl of trait `futures::Stream` for type `tonic::Request<_>` in future versions
```

A free marker separates them by trait argument and the conflict evaporates. The
capability that buys — metadata and deadlines on a client-streaming call — is
worth more than the `, _` in a turbofish that nothing in the crate needs.
Deriving the marker from the kind also leans on trait selection with a
*projection of an inference variable* in the trait arguments, which is a less
travelled path than the axum pattern.

**(d) A wrapper type at the call site** (`client.call(Streaming(s))`) as the
fallback. It does not remove the need for a marker: `impl IntoCall<R> for R` and
`impl IntoCall<R> for Streaming<S>` still overlap, because coherence cannot rule
out `Streaming<S>: Rpc`. It only works if the message side stops being blanket
(one emitted impl per request type, from `service!`), at which point the call
site is uglier *and* the machinery is bigger. Not needed.

**Is this the same wall tonic hit?** tonic's generated clients expose two
methods (`impl IntoRequest<R>` for unary, `impl IntoStreamingRequest<Message =
R>` for client-streaming), which looks like the same conclusion, but it is not
the same problem: tonic's methods are *generated per RPC*, so the kind is
already fixed by the method name and there is nothing to disambiguate. tonic
never needed one signature to cover both, so its two-method shape is not
evidence that one is impossible. (Its `IntoStreamingRequest` is sealed anyway,
so it could not have served as our disambiguator.)

- `armonik::client::<service>::stub` and `armonik::server::<service>::stub` are
  removed. There are no generated stubs. This is the load-bearing break.
- `GrpcCall`, `GrpcCallStream` **and `call_streaming`** are removed. All three
  call kinds go through `client.call(x)`, which takes whatever identifies the
  RPC: the request message, a `tonic::Request` around it, a `Stream` of request
  messages, or a `tonic::Request` around such a stream (section 3.5). So per-call
  metadata and deadlines are expressible on every kind, client-streaming
  included, which no shape of `call_streaming` offered.
  There is no inference wart left: the documented one (a pre-built
  `tonic::Request` needing `call::<R>(request)`, because tonic's two blanket
  `IntoRequest` impls leave `R` ambiguous) is gone, since `IntoCall`'s impls
  carry `R: Rpc` and only one of them can match. `call` gains an inferred marker
  parameter, so an *explicit* turbofish now reads `call::<R, _>(..)`; nothing in
  the crate or its tests needs one.
- `Dispatch` loses its `dispatch` method to the new `DispatchMessage`
  subtrait and keeps only `Output<R>`, and gains an impl for `ClientStream`.
  Downstream code naming `Dispatch::dispatch` (there is none: it exists to be
  called by `call`) would break.
- `Sessions<T>` and friends become type aliases for `ServiceClient<Svc, T>`.
  `with_channel` and the convenience methods resolve unchanged; diagnostics
  mention the underlying type. `#[deprecated]` moves to the `Submitter` alias
  and should be repeated on its convenience methods for parity.
- `armonik-types` no longer exists as a crate. `armonik::results::list::Request`
  and every other path is unchanged, since `armonik` re-exported the whole
  surface already.
- `#[armonik(replace(...))]` is removed from the attribute grammar (which is
  `#[doc(hidden)]` and unsupported, so this is not a public break).
- The client's `tracing` span names change; see section 7.
- `Rpc`, `Service`, `Unary`, `ServerStream`, `ClientStream`, `Dispatch`,
  `DispatchMessage`, `IntoCall`, its four markers (`ByMessage`, `ByRequest`,
  `ByStream`, `ByStreamRequest`) and `Channel` are new public API. `Rpc` should be supported and documented, unlike
  the attribute grammar.
- New public knobs on the clients: `send_compressed`, `accept_compressed`,
  `max_decoding_message_size`, `max_encoding_message_size`.

## 7. Risks & mitigations

| Risk | Mitigation |
|---|---|
| The router is ours now, and it is protocol-adjacent | Dispatch into `tonic::server::Grpc`, so framing and status handling stay tonic's. Spike the table-driven shape on `Results` against `tests/results.rs` before anything else (section 8); match-emission is the in-step fallback |
| Client span names change | Resolved: confirmed not load-bearing (human debugging only; no dashboards or `EnvFilter` directives depend on them). Fixed callsite `"armonik.rpc"` plus `rpc = R::LABEL` and `otel.name = R::LABEL` fields |
| Server span names | **Changed**, not preserved: `main` emitted `debug_span!("list")`, the branch emits `"SessionsService::list"` (`service.rs`). Better, and still a change |
| `service!` reads a file at expansion | Same mechanism, same `OnceLock` cache and same fingerprint tripwire as the existing derives. Not a new class of coupling |
| Losing the two build-time extern guards | Replaced by expansion-time completeness (section 3.3) and the const asserts. The duplicate-generated-struct failure `guard_all_messages_externed` prevented cannot occur without codegen |
| One larger crate, worse incrementality | Accepted. Touching a client file now recompiles the codec; ~16k lines total |
| Feature matrix consolidates into one crate | Resolved by dropping the feature: the harness is a `#[cfg(test)]` module, so `prost-reflect` and `linkme` are dev-dependencies and no configuration ships it |
| tonic 0.14 internals (`client::Grpc`, `server::Grpc`, `GrpcMethod`) | These are public API, and they are exactly what the generated code calls. Breakage tracks tonic majors we would upgrade deliberately. The router did call two `#[doc(hidden)]` config appliers, which a 0.14.x *patch* could have reshaped; it now uses the public builders (`accept_compressed`, `send_compressed`, `max_*_message_size`) and drains the encoding sets through the public `pop` |

## 8. Migration plan (all landed)

Each step compiled, was separately reviewable, and was committed on its own.
One deviation from the written order: step 8 (the merge) had to follow step 7
for the reason recorded in step 8 itself.

0. **Spike** on one service, `Results` (every call kind), against the
   existing `tests/results.rs` and the mock server, with the generated stubs
   still in place alongside: the `Channel` bundle, hand-written
   `Rpc`/`Service`/kind markers, `Dispatch` for `Unary` and `ServerStream`,
   `ServiceClient::{call, call_streaming}`, and the table-driven router of
   section 3.6. Everything after this step is subtraction; this step is the only
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

- **`armonik::Status`** (section 5.16), if a second transport materialises.
- **Convenience method compression** (`macro_rules!` over the common shape), if
  the bodies turn out uniform enough to be worth its diagnostics cost. Evaluate
  after step 9, with real numbers, not before (section 3.7).
- **Reflection service**, now cheap: the descriptor is embedded already, and
  nothing prunes it any more.

## 10. Resolved questions

1. **Grammar**: as in section 3.3, with the `as` override for the two `create_tasks`
   methods and `stream` as a position prefix.
2. **`unexposed`**: in the invocation; the macro emits the harness allowlist
   from it (section 3.3, section 3.8), so the two allowlists cannot drift.
3. **Span names**: confirmed not load-bearing (human debugging only); free to
   change (section 7).
4. **`armonik-transport`**: stays a separate crate. Unlike `armonik-types` the
   split costs nothing (no `=` pin in the chain, no build-dependency edge, no
   macro coupling) and it quarantines the hyper/rustls surface, with
   `wk/feat/rust-proxy` a plausible second consumer.

# Part III: the simplification pass

Everything above describes the design and how it was arrived at. This part
records what a review of the shipped branch changed, and, more usefully, what it
considered and rejected, so that none of it is re-proposed from scratch.

## 11.1 What changed

| | |
|---|---|
| Macro diagnostics | A resolution error used to delete the annotated type, so one mistake became a page of unresolved imports with actively wrong suggestions. The item is now re-emitted next to the `compile_error!`, with stub impls for whichever trait its users reach it through (12 diagnostics down to 1 on the measured case). The rpc-line const asserts are spanned onto their own type paths, and `stream` is checked against the descriptor before `manual` is asked for |
| Compile-fail suite | 31 `trybuild` cases in `armonik-macros/tests/ui`, one per error class, against a fixture schema the test compiles itself. The expansion-time diagnostics were the branch's best feature and had no coverage at all. The const-assert classes stay out: their messages belong to `armonik::codec` and fire at const-eval against the real impls |
| Doc harvesting | Two independent bugs. Value matching stripped the *Rust* type's name where proto values are prefixed with the *proto* enum's, so `health_checks::Status` silently harvested nothing; and a same-line `/** */` comment, which protox records against the *next* element, documented every enum value as its predecessor. Both fixed; `names.rs` is now the one home of the naming rules |
| Emission | The `serde` `cfg_attr` (198 byte-identical sites), the `serde(with = ...)` adapter of a well-known-type field (a function of the field's type, and 36 sites of four lines each), the twelve client aliases (section 3.3) and the per-rpc const asserts (16 emitted lines each, 944 across the twelve invocations) come from the macros. `PartialEq`/`Debug` bounds on generic parameters, left over from the deleted `is_default` family, are gone |
| Features | `futures`, `snafu`, `tonic-prost`, `tracing` and `tracing-futures` are optional and hang off the two internal gen features, which is what four configurations needed to pass CI's own lint. A matrix job runs that lint over each of the seven a user can select |
| Registry | A `#[cfg(test)]` module rather than a `_differential` feature enabled through a dev-dependency of the crate on itself, so what the suites link is the artifact the crate ships. `DESCRIPTOR`, `Role::Message`'s unmatched form and the `Diff`/`Entry` mirror are gone (section 3.8) |
| Codec | `ProtoOneof` is deleted: its three methods were signature-equivalent to `prost::Message`'s, and the fifteen whole-message enums implemented both, one forwarding to the other. The four hand-rolled length-framing sites go through `prost::encoding::merge_loop`, which loops on the same buffer, so the transparent chain needs no `dyn Buf` recursion and all four gain prost's recursion limit and exact-length check. `PairMap` loses const generics it never varied |
| Resolution | The plan carries the descriptor's own kind and cardinality; `Card` and `FieldChecks`, which mirrored the codec's `Cardinality` and `Expect` behind four laundering functions, are gone. Two validation branches with no user go with them: a struct standing for several protos, and a `tag` restating the descriptor |
| Client | One `.call()` for all three RPC kinds (section 3.5, section 5.17), and each rpc line spells its projection (section 3.7) |
| Server | The router implements `Service` for any request body, as a generated tonic server does. It was fixed at `tonic::body::Body`, which nothing caught because `add_service` asks for exactly that; mounting on plain hyper, nesting under axum or layering anything that changes the body all failed. `tests/server_mounting.rs` is the coverage that was missing, and the client-supplied path the UNIMPLEMENTED status repeats back is now bounded |
| Tests | `codec/tests.rs` goes from 707 lines to 177: what is left is the four properties the differential harness structurally cannot see, on real object types, plus the leaf impls no API field instantiates. The mock-server cases are `#[ignore]`d, and CI runs them with `--include-ignored` |

## 11.2 Settled decisions

Rejected, each by a specific fact. Refute the fact before proposing any of them
again.

- **Reverting to prost-derive plus a conversion layer.** `armonik-macros` is a
  fixed cost against a growing schema; it replaced ~6,700 lines of `From` impls
  that were O(messages) and drifted silently.
- **Removing the `Shape`/`Expect` const asserts.** They are the only thing
  validating the Rust side: every field emits
  `<Ty as ProtoField>::encode_field(tag, ..)`, so any `ProtoField` type
  type-checks at any tag. Two bug classes they catch uniquely, both of which
  compile clean *and pass the differential harness*: same-kind enum substitution
  (`StatusCount.status: TaskStatus -> ResultStatus`, invisible because both
  `From<i32>` conversions are total and mutually inverse) and integer widening
  (`page: i32 -> i64`, invisible because prost's int32 varint is sign-extended).
- **Deleting `codec::Msg` and blanketing over `prost::Message`.** E0119 seven
  times ("upstream crates may add a new impl of `prost::Message` for
  `Option<_>`"). `NAMES` cannot move to an inherent const, and dropping it
  silently loses the wrong-message-type check on ~55 field sites. Best variant
  saves zero lines.
- **serde-style free-function modules instead of `ProtoAdapter<T>`.** Built end
  to end: +11 lines, diagnostics go from one trait-bound error to two or three
  argument-type ones, and const generics are lost.
- **Delegating the packed enum helpers to prost.** Needs a materialized
  `Vec<i32>`: two heap allocations per repeated-enum field per encode, to save
  27 lines. prost cannot help at all here, because it declares both
  `varint!(i32, int32)` and `varint!(i32, sint32)`, so the Rust type cannot
  determine the codec.
- **Dropping the `stream` keyword from the grammar.** Redundant with the
  descriptor, which the macro asserts against, but it decides the wire framing
  and it is what a reviewer reads.
- **The terser `rpc ListResults(list);` form** (section 5.15).
- **De-GATing the client into three inherent methods.** `.call()` must return a
  stream for a server-streaming RPC, because the request type conveys everything
  about the RPC. Section 5.17 went the other way.
- **Making the four use-case features diverge in emission.** Measured: -17%
  check time on `agent`/`worker`, which is 0.57 s out of a 65 s cold build, and
  0.03 s on the default `client`. It costs 58 lines, a second per-service place
  to keep in step, and five permanent `#[allow]`s blunting `dead_code` in every
  configuration.
- **Convenience methods taking `impl Into<Request>`** (~-700 lines), and
  **deleting the generated convenience layer** (~-430). Both rejected by the
  owner: the spread-parameter ergonomics are the point of the layer.
- **Turning `FieldKind::Unsupported` into an `Err` from `field_kind`.** Proposed
  as "the same protection for 6 lines instead of 32". It is not the same: it runs
  while building the descriptor index, which every derive loads, so one
  `sfixed32` anywhere in the schema would fail the index and report the same
  error at ~200 derives, none of them spanned at the field. The current form is
  one error at the Rust field that maps it, and only if one does.
