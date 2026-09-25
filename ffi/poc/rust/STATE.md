# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only thing that
survives the end of a session. Phase (README section 1.1): setup and design. A container
timing is instrumentation; this file states what exists and what was checked, and never
what a binding should choose (the decision is the owner's).

| | |
|---|---|
| **Status** | **FIX-PLAN WP5 step 7 (decision 11's unknown-field mechanism, as the owner specified it) built in the plan, the core and the Rust backends, 2026-09-25**: decode groups carry `unknown: ak_unk_buf`, per-root `ak_dec_<Root>_opts` (one host pointer, one `ak_unk_opts` per message position), `ak_dec_ctx_new_<Root>` / `ak_dec_reset_<Root>`; the `unknown`/`unk_<slot>` callbacks, `ak_unk_f` and `ak_uspan` are gone; ffi-retain now writes the retained form on the 16 `U-leaf-*`/`U-deep-*` rows; push and pull both capture. Other slices' generated trees are STALE until their owners render the options (their backends were given a transitional drop-mode change so their generators run). Before that: **FIX-PLAN WP5 step 6 (consolidation) done, 2026-09-24**: the fixed ABI (codes, structs, entry points, vtable order, pull numbering, RPC counting) lives in `plan.FIXED` and every backend renders it; one C header backend (`c_abi.py`); the guard covers all 26 backend modules; `poc/codec/gen/generate.py [--check]` regenerates every slice; field-number and map-order rules stated and applied (commits 57b6180, 3cee365, 41eb485). Before that: **FIX-PLAN WP4 items 7, 9, 10 done (Part A) and WP5 step 1 done (Part B).** The shared core and the core-native control are now rendered by ONE rule layer (`poc/codec/gen/plan.py`) through Rust backends that render plans only; the full conformance corpus passes through the C ABI and core-native in both unknown-field modes. Stages 1 to 6 before that (four arms, every shape, RPC arm, pull family, concurrency suite, lifecycle) |
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
one_core.sh      R0's mechanical check; allows codec/crates/*/src/generated_corpus/.
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
gen/corpus.sh          the corpus, four arms, plus four controls that must fail
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
- **Group layout export**: both sides of this slice compile against one generated header, so
  section 10's load-time check is untested here.
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
