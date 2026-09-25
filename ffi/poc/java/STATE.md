# java slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only thing that
survives the end of a session. It says what exists and what was checked; it does not say
what a binding should choose (`CLAUDE.md`: the decision is the owner's).

**Phase** (README 1.1): setup and design. **Every timing in this directory is container
instrumentation**: it shows a harness runs or exposes a harness defect, and it is not a
result. This file quotes no timing figure. What counts as a result here is byte identity,
crossing counts, floor builds, corpus passes, feasibility and defects found.

| | |
|---|---|
| **Status** | FIX-PLAN WP5 step 3 done (2889d87, 287deca); WP5 tail D38 and D39 fixed (bb98e8f, 271fdd5) and re-gated from a clean core build at 271fdd5: payload gate 1,389 checks per arm-set, 0 failures (`wp5s6-gate.log`); full corpus (702 rows) on six arms, target and floor, 0 failing arm-rows (`wp5s6-corpus.log`); the rust slice's probe manifest, 0 failing (`wp5s6-probe.log`). **WP3 (2026-09-25): campaign harness built and smoke-run** (`gen/run_campaign.sh`; checklist below; unmet: req 7 unknown-field rows unnamed, req 10 core-ffi retain pending decision 11, req 20 perf unavailable here, req 29 log path by ownership). |
| **Levels** (owner decision D3) | **floor Java 8** (correctness gate only): `openjdk 1.8.0_504`, JDK 8 `javac`. **target JDK 17**: `openjdk 17.0.20.1`. JDK 21 only for the two secondary probes (virtual threads, FFM preview). |
| **Incumbent** | protobuf-java **3.25.5** (resolved from `packages/java`'s pins), protoc 3.19.0 (copied from `~/.m2` when present). R14's baseline path is `io.grpc.protobuf.lite.ProtoLiteUtils`' marshaller (`RunR14`). |
| **Core** | the shared crate `poc/codec/crates/ak-core` (R0), no copy here. `gen/build.sh` builds it from a `git archive` SNAPSHOT of the committed `ffi/poc/codec` (`AK_CORE_REV`, default HEAD; `AK_CODEC=<dir>` for another tree), because other slices regenerate the core in the same working tree; it builds into `core-build/<key>/`, key = the git tree hash of `ffi/poc/codec` at that revision, `core-build/current` -> the key built, and every shim is checked to link that key's core (D39: a reused target dir once kept a stale core). The snapshot commit and key go to `build/core-rev.txt` and into every gate log. **Every codec build carries `init-guard`** (R-G7); the corpus build is `corpus,init-guard`. |
| **Generator** | `gen/generate.py` is glue: it calls `poc/codec/gen/java_backend.emit` for two descriptions (shapes.json, 7 roots; the corpus reader schema) and renders the slice's harness glue (payload builders, arm dispatchers, the corpus projection), all from the plan's descriptor view. It writes under `poc/java/` only. `--check`: every generated file current, `poc/codec/gen/generate.py --check` exit 0, and that file's import guard applied to the nine Java backend modules and the glue, planted violation caught. |
| **Blocked on** | nothing for correctness. |

## What exists

| | |
|---|---|
| `poc/codec/gen/java_*.py` | **the Java backend** (commit 2889d87): `java_backend` (entry), `java_rcodec` (arm R, drop and retain), `java_layout` (offsets), `java_abi` (the C header and the slot tables), `java_jni` (shim, NativeEntry, `ak_init`), `java_binding` + `java_pull` (the Java half, per level), `java_facade`, `java_names`. Each imports `plan`, never the IR |
| `gen/` | glue: `generate.py`, `java_build.py`, `java_pbbuild.py`, `java_arms.py`, `pbarms.py`, `java_ffiarms.py`, `java_corpus.py` (corpus dispatch + projection); `build.sh`, `gate.sh` (payload gate, arms a/b/c), `corpus.py` + `corpus.sh` (corpus gate), `layout_break.sh`, `boundary.sh`, older measurement scripts |
| `src/java/ak/` | hand-written runtime (`Enc`, `Dec` -- primitives only, its skip takes the plan's `MAX_FIELD_NUMBER` / `GROUP_DEPTH_LIMIT` from the generated codec (D38), `Utf8`, `Utf8View`, `Str17`, `Mem`, `Native`, ...) and harnesses: `RunConformance`, `RunUnknown`, `RunCounts`, `RunCorpus`, `RunRuleGaps`, `Bench`, `RunDelta`, `RunR14`, `RunRpc` |
| `src/generated/{java17,java8}/`, `native/generated/` | the shapes description: `ak.shapes` (facade, `Codec`, `CodecRetain`, `Layout`, `Binding`, glue), `ak.floor` (arm b), `ak.borrow` (decision 13); `ak_abi.h` + `shim.c` |
| `src/generated_corpus/{java17,java8}/`, `native/generated_corpus/` | the corpus description: `ak.corpus` (+ `Dispatch`, `Project`), `ak.corpus.borrow`; its own header and shim, linked against the `corpus` core |
| `src/generated{,_corpus}/shared/` | `ak.NativeEntry`, `ak.corpus.NativeEntry`: the per-message natives and `ensureInit()` |
| `native/rpc.c` | ABI v1 section 9 from the JVM; every struct and prototype from the generated header (plan.rpc) |
| `native/test/nullpin.*`, `probe/` | R-D9 fault injection; crossing-price, FFM and JNI-accessor probes |

### Arms

| arm | what it is | payload gate | corpus gate |
|---|---|---|---|
| `R` | arm R, unknown fields dropped (`Codec`) | yes | yes |
| `R-retain` | arm R, unknown fields retained (`CodecRetain`) | no (RunUnknown is drop-mode) | yes |
| `ffi`, `ffi-nobatch`, `ffi-zeroed`, `ffi-nobatch-zeroed` | the C ABI through JNI, push family; batching and decision 9's fill as registrations | yes (all four) | `ffi` |
| `ffi-pull`, `ffi-pull-walk` | ABI v1 7.1's pull family, drained and walked; re-encoded with the push encode | yes | yes |
| `ffi-borrow` | decision 13's borrowed facade | yes | yes |
| `pbj` | protobuf-java | yes | no (not generated; the incumbent) |
| RPC arm (`RunRpc`, `native/rpc.c`) | section 9 end to end | built, **not run in this unit**, not in either gate | -- |

## Campaign readiness (design/CAMPAIGN.md W11 draft, a10ac81) -- FIX-PLAN WP3

The harness: `gen/run_campaign.sh --suite codec|rpc|calib|gate --out <dir>` over
`ak.CodecJmh` on JMH (req 22a; arms in `ak.CampaignCodec`), `ak.CampaignRpc` (client and `--serve`), `ak.CampaignCalib` +
`probe/CampaignRev`, the gate (`gen/gate.sh`, `gen/corpus.sh`, crossing counts against
`gen/campaign/counts.ref`). Smoke run (req 32): `logs/java/campaign-smoke/`, 1 launch, 1
round, reduced iterations; **the committed copy has every figure stripped** (cpu_ns, wall_ns,
jit_ms replaced by "stripped"), so it shows structure and coverage and nothing else.

| # | requirement | status |
|---|---|---|
| 1 | one machine, slices sequential | not applicable to a harness (the owner's run); the runner runs one suite at a time |
| 2 | governor, turbo, SMT | met as recording: header prints governor, no_turbo/boost, SMT from sysfs; setting them is the owner's |
| 3 | isolation, mechanism recorded | met as recording: `AK_ISOLATION` verbatim plus `/sys/devices/system/cpu/isolated` |
| 4 | CLIENT / SERVER sets as parameters | met: `AK_CPU_CLIENT` / `AK_CPU_SERVER` via `taskset`, required outside smoke; NUMA/SMT-sibling disjointness is the owner's (node count printed) |
| 5 | floors gated, not timed | met: the gate suite runs arms b and c (java8 tree, JDK 17 and JDK 8) and the corpus on JDK 8; nothing on 8 is timed (Campaign* is not in the floor build) |
| 6 | build flags printed | met: core snapshot commit and tree key, cargo features (`init-guard` on), cdylib, shim `-O2`, JVM flags (heap fixed, G1, tiered JIT) |
| 7 | 16 payloads, 3 content sets, unknown rows | **partly met**: 16 payloads x 3 sets (P7.1 is decode-only, ASCII vector). **Not met**: the corpus unknown-field rows "named in section 4.3" -- section 4.3 names none; needs the list |
| 8 | arms | met: incumbent-prod (grpc-java `ProtoLiteUtils` marshaller, R14), incumbent-best (`toByteArray` / `parseFrom(byte[])`), core-ffi (push), core-ffi-pull (labelled extra), host-gen (arm R) |
| 9 | encode, decode bare, decode + read all | met: `encode`, `decode`, `decode-read` (generated `Walk` / `PbWalk` read every field) |
| 10 | drop and retain for core-ffi and host-gen | **pending decision 11 port**: host-gen drop and retain run (`Codec`, `CodecRetain`); core-ffi retain is wired as an `unknown_mode` hook and recorded as pending in a meta line, because the binding does not render decision 11's mechanism yet (being ported into poc/codec); incumbent: protobuf-java's default (retains) |
| 11 | serialise once per iteration, fresh graph | met: a pool of `iters` freshly built objects per encode sample, built in JMH's untimed `@Setup(Level.Iteration)`; same for the facade arms |
| 12 | cells A-D | met |
| 13 | server separate process, pre-serialised | met: `--serve`, its own JVM on `AK_CPU_SERVER`; (a) the same pre-serialised P2.2 bytes; (b) parses with protobuf-java in every cell |
| 14 | directions (a) and (b) | met; the optional streamed upload is not built |
| 15 | 1 / 8 / 16 in flight | met (blocking threads) |
| 16 | B and C blocking delivery | met; queue/callback extra rows not in the campaign harness (RunRpc has the queue) |
| 17 | shipped and pinned | met: shipped = packages/java's GrpcChannelBuilder (defaults, 8 MiB inbound, 1 MiB metadata), core with no options, server grpc-java defaults; pinned = 4 MiB windows everywhere, grpc-java BDP off, core adaptive 0 and Nagle 0. One server and one client process per transport |
| 18 | every call checked, abort | met: exception / non-OK aborts; response length checked (P2.2 in a, 0 in b); samples are written only if the run completes |
| 19 | crossing counts gate | met: the gate diffs the counting build's rows against `gen/campaign/counts.ref` (94 rows, equal to wp5-counts.log) |
| 20 | crossing cost, fwd and rev, perf stat | **partly met**: forward through the core's shim, JNI forward and JNI upcall (reverse) as JSON samples, the rust slice's crossing bench beside them; `perf stat` wraps each when `perf` exists -- not installed in this container, so cycles/instructions are unverified here |
| 21 | CPU time, fine clock | met: codec -- JMH measures wall only, so the benchmark method reads the thread CPU clock (ThreadMXBean, CLOCK_THREAD_CPUTIME_ID) at its start and end and the converter joins it per iteration (the two reads sit inside JMH's timed region); RPC process CPU from `clock_gettime(CLOCK_PROCESS_CPUTIME_ID)` via rpc.c (not the JDK's `getProcessCpuTime`, which reads `times()`); wall beside it |
| 22 | interleaved, rotated | met in blocks for codec (req 22a): JMH runs each cell in its own fork, rounds consecutively; the arm order inside each (payload, content, dir) block is rotated between launches. RPC: cells rotated per round inside each (dir, inflight) |
| 23 | >= 5 rounds, 3 launches, every round committed | met: `AK_ROUNDS` 5 (JMH measurement iterations), `AK_LAUNCHES` 3 (one JMH invocation each, `-f 1`, i.e. one fork per cell per launch); every raw JMH iteration exported, JMH's JSON kept beside the lines |
| 24 | warm-up stated, both coder states | met: codec -- JMH warm-up iterations per cell (`-wi`, 5; each in the cell's own fork, so every arm gets the same), recorded in a meta line per cell with the mode, forks, JVM and its arguments (tiered JIT, default thresholds); RPC 2 warm-up samples and the JIT's compile time per round; every codec launch runs twice, compact strings and `-XX:-CompactStrings` (`coder` field) |
| 22a | codec suite on the ecosystem's framework (owner) | met: JMH 1.37 (`ak.CodecJmh`, `deps/jmh/pom.xml`), SingleShotTime, one iteration = one sample, Blackhole on the result sink, `-foe true` with the correctness check in the trial setup; RPC and calib stay on the runner (req 13 separate server, req 18 abort on any failed call) |
| 25 | allocator warmed, GC stated | met: same warm-up for every arm, heap fixed (-Xms=-Xmx 4g), G1 on |
| 26 | gate before timing | met: a timing suite runs only with a passed gate stamp for the build (run first if absent) |
| 27 | header | met (see 2-6, transport, repeats); dirty tree refused for poc/java (poc/codec, schema and corpus enter only through the `git archive` snapshot) |
| 28 | JSON lines body | met, the required fields plus `coder`, `engine`, `delivery` (extra) and `{"meta":...}` lines; codec lines converted from JMH's rawData by `gen/jmh_to_jsonl.py` (a sample with no CPU reading is refused) |
| 29 | logs to ffi/logs/campaign/java/ | **not met here by ownership**: `--out` takes any directory; the smoke log is committed under `logs/java/campaign-smoke/` because a slice writes `logs/<lang>/` only |
| 30 | summaries | not produced (optional); the raw lines carry what section 8 needs |
| 31 | runner interface, top-level campaign.sh | met for the slice runner; `ffi/campaign.sh` is the aggregating session's (not written here) |
| 32 | smoke run, readiness recorded | met: this section and `logs/java/campaign-smoke/` |

## Correctness (results)

**WP5 tail re-gate** (supersedes the wp5-* results below; same structure): clean build
(`wp5s6-build.log`, core snapshot 271fdd5, tree key in the log). `wp5s6-gate.log`: arms a,
b, c 1,389 checks each, 0 failures; unknown 66/0; pull reverse 0. `wp5s6-corpus.log`: 702
rows; R and R-retain pass 696 (555 / 141), the four ffi arms pass 680 (542 / 138) with 16
`Nest` rows outside the C ABI; 0 failing, 6 disputed (`U-map-entry`, `X-tag-zero-Empty`,
`X-tag-zero-nested-Empty`, `X-field-over-max`, `X-field-2p32-plus-2`,
`X-field-over-max-in-group`: every arm refuses the last five, "field number 0 or above
2^29-1"); controls proj 156, reenc 156, accept 570, noinit 104 failing arm-rows.
`wp5s6-probe.log` (the rust slice's probe manifest, 11 rows, 6 arms, 8 and 17): 0 failing;
`P-field-maxplus1-in-group` refused by arm R and by the core (D38).

**Payload gate: `logs/java/wp5-gate.log`** (tree 287deca, core snapshot 287deca = the Rust
sources of aba944a, `init-guard`), verbose per-arm logs `wp5-conformance-arm-{a,b,c}.log`.

- **Arms a, b, c: 1,389 checks each, 0 failures**, every registered arm listed with its row
  count (encode-capable arms 319 rows, decode-only arms 94, 46 round trips each). Same
  structure as `rd5-gate.log`; the generated text changed, the bytes did not.
- **Unknown-field vectors** (`RunUnknown`): 66 checks, 0 failures.
- **The pull family is running**: 0 reverse crossings for both deliveries on every payload,
  counting core, same run.
- **Layout guard** (`wp5-layout-guard.log`): the run-time comparison fires and names the
  perturbed fact; NEW, the header's static assertion of every Java offset fails the shim's
  compile when one number is perturbed, naming the member. **Boundary** (`wp5-boundary.log`):
  every entry point an undefined import of the shim; no codec symbol in the shim.

**Corpus gate: `logs/java/wp5-corpus.log`** (`gen/corpus.sh`), every row of `ffi/corpus`
(691), each (arm, row) on its own thread under a 5 s timeout, against the core built with
`corpus,init-guard` (the log shows the features and that the loaded core exports
`ak_decode_WireZoo`).

| arm | pass (accept / refused) | fail | disputed | not in the C ABI | timeouts |
|---|---|---|---|---|---|
| R | 688 (547 / 141) | 0 | 3 | 0 | 0 |
| R-retain | 688 (547 / 141) | 0 | 3 | 0 | 0 |
| ffi, ffi-pull, ffi-pull-walk, ffi-borrow | 672 (534 / 138) each | 0 | 3 | 16 (`Nest`) | 0 |

Identical on the target (java17, JDK 17) and the floor (java8, JDK 8). Disputed rows, as
read: `U-map-entry` accepted, matching the pure-python reading; `X-tag-zero-Empty` and
`X-tag-zero-nested-Empty` refused (field number 0). R-retain writes the retained form on
every `unknown` row (no retention gap). **Controls, each seen failing** (same log):
`proj` 156, `reenc` 156, `accept` 570 failing arm-rows on a 121-row subset; `noinit`
(`ak_init` skipped) fails every accept row of every ffi arm with AK_ERR_UNINITIALIZED.

**R-E4 closed** (`wp5-re4-closure.log`): the 18 `X-tag-zero-<Root>` rows are refused
("field number 0") and `E-map-entry-empty` re-encodes to the 6 B canonical form (pre-WP5:
8 B, key written and empty value omitted). **Rule gaps outside the corpus** (same log,
section 4, arm R and ffi): a repeated singular message and a repeated oneof message member
MERGE (`{seconds:1, nanos:2}`, as protobuf C++ reads the same bytes); `body_case = 99` is
refused (arm R `Enc.Refused`, ffi AK_ERR_ABI); `WireZoo.v_double = -0.0` is written as
`21 0000000000000080` and +0.0 is not; `0a ffffffff07` is refused at the length (R-G8; arm
R "length past end of buffer", ffi AK_ERR_TRUNCATED).

## Crossing counts (results) -- `logs/java/wp5-counts.log`

Every row identical to `rd5-counts.log` except the transcoder column of P5.1 to P5.4
(3 -> 2): ABI v1 section 8's direct path now runs (see defects).

| payload | encode fwd / rev | per element | decode (push) fwd / rev | per element |
|---|---|---|---|---|
| P1.2 (1,000 M1 rows) | 8 / 1 | 0.009 | 1 / 5 | 0.006 |
| P2.2 (500 M2 elements) | 2,511 / 2,501 | 10.024 | 1 / 3,501 | 7.004 |
| P2.3 | 629 / 626 | 10.040 | 1 / 876 | 7.016 |
| P2.4 | 403 / 401 | 10.050 | 1 / 561 | 7.025 |

Unbatched, P2.2, P2.3 and P2.4 encode at 22.0, 130.0 and 316.0 crossings per element. The
pull family: reverse 0 on every payload; forward 2 for the walk delivery at any size, 1 plus
one per 32 KB chunk for the drain. Decision 5's learned width: 0 prefix moves on every
uniform payload, 80 on P2.4, 0 grow callbacks. RPC (`rpc.log`, counted, pre-WP5): blocking
call 2 forward / 0 reverse, queue delivery 3 forward / 0 reverse, per call.

## Feasibility findings and defects found

- **WP5 step 3, found by the port** (JOURNAL J21), each fixed where the rule is rendered:
  (1) **section 8's direct path never ran**: the fill staged the bulk bytes with the
  passthrough transcoder, and the core reads the direct argument only when the slot is
  `AK_STR_DIRECT`; now the slot carries the sentinel (P5.x transcodes 3 -> 2, counted);
  (2) the **zeroed fill dropped -0.0** (`x != 0.0`, E5's defect in the binding; latent,
  no singular double in shapes.json); (3) **`Utf8View` refused ASCII after a multi-byte
  character** (corpus T-enc-lone-*, ffi-borrow): the payload gate's non-ASCII content sets
  recode every character, so no gated string had that shape.
- **R-G7's Java instance**: the binding never called `ak_init`, and no core build carried
  `init-guard`, so nothing noticed. Both fixed; the `noinit` control shows the check live.

- **The Java 8 floor build was broken from 5241ced to this work unit.** `RunRpc.java` used
  `ProcessHandle` (Java 9) and is in the floor's compilation, so `gen/build.sh` stopped at
  the JDK 8 `javac` step (`set -e`). Fixed with a Java 8 API; arms b and c build and pass
  again (rd5-gate). No floor log was committed while it was broken: `floor.log` and
  `w10-regate.log` predate 5241ced.
- **R-D9, Java part, fixed**: the bulk-bytes encode entry did not NULL-check
  `GetPrimitiveArrayCritical`; the parse entries did. Fixed in the emitter
  (now `poc/codec/gen/java_jni.py`), regenerated. Swept: the two `uri` pins in `native/rpc.c` were also
  unchecked and are fixed. `rd9-jni-null.log` shows the old shim entering the core with
  `(NULL, 65536)` under fault injection and the new one returning `AK_ERR_HOST` without
  entering it, frame stack balanced over 17 calls.
- **R-D2 / R-G5 in `native/rpc.c`**: it hand-declared `ak_client_opts`, `ak_bytes`,
  `ak_completion` and every RPC prototype (handles as `void *`); all now come from the
  generated header, rendered from `plan.rpc` with size and offset asserts.
- **Packaging (README 5.1.3)**: the floor and the target are different code because the
  target reads `String.coder`/`String.value`, which Java 8 lacks; `ak.Str17` shows runtime
  dispatch working in one tree. Both levels use `sun.misc.Unsafe`, which is terminally
  deprecated from JDK 23 with FFM (JDK 22) as its successor, above the target. `--release 8`
  on JDK 17 cannot build the floor (`ct.sym` has no `sun.misc.Unsafe`), so the floor is
  compiled by JDK 8's `javac`.
- **Critical sections**: holding `GetPrimitiveArrayCritical` across a blocking RPC call
  whose completion needs another Java thread (the in-process grpc-java server) deadlocked;
  the RPC arm copies instead. The pull parse and bulk-bytes pins are safe because the call
  makes no upcall and completes on its own.
- **Virtual threads** (`pinning.log`, JDK 21): blocking in a native frame pins the carrier
  and parking on a future does not; observed as elapsed time scaling with the pinned
  prediction at 1, 2 and 4 carriers. A container observation of a qualitative behaviour.
- **R9's JIT effect** (`r9-mechanism.log`): C2 prunes `StringUTF16.charAt` from
  protobuf-java's encoder as "call site not reached" in some runs and not others
  (`-XX:+LogCompilation`), bimodally; the core's transcoder is not in the JIT's reach. A
  harness hazard for the campaign on wide content, recorded with its mechanism.

## Instrumentation (container timings; not results)

These logs exist and show the harnesses run. **No figure from them is quoted here or is to
be quoted as a result.** Known harness issues the campaign must not repeat:

- the R14 baseline (gRPC's marshaller) is a separate harness (`RunR14`); the main `Bench`
  arms use `toByteArray` / `parseFrom` (R-C10);
- the RPC grid ran four cells in four JVMs, unpaired, and `rpc.log` is a curated summary
  with **no raw runner output committed** (R-C7, R-C9); its in-process rows had client and
  server contending in one JVM (R-C4);
- wide-content encode timings are subject to R9's bimodal JIT state (R-C12);
- none of these harnesses has been checked against `design/CAMPAIGN.md` (WP3).

Timing logs: `encode.log`, `encode-take-fix.log`, `decode.log`, `decode-pull.log`,
`delta.log`, `drift.log`, `floor.log` (its correctness block is a result, its timings are
not), `contentsets.log`, `deopt.log`, `r9-mechanism.log`, `crossing.log`,
`calibration-r13.log`, `baseline.log`, `ffm.log`, `shim-probe.log`, `pinning.log`,
`rpc.log`, `r14.log`, `r14-summary.md`, `w10-regate.log` (correctness plus a drift check).

## Open defects and gaps

| # | what | status |
|---|---|---|
| E1-E6 | arm R's five R-E4 gaps and the `int` length check (R-G8) | **closed** by WP5 step 3 (wp5-corpus.log, wp5-re4-closure.log) |
| E7 | `RunConformance`'s comment on `ak.BorrowArm` and the floor | fixed (comment) |
| E8 | `gen/generate.py` wrote into `poc/codec` | **closed**: writes under `poc/java` only |
| G1 | plan gap: the vtable structs' member order and shape, and the pull record slot numbering, are rendered by `rust_abi` (a backend), not stated by the plan; `java_abi` restates them from the same plan facts. No layout guard covers a vtable | reported to the aggregating session |
| G2 | plan gap: `plan.lifecycle` names `AK_INIT_*` but not their values; `ak_err`, the error codes and `ak_bdr_rec` are fixed text in `ak-abi` restated in `java_abi` (the record header is static-asserted) | reported |
| G3 | the ffi arms have no retain mode: decision 11's slots stay NULL and there is no `ak_uencode_*` path in the binding (R-G11 would also bound it) | open, not built |
| G4 | a `lossy` UTF-8 plan option raises in `java_rcodec` (the runtime renders `reject` only) | by design until the option is used |
| D38 | `Dec`'s group skip accepted a field number above 2^29-1 inside a group; limits were runtime constants | **fixed** (bb98e8f, 271fdd5), wp5s6-probe.log |
| D39 | `gen/build.sh` reused a cargo target dir over a git-archive snapshot and once kept a stale core | **fixed**: target dir keyed on the snapshot tree, shims checked (wp5s6-d39-keys.log) |
| D1-D7 | earlier slice defects | fixed; see JOURNAL J2-J16 |

## What is not measured or not run

- **ffi retain** (G3); **corpus C5 (produce)**, so the transcode encode half (`T-enc-*`,
  `produce` names java) is consumed but not produced: `ak.Utf8`'s U+FFFD against
  protobuf-java's `?` is written down and not exercised; **the chunking class** (`C-*`)
  passes on the ffi arms, but how many chunks each row crossed in this binding was not
  counted.
- **pbj against the corpus** (the incumbent is not generated; only its payload bytes are
  gated).
- **The RPC arm**: rebuilt against the plan-rendered header (compiles, links), not re-run
  in this unit, and never under a correctness gate.
- **Concurrency** (ABI v1 obligation 12.5): one thread everywhere; the shim's thread-local
  frame stack is argued, not tested.
- **Allocation and footprint**, **message size limits**, **the codec's rollback of a
  half-written field**.
- **FFM as a binding**, **the C-shim arm of README 9.1**: not built (probes only).
- **Rust MSRV 1.88** for the core: not verified here (rustc 1.94.1).
- The timed core now carries `init-guard`; no timing was taken in this unit, and any old
  timing log predates it.

## Next step

0. **Campaign (W13), when the owner runs it**: `AK_CPU_CLIENT=.. AK_CPU_SERVER=.. AK_ISOLATION=..
   gen/run_campaign.sh --suite gate|codec|rpc|calib --out ffi/logs/campaign/java`. Before it:
   render decision 11's mechanism in the Java binding (req 10), and take the unknown-field
   row list for req 7.

1. **ffi retain** (G3), if the owner wants both unknown-field modes on the C ABI arms:
   wire `unknown` / `unk_<slot>` trampolines and the `ak_uencode_*` / `ak_uelem*` path with
   `ufix` fills; R-G11 bounds what it can retain.
2. **WP3**: conform `Bench`, `RunR14` and `RunRpc` to `design/CAMPAIGN.md`; re-run the RPC
   arm and gate its response bytes.
3. When the plan states the vtable layout and record numbering (G1), switch `java_abi` to it.

## Requests to the aggregating session

1. **Plan gaps G1 and G2** above: the vtable member order and shape, the pull record slot
   numbering, the lifecycle flag values and `ak_err` are not in the plan contract; the
   Java backend restates the Rust backend's rendering. The shared `generate.py`'s
   `BACKENDS` guard list does not name the `java_*.py` modules (this slice's `--check`
   applies the same guard to them).
2. The C header is now rendered twice from the plan (the cpp slice's and `java_abi`'s);
   a shared C-header backend would make it once.
3. The corpus has no vector for a repeated singular message field or a repeated oneof
   message member (merge), and the payload gate's content sets never mix ASCII with
   multi-byte characters in one string (how the `Utf8View` defect stayed hidden).
4. `ak_bdr_count_forward`'s doc comment tells a host to call it after a drain, but
   `ak_bdr_drain` is itself an entry point that bumps `forward` (still true).

## Log index

**Results** (correctness, counts, feasibility):

| Log | What it establishes |
|---|---|
| `campaign-smoke/` | WP3 smoke run of `gen/run_campaign.sh` (gate, codec, rpc, calib): structure and coverage; every figure stripped |
| `wp5s6-gate.log`, `wp5s6-conformance-arm-{a,b,c}.log` | WP5 tail: the payload gate from a clean core build at 271fdd5, 0 failures |
| `wp5s6-corpus.log` | WP5 tail: 702 corpus rows, six arms, 8 and 17, 0 failing; controls; rule gaps |
| `wp5s6-probe.log` | the rust slice's probe manifest on six arms, 8 and 17, 0 failing (D38) |
| `wp5s6-d39-keys.log`, `wp5s6-build.log` | D39: the keyed core target dir, every shim on this key's core; the clean build |
| `wp5-gate.log` | WP5 step 3: the payload gate on arms a, b, c, every arm listed, 1,389 checks each, 0 failures; unknown vectors 66/0; pull reverse crossings 0; core with `init-guard` |
| `wp5-conformance-arm-{a,b,c}.log` | the verbose per-row gate output behind it |
| `wp5-corpus.log` | the full corpus on six arms, target and floor, 0 failing arm-rows; generate --check; controls seen failing (proj, reenc, accept, noinit); the rule gaps on arm R and ffi with protobuf C++'s reading |
| `wp5-re4-closure.log` | the 19 rows of re4-corpus-armR.log, each looked up: 18 refused, E-map-entry-empty 6 B |
| `wp5-counts.log` | crossing counts, identical to rd5-counts.log except P5.x transcodes 3 -> 2 (direct path) |
| `wp5-layout-guard.log` | section 10's guard failing at load AND the header's static assertion failing the compile |
| `wp5-boundary.log` | R5's boundary check on the WP5 build |
| `wp5-build.log` | the build, filtered of compiler warnings: core snapshot commit, features, every step |
| `rd5-gate.log`, `rd5-conformance-arm-{a,b,c}.log`, `rd5-counts.log` | the pre-WP5 gate and counts (superseded) |
| `re4-corpus-armR.log` | the pre-WP5 arm R corpus run: 19 failing of 392 (superseded) |
| `rd9-jni-null.log` | R-D9 fault injection |
| `counts.log`, `unknown.log`, `conformance.log`, `boundary.log`, `layout-guard.log`, `flow-control.log` | earlier runs |

**Instrumentation** (container timings; see the section above): `encode.log`,
`encode-take-fix.log`, `decode.log`, `decode-pull.log`, `delta.log`, `drift.log`,
`floor.log`, `contentsets.log`, `deopt.log`, `r9-mechanism.log`, `crossing.log`,
`calibration-r13.log`, `baseline.log`, `ffm.log`, `shim-probe.log`, `pinning.log`,
`rpc.log`, `r14.log`, `r14-summary.md`, `w10-regate.log`.
