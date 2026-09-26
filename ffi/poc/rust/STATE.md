# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what is true now;
the history is in `JOURNAL.md`. Phase (README section 1.1): setup and design. Every timing
this slice has taken is **container instrumentation**, not a result, and none is quoted
here. This file states what exists and what was checked; the choice is the owner's.

| | |
|---|---|
| **Status** | Built and gated: four codec arms plus the pull family, the RPC grid (cells A-F), the full conformance corpus through the C ABI and core-native, decision 11's unknown-field mechanism, the no-unknown build, and the campaign harness conformed to the 2026-09-26 contract (FIX-PLAN WP7: process CPU, content sets on P1.2/P2.2/P2.4, encode variants, cells E and F, one server per launch on a Unix socket, `a`/`a+read`, counts with resets and per-call RPC rows, worker thread counts). The gate passes from a clean worktree at c8e8694eb for both builds on stable 1.94.1 and on the MSRV floor 1.88.0, ThreadSanitizer included, and the smoke run shows every new row with its figures stripped (`logs/rust/campaign-wp7/`) |
| **Next step** | none assigned |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | MSRV 1.88.0: the full gate, both builds, passes on rustc 1.88.0 from a clean worktree at c8e8694eb (`logs/rust/campaign-wp7/gate-floor-1.88.log`) |
| **Target** | stable 1.94.1 in this container; README section 5: for Rust the floor is the target language level, one configuration |
| **Incumbent** | prost 0.14.4, tonic 0.14.6, tonic-prost 0.14.6 (from Cargo.lock, printed in every campaign header). R14: tonic-prost's codec calls `Message::encode`/`decode`, so the production path and the library entry point are the same call |
| **Questions this slice has open for the aggregating session** | (1) the proposed corpus rows of `gen/probe_corpus.py` (field numbers above 2^29-1, the 10th varint byte, two map-order rows) are not in `corpus/`; (2) no corpus row or payload has a repeated singular message with differing content, so merge-on-repeat (R-E4) is rendered and never observed; (3) a map entry has no unknown-field bag in the Rust facade (D42) |

## What exists

### The shared generator (`poc/codec/gen/`)

```
ir.py            the front end: shapes.json and the corpus reader view. Only plan.py imports
                 it (plus forwarding names at ir.py:183 that nothing in this slice uses).
plan.py          the RULE LAYER: IR -> MessagePlan (encode and decode plans), the ABI layout
                 functions, plan.FIXED (the fixed ABI), plan.rpc, plan.lifecycle, decision 11
                 (positions, options layout, entry points). Options(unknown, utf8,
                 recursion_limit); on the C ABI unknown="both" is decision 11's mechanism and
                 unknown="drop" is THE NO-UNKNOWN VARIANT (unknown_compiled_out).
rust_abi.py      abi.rs, codec.rs (push and pull), the RPC and FIXED regions of
                 ak-abi/src/lib.rs, rpc_check.rs, abi_check.rs
rust_native.py   core-native, one module per unknown-field mode
rust_binding.py  this slice's host binding (binding.rs, binding_nounk.rs)
rust_facade.py   the facade types (types.rs; types_nounk.rs without `unknown_fields`) and the
                 `armonik` arm's prost impl, from the plan's encode steps (R-H12; moved here
                 from poc/rust/gen)
cpp_layout.py    section 10's layout export (layout.rs), per variant
c_abi.py         the one C header backend (ak_abi.h, ak_layout.h, ak_layout_names.h), per
                 variant; the no-unknown header defines AK_NO_UNKNOWN_FIELDS 1
cpp_*, java_*, cs_*, py_*   the other slices' backends (not this slice's)
generate.py      writes every core file (shapes core, corpus core, both no-unknown variants),
                 then runs every slice's generator; --check adds drift, the import guard over
                 the 27 backend modules, and the slice guard over every poc/<slice>/gen/*.py
                 (no front-end import, no wire-code emitter outside a listed plan renderer, no
                 renamed copy; R-H13), each with planted violations that must be caught
one_core.sh      R0's check; --selftest plants each violation and requires it to fail
```

