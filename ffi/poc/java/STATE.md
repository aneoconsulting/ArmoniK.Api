# java slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only thing that
survives the end of a session. It says what exists and what was checked; it does not say
what a binding should choose (`CLAUDE.md`: the decision is the owner's). History is in
`JOURNAL.md`.

**Phase** (README 1.1): setup and design. **Every timing in this directory and in
`ffi/logs/java/` is container instrumentation**: it shows a harness runs or exposes a
harness defect, and it is not a result. This file quotes no timing figure. What it quotes
is byte identity, crossing counts, floor builds, corpus passes, feasibility and defects.

| | |
|---|---|
| **Status** (2026-10-03) | WP13 (the WP12 contract): every RPC cell on TCP 127.0.0.1 with TCP_NODELAY read back on the live sockets, client CPU from perf task-clock of the whole process (a JVM agent) with the process clock and the CLIENT CPUs' softirq beside it, every pool at `AK_WORKERS` (8), both h2 variants of the RPC cores (stock and h2-batch, `AK_H2`, every RPC sample labelled `h2`), D9's allocator tunables on the RPC forks; rebuilt on the core at 777e02e8b (p1). Gates from one clean worktree at 88fa071e2 (core tree 89bfc215, poc/rust fcb7eeda6), outside smoke, server unpinned: stock GATE PASSED (`logs/java/campaign-wp13/gate-b312f1ba7acf7027.log`) and h2-batch GATE PASSED (`gate-b312f1ba7acf7027-h2b.log`), each with payload, corpus, controls, the four count files identical, the variant's h2 checked in every RPC core, the upload checks over TCP with TCP_NODELAY read back (17/17 and 10/10 sockets), both plants. Minimal smoke (grouped, h2-batch, pinned, full build, 1 launch, 1 round, 20 ms iterations): 289 samples, every one `h2: h2-batch`, `net: tcp`, figures stripped. One runner line changed after 88fa071e2 (the smoke's sample count counted the 17 meta lines too: 306 printed for 289 samples), not re-run. Two builds of the binding (full, no-unknown), the RPC grid on JMH against the shared Rust server, one fork per (cell, combination) by default. |
| **Levels** (owner decision D3) | **floor Java 8** (correctness only): `openjdk 1.8.0_504`, JDK 8 `javac`. **target JDK 17**: `openjdk 17.0.20.1`. JDK 21 only for two secondary probes (virtual threads, FFM preview). |
| **Incumbent** | protobuf-java **3.25.5** (resolved from `packages/java`'s pins), grpc-java 1.74.0, protoc 3.19.0. The production path (R14) is `io.grpc.protobuf.lite.ProtoLiteUtils`' marshaller. |
| **Core** | the shared crate `poc/codec/crates/ak-core` (R0), no copy here. RPC cores in two h2 variants (D11): stock (crates.io h2 0.4.19) and h2-batch (poc/codec/h2-batch/: the full rpc core through its build.sh, the no-unknown and counting rpc cores through the same patched source and `--config patch.crates-io.h2.path`), shims `jnirpc[-h2b][-nounk]`, `jnirpccnt[-h2b][-nounk]`; `build/h2-compiled.txt` names the h2 each RPC core compiled (framed_write.rs's panic path) and the gate requires the variant's. `gen/build.sh` builds it from a `git archive` snapshot of the committed `ffi/poc/codec` into `core-build/<tree key>/`, one target directory per feature set: timed, counting, corpus, rpc, and the same four with `--no-default-features` (the no-unknown build). **Every build carries `init-guard`.** Every shim is checked to link this key's core and the core of its own variant (a full core exports `ak_uencode_*`, a no-unknown core none). |
| **Generator** | `gen/generate.py` is glue: it calls `poc/codec/gen/java_backend.emit` for two descriptions (shapes.json with 7 roots, the corpus reader schema), each from the plan and from the plan relowered with `unknown="drop"`, and renders the harness glue. It writes under `poc/java/` only. `--check`: every generated file current, `poc/codec/gen/generate.py --check --core-only` exit 0, the import guard applied to the 8 Java backend modules and the glue with a planted violation caught. |
| **Blocked on** | nothing for correctness. |

## What exists

| | |
|---|---|
| `poc/codec/gen/java_*.py` | the Java backend, 8 modules: `java_backend` (entry), `java_names`, `java_facade`, `java_rcodec` (arm R, drop and retain), `java_layout` (offsets), `java_jni` (shim and `NativeEntry`, `ak_init`), `java_binding` and `java_pull` (the Java half, per level). Each imports `plan`, never the IR. The C header is `c_abi.emit` of the same plan (the one C header backend), with the Java offsets appended as static assertions. Each module renders the no-unknown variant from `plan.unknown_compiled_out` |
| `gen/` | glue (`generate.py`, `java_build.py`, `java_pbbuild.py`, `java_arms.py`, `pbarms.py`, `java_ffiarms.py`, `java_corpus.py`, `java_walk.py`); `build.sh`; the gate (`gate.sh`, `corpus.py`, `corpus.sh`, each taking `AK_VARIANT=nounk`); the campaign runner (`run_campaign.sh`, `jmh_to_jsonl.py`, `rpc_jmh_to_jsonl.py`, `campaign/counts.ref`, `campaign/counts-nounk.ref`); `layout_break.sh`, `boundary.sh`, `audit_tracked.sh`; older measurement scripts (`bench.sh`, `calibrate.sh`, `contentsets.sh`, `deopt.sh`, `drift.sh`, `floor.sh`, `run_all.sh`) |
| `src/java/ak/` | hand-written runtime (`Enc`, `Dec`, `Utf8`, `Utf8View`, `Str17`, `Mem`, `Arena`, `Native`, `NativeRpc`, ...); gate harnesses `RunConformance`, `RunUnknown`, `RunCounts`, `RunCorpus`, `RunRuleGaps`, `RunUnkControls`, `RunUnkLeak`, `RunUnkOneof`, `RunVariant`; campaign harnesses `CampaignCodec`, `CampaignRpc`, `CampaignCalib`; older timing harnesses `Bench`, `RunDelta`, `RunR14`, `RunBaseline`, `RunNoop` (`RunRpc`, the older RPC harness with its own in-process servers, removed in WP10) |
| `src/jmh/ak/CodecJmh.java`, `src/jmh/ak/RpcJmh.java` | the codec suite and the RPC grid on JMH 1.37 (req 22a as amended 2026-09-27); `gen/jmh_to_jsonl.py` and `gen/rpc_jmh_to_jsonl.py` turn JMH's JSON into the section 7 lines |
| `src/generated/`, `native/generated/` | full build, shapes description: `ak.shapes` (facade, `Codec`, `CodecRetain`, `Layout`, `Binding`, glue), `ak.floor` (arm b), `ak.borrow` (decision 13), per level (`java17`, `java8`); `shared/` (`ak.NativeEntry`, `ak.Variant`); `ak_abi.h`, `shim.c` |
| `src/generated_corpus/`, `native/generated_corpus/` | full build, corpus description: `ak.corpus` (+ `Dispatch`, `Project`, `Walk`), `ak.corpus.borrow`; linked against the `corpus` core |
| `src/generated_nounk/`, `src/generated_corpus_nounk/`, `native/generated_nounk/`, `native/generated_corpus_nounk/` | the no-unknown build, same package names, compiled into its own class trees (`build/cls17-nounk`, `build/cls8-nounk`, `build/jmh17-nounk`), linked to the no-unknown cores (shims `jni-nounk`, `jnicnt-nounk`, `jnicorpus-nounk`, `jnirpc-nounk`). No `CodecRetain`, no `unknownFields` in the facade, no retain path in the binding; `RunUnkControls` and `RunUnkLeak` are not compiled into it |
| `native/rpc.c` | ABI v1 section 9 from the JVM, every struct and prototype from the generated header |
| `native/test/nullpin.*`, `probe/` | R-D9 fault injection; crossing-price, FFM and JNI-accessor probes |

### Arms

| arm | what it is | payload gate | corpus gate |
|---|---|---|---|
| `R` | arm R, unknown fields dropped (`Codec`) | both builds | both builds |
| `R-retain` | arm R, unknown fields retained (`CodecRetain`) | no | full build |
| `ffi`, `ffi-nobatch`, `ffi-zeroed`, `ffi-nobatch-zeroed` | the C ABI through JNI, push family; batching and decision 9's fill as registrations | both builds | `ffi`, both builds |
| `ffi-pull`, `ffi-pull-walk` | ABI v1 7.1's pull family, drained and walked | both builds | both builds |
| `ffi-borrow` | decision 13's borrowed facade | both builds | both builds |
| `ffi-retain`, `ffi-pull-retain` | decision 11 retain: every position armed, u-group encode | full build | full build (+ `ffi-pull-walk-retain`, `ffi-borrow-retain`) |
| `pbj` | protobuf-java | both builds | no (not generated for the corpus) |
| campaign RPC grid (`CampaignRpc` cells, timed by `RpcJmh`) | full build A, B, C/D/E/F-retain and -drop, Bf, Cf/Ef/Cc-retain and -drop; no-unknown build A, B, C/D/E/F-nounk, Bf, Cf/Ef/Cc-nounk | every call checked in the run (req 18); smoke-run | -- |

Decision 11 as implemented: one options struct per root in native memory, every position
armed with the shim's grow, no pre-allocated buffer; a oneof has one options entry, and its
buffer arrives in the active member's decode group (rule 4 as confirmed, ABI-v1.md
2026-09-26); the binding takes every non-NULL delivered buffer or frees it, and after a
failed decode frees what the core had grown (rule 3; D40).

## Campaign readiness (design/CAMPAIGN.md section 10)

The harness: `gen/run_campaign.sh --suite codec|rpc|calib|gate --out <dir>` (default
`ffi/logs/java/campaign/`), over `ak.CodecJmh` on JMH (arms in `ak.CampaignCodec`),
`ak.RpcJmh` on JMH (cells, checks and server in `ak.CampaignRpc`), `ak.CampaignCalib` + `probe/CampaignRev`, and the
gate. Codec and RPC run each build (full, no-unknown) in its own process per launch, the
order of the two builds alternating by launch. Smoke runs: `logs/java/campaign/` and
`logs/java/campaign-wp5s10/`, 1 launch, 1 round, reduced iterations, **every figure
stripped**.

| # | requirement | status |
|---|---|---|
| 1 | one machine, slices sequential | not applicable: the owner's run; the runner runs one suite at a time |
| 2 | governor, turbo, SMT | met as recording (header, from sysfs); setting them is the owner's |
| 3 | isolation, mechanism recorded | met as recording: `AK_ISOLATION` verbatim plus `/sys/devices/system/cpu/isolated` |
| 4 | CPU sets, fixed sizes, worker threads (amended D8, D14) | met: `AK_CPU_CLIENT` / `AK_CPU_SERVER` via `taskset`, from `ffi/campaign.machine` outside smoke (4 cores with both SMT threads each: CLIENT 1-4,11-14, SERVER 5-8,15-18); every pool sized to `AK_WORKERS` (8): the core runtime (`ak_runtime_new`), the Netty event loop group, grpc-java's call executor (a fixed pool in place of its default cached one), G1's `ParallelGCThreads`, the server's tokio runtime (`AK_SERVER_THREADS`); ConcGCThreads and CICompilerCount are the JVM's own, recorded in each header with the GC and JIT thread counts; no grpc-core in this slice; every RPC log states the pools and each fork's `RPCJMH-CELL` line carries them |
| 5 | floors gated, not timed | met: the gate runs arms b and c (java8 tree on JDK 17 and JDK 8) and the corpus on JDK 8, for both builds; no `Campaign*` class is in the floor build |
| 6 | build flags printed | met: core snapshot commit and tree key, cargo features (`init-guard`), cdylib, shim `-O2`, JVM flags |
| 7 | payloads, content sets, U-* rows (amended 2026-09-26) | met: 16 payloads x 3 content sets (P7.1 decode-only, ASCII); Latin-1 and wide on P1.2, P2.2, P2.4 are `row_class: required`, on the other payloads `extra`; the 92 accepted `U-*` rows at the shapes core's 7 ABI roots in encode, decode and decode-read through the timed shapes core (`content: corpus`, `core: shapes`, required); every other non-disputed `U-*` row through the corpus description and core, a labelled extra (`core: corpus`) |
| 8 | arms | met: incumbent-prod (`ProtoLiteUtils` marshaller), incumbent-best (`toByteArray` / `parseFrom(byte[])`), core-ffi, core-ffi-pull (labelled extra), host-gen (arm R) |
| 9 | encode, decode, decode + read | met: `encode`, `decode`, `decode-read` (generated `Walk` / `PbWalk`) |
| 10 | retain, drop, no-unknown | met: full build `core-ffi` drop and retain, `core-ffi-pull` drop, `host-gen` drop and retain; no-unknown build `core-ffi`, `core-ffi-pull`, `host-gen` in `no-unknown` (host-gen is generated from the plan: arm R in drop mode over a facade with no `unknownFields`), the incumbent arms beside them (JMH forks one JVM per cell, so no two arms share a process; the incumbent arms are a control across the two builds' invocations, not an in-process one); `unknown_mode` and `build` on every sample; own counts file and own gate (below) |
| 11 | serialise once per iteration; encode variants (amended 2026-09-26) | met: graphs built in JMH's untimed `@Setup(Level.Iteration)`; four labelled encode rows per (arm, payload, set): end state `buf` (bytes in a reused buffer: incumbent-prod `stream()` drained into the reused sink, incumbent-best `writeTo` a CodedOutputStream over it, core-ffi `ak_enc_take` into it, host-gen a copy into it) or `transport` (incumbent-prod `stream()` drained into a fresh exact-size array, as grpc-java's framer drains into buffers it allocates; incumbent-best `toByteArray()`; core-ffi `take()`, host-gen `Enc.toBytes()`, the arrays cells C to F hand their transport), input `hot` (one graph re-encoded) or `pool` (distinct graphs whose serialised bytes exceed 2 x `AK_LLC_BYTES`, default 13.75 MB) |
| 12 | cells A-F, C to F per mode | met: full build A, B, C-retain, C-drop, D-retain, D-drop, E-retain, E-drop, F-retain, F-drop; no-unknown build A, B, C-nounk, D-nounk, E-nounk, F-nounk, one JMH fork (one JVM, one channel or client) per cell per build and transport; the framed twins Bf, Cf-*, Ef-* (the core's second send path, ak_client_set_framed) in every direction, labelled `send_path`; grpc-java has no second send path. Cell B parses the core's response in place (direct ByteBuffer); C and E copy it into a Java array (C's binding once more into native scratch). Cell C (and Cf) sends its request on the move path: ak_call_unary_enc in b and c, ak_call_send_enc in d (the encode context's output moved, no Java array), as the rust, cpp and python slices do; Cc-* is C on the copy path (take() + ak_call_unary), a labelled extra; every sample's `send_path` names it. Cell D keeps take(): grpc-java's send path copies every message through an OutputStream into buffers it allocates, so an owned native buffer from ak_enc_take_owned would still be copied (through a heap array), stated. Each C to F cell re-encodes P2.2 in its mode byte-identical before timing; the leak counters must read 0 after the run |
| 13 | server separate, pre-serialised, one per launch (amended 2026-09-26) | met (as amended at 9f6d579fa, WP10): the server is the Rust slice's tonic `rpc_server`, the one RPC server of every slice (interface `poc/rust/SERVER.md`), ONE process per launch started by the runner through `poc/rust/serve.sh` (build, start, warm, stop; its state file in the run's own directory) pinned to `AK_CPU_SERVER`, serving every cell of both builds on its two Unix sockets; service `armonik.ffi.campaign.v1.Grid`: Fetch (a, P2.2 pre-serialised once), Push (b) and Upload (c) decoded with prost and answered empty, UploadStream (d) every message decoded, the byte count answered, UploadStreamCheck (the upload check, count and SHA-256); warmed by `serve.sh warm` before JMH (row 24); `CampaignRpc --serve` and its server code are removed; every cell opens its own channel (grpc-java) or client (the core) in its fork's `@Setup(Level.Trial)`, one per cell per fork |
| 14 | directions a, a+read, b, c, d (amended 2026-09-27) | met: `a`, `a+read`, `b` at 1/8/16 in flight; `c` a unary upload of P5.3 / P5.4 (the payload builders' M5, checked against the manifest), the server decodes it with prost and answers empty; `d` the client-streamed upload of 4 MiB and 16 MiB in 2 MiB M5 chunks (splitmix64 data as the rust slice's, ids on the first message), the server decodes every message and answers the byte count (and SHA-256 on the check path), the ids on the first message required (SERVER.md); c and d at 1 and 8 in flight, every cell and framed twin, each (direction, payload, in-flight) combination one JMH iteration of the same duration as a, a+read and b's. B, E send protobuf-java's / arm R's bytes with ak_call_send, C the binding's encode with ak_call_send_enc; A, D, F grpc-java's `asyncClientStreamingCall` with a StreamObserver |
| 15 | 1 / 8 / 16 in flight | met: one JMH invocation = one batch of k calls in flight (call 0 on JMH's benchmark thread, 1..k-1 on persistent helper threads released per batch), counted as k calls (`iters`); the hand-written sampler ran `calls / k` calls per thread in chunks, so a sample there was not one batch |
| 16 | delivery | met: B, C, E the core's blocking call, and its blocking client stream in d; A, D, F `ClientCalls.blockingUnaryCall` (a blocking stub's call; packages/java's clients use blocking stubs) and, in d, `ClientCalls.asyncClientStreamingCall` (client streaming has no blocking stub: the async stub's form), stated in every RPC header; the incumbent cells' MethodDescriptors are built in `CampaignRpc` with the full method names of SERVER.md's service and `ProtoLiteUtils` marshallers over the protobuf-java classes generated from shapes.json (the same messages as SERVER.md), which is what a protoc-gen-grpc-java stub builds and hands to `ClientCalls`; no service stub is generated; no queue delivery (`RunRpc`, which had it, is removed) |
| 17 | shipped and pinned, TCP (amended D10) | met: TCP 127.0.0.1 for every cell (the shared server started with `AK_SERVER_TCP=0`); Nagle off on every client socket: grpc-java over Netty epoll with `ChannelOption.TCP_NODELAY` true, the core with tonic's default nodelay (shipped, no options) or `tcp_nagle = 0` (pinned); read back with `getsockopt(TCP_NODELAY)` on every live socket of the process connected to the server's port (`Native.tcpNodelay`, native/tax.c), in every fork before timing and in the gate's upload checks (17/17 sockets), a fork with one socket without it refuses to run; the server's TCP listener runs the pinned server configuration only, so shipped and pinned differ on the client side only: shipped = grpc-java's and tonic's client defaults, pinned = 4 MiB windows (BDP / adaptive window off), 8 MiB messages. A Unix socket path is still accepted by the cells, used by no runner path |
| 18 | every call checked, abort | met: exception, non-OK status (AK_ERR_RPC_STATUS from the core, a failed status from grpc-java) or a wrong response length or upload count fails the call; under JMH a failed check throws (from a helper thread too), JMH records it and `-foe true` stops the run; the runner then deletes the launch's JSON lines, JMH JSON and JMH output and keeps only `rpc-launch-<l>.FAILED.txt` (the error lines); the gate's upload check runs every cell's c and d (d through the SHA-256 path) on both sockets and builds, and a planted wrong expectation (`ak.camp.plant=1`) must abort, both in the upload check and through JMH (`rpc JMH plant`: JMH exit non-zero, its JSON holds no measurement) |
| 19 | crossing counts gate | met for both builds: `counts.ref` / `counts-nounk.ref` (per-payload rows and `EP` rows: every entry point per operation, resets included and placed -- encode `ak_enc_reset` before each encode; decode drop none; retain one `ak_dec_reset_<Root>(opts)` before each decode (WP8, rules 1 and 7); pull none in drop, the same one in retain -- per payload and the 92 shapes-root U-* rows, retain with no pre-placed buffer and the timed build's geometric grow); `rpc-counts.ref` / `rpc-counts-nounk.ref` (B, C, D, E and the framed twins per call in a, b, c/P5.3, c/P5.4, d/4MiB, d/16MiB); all four diffed by the gate |
| 20 | crossing cost, fwd and rev, perf stat | **not met**: forward and reverse are sampled (the core's shim, JNI forward, JNI upcall, the rust slice's bench beside them), and the runner wraps each in `perf stat` when `perf` exists; `perf` is absent in this container, so that path has never run and cycles / instructions are unverified. Needs `perf` on the campaign machine, no owner decision |
| 21 | CPU time (amended 2026-10-01) | met: codec -- the benchmark method reads `CLOCK_PROCESS_CPUTIME_ID` per JMH iteration; RPC -- `cpu_ns` is perf task-clock of the whole client process: a JVM agent (native/taskclock.c, `-agentpath`) opens one task-clock counter with inherit on the JVM's main thread in `Agent_OnLoad`, before the JVM creates its other threads, so every later thread is an inherited child the kernel sums into it; read around every invocation; `process_cpu_ns` (CLOCK_PROCESS_CPUTIME_ID) beside it; `softirq_ticks_client`, /proc/stat softirq on the CLIENT CPUs per iteration (USER_HZ ticks); a fork without the counter refuses to run; calib -- the process clock |
| 22 | order (amended 2026-09-26) | met, order stated (R-H23): JMH runs cells in the order given and cannot randomise across forks; the codec arm order inside each (payload, content, dir) block is rotated one step per launch; RPC -- campaign default one JMH fork per (cell, combination) (`combo` @Param), JMH's order of the cross product, cells rotated one step per launch; grouped (`AK_RPC_GROUP=1`, smoke default) one fork per cell, JMH iteration i running combination (i + launch - 1) mod 17; the grouping is stated in each header; the two builds alternate by launch |
| 22a | codec suite and RPC grid on the ecosystem's framework (amended 2026-09-27) | met: JMH 1.37 for both, `-f 1` (one JVM per cell), `-foe true`, Blackhole, every raw measurement iteration exported from JMH's JSON (primary rawData and the `@AuxCounters` rawData); codec SingleShotTime, one iteration = one sample; RPC AverageTime, one invocation = one batch of k calls, one fork per (cell, combination) by default (owner, e6c909630), grouped per cell only under `AK_RPC_GROUP=1` (smoke, exploration); the custom pieces and the requirement each serves are listed below this table; calib stays on the runner |
| 23 | >= 5 rounds, 3 launches | met: `AK_ROUNDS` 5, `AK_LAUNCHES` 3, every round committed raw |
| 24 | warm-up stated, both coder states | met: JMH warm-up iterations per cell in the cell's own fork, recorded per cell; every codec sample carries `jit_ms_during`, the JIT compile time spent during that iteration (0 = no compilation while it ran; R-H19); the JIT is HotSpot's tiered default, and which tier each method reached is not recorded (no WhiteBox API in a product JVM); RPC: the server warmed before JMH by `poc/rust/serve.sh warm AK_RPC_SERVER_WARM` (default 200, smoke 20: on each socket N checked Fetch, Push, Upload and ceil(N/4) UploadStream calls from a tonic client and from the core's client; no JIT on the server), each fork JMH's warm-up of `AK_WARM` x 17 iterations (default 2, smoke 1) of `AK_RPC_WARM_TIME` (1s, smoke 20ms), measurement `AK_ROUNDS` x 17 iterations of `AK_RPC_ITER_TIME` (1s, smoke 20ms); GC and JIT between iterations JMH's defaults; the RPC grid records no per-iteration JIT figure; codec runs with compact strings and with `-XX:-CompactStrings` |
| 25 | allocator, GC (amended D9) | met: same warm-up for every arm, heap fixed at 4 GB, G1, ParallelGCThreads = AK_WORKERS. D9 applies to this slice's native side: the core and its transport (tonic, h2 and their buffers) allocate through glibc malloc inside the client JVM, as in a native slice, so every RPC client fork runs under `GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432` (inherited by JMH's forks), stated in each header, and `AK_D9_DEFAULT_ALLOC=1` is the default-allocator pass; Netty's pooled direct buffers and the Java heap are not glibc malloc; the codec suite runs with the default allocator |
| 26 | gate before timing | met: a timing suite runs only with a passed gate stamp covering both builds |
| 27 | header | met; a dirty `poc/java` is refused (poc/codec, schema and corpus enter only through the `git archive` snapshot) |
| 28 | JSON lines body | met, plus `coder`, `engine`, `build`, `codec`, `delivery`, `end`, `input`, `row_class`, `core`, `jit_ms_during` and `{"meta":...}` lines (RPC: JMH's mode, iterations and times, JVM arguments, threads, order, ratio basis); both suites converted from JMH's JSON (`gen/jmh_to_jsonl.py`, `gen/rpc_jmh_to_jsonl.py`), RPC labels (mode, codec, send path, delivery, transport, build, per-iteration combination) from the fork's `RPCJMH-CELL` / `RPCJMH-ITER` lines, a measurement without its label or a label without its measurement refused; `wall_ns` = JMH's per-iteration score x invocations, checked against JMH's per-call score (the `calls` OPERATIONS counter) |
| 29 | logs under ffi/logs/java/campaign/ | met: the runner's default `--out` |
| 30 | summaries | not applicable: optional, not produced; the raw lines carry what section 8 needs. A ratio formed from them uses per-launch medians (owner R-H24), cross-process for the codec suite |
| 31 | runner interface | met for the slice runner; `ffi/campaign.sh` is the aggregating session's |
| 32 | smoke run, readiness recorded | met: `logs/java/campaign-wp9/` (WP9, owner direction 2026-09-27 "small tests only"): rpc `AK_CAMPAIGN_SMOKE=1`, 1 launch, 1 round, `AK_SMOKE_WARM` 1 (17 warm-up iterations per fork), 20 ms iterations, server warm-up 20 calls per combination, 2 GB heap, both transports and both builds: 289 samples per transport in the full build (17 cells x 17 combinations) and 170 in the no-unknown build (10 x 17); codec `AK_CAMPAIGN_SMOKE=1` with `AK_CODEC_PROPS="-Dak.camp.ids=P2.2 -Dak.camp.sets=0"` and `AK_SMOKE_UROWS=1` (P2.2 ASCII and one U-* row, both coder states, both builds: 58 / 58 / 14 and 40 / 40 / 8 samples); figures stripped. Calib last smoke-run in `campaign-wp7/`. The one requirement not met is 20 |

### Custom code around JMH, and the requirement each serves (req 22a as amended)

JMH's own mechanisms, used as they are, in both suites: process isolation (`-f 1`, one JVM
per `cell` value), warm-up and measurement iterations (`-wi`, `-i`, and for RPC `-w`, `-r`),
the invocation loop and its wall clock, untimed `@Setup` at trial, iteration and invocation
level, the Blackhole, stop on first error (`-foe true`), and the raw export (`-rf json`:
the primary score and every `@AuxCounters` counter per measurement iteration). No hand-written
warm-up loop, sampler, settle wait or wall clock remains in either suite.

| piece | suite | requirement | why JMH's own does not serve |
|---|---|---|---|
| process CPU read around the timed work, into an `@AuxCounters` counter | codec, RPC | 21 | JMH measures wall only |
| JIT compile time between an iteration's setup and teardown (`jitMs` counter) | codec | 24 (managed clause) | JMH records no compilation state per iteration |
| `iters` operations per SingleShotTime invocation, from the cell's wire size | codec | 11 (graphs prepared untimed, pool larger than the LLC) | the prepared pool fixes the operation count before the timed call; JMH's `-bs` is one value for all cells |
| cell list from `ak.CampaignCodec`, arm order rotated per launch | codec | 22 | JMH runs `-p` values in the order given and cannot randomise across forks |
| correctness check in `@Setup(Level.Trial)` | codec, RPC | 18, 26 | JMH checks nothing; a throw there with `-foe true` stops the run before any figure |
| starting, warming and stopping the shared server (`poc/rust/serve.sh`) around JMH | RPC | 13, 24 | JMH runs benchmarks, not a separate peer process |
| the 17-combination cycle inside one fork, behind `AK_RPC_GROUP=1` (default on under smoke only) | RPC | smoke and small exploration runs (req 22a as decided e6c909630) | the campaign default is JMH's own isolation: `-p combo=<17 ids>`, one fork per (cell, combination) |
| k-1 helper threads per batch | RPC | 14, 15 | JMH's `-t` threads each run whole invocations on their own; they cannot put the k calls of one batch in flight together |
| perf task-clock JVM agent (`-agentpath:build/taskclock/libaktc.so`) read around each invocation (`rpcTaskClockNs`), softirq on the CLIENT CPUs per iteration (`softirqTicks`) | RPC | 21 as amended | JMH has no perf counter of the whole process (its `-prof perfnorm` attaches perf stat from outside, which req 21 says inflates cells by their context switches, and reports per benchmark, not per iteration) |
| TCP_NODELAY read-back on the live sockets (`Native.tcpNodelay`) | RPC | 17 as amended | not a framework concern |
| `CampaignRpc.THROW_ON_FAIL` | RPC | 18 | the old path exited the JVM; a throw lets JMH record the error and stop |
| discard of the launch's output on failure | RPC | 18, 22a | JMH writes what it measured before the failure to its text output |
| labels: `RPCJMH-CELL` / `RPCJMH-ITER` lines and the converters | codec, RPC | 28 (section 7) | JMH's JSON carries parameters and numbers, not the section 7 labels |
| leak counters must read 0 at `@TearDown(Level.Trial)` | RPC | decision 11 rule 3 | not a timing concern |

Differences JMH forces against the hand-written sampler: one process per cell (the sampler
held every cell of a build and transport in one process, so cells A and B are no longer an
in-process control for C to F: the comparison is across forks, as in the codec suite); one
invocation is one batch of k calls, not `calls / k` calls per thread; an iteration lasts a
time (`-r`) rather than a call count, for c and d as for a, a+read and b; JMH's own summary
score per benchmark averages unlike combinations and is not a figure (the raw per-iteration
data is).

## Correctness (results)

**Clean gate, both builds** (`logs/java/wp6-h/gate-3f17fcaaba99996f.log`, `counts-8339265cb.txt`, `counts-nounk-8339265cb.txt`, `build-8339265cb.log`; the previous one, at d2cd0b02f, is `logs/java/wp6-clean/`): a fresh
git worktree at origin HEAD, no uncommitted changes, no reused build directory; header
shows a clean commit. Contents per build:

- **full build**: payload gate on arms a, b, c (every registered arm listed with its row
  count), 66 unknown-field vectors, pull reverse crossings 0; corpus, 10 arms x 702 rows on
  8 and 17; planted controls (proj, reenc, accept, noinit) each seen failing; decision 11
  controls (per-position discard, pull == push, wrong root; planted mask-ignored seen
  failing); leak control on the four retain arms on 8 and 17 (planted no-reclaim seen
  failing); rule gaps; counts identical to `counts.ref`.
- **no-unknown build**: payload gate on arms a, b, c (no retain arm), 66 unknown-field
  vectors; layout facts 240 (shapes) and 340 (corpus) agree with the variant core's
  `ak_layout_facts`; a full tree on the no-unknown core and a no-unknown tree on the full
  core both refused at load; corpus, 5 arms x 702 rows on 8 and 17, every unknown row read
  and written in the dropped form; the same planted controls seen failing; rule 6 on the
  variant's root-bound contexts; counts identical to `counts-nounk.ref`.

Result, at commit 8339265cb (the core with the register-H changes of 31fc3eecf): **GATE PASSED** for both builds; the oneof control and its plant (full build, 8 and 17) included. Payload gate 1,802 checks per
arm set (full build, retain arms included) and 1,389 (no-unknown build), 0 failures on arms
a, b and c; corpus 0 failing arm-rows on 8 and 17 in both builds (R and R-retain pass 696 of
702, the ffi arms 680, with 6 disputed rows excluded and 16 `Nest` rows outside the C ABI);
planted controls: proj 260 / 130, reenc 260 / 130, accept 950 / 475, noinit 104 failing
arm-rows (full / no-unknown); both counts files identical to their references.

**Rule gaps outside the corpus** (`RunRuleGaps`, in the corpus gate, arm R and ffi): a
repeated singular message and a repeated oneof message member merge (as protobuf C++ reads
the same bytes, shown in the log); `body_case = 99` is refused; `-0.0` is written as
`21 0000000000000080` and `+0.0` is not; `0a ffffffff07` is refused at the length.

**Oneof control** (R-H8, `RunUnkOneof`, corpus gate section 3b', 8 and 17): the corpus has
no row with unknowns inside a oneof member, so the C++ slice's three sequences
(stamp -> nothing, stamp -> nothing -> stamp, stamp -> as_int) run through ffi-retain,
ffi-pull-retain and ffi-pull-walk-retain: final case and bag exact, the buffer left in an
inactive member's slot freed (`unkLeftAfterSuccess` 0, 0 buffers alive); the planted unarmed
decode fails.

**Leak control** (D40, `RunUnkLeak`): the corpus's refused rows are refused before any
unknown field is taken (0 buffers reclaimed over them), so the control also decodes derived
rows (each unknown-class accept row with an invalid tag appended and truncated at every
length); 0 buffers alive after every row on every retain arm.

## Crossing counts (results) -- `gen/campaign/counts.ref`, `gen/campaign/counts-nounk.ref`

From the counting core; machine-independent. Selected rows of the full build:

| payload | encode fwd / rev | decode (push) fwd / rev |
|---|---|---|
| P1.2 (1,000 M1 rows) | 8 / 1 | 1 / 8 |
| P2.2 (500 M2 elements) | 2,511 / 2,501 | 1 / 3,501 |
| P2.3 | 629 / 626 | 1 / 876 |
| P2.4 | 403 / 401 | 1 / 561 |

Full build against no-unknown build: P1.2 decode reverse 8 -> 5 (batched and unbatched), no
other push count moves; in the pull section the drain needs fewer 32 KB chunks (P1.2 9 -> 6,
P2.2 23 -> 16, P2.3 12 -> 11, P2.4 14 -> 13, P4.1 6 -> 5) and the record footprint is
smaller on every payload (each decode group is 16 bytes smaller). The pull family: reverse
0 on every payload in both builds; forward 2 for the walk delivery at any size.

## Feasibility findings and defects found

- **Critical sections**: holding `GetPrimitiveArrayCritical` across a blocking RPC call
  whose completion needs another Java thread (an in-process grpc-java server) deadlocked;
  the RPC paths copy instead. The pull parse and bulk-bytes pins make no upcall and
  complete on their own (JOURNAL).
- **Packaging (README 5.1.3)**: floor and target are different code because the target
  reads `String.coder`/`String.value`, which Java 8 lacks. Both levels use
  `sun.misc.Unsafe` (terminally deprecated from JDK 23, FFM its successor). `--release 8` on
  JDK 17 cannot build the floor (`ct.sym` has no `sun.misc.Unsafe`), so JDK 8's `javac`
  builds it.
- **Two builds cannot share a process**: each shim loads a core by the same soname, so the
  no-unknown build is a separate class tree and a separate process; comparisons across the
  two builds carry cells A and B (RPC) or the incumbent arms (codec) beside them, each in its
  own JMH fork, so a control across processes.
- **Virtual threads** (`pinning.log`, JDK 21, instrumentation): blocking in a native frame
  pins the carrier and parking on a future does not.
- **R9's JIT effect** (`r9-mechanism.log`, instrumentation): C2 prunes
  `StringUTF16.charAt` from protobuf-java's encoder in some runs and not others; a harness
  hazard on wide content, which is why the codec suite runs both coder states.
- Defects found and fixed in this slice, with their logs: section 8's direct path never
  ran, the zeroed fill dropped -0.0, `Utf8View` refused ASCII after a multi-byte character
  (WP5 step 3); `ak_init` never called (R-G7); the Java 8 floor build broken by a Java 9
  API; R-D9 (NULL pin unchecked); `rpc.c` hand-declared the RPC structs (R-D2); D38 (group
  skip accepted field numbers above 2^29-1); D39 (stale core over a reused target dir); D40
  (failed retain decode leaked buffers); D41 (a control's exit status masked by a pipe); the
  RPC client kept every sample thread's Binding to process end. Details in JOURNAL.

## Open defects and gaps

| # | what | status |
|---|---|---|
| G4 | a `lossy` UTF-8 plan option raises in `java_rcodec` (the runtime renders `reject` only) | open until the option is used |
| G6 | at be47d7614 a gate outside smoke pinned the server to `ffi/campaign.machine`'s CPUs, which this container lacks; the gate's server is now unpinned (correctness only); verified by the gate at d9467f77f, run outside smoke with `AK_CPU_SERVER=5-8` from campaign.machine: GATE PASSED | closed |
| G5 | in the 860cbd92d smoke the server warm-up's lines were overwritten in the server's log; fixed (the warm-up writes `rpc-server-warm-launch-<l>.txt`); the WP10 smoke at be47d7614 has that file with both sockets' warm-up lines | closed |

No other defect known to this slice is open. Closed items are in JOURNAL (J2-J28).

## What is not measured or not run

- **Any performance result.** Every timing is container instrumentation.
- **A JIT figure per RPC iteration**: the codec suite records `jit_ms_during`, the RPC grid does not.
- **The JMH host JVM's CPU**: it runs under the same `taskset` as its fork (`CLIENT`) and is mostly idle; the fork's process CPU does not include it.
- **`perf stat`** cycles and instructions (req 20): `perf` absent here.
- **The RPC queue and callback deliveries** as campaign rows; **TLS, retry, metadata, deadlines, numeric status, streaming** (owner
  position 3).
- **Decision 11's pre-allocated buffers and pools**: every retained buffer comes from the
  shim's grow; `ak_unk_pool` entries hold no buffer.
- **A failure inside the shim's grow** (realloc returning NULL): not exercised.
- **Corpus C5 (produce)**: the transcode encode half (`T-enc-*`) is consumed, not
  produced; `ak.Utf8`'s U+FFFD against protobuf-java's `?` for an unpaired surrogate is
  written down, reached by no vector.
- **Chunk counts per corpus `C-*` row**: the rows pass; how many chunks each crossed is not
  counted.
- **protobuf-java against the corpus**: not generated for it; only its payload bytes are
  gated.
- **Concurrency** (ABI v1 obligation 12.5): one thread in every gate; the shim's
  thread-local frame stack is argued, not tested.
- **Allocation and footprint**, **message size limits**, **the codec's rollback of a
  half-written field**, **whether a thread parked in a drain costs a collection nothing**.
- **FFM as a binding** and any FFM upcall; **the C-shim arm of README 9.1** (probes only).
- **What moves arm R under the R9 probe**: unattributed.
- **Rust MSRV 1.88** for the core: not verified here (rustc 1.94.1).

## Next step

1. **Campaign (W13), when the owner runs it**: `AK_CPU_CLIENT=.. AK_CPU_SERVER=..
   AK_ISOLATION=.. gen/run_campaign.sh --suite gate|codec|rpc|calib`.

## Requests to the aggregating session

1. The corpus has no vector for a repeated singular message field or a repeated oneof
   message member (merge; `RunRuleGaps` covers both outside the corpus), and the payload
   gate's content sets never mix ASCII with multi-byte characters in one string.
2. `ak_bdr_count_forward`'s description ("the host reports the forward crossings of a drain
   loop") can be read as "call it after a drain", but `ak_bdr_drain` is itself an entry
   point that bumps `forward`; the Java shim does not call it.
3. `logs/java/rpc.log` is deleted in this unit (R-C9: a curated RPC grid summary with no raw
   runner output). `README.md:1191` and `findings/java.md:48,105` still cite it.

## Log index

**Results** (correctness, counts, feasibility):

| Log | What it establishes |
|---|---|
| `campaign-wp13/` | WP13: the two gates (stock, h2-batch) from a clean worktree at 88fa071e2 (TCP, TCP_NODELAY read back, h2 per core, counts, upload checks, plants; the upload-check and count files are the h2-batch gate's, which ran second), the server logs, and the minimal grouped rpc smoke on h2-batch (its header and a per-cell summary, `rpc-pinned-h2b-launch-1.summary.txt`; the raw .jsonl is not committed, logs/PURGED.md). A first attempt at 1fc059f8b/2f9ce4823 failed: its worktree was deleted mid-build by another session sharing the scratchpad name, and its gate's upload check split the `tcp:` address on `:` (fixed in 88fa071e2) |
| `campaign-wp9g/` | req 22a grouping switch: the gate from a clean worktree at d9467f77f (outside smoke, which re-checks G6), and the grouped rpc smoke (pinned, full build); figures stripped |
| `campaign-wp10/` | WP10: the gate for both builds from a clean worktree at be47d7614 against the shared Rust server (the four count files, the upload checks on both sockets, upload plant, rpc JMH plant), the server logs (`gate-rpc-server/`, `rpc-server-launch-1/`), `serve.sh warm` output, and the minimal rpc smoke (pinned, full build); figures stripped |
| `campaign-wp9/` | WP9: the gate for both builds from a clean worktree at 860cbd92d (the four count files identical, upload checks, upload plant, rpc JMH plant `rpc-jmh-plant.txt`), the rpc smoke on JMH (both transports and builds) and the reduced codec smoke; figures stripped; JMH's own JSON and text output (which carry figures) are not committed |
| `campaign-wp8b/` | WP8 with cell C on the move path: gate for both builds from a clean worktree at a2c38db01 (counts, rpc counts with Cc rows, upload checks, plant), and the rpc and codec smoke; figures stripped |
| `campaign-wp8/` | WP8: the gate for both builds from a clean worktree at a1eed5321 (the merged core), the four count files, the upload checks and plant; the rpc (directions a to d, framed twins) and codec smoke of both builds; figures stripped |
| `campaign-wp7/` | WP7: the gate for both builds from a clean worktree at f5cba4acf (payload, corpus, controls, the four crossing-count files: counts, counts-nounk, rpc-counts, rpc-counts-nounk), and the smoke of codec, rpc and calib for both builds; figures stripped |
| `wp6-h/` | register H: the gate for both builds from a clean worktree at 8339265cb, with the oneof control (R-H8) |
| `wp6-clean/` | WP6 step 1: the gate for both builds from a clean worktree at origin HEAD (header shows a clean commit), both counts files, the build log |
| `campaign-wp5s10/` | WP5 step 10 smoke: gate logs of both builds, both counts files, codec and RPC for both builds; figures stripped |
| `campaign/` | WP3 and req 12 smoke runs (codec, U-rows, rpc six cells, calib, gates); figures stripped |
| `wp5s9-leak/gate.log` | D40: the leak control's first gate |
| `wp5s9/gate.log` | WP5 step 9 (decision 11) gate |
| `wp5s6-*.log` | WP5 tail: D38 probe manifest (`wp5s6-probe.log`), D39 keyed target dirs (`wp5s6-d39-keys.log`), gate, corpus, build |
| `wp5-*.log` | WP5 step 3: gate, corpus, counts, layout guard failing at load and at compile (`wp5-layout-guard.log`), boundary, R-E4 closure, build |
| `rd9-jni-null.log` | R-D9 fault injection: the old shim entering the core with `(NULL, 65536)`, the new one refusing |
| `rd5-*.log`, `re4-corpus-armR.log`, `counts.log`, `unknown.log`, `conformance.log`, `boundary.log`, `layout-guard.log` | earlier gate runs, superseded by the ones above |
| `flow-control.log` | grpc-java 1.74.0's flow-control window, read from its bytecode |

**Instrumentation** (container timings, not results, no figure quoted here): `encode.log`,
`encode-take-fix.log`, `decode.log`, `decode-pull.log`, `delta.log`, `drift.log`,
`floor.log`, `contentsets.log`, `deopt.log`, `r9-mechanism.log`, `crossing.log`,
`calibration-r13.log`, `baseline.log`, `ffm.log`, `shim-probe.log`, `pinning.log`,
`r14.log`, `r14-summary.md` (curated, its raw table is `r14.log`), `w10-regate.log`. Known
issues the campaign harness replaces: `Bench` times against `toByteArray` rather than the
marshaller (R-C10, the campaign's incumbent-prod is the marshaller); wide-content encode
subject to R9 (R-C12, the campaign runs both coder states).
