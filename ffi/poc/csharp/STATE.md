# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists in the tree
and what was checked. It carries no recommendation and no verdict (the decision is the owner's).
Every figure in this slice is container instrumentation (README 1.1), never a result; timing
waits for the campaign. The history of how each item got here is in `JOURNAL.md` (entries 1 to
65); this file states what is true now.

| | |
|---|---|
| **Status** | FIX-PLAN WP13 done (TCP 127.0.0.1 with TCP_NODELAY read back, perf task-clock beside the process clock, softirq on the CLIENT CPUs, pools at AK_WORKERS, both h2 variants gated and labelled, D9 stated: see **WP13**). Before it: WP10 done (every RPC cell against the Rust slice's rpc_server; this slice's server removed), then req 22a as amended (e6c909630): BDN's default toolchain (one child process per case) for the campaign, InProcessEmit grouping a small-run switch. Gate and smoke: see **Gate** and **Smoke**. Findings are in scope only if they can change what the campaign measures (ffi/CLAUDE.md, "Scope of findings"). |
| **Levels** (FIX-PLAN D2) | target **net8.0** (.NET 8.0.31, SDK 8.0.131); floor **net6.0** (.NET 6.0.36 from the NuGet runtime pack, self-contained publish): gated; floor **.NET Framework 4.8**: compiled only (`src/HarnessFloor`), never run (needs Windows; the container has no Mono) |
| **Incumbent** | Google.Protobuf 3.32.0, Grpc.Tools 2.72.0, Grpc.Net.Client and Grpc.AspNetCore 2.71.0 (the versions `packages/csharp` ships) |
| **Core** | the one core, `ffi/poc/codec`, built from `git archive HEAD` by `gen/build_core.sh`, every build with `init-guard`: full `target-core` (`rpc`), `target-core-count` (`rpc,count`), `target-core-corpus` (`corpus`); no-unknown (ak-core `--no-default-features`) `target-core-nounk`, `target-core-count-nounk`, `target-core-corpus-nounk`, each in its own target dir; the same four transport cores against h2-batch (`poc/codec/h2-batch/`, D11 as amended) as `target-core[-count][-nounk]-h2b`; the h2 compiled into each is printed by build_core.sh |
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
                    NativeMemory.Realloc (geometric, clamped to INT32_MAX), ONE reset per decode
                    (before it), bags taken into UnknownFields, UNDELIVERED check, per-position
                    discard helpers, UnkHost.Exact (an exact-size grow, a gate control only);
                    no-unknown: none of these
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
gen/rpc-counts.txt          the same per call of every RPC cell A-F, framed twin, Cc copy cell,
                            direction and upload payload (105 rows)
gen/rpc-counts-nounk.txt    the same, no-unknown client (62 rows)
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
src/Rpc/                    akrpc (a client only, no server since WP10): `bench` (the timed RPC grid on
                            BenchmarkDotNet, RpcBench.cs), `campaign --suite calib`, `--suite rpc
                            --counts` and `--upload-check` (the counting build, the gate); cells A-F
                            against the campaign server's armonik.ffi.campaign.v1.Grid; the gate's
                            --layout and --error-path --sock; --shared-ctx
src/BenchDotNet/            the codec suite's engine: BenchmarkDotNet 0.15.8, InProcessEmit, one
                            process per arm:mode unit, process CPU per iteration (CpuClock), JIT tier
                            read back (JitTiers.cs), encode variants and pools (Cases.cs), the Grpc.Net
                            frame (GrpcFrame.cs), `--counts` (CountRun.cs, the counting build)
src/HarnessFloor/           net48, compile only (the binding; the host half is compiled out)
run_campaign.sh             --suite codec|rpc|calib|gate --out DIR (CAMPAIGN req 31)
```

## Gate (WP13): clean checkout, one per h2 variant

`logs/csharp/wp13-gate-stock.log` and `logs/csharp/wp13-gate-h2-batch.log`: **GATE PASSED** for
each, at commit `ea02da5`, run by `run_campaign.sh --suite rpc --smoke` from a fresh worktree
(one gate per variant, in the same tree, as the campaign runs them); net8.0 and net6.0, both
builds; 0 step failures; 30 planted controls failing as required in each (WP10's 29 plus the
Nagle-on control). The core is `ffi/poc/codec` at `59a96f8` (p1, the spare ring); the gate's
RPC steps run over TCP against the shared server's listener (`AK_SERVER_TCP=0`), the upload
check reading TCP_NODELAY back on 33 of 33 (full) and 17 of 17 (no-unknown) sockets; in the
h2-batch gate every transport core placed is a `-h2b` build (the gate log names the h2 in
each). Earlier the same day at `2f9ce48`: stock passed, h2-batch FAILED on a second gate in
one tree (`wp13-gate-h2-batch-FAILED-2f9ce48.log`: the net48 floor build compiled the counting
builds' generated Shapes.cs; fixed in `ea02da5`, JOURNAL 65). Before WP13: `wp10b-gate.log`.

## WP13 (FIX-PLAN, 2026-10-03)

- **TCP (D10, req 17 as amended).** Every timed cell runs over TCP 127.0.0.1 against the shared
  server's TCP listener (`AK_SERVER_TCP=0`; serve.sh prints `tcp 127.0.0.1:PORT`), in the runner
  and in the gate's RPC steps (`--sock tcp:127.0.0.1:PORT`). Nagle off: Grpc.Net `Socket.NoDelay`
  in the connect callback; the core `ak_client_opts.tcp_nagle = 0`. Read back in each case's
  setup after one untimed call (`src/Rpc/NoDelay.cs`: every ESTABLISHED socket of the process
  to the port, getsockopt TCP_NODELAY; none found or one without it fails the case) and in the
  upload check. The listener runs the PINNED server configuration only, so `shipped` and
  `pinned` differ on the client side only (stated in the runner and unit headers). Control
  `AK_CAMPAIGN_PLANT=nagle`: in the gate (upload check) and in `--plant` (A and B).
- **Client CPU (req 21 as amended).** `cpu_ns` = perf task-clock of the whole process (one
  perf_event_open SOFTWARE/TASK_CLOCK counter per thread, new threads picked up at each read),
  `proc_cpu_ns` = CLOCK_PROCESS_CPUTIME_ID beside it, both per BDN iteration at the same
  boundaries; `client_softirq_ticks` / `client_irq_ticks` from /proc/stat over AK_CPU_CLIENT
  per case (round-1 row). Both toolchains.
- **Pools (D8, D14).** AK_WORKERS (campaign.machine, default 8): the core runtime
  (`ak_runtime_new`), the .NET thread pool worker min and max, the server's tokio workers
  (AK_SERVER_THREADS defaults to it). Caller threads = the in-flight level k (not a worker
  pool); grpc-core not used. In every header.
- **h2 (D11 as amended).** Stock and h2-batch cores built and gated (`AK_H2=stock|h2-batch
  gen/gate.sh`); the runner loops the rpc suite over `AK_H2_VARIANTS` (default both), one gate
  each (`gate.log`, `gate.h2-batch.log`); every akrpc process reads the loaded core's h2, puts
  `h2` on every row and aborts if it differs from AK_H2. Codec rows carry `h2: stock`.
- **D9 / req 25 as amended (ad1a15be5).** The rule covers this slice: the shared core's buffers
  and transport allocate through glibc malloc inside the .NET process (no shim of its own;
  managed objects are on the GC heap). Runner switch `AK_CAMPAIGN_ALLOC`: `default` (the main figures,
  GLIBC_TUNABLES unset, as production) or `pinned` (the labelled diagnostic pass,
  GLIBC_TUNABLES with trim_threshold 256 MiB and mmap_threshold 32 MiB; files suffixed
  `.alloc-pinned`), both suites. Every row carries `alloc` and `minflt` (the process's minor
  page faults over the iteration, read by the job's clock beside the CPU clock, both
  toolchains; per call = minflt / iters). Each process reads back the mode at start (one
  16 MiB malloc and mallinfo2's mmapped-block count: default "mmapped", pinned "heap"), states
  it in the header, and refuses to run if AK_CAMPAIGN_ALLOC disagrees with its GLIBC_TUNABLES or if the
  pinned readback is not "heap". The RPC server runs the default allocator in both passes.
- **Probe, no pre-grow (owner decisions 2026-10-03).** The probe runs once per process (the
  host's Main and the first GlobalSetup of each BDN child; later setups return) and keeps its
  16 MiB block mapped (never freed: freeing it raised glibc's dynamic mmap threshold, java
  2892e207b); every case's first row carries `alloc_probe` (the child's own under the default
  toolchain). There is NO heap pre-grow: it was built and then reverted by the owner, because
  a pre-grow on one thread cannot reach the other threads' malloc arenas (C++ found the first
  benchmark still faulting through the core's worker threads); BDN's warm-up runs the real
  call path on every thread, and `minflt` on every row shows whether it sufficed.
- **Runner settings for small tests:** `AK_RPC_TRANSPORTS`, `AK_RPC_BUILDS`, `AK_H2_VARIANTS`; the allocator pass `AK_CAMPAIGN_ALLOC`.

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
| D42 | (closed, WP10) | the pre-campaign timing modes of `akrpc` (in-process server) are removed |
| D45 | `src/Rpc/RpcBench.cs` unit header | the per-unit `# build ...` line still says "the server's pinned socket" and "Unix socket tcp:127.0.0.1:PORT (req 17: UDS)" over TCP; the `# network` line beside it is right. Wording only, out of scope (JOURNAL 65), not fixed |

## Campaign readiness (design/CAMPAIGN.md at 3210f28; section 10 checklist)

Assessed against CAMPAIGN.md as amended through 3210f28 (the owner's 2026-09-26 decisions,
R-H22 to R-H36). The runner is `poc/csharp/run_campaign.sh`. The codec suite runs under
BenchmarkDotNet (req 22a), and so does the RPC grid since WP9 (req 22a as amended 2026-09-27):
`akrpc bench`, one pinned process per cell, the launch's one server started and warmed by the
runner first. **The server is the Rust slice's tonic rpc_server** (FIX-PLAN WP10, CAMPAIGN req
13 as amended; poc/rust/SERVER.md): one process per launch through poc/rust/serve.sh, pinned
to AK_CPU_SERVER, its tokio worker count in its log; `shipped` and `pinned` are this client's
configuration against its two sockets (tonic's server defaults; 4 MiB windows, adaptive off). The calib suite is `akrpc campaign --suite calib`. Each suite runs both builds
(full and no-unknown) per launch, in an order alternated by launch.

**Toolchain (CAMPAIGN req 22a as amended e6c909630).** The campaign runs BDN's default
toolchain: a generated project and ONE CHILD PROCESS PER CASE, in both suites. Grouping every
case of a unit in one process (InProcessEmit) is the switch `AK_BDN_GROUPED=1`, on by default
under `--smoke` and for small exploration runs only; the header says which. Under the default
toolchain the process CPU per iteration comes from the child's own clock reads (written at its
GlobalCleanup, paired in the host and checked against BDN's wall measurement of each
iteration); the JIT tier is **not** read back in that mode (the listener sees the host only;
open, JOURNAL 64), so the codec suite's JIT check applies to grouped runs only.

