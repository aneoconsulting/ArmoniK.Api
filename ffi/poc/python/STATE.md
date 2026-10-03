# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists and what
was checked. It does not say what a binding should choose (`CLAUDE.md`, roles). History is in
`JOURNAL.md`; this file says what is true at the commit it was written at.

**Phase** (README 1.1): setup and design. Every timing this slice has produced is container
**instrumentation**. None is quoted here. The logs that carry timings are listed at the foot,
labelled.

| | |
|---|---|
| **Status** | FIX-PLAN WP13 done (on WP9 and WP10): every timed RPC cell over TCP 127.0.0.1 with TCP_NODELAY read back, perf task-clock as the RPC CPU figure, every pool at AK_WORKERS (8; grpc-core through a sysconf shim), both h2 variants of the rpc core built, gated and labelled, rebuilt on the current core (p1). The rebuild found a shim defect in the first encode on a new context (fixed, 8b87eea10). Gated once from a clean worktree at 86c141a1c (both builds, 3.12 and 3.7, both h2 variants: `gate exit 0`, `GATE PASSED`), then a minimal smoke. D9 as amended applies: default allocator for the main figures, a pinned diagnostic pass behind a switch |
| **Target** (owner D1) | CPython 3.12.3; grpcio 1.84.0, protobuf 7.36.2 (upb) |
| **Floor** (owner D1) | CPython 3.7.5 (Ubuntu 18.04's packages, `fetch_py37.sh`, sha256-pinned); protobuf 4.24.4 (upb) as the incumbent there. The floor runs the correctness gate only |
| **Incumbent** (R14) | protobuf on upb through gRPC's generated marshaller path (`SerializeToString` / `FromString`); derived from `Protos/V1` by `verify_r14.py` (log 52) |
| **Core** | `poc/codec`, the one core (R0), read by path from this checkout. Eight builds, every one with `init-guard`. Full: plain, `count`, `rpc`, `corpus`. No-unknown (`--no-default-features`): the same four, each in its own target directory |

## What exists

```
poc/codec/gen/py_pure.py   (this slice's backend) facade.py, pycodec.py (drop), pycodec_retain.py
                           (error codes read from plan.FIXED.codes);
                           from a plan with unknown fields compiled out, a facade with no
                           `_unknown` (no attribute, __slots__ entry or constructor argument)
poc/codec/gen/py_capi.py   (this slice's backend) binding.c: the CPython shim; C facade types;
                           three accessor backends (attr, cext, pyacc); both directions;
                           decision 11 (per-root options armed per decode, host grow, slots
                           delivered into `_unknown`, the ak_uencode_* family); per-thread
                           decode and encode contexts; ak_init (plan.lifecycle); the section 10
                           layout table; PY_VERSION_HEX conditionals. From the no-unknown plan:
                           none of the unknown-field code, no `_unknown` member, retain refused
poc/codec/gen/c_abi.py     (shared, used as is) ak_abi.h for both variants
gen/generate.py            glue: renders gen/out/{., corpus, nounk, corpus-nounk}; --check runs
                           the one-generator guard with its planted violation
gen/out/                   committed generated files (18)
native/binding.c           the module wrapper (names no message or field): entry points, layout
                           check at import, counters, controls (unk_positions, wrong_root,
                           last_reclaimed, unk_totals, tls_created, nounk); -DAK_NOUNK selects
                           the variant and #errors on a shim/header mismatch
build.sh                   R0, R1, eight cores, per interpreter 10 measured shims + controls, R5,
                           the must-fail controls (noinit, layout plant, no AK_RPC, variant shim
                           over the full core, the process-wide reclaim twin)
gate.sh                    the correctness gate at the target and the floor -> logs 90-98, 100-104
test_camp_summary.py       camp_summary's test: two builds, per-launch medians (gate 104)
rpc_counts.py              req 19: every ABI call per call of the RPC cells B-E (gate 105)
counts/abi-{full,nounk}.txt, counts/rpc-{full,nounk}.txt   req 19's committed counts
counts/crossings-{drop,nounk}.txt   committed whole-number crossing counts per call (req 19)
fetch_py37.sh, floor_check.sh       the floor interpreter; the source check against 3.7.5 headers
conformance.py, corpus.py, rpc_gate.py, rd1_lenwrap.py, u1_map_unknown.py   gate steps
facts.py, payload_values.py, arms.py, arms_plan.py   harness glue (no wire rule)
run_campaign.sh, camp_*.py, counts_expected.txt      the campaign harness (CAMPAIGN.md):
                           camp_pyperf.py + camp_pyperf_export.py (codec suite on pyperf; cases
                           from camp_codec.py, now a case library with no loop of its own),
                           camp_rpc_pyperf.py + camp_rpc_pyperf_export.py (RPC grid on pyperf;
                           cells from camp_rpc.py, now a library with no sampler),
                           camp_rpc_srv.py (the shared Rust server's start / warm / stop through
                           poc/rust/serve.sh), camp_calib.py, camp_summary.py, camp_lib.py
proto/campaign_grid.proto  cell A's typed stub for the shared server (SERVER.md's Grid service);
                           grpc_tools writes campaign_grid_pb2{,_grpc}.py beside shapes_pb2
rpc.py, bench.py, allocator.py, gcbias.py, concurrency.py, verify_r14.py, run.sh   older harnesses
mech/                      the crossing-mechanism microbenchmark (no wire rule) and the
                           shapes_pb2 step (`mech/build.sh`) arms.py uses on 3.12
```

## The clean gate

Fresh `git worktree` at **86c141a1c** (WP13; `git status` empty), fresh build directories:
`./fetch_py37.sh`, `mech/build.sh python3.12`, `./run_campaign.sh --suite gate` (gate.sh at 3.12
and 3.7, the RPC controls, the RPC TCP prechecks). The shared server is built by gate step 90
from the snapshot's `poc/rust`, started unpinned (no `AK_CPU_SERVER` in a container) with
`AK_SERVER_TCP=0`.

Result: **`gate exit 0`** and **`GATE PASSED`**. All logs carry `# commit: 86c141a1c`.
- 91-98, 100-104 as before. The h2-batch steps 106 (conformance on `_akffi_rpc`), 107 (R-D3, 80
  aborted, 0 timed) and 108 (`_akffi_rpc_nounk`) pass at both levels, each naming the
  `h2batch/` module and its core.
- 103 and 98 are identical. 105 is identical on both h2 variants against `counts/rpc-*.txt`, as
  replaced at 86c141a1c: D's (d) rows have +1 call and +1 reset per call, from the shim fix
  (J56). `abi-full` 560, `abi-nounk` 344, `rpc-full` 84, `rpc-nounk` 48.
- The three RPC controls fail inside a pyperf benchmark with no JSON (`campaign/gate/rpc-control-*.out`).
- The RPC TCP prechecks (`campaign/gate/rpc-tcp-precheck-{full,nounk}-{stock,h2-batch}.out`):
  every cell on both client configurations; TCP_NODELAY on every live socket to the server
  (17/17, 18/18 full; 8/8, 9/9 no-unknown); write syscalls per d/16MiB call on Cf at k = 1:
  stock 1,035-1,038, h2-batch 77-80.
- An earlier gate at 08053a6c6 failed at 105 only (the count change above), kept as
  `logs/python/109-wp13-rpc-counts-before-replacing.log`.

**Minimal smoke** (owner's small-test rule), same worktree, figures stripped: rpc `--smoke`,
one launch and round, `AK_CAMPAIGN_RPC_TRANSPORTS=pinned`, `AK_CAMPAIGN_RPC_BUILDS=full`, both h2
variants, `--warmups 1`, `--loops 2 / 1 / 1`, `serve.sh warm 8`, AK_WORKERS 8 with the sysconf
shim. Values per h2 variant: ab 123, c 68, d 68. Every sample carries `h2`; TCP_NODELAY read
back set on all 4,026 socket reads; every worker's facts show `sysconf_nprocessors_conf` 8,
`event_engine` 8, `tokio-rt-worker` 8. Codec check: P1.1, full build, 38 benchmarks. Per-sample
JSON lines are not committed (`ffi/logs/PURGED.md`); each RPC log's header is
(`campaign/rpc-*-launch1.header.txt`). The rest of `campaign/` (codec families, calib) is an
older smoke, not rerun.

**Grouping:** nothing is grouped. Each pyperf benchmark gets its own worker process (22a at
e6c909630).

## The RPC grid on pyperf (FIX-PLAN WP9, CAMPAIGN req 22a as amended)

The hand-written sampler (`camp_rpc.sample` / `main`) is removed, not kept as a second arm;
`camp_rpc.py` exits with a pointer if run. The codec suite's by-hand loop (`camp_codec.main`,
with its own calibration, warm-up, rounds and clock) is removed too; `camp_codec.py` is the
case library pyperf's `camp_pyperf.py` builds from.

- **Benchmark** `rpc|build|transport|dir|payload|cell|k`, one per cell, direction (a, a+read,
  b, c, d), payload (P2.2; P5.3 / P5.4 for c; 4MiB / 16MiB for d), in-flight level (a, a+read,
  b: 1, 8, 16; c, d: 1, 8) and transport. Every cell, extra, framed twin and copy-path twin of
  the old grid is there; per launch: full 518 benchmarks (ab 246, c 136, d 136), no-unknown 292.
- **Invocation** = one batch of k calls in flight (one call on each of the k threads of a pool
  created in the setup), `inner_loops = k`. `--loops` is fixed per group, so pyperf spawns no
  calibration worker.
- **Grouping** (to keep the worker count sane): one pyperf invocation per build and direction
  group, ab (a, a+read, b), c and d, each with its own `--loops`
  (`AK_CAMPAIGN_RPC_LOOPS_AB / _C / _D`: campaign 25 / 8 / 3, smoke 2 / 1 / 1),
  `--processes 1 --values ROUNDS --warmups AK_CAMPAIGN_RPC_WARMUPS` (campaign 3, smoke 1),
  `--affinity AK_CPU_CLIENT --copy-env`. Six invocations per launch, in the build order
  alternated by launch.
- **Server:** the runner starts the launch's one server (since WP10 the Rust rpc_server,
  below) and warms it before any invocation. Each benchmark worker opens its channel in its
  setup: one channel per cell per benchmark process.
