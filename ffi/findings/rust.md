# Reading the rust slice

Phase note: this file records facts only; container timings were removed on 2026-09-24 (design/FIX-PLAN.md WP2). The raw logs remain in logs/rust/.

The aggregating session's reading of `poc/rust` (the slice's own record is
`poc/rust/STATE.md`). It lists what was built, what was checked, what the
interface counts are, which defects were found and by what, and what is not
established. It contains no performance result and no recommendation (README
section 1.1).

## Configuration

rustc 1.94.1, release, PIE. prost 0.14.4, prost-build 0.14.4, protox 0.9.1,
tonic 0.14.6 (stage 6). The core `ak-core` is a `cdylib` resolved by the dynamic
linker; the harness checks this from the built artifact (below). Shared 4 vCPU
container, no pinning.

**Floor**: MSRV 1.88 is declared and **not verified**. No 1.88 toolchain exists in
the container, so README section 5.2's arm a is unverified as being at the floor
(slice defect D2, open). For Rust the floor and the target are the same code.

## What was built

- **Arms**: `prost` (incumbent; tonic-prost 0.14.6 calls `Message::encode` and
  `Message::decode`, so for Rust the production path and the library entry point
  are the same call and R14 has no second row); `armonik` (facade types with
  generated `prost::Message` impls, the `packages/rust` pattern, without
  `armonik-macros`); `core-native` (the generated traversal in the host, no
  boundary); `core-ffi-rust` (the same traversal through the C ABI, push family);
  `core-native-noinline` and `core-native-opaque` (inlining controls); the pull
  arms `core-ffi-pull`, `core-ffi-pull-walk`, `core-ffi-pull-opaque`,
  `core-ffi-parse-only`; a zeroed-group fill arm (open decision 9); an
  unknown-field bag arm (open decision 11); three decode UTF-8 policy builds
  (lossy, reject, reject with SIMD) and the encode transcoders `ak_tc_utf8`,
  `ak_tc_utf8_simd`, `ak_tc_utf8_trusted`.
- **Shapes and payloads**: every message and payload of `design/SHAPES.md`
  (M1 to M7, P1.1 to P7.1).
- **Content sets**: ascii, latin1 and wide on every payload
  (`stage5-content-all.log`): 16 payloads in correctness, 11 in the timing pass.
- **RPC**: a unary arm against tonic over loopback h2 (`stage4-rpc.log`), then
  section 9's three deliveries (blocking, callback, completion queue) and an A/B/C
  grid (A prost + tonic, B prost + core transport, C core + core) over a Unix
  socket and loopback TCP, with ArmoniK's window and message settings pinned
  through `ak_client_new_opts` (`stage6-rpc-grid.log`). Server in-process.
- **Pull decode family** (ABI v1 section 7.1): `ak_parse_<Root>`, the `ak_bdr_*`
  record buffer, one `dec_walk` emitted once and instantiated for both families
  (`stage5-pull-decode.log`).
- **Concurrency suite** (obligation 12.5), codec half only
  (`stage5-concurrency.log`).
- **Lifecycle** (ABI v1 section 3): `ak_init`, options and flags, the version
  check, `ak_build_id`, the log bridge, the panic hook, the
  `AK_ERR_UNINITIALIZED` guard behind feature `init-guard`
  (`stage5-lifecycle.log`).

All additions to the shared core at `poc/codec/` were additive (stage 5's first
commit: 2,048 insertions, zero deletions) and off by default or new surface.

## Correctness results

- **The manifest is an oracle.** Every payload except P7.1 is byte-identical
  between prost and a second encoder sharing no code with it; P7.1 cannot be
  produced by a canonical writer and is validated by decode. Re-validated 16 of
  16 after the adapter fix and after the packed-enum schema change
  (`stage1-manifest-vs-prost-after-fix.log`,
  `stage1-manifest-vs-prost-packed-enum.log`).
- **All four encoder arms are byte-identical to the manifest** on every payload
  and decode their own output to an equal value. `prost` and `armonik` are
  independent encoders; `core-native` and `core-ffi-rust` share the codec and so
  validate only the binding.
- **Pull arms are gated by value identity** on 16 payloads over 7 roots against
  the `armonik` and push decoders, including M7 (two interleaved repeated fields
  of one type, which exercises the section 7.3 flush on a foreign tag).
- **M3**: explicit presence exercised as three cases (absent, present-and-zero,
  present-and-nonzero) on all three `optional` fields, all four arms agreeing; the
  payload-free oneof member occurs 40 times in 200 elements.