**What the framework forces or what differs from the hand-written sampler (WP9), stated:**
- a runner call per unit: one unit = one cell (21 in the full build, 10 no-unknown, per transport),
  its cases = its directions, payloads and in-flight levels; three benchmark classes (RpcK1,
  RpcK8, RpcK16) because OperationsPerInvoke is an attribute constant; one invocation = one
  batch of k calls in flight, k operations (`iters` = calls, `invocations` = batches);
- the channels live for the process (opened before BDN starts, one channel per cell per
  benchmark process), no longer for the whole launch across cells;
- BDN picks the invocations per iteration (pilot, iteration time 100 ms campaign, 20 ms smoke;
  unroll factor 1) where the sampler had a fixed number of calls per sample;
- the warm-up is BDN's (jitting stage, pilot, 10 warm-up iterations campaign / 1 smoke), not a
  JIT-settled loop; the JIT tier is read back per case and reported (`hot_tier0`), not fatal
  for RPC cases; order: units a seeded shuffle per launch, cases a seeded shuffle per process.

**Custom code on top of BDN, each for a requirement** (WP9 addendum, bc7cf94b1): starting and
warming the shared server through serve.sh (req 13); CpuClock, the job's clock reading process CPU at each
iteration boundary (req 21: BDN has no CPU per iteration); the seeded IOrderer (req 22: BDN has
no random order); the JSON-lines exporters with every label, writing no sample when any case
failed (req 28, 18); the runner's discard of a failed launch's output (req 18/22a); the JIT tier
read back (req 24's "recorded"); in the codec suite, the two prime cases (BDN cases, not
exported): without them the first case of every process measured BDN's own first-touched
runtime helpers at tier 0 and the JIT check failed (JOURNAL 51, 62). The codec suite's
hand-written pre-warm loop, its settle wait and knobs are removed (JOURNAL 62).

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | not applicable in the container: the machine is the owner's; the runner runs one measured process at a time |
| 2 | governor, turbo, SMT | not applicable in the container: set by the owner; recorded in every header (sysfs) |
| 3 | isolation | not applicable in the container: set by the owner; isolcpus/nohz_full and the cgroup cpuset recorded |
| 4 | three disjoint CPU sets, fixed sizes, thread counts | met (D8, D14: every pool at AK_WORKERS, see WP13): `AK_CPU_CLIENT` / `AK_CPU_SERVER` from the environment (ffi/campaign.sh exports ffi/campaign.machine's) or, run alone outside a smoke, read from ffi/campaign.machine; a set whose size is not `AK_SET_SIZE` is refused; `taskset` per process; the header names the source. Worker thread counts in every header: .NET thread pool min/max and current, caller threads, the core runtime's workers (the codec suite creates none), the server's |
| 5 | floors gated for correctness | net6.0: met (the gate, both builds). .NET Framework 4.8: **not met**: compiled only, needs a Windows machine (owner decision: provide one, or record that the floor was not run) |
| 6 | build flags printed | met: Release, net8.0, core features (init-guard on), shared `libak_core.so`, build variant, JIT and GC settings, in every header; the core's cargo profile (`lto = false`) is stated here, not printed |
| 7 | payloads | met as amended: 16 payloads; Latin-1 and wide on P1.2, P2.2 and P2.4 (R-H26); the 92 accepted, non-disputed U-* rows at the shapes core's 7 ABI roots through the timed shapes core in encode (`encode-hot`: the arm's own decode of the row, untimed, re-encoded), decode and decode-read, every arm including incumbent-best (R-H27); decode-reencode a labelled extra |
| 8 | arms | met: incumbent-prod (Grpc.Tools' marshaller shape: CalculateSize + WriteTo(IBufferWriter), ParseFrom(ReadOnlySequence)), incumbent-best (WriteTo(IBufferWriter) / ParseFrom(ReadOnlySpan)), core-ffi (push; pull as `core-ffi-pull`), host-gen |
| 9 | directions | met: encode (4 variants, req 11), decode, decode-read (a generated visitor reads every field) |
| 10 | unknown fields, three modes | met: core-ffi and host-gen in retain and drop (full build; `Codec` and `CodecRetain`) and no-unknown (its own build and core, no facade member, R-H22); core-ffi-pull in drop and no-unknown; incumbent default (retains); `unknown_mode` and `build` on every sample |
| 11 | serialised once per iteration; encode variants | met as amended (R-H29): `encode` (pool + reused buffer), `encode-hot` (one graph + reused buffer), `encode-transport` (pool + the Grpc.Net form), `encode-transport-hot`; rows carry `enc_end`, `enc_input`, `pool_graphs`, `pool_bytes`. Reused buffer: the incumbent's BufWriter, host-gen's Enc, the core's encode buffer. Transport form: the serializer cells A, F, D run (shared code) into a frame built as Grpc.Net.Client 2.71's GrpcCallSerializationContext builds it (checked by reflection); on the core's transport (C, E) the form is the buffer row, stated; incumbent-best has no transport row. Pool: retained heap >= 2 x AK_LLC_BYTES (13.75 MB default), measured and topped up; a hot input is a pool of one (same per-call step); graph construction always in the case's setup. Google.Protobuf keeps no size memo |
| 12 | cells A-F, modes | met as amended (R-H35): full client A, B, C-retain, C-drop, D-retain, D-drop, E-retain, E-drop, F-retain, F-drop (+ labelled B/C callback and queue rows); no-unknown client A, B, C-nounk, D-nounk, E-nounk, F-nounk |
| 13 | server: separate, pre-serialised, one per launch, warmed; one channel per cell | met as amended (WP10, WP13): the Rust slice's tonic rpc_server through poc/rust/serve.sh, one process per launch pinned to AK_CPU_SERVER, AK_SERVER_THREADS = AK_WORKERS, its TCP listener serving every h2 variant, build and client configuration; warmed by serve.sh warm (2,000 campaign / 100 smoke checked calls per direction, on its sockets and its TCP listener) before any client; one channel per cell per benchmark process; P2.2 pre-serialised |
| 14 | directions a, a+read, b, c, d | met as amended (R-H36; 2026-09-27): a and a+read, b; c = unary upload of P5.3 and P5.4; d = the streamed upload (M5 messages of 2 MiB, ids on the first only, 4 MiB and 16 MiB, the server checking the count on every call, count and SHA-256 once per cell before timing), every cell and mode, at 1 and 8 in flight; B/C/E through the core's client streaming (ak_call_open, ak_call_send / ak_call_send_enc, ak_call_recv), A/D/F through Grpc.Net's AsyncClientStreamingCall; the core's framed send path beside its reference as Bf, Cf-*, Ef-* on b, c and d (a has an empty request). C runs the MOVE path (ak_call_unary_enc on b and c, ak_call_send_enc on d), its copy path kept as the labelled extra Cc-*; D copies (Grpc.Net's serializer can only write into the call's own buffer, so ak_enc_take_owned would not remove the copy; stated); ak_call_opts not used |
| 15 | 1/8/16 in flight | met |
| 16 | delivery | met as amended (R-H30): B, C, E the core's blocking call on caller threads created before the warm-up; A, D, F Grpc.Net's idiomatic `await CallInvoker.AsyncUnaryCall` (as Grpc.Tools' generated client does), k in flight = k async loops on the thread pool, stated in the header; callback and queue rows labelled extras, awaited |
| 17 | shipped and pinned, TCP 127.0.0.1 | met as amended (D10, WP13): TCP 127.0.0.1 against the server's TCP listener, which runs the pinned server configuration only; shipped and pinned are the client's configuration (shipped: Grpc.Net DisableDynamicWindowSizing and no window, the core's windows at 0; pinned: 4 MiB windows on both client transports); Nagle off on every client socket, read back per case |
| 18 | every call checked, abort | met: status (a non-OK gRPC status is AK_ERR_RPC_STATUS on the core's transport, an RpcException on Grpc.Net) and length or count on every call, the server warm-up's included; a retained decode that leaves a buffer undelivered fails its call; the `--plant` controls, per build and transport, a wrong expected length on a, c and d and a wrong SHA-256 on d, each on A, B, Bf and D one cell at a time, all abort with no sample |
| 19 | crossing counts gate | met as amended (R-H31, 2026-09-26 geometric grow): every exported entry point the timed code calls, counted by name in a counting build, resets included and placed (one per decode, before it: decision 11 rule 7 as amended), retain with no pre-placed buffer and the timed build's geometric grow; per codec case (`gen/counts*.txt`) and per call of the RPC cells (`gen/rpc-counts*.txt`, B to E and the framed twins, A and F listed, directions a to d); gated in step 9 with a must-differ control (exact-size grow). The R5 counts (`gen/crossings*.txt`) are gated too and checked before calib |
| 20 | crossing cost fwd/rev, perf stat | **not met here**: calib has a forward row (ak_noop) and a forward-and-reverse row; `perf stat` runs when installed and is not installed in this container (owner: install perf on the campaign machine) |
| 21 | CPU is process CPU per round | met as amended (R-H25): codec, CLOCK_PROCESS_CPUTIME_ID per BDN iteration (the job's clock, read at the same iteration boundaries as the wall time; a case without one value per iteration fails); rpc (as amended, WP13), perf task-clock of the whole client process per BDN iteration (`cpu_ns`), CLOCK_PROCESS_CPUTIME_ID beside it (`proc_cpu_ns`), softirq and irq ticks on the CLIENT CPUs per case; calib (the crossing benchmark, req 20) keeps CLOCK_THREAD_CPUTIME_ID of its one loop thread |
| 22 | order randomised where the framework allows | met: codec, the unit order of a launch and the case order in each BDN process are seeded shuffles, seeds in the headers; builds alternate by launch; rpc, the cell order of every round a seeded shuffle; transports and builds alternated by launch |
| 22a | benchmark engine | met as amended 2026-09-27: BenchmarkDotNet for the codec suite and the RPC grid (InProcessEmit, pinned by the runner, StopOnFirstError for RPC, raw measurements exported, warm-up and tier recorded; what the framework forces and the custom pieces are listed above) |
| 23 | 5 rounds x 3 launches | met (defaults) |
| 24 | warm-up stated, identical; GC/JIT defaults stated; every warm-up a runner parameter | met (amended 85cfd4826): every warm-up is a runner parameter with the campaign default in the header and a short smoke default (run_campaign.sh's AK_RPC_WARM_*, AK_RPC_SERVER_WARM, AK_BDN_WARMUP / _ROUNDS / _ITERATION_MS / _PREWARM_*; JOURNAL 61). codec, per BDN process a pre-warm to JIT quiescence (the job's clock included) and 2 unexported prime cases, then per case BDN's jitting, pilot and a fixed warm-up count; the JIT tier read back per case, `jit check: FAIL` fails the unit. rpc: warm-up rounds of 64 calls per cell, direction and level until a round compiles nothing (at most 10); `jit_in_window` per sample. GC and JIT between blocks at the framework defaults, stated Req 24 as amended 8c02e7c58 (>= 20 calls per calling thread before the first measured value): met for B, C, E and their twins at the campaign default, unchanged (10 warm-up iterations x BDN's minimum 4 invocations x 1 call per caller thread per invocation = >= 40, plus 1 jitting and >= 4 pilot invocations; the caller threads are created in the case's process before BDN's first stage; checked in BDN child mode on d/16 MiB at k = 8: 45 per thread, `logs/csharp/wp13-req24-warmup-count.log`), stated per unit in the `# warm-up/thread` header line; A, D, F and the callback/queue extras: >= 40 calls per async loop on the fixed thread pool, the per-thread count not controlled (stated) |
| 25 | allocator/GC warm, GC stated | met as amended 2026-10-03 (D9): warm-up per arm, workstation concurrent GC stated, GC counts and pause per BDN case (summary row); main figures on glibc's default allocator, `AK_CAMPAIGN_ALLOC=pinned` the labelled GLIBC_TUNABLES pass, `alloc` and `minflt` on every row, the mode read back per process (see WP13) |
| 26 | correctness before timing | met: the runner requires the gate passed at identical content (both builds, counts included); every BDN process re-checks byte identity of every encode arm and variant (transport frames and pooled graphs included) and every U-* row's encode and re-encode forms before timing |
| 27 | header | met: commit (dirty tree refused), machine, CPU sets and their source, runtime and incumbent versions, build flags and variant, core features, transport, threads, warm-up and repeats |
| 28 | JSON lines, raw | met: one line per BDN iteration and per rpc/calib sample, section 7's fields plus `build` and the encode-variant fields; the per-case BDN summary row carries `row: case-summary` and no `cpu_ns`/`wall_ns` |
| 29 | logs in ffi/logs/csharp/campaign/ | met |
| 30 | summaries | not applicable: this slice produces none; any ratio is to come from per-launch medians (stated in every header) |
| 31 | runner interface | met for the slice; the top-level `ffi/campaign.sh` is the aggregating session's |
| 32 | smoke run | see **Smoke** below |

**Smoke** (the owner's small-test rule; figures stripped; container instrumentation):
- `logs/csharp/campaign/wp13-smoke/` at `ea02da5`, through the runner from the gate's worktree,
  grouped switch on (the smoke default): TCP 127.0.0.1, client configuration `pinned` only,
  full build only (AK_RPC_TRANSPORTS=pinned, AK_RPC_BUILDS=full), both h2 variants, one launch,
  BDN 1 round, 1 warm-up, 20 ms iterations; the Rust server started with AK_SERVER_TCP=0 and 8
  workers and warmed (100) by serve.sh: per variant all 21 units, 283 samples, 0 failed cases;
  every row `net: tcp`, `h2` equal to the variant, with `cpu_ns` and `proc_cpu_ns`, and
  `client_softirq_ticks`. `plant-controls.log` (stock, pinned, full): 21 controls (WP10's 19 and
  Nagle on, on A and B), every one aborting with 0 samples. The raw .jsonl and .bdn.log are not
  committed (logs/PURGED.md); `*.stripped.log` keep every header line and per-cell counts.
- Not run in the smoke: `shipped`, the no-unknown RPC client, calib, the codec suite, the
  default toolchain (checked on one RPC unit, Bf, with task-clock: 10 samples, JOURNAL 65).
- `logs/csharp/campaign/wp10b-smoke/` at `d1a3a3b`, through the runner, with the grouped switch
  on (the smoke default): full build, `shipped` only (AK_RPC_TRANSPORTS=shipped,
  AK_RPC_BUILDS=full), one launch, BDN 1 round, 1 warm-up, 2 ms iterations; the Rust server
  started and warmed (100) by serve.sh: all 21 units, 283 samples, 0 failed cases; `plant/`: 19
  controls (wrong length on a, c, d and wrong SHA-256 on d, on A, B, Bf, C-drop, D-drop), every
  one aborting with 0 samples.
- `logs/csharp/campaign/wp10-smoke/` at `76ac71e` (WP10 before the switch): the same, the same
  result.
- The default toolchain (one child process per case) was checked on one RPC unit and one codec
  unit (JOURNAL 64), not in these smokes. Not run: the pinned transport, the no-unknown RPC
  client, calib, the full codec suite.

**Engine cost, container instrumentation** (`logs/csharp/bdn-default-job-unit/`, before WP7):
one unit of 336 cases at the default BDN job (10 warm-up, 5 x 100 ms) ran 17 to 18 minutes.
WP7 roughly triples the encode cases (four variants, U-* encode) and adds pool setups; the
campaign's codec suite is correspondingly longer.

## Campaign duration estimate (2026-10-03, computed, nothing run)

Campaign defaults, BDN's per-case child processes, both allocator passes. Per-case times are
container figures used only for sizing (JOURNAL 70): RPC from `logs/csharp/wp13-req24-warmup-count.log`'s
run (7 cases, 24.9 s run time, 16.1 s of iterations: about 1.25 s per case for child start
and setup; BDN's project build 26.7 s per unit process); codec from `bdn-default-job-unit/`
(336 cases in 17 to 18 min in-process at 10 warm-up and 5 x 100 ms, about 3.1 s per case) plus
the same 1.25 s child overhead (not measured for the codec suite).

| Item | Count per pass | Per item | Per allocator pass |
|---|---|---|---|
| codec, full build, per launch | 7 units, 2,908 cases (incumbent-prod 500, incumbent-best 364, host-gen and core-ffi retain/drop 500 each, core-ffi-pull 44) + 2 primes per unit | ~4.3 s per case (3.1 + 1.25), 27 s build per unit | 3.5 h per launch |
| codec, no-unknown build, per launch | 5 units, 1,908 cases | same | 2.3 h per launch |
| codec suite, 3 launches (net8.0 only; stock h2; no transport) | 14,448 cases | | ~17.4 h (14.5 to 19 h for 3.6 to 4.6 s per case) |
| RPC, one launch x transport x h2: full client | 21 units, 283 cases (10 cells x 17, 7 x 11, 4 x 9) | fast case ~2.8 s; d/16 MiB at k = 8 ~11 s (BDN's 4-invocation floor: 4 batches of 8 x 16 MiB per iteration, ~0.58 s each, 16 iterations); 27 s build per unit | ~27 min |
| RPC, same: no-unknown client | 10 units, 146 cases (6 x 17, 4 x 11) | same | ~14 min |
| RPC grid: 3 launches x 2 transports x 2 h2 variants | 5,148 cases, 372 unit processes | | ~8.1 h (6.5 to 10 h), + server start and 2,000-call warm per launch ~0.1 h |
| calib | 3 launches | a few minutes each (not timed here) | ~0.25 h |
| **one allocator pass** | | | **~26 h (21 to 30 h)** |
| gates (stock and h2-batch, once; reused by the second pass if the content is identical) | 2 | ~21.5 min each (wp13 gates) | ~0.75 h once |
| **both passes (default + pinned diagnostic)** | | | **~52 h (43 to 60 h)** |

The three largest contributors, per pass: (1) the codec suite's U-* rows: 92 rows x 3 or 4
directions x every arm and both builds, 3,496 of the 4,816 codec cases per launch, ~12.6 h;
(2) the per-case child process (start, setup, BDN's own stages outside the timed iterations),
~1.25 s x 19,596 cases, ~6.8 h, plus BDN's project build per unit process (~27 s x 408 units,
~3 h); (3) the RPC grid's multiplicity (3 launches x 2 transports x 2 h2 variants = 12 copies of
429 cases), ~8.1 h, of which the c and d cases at k = 8 (BDN's 4-invocation floor at 2 to 16 MiB
per call) are about a quarter. The pinned allocator pass doubles all of it.

## What is not measured or not established

- **No timing in this slice is a result.** Every figure is container instrumentation.
- Anything on .NET Framework 4.8 (compiled only); no floor runs the RPC suite or BDN.
- `perf stat` cycles and instructions (not installed here), so req 20's per-iteration counts.
- The JIT tier under the default toolchain (open since JOURNAL 64).
- The runner end to end with `AK_CAMPAIGN_ALLOC=pinned` (the switch was smoked on units directly, `wp13-d9-alloc-smoke/`); the server's allocator is not switched.
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
- Server-streaming and bidirectional calls (reserved in the ABI, not built); `ak_call_opts`
  (deadline, metadata) is passed NULL; the callback/queue deliveries of a stream (not built).
- The framed send path on direction a (an empty request); D without a copy (not possible
  through Grpc.Net's serializer).
- `packages/csharp`'s own object model; ReadyToRun; GC under load; content sets beyond P1.2,
  P2.2 and P2.4.

## Next step

1. The aggregating session reads WP13 (JOURNAL 65) and pushes; this slice changes nothing further
   unless a finding in scope (ffi/CLAUDE.md, "Scope of findings") comes back.
2. The net48 gate on a Windows machine (D4), which first needs a net48 host half.
3. The campaign itself is the owner's: `run_campaign.sh` (through `ffi/campaign.sh`) on the
   campaign machine; it runs one gate per h2 variant and the rpc suite once per variant.

## Log index

| Log | What it establishes |
|---|---|
| `wp13-req24-warmup-count.log` | BDN's stage counts per case at the campaign warm-up, child mode, C-drop at k = 8 (req 24 as amended) |
| `wp13-alloc-probe-smoke/` | after the pre-grow's revert: C-drop per mode (grouped), Bf under the default toolchain (pinned, each child its own probe), a codec unit (default) |
| `wp13-pregrow-smoke/` | the glibc pre-grow, since reverted (JOURNAL 67, 68) |
| `wp13-d9-alloc-smoke/` | the allocator switch (D9 as amended): an RPC and a codec unit in both modes, the readback, a mismatch control, minflt under the default toolchain |
| `wp13-gate-stock.log`, `wp13-gate-h2-batch.log` | the clean-checkout gates of WP13 at `ea02da5`, one per h2 variant (see Gate) |
| `wp13-gate-h2-batch-FAILED-2f9ce48.log` | the second gate in one tree failing on the floor build before the fix (JOURNAL 65) |
| `campaign/wp13-smoke/` | the WP13 minimal smoke: TCP, pinned, full build, both h2 variants, stripped; the plant controls with Nagle on |
| `campaign/wp8c-warmup-knobs/` | the RPC client's warm-up knobs in smoke mode, one transport, full build (req 24 as amended) |
| `wp10b-gate.log`, `wp10-gate.log` | the clean-checkout gates after the toolchain switch (`d1a3a3b`) and WP10 (`76ac71e`) |
| `campaign/wp10b-smoke/`, `campaign/wp10-smoke/` | the minimal smokes against the Rust server, grouped, one transport, full build, with plant controls |
| `wp9-gate.log` | the clean-checkout gate of WP9 at `6a7c0cf`, both builds, net8.0 and net6.0 (see Gate) |
| `campaign/wp9-smoke/` | the minimal WP9 smoke: the RPC grid on BDN, one transport, full build; one codec unit (see Smoke) |
| `wp8b-gate.log` | the clean-checkout gate after the WP8 parity items, at `9114d6b`, both builds, net8.0 and net6.0 (see Gate) |
| `campaign/wp8b-smoke/` | the smoke after the WP8 parity items (see Smoke) |
| `wp8-gate.log` | the clean-checkout gate of WP8 at `d97ea52`, both builds, net8.0 and net6.0, counts and upload check included (see Gate) |
| `campaign/wp8-smoke/` | the WP8 smoke runs of every suite and the plant controls (see Smoke) |
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
