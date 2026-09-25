# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only thing that
survives the end of a session. Phase (README section 1.1): setup and design. A container
timing is instrumentation; this file states what exists and what was checked, and never
what a binding should choose (the decision is the owner's).

| | |
|---|---|
| **Status** | **FIX-PLAN WP5 step 10 (2026-09-25): the NO-UNKNOWN variant** -- `plan.Options(unknown="drop")` on the C ABI renders a complete compile-time variant with unknown-field support compiled out (no `unknown` slot in the decode groups, no `ak_ufix_*`, no `ak_dec_<Root>_opts`, no `ak_dec_reset_<Root>`, no `ak_uencode_*`/`ak_uelem*_*`, no capture; root-bound contexts kept as `ak_dec_ctx_new_<Root>(void)`), selected by the `unknown-fields` cargo feature of ak-abi/ak-core (default on) and, for C, by a second complete header from `c_abi.emit` of the drop plan (defines `AK_NO_UNKNOWN_FIELDS 1`, 240 layout facts vs 400). Both variants gated (`logs/rust/wp5s10-gate.log`, `logs/rust/campaign-wp5s10/gate.log`); the campaign harness runs the codec suite's third mode and the RPC grid's C/D cells in all three modes; smoke run in `logs/rust/campaign-wp5s10/` (instrumentation). Commits d89bdfc (codec), f0c82bb (rust). Before that: **FIX-PLAN WP5 step 8 (2026-09-25): decision 11's confirmed implementation rules** -- options read in place (pools for repeated positions, entries cleared as consumed, host refills), one position and one buffer per oneof, root-bound decode contexts (`ak_dec_ctx_new_<Root>`, no untyped `ak_dec_ctx_new`); rust gate PASSED with pool, refill, oneof-switch and wrong-root controls (`logs/rust/wp5s8-gate.log`); crossing counts unchanged. Before that: **FIX-PLAN WP3 (2026-09-25): campaign harness built and smoke-run** -- `run_campaign.sh`, criterion codec suite, separate-process RPC grid, calib, crossing-count gate; section 10 checklist below (unmet: perf not installed; 1.88 toolchain not verified; codec wall time not recorded; callback/queue deliveries not in the campaign runner). Before that: **FIX-PLAN WP5 step 7 (decision 11's unknown-field mechanism, as the owner specified it) built in the plan, the core and the Rust backends, 2026-09-25**: decode groups carry `unknown: ak_unk_buf`, per-root `ak_dec_<Root>_opts` (one host pointer, one `ak_unk_opts` per message position), `ak_dec_ctx_new_<Root>` / `ak_dec_reset_<Root>`; the `unknown`/`unk_<slot>` callbacks, `ak_unk_f` and `ak_uspan` are gone; ffi-retain now writes the retained form on the 16 `U-leaf-*`/`U-deep-*` rows; push and pull both capture. Other slices' generated trees are STALE until their owners render the options (their backends were given a transitional drop-mode change so their generators run). Before that: **FIX-PLAN WP5 step 6 (consolidation) done, 2026-09-24**: the fixed ABI (codes, structs, entry points, vtable order, pull numbering, RPC counting) lives in `plan.FIXED` and every backend renders it; one C header backend (`c_abi.py`); the guard covers all 26 backend modules; `poc/codec/gen/generate.py [--check]` regenerates every slice; field-number and map-order rules stated and applied (commits 57b6180, 3cee365, 41eb485). Before that: **FIX-PLAN WP4 items 7, 9, 10 done (Part A) and WP5 step 1 done (Part B).** The shared core and the core-native control are now rendered by ONE rule layer (`poc/codec/gen/plan.py`) through Rust backends that render plans only; the full conformance corpus passes through the C ABI and core-native in both unknown-field modes. Stages 1 to 6 before that (four arms, every shape, RPC arm, pull family, concurrency suite, lifecycle) |
| **Next step** | none assigned. Open for the aggregating session: the other slices render the no-unknown variant (their backends: see JOURNAL, WP5 step 10); a campaign-machine run of `run_campaign.sh` with both builds |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | MSRV 1.88 declared. **Not verified: no 1.88 toolchain in this container**, stable 1.94.1 only (plus a nightly 1.100, used for ThreadSanitizer and nothing else) |
| **Target** | the same, one configuration (README section 5) |
| **Incumbent** | prost 0.14.4 (+ tonic 0.14 for the RPC arm). R14 checked: tonic-prost's codec calls `Message::decode`/`encode`, so production path and library entry point are the same call |
| **Outstanding for the aggregating session** | three rule/ABI questions this work surfaced, none decided here: (1) the map-key rule (below, "Rule questions"); (2) unknown fields inside an inlined child / oneof message member / map entry cannot be retained through the C ABI (D34); (3) a recursive message cannot cross the C ABI (D35) |

## What exists

### The generator (shared, `poc/codec/gen/`, FIX-PLAN WP5 step 1)

