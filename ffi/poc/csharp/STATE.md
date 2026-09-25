# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the handoff.
It says what exists and what was checked. It carries no recommendation (the
decision is the owner's) and no timing presented as a result: in the setup and
design phase every container timing is instrumentation (README 1.1). The
reasoning behind each change is in `JOURNAL.md` (55 for the no-unknown variant, 54 for the gate-failure reproduction, 52-53 for decision 11 and req 12, 50-51 for BDN, 49 for WP3).

| | |
|---|---|
| **Status** | **WP5 step 10 (2026-09-25): the NO-UNKNOWN variant built, gated and in the campaign harness.** `/p:AkNounk=true` is its own build (GeneratedNounk/, rendered from the drop-relowered plans; bin-nounk/), against `target-core*-nounk` (ak-core `--no-default-features`); `logs/csharp/wp5s10-gate.log` GATE PASSED at `2410125`: 240/340 layout facts, byte identity, the loaded core is the variant, its own crossing counts (P1.2 decode reverse 8 -> 5 against the full build, the one difference), the corpus with every unknown row in the dropped form, rule 6, net8.0 and net6.0. BDN modes and RPC cells C-nounk/D-nounk added; smokes with figures stripped. The gate failure seen once at 253f487 did not reproduce in 6 runs (`logs/csharp/gate-repro/`), cause not found (JOURNAL 54). Before it: **WP5 step 9, decision 11 port (2026-09-25): D41 closed.** The C# backends render `ak_dec_<Root>_opts`, the root-bound contexts and a retained decode through one grow; `logs/csharp/wp5s9-gate.log` GATE PASSED at `8d2e7ac` (core `e897f57`), net8.0 and net6.0: ffi-retain keeps the retained form on every non-disputed unknown row (0 gaps; the 307-row regression gone, now a gate failure under `AK_CORPUS_RETAIN_STRICT`), per-position discard / pull == push / wrong-root controls pass with their plants failing, crossing counts unchanged. CAMPAIGN requirement 10 met in the BDN codec suite. Before it: **Unit 4 (2026-09-25): BenchmarkDotNet restored as the codec-suite engine (CAMPAIGN.md 22a, `src/BenchDotNet`), driven by `run_campaign.sh --suite codec`; the rpc and calib suites stay on `akrpc campaign` (reason in the checklist section). BDN smoke run executed (`logs/csharp/campaign/codec-launch1.*`, figures = instrumentation). Follow-up (JOURNAL 51): the per-case overhead traced to BDN's forced GCs over a live heap that grows with the cases in one process; a launch is now one process per arm:mode unit; the JIT tier of the measured code is read back per case (PASS on every case of the smoke).** WP3 harness conformance done before it. Campaign-readiness checklist below (requirements 1 to 32; unmet items listed with reasons). WP5 (the one generator) done earlier; regenerated at decision 11 (`29d515e`), where core-ffi retain is the backend's TRANSITIONAL drop mode until the decision 11 options are rendered in `cs_host`. |
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
                    bags), push and pull decode on ONE root-bound context, drop mode or
                    retained (decision 11: native options, one grow, reset/decode/reset,
                    group readers taking each slot into UnknownFields), Try* forms, the
                    per-position discard helpers; compiled where UnmanagedCallersOnly
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
                            Cases.cs (the case list, the arm:mode process units and their
                            rotation, pre-timing checks, one Func per case, prime cases),
                            Codec.cs (one [Benchmark], the case a [ParamsSource]), Engine.cs
                            (getrusage/GC diagnoser, orderer, JSON-lines exporter), JitTiers.cs
                            (runtime JIT events: tier read back per case), Program.cs (job,
                            pre-warm, header, JIT check); Generated/CampaignOps.cs (glue,
                            gen/cs_campaign.py: per-root calls and the Touch field visitors)
