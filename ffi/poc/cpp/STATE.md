# cpp slice: state

**Read this first, and rewrite it at the end of every work unit.** It is the only thing
that survives the end of a session. It says what is true of the tree now. The history of
how it got here, including every refuted idea and every superseded figure, is in
`JOURNAL.md`.

**Phase.** The branch is in setup and design (README 1.1). No figure in this file or in
any log listed here is a performance result. Every timing ever taken by this slice is
**container instrumentation**: it shows that a harness runs, or it exposes a harness
defect. What this file reports as results are correctness outcomes and crossing counts.

| | |
|---|---|
| **Status** | 2026-09-26, FIX-PLAN WP7: the harness conforms to the owner's 2026-09-26 contract (process CPU, content sets and U rows in three directions, encode end/input rows, cells E and F, one server per launch on Unix sockets, a and a+read, counts with resets, retain and RPC cells B-E). Both builds gated from a clean checkout at `a03b06ab2`, 0 failed steps (`logs/cpp/wp5-*.log`, `wp5s10-nounk.log`), C++17, C++14 and C++11; ASan+LSan clean on both builds (`asan.log`); campaign smoke (gate, codec, rpc, calib) green, every new row present, figures stripped, `"smoke": true` |
| **Core** | the shared one at `ffi/poc/codec/crates/ak-core` (R0). CMake builds it with cargo, `init-guard` in every configuration. Full-build flavours: plain, `count`, `corpus`, `rpc`, `rpc,count`, and three planted cores (`pad-widths`, `global-widths`, both). No-unknown flavours: `--no-default-features` plus `init-guard` alone, `count`, `corpus` or `rpc`. Each flavour has its own target dir under `core-build/` |
| **Generator** | one generator (W14). `poc/codec/gen/plan.py` holds the rules. This slice's backend modules in `poc/codec/gen/` are `cpp_binding.py`, `cpp_native.py`, `cpp_facade.py`, `cpp_names.py` and `cpp_layout.py`, plus `c_abi.py`, which renders the C header for every slice. `gen/generate.py` is glue: it renders the targets from plans and imports no IR (the guard in `generate.py --check`) |
| **Floor / target** | C++11 floor, C++17 target, both builds. C++14 also builds and is gated (full build) |
| **Incumbent** | protobuf C++ 3.21.12 and grpc++ 1.51.1, apt's, the only versions in this container. `packages/cpp` pins neither. The runner builds against gRPC v1.54.0 and a current version through AK_INCUMBENT_PREFIX, one run per prefix (section 3); neither prefix exists here (checklist row 3) |
| **Compiler** | g++ 13.3.0, `-O2 -g -DNDEBUG`; rustc 1.94.1 |

## What exists

```
poc/codec/gen/cpp_binding.py   the C++ host binding over the C ABI (arm core-ffi), rendered
                               from a plan. Full build:
                                 - encode_into_* (and _zeroed, _nobatch), encode_into_*_unk;
                                 - decode_with_* (drop context), decode_with_*_opts (armed
                                   in place), decode_with_*_unk (retain everywhere),
                                   decode_with_*_pool (pre-allocated pools, refilled in place);
                                 - unk_opts_*, unk_clear_*;
                                 - DecRoot<T>, DecCtxs, dec_ctx_new_for<T>() (contexts bound
                                   to their root, rule 6).
                               No-unknown build (plan relowered with unknown="drop"): none of
                               the unknown-field family; ak_dec_ctx_new_<Root>(void).
                               Buffers still in the options after a decode are the host's
                               and are left there (R-H7: reusable options, rule 7).
poc/codec/gen/cpp_facade.py    the facade; in the no-unknown build without `unknown_fields`
                               (owner decision R-H22). That build is a separate configuration
                               with its own header directory, so no installed header changes
                               layout under a consumer's -std (README 5.1)
poc/codec/gen/cpp_native.py    arm host-gen: the codec generated into C++ from the same plan,
                               drop and retain renderings
include/ak_abi.h, include/generated/ak_layout*.h   full-build C header (c_abi.py, 400 facts)
nounk/include/...                                  no-unknown header (AK_NO_UNKNOWN_FIELDS,
                                                   240 facts)
corpus/include/..., corpus/nounk/include/...       the same two for the corpus reader schema
nounk/src/generated/types*.{h,cpp}, corpus/nounk/src/generated/types.{h,cpp}
                               the no-unknown facades (no unknown_fields), found first on the
                               nounk targets' include path (sources include <generated/types.h>)
src/generated/                 facade types, binding(_nounk), borrowed-facade binding(_nounk),
                               core_native(_retain), builders (facade and protobuf), cases,
                               projection, touch (read-every-field traversal)
corpus/src/generated/          the corpus reader schema's facade, native codecs, binding(_nounk),
                               projection, dispatch(_nounk)
build-upbclang/, build-upbft/     TRACKED build configurations of the upb FASTTABLE experiment
                               (upb-fasttable.log); not used by any gate
include/ak/rt.h, vocab.h, values.h, projjson.h    hand-written runtime for the native codec
                               and the facade vocabulary (decode-rule constants come from
                               include/generated/ak_rules.h, rendered from the plan)
```

