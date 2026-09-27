# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what is true now;
the history is in `JOURNAL.md`. Phase (README section 1.1): setup and design. Every timing
this slice has taken is **container instrumentation**, not a result, and none is quoted
here. This file states what exists and what was checked; the choice is the owner's.

| | |
|---|---|
| **Status** | Built and gated on origin/rust/native-core-ffi-poc: four codec arms plus the pull family, the RPC grid (cells A-F), the full conformance corpus through the C ABI and core-native, decision 11's unknown-field mechanism, the no-unknown build, and the campaign harness conformed to the 2026-09-26 contract (FIX-PLAN WP7). That branch is now MERGED with the optimisation experiment (claude/rust-slice-optimization-sy1f4n): every kept optimisation (D1, E1, E2, E4, N1, U1, U2 geometric growth with the INT32_MAX clamp, E3, D2, D3b, D4, R1, R2 `ak_call_unary_enc`, F1 = R-H22, E5, Z1 `core-ffi-zc`, C2 simdutf8) runs on the WP7 harness. **The merged HEAD is NOT gated** (the owner did not ask for a gate): checked by builds of both variants and the corpus workspace, generate --check, one_core.sh, the pre-check on both builds and the counting builds; the last gates are WP7's (c8e8694eb, their branch) and the experiment's (3c737d1, ours), each on one side only |
| **Next step** | HELD after step 2 on the RawEncoder copy finding (the owner decides between the options reported); then N2, N3, N6, a stable gate checkpoint, N5, the final gate |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | MSRV 1.88.0: the full gate, both builds, passes on rustc 1.88.0 from a clean worktree at c8e8694eb (`logs/rust/campaign-wp7/gate-floor-1.88.log`) |
| **Target** | stable 1.94.1 in this container; README section 5: for Rust the floor is the target language level, one configuration |
| **Incumbent** | prost 0.14.4, tonic 0.14.6, tonic-prost 0.14.6 (from Cargo.lock, printed in every campaign header). R14: tonic-prost's codec calls `Message::encode`/`decode`, so the production path and the library entry point are the same call |
| **Questions this slice has open for the aggregating session** | (1) the proposed corpus rows of `gen/probe_corpus.py` (field numbers above 2^29-1, the 10th varint byte, two map-order rows) are not in `corpus/`; (2) no corpus row or payload has a repeated singular message with differing content, so merge-on-repeat (R-E4) is rendered and never observed; (3) a map entry has no unknown-field bag in the Rust facade (D42) |

## Optimisation unit 2 (in progress; every figure is container instrumentation)

Owner-approved steps 0-6 (T1 native, T1 ffi, N2, N3, N6, N5), one commit per step, each
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
| 2 | 1c181021 | T1 ffi: additive RPC entry `ak_enc_take_owned(enc, out)` (owned ak_bytes, released with ak_bytes_free, buffer back to the context's spare), used by cell D and core-ffi's transport-ready-tonic row | **held** (coordinator: RawEncoder still copies once into tonic's EncodeBuf on every raw-bytes cell, so D goes 2 -> 1 copies, not 0; awaiting the owner) | pre-check 0 failures (moved forms' bytes checked); crossings rpc:D b +1 forward (ak_bytes_free), regenerated | `opt/t1-ffi` |

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
| 11 | serialised once per iteration; encode variants | met: prost recomputes `encoded_len` on every encode, the core encoders reset per call, no size memo. Every encode arm x mode has labelled rows (R-H29): end_state `reused-buffer`, `transport-ready-tonic` (the form the arm hands tonic: incumbent-prod and armonik a frozen `Bytes` split from a reused `BytesMut`, cell A; core-ffi a `Bytes` copy of `ak_enc_take`'s bytes, cell D; core-native a `Bytes` copy of its buffer, cell F) and, for core-native and core-ffi only, `transport-ready-core` (the form handed the core's transport: cell C's encode context, whose buffer `ak_call_unary_enc` moves inside the call, and cell E's reused buffer, which `ak_call_unary` copies inside the call; the host does nothing after the encode, so the op is the reused-buffer op timed as its own row) x input `hot` or `pool` (graphs cloned until the heap they hold, measured with mallinfo2, reaches `AK_POOL_BYTES` = 2 x `AK_LLC_BYTES`, 13.75 MiB by default; built before the case's warm-up and freed after; rows carry `pool_graphs`, `pool_heap_bytes`). A pre-check requires every variant to write the hot row's length |
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
Unix socket), the core's RPC counters included. `gen/crossings.txt` 696 rows,
`gen/crossings-nounk.txt` 349 rows; the change from the pre-WP7 files (every existing
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
| 19 | crossing counts gate | met: see "Crossing counts: what they cover" (resets and every exported call included; RPC cells B-E per call; retain with no pre-placed buffer and geometric grow, the owner's override); both files compared in the gate and by the runner before codec and calib; a difference stops the run |
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
| D43 | opt harness | the incumbent's own median moves 5-10% between two processes of different builds with no change to its code (optimisation experiment; e.g. prost P1.2/wide decode 856 -> 1045 us between the two harness-v5 runs); a 32-byte layout shift explains at most 3.4% of a group mean (`opt/layout-exp`). Cross-run absolutes carry it; prost's column is the control | open, cause not identified |
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
- **Encode variants on the core's own transport**: the `transport-ready-core` row of core-native
  and core-ffi times the encode only; the move (C, `ak_call_unary_enc`) or copy (E,
  `ak_call_unary`) into the request happens inside the RPC call and is timed only in the grid.
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