- **Checks:** every call. A failed call, a retain decode that leaves a buffer undelivered, or
  contexts that are not per thread fail the worker; pyperf fails; the runner discards the
  whole launch (`rpc-launchN.ABORTED`) and stops. The gate's planted controls run a pyperf
  benchmark and must fail inside it with no pyperf JSON (WP10 form below).

**What the framework forces that differs from the hand-written sampler** (stated in every RPC
header as `framework_forces`):
1. A worker PROCESS per benchmark, where the sampler used one client process per build and
   launch: each benchmark has its own channel, its own core runtime (where it uses the core),
   and its own pool of k threads (was one pool of 16 for every k). Cells of one block therefore
   no longer share a process; A is a same-launch control, not an in-process one.
2. A sample is `loops` batches of k calls, not a fixed `calls` count split over k threads; the
   uploads run fewer batches than the small calls (per-group `--loops`).
3. The warm-up is pyperf's warm-up values per worker (`--warmups`), replacing "one sample's
   calls per cell before round 1". The server warm-up stays (req 13).
4. The order is pyperf's sequential order: blocks of (transport, direction, payload, k), the
   cells rotated by one per launch inside a block, the whole list rotated by a third per
   launch. Cells are no longer rotated per round inside one process (pyperf cannot
   interleave, as for the codec suite, R-H23).
5. A failed check discards the launch (both builds, every group), where the sampler closed its
   own log as ABORTED.

**Custom code beside pyperf, and the requirement each piece serves** (everything else is
pyperf's: warm-up, loop and invocation control, the worker process, values, ordering of runs,
the raw JSON):

| piece | where | requirement |
|---|---|---|
| the one server process per launch, started, warmed and stopped by the runner through poc/rust/serve.sh | run_campaign.sh, camp_rpc_srv.py | 13 as amended at 9f6d579fa (the shared server, pinned, both configurations), 4, 24 (`serve.sh warm`) |
| the grid precheck per build before its invocations | camp_rpc_pyperf.py `--precheck` (camp_rpc.gate) | 26, 18 |
| the setup's checked call (and (a)'s re-encode to P2.2) in each worker | camp_rpc_pyperf.py `setup` | 18, 26 |
| the time_func: CLOCK_PROCESS_CPUTIME_ID over the batches (pyperf's clock is perf_counter) | camp_rpc_pyperf.py, camp_pyperf.py | 21 as amended |
| wall of the same batches, to a side file joined by the exporter | camp_rpc_pyperf.py, *_export.py | 21 (wall beside CPU) |
| the per-call checks, the leak and per-thread context checks, raising in the worker | camp_rpc_pyperf.py `time_func` | 18 |
| abort-and-discard of the launch | run_campaign.sh `discard` | 18 |
| `gc.collect()` before each value, untimed | both time_funcs | 25 |
| M_TOP_PAD at import | camp_codec.py (allocator.py) | 25 |
| the export to section 7's JSON lines with every label (cell, dir, inflight, transport, build, unknown_mode, send_path, payload, launch, round, phase) | *_export.py | 28 |
| the per-launch rotation of the benchmark list | names_of / camp_pyperf.py | 22 (pyperf cannot interleave) |
| the fixed `--loops` per group | run_campaign.sh | 22a grouping (no calibration worker for batches of k in-flight calls) |

**Not moved to pyperf:** `camp_calib.py`'s crossing-cost loop (req 20). The timed work is one
C loop in the shim (`crossing(n, kind)`), with its own rounds and a warm-up call, and the
`perf stat` readings need separate processes of a fixed n. It was not in WP9's scope (the RPC
grid and the codec suite); it is stated here, not changed.

