# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists in the tree
and what was checked. It carries no recommendation and no verdict (the decision is the owner's).
Every figure in this slice is container instrumentation (README 1.1), never a result; timing
waits for the campaign. The history of how each item got here is in `JOURNAL.md` (entries 1 to
58); this file states what is true now.

| | |
|---|---|
| **Status** | FIX-PLAN WP7 (the 2026-09-26 contract, R-H22 to R-H36) built and gated for this slice; both builds, full and no-unknown, gated from a fresh worktree at `c35bd22`: see **Gate** below; smoke of every suite: see **Smoke**. Findings from now on are in scope only if they can change what the campaign measures (ffi/CLAUDE.md, "Scope of findings"). |
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
cs_types.py         facade types and the structural comparer; every class carries `UnknownFields` in the
                    full build and none in the no-unknown build (CAMPAIGN req 10, R-H22)
cs_managed.py       the managed codec (host-gen): one-pass and two-pass encode, decode; utf8 and the
                    depth limit are the plan's options; ONE unknown-field mode per codec ("both"
                    refused, R-H11): `Codec` from the drop plan (no capture code), `CodecRetain`
                    from the retain plan (the full build only)
cs_binding.py       the P/Invoke binding: groups, vtables, presence bits, every import as
                    LibraryImport under #if NET7_0_OR_GREATER and DllImport under #else, and
                    under AK_HOST_COUNT a wrapper counting its calls by name (req 19, WP7),
                    ak_init from plan.lifecycle, AbiLayout.Table()/Facts(), decision 11's
                    ak_dec_<Root>_opts and ak_dec_ctx_new_/ak_dec_reset_<Root> (full only),
                    AbiVariant (which variant, and a check that the loaded core is it); the RPC half