The harness binaries, one source each (`CMakeLists.txt`):

| Source | Targets | What it does |
|---|---|---|
| `src/conformance.cpp` | `conformance_{a17,b17,c14,c11}_shared`, `conformance_a17_static`, `conformance_a17_noinit` (plant); `conformance_nounk_{a17,c11,static}` | payload byte identity against `ffi/schema/generated/manifest.json` on every encoder and decoder arm, the layout table against the core's `ak_layout_facts`, absent/unknown/malformed vectors; decision 11's pool, refill, oneof, error and wrong-root cases (full); rule 6 and the drop of unknowns (no-unknown) |
| `corpus/src/corpus_main.cpp` | `corpus_all_{a17,c14,c11,a17_static}`, `corpus_all_noinit` (plant); `corpus_nounk_{a17,c11}`, `corpus_nounk_noinit` (plant) | one corpus row per process, four arms (ffi-drop, ffi-retain, native-drop, native-retain); `--unk` runs decision 11's controls on one row |
| `src/counts.cpp` | `counts_a17_shared`, `counts_a17_static`, `counts_a17_static_lto`; `counts_nounk` | crossing counts from a counting core plus the binding's host counter (resets) and `ak_enc_take`: payloads and, with `--corpus DIR --rows TSV`, the 92 U rows; drop and retain (R5, req 19) |
| `src/bench.cpp` | `bench_*` (levels, linkages, guard off, perturbation, crossing tax, LTO, and the `bench_a17_gateplant` plant) | the pre-campaign codec timing harness; its arms are gated before timing |
| `src/contentsets.cpp`, `src/concurrency.cpp`, `src/groupskip.cpp`, `src/utf8check.cpp`, `src/odr_*.cpp`, `src/fusion_probe.cpp` | `contentsets_a17`, `conc_*` (incl. four planted), `groupskip_*` (incl. two planted), `utf8check_*`, `odrcheck`, `fusion_probe` | content sets, concurrency suite, group skip, UTF-8 validator differential, ODR across levels, the boundary checker's control |
| `src/rpcbench.cpp`, `rpcflow.cpp`, `rpccounts.cpp` | `rpcbench`, `rpcflow`, `rpccounts` | the pre-campaign RPC grid, the flow-control probe, RPC crossing counts |
| `src/campaign_codec.cpp` | `campaign_codec`, `campaign_codec_nounk` | campaign codec suite (Google Benchmark), with its own gate and plant |
| `src/campaign_rpc.cpp`, `campaign_server.cpp`, `campaign_calib.cpp` | `campaign_rpc(_nounk)`, `campaign_rpc_count(_nounk)`, `campaign_server`, `campaign_calib` | campaign RPC client, cells A-F, directions a, a+read, b (two builds; `--warm-server N` warms the server; the `_count` builds print per-call counts for B-E with `--count N`); server in its own process on two Unix sockets (shipped, pinned), one per launch; crossing-cost loops |
| `src/upbbench.cpp` | `upbbench` (needs `gen/fetch_upb.sh`) | the upb ceiling arm |

Scripts (`gen/`):
- `wp5_gate.sh`: the correctness gate, both builds. It builds everything and refuses stale
  binaries, then runs:
  - the generator checks;
  - conformance at five levels/linkages plus the noinit plant;
  - the full corpus at four builds, with retention gaps bounded to `U-map-entry`;
  - decision 11's controls at four builds, and their plant;
  - corpus plants and `--compare`;
  - the oracle-probe rows;
  - the byte audit against the retired harness;
  - boundary and layout;
  - groupskip, concurrency, ODR, bench gates, content sets, crossing counts (against
    `logs/cpp/counts-baseline.log`), RPC counts;
  - `nounk_gate.sh`.
- `nounk_gate.sh`: the no-unknown build's own gate.
  - Each binary is checked to load the core of its variant.
  - Both headers are checked against both cores (`poc/rust/gen/c_variant.sh`, read-only).
  - Byte identity at C++17, C++11 and static.
  - The corpus at C++17 and C++11, with every unknown row written in its dropped form.
  - Plants, and counts against `logs/cpp/counts-nounk-baseline.log`.
- `d11_asan.sh`: both builds' conformance and corpus under ASan+LSan.
- `run_campaign.sh --suite codec|rpc|calib|gate --out <dir>`: the campaign runner (see the
  checklist).
- `campaign_summary.py`: requirement 30's summaries, ratios from per-launch medians.
- `u_rows.py`: the 92 U rows (accepted, non-disputed, at the seven shapes roots) from the corpus manifest, as the TSV the codec suite and counts read.
- `gbench_to_jsonl.py`: Google Benchmark JSON to section 7's lines.
- `corpus_all.py`: the corpus driver, with `--unk-controls`, `--expect-dropped`,
  `--max-retain-gap`, `--plant`, `--record` and `--compare`.