```

Retired this unit: `gen/ir.py`, `abi_ir.py` (the second IR), `csnames.py`, `cs_facade.py`,
`cs_managed.py`, `cs_abi.py`, `cs_core.py`, `cs_coreffi.py`, `cs_coreffi2.py` (dead),
`rs_probe.py`, `protoparse.py` (the corpus now comes from `plan.load_corpus`),
`abi-layout.json` (no explicit offsets any more), `src/Harness/Corpus/`, `CorpusRun.cs`,
`src/Harness/Generated/Core_*.cs`, the `AkStrict` build.

## What was checked, and holds today

**Decision 11 (WP5 step 9), `logs/csharp/wp5s9-gate.log`, GATE PASSED at `8d2e7ac`, core
`e897f57`, net8.0 and net6.0.** Layout by name both ways including every
`ak_dec_<Root>_opts` (now required to be bound): 105 structs / 490 members (shapes), 199 / 780
(corpus), section 10 facts agree. Conformance 152/152; core-ffi every shape incl. retain;
crossing counts = `gen/crossings.txt` unchanged (the arm and disarm resets are forward calls
the core's R5 counters do not count; the host counts them apart, `ResetCalls`). The corpus,
four arms: ffi-retain 680 pass / 0 fail, 6 disputed, 16 not in the ABI; forms as the rust
slice's (105 "as committed / unknown-retained", 6 "unknown-retained, appended in tag order");
**no retain arm writes the dropped form on a non-disputed row** (`AK_CORPUS_RETAIN_STRICT=1`,
a gate failure otherwise; control `unkdrop`, ffi-retain in drop mode, fails with 307 gaps).
`corpus --unk-controls`: 543 accept rows whose root crosses the ABI, 2,290 (row, position)
pairs, 307 rows with unknowns at some position (315 pairs changed by zeroing), each position
zeroed in turn equal to the retained value with that position's bags cleared on every row,
pull == push on every row, no buffer left undelivered; wrong root (a `Timestamp` context on
`Duration`'s entry points): decode, parse and reset refused with AK_ERR_INVALID_STATE, the own
root still resets and parses. Plant (the expectation's clearing skipped): 307 mismatching rows,
fails as required. The same counts as the rust slice's step-8 gate. U-map-entry: the map
entry's bytes reach the host and are freed (the facade map has no bag), disputed.

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
(`ak_uencode_*` + the armed options since step 9, same bytes); R5 against the counting core: the host's
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
  form everywhere; ffi-retain (step 4 core) wrote the dropped form on 16 unknown-class rows
  (no carrier then; since step 9, 0 rows: see the decision 11 paragraph above).
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
| D42 | `akrpc` legacy modes (`--grid`, `--stream`, `Bench.cs`) | pre-campaign timing harnesses, not conformant (in-process server, `TotalProcessorTime`); superseded by `akrpc campaign`, kept only for `--error-path` and `--layout` (gate); none is a campaign harness |

Closed in WP5 step 9: **D41** (decision 11's options and root-bound contexts rendered in
`cs_binding`/`cs_host`; core-ffi retain retains; `logs/csharp/wp5s9-gate.log`).

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
is measured by **BenchmarkDotNet** (`src/BenchDotNet`, req 22a): per launch, one pinned
process per **unit** `arm:mode` (7 units: incumbent-prod, incumbent-best, host-gen drop and
retain, core-ffi drop and retain, core-ffi-pull; 40 to 336 cases each), in an order rotated by
launch (`BenchDotNet --launch N --list-units`); InProcessEmit toolchain, so `taskset` pins
every case; one case per (arm, direction, payload, content set, unknown mode), 1,780 cases.
Each process: pre-timing checks, a pre-warm to JIT quiescence, two unexported prime cases,
then BDN; every unit appends its header block and rows to `codec-launch<N>.jsonl`. The rpc and calib suites
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
| 10 | unknown fields, three modes (amended 85cb00f) | met: **retain** and **drop** in the full build (host-gen drop/retain, core-ffi drop/retain, decision 11 armed at every position for retain, checked before timing: retain re-encodes every unknown row to the incumbent's bytes), core-ffi-pull drop; **no-unknown** in its own build (`/p:AkNounk=true`, WP5 step 10): core-ffi and core-ffi-pull against `target-core-nounk` (ak-core `--no-default-features`), and **host-gen too, plan-generated** (the managed codec rendered from the drop plan has no capture code); every sample carries `unknown_mode`; incumbent default (retains), stated. The one position the C# facade cannot hold is a map entry's bag (U-map-entry, disputed) |
| 11 | serialised once per iteration, no amortised memo | met: Google.Protobuf's C# messages keep no serialized-size memo (CalculateSize recomputes); core-ffi and host-gen reset their contexts per call; the graph is reused |
| 12 | cells A-D; C and D per unknown-field mode (amended 85cb00f) | met: the full client runs `A`, `B`, `C-retain`, `C-drop`, `D-retain`, `D-drop` (+ the B/C callback/queue extras, C.* in drop), the no-unknown client (`src/Rpc` built with `/p:AkNounk=true`, its core `target-core-nounk`) runs `A`, `B`, `C-nounk`, `D-nounk` with A and B as its in-process controls; both clients against the same server process per transport and launch, in an order alternated by launch; `unknown_mode` on every sample; each client checks at start that its core is its variant |
| 13 | server separate process, pre-serialised | met: Kestrel in its own process on `AK_CPU_SERVER`, P2.2 bytes pre-serialised; direction b decoded by the incumbent in every cell |
| 14 | directions a and b | met; the optional streamed upload is not built |
| 15 | 1/8/16 in flight | met |
| 16 | B/C blocking; callback/queue labelled | met (`B`, `C` blocking; `B.callback`, `B.queue`, `C.callback`, `C.queue` extra) |
| 17 | shipped and pinned, B/C follow | met: shipped = packages/csharp's UDS config (DisableDynamicWindowSizing, no window) and Kestrel defaults, core client adaptive off with the stack's windows; pinned = 4 MiB stream and connection windows everywhere, adaptive off, Nagle off (UDS: Nagle has no effect) |
| 18 | every call checked, abort | met: status and length on every call; an exception aborts with no sample; control `--plant` (wrong expected length) aborts, logged |
| 19 | crossing counts gate | met for both builds: the full build against `gen/crossings.txt`, the no-unknown build (counting core `target-core-count-nounk`) against its own `gen/crossings-nounk.txt`, in the gate and before calib. The two differ in one row: P1.2 push-decode reverse 8 (full) vs 5 (no-unknown), as in the rust slice |
| 20 | crossing cost fwd/rev, perf stat | partly: `crossing-forward` (ak_noop) and `crossing-forward-reverse` (ak_noop_reverse) rows, the reverse cost is their difference; `perf stat` is run when installed, **not installed in this container** |
| 21 | CPU clocks | met with the stated fallback. codec (BDN): **wall per iteration** (BDN Stopwatch, exported raw) plus **process CPU per case** (getrusage RUSAGE_SELF, a diagnoser on BDN's BeforeActualRun/AfterActualRun) across the **actual stage** (BDN signals it after the warm-up), which includes the 4 full GCs BDN forces per iteration: their pause time is recorded beside it (`gc_pause_ns`, `gc`); **per-iteration and thread CPU are not available** (no BDN diagnoser or column gives them), stated in every header. rpc: getrusage(RUSAGE_SELF) of the client beside wall per batch. calib: CLOCK_THREAD_CPUTIME_ID. No Process.TotalProcessorTime |
| 22 | blocks, order rotated between launches | codec: met, one process per arm:mode unit, the unit order rotated by launch (arms rotated by launch - 1, the modes within an arm too; 5 arms, so launches 1 to 3 start with a different arm); rpc: cells interleaved per round, rotated |
| 23 | 5 rounds x 3 launches, every round committed | met (defaults): codec = 5 BDN actual iterations per case x 3 launches, every iteration exported; rpc/calib = 5 rounds x 3 launches; the smoke run is 1 x 1 by requirement 32 |
| 24 | warm-up stated, identical | met. codec: per process a pre-warm (rounds of 64 calls to every case of the unit, 0.5 s apart, until a round compiles nothing: 7 to 9 rounds in the smoke) and 2 unexported prime cases; per case BDN's jitting stage and pilot, then a FIXED warm-up count (10; smoke 1), every stage exported (`bdn_stages`). **Tier read back**: an in-process listener on the runtime's JIT events (MethodLoadVerbose, tier from MethodFlags) gives per case the compilations before and inside the actual stage by tier and `hot_tier0` (measured code still at tier 0 during the case and promoted later); every unit's header states `jit check: PASS/FAIL`. Smoke: PASS on all 1,780 cases. Tiered JIT and PGO at net8.0 defaults (stated). rpc: two passes of max(64 calls, 50 ms) per cell, then fixed iterations, tier not read back |
| 25 | allocator/GC warm, GC stated | met: codec: pre-warm, BDN warm-up per case and its forced GCs between iterations (default, kept), GC counts, pause time and heap size recorded per case; rpc: warm-up for every arm, GC.Collect before every round; workstation concurrent GC stated |
| 26 | correctness before timing | met: the runner requires `gen/gate.sh` passed at identical content (the gate builds BenchDotNet and gates the no-unknown build in its step 8: layout 240/340 facts, byte identity, the loaded core is the variant with a control on the full core, its crossing counts, the corpus with every unknown row in the dropped form, rule 6; net8.0 and net6.0); every BDN process re-checks byte identity of every timed encode arm and every arm on every unknown row before timing (full: retain = incumbent's bytes; no-unknown: host-gen and core-ffi agree on the dropped form), and refuses to run if its core is not its variant |
| 27 | header | met: runner (commit, dirty tree refused unless AK_ALLOW_DIRTY, machine, flags, repeats, gate) + process (BDN version and toolchain, runtime, tiering, incumbent version, plan options, the job: iterations, warm-up, iteration time, strategy, EvaluateOverhead=false, clocks, arm order, checks, case count) |
| 28 | JSON lines, raw | met: a BDN exporter writes one line per raw Workload/Actual measurement (no outlier removal, no overhead subtraction: EvaluateOverhead=false; BDN's outlier handling touches only its console summary), plus one round-0 CPU row per case; failed cases are written as `# FAILED CASE` and fail the process |
| 29 | logs per suite and launch in ffi/logs/csharp/campaign/ | met |
| 30 | summaries | none produced by this slice (optional) |
| 31 | runner interface | met for the slice; the top-level `ffi/campaign.sh` is not this slice's file |
| 32 | smoke run | met: `logs/csharp/campaign/`, codec (both builds) and rpc (both clients) smokes, marked instrumentation; the rpc figures stripped |

