# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists in the tree
and what was checked. It carries no recommendation and no verdict (the decision is the owner's).
Every figure in this slice is container instrumentation (README 1.1), never a result; timing
waits for the campaign. The history of how each item got here is in `JOURNAL.md` (entries 1 to
56); this file states what is true now.

| | |
|---|---|
| **Status** | FIX-PLAN WP6 step 1 (2026-09-26). Both builds, full and no-unknown, gated from a clean checkout: see **Gate** below. |
| **Levels** (FIX-PLAN D2) | target **net8.0** (.NET 8.0.31, SDK 8.0.131); floor **net6.0** (.NET 6.0.36 from the NuGet runtime pack, self-contained publish): gated; floor **.NET Framework 4.8**: compiled only (`src/HarnessFloor`), never run (needs Windows; the container has no Mono) |
| **Incumbent** | Google.Protobuf 3.32.0, Grpc.Tools 2.72.0, Grpc.Net.Client and Grpc.AspNetCore 2.71.0 (the versions `packages/csharp` ships) |
| **Core** | the one core, `ffi/poc/codec`, built from `git archive HEAD` by `gen/build_core.sh`, every build with `init-guard`: full `target-core` (`rpc`), `target-core-count` (`rpc,count`), `target-core-corpus` (`corpus`); no-unknown (ak-core `--no-default-features`) `target-core-nounk`, `target-core-count-nounk`, `target-core-corpus-nounk`, each in its own target dir |
| **Machine** | a container, 4 vCPU Intel Xeon, Linux 6.18.44; nothing in this file depends on it |

## What exists

### The shared C# backends (`ffi/poc/codec/gen/`, owned by this slice)

Each imports `plan` (and `cs_names`/`cs_types`) and nothing from the IR or a description.
Every one renders both variants: the full plan, and the plan relowered with `unknown="drop"`
(`plan.unknown_compiled_out`), from the same functions.

```
cs_names.py         C# spellings and the file banner
cs_types.py         facade types (every class carries `UnknownFields`) and the structural comparer
cs_managed.py       the managed codec (host-gen): one-pass and two-pass encode, decode; utf8,
                    unknown mode and depth limit are the plan's options (drop plan: no capture code)
cs_binding.py       the P/Invoke binding: groups, vtables, presence bits, every import as
                    LibraryImport under #if NET7_0_OR_GREATER and DllImport under #else,
                    ak_init from plan.lifecycle, AbiLayout.Table()/Facts(), decision 11's
                    ak_dec_<Root>_opts and ak_dec_ctx_new_/ak_dec_reset_<Root> (full only),
                    AbiVariant (which variant, and a check that the loaded core is it); the RPC half
cs_host.py          CoreFfi_<Root>: encode / uencode, push and pull decode on one root-bound
                    context; full: retain through native options, one grow over
                    NativeMemory.Realloc, reset/decode/reset, bags taken into UnknownFields,
                    UNDELIVERED check, per-position discard helpers; no-unknown: none of these
cs_layout_probe.py  the layout probe's Rust source, parsed out of ak-abi's Rust declaration text
                    (both variants, shapes and corpus); reads no plan (R-E6)
```

### This slice (`poc/csharp/`)

```
gen/generate.py [--check]   renders every generated file (both variants) through the shared
                            backends; --check = drift + the one-generator guard + its planted test
gen/glue.py, cs_values.py, cs_build.py, cs_arms.py, cs_proj.py, cs_registry.py, cs_campaign.py
                            harness glue only (values, payload builders, arm table, corpus
                            projection and dispatch, core-ffi registry, campaign per-root calls)
gen/build_core.sh           the core snapshot, the six core builds, the four layout probes
gen/gate.sh                 THE GATE (correctness only, nothing timed), both variants, net8.0 + net6.0
gen/crossings.txt           committed crossing counts, full build (18 rows)
gen/crossings-nounk.txt     committed crossing counts, no-unknown build (18 rows)
abi/                        the layout probe crate (features: unknown-fields default, corpus)
Directory.Build.props/.targets  build configurations: /p:AkNounk=true (the no-unknown build:
                            GeneratedNounk/ replaces the variant files, define AK_NO_UNKNOWN_FIELDS,
                            output bin-nounk/ obj-nounk/); /p:AkFloor=true (README 5.2 arm b, not gated)
src/Facade/                 the hand-written runtime (Wire.cs, OrderedMap.cs) + Generated/ (and
                            GeneratedNounk/Codec.cs)
src/Harness/                harness: conformance | unknown | groups | utf8 | mapforms | layout |
                            coreffi | content | counts | bench; net8.0 + net6.0; Generated/,
                            GeneratedNounk/
src/Corpus/                 the corpus runner (net8.0 + net6.0): 4 arms full, 2 arms no-unknown,
                            each row in a child process; --unk-controls, --wrong-root, --variant
src/Rpc/                    akrpc: `campaign --suite rpc|rpc-server|calib`, the gate's --layout and
                            --error-path, and the legacy modes (--grid, --stream, Bench.cs; D42)
src/BenchDotNet/            the codec suite's engine: BenchmarkDotNet 0.15.8, InProcessEmit, one
                            process per arm:mode unit, JIT tier read back (JitTiers.cs)
src/HarnessFloor/           net48, compile only (the binding; the host half is compiled out)
run_campaign.sh             --suite codec|rpc|calib|gate --out DIR (CAMPAIGN req 31)
```

## Gate (FIX-PLAN WP6 step 1): clean checkout

`logs/csharp/wp6s1-gate.log`: **GATE PASSED** at commit `ee94026`, run in a fresh git worktree
(no uncommitted change, every build directory new; the worktree and its builds deleted after),
core commit `d89bdfc`, net8.0 and net6.0. Correctness only; nothing is timed.

- **Generator:** `generate.py --check` current against a snapshot of the committed `poc/codec/gen`;
  the one-generator guard sees the shared backends and the glue import no IR, and catches its plant.
- **Full build**, net8.0 and net6.0:
  - layout by name both ways, 105 structs / 490 members, and 400 section-10 facts (shapes); 199 /
    780 and 574 facts (corpus); the layout plant fails;
  - conformance 152/152; unknown, groups, utf8, mapforms; core-ffi on every payload, push, pull
    and retain; crossing counts = `gen/crossings.txt` (18 rows), the host's tally = the core's (R5);
    the no-ak_init control fails;
  - the corpus, 4 arms: managed 696 pass / 0 fail, ffi 680 pass / 0 fail (16 roots not in the ABI),
    6 disputed; strict retain: no retain arm wrote the dropped form on a non-disputed row (its
    `unkdrop` control fails with 307 gaps); proj, reenc, accept and noinit controls fail;
  - decision 11's controls: 543 rows, 2,290 (row, position) pairs, 307 rows with unknowns, 0
    discard mismatches, pull == push, 0 undelivered; the wrong-root refusal passes; the plant
    fails with 307 rows;
  - the oracle-probe rows 11/11; akrpc layout (6 structs / 20 members) and error path.
- **No-unknown build**, net8.0 and net6.0: layout 78 structs / 305 members and 240 facts (shapes),
  340 facts (corpus); conformance 152/152; the loaded core is the variant (0 `ak_uencode_*`
  exports against 7) and the control that loads the full core under this binding fails; crossing
  counts = `gen/crossings-nounk.txt`; the corpus, 2 arms (managed 696, ffi 680 pass), every row
  dropped in the dropped form (313 and 307), none retained; the wrong-root refusal passes.
- **Crossing counts, no-unknown against full:** the three P1.2 rows (ASCII, Latin-1, wide) differ,
  push-decode reverse 8 -> 5; every other row is equal.
- net48: the binding compiles (DllImport branch); nothing runs it.

## Open defects

| # | Where | What |
|---|---|---|
| D4 | net48 | compiled only; no gate on .NET Framework (needs Windows); the core-ffi host half has no net48 form (needs delegate thunks rooted for the vtable's lifetime) |
| D42 | `akrpc` legacy modes (`--grid`, `--stream`, `Bench.cs`) | pre-campaign timing harnesses, not campaign-conformant (in-process server, `Process.TotalProcessorTime`); kept in the tree, not used by the runner or the gate |

## Campaign readiness (design/CAMPAIGN.md at 85cb00f; section 10 checklist)

The runner is `poc/csharp/run_campaign.sh`. The codec suite runs under BenchmarkDotNet (req
22a); the rpc and calib suites under `akrpc campaign`. Each suite runs both builds (full and
no-unknown) per launch, in an order alternated by launch. The RPC suite stays on the slice's
runner because BDN would break req 18 (it records a failed case and continues, where one failed
call must abort the run) and its pilot would change the number of calls per sample; req 22a
allows this, stated here.

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | not applicable in the container: the machine is the owner's; the runner runs one process at a time per suite |
| 2 | governor, turbo, SMT | not applicable in the container: set by the owner; the runner records them in every header (sysfs) |
| 3 | isolation | not applicable in the container: set by the owner; the runner records isolcpus/nohz_full and the cgroup cpuset |
| 4 | three disjoint CPU sets | met: `AK_CPU_CLIENT` / `AK_CPU_SERVER`, `taskset` per process; OS set is everything else |
| 5 | floors gated for correctness | net6.0: met (the gate, both builds). .NET Framework 4.8: **not met**: compiled only, needs a Windows machine (owner decision: provide one, or record that the floor was not run) |
| 6 | build flags printed | met: Release, net8.0, core features (init-guard on), shared `libak_core.so`, build variant, JIT and GC settings, in every header; the core's cargo profile (`lto = false`) is stated here, not printed |
| 7 | payloads | met: 16 payloads; Latin-1 and wide content sets on P1.2 and P2.2 (SHAPES.md); every corpus `U-*` row with a shapes root, disputed excluded (92 rows) |
| 8 | arms | met: incumbent-prod (Grpc.Net marshaller shape over ReadOnlySequence), incumbent-best (WriteTo(IBufferWriter) / ParseFrom(ReadOnlySpan)), core-ffi (push; pull as `core-ffi-pull`), host-gen (the managed codec from the same generator) |
| 9 | directions | met: encode, decode, decode-read (a generated visitor reads every field); decode-reencode on the unknown rows |
| 10 | unknown fields, three modes | met: core-ffi and host-gen in retain and drop (full build) and no-unknown (its own build, `/p:AkNounk=true`, core `--no-default-features`; host-gen no-unknown is the managed codec rendered from the drop plan); core-ffi-pull in drop and no-unknown; incumbent in its default mode (retains); `unknown_mode` on every sample. A map entry's unknown fields are not held by the C# facade (U-map-entry, disputed) |
| 11 | serialised once per iteration | met: Google.Protobuf keeps no size memo (CalculateSize recomputes); core-ffi and host-gen reset per call |
| 12 | cells A-D, C and D per unknown mode | met: full client A, B, C-retain, C-drop, D-retain, D-drop (+ labelled B/C callback and queue rows, C.* in drop); no-unknown client A, B, C-nounk, D-nounk (A and B its in-process controls); both clients against the same server per transport and launch |
| 13 | server separate process, pre-serialised | met: Kestrel in its own process on `AK_CPU_SERVER`, P2.2 pre-serialised; direction b decoded by the incumbent |
| 14 | directions a and b | met; the optional streamed upload is not built |
| 15 | 1/8/16 in flight | met |
| 16 | B/C blocking; callback/queue labelled | met |
| 17 | shipped and pinned | met: shipped = packages/csharp's UDS client configuration and Kestrel defaults; pinned = 4 MiB stream and connection windows, adaptive off, Nagle off (no effect on UDS, stated) |
| 18 | every call checked, abort | met: status and length on every call, a retained decode that leaves a buffer undelivered fails its call; one failure aborts with no sample; the `--plant` control (wrong expected length) aborts, both clients |
| 19 | crossing counts gate | met, both builds: `gen/crossings.txt` and `gen/crossings-nounk.txt` (P1.2 in all three content sets included), in the gate and before calib |
| 20 | crossing cost fwd/rev, perf stat | **not met here**: calib has a forward row (ak_noop) and a forward-and-reverse row (ak_noop_reverse; reverse = the difference); `perf stat` runs when installed and is not installed in this container (owner: install perf on the campaign machine) |
| 21 | CPU clocks | met with the stated fallback: codec (BDN) wall per iteration, raw, plus process CPU (getrusage RUSAGE_SELF) per case over BDN's actual stage, including BDN's forced GCs (their pause recorded beside it); per-iteration and thread CPU are not available from BDN, stated in every header; rpc: getrusage of the client beside wall per batch; calib: CLOCK_THREAD_CPUTIME_ID |
| 22 | order rotated between launches | met: codec, one process per arm:mode unit, unit order rotated by launch; the two builds alternate by launch; rpc: cells rotated per round, clients alternate by launch |
| 22a | benchmark engine | met: BenchmarkDotNet for the codec suite (InProcessEmit, pinned by the runner, raw measurements exported, warm-up and tier recorded); the RPC grid on the runner, reason above |
| 23 | 5 rounds x 3 launches | met (defaults): codec 5 actual iterations per case x 3 launches, every one exported; rpc and calib 5 rounds x 3 launches |
| 24 | warm-up stated, identical, optimising tier | met: per BDN process a pre-warm to JIT quiescence and 2 unexported prime cases, then per case BDN's jitting, pilot and a fixed warm-up count; the JIT tier is read back per case from the runtime's JIT events and every unit states `jit check: PASS/FAIL`; rpc: two warm-up passes per cell, tier not read back |
| 25 | allocator/GC warm, GC stated | met: warm-up per arm, workstation concurrent GC stated, GC counts and pause recorded per BDN case |
| 26 | correctness before timing | met: the runner requires the gate passed at identical content (both builds, byte identity, the corpus through every arm in drop and retain and in no-unknown, the planted controls), and every BDN process re-checks byte identity and the unknown rows before timing |
| 27 | header | met: commit (dirty tree refused), machine, CPU sets, runtime and incumbent versions, build flags and variant, core features, transport, warm-up and repeats |
| 28 | JSON lines, raw | met: one line per raw BDN measurement and per rpc/calib sample, section 7's fields, `unknown_mode` included; raw output committed |
| 29 | logs in ffi/logs/csharp/campaign/ | met |
| 30 | summaries | not applicable: this slice produces none (optional) |
| 31 | runner interface | met for the slice; the top-level `ffi/campaign.sh` is the aggregating session's |
| 32 | smoke run | met: `logs/csharp/campaign/`, codec (both builds) and rpc (both clients) and the rpc abort control, figures stripped; calib smoke marked instrumentation |

**Smoke logs** (`logs/csharp/campaign/`, 1 launch, CLIENT=0 SERVER=1): codec at `42f5372`
(2,888 cases, 12 unit processes, 0 failed, JIT check PASS in every unit), rpc at `3a9b67c` (full
client 60 samples per transport, no-unknown client 24; the abort control 0 samples for both),
calib at `5d81225` (2 samples). Timing fields stripped in the codec and rpc files; the calib file
is marked instrumentation. Every runner gate run is kept by name
(`gate-<commit>-<utc>-<suite>[-PLANT].log`).

**Engine cost, container instrumentation** (`logs/csharp/bdn-default-job-unit/`): one unit of 336
cases at the default BDN job (10 warm-up, 5 x 100 ms) ran 17 to 18 minutes. A campaign launch has
12 such processes (40 to 336 cases each). How the per-case overhead was traced to BDN's forced GCs
is in JOURNAL 51.

## What is not measured or not established

- **No timing in this slice is a result.** Every figure is container instrumentation.
- Anything on .NET Framework 4.8 (compiled only); no floor runs the RPC suite or BDN.
- `perf stat` cycles and instructions (not installed here), so req 20's per-iteration counts.
- Per-iteration and thread CPU time under BenchmarkDotNet (BDN offers neither).
- The JIT tier of the RPC suite's code (read back only in the codec suite).
- A lossy-UTF-8 codec (the plan's alternative option) is not generated.
- Unknown fields inside a map entry, in either codec: the facade map has no bag.
- Decision 11's placement paths other than grow through this host: pre-allocated pools,
  in-place refill between deliveries, the oneof buffer move, AK_ERR_CAPACITY with no grow.
- Google.Protobuf's message-size limit (ABI v1 decision 8) in the managed codec.
- ABI v1 decisions 4, 6, 10, 12 on .NET; decision 13 (borrowed strings) bounded only by a
  no-string ceiling (`G.SkipStrings`).
- The streamed upload of req 14 (optional) and bidirectional streaming.
- `packages/csharp`'s own object model; ReadyToRun; GC under load; content sets beyond P1.2
  and P2.2.

## Next step

1. The net48 gate on a Windows machine (D4), which first needs a net48 host half.
2. The campaign itself is the owner's: `run_campaign.sh` on the campaign machine.

## Log index

| Log | What it establishes |
|---|---|
| `wp6s1-gate.log` | the clean-checkout gate of WP6 step 1, both builds, net8.0 and net6.0 (see Gate) |
| `wp5s10-gate.log` | the step-10 gate (no-unknown variant added as step 8), at `2410125` |
| `wp5s9-gate.log` | the step-9 gate (decision 11 port), at `8d2e7ac` |
| `gate-repro/` | the gate at 253f487 and ef00211, three runs each, each log under its own name: 6 of 6 passed (the once-seen failure at 253f487 did not reproduce; cause not found, JOURNAL 54) |
| `campaign/` | the smoke runs of `run_campaign.sh` and the runner's gate logs |
| `bdn-default-job-unit/` | a trial of one BDN unit at the default job, for the JIT check and the engine cost (instrumentation) |
| `wp5-tail-gate.log`, `wp5-step4-gate.log`, `wp5-step4-before-after.log`, `wp4-rd9-bytes-free.log`, `wp3-17-net6-floor-build.log`, `stage21-wp4-regate.log` | earlier gates and checks, superseded by the gates above |
| `stage1` ... `stage20`, `bdn-results/`, `calibration-rust-crossing.log` | pre-campaign stages; every timing in them is instrumentation and none is used. **Stages 18 and 19 were built against a core not on the branch (FIX-PLAN R-C9)**: relabelled in their headers, no figure from them is usable |