- Pre-campaign timing and probe scripts, which produce instrumentation only: `run_all.sh`,
  `rpc.sh`, `rpcflow.sh`, `contentsets.sh`, `concurrency.sh`, `utf8.sh`, `tax.sh`,
  `drift.sh`, `opt.sh`, `c16.sh`, `c24_timing.py`, `upb_ab.sh`, `calibrate.sh`,
  `fetch_upb.sh`.
- Checks: `boundary.sh`, `groupskip.sh`, `odr_check.sh`, `rd2_guard.sh`, `rd2_history.sh`,
  `refusal_test.py`, `audit_tracked.sh`, `wp5_bytes.py`.

## What was checked, and where the log is

From a fresh `git worktree` at `8f5b575c0`, with no uncommitted changes and new build
directories (`CLEAN=1 gen/wp5_gate.sh build`, then `gen/d11_asan.sh`):

| Check | Result | Log |
|---|---|---|
| build | every target configured and built from scratch; every gated binary newer than its sources | `wp5-build.log` |
| generator | `generate.py --check` every target current; guard: the shared C++ modules import plans only, glue imports no IR, a planted import is caught; the shared `--check` over every slice; `refusal_test.py`, `rd2_guard.sh`, `audit_tracked.sh`, `one_core.sh` and `one_core.sh --selftest` (0 controls failed to fire) | `wp5-generator.log` |
| payload byte identity, full build | 577 checks, 0 failures at C++17 target, C++17 floor, C++14, C++11 and static; the noinit plant fails its checks cleanly (exit 1, 283 failures; a crash no longer counts as the plant failing) | `wp5-conformance.log` |
| full corpus, full build | 702 rows, four builds (C++17, C++14, C++11, static): ffi 680/0, native 696/0, 6 disputed (excluded), 16 roots not in the C ABI; 2808 (row, arm) outcomes identical across the four builds; retain arms write the dropped form only on `U-map-entry`; plants proj/reenc/accept/noinit fail; `--compare` sees a planted difference | `wp5-corpus.log` |
| decision 11 controls | four builds: 686 rows, 2290 positions, 0 failing rows (pool = retain, drop = retain cleared, each position zeroed drops exactly it, map-entry bytes right); the plant fails 307 rows | `wp5-corpus.log` |
| oracle-probe rows | 11/11 on all four arms, C++17 and C++11 | `wp5-probe.log` |
| byte audit | native and ffi identical in outcome and bytes on 213 of 213 rows against the retired harness (`aba944a`) | `wp5-bytes.log` |
| boundary, layout | 23 checks, 0 failed; 574 corpus layout facts agree, shared and static | `wp5-boundary.log` |
| other gates | groupskip (with its two plants failing), concurrency (T7 off; the planted cores fail), ODR, bench gates and the gate plant, content sets, crossing counts 87 rows identical to `counts-baseline.log`, RPC counts | `wp5-gates.log` |
| no-unknown build | every variant binary loads a core with 0 u-family exports, every full one a core with them; both headers against both cores (matched agree, mismatched caught); 478 checks, 0 failures at C++17, C++11 and static (240 layout facts); corpus C++17 and C++11: ffi 680/0, native 696/0, 0 unknown rows written non-dropped, outcomes identical; plants fail; 87 count rows identical to `counts-nounk-baseline.log` | `wp5s10-nounk.log` |
| ASan + LSan | full build: conformance 577/0 (incl. the R-H7 options-reuse control), decision 11 controls 0 failing rows, corpus green; no-unknown build: conformance 478/0, corpus green with every unknown row dropped; 0 sanitizer reports | `asan.log` |

**Decision 11, as the owner confirmed it** (ABI-v1 rule 4 amended 2026-09-26): there is one
options entry per oneof, and the core fills it in the active member's decode group. The
binding takes the active message member's slot into that member's bag and frees any other
member's non-NULL slot. This is unchanged by the amendment.

## Crossing counts (R5, req 19; counts, not timings)

Every count is forward = core entry points + host-counted resets (`ak_enc_reset`,
`ak_dec_reset_<Root>`, counted in the binding under AK_COUNTING) + `ak_enc_take`, with the
breakdown on each row. Reset places: encode resets once before the encode; a retain decode
resets twice (before, arming the options; after, disarming); a drop decode makes none.
Retain: no pre-placed buffer, `unk_grow` allocates exactly the size requested.

- **Full build:** `logs/cpp/counts-baseline.log`, 485 rows: 117 payload rows (encode,
  its unbatched, zeroed-fill and host variants, decode, and retain decode and encode) and 368 U rows
  (92 rows x 4) (decode,
  encode, decode retain, encode retain). The core-counted part of the 87 payload rows is
  identical to the previous baseline; the re-take added the host and take columns and the
  retain and U rows (reason in its header).