D1 (`src/BenchDotNet`, retired at WP3) is **restored** as the codec-suite engine (req 22a) and builds in the gate; D1 is closed.

**Smoke runs** (`logs/csharp/campaign/`, CLIENT=0 SERVER=1, 1 launch, every file headed
"instrumentation, not a result"; **figures stripped** in the codec and rpc files):

- **codec, both builds** (WP5 step 10; commit `42f5372`, gate PASSED at `2410125`, identical
  content): 12 unit processes (7 full, 5 no-unknown), each checking its core is its variant;
  2,888 cases, 0 failed, JIT check PASS in every unit; by mode: core-ffi retain/drop/no-unknown
  336 each, host-gen retain/drop/no-unknown 336 each, core-ffi-pull drop and no-unknown 40 each,
  incumbent-prod 672 and incumbent-best 120 (default: both builds carry them as controls).
- **rpc, both clients** (commit `3a9b67c`, gate PASSED at that commit): full client 60 samples per
  transport (A, B, C-retain, C-drop, D-retain, D-drop, the B/C extras; x 2 directions x 3
  in-flight levels), no-unknown client 24 per transport (A, B, C-nounk, D-nounk); every call
  checked, none failed; the requirement-18 control aborted with 0 samples for both clients on
  both transports (`*.PLANT.jsonl`).
