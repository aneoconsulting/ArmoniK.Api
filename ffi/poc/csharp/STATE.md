# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the handoff.
It says what exists and what was checked. It carries no recommendation (the
decision is the owner's) and no timing presented as a result: in the setup and
design phase every container timing is instrumentation (README 1.1). The
reasoning behind each change is in `JOURNAL.md` (entry 50 for this unit; 49 for WP3).

| | |
|---|---|
| **Status** | **Unit 4 (2026-09-25): BenchmarkDotNet restored as the codec-suite engine (CAMPAIGN.md 22a, `src/BenchDotNet`), driven by `run_campaign.sh --suite codec`; the rpc and calib suites stay on `akrpc campaign` (reason in the checklist section). BDN smoke run executed (`logs/csharp/campaign/codec-launch1.*`, figures = instrumentation).** WP3 harness conformance done before it. Campaign-readiness checklist below (requirements 1 to 32; unmet items listed with reasons). WP5 (the one generator) done earlier; regenerated at decision 11 (`29d515e`), where core-ffi retain is the backend's TRANSITIONAL drop mode until the decision 11 options are rendered in `cs_host`. |
| **Owner's levels** (FIX-PLAN section 6, D2) | floors **net6.0** and **.NET Framework 4.8** (correctness only), target **net8.0** |
| **Target** | net8.0 on .NET 8.0.31, SDK 8.0.131 (Ubuntu 24.04 `dotnet-sdk-8.0`). Everything below |
| **net6.0 floor** | .NET 6.0.36 (runtime pack `Microsoft.NETCore.App.Runtime.linux-x64` 6.0.36 from NuGet, self-contained publish). **Builds and passes everything below, core-ffi included** (it did not build before this unit: `LibraryImport`) |
| **net48 floor** | **compiled only.** The generated P/Invoke binding's `DllImport` branch compiles for net48 (`src/HarnessFloor`); the core-ffi host half is `#if NET5_0_OR_GREATER` (needs `[UnmanagedCallersOnly]`) and is compiled out. **Nothing is run on .NET Framework**: it needs Windows; this container has no Mono either. Every earlier net48 result in this slice's history was Mono 6.8 and is not the gate (FIX-PLAN WP3 item 17) |
| **Incumbent** | `Google.Protobuf` 3.32.0, `Grpc.Tools` 2.72.0, `Grpc.Net.Client` / `Grpc.AspNetCore` 2.71.0: the versions `packages/csharp` ships (D5 closed, re-gated). |
| **Core** | the one core at `ffi/poc/codec`, from a `git archive HEAD` snapshot (last core commit `41eb485`), built by `gen/build_core.sh`, **every build with `init-guard`**: `target-core` (`rpc,init-guard`), `target-core-count` (`rpc,count,init-guard`), `target-core-corpus` (`corpus,init-guard`) |
| **Machine** | a container, 4 vCPU Intel Xeon @ 2.10GHz, Linux 6.18.44. Nothing here depends on it |

## What exists

### The shared C# backends (`ffi/poc/codec/gen/`, FIX-PLAN WP5 step 4)

Each imports `plan` (and `cs_names`/`cs_types`) and nothing from the IR or a description.

```
cs_names.py         C# spellings (members, enums, oneof cases, the ABI vocabulary) + banner
cs_types.py         the facade types and the structural comparer of a message set; every
                    class carries the unknown-field bag `UnknownFields` (null = none)
cs_managed.py       the MANAGED codec: Write (one pass, learned width), SizeOf/WriteSized
                    (two pass), Read (decode, `Read<M>(ref Dec, M, depth)`), rendered from
                    the encode steps and the (number, wire) decode table; utf8, unknown mode
                    and depth limit are the plan's options
cs_binding.py       the P/Invoke BINDING, one file per message set: vocabulary structs, the
                    e/d/u groups, presence bits, vtables, every import as LibraryImport under
                    `#if NET7_0_OR_GREATER` and DllImport under `#else`, `ak_init` from
                    plan.lifecycle (static constructor), AbiLayout.Table()/Facts(); and the
                    RPC half from plan.rpc (RpcAbi.cs, with its own ak_init)
cs_host.py          the core-ffi HOST half: per root, encode (ak_encode / ak_uencode with the
                    bags), push decode (with and without the capture callbacks), pull decode,
                    Try* forms returning the core's code; compiled where UnmanagedCallersOnly
                    exists (NET5_0_OR_GREATER)
cs_layout_probe.py  the layout probe's Rust source, parsed out of ak-abi's RUST declaration
                    text (reads no plan: R-E6)
```

### This slice (`poc/csharp/`)

```
gen/generate.py [--check]   the driver: calls the shared backends, writes every generated
                            file; --check = drift + the one-generator guard (the shared
                            generator's own rule, loaded by path) + its planted self-test
gen/glue.py, cs_values.py, cs_build.py, cs_arms.py, cs_proj.py, cs_registry.py
                            HARNESS GLUE only: value rules, payload builders (two object
                            models), the arm table, the corpus projection and root dispatch,
                            the core-ffi arm registry and the corpus ffi dispatch
gen/build_core.sh           the core snapshot, the three builds, the probe -> target-core*/layout.json
gen/gate.sh                 THE GATE (nothing timed); writes the whole of wp5-step4-gate.log
abi/                        the layout probe crate (src/main.rs generated), --features corpus
src/Facade/Wire.cs          THE hand-written runtime (varints, buffer, learned width, reader):
                            StrReject/StrLossy (the codec picks per the plan), Skip(tag, wire,
                            limit), 64-bit length checks, Raw/Append for the bag
src/Facade/Generated/       Types, Eq, Codec (shared backends); Values, Build (glue)
src/Harness/                conformance | unknown | groups | utf8 | mapforms | layout |
                            coreffi | content | counts | bench;  net8.0 + net6.0
src/Harness/Generated/      Abi, CoreFfi (shared backends); BuildGp, Arms, CoreArms (glue)
src/Corpus/                 the corpus runner (net8.0 + net6.0): 4 arms, a child process per
                            row; Generated/ Types, Eq, Codec, Abi, CoreFfi (shared), Proj,
                            Dispatch (glue); loads the corpus-schema core
src/Rpc/                    akrpc: `campaign --suite rpc|rpc-server|calib` (Campaign.cs), the
                            layout/error-path gate modes, legacy modes (D42); Generated/RpcAbi.cs
                            from plan.rpc and plan.FIXED (nothing of the ABI by hand)
src/HarnessFloor/           net48, COMPILE ONLY (binding included, host half compiled out)
src/BenchDotNet/            THE CODEC-SUITE ENGINE: BenchmarkDotNet 0.15.8, net8.0, InProcessEmit.
                            Cases.cs (the case list, pre-timing checks, one Func per case),
                            Codec.cs (one [Benchmark], the case a [ParamsSource]), Engine.cs
                            (getrusage diagnoser, arm-rotating orderer, JSON-lines exporter),
                            Program.cs (job, header); Generated/CampaignOps.cs (glue,
                            gen/cs_campaign.py: per-root calls and the Touch field visitors)
```

Retired this unit: `gen/ir.py`, `abi_ir.py` (the second IR), `csnames.py`, `cs_facade.py`,
`cs_managed.py`, `cs_abi.py`, `cs_core.py`, `cs_coreffi.py`, `cs_coreffi2.py` (dead),
`rs_probe.py`, `protoparse.py` (the corpus now comes from `plan.load_corpus`),
`abi-layout.json` (no explicit offsets any more), `src/Harness/Corpus/`, `CorpusRun.cs`,
`src/Harness/Generated/Core_*.cs`, the `AkStrict` build.

## What was checked, and holds today

All in `logs/csharp/wp5-step4-gate.log` (one run of `gen/gate.sh`, `GATE PASSED`).

**Generator.** `--check`: 19 files current; the guard sees 6 shared backends and 7 glue
modules import no IR or description, and catches a planted `import ir`/`from shapes`.

**Payload byte identity (`harness conformance`), net8.0 and net6.0.** 152 checks, 0
failures on each: every encode arm (incumbent four, managed one-pass and two-pass) against
`ffi/schema/generated/manifest.json` on all 16 payloads, P1.3 and P2.5 included; decode then
re-encode; decode compared through the generated comparer; P7.1 as a permutation.

**Unknown fields (`harness unknown`), net8.0 and net6.0.** Seven hand-built vectors: all
decode, no known value lost; the default reader drops; **retain mode (`Dec.Retain`)
re-encodes byte-identical to Google.Protobuf on all 7**.

**Groups, UTF-8, map forms.** Five group vectors and the depth bound (200 and 200,000
nests, ErrDepth, process alive); the managed codec refuses 15 of 15 root-site T-dec vectors
(the plan's `utf8 = reject`), Google.Protobuf 0 of 15; the P2.5 map form reads both ways.

**Layout (`harness layout`, `corpus --layout`, `akrpc --layout`), by name both ways.**
Shapes binding: 96 C# structs / 443 members against the Rust declaration: all agree;
section 10's `ak_layout_facts`: 380 facts agree. Corpus binding: 168 structs / 660
members, 542 facts, agree. RPC binding: 5 structs / 18 members agree. **Control**: a planted
offset swap in the C# table fails (2 disagreements), on net8.0 and net6.0.

**core-ffi, every shape (`harness coreffi`), net8.0 and net6.0.** All 16 payloads: encode
byte identity, decode round trip, value identity, pull against push, and the retain path
(`ak_uencode_*` + capture callbacks, same bytes); R5 against the counting core: the host's
tally equals the core's counters on every row. **Control**: `ak_init` skipped
(`AK_GATE_PLANT_NO_INIT=1`) against the init-guard core: 16 of 16 rows fail.

**Crossing counts** (counting core; unchanged to the digit from the previous binding):

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

Whole run per element-entry call; a leaf element batches (constant), a non-leaf does not
(linear), ABI v1 7.2. M5's bulk field now crosses as ABI v1 section 8's DIRECT argument
(the previous binding staged it and declared `ak_encode_UploadResultDataMessage` without
the two direct parameters the core exports).

**The corpus (`src/Corpus`), net8.0 and net6.0, 691 rows, each in a child process under a
20 s timeout; 0 hangs, 0 crashes:**

| arm | pass | fail | disputed (excluded) | not in the C ABI |
|---|---|---|---|---|
| managed-drop | 688 | 0 | 3 | 0 |
| managed-retain | 688 | 0 | 3 | 0 |
| ffi-drop | 672 | 0 | 3 | 16 (root `Nest`, refused by the generator) |
| ffi-retain | 672 | 0 | 3 | 16 |

- Disputed rows: `U-map-entry` accepted, reads as protobuf pure-python; `X-tag-zero-Empty`
  and `X-tag-zero-nested-Empty` refused (malformed) by all four arms.
- On every reject row the managed codec and the core refuse with the same class (73
  truncated, 35/34 malformed, 31 transcode; the 2 depth rows and 1 malformed row are
  `Nest`, managed only).
- Forms (C3): the drop arms write the dropped form; managed-retain writes the retained
  form everywhere; ffi-retain writes the dropped form on 16 unknown-class rows (an unknown
  inside an inlined singular child has no carrier in the C ABI; the rust slice reports the
  same, D34).
- Google.Protobuf as a second oracle: accept/reject agrees with managed-drop on all 395
  rows whose root it has.
- **Controls, each required to fail and failing, on net8.0 and net6.0**: planted projection
  key (C2), planted re-encode byte (C3), refusals read as acceptances (C4), `ak_init`
  skipped (the ffi arms fail; managed has no ak_init).

**Byte and verdict changes against the previous generator** (the retired `cs_managed.py`,
run from its last build on the same corpus): **no C3 form changed** (the drop-mode form
distribution is identical row for row in count); **31 `T-dec-*` rows** were accepted by the
default (lossy) build and are now refused `-6` (the plan's `utf8 = reject`, R-E7); **1 C4
code changed**: `X-varint-key-truncated` was `-4` (malformed), now `-3` (truncated), the
core's class. The managed decoder's depth rule is now the plan's ("more than 100 levels
below the root"); the retired codec counted the root as level 1 (one level stricter); no
corpus row sits on that boundary.

**akrpc (net8.0)**: the generated RPC binding works end to end: R-D9's error path on every
delivery, 0 failures, crossings per call as ABI v1 section 9 (counting core).

## Open defects

| # | Where | What |
|---|---|---|
| D4 | net48 | compiled only; no gate on .NET Framework (needs Windows); the host half has no net48 form |
| D41 | `cs_host.py` (shared) | decision 11's options (`ak_dec_<Root>_opts`, `ak_dec_ctx_new_<Root>`) are not rendered: core-ffi decode is in the transitional DROP mode since `29d515e`, so the campaign's core-ffi "retain" rows decode in drop mode (encode still hands the bags over through `ak_uencode_*`). Requirement 10 is pending this port; this unit was told not to edit poc/codec |
| D42 | `akrpc` legacy modes (`--grid`, `--stream`, `Bench.cs`) | pre-campaign timing harnesses, not conformant (in-process server, `TotalProcessorTime`); superseded by `akrpc campaign`, kept only for `--error-path` and `--layout` (gate); none is a campaign harness |

Closed in the WP5 tail: **D38** (the runtime's group skip accepted a field number above
2^29 - 1 inside a group: it now takes `Codec.MaxFieldNumber`/`Codec.GroupDepthLimit`,
rendered from `plan.MAX_FIELD_NUMBER`/`plan.GROUP_DEPTH_LIMIT`, and refuses on the full
64-bit value as the core does), **D40** (the RPC counting surface is rendered from
`plan.FIXED` into `Generated/RpcAbi.cs` with both import forms; `CoreTransport.cs`
declares nothing of the ABI by hand), **D10** (the shared `generate.py` now guards the C#
backends and runs this slice's `--check`, done by the aggregating session at `57b6180`).

Closed in step 4: D2 (net6.0/net48 binding), D3 (transport binding generated, except the
counters below), D7 (retain mode: managed and core-ffi), D8 (`MapForms` narrowing; also
`Triples`), D9 (`ak_init_opts`/`ak_err` are in the by-name layout check now).

## Campaign readiness (design/CAMPAIGN.md a10ac81 + 0e8e9eb + 975001b; section 10 checklist)

Runner: `poc/csharp/run_campaign.sh --suite codec|rpc|calib|gate --out DIR` (reads
`AK_CPU_CLIENT`, `AK_CPU_SERVER`; campaign DIR `ffi/logs/csharp/campaign/`). The codec suite
is measured by **BenchmarkDotNet** (`src/BenchDotNet`, req 22a): one pinned process per
launch (InProcessEmit toolchain, so `taskset` pins every case), one case per
(arm, direction, payload, content set, unknown mode), 1,780 cases. The rpc and calib suites
are measured by `akrpc campaign` (`src/Rpc/Campaign.cs`). Per-root calls are generated by
`gen/cs_campaign.py`.

**Why the rpc suite stays on `akrpc campaign`, not BDN** (evaluated this unit, not done):
requirement 18 aborts the whole run with no figure on any failed call, where BDN records one
failed case and continues (it would need an exit from inside a benchmark, which bypasses
BDN's reporting); one rpc sample is one batch of C concurrent calls at 1/8/16 in flight,
timed with process CPU (getrusage, client) beside wall, where BDN times invocations of one
method per iteration and has no per-iteration CPU; BDN's pilot sizes the invocation count
per case, which on an RPC batch changes the number of calls per sample between cases and
launches; and the server lifecycle is per transport and launch, owned by the runner. A BDN
port would keep only BDN's loop while working around each of these, so the existing
runner (which meets 13 to 18, 21, 23, 27, 28) is kept.

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | met by the runner (one process at a time per suite); the machine is the owner's |
| 2 | governor, turbo, SMT | recorded in every header (sysfs); setting them is the owner's |
| 3 | isolation | recorded (isolcpus/nohz_full from cmdline, cgroup cpuset); the mechanism is the owner's |
| 4 | three disjoint CPU sets | met: `AK_CPU_CLIENT`/`AK_CPU_SERVER`, `taskset` per process; OS set is everything else |
| 5 | floors gated for correctness | net6.0: met (gate). net48: **not met here**: compiled only; needs a Windows machine (owner) |
| 6 | build flags printed | met: Release, target net8.0, core features (init-guard on), shared `libak_core.so`, JIT/GC settings; the core's cargo profile is `lto = false` (Cargo.toml), stated here, not printed |
| 7 | payloads | met: 16 payloads; content sets latin1/wide on P1.2 and P2.2 (SHAPES.md); every `U-*` corpus row with a shapes root, disputed excluded (92 rows) |
| 8 | arms | met: incumbent-prod (Grpc.Tools marshaller shape: CalculateSize + WriteTo(IBufferWriter); ParseFrom(ReadOnlySequence)), incumbent-best (WriteTo(IBufferWriter) without the size pass; ParseFrom(ReadOnlySpan)), core-ffi (push; pull as `core-ffi-pull`), host-gen (managed codec); on the unknown rows: incumbent-prod, host-gen and core-ffi, each drop/retain where it has the mode |
| 9 | directions | met: encode, decode, decode-read (a generated `Touch` visitor reads every field, both object models); decode-reencode on the unknown rows |
| 10 | drop and retain | **pending decision 11 port** (D41): host-gen drop/retain met; core-ffi retain rows run, but decode in the transitional drop mode; incumbent default (retains), stated |
| 11 | serialised once per iteration, no amortised memo | met: Google.Protobuf's C# messages keep no serialized-size memo (CalculateSize recomputes); core-ffi and host-gen reset their contexts per call; the graph is reused |
| 12 | cells A-D | met |
| 13 | server separate process, pre-serialised | met: Kestrel in its own process on `AK_CPU_SERVER`, P2.2 bytes pre-serialised; direction b decoded by the incumbent in every cell |
| 14 | directions a and b | met; the optional streamed upload is not built |
| 15 | 1/8/16 in flight | met |
| 16 | B/C blocking; callback/queue labelled | met (`B`, `C` blocking; `B.callback`, `B.queue`, `C.callback`, `C.queue` extra) |
| 17 | shipped and pinned, B/C follow | met: shipped = packages/csharp's UDS config (DisableDynamicWindowSizing, no window) and Kestrel defaults, core client adaptive off with the stack's windows; pinned = 4 MiB stream and connection windows everywhere, adaptive off, Nagle off (UDS: Nagle has no effect) |
| 18 | every call checked, abort | met: status and length on every call; an exception aborts with no sample; control `--plant` (wrong expected length) aborts, logged |
| 19 | crossing counts gate | met: counting build vs `gen/crossings.txt` in the gate and before calib; it caught the decision 11 change (P1.2, re-baselined) |
| 20 | crossing cost fwd/rev, perf stat | partly: `crossing-forward` (ak_noop) and `crossing-forward-reverse` (ak_noop_reverse) rows, the reverse cost is their difference; `perf stat` is run when installed, **not installed in this container** |
| 21 | CPU clocks | met with the stated fallback. codec (BDN): **wall per iteration** (BDN Stopwatch, exported raw) plus **process CPU per case** (getrusage RUSAGE_SELF, a diagnoser on BDN's BeforeActualRun/AfterActualRun signals) across the warm-up + actual span, with gen0/1/2 collection counts (the span includes BDN's forced GCs); **per-iteration and thread CPU are not available** (no BDN diagnoser or column gives them), stated in every header. rpc: getrusage(RUSAGE_SELF) of the client beside wall per batch. calib: CLOCK_THREAD_CPUTIME_ID. No Process.TotalProcessorTime |
| 22 | blocks, order rotated between launches | codec: met, blocks by arm (BDN orderer), the arm order rotated by launch number (5 arms, so launches 1-3 start with a different arm); rpc: cells interleaved per round, rotated |
| 23 | 5 rounds x 3 launches, every round committed | met (defaults): codec = 5 BDN actual iterations per case x 3 launches, every iteration exported; rpc/calib = 5 rounds x 3 launches; the smoke run is 1 x 1 by requirement 32 |
| 24 | warm-up stated, identical | codec: BDN's jitting stage (1 overhead + 1 workload invocation) and pilot, then a FIXED warm-up count (10; smoke 1) for every case, exported per case (`bdn_warmup`, and every stage's iterations/ops/ns in `bdn_stages`); tiered JIT and PGO at net8.0 defaults (stated), the tier reached is not read back. rpc: two passes of max(64 calls, 50 ms) per cell, then fixed iterations |
| 25 | allocator/GC warm, GC stated | met: codec: BDN warm-up per case and its forced GC between iterations (default), GC counts and heap size recorded per case; rpc: warm-up for every arm, GC.Collect before every round; workstation concurrent GC stated |
| 26 | correctness before timing | met: the runner requires `gen/gate.sh` passed at identical content (the gate builds BenchDotNet); the BDN process re-checks byte identity of every timed encode arm (incumbent-prod, incumbent-best, host-gen, core-ffi drop and retain) per payload and content set, and that every arm accepts every unknown row, before BDN runs (560 checks) |
| 27 | header | met: runner (commit, dirty tree refused unless AK_ALLOW_DIRTY, machine, flags, repeats, gate) + process (BDN version and toolchain, runtime, tiering, incumbent version, plan options, the job: iterations, warm-up, iteration time, strategy, EvaluateOverhead=false, clocks, arm order, checks, case count) |
| 28 | JSON lines, raw | met: a BDN exporter writes one line per raw Workload/Actual measurement (no outlier removal, no overhead subtraction: EvaluateOverhead=false; BDN's outlier handling touches only its console summary), plus one round-0 CPU row per case; failed cases are written as `# FAILED CASE` and fail the process |
| 29 | logs per suite and launch in ffi/logs/csharp/campaign/ | met |
| 30 | summaries | none produced by this slice (optional) |
| 31 | runner interface | met for the slice; the top-level `ffi/campaign.sh` is not this slice's file |
| 32 | smoke run | met: `logs/csharp/campaign/`, marked instrumentation |

D1 (`src/BenchDotNet`, retired at WP3) is **restored** as the codec-suite engine (req 22a) and builds in the gate; D1 is closed.

**Smoke runs** (`logs/csharp/campaign/`, CLIENT=0 SERVER=1, 1 launch, every file headed
"instrumentation, not a result"):

- **codec under BDN** (this unit, `codec-launch1.jsonl` + `codec-launch1.bdn.log`, commit and
  gate in its header): commit `7f7f6b6`, gate PASSED at
  that commit (`gate.log`), 560 in-process pre-timing checks passed, 1,780 cases, 0 failed:
  1,780 raw iteration rows (round 1) + 1,780 CPU rows (round 0); by direction encode 120,
  decode 600, decode-read 600, decode-reencode 460; 36 to 44 min per launch at the smoke job
  (1 warm-up, 1 iteration of 2 ms), almost all of it BDN's per-case overhead
- rpc, calib and the abort control (WP3, commit `5d81225`): rpc 48 samples per transport
  (16 cell/delivery rows x 3 in-flight levels, shipped and pinned); the requirement-18
  control aborted with 0 samples on both transports (`*.PLANT.jsonl`); calib 2 samples after
  the crossing-count gate (`calib-crossing-counts.log`).

`gate.log` is the gate the codec smoke ran behind: net8.0 and net6.0, conformance, core-ffi,
crossing counts = `gen/crossings.txt`, corpus (managed and ffi arms), probe rows, controls.
In its corpus, ffi-retain writes the dropped form on the unknown rows: the transitional drop
mode of D41, accepted by the contract, reported as a retention gap.

**Engine cost (container, instrumentation):** BDN's per-case overhead grows with the number of
cases in the process (60 cases: about 0.1 to 0.3 s per case; 400: about 0.44 s; 1,780: about
1.2 to 1.5 s, outside the measured iterations; heap at span start 61 to 72 MB with no trend over the 1,780 cases, 4/4/4 gen0/1/2 collections in every case's span). A
full launch at 100 ms iterations is therefore hours, not minutes; `AK_BDN_ONLY` (payload or
row ids) and `AK_BDN_NO_UNKNOWN=1` narrow a launch.

## What the plan did not state

Reported at step 4 (vocabulary struct layouts, fixed entry points, flag values, the RPC
counting surface, the field-number limit). All are in the plan since `57b6180`/`41eb485`
(`plan.FIXED`, `MAX_FIELD_NUMBER`, `GROUP_DEPTH_LIMIT`) and the C# backends render them;
nothing is declared by hand any more.

## What is not measured or not established

- **No timing in this slice is a result.** Timing waits for the campaign (W13).
- Anything on .NET Framework 4.8; the RPC arm on any floor.
- A lossy-UTF-8 managed codec (the plan's alternative option) is not generated.
- Unknown fields inside an inlined child, an inner run's element or a map entry through the
  C ABI (no carrier); inside a map entry in the managed codec (the facade map has no bag).
- Google.Protobuf's message-size limit (ABI v1 decision 8) in the managed codec.
- ABI v1 decisions 4, 6, 10, 12 on .NET; decision 13 bounded only by the no-string ceiling.
- The `[UnmanagedCallersOnly]` guard removed; `packages/csharp`'s own object model; R2R;
  GC under load; content sets beyond P1.2/P2.2; bidirectional streaming.

## Next step

1. Render decision 11's options in `cs_host.py` (D41) when authorized, then requirement 10
   is met and the core-ffi retain rows really retain.
2. The net48 gate on a Windows machine (D4), which first needs a net48 host half
   (delegate thunks rooted for the vtable's lifetime).
3. The campaign itself is the owner's: `run_campaign.sh` on the campaign machine (codec: 3
   launches under BDN, several hours per launch at the default job; see the engine cost).

## Log index

| Log | What it establishes |
|---|---|
| `campaign/` | smoke runs of `run_campaign.sh`: `gate.log` + `codec-launch1.jsonl`/`.bdn.log` (BDN, unit 4); rpc shipped+pinned, the rpc abort control, calib (WP3). Every figure is container instrumentation |
| `wp5-tail-gate.log` | the WP5-tail gate, core `41eb485`, clean core builds: as step 4's, plus the oracle-probe rows (11/11 on all four arms, net8.0 and net6.0, `P-field-maxplus1-in-group` refused); corpus 702 rows: managed 696/0, ffi 680/0 (+16 Nest), 6 disputed; RPC layout 6 structs / 20 members incl. `ak_rpc_counters` |
| `wp5-step4-gate.log` | the step-4 gate (core `77f91ee`): generator check and guard; core builds (init-guard); net8.0 and net6.0: conformance 152/152, unknown (retain = incumbent bytes), groups, utf8, mapforms, layout by name + section 10 (+ plant), core-ffi every shape incl. retain, R5 on the counting core, no-ak_init control; akrpc layout and error path; the corpus, four arms, net8.0 and net6.0, with four controls each; net48 compile |
| `wp5-step4-before-after.log` | the retired managed codec against the plan-rendered one on the corpus: C3 forms, C4 codes per row |
| `wp4-rd9-bytes-free.log` | R-D9 on the previous binding, with a negative control |
| `wp3-17-net6-floor-build.log` | net6.0 before this unit: the binding did not build |
| `stage21-wp4-regate.log` | the previous generator's gate (core `6ede244`) |
| `stage1` ... `stage20` | earlier stages; their timing parts are instrumentation |
