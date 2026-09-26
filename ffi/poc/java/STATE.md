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
| **Status** (2026-09-26) | Two builds of the binding exist and are gated: the **full build** (decision 11: unknown fields retained or dropped per call) and the **no-unknown build** (unknown-field support compiled out of the core, the binding and the facade). The harness is on the 2026-09-26 contract (FIX-PLAN WP7). Gate for both builds on Java 8 and 17 from a clean worktree at f5cba4acf, all four crossing-count files identical (`logs/java/campaign-wp7/gate-24d36198b0436a0a.log`); smoke of codec, rpc and calib for both builds with every new row, figures stripped (`logs/java/campaign-wp7/`). |
| **Levels** (owner decision D3) | **floor Java 8** (correctness only): `openjdk 1.8.0_504`, JDK 8 `javac`. **target JDK 17**: `openjdk 17.0.20.1`. JDK 21 only for two secondary probes (virtual threads, FFM preview). |
| **Incumbent** | protobuf-java **3.25.5** (resolved from `packages/java`'s pins), grpc-java 1.74.0, protoc 3.19.0. The production path (R14) is `io.grpc.protobuf.lite.ProtoLiteUtils`' marshaller. |
| **Core** | the shared crate `poc/codec/crates/ak-core` (R0), no copy here. `gen/build.sh` builds it from a `git archive` snapshot of the committed `ffi/poc/codec` into `core-build/<tree key>/`, one target directory per feature set: timed, counting, corpus, rpc, and the same four with `--no-default-features` (the no-unknown build). **Every build carries `init-guard`.** Every shim is checked to link this key's core and the core of its own variant (a full core exports `ak_uencode_*`, a no-unknown core none). |
| **Generator** | `gen/generate.py` is glue: it calls `poc/codec/gen/java_backend.emit` for two descriptions (shapes.json with 7 roots, the corpus reader schema), each from the plan and from the plan relowered with `unknown="drop"`, and renders the harness glue. It writes under `poc/java/` only. `--check`: every generated file current, `poc/codec/gen/generate.py --check --core-only` exit 0, the import guard applied to the 8 Java backend modules and the glue with a planted violation caught. |
| **Blocked on** | nothing for correctness. |

## What exists

| | |
|---|---|
| `poc/codec/gen/java_*.py` | the Java backend, 8 modules: `java_backend` (entry), `java_names`, `java_facade`, `java_rcodec` (arm R, drop and retain), `java_layout` (offsets), `java_jni` (shim and `NativeEntry`, `ak_init`), `java_binding` and `java_pull` (the Java half, per level). Each imports `plan`, never the IR. The C header is `c_abi.emit` of the same plan (the one C header backend), with the Java offsets appended as static assertions. Each module renders the no-unknown variant from `plan.unknown_compiled_out` |
| `gen/` | glue (`generate.py`, `java_build.py`, `java_pbbuild.py`, `java_arms.py`, `pbarms.py`, `java_ffiarms.py`, `java_corpus.py`, `java_walk.py`); `build.sh`; the gate (`gate.sh`, `corpus.py`, `corpus.sh`, each taking `AK_VARIANT=nounk`); the campaign runner (`run_campaign.sh`, `jmh_to_jsonl.py`, `campaign/counts.ref`, `campaign/counts-nounk.ref`); `layout_break.sh`, `boundary.sh`, `audit_tracked.sh`; older measurement scripts (`bench.sh`, `calibrate.sh`, `contentsets.sh`, `deopt.sh`, `drift.sh`, `floor.sh`, `run_all.sh`) |
| `src/java/ak/` | hand-written runtime (`Enc`, `Dec`, `Utf8`, `Utf8View`, `Str17`, `Mem`, `Arena`, `Native`, `NativeRpc`, ...); gate harnesses `RunConformance`, `RunUnknown`, `RunCounts`, `RunCorpus`, `RunRuleGaps`, `RunUnkControls`, `RunUnkLeak`, `RunUnkOneof`, `RunVariant`; campaign harnesses `CampaignCodec`, `CampaignRpc`, `CampaignCalib`; older timing harnesses `Bench`, `RunDelta`, `RunR14`, `RunBaseline`, `RunRpc`, `RunNoop` |
| `src/jmh/ak/CodecJmh.java` | the codec suite on JMH 1.37 (req 22a) |
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
| campaign RPC client (`CampaignRpc`) | cells A, B, C-retain, C-drop, D-retain, D-drop (full build); A, B, C-nounk, D-nounk (no-unknown build) | every call checked in the run (req 18); smoke-run | -- |
| `RunRpc` | the older RPC harness (blocking and queue delivery) | built, not run since before WP5, in no gate | -- |

Decision 11 as implemented: one options struct per root in native memory, every position
armed with the shim's grow, no pre-allocated buffer; a oneof has one options entry, and its
buffer arrives in the active member's decode group (rule 4 as confirmed, ABI-v1.md
2026-09-26); the binding takes every non-NULL delivered buffer or frees it, and after a
failed decode frees what the core had grown (rule 3; D40).