cs_host.py          CoreFfi_<Root>: encode / uencode, push and pull decode on one root-bound
                    context; full: retain through native options, one grow over
                    NativeMemory.Realloc, reset/decode/reset, bags taken into UnknownFields,
                    UNDELIVERED check, per-position discard helpers, UnkHost.Exact (exact-size
                    grow, set by the counting runs); no-unknown: none of these
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
gen/crossings.txt           committed R5 counts, full build (22 rows: the CoreArms tally against the
                            core's own counters; P1.2, P2.2, P2.4 in three content sets)
gen/crossings-nounk.txt     the same, no-unknown build (22 rows)
gen/counts.txt              CAMPAIGN req 19 as amended: every exported entry point one call of each
                            timed core-ffi case calls, by name, resets and grows placed (1,044 cases)
gen/counts-nounk.txt        the same, no-unknown build (544 cases)
gen/rpc-counts.txt          the same per call of every RPC cell A-F and direction (30 rows)
gen/rpc-counts-nounk.txt    the same, no-unknown client (18 rows)
abi/                        the layout probe crate (features: unknown-fields default, corpus)
Directory.Build.props/.targets  build configurations: /p:AkNounk=true (the no-unknown build:
                            GeneratedNounk/ replaces the variant files, define AK_NO_UNKNOWN_FIELDS,
                            output bin-nounk/ obj-nounk/); /p:AkHostCount=true (the counting build,
                            AK_HOST_COUNT, bin-count[-nounk]/, never timed); /p:AkFloor=true (README
                            5.2 arm b, not gated)
src/Facade/                 the hand-written runtime (Wire.cs: error codes equal plan.FIXED's, R-H14;
                            OrderedMap.cs) + Generated/ (Types, Eq, Codec, CodecRetain, Values, Build)
                            and GeneratedNounk/ (Types and Eq without UnknownFields, Codec)
src/Harness/                harness: conformance | unknown | groups | utf8 | mapforms | layout |
                            coreffi | content | counts | bench; net8.0 + net6.0; Generated/,
                            GeneratedNounk/
src/Corpus/                 the corpus runner (net8.0 + net6.0): 4 arms full, 2 arms no-unknown,
                            each row in a child process; --unk-controls, --wrong-root, --variant
src/Rpc/                    akrpc: `campaign --suite rpc|rpc-server|rpc-warm|calib`, `--suite rpc --counts`
                            (the counting build); cells A-F, one channel each, the gate's --layout and
                            --error-path, and the legacy modes (--grid, --stream, Bench.cs; D42)
src/BenchDotNet/            the codec suite's engine: BenchmarkDotNet 0.15.8, InProcessEmit, one
                            process per arm:mode unit, process CPU per iteration (CpuClock), JIT tier
                            read back (JitTiers.cs), encode variants and pools (Cases.cs), the Grpc.Net
                            frame (GrpcFrame.cs), `--counts` (CountRun.cs, the counting build)
src/HarnessFloor/           net48, compile only (the binding; the host half is compiled out)
run_campaign.sh             --suite codec|rpc|calib|gate --out DIR (CAMPAIGN req 31)
```

## Gate (FIX-PLAN WP7): clean checkout

`logs/csharp/wp7-gate.log`: **GATE PASSED** at commit `c35bd22` (this slice's WP7 commit, on
`44f9138`, the cs_binding/cs_host change), run in a fresh git worktree with every build directory
new, core from `git archive HEAD` of `poc/codec`, net8.0 and net6.0, both builds. Correctness
only; nothing is timed. 0 gate-step failures; all 27 planted controls fail as required.

- Steps 1 to 8 as in WP6 (generator from a snapshot of the committed `poc/codec/gen`; core and
  layout probe; full build net8.0 and net6.0: layout, conformance, unknown, groups, utf8,
  mapforms, core-ffi push/pull/retain, R5 counts = `gen/crossings.txt` now with P2.2/* and
  P2.4/* rows, the missing-row and empty-file controls; akrpc layout and error path; the corpus,
  4 arms, C4 codes, strict retain, decision 11's controls and plants, probe rows; the no-unknown
  build, net8.0 and net6.0, with its own layout, conformance, variant check, counts, corpus).
- **Step 9 (new, req 19 as amended):** the counting builds (`/p:AkHostCount=true`, both
  variants); `gen/counts.txt` (1,044 rows) and `gen/counts-nounk.txt` (544) equal row for row
  to what the counting BenchDotNet produces from each core-ffi case's own timed closure;
  `gen/rpc-counts.txt` (30) and `gen/rpc-counts-nounk.txt` (18) equal to one call per cell and
  direction against a server process; a control with the timed runs' doubling growth must
  differ, and does (the exact-size grow is live).

## Register H (WP6) and WP7

The register H findings for this slice (R-H2, R-H3, R-H6, R-H9, R-H11, R-H14, R-H15, R-H18,
R-H19, R-H22, R-H23) were confirmed and fixed in WP6; evidence and dispositions per finding in
JOURNAL 57. WP7's ten items and what each built are in JOURNAL 58 and in the checklist below.

**Rows C4 does not code-check.** In the conformance corpus: none; every one of its 146 reject
rows states a `reject.reason` and each maps to a code (malformed, truncated, depth or transcode,
per plan.py's DECODE RULES), so C4 checks the code on every reject row there. In the
oracle-probe manifest (`poc/rust/gen/probe_corpus.py`, run by the gate): the **3** reject rows
`P-field-maxplus1`, `P-field-2p32plus2`, `P-field-maxplus1-in-group`, which carry no `reject`
object, so no expected code exists to compare with. Their refusal is still required (an
acceptance or an exception fails the row), and the code each arm returned (-2) is printed in
the row's form, so the skip is visible, not silent; what is not checked is only a code the row
itself does not specify.

## Open defects

| # | Where | What |
|---|---|---|
| D4 | net48 | compiled only; no gate on .NET Framework (needs Windows); the core-ffi host half has no net48 form (needs delegate thunks rooted for the vtable's lifetime) |
| D42 | `akrpc` legacy modes (`--grid`, `--stream`, `Bench.cs`) | pre-campaign timing harnesses, not campaign-conformant (in-process server, `Process.TotalProcessorTime`); kept in the tree, not used by the runner or the gate |

## Campaign readiness (design/CAMPAIGN.md at 3210f28; section 10 checklist)

Assessed against CAMPAIGN.md as amended through 3210f28 (the owner's 2026-09-26 decisions,
R-H22 to R-H36). The runner is `poc/csharp/run_campaign.sh`. The codec suite runs under
BenchmarkDotNet (req 22a); the rpc and calib suites under `akrpc campaign`. Each suite runs both
builds (full and no-unknown) per launch, in an order alternated by launch. The RPC suite stays on
the slice's runner because BDN would break req 18 (it records a failed case and continues, where
one failed call must abort the run) and its pilot would change the number of calls per sample;
req 22a allows this, stated here.

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | not applicable in the container: the machine is the owner's; the runner runs one measured process at a time |
| 2 | governor, turbo, SMT | not applicable in the container: set by the owner; recorded in every header (sysfs) |
| 3 | isolation | not applicable in the container: set by the owner; isolcpus/nohz_full and the cgroup cpuset recorded |
| 4 | three disjoint CPU sets, fixed sizes, thread counts | met: `AK_CPU_CLIENT` / `AK_CPU_SERVER` from the environment (ffi/campaign.sh exports ffi/campaign.machine's) or, run alone outside a smoke, read from ffi/campaign.machine; a set whose size is not `AK_SET_SIZE` is refused; `taskset` per process; the header names the source. Worker thread counts in every header: .NET thread pool min/max and current, caller threads, the core runtime's workers (the codec suite creates none), the server's |
| 5 | floors gated for correctness | net6.0: met (the gate, both builds). .NET Framework 4.8: **not met**: compiled only, needs a Windows machine (owner decision: provide one, or record that the floor was not run) |
| 6 | build flags printed | met: Release, net8.0, core features (init-guard on), shared `libak_core.so`, build variant, JIT and GC settings, in every header; the core's cargo profile (`lto = false`) is stated here, not printed |
| 7 | payloads | met as amended: 16 payloads; Latin-1 and wide on P1.2, P2.2 and P2.4 (R-H26); the 92 accepted, non-disputed U-* rows at the shapes core's 7 ABI roots through the timed shapes core in encode (`encode-hot`: the arm's own decode of the row, untimed, re-encoded), decode and decode-read, every arm including incumbent-best (R-H27); decode-reencode a labelled extra |
| 8 | arms | met: incumbent-prod (Grpc.Tools' marshaller shape: CalculateSize + WriteTo(IBufferWriter), ParseFrom(ReadOnlySequence)), incumbent-best (WriteTo(IBufferWriter) / ParseFrom(ReadOnlySpan)), core-ffi (push; pull as `core-ffi-pull`), host-gen |
| 9 | directions | met: encode (4 variants, req 11), decode, decode-read (a generated visitor reads every field) |
| 10 | unknown fields, three modes | met: core-ffi and host-gen in retain and drop (full build; `Codec` and `CodecRetain`) and no-unknown (its own build and core, no facade member, R-H22); core-ffi-pull in drop and no-unknown; incumbent default (retains); `unknown_mode` and `build` on every sample |
| 11 | serialised once per iteration; encode variants | met as amended (R-H29): `encode` (pool + reused buffer), `encode-hot` (one graph + reused buffer), `encode-transport` (pool + the Grpc.Net form), `encode-transport-hot`; rows carry `enc_end`, `enc_input`, `pool_graphs`, `pool_bytes`. Reused buffer: the incumbent's BufWriter, host-gen's Enc, the core's encode buffer. Transport form: the serializer cells A, F, D run (shared code) into a frame built as Grpc.Net.Client 2.71's GrpcCallSerializationContext builds it (checked by reflection); on the core's transport (C, E) the form is the buffer row, stated; incumbent-best has no transport row. Pool: retained heap >= 2 x AK_LLC_BYTES (13.75 MB default), measured and topped up; a hot input is a pool of one (same per-call step); graph construction always in the case's setup. Google.Protobuf keeps no size memo |
| 12 | cells A-F, modes | met as amended (R-H35): full client A, B, C-retain, C-drop, D-retain, D-drop, E-retain, E-drop, F-retain, F-drop (+ labelled B/C callback and queue rows); no-unknown client A, B, C-nounk, D-nounk, E-nounk, F-nounk |
| 13 | server: separate, pre-serialised, one per launch, warmed; one channel per cell | met as amended (R-H33): one server process per launch (two Kestrel hosts in it: shipped and pinned sockets, since Kestrel's windows are per host), serving both builds; before any client, 2,000 calls per direction from each client transport (Grpc.Net, the core's) per socket, every call checked (logged; the server prints what it served); every cell its own channel for the whole launch, opened and warmed in the client's warm-up; P2.2 pre-serialised; direction b decoded by the incumbent |
| 14 | directions a, a+read, b | met as amended (R-H36); the optional streamed upload is not built |
| 15 | 1/8/16 in flight | met |
| 16 | delivery | met as amended (R-H30): B, C, E the core's blocking call on caller threads created before the warm-up; A, D, F Grpc.Net's idiomatic `await CallInvoker.AsyncUnaryCall` (as Grpc.Tools' generated client does), k in flight = k async loops on the thread pool, stated in the header; callback and queue rows labelled extras, awaited |
| 17 | shipped and pinned, UDS | met: UDS; shipped = packages/csharp's UDS client configuration and Kestrel defaults; pinned = 4 MiB stream and connection windows, adaptive off, Nagle off (no effect on UDS, stated) |
| 18 | every call checked, abort | met: status and length on every call (the server warm-up's included); a retained decode that leaves a buffer undelivered fails its call; the `--plant` control aborts with no sample, both clients |
| 19 | crossing counts gate | met as amended (R-H31): every exported entry point the timed code calls, counted by name in a counting build, resets included and placed (two per decode: before, with the options or NULL, and after, NULL), retain with no pre-placed buffer and an exact-size grow; per codec case (`gen/counts*.txt`) and per call of the RPC cells (`gen/rpc-counts*.txt`, B to E, A and F listed); gated in step 9 with a must-differ control. The R5 counts (`gen/crossings*.txt`) are gated too and checked before calib |
| 20 | crossing cost fwd/rev, perf stat | **not met here**: calib has a forward row (ak_noop) and a forward-and-reverse row; `perf stat` runs when installed and is not installed in this container (owner: install perf on the campaign machine) |
| 21 | CPU is process CPU per round | met as amended (R-H25): codec, CLOCK_PROCESS_CPUTIME_ID per BDN iteration (the job's clock, read at the same iteration boundaries as the wall time; a case without one value per iteration fails); rpc, getrusage(RUSAGE_SELF) of the client per sample beside wall; calib (the crossing benchmark, req 20) keeps CLOCK_THREAD_CPUTIME_ID of its one loop thread |
| 22 | order randomised where the framework allows | met: codec, the unit order of a launch and the case order in each BDN process are seeded shuffles, seeds in the headers; builds alternate by launch; rpc, the cell order of every round a seeded shuffle; transports and builds alternated by launch |
| 22a | benchmark engine | met: BenchmarkDotNet for the codec suite (InProcessEmit, pinned by the runner, raw measurements exported, warm-up and tier recorded); the RPC grid on the runner, reason above |
| 23 | 5 rounds x 3 launches | met (defaults) |
| 24 | warm-up stated, identical; GC/JIT defaults stated | met: codec, per BDN process a pre-warm to JIT quiescence (the job's clock included) and 2 unexported prime cases, then per case BDN's jitting, pilot and a fixed warm-up count; the JIT tier read back per case, `jit check: FAIL` fails the unit. rpc: warm-up rounds of 64 calls per cell, direction and level until a round compiles nothing (at most 10); `jit_in_window` per sample. GC and JIT between blocks at the framework defaults, stated |
| 25 | allocator/GC warm, GC stated | met: warm-up per arm, workstation concurrent GC stated, GC counts and pause per BDN case (summary row) |
| 26 | correctness before timing | met: the runner requires the gate passed at identical content (both builds, counts included); every BDN process re-checks byte identity of every encode arm and variant (transport frames and pooled graphs included) and every U-* row's encode and re-encode forms before timing |
| 27 | header | met: commit (dirty tree refused), machine, CPU sets and their source, runtime and incumbent versions, build flags and variant, core features, transport, threads, warm-up and repeats |
| 28 | JSON lines, raw | met: one line per BDN iteration and per rpc/calib sample, section 7's fields plus `build` and the encode-variant fields; the per-case BDN summary row carries `row: case-summary` and no `cpu_ns`/`wall_ns` |
| 29 | logs in ffi/logs/csharp/campaign/ | met |
| 30 | summaries | not applicable: this slice produces none; any ratio is to come from per-launch medians (stated in every header) |
| 31 | runner interface | met for the slice; the top-level `ffi/campaign.sh` is the aggregating session's |
| 32 | smoke run | see **Smoke** below |

**Smoke** (`logs/csharp/campaign/wp7-smoke/`, run from the clean worktree at `c35bd22` after
its gate passed; 1 launch, 1 round, CLIENT=0 SERVER=1, smoke pool 64 KiB, 6 of the 92 U-* rows;
figures stripped in every JSON-lines file, the calib file and the `.bdn.log` files headed as
instrumentation):
- codec: 12 BDN unit processes (both builds), 1,548 samples, 0 failed cases, `cpu check: PASS`
  and `jit check: PASS` in every unit; every new row present: encode, encode-hot,
  encode-transport, encode-transport-hot; latin1 and wide on P1.2, P2.2 and P2.4; U-* rows in
  encode-hot, decode, decode-read (and decode-reencode) for every arm of both builds, including
  incumbent-best; `cpu_ns` on every sample.
- rpc: one server process for the launch, warmed by 100 calls per direction per client transport
  per socket (smoke count; 2,000 in the campaign); full client 126 samples per transport (cells
  A, B, C-retain/-drop, D-retain/-drop, E-retain/-drop, F-retain/-drop and the 4 extras, directions
  a, a+read, b), no-unknown client 54 (A, B, C/D/E/F-nounk), 0 aborts. The `--plant` control:
  every client (both builds, both transports) aborted with 0 samples. The plant run reused the
  file names `rpc-launch1.server.log` and `rpc-launch1.server-warm.log`, so the ones committed are
  the plant run's server (400 warm-up calls, 4 aborted client calls); the real run's server log
  was overwritten (a log-naming slip; no effect on any sample, not fixed under the scope rule).
- calib: 2 samples, crossing counts equal to both committed files.

**Engine cost, container instrumentation** (`logs/csharp/bdn-default-job-unit/`, before WP7):
one unit of 336 cases at the default BDN job (10 warm-up, 5 x 100 ms) ran 17 to 18 minutes.
WP7 roughly triples the encode cases (four variants, U-* encode) and adds pool setups; the
campaign's codec suite is correspondingly longer.

## What is not measured or not established

- **No timing in this slice is a result.** Every figure is container instrumentation.
- Anything on .NET Framework 4.8 (compiled only); no floor runs the RPC suite or BDN.
- `perf stat` cycles and instructions (not installed here), so req 20's per-iteration counts.
- Thread CPU time under BenchmarkDotNet (the campaign's figure is process CPU, req 21).
- Cell B's encode form (incumbent into a span for the core's transport) as a codec-suite row.
- U-* rows with a pool input or a transport end state (encode-hot only).
- A lossy-UTF-8 codec (the plan's alternative option) is not generated.
- Unknown fields inside a map entry, in either codec: the facade map has no bag.
- Decision 11's placement paths other than grow through this host: pre-allocated pools,
  in-place refill between deliveries, the oneof buffer move, AK_ERR_CAPACITY with no grow.
- Google.Protobuf's message-size limit (ABI v1 decision 8) in the managed codec.
- ABI v1 decisions 4, 6, 10, 12 on .NET; decision 13 (borrowed strings) bounded only by a
  no-string ceiling (`G.SkipStrings`).
- The streamed upload of req 14 (optional) and bidirectional streaming.
- `packages/csharp`'s own object model; ReadyToRun; GC under load; content sets beyond P1.2,
  P2.2 and P2.4.

## Next step

1. The aggregating session reads WP7 (JOURNAL 58) and pushes; this slice changes nothing further
   unless a finding in scope (ffi/CLAUDE.md, "Scope of findings") comes back.
2. The net48 gate on a Windows machine (D4), which first needs a net48 host half.
3. The campaign itself is the owner's: `run_campaign.sh` (through `ffi/campaign.sh`) on the
   campaign machine.

## Log index

| Log | What it establishes |
|---|---|
| `wp7-gate.log` | the clean-checkout gate of WP7 at `c35bd22`, both builds, net8.0 and net6.0, counts included (see Gate) |
| `wp7-grpcnet-context-reflection.log` | what Grpc.Net.Client 2.71.0's internal serialization context calls (req 11's transport form) |
| `campaign/wp7-smoke/` | the WP7 smoke runs of every suite (see Smoke) |
| `wp6h-gate.log` | the clean-checkout gate after the register H fixes, at `b758b27` |
| `wp6s1-gate.log` | the clean-checkout gate of WP6 step 1, at `ee94026` |
| `wp5s10-gate.log` | the step-10 gate (no-unknown variant added as step 8), at `2410125` |
| `wp5s9-gate.log` | the step-9 gate (decision 11 port), at `8d2e7ac` |
| `gate-repro/` | the gate at 253f487 and ef00211, three runs each, each log under its own name: 6 of 6 passed (the once-seen failure at 253f487 did not reproduce; cause not found, JOURNAL 54) |
| `campaign/` | the smoke runs of `run_campaign.sh` and the runner's gate logs |
| `bdn-default-job-unit/` | a trial of one BDN unit at the default job, for the JIT check and the engine cost (instrumentation) |
| `wp5-tail-gate.log`, `wp5-step4-gate.log`, `wp5-step4-before-after.log`, `wp4-rd9-bytes-free.log`, `wp3-17-net6-floor-build.log`, `stage21-wp4-regate.log` | earlier gates and checks, superseded by the gates above |
| `stage1` ... `stage20`, `bdn-results/`, `calibration-rust-crossing.log` | pre-campaign stages; every timing in them is instrumentation and none is used. **Stages 18 and 19 were built against a core not on the branch (FIX-PLAN R-C9)**: relabelled in their headers, no figure from them is usable |
