# Design: RPC definitions, and dropping the stub codegen

Status: **accepted** (successor to `DESIGN.md`, which covered the message layer;
decisions settled in review, 2026-08-03)
Target: same big-bang beta branch (`rust/direct-message-impls`).
Prerequisite: the direct-message revamp (`DESIGN.md`) landed.

> **The spike (§8 step 0) has landed.** The table-driven router (§3.6) and the
> client dispatch machinery (§3.5) are implemented for `Results` and pass the
> full `tests/results.rs` scenario matrix in all three old/new combinations
> (`tests/results_spike.rs`), plus three calls against the dotnet mock over a
> real HTTP/2 connection. Everything that remains is subtraction.

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

The duplication has already drifted, which is the better argument than the line
count:

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

Estimated deletions: `armonik/build.rs` (344), `stubs.rs` (47), client dispatch
(~870), client per-service boilerplate (~330), `Client` accessor pairs (~190),
the two server lists (~210), `ServiceExt` boilerplate (~145), the 19
`#[armonik(replace(...))]` annotations (~110), `ReplaceSpec` + its emitter
(~90), the build-facing half of `wire.rs` (~100), the hand-maintained
`UNEXPOSED_RPC_MESSAGES` allowlist, per-RPC path tests (~1500, of which ~200
return as the per-service round-trips of §4). Roughly **3,900 lines**.

Estimated additions: the `service!` proc macro, validation and doc harvest
included (~480), the generic router plus the three `serve_*` bodies (~200),
`ServiceClient` + `Dispatch` (~150), the `Rpc`/`Service`/kind markers and the
`Channel` bundle (~120). Roughly **950 lines**.

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
(`DESIGN.md` §3.1) were a pure-types dependency and, "the reason the split earns
its keep", the extern-map harvest across the build-dependency edge. The harvest
is deleted by §3.2, and no consumer of the pure-types build exists in the
repository. By the design doc's own test the split stops earning its keep.

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

### 3.7 The convenience layer stays hand-written

The ~50 convenience methods are kept as they are: hand-written inherent impls
on the client aliases, bodies unchanged (they already build the request structs
inline and go through `call`). An earlier draft had `#[derive(Message)]` emit a
`Request::new(..)` constructor to shrink the bodies; it is dropped. It added
~150 lines of macro machinery and a positional-argument constructor whose arity
would track the proto, to save bodies that are already one struct literal each.
The one true duplication in the layer, the nested-filter collect copy-pasted
into six `list` methods, becomes a single private helper.

Whether a `macro_rules!` over the common method shape earns its keep is
measured after the cutover, with real numbers, not before (§9): this is the
most user-facing code in the crate, and compressing it trades hover-docs and
readability in exactly the wrong place.

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
in `DESIGN.md` §7) for no consumer. Superseded outright by §5.2 and by the merge.

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
  (`DESIGN.md` §6, request types foreign to the client crate) is moot after the
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

## 8. Migration plan

Each step compiles, is separately reviewable, and is committed on its own.

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
11. Docs: fold this document into `DESIGN.md`, update its §1.1 ledger (this
    round genuinely reduces hand-written code), §3.1 (crate layout), §3.2 (one
    build script), §6 (the breaks above, and the stale `call_streaming`
    rationale). CHANGELOG, version bump, release pipeline down to
    `armonik-macros` + `armonik` pinned, `armonik-transport` unlocked.

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