## Campaign readiness (design/CAMPAIGN.md section 10)

The harness: `gen/run_campaign.sh --suite codec|rpc|calib|gate --out <dir>` (default
`ffi/logs/java/campaign/`), over `ak.CodecJmh` on JMH (arms in `ak.CampaignCodec`),
`ak.CampaignRpc` (client and `--serve`), `ak.CampaignCalib` + `probe/CampaignRev`, and the
gate. Codec and RPC run each build (full, no-unknown) in its own process per launch, the
order of the two builds alternating by launch. Smoke runs: `logs/java/campaign/` and
`logs/java/campaign-wp5s10/`, 1 launch, 1 round, reduced iterations, **every figure
stripped**.

| # | requirement | status |
|---|---|---|
| 1 | one machine, slices sequential | not applicable: the owner's run; the runner runs one suite at a time |
| 2 | governor, turbo, SMT | met as recording (header, from sysfs); setting them is the owner's |
| 3 | isolation, mechanism recorded | met as recording: `AK_ISOLATION` verbatim plus `/sys/devices/system/cpu/isolated` |
| 4 | CPU sets, fixed sizes, worker threads | met: `AK_CPU_CLIENT` / `AK_CPU_SERVER` via `taskset`; outside smoke, when the environment does not set them, the runner reads `ffi/campaign.machine` (ffi/campaign.sh exports the same, and checks the sizes); every log header records the JVM's GC and JIT compiler thread counts as the pinned JVM sizes them, JMH's one benchmark thread, and every RPC log the Netty event loops (fixed at 2 x the CPUs the JVM sees unless `ak.rpc.eventLoops`), the core's runtime workers (2) and grpc-java's executor (its default shared cached pool) |
| 5 | floors gated, not timed | met: the gate runs arms b and c (java8 tree on JDK 17 and JDK 8) and the corpus on JDK 8, for both builds; no `Campaign*` class is in the floor build |
| 6 | build flags printed | met: core snapshot commit and tree key, cargo features (`init-guard`), cdylib, shim `-O2`, JVM flags |
| 7 | payloads, content sets, U-* rows (amended 2026-09-26) | met: 16 payloads x 3 content sets (P7.1 decode-only, ASCII); Latin-1 and wide on P1.2, P2.2, P2.4 are `row_class: required`, on the other payloads `extra`; the 92 accepted `U-*` rows at the shapes core's 7 ABI roots in encode, decode and decode-read through the timed shapes core (`content: corpus`, `core: shapes`, required); every other non-disputed `U-*` row through the corpus description and core, a labelled extra (`core: corpus`) |
| 8 | arms | met: incumbent-prod (`ProtoLiteUtils` marshaller), incumbent-best (`toByteArray` / `parseFrom(byte[])`), core-ffi, core-ffi-pull (labelled extra), host-gen (arm R) |
| 9 | encode, decode, decode + read | met: `encode`, `decode`, `decode-read` (generated `Walk` / `PbWalk`) |
| 10 | retain, drop, no-unknown | met: full build `core-ffi` drop and retain, `core-ffi-pull` drop, `host-gen` drop and retain; no-unknown build `core-ffi`, `core-ffi-pull`, `host-gen` in `no-unknown` (host-gen is generated from the plan: arm R in drop mode over a facade with no `unknownFields`), the incumbent arms beside them (JMH forks one JVM per cell, so no two arms share a process; the incumbent arms are a control across the two builds' invocations, not an in-process one); `unknown_mode` and `build` on every sample; own counts file and own gate (below) |
| 11 | serialise once per iteration; encode variants (amended 2026-09-26) | met: graphs built in JMH's untimed `@Setup(Level.Iteration)`; four labelled encode rows per (arm, payload, set): end state `buf` (bytes in a reused buffer: incumbent-prod `stream()` drained into the reused sink, incumbent-best `writeTo` a CodedOutputStream over it, core-ffi `ak_enc_take` into it, host-gen a copy into it) or `transport` (incumbent-prod `stream()` drained into a fresh exact-size array, as grpc-java's framer drains into buffers it allocates; incumbent-best `toByteArray()`; core-ffi `take()`, host-gen `Enc.toBytes()`, the arrays cells C to F hand their transport), input `hot` (one graph re-encoded) or `pool` (distinct graphs whose serialised bytes exceed 2 x `AK_LLC_BYTES`, default 13.75 MB) |
| 12 | cells A-F, C to F per mode (amended 2026-09-26) | met: full build A, B, C-retain, C-drop, D-retain, D-drop, E-retain, E-drop, F-retain, F-drop; no-unknown build A, B, C-nounk, D-nounk, E-nounk, F-nounk in its own client process (every cell of one build shares one client process, so A and B are in-process controls there); E and F run host-gen (arm R: `Codec` / `CodecRetain`) over the core's transport and over grpc-java; cell B parses the core's response in place through a direct ByteBuffer; cells C and E copy it into a Java array (and C's binding once more into native scratch); each C to F cell re-encodes P2.2 in its mode byte-identical before timing; the leak counters must read 0 after the run |
| 13 | server separate, pre-serialised, one per launch (amended 2026-09-26) | met: ONE server JVM per launch on `AK_CPU_SERVER`, serving every cell of both builds on two Unix sockets (shipped and pinned), pre-serialised P2.2 in (a), protobuf-java parse in (b); every cell opens its own channel (grpc-java) or client (the core) once per launch, warmed by `warm` samples of every (dir, inflight) before round 1, the total warm-up calls in each client's meta line |
| 14 | directions (a), (b); a and a+read | met: `a`, `a+read` (every field of the response walked: generated `Walk` / `PbWalk`), `b`; the optional streamed upload is not built |
| 15 | 1 / 8 / 16 in flight | met (blocking threads) |
| 16 | delivery | met: B, C, E the core's blocking call; A, D, F grpc-java's `ClientCalls.blockingUnaryCall`, the call a generated blocking stub makes (packages/java's clients use blocking stubs), stated in every RPC header; the queue delivery exists only in `RunRpc`, not as a campaign row |
| 17 | shipped and pinned, UDS | met: Unix domain socket over Netty epoll everywhere; shipped = grpc-java's defaults (packages/java configures no UDS channel), the core with no options, the server with grpc-java's defaults; pinned = 4 MiB windows (grpc-java BDP off), core adaptive windows off and Nagle off, 8 MiB messages |
| 18 | every call checked, abort | met: exception or non-OK aborts; response length checked; samples written only if the run completes |
| 19 | crossing counts gate (amended 2026-09-26) | met for both builds: `gen/campaign/counts.ref` and `counts-nounk.ref` (the per-payload rows and, `EP` rows, every entry point per operation counted by the counting shim, resets included and placed: encode `ak_enc_reset` before each encode; decode drop none; retain `ak_dec_reset_<Root>(opts)` before and `(NULL)` after; pull none; per payload and the 92 shapes-root U-* rows, retain with no pre-placed buffer and exact-size grow, grow calls counted); `gen/campaign/rpc-counts.ref` and `rpc-counts-nounk.ref` (cells B, C, D, E per call, both directions, from rpc,count cores); all four diffed by the gate |
| 20 | crossing cost, fwd and rev, perf stat | **not met**: forward and reverse are sampled (the core's shim, JNI forward, JNI upcall, the rust slice's bench beside them), and the runner wraps each in `perf stat` when `perf` exists; `perf` is absent in this container, so that path has never run and cycles / instructions are unverified. Needs `perf` on the campaign machine, no owner decision |
| 21 | CPU time is process CPU (amended 2026-09-26) | met: codec -- the benchmark method reads `CLOCK_PROCESS_CPUTIME_ID` (the shim's `Native.processCpuNs`) at its start and end, per JMH iteration (= round); RPC -- the client's `CLOCK_PROCESS_CPUTIME_ID` per sample; calib -- the same; wall beside it |
| 22 | order (amended 2026-09-26) | met, order stated (R-H23): JMH runs cells in the order given and cannot randomise across forks; the codec arm order inside each (payload, content, dir) block is rotated one step per launch; RPC cells are rotated one step per round, with the same schedule in every launch; the two builds alternate by launch |
| 22a | codec suite on the ecosystem's framework | met: JMH 1.37, SingleShotTime, one iteration = one sample, Blackhole, `-foe true`, every raw iteration exported; RPC and calib stay on the runner (req 13, req 18) |
| 23 | >= 5 rounds, 3 launches | met: `AK_ROUNDS` 5, `AK_LAUNCHES` 3, every round committed raw |
| 24 | warm-up stated, both coder states | met: JMH warm-up iterations per cell in the cell's own fork, recorded per cell; every codec sample carries `jit_ms_during`, the JIT compile time spent during that iteration (0 = no compilation while it ran; R-H19); the JIT is HotSpot's tiered default, and which tier each method reached is not recorded (no WhiteBox API in a product JVM); RPC 2 warm-up samples and the JIT's compile time per round; codec runs with compact strings and with `-XX:-CompactStrings` |
| 25 | allocator warmed, GC stated | met: same warm-up for every arm, heap fixed at 4 GB, G1 |
| 26 | gate before timing | met: a timing suite runs only with a passed gate stamp covering both builds |
| 27 | header | met; a dirty `poc/java` is refused (poc/codec, schema and corpus enter only through the `git archive` snapshot) |
| 28 | JSON lines body | met, plus `coder`, `engine`, `build`, `codec`, `delivery`, `end`, `input`, `row_class`, `core`, `jit_ms_during` and `{"meta":...}` lines (RPC: threads, warm-up calls, order, ratio basis); codec lines converted from JMH's raw data by `gen/jmh_to_jsonl.py` |
| 29 | logs under ffi/logs/java/campaign/ | met: the runner's default `--out` |
| 30 | summaries | not applicable: optional, not produced; the raw lines carry what section 8 needs. A ratio formed from them uses per-launch medians (owner R-H24), cross-process for the codec suite |
| 31 | runner interface | met for the slice runner; `ffi/campaign.sh` is the aggregating session's |
| 32 | smoke run, readiness recorded | met: `logs/java/campaign-wp7/` (WP7, both builds: codec with the encode variants, the shapes-core U-* rows and row classes; rpc cells A-F with a, a+read, b against one server per launch; calib), figures stripped; the one requirement not met is 20 |

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
  two builds carry cells A and B (RPC, in the same client process as C and D) or the incumbent
  arms (codec, each in its own JMH fork, so a control across processes) beside them.
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

No other defect known to this slice is open. Closed items are in JOURNAL (J2-J28).

## What is not measured or not run

- **Any performance result.** Every timing is container instrumentation.
- **`perf stat`** cycles and instructions (req 20): `perf` absent here.
- **The RPC queue and callback deliveries** as campaign rows (the queue exists in
  `RunRpc`, not run since before WP5, in no gate); **the streamed upload** (req 14,
  optional); **TLS, retry, metadata, deadlines, numeric status, streaming** (owner
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
2. Decision 11 pre-allocated buffers and pools, if the aggregating session asks for them.

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
