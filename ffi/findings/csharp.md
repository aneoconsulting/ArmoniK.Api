# Reading the csharp slice

Phase note: this file records facts only; container timings were removed on 2026-09-24 (design/FIX-PLAN.md WP2). The raw logs remain in logs/csharp/.

The aggregating session's reading of `poc/csharp`. Sources: `poc/csharp/STATE.md`
and `logs/csharp/`. Every finding below that came from the 2026-09-24 review is
unconfirmed until the slice answers it.

## 1. Configuration and levels

- Measured configuration: .NET 8.0.31 (SDK 8.0.131), `Google.Protobuf` 3.28.3,
  codegen by `Grpc.Tools` 2.66.0 (`src/Harness/Harness.csproj`,
  `src/HarnessFloor/HarnessFloor.csproj`).
- ArmoniK ships `Google.Protobuf` 3.32.0, `Grpc.Net.Client` 2.71.0 and
  `Grpc.Tools` 2.72.0 (`packages/csharp`, per `design/FIX-PLAN.md` section 4).
  The slice therefore measured an older incumbent than the one ArmoniK ships.
- The owner's levels (README section 5): floors **net6.0** and **.NET Framework
  4.8**, target **net8.0**. `LibraryImport` needs .NET 7 or later, so both floors
  use `DllImport`, in the same generated file under `#if NET7_0_OR_GREATER`.
- Floor status:
  - netstandard2.0: built, passes the gate (arm b, run on .NET 8).
  - net48 build run on **Mono 6.8.0.105**: built, passes the gate (arm c). Mono is
    **not .NET Framework 4.8**; .NET Framework runs only on Windows and no Windows
    gate exists.
  - **net6.0: not built and not run.** The C# worker ships net6.0.
  - The `core-ffi` binding is not built on arms b or c. As generated it uses
    `LibraryImport` and `UnmanagedCallersOnly`, which net48 lacks. `Core_*.cs`,
    `CoreArms.cs` and `CoreGate.cs` are excluded from arm c explicitly. A floor
    binding would be `DllImport` plus delegate pointers, and the delegates must be
    rooted for the lifetime of the vtable or the collector reclaims a thunk the
    codec still holds (a crash).

## 2. What was built

- **A generator** (`gen/`) driven by `ffi/schema/emit/shapes.py`, with a second
  front end (`gen/protoparse.py`) over `corpus.proto`, cross-checked against
  `shapes.json` at generation time on the nineteen messages and three enums they
  share. `gen/abi_ir.py` derives the by-value group, presence bits, loop slots and
  vtables; `gen/rs_probe.py`, `gen/cs_abi.py` and `gen/cs_core.py` emit the layout
  probe, the managed declaration and the host binding from it. One hand-written
  runtime file, `src/Facade/Wire.cs`.