- **No-unknown build:** `logs/cpp/counts-nounk-baseline.log`, 271 rows (no retain rows).
  It still differs from the full build in one core-counted payload row: P1.2 decode
  reverse, 8 full against 5 without unknown-field support (smaller decode groups, fewer
  arena chunks).
- **RPC cells, per call:** `logs/cpp/rpc-counts.log` (B, C-retain, C-drop, D-retain,
  D-drop, E-retain, E-drop) and `rpc-counts-nounk.log` (B, C/D/E-nounk), directions a and b,
  from `campaign_rpc_count(_nounk) --count 4`. Examples: B 2/0 both directions; C-drop a
  3/3501 (2 rpc + 1 decode), C-retain a 5/3501 (+2 resets); C b 2515 or 2521 forward (rpc 2,
  codec, 1 reset, 1 take); D is C without the 2 rpc calls; E is 2/0 (the codec is the
  host's). a+read adds no boundary call. The campaign gate diffs both files.
- **Pre-campaign RPC delivery counts:** `logs/cpp/rd2-rpccounts.log`. Blocking 2/0,
  callback 3/1, queue 4/0 forward/reverse per call, counting `ak_call_destroy`.

## Timing logs in the tree (instrumentation only)

These logs exist and are committed raw. They were taken in containers: two machines, a
4 vCPU Xeon at 2.80 GHz and another at 2.10 GHz, with no isolation and no governor
control. Most predate the port to the shared plan, decision 11, `init-guard` and the
facade's `unknown_fields` member, and none was re-taken after those changes. They are
listed so that nobody re-derives them. **No figure from them is quoted here.**

- `bench_*.log`, `drift.log`, `tax.log`, `opt.log`, `c16.log`, `c24-timing.log`,
  `utf8.log`, `contentsets.log`, `w10-one-core.log`: the pre-campaign codec bench and its
  controls (2.80 GHz).
- `upb.log`, `upb-fasttable.log`: the upb v25.3 ceiling arm (2.80 GHz).
- `rpc.log`, `rpcflow.log`: the pre-campaign RPC grid and the flow-control probe (2.10
  GHz). They were taken before the 6-field `ak_client_opts` existed (`rd2-history.log`).
- `calibration-r13.log`: the rust slice's crossing bench on the 2.80 GHz container.
- `campaign/*.jsonl`, `campaign/*.gbench.json`: campaign smoke runs.
  - `codec-*`, `rpc-*`, `calib-*`: 1 launch, 1 round, reduced sizes, both builds, at
    `a03b06ab2` (WP7), `"smoke": true`. Every timing is stripped (`"figures": "stripped
    (smoke)"`, gbench `*_time` = "stripped", the server's CPU line replaced). Rows: codec
    full 3820 and no-unknown 2388 samples (encode end x input, set=required/extra, row=U in
    three directions, `cpu_clock: process`); rpc 288 samples (A-F per mode, a, a+read, b,
    socket uds, shipped and pinned, one server for the launch).

## CAMPAIGN.md section 10 checklist

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | **met** on the runner's side: suites run one after another and nothing runs concurrently. The machine is the owner's |
| 2 | governor, turbo, SMT | **met (recorded)**: the header reads scaling_governor, intel_pstate/no_turbo, cpufreq/boost and smt/active. The runner does not set them (owner) |
| 3 | isolation | **met (recorded)**: `/sys/devices/system/cpu/isolated`, isolcpus/nohz_full from the kernel cmdline, the runner's cpuset |
| 4 | three disjoint CPU sets, one NUMA node, no shared SMT siblings, parameters; sizes fixed; worker thread counts in every header | **met** on the runner's side: AK_CPU_CLIENT and AK_CPU_SERVER are required (from `ffi/campaign.machine` through `ffi/campaign.sh`, which exports them; the sizes 4+4 are checked there, not here); overlap, a shared SMT sibling and a NUMA span are refused (`campaign/runner-controls.log`). Thread counts: the runner header carries a `threads` object; the codec header its process thread count; the RPC client header `caller_threads`, `core_runtime_workers` and `process_threads_after_warmup` (grpc-core sizes its own pollers, counted in the process total); the server's thread count is in READY and in its exit line. The OS set is "everything else" and is not checked |
| 3 (table) | incumbent at gRPC v1.54.0 and at a current version | **not met here**. The runner builds against the incumbent under AK_INCUMBENT_PREFIX (CMAKE_PREFIX_PATH, PKG_CONFIG_PATH and PATH, recorded as `incumbent_prefix` in every header); the campaign runs it twice, once with a gRPC v1.54.0 prefix (ArmoniK's level) and once with a current one. This container has only apt's grpc++ 1.51.1 / protobuf 3.21.12, so neither prefix exists and every run here used the system install. The owner provides the two prefixes |
| 5 | floors gated, no timing | **met**: the C++11 (and C++14) conformance and the C++11 corpus in the gate, for the full build; C++11 conformance and corpus for the no-unknown build |
| 6 | build flags printed | **met**: flags from the build, shared linkage, LTO off, core features, and the build variant (full or no-unknown) in every header |
| 7 | 16 payloads, content sets, U-* rows | **met**. Latin-1 and wide run on P1.2, P2.2 and P2.4, tagged `set=required`; P3.1, P4.1 and P6.1 run too, tagged `set=extra` (R-H26). The U-* rows are the 92 at the seven shapes roots (`gen/u_rows.py` from the manifest), tagged `row=U`, in all three directions: encode (host-gen re-encode as the check), decode and decode_read, for core-ffi drop and retain (retain in the full build only), host-gen and the incumbent (R-H27). Rows at other roots run only in the corpus build, for correctness |
| 8 | arms | **met**: incumbent-prod (grpc++ SerializationTraits), incumbent-best, core-ffi (push), host-gen. The pull family is not in this slice. The Rust-only arms are not applicable |
| 9 | encode, decode twice | **met**: `decode` and `decode_read` (the generated read-every-field traversal) |
| 10 | three modes for core-ffi and host-gen | **met**: core-ffi drop and retain in `campaign_codec`, no-unknown in `campaign_codec_nounk`; host-gen drop and retain in `campaign_codec`, no-unknown in `campaign_codec_nounk` (the drop rendering over the facade without `unknown_fields`, R-H22). The no-unknown build has no retain arm of either kind. Every sample carries `unknown_mode` and `build` |
| 11 | serialise once per iteration, fresh object; encode variants | **met**. Decode goes into a fresh object every iteration; protobuf C++ recomputes ByteSizeLong on every Serialize. Encode rows are tagged `end` and `input` (R-H29), graphs built in setup, outside the timed window. end=reused: bytes in a reused buffer (incumbent-best `SerializeToString` into a reused string, core-ffi `ak_enc_take` of the context's buffer, host-gen into its reused `ak::Enc`). end=transport: the form handed to grpc++ (incumbent-prod `SerializationTraits<Message>::Serialize` to a `grpc::ByteBuffer`; core-ffi and host-gen copy their bytes into a `grpc::Slice` wrapped as a `ByteBuffer`, what cells D and F do). Cell C hands the core's own buffer to the transport, which is the end=reused row, stated. input=hot: one graph; input=pool: distinct copies totalling `--pool-bytes` (runner: 2 x AK_LLC_BYTES, default LLC 13.75 MiB), round-robin |
| 12 | cells A-F; C, D, E, F in each mode | **met**: the full client runs A, B, C/D/E/F-retain and -drop; the no-unknown client runs A, B and C/D/E/F-nounk. E is host-gen over the core's transport, F host-gen over grpc++ (R-H35). A pre-run check in every C-F mode compares the decoded and re-encoded message with the incumbent's re-serialisation. Caller threads are created once and reused (R-H2) |
| 13 | server out of process, pre-serialised; one server per launch; one channel per cell | **met**. One `campaign_server` per launch serves both builds' clients and every cell on two Unix sockets (shipped and pinned configuration), warmed before round 1 by AK_CAMPAIGN_SERVER_WARMUP (default 200) calls per direction from each client transport (grpc++ and core) per socket (`--warm-server`), stated in the header. Each cell opens its own channel (a distinct channel arg for grpc++, its own `ak_client` for the core) at start and warms it. In direction b the server decodes with the incumbent in every cell |
| 14 | directions a, a+read and b | **met**: `a` (decode), `a+read` (decode, then read every field) and `b` (R-H36). The optional streamed upload is not built |
| 15 | 1, 8, 16 in flight | **met** |
| 16 | B and C blocking; A, D, F idiomatic | **met**. B, C and E use the core's blocking `ak_call_unary`; A, D and F use grpc++'s synchronous generic call, packages/cpp's idiom (R-H30), stated in the header's `delivery`. The callback and queue rows exist only in the pre-campaign `rpcbench` |
| 17 | shipped and pinned; Unix domain socket | **met**, stated. Every cell dials `unix:<path>` (grpc++ and the core, R-H28). grpc++ shipped = packages/cpp's channel args minus its retry service config. grpc++ pinned has no connection-window argument, so only the stream half is pinned. core shipped = ak_client_new defaults |
| 18 | every call checked | **met**: status and length on every call (cell A: content on every call, wire length once before the rounds); the first failure aborts and **leaves no sample** (R-H4): samples are buffered in the client and written only on success, and the runner deletes a failed launch file. The gate checks "no sample" for a wrong length, for an abort after two samples (`--fail-after 2`) and for the runner's discard. A failing `campaign_calib` is propagated the same way (R-H5) |
| 19 | crossing counts gate, every entry point, RPC B-E, retain | **met**. `counts_a17_shared` (and static) against `counts-baseline.log`, `counts_nounk` against `counts-nounk-baseline.log`: forward = core + host resets + `ak_enc_take`, with the reset's place in the header; payloads and the 92 U rows, drop and retain (retain: no pre-placed buffer, `unk_grow` allocates exactly the size asked). RPC cells B, C, D, E per call, per mode, directions a and b: `campaign_rpc_count(_nounk)` against `rpc-counts.log` / `rpc-counts-nounk.log`, compared in the campaign gate (R-H31) |
| 20 | crossing cost, fwd and rev, perf stat | **not met here**: perf is not installed. The runner builds and runs the rust slice's crossing bench, which did not build in the WP3 out-of-tree snapshot, and has not been re-run since. Reverse is reported as a fwd+rev row, from which the forward row is subtracted |
| 21 | process CPU per round | **met** for codec and RPC. Codec: Google Benchmark with `MeasureProcessCPUTime()`, every sample `"cpu_clock":"process"`, real_time beside it. RPC: getrusage(RUSAGE_SELF) of the client per round, plus wall. Calib (req 20's stand-in) still reads CLOCK_THREAD_CPUTIME_ID on one thread |
| 22 | order | **met**, stated. Codec: Google Benchmark's `--benchmark_enable_random_interleaving` plus registration order rotated by launch. RPC: the cell order of every (launch, round, dir, in-flight) group is a seeded shuffle, recorded as `order_pos`, and the two builds' binaries alternate by launch |
| 22a | benchmark engine | **met** for the codec suite: Google Benchmark v1.8.3, a Release build made by the runner from the upstream tag with its commit checked. Every repetition is exported raw and converted to section 7's lines. The RPC and calib suites stay on the runner, because a separate server process and abort-on-first-failure do not fit a Google Benchmark registration |
| 23 | 5 rounds x 3 launches, every round committed | **met** (runner defaults; the smokes used fewer, stated) |
| 24 | warm-up fixed and identical | **met**: a byte budget per codec arm and a call count per RPC cell, before round 1. Google Benchmark adds none (`--benchmark_min_warmup_time=0`). JIT is not applicable |
| 25 | allocator warmed identically | **met**: every arm's warm-up precedes round 1. GC is not applicable |
| 26 | correctness gate first | **met**: the campaign gate runs the full build's conformance, corpus, plants and counts, `nounk_gate.sh`, each codec binary's own gate and plant, and both RPC clients' length abort |
| 27 | header; dirty tree refused | **met**: a dirty tree is refused unless AK_CAMPAIGN_ALLOW_DIRTY=1 (smoke only). The header's `"instrumentation"` is true on a dirty tree or with AK_CAMPAIGN_SMOKE=1, which also sets `"smoke": true`, so a clean-tree smoke is marked (R-H19) |
| 28 | one JSON object per sample | **met**, plus a `build` field |
| 29 | logs in `ffi/logs/cpp/campaign/` | **met** |
| 30 | summaries only as specified | **met**: `gen/campaign_summary.py` keys on (build, arm or cell, payload, content, direction, mode, end, input, set, row) and forms ratios to incumbent-prod (end=transport for encode) or cell A from per-launch medians; no committed summary |
| 31 | runner interface; top-level `ffi/campaign.sh` | **met** for the slice runner. `ffi/campaign.sh` belongs to the aggregating session |
| 32 | smoke committed, marked | **met**: the WP5 step 10 smoke, figures stripped |

## Register H (WP6 re-review): findings for cpp, proposed dispositions

| Finding | Evidence | Proposed disposition |
|---|---|---|
| R-H7 options reused after a decode: freed buffers left in the options | reproduced under ASan before the fix: heap-use-after-free in `unk_put` on the second decode with the same options (`logs/cpp/rh7-before.log`) | **confirmed, fixed** (`95c399de9`): `decode_with_<root>_opts` untracks every buffer still in the options, so reclaim frees only buffers the core consumed and the binding did not deliver; `_pool` frees its own leftovers. The reproduction is kept as a conformance control (`d11_reuse`), clean under ASan (`asan.log`) |
| R-H4 aborted RPC run leaves samples; control checks only rc | the client printed each sample as it was taken and the runner appended its output; the control grepped nothing | **confirmed, fixed** (`7875a980b`): samples buffered and written only on success; the runner deletes a failed launch file; three gate controls check "no sample" (`campaign/gate.log`) |
| R-H5 `campaign_calib` failure not propagated | the runner ignored its exit status; calib had no failure path | **confirmed, fixed**: calib refuses malformed arguments (exit 2) and writes only after every round; the runner propagates and deletes the file; gate control (`campaign/gate.log`) |
| R-H22 no-unknown keeps the facade member (owner decision) | `unknown_fields` rendered unconditionally | **done**: `cpp_facade` omits it under `unknown_compiled_out`; variant facades under `nounk/src`; host-gen no-unknown arm; retain arms not built in that build; full build unchanged |
| R-H2 RPC threads spawned per batch inside the window | `batch()` created k threads per call | **confirmed, fixed**: a pool created before the rounds, one condition-variable round trip per thread in the window |
| R-H23 order of arms (owner decision) | codec already used Google Benchmark's random interleaving (22a); RPC rotated with one schedule | **done**: codec unchanged (stated); RPC seeded shuffle per (launch, round, dir, in-flight) with `order_pos` (covers C++'s part of R-H18) |
| R-H19 `"instrumentation"` true only on a dirty tree | `run_campaign.sh` header | **confirmed, fixed**: AK_CAMPAIGN_SMOKE=1 marks a clean-tree smoke (`"smoke": true`) |
| R-H15 `cpp_layout.py` tests `options.unknown == "drop"` | as reported | **fixed by the rust agent** in `31fc3eecf` (with `c_abi.py`); `cpp_native.py`'s `unknown` argument is the host-gen mode, not the ABI variant, and was left |

Found while gating, both under the performance-scope rule's gate clause (a gate that would
let a wrong output be timed):
- The noinit plant aborted with a double free (`logs/cpp/rh7-noinit-doublefree.log`). When a
  reset was refused, `decode_with_*_opts` returned before leaving the options' buffers to the
  host. Fixed (`b915dc107`), with a control (a wrong root through the pool decode). The gate
  now requires the plant to exit 1, so a crash counts as a FAIL.
- `nounk_gate.sh`'s dropped-form control ran on the no-unknown build's
native-retain arm, which R-H22 removed, so the control could no longer fail. The gate at
`3a211100c` reported it as blind; it now runs on the full build's native-retain
(`8f5b575c0`).

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| C6 | this container | grpc++ 1.51.1 and protobuf 3.21.12 are apt's; `packages/cpp` pins neither, and CAMPAIGN.md asks for v1.54.0 and a current version | open; this container cannot fix it |
| C15 | `src/bench.cpp` | the `groupfill` arm has measured larger than the (`ffi` - `native`) delta it is a component of, on P1.3 (instrumentation, `bench_a17_shared.log`) | open; the direct-call hypothesis is refuted (JOURNAL). `groupfill` is labelled an upper bound |
| C40 | `design/ABI-v1.md` section 5 vs `ak-abi` | `ak_err` is `{code, msg_len, msg}` in the specification's text and `{code, detail}` in the core; the C header follows the core | open, for the aggregating session |

The retired defects C1-C37, R-D1, R-D2 and R-G7 were fixed, or were closed by their owners,
and the record is in JOURNAL.md. The items reported against other owners were re-checked on
2026-09-26 and are closed:
- C19: the file no longer exists.
- C25: `U-map-entry` is now a disputed row.
- C26: `B-P7_1` now has `permutation_accepted`.
- C28: SHAPES.md's window table is corrected.
- C29: ABI-v1 section 9's delivery table no longer carries the counts.
- C38: `one_core.sh --selftest` passes in this gate (0 controls failed to fire), though
  `wp5_gate.sh` still labels that step as a known defect.
- C39: `rust_core.py` is named only in a comment.

## What is not measured

**Timing, in general.**
- **Any timing on the campaign machine.** Every timing in the tree is container
  instrumentation (above).
- **Any timing of the current tree.** No timing log was re-taken after the port to the
  shared plan, decision 11, the no-unknown build, `init-guard` and the facade's
  `unknown_fields` member.
- **The price of unknown-field support.** The campaign codec suite has core-ffi drop,
  retain and no-unknown, and the RPC grid has C/D per mode, but the only runs so far are
  smokes with their figures stripped.
- **The copy of a retained bag into `std::string`.** Rust adopts the core's buffer; C++
  copies it, because the facade type is `std::string`. This is not priced.

**Unknown fields.**
- **Map entries.** A map entry's unknown fields have no facade bag. They are counted,
  freed and dropped (the `U-map-entry` retention gap, a disputed corpus row).
- **Rule 5** (every capacity capped at INT32_MAX, amended 2026-09-26, core `31fc3eecf`):
  not exercised from C++. It needs a message whose unknown runs exceed 2 GiB; the core's
  own unit tests cover `unk_room`.
- **host-gen retain in the no-unknown build**: not built (its facade has no bag), so the
  no-unknown corpus reports native-retain and ffi-retain NOT BUILT.
- **The RPC server side.** The server decodes with the incumbent in every cell, so no
  cell runs the core's decoder on the server.

**Coverage.**
- **The pull decode family.** It is not rendered for C++.
- **A lossy (U+FFFD) decode policy** in the C++ native codec. It is not rendered, and the
  backend raises.
- **The chunking class, `Surrogate` and similar roots in timing.** The corpus build runs
  every row for correctness only. The timed codec covers the seven shapes roots.
- **CONTRACT.md C5 (produce)** for the `E-*` and `S-*` rows. It needs the corpus's value
  rules in C++. The 8 `baseline` rows are produced, by `conformance`.
- **Nesting past depth 3** and P7.1 being decode-only: gaps inherited from the payload set.
- **A C++20 coroutine surface.** A sketch only, not built.

**Campaign requirements this container cannot meet.**
- **The two incumbent versions** CAMPAIGN.md asks for (C6).
- **`perf stat` cycles and instructions** (req 20): perf is absent.
- **The rust slice's crossing bench** (R13, req 20): not re-run on this container since
  2026-09-24.

**Sanitizers and allocation.**
- **A thread sanitizer run.** The core is a Rust cdylib built without TSan, so the result
  would be noise.
- **Allocation counts and peak memory.** Nothing counts them.
- **Concurrency on the RPC half, and cancellation.** The RPC half has no byte-checked
  suite under contention and no plant. `ak_call_cancel` is exported and counted but never
  called.
- **Streaming, TLS, retry, deadlines, metadata** on the RPC path.
- **A second compiler.** clang++ 18 is installed and unused for the gates.

**Other open questions.**
- **When glibc's mmap threshold adapts**: the residual of C16. The cause of the P1.2
  outlier round was demonstrated by removal (`c16.log`); its timing is not predicted.
- **What a `protoc-gen-upb` minitable would add** above upb's generic decoder: it needs
  Bazel.

## Next step

Nothing is queued for this slice. The open items belong to others: the incumbent versions
and perf (the owner's machine), C40 (the aggregating session), and whether the RPC server
should decode with the core in C/D cells (a harness question for the aggregating session).
To re-run the gate: `CLEAN=1 gen/wp5_gate.sh build` (it builds everything, about 40
minutes here), then `gen/d11_asan.sh`. `gen/run_all.sh` takes timings and is not a gate.

## Log index

Current gate (clean checkout at `a03b06ab2`):

| Log | What it contains |
|---|---|
| `wp5-build.log` | generate, configure and build every target, freshness check |
| `wp5-generator.log` | `generate.py --check` and the one-generator guard, the shared `--check`, `refusal_test.py`, `rd2_guard.sh`, `audit_tracked.sh`, `one_core.sh` |
| `wp5-conformance.log` | payload byte identity at C++17 target/floor, C++14, C++11, static (full build), decision 11's cases; the noinit plant |
| `wp5-corpus.log` | full corpus x4 builds, retention-gap bound, decision 11 controls x4 and their plant, corpus plants, `--compare` |
| `wp5-probe.log` | the rust slice's oracle-probe rows, four arms |
| `wp5-bytes.log` | the C++ arms before the port (`aba944a`) against after, row by row |
| `wp5-boundary.log` | `boundary.sh` (R5 from the artifact), corpus layout facts |
| `wp5-gates.log` | groupskip, concurrency (T7 off), ODR, bench gates and plant, content sets, crossing counts (+ retain rows), RPC counts |
| `wp5s10-nounk.log` | the no-unknown build's gate |
| `asan.log` | both builds under ASan+LSan |

Committed references and earlier correctness logs:

| Log | What it contains |
|---|---|
| `counts-baseline.log`, `counts-nounk-baseline.log` | the committed crossing counts, full (payloads and U rows, drop and retain) and no-unknown |
| `rpc-counts.log`, `rpc-counts-nounk.log` | the committed per-call RPC counts, cells B-E per mode, directions a and b |
| `wp3-gate-count-stop.log` | the campaign gate stopping on the P1.2 count change decision 11 caused |
| `wp5s9-asan.log` | the WP5 step 9 ASan run (full build only), superseded by `asan.log` |
| `d38-probe-before.log`, `d39-stale-refusal.log` | D38 (field number above 2^29-1 in a skipped group) before the fix; D39's stale-binary refusal control |
| `rd1-lenwrap.log`, `rd2-guard.log`, `rd2-history.log`, `rd2-rpccounts.log`, `rd5-gate.log`, `rd5-before.log`, `rd7-before.log`, `rd-generator.log` | the R-D1, R-D2, R-D5 and R-D7 findings, before and after |
| `groupskip.log`, `odr.log`, `boundary.log`, `concurrency.log`, `conformance.log`, `generator.log` | earlier runs of checks the current gate re-runs (C24's group skip, the ODR check, R5, the concurrency suite, R2); superseded by the `wp5-*` logs |
| `corpus.log`, `corpus-native.log` | the retired subset corpus harness; superseded by `wp5-corpus.log` |
| `campaign/gate.log`, `campaign/counts.log`, `campaign/rpc-counts.log`, `campaign/rpc-counts_nounk.log`, `campaign/runner-controls.log`, `campaign/campaign_unknown_rows.tsv` | the campaign gate of the last smoke, its payload/U and RPC counts, the runner's CPU-set/dirty-tree refusals, the U-* rows the codec suite times |

Timing logs (instrumentation only): see "Timing logs in the tree" above.