- **Unknown fields**: seven hand-built vectors, all four arms agreeing on decoded
  value and re-encoded bytes. At the wire level an unknown oneof member is an
  unknown field (the case stays at the last known member and the payload is
  dropped); an unknown enum value round-trips losslessly. With the bag arm, the
  bag's bytes are preserved exactly, but the layout is not when an unknown tag
  lies between two known ones, because the bag is appended rather than merged; a
  message round-tripped through the core is then not byte-comparable with one
  round-tripped through protobuf-java.
- **Separate construction routes caught a defect** (D15): after the adapter
  change the three generated-builder arms matched the new hashes and the
  hand-written prost builder did not.

## Crossing counts

Counted from a counting build unless noted.

| what | count | log |
|---|---|---|
| M1, 1,000 rows | 9 to encode, 6 to decode (the drafted ABI: 15,137) | `stage2-four-arms-M1.log` |
| M2, per `TaskDetailed` | 10.024 to encode, 7.004 to decode (ABI v1 7.2 predicted 7; drafted ABI 43) | `stage3-M2-M4-revalidated.log` |
| M3, 200 elements | 3 in each direction (oneof and optionals ride in the group) | `stage3-M3.log` |
| one unary RPC | 2, and 0 per field (read from the code: neither `crates/rpc` nor ak-core's rpc module mentions a message type; `gen/rpcgrid.sh` step 1 checks it) | `stage4-rpc.log`, `stage6-rpc-grid.log` |
| pull, reverse calls | 0 on all 13 counted payloads | `stage5-pull-decode.log` |
| pull, records vs push reverse calls | equal on all 13 (P2.2: 3,501 and 3,501) | `stage5-pull-decode.log` |
| pull, forward calls per decode | 3 if the host sizes its chunk to the footprint; 16 on P2.2 at 32 KB chunks | `stage5-pull-decode.log` |
| init guard | per entry point, so one guarded crossing per decode; P2.2 encode makes 2,511 forward crossings | `stage5-lifecycle.log` |

Counts were unchanged to the digit after stage 5's core additions. Related
structural facts:

- **Decision 5 (learned prefix width)**: zero warm prefix misses and zero bytes
  moved on every uniform payload; on P2.4 one miss per element, 980,938 bytes
  moved of a 981,222-byte output; zero grow-callback invocations anywhere.
- **Pull record stream**: on P1.3 it is 38,488 B for a 605 B message (a record
  carries an absent element's whole fixed group), a property of the shape.
- **Unknown-field bag**: as one opaque bytes blob it changes the leafness of no
  message; as a repeated field it takes the schema from 9 leaf messages to 0 and
  every batched run fails (`gen/unknown_predicate.py`).
- **Batching predicate**: it saves decode crossings and not encode crossings,
  because on encode the host drives its own containers (ABI v1 section 6).

## Concurrency suite and lifecycle

- **Codec half clean**: 0 wrong out of 2,840 encodes and 2,840 decodes across 2, 4
  and 8 threads with a context each, phases offset, every encode compared byte for
  byte with a single-threaded reference and every decode by value.
- **Section 6's two refusals built separately**, four builds with required
  outcomes, all as required: shipped 0 wrong; `global-widths` 0 wrong (a data
  race, not a byte defect); `pad-widths` 10 wrong; both together 1,410 wrong.
- **The oracle was changed**: re-encoding with a fresh `core-ffi` context sees 0
  wrong on the combined build where prost sees 4, because a global table pollutes
  the "fresh" context too. The suite now uses prost.
- **Contention arm checked to contend**: a warm context on one shape misses zero
  length prefixes; the same context alternating the pair misses exactly one per
  encode.
- **Lifecycle**: fourteen cases, each in its own process. Eight threads racing
  `ak_init` give exactly one `AK_OK` and seven `AK_ALREADY_INITIALIZED`, and no
  caller returns before the installs are visible. The core's panic hook does not
  see a Rust host's panics (a cdylib carries its own `std`). Init flags are a
  process-wide negotiation where the first caller's flags apply; section 3 does not state
  this.

## Defects found, and what found them

- **Panic across `extern "C"` aborts the host** (concurrency suite positive
  control: four threads on one context). The core panics inside `Enc`, the unwind
  through the `extern "C"` entry point is refused, the process aborts. Section 5's
  error channel has nothing for a panic inside the core; section 3's panic hook
  changes what is printed, not whether the abort happens. Raised as an ABI change
  (an owning-thread id beside the context's `kind` word), not taken.
- **D3, entry points inlined into the host** (disassembly). With the core as an
  rlib, the release binary had zero call sites to the encode, decode and element
  entry points, and the counting build still reported 3, 9 and 6 because the
  counters were inlined too. Fixed with a cdylib; `nm -D --undefined-only` on the
  harness lists the thirteen ABI entry points as undefined, checked every run.
  Exposure: Rust and C++ (`-flto` over a static core); managed hosts cannot inline
  across the boundary.
- **D9, open-field state not restored by an element run** (a length-prefix site
  that would not converge). A chunking host wrote every chunk after the first
  under the inner field's tag. Byte identity passed throughout because
  `ListTasksDetailedResponse.tasks` and `TaskOptions.options` are both tag 1; no
  root in `design/SHAPES.md` can catch this class, hence corpus requirement 5.
- **D16, RPC client mutated shared state** (8 calls in flight in stage 4, by
  accident). Worked at 1 in flight, failed at 8. The failure surfaced as
  `AK_ERR_HOST` with no cause attached, because ABI v1 has no channel for a source
  chain (now part of open decision 12).
- **D17, sticky decode error slot never cleared**: one rejected decode poisoned
  every later decode on the context. Now a stated requirement in ABI v1 section 5.
- **D20, empty string pointer equals `AK_STR_DIRECT`** (content sets on every
  payload, first run). `as_ptr()` on an empty slice returns 1, the section 8
  sentinel, so empty strings took the direct-argument path and produced correct
  bytes only while the context had never encoded `UploadResultDataMessage`. Fixed
  in the binding; the ABI hazard (a sentinel in a range an empty buffer can
  occupy) is open and the other bindings are unchecked.
- **D21, `ak_client_opts` declared with 4 fields while the core read 6**
  (reconciliation review). Fixed; a const block now asserts size, alignment and
  every field offset, verified failing on the old declaration.
- **M4 adapter states unreachable from the payload set** (checking the adapter by
  state). The nested site produced (true, non-empty), which
  `TaskDetailed.Output` forbids; at the plain site Ok and Invalid both flatten to
  the empty string, so one must come back wrong whatever the adapter chooses. The
  generator now cycles the three states: 167 Ok, 167 Error, 166 absent at P2.2's
  nested site; P4.1's plain site 67 non-empty against 133 empty-or-absent. Schema
  fact surfaced: `TaskDetailed.Output` (`{bool success, string error}`) and
  `objects.proto`'s `Output` (`oneof {Empty ok, Error error}`) share a name.
- **Packed repeated enum falsely marked covered** for M2; now on M6 as
  `MetricsBatch.statuses` (`945d3cd1`).
- **Generator defects**: D4 (`all_absent` precedence, visible only on P1.3), D5,
  D6, D8 (depth two and beyond only), D12 (oneof members ignored by every
  backend), D13 (`direct_fields` refused nothing).
- **Design fact**: four of five incumbents (Google.Protobuf, protobuf-java,
  protobuf C++, upb) retain unknown fields from decode to re-encode; prost does
  not, and the core follows prost. This is ABI v1 open decision 11.
- **Design fact**: a oneof is laid out as a discriminant plus every member flat,
  not a union, so group layout stays reproducible by hand (ABI v1 section 6).
- **Design fact**: encode-side UTF-8 validation was removed (decision 3):
  `ak_tc_bytes()` and `ak_tc_utf8_trusted()` return the same function pointer in
  this tree; decode validates. `ak_tc_utf8_simd` has the same accept and reject
  sets as the scalar validator (checked in the source).

## Harness facts

- **In this slice's grid, `B - A` is not a transport comparison**: cell A (prost
  on tonic) and cell B (the generated codec's host side on the core's transport)
  both run on tonic, so the host transport and the core transport are the same
  stack here (`logs/rust/stage6-rpc-grid.log`).
- **D19**: the root `.gitignore`'s `[Bb]in/` excluded every measurement binary
  until `7fb30be5`, so logs from `cc7f68c6` and `d03c5161` have no committed
  harness and cannot be re-derived. `ffi/.gitignore` now re-includes `bin/`.
- **A counter is not evidence that a call happened** (D3). R5 requires showing
  from the artifact that entry points are unresolved imports.
- **The no-boundary control must be checked in the other direction too**:
  `gen/inline_check.sh` compares the calling closure's size with the traversal's
  (472 bytes against 4,299 and 11,311 in the audited build). With LTO off and
  non-generic, non-`#[inline]` entry points the traversal cannot cross into the
  harness; an `#[inline]` or generic entry point, or LTO, would change that.
  `core-native` is therefore "no boundary, still a GOT-indirect call", not a fully
  inlined native codec.
- **D14**: the `core-native` M4 to M7 encode case allocated and grew a fresh `Vec`
  per call while other arms reused a buffer.
- **D18**: running policy builds in a fixed order made the always-first build
  read differently between invocations; round robin with rotating order fixed it.
- **Two-build comparisons failed their control** (`gen/guardprice.sh`):
  `core-native`, identical in both builds, moved between them. The guard question
  was re-asked within one process with a twin arm that measures the method's
  floor.
- **A within-arm delta and a cross-arm ratio to a third arm behaved differently
  across sessions** on the same finding (content sets); R4 records this.
- **Nagle on the stage 4 server socket**: `rpc::serve` had Nagle on while tonic's
  client had it off, which governed loopback-TCP wall time (not HTTP/2 flow
  control, as first stated). `rpc::serve` now sets `TCP_NODELAY`;
  `rpc::serve_nagle` keeps the old form. Every loopback-TCP figure taken before
  the change is affected.
- **Stage 6 asserts only response length** at 8 and 16 in flight: a load, not a
  correctness suite.

## Open review findings (2026-09-24, unconfirmed)

From `design/FIX-PLAN.md` section 7. None is confirmed by the slice agent yet.

- **R-C14**: `poc/rust/STATE.md:110-137` still shows the retired M1/M2 table.
- **R-C16**: the crossing benchmark is bimodal between two builds, and the
  init-guard bound is quoted against a control resolution far coarser than it.
- **R-D1**: length-varint wrap in the shared core (`ak-rt` `dec.rs`): hang,
  out-of-bounds span to the host, panic across `extern "C"` (source verified, not
  run).
- **R-D6**: encode entry points ignore the sticky error slot.
- **R-D8**: the concurrency suite's `together()` uses only absent-path payloads
  (P1.3, P2.5); P1.2 and P2.2 to be added, ThreadSanitizer if available.
- **R-D9**: minor boundary items, including `from_raw_parts(null, 0)` in
  `tc_utf8*`, `u32` span truncation over 4 GiB, and `lifecycle.sh` /
  `guardprice.sh` no longer building a guard-off arm since `init-guard` became
  default.
- **R-E1**: the "one traversal" claim in `rust_core.py` is false; `core-native`
  and `core-ffi-rust` do not come from one plan.
- **R-A7**: Rust MSRV unverified.
- **R-A9 / R-C8**: no RPC arm measures streaming or the worker path.

## What is not established

- **No performance result.** Every timing in `logs/rust/` is container
  instrumentation, including the crossing cost, the RPC grid, the pull and push
  comparison, the zeroed-group fill, the unknown-field bag, the guard and the UTF-8
  policies. The campaign (W13) measures these.
- **The floor**: MSRV 1.88 not built or run.
- **Nesting beyond 4 levels**: the payload set carries responses only; the real
  schema's depth 6 lies on filter and request messages. Decision 7 (recursion
  limit) is unexercised.
- **Content sets**: no manifest oracle for latin1 or wide (`ffi/schema/` emits
  ASCII only), so correctness there is arm against prost arm. SIMD validator
  behaviour without AVX2 is not tested.
- **Transcode pair** (README section 10 item 4): unreachable, a Rust `String`
  cannot hold an unpaired surrogate.
- **Direct-argument path** (section 8): built and byte-identical; on a Rust host
  there is nothing to pin, so nothing here bears on it.
- **`packages/rust` itself**: the `armonik` arm reproduces its pattern without
  `armonik-macros` and does not price the in-repo crate.
- **RPC half**: metadata, deadlines, numeric status, retry and backoff, TLS,
  streaming, a real network, failure injection and the server side are not built;
  `ak_call_cancel` is built and never called; configuration precedence and
  `worker_threads` defaults are unexercised. Rust has no carrier thread, so the
  idiomatic-wait requirement is not testable here. Server always in-process.
- **Concurrency**: RPC half not in the suite; no race detector run; two shapes
  only.
- **Pull family**: not wired to unknown-field capture; `ak_bdr_reserve` never
  called; `AK_ERR_CAPACITY` on an oversized record never triggered; no
  `core-native-pull` control.
- **Error paths**: encode-side rollback of a half-written field is written and
  never triggered; `AK_ERR_MALFORMED` and `AK_ERR_TRUNCATED` have no vector;
  malformed UTF-8 is exercised by one vector and returns `AK_ERR_TRANSCODE`
  (whether `AK_ERR_MALFORMED` is the right code is open).
- **Specified and unbuilt**: group layout export and load-time assert, the
  `ak_span.coder` hint, size and recursion limits, the unbatched decode element
  form, the opt-in diagnostic encode mode, merge-by-tag for the bag.
- **Unknown-field bag**: non-empty bag throughput, decode-side crossings for
  unknown runs, a oneof's message member and inner slots of non-leaf elements.
- **Allocation and footprint**: not counted in any arm.
- **Linkage**: shared library only; a static link is a different mechanism.
- **Corpus** (`ffi/corpus/`): not consumed by this slice.