## The shared RPC server (FIX-PLAN WP10, CAMPAIGN req 13 as amended at 9f6d579fa)

- **Server:** the Rust slice's tonic `rpc_server` (poc/rust/SERVER.md), built by `build.sh`
  (gate step 90) through `serve.sh build` of the tree the build reads (the runner's snapshot, so
  the server is the commit the run names), and started per launch by the runner:
  `serve.sh start --out <out>/rpc-server-launchN` (pinned by serve.sh to `AK_CPU_SERVER`,
  `AK_SERVER_THREADS` tokio workers, default 4), `serve.sh warm AK_CAMPAIGN_SERVER_WARMUP`
  (checked calls per direction from a tonic and a core client, both sockets; campaign 64,
  smoke 8), `serve.sh stop`. The runner keeps serve.sh's state file private (`AK_SERVE_STATE`
  under `build/`), so another slice's start does not collide. The server's own log is kept
  beside the RPC logs and quoted in every RPC header (`server_log`).
- **Configurations:** server `shipped` = tonic's defaults; `pinned` = stream and connection
  windows 4 MiB, adaptive window off; receive limit 8 MiB on both. The client's `shipped` /
  `pinned` configuration (grpcio options, core client options, unchanged) dials the socket of
  the same name.
- **Paths:** `/armonik.ffi.campaign.v1.Grid/` Fetch (a, a+read), Push (b), Upload (c),
  UploadStream (d, timed; the 8-byte count checked on every call), UploadStreamCheck (d,
  untimed: count and SHA-256 of the messages as received, once per cell in the precheck and in
  each worker's setup). Messages, limits and the ids-on-first-message rule as SERVER.md; the
  stream sends ids `session-u2` / `result-u2` on message 0 only.
- **Cell A** now calls through the generated stub (`campaign_grid_pb2_grpc.GridStub`, from
  `proto/campaign_grid.proto` by grpc_tools, as ArmoniK's Python client is generated). Its
  methods are registered (`_registered_method=True`, what grpcio 1.84's generated code emits);
  before WP10 A, D and F called `channel.unary_unary(path, ...)` unregistered, which is not
  what a generated stub does. D and F now pass `_registered_method=True` too, so they make the
  stub's call with another codec. A's per-call check is the stub's FromString succeeding (a
  short body fails to parse) and the task count; A no longer sees the raw length.
- **Authority:** grpcio's default `:authority` on a `unix:` target is the percent-encoded
  socket path, which the server's h2 refuses with RST_STREAM PROTOCOL_ERROR (checked: every
  grpcio cell failed, every core cell passed). Both client configurations set
  `grpc.default_authority=localhost`; `shipped` carries no other grpcio option.
- **Planted controls** (req 18; the server is shared, so the plant is on the client,
  `AK_CAMP_PLANT`, passed to pyperf's worker as `--plant`): `short` calls FetchShort,
  `count` expects one byte more from UploadStream, `digest` expects a wrong digest from
  UploadStreamCheck, each from a cell's 2nd call on. In the gate suite: `short` fails B at k=1
  inside pyperf's timed loop ("response is 540421 bytes, want 540422"), `count` fails C-drop in
  d inside the timed loop ("the server counted 4194304 bytes, want 4194305"), `digest` fails
  C-drop's setup check; no pyperf JSON in any of the three.
- **Removed:** `camp_server.py` (the grpcio server), the Python server warm-up, and the server
  plants (`AK_CAMP_PLANT` on the server). `rpc_gate.py` (gate 94, R-D3's injected failures)
  keeps its own in-process grpcio server: it is a correctness step, never timed.
- **Counts:** `rpc_counts.py` (gate 105) against the Rust server gives `counts/rpc-full.txt`
  (84 rows) and `rpc-nounk.txt` (48) unchanged.

## The WP12 contract (FIX-PLAN WP13, owner 2026-10-03; D8-D11, D14; reqs 4, 17, 21, 25 as amended)

1. **TCP 127.0.0.1 for every timed cell (D10).**
   - The shared server is started with `AK_SERVER_TCP=0`: any free port, the PINNED server
     configuration, TCP_NODELAY on accept (SERVER.md).
   - Every cell dials `127.0.0.1:PORT` (grpcio target) or `http://127.0.0.1:PORT` (core
     client), whatever its client configuration.
   - So over TCP, `shipped` and `pinned` differ on the client side only:
     - grpcio channel options. shipped: only the default authority. pinned: 4 MiB
       lookahead, BDP probe off, 16 MiB limits.
     - The core client. shipped: `ak_client_new`, tonic's defaults. pinned: `ak_client_opts`
       with 4 MiB windows, adaptive off, tcp_nagle 0.
   - This is stated in every RPC header (`transport_net`). The server keeps its Unix sockets
     open; no cell dials them.
2. **Nagle off, read back.**
   - Every worker reads TCP_NODELAY with getsockopt on every live socket of its process that
     is connected to the server (`camp_meas.nodelay_summary`). It reads after its setup and
     after every value.
   - One socket without TCP_NODELAY fails the benchmark.
   - grpc-core sets it on its client sockets (grpc_set_socket_low_latency); tonic's Endpoint
     sets it by default. The read-back is what gets recorded. In the smoke every socket had it
     set, and the count is in the header.
   - The precheck reads it back too.
3. **RPC client CPU = perf task-clock (req 21 as amended).**
   - `camp_meas.task_clock_open()` opens a PERF_COUNT_SW_TASK_CLOCK counter on the process
     (pid 0, inherit) when the worker imports it, before any thread exists. So the pool, the
     core runtime's workers and grpc-core's threads are all counted, and threads that have
     exited are summed on read.
   - The time_func returns task-clock. That is pyperf's value, and `cpu_ns` in the export.
   - Recorded beside it, around the same batches:
     - `process_cpu_ns` (CLOCK_PROCESS_CPUTIME_ID);
     - over the worker's affinity set (the CLIENT CPUs): /proc/stat's irq and softirq ticks,
       and /proc/softirqs' NET_RX and NET_TX (`client_softirq_ticks` and the related fields).
   - A worker whose perf_event_open is refused fails. In this container perf_event_paranoid
     is 2, and the open is allowed.
4. **Pools at AK_WORKERS (D8, D14).** The runner reads `AK_WORKERS` (8) from campaign.machine
   and sizes three pools with it:
   - the core runtime: `ak_runtime_new(AK_WORKERS)`. `AK_CORE_WORKERS` still overrides the
     core's pool on its own;
   - the server: `AK_SERVER_THREADS=AK_WORKERS`;
   - grpc-core. It sizes itself from `sysconf(_SC_NPROCESSORS_CONF)`, not from the affinity
     mask, and grpcio has no setting for it. So the runner LD_PRELOADs `build/ncpus_shim.so`
     with `AK_SHIM_NCPUS=AK_WORKERS` into the RPC grid's pyperf processes. The shim's source
     is `native/ncpus_shim.c`, a copy of the C++ slice's.

   Checked in the smoke's workers: 8 `event_engine` threads and 8 `tokio-rt-worker` threads.
   Without the shim, this 4-CPU container gives 4 `event_engine` threads.
   - grpcio's Python client has no executor of its own; a ThreadPoolExecutor exists on servers
     only.
   - Every worker records its CPU facts and its threads by name; the header quotes them
     (`pools`).
   - The precheck and the gate's controls run without the shim, because they check
     correctness only.
5. **Both h2 variants (D11 as amended).**
   - **Build.** `build.sh` builds the h2-batch variant of every core that carries h2: rpc,
     rpc-nounk, rpc-count and rpc-count-nounk.
     - The full rpc core goes through `poc/codec/h2-batch/build.sh`.
     - The other three are built by hand with the same `--config` and the same patched
       source. The script takes features only, and the no-unknown cores need
       `--no-default-features`.
   - **Source check.** The compiled-in h2 source of all eight rpc cores is checked: stock
     cores must show `h2-0.4.19`, h2-batch cores `h2-batch-src`.
   - **Loading.** The h2-batch shims keep the same module names, in `build/<tag>/h2batch/`.
     `AK_H2=h2-batch` (or `--h2`) puts that directory first on the path (arms.py). The
     codec-only cores have no h2 and are shared by both variants.
   - **Gate.**
     - Steps 92, 94 and 102 run on stock. Steps 106-108 are the same three on h2-batch; each
       log names the module file and the core it mapped.
     - Step 105 runs on both variants.
     - The RPC grid's precheck runs over TCP for both builds and both variants. It includes a
       write-count marker: write syscalls per d/16MiB call on the framed core cell at k = 1.
       Stock gives about 1,035 and h2-batch about 76-80, matching the Rust slice's 1,030 and
       73-78. A count on the wrong side of 400 fails.
   - **Labels.** Every RPC sample carries `h2`. Codec samples carry `h2: "none"`, because the
     codec cores have no rpc feature. `camp_summary` keys both the groups and the cell-A
     baselines on `h2`.
6. **Rebuilt on the current core** (p1, the ring of 6 spare buffers).
   - On this core, the python shim's first encode on each thread came out 5 bytes short.
   - Cause: since the framed default (e8fe14868), `ak_enc_ctx_new` sets 5 bytes of headroom
     that only `ak_enc_reset` lays down, and the shim reset only a reused context.
   - Symptoms: conformance failed at byte 0, and the RPC precheck failed on the first P5.3
     encode.
   - Fix: `py_capi.py` now resets a new context as well (8b87eea10).
   - Open for the aggregating session: whether `ak_enc_ctx_new` should lay down its own
     headroom. Any host that encodes before a first reset would hit the same thing.
7. **D9 (glibc trim, req 25), as amended by the owner on 2026-10-03.** D9 applies to this slice:
   the core's and CPython's large buffers both come from glibc's arena.
   - **Main figures (RPC grid):** glibc's DEFAULT allocator, as in production. No
     GLIBC_TUNABLES (the runner unsets it), and no mallopt: `camp_rpc.py` no longer applies
     J26's M_TOP_PAD.
   - **Diagnostic:** `AK_CAMPAIGN_ALLOC_PINNED=1` adds a pinned pass under D9's
     `GLIBC_TUNABLES`. Its files are suffixed `-allocpinned`.
   - **Labels:** every RPC sample carries `allocator` (default or pinned) and `minflt`
     (getrusage ru_minflt of the worker around the same batches). The header states both
     modes.
   - **Check:** a worker whose environment does not match its label fails. The control is in
     `logs/python/d9-alloc-smoke/mislabelled-pass-control.out`.
   - **Minimal smoke** (`logs/python/d9-alloc-smoke/`, d/16MiB at k = 1, pinned client
     configuration, stock h2): minor faults per call were A 3 and Cf-drop 514 under the
     default allocator, against A 5 and Cf-drop 3 pinned. So the pass is running.
   - The codec suite still applies M_TOP_PAD (J26, req 25's warming rule). This is stated, not
     changed.
8. **Unchanged, and stated:** each benchmark worker builds every cell of its direction key.
   So idle grpcio channels and a core runtime exist in every worker beside the timed cell.
   WP11 item 4 measured this for C++; it is not measured here.

## What was checked, and the log that carries it

Full build, logs 90-98:
- **Build (90):**
  - R0 and R1;
  - every core exports `ak_init`, and every shim imports it and exports one `PyInit_`;
  - R5 (the shims' `ak_*` imports are unresolved);
  - must-fail controls:
    - the noinit shim imports no `ak_init`;
    - a planted layout mismatch refuses to import;
    - a shim without `AK_RPC` imports 0 of 6 section 9 entry points.
- **Conformance (91 `_akffi`, 92 `_akffi_rpc`):** R2 encode and decode on 16 payloads, every
  arm; section 10 layout (400 facts) compared at import; crossing counts from the counting
  build, both halves.
- **Corpus (93, 702 rows, each row in a worker process under a timeout):**
  - ffi-cext, ffi-attr, ffi-chunk256 and ffi-retain: 680 pass, 0 fail, 6 disputed
    (excluded), 16 not in the C ABI (`Nest`, refused by name);
  - py-drop and py-retain: 696 pass, 0 fail, 6 disputed;
  - between-arm identity: drop arms 542 rows, 0 differ; retain arms 542 rows, 0 differ;
  - every unknown row is written in the retained form by both retain arms, except the
    disputed `U-map-entry`;
  - the 3.7 run is compared byte for byte with the 3.12 run;
  - controls: `proj`, `reenc` and `accept` fail on every arm, and `noinit` fails on the ffi arms;
  - decision 11 controls:
    - zeroed position: 1,373 pairs, 0 mismatches;
    - wrong root: 812 pairs refused with -8;
    - leak: 0 undelivered buffers;
    - per-thread contexts: 0 created on reuse, 232 across 8 threads;
    - per-thread `AK_LAST_RECLAIMED`, and its process-wide twin failing as required;
  - rule 4 (R-H8): poc/cpp's three `d11_oneof` sequences through the retain arm. The final
    bag is the last member's own run (stamp -> nothing: run 2; stamp -> nothing -> stamp:
    run 3). After the switch to the scalar `as_int`, no object carries a bag. Every sequence
    has `last_reclaimed() == 0`, so each inactive slot was freed at delivery;
  - the leak check's must-fail twin (R-H9): `ctl/_akffi_corpus_skiprelease` (release skipped)
    is flagged on 307 of 311 rows by the leak check and on 3 of 3 sequences by the oneof control.
- **R-H14** (91, 92, 100, 102), every arm: an undeclared oneof case is refused with code -11
  (`.code`); a selected message member holding None writes an empty body, byte-identical to
  upb with the member set to an empty message.
- **94:** the RPC gate under injected failure (R-D3): 80 failure rows aborted, 0 timed.
- **95:** R-D1 wrapped lengths. **96:** U1.
- **97:** the source check: 10 build variants against 3.7.5 headers (`-Werror`), and against the
  3.9, 3.10, 3.11, 3.12 and 3.13 headers present; the pre-port tree fails, as the control.
- **98:** crossing counts per element, row for row identical to log 85; and the rendered C
  header compared with the cpp slice's (information, not a gate).

No-unknown build, logs 100-103:
- **Build (90):**
  - the variant cores export 0 u-family entry points (full: 21);
  - the variant shims import 0 of them;
  - each module resolves `libak_core.so` to its own core;
  - layout facts: 240 in the variant (full: 400; corpus 340 against 574);
  - must-fail: the variant shim over the full core refuses to import.
- **100 / 102:** conformance on `_akffi_nounk` / `_akffi_rpc_nounk`. Includes the check that no
  facade class, C type or decoded object has `_unknown` (38 classes, 19 C types, 32 objects).
- **101:** the corpus through the variant (4 arms):
  - 680 / 696 pass, 0 fail;
  - the 307 unknown rows that have a dropped form write it on every ffi arm; the 4 open-enum
    rows have none in the manifest and are checked by C3;
  - byte identity with the full build's drop arms at the same level: 2,181 re-encodings,
    0 differ;
  - the controls fail as required;
  - variant controls: 0 positions; 622 of 622 retain calls refused; wrong root -8; per-thread
    contexts; no `_unknown` on 60 classes, 29 C types and 622 decoded objects, with the full
    facade's 150 findings as the must-fail twin.
- **103:** whole-number crossing counts of both builds, identical to `counts/`. They differ in
  5 of 160 rows: P1.2 decode reverse is 8 in the full build (drop) and 5 in the no-unknown
  build, on all 5 backends.

## Crossing counts (R5)

Counting build, both halves, per element, log 91 (3.12; 3.7 identical). There are three edges
and they are never added: shim to CPython (counted by the shim), core forward and core
reverse (counted by the core). Whole-number totals per call are in `counts/`.

| payload | shim -> CPython, C ext type (enc / dec) | shim -> CPython, plain and `__slots__` (enc / dec) | core fwd (enc / dec) | core rev (enc / dec) |
|---|---|---|---|---|
| P1.2 M1 | 7.00 / 7.00 | 29.00 / 24.00 | 0.01 / 0.00 | 0.00 / 0.01 |
| P2.2 M2 | 51.67 / 51.67 | 146.68 / 131.35 | 5.02 / 0.00 | 5.00 / 7.00 |
| P3.1 M3 | 5.40 / 3.15 | 13.40 / 9.96 | 0.01 / 0.01 | 0.01 / 0.01 |
| P4.1 M4 | 23.00 / 23.00 | 54.01 / 49.01 | 1.01 / 0.01 | 1.00 / 3.00 |
| P5.1-P5.4 M5 | 3.00 / 3.00 | 7.00 / 8.00 | 1.00 / 1.00 | 0.00 / 1.00 |
| P6.1 M6 | 302.00 / 152.00 | 308.00 / 158.00 | 5.01 / 0.01 | 5.00 / 7.00 |
| P7.1 M7 | 2.00 / 2.00 | 5.33 / 4.33 | 0.50 / 0.17 | 0.33 / 1.17 |

## Decision 11 and the two builds, as rendered

- **Contexts:** one decode context per root and one encode context per thread (a pthread key),
  created on first use and reused. A decode or encode re-entered on a busy slot gets a
  temporary context.
- **Full build decode:** `ak_dec_reset_<R>(ctx, retain ? &opts : NULL)`, the decode, and in
  retain `ak_dec_reset_<R>(ctx, NULL)`. Options are laid out from `plan.unk_opts_layout`; one
  entry per oneof, filled in the active member's group (ABI v1 rule 4 as amended 2026-09-26).
  Every armed position grows through `ak_py_grow`.
- **Full build delivery:** each group's slot becomes the facade's `_unknown`. Absent children's
  and inactive members' slots are freed. A map entry's buffer is freed (the U-map-entry gap).
- **Full build encode:** `retain` selects `ak_uencode_<R>` over `ak_ufix` groups.
- **No-unknown build:** `ak_dec_ctx_new_<R>(void)`, the decode, and `ak_encode_<R>`. None of the
  above exists in it, and `retain` raises ValueError.
- **`AK_LAST_RECLAIMED`** is thread-local.

## CAMPAIGN.md section 10 checklist

| # | status | how, or why not |
|---|---|---|
| 1 | met | the suites run one after another. The machine and its tenancy are the owner's |
| 2 | met | governor, turbo and SMT are read from /sys into every header. Setting them is the owner's |
| 3 | met | `AK_ISOLATION`, or isolcpus/nohz_full from /proc/cmdline, in every header |
| 4 | met | the CPU sets and `AK_WORKERS` (8, D8/D14) come from `ffi/campaign.machine`; the client pins itself to `AK_CPU_CLIENT`, the server to `AK_CPU_SERVER`; every pool at AK_WORKERS: core runtime, server tokio workers, grpc-core through the sysconf shim (WP13 item 4); each worker's affinity, sysconf counts and threads by name in every RPC header; codec and calib 1 thread |
| 5 | met | 3.7 runs the gate (corpus, byte identity) and no timing suite |
| 6 | met | `buildinfo.json` in every header: cc, CFLAGS, shared linkage, core profile (lto off), the features of every core of both builds; GC stated |
| 7 | met | 16 payloads; Latin-1 and wide on P1.2, P2.2 and P2.4 (R-H26); family `unknown` = the 92 accepted U-* rows at the shapes core's 7 roots, encode, decode and decode+read, through the timed `_akffi` / `_akffi_nounk` (R-H27); family `unknown-corpus` (the corpus-schema core, 311 rows) is a labelled extra |
| 8 | met | incumbent-prod, incumbent-best (labelled), core-ffi (push), host-gen; core-ffi-attr as a labelled extra. No pull arm exists in this slice (not measured) |
| 9 | met | encode, decode, and decode+read through one reader for every arm |
| 10 | met | core-ffi retain and drop in the full build; core-ffi no-unknown in the separate build (`--variant nounk`, its own pyperf invocation that also times the incumbent, order alternated by launch). host-gen drop and retain, over the same C-extension facade objects as core-ffi (R3, R-H16), and host-gen no-unknown: host-gen drop (the same text from both plans) over the no-unknown build's facade, which has no `_unknown` (R-H22), in the no-unknown process. host-gen over the plain facade is the labelled extra `host-gen-plain`. Incumbent at upb's default (retains), stated |
| 11 | met | every encode arm is timed with the input as one hot graph (`encode`) and as a pool of distinct graphs whose wire bytes reach `AK_POOL_BYTES` (default 13.75 MiB, cap `AK_POOL_MAX` 65536 graphs; built and checked outside the window; the smoke uses 1 MiB, stated) (`encode-pool`); the end state is grpcio's transport-ready `bytes` for every arm, and for core-ffi also the bytes copied into a bytearray sized once (`encode-reused`, `encode-pool-reused`). upb-python has no serialise-into entry point and host-gen appends to a bytearray that CPython reallocates when cleared, so neither has a reused-buffer row (stated in the header). Samples carry `input` and `end_state` |
| 12 | met | full build: A, B, C-retain, C-drop, D-retain, D-drop, E-retain, E-drop, F-retain, F-drop, and the framed send-path twins Bf, Cf-*, Ef-* in b, c and d (grpcio has no framed path) (E and F: host-gen over the core's transport and over grpcio, R-H35), plus the labelled extras; no-unknown build (its own process): A, B, C-nounk, D-nounk, E-nounk, F-nounk (host-gen drop over the no-unknown facade); `unknown_mode` on every sample |
| 13 | met | one server per launch, the Rust slice's tonic rpc_server (WP10, req 13 as amended at 9f6d579fa), started, warmed and stopped by `run_campaign.sh` through the snapshot's `poc/rust/serve.sh`, pinned to `AK_CPU_SERVER`, serving every cell of both builds on two Unix sockets (shipped: tonic's defaults; pinned: windows 4 MiB, adaptive off); P2.2 pre-serialised at start-up on (a), (b) and (c) decoded by prost; warmed by `serve.sh warm AK_CAMPAIGN_SERVER_WARMUP` (64) before any pyperf invocation; one channel per cell per benchmark process (a grpcio channel for A, D, F; a core client for B, C, E and each extra), opened in the worker's setup (WP9) |
| 14 | met | (a) as `a` and `a+read`; (b); (c) a unary upload of P5.3 / P5.4, decoded by prost on the server, empty answer; (d) the client-streamed upload of 2 MiB M5 chunks (ids on the first), 4 MiB and 16 MiB, the server answering the data byte count on UploadStream, checked on every call, and the count and SHA-256 on UploadStreamCheck, checked once per cell before timing (WP10). c and d at 1 and 8 in flight in every cell, d with a third of the calls. B, C and E stream through ak_call_open / ak_call_send / ak_call_recv; A, D and F through grpcio's stream_unary. The server's receive limit is 8 MiB per message for both configurations (covers P5.4 and a 2 MiB chunk); the core's limits are its defaults (shipped: 4 MiB received) or 16 MiB (pinned), enforced (D44) |
| 15 | met | 1, 8 and 16 in flight |
| 16 | met | B, C and E use the core's blocking delivery; queue and callback are labelled extras, direction (a) only; A uses the generated stub's blocking unary multicallable (`GridStub`, registered methods, WP10); D and F make the same registered call with another codec; A, D and F stream through grpcio's stream_unary (registered). Stated in the header |
| 17 | met | WP13 (D10): TCP 127.0.0.1 for every cell, the shared server's TCP listener (pinned server configuration), so shipped and pinned differ on the client side only (grpcio options; ak_client_new against ak_client_opts), stated in every header; TCP_NODELAY read back on every live socket to the server after every setup and value, a socket without it fails the benchmark |
| 18 | met | every call checked (status; (a) the length, A by its stub's FromString and task count; (b), (c) the empty answer; (d) the server's byte count; a non-OK gRPC status is AK_ERR_RPC_STATUS and fails the call); (d)'s digest through UploadStreamCheck once per cell before timing (precheck, worker setup); the server checks every request; a failure fails the pyperf worker, and the runner discards the whole launch (`rpc-launchN.ABORTED`) and stops; three planted controls, selected on the client (the server is shared), fail inside a pyperf benchmark in the gate suite with no pyperf JSON: FetchShort and a wrong (d) count in the timed loop, a wrong digest in the setup check |
| 19 | met | the counting builds count every ABI call the timed call makes, resets apart (the shim's macros), after one warm call, context creation and destruction excepted (per thread, not per call); the resets' place is stated (decode: one before, in both modes; encode: before). The counting build grows geometrically, as the timed one does (req 19 as amended, WP8). Committed and gated: `counts/abi-full.txt` (drop and retain, 16 payloads x 5 backends and the 92 U-* rows; retain with no pre-placed buffer and exact-size grow), `counts/abi-nounk.txt`, `counts/rpc-full.txt` / `rpc-nounk.txt` (RPC cells B, C, D, E per call in each build's modes, E-nounk included, on the rpc counting builds, gate 105), and the older `counts/crossings-{drop,nounk}.txt`; calib stops on a difference from `counts_expected.txt`; gate 98 against log 85 |
| 20 | not met | host forward and forward+reverse are measured separately, and the rust slice's crossing benchmark is built and run pinned. `perf stat` is implemented, but `perf` is absent in this container, so cycles and instructions have never been read |
| 21 | met | codec: CLOCK_PROCESS_CPUTIME_ID per pyperf value; RPC (WP13, as amended): perf task-clock of the whole worker process, opened before its first thread and inherited, as pyperf's value and `cpu_ns`, with `process_cpu_ns` and the CLIENT CPUs' irq / softirq ticks and NET_RX / NET_TX beside it; calib around its C loop; wall beside every one |
| 22 | met | pyperf cannot interleave (not a defect, owner 2026-09-26). codec: blocks of (payload, content, direction), the arms inside a block rotated by one per launch, the whole list rotated by a third per launch. rpc (WP9): blocks of (transport, direction, payload, k), the cells rotated by one per launch inside a block, the list rotated by a third per launch. Both stated in every header |
| 22a | met | pyperf 2.10.0 for both codec and rpc. codec: `--processes 1 --values ROUNDS --warmups 3 --min-time 0.1 --affinity`. rpc (WP9): one invocation per build and group (ab, c, d), `--processes 1 --values ROUNDS --warmups 3 --loops 25 / 8 / 3 --affinity --copy-env`, one batch of k calls in flight per loop (`inner_loops = k`); the hand-written sampler and the codec by-hand loop are removed. Every raw value and warm-up exported by the two exporters, the raw pyperf JSON beside it (the committed smoke has its figures stripped and omits that JSON). What pyperf forces that differs from before: section "The RPC grid on pyperf" |
| 23 | met | defaults 5 rounds x 3 launches; every sample written (smoke: 1 x 1) |
| 24 | met | every warm-up a runner parameter, in every header: codec, pyperf's warm-up values and loop calibration (`AK_CAMPAIGN_PYPERF_WARMUPS`, `AK_CAMPAIGN_PYPERF_MIN_TIME`); rpc, pyperf's warm-up values per worker (`AK_CAMPAIGN_RPC_WARMUPS`, 3; smoke 1) and the server warm-up (`AK_CAMPAIGN_SERVER_WARMUP`, 64; smoke 8) |
| 25 | met | M_TOP_PAD before any allocation in the codec suite; GC on, `gc.collect()` before every sample. RPC grid (D9 as amended 2026-10-03): default allocator for the main figures, the pinned GLIBC_TUNABLES pass behind AK_CAMPAIGN_ALLOC_PINNED=1, `allocator` and `minflt` on every sample, the environment checked against the label |
| 26 | met | codec, rpc and calib each call `need_gate` and refuse to time without a `gate.ok` for the trees they read (calib added, R-H19); the gate covers every codec arm in both unknown-field modes and the no-unknown build (101) |
| 27 | met | header: commit (a dirty tree refused unless `--allow-dirty`, smoke only), machine, CPU sets, versions, build, transport, warm-up, repeats |
| 28 | met | one JSON object per raw measurement with the listed fields; rpc rows carry cell, payload, dir, transport, inflight, build, unknown_mode, send_path (move, copy, framed, grpcio, reference), launch, round, phase (warmup or value), cpu_ns, wall_ns, iters = loops x k |
| 29 | met | `--out`; the smoke is in `logs/python/campaign/` |
| 30 | met | `camp_summary.py`: median [min, max] per (build, arm or cell, ...), and the ratio to the same build's incumbent-prod or cell A (same transport, direction, payload and in flight: the payload was added in WP9, so (c)'s P5.3 / P5.4 and (d)'s 4MiB / 16MiB each have their own cell A; tested with a must-fail twin) formed from per-launch medians (owner R-H24), labelled cross-process; references and groups keyed on `build`, which every sample carries (R-H1); tested with two builds whose incumbents differ (gate 104) |
| 31 | met | `run_campaign.sh` with the common interface. The top-level `ffi/campaign.sh` is the aggregating session's |
| 32 | met | the smoke logs' headers carry the INSTRUMENTATION line; no figure from them is quoted |

## Register H (WP6 re-review), python's findings: disposition proposed

| finding | confirmed? | evidence | fix | proposed disposition |
|---|---|---|---|---|
| R-H1 | confirmed | the old `camp_summary.py` keyed `base` on (payload, content, dir, launch, round) and the groups without a build, and the sorted glob loads `-nounk-` last, so it overwrote the full build's incumbent-prod and cell A | `build` in every sample (`camp_lib.Log(build=)`); references and groups keyed on it; ratios from per-launch medians; `test_camp_summary.py` (two builds, incumbents 100 and 400 ns, RPC A 1000 and 6000; its twin without `build` gives the pooled 0.8, not 2.0), gate 104 | fixed |
| R-H16 | confirmed | `camp_codec` host-gen used `fp` / `CT_PLAIN` while core-ffi used `fc` / `CT_CEXT` | host-gen drop and retain now over the C-extension facade objects core-ffi uses (shapes and unknown families); `host-gen-plain` is the labelled extra. The C type is the headline because it is the facade the composed binding ships: a field is a struct member to the shim | fixed |
| R-H8 | confirmed | no corpus row put unknowns in a oneof member through ffi-retain | poc/cpp's three sequences in the d11 controls: exact bags, the scalar switch leaves no bag, `last_reclaimed() == 0` (log 93) | fixed |
| R-H9 | confirmed | the leak check had no twin | `AK_PLANT_SKIP_RELEASE` shim: flagged on 307 of 311 rows and 3 of 3 sequences (log 93) | fixed |
| R-H14 | confirmed, all three parts | py_pure had its own `ERR` table; the undeclared case raised a bare ValueError (pure and shim); a selected None member raised (pure and shim) | codes read from `plan.FIXED.codes`; an undeclared case is refused with -11 as `.code` (pure: `EncodeError`; the shim passes the case to the core, whose AK_ERR_ABI is raised with `.code`); a None member is written as an empty body, per plan ENCODE RULES ("written iff the case selects it, whatever its value"); checked on every arm (logs 91, 92, 100, 102) | fixed |
| R-H2 (python part) | confirmed | `camp_rpc.sample` created and joined `inflight` threads inside the timed window | one pool of max(in flight) threads, created before the first window and reused (idle threads block on an Event); smoke at 31fc3eecf: 16 threads started, 32 contexts, 0 leaked, every call checked | fixed |
| R-H19 (python part) | confirmed, both | `run_campaign.sh` calib never called `need_gate`; "in-process control" wording in `run_campaign.sh` and `camp_codec.py` | calib calls `need_gate`; wording corrected to "same-launch control" (pyperf spawns a worker per benchmark). The RPC client's "in-process controls" A and B are left as they are: they do share the client process | fixed |
| R-H23 | owner decision | pyperf cannot interleave | order stated (checklist row 22); arms rotated inside each block per launch | stated; rotation added |

## What the plan does not carry (reported, not decided here)

1. **ABI v1 sections 3-5's fixed vocabulary**: error codes, the span and blob types, the
   context and callback types, the fixed exports and the counters. Every C header renderer
   restates them as fixed text.
2. **`plan.lifecycle`** names but does not define the `AK_INIT_*` values, `ak_err`'s layout or
   `ak_log_fn`'s signature.
3. **Map entry order** on encode. This backend sorts by key (UTF-8 byte order).
4. **`presence == "direct"`** has no encode rule. On the wire it is implicit-presence bytes.
5. **The 10th varint byte**: bits past 64 are dropped by the core and by the pure-Python codec.
6. **Group depth**: the core bounds groups at 100 per skip, independent of message depth, and
   the pure-Python codec follows the core.
7. **Field numbers above 2^29-1**: not stated. The core truncates `key >> 3` to u32; the
   pure-Python codec does not. No corpus row reaches it.
8. **Unknown fields inside a map entry** have no bag in any facade (`U-map-entry`, disputed).

## Open defects

| # | where | what |
|---|---|---|
| U1 | the incumbent | upb drops a map entry carrying an unknown field (log 96) |
| noinit C4 | `corpus.py` | under the `noinit` control a reject row is "refused" with -10 and counts as refused; only the accept rows show the control. The rust harness does the same |

Fixed defects (D1-D14, the NULL module state in `mod_traverse`, the process-wide
`AK_LAST_RECLAIMED`) are in JOURNAL.md.

## What is not measured, or not built

- **Every performance figure.** Deferred to the campaign. The container smokes are
  instrumentation.
- **Hardware counters** (req 20): `perf` is absent here.
- **The re-entrant temporary-context path** (a decode or encode called from inside a
  callback on the same thread for the same root): implemented, exercised by no control.
- **3.8 to 3.11 and 3.13 at run time.** Those interpreters have no protobuf/grpcio here. The
  shim is compiled against their headers (log 97) but not run.
- **3.7 floor limits:** no `ssl` module (bionic's interpreter without libssl1.1); protobuf
  4.24.4 as the incumbent there.
- **Not built:**
  - the pull decode family (no pull arm);
  - the streamed-upload direction (req 14, optional);
  - an encode-side RPC server arm;
  - streaming, TLS, deadlines, metadata;
  - free-threaded CPython;
  - abi3 for the shim;
  - decision 13's borrowed span;
  - allocation per operation.
- **C5 for the five large rows** without a projection (`B-P2_5`, `B-P4_1`, `C-elemu-512`,
  `C-leaf-2048`, `C-mixed-100`): checked by C3 re-encode only (log 93).
- **The concurrency suite** (`concurrency.py`) has no current committed log. Log 56 is from an
  older commit.
- **Content sets** are measured on P2.4 only; P1.2's crossing counts cover ASCII only.
- **The facade's `_unknown` in the full build** is one slot per object. It is priced only
  through the full build against the no-unknown build.
- **The campaign smoke** in `logs/python/campaign/` is the WP13 minimal smoke (pinned client
  configuration, full build, both h2 variants; headers and stripped pyperf output only). The
  `shipped` configuration and the no-unknown build have not run on pyperf over TCP in a
  committed log; the gate's TCP prechecks called every cell of both. Not measured either: the
  default-allocator pass of D9, and the idle channels' cost in each worker (WP13 item 8).
- **No reused-buffer encode for the incumbent and host-gen** (req 11 (i)): upb-python has no
  serialise-into entry point, and host-gen appends to a bytearray that CPython reallocates.
- **RPC counts** are taken on the `shipped` transport, one call at a time.
- **The move path.**
  - Cell C, and its framed twin Cf-*, sends through `ak_call_unary_enc` (b, c) and
    `ak_call_send_enc` (d). The facade is encoded into the thread's core encode context and the
    context's buffer becomes the request; no Python bytes object is made. This is the shim's
    `encode(..., into=(client, path))` or `(call, last)`.
  - The copy path (`ak_enc_take` into a `bytes`, then `ak_call_unary` / `ak_call_send`) is kept
    as the labelled extra `Cc-*`.
  - Direction (a) has an empty request and sends it with `ak_call_unary`, as the Rust slice does.
  - **D cannot use `ak_enc_take_owned`.** grpcio's request serializer must return a `bytes`
    (checked: a memoryview and a bytearray are refused with TypeError), and a `bytes` owns its
    memory, so the core's buffer cannot be handed over without a copy. D keeps its one copy
    (`ak_enc_take` into a bytes object).
- **Decode-side re-validation** (WP8 item 4): CPython builds a `str` from UTF-8 only through a
  validating decode (`PyUnicode_DecodeUTF8`); no non-validating constructor exists, so a
  string the core accepted is validated again as it is decoded. The sparse fill already
  clears only the elements it fills (`memset` of the `k` elements of a chunk).
- **D and F streams** encode on a grpcio thread started per call (grpcio consumes a request
  iterator on its own thread), so they create one encode context per call. This is counted
  and allowed for in the per-thread check.

## Next step

1. When the core's `ak_enc_ctx_new` returns a ready context (the owner's decision; the Rust agent):
   the shim's extra reset on a NEW context stays for now. Dropping it would take one call and
   one reset per call off D's (d) rows (full 4 rows, no-unknown 2), back to the counts before
   86c141a1c; no other row would change.
2. The owner's campaign run: `./run_campaign.sh --suite gate|calib|codec|rpc --out
   ffi/logs/python/campaign` without `--smoke`, with the CPU sets exported.
3. Re-render (`gen/generate.py`) and re-gate (`./gate.sh python3.12 build/py37/python3.7`)
   whenever `plan.py`, `c_abi.py`, a python backend or the core changes. A changed crossing
   count in either build stops the gate until the file in `counts/` is reviewed and replaced.

## Log index

**Results** (correctness, counts, feasibility):

| log | what it establishes |
|---|---|
| `90-wp5-build.log` | the build of both variants at 3.12.3 and 3.7.5, R0, R1, R5 and every build control |
| `91-wp5-conformance-py3.{12,7}.log` | R2 both directions on `_akffi`, layout at import, crossing counts (per element and whole numbers) |
| `92-wp5-conformance-rpc-shim-py3.{12,7}.log` | the same on `_akffi_rpc` |
| `93-wp5-corpus-py3.{12,7}.log` | the whole corpus, 6 arms, controls, decision 11 controls; 3.7 against 3.12 byte for byte |
| `94-wp5-rpc-gate-py3.{12,7}.log` | R-D3 |
| `95-wp5-rd1-lenwrap-py3.{12,7}.log` | R-D1 |
| `96-wp5-u1-py3.{12,7}.log` | U1 |
| `97-wp5-floor-source.log` | 10 variants against 3.7.5 headers and the other header sets; the pre-port control |
| `98-wp5-counts-vs-85.log` | per-element crossing counts against log 85 |
| `100-wp5s10-conformance-nounk-py3.{12,7}.log` | conformance on `_akffi_nounk`, the `_unknown` facade check |
| `101-wp5s10-corpus-nounk-py3.{12,7}.log` | the corpus through the no-unknown build, its controls |
| `102-wp5s10-conformance-rpc-nounk-py3.{12,7}.log` | conformance on `_akffi_rpc_nounk` |
| `103-wp5s10-counts-drop-vs-nounk.log` | whole-number counts of both builds against `counts/`, and their difference |
| `105-wp7-rpc-counts.log` | RPC cells B, C, D, E per call on the rpc counting builds, both builds (req 19) |
| `106-wp13-conformance-rpc-h2batch-py3.{12,7}.log`, `107-wp13-rpc-gate-h2batch-py3.{12,7}.log`, `108-wp13-conformance-rpc-nounk-h2batch-py3.{12,7}.log` | 92, 94 and 102 on the h2-batch rpc cores |
| `109-wp13-rpc-counts-before-replacing.log` | 105 at 08053a6c6, before `counts/rpc-*.txt` were replaced (D's (d) rows +1 reset) |
| `campaign/gate/rpc-tcp-precheck-*.out` | the RPC grid over TCP, both builds and h2 variants: TCP_NODELAY read back, write-count marker |
| `104-wp6-camp-summary-test.log` | camp_summary's two-build test (R-H1) and its twin |
| `85-conformance-rpc-shim.log` | the pre-port shim's crossing counts (the reference for 98) |
| `52-r14-baseline.log` | R14 derived from `Protos/V1` |
| `99-wp5-corpus-chunk256-before-D13.log` | D13 before its fix (a defect record; uncommitted code, stated in its header) |
| `70`, `81`-`89`, `53`, `54`, `01` | superseded or older-commit correctness logs, kept as history |

**Instrumentation** (container timings; not quoted): `campaign/` (the WP6 smoke of both
builds from the clean worktree at 31fc3eecf: gate, codec, rpc, calib; headers marked
INSTRUMENTATION, **figures stripped**: no cpu_ns/wall_ns, no timing column, no pyperf JSON), `00-r13-rust-crossing.log`,
`10`, `20`, `30`/`31`, `40`/`41`, `50`/`51`/`60`/`61`, `55-allocator.log`, `56-concurrency.log`,
`57-gc-bias.log`, `62`/`63`, `86-rpc-smoke-gated.log` (timing rows deleted). The `wall` line of
log 93 is instrumentation. `20` and `40`/`41` come from the retired `mech/gen` codec arms and
cannot be reproduced from the tree.