Core crates (`poc/codec/crates/`): ak-abi, ak-rt, ak-core (cdylib + staticlib), rpc.
Features: `unknown-fields` (default on; off = the no-unknown variant from
`src/generated_nounk/`), `corpus` (the corpus reader schema's core, `generated_corpus*/`),
`init-guard`, `count`, `rpc`, `dec-reject*`, `global-widths`, `pad-widths`.

### This slice (`poc/rust/`)

```
gen/generate.py        this slice's generator (facade, payloads, prost impl, and through the
                       shared backends core_native*.rs, binding*.rs, the corpus harness files,
                       the campaign table)
gen/gate.sh            THE CORRECTNESS GATE (nothing timed): steps 0-12 below; --tsan adds
                       ThreadSanitizer
gen/corpus.sh          the corpus on the full build (4 arms) and on the no-unknown build
                       (3 arms), each with its planted controls
gen/c_variant.sh       both C headers against both cores (section 10's check from a C++ host)
gen/tsan.sh            the concurrency suite under ThreadSanitizer (nightly) with a planted race
gen/crossings.txt      committed crossing counts, full build (drop, retain)
gen/crossings-nounk.txt  committed crossing counts, no-unknown build
gen/check_direct.py    generator-time refusals: direct fields (ABI v1 section 8), packed fixed32 (R-H15)
gen/facade_order.py    the armonik arm's encode order on a planted description (R-H12)
gen/oracle_probe.py, probe_corpus.py, probe_pycodec.py, corpus_before.py
gen/stage*.sh, pull.sh, concur.sh, lifecycle.sh, ...   stage harnesses (timing sections
                       are instrumentation)
run_campaign.sh        the campaign runner (CAMPAIGN.md req 31)
crates/facade, harness (bins conformance, shapes, counts, content, contentall, concur,
                       lifecycle, pullbench, rdrepro, stickyerr, and stage bins),
       campaign (benches/codec_suite.rs; bins rpc_server, rpc_client, calib, crossings),
       shapes-prost, shapes-values, stage1-validate
corpus/                a workspace of its own (the core with `corpus`): facade, harness
                       (the `corpus` driver, each row in a child process under a timeout)
```

Codec arms: `incumbent-prod` (prost through tonic's codec calls), `armonik`
(`packages/rust`'s facade + generated prost impl), `core-native` (drop and retain),
`core-ffi` (push, through the C ABI), `core-ffi-pull`. Corpus arms: full build `ffi-drop`,
`ffi-retain`, `native-drop`, `native-retain`; no-unknown build `ffi-nounk`, `native-nounk`
(core-native's drop rendering over the facade without `unknown_fields`; no retain
rendering exists in that build, R-H22). RPC cells (`crates/campaign/src/grid.rs`): A prost +
tonic, B prost + core transport, C core-ffi + core, D core-ffi + tonic, E core-native + core,
F core-native + tonic; C-F in retain and drop (full build) or no-unknown (no-unknown build).

## What was checked

**The gate from a clean checkout after WP7**: a fresh worktree at c8e8694eb (0 changed
paths, no build directory reused), `run_campaign.sh --suite gate`
(`logs/rust/campaign-wp7/gate.log`, header with a clean commit): GATE PASSED, both builds.
The codec pre-check now covers the encode variants (full build 3,478 checks on 114 inputs
and 4,212 cases; no-unknown 2,126 checks, 2,624 cases); crossing counts identical to the
new committed files (701 and 354 lines, the RPC rows included). The same gate under
`RUSTUP_TOOLCHAIN=1.88.0`: GATE PASSED (`gate-floor-1.88.log`). ThreadSanitizer: 0 warnings
in the suite, 190 on the planted race (`tsan.log`). Disk peaked at 82% during the smoke,
41-47% after the worktree was deleted (`df.txt`).

**The gate from a clean checkout after register H**: a fresh `git worktree` at 766f8dcd9 (0
changed paths, no build directory reused), `run_campaign.sh --suite gate`, header with a
clean commit (`logs/rust/wp6h/clean-gate/gate.log`): GATE PASSED. What differs from the
d2cd0b02f run below: step 1 adds the slice guard (46 slice modules, 3 plants caught),
check_direct.py (0 wrong, packed fixed32 refused) and facade_order.py (tag order); step 2
runs 6 ak-core tests (the R-H21 ones included); step 11's decision 11 controls add
"refused parse keeps A" (480 record bytes before and after), "decided at arm time" (rc 0,
no bag; the twin armed at reset gets 3 bytes) and "oneof stamp -> int" (the scalar, no bag,
1 buffer reclaimed, 0 live); the no-unknown corpus runs ffi-nounk 680/0 and native-nounk
696/0. Crossing counts identical to both committed files (no count changed, so neither
file was touched). Disk 53% -> 72% -> 53% (`df.txt`).

**The gate from a clean checkout** (WP6 step 1): a fresh `git worktree` at d2cd0b02f
(origin HEAD, 0 changed paths, no build directory reused), `run_campaign.sh --suite gate`,
whose header records the commit with no "uncommitted" mark (`logs/rust/wp6-clean-gate/gate.log`):

| Step | What | Result (line in `gate.log`) |
|---|---|---|
| 1 | generators current, one core | `generate.py --check` every slice, guard over 26 backend modules with its planted import caught; one_core clean |
| 2 | core unit tests (debug build) | 13 passed, 0 failed |
| 3, 4 | byte identity on the 16 payloads (P1.3, P2.5 included); presence, oneof, unknown-field vectors | VERDICT pass on every arm (lines 93, 120) |
| 5 | floors gated | met in this container: Rust's floor is MSRV 1.88.0, the same configuration as the target; the full gate, both builds, passes on 1.88.0 from a clean worktree at c8e8694eb (`logs/rust/campaign-wp7/gate-floor-1.88.log`). `run_campaign.sh --suite gate` runs the stable toolchain; the floor run is `RUSTUP_TOOLCHAIN=1.88.0 bash gen/gate.sh` |
| 6 | content sets, every payload | every payload and set "ok" (identity against the incumbent; timing sections skipped) |
| 7 | payloads | met: the 16 payloads; Latin-1 and wide content sets on P1.2, P2.2 and P2.4 (req 7 as amended, R-H26); the 92 accepted non-disputed `U-*` rows at the shapes core's 7 ABI roots, three directions, through the timed shapes core (R-H27 as answered; 114 inputs). P7.1 decode only (SHAPES.md) |
| 8 | lifecycle, guard off and on | VERDICT pass (lines 948, 1023) |
| 9, 10 | R-D1 length-wrap reproductions (and the depth-2 error); R-D6 sticky error slot | every case returned promptly (0 hangs, aborts or crashes) and the depth-2 error came back as an error on both arms; R-D6 7 of 7 ("ALL PASS") |
| 11 | serialised once per iteration; encode variants | met: prost recomputes `encoded_len` on every encode, the core encoders reset per call, no size memo. Every encode arm x mode has four labelled rows (R-H29): end_state `reused-buffer` or `transport-ready` (incumbent-prod and armonik: a frozen `Bytes` split from a reused `BytesMut`, tonic's encode buffer; core-native and core-ffi: a `Bytes` copy of their buffer, what cells F and D hand tonic; over the core's transport the form is the reused buffer itself) x input `hot` or `pool` (graphs cloned until the heap they hold, measured with mallinfo2, reaches `AK_POOL_BYTES` = 2 x `AK_LLC_BYTES`, 13.75 MiB by default; built before the case's warm-up and freed after; rows carry `pool_graphs`, `pool_heap_bytes`). A pre-check requires every variant to write the hot row's length |
| 11 | decision 11 controls | 543 rows, 2,290 (row, position) pairs; zeroing a position drops exactly that position (0 mismatches; 307 rows carry unknowns); pull == push; pool with and without grow, in-place refill and its no-refill control, the oneof switch (one position, the buffer in the active member's group: ABI-v1 rule 4 as amended), wrong root refused with -8: all PASS; the planted "bags not cleared" fails on 307 rows |
| 11 | corpus, no-unknown build | ffi-nounk 680 / 0, native-drop and native-retain 696 / 0 (at d2cd0b02f; since R-H22 the build has ffi-nounk and native-nounk, see above); 0 retained-form lines from ffi-nounk; the four controls fail as required |
| 11b | codec harness pre-check, full build | 962 checks, 112 inputs, 2,296 cases, 0 failures |
| 11c | crossing counts vs `gen/crossings.txt` | 671 lines identical |
| 12 | cells A-F; C-F per mode | met: A, B, C, D, E (core-native over the core's transport), F (core-native over tonic); full client C-F in retain and drop, no-unknown client C-F in no-unknown, A and B in both |
| | verdict | GATE PASSED; the runner re-ran both crossing-count comparisons after it (identical) |

ThreadSanitizer in the same worktree (`tsan.log`, nightly 1.100): 0 warnings in the
concurrency suite; 123 on the planted shared-context race.

The floor, 1.88.0, in the same worktree (`gate-floor-1.88.log`): the same gate, steps 0-12, under `RUSTUP_TOOLCHAIN=1.88.0`
(rustc 1.88.0 printed by the gate's own steps): GATE PASSED, with the same results as on
stable -- byte identity and shapes VERDICT pass, both corpora (680 / 696 / 696 per build),
pre-checks 962 and 520, crossing counts identical to both committed files, both header
pairs agreeing and both mismatches caught. The Cargo.lock builds unchanged on 1.88.0.

Disk (`df.txt`): 42% before the builds, 66% at the peak, 41% after the worktree and
every build directory in it were deleted. The stale `target-*` directories of the main
checkout were deleted before the run.

**Evidence kept from earlier units, still current** (the code it checks has not changed
since; each log is committed):

- Oracles on the 10th varint byte and field numbers above 2^29-1 (`logs/rust/wp5s6-oracles.log`):
  all three (upb, pure-python protobuf, protobuf C++) accept the 10th byte's excess bits,
  and the plan discards them; upb and protobuf C++ refuse field numbers above 2^29-1,
  pure-python accepts them, and the plan refuses them.
- The no-unknown crossing-count diff (`logs/rust/wp5s10/counts-diff.txt`): 335 rows per mode;
  only P1.2, P1.2/latin1 and P1.2/wide decode differ from the full build's drop mode
  (reverse 8 there, 5 in the no-unknown build, which is the value before decision 11,
  `logs/rust/wp5s6-gate.log` line 118).

### Crossing counts: what they cover

`crossings` (counting build) counts, per timed core-ffi case (every input x encode /
decode / decode-pull x mode), **every exported entry point the loop calls** (CAMPAIGN req
19 as amended, R-H31): the core's per-context counters plus the plain exports the binding
tallies in the counting build (`ak_enc_reset`, `ak_dec_reset_<Root>`, `ak_enc_take`,
`ak_dec_err`). A `resets` column says how many of the forward calls are resets and where:
one `ak_enc_reset` before every encode; two `ak_dec_reset_<Root>` around a retain decode or
pull (before, arming the options; after, disarming with NULL). Retain runs with no
pre-placed buffer and a grow that allocates exactly the size requested. `rpc:<cell>` rows
count ONE call of cells B, C, D and E per direction and mode (P2.2, in-process server on a
Unix socket), the core's RPC counters included. `gen/crossings.txt` 696 rows,
`gen/crossings-nounk.txt` 349 rows; the change from the pre-WP7 files (every existing
row's reverse unchanged; forward +1 per encode, +2 per retain decode, +1 / +3 per pull;
new rows for P2.4's content sets and the RPC cells) is in
`logs/rust/wp7/crossings-change.txt`. Not counted: `ak_dec_ctx_new_<Root>` /
`ak_enc_ctx_new` (setup, outside the timed call); cells A and F have no core crossing.

## Campaign readiness (design/CAMPAIGN.md section 10)

Runner: `poc/rust/run_campaign.sh --suite codec|rpc|calib|gate --out DIR`, reading
`AK_CPU_CLIENT` and `AK_CPU_SERVER`. The full build lives in `target/`, the no-unknown
build in `target-nounk/` (a shared target overwrites `libak_core.so`); the runner checks
each binary loads the core of its variant. Latest smoke run: `logs/rust/campaign-wp7/` at
c8e8694eb (every suite; figures stripped).
Every figure in both is instrumentation.

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | met by the runner (one measured process at a time); the machine and the sequencing across slices are the owner's |
| 2 | governor, turbo, SMT | met as recording: read from sysfs into every header; setting them is the owner's (this container: n/a) |
| 3 | isolation | met as recording: isolcpus/nohz_full/rcu_nocbs from /proc/cmdline and the cgroup cpuset in every header; the mechanism is the owner's |
| 4 | three disjoint CPU sets, fixed size, thread counts | met: `AK_CPU_CLIENT`/`AK_CPU_SERVER` from the environment (ffi/campaign.sh exports them from ffi/campaign.machine, which fixes the size at 4; run alone, the runner reads that file when they are unset); `taskset -c` on the measured process and the server. Every header records each stack's worker threads (criterion: 1 measuring thread; RPC client: tokio 2 workers per A/D/F cell, `ak_runtime_new(2)` per B/C/E client, k = 1/8/16 callers; server: tokio `AK_SERVER_THREADS` = 4). Disjointness and SMT siblings are checked by campaign.sh, not by this runner |
| 5 | floors gated | met in this container: Rust's floor is MSRV 1.88.0, the same configuration as the target; the gate on 1.88.0: verified: the full gate, both builds, passes on rustc 1.88.0 from the clean worktree (`gate-floor-1.88.log`). `run_campaign.sh --suite gate` runs the stable toolchain only; the floor run is `RUSTUP_TOOLCHAIN=1.88.0 bash gen/gate.sh` |
| 6 | build flags printed | met: release, opt-level 3, lto off, codegen-units default, core as a cdylib through the dynamic linker, core features (rpc, init-guard, and unknown-fields or not), harness guard on, transcoder |
| 7 | payloads | met: the 16 payloads, the latin1 and wide content sets on P1.2 and P2.2, and the 92 non-disputed `U-*` corpus rows whose root is one of this slice's 7 ABI roots (112 inputs). P7.1 decode only (SHAPES.md) |
| 8 | arms | met: incumbent-prod (tonic's codec calls); incumbent-best is the same entry point (R14), so no second row; core-ffi push, core-ffi-pull as a labelled extra; host-gen = core-native; armonik |
| 9 | directions | met: encode, decode, decode-read (a generated visitor reads every field of both object models) |
| 10 | three unknown-field modes | met: full build: core-native, core-ffi, core-ffi-pull in retain (every options position armed) and drop; no-unknown build, a separate binary, whose facade has no `unknown_fields` member (R-H22): the same arms in no-unknown, with incumbent-prod and armonik as in-process controls. Incumbent in prost's default (drop), stated. On 27 `U-wire-*` rows prost refuses a known field at a foreign wire type; those (row, incumbent/armonik) pairs are not timed and are listed in the codec log header. The no-unknown build has its own committed counts and its own gate (gate step 12, corpus.sh section 6) |
| 11 | serialised once per iteration | met: prost recomputes `encoded_len` on every encode, the core encoders reset their context per call, and no Rust object carries a size memo |
| 12 | cells A-D; C, D in three modes | met: full client A, B, C-retain, C-drop, D-retain, D-drop; no-unknown client C-nounk, D-nounk with A and B as in-process controls; same server |
| 13 | separate server, pre-serialised, one per launch, warmed; one channel per cell | met: `rpc_server` on `AK_CPU_SERVER`, P2.2 pre-serialised and checked against the manifest hash; ONE server per (transport, launch) serving both builds' clients; each client warms it by `AK_RPC_SERVER_WARMUP` (64; smoke 16) checked calls from each of its transports before round 1; one channel per cell per launch, opened and warmed before that cell's rounds; direction (b) decoded by prost |
| 14 | directions a (as a and a+read) and b | met: `a` = Fetch and decode, `a+read` = Fetch, decode, read every field (the codec suite's generated visitor), `b` = encode and Push; the optional streamed upload is not built |
| 15 | 1/8/16 in flight | met: B, C, E: k host threads created once per (cell, dir, k) before the warm-up and reused (R-H2); A, D, F: k tokio tasks |
| 16 | B/C blocking; A/D/F idiomatic | met: B, C and E use the core's blocking `ak_call_unary`; A, D and F use tonic's async unary call from k tokio tasks on a 2-worker runtime, the shape of packages/rust's client (stated in the header). Callback and queue deliveries are not in the campaign runner (stage 6 harness `rpcgrid` only), stated |
| 17 | shipped and pinned; Unix socket | met: every cell over a Unix domain socket (R-H28; the core dials `unix:`, tonic serves a `UnixListenerStream`); shipped = tonic endpoint and server defaults, `ak_client_new`; pinned = 4 MiB stream and connection windows, adaptive off, on tonic, the core client and the server (Nagle does not apply to a Unix socket) |
| 18 | every call checked, abort | met: status and response length on every call (the server warm-up included), C-F also their decode; the first failure exits with no output file; the planted wrong length is run per transport and per client binary and must abort with no file |
| 19 | crossing counts gate | met: see "Crossing counts: what they cover" (resets and every exported call included; RPC cells B-E per call; retain with no pre-placed buffer and exact grow); both files compared in the gate and by the runner before codec and calib; a difference stops the run |
| 20 | crossing cost fwd/rev, perf stat | **not met**: `calib` reports forward (`ak_noop`) and forward+reverse (`ak_noop_reverse`) in the same round; `perf stat` cycles and instructions are collected by the runner when perf exists, and **perf is not installed in this container** (the smoke's `calib-perf-launch1.txt` says so). Needs the campaign machine with perf |
| 21 | process CPU per round | met: codec and calib `CLOCK_PROCESS_CPUTIME_ID` per criterion sample / round (a custom criterion Measurement, R-H25); RPC `getrusage(RUSAGE_SELF)` of the client per round with wall beside it. The codec suite records no wall time (not required), stated |
| 22 | order randomised as far as the engine allows | met: codec in blocks by arm, arm blocks and the cases inside each block in a seeded random order per launch (seed = launch, in the header; criterion runs benchmarks in the order the suite registers them), the two builds alternated by launch; RPC cells in a seeded random order per launch, directions and in-flight counts nested inside a cell, the two clients alternated; calib's two arms alternate |
| 22a | benchmark engine | met: the codec suite runs on criterion 0.5 with every sample exported from criterion's `sample.json` to JSON lines; the RPC grid and calib stay on the runner (req 18's abort-with-no-output and req 13's separate server process, and one RPC sample is a batch of k concurrent calls timed with process CPU), stated |
| 23 | 5 rounds x 3 launches, every round committed | met by default: codec 10 criterion samples per case (criterion's floor) x 3 launches; rpc and calib 5 rounds x 3 launches. The smoke is 1 launch, 1 round |
| 24 | warm-up stated, identical | met: codec a fixed iteration count per case (100; smoke 3) then criterion warm-up (500 ms; smoke 5 ms), identical per arm, after a pool case's graphs are built; rpc: the server warm-up above, then 64 calls per (cell, dir, in-flight) (smoke 16); calib iters/10. No JIT, no GC |
| 25 | allocator warmed, GC stated | met: the fixed warm-up runs each case's own allocations before timing; no GC (Rust), stated |
| 26 | correctness before timing | met: every timing suite requires a gate PASS for the same tree-id (runs the gate otherwise); the codec process re-checks every timed arm on every input and every encode variant before criterion starts (full build 3,478 checks, no-unknown 2,126) |
| 27 | header | met: commit and tree-id (a dirty tree is refused unless `AK_ALLOW_DIRTY=1`, which is recorded), machine, SMT, governor, turbo, kernel, isolation, CPU sets, rustc/cargo, incumbent versions, build flags, core variant and features, transport, warm-up, repeats |
| 28 | JSON lines, raw | met: one line per criterion sample, RPC round and calib round, with the req-28 field names (`unknown_mode` included) plus `end_state`, `input`, `pool_graphs`, `pool_heap_bytes` on encode rows |
| 29 | logs per suite and launch in `ffi/logs/rust/campaign/` | met by the runner (one file per suite, transport, build and launch under `--out`; campaign.sh points it at `ffi/logs/rust/campaign/run-<stamp>/`). The latest smoke is in `logs/rust/campaign-wp7/` |
| 30 | summaries (ratios from per-launch medians) | not applicable: this slice produces no summaries (optional); each launch file holds every round, so per-launch medians can be formed from it |
| 31 | runner interface | met for the slice; `ffi/campaign.sh` is not this slice's file |
| 32 | smoke run | met: `logs/rust/campaign-wp7/` at c8e8694eb (gate; codec both builds: 42,120 and 26,240 sample rows, content sets on P1.2/P2.2/P2.4 and all four encode variants present; rpc both clients x both transports: cells A-F x a/a+read/b x 1/8/16, the plant aborting on each; calib), figures stripped (`gen/strip_figures.py`: cpu_ns and wall_ns removed, labels kept) |

## Register H (WP6 re-review), the findings assigned to rust: proposed dispositions

| Finding | Disposition | Evidence |
|---|---|---|
| R-H21 (core) | confirmed and fixed (31fc3eecf): every capacity capped at INT32_MAX, AK_ERR_LIMIT above it with or without grow; the host's `cap` is reported unchanged | before: `logs/rust/wp6h/rh21-before.log` (CAPACITY at INT32_MAX + 1 without grow; a host cap of 2^32-1 let `len` pass INT32_MAX); after: 5 ak-core unit tests incl. exactly INT32_MAX on a real buffer (gate step 2) |
| R-H10 (core) | confirmed and fixed (31fc3eecf, through rust_abi, four codecs) | before: `logs/rust/wp6h/rh10-before.log` (480 record bytes -> 0); after: "refused parse keeps A" PASS (clean gate) |
| R-H20 | no code change (owner); a control shows the rule | "decided at arm time" PASS with its armed twin (clean gate) |
| R-H15 | confirmed and fixed: packed fixed32 refused in `check_expressible`; four renderers use `unknown_compiled_out`. PACKED_KIND: refuted as an ABI fact (it sets `open_kind`, which no run symbol reads and no host sees); left in rust_abi, documented | `gen/check_direct.py` (`logs/rust/wp6h/rh12-rh15-generator.log`) |
| R-H13 | confirmed and fixed: slice guard over every slice gen/ (imports, wire emitters, renamed copies, 3 plants); one_core's stale csharp/gen/ir.py exception removed, a renamed-emitter plant added | gate step 1; its first run flagged rust/gen/rust_facade.py (R-H12) |
| R-H12 | confirmed and fixed by rendering from the plan (rust_facade.py in codec/gen, encode steps in tag order, the plan's implicit presence). shapes.json bytes unchanged; the prost-build diff was not done | `gen/facade_order.py` (`rh12-rh15-generator.log`: tag order where the old renderer wrote the oneof last); conformance VERDICT on every arm |
| R-H22 | confirmed for Rust and fixed: types_nounk.rs from the no-unknown plan behind the facade's `unknown-fields` feature; retain rendering compiled out there | no-unknown build compiles and passes conformance, shapes and the corpus (ffi-nounk, native-nounk) |
| R-H2 | confirmed for Rust and fixed: a reused pool per (cell, dir, k), created before the warm-up | `crates/campaign/src/bin/rpc_client.rs` (`Pool`) |
| R-H8 | confirmed (no switch to a scalar member) and a case added | "oneof stamp -> int" PASS (clean gate) |
| R-H23 | confirmed and changed: seeded random order per launch for codec arm blocks and cases and for RPC cells, recorded in each header | `crates/campaign/src/lib.rs` (`shuffle`), checklist row 22 |

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| D42 | rust facade | a map entry has no unknown-field bag in the facade, so `U-map-entry` is written in the dropped form by ffi-retain and native-retain although the core delivers the entry's bytes (accepted by the contract) | open, a facade question |

Closed since the last rewrite (evidence in `JOURNAL.md`): D2 (the 1.88 floor now runs,
the gate passes on it); D34 (retention inside inlined children, closed by decision 11 for the C
ABI); D35 (recursive messages: refused from the C ABI by owner decision, ABI-v1; `Nest` runs
on core-native only, a scope limit listed below); D38, D39, D40 (fixed by their slices,
FIX-PLAN R-G17); D41 (every slice's generated tree is current: `generate.py --check`).

## What is not measured

- **Timings.** None are results in this phase; every timing log is container
  instrumentation, and no figure is quoted in this file.
- **perf counters** (req 20): perf is not installed here.
- **Codec wall time**: the codec suite records CPU time only.
- **Encode variants on the core's own transport**: the transport-ready row of core-native and
  core-ffi is the tonic form (a `Bytes` copy); over the core's transport (cells C, E) the form
  is the reused buffer, measured as the reused-buffer row, not as a separate row.
- **Pool sizing**: the pool's heap is measured with glibc `mallinfo2` at build time; memory
  held by the allocator but not in use is not counted.
- **RPC callback and queue deliveries** in the campaign runner (stage 6 harness only).
- **The streamed upload** (req 14, optional): not built.
- **The RPC half beyond unary calls**: no metadata, deadlines, status numbers, retry, TLS or
  streaming; `ak_call_cancel` is never called; no concurrency suite over RPC.
- **C5 (produce)** of the corpus: the harness builds no values of its own.
- **Unknown fields**: map-entry unknowns in the facade (D42); `_unknown` projections are not
  compared (optional in the contract).
- **Merge-on-repeat** (R-E4): no payload or corpus row repeats a singular message with
  differing content. (`-0.0` is exercised: corpus row `S-double-minus-zero`.)
- **Recursive messages through the C ABI**: refused by decision; the 16 `Nest` rows run on
  core-native only (depth limit 100).
- **Nesting past depth 3 on the C ABI**: the schema's roots stop there.
- **The probe rows** (field numbers above 2^29-1, the 10th varint byte, map order): checked
  in a scratch manifest (`logs/rust/wp5s6-probe-*.log`), not in the corpus.
- **The no-unknown build's run-time switch**: it has no options and no reset, so a host
  cannot turn retention on at run time; that is the variant's definition, not a gap.
- **Other slices' runtimes and bindings**: measured by their slices.

## Notes

- The core's own default features leave `init-guard` off; this slice's harness, campaign
  and corpus harness turn it on (CAMPAIGN.md req 6), and the corpus gate has a control that
  fails when a binding skips `ak_init`.
- Reads `packages/rust`; edits nothing under `packages/`.
- Each no-unknown build needs its own `CARGO_TARGET_DIR` (target-nounk,
  target-count-nounk, target-corpus-nounk).

## Log index

| Log | What it establishes |
|---|---|
| `logs/rust/campaign-wp7/` | WP7: gate (stable, both builds), floor 1.88.0 gate, ThreadSanitizer, and the smoke of codec, rpc and calib from a clean worktree at c8e8694eb; figures stripped; disk (`df.txt`) |
| `logs/rust/wp7/crossings-change.txt` | the crossing files before and after WP7, row by row |
| `logs/rust/wp6h/clean-gate/` | the gate from a clean worktree at 766f8dcd9 after register H: both builds, crossing counts, disk |
| `logs/rust/wp6h/rh21-before.log`, `rh10-before.log` | R-H21 and R-H10 reproduced on the core before the fix |
| `logs/rust/wp6h/rh12-rh15-generator.log` | the armonik arm's order (new vs pre-R-H12 renderer) and the generator-time refusals |
| `logs/rust/wp6-clean-gate/` | the gate from a clean worktree at d2cd0b02f: stable (`gate.log`, both builds, crossing counts), floor 1.88.0 (`gate-floor-1.88.log`), ThreadSanitizer (`tsan.log`), disk before/after (`df.txt`) |
| `logs/rust/campaign-wp5s10/` | smoke run of the harness at f0c82bb (instrumentation): codec both builds, rpc both clients x both transports, plant controls |
| `logs/rust/campaign/` | the earlier smoke run (instrumentation), calib included |
| `logs/rust/wp5s10/` | the no-unknown vs drop crossing-count diff, generate --check, one_core --selftest at step 10 |
| `logs/rust/wp5s10-gate.log` | the gate when the no-unknown build was added |
| `logs/rust/wp5s8/`, `wp5s8-gate.log` | decision 11's implementation rules and their controls |
| `logs/rust/wp5s7/`, `wp5s7-gate.log` | decision 11's mechanism, its controls, the counts diff it caused |
| `logs/rust/wp5s6*` | consolidation: generate --check, R0 selftest, oracles, probe rows, other slices' gates at the time |
| `logs/rust/wp5-*.log`, `wp4-gate.log` | the plan rewrite: gate, corpus, corpus before, depth-2 error, TSan |
| `logs/rust/rd*.log` | review defects R-D1, R-D6, R-D8, R-D9 reproduced and fixed |
| `logs/rust/stage*.log` | stages 1 to 6; their timing sections are instrumentation |
