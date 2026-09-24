# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the handoff.
It says what exists and what was checked. It carries no recommendation (the
decision is the owner's) and no timing presented as a result: in the setup and
design phase every container timing is instrumentation (README 1.1). The
previous version of this file mixed both in and contradicted itself; it is in
git history at `817174f` and its reasoning is in `JOURNAL.md`.

| | |
|---|---|
| **Status** | Correctness gate green on the target against the core at `6ede244` (`stage21-wp4-regate.log`). WP4 item 10 (R-D9) C# part fixed and gated. Not campaign-ready: the WP3 harness changes and the WP5 generator port are not done |
| **Owner's levels** (FIX-PLAN section 6, D2) | floors **net6.0** and **.NET Framework 4.8** (correctness only), target **net8.0** |
| **Target** | net8.0 on .NET 8.0.31, SDK 8.0.131 (Ubuntu 24.04 `dotnet-sdk-8.0`). Builds and passes everything below |
| **net6.0 floor** | **does not build with the core-ffi binding**: the generated `Abi.cs` uses `[LibraryImport]` (.NET 7+). The managed half (facade, managed codec, corpus consumer) builds and passes on the .NET 6.0.36 runtime. See "Levels" |
| **net48 floor** | **no .NET Framework evidence exists.** Every net48 result in this slice's history was taken on **Mono 6.8.0.105**, which is not .NET Framework and does not count as the gate (FIX-PLAN WP3 item 17). .NET Framework runs only on Windows; this container has neither it nor Mono now |
| **Incumbent** | `Google.Protobuf` 3.28.3, `Grpc.Tools` 2.66.0, `Grpc.Net.Client`/`Grpc.AspNetCore` 2.66.0. The owner names 3.32.0 and 2.71.0 (the `packages/csharp` versions, WP3 item 17); not moved yet |
| **Core** | the one core at `ffi/poc/codec`, built from a `git archive HEAD` snapshot (last core commit `6ede244`, branch HEAD `817174f`) into `target-core` (`rpc`) and `target-core-count` (`rpc count`) |
| **Machine** | a container, 4 vCPU Intel Xeon @ 2.80GHz, Linux 6.18.44. Nothing here depends on it except the instrumentation logs |

## Levels

What the owner fixed, and where the slice stands against each level.

| Level | Role | What builds | What passes | Evidence |
|---|---|---|---|---|
| net8.0 | target | everything: facade, managed codec, core-ffi binding (all 7 shapes), strict build, corpus consumer, RPC arm | all gates below | `stage21-wp4-regate.log` |
| net8.0 with floor sources (`/p:AkFloor=true`, README 5.2 arm b) | floor sources on the target runtime | facade, managed codec, core-ffi binding | conformance 152/152, unknown, groups, corpus (same line as arm a) | `stage21-wp4-regate.log` |
| **net6.0** | floor | facade alone; the managed floor harness (`src/HarnessFloor` retargeted to net6.0 in a scratch copy, core-ffi excluded as on net48) | on the .NET 6.0.36 runtime (self-contained, runtime pack from NuGet): conformance 152/152, unknown, groups, corpus (same line as arm a) | `stage21-wp4-regate.log` ("net6 managed floor"), `wp3-17-net6-floor-build.log` |
| **net6.0 with the core-ffi binding** | floor | **does not build**: 57 `[LibraryImport]` declarations in `src/Harness/Generated/Abi.cs`, 171 errors (CS0246, CS8795), no error elsewhere. `src/Rpc/CoreTransport.cs` carries 18 more `[LibraryImport]` by hand | - | `wp3-17-net6-floor-build.log` |
| **net48** | floor | on Mono 6.8 only, historically, with core-ffi excluded (net48 has neither `LibraryImport` nor `UnmanagedCallersOnly`) | on Mono 6.8 only: conformance 152/152 (`stage1-conformance.log`), corpus (`stage13`) | **not .NET Framework**; not re-run in this unit (no Mono in this container) |
| netstandard2.0 | the shipped client's TFM (`packages/csharp` Client) | the facade (`AK_FLOOR` sources) | through arm b and the net6/Mono runs above | |

`[UnmanagedCallersOnly]` (.NET 5+) exists on net6.0: it is a net48 problem only.
The fix for both floors is WP5's one generated binding with `DllImport` under
`#else` of `#if NET7_0_OR_GREATER` (and delegate pointers, rooted for the
vtable's lifetime, where `UnmanagedCallersOnly` is missing). Not attempted here:
it belongs to WP5.

## What exists

```
gen/generate.py [--check]  the slice generator. Imports ffi/schema/emit/shapes.py (R1)
gen/ir.py, csnames.py      IR (walk() with the oneof walker guard) and naming
gen/cs_facade.py           facade types and a generated structural comparer
gen/cs_values.py           value rules and content sets
gen/cs_build.py            payload construction, emitted over two object models
gen/cs_managed.py          the managed codec: Write, SizeOf, WriteSized, Read
gen/cs_arms.py             the per-payload arm table
gen/abi_ir.py              by-value groups, presence bits, loop slots, vtables
gen/rs_probe.py            the Rust layout probe (abi/), from abi_ir
gen/cs_abi.py              the managed ABI declaration + ak_init + layout asserts
gen/cs_core.py             the host binding for every root (Core_*.cs, CoreArms.cs)
gen/protoparse.py          second front end over corpus.proto, cross-checked with shapes.json
gen/cs_coreffi.py, cs_coreffi2.py   DEAD: superseded by cs_core.py at b59139a, not called
abi/                       the layout probe (Rust bin, links ak-abi read-only)

src/Facade/Wire.cs         THE hand-written runtime: varints, buffers, the reader
src/Facade/Generated/      Types, Eq, Values, Build, Codec
src/Harness/               harness: conformance | unknown | groups | corpus | utf8 |
                           content | coreffi | mapforms | counts | bench
src/Harness/Generated/     BuildGp, Arms, Abi, Core_*, CoreArms
src/Harness/Corpus/        the corpus consumer's codec, generated from corpus.proto
src/HarnessFloor/          net48 build of the same sources, linked (core-ffi excluded)
src/Rpc/                   akrpc: grpc-dotnet client and server over a UDS, the core's
                           transport bound by hand (CoreTransport.cs), --error-path gate
src/BenchDotNet/           BenchmarkDotNet harness. DOES NOT BUILD (open defect D1)
```

Builds: default (arm a), `/p:AkFloor=true` (arm b), `/p:AkStrict=true`
(ABI v1 decision 3's rejecting UTF-8 policy), `/p:AkCount=true` (the host's
counting build), `src/HarnessFloor` (net48), `src/Rpc`.

Arms that exist, correctness-gated (no timing claimed for any of them):
`gp-tobytearray`, `gp-writeto`, `gp-marshaller` (the stub's
`CalculateSize` + `WriteTo(IBufferWriter)`, the R14 production path),
`gp-bufferwriter`, `gp-parse`/`gp-parse-seq`/`gp-parse-seg`, `managed`,
`managed-2pass`, `managed-parse`, `core-ffi` (push decode), `core-ffi pull`
(ABI v1 7.1), `core-ffi utf16`, `core-ffi fill`, `core-ffi no-string` (a ceiling
for decision 13, not an implementation), the strict build, and the RPC arm
(cells A/B/C/D, all three section 9 deliveries, streaming both directions).

## What was checked, and holds today

All in `stage21-wp4-regate.log` unless stated; core snapshot `6ede244`.

**Generator and layout.** `gen/generate.py --check` clean. The layout probe,
rebuilt against the snapshot's `ak-abi`, is identical to `gen/abi-layout.json`:
42 structs and 28 vtables match the Rust build, `ak_abi_version()=1`,
`ak_init()=0`.

**Byte identity (`harness conformance`).** 152 checks, 0 failures, on arm a,
arm b and the net6.0 managed floor: every encode arm on all 16 payloads against
`ffi/schema/generated/manifest.json`, decode-then-re-encode, and decode-then-
compare through the generated comparer. P1.3 and P2.5 (absent path) included.
P7.1 is checked as a permutation of (tag, wire type, body) triples. (Older logs
show 136 checks: the check set grew with the arms; each log's count is the set
of its commit. The figure "120" in the previous STATE had no log.)

**Unknown fields (`harness unknown`).** Seven hand-built vectors (root varint,
root length-delimited, inside element 0 at tag 7, fixed64, fixed32, nested
message body, unrecognised oneof member): all decode in both arms, no known
value lost. `Google.Protobuf` retains and re-emits them; the managed codec and
core-ffi drop them. There is no retain mode yet (D4 is the owner's, WP3 item 21).

**Group skip (`harness groups`).** The five corpus GROUP vectors pass; the
depth bound returns `ErrDepth` at 200 and at 200,000 nests with the process
alive.

**Corpus consumer (`harness corpus`).** 691 vectors of `ffi/corpus` at
`c1c97ff`, codec generated from `corpus.proto` (rule 0) over the same runtime
the arms use:

| class | vectors | result |
|---|---|---|
| baseline, chunking, empty, shape | 209 | all parse, project (where projected) and re-encode to an accepted form |
| unknown | 316 agreed + 1 disputed | all agreed rows pass |
| malformed | 110 agreed + 2 disputed | all 110 refused with an error code (72 truncated, 36 malformed, 2 depth) |
| transcode | 53 | 22 accept rows pass; **31 `T-dec-*` accepted** by the default (lossy) build, **refused by the strict build (0 failures)** |

- **Disputed rows** (CONTRACT.md 1.5), excluded from pass/fail and reported:
  `U-map-entry` reads as protobuf pure-python's reading (entries kept in the
  map), and `Google.Protobuf` keeps the map too; `X-tag-zero-Empty` and
  `X-tag-zero-nested-Empty` are refused (`ErrMalformed`), as by
  `Google.Protobuf`.
- **Incumbent as a second oracle**: accept/reject agrees with `Google.Protobuf`
  on all 395 vectors whose root it has.
- **`Google.Protobuf` is lossy on malformed UTF-8 too** (`harness utf8`: accepts
  all 15 root-site vectors; `UTF8Encoding(throwOnInvalidBytes: true)` rejects
  all 15). The rejecting policy is a behaviour change for .NET.
- **Not claimed** (CONTRACT.md 5.6): C5 (produce) for corpus-only roots; the
  chunking class as chunking (the managed codec does not batch, the core-ffi
  binding does not reach corpus-only roots); `_unknown` comparison (the codec
  drops).

**core-ffi, every shape (`harness coreffi`).** All 16 payloads: encode byte
identity, decode round trip, value identity against the builder's graph, pull
against push on the same bytes, and R5 (the host's crossing tally equals the
core's own `ak_enc_counters`/`ak_dec_counters`, counting build). Also green
against a core built with `init-guard`, i.e. with ABI v1 section 3 enforced.

**Crossing counts** (counting core, whole run handed over in one call; a
property of the interface, not of the machine):

| payload | root | encode fwd | encode rev | push dec fwd | push dec rev | pull dec fwd | pull dec rev |
|---|---|---|---|---|---|---|---|
| P1.1 / P1.2 / P1.3 | ListResultsResponse | 2 | 1 | 1 | 2 / 5 / 3 | 2 | 0 |
| P2.1 | ListTasksDetailedResponse | 7 | 6 | 1 | 8 | 2 | 0 |
| P2.2 | ListTasksDetailedResponse | 2502 | 2501 | 1 | 3501 | 2 | 0 |
| P2.3 | ListTasksDetailedResponse | 627 | 626 | 1 | 876 | 2 | 0 |
| P2.4 | ListTasksDetailedResponse | 402 | 401 | 1 | 561 | 2 | 0 |
| P2.5 | ListTasksDetailedResponse | 102 | 101 | 1 | 141 | 2 | 0 |
| P3.1 | ListProbeResponse | 2 | 1 | 1 | 2 | 2 | 0 |
| P4.1 | ListTaskSummaryResponse | 202 | 201 | 1 | 601 | 2 | 0 |
| P5.1 - P5.4 | UploadResultDataMessage | 1 | 0 | 1 | 1 | 2 | 0 |
| P6.1 | ListMetricsResponse | 1002 | 1001 | 1 | 1401 | 2 | 0 |
| P7.1 | DualResponse | 3 | 2 | 1 | 3 | 2 | 0 |

A leaf element batches, so its count is constant in the element count; a
non-leaf (M2, M4, M6) is linear, per ABI v1 7.2. The M1 decode count is
ceil(n/arena)+1, not 1 (`stage14`). With `AK_CHUNK=150` the host reproduces the
Rust slice's M1 encode counts exactly (`stage10`): the counts are comparable
across slices only when each states its chunk size.

**Transport crossings per call** (ABI v1 section 9, counting core,
`wp4-rd9-bytes-free.log`, same as `stage19`): blocking 2 forward, 0 reverse;
callback 3 forward, 1 reverse; queue 4 forward, 0 reverse. The extra forward
calls are `ak_call_destroy` and, for the queue, `ak_queue_next`. Identical on a
failed call (R-D9 below).

**Prefix-width misses** (ABI v1 decision 5, `stage2-counts.log`, counting
build): zero warm misses on every payload but P2.4, which misses once per
element (80 in 80), site `ListTasksDetailedResponse.tasks`; 0 to 3 cold misses
per fresh context.

**Concurrency** (`stage17-streaming.log`): a shared encode context aborts the
process with SIGABRT out of `ak_encode_*`, which no managed `catch` observes;
one context per thread (`[ThreadStatic]`) runs. The RPC arm ran 1, 8 and 16
calls in flight. The codec gates themselves are single-threaded.

**Recursion limit.** The managed codec enforces 100 (`W.MaxDepth`), added when
the corpus's `X-depth-101`/`X-depth-300` were accepted (`stage13`); both are
refused with `ErrDepth` today. Google.Protobuf's message-size limit has no
counterpart in the managed codec (ABI v1 decision 8, unbuilt).

## This unit (2026-09-24)

- **R-D9, C# part: fixed.** `CoreChannel.CallCbAsync`/`CallQAsync` threw on a
  non-OK status without `ak_bytes_free`; `CallBlocking` did the same. All three
  now free before throwing (`TakeOrThrow`), and `Grid.cs` frees in a `finally`
  when a decode throws. Gate: `akrpc --error-path` calls a method the server
  does not serve on every delivery and requires each failed call to throw AND
  to cross exactly as many times forward as a successful one. With the fix: 0
  failures in three runs. Negative control (the frees removed): each error row
  one forward crossing short, 3 failures. The core returns an empty `ak_bytes`
  on failure today, so no memory was leaking; the binding no longer depends on
  that. `wp4-rd9-bytes-free.log`.
- **Found: the binding never called `ak_init`.** ABI v1 section 3 requires it
  before every other entry point. Every gate passed because the core's guard is
  the `init-guard` feature, off in every build here (the snapshot's `ak-core`
  default features are empty). Against a core built with `init-guard`, core-ffi
  failed 16 of 16. Fixed in the generator: `Abi`'s static constructor calls
  `ak_init` (version from `ak-abi`'s `AK_ABI_VERSION`, flag `NO_CRYPTO`), and
  the transport calls the same `AbiInit.Ensure()`. 0 failures against the
  guarded core. `ak_init_opts`/`ak_err` are declared by hand with a size assert,
  not through the layout probe. `stage21-wp4-regate.log` carries before and after.
- **Found: the managed reader narrowed a length prefix.** `Dec.LenEnd` cast the
  64-bit prefix to `int` and tested `Pos + n > End`, so a prefix of 2^32 + k read
  as k and one near `int.MaxValue` overflowed the check. The new corpus row
  `X-len-huge` reached `ArgumentOutOfRangeException` from `Encoding.UTF8`
  instead of an error code (the runner counted that as a refusal). Now compared
  on the full 64 bits against the bytes left; `X-len-huge` returns
  `ErrTruncated`. The runner now fails a reject row that is refused by an
  exception rather than an error code.
- **Corpus runner**: disputed rows (new in the corpus) are excluded from
  pass/fail and reported with the reading this codec produced; an accounting
  bug that printed "-1 other" and exited 255 on a clean run is fixed.
- **Harness header**: the harness printed a fixed crossing calibration
  ("1.8 ns ... on THIS machine") on every run on every machine; removed.
- **net6.0 floor recorded** (build log above). **R-F1**: this file rewritten.

## Open defects

| # | Where | What |
|---|---|---|
| D1 | `src/BenchDotNet` | does not build since `b59139a`: references `CoreFfiM1`/`CoreFfiM2`, removed when the binding generator was unified. `gen/cs_coreffi.py` and `cs_coreffi2.py` are the dead emitters of those classes |
| D2 | `src/Harness/Generated/Abi.cs`, `src/Rpc/CoreTransport.cs` | no net6.0/net48 build of the core-ffi binding (`LibraryImport`); WP5 |
| D3 | `src/Rpc/CoreTransport.cs` | the transport binding (`AkRpc` imports, `AkBytes`, `AkCompletion`, `AkClientOpts`) is hand-written, not generated; outside the one generator (W14) |
| D4 | net48 | no gate on .NET Framework; needs a Windows machine |
| D5 | `src/Harness/*.csproj`, `src/Rpc/Rpc.csproj` | incumbent at 3.28.3 / 2.66.0, not the owner's 3.32.0 / 2.71.0 |
| D6 | RPC harness | not WP3-conformant: server in the client's process; CPU from `Process.TotalProcessorTime`; cells B/C stay pinned under `--shipped` while A/D follow it (R-C13); min-of-N reported instead of committed per-round values |
| D7 | managed codec, core-ffi | no unknown-field retain mode (WP3 item 21) |
| D8 | `src/Harness/MapForms.cs` | the harness's own rewriter narrows a length prefix to `int` the way `Dec.LenEnd` did; it walks only bytes this slice emitted, so it is not reachable from the corpus, but it is the same class |
| D9 | `gen/cs_abi.py` | `ak_init_opts`/`ak_err` are not in the layout probe; size-asserted only |

## Facts established by reading, not by running

- `packages/csharp` ships the client as netstandard2.0 and the worker as net6.0.
  It makes no direct message serialisation call: everything goes through the
  `Grpc.Tools` marshaller (`SetPayloadLength(CalculateSize())` then
  `WriteTo(bufferWriter)`; decode `ParseFrom(PayloadAsReadOnlySequence())`).
- `packages/csharp`'s `GrpcChannelProvider` pins no HTTP/2 window; on the UDS
  path it sets `Http2FlowControl.DisableDynamicWindowSizing` (a workaround for
  grpc-dotnet #2361). On .NET the connection window is hardcoded at 64 MiB
  (`Http2Connection`) and `InitialHttp2StreamWindowSize` is where the stream
  window starts, not a cap, unless dynamic sizing is disabled.
- `packages/rust/armonik-transport`'s `ClientConfig` has no window field.
- `ak_client_opts` has six fields, including `tcp_nagle`.

## What is not measured or not established

- **No timing in this slice is a result.** Every timing log in the index is
  container instrumentation; its figures are not quoted here and are not to be
  quoted from here. Timing waits for the campaign (`design/CAMPAIGN.md`, W13).
- net48 on .NET Framework; net6.0 with the core-ffi binding; the RPC arm on
  any floor.
- Unknown-field retention in the managed codec or core-ffi.
- Google.Protobuf's message-size limit (decision 8) in the managed codec.
- ABI v1 decisions 4, 6, 10, 12 on .NET; decision 13 is bounded by the
  no-string arm, not built; decision 9 (sparse fill) not offered by the ABI.
- A zero-copy string form (pinned managed pointers handed to the core).
- The `[UnmanagedCallersOnly]` abort guard removed (the only way to price it;
  removing it turns a managed exception into a process abort).
- Nesting past depth 3 in the payload set; the adapter's non-injective states
  are on the wire but the facade keeps `TaskOutput` a plain struct.
- `packages/csharp`'s own object model; startup/R2R; GC behaviour under load.
- Content sets beyond P1.2 and P2.2; arm c beyond six payloads.
- Bidirectional streaming, cancellation mid-stream, more than one queue drainer.

## Next step

1. WP3 conformance of the harness (D6), then a smoke log.
2. WP5: port this slice's backend to render the shared generator's plans, with
   one binding file carrying `LibraryImport` under `#if NET7_0_OR_GREATER` and
   `DllImport` otherwise (D2), and the transport binding generated (D3). Then
   run the full gate on net8.0 and net6.0 including core-ffi.
3. The net48 gate on a Windows machine (D4).
4. Move the incumbent to 3.32.0 / 2.71.0 (D5) and re-gate.
5. Retain mode (D7) when WP3 item 21 lands.
6. Repair or retire `src/BenchDotNet` (D1).

## Log index

Correctness, counts and feasibility (results):

| Log | What it establishes |
|---|---|
| `stage21-wp4-regate.log` | **the current gate**, core `6ede244`: generator check; conformance 152/152, unknown, groups, utf8, mapforms, corpus (691, 31 T-dec, 3 disputed), coreffi (plain, counting, init-guard) on arm a; conformance, unknown, groups, corpus on arm b; corpus on the strict build (0 failures); conformance, unknown, groups, corpus on the net6.0 runtime (managed); the init-guard failure before the `ak_init` fix |
| `wp4-rd9-bytes-free.log` | R-D9: free on a non-OK status, every delivery, with a negative control; transport crossings per call |
| `wp3-17-net6-floor-build.log` | net6.0: the binding does not build (LibraryImport), the managed half does; the refused .NET download host |
| `stage1-conformance.log` | 152/152 on arm a, arm b and **Mono 6.8** (not .NET Framework); unknown-field retention asymmetry; P2.5's two encodings |
| `stage2-counts.log` | decision 5 prefix-width misses; zero crossings in managed arms |
| `stage10-crossing-reconciliation.log` | M1 crossing counts equal to the Rust slice at `AK_CHUNK=150` |
| `stage12-group-skip-and-d7-regate.log` | the group-skip defect, failing then fixed |
| `stage13-corpus-consumer.log` | first corpus run (336 vectors then): tag zero, recursion limit, -0.0 and group skip found; incumbent oracle |
| `stage14-all-shapes-and-pull.log` | core-ffi on every shape; pull family; the crossing table; M1 decode count corrected |
| `stage9-shared-core.log`, `stage11-core-ffi-m2.log`, `stage8-core-ffi.log` | earlier re-gates of core-ffi (M1, then M2) against earlier cores |
| `stage17-streaming.log` | the shared-context SIGABRT and the per-thread context (its timings are instrumentation) |
| `stage19-grid-pinned-nagle-crossings.log` | transport crossings per delivery (its timings are instrumentation) |
| `stage16-decision3-and-13.log` | the strict build closes the 31 T-dec vectors (its timings are instrumentation) |

Container instrumentation (timings; no figure in them is a result):
`calibration-rust-crossing.log`, `stage3-arms.log`, `stage4-floor-arm-b.log`,
`stage4-floor-arm-c.log` (Mono), `stage5-content-sets.log`,
`stage5-content-sets-floor.log`, `stage6-tiering-sensitivity.log`,
`stage7-benchmarkdotnet.log` and `bdn-results/`, `stage15-rpc-arm.log`,
`stage18-rpc-grid.log`, `stage20-shipped-window.log`, and the timing parts of
stages 8, 9, 11, 14, 16, 17 and 19.