- calib (WP3, commit `5d81225`): 2 samples after the crossing-count gate.
- the gates each runner run took are kept by name (`gate-<commit>-<utc>-<suite>[-PLANT].log`).

`gate.log` is a copy of the last passed runner gate (the rpc smoke's, `3a9b67c`); the gate
content, full and no-unknown:
net8.0 and net6.0, conformance, core-ffi, crossing counts = `gen/crossings.txt`, corpus (managed
and ffi arms, strict retain), decision 11's controls, probe rows, controls.

**Engine cost (container, instrumentation; JOURNAL 51).** The per-case overhead is BDN's forced
GCs: 4 full collections per iteration (about 30 before the actual stage of a smoke case, about
115 per case at the default job), each walking the live heap. BDN keeps tens of kB per case
alive for the whole process (the harness itself holds under 1 MB after its checks), so with all
1,780 cases in one process the heap was 54 to 75 MB and each collection cost about 15 ms,
against about 5 ms with 60 to 110 cases (10 to 24 MB): GC pause was 50 to 80 % of each smoke
case. Per-case setup is 0.1 to 0.2 ms, except the P2.2 graphs (110 to 170 ms, 28 to 37 MB
allocated, outside the timing). Split per unit, a smoke case costs about 0.2 s (was 1.2 to
1.5 s). At the default job (100 ms iterations, 10 warm-up, 5 actual) a case costs about 2.5 s,
of which about 1.9 s precedes the actual stage (jitting, pilot, warm-up) and about 11 % is GC
pause: one unit of 336 cases took 17 to 18 min including its pre-warm (`logs/csharp/bdn-default-job-unit/`,
JIT check PASS at the default job). A launch is therefore about 80 to 100 min and 3 launches
about 4 to 5 h; the floor is the job itself (15 iterations of 100 ms plus the pilot),
not overhead. `AK_BDN_ONLY` (payload or row ids), `AK_BDN_NO_UNKNOWN=1` and `--unit` narrow a
run; `AK_BDN_TRACE=1` prints per-signal GC and setup timings to stderr.

## What the plan did not state

Reported at step 4 (vocabulary struct layouts, fixed entry points, flag values, the RPC
counting surface, the field-number limit). All are in the plan since `57b6180`/`41eb485`
(`plan.FIXED`, `MAX_FIELD_NUMBER`, `GROUP_DEPTH_LIMIT`) and the C# backends render them;
nothing is declared by hand any more.

## What is not measured or not established

- **No timing in this slice is a result.** Timing waits for the campaign (W13).
- Anything on .NET Framework 4.8; the RPC arm on any floor.
- A lossy-UTF-8 managed codec (the plan's alternative option) is not generated.
- The JIT read-back's split of measured code from BDN engine code is by method name (listed in
  `JitTiers.cs`, every counted method named in the rows); a method first compiled before the
  listener started (runtime start-up code) is never attributed to a case. The rpc and calib
  suites do not read the tier back. BDN's forced GCs are inside the CPU span (their pause is
  recorded, not subtracted).