```
ir.py            the ONE front end: shapes.json (load) and the corpus reader view
                 (load_corpus, via corpus/emit/spec.py -- the merge corpus.proto comes from).
                 Only plan.py imports it. Its old layout helpers are forwarding names for the
                 unported cpp/java/csharp/python generators.
plan.py          the RULE LAYER. IR -> MessagePlan (encode plan: EncStep list in tag order;
                 decode plan: {(field, wire): DecAction}) + the ABI layout functions + the RPC
                 ABI (R-G5) + the lifecycle (R-G7). Its module docstring is the CONTRACT a
                 backend follows: what a plan contains, what a backend may and may not decide.
                 Options(unknown=drop|retain|both, utf8=reject|lossy, recursion_limit=100).
                 On the C ABI: both = decision 11's mechanism; drop = THE NO-UNKNOWN VARIANT
                 (WP5 step 10, `relower(p, p.options.with_unknown("drop"))`,
                 `unknown_compiled_out(p)`).
rust_abi.py      Rust backend: abi.rs, codec.rs (push and pull families), the RPC region of
                 ak-abi/src/lib.rs, rpc_check.rs. Renders plans only.
rust_native.py   Rust backend: core-native, one module per unknown-field mode, from the SAME
                 plans (R-E1). Replaces rust_core.py's emitter.
rust_binding.py  Rust backend: the rust slice's host binding (moved out of rust_abi.py);
                 renders ak_init from plan.lifecycle.
cpp_layout.py    Rust backend: section 10's layout export, from plan layout functions.
c_abi.py         the ONE C header backend (ak_abi.h, ak_layout.h, ak_layout_names.h) from
                 plan.FIXED + plans; used by the C++, Java and Python slices. C# renders its
                 own P/Invoke declarations from plan.FIXED (cs_binding.py).
rust_core.py     DELETED (WP5 step 6); poc/cpp/gen/cpp_header.py and java_abi.py too.
generate.py      ONE command writes everything the core owns (incl. the corpus-schema core
                 and the `plan.fixed` region of ak-abi/src/lib.rs, abi_check.rs), then runs
                 every slice's generator (rust, cpp, java, csharp, python) with the same
                 --check flag. --check: drift + the GUARD over every *.py backend module
                 (26; plan/ir/generate excluded) + its planted-violation self-test.
                 --core-only skips the slices.
one_core.sh      R0's mechanical check; allows codec/crates/*/src/generated_{corpus,nounk,corpus_nounk}/.
crates/ak-{abi,core}/src/generated_nounk/, generated_corpus_nounk/
                 the no-unknown variant's abi.rs / codec.rs / layout.rs, selected when the
                 `unknown-fields` feature (default on) is OFF; a build of it needs its own
                 CARGO_TARGET_DIR (a shared one overwrites libak_core.so).
```

### This slice

```
gen/generate.py        this slice's generator: facade types, payload builder, prost impl
                       (slice glue, reading the plan's descriptor view), and -- through the
                       shared backends -- core_native.rs, core_native_retain.rs, binding.rs,
                       the shared core files, and the whole corpus harness's generated files
gen/rust_project.py    corpus glue: CONTRACT.md section 3 projection of a facade value
gen/rust_corpus.py     corpus glue: the per-root dispatch table (4 arms)
gen/gate.sh            THE CORRECTNESS GATE, nothing timed (AK_NO_TIMING): generators,
                       core unit tests, byte identity, shapes, crossing counts, content sets,
                       four-build concurrency suite, lifecycle, R-D1 repros, R-D6, corpus.
                       `--tsan` adds ThreadSanitizer
gen/corpus.sh          the corpus, four arms, plus four controls that must fail; section 6:
                       the no-unknown build (ffi-nounk, native-drop, native-retain) + controls
gen/c_variant.sh       both C headers (c_abi of the plan and of its drop relowering): C99 and
                       C++11 compile, AK_LAYOUT_HOST vs each core's ak_layout_facts(), the two
                       mismatched pairs required to fail
gen/crossings-nounk.txt  the no-unknown build's committed crossing counts (gate step 12)
gen/tsan.sh            the concurrency suite under ThreadSanitizer (nightly), with a planted
                       race that TSan must see
gen/corpus_before.py   reproduces the pre-WP5 corpus run (logs/rust/wp5-corpus-before.log)
gen/check_direct.py    ABI v1 section 8's refusals, now asked of plan.py
gen/oracle_probe.py    WP5 step 6: asks upb, pure-python protobuf and protobuf C++ about the
                       10th varint byte and field numbers above 2^29-1
gen/probe_corpus.py    writes those inputs (plus two map-order rows) as a scratch manifest in
                       the corpus format -- PROPOSED corpus rows, not the corpus
gen/probe_pycodec.py   runs that manifest against the python slice's generated pure codec
gen/stage*.sh, pull.sh, concur.sh, lifecycle.sh, contentall.sh, rd1_repro.sh, ...
                       the stage harnesses; their timing sections are instrumentation
corpus/                a workspace of its own (the core with `corpus`, a different ABI):
                       crates/facade (types, core_native, core_native_retain, project),
                       crates/harness (binding, dispatch, src/main.rs = the `corpus` driver:
                       each row in a child process under a timeout)
crates/                facade, harness (bins: conformance, counts, shapes, content, contentall,
                       concur, lifecycle, pullbench, rdrepro, stickyerr, bench, ...),
                       shapes-prost, shapes-values, stage1-validate
```

Arms: `prost`, `armonik` (facade + generated prost impl), `core-native` (drop) and
`core-native-retain` (new), `core-ffi-rust` (push), the pull arms, and in the corpus harness
`ffi-drop`, `ffi-retain`, `native-drop`, `native-retain`.