- **The managed control codec** (a generated pure-C# codec over the facade),
  encode and decode, one-pass and two-pass, on all 16 payloads and all 7 shapes.
- **`core-ffi` over the one core at `ffi/poc/codec`**, on every shape and all 16
  payloads, arm a only: encode, push decode, **pull decode** (`ak_parse_*`), the
  UTF-16 string form (`ak_tc_utf16`), a host-fill-only arm, and a no-string decode
  arm (a ceiling for ABI decision 13, not an implementation of it).
  (`stage8`, `stage11`, `stage14`, `stage16`.)
- **A strict decode build** (`/p:AkStrict=true`) implementing ABI decision 3's
  rejecting UTF-8 policy (`stage16`).
- **A corpus consumer**: all 336 vectors of `ffi/corpus`, on arms a, b and c, codec
  generated from `generated/corpus.proto` (`stage13`).
- **An RPC arm**: a grpc-dotnet client against a grpc-dotnet server over a Unix
  domain socket, server marshaller a `byte[]` passthrough, four codecs
  (`gp-marshaller`, `managed`, `core-ffi`, `core-ffi pull`), ArmoniK's transport
  pinned, stack default and loopback TCP as labelled rows, 1/8/16 in flight, P2.2
  (`stage15`).
- **The RPC grid**: cells A, B, C, D in one process, against the core's transport
  (`rpc` feature) with the blocking, callback and queue deliveries (`stage18`,
  re-run with the core client pinned via `ak_client_new_opts` in `stage19`).
- **Streaming over its own transport**: grpc-dotnet both ends over a UDS, both
  directions, five arms including a no-codec arm, P2.2 and P5.3, 1/8/16 streams
  (`stage17`). Added by the slice beyond its brief; the owner has said it is not
  pursued further.
- **A BenchmarkDotNet harness** (`src/BenchDotNet`, `stage7`) beside the
  hand-rolled one.

## 3. Correctness and byte identity

- **136 checks, 0 failures, on each of arms a, b and c** (`stage1`): byte identity
  against `ffi/schema/generated/manifest.json` for all five encode arms on all 16
  payloads; decode checked by re-encode and by a generated field-by-field
  comparer. P7.1 is checked as a permutation of (tag, wire type, body) triples.
  The absent-path payloads P1.3 and P2.5 are in the gate. Re-gated at 152 checks
  on the shared core after W10 (`stage9`).
- **`core-ffi`**: byte identity, round trip and value identity on all 16 payloads,
  push and pull gated against each other on the same bytes and comparer
  (`stage14`). Layout agreement: 42 structs and 28 vtables match the Rust build,
  `ak_abi_version()=1`.
- **Corpus** (`stage13`): 336 vectors run on three arms with identical output.
  `Google.Protobuf` wired in as an independent oracle agrees on accept/reject on
  all 169 vectors where it has the type.
  - 31 `T-dec-*` vectors (malformed UTF-8) are accepted by the default lossy
    build, as `Google.Protobuf` also accepts them. The strict build rejects all 31
    (`stage16`).
  - `U-map-entry` is open: this decoder and `Google.Protobuf` both keep the map;
    the corpus projection leaves it absent. Raised as a question about the vector.
- **ABI decision 5** (`stage2`, counting build): zero warm prefix-width misses on
  every payload except P2.4, which misses once per element (80 of 80), site
  `ListTasksDetailedResponse.tasks`. Cold misses 0 to 3 per payload.
- **ABI decision 11** (`stage1`, seven hand-built vectors): all decode in both arms
  without losing a known value. `Google.Protobuf` **retains** unknown fields and
  writes them back; the generated codec drops them. Adopting the core's codec on
  .NET removes a behaviour that exists today.
- **Concurrency contract** (`stage17`): one decode/encode context per thread gives
  0 wrong of 200,000; one shared context aborts the process (SIGABRT) and the
  `catch (Exception)` around it does not see it. The thread pool created 7 or 8
  contexts per direction.
- **Unpaired surrogates**: `Google.Protobuf` and `Encoding.UTF8` both substitute
  U+FFFD, so both arms lose the surrogate identically.
- gRPC delivered a segmented body on every instrumented call (3,960 of 3,960) at
  P2.2's size, so the single-segment path is not taken there (`stage16`).

## 4. Crossing counts

Counted by the host at every callback, and checked against the core's own
counters (`ak_enc_counters`, a `--features count` build). Whole run per element
entry call (`stage14-all-shapes-and-pull.log`):

| payload | enc fwd | enc rev | push dec fwd | push dec rev | pull dec fwd | pull dec rev |
|---|---|---|---|---|---|---|
| P1.1 | 2 | 1 | 1 | 2 | 2 | 0 |
| P1.2 | 2 | 1 | 1 | 5 | 2 | 0 |
| P1.3 | 2 | 1 | 1 | 3 | 2 | 0 |
| P2.1 | 7 | 6 | 1 | 8 | 2 | 0 |
| P2.2 | 2502 | 2501 | 1 | 3501 | 2 | 0 |
| P2.3 | 627 | 626 | 1 | 876 | 2 | 0 |
| P2.4 | 402 | 401 | 1 | 561 | 2 | 0 |
| P2.5 | 102 | 101 | 1 | 141 | 2 | 0 |
| P3.1 | 2 | 1 | 1 | 2 | 2 | 0 |
| P4.1 | 202 | 201 | 1 | 601 | 2 | 0 |
| P5.1 to P5.4 | 1 | 0 | 1 | 1 | 2 | 0 |
| P6.1 | 1002 | 1001 | 1 | 1401 | 2 | 0 |
| P7.1 | 3 | 2 | 1 | 3 | 2 | 0 |

- A leaf element batches, so encode is constant in the element count. A non-leaf
  cannot (ABI section 7.2), so counts become linear in it: on P2.2, 10.00 per task
  on encode and 7.00 per task on decode (`stage11`).
- Push decode is not constant even for a leaf: the codec flushes a run when its
  element arena fills (ABI 7.3), so `add_results` is called `ceil(n / arena) + 1`
  times. The stage 8 statement "1 forward, 2 reverse, constant" was wrong and is
  corrected in `stage14`.
- Pull decode makes no reverse call on any shape.
- Chunk size is a host choice: this host hands the whole run over in one call; the
  Rust host chunks at 150. With `AK_CHUNK=150` this slice reproduces the Rust
  slice's counts exactly (`stage10`).
- Transport crossings per call, P2.2, independent of payload (`stage19`):
  blocking 2 forward, 0 reverse; callback 3 forward, 1 reverse; queue 4 forward,
  0 reverse.
- Managed arms cross nothing (zero, a property of the arm).

## 5. Defects found

In the slice's own code, all fixed and logged:

- Group skip: `Dec.Skip` rejected wire type 3, which `Google.Protobuf` accepts.
  Fixed by matching the END_GROUP field number with a depth bound of 100
  (`stage12`). Three wrong fixes were each seen failing, and no single gate
  (conformance, groups, unknown) catches all three. Without the bound, 200,000
  nests abort the process with a stack overflow .NET cannot catch.
- From the corpus (`stage13`): field number 0 accepted; no recursion limit
  (ABI decision 7); `-0.0` dropped because the omit rule compared `!= 0.0` rather
  than bits (`Google.Protobuf` has the same hole).
- M3 explicit-presence string took the implicit path, so present-but-empty was
  indistinguishable from absent (`stage14`, found by the general emitter).
- Harness and build: `AkFloor` define lost from the facade (C1); a `const bool`
  inlined into reading assemblies (C2); a diff-based miss tally that missed an
  oscillating site (C3); a `wide` content set of mostly two-byte characters (C4);
  floor and target outputs globbed into each other's compile items (C5); the
  decode baseline did an extra full traversal to defeat dead-code elimination
  (C6); a reporting error about `gp-writeto` (C7).
- Stale artifacts: once a failed build left a stale core in the output directory,
  and once arm c failed to build (172 errors) while Mono ran an older binary and
  reported a pass. The loaded artifact is now confirmed with `LD_DEBUG=libs`.
- RPC grid: stage 18's cells B and C took tonic's defaults while A and D pinned
  ArmoniK's transport; `ak_client_new_opts` pinned them in stage 19.
- Stage 20: neither transport row the RPC arm carried is what production runs
  (see section 6).

Gaps outside the slice: the core exports `ak_abi_version` and no layout, so a
core rebuilt with a changed layout and unchanged version passes the load check
(ABI obligation 12.3).

## 6. Harness and runtime facts

- CPU per call in `src/Rpc/Program.cs`, `Grid.cs` and `StreamRun.cs` is read from
  `Process.TotalProcessorTime`, which advances in 10 ms steps on Linux.
- The RPC grid runs client and server in one process; the grid reports min of 9
  rounds; blocks are not interleaved (R-C4, R-C6).
- Arm order moves results in the streaming harness, so a reversed-order control is
  required there. Two arms that no change touched moved between sittings on the
  same container, so numbers from different stages are not comparable.
- Whatever stops a result being optimised away must cost the same in every arm
  (defect C6).
- `Grpc.Tools`' marshaller calls `SetPayloadLength(CalculateSize())` then
  `WriteTo(bufferWriter)`, so a gRPC client pays the size pass; `gp-writeto` is
  that path. `ArrayBufferWriter<T>` is .NET Core 3.0+ and its `Clear()` zeroes the
  written span; the harness's `BufWriter` resets without zeroing.
- JIT: every figure is a warmed tier-1 figure with tiering and PGO on;
  `stage6` ran three JIT configurations. Cold start and R2R are not measured.
- .NET HTTP/2 window behaviour, read from the runtime source:
  - `Http2Connection` hardcodes a 64 MiB connection window, so the "stream window
    raised, connection window left at 65,535" hazard is not reachable on .NET.
  - A configured stream window is a starting point: `Http2StreamWindowManager`
    grows it up to 16 MiB unless the `DisableDynamicWindowSizing` AppContext
    switch is set.
  - `packages/csharp` sets no window. Its client uses an `HttpClientHandler` where
    `InitialHttp2StreamWindowSize` is not reachable. On the UDS path
    `GrpcChannelProvider` sets `DisableDynamicWindowSizing` (workaround for
    grpc-dotnet#2361). Production is therefore .NET's 64 KB default with
    auto-tuning off, which neither the pinned nor the stack-default row is.
  - `GrpcChannel` defaults to a Unix socket at `/tmp/armonik.sock` and the worker
    calls `ListenUnixSocket`.
- Both ends of the RPC arm are grpc-dotnet, which sets `TCP_NODELAY` by default.
- In the blocking delivery, a pool thread parked in a native frame must be
  replaced by the .NET thread pool, which injects threads slowly; CPU and wall
  clock diverge there.

## 7. Not measured or not established

- Any performance result: every timing in `logs/csharp/` is container
  instrumentation.
- net6.0 and .NET Framework 4.8 (on Windows); `core-ffi` on any floor.
- `core-ffi` with Google.Protobuf 3.32.0; `packages/csharp`'s own object model.
- Decision 9's sparse fill (not in the ABI); decision 13 as an implementation;
  a zero-copy string form (pinned `GCHandle` per string); the abort guard removed;
  ABI decisions 4, 6, 8, 10 and 12.
- Unknown-field retention in the generated codec (it has no bag).
- Malformed-wire paths (`ErrTruncated`, `ErrMalformed`) beyond the corpus.
- Nesting past depth 3 in the payload set.
- The upstream RPC direction, more than one queue drainer, cancellation and
  deadlines, streaming over the core's transport, bidirectional streaming.
- Concurrency outside the streaming contract check; GC pauses and working set.
- Content sets on payloads other than P1.2 and P2.2; arm c on six payloads only.

## 8. Open review findings (design/FIX-PLAN.md section 7, unconfirmed)

- R-A7: floors incomplete: net48 only on Mono; net6.0 not built.
- R-B5: the cross-slice crossing table quoted a C# per-crossing figure the slice
  never measured (removed here).
- R-C4: the C# grid is in-process.
- R-C6: C# CPU quantised at 10 ms; min-of-9 selection; blocks not interleaved.
- R-C9: C# stages 18-19 built against a core not on the branch.
- R-C11: measured on .NET 8 and Google.Protobuf 3.28.3; ArmoniK ships a net6.0
  worker and 3.32.0.
- R-C13: grid cells B/C stay pinned under `--shipped`; the grid uses the pinned
  rather than the shipped transport.
- R-D9: `ak_bytes_free` not called on non-OK status (`src/Rpc/CoreTransport.cs`).
- R-E6: the C# generator re-derives the ABI layout, and the probe shares the field
  list it checks.
- R-E7: UTF-8 decode policy differs per runtime (C# default build is lossy).
- R-F1: C# `STATE.md` contradicts itself on what exists (for example, "The RPC
  arm does not exist" under "Measurement coverage").