- Unknown fields inside a map entry, in either codec: the facade map has no bag (the core
  delivers the entry's buffer and the host frees it). Decision 11's placement paths other
  than grow (pools, in-place refill, AK_ERR_CAPACITY) through this host.
- Google.Protobuf's message-size limit (ABI v1 decision 8) in the managed codec.
- ABI v1 decisions 4, 6, 10, 12 on .NET; decision 13 bounded only by the no-string ceiling.
- The `[UnmanagedCallersOnly]` guard removed; `packages/csharp`'s own object model; R2R;
  GC under load; content sets beyond P1.2/P2.2; bidirectional streaming.

## Next step

1. Decision 11's placement controls (a pre-allocated pool, in-place refill between
   deliveries, the oneof buffer move, AK_ERR_CAPACITY with no grow) are not built for C#:
   this host arms every position with grow only. The rust slice runs them.
2. The net48 gate on a Windows machine (D4), which first needs a net48 host half
   (delegate thunks rooted for the vtable's lifetime).
3. The campaign itself is the owner's: `run_campaign.sh` on the campaign machine (codec: 3
   launches under BDN, about 80 to 100 min per launch at the default job in this container;
   see the engine cost).

## Log index

| Log | What it establishes |
|---|---|
| `wp5s10-gate.log` | the WP5 step 10 gate: as step 9's, plus step 8, the NO-UNKNOWN build (layout 240/340 facts, byte identity, variant check with its control, crossing counts = `gen/crossings-nounk.txt`, corpus with drop strict, rule 6), net8.0 and net6.0 |
| `gate-repro/` | the gate at 253f487 and at ef00211, three runs each, each log under its own name with load and disk: 6 of 6 passed |
| `wp5s9-gate.log` | the WP5 step 9 gate (decision 11 port), core `e897f57`, clean core builds: layout incl. the options structs, conformance, core-ffi, crossing counts, corpus with strict retain (0 gaps) and its unkdrop control, decision 11's controls (per-position discard, pull == push, wrong root) and their plant, net8.0 and net6.0 |
| `bdn-default-job-unit/` | a TRIAL, not a campaign run: one unit (core-ffi:drop, 336 cases) at the default BDN job, for the JIT check (PASS) and the engine cost at that job |
| `campaign/` | smoke runs of `run_campaign.sh`: `gate.log` + `codec-launch1.jsonl` and one `codec-launch1.<unit>.bdn.log` per process (BDN, units, JIT check; unit 4 follow-up); rpc shipped+pinned, the rpc abort control, calib (WP3). Every figure is container instrumentation |
| `wp5-tail-gate.log` | the WP5-tail gate, core `41eb485`, clean core builds: as step 4's, plus the oracle-probe rows (11/11 on all four arms, net8.0 and net6.0, `P-field-maxplus1-in-group` refused); corpus 702 rows: managed 696/0, ffi 680/0 (+16 Nest), 6 disputed; RPC layout 6 structs / 20 members incl. `ak_rpc_counters` |
| `wp5-step4-gate.log` | the step-4 gate (core `77f91ee`): generator check and guard; core builds (init-guard); net8.0 and net6.0: conformance 152/152, unknown (retain = incumbent bytes), groups, utf8, mapforms, layout by name + section 10 (+ plant), core-ffi every shape incl. retain, R5 on the counting core, no-ak_init control; akrpc layout and error path; the corpus, four arms, net8.0 and net6.0, with four controls each; net48 compile |
| `wp5-step4-before-after.log` | the retired managed codec against the plan-rendered one on the corpus: C3 forms, C4 codes per row |
| `wp4-rd9-bytes-free.log` | R-D9 on the previous binding, with a negative control |
| `wp3-17-net6-floor-build.log` | net6.0 before this unit: the binding did not build |
| `stage21-wp4-regate.log` | the previous generator's gate (core `6ede244`) |
| `stage1` ... `stage20` | earlier stages; their timing parts are instrumentation |