## What was checked (results; each with its log)

WP5 step 10 (the no-unknown variant), commits d89bdfc + f0c82bb:

- **Gate, both variants** (`logs/rust/wp5s10-gate.log` on the working tree,
  `logs/rust/campaign-wp5s10/gate.log` on f0c82bb): GATE PASSED. Step 12 on the no-unknown
  build (`target-nounk`, `--no-default-features --features init-guard`): the core exports 0
  `ak_uencode_*`/`ak_uelem*_*`/`ak_dec_reset_*`; conformance (byte identity on every payload,
  P1.3/P2.5 included) and shapes (presence, oneof, unknown-field vectors) VERDICT pass; the
  codec pre-check 520/520 on 112 inputs; crossing counts identical to `gen/crossings-nounk.txt`;
  `gen/c_variant.sh`: both headers compile C99/C++11 -Wall -Werror, full 400 and no-unknown
  240 layout facts agree with their cores, both mismatched pairs caught.
- **Corpus on the no-unknown build** (gate log, section 11 / corpus.sh section 6): ffi-nounk
  680/0, native-drop/native-retain 696/0; 0 retained-form lines from ffi-nounk (every unknown
  row written in the dropped form, accepted by the contract); controls proj/reenc/accept/noinit
  fail as required on that build. The full build unchanged (ffi-drop/ffi-retain 680/0).
- **Crossing counts** (`logs/rust/wp5s10/counts-diff.txt`): 335 rows per mode; the no-unknown
  build differs from the full build's drop mode on 3 rows only -- P1.2, P1.2/latin1, P1.2/wide
  decode reverse 8 -> 5, which is the pre-decision-11 value (`wp5s6-gate.log` line 118).
- **Generators**: `poc/codec/gen/generate.py --check` all five slices exit 0, 264 files 0
  problems in csharp (`logs/rust/wp5s10/generate-check.log`); the full variant's generated
  text (codec, abi, layout, binding.rs, every slice's ak_abi.h) is byte-unchanged by the step;
  `one_core.sh --selftest` 0 controls silent (`logs/rust/wp5s10/one-core-selftest.log`).

WP5 step 7 (decision 11 mechanism), uncommitted state -> the poc(codec) commit named in JOURNAL:

- **Rust gate** (`logs/rust/wp5s7-gate.log`): GATE PASSED. Byte identity on every payload
  and arm; R-D6 case D3 now plants the failure in the `grow` upcall (rc -1, 0 upcalls after).
- **Corpus** (same log, section 11): ffi-drop/ffi-retain 680/0, native 696/0 (702 rows, 6
  disputed, 16 `Nest` rows outside the ABI); ffi-retain's retention gaps went from 17 rows
  to 1 (`U-map-entry`, which native-retain shares: the facade map has no per-entry bag).
- **Decision 11 controls** (`corpus --unk-controls`, `logs/rust/wp5s7/unk-controls.out`):
  543 accept rows whose root crosses the ABI, 2,350 (row, position) pairs; zeroing each
  position in turn dropped exactly that position on every row (307 rows carry unknowns,
  315 pairs change); pull (walk, all armed) == push on every row; U-map-entry: the core
  delivers the entry's 8 bytes and delivers none with the entry position zeroed. The plant
  (bags not cleared) fails on all 307 rows (`unk-controls-plant.out`). Placement: a
  pre-allocated buffer with no grow serves element 0 and element 1 is refused with
  AK_ERR_CAPACITY (never the same buffer twice); with grow, element 1 gets a distinct buffer.
- **Other slices, in a scratch worktree** (their gates, never their working trees:
  `logs/rust/wp5s7/scratch/`): with the transitional drop-mode backends, cpp wp5_gate 0 steps
  failed, java build + corpus.sh CORPUS GATE PASSED + gate.sh PASS a/b/c, csharp gate.sh
  GATE PASSED, python gate (3.12 only, no 3.7 in the scratch tree) corpus passes on 5 arms
  and fails P7.1 conformance on every arm including the pure-Python codec (the permutation
  judge needs the incumbent the scratch tree lacks; not this change). Their ffi-retain arms
  now write the DROPPED form on ~307-308 unknown rows (cpp, csharp): retention through the
  C ABI is lost there until they render the options.
- **Crossing counts** (`logs/rust/wp5s7/crossing-counts-diff.txt`): encode unchanged; decode
  reverse calls rise where the 16-byte slot shrinks the 32 KB arena's element count
  (P1.2 add calls 5 -> 8 per 1,000 elements; P2.2 pull records unchanged, pull footprint
  bytes up 20-65%). Counts, not timings: the layout cost the owner asked to measure.

WP5 step 6 (consolidation), on 41eb485 unless stated:

- **One command** (`logs/rust/wp5s6-generate-check.log`): `poc/codec/gen/generate.py
  --check` exit 0; guard over 26 backend modules, planted IR import caught; every slice's
  --check exit 0 (java: 258 generated files, 0 problems).
- **R0** (`logs/rust/wp5s6-one-core-selftest.log`): one_core 0 failures; the scratch copy now
  includes ffi/corpus; all five planted controls fail as required.
- **Oracles** (`logs/rust/wp5s6-oracles.log`): 10th varint byte with bits beyond 64 --
  upb 7.36.2, pure-python and protobuf C++ 3.21.12 all ACCEPT (discard); the rule stays
  "discard". Field number 2^29, 2^32+2 and 2^29 inside a group -- upb and C++ refuse,
  pure-python accepts; the rule is "refuse" (ERR_MALFORMED).
- **Probe rows, before/after** (not corpus rows; gen/probe_corpus.py): rust core + native
  12 fails before, 0 after (`wp5s6-probe-before.log`, `wp5s6-probe-after.log`, which also
  carries the python pure codec: 3 wrong per mode before, 0 after); java before 5 per arm
  (3 field rows accepted, 2 map rows written in UTF-16 order, `wp5s6-probe-java-before.log`),
  after: ffi arms 0, R arms 1 (`wp5s6-probe-java-after.log`); cpp after: ffi 0, native 1
  (`wp5s6-probe-cpp-after.log`); C# after: ffi 0, managed 1 (`wp5s6-probe-csharp-after.log`).
  The remaining 1 is `P-field-maxplus1-in-group` in a hand-written runtime (D38). No C#
  before was captured.
- **Rust gate** (`logs/rust/wp5s6-gate.log`): GATE PASSED, byte identity, corpus ffi 672/0,
  native 688/0, controls fail.
- **Other slices' gates** (logs under `logs/rust/wp5s6/`, each slice's own logs restored):
  java gen/corpus.sh CORPUS GATE PASSED (0 failing arm-rows, target and floor, 4 controls
  fail) and gen/gate.sh PASS on arms a, b, c; csharp gen/gate.sh GATE PASSED; python
  ./gate.sh python3.12 build/py37/python3.7 exit 0; cpp gen/wp5_gate.sh 0 steps failed on a rebuilt `build/` at 41eb485 (`cpp-3`;
  `cpp-2-stale-binaries` ran on binaries older than the C++ backend: D39).

