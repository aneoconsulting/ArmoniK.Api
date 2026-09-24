# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only thing that
survives the end of a session. Phase (README section 1.1): setup and design. A container
timing is instrumentation; this file states what exists and what was checked, and never
what a binding should choose (the decision is the owner's).

| | |
|---|---|
| **Status** | **FIX-PLAN WP4 items 7, 9, 10 done (Part A) and WP5 step 1 done (Part B), 2026-09-24.** The shared core and the core-native control are now rendered by ONE rule layer (`poc/codec/gen/plan.py`) through Rust backends that render plans only; the full conformance corpus passes through the C ABI and core-native in both unknown-field modes. Stages 1 to 6 before that (four arms, every shape, RPC arm, pull family, concurrency suite, lifecycle) |
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
rust_core.py     RETIRED as an emitter; a legacy adapter (walk_encode/walk_decode/Sites,
                 plan-ordered) kept only because poc/cpp/gen/cpp_core.py imports it.
generate.py      ONE command writes everything the core owns, including the corpus-schema
                 core (generated_corpus/, behind ak-abi/ak-core feature `corpus`).
                 --check: drift + the one-generator GUARD (a backend importing ir/shapes/
                 spec/... fails) + the guard's own planted-violation self-test.
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

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| D35 | ABI v1 section 6 | **A recursive message cannot cross the C ABI**: a group inlines its whole singular subtree, so `Nest` has no finite group. `plan.check_expressible` refuses it at generator time by name; the 16 `Nest` rows run on core-native only (depth limit 100, `X-depth-101/300` refused with -4). | open, an ABI decision |
| D34 | ABI v1 decision 11 candidate | **Retain through the C ABI covers the root and every repeated element, not an inlined singular child, a oneof message member or a map entry**: the decode side has no slot to deliver their unknown runs to (`ak_uspan` carries a token, not a path). 16 corpus rows write the dropped form in `ffi-retain` (accepted by the contract). Core-native retains at every level except map entries. | open, needs an ABI shape |
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
- **Other slices' backends**: WP5 steps 2 to 5 (C++, Java, C#, Python) are not ported; their
  generators still carry their own wire rules. The python generator reports `ak_abi.h` STALE
  at the base commit already (it renders the cpp slice's header), not caused by this work.

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
