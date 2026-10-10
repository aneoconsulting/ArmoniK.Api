# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what is true now;
the history is in `JOURNAL.md`. Phase (README section 1.1): setup and design. Every timing
this slice has taken is **container instrumentation**, not a result, and none is quoted
here. This file states what exists and what was checked; the choice is the owner's.

| | |
|---|---|
| **Status** | Built on the merged branch (claude/rust-slice-optimization-sy1f4n): four codec arms plus the pull family, the RPC grid (cells A-F), the corpus through the C ABI and core-native, decision 11, the no-unknown build, the WP7 campaign harness, and every kept optimisation. Optimisation unit 2 (the owner) added: encode variants labelled by transport form; T1 (Enc::take, a moved Bytes; additive `ak_enc_take_owned`); the FRAMED send path as labelled extra cells (Bf-Ff, additive `ak_client_set_framed`); N2, N3; the labelled extra RPC directions c (unary upload of P5.3/P5.4) and d (req 14's streamed upload, ABI section 9's client streaming in the core: `ak_call_open/send/send_enc/recv/close`, close removed in unit 3). Not kept: N5 (apply-first decode order, reverted), core-only fat LTO (tooling left, off). N6 not reproduced. Gates: stable checkpoints before N5 passed twice (`opt/pre-n5-gate`, `opt/pre-n5-gate2`); the FINAL gate at d54ea963 from a clean tree PASSED on stable and on the 1.88.0 floor (`opt/final2-gate`); final run `opt/final2`. **Unit 3** (the owner): ABI v1 section 9 as specified (fe79f874, 22ebb97f) in the shared core and generator: call kinds, `ak_call_opts` (deadline, metadata), `ak_call_close` removed and `ak_call_cancel` on streams, the gRPC status number on the stream and on every unary delivery (`ak_completion.grpc_status`, trailing `grpc_status` on the blocking entries), D44's limits enforced; `bin/rpc_semantics` in the gate (11f). **D23** (2026-10-09, owner): the FSM decode family in the shared core and generator, separate from push and pull, checked (D23 CHECKS PASSED) and timed in the container; owner's amendment the same day: the op is the return value (re-checked, `checks/rust-checks-op-return.log`); section "D23". **D24** (2026-10-09, owner): the FSM is the target's decode family in this slice (core-ffi codec arm, cells C and D, the corpus's ffi arms); push is the labelled extra `core-ffi-push`; checks only; section "D24". **reset-on-entry** (2026-10-10, owner): a measurement experiment, core feature default OFF, checked and timed in the container; section "reset-on-entry". **D27** (2026-10-10, owner): reset on entry ADOPTED, the core's only behaviour; the Rust and C++ bindings make no per-operation reset; fast checks only (owner's "checks, not gates"); section "D27" |
| **Next step** | D26 for Rust and C++ (the owner's order; not started). Latest unit (2026-10-10, owner): **D27**, reset on entry made the core's only behaviour (section "D27", `logs/rust/opt/d27-reset-on-entry/`, `logs/cpp/d27/`); fast checks only, the full gates NOT run (listed in the section). Before it the same day: the reset-on-entry experiment (section "reset-on-entry"). 2026-10-09: D24, D23. Earlier units: 2026-10-04 static decode vtables, D20, D19; 2026-10-02 the backward-encode experiment, WP12 item 1 and the TCP worker sweep. Last gate on the campaign machine: the landed p1 with stock h2 (`opt/p1-landed/gate.log`) |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | MSRV 1.88.0: the full gate (steps 0-12b, 11g included), both builds, passes on rustc 1.88.0 at c7b3392f with no uncommitted change in poc/rust or poc/codec and every target directory of this slice rebuilt from empty (`logs/rust/opt/d24-fsm-target/gate-floor-1.88.log`, D24). Earlier: a clean worktree at c8e8694eb (`logs/rust/campaign-wp7/gate-floor-1.88.log`) |
| **Target** | stable 1.94.1 in the container; rustc 1.95.0 (the NixOS machine's ambient toolchain) on the campaign machine; README section 5: for Rust the floor is the target language level, one configuration |
| **Incumbent** | prost 0.14.4, tonic 0.14.6, tonic-prost 0.14.6 (from Cargo.lock, printed in every campaign header). R14: tonic-prost's codec calls `Message::encode`/`decode`, so the production path and the library entry point are the same call |
| **Questions this slice has open for the aggregating session** | (1) the proposed corpus rows of `gen/probe_corpus.py` (field numbers above 2^29-1, the 10th varint byte, two map-order rows) are not in `corpus/`; (2) no corpus row or payload has a repeated singular message with differing content, so merge-on-repeat (R-E4) is rendered and never observed; (3) a map entry has no unknown-field bag in the Rust facade (D42); (4) D23: pull's records of a group carry the group local's padding bytes as the stack held them (no-unknown TaskDetailed, bytes 348-351): a host reading members is unaffected, a byte-level comparison is not (the FSM differential zeroes padding on both sides) |

## D27: reset on entry, the core's behaviour (2026-10-10, owner; FIX-PLAN D27, ABI-v1 rule 7 superseded, R-H20 amended)

Logs `logs/rust/opt/d27-reset-on-entry/` (and the C++ slice's `logs/cpp/d27/`). No timing.
- **Core** (`poc/codec`): the `reset-on-entry` feature and the measurement exports
  (`ak_measure_reset_on_entry`, `ak_measure_{enc,dec}_set_roe`) are removed; the behaviour is
  unconditional: every top-level encode entry calls `enc_reset_on_entry` (lib.rs; what
  `ak_enc_reset` does), every decode entry (`ak_decode_*`, `ak_parse_*`, `ak_fsm_begin_*`) calls
  `rearm_on_entry` (lib.rs; `unk_arm` from the stored options pointer, NULL nothing; not in the
  no-unknown build). Rendered on lines of their own now (rust_abi.py, rust_fsm.py). `ak_enc_reset`
  and `ak_dec_reset_<Root>` stay (set the options pointer, optional early release).
- **Rust binding** (rust_binding.py): the former variant rendering is the only one: no `ak_enc_reset`
  before an encode, `ak_dec_reset_<Root>` only to set the pointer (armed once with the options at
  their stable address in `DecCtxs`, disarmed to NULL for drop), the D27 check helpers
  (`roe_rearm_check_<root>`, `roe_fail_decode_<root>`) in every binding. `binding_roe*.rs`, the
  harness/campaign/corpus feature, `binding_explicit`, bin roe_bench, gen/roe_bench.sh and
  gen/roe_checks.sh are removed (the experiment's record is commit 0f0164c0 and
  `logs/rust/opt/reset-on-entry/`; gen/roe_tables.py kept, marked historical).
- **C++ binding** (cpp_binding.py): no `ak_enc_reset` in any `encode_into_*`; `decode_with_*_unk`
  and `pull_with_*_unk` arm a context ONCE (`unk_arm_once_*`, the per-thread options at their
  stable address). Found by the corpus: a context freed with a raw `ak_dec_ctx_free` stayed in the
  binding's armed list, so a new context at the same address was taken for an armed one and
  dropped every unknown field (ffi-pull-retain, 22 Chunk* rows). Fixed in the binding: contexts made
  through `DecRoot<T>::ctx_new` / `DecCtxs` (`bound_ctx_new_*`) and resets through `DecRoot<T>::reset`
  (`bound_reset_*`) forget the context first. A context created with a raw `ak_dec_ctx_new_<Root>`
  at a reused address is still exposed (the list goes with D26). C++ STATE has the details.
- **Rust gate steps run** (`gate-stopped-at-11g.log`: the gate was started and then stopped by the
  owner's "checks, not gates" message): 0-11f all passed (conformance, shapes, counts, content
  sets, concurrency, lifecycle, R-D1, R-D6, the corpus six arms and its controls with decision
  11's "decided at entry" case PASS, pre-check 720 checks, crossing counts identical to the new
  gen/crossings.txt, the framed path, uploads, section 9's semantics), 11g's F1 and F2 (the FSM
  differential full and no-unknown) passed before the stop. Then (`d27-checks.log`,
  `fast-checks.log`): gen/d27_checks.sh (gate step 11h, new: roe_check full and no-unknown, and
  two plants in a shadow tree, the re-arm removed and the encode reset removed, both caught); the
  corpus FSM differential both builds; the no-unknown pre-check (409 checks), conformance, shapes
  and crossing counts; every slice's generator --check (`generators-check.txt`: rust, cpp, java,
  python current; csharp STALE on the C# agent's own uncommitted work in progress, not this
  unit's: its CoreFfi.cs files and cs_host.py are being edited by it).
- **Not run** (owner, 2026-10-10: checks, not gates): gate steps 11g F3-F6 (counting
  differential, corpus differential in the gate, FSM plants, C header), 12 (the no-unknown build's
  upload and RPC checks, the C header variants), 12b (TCP and h2-batch); the h2-batch variant; the
  1.88 floor; TSan.
- **Crossing counts** regenerated (`gen/crossings.txt`, `gen/crossings-nounk.txt`): the rows are
  exactly the experiment's variant counts (`logs/rust/opt/reset-on-entry/crossings/change.txt`:
  every row's forward = the previous forward - its resets, reverse unchanged, resets 0); only the
  header lines changed against them.
- **Java and Python**: not touched, not run here. Their bindings reset before every operation,
  which the core now also does on entry: redundant, the same state. Java's gate needs JDK 17 and
  8 (this container has 21 only); Python's gate was not run under the owner's rule.

## reset-on-entry (2026-10-10, owner; measurement experiment; container; ADOPTED as D27, files removed, see above)

Nothing changes by default; the ABI text is unchanged. Logs `logs/rust/opt/reset-on-entry/`.
- **Core** (`poc/codec`): feature `reset-on-entry`, default OFF. Every top-level encode entry
  (`ak_encode_<Root>`, `ak_uencode_<Root>`) resets its context on entry as `ak_enc_reset` does; every
  decode entry (`ak_decode_<Root>`, `ak_parse_<Root>`, `ak_fsm_begin_<Root>`) re-arms from the
  options pointer the context holds (`ak_dec_ctx_new_<Root>`'s or the last `ak_dec_reset_<Root>`'s),
  with `unk_arm` (the reset's code); NULL re-arms nothing. `ak_enc_reset` / `ak_dec_reset_<Root>`
  stay. Two macros at the end of lib.rs, invoked on EXISTING lines of the generated entries
  (rust_abi.py, rust_fsm.py; `UNK_LAYOUT_*` made `pub(crate)` on its line), and the `roe` fields on
  existing lines, so no line moves and the default build is byte-identical: libak_core.so sha256
  equal before and after in four feature sets (full, no-unknown, corpus, count;
  `so-base.txt`, roe-checks step 2). Measurement-only exports in that build:
  `ak_measure_reset_on_entry` (marker), `ak_measure_{enc,dec}_set_roe(ctx, on)` (per-context switch,
  so both paths run on one core in one process).
- **Generator**: `rust_binding.emit_binding(ir, reset_on_entry=True)` renders the variant binding:
  no `ak_enc_reset` before an encode; `ak_dec_reset_<Root>` only to change the pointer (armed once
  with the options at their stable address in `DecCtxs`, disarmed to NULL for drop); `ak_init_once`
  refuses a core without the marker. Variant only: `roe_rearm_check_<root>`, `roe_fail_decode_<root>`
  (the checks below). Files `crates/harness/src/generated/binding_roe.rs`, `binding_roe_nounk.rs`,
  `corpus/.../binding_roe.rs`, `binding_roe_nounk.rs`, selected by the harness / campaign / corpus
  feature `reset-on-entry` (the harness also compiles the default binding as `binding_explicit`
  under it, for the measurement). `cs_host.emit_host(..., reset_on_entry=True)`: no
  `Abi.ak_enc_reset` before an encode; `ArmFor` resets only when the core context's pointer changes
  (NULL <-> the stable Uo, the struct rewritten in place). Rendered, NOT compiled here; the C#
  default output unchanged (the C# `gen/generate.py --check` passes; `csharp-render/CoreFfi-roe.diff`).
  The default Rust bindings, roots.rs and the corpus dispatch are identical to b9adfaf7.
- **Checks** (`gen/roe_checks.sh b9adfaf7`, `checks/roe-checks.log`: ROE CHECKS PASSED): generators
  current in every slice; the default build unchanged (the .so, the bindings, crossing counts); core
  and runtime tests with the feature; variant FSM differential (44,302 / 22,172 checks, 0 failures);
  variant pre-check, full grid (6,196 / 3,485 checks); conformance and shapes; `bin/roe_check` in
  the default and variant builds (ABANDON: 310 / 155 FSM decodes stopped after begin or half their
  events, then fresh FSM, push and pull decodes on the same context equal a reference; ERR: 684 / 342
  failed decodes, the context's error slot read after the call, after other operations, after the
  next decode; ENC: 226 / 113 encodes, the output read after the call and after other operations, and
  the next encode writing exactly its own bytes): the 1,220 / 610 lines both builds print are
  identical; variant only, HOSTERR: 264 decodes the HOST fails (`ak_fail` from a grow): the code on
  the slot after the call and after other operations, cleared by the next decode, the same with the
  core's re-arm off; REARM: on 114 inputs, every position, three families, the struct rewritten in
  place with no reset (all grow -> p zeroed -> all grow, and p zeroed -> all grow) equals today's
  explicit-reset path (12,780 comparisons; 288 cases where zeroing p changes the value), and its
  control (the core's re-arm off on that context) fails on 88 of 88 inputs where a position matters;
  the variant corpus six arms 680 / 680 / 680 / 680 / 696 / 696 pass, no-unknown three arms pass, the
  FSM corpus differential both builds; decision 11's controls identical to the default build's
  except ONE case by definition (below), the plant caught; crossing counts regenerated
  (`gen/crossings-roe.txt`, `gen/crossings-roe-nounk.txt`); uploads and section 9's semantics (72
  cases) on the variant.
- **The one semantic difference found**: FIX-PLAN R-H20 / rule 1 as amended ("whether a decode
  retains is decided when the context is reset"). With the variant the decision is made at the
  DECODE ENTRY: options all zero at the reset, an entry refilled after it, a decode with no reset
  in between retains (3-byte bag) where the default build does not (no bag). unkctl states the
  variant's expectation for that case (`decided at entry (roe)`); both lines are in roe-checks
  step 8.
- **Crossing counts** (`crossings/change.txt`): on every row of both files, variant forward =
  default forward - default resets, reverse unchanged, resets 0: -1 per encode, per retained
  decode, decode-read, decode-push and decode-pull, and per C/D RPC call that encodes or decodes
  retained (-2 and -8 on d/4MiB and d/16MiB: one per chunk); drop decodes and cells B, E, F
  unchanged. 754 of 1,306 rows change (143 of 667 no-unknown). Counted warm, as before (the warm
  call armed the context).
- **Timings** (`bench/`, `bench/tables.md`; gen/roe_bench.sh, bin roe_bench; 3 launches on CPU 1,
  21 rounds x 20 ms per arm, both paths interleaved in each process on the variant core, the
  explicit path with the core's switch off on its contexts, which adds one predictable branch per
  entry to it; CONTAINER INSTRUMENTATION, absolute times). The reset calls alone: `ak_enc_reset`
  5.2-5.5 ns; `ak_dec_reset_<Root>` with the binding's options 5.4 ns (UploadResultDataMessage) to
  33 ns (ListTasksDetailedResponse, 18 positions). Encode and FSM decode-read, the two paths: most
  rows within each other's launch ranges; rows outside them go both ways (decode-read P2.2/wide
  2.234 / 2.144 ms, P6.1 retain 208.8 / 195.9 us; encode U-oneof-u-repeated 46.7 / 53.8 ns,
  transport-ready-tonic U-wire-DualResponse 211 / 231 ns, explicit / on-entry). Not attributed.

## D24: the FSM is the target decode family (2026-10-09, owner; Rust slice only; checks, no timing)

FIX-PLAN D24 / CAMPAIGN 4.0 and 4.1 req 8 as updated in 28a8dba3. The core is unchanged (A+B,
081de788); no file under poc/codec changed. Logs `logs/rust/opt/d24-fsm-target/`.
- **What decodes with the FSM now** (binding `fsm_with_<root>`, `_unk` in retain; the token
  scratch reused): the codec suite's `core-ffi` arm, decode and decode-read, drop, retain and the
  no-unknown build (so the core grid's core-ffi decode-read); cells C and D, framed and callback
  twins included (`grid::Slot::f_decode_resp`, one token scratch per slot); the harness arms
  (`core_ffi_arm::decode`, `core_ffi_unk::decode` for M1-M3, `arms_rest::ffi::dec_m4..m7`), hence
  conformance, shapes, counts, contentall, concur, lifecycle; the corpus's `ffi-drop`,
  `ffi-retain`, `ffi-nounk` (generated dispatch, gen/rust_corpus.py, `ffi_fsm`). Cells E and F
  decode in the host (core-native) and are unchanged.
- **Push** is the labelled extra `core-ffi-push` (ARMS, so in the randomised arm blocks;
  CODEC_EXTRAS; headers), `decode_push` / `dec_mN_push` in the harness (a `core-ffi-push` row in
  conformance and in the shapes agreement, `dec push` rows in counts), `ffi-push-drop`,
  `ffi-push-retain`, `ffi-push-nounk` in the corpus. **Pull** stays `core-ffi-pull`. No push twin
  of the RPC cells (rpc_suite has no decode-family switch). D23's extra arm `core-ffi-fsm` and
  `AK_FSM` are retired: the codec suite panics when AK_FSM is set (gen/d23_bench.sh and
  gen/d23_checks.sh are marked historical). Still on push on purpose: pullbench (push against
  pull), unkctl's decision-11 controls (pull == push), rdrepro, d20_toggle, decpolicy, fsm_attrib.
- **Pre-check** (codec suite, every input and mode): core-ffi's value is the FSM's; push and pull
  each equal core-ffi; the FSM's events equal pull's records and its graph push's
  (`fsm_check_root`), now always on. Core grid: full build 720 checks / 139 cases, no-unknown 409 /
  98 (before: 620 and 359, without the FSM checks); full grid, full build: 6,196 checks / 5,572 cases.
- **Crossing references regenerated** (crossings bin, counting builds): `gen/crossings.txt` 1,078 ->
  1,306 rows, `gen/crossings-nounk.txt` 553 -> 667. Added: `decode-push` rows (228 / 114), each
  equal to the old `decode` row. Changed: `decode` and `decode-read` (FSM): forward = ak_fsm_begin
  + one ak_fsm_next per further event + ak_dec_err + resets, reverse = unknown-field grows only
  (0 on every drop and no-unknown row; > 0 on 176 retain rows); events + grows = push's reverse
  on all 456 / 228 rows. RPC a and a+read of C, Cf, D, Df, C-cb, Cf-cb (e.g. rpc:C retain 4 / 3,501
  -> 3,505 / 0; rpc:C-cb drop 4 / 3,502 -> 3,505 / 1, the 1 being the completion). Every other row is
  unchanged. `crossings/change.txt` (with the two scripts and the files before).
- **Gate**: new step 11g, gen/fsm_checks.sh (D23's checks): the differential full / no-unknown /
  counting, the corpus differential both builds, six planted defects caught in a SHADOW copy of
  poc/codec and poc/rust (`target-fsm-plant/`, after an unplanted shadow pass; this tree's FSM
  checked untouched), the C header (C11 / C++17, both variants) and a C host.
- **Results**: the full gate, stable 1.94.1, on the working tree of this unit's commit (poc/rust and
  poc/codec as committed; header says "uncommitted" because it ran before the commit):
  GATE PASSED (`gate-stable.log`): conformance and shapes with the core-ffi-push rows, the corpus
  six arms 680 / 680 / 680 / 680 / 696 / 696 passed 0 failed (no-unknown three arms), crossing
  counts identical to the new files (1,324 / 685 lines), 11d-11f, 11g FSM CHECKS PASSED, 12, 12b
  (TCP and h2-batch). The floor, rustc 1.88.0, at the commit c7b3392f (header without
  "uncommitted"), every target directory of this slice removed first and rebuilt: GATE PASSED
  (`gate-floor-1.88.log`), the same counts, verdicts and plants caught. In this tree, not a fresh
  worktree: the disk had 3.9 GB free after the stable gate.

## D23 fix C isolated, withdrawn (2026-10-09, owner's option 3; FSM only; container)

The FSM is A+B (fix C reverted by the commit after 3cec33ea). Logs `logs/rust/opt/d23-fsm-c-isolate/`:
`builds.txt` (six variants, each passed the FSM differential), `bench/` + `tables.md`
(gen/d23_cparts_bench.sh: AB, C0 = A+B + inlined step + merged next checks, C0v = C0 + values cached
at begin, C0f = C0 + unchecked frame fetch, C0i = C0 + branch-free input slice, C = full fix C; two
launches each alternated, drop and retain, fsm_attrib, CPU 1), `perf/` (P2.2 fsm-core C0 vs C0f),
`cparts-probe-switch.patch` (the uncommitted generator switch the variants were built with),
`checks-final.log` (gen/d23_checks.sh on the final FSM).
- fsm-core slopes AB / C0 / C0v / C0f / C0i / C (pull-core): per event 8.41 / 8.72 / 7.72 / 8.10 /
  8.39 / 7.90 (5.38); 6-byte varint 3.68 / 3.37 / 3.39 / 3.36 / 3.39 / 3.36 (4.05); 3-byte 2.47 /
  1.86 / 1.83 / 2.47 / 1.83 / 1.83 (2.18); 1-byte 0.94 / 1.23 / 0.94 / 0.99 / 1.17 / 1.25 (1.27);
  double 0.62 / 0.99 / 0.85 / 0.62 / 0.88 / 0.61 (2.08).
- f (unchecked frame fetch) carries the string-row loss: fsm-core P2.2 637 -> 728 us, latin1 737 ->
  817, wide 782 -> 865, P4.1 93 -> 102 us against C0. Static asm of P2.2's next(): 38.3 -> 46.3 KB,
  stack references 871 -> 2,337 (stores 418 -> 445, so mostly reloads); perf: the share of next()'s
  samples on stack-touching instructions 16.9 -> 19.3 %, ops per 4 s 3,904 -> 3,456. Reading:
  removing the bounds check let LLVM restructure the step with far more spill reloads and a larger
  body; likely, not proven (no PMU for cache or uop-cache counters).
- v (cached values) gains -1.0 ns per event but costs P2.2/latin1 and /wide core (+6.7 %, +4.3 %)
  and the full decode of P2.2/latin1 drop (+10 %) and U-root-before drop (+5.5 %), outside the launch
  ranges, with the step's static asm unchanged against C0 (cause not found). i (branch-free input)
  costs P6.1 core (+7 %) and U-wire-ListMetricsResponse core (+6.6 %). C0 against AB: better on
  3- and 6-byte varints, U-wire-ListMetricsResponse and U-oneof-group drop, worse on doubles
  (PK-values-4096 +14 %), PK-flags retain, P4.1 drop (+1.1 %), U-root-before retain (+1.1 %); two C0
  retain outliers (U-oneof-group, U-wire-ListProbe) are one launch only. The swings of the packed
  probes across variants that do not touch the packed loop (doubles 0.61 to 0.99 ns) point at code
  layout as a large part of all these differences.
- Kept: none of C (no part gains without a listed-row regression; C0 itself costs on some rows).

## D23 fixes A, B, C (2026-10-09, owner-approved; FSM only; container)

Commits 55c2771c (A), 01c73821 (B), 75f819f8 (C), all poc/codec (rust_fsm.py, fsm.rs, the four
generated fsm.rs); push and pull untouched (codec.rs identical to dfa9eb04), event contract and
arena layout unchanged, no other slice's generated code changed. Logs `logs/rust/opt/d23-fsm-fixes/`:
`checks.log` (gen/d23_checks.sh on A+B+C, every slice's generator checked: D23 CHECKS PASSED),
`bench/` + `tables.md` (gen/d23_fix_bench.sh: four builds = before, A, A+B, A+B+C, alternated, 2
launches, drop and retain, fsm_attrib, CPU 1), `probe-c-inline/` (variants of C, probes only).
- A: a packed body is decoded in a tight loop in the packed arm (locals, typed values into the arena
  as before), a packed frame only to resume after a full arena. B: FRd is the prefix of the input
  ending at the open message's end (one bounds check per byte). C: step inlined into begin/next,
  next's checks merged into one branch (refusals in a cold fsm_refuse, same codes and order), group /
  arena pointers and the root's unknown-field cursor cached at begin, frame fetched unchecked, input
  slice branch-free.
- Effect (tables.md, fsm-core per-unit slopes before / A / A+B / A+B+C, pull-core beside): per
  event 9.45 / 9.29 / 8.42 / 7.81 (pull 5.15); per 6-byte varint 6.61 / 5.02 / 3.65 / 3.32 (3.99);
  3-byte 3.62 / 2.16 / 2.43 / 1.84 (2.17); 1-byte 2.47 / 1.22 / 0.93 / 1.24 (1.27); double 2.72 /
  1.13 / 0.61 / 0.62 (2.04). Rows, FSM full decode drop before -> A+B+C (push / pull): P6.1 274 ->
  194 us (201 / 206-208); U-wire-ListMetricsResponse 2.35 -> 1.74 us (1.73-1.76 / 1.91); P7.1 422 ->
  383 ns (347 / 366); P4.1 314 -> 313 us (304-309 / 282-287); small U-* rows 5 to 20 ns lower; P2.2
  1.887 -> 1.901 ms (1.877-1.895 / 1.716-1.738).
- C is mixed: against A+B it lowers the per-event cost (-0.6 ns) and the 6- and 3-byte varint
  costs, and raises fsm-core on string-heavy rows (P2.2 629 -> 676 us, P4.1 90 -> 101 us,
  U-root-before 293 -> 311 ns) and 1-byte packed values (0.93 -> 1.24 ns). Probes: the forced
  inlining is not the cause (#[inline] instead: same); C limited to inlining + merged checks keeps
  A+B's string rows (P2.2 622-637 us) but not C's per-event or varint gains. Which of C's cached
  derivations moves the string rows is not isolated.

## D23 attribution: why the FSM is slower on some rows (2026-10-09, coordinator; measurement only)

Logs `logs/rust/opt/d23-fsm-attrib/` (commands.txt says how each file was made). Container
instrumentation; nothing fixed. Tooling: bin `fsm_attrib` (push / pull / FSM interleaved round by
round in one process; core-only arms `pull-core` = ak_parse alone, `fsm-core` = begin/next with an
empty dispatch; probe `fsm-collect` = FSM events copied into a pull-format buffer then pull's
replay, rendered by rust_fsm.py, Ops::f_fsm_collect; synthetic probes PK-{ticks,codes,flags,values}-N
and EV-empty-K), gen/d23_attrib_tables.py, gen/d23_wirestats.py; perf 6.8 installed (cpu-clock
sampling only: no PMU in this VM).
- Per-unit costs (slopes, `derived-bench-2.txt`): per event, fsm-core 9.44 ns against pull-core 5.27
  (+4.17); per packed value, fsm-core minus pull-core: 6-byte varint +2.70 ns, 3-byte +1.48, 1-byte
  +1.27, double +0.55. Fixed per decode about +4 ns.
- Causes seen in perf: (1) every packed value goes around the step loop (frame reload from FsmCx,
  FRd rebuilt on the stack, jump table on the frame kind, f.n and f.pos written back to memory):
  about half the FSM's samples in a packed body; (2) the FSM reader checks both the frame end and the
  slice length on every byte and keeps pos on the stack; (3) per event, the next() wrapper (guard,
  root, state, sticky-slot checks, call) about a quarter and the step prologue (six saved registers,
  state reload, FsmU::root) about half of the step's per-event samples; (4) on facade-heavy rows the
  host builds the facade between decode steps (as push does): fsm minus fsm-core exceeds pull minus
  pull-core by 112 us on P2.2 and 18 us on P4.1; collecting the events and replaying after recovers
  45 / 5 us net of its own copy.
- Per row (drop, bench-2): P6.1 fsm 278 us, pull 208, push 202: packed values about 44 us, events
  6 us, per packed body / frame about 7 us, facade side 14 us (2 us interleaving, rest not
  attributed). U-wire-ListMetricsResponse fsm 2.31 us, pull 1.91, push 1.75: packed 0.44 us, events
  0.06 us. P7.1 fsm 414 ns, pull 389 (drop; retain 442 / 403): events 29 ns, fixed 4 ns, residual
  about 20 ns (rewound tags of the interleaved runs, not isolated). P4.1 fsm 318 us, pull 295: core
  5.5 us (events 2.5), facade side 17.8 us. Small U-* rows: +6 to +15 ns = about 4 ns per event + 4
  ns per decode.
- Candidate FSM-only fixes and expected effect (estimates from the slopes, not measured): a tight
  inner loop for packed bodies (locals, exit only on a full arena or the body's end): P6.1 about -40
  us, U-wire-ListMetrics about -0.4 us; a reader bounded by a sub-slice (one check per byte, pos in
  a register): part of the varint deltas and the leaf decodes (P1.1, U-root-before about -30 ns,
  P2.2 core up to -30 us); a lighter per-call path (step inlined into begin/next, the checks merged,
  the step's per-call derivations cached at begin): up to -3 to -5 ns per event (P2.2 -10 to -17 us,
  small rows -5 to -10 ns). The facade-interleaving cost belongs to an event-at-a-time host and is
  not an FSM core property.

## D23: the FSM decode family (2026-10-09, owner; shared core and generator; container)

Commits 8030b7f9 (poc/codec: plan, rust_fsm.py, fsm.rs, generated*/fsm.rs, declarations, every
slice regenerated) and 475b51a2 (this slice: consumer, checks, harness). Logs:
`logs/rust/opt/d23-fsm/` (`checks/rust-checks.log`, `checks/floor-1.88.log`, `checks/base/`,
`checks/events-counting.txt`, `bench/`). Nothing here is a recommendation; every timing is
container instrumentation.

- **ABI** (plan.py, THE FSM DECODE FAMILY; `fsm_entry_points`; FIXED `ak_fsm_ev`), **as amended
  by the owner 2026-10-09 (op = return value)**: `int32_t ak_fsm_begin_<Root>(ak_dec_ctx*, const
  uint8_t *buf, size_t len, ak_fsm_ev *ev)` writes the FIRST event; `int32_t
  ak_fsm_next_<Root>(ak_dec_ctx*, ak_fsm_ev *ev)` the next; both RETURN the event's op, positive
  (AK_BDR_NEW 3, AK_BDR_ADD 2, AK_BDR_APPLY_ELEM 4, AK_BDR_APPLY 1; APPLY, the root group, is
  always the last event and so is the end; no AK_FSM_END), or a negative error code with no event
  written. `int32_t ak_fsm_set_pvt_<Root>(ak_dec_ctx*, const ak_pvt_<Root>*)` copies the D20 mask
  for later FSM decodes (the FSM's own setter). A decode is begin + (events - 1) next; an empty
  message is one call. After the root group or an error, a next without begin, a context bound to
  another root, a NULL ctx or event: AK_ERR_INVALID_STATE. `ak_fsm_ev` (repr(C), 32 B, align 8,
  asserted in fsm.rs and every C header; the C# by-name probe is regenerated by the C# slice):
  `slot u32 @0, n u32 @4, token i64 @8, data *const void @16, bytes u32 @24` (4 bytes tail
  padding); no `op` member, no relation to `ak_bdr_rec`. `bytes` exact (pull pads to 8); `data`
  NULL for AK_BDR_NEW. Payload valid until the next call on the context; spans index the input,
  which must stay valid and unmoved until the root group's event or an error. (The first version,
  op a member and AK_OK / AK_FSM_END returns, is what the timing below ran; the change moves no
  work: the same write minus one member, the op returned instead.)
- **Separation** (owner's rule): the FSM's emitter `poc/codec/gen/rust_fsm.py` shares no function
  with rust_abi.py's push/pull emitter (`dec_walk` etc.); its runtime `crates/ak-core/src/fsm.rs`
  has its own reader `FRd` (varint, fixed, len_body, skip, bounded group skip), UTF-8 check
  (simdutf8), decision-11 placement `fsm_unk_put` (same grow contract, restated), frame stack, arena
  and constants (restated, equality asserted). What it reads from the context is configuration:
  root binding, counters, the sticky slot, decision 11's armed positions. Proof (checks step S):
  the four `codec.rs` byte-identical to dfa9eb04; every Rust binding's pre-D23 text a prefix of the
  new; every other generated file changed by additions only (29 files); hand-written changes with
  removed lines: corpus main.rs (arg parsing) and codec_suite.rs (header vector); counts, pullbench,
  crossings (1,092) and no-unknown crossings (567) identical to the base commit's.
- **Design**: per root a step function over a frame stack in the context (`FsmCx`, Box in
  `DecCtxImpl.fsm`, allocated on the first begin, reused): frames = root, inlined singular child
  (shares the scope's slots), non-leaf element, packed body; one absolute cursor; depth static per
  root (shapes: at most 4 of 16; the generator refuses a root needing more than FSM_MAX_FRAMES = 16,
  a push beyond is AK_ERR_DEPTH). One open run (`cur`, `n`) and one 32 KB arena (u64 words, in the
  context; same `ARENA_BYTES / size` budget per slot as push/pull, so runs and event counts are
  pull's). A flush returns the run event and leaves the cursor at the tag that caused it (re-read on
  the next call, run closed); a full arena inside one packed body returns the event and the next
  call resumes at the next value. Root group and current element group in `FsmCx.grp`. Leaf
  elements and oneof message members decoded synchronously by the FSM's own leaf decoders. Tokens
  minted 0.. per decode in pull's order. Retain: the context's armed options, grows counted as
  reverse crossings. Errors: the first error ends the decode (state FAILED); pull's quirk on a
  non-leaf element with a truncated length (NEW + empty APPLY_ELEM, then the error) reproduced
  (`pend`). A host `ak_fail` between events ends the decode at the next call with that code.
  Re-entrancy: state per context, none thread-local.
- **Checks** (`checks/rust-checks.log`, `gen/d23_checks.sh dfa9eb04 checks/base`: D23 CHECKS
  PASSED): generators current (every slice), one core; unit tests; **differential** (bin
  `fsm_diff`, campaign crate) on all 114 codec-suite inputs (16 shapes, 6 content-set rows, 92 U-*
  rows) x drop/retain: events == pull records (op, slot, token, n, payload with group padding
  zeroed), same return code, a call after the end refused, retain buffers byte-identical (bump
  grow, both families); FSM consumer graph == push's; 456 valid re-orderings (unknown field spliced
  between top-level records, records reversed); 4 synthetic inputs (packed runs over the arena
  inside one body, 8,000-string runs); 43,568 malformed variants (truncations, byte flips) under
  D20 masks 0 and all-ones, 28,558 refused by pull: same codes, same events before them; 42 API
  checks. 44,302 checks full build, 22,172 no-unknown, 0 failures. Corpus (`corpus --fsm-diff`,
  every row the ABI carries, 686 rows, accept rows also with mask all-ones): 2,458 / 1,229
  comparisons (full / no-unknown), 27,360 / 13,676 records = events, 0 failures. Plants in the
  generated FSM, each caught: token off by one, a lost run, a run split early, rewind one byte late,
  owed error dropped, mask ignored. C header: C11 and C++17 -Werror (full, no-unknown), 21 FSM
  declarations each; a C program decodes through `ak_fsm_*` against the core. Codec-suite pre-check
  with AK_FSM=1 (full grid): 6,196 / 3,485 checks 0 failures; default pre-check unchanged 620 / 359.
  Gate steps 3, 4, 5, 9, 10, 11 (corpus.sh), 11b, 11c, 12 pass. Floor 1.88.0: build + differential
  (`checks/floor-1.88.log`).
- **Events per decode** (`checks/events-counting.txt`, counting build): FSM events = pull records on
  every row; FSM forward crossings = its calls = events (P1.1 2, P1.2 8, P2.2 3,501, P2.3 876,
  P2.4 561, P6.1 1,401, P7.1 7, P5.1 1); pull's forward count of the parse is 1; reverse (grows)
  equal between the families.
- **Instrumentation** (`bench/tables.md`, `gen/d23_bench.sh`: one process per launch holding push,
  pull, FSM, 3 launches, seeded arm order, CPU 1, load < 1.0 before each, wall 619 s): decode-read
  absolutes per arm, drop and retain, all 114 inputs (228 input x mode cells). The FSM's range of
  launch medians lies above both push's and pull's in 49 cells and below both in none; the rest
  overlap. Where it lies above: the packed rows (P6.1 302-312 us against push 213-239 and pull
  222-243, both modes; the three U-wire-ListMetricsResponse rows 2.51-2.91 us against 1.89-2.24),
  P4.1 drop (351-372 us against 347-351 / 327-336), P7.1 retain (429-511 ns against 372-373 /
  386-414), and small U-* rows (the U-oneof-* family, U-wire-ListProbe / UploadResultData /
  DualResponse rows, U-root-* drop) by about 5 to 25 ns or 3 to 15 percent. Not attributed: the
  per-value frame dispatch inside a packed body and the per-event call are candidates, not
  measured (perf not run). Large string rows (P1.2, P2.2, P2.4 and their content sets) overlap.
- **Not covered**: a C# (or any non-Rust) consumer; the JVM (D23's stated later test); TSan/ASan;
  the campaign machine; the FSM in the RPC grid; timing of the no-unknown build; the FSM's own
  C# DllImport declarations (cs_binding renders FIXED's `ak_fsm_ev` struct, not the per-root
  entries).
- **What a C# consumer needs** (for the C# slice): DllImports per root `int ak_fsm_begin_<R>(IntPtr
  ctx, byte* buf, nuint len, ak_fsm_ev* ev)`, `int ak_fsm_next_<R>(IntPtr ctx, ak_fsm_ev* ev)`,
  `int ak_fsm_set_pvt_<R>(IntPtr ctx, ak_pvt_<R>* pvt)`; struct `ak_fsm_ev { uint slot; uint n;
  long token; IntPtr data; uint bytes; }` (32 B, rendered from plan.FIXED by the C# generator);
  pin the input with ONE `fixed` spanning begin and every next until APPLY or an error; loop
  `op = begin(..., &ev); while (op > 0) { Dispatch((uint)op, ev); if (op == AK_BDR_APPLY) break;
  op = next(..., &ev); } if (op < 0) fail`; dispatch on `(op, ev.slot >> 16, ev.slot & 0xFFFF)`
  with the body of today's `Replay` (`r.op` -> the returned op, `body` -> `(byte*)ev.data`, `r.n`
  -> `ev.n`, `r.token` -> `ev.token`); consume each payload before the next call; arming, retain
  ownership and the failure path are pull's; `_fwd += calls` (= events); a failed decode discards
  the partially built object. The mask is 0 unless `ak_fsm_set_pvt_<R>` is called.

## D20: decode UTF-8 validation per string field, host-selected (2026-10-04, owner; shared core and generator; container)

Commits 3882bb74 (poc/codec: plan, core, every backend, every slice regenerated), f255a65e and
4666f106 (this slice: two hand-written vtables, tooling). Logs: `logs/rust/opt/d20-utf8-bits/`.
Nothing here is a recommendation; every timing is container instrumentation. No host sets a bit.

- **Design (plan.py, "UTF-8 SKIP BITS ON DECODE")**. One `u64` mask `utf8_skip`; bit 1 = the
  core does not validate that string field (its bytes are delivered as they are), 0 = validate
  and reject with AK_ERR_TRANSCODE as before. **Push**: the FIRST member of every `ak_dvt_M`
  (`dec_vtable`'s first row, kind `utf8_skip`, data not a callback, left out of `dec_slots`),
  read at each `ak_decode_<Root>` from that call's vtable. **Pull**: `ak_pvt_<Root> { uint64_t
  utf8_skip; }` (mask first, room for later members), handed by the additive setter
  `int32_t ak_dec_set_pvt_<Root>(ak_dec_ctx*, const ak_pvt_<Root>*)`, which COPIES it into the
  root-bound context (NULL = all zero; AK_ERR_INVALID_STATE on a context bound to another root;
  both variants); every later `ak_parse_<Root>` uses it; resets and parses do not change it; push
  ignores it. **Numbering** (`utf8_bits`): the decode tree of M, preorder: M's own `string`
  fields in field-number order (singular, repeated = one bit for every element, oneof members),
  then each message-typed field (child, repeated message, map entry, oneof message member) in
  field-number order, the child's whole numbering. A child's bits are contiguous, so an element
  type's numbering is the same wherever it is reached; a type reached at two positions has a
  bit per position. Widest tree: 21 bits (ListTasksDetailedResponse / TaskDetailed, shapes and
  corpus); over 64 the generator refuses and names `uint64_t utf8_skip[ceil(n/64)]`. Names:
  `AK_DVT_<M>_UTF8_<PATH>` and `_ALL` (C header, abi.rs), `AkUtf8Skip.<M>_<path>` (C#).
- **Core**: every decode function carries `sk` (its subtree's bits): `if n != 0 && sk & (1 << k)
  == 0 && check_utf8(..)`, a constant bit per site; a child gets `sk >> offset` (0 when its
  tree has no string). Bytes fields have no bit. `DecCtxImpl.pvt_utf8_skip`.
- **Layout**: C header and abi.rs both assert `offsetof(ak_dvt_M, utf8_skip) == 0`, `sizeof ==
  rows * 8` and `sizeof(ak_pvt_<Root>) == 8`; the C# slice's by-name probe checks every dvt
  (not the pvt structs, which its binding declares but does not bind). The layout FACTS of
  section 10 still cover groups only (unchanged, 400 / 504 members agree). `ak_abi_version`
  not bumped (decision 11 did not bump it either).
- **Every host sets 0**: rust_binding, the two hand-written harness vtables (rdrepro,
  stickyerr), cpp_binding (`vt.utf8_skip = 0`; its vtable was uninitialised stack memory),
  cs_host, java_binding (`putLong(vt + 0, 0)`; its trampoline numbering is `dec_slots`',
  unchanged), py_capi (designated initialisers). No host calls the setter.
- **Checked** (`checks/`): `rust-checks.log` (gen/d20_checks.sh): generators current for every
  slice, one core; unit tests 10 + 16; the D20 core tests on all four plans (598 checks shapes,
  1,638 corpus, both variants: every bit of every root, push and pull, bit set = accepted and
  the planted span delivered, bit clear / all others set = AK_ERR_TRANSCODE, valid UTF-8
  accepted, push ignores the pvt, NULL restores, wrong root refused; one message with every
  string malformed at once: accepted only with every bit set); three planted defects (a
  neighbour's bit, pull ignoring the pvt, a child shifted by one) each caught; conformance and
  shapes VERDICT pass; counts; R-D1 and R-D6; corpus 680 / 696 per arm with controls, both
  builds; pre-check 620 / 359 checks, 0 failures; crossing counts 1,092 / 567 lines identical.
  `cpp.log`: configure, core targets, conformance a17 shared / a17 static / c11 shared / nounk
  a17 run (608 / 608 / 608 / 478 checks, 0 failures), counts_a17_shared and _static 530 rows
  identical to logs/cpp/counts-baseline.log. `csharp.log`: build_core.sh, then
  `AK_GATE_LEVELS=8 AK_GATE_KEEP_CORE=1 gen/gate.sh`: CHECKS PASSED (net8.0 only), crossing
  counts compared on every row (full and no-unknown), by-name layout agree. `java-python.log`:
  four cores; four JNI shims (gcc, generated headers' asserts) link; all 20 generated
  Binding.java compile (JDK 21 javac --release 17); four CPython shims build -Werror and import;
  through `_akffi` and `_akffi_nounk` a valid message decodes and a malformed session_id is
  refused (-6).
- **Instrumentation** (`tables.md` = `bench/tables.md`; gen/d20_bench.sh, one session, 3
  launches per variant, order rotated, CPU 1; core grid decode-read, core-ffi with core-native as
  the in-process control; bin d20_toggle confirmed each build's mask before timing). after /
  before, per-row medians over the 25 rows: full build 1.008 (rows 0.935-1.072), control
  core-native 1.018 (0.914-1.065); no-unknown 1.008 (0.944-1.069), control 0.972 (0.925-1.051).
  Rows above 1 us sit inside the control's band; on some rows under 400 ns the launch ranges
  separate in both directions (full: U-wire-UploadResultDataMessage 165 -> 177 ns, P7.1 357 ->
  370 ns; no-unknown: U-nested-before 319 -> 343 ns; but no-unknown P7.1 328 -> 322 ns,
  DualResponse 237 -> 233 ns), not resolved here. skipall / after (every string bit set): 0.733-
  0.92 on the string-dense rows (P2.2 ascii 2.08 -> 1.62 ms, P2.2 wide 2.41 -> 1.80 ms, P4.1
  325 -> 238 us; no-unknown P5.1 100 -> 74 ns), 0.95-1.13 on rows with few or no strings (P1.3, ListMetrics,
  P5.2-P5.4, P6.1). Benchmark wall 414 s.
- **Not covered**: the corpus through a host with bits set (only the core tests set bits); the
  C# full gate (net6.0 floor, net48) and its corpus/RPC levels beyond the quick checks; Java's
  own build and run-time gate (no JDK 8/17, no Maven classpath here); Python's own build.sh and
  gate (the shims were built to scratch); the pull family timed (core-ffi-pull is a core-grid
  extra, not run); content sets on payloads other than P2.2; the C# pvt layout by name; the
  campaign machine.

## D19: ak_tc_utf16 on simdutf, additive UTF exports (2026-10-04, owner; shared core; container)

Built in a worktree on 1d18e637 (commits ed31dbfd core, 761b4489..84900734 this slice). Logs:
`logs/rust/opt/d19-simdutf/`. Nothing here is a recommendation; every timing is container
instrumentation.

- **What changed in the core** (`poc/codec/crates/ak-core/src/lib.rs`): `tc_utf16` reserves 3
  bytes per UTF-16 unit through the grow contract when the buffer is shorter (the core's
  `ak_grow`, `Vec::reserve`, geometric), converts with simdutf `convert_utf16le_to_utf8` into the
  encode buffer, and on input simdutf reports invalid (a lone surrogate) runs the pre-D19 write
  loop (`utf16_write_replacing`, U+FFFD) over the same buffer. A buffer still short of the worst
  case after that request (a host grow giving less or refusing; more than INT32_MAX/3 units)
  measures the exact length and keeps the pre-D19 grow and AK_ERR_CAPACITY behaviour. The
  pre-D19 transcoder is kept unchanged as `tc_utf16_scalar`, exported `ak_tc_utf16_scalar`
  (oracle and control; no header declares it). Big-endian builds are refused (compile_error).
  simdutf = "=0.7.0" (simdutf C++ 7.7.1, built by cc with -std=c++11) is a dependency of
  ak-core only.
- **Additive exports** (pure, no `ak_init`, not counted, declared by no generated header; this
  slice declares them in `crates/harness/src/d19.rs`): `ak_utf16_to_utf8`, `ak_utf16_utf8_len`,
  `ak_utf8_to_utf16`, `ak_utf8_utf16_len`, `ak_utf8_validate`, `ak_utf16_validate`. Signatures and
  contracts are in the doc comments beside them in lib.rs.
- **Checked** (`checks/checks.log`, `gen/d19_checks.sh`, commit 9b21c21b): ak-core / ak-rt unit
  tests (8 + 16, new `d19_utf16_tests`); conformance and shapes VERDICT pass; the differential
  (`bin/tc16_diff`, scale 8): 2,991,408 UTF-16 inputs over 8 content sets (3 with lone
  surrogates) x 8 capacity / grow cases, 95.7 M checks, 0 failures, including the pre-D19 core
  BINARY (loaded RTLD_LOCAL) against the scalar export on bytes, return code and grow requests;
  the exports 109 M checks, 0 failures (against std, ak_utf8_check, the RFC 3629 edge cases,
  random and mutated bytes, canaries past every capacity); 160,000 ListResultsResponse messages
  with every string field through ak_tc_utf16 in a real encode context (fresh and reused) equal
  to prost over the lossy strings; planted `?` transcoder differs on exactly the inputs with a
  lone surrogate, planted overrun caught 1000/1000. Corpus 680 per C ABI arm with its controls,
  both builds; pre-check 620 / 359 checks, 0 failures; crossing counts 1,092 / 567 lines
  identical (no crossing changed: the Rust host never calls ak_tc_utf16).
- **Build impact** (`build-impact/`): every slice's ak-core feature set, the three h2-batch build
  forms, the Java JNI and Python shims (gcc against the cdylib; the Python module imports), a C
  program through the .so, the staticlib with g++ and with gcc + -lstdc++, the 1.88.0 floor
  (build and unit tests): all pass (`matrix.log`). The C++ slice's CMake core targets and its
  conformance arms (a17 shared, a17 static, c11 shared; counts_a17_static_lto built) pass
  (`cpp.log`). The C# slice's gen/build_core.sh: `csharp.log`. What changed for every slice:
  the core build needs a C++11 compiler; libak_core.so gains DT_NEEDED libstdc++.so.6 (1.44 MB
  against 0.84 MB); a C host that links libak_core.a with gcc needs -lstdc++ (50 undefined
  C++ runtime references without it; rustc's native-static-libs lists it); no slice does that
  today (the C++ slice links with g++).
- **Instrumentation** (`tables.md` = `bench/tables.md`, clean tree at d4ee871c; `bench-first/` an
  earlier session; `gen/d19_bench.sh`, 3 processes each; simdutf kernel here: haswell, AVX2): the
  transcoder alone, capacity ample, against the pre-D19 core's own ak_tc_utf16 in the same
  process. new/pre-D19 per process, both sessions: ascii 0.88-1.14 at 8 units, 0.35 at 32,
  0.047-0.057 at 1 Ki and 64 Ki; latin1 0.71-0.78 at 8, 0.041-0.053 at 1 Ki and up; bmp-wide
  0.95-0.96 at 8, 0.14-0.16 at 1 Ki and up; astral 1.20-1.28 at 8 (slower), 0.74-0.79 from 128;
  mixed (classes uniform at random per code point) 0.53-0.72; lone surrogates 0.39-0.59 (the
  fallback writes once where the pre-D19 code counted, then wrote). Absolutes at 64 Ki units,
  clean session: ascii 110 us -> 5.3 us (1.2 -> 24.6 GB/s of UTF-16), bmp-wide 207 -> 30 us.
  Control: the scalar export is not the pre-D19 binary's codegen (pre-D19 / scalar export:
  ascii 0.83-0.96, bmp-wide 1.06-1.15, the other sets 0.87-1.12). The Rust codec suite has no row
  through ak_tc_utf16 (the binding passes UTF-8 with ak_tc_utf8_trusted / ak_tc_bytes), so no
  codec row moves.
- **Not covered**: the C# and Java slices' own gates and corpora on the new core (their hosts
  are the ones calling ak_tc_utf16); the campaign machine (NixOS: whether libstdc++ resolves
  for every loader there, and which simdutf kernel its CPU gets); ThreadSanitizer (nightly not
  installed; simdutf is uninstrumented C++); grow-request counts on encode (the new transcoder
  asks for the worst case, so a context whose buffer is short grows earlier: 22.8 M against
  11.9 M grow calls over the differential's capacity cases; not a crossing, no committed count
  file has an encode grow column); strings over INT32_MAX/3 units (the exact-length path is
  covered with host grows that give less, not with a 700 M-unit string).

## Backward-encode experiment (2026-10-02, owner; container; patch only, poc/codec unchanged)

The owner's idea (encode backward, as upb does) with the owner's change of design mid-task: no core-side
reordering of call blocks; instead the HOST delivers every repeated field LAST TO FIRST across its calls,
each call's array in forward order, walked last to first by the codec. Everything is in
`logs/rust/opt/patches/backward-encode/` (README: what was built, the contract, the checks, the tables, what is
not measured): `backward-encode.patch` (sources: ak-rt `BEnc`, ak-core, `rust_abi.py`, `rust_binding.py`, the
experiment's harness checks), `generated.patch.gz` (the regenerated core and Rust bindings; every other
generated file of every slice, `abi.rs` and the C headers included, unchanged), `dropped-v2-sized-run.patch`.
Built and run in a private worktree (scratchpad, removed after the unit).

- **Checks** (`checks/gate/`, `gen/bwd_gate.sh`: gate.sh's steps unchanged, step 7 recorded not fatal): GATE
  PASSED on the backward core, both builds; crossing counts identical (836 / 435 lines); pre-check 5,740 / 3,257
  checks 0 failures; corpus 680 / 696 per mode; rpc_semantics, upload_check, header_diff pass. Step 7: shipped
  and global arms 0 wrong; the planted pad-widths and global+pad arms cannot fail (no learned width): the
  obligation is vacuous on this core, recorded. `bwd_check` 78/78 (n = 1, chunks, mixed forms and transcoders,
  grows mid-message and mid-field on fresh contexts, rollback after ak_fail mid-field, CAPACITY); its planted
  forward-order control fails 47 rows.
- **Mismatched pair** (`checks/mismatch/`): this branch's forward binding against the backward core links and
  runs; output lengths equal, bytes permuted on every field delivered in more than one call. Conformance fails
  on 5 payloads, the pre-check on 69 checks, the corpus on 6 arm rows; shapes passes.
- **Measured** (`bench/`, 528 s of benchmark in all; absolutes in `tables.md`): mostly inside the control row's
  build-drift band (core-native, unchanged code, disjoint launch ranges on 26 of 84 rows); the P2.4 family lower
  on the backward core; **P6.1 (packed runs) higher, 60.7-65.4 us committed against 75.2-79.5 us backward**,
  confirmed with the same harness binary (`bench-p6-2x2/`); v2 (sized hole) slower still, dropped. RPC c and d
  work through the backward core; no difference resolved.
- Tooling added (committed, defaults unchanged): `crates/campaign/benches/codec_suite.rs` case filters
  `AK_CASE_ARMS`, `AK_CASE_DIRS`, `AK_CASE_END`, `AK_CASE_INPUT` (applied after the full pre-check, recorded in
  the header); `gen/bwd_gate.sh`, `gen/bwd_bench.sh`, `gen/bwd_ab.sh`, `gen/bwd_mismatch.sh`, `gen/bwd_tables.py`.

## WP12 gates (2026-10-02, container; nothing timed)

FIX-PLAN WP12 item 1 "both slices build and gate both variants", for this slice. `gen/gate.sh` (steps
unchanged) run three times by `gen/wp12_gate.sh` at e3f6afe8 from a clean tree, rustc 1.94.1:
**h2-batch PASSED, stock PASSED, plain PASSED** (`logs/rust/opt/wp12-gates/{h2-batch,stock,plain}/gate.log`;
the plain run at d5e23249, a poc/cpp-only commit after e3f6afe8). Each run covers both builds (full and
no-unknown: steps 1-11f and 12).

- **How a variant is put in place.** ak-core is a cdylib and only a cdylib in this slice (no rlib, no
  staticlib linked), so every gate binary links it through the dynamic linker (RUNPATH, no DT_RPATH, no
  `ak_*` symbol defined in any executable: `loads.txt`, static-link check). `gen/wp12-shim/cargo` reads,
  after each build, the feature set ak-core was compiled with in that target directory (cargo's own
  artifact message), has `gen/wp12_core.sh` build the variant's core with exactly that set, and links
  `<target>/release/ak-variant` to it; `LD_LIBRARY_PATH='$ORIGIN/ak-variant:$ORIGIN/../ak-variant'` makes
  every binary of that directory load it ahead of its RUNPATH. `cargo run` becomes build + exec (cargo
  puts its deps directory first in LD_LIBRARY_PATH). 11 feature sets per variant (full, count, gw, pad,
  gw+pad, noig = `rpc` only, nounk, count-nounk, corpus, corpus-nounk; ig = full): each one's core
  sha256 and h2 source are in the gate.log footer and `cores/`. build.sh builds the sets it can (rpc and
  unknown-fields on); the no-unknown sets and the corpus cores are the same cargo command by hand with
  the same `--config` (build.sh passes `--features` only, so it cannot turn the default `unknown-fields`
  off; its last line, `strings | grep framed_write.rs`, fails under `set -e` on a core holding no h2, the
  corpus core). Every cargo build held `/tmp/claude-0/ak-codec-build.lock`.
- **Proof the variant is in effect.** The dynamic linker's record (LD_DEBUG=libs, one file per process,
  per gate step): 2,166 processes per run loaded a libak_core.so, each exactly one, each the intended
  variant core of its own feature set (`loads.txt`, LOADS CHECK PASSED in every run); c_variant.sh's
  temporary copies are named with their source path and sha256 in the gate log. The h2 compiled into each
  core: `h2-batch-src` in every h2-batch core with rpc, `h2-0.4.19` in every stock core; the corpus cores
  hold no h2 and are byte-identical across the two variants (sha256 99235c73..., 1dd4b46c...). The host
  side (tonic in the harness, the in-process servers, serve.sh's rpc_server, which links no core) keeps
  crates.io h2 0.4.19 in every run, as in the timed runs. Marker (stream_probe, d/16 MiB, k=1, TCP,
  write syscalls per call over 3 rounds): Cf 77.8-78.0 on h2-batch against 1,035-1,040 on stock and
  plain; cell A (host tonic, the control) 1,034-1,040 in all three runs.
- **RPC checks over TCP 127.0.0.1 (D10)**, after the gate, every run: upload_check, rpc_semantics (72
  cases) and header_diff with `AK_CHECK_TRANSPORT=tcp` (in-process server on TCP, pinned configuration,
  TCP_NODELAY on accept; clients pinned) on the full build, upload_check and rpc_semantics on the
  no-unknown build; burst_check on serve.sh's Unix sockets and on its TCP listener (`AK_RPC_TCP`). All
  PASSED on all three. The gate's own RPC steps (11c-11f, 12) stay on Unix sockets, as committed.
- **D16** (data after a local reset on h2-batch): no step failed on h2-batch only, so nothing needed to
  be traced to it. The cancel cases that pass (rpc_semantics' cancel, burst_check's cancel after 1 and 4
  of 8 chunks) check the CANCELLED status and a full checked stream afterwards on the same connection,
  not how many DATA frames the server received before the RST_STREAM, so they do not observe the
  divergence either way.
- Not covered: gate step 2 (the core's unit tests, `cargo test -p ak-core -p ak-rt` in poc/codec) builds
  ak-core without `rpc`, so no h2 is in that build on either variant; h2's own suite (excluded by the
  task); the 1.88 floor on either variant; the campaign machine.

## Owner decisions of 2026-10-01 and what was done

- **p1 is landed** in poc/codec (82f3712a): `SPARES = 6`, and a returned buffer waits for the
  ring's lock. Both are constants: the core read no experiment knobs, so AK_SPARES and
  AK_SPARE_LOCK are now no-ops. The C++ queue cell (one context shared by its calls) used 24 in
  its p1 measurement; that is recorded at the constant. Full gate PASSED on it
  (`logs/rust/opt/p1-landed/gate.log`).
- **h2 has two variants.** stock (crates.io 0.4.19) is the default build. h2-batch (PR 903 port
  + p4, AK_H2_COALESCE default 16) is opt-in: `poc/codec/h2-batch/` holds the patch (sha256
  c64ffd96...), `build.sh`, and a README with provenance and how each slice builds a variant
  (e4853d55). Checked on both cores (`logs/rust/opt/h2-batch-variant/`): pre-check 0 failures,
  upload_check, rpc_semantics and burst_check PASSED. The default of 16 is in effect without env
  (78 writes per d/16 call on h2-batch, against 1036 on stock).
- **Dropped by the owner:** p2, p3, p5, p8 and p9, and zero copy (p6, p7). Their patches stay
  under logs/ as history; the probe cells that use them are found by dlsym and are skipped on a
  core without them.
- **TCP only** for every benchmark from now on. gen/inproc.sh is TCP by default
  (AK_IP_TRANSPORT=uds is the explicit option): server on 127.0.0.1, TCP_NODELAY read back on
  every client socket, CPU from task-clock with the process clock beside it, server task-clock
  per call, netfilter modules in the header. Drivers not built on inproc.sh are UDS-only history.
- **TCP worker sweep:** gen/tcp_sweep.sh, AK_CORE_WORKERS 1/2/4/8 by {stock, h2-batch}, cells A,
  Cf, Cf-cb (Cf-m4, Cf-cb-m4 at k >= 16), d/16 MiB and c/P5.4 at k 1/8/16/32; tables by
  gen/sweep_tcp_tables.py: `logs/rust/opt/tcp-sweep/` (886 s of benchmark, 3 processes per
  condition), run after the owner's re-pin (47 IRQs on 0,9-10,19, 2 on 0-19, sleep inhibitor
  without limit). The first run, after a second suspend with the IRQs NOT re-pinned, is kept as
  `tcp-sweep-unpinned-irqs/` (its medians agree with the re-run within the run-to-run spread).
- **Response-delivery comparison** (owner): gen/delivery.sh, gen/deliv_tables.py, raw figures in
  `logs/rust/opt/delivery/tables.md`. Harness-only cells in stream_probe (`delivery_batch`):
  A-blk (block_on from caller threads), A-cb (spawned, completion callback on the runtime worker
  counting down a latch the caller parks on), A-q (spawned, completions posted to one mpsc queue
  per cell drained by the caller thread), Cf-q (the core's ak_queue, one per cell, one caller
  thread issues k and drains, one encode context per in-flight slot). Session a: A forms on stock
  and on h2-batch in the host (host-too; target-deliv-h2batch); session cf: A, Cf, Cf-cb, Cf-q on
  {stock, h2-batch} x core workers {8, 1}. Checked before timing (delivery/checks: server byte
  count + SHA-256 for d, response length for c, plant control fails each new cell); the plant
  first passed on A-blk, a defect in its construction, fixed before timing. Builds:
  gen/deliv_build.sh (target-deliv, target-deliv-h2batch).

## h2 PR 903 unit (2026-10-01, owner): measured, two interleaved sessions

hyperium/h2#903 (head a1f880b, base dbc204e, version 0.4.13; hyper 1.11.1 needs h2 >= 0.4.14) ported
onto h2 v0.4.19 (tree 221c21e; conflicts with #918 and #921 resolved, one port defect found by h2's
tests and fixed: DATA frames need a queue slot, other frames only buffer room), and combined with
p4 (tree b871798, AK_H2_COALESCE=N). Patches, HOWTOs, tree commits, patch and core sha256:
`logs/rust/opt/patches/h2-pr903/`, `h2-pr903-p4/`. Cores built in the worktree's poc/codec
(stack p1-p9) by gen/h2_variant_build.sh and loaded by LD_LIBRARY_PATH; host-too = the harness
built with the patch (target-pr903host).
Checked: h2's own suites against stock v0.4.19 (lib and every integration test alone; the
combination at N=16 adds one failure, send_err_with_buffered_data, which p4 alone at N=16 also
fails); pre-check 0 failures, upload_check, rpc_semantics, burst_check on the PR core, the
host-too build and the combined core at N=1 and 16; partial writev returns under burst_check.
Write counts (UDS, /proc/self/io): the PR leaves k=1 at about 1,040 writes per d/16 call (one
partial DATA frame in flight per stream, `in_flight_partial_send`), halves them at k=8 (Cf about
530; A in host-too about 490); the original head gives the same counts as the port; PR+p4 N=16
gives d/16 k=8 Cf about 105, k=1 about 135. Timed: `h2-pr903/timed/` (stock, p4, PR core-only,
PR host-too) and `h2-pr903-p4/timed/` (stock, p4, PR, PR+p4), UDS and TCP, 3 processes each,
task-clock beside the process clock, server task-clock per call; compact table
`h2-pr903-p4/headline-both-sessions.md`. Attributed: the PR core-only costs about 1.5 to 2 ms
more client CPU than stock per d/16 k=1 call at the same write count; perf record (h2-pr903/record2)
puts 18% of Cf's client cycles in the PR's poll_write_buf, about 92% of whose samples are the
stores initialising its 1,024-entry IoSlice array on every write.
Syscall census (strace -f -c, untimed, whole process): `h2-pr903-p4/strace/tables.md`.
Harness added: stream_probe AK_PROBE_TASKCLOCK (inherited perf task-clock), AK_PROBE_SERVER_PID
(per-thread server task-clock), io_syscr/io_syscw per round; inproc.sh @SPID@ and the core named
per condition (LD_LIBRARY_PATH honoured); gen/h2_variant_build.sh, h2_pr903_checks.sh,
h2_pr903_bench.sh (AK_H9_SESSION=2), h2_pr903_strace.sh, h2_pr903_record.sh, h2pr903_tables.py,
h2pr903_headline.py, h2pr903_strace_tables.py.
Machine: suspended 02:00:53 to 06:53:46 local; both sessions ran after the IRQ re-pin. One
unlocked debug build (05:09:08Z to 05:09:17Z) overlapped the C++ agent's session h2 (its
processes 0065 to 0071), none of this slice's.

## Physical-machine optimisation unit (2026-09-30, owner's goals 1 and 2): patch experiments, all measured

Rules followed: every core change in a private worktree (scratchpad `wt-rust`), a patch file in
`logs/rust/opt/patches/<name>/`, checks per patch (`gen/patch_checks.sh`: codec pre-check both
builds 0 failures, upload_check, rpc_semantics; plus the server's SHA-256 path for each new cell),
nothing committed under `poc/codec`. Timed runs under `flock /tmp/ak-physical-bench.lock`, own
server per session, client 1-4,11-14, server 5-8,15-18, 8 workers everywhere; builds on 0,9,10,19.

| Patch | What | Stack | Cells | Log |
|---|---|---|---|---|
| p1-ring | ak-rt ring size AK_SPARES and AK_SPARE_LOCK (knobs; default = HEAD) | on HEAD | (all core cells) | patches/p1-ring |
| p2-take-framed | additive ak_enc_take_owned_framed | p1 | Df-1f (probe) | patches/p2-take-framed |
| p3-exec-slot | host executor slot: ak_runtime_new_hosted, ak_task_poll, ak_task_free, waker vtable; hyper via Endpoint::executor | p1 p2 | Cf-cb-1rt (probe); Cn-1rt is harness only | patches/p3-exec-slot |
| p4-h2-coalesce | patched h2 0.4.19: a queued DATA frame spans AK_H2_COALESCE max frames, one vectored write | dependency patch, core built in poc/codec (LD_LIBRARY_PATH); `host-too` also the harness | (core cells) | patches/p4-h2-coalesce |
| p5-deferred | additive ak_call_send_deferred (the transport encodes each chunk on its writing worker; wait 0/1) | p1-p3 | Cf-enc, Cf-encp | patches/p5-deferred |
| p6-zero-copy | ak_enc_set_zc, ak_call_send_enc_zc: large blobs borrowed from the host, released per message | p1-p5 | Cf-zc | patches/p6-zero-copy |
| p7-deferred-zc | ak_call_send_deferred_zc: deferred head encode on a zero-copy context | p1-p6 | Cf-zcp (wait 0), Cf-zcw (wait 1) | patches/p7-deferred-zc |
| p8-cb-inline | AK_CB_INLINE=1: a callback/queue send that fits the channel completes inline; AK_CORE_CHAN_DEPTH | p1-p7 | (Cf-cb) | patches/p8-cb-inline |
| p9-cb-at-take | AK_CB_AT_TAKE=1: a callback/queue send queued with its completion, delivered when the body takes it (no task per send) | p1-p8 | (Cf-cb) | patches/p9-cb-at-take |

Harness (committed, defaults unchanged): stream_probe gained k, direction c, perf control
(AK_PERF_CTL, AK_PERF_CELL), thread pinning, AK_PROBE_SKIP_MISSING, cells A2, Ff-1f, Df-1f,
Cn-1rt, Cf-cb-1rt, Cf-enc, Cf-encp, Cf-zc, Cf-zcp, Cf-zcw (the patched ones found by dlsym);
grid.rs hosted core clients (`CoreClient::new_hosted`, AK_CORE_HOSTED=1 for upload_check and
rpc_semantics); bins burst_check (p4 properties); gen/attrib.sh, perf_classify.py (System.map
kernel buckets), attrib_tables.py, inproc.sh, inproc_tables.py, optstack_tables.py,
patch_checks.sh, cargo-shim, probe/allocprobe.c AKP_BT + probe/akp_bt.py.
Stability campaign (physical-probe/stability, 858 s, 28 alternating repetitions of HEAD and the
stack, pinned allocator, one lock hold): gen/stability_campaign.sh, gen/stability_tables.py;
every inproc.sh header now carries the isolation read from cgroups, IRQ affinity and thread masks
(gen/machine_header.sh). Cf-cb attribution: logs/rust/opt/cb-track (the gap is on the core
runtime's workers; 1 core worker instead of 8 removes it; p9 halves it at 8 workers).
Since 2026-09-30 evening every gen/inproc.sh session checks the server's affinity (every thread)
around every client process and the probe checks its own (AK_EXPECT_CPUS) before and after its
timed rounds; a mismatch aborts. Probe cells `<cell>-m<N>`: N core clients, call i on client i % N.
Attribution facts, measured: A's d/16 cost is bimodal by glibc malloc trim (static thresholds
remove it; owner: main figures under GLIBC_TUNABLES static thresholds, plus one default pass);
every Rust cell writes about 1,030 writev per d/16 call; Cf's residual over A is copy_from_user of
data encoded on another CPU. Figures: JOURNAL 2026-09-30 and `physical-probe/opt-stack/tables.md`.

## Physical-machine probe (2026-09-29/30): segment 1 (main, 8 workers) and segment 2 (variant, 4 workers; client-only control) RUN

Segment 2: shared server `server-s4/` (AK_SERVER_THREADS=4, pid 152128, state
/tmp/ak-physical-s4.state, warm 64, LEFT RUNNING for the C++ var4 segment); `variant-w4/` (host 4
/ core 4, 3 passes, benchmark wall 115 s) and `variant-c8/` (host 8 / core 8 against the same
4-worker server, cells probe A, A2, Df, Cf, grid A, Df, Cf, 2 passes, 57 s). No abort. The
level of A seen in main-w8's spread passes (below) is recorded in the journal entry of
2026-09-30 "segment 2" with the facts that separate it from the session's start.

Segment 1: `logs/rust/opt/physical-probe/main-w8/` (tables.md), shared server `server-s8/`
(AK_SERVER_THREADS=8, 5-8,15-18, pid 147567, state /tmp/ak-physical-s8.state, warmed with
serve.sh warm 64, LEFT RUNNING for the C++ main segment), client 1-4,11-14, host 8 / core 8
workers, commit f57173ff; benchmark wall 242 s plus a 4 s attribution pass; no abort. Recorded in
the journal: in the four spread probe processes (cells A, A2, Df) A and A2 measured 10.45-11.57 ms
per 16 MiB call in 7 of 8 instances against 7.58-8.27 in the main probe processes, so the spread
passes' probe Df - A (-2.02 to -3.29) and A2 - A range (3.21) carry that level; the first spread
grid pass's A d/16MiB k1 (11.29) likewise against 7.47-7.95 in the other three.

### Preparation

Campaign machine (i9-7900X, SMT on, governor performance, no_turbo 1, 3.3 GHz min = max, one
NUMA node, no isolation: taskset only). Question (FIX-PLAN WP11, coordinator step 2): direction
d's A -> Df and Df -> Df-chan client CPU, with c/P5.4 beside it, k = 1 and 8.

| What | As built |
|---|---|
| Driver | `gen/physical_probe.sh main\|variant\|smoke OUT_DIR`: shared-server mode (AK_SERVE_STATE names a running serve.sh server; the driver dials its pinned socket, never starts or stops it, checks pid, affinity = AK_CPU_SERVER and worker count = the preset's before every client process, aborts otherwise); AK_PP_OWN_SERVER=1 for a smoke. Header: CPU, kernel, SMT, no_turbo, governor and scaling min/max/cur frequency per CPU of both sets, siblings, isolation mechanism, cgroup, the server's pid/affinity/workers/threads, the client's worker knobs, rustc, binaries' sha256 and the core each loads |
| Presets | main: host 8, core 8, server 8 workers; 4 spread passes (probe A, A2, Df; grid A, Df) then 3 main passes (probe A, A2, Df, Df-chan, Cf, Cf-cb, C, C-cb; grid A, Df, Cf, Cf-cb, C, C-cb) then one attribution pass (not timed). variant: 4 / 4 / 4, 3 passes (probe A, A2, Df, Df-chan, Cf, Cf-cb; grid A, Df, Cf, Cf-cb). Grid workloads c/P5.4, d/4MiB, d/16MiB x k 1, 8; probe d 16/4 MiB, k 1 |
| Pass | one probe process (block order, cell order rotated per pass) and one grid process (criterion, seeded order, seed = pass), alternating which runs first |
| Notable | the range over the spread passes of the per-pass in-process Df - A (per workload), with the A2 - A range (probe) as the same-code floor; the main passes' own per-gap ranges beside it |
| Harness additions (defaults unchanged) | rpc_suite: getrusage deltas per sample (`ru_nvcsw`, `ru_nivcsw`, `ru_minflt`), AK_RPC_PAYLOADS narrowing; stream_probe: cell `A2`; `gen/physical_tables.py` (absolutes, gaps per pass, ranges, switches/faults, threads, attribution; a side-by-side of sessions) |
| Checks on this machine | gate at cb37633f PASSED (`prep/gate.log`: pre-check 5,740 / 3,257 checks 0 failures, crossings 836 / 435 identical, upload byte check, rpc_semantics, corpus, both builds); `prep/upload_check.log` PASSED; smoke of every phase with its own server and the shared-mode abort control (`prep/smoke/`, no figures kept) |
| Build directory | the machine's `~/.cargo/config.toml` sets `build.build-dir` to one directory per workspace, so every target directory of this workspace shared one `libak_core.so` (RUNPATH); `gen/cargo-shim/cargo` makes the build dir the target dir; serve.sh and the driver use it, the gate was run with it on PATH |

## Framed default (2026-09-28, owner; container instrumentation)

The core's framed send path is the DEFAULT (unary and streaming, every delivery; the
reference tonic-codec path via `ak_client_set_framed(c, 0)`); each request message goes out
as ONE body frame (5 bytes of headroom in every core encode context, `ak_rt::Enc::head` /
`take_framed`; copying entries copy with the prefix); the encoder's spare slot is a ring of
3 (`ak_rt::enc::SPARES`). The codec's output is unchanged (the message alone). Cells keep
their names: C/B/E = reference path (labelled row), Cf/Bf/Ef = framed path (the core's
default); the grid sets the path explicitly on every core cell. Checks and the in-session
measurement: `logs/rust/opt/framed-default/` (checks/, ring-size/, session/tables.md,
alloc/, counts/). Other slices' harnesses set the path only for their framed cells, so
their reference cells now run framed until they call `ak_client_set_framed(c, 0)` (listed
in the report to the aggregating session).

## Runtime probe (2026-09-28; container instrumentation; harness knobs kept, core patch reverted)

Question (coordinator): is Cf-cb's and Df-chan's extra client CPU over A in direction d the
per-chunk cross-thread hand-offs, made worse by two tokio runtimes on one client CPU?
Harness (kept, defaults unchanged): `AK_HOST_WORKERS` (N, or `ct` = current-thread) for every
cell's own runtime (`cell-rt`: A/D/F and the -cb cells) and `AK_CORE_WORKERS` for
`ak_runtime_new`, read by grid.rs (`host_workers`, `core_workers`, `host_runtime`), printed in
rpc_suite's and run_campaign.sh's headers and in the probe's; the probe adds `AK_CHAN_DEPTH`
(Df-chan's mpsc depth), getrusage deltas per round (voluntary / involuntary context switches,
minor faults), and the thread count per class after warm-up. Df-chan refuses a current-thread
host runtime (nothing drives the body while the host thread blocks in blocking_send).
Scripts `gen/runtime_probe.sh`, `gen/runtime_tables.py`. Core experiment: the stream channel's
depth from `AK_CORE_CHAN_DEPTH` (`logs/rust/opt/runtime-probe/core-depth.patch`), built in a
separate target, checked (upload_check depth 1-4, rpc_semantics depth 1-2, codec pre-check
5,740 checks 0 failures at depth 2), measured, reverted; not committed.
Measured: `logs/rust/opt/runtime-probe/tables.md` (two sessions, 597 s of benchmark). Every
variant changes the context-switch counts as expected; none moves the in-process gap of Cf-cb
or Df-chan to A beyond the session spread, and the two sessions disagree on the sign for h1,
c1 and b1 (JOURNAL 2026-09-28 "runtime probe"). Depth 2 moves the host's wait from the sends
to the final recv and adds 0.5-1.25 fresh >= 1 MiB allocations per 16 MiB call, no
per-call CPU change resolved; depth 3/4 and k = 8 not run.

## Stream probe (2026-09-28; container instrumentation, no code change kept)

Tooling: `bin/stream_probe` + `gen/stream_probe.sh` / `.py` (direction d only, k = 1, one
process, per-thread CPU by class, faults, context switches, allocation counts through the
LD_PRELOAD shim `gen/probe/allocprobe.c`), `gen/stream_ab.sh` / `.py` (alternated A/B
processes). The cells' own runtime threads are named `cell-rt` in grid.rs (harness only).
Measured and ablations: JOURNAL 2026-09-28 "the stream probe"; logs `logs/rust/opt/stream-probe/`.
No core change kept (the spare-slot ring (c) was reported, not kept).
Stream probe 2 (same day): cells `Df-chan`, `Cf-split`, `C-split` in the probe only; in-session grid control
(`gen/stream_probe2.sh`); HTTP/2 sniffer (`gen/probe/h2sniff.py`); env-gated A/B (`AK_AB_ENV_B`).
The framed paths' extra cost over Df is measured as the encode into a fresh buffer; the one-frame
prefix headroom and the spare ring are reported, not kept.

## Callback deliveries (2026-09-28; every figure is container instrumentation)

Owner: "use callback with oneshot channel for the core-transport"; CAMPAIGN req 16 as amended
(c1d3db50): the core-transport cells use the host's idiomatic core delivery, for Rust the
callback bridged to async with a tokio oneshot; the blocking cells stay as a labelled row.

| What | As built |
|---|---|
| Core (afc585db, additive) | the stream's callback and queue deliveries `ak_call_send_cb/_q`, `ak_call_send_enc_cb/_q`, `ak_call_recv_cb/_q` (int32_t: AK_OK and one completion follows, else the refusal and none; a tag on every form) and `ak_call_unary_enc_cb/_q`; one call path per operation with the blocking forms (shared prologue/epilogue, the same send and response futures). A send completes when the call has accepted the message (empty bytes), after the sender is back in its slot. Refusals at the entry: send limit AK_ERR_LIMIT, a send while one is pending or after last AK_ERR_INVALID_STATE, a second recv of any delivery AK_ERR_INVALID_STATE. A send on an ended call completes AK_ERR_HOST (-1), a cancelled pending send too; the recv completion carries the status (CANCELLED, RESOURCE_EXHAUSTED, the server's code) |
| Cells (7d10668d) | B-cb, C-cb-*, E-cb-* and framed twins Bf-cb, Cf-cb-*, Ef-cb-*: every direction a, a+read, b, c, d through the callback forms, each completion into a tokio oneshot awaited by one of k tasks on the cell's runtime (2 workers). The REFERENCE core cells for Rust; the blocking B, C, E cells are the labelled row. Tables mark `(ref)` / `(blk)` |
| Checks | `logs/rust/opt/cb-deliveries/checks/`: generate --check, one_core, pre-check 0 failures (both builds), crossing files regenerated with additions only (+60 / +36 rows), rpc_semantics 72 cases on both builds, upload_check both builds (cb controls included), the C++ slice builds against the new header. No full gate run in this unit |
| Grid | `logs/rust/opt/rpc-same-machine-cb/` (settings of 22a08fe2 plus the cb cells): 1,288 entries, 783 s, 353 one-batch entries (k = 8) |

## Unit 3: ABI v1 section 9 as specified (every figure is container instrumentation)

Spec: ABI-v1.md section 9, "Streaming, as built", "Two more additive entries" (fe79f874) and
"The status number, on unary calls too" (22ebb97f). Implemented once in `poc/codec` (plan.RpcAbi
and the ak-core/rpc crates) and rendered into every slice's header and binding; other slices'
generated output regenerated (b3ac5050), this slice's hosts and tests in f9c25d0b.

| What | As built |
|---|---|
| Call kinds | `AK_CALL_CLIENT_STREAM` 1 (built), `AK_CALL_SERVER_STREAM` 2 and `AK_CALL_BIDI_STREAM` 3 reserved: `ak_call_open` returns NULL |
| Options | `struct ak_kv {key, key_len, val, val_len}`, `struct ak_call_opts {deadline_ms, metadata, n_metadata}`; `ak_call_open(c, path, path_len, kind, opts)`, opts nullable. Deadline -> `grpc-timeout` plus a client-side timer (tonic's server reports an expired grpc-timeout as CANCELLED "Timeout expired", mapped to DEADLINE_EXCEEDED). Metadata: `-bin` keys binary, others printable ASCII 0x20-0x7E (checked explicitly: tonic accepts obs-text); an invalid pair -> NULL. Both send paths |
| Cancel | `ak_call_close` removed; `ak_call_cancel` on a stream handle unblocks a pending send/recv (CANCELLED) and frees nothing; on cb/q handles it now delivers a CANCELLED completion (before: the task was aborted and no completion arrived) |
| Status | `AK_ERR_RPC_STATUS` = -12. `ak_call_recv(h, out, int32_t *grpc_status)`: the code is written whenever the call completed; AK_OK iff 0; misuse (AK_ERR_INVALID_STATE) leaves it untouched; send on a failed stream AK_ERR_HOST. Unary: `ak_completion {tag, status, grpc_status, bytes}` (size 40, bytes at 16 as before: grpc_status sits in the old padding) and a trailing nullable `grpc_status` on `ak_call_unary` / `ak_call_unary_enc`; -1 when no call reached the transport |
| Limits (D44) | `ak_client_opts.max_send_message` / `max_recv_message` enforced on every call, delivery and send path, per message on a stream; 0 = tonic's default (unlimited send, 4 MiB receive). Over the send limit: AK_ERR_LIMIT before anything is sent (unary grpc_status -1). Over the receive limit: RESOURCE_EXHAUSTED (8) on a stream, AK_ERR_LIMIT with grpc_status 8 on unary (tonic's OUT_OF_RANGE decode error is translated). The grid runs 0/0 (stated in rpc_client's header: the largest response is P2.2, 540,422 B; requests are under an unlimited send); packages/rust's transport config unchanged |
| Layout checks | plan.RpcAbi.layout() derives each RPC struct's 64-bit size and offsets once; asserted in every C header (AK_SASSERT under a UINTPTR_MAX guard) and in the core (rpc_check.rs const asserts); c_variant.sh passes; the C# layout probe prints ak_kv, ak_call_opts and the new ak_completion |
| Tests | `bin/rpc_semantics` (gate step 11f, both builds, reference and framed path): status on the blocking, callback and queue deliveries, on ak_call_unary_enc and on the stream; deadline; metadata echo (and grpc-timeout seen by the server); invalid metadata; cancel of a pending stream recv and of cb/q calls; misuse; reserved kinds; send limit (unary and stream, the server sees nothing over it) and receive limit (unary blocking and queue, stream). The campaign server's test paths are `StatusU<n>`, `StatusS<n>`, `SleepU`, `SleepS`, `EchoS` under the Grid service. ak-core unit tests 11 passed |
| Checks | `logs/rust/opt/abi9/checks/checks.log`: generate --check and one_core ok; pre-check 0 failures on both builds; crossing counts identical (775 / 398 rows): the new arguments are out-parameters and no entry is added on a measured path |
| Final | gate from a clean tree at 6727646b PASSED on stable (`opt/final3-gate/gate.log`) and on rustc 1.88.0 (`gate-floor-1.88.log`); a first attempt at 98b187ce stopped in step 11f on a gate-script defect (set -e and a grep with no match; `failed-98b187ce/`), fixed in 6727646b. One full opt_bench v5 run at 0a26bdc4 (`opt/final3/`, 855 s): pre-check 0 failures in all four codec processes, the 48 planted controls aborted; its tables, absolute and with no comparison, in `tables-codec.md` and `tables-rpc.md` (gen/opt_tables.py) |
| O1 | the full client's Bf cell on the shipped transport: observed in the container, not investigated (owner: "most likely VM contention or system activity"), deferred to the campaign machine |

## Optimisation unit 2 (done; every figure is container instrumentation)

Owner-approved steps 0-6 (T1 native, T1 ffi, N2, N3, N6, N5), then the owner's additions
(the framed send path, core-only LTO, U1-unary, U2-stream), one commit per step, each
measured once in full with opt_bench v5 (`logs/rust/opt/<step>/`) against the previous kept
step (`variants-before-after.txt`, `by-direction.txt`), iterations inside a step with
narrowed alternated runs (`gen/opt_narrow.sh`, `gen/opt_ab.py`). Per-step checks:
`gen/step_checks.sh` (generate --check, one_core.sh, pre-check both builds, counting builds
vs the committed crossing files) -> `<step>/checks/checks.log`. A stable gate checkpoint runs
after step 5, the full gate (stable + 1.88) at the end.

| Step | Commit | What | Kept | Checks | Log |
|---|---|---|---|---|---|
| 0 | 19b339a2 | encode variants name the transport form: `transport-ready-tonic` (cells A, D, F) and, core arms only, `transport-ready-core` (cells C, E; the host does nothing after the encode, so it repeats the reused-buffer op) | yes (harness) | pre-check 0 failures; crossings identical | `opt/t0-ref` |
| 1 | cf844df5 (amends the first form, 5c1a31d1, not on the branch), aac7120e, 38f3e701 | T1 native: `ak_rt::Enc::take` moves the encoded buffer into a `Bytes` (from_owner) and a dropped body returns it to a spare slot; cell F and core-native's tonic row use it; ak_call_unary_enc uses it (R2's swap moved into ak-rt). The BytesMut forms (first three commits) were superseded: they cost 5-20% on the reused-buffer encode | yes (38f3e701) | pre-check 0 failures; crossings identical; ak-rt take tests | `opt/t1-native` (+ `-first`, `-b`, `-swept`, `-ab`, `-c-ab`) |
| 2 | 1c181021 | T1 ffi: additive RPC entry `ak_enc_take_owned(enc, out)` (owned ak_bytes, released with ak_bytes_free, buffer back to the context's spare), used by cell D and core-ffi's transport-ready-tonic row | yes (owner; RawEncoder still copies once into tonic's buffer on the reference path, so D goes 2 -> 1 copies, 0 with option 3) | pre-check 0 failures (moved forms' bytes checked); crossings rpc:D b +1 forward (ak_bytes_free), regenerated | `opt/t1-ffi` |
| 2b | d18540f0 | T1 option 3 (owner: optional): the FRAMED send path as labelled extra cells Bf, Cf, Df, Ef, Ff (`rpc::unary_framed`, two body frames, no copy; core switch = additive `ak_client_set_framed`), the reference path unchanged beside it | yes (extra cells) | request headers identical on the wire (`bin/header_diff`, gate step 11d); plant control per send path (A, B, Bf, Df); crossings: new rows equal to their reference twins | `opt/framed`, `opt/framed-rpc-narrow` |
| 3 | b1ecc8f8 | N2: core-native decode counts repeated message/blob fields in one key pass and reserves each Vec exactly | yes | pre-check 0 failures; crossings identical | `opt/n2`, `opt/n2-ab` |
| 4 | 1326b546 | N3: core-native retain encode skips an empty unknown-field bag | yes | pre-check 0 failures; crossings identical | `opt/n3`, `opt/n3-ab` |
| 5 | none | N6: ffi-retain P4.1 decode over drop -- not reproduced (retain/drop 0.90-1.11 across 9 processes; one extra forward crossing, the reset, and no per-element one) | no code | -- | `opt/n6-probe` |
| 5b | (tooling only) | fat LTO on the core cdylib only (gen/core_lto.sh, its own cargo invocation; host loads it first through AK_CORE_LIB_DIR, checked with ldd; host not LTO'd, inline_check unchanged) | **not kept** (ffi 1.02-1.04 in the narrowed A/B, prost 0.93-1.02; tooling opt-in, off by default) | -- | `opt/lto-ab` |
| U1 | f1dc5de8 | U1-unary (owner): labelled extra RPC direction `c`, an upload of P5.3 / P5.4 (M5) for every cell and framed twin at k = 1 and 8; grid server receive limit 8 MiB | yes (extra direction) | req 18 per call and a plant per send path x direction (16 controls aborted); crossings: new c rows | `opt/u1-unary`, `opt/u1-unary-narrow` |
| 6 | 19dc9237, reverted by 0da6fcfe | N5 experiment: apply-first element order (`ak_decode_<R>_af`, plan option elem_order, fallback on arena or held-table overflow) | **reverted** (core-ffi push decode 1.03-1.10 slower in two narrowed A/Bs) | pre-check 0 failures incl. two fallback inputs; corpus passed; crossings -1 reverse per element (reverted with it) | `opt/n5`, `opt/n5-ab`, `opt/n5b-ab` |
| U2 | 122dc8ae | U2-stream (owner): ABI section 9's client streaming in the core (`ak_call_open`, `ak_call_send`, additive `ak_call_send_enc`, `ak_call_recv`, `ak_call_close`; blocking, both send paths) and the labelled extra RPC direction `d` (4 MiB / 16 MiB in 2 MiB chunks, M5 per chunk, k = 1 and 8, every cell and framed twin) | yes (extra direction) | upload byte check (count + SHA-256, controls) in the gate (11e, both builds); plant per send path x direction; ak-core unit test; crossings: new d rows | `opt/u2-stream` |

## Optimisation experiment (done; every figure is container instrumentation)

The owner approved a list of optimisation candidates, core and shared generator included
(the other slices' timings will be redone). One commit per step; each step measured with
`gen/opt_step.sh NAME PREV` (= `gen/opt_bench.sh` into `logs/rust/opt/NAME`, then
`gen/opt_compare.py` against the previous kept step and against `baseline2`, calibrated by
the A/A pair `baseline2` / `baseline2-aa`). The codec pre-check (0 failures) and both
crossing-count files hold on every step; the full gate runs once at the end. Harness v3:
the interleaved sampler (AK_ORDER=interleave), not criterion (JOURNAL, step 0). **Since
removed by the owner** ("if criterion cannot interleave, forget about interleaving"):
gen/opt_bench.sh is now harness v5: the MERGED campaign harness (criterion, process CPU,
requirement-11 encode variants, R-H23 order, RPC cells A-F), fewer samples, one launch, so
every opt run in the table below was taken with a harness that no longer exists. The two
runs on harness v5: `opt/merged` (the merged HEAD) and `opt/merged-before` (a clean
worktree of origin/rust/native-core-ffi-poc baa173a2, their fixes and none of our
optimisations, the same script). Absolute times per variant: `variants-codec.txt` in
each; before -> after: `opt/merged/variants-before-after.txt`,
`opt/merged/headline-before-after.txt`. Geometric mean after/before over the 128 payload
rows: prost 0.97 (the control, unchanged code), armonik 0.98, native-drop 0.84, ffi-drop
0.74, ffi-retain 0.73, pull-drop 0.70, native-nounk 0.83, ffi-nounk 0.72.
Hazard: between steps the incumbent's encode median has moved 7-10% (steps 1, 3, 4) with no
change to its code, so the /inc encode ratios carry it; core-ffi/core-native is the check.
A deliberate 32-byte layout shift (`opt/layout-exp`, A/B/A/B) moved group means by at most
3.4% (A/A: 1.8%), so a small shift does not explain it; the cause is not identified.

| Step | Commit | What | Kept | Main change (in-process ratios, group geometric means) | Log |
|---|---|---|---|---|---|
| 0 | f08a3d9 | harness v3, A/A pair | yes | A/A ratio rows: median 1.2-4%, p90 4-13% | `opt/baseline2`, `opt/baseline2-aa` |
| 1 | e25da96 | D1: binding s_of follows the plan's utf8 (no host re-validation) | yes | core-ffi/inc decode 0.74-0.78 x (P), 0.70-0.77 x (U); core-native/inc decode noise | `opt/s1-d1` |
| 2 | 59b16ec | E1: one-pass write of a passthrough blob (ak-core enc_blob) | yes | core-ffi/inc encode 0.84-0.87 x (P, U, all modes); core-ffi/core-native encode 0.83-0.86 x; core-native/inc noise | `opt/s2-e1` |
| 3 | fdecae0 | E2: sparse fill clears min(n, chunk); core-ffi encode arm (and RPC C/D) on the sparse path | yes | core-ffi/core-native encode: P1.3 2.40 -> 2.08 (drop), 2.43 -> 1.67 (retain), 2.66 -> 2.10 (no-unknown); groups 0.96-1.00; the /inc groups moved 0.88 with core-native/inc (incumbent layout drift) | `opt/s3-e2` |
| 4 | ab133a3 | E4 + N1: packed bool/enum without a heap Vec (binding); core-native packed decode reserves | yes | core-native/inc P6.1 decode 0.62 -> 0.44 (drop), 0.65 -> 0.40 (no-unknown); core-ffi/core-native P6.1 encode 1.11 -> 1.02 (drop), 1.11 -> 1.00 (retain); core-ffi P6.1 decode ratio up 10-19% with no decode change (see JOURNAL) | `opt/s4-e4n1` |
| 5 | 56ca80c | U1 + U2: retain options at a stable address, one reset per decode, no disarm; UNK_LIVE a map (geometric growth held back then, applied later on the owner's decision, see below) | yes (cleanliness: rule 7, O(1) buffer tracking) | no group beyond noise; in-process core-ffi retain/drop decode on U rows 1.281 -> 1.263 | `opt/s5-u1u2`, `opt/s5-u1u2-heldback` |
| 6 | b259af0 | E3 + D2 + D3b + D4: small encode chunk + out-of-line big path; one shared decode arena; in-place element groups; pull runs straight into the record buffer | yes | core-ffi/core-native decode 0.950 (P drop), 0.956 (P retain), 0.946 (U drop); P1.3 1.41 -> 1.05, P6.1 1.55 -> 1.34, P7.1 1.33 -> 1.16; core-ffi-pull/inc decode 0.94 (P), 0.90 (U); no-unknown decode 0.986; encode (E3) noise | `opt/s6-e3d2d3d4`, `opt/s6-sanity` |
| 7 | 85d9034 | R1: raw decoders return copy_to_bytes directly (rpc crate, rpc_server); R2: additive `ak_call_unary_enc` (the encode context's buffer moved into the request, recycled through a slot) used by cell C direction b | yes (R1 cleanliness; R2 additive, no gain resolved) | every RPC cell/A row noise; cell C direction b (full client) 0.02-0.08 lower on every in-flight but C-nounk unchanged, so not attributable | `opt/s7-r1r2` |
| 8 | f2a88d3 | F1: the no-unknown build has no facade unknown_fields member (and no core_native_retain; the corpus no-unknown build loses its native-retain arm) | yes | no-unknown build, decode: core-native/inc 0.94 (P), 0.89 (U); core-ffi/inc 0.96 (P), 0.90 (U); armonik/inc 0.96 (P), 0.92 (U); core-ffi/core-native 1.02 (both faster); full build noise | `opt/s8-f1`, `opt/s8-sanity` |
| 9 | 4bbaefe | E5: encoder reserves once per field / packed run, raw-pointer stores (ak-rt Enc; ak_run_*; core-native packed) | yes | encode: core-native/inc 0.75 (P), 0.70 (U), 0.65-0.73 (no-unknown); core-ffi/inc 0.84 (P), 0.85 (U), 0.79-0.82 (no-unknown); core-ffi/core-native up 1.10-1.23 because core-native gained more; decode noise | `opt/s9-e5` |
| 10 | ef65732 | Z1: zero-copy bulk bytes decode as a LABELLED EXTRA arm `core-ffi-zc` (AK_ZC=P5.) | yes (extra arm) | core-ffi-zc/core-ffi decode: P5.1 0.84, P5.2 0.042, P5.3 0.002, P5.4 < 0.001 (a slice instead of a copy: O(1) in the payload size); headline groups noise except U core-ffi/core-native decode drop 1.065 (core-native/inc 0.964 the other way; layout band) | `opt/s10-z1` |
| 11 | 49b928a | C2: the decoder's UTF-8 check (ak-rt check_utf8) uses simdutf8, an ak-rt DEFAULT feature (so every slice's core build gets it) | yes | decode on the non-ASCII content sets, both core arms: core-ffi/inc P1.2/latin1 0.84 -> 0.54, P1.2/wide 0.88 -> 0.45, P2.2/latin1 0.95 -> 0.75, P2.2/wide 0.98 -> 0.69 (core-native/inc the same, so core-ffi/core-native unchanged); ASCII rows no resolved change | `opt/s11-c2`, `opt/s11-sanity` |
| 12 | e804e0d, reverted by 46d14f8 | C1: codegen-units = 1 (poc/rust, corpus, poc/codec), no LTO | **reverted** | every core-ffi/core-native group noise; core arms' absolute medians inside the A/A absolute drift; incumbent encode ~10% faster in the no-unknown process only (so the no-unknown /inc encode groups rose 1.09-1.11); builds take ~2x as long | `opt/s12-c1` |

Final run (`opt/final`, code of 3c737d1, same harness) against `baseline2`, group geometric
means of in-process ratios, full build drop / no-unknown: core-ffi/inc decode 0.66 / 0.70
(P), 0.68 / 0.69 (U); encode 0.80 / 0.68 (P), 0.86 / 0.68 (U); core-ffi-pull/inc decode
0.68 / 0.70 (P); core-ffi/core-native decode 0.74 / 0.80 (P), 0.72 / 0.75 (U). RPC, cell C and
D client CPU over cell A: direction a 1.05-1.31 -> 0.87-1.01, direction b 0.55-0.65 ->
0.42-0.52. Per payload: `opt/final/per-payload-vs-baseline2.txt`. Against the first
baseline (a different harness): `opt/final/compare-vs-baseline-DIFFERENT-HARNESS.txt`.

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
gen/corpus.sh          the corpus on the full build (6 arms) and on the no-unknown build
                       (3 arms), each with its planted controls
gen/fsm_checks.sh      the FSM family's checks (gate step 11g): differential, corpus
                       differential, six plants in a shadow tree, C header and C host
gen/d27_checks.sh      reset on entry's checks (gate step 11h): bin roe_check, both builds, two plants
gen/roe_tables.py      historical: the reset-on-entry experiment's tables
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
`core-ffi` (through the C ABI; decode with the FSM family since D24), `core-ffi-push`
(push decode, labelled extra, D24), `core-ffi-pull`. Corpus arms: full build `ffi-drop`,
`ffi-retain` (FSM decode, D24), `ffi-push-drop`, `ffi-push-retain` (push, D24), `native-drop`,
`native-retain`; no-unknown build `ffi-nounk`, `ffi-push-nounk`, `native-nounk`
(core-native's drop rendering over the facade without `unknown_fields`; no retain
rendering exists in that build, R-H22). RPC cells (`crates/campaign/src/grid.rs`): A prost +
tonic, B prost + core transport, C core-ffi + core, D core-ffi + tonic, E core-native + core,
F core-native + tonic; C-F in retain and drop (full build) or no-unknown (no-unknown build).
Labelled extra cells Bf, Cf, Df, Ef, Ff: the same on the FRAMED send path
(`rpc::unary_framed` / `client_streaming_framed`: tonic's Channel, each message sent as its
5-byte prefix frame and the caller's Bytes, no copy into tonic's buffer; the core's client
switched by `ak_client_set_framed`, the harness calling rpc for D/F; request headers
identical to the reference on the wire, `bin/header_diff`). Directions: a, a+read, b (P2.2),
and the labelled extras c (U1-unary: an upload of P5.3 / P5.4, the server decodes it with
prost and answers empty; k = 1, 8) and d (U2-stream: a client-streamed upload of 4 MiB / 16
MiB in 2 MiB chunks, M5 per chunk, the ids on the first; the server answers the data byte
count; k = 1, 8; B/C/E through `ak_call_open` / `ak_call_send` (C `ak_call_send_enc`) /
`ak_call_recv`). `bin/rpc_semantics`: the section 9 semantics test (unit 3, gate 11f). The grid's server accepts 8 MiB messages (P5.4 is over tonic's 4 MiB).

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
| 11 | serialised once per iteration; encode variants | met: prost recomputes `encoded_len` on every encode, the core encoders reset per call, no size memo. Every encode arm x mode has labelled rows (R-H29): end_state `reused-buffer`, `transport-ready-tonic` (the form the arm hands tonic: incumbent-prod and armonik a frozen `Bytes` split from a reused `BytesMut`, cell A; core-ffi `ak_enc_take_owned`'s buffer wrapped by `Bytes::from_owner`, cell D; core-native `Enc::take`, its buffer moved into a `Bytes`, cell F; optimisation T1) and, for core-native and core-ffi only, `transport-ready-core` (the form handed the core's transport: cell C's encode context, whose buffer `ak_call_unary_enc` moves inside the call, and cell E's reused buffer, which `ak_call_unary` copies inside the call; the host does nothing after the encode, so the op is the reused-buffer op timed as its own row) x input `hot` or `pool` (graphs cloned until the heap they hold, measured with mallinfo2, reaches `AK_POOL_BYTES` = 2 x `AK_LLC_BYTES`, 13.75 MiB by default; built before the case's warm-up and freed after; rows carry `pool_graphs`, `pool_heap_bytes`). A pre-check requires every variant to write the hot row's length |
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
one `ak_enc_reset` before every encode; ONE `ak_dec_reset_<Root>` before a retain decode or
pull (arming; the binding leaves the context armed, optimisation U1) and one before a drop
decode only if a retaining decode left it armed (the counted call runs warm, so none).
Retain runs with no pre-placed buffer and the binding's GEOMETRIC grow (optimisation U2,
max(want, 2 x capacity, 64) capped at INT32_MAX; the owner's override of R-H31's exact-size
grow for this slice). `rpc:<cell>` rows
count ONE call of cells B, C, D and E per direction and mode (P2.2, in-process server on a
Unix socket), the core's RPC counters included. Since optimisation unit 2 also: the framed twins (rpc:Bf, Cf, Df, Ef; each equal to its
reference twin, `ak_client_set_framed` being called once at open), direction c (rpc:<cell>
c/P5.3, c/P5.4) and direction d (d/4MiB, d/16MiB: open, a send per chunk, recv, free,
destroy, plus C's and D's per-chunk encode crossings; the counting build encodes D/F's
chunks on the calling thread because the binding's tallies are thread-local), and rpc:D b
+1 forward (`ak_bytes_free` of the owned body, T1). `gen/crossings.txt` 766 rows,
`gen/crossings-nounk.txt` 389 rows (non-comment); the change from the pre-WP7 files (every existing
row's reverse unchanged; forward +1 per encode, +2 per retain decode, +1 / +3 per pull;
new rows for P2.4's content sets and the RPC cells) is in
`logs/rust/wp7/crossings-change.txt`; the merge with the optimisations changed 232 rows of
`gen/crossings.txt` and 1 of `gen/crossings-nounk.txt` against that branch's files (U1 -1
reset per retain decode/pull, U2 fewer grow upcalls on 36 rows, R2 -1 forward on rpc:C b),
row by row in `logs/rust/opt/merge-counts/`. Not counted: the labelled extra `core-ffi-zc`
arm (the same calls as core-ffi decode); `ak_dec_ctx_new_<Root>` /
`ak_enc_ctx_new` (setup, outside the timed call); cells A and F have no core crossing.

## Campaign readiness (design/CAMPAIGN.md section 10)

Runner: `poc/rust/run_campaign.sh --suite codec|rpc|calib|gate --out DIR`, reading
`AK_CPU_CLIENT` and `AK_CPU_SERVER`. The full build lives in `target/`, the no-unknown
build in `target-nounk/` (a shared target overwrites `libak_core.so`); the runner checks
each binary loads the core of its variant. Latest smoke run: `logs/rust/campaign-wp7/` at
c8e8694eb (every suite; figures stripped).
Every figure in both is instrumentation.

**D14 (owner, 2026-10-03): every pool is AK_WORKERS workers, default 8.** The server
(AK_SERVER_THREADS), the core runtime (AK_CORE_WORKERS) and the tokio client runtimes
(AK_HOST_WORKERS) default to AK_WORKERS, else 8 (grid.rs, rpc_server, run_campaign.sh,
serve.sh); every header states them. Minimal smoke: `logs/rust/d14/`.

**FIX-PLAN WP10 (CAMPAIGN req 13 as amended at 9f6d579fa): the one RPC server of every
slice.** `serve.sh build | start --out DIR | warm N | stop` runs one `rpc_server` process per
launch, pinned to AK_CPU_SERVER, on two Unix sockets (shipped and pinned server
configuration) in `mktemp -d /tmp/aksrv.XXXXXX`; `warm N` makes N checked calls per direction
(a, b, c; d ceil(N/4)) from a tonic and a core client on both sockets (`bin/rpc_warm`).
Interface: `SERVER.md` (service, paths, messages, limits, the planted `FetchShort`,
workers, configurations). This slice's runner starts, warms and stops it through serve.sh.

**WP9 + WP10 verification (small tests, the owner's rule), from a clean worktree at
bed13a6ea (`logs/rust/campaign-wp9/`, figures stripped):**
- gate: `run_campaign.sh --suite gate`, both builds, stable 1.94.1: GATE PASSED; crossing
  counts identical (775 / 398 rows);
- floor: the campaign crate's benches and bins BUILT on rustc 1.88.0 (`floor-1.88-build.log`);
  the full floor gate was not re-run (last passed at 6727646b, `opt/final3-gate/`);
- smoke, settings: AK_SMOKE=1, AK_RPC_TRANSPORTS=shipped, AK_RPC_BUILDS=full, one launch,
  AK_RPC_WARMUP_MS=2, AK_RPC_MEASURE_MS=5, criterion's 10 samples, AK_RPC_SERVER_WARMUP=4
  (serve.sh warm 4), AK_NRESAMPLES=100; codec: AK_ONLY=P1.1, AK_POOL_BYTES=65536,
  AK_WARMUP_MS=2, AK_MEASURE_MS=5, both builds;
- smoke, results: serve.sh started, warmed (4 checked calls per direction, both sockets) and
  stopped the server once for the plants and once for the launch; the 12 warm-up plants
  (cells A, B, Bf, Df x directions a, c, d) and the plant inside criterion each aborted with
  no sample; the RPC launch wrote 3,230 rows: 323 benchmarks (19 cells x a, a+read, b at
  k 1, 8, 16, c at P5.3/P5.4 x k 1, 8, d at 4 MiB/16 MiB x k 1, 8), 10 samples each, with
  every label; codec 480 (full) and 300 (no-unknown) rows.
- Disk reached 91% during the gate (other slices' runs at the same time); 77% after.

**FIX-PLAN WP9 (CAMPAIGN req 22a as amended 2026-09-27): the RPC grid on criterion.** What
the framework forces that differs from the hand-written sampler: one criterion iteration is
one batch of k calls in flight (before: a round of `calls` calls per caller back to back),
so the batch synchronisation is paid per k calls; criterion sizes the iterations per sample
from its warm-up; its warm-up replaces the fixed pre-calls; CPU is its (process-CPU)
Measurement and wall is a column beside it. Custom code kept beside criterion, each for a
requirement criterion does not cover:
- the server process, started by the runner, and its warm-up from each client transport
  (req 13);
- the ProcessCpu Measurement (req 21) and the wall column read in the RPC routine (req 21's
  wall for RPC cells);
- abort-and-discard: a failed check panics the benchmark and the runner deletes the
  launch's files (req 18), with its planted controls;
- the seeded registration order (req 22: criterion has no shuffle);
- the export of criterion's raw samples to section 7's JSON lines with the labels (req 28);
- in the codec suite: the pre-check before timing (req 26) and the pool built before a
  case and freed after (req 11).

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | met by the runner (one measured process at a time); the machine and the sequencing across slices are the owner's |
| 2 | governor, turbo, SMT | met as recording: read from sysfs into every header; setting them is the owner's (this container: n/a) |
| 3 | isolation | met as recording: isolcpus/nohz_full/rcu_nocbs from /proc/cmdline and the cgroup cpuset in every header; the mechanism is the owner's |
| 4 | three disjoint CPU sets, fixed size, thread counts | met: `AK_CPU_CLIENT`/`AK_CPU_SERVER` from the environment (ffi/campaign.sh exports them from ffi/campaign.machine, which fixes the size at 4; run alone, the runner reads that file when they are unset); `taskset -c` on the measured process and the server. Every header records each stack's worker threads (criterion: 1 measuring thread; RPC client: tokio 2 workers per A/D/F cell, `ak_runtime_new(2)` per B/C/E client, k = 1/8/16 callers; server: tokio `AK_SERVER_THREADS` = 4). Disjointness and SMT siblings are checked by campaign.sh, not by this runner |
| 5 | floors gated | met in this container: Rust's floor is MSRV 1.88.0, the same configuration as the target; the gate on 1.88.0: verified: the full gate, both builds, passes on rustc 1.88.0 from the clean worktree (`gate-floor-1.88.log`). `run_campaign.sh --suite gate` runs the stable toolchain only; the floor run is `RUSTUP_TOOLCHAIN=1.88.0 bash gen/gate.sh` |
| 6 | build flags printed | met: release, opt-level 3, lto off, codegen-units default, core as a cdylib through the dynamic linker, core features (rpc, init-guard, and unknown-fields or not), harness guard on, transcoder |
| 7 | payloads | met: the 16 payloads, the latin1 and wide content sets on P1.2 and P2.2, and the 92 non-disputed `U-*` corpus rows whose root is one of this slice's 7 ABI roots (112 inputs). P7.1 decode only (SHAPES.md) |
| 8 | arms (as updated 28a8dba3, D24) | met: incumbent-prod (tonic's codec calls); incumbent-best is the same entry point (R14), so no second row; core-ffi decoding with the FSM family (ak_fsm_begin/next, binding fsm_with_<root>; D24), in drop, retain and the no-unknown build, decode and decode-read; core-ffi-push (the push family) and core-ffi-pull (the pull family) as labelled extra decode arms in the same randomised blocks; host-gen = core-native; armonik. The core grid's core-ffi decode-read is the FSM |
| 9 | directions | met: encode, decode, decode-read (a generated visitor reads every field of both object models) |
| 10 | three unknown-field modes | met: full build: core-native, core-ffi, core-ffi-pull in retain (every options position armed) and drop; no-unknown build, a separate binary, whose facade has no `unknown_fields` member (R-H22): the same arms in no-unknown, with incumbent-prod and armonik as in-process controls. Incumbent in prost's default (drop), stated. On 27 `U-wire-*` rows prost refuses a known field at a foreign wire type; those (row, incumbent/armonik) pairs are not timed and are listed in the codec log header. The no-unknown build has its own committed counts and its own gate (gate step 12, corpus.sh section 6) |
| 11 | serialised once per iteration | met: prost recomputes `encoded_len` on every encode, the core encoders reset their context per call, and no Rust object carries a size memo |
| 12 | cells A-D; C, D in three modes | met: full client A, B, C-retain, C-drop, D-retain, D-drop; no-unknown client C-nounk, D-nounk with A and B as in-process controls; same server. C and D (framed and callback twins included) decode the response with the FSM family (D24, `grid::Slot::f_decode_resp`); no push twin of these cells (the runner has no decode-family switch) |
| 13 | separate server, pre-serialised, one per launch, warmed; one channel per cell | met: `rpc_server` on `AK_CPU_SERVER`, started by the runner, P2.2 pre-serialised and checked against the manifest hash; ONE server per (transport, launch) serving both builds' client processes; each client process warms it by `AK_RPC_SERVER_WARMUP` checked calls from each of its transports before its first benchmark; one channel per cell per client process, opened before the first benchmark and shared by all of that cell's benchmarks; direction (b) decoded by prost |
| 14 | directions a (as a and a+read) and b | met: `a` = Fetch and decode, `a+read` = Fetch, decode, read every field (the codec suite's generated visitor), `b` = encode and Push; the optional streamed upload is BUILT as the labelled extra direction `d` (U2-stream: 2 MiB chunks, 4 MiB and 16 MiB, every cell), with the unary upload `c` beside it |
| 15 | 1/8/16 in flight | met: one criterion iteration = one batch of k calls in flight (Throughput::Elements(k)); B, C, E: k host threads created per benchmark before criterion runs it and reused by every iteration; A, D, F: k tokio tasks per iteration |
| 16 | B/C blocking; A/D/F idiomatic | met: B, C and E use the core's blocking `ak_call_unary`; A, D and F use tonic's async unary call from k tokio tasks on a 2-worker runtime, the shape of packages/rust's client (stated in the header). Callback and queue deliveries are not in the campaign runner (stage 6 harness `rpcgrid` only), stated |
| 17 | shipped and pinned; Unix socket | met: every cell over a Unix domain socket (R-H28; the core dials `unix:`, tonic serves a `UnixListenerStream`); shipped = tonic endpoint and server defaults, `ak_client_new`; pinned = 4 MiB stream and connection windows, adaptive off, on tonic, the core client and the server (Nagle does not apply to a Unix socket) |
| 18 | every call checked, abort | met: status and response length on every call (the server warm-up included), C-F also their decode. A failed check PANICS inside the criterion benchmark (criterion has no stop-on-error), which aborts the process before any output is written; the runner then discards the launch's files and stops. Controls, each required to abort with no sample: the planted wrong length in the warm-up once per send path (A, B, Bf, Df) and direction (a, c, d), and once inside a criterion benchmark (AK_RPC_PLANT=bench), per client binary and transport |
| 19 | crossing counts gate | met: see "Crossing counts: what they cover" (resets and every exported call included; RPC cells B-E per call; retain with no pre-placed buffer and geometric grow, the owner's override); both files compared in the gate and by the runner before codec and calib; a difference stops the run |
| 20 | crossing cost fwd/rev, perf stat | **not met**: `calib` reports forward (`ak_noop`) and forward+reverse (`ak_noop_reverse`) in the same round; `perf stat` cycles and instructions are collected by the runner when perf exists, and **perf is not installed in this container** (the smoke's `calib-perf-launch1.txt` says so). Needs the campaign machine with perf |
| 21 | process CPU per round | met: codec, RPC and calib `CLOCK_PROCESS_CPUTIME_ID` per criterion sample / round (the custom criterion Measurement `ProcessCpu`, shared by both suites). RPC also records wall time per sample (req 21: wall beside CPU for RPC cells): criterion keeps one quantity, so the RPC routine (`iter_custom`) reads the monotonic clock around the same iterations and the export matches it to criterion's samples by (iterations, cpu) |
| 22 | order randomised as far as the engine allows | met: criterion runs benchmarks in registration order and has no shuffle; the codec suite registers arm blocks and the cases inside each block, and the RPC suite registers every (cell, dir, payload, k), in a seeded random order per launch (seed = launch, in the header); the two builds alternate by launch; calib's two arms alternate |
| 22a | benchmark engine (amended 2026-09-27, FIX-PLAN WP9) | met: BOTH the codec suite and the RPC grid run on criterion 0.5 (`benches/codec_suite.rs`, `benches/rpc_suite.rs`), SamplingMode::Flat, raw samples exported from criterion's `sample.json`; the hand-written RPC sampler (`bin/rpc_client`) is removed. calib stays on the runner (two arms, a fixed iteration count for perf stat) |
| 23 | 5 rounds x 3 launches, every round committed | met by default: codec and RPC 10 criterion samples per benchmark (criterion's floor) x 3 launches; calib 5 rounds x 3 launches. The smoke is 1 launch (criterion's 10 samples, short times) |
| 24 | warm-up stated, a runner parameter; RPC >= 20 calls per calling thread (amended 8c02e7c58) | met by a time rule, not checked on the machine yet: criterion's own warm-up everywhere, no hand-written warm-up loop beside it (the codec suite's fixed pre-iterations removed in WP9): codec AK_WARMUP_MS 500 / smoke 5 ms, RPC, per benchmark through BenchmarkGroup::warm_up_time (owner 2026-10-03): AK_RPC_WARMUP_MS 1500 / 5 ms for directions a, a+read, b and AK_RPC_WARMUP_LONG_MS 5000 / 5 ms for c, d. The short value is 1500, not the old 500: at k = 16, a, a+read and b took up to 60 ms median and 96 ms worst wall per iteration, so 15 iterations take 1.44 s, and 30 of 87 k = 16 benchmarks were over 500/15 ms (logs/rust/req24-warmup/ab/). Smoke: the c benchmark warmed with the long value and the a benchmark with the short (logs/rust/req24-warmup/smoke/). The 500 ms default gave the slowest cell (d/16MiB, k = 8) about 3 calls per thread. Criterion's warm-up doubles 1, 2, 4, 8, 16 iterations of the same routine on the same callers until its wall time exceeds the setting, so 20 calls need 15 iterations under it. The container measured 125-227 ms median wall per iteration, worst 461 ms (logs/rust/req24-warmup/). Async cells: the scheduler, not the harness, spreads tasks over workers. The rpc header carries the statement; per-sample minflt is the check; the server warm-up AK_RPC_SERVER_WARMUP 64 / 16 checked calls from each client transport before the first benchmark (requirement 13); calib iters/10. Knobs listed in the runner header, the values used in each log header |
| 25 | allocator (as amended in ad1a15be5, D9), GC stated | met (smoke only, 2026-10-03): `AK_CAMPAIGN_ALLOC=default|pinned` in run_campaign.sh, default `default` (GLIBC_TUNABLES unset everywhere: the main figures); `pinned` sets GLIBC_TUNABLES=trim 256 MiB / mmap 32 MiB on the measured clients only (codec bench, rpc criterion clients, calib and its perf runs), files labelled `-alloc-pinned`; serve.sh starts the shared server under `env -u GLIBC_TUNABLES` in both modes (checked: /proc/PID/environ of a server started with the tunables ambient has none). Each measured process checks at start (campaign::alloc_check: mode vs GLIBC_TUNABLES, then one 16 MiB malloc and mallinfo2 hblks: default must read mmapped, pinned heap) and exits 4 with no sample otherwise; every row carries `alloc` and `minflt` (ru_minflt over the measured span, read outside the timer); headers state both modes and the one that ran. The 16 MiB probe is made once per process and never freed (owner, after java 2892e207b: a freed mmapped probe raises glibc's dynamic mmap threshold). No heap pre-grow (owner, 2026-10-03: added in b6e604f25, reverted). A pre-grow on one thread cannot reach the other threads' malloc arenas (C++ found its first benchmark still faulting through the core's worker threads). Criterion's own warm-up runs the real call path on every thread, and the per-sample minflt shows whether it was enough. No GC (Rust). logs/rust/d9/, logs/rust/d9-revert/ |
| 26 | correctness before timing | met: every timing suite requires a gate PASS for the same tree-id (runs the gate otherwise); the codec process re-checks every timed arm on every input and every encode variant before criterion starts, the FSM differential included since D24 (core grid: full build 720 checks, no-unknown 409; full grid, full build 6,196) |
| 27 | header | met: commit and tree-id (a dirty tree is refused unless `AK_ALLOW_DIRTY=1`, which is recorded), machine, SMT, governor, turbo, kernel, isolation, CPU sets, rustc/cargo, incumbent versions, build flags, core variant and features, transport, warm-up, repeats |
| 28 | JSON lines, raw | met: one line per criterion sample (codec, RPC) and calib round, with the req-28 field names; RPC rows carry cell, dir, payload, inflight, transport, build, unknown_mode, send_path (reference or framed), launch, round, cpu_ns, wall_ns, iters (= calls = criterion iterations x k) and batches (criterion iterations) |
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
| D43 | opt harness | the incumbent's own median moves 5-10% between two processes of different builds with no change to its code (optimisation experiment; e.g. prost P1.2/wide decode 856 -> 1045 us between the two harness-v5 runs); a 32-byte layout shift explains at most 3.4% of a group mean (`opt/layout-exp`). Cross-run absolutes carry it; prost's column is the control | open, cause not identified |
| O1 | opt harness | the full client's Bf cell ran slower than its reference in every full run (shipped: 1.6-3x on c and d, opt/framed, u1-unary, u2-stream; direction a 1.13-1.25 in opt/framed and final2) while the no-unknown client's Bf did not, and the narrowed runs did not reproduce it (Bf/B 0.80-0.98) | observed in the container, not investigated (owner), deferred to the campaign machine |
| W8 | other slices' hand-written RPC hosts | unit 3 changed `ak_call_unary`'s arity (a trailing `int32_t *grpc_status`) and `ak_completion` (a new `grpc_status` field; same size, same `bytes` offset); their generated headers/bindings are regenerated, their hand-written call sites are not (this slice does not write them). Sites (b3ac5050's tree): cpp `src/campaign_rpc.cpp` 323, 481, 485, 614 and `src/rpcflow.cpp` 96 (ak_call_unary, 6 args); cpp `src/rpcbench.cpp` 104 (ak_call_unary), 125-233 (completions: brace or memberwise init to check), `src/rpccounts.cpp` 29-106 (ak_call_unary at 54, completions); csharp `src/Rpc/CoreTransport.cs` 167 (ak_call_unary), 120 and 190 (ak_completion), `src/Rpc/Campaign.cs` 358, 417 (ak_call_unary); java `native/rpc.c` 96 (ak_call_unary), 178-187 (a local ak_completion); python `native/binding.c` 438 (ak_call_unary), 370, 502, 534 (ak_completion). None uses ak_call_open/recv/close | for the aggregating session's WP8 list |
| D42 | rust facade | a map entry has no unknown-field bag in the facade, so `U-map-entry` is written in the dropped form by ffi-retain and native-retain although the core delivers the entry's bytes (accepted by the contract) | open, a facade question |

Closed in unit 3: D44 (limits enforced as specified), D45 (section 9's text defines the kind, the options, one cancel entry and the status number; built as written).
Closed earlier (evidence in `JOURNAL.md`): D2 (the 1.88 floor now runs,
the gate passes on it); D34 (retention inside inlined children, closed by decision 11 for the C
ABI); D35 (recursive messages: refused from the C ABI by owner decision, ABI-v1; `Nest` runs
on core-native only, a scope limit listed below); D38, D39, D40 (fixed by their slices,
FIX-PLAN R-G17); D41 (every slice's generated tree is current: `generate.py --check`).

## D18: the campaign grid (CAMPAIGN 4.0, ef26e76d0, and its transport amendment b58543f7b)

The grid switch is `AK_CAMPAIGN_GRID=core|full` (default `core`) in run_campaign.sh, exported to the suites. `full` runs today's grid with every row labelled `row = core | extra`. Every extra stays buildable and runnable under `full`. The headers state the grid that ran and every extra left out.

- **Codec, core grid.** The selection is `campaign::core_codec_input` and `core_codec_case`, applied BEFORE the pre-check, so the pre-check covers every arm on every timed input.
  - **Inputs (25):** the 16 shapes in ASCII (P7.1 decode only), P2.2 Latin-1 and wide, and the 7 named U-* rows.
  - **Arms:** incumbent-prod (full build only), core-ffi (push), and host-gen.
  - **host-gen in Rust is `core-native`:** the codec the shared generator writes into Rust, with no C ABI boundary.
  - **Encode at end state (ii) on the hot input:**
    - incumbent-prod: transport-ready-tonic, cell A's form;
    - core-ffi and core-native: transport-ready-core, the form cells Cf and Ef hand the core transport.
  - **Decode:** decode-read only.
  - **Modes:** retain in the full build, no-unknown in the no-unknown build.
  - **Smoke counts:**
    - full build: 139 benchmarks (pre-check 620 checks); no-unknown build: 98 (359 checks).
    - incumbent-prod times 21 decode-read and 20 encode rows. prost refuses the four `U-wire-*` rows, which are stated, not timed.
- **RPC, core grid.** The selection is `core_rpc_spec`.
  - **Cells:** A, Bf-cb, Cf-cb-retain and Ef-cb-retain, the framed core cells with Rust's idiomatic delivery (req 16 as amended: the callback bridged to async).
  - **Grid:** a+read and b at P2.2, c at P5.4, d at 16 MiB, each at k = 1 and 8; full build only. That is 32 benchmarks per launch.
  - **h2-batch:** Cf-cb-retain on c and d at k = 1 and 8 (4 benchmarks), labelled `h2 = h2-batch`, main pass only.
    - Same rpc_suite binary, loading the h2-batch core through LD_LIBRARY_PATH.
    - The core is built by `gen/h2batch_core.sh` from this workspace's Cargo.lock, with poc/codec/h2-batch/h2-batch.patch and the campaign crate's own feature resolution.
    - rpc_suite refuses (`h2_check`, exit 5) when AK_H2 disagrees with the h2 compiled into the core it mapped.
  - **Pinned allocator pass:** A and Cf-cb-retain on c and d at k = 1. The codec suite and calib are skipped in that pass.
  - **Bench plant:** names Cf-cb and must print ABORT.
- **Transport, core grid: ONE configuration, `armonik`, TCP 127.0.0.1** (the server's TCP listener). `shipped` and `pinned` on the Unix sockets stay under `full`.
  - **Cell A:** calls `packages/rust/armonik-transport` `connect` directly, with ClientConfigArgs::default() plus the endpoint. That gives a hyper-util connector, nodelay true, connect timeout 60 s, no keepalives, hyper-rustls https_or_http (plain HTTP here), and tonic/hyper's default windows. Path dependency, read only; the effective ClientConfig is printed in the header.
  - **Core cells:** the core's current client (ak_client_new with no options). It differs from armonik-transport in three ways, all stated in the header:
    - no connect timeout;
    - tonic's own connector instead of the hyper-rustls wrapper;
    - no HTTP/1 on the connector.
  - **Nagle off, read back on both ends:**
    - client: `campaign::nodelay_readback`, getsockopt TCP_NODELAY on every live TCP socket to the server, before the first benchmark and after the last; refuses with exit 7.
    - server: rpc_server reads it back on every accepted socket and logs it; exits 7 if it is off.
    - plant (AK_RPC_PLANT=nagle, Nagle switched on on one live socket) refused with no output.
- **Smoke** (logs/rust/d18/, AK_SMOKE=1, figures stripped):
  - main RPC pass: 32 + 4 benchmarks, row = core, Nagle read back on 4 sockets, 17 accepted server sockets all true;
  - pinned pass: 4 benchmarks, plus the Nagle plant;
  - codec: 139 + 98 benchmarks; the pinned pass skips the codec suite.
  - Plants: every req 18 plant aborted, and the Nagle plant refused.
  - It ran through a copy of the runner with the gate and crossing checks stubbed out (no gate for this unit).

**Counts and send path for the core grid (2026-10-03, owner):**

- Every timed core-grid row has gated crossing counts.
- The counting build now also writes `decode-read` rows (every input, every mode; the decode plus every field read) and `rpc:<cell> a+read` rows (cells B-E).
  - gen/crossings.txt grows from 836 to 1,092 lines; gen/crossings-nounk.txt from 435 to 567.
  - Every existing row is unchanged, and every new row equals its decode / a row (the reads cross nothing).
- Rows with no new count of their own:
  - The core-ffi encode at end state (ii) `transport-ready-core` is the `encode` row's op: the move happens inside ak_call_unary_enc, counted in rpc:C.
  - core-native and incumbent-prod call no ak_* entry point.
  - h2 is below every counted entry point, so h2-batch rows need no count of their own.
- Send path:
  - Every timed core cell sets its path explicitly at `Conn::open`: `ak_client_set_framed(client, framed(cell))`, so B, C, E and their -cb variants run reference and Bf, Cf, Ef and their -cb variants run framed, under core and full alike.
  - That code (cd47d9cf5) landed in the same push as the core's framed default (e8fe14868, both 2026-09-28 19:47:35), so no reference row ever ran framed.
- **Gate:** run from a clean worktree, both builds and the h2-batch core.
  - **PASSED at d76ce8d91** (tree-id fecb7495ac67d788), 979 s with an incremental build in the same worktree; crossing counts 1,092 / 567 rows identical (logs/rust/d18-gate/run2-d76ce8d91/).
  - The first run at 049cfba75 (4,627 s from a fresh build) passed every step through 12, then failed 12b on a harness defect: `ldd | grep -q` under pipefail. Fixed in d76ce8d91 (logs/rust/d18-gate/run1-049cfba75/).
- Gate step 12b (new) runs:
  - upload_check over TCP loopback on the stock core;
  - gen/h2batch_core.sh;
  - upload_check (UDS and TCP) and rpc_semantics on the h2-batch core, loaded through LD_LIBRARY_PATH and verified with ldd.

## Campaign duration estimate, core grid (2026-10-03, computed; container-sized per-benchmark times)

Defaults: 3 launches; codec warm-up 500 ms + measurement 2 s; RPC warm-up 1500 ms (a+read, b) or 5000 ms (c, d) + 2 s; 100,000 resamples.
- Criterion's doubling makes the warm-up last 1x to 2x its setting (1.5x used).
- Analysis takes ~45 ms per benchmark.
- Per benchmark: codec ~2.8 s (2.55-3.05); RPC short ~4.3 s (3.6-5.1); RPC long ~9.65 s (7.2-12.2).

| part | benchmarks | per launch | x3 launches |
|---|---|---|---|
| codec, full build | 139 | ~6.8 min (6.2-7.4) incl. ~20 s pre-check | ~20 min |
| codec, no-unknown build | 98 | ~4.9 min (4.4-5.3) | ~15 min |
| rpc main, armonik transport, full build | 16 short + 16 long | ~4.0 min (3.1-4.9) | ~12 min |
| rpc h2-batch (Cf-cb, c and d) | 4 long | ~0.7 min (0.5-0.9) | ~2 min |
| rpc plants (once), server start and warm, process startups | | ~0.3 min | ~1.5 min |
| calib and crossing checks | | | ~2 min |
| **main pass** | | | **~52 min (45-58)** |
| **pinned allocator pass** (rpc A and Cf-cb on c, d at k = 1) | 4 long | ~0.8 min | **~3 min (2.5-4)** |
| **campaign total** | | | **~55 min (48-62)** |

Not included: the gate and the builds, once per tree, ~0.5-1 h. The h2-batch core build is part of that: 6.6 min in the container.

## What is not measured

- **reset-on-entry's gaps**: the C# rendering is not compiled or run here (the C# agent's); the full default gate was not re-run (the default .so is byte-identical, the default bindings and counts identical, roe-checks steps 1-2, 7, 8); no A/A control column in the timing (the explicit path carries the switch's branch, the default build's own path is not timed beside it); no attribution of the rows where the two paths differ; the variant's gate steps 7-13 (concurrency, lifecycle, R-D1, R-D6, TSan, h2-batch) not run on it; the floor (1.88) not built with the feature; the campaign machine.
- **D24's gaps**: no timing at all in D24 (the codec suite's new core-ffi and core-ffi-push rows, cells C and D with the FSM: never run); no push twin of the RPC cells; ThreadSanitizer not re-run on the FSM path (concur's decode checks now use it); the legacy harness timing bins (bench, rpcgrid, content, unknown, inlining) now reach the FSM through `core_ffi_arm::decode` under their old labels and were not run; the campaign machine.
- **D20's gaps**: listed in section D20 above (bits set through a host, other slices' full gates, pull timed, content sets beyond P2.2, the campaign machine).
- **D19's gaps**: listed in section D19 above (other slices' gates on the new core, the campaign machine, TSan, encode grow counts, strings over INT32_MAX/3 units).
- **The backward-encode experiment's gaps**: listed in `logs/rust/opt/patches/backward-encode/README.md` (pool inputs, decode, the 2x2 on order-dependent payloads, the P6.1 attribution, the floor and the campaign machine on the patched tree).
- **Timings.** None are results in this phase; every timing log is container
  instrumentation, and no figure is quoted in this file.
- **perf counters** (req 20): perf is not installed here.
- **Codec wall time**: the codec suite records CPU time only.
- **Encode variants on the core's own transport**: the `transport-ready-core` row of core-native
  and core-ffi times the encode only; the move (C, `ak_call_unary_enc`) or copy (E,
  `ak_call_unary`) into the request happens inside the RPC call and is timed only in the grid.
- **Pool sizing**: the pool's heap is measured with glibc `mallinfo2` at build time; memory
  held by the allocator but not in use is not counted.
- **RPC callback and queue deliveries** in the campaign runner (stage 6 harness only).
- **The streamed call beyond client streaming**: `ak_call_open` builds one kind
  (AK_CALL_CLIENT_STREAM); server and bidirectional streaming are reserved (NULL).
- **Section 9's semantics under timing**: metadata, deadlines, cancel and limits are checked
  by `bin/rpc_semantics` (gate 11f) and never timed; the grid passes NULL options and 0/0
  limits. No retry, TLS, concurrency suite over RPC; compression off on every path (the
  framed path implements none).
- **Limits over 4 GiB**: the framed path's prefix is 32 bits; not exercised.
- **N5's fallback under timing**: no payload reaches it (two synthetic inputs checked it
  before the revert).
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
- **The runtime probe's variants at k = 8, and feed depth 3 or 4**: not run (no variant moved
  the k = 1 result beyond the spread; the benchmark budget was spent). The worker-count knobs
  are not used by any campaign run (defaults 2 / 2).
- **Other slices' runtimes and bindings**: measured by their slices.
- **The patch experiments in the grid** (rpc_suite): the new cells are probe-only; the grid's cell
  set is the campaign's and unchanged. Zero-copy and deferred entries are stream-only (no direction
  c); the send limit is not checked on those experimental paths.
- **p4 edge cases**: PING or SETTINGS change mid-burst, GOAWAY during a burst.
- **h2-batch beyond the gate**: the 1.88 floor gate and the campaign machine's gate on the h2-batch core;
  the gate's step 2 (no h2 in that build); a check that counts the DATA frames a server receives after a
  local cancel (D16's divergence is not observed by any check).
- **PR 903 and PR+p4**: the syscall census under timing (strace runs are untimed, whole-process); PR host-too on the combined h2; PR 903's cost with its IoSlice array sized to the work (not built); the edge cases listed for p4; h2's hammer test in debug (30 s timeout on every variant, stock included).
- **WP11 item 5's added cells** (Df-chan on two runtimes, Ff in the probe, Df with one frame per
  message) and `perf record` / `perf stat`: not built into the physical probe (perf 7.2.8 exists on
  the campaign machine, perf_event_paranoid 1); the shipped transport is not in it (pinned only).

## Notes

- The core's own default features leave `init-guard` off; this slice's harness, campaign
  and corpus harness turn it on (CAMPAIGN.md req 6), and the corpus gate has a control that
  fails when a binding skips `ak_init`.
- Reads `packages/rust`; edits nothing under `packages/`.
- On a machine whose cargo config sets `build.build-dir`, put `gen/cargo-shim` first on PATH
  (the gate, the runner) so each target directory keeps its own core.
- Each no-unknown build needs its own `CARGO_TARGET_DIR` (target-nounk,
  target-count-nounk, target-corpus-nounk).

## Log index

| Log | What it establishes |
|---|---|
| `logs/rust/opt/d27-reset-on-entry/` | D27: `gate-stopped-at-11g.log` (steps 0-11f and 11g F1-F2 passed, then stopped), `d27-checks.log` (gen/d27_checks.sh: D27 CHECKS PASSED, plants caught), `fast-checks.log` (corpus FSM differential, the no-unknown checks), `generators-check.txt` |
| `logs/rust/opt/reset-on-entry/` | reset-on-entry: `checks/roe-checks.log` (gen/roe_checks.sh: ROE CHECKS PASSED) and `checks/roe_check-*.txt` (the four builds' lines), `so-base.txt` (the base .so hashes), `crossings/` (change.txt, roe_diff.py), `csharp-render/` (the C# variant's diff), `bench/` (header, three launches, tables.md) |
| `logs/rust/opt/d24-fsm-target/` | D24: `gate-stable.log` (the full gate, GATE PASSED, step 11g included), `gate-floor-1.88.log` (the floor), `crossings/` (change.txt, crdiff.py, crinv.py, the reference files before) |
| `logs/rust/opt/d23-fsm-c-isolate/` | fix C isolated: builds.txt, bench/ and tables.md (six variants), perf/ (C0 vs C0f), the probe patch, checks-final.log on the final FSM (A+B) |
| `logs/rust/opt/d23-fsm-fixes/` | D23 fixes A, B, C: checks.log (D23 CHECKS PASSED), bench/ and tables.md (four builds alternated, drop and retain, slopes), probe-c-inline/ (variants of C) |
| `logs/rust/opt/d23-fsm-attrib/` | D23 attribution: bench-1.txt (drop and retain), bench-2-drop.txt (with probes and fsm-collect), derived-bench-2.txt (slopes, per-row deltas), wirestats.txt, perf/ (symbol tops and annotations), commands.txt |
| `logs/rust/opt/d23-fsm/` | D23: `checks/rust-checks.log` (gen/d23_checks.sh: generators, separation vs dfa9eb04, unit tests, the FSM differential full / no-unknown / counting, the corpus differential both builds, six plants caught, C header and C host, pre-checks with and without the FSM arm, gate steps 3-5, 9-12), `checks/base/` (the base commit's counts, pullbench, crossings), `checks/events-counting.txt` (events and crossings per decode), `checks/floor-1.88.log`; `bench/` (header, three codec_suite processes, raw JSON lines) and `bench/tables.md` |
| `logs/rust/opt/d20-utf8-bits/` | D20: `checks/rust-checks.log` (gen/d20_checks.sh: generators, unit and D20 core tests with plants, conformance, shapes, counts, R-D1, R-D6, corpus, pre-checks, crossing counts, both builds), `checks/cpp.log`, `checks/csharp.log` (build_core.sh and the net8.0 quick gate), `checks/java-python.log`; `bench/` (header with the toggle proofs, 18 processes, d20_bench.out) and `tables.md` |
| `logs/rust/opt/d19-simdutf/` | D19: `checks/checks.log` (unit tests, conformance, shapes, the differential at scale 8, corpus both builds, pre-checks, crossing counts), `bench/` (header, three tc16_bench processes, tables.md), `build-impact/` (matrix.log: feature sets, h2-batch, C hosts, staticlib, floor; cpp.log; csharp.log) |
| `logs/rust/opt/patches/backward-encode/` | the backward-encode experiment: the patches (sources, generated, dropped v2), README, the gate and bwd_check on the backward core (checks/gate), the mismatched pair (checks/mismatch), the alternated encode session and RPC probe (bench/, tables.md), P6.1 v1/v2 and harness x core (bench-p6-v2/, bench-p6-2x2/) |
| `logs/rust/opt/wp12-gates/` | WP12: the full gate on the h2-batch core, on the stock core (both built per feature set by build.sh or the same cargo command) and as committed (plain), each with loads.txt (which core every process loaded), the TCP checks and the write-count marker; NOTE.txt |
| `logs/rust/opt/tcp-sweep/` | the TCP core worker sweep after the re-pin: low/ (k 1, 8) and high/ (k 16, 32), tables.md; `tcp-sweep-unpinned-irqs/` the same run before the re-pin (machine condition in its NOTES.txt) |
| `logs/rust/opt/delivery/` | the response-delivery comparison: build.raw, checks/, a/ and cf/ sessions, tables.md |
| `logs/rust/opt/p1-landed/`, `logs/rust/opt/h2-batch-variant/` | the gate on the landed p1; both h2 variants' builds (sha256), checks and the TCP-default smoke |
| `logs/rust/opt/patches/h2-pr903/` | h2 PR 903: HOWTO (sha, version, port, trees, patch sha256), the port and original patches, h2-tests/ (port, stock, original, before/after the port fix), build/ (core and host-too sha256, h2 compiled in), checks/, counts-uds/ (write counts incl. the original head), timed/ (session 1: stock, p4, PR core-only, PR host-too; h2pr903.md), record/, record2/ (perf report and annotate, d/16 k=1 UDS, Cf) |
| `logs/rust/opt/patches/h2-pr903-p4/` | PR 903 + p4 combined: HOWTO, patches, h2-tests/ (N=1, N=16, p4 alone N=16), build/, checks-N1/, checks-N16/, partial-writes/, counts-uds/, timed/ (session 2: stock, p4, PR, PR+p4; h2pr903.md), headline-both-sessions.md |
| `logs/rust/opt/physical-probe/stability/` | the stability campaign: per-process medians in run order, gap distributions, A2 - A floor, pooled absolutes (stability.md) |
| `logs/rust/opt/cb-track/` | the Cf-cb attribution: one-cell and in-process profiles, per core worker count 1, 2, 8; `workers-sweep/` the core worker count 1/2/4/8 x p9 at k 1 to 32 with one and four core clients (gen/workers_sweep.sh, gen/sweep_tables.py) |
| `logs/rust/opt/physical-probe/opt-stack/` | the consolidated run: HEAD core against the stack p1+p2+p3+p5+p6+p7 and the same with p4, cells A, Df, Df-1f, Cn-1rt, C, Cf, Cf-cb, Cf-encp, Cf-zc, Cf-zcp, Cf-zcw, every workload at k 1 and 8, pinned allocator (3 processes) and default allocator (1) |
| `logs/rust/opt/patches/` | one directory per patch experiment: the patch, STACK.txt, checks/, in-process timings, attribution |
| `logs/rust/opt/attrib/`, `logs/rust/opt/enc-track/` | the attribution of A / Df / Cf / Ff (one-cell and in-process, allocator modes, pinning, one-frame) and of the enc / zero-copy cells |
| `logs/rust/opt/physical-probe/variant-w4/`, `variant-c8/`, `server-s4/` | segment 2: 4 workers everywhere, and the 8-worker client against the 4-worker server |
| `logs/rust/opt/physical-probe/main-w8/`, `server-s8/` | segment 1 of the physical-machine probe (8 workers everywhere): spread and main passes, attribution pass, tables.md |
| `logs/rust/opt/physical-probe/prep/` | the campaign machine's preparation: gate at cb37633f (rustc 1.95.0) PASSED, upload check, the driver's smoke (headers and row counts only) and its worker-count abort control |
| `logs/rust/opt/t0-ref/`, `t1-native*/`, `t1-ffi/`, `framed/`, `framed-rpc-narrow/`, `n2/`, `n2-ab/`, `n3/`, `n3-ab/`, `n6-probe/`, `lto-ab/`, `u1-unary/`, `u1-unary-narrow/`, `n5/`, `n5-ab/`, `n5b-ab/`, `u2-stream/` | optimisation unit 2, one directory per step (full opt_bench v5 runs, narrowed alternated A/B runs, step checks in `checks/`); before/after in `variants-before-after.txt` / `by-direction.txt`; the framed path's wire evidence in `framed/header-diff*.txt`; direction c and d tables in `u1-unary/c-direction.txt`, `u2-stream/d-direction.txt` |
| `logs/rust/opt/pre-n5-gate/`, `pre-n5-gate2/` | the stable gate checkpoints of unit 2 (PASSED at 33636e1d and 186a4e52) |
| `logs/rust/opt/abi9/checks/` | unit 3's step checks (generate --check, one_core, pre-check, crossings identical) |
| `logs/rust/opt/runtime-probe/` | runtime probe: host / core worker counts, current-thread host, feed depth 2 (patched core, reverted; checks/), direction d k = 1, two sessions (main/, confirm/), tables.md (absolute CPU per call, in-process gap to A, context switches, faults, split cells' per-chunk host work, attribution pass) |
| `logs/rust/opt/framed-default/` | the framed default, the spare ring and the one-frame prefix: checks, ring-size check, in-session grid (b, c, d, k 1 and 8) + probe with split cells for both deliveries (tables.md), allocation/fault/write counts, body poll/wake counts |
| `logs/rust/opt/stream-probe2/` | stream probe 2: the probe's level beside the grid (level/), HTTP/2 settings (settings/), Df-chan (chan/), per-chunk host timing (split/), body poll/wake counts (counts/), ablations ring3/ and head/ (patches, reverted) |
| `logs/rust/opt/stream-probe/` | the stream probe (direction d, k = 1, pinned): per-thread CPU split, allocations, faults, copies per cell; A/A calibration; ablations (a) channel 4, (c) spare ring 3, (d) current-thread cb runtime, each with its patch, all reverted |
| `logs/rust/opt/cb-deliveries/checks/`, `logs/rust/opt/rpc-same-machine-cb/` | the callback-delivery unit's checks and its same-machine grid run (cb cells = Rust's reference core cells) |
| `logs/rust/opt/rpc-same-machine/` | the RPC grid only at dcbb0205 through serve.sh (gen/rpc_same_machine.sh), k = 1 and 8, for a same-machine side-by-side with the C++ grid; 311 of 840 entries one batch per sample (marked); instrumentation |
| `logs/rust/opt/final3-gate/`, `logs/rust/opt/final3/` | unit 3's final gate (stable and the 1.88 floor; the failed first attempt in `failed-98b187ce/`) and final run with the one-run tables `tables-codec.md`, `tables-rpc.md` |
| `logs/rust/opt/final2-gate/`, `logs/rust/opt/final2/` | unit 2's final gate (stable and the 1.88 floor) and final run, with tables against `t0-ref` |
| `logs/rust/opt/merged/`, `logs/rust/opt/merged-before/` | opt_bench v5 (the merged campaign harness) on the merged HEAD and on their branch without our optimisations: per-case `summary-codec.tsv`, absolute `variants-codec.tsv/.txt`, `unknown-retain-vs-drop.tsv`, RPC and calib summaries; before/after tables in `merged/` |
| `logs/rust/opt/merge-counts/` | the merged counting build's crossing files against that branch's committed ones, with the reason per row class |
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