- **Byte identity**: every arm byte-identical to `ffi/schema/generated/manifest.json` on all
  16 payloads, after the plan-driven rewrite (`logs/rust/wp5-gate.log` step 3). The core's
  output on the payload set did not move.
- **The conformance corpus, full, four arms** (`logs/rust/wp5-corpus.log`): ffi-drop and
  ffi-retain pass 672 / fail 0, native-drop and native-retain pass 688 / fail 0; 3 disputed
  rows excluded and reported (`U-map-entry`: accepted; `X-tag-zero-Empty`,
  `X-tag-zero-nested-Empty`: refused with -2 by all four arms); 16 rows `not in the C ABI`
  (root `Nest`, D35) on the two ffi arms, run on native. Every row in its own process under a
  10 s timeout: 0 hangs, 0 crashes. **Controls, each required to fail and failing**: planted
  projection key (C2), planted re-encode byte (C3), refusals turned into acceptances (C4),
  `ak_init` skipped on an init-guard core (R-G7).
- **The same corpus against the pre-WP5 generator** (`logs/rust/wp5-corpus-before.log`,
  WireZoo and Nest unreachable then): 37 fails per ffi arm, 43 per native arm -- 31 `T-dec-*`
  invalid-UTF-8 rows accepted, 6 (ffi) / 12 (native) `U-wire-*` rows where a packed field's
  unpacked form at a foreign wire type was read as a value. Those are the rule fixes.
- **Unknown fields, which form each mode wrote** (C3): drop arms write the dropped form
  everywhere; retain arms write the retained form except 16 ffi rows (`U-leaf-*`, `U-deep-*`:
  an unknown inside an inlined singular child, D34) and `U-map-entry` (a map entry has no bag
  in either arm).
- **Crossing counts** (counting build, `wp5-gate.log` step 5): unchanged to the digit --
  M1 P1.2 9 encode / 6 decode per 1,000 elements; M2 P2.2 10.024 encode / 7.004 decode per
  element; pull 0 reverse, records == push reverse calls on every counted payload.
- **Concurrency** (`wp5-gate.log` step 7, `rd8-tsan.log`, `wp5-tsan.log`): four shapes rotated per round
  (R-D8); shipped and global-widths 0 wrong, pad and pad+global caught; ThreadSanitizer: 0
  warnings in the suite, 77 on the planted shared-context race.
- **Sticky error slot** (R-D6, `rd6-sticky-before.log` / `-after.log`): 7 of 7 cases now fail
  with zero upcalls after `ak_fail`.
- **Lifecycle** (`wp5-gate.log` step 8): 15 cases, init-guard off and on, including the new
  options-collision case.
- **R-D1 repros** plus the depth-2 error case (`wp5-gate.log` step 9).
- **The one-generator guard** (`generate.py --check`): 6 backend modules import plans only;
  a planted `import ir` / `from shapes import` is caught. The cpp and java generators'
  `--check` report no drift against the new core text (checked read-only).

## Campaign readiness (design/CAMPAIGN.md, section 10 checklist; FIX-PLAN WP3)

Runner: `poc/rust/run_campaign.sh --suite codec|rpc|calib|gate --out DIR` (reads
`AK_CPU_CLIENT`, `AK_CPU_SERVER`; campaign DIR `ffi/logs/rust/campaign/`). Harness:
`crates/campaign` (`benches/codec_suite.rs` on criterion; bins `rpc_server`, `rpc_client`,
`calib`, `crossings`), its per-root table rendered by `gen/rust_campaign.py`.
**The codec suite runs on criterion 0.5** (req 22a) with a thread-CPU `Measurement`.
**The RPC grid stays on the slice's runner** (req 22a allows it): requirement 18 aborts the
whole run with no figure on one failed call and requirement 13 needs the server as another
process whose lifecycle the runner owns; criterion records per-benchmark results and has no
abort-with-no-output path, and one RPC sample is a batch of k concurrent calls timed with
process CPU (getrusage) beside wall, which criterion's one-measurement-per-iteration model
does not express. The calib suite is a runner too (two arms, fixed iteration count, so
`perf stat` counts per iteration are exact).

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | met by the runner (one process at a time); the machine is the owner's |
| 2 | governor, turbo, SMT | recorded in every header from sysfs; setting them is the owner's (container: n/a) |
| 3 | isolation | recorded (isolcpus/nohz_full/rcu_nocbs from /proc/cmdline, cgroup cpuset.cpus.effective); the mechanism is the owner's |
| 4 | three disjoint CPU sets | met: `AK_CPU_CLIENT` / `AK_CPU_SERVER` required from the environment, `taskset -c` on every measured process and on the server; OS = the rest |
| 5 | floors gated | not applicable as a separate floor: Rust's floor IS the target (MSRV 1.88, README 5). **The 1.88 toolchain itself is not verified in this container** (D2, stable 1.94.1 only) |
| 6 | build flags printed | met: release, opt-level 3, lto off, core as a cdylib through the dynamic linker, core features `rpc,init-guard`, harness guard on, trusted UTF-8 transcoder |
| 7 | payloads | met: 16 payloads (ASCII); latin1 and wide content sets on P1.2 and P2.2; 92 non-disputed `U-*` corpus rows whose root is one of this slice's 7 ABI roots (112 inputs). P7.1 decode only (SHAPES.md: no canonical writer) |
| 8 | arms | met: incumbent-prod = prost through tonic's codec calls (`Message::encode` into a `BytesMut`, `decode` from a `Buf`); **incumbent-best is the same entry point** (R14, this STATE's incumbent row), so not a second row; core-ffi push, core-ffi-pull as a labelled extra; host-gen = `core-native`; Rust's `armonik` |
| 9 | directions | met: encode, decode, decode-read (a generated visitor reads every field of both object models: scalars, enums, strings/bytes length and first/last byte, every element and map entry, present children and the active oneof member) |
| 10 | drop, retain and no-unknown | met: no-unknown = a SEPARATE binary built without `unknown-fields` (target-nounk), core-native/core-ffi/core-ffi-pull in mode no-unknown with incumbent-prod and armonik as in-process controls (`codec-nounk-launchN.jsonl`). Drop and retain: core-native, core-ffi and core-ffi-pull in drop and retain (retain = every position of decision 11's options armed, `decode_with_*_unk`); incumbent and armonik in prost's default (drop), stated. On 27 `U-wire-*` rows prost REFUSES a known field at a foreign wire type (protobuf reads it as unknown): those (row, incumbent/armonik) pairs are not timed and are listed in the codec log header |
| 11 | serialised once per iteration, no memo | met: prost recomputes `encoded_len` every encode and the core-ffi / core-native encoders reset their context per call; Rust objects carry no size memo, so re-encoding one graph is a fresh serialisation |
| 12 | cells A-D, C and D in three modes | met: full client A, B, C-retain, C-drop, D-retain, D-drop (`rpc-T-launchN.jsonl`); no-unknown client (target-nounk) C-nounk, D-nounk with A and B as in-process controls (`rpc-T-nounk-launchN.jsonl`), against the same server; the plant control per client binary; the runner checks each binary loads the core of its variant (u-family exports) |
| 13 | server separate process, pre-serialised | met: `rpc_server` on `AK_CPU_SERVER`, P2.2 pre-serialised at start-up and checked against the manifest hash; direction (b) decoded by prost in every cell |
| 14 | directions a and b | met; the optional streamed upload is not built |
| 15 | 1/8/16 in flight | met (k host threads, each blocking per call) |
| 16 | B/C blocking; callback/queue labelled | B and C use `ak_call_unary` (blocking). The callback and queue deliveries are not in the campaign runner (they exist in the stage 6 harness, `rpcgrid`) |
| 17 | shipped and pinned, B/C follow | met: shipped = tonic endpoint defaults + nodelay (packages/rust's `GrpcClient__TcpNagleAlgorithm` default false), `ak_client_new`, server defaults; pinned = 4 MiB stream and connection windows, adaptive off, Nagle off on tonic, the core client and the server |
| 18 | every call checked, abort | met: status and response length on every call (cell A through its decoder's byte count), C and D also the core's decode; the first failure exits 3 with no output file; control `--plant` (wrong expected length) run per transport by the runner and required to abort with no file (`rpc-*-PLANT.log`) |
| 19 | crossing counts gate | met: `crossings` (counting build) counts every timed core-ffi case (671 lines: every input x encode/decode/decode-pull x drop/retain), compared with the committed `gen/crossings.txt` in the gate and before codec and calib; a difference stops the run. The no-unknown build: 335 rows (x no-unknown) vs `gen/crossings-nounk.txt`, same places. Not counted: the two `ak_dec_reset_<Root>` calls around a retain decode (forward crossings the core's context counters do not see) |
| 20 | crossing cost fwd/rev, perf stat | partly: `calib` rows `forward` (ak_noop) and `forward-reverse` (ak_noop_reverse), the reverse cost is their difference in the same round; `perf stat -e cycles,instructions` per arm when perf is installed, **not installed in this container** (`calib-perf-launch1.txt` says so) |
| 21 | CPU clocks | codec: CLOCK_THREAD_CPUTIME_ID per criterion sample (criterion's measurement replaced); **no wall time for the codec suite** (criterion measures one quantity), stated in the header. rpc: getrusage(RUSAGE_SELF) of the client per round, wall beside it. calib: CLOCK_THREAD_CPUTIME_ID |
| 22 | blocks, order rotated between launches | met: codec in blocks by arm, the 5-arm order rotated by launch; rpc cells rotated by launch; calib's two arms alternate |
| 23 | 5 rounds x 3 launches, every round committed | met by default: codec = criterion samples per case (10, criterion's floor, >= 5) x 3 launches, every sample exported; rpc/calib = 5 rounds x 3 launches. The smoke run is 1 launch, 1 round (codec 10 samples, the floor) |
| 24 | warm-up stated, identical | met: codec = a fixed iteration count per case (default 100; smoke 3) then criterion's warm-up time (default 500 ms; smoke 5 ms), identical per arm, recorded; rpc = 64 calls per (cell, dir, in-flight) (smoke 16); calib = iters/10 per arm. No JIT |
| 25 | allocator warmed, GC stated | met: the fixed warm-up runs every case's own allocations before timing; no GC (Rust), stated |
| 26 | correctness before timing | met: every timing suite requires `gen/gate.sh` PASSED for the same tree-id (runs it otherwise): byte identity, the full corpus through the C ABI and core-native in drop and retain with its controls, decision 11's controls, crossing counts; and the codec process re-checks every timed arm on every input before criterion starts (962 checks; manifest hashes, content-set identity against the incumbent, cross-arm equality per mode, retain re-encodes an accepted form of every `U-*` row) |
| 27 | header | met: commit + tree-id (**a dirty tree is refused** unless `AK_ALLOW_DIRTY=1`, which is recorded), machine, SMT, governor, turbo, kernel, isolation, CPU sets, rustc/cargo, incumbent versions from Cargo.lock, build flags, core features, transport, warm-up, repeats |
| 28 | JSON lines, raw | met: one line per criterion sample (converted from criterion's saved `sample.json`, none dropped), per rpc round, per calib round; header lines start with `#` |
| 29 | logs per suite and launch | met: `ffi/logs/rust/campaign/<suite>[-<transport>]-launch<N>.jsonl` |
| 30 | summaries | none produced by this slice (optional) |
| 31 | runner interface | met for the slice; `ffi/campaign.sh` is not this slice's file |
| 32 | smoke run | met, WP5 step 10: `logs/rust/campaign-wp5s10/` (codec full 22,960 rows + no-unknown 14,020 rows; rpc 36 + 24 rows per transport, the plant aborting on both clients and both transports; gate on f0c82bb). WP3: `logs/rust/campaign/`, every file headed INSTRUMENTATION: gate PASSED; calib 2 rows; rpc 24 rows per transport, the plant control aborting on both; codec 2,296 cases x 10 samples = 22,960 rows, precheck 962/962, about 2 minutes |

**Smoke run** (container, `AK_CPU_CLIENT=1 AK_CPU_SERVER=2,3`, `AK_SMOKE=1`, commit 833ea32 for
the timing suites; figures are instrumentation): `logs/rust/campaign/`.

## Rule questions for the aggregating session (decided by nobody here)

1. **Map key.** FIX-PLAN WP5 says "map key always written". Literally ("key always, empty
   value omitted") that writes `0A 02 0A 00` for `E-map-entry-empty`, which is NOT one of the
   corpus's accepted forms; the corpus accepts either the canonical form (empty key and value
   omitted -- what the payload manifest hashes) or "key and value always written" (protobuf
   C++, upb, protobuf-java). `plan.py` states the canonical form, once, and says why.
2. **-0.0 and merge have no corpus row that changes bytes.** The double presence rule (bit
   pattern, R-E3) and merge-on-repeat for a oneof message member / core-native singular
   child (R-E4) are stated in the plan and rendered, but no payload and no corpus vector has
   a singular -0.0 or a repeated singular message with differing content, so neither change
   is observed. A corpus gap, reported.
3. **D34 and D35** below are ABI questions, not codec bugs.
4. **10th varint byte**: the step-6 brief suggested refusing overflow; all three oracles
   accept it, so the plan keeps "discard" and says so. Decision back to the session.
5. **Proposed corpus rows**: `P-field-maxplus1`, `P-field-2p32plus2`,
   `P-field-maxplus1-in-group`, `P-field-max`, `P-varint10-*`, `P-map-order-sorted/-reversed`
   (gen/probe_corpus.py). The map-order rows are the only inputs found that move bytes
   (Java, C#).

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| D35 | ABI v1 section 6 | **A recursive message cannot cross the C ABI**: a group inlines its whole singular subtree, so `Nest` has no finite group. `plan.check_expressible` refuses it at generator time by name; the 16 `Nest` rows run on core-native only (depth limit 100, `X-depth-101/300` refused with -4). | open, an ABI decision |
| D34 (closed by WP5 step 7 for the C ABI) | ABI v1 decision 11 candidate | **Retain through the C ABI covers the root and every repeated element, not an inlined singular child, a oneof message member or a map entry**: the decode side has no slot to deliver their unknown runs to (`ak_uspan` carries a token, not a path). 16 corpus rows write the dropped form in `ffi-retain` (accepted by the contract). Core-native retains at every level except map entries. | open, needs an ABI shape |
| D38 | hand runtimes outside this unit's allowance | the field-number limit inside a skipped group is not in `poc/cpp/include/ak/rt.h`, `poc/java/src/java/ak/Dec.java`, `poc/csharp/src/Facade/Wire.cs`: native/R/managed arms accept `P-field-maxplus1-in-group` | open, owner slices |
| D39 | other slices' build scripts | a gate can run on stale artifacts: java gen/build.sh reused `core-build/` over a `git archive` snapshot and kept a core without the field check (fixed by `rm -rf core-build`); cpp gen/wp5_gate.sh does not build, and `build/` dated from before the C++ backend | open, owner slices |
| D40 | poc/csharp CoreTransport.cs | hand-declared RPC counting; the plan's RPC counting surface is not rendered for C# until it is removed | open, csharp slice |
| D41 | other slices | their generated trees are STALE against the step-7 plan until each slice regenerates and renders `ak_dec_<Root>_opts` (their bindings currently decode in drop mode through a transitional backend change) | open, owner slices |
| D42 | rust facade | a map entry has no bag in the facade, so `U-map-entry` is still written dropped by ffi-retain and native-retain although the core delivers the entry's bytes | open, a facade decision |
| D2 | this container | no rustc 1.88, so the declared MSRV is unverified | open, cannot be fixed here |

### Fixed this session

| # | Where | What | Fix |
|---|---|---|---|
| D24 | core emitter + ak-core | R-D6: encode ignored `hdr.err`; decode kept making upcalls after `ak_fail`; a negative element token silently dropped the element | c10e934; checked after every upcall, `enc_status`, transcoder as upcall |
| D25 | ak-core transcoders | R-D9: `from_raw_parts(NULL, 0)` / `copy_nonoverlapping(NULL, .., 0)`; aborts in a debug build | c10e934; len 0 guard in five transcoders and the direct path |
| D26 | ak-core `ak_init` | R-D9: options folded with XOR could collide; a differing second call succeeded | c10e934; three words compared |
| D27 | lifecycle.sh, guardprice.sh, stage2/3.sh | R-D9: the "guard OFF" arm had the init guard (a HARNESS default feature, not a core one) | 24dce7d |
| D28 | old decode emitter | every nested reader named `cd`: at depth two an error was assigned to itself and dropped (rustc: "useless assignment", 4 sites) | WP5 rewrite: readers named by depth; `rdrepro nested2-*` |
| D29 | host binding | direct-argument path `.unwrap()`ed an absent parent: `S-UploadResultDataMessage-min` panicked the host | WP5: Option chain, empty argument |
| D30 | host binding | a root whose loop slot lives on an inlined child (`TaskDetailed`, `TaskSummary`, `ChunkElement` as roots) did not compile | WP5: paths through Options |
| D31 | core + native | invalid UTF-8 accepted on decode (C ABI never validated; native lossy by default): 31 `T-dec-*` rows | WP5 plan rule utf8="reject" (the lossy policy is the one option) |
| D32 | core + native | a packed field's unpacked form accepted at a foreign wire type (core: 0 or 1 for any kind; native: any) | WP5 plan decode table (R-E2) |
| D33 | core | a non-leaf element's own unknown fields were never captured, so retain dropped them (`U-element-*`, `U-chunkelem-*`) | WP5: element-level capture through the existing `unk_` slot |
| D36 | gen/concur.sh | `| head` under `pipefail` could SIGPIPE the counting run (flaky exit 101) | `awk 'NR<=14'` |
| D37 | gen/check_direct.py | imported `ir` from `poc/rust/gen/` since R0 moved it; could not run | asks `plan.py` now |

Earlier defects D1 to D23 (stages 1 to 6, WP4 item 1) are closed; their evidence is in the
log index and `JOURNAL.md`. D20's ABI hazard (an empty buffer's pointer can be section 8's
`AK_STR_DIRECT` sentinel) remains an ABI observation for the other slices' bindings.

## What is not measured

- **Timings.** None are results in this phase. Every timing log below is container
  instrumentation (harness validated or harness defect found). The retired M1/M2 ratio table
  that used to sit in this file is removed (R-C14); `stage3-reproducibility.log` shows why it
  could not be re-derived.
- **The floor**: no 1.88 toolchain; README 5.2's arms b and c do not exist for Rust.
- **C5 (produce)** of the corpus: the harness builds no values of its own.
- **Unknown fields**: the C ABI gap D34; map-entry unknowns in both arms; the pull family
  does not capture (by design, `ak_parse_*` skips); `_unknown` projections are not compared
  (optional in the contract).
- **A singular -0.0 and merge-on-repeat** have no vector that shows a byte change (see Rule
  questions 2).
- **The RPC half** beyond the three unary deliveries (no metadata, deadlines, status numbers,
  retry, TLS, streaming); `ak_call_cancel` never called; no concurrency suite over RPC.
- **Nesting past depth 3 on the C ABI**: the schema's roots stop there; decision 7's limit is
  exercised on core-native (`Nest`) only.
- **Group layout export**: the Rust host compiles against the same generated declarations as
  the core, so its own load-time check is trivially true; `gen/c_variant.sh` exercises section
  10's check from a C++ host on both header variants (and its failure on a mismatch).
- **The no-unknown variant's cost**: built, gated and in the campaign harness; no figure (a
  container figure is instrumentation). `ak_dec_reset_*` and `ak_dec_<Root>_opts` do not exist
  there, so a no-unknown host cannot be switched to retain at run time.
- **Other slices' runtimes**: the hand-written runtimes (D38) are not generated, so a rule
  in plan.py reaches them only through their owners. `cs_layout_probe` still parses the Rust
  declaration's text (intentional, R-E6).
- **The step-6 rules on the corpus**: no committed corpus row reaches the field-number or
  map-order rule; the evidence is the probe manifest.

## Slice-specific notes

- The core default (`ak-core` features) still has `init-guard` OFF; this slice's harness and
  the corpus harness turn it ON, and the corpus gate has a control that fails when a binding
  skips `ak_init` (R-G7).
- `ak_queue_next`'s timeout is now `u64` in the declaration every Rust host compiles against
  (R-G5); `rpc_check.rs` makes a signature drift a compile error.
- Reads `packages/rust`; edits nothing under `packages/`.

## Log index

| Log | What it establishes |
|---|---|
| `logs/rust/wp5s10-gate.log` | WP5 step 10: the gate with step 12 (the no-unknown build) on the working tree |
| `logs/rust/wp5s10/` | counts diff (no-unknown vs drop), generate --check, one_core --selftest |
| `logs/rust/campaign-wp5s10/` | WP5 step 10 smoke run: gate on f0c82bb, codec full + no-unknown, rpc both clients x both transports, plant controls (instrumentation) |
| `logs/rust/wp5s8-gate.log` | WP5 step 8: the rust gate with the in-place/pool/oneof/root-bound rules and their controls |
| `logs/rust/campaign/` | WP3 smoke run of every suite (instrumentation), gate log, runner logs |
| `logs/rust/wp5s7-gate.log` | the rust gate with decision 11's mechanism (corpus, controls, counts) |
| `logs/rust/wp5s7/` | decision 11 controls and plant, crossing-count diff, other slices' scratch gates |
| `logs/rust/wp5s6-gate.log` | the rust gate on the step-6 final state |
| `logs/rust/wp5s6-gate-a.log` | the rust gate after the fixed-ABI consolidation (57b6180) |
| `logs/rust/wp5s6-generate-check.log` | one command, every slice, --check; the 26-module guard |
| `logs/rust/wp5s6-one-core-selftest.log` | R0 and its planted controls |
| `logs/rust/wp5s6-oracles.log` | three oracles on the 10th byte and field-number limit |
| `logs/rust/wp5s6-probe-*.log` | probe rows before/after, rust, python, java, cpp, csharp |
| `logs/rust/wp5s6/{cpp-1,cpp-2-stale-binaries,cpp-3,java,csharp,python}/` | other slices' gates run for step 6 |
| `logs/rust/wp5-gate.log` | the full correctness gate after WP5 step 1 (all steps, corpus included) |
| `logs/rust/wp5-corpus.log` | the corpus, four arms, and the four controls |
| `logs/rust/wp5-corpus-before.log` | the corpus against the pre-WP5 generator: the rule fixes' before |
| `logs/rust/wp5-tsan.log` | ThreadSanitizer over the concurrency suite on the plan-rendered core: 0 warnings, the planted control seen |
| `logs/rust/wp5-nested2-before.log` | D28: the depth-2 error decoded as a success on the old core |
| `logs/rust/wp4-gate.log` | the gate after Part A (WP4 items 7, 9, 10) |
| `logs/rust/rd6-sticky-before.log`, `rd6-sticky-after.log` | R-D6 reproduced (7 fail) and fixed (7 pass) |
| `logs/rust/rd8-tsan.log` | the concurrency suite under ThreadSanitizer, with its planted control |
| `logs/rust/rd9-tc-null-before.log`, `rd9-tc-null-after.log` | R-D9 transcoders on (NULL, 0) |
| `logs/rust/rd9-opts-collision.log` | R-D9 options collision on the old core |
| `logs/rust/rd9-guard-off-arm.log` | R-D9 the "guard OFF" arm was not off |
| `logs/rust/rd1-*.log` | R-D1 length wrap reproduced and fixed (WP4 item 1) |
| `logs/rust/stage1-*.log` | the manifest validated against prost (16/16) and its history |
| `logs/rust/stage2-*.log`, `stage3-*.log`, `stage4-rpc.log`, `stage5-*.log`, `stage6-rpc-grid.log` | stages 2 to 6: byte identity, shapes, crossing counts, pull, concurrency, lifecycle; **their timing sections are container instrumentation, not results** |
