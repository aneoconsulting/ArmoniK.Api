# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists and what
was checked. It does not say what a binding should choose (`CLAUDE.md`, roles). History is in
`JOURNAL.md`; this file says what is true at the commit it was written at.

**Phase** (README 1.1): setup and design. Every timing this slice has produced is container
**instrumentation**. None is quoted here. The logs that carry timings are listed at the foot,
labelled.

| | |
|---|---|
| **Status** | FIX-PLAN WP7 done: the harness conforms to the 2026-09-26 contract (checklist below). Gated from a clean worktree at c7c083f68 (origin HEAD, with the no-unknown host-gen arm and cells E-nounk / F-nounk), both builds at 3.12 and 3.7, and the smoke run there shows every new row (figures stripped). Under the owner's scope rule (ffi/CLAUDE.md "Scope of findings"), only defects that could affect what the campaign measures are fixed from here |
| **Target** (owner D1) | CPython 3.12.3; grpcio 1.84.0, protobuf 7.36.2 (upb) |
| **Floor** (owner D1) | CPython 3.7.5 (Ubuntu 18.04's packages, `fetch_py37.sh`, sha256-pinned); protobuf 4.24.4 (upb) as the incumbent there. The floor runs the correctness gate only |
| **Incumbent** (R14) | protobuf on upb through gRPC's generated marshaller path (`SerializeToString` / `FromString`); derived from `Protos/V1` by `verify_r14.py` (log 52) |
| **Core** | `poc/codec`, the one core (R0), read by path from this checkout. Eight builds, every one with `init-guard`. Full: plain, `count`, `rpc`, `corpus`. No-unknown (`--no-default-features`): the same four, each in its own target directory |

## What exists

```
poc/codec/gen/py_pure.py   (this slice's backend) facade.py, pycodec.py (drop), pycodec_retain.py
                           (error codes read from plan.FIXED.codes);
                           from a plan with unknown fields compiled out, a facade with no
                           `_unknown` (no attribute, __slots__ entry or constructor argument)
poc/codec/gen/py_capi.py   (this slice's backend) binding.c: the CPython shim; C facade types;
                           three accessor backends (attr, cext, pyacc); both directions;
                           decision 11 (per-root options armed per decode, host grow, slots
                           delivered into `_unknown`, the ak_uencode_* family); per-thread
                           decode and encode contexts; ak_init (plan.lifecycle); the section 10
                           layout table; PY_VERSION_HEX conditionals. From the no-unknown plan:
                           none of the unknown-field code, no `_unknown` member, retain refused
poc/codec/gen/c_abi.py     (shared, used as is) ak_abi.h for both variants
gen/generate.py            glue: renders gen/out/{., corpus, nounk, corpus-nounk}; --check runs
                           the one-generator guard with its planted violation
gen/out/                   committed generated files (18)
native/binding.c           the module wrapper (names no message or field): entry points, layout
                           check at import, counters, controls (unk_positions, wrong_root,
                           last_reclaimed, unk_totals, tls_created, nounk); -DAK_NOUNK selects
                           the variant and #errors on a shim/header mismatch
build.sh                   R0, R1, eight cores, per interpreter 10 measured shims + controls, R5,
                           the must-fail controls (noinit, layout plant, no AK_RPC, variant shim
                           over the full core, the process-wide reclaim twin)
gate.sh                    the correctness gate at the target and the floor -> logs 90-98, 100-104
test_camp_summary.py       camp_summary's test: two builds, per-launch medians (gate 104)
rpc_counts.py              req 19: every ABI call per call of the RPC cells B-E (gate 105)
counts/abi-{full,nounk}.txt, counts/rpc-{full,nounk}.txt   req 19's committed counts
counts/crossings-{drop,nounk}.txt   committed whole-number crossing counts per call (req 19)
fetch_py37.sh, floor_check.sh       the floor interpreter; the source check against 3.7.5 headers
conformance.py, corpus.py, rpc_gate.py, rd1_lenwrap.py, u1_map_unknown.py   gate steps
facts.py, payload_values.py, arms.py, arms_plan.py   harness glue (no wire rule)
run_campaign.sh, camp_*.py, counts_expected.txt      the campaign harness (CAMPAIGN.md)
rpc.py, bench.py, allocator.py, gcbias.py, concurrency.py, verify_r14.py, run.sh   older harnesses
mech/                      the crossing-mechanism microbenchmark (no wire rule) and the
                           shapes_pb2 step (`mech/build.sh`) arms.py uses on 3.12
```

## The clean gate

Fresh `git worktree` at **c7c083f68** (origin HEAD: the WP7 harness and the no-unknown host-gen arm, on the register H core 31fc3eecf; `git status` empty), fresh build
directories. Prerequisites run first in that worktree: `./fetch_py37.sh`, then
`mech/build.sh python3.12` (it writes shapes_pb2 for 3.12). Then `./gate.sh python3.12
build/py37/python3.7`. The worktree was built from the committed core, not from `poc/codec`'s
working tree, which another agent is editing.

Result: **`gate exit 0`**. All 24 logs carry `# commit: c7c083f68`, and none says uncommitted.
The worktree and its builds were deleted after the run.
- `gen/generate.py --check` is clean (18 files).
- The shared `poc/codec/gen/generate.py --check` with the new slice guard (R-H13): the python
  slice passes ("slice python --check: exit 0"; no module of `poc/python/gen/` carries a wire
  token). The command's overall exit is 1, because the rust slice's check fails there; that is
  not this slice's.
- Crossing counts are unchanged by the core changes: 103 and 98 are identical.
- The earlier clean gates (d2cd0b0, b5f5bcfc4, 31fc3eecf, 3f2574775) also passed; their logs are replaced by these.
- Req 19's new count files are identical at both levels: `abi-full` 560 rows, `abi-nounk` 344,
  `rpc-full` 21, `rpc-nounk` 12 (with E-nounk; gate 103 and 105).

| step | what | 3.12 | 3.7 |
|---|---|---|---|
| 90 | build, both variants, every build control | pass | pass |
| 91 / 92 | conformance `_akffi` / `_akffi_rpc` | all checks pass | all checks pass |
| 93 | corpus, 6 arms, controls, decision 11 controls | pass; controls fail as required; D11 pass | the same; 3,278 re-encodings against 3.12, 0 differ |
| 94 | RPC gate under injected failure | 80 aborted, 0 timed | 80 aborted, 0 timed |
| 95 / 96 | R-D1 / U1 | pass / 8 readings | pass / 8 readings |
| 100 / 102 | conformance `_akffi_nounk` / `_akffi_rpc_nounk` | all checks pass | all checks pass |
| 101 | corpus through the no-unknown build | 4 arms pass; controls fail as required; variant controls pass | the same |
| 103 | whole-number crossing counts against `counts/` | drop and no-unknown identical | drop and no-unknown identical |
| 97 | the source check against 3.7.5 (and 3.9-3.13) headers | pass (one log) | |
| 98 | per-element counts against log 85 | identical | identical |
| 104 | camp_summary's two-build test (R-H1) | pass (one log) | |
| 105 | RPC cells B-E per call, both builds, against `counts/rpc-*.txt` | identical (one log, 3.12) | |

## What was checked, and the log that carries it

Full build, logs 90-98:
- **Build (90):**
  - R0 and R1;
  - every core exports `ak_init`, and every shim imports it and exports one `PyInit_`;
  - R5 (the shims' `ak_*` imports are unresolved);
  - must-fail controls:
    - the noinit shim imports no `ak_init`;
    - a planted layout mismatch refuses to import;
    - a shim without `AK_RPC` imports 0 of 6 section 9 entry points.
- **Conformance (91 `_akffi`, 92 `_akffi_rpc`):** R2 encode and decode on 16 payloads, every
  arm; section 10 layout (400 facts) compared at import; crossing counts from the counting
  build, both halves.
- **Corpus (93, 702 rows, each row in a worker process under a timeout):**
  - ffi-cext, ffi-attr, ffi-chunk256 and ffi-retain: 680 pass, 0 fail, 6 disputed
    (excluded), 16 not in the C ABI (`Nest`, refused by name);
  - py-drop and py-retain: 696 pass, 0 fail, 6 disputed;
  - between-arm identity: drop arms 542 rows, 0 differ; retain arms 542 rows, 0 differ;
  - every unknown row is written in the retained form by both retain arms, except the
    disputed `U-map-entry`;
  - the 3.7 run is compared byte for byte with the 3.12 run;
  - controls: `proj`, `reenc` and `accept` fail on every arm, and `noinit` fails on the ffi arms;
  - decision 11 controls:
    - zeroed position: 1,373 pairs, 0 mismatches;
    - wrong root: 812 pairs refused with -8;
    - leak: 0 undelivered buffers;
    - per-thread contexts: 0 created on reuse, 232 across 8 threads;
    - per-thread `AK_LAST_RECLAIMED`, and its process-wide twin failing as required;
  - rule 4 (R-H8): poc/cpp's three `d11_oneof` sequences through the retain arm. The final
    bag is the last member's own run (stamp -> nothing: run 2; stamp -> nothing -> stamp:
    run 3). After the switch to the scalar `as_int`, no object carries a bag. Every sequence
    has `last_reclaimed() == 0`, so each inactive slot was freed at delivery;
  - the leak check's must-fail twin (R-H9): `ctl/_akffi_corpus_skiprelease` (release skipped)
    is flagged on 307 of 311 rows by the leak check and on 3 of 3 sequences by the oneof control.
- **R-H14** (91, 92, 100, 102), every arm: an undeclared oneof case is refused with code -11
  (`.code`); a selected message member holding None writes an empty body, byte-identical to
  upb with the member set to an empty message.
- **94:** the RPC gate under injected failure (R-D3): 80 failure rows aborted, 0 timed.
- **95:** R-D1 wrapped lengths. **96:** U1.
- **97:** the source check: 10 build variants against 3.7.5 headers (`-Werror`), and against the
  3.9, 3.10, 3.11, 3.12 and 3.13 headers present; the pre-port tree fails, as the control.
- **98:** crossing counts per element, row for row identical to log 85; and the rendered C
  header compared with the cpp slice's (information, not a gate).

No-unknown build, logs 100-103:
- **Build (90):**
  - the variant cores export 0 u-family entry points (full: 21);
  - the variant shims import 0 of them;
  - each module resolves `libak_core.so` to its own core;
  - layout facts: 240 in the variant (full: 400; corpus 340 against 574);
  - must-fail: the variant shim over the full core refuses to import.
- **100 / 102:** conformance on `_akffi_nounk` / `_akffi_rpc_nounk`. Includes the check that no
  facade class, C type or decoded object has `_unknown` (38 classes, 19 C types, 32 objects).
- **101:** the corpus through the variant (4 arms):
  - 680 / 696 pass, 0 fail;
  - the 307 unknown rows that have a dropped form write it on every ffi arm; the 4 open-enum
    rows have none in the manifest and are checked by C3;
  - byte identity with the full build's drop arms at the same level: 2,181 re-encodings,
    0 differ;
  - the controls fail as required;
  - variant controls: 0 positions; 622 of 622 retain calls refused; wrong root -8; per-thread
    contexts; no `_unknown` on 60 classes, 29 C types and 622 decoded objects, with the full
    facade's 150 findings as the must-fail twin.
- **103:** whole-number crossing counts of both builds, identical to `counts/`. They differ in
  5 of 160 rows: P1.2 decode reverse is 8 in the full build (drop) and 5 in the no-unknown
  build, on all 5 backends.

## Crossing counts (R5)

Counting build, both halves, per element, log 91 (3.12; 3.7 identical). There are three edges
and they are never added: shim to CPython (counted by the shim), core forward and core
reverse (counted by the core). Whole-number totals per call are in `counts/`.

| payload | shim -> CPython, C ext type (enc / dec) | shim -> CPython, plain and `__slots__` (enc / dec) | core fwd (enc / dec) | core rev (enc / dec) |
|---|---|---|---|---|
| P1.2 M1 | 7.00 / 7.00 | 29.00 / 24.00 | 0.01 / 0.00 | 0.00 / 0.01 |
| P2.2 M2 | 51.67 / 51.67 | 146.68 / 131.35 | 5.02 / 0.00 | 5.00 / 7.00 |
| P3.1 M3 | 5.40 / 3.15 | 13.40 / 9.96 | 0.01 / 0.01 | 0.01 / 0.01 |
| P4.1 M4 | 23.00 / 23.00 | 54.01 / 49.01 | 1.01 / 0.01 | 1.00 / 3.00 |
| P5.1-P5.4 M5 | 3.00 / 3.00 | 7.00 / 8.00 | 1.00 / 1.00 | 0.00 / 1.00 |
| P6.1 M6 | 302.00 / 152.00 | 308.00 / 158.00 | 5.01 / 0.01 | 5.00 / 7.00 |
| P7.1 M7 | 2.00 / 2.00 | 5.33 / 4.33 | 0.50 / 0.17 | 0.33 / 1.17 |

## Decision 11 and the two builds, as rendered

- **Contexts:** one decode context per root and one encode context per thread (a pthread key),
  created on first use and reused. A decode or encode re-entered on a busy slot gets a
  temporary context.
- **Full build decode:** `ak_dec_reset_<R>(ctx, retain ? &opts : NULL)`, the decode, and in
  retain `ak_dec_reset_<R>(ctx, NULL)`. Options are laid out from `plan.unk_opts_layout`; one
  entry per oneof, filled in the active member's group (ABI v1 rule 4 as amended 2026-09-26).
  Every armed position grows through `ak_py_grow`.
- **Full build delivery:** each group's slot becomes the facade's `_unknown`. Absent children's
  and inactive members' slots are freed. A map entry's buffer is freed (the U-map-entry gap).
- **Full build encode:** `retain` selects `ak_uencode_<R>` over `ak_ufix` groups.
- **No-unknown build:** `ak_dec_ctx_new_<R>(void)`, the decode, and `ak_encode_<R>`. None of the
  above exists in it, and `retain` raises ValueError.
- **`AK_LAST_RECLAIMED`** is thread-local.

## CAMPAIGN.md section 10 checklist

| # | status | how, or why not |
|---|---|---|
| 1 | met | the suites run one after another. The machine and its tenancy are the owner's |
| 2 | met | governor, turbo and SMT are read from /sys into every header. Setting them is the owner's |
| 3 | met | `AK_ISOLATION`, or isolcpus/nohz_full from /proc/cmdline, in every header |
| 4 | met | the CPU sets come from `ffi/campaign.machine` (sizes 4 and 4), exported by `ffi/campaign.sh`, and read by `run_campaign.sh` from that file when unset; the client pins itself to `AK_CPU_CLIENT` and the server to `AK_CPU_SERVER` before any thread; the affinity in force is logged. Worker thread counts in every campaign log header: codec and calib 1 (no gRPC stack, no core runtime); RPC client pool 16, the core runtime's workers (`AK_CORE_WORKERS`, 2), the number of core clients and grpcio channels, and the process's OS threads before and after; the server's grpcio executor workers (32) and its threads. NUMA and SMT siblings are checked by campaign.sh, not by this runner |
| 5 | met | 3.7 runs the gate (corpus, byte identity) and no timing suite |
| 6 | met | `buildinfo.json` in every header: cc, CFLAGS, shared linkage, core profile (lto off), the features of every core of both builds; GC stated |
| 7 | met | 16 payloads; Latin-1 and wide on P1.2, P2.2 and P2.4 (R-H26); family `unknown` = the 92 accepted U-* rows at the shapes core's 7 roots, encode, decode and decode+read, through the timed `_akffi` / `_akffi_nounk` (R-H27); family `unknown-corpus` (the corpus-schema core, 311 rows) is a labelled extra |
| 8 | met | incumbent-prod, incumbent-best (labelled), core-ffi (push), host-gen; core-ffi-attr as a labelled extra. No pull arm exists in this slice (not measured) |
| 9 | met | encode, decode, and decode+read through one reader for every arm |
| 10 | met | core-ffi retain and drop in the full build; core-ffi no-unknown in the separate build (`--variant nounk`, its own pyperf invocation that also times the incumbent, order alternated by launch). host-gen drop and retain, over the same C-extension facade objects as core-ffi (R3, R-H16), and host-gen no-unknown: host-gen drop (the same text from both plans) over the no-unknown build's facade, which has no `_unknown` (R-H22), in the no-unknown process. host-gen over the plain facade is the labelled extra `host-gen-plain`. Incumbent at upb's default (retains), stated |
| 11 | met | every encode arm is timed with the input as one hot graph (`encode`) and as a pool of distinct graphs whose wire bytes reach `AK_POOL_BYTES` (default 13.75 MiB, cap `AK_POOL_MAX` 65536 graphs; built and checked outside the window; the smoke uses 1 MiB, stated) (`encode-pool`); the end state is grpcio's transport-ready `bytes` for every arm, and for core-ffi also the bytes copied into a bytearray sized once (`encode-reused`, `encode-pool-reused`). upb-python has no serialise-into entry point and host-gen appends to a bytearray that CPython reallocates when cleared, so neither has a reused-buffer row (stated in the header). Samples carry `input` and `end_state` |
| 12 | met | full build: A, B, C-retain, C-drop, D-retain, D-drop, E-retain, E-drop, F-retain, F-drop (E and F: host-gen over the core's transport and over grpcio, R-H35), plus the labelled extras; no-unknown build (its own process): A, B, C-nounk, D-nounk, E-nounk, F-nounk (host-gen drop over the no-unknown facade); `unknown_mode` on every sample |
| 13 | met | one `camp_server.py` per launch, started by `run_campaign.sh`, pinned to `AK_CPU_SERVER`, serving every cell of both builds on two Unix sockets (one per transport configuration, two grpcio servers in the one process); pre-serialised P2.2 on (a), (b) decoded by upb for every cell; warmed by `AK_CAMPAIGN_SERVER_WARMUP` (64) Get calls from each client transport (grpcio, core) before round 1; one channel per cell per launch (a grpcio channel for A, D, F; a core client for B, C, E and each extra), warmed by the per-cell warm-up |
| 14 | met | (a) reported as `a` and `a+read`, and (b). The optional streamed upload is not built (not measured) |
| 15 | met | 1, 8 and 16 in flight |
| 16 | met | B, C and E use the core's blocking delivery; queue and callback are labelled extras, direction (a) only; A, D and F use grpcio's idiomatic call, the generated stub's blocking unary multicallable (stated in the header) |
| 17 | met | shipped and pinned, the same switch for every cell; the socket is a Unix domain socket for every cell (grpcio `unix:` targets; the core dials `unix:` through tonic). grpcio has no connection-window argument; Nagle has no meaning on a Unix socket; both stated in the header |
| 18 | met | every call checked (status, length); the server checks every request; one failure aborts with no sample; the planted short-body control fails in the gate suite |
| 19 | met | the counting builds count every ABI call the timed call makes, resets apart (the shim's macros), after one warm call; the resets' place is stated (decode: before, and after in retain; encode: before). Committed and gated: `counts/abi-full.txt` (drop and retain, 16 payloads x 5 backends and the 92 U-* rows; retain with no pre-placed buffer and exact-size grow), `counts/abi-nounk.txt`, `counts/rpc-full.txt` / `rpc-nounk.txt` (RPC cells B, C, D, E per call in each build's modes, E-nounk included, on the rpc counting builds, gate 105), and the older `counts/crossings-{drop,nounk}.txt`; calib stops on a difference from `counts_expected.txt`; gate 98 against log 85 |
| 20 | not met | host forward and forward+reverse are measured separately, and the rust slice's crossing benchmark is built and run pinned. `perf stat` is implemented, but `perf` is absent in this container, so cycles and instructions have never been read |
| 21 | met | process CPU per round: codec (pyperf's value is CLOCK_PROCESS_CPUTIME_ID per loop), calib and rpc CLOCK_PROCESS_CPUTIME_ID; wall beside every one (R-H25) |
| 22 | met | codec: pyperf cannot interleave (not a defect, owner 2026-09-26); the order used: blocks of (payload, content, direction), the arms inside a block rotated by one per launch, the whole list rotated by a third per launch, stated in every codec header. rpc: the cells rotated by one per round, stated in the header |
| 22a | met | pyperf 2.10.0; `--processes 1 --values ROUNDS --warmups 3 --min-time 0.1 --affinity`; every raw value, warm-up and calibration exported by `camp_pyperf_export.py`, the raw pyperf JSON written beside it (the committed smoke has its figures stripped and omits that JSON) |
| 23 | met | defaults 5 rounds x 3 launches; every sample written (smoke: 1 x 1) |
| 24 | met | codec: pyperf's warm-up and loop calibration, exported; rpc: one sample's calls per cell before round 1 |
| 25 | met | M_TOP_PAD before any allocation; GC on, `gc.collect()` before every sample |
| 26 | met | codec, rpc and calib each call `need_gate` and refuse to time without a `gate.ok` for the trees they read (calib added, R-H19); the gate covers every codec arm in both unknown-field modes and the no-unknown build (101) |
| 27 | met | header: commit (a dirty tree refused unless `--allow-dirty`, smoke only), machine, CPU sets, versions, build, transport, warm-up, repeats |
| 28 | met | one JSON object per sample with the listed fields |
| 29 | met | `--out`; the smoke is in `logs/python/campaign/` |
| 30 | met | `camp_summary.py`: median [min, max] per (build, arm or cell, ...), and the ratio to the same build's incumbent-prod or cell A formed from per-launch medians (owner R-H24), labelled cross-process; references and groups keyed on `build`, which every sample carries (R-H1); tested with two builds whose incumbents differ (gate 104) |
| 31 | met | `run_campaign.sh` with the common interface. The top-level `ffi/campaign.sh` is the aggregating session's |
| 32 | met | the smoke logs' headers carry the INSTRUMENTATION line; no figure from them is quoted |

## Register H (WP6 re-review), python's findings: disposition proposed

| finding | confirmed? | evidence | fix | proposed disposition |
|---|---|---|---|---|
| R-H1 | confirmed | the old `camp_summary.py` keyed `base` on (payload, content, dir, launch, round) and the groups without a build, and the sorted glob loads `-nounk-` last, so it overwrote the full build's incumbent-prod and cell A | `build` in every sample (`camp_lib.Log(build=)`); references and groups keyed on it; ratios from per-launch medians; `test_camp_summary.py` (two builds, incumbents 100 and 400 ns, RPC A 1000 and 6000; its twin without `build` gives the pooled 0.8, not 2.0), gate 104 | fixed |
| R-H16 | confirmed | `camp_codec` host-gen used `fp` / `CT_PLAIN` while core-ffi used `fc` / `CT_CEXT` | host-gen drop and retain now over the C-extension facade objects core-ffi uses (shapes and unknown families); `host-gen-plain` is the labelled extra. The C type is the headline because it is the facade the composed binding ships: a field is a struct member to the shim | fixed |
| R-H8 | confirmed | no corpus row put unknowns in a oneof member through ffi-retain | poc/cpp's three sequences in the d11 controls: exact bags, the scalar switch leaves no bag, `last_reclaimed() == 0` (log 93) | fixed |
| R-H9 | confirmed | the leak check had no twin | `AK_PLANT_SKIP_RELEASE` shim: flagged on 307 of 311 rows and 3 of 3 sequences (log 93) | fixed |
| R-H14 | confirmed, all three parts | py_pure had its own `ERR` table; the undeclared case raised a bare ValueError (pure and shim); a selected None member raised (pure and shim) | codes read from `plan.FIXED.codes`; an undeclared case is refused with -11 as `.code` (pure: `EncodeError`; the shim passes the case to the core, whose AK_ERR_ABI is raised with `.code`); a None member is written as an empty body, per plan ENCODE RULES ("written iff the case selects it, whatever its value"); checked on every arm (logs 91, 92, 100, 102) | fixed |
| R-H2 (python part) | confirmed | `camp_rpc.sample` created and joined `inflight` threads inside the timed window | one pool of max(in flight) threads, created before the first window and reused (idle threads block on an Event); smoke at 31fc3eecf: 16 threads started, 32 contexts, 0 leaked, every call checked | fixed |
| R-H19 (python part) | confirmed, both | `run_campaign.sh` calib never called `need_gate`; "in-process control" wording in `run_campaign.sh` and `camp_codec.py` | calib calls `need_gate`; wording corrected to "same-launch control" (pyperf spawns a worker per benchmark). The RPC client's "in-process controls" A and B are left as they are: they do share the client process | fixed |
| R-H23 | owner decision | pyperf cannot interleave | order stated (checklist row 22); arms rotated inside each block per launch | stated; rotation added |

## What the plan does not carry (reported, not decided here)

1. **ABI v1 sections 3-5's fixed vocabulary**: error codes, the span and blob types, the
   context and callback types, the fixed exports and the counters. Every C header renderer
   restates them as fixed text.
2. **`plan.lifecycle`** names but does not define the `AK_INIT_*` values, `ak_err`'s layout or
   `ak_log_fn`'s signature.
3. **Map entry order** on encode. This backend sorts by key (UTF-8 byte order).
4. **`presence == "direct"`** has no encode rule. On the wire it is implicit-presence bytes.
5. **The 10th varint byte**: bits past 64 are dropped by the core and by the pure-Python codec.
6. **Group depth**: the core bounds groups at 100 per skip, independent of message depth, and
   the pure-Python codec follows the core.
7. **Field numbers above 2^29-1**: not stated. The core truncates `key >> 3` to u32; the
   pure-Python codec does not. No corpus row reaches it.
8. **Unknown fields inside a map entry** have no bag in any facade (`U-map-entry`, disputed).

## Open defects

| # | where | what |
|---|---|---|
| U1 | the incumbent | upb drops a map entry carrying an unknown field (log 96) |
| noinit C4 | `corpus.py` | under the `noinit` control a reject row is "refused" with -10 and counts as refused; only the accept rows show the control. The rust harness does the same |

Fixed defects (D1-D14, the NULL module state in `mod_traverse`, the process-wide
`AK_LAST_RECLAIMED`) are in JOURNAL.md.

## What is not measured, or not built

- **Every performance figure.** Deferred to the campaign. The container smokes are
  instrumentation.
- **Hardware counters** (req 20): `perf` is absent here.
- **The re-entrant temporary-context path** (a decode or encode called from inside a
  callback on the same thread for the same root): implemented, exercised by no control.
- **3.8 to 3.11 and 3.13 at run time.** Those interpreters have no protobuf/grpcio here. The
  shim is compiled against their headers (log 97) but not run.
- **3.7 floor limits:** no `ssl` module (bionic's interpreter without libssl1.1); protobuf
  4.24.4 as the incumbent there.
- **Not built:**
  - the pull decode family (no pull arm);
  - the streamed-upload direction (req 14, optional);
  - an encode-side RPC server arm;
  - streaming, TLS, deadlines, metadata;
  - free-threaded CPython;
  - abi3 for the shim;
  - decision 13's borrowed span;
  - allocation per operation.
- **C5 for the five large rows** without a projection (`B-P2_5`, `B-P4_1`, `C-elemu-512`,
  `C-leaf-2048`, `C-mixed-100`): checked by C3 re-encode only (log 93).
- **The concurrency suite** (`concurrency.py`) has no current committed log. Log 56 is from an
  older commit.
- **Content sets** are measured on P2.4 only; P1.2's crossing counts cover ASCII only.
- **The facade's `_unknown` in the full build** is one slot per object. It is priced only
  through the full build against the no-unknown build.
- **The campaign smoke** in `logs/python/campaign/` is from the clean worktree at c7c083f68,
  figures stripped. It shows every WP7 row:
  - codec full: 836 shape values, which include `encode-pool`, `encode-reused` and
    `encode-pool-reused`, and Latin-1/wide on P1.2, P2.2 and P2.4;
  - codec no-unknown: 396 shape values, host-gen no-unknown included (and in the unknown
    family, 45 values);
  - the unknown family through the shapes core, and the unknown-corpus extra;
  - RPC full: 204 samples, which include E-drop, E-retain, F-drop and F-retain;
  - RPC no-unknown: 108 samples, cells A, B, C-nounk, D-nounk, E-nounk, F-nounk;
  - one server for both builds, over Unix sockets.
  The smoke's pool is 1 MiB (the campaign's default is 13.75 MiB).
- **No reused-buffer encode for the incumbent and host-gen** (req 11 (i)): upb-python has no
  serialise-into entry point, and host-gen appends to a bytearray that CPython reallocates.
- **RPC counts** are taken on the `shipped` transport, one call at a time.

## Next step

1. The owner's campaign run: `./run_campaign.sh --suite gate|calib|codec|rpc --out
   ffi/logs/python/campaign` without `--smoke`, with the CPU sets exported.
2. Re-render (`gen/generate.py`) and re-gate (`./gate.sh python3.12 build/py37/python3.7`)
   whenever `plan.py`, `c_abi.py`, a python backend or the core changes. A changed crossing
   count in either build stops the gate until the file in `counts/` is reviewed and replaced.

## Log index

**Results** (correctness, counts, feasibility):

| log | what it establishes |
|---|---|
| `90-wp5-build.log` | the build of both variants at 3.12.3 and 3.7.5, R0, R1, R5 and every build control |
| `91-wp5-conformance-py3.{12,7}.log` | R2 both directions on `_akffi`, layout at import, crossing counts (per element and whole numbers) |
| `92-wp5-conformance-rpc-shim-py3.{12,7}.log` | the same on `_akffi_rpc` |
| `93-wp5-corpus-py3.{12,7}.log` | the whole corpus, 6 arms, controls, decision 11 controls; 3.7 against 3.12 byte for byte |
| `94-wp5-rpc-gate-py3.{12,7}.log` | R-D3 |
| `95-wp5-rd1-lenwrap-py3.{12,7}.log` | R-D1 |
| `96-wp5-u1-py3.{12,7}.log` | U1 |
| `97-wp5-floor-source.log` | 10 variants against 3.7.5 headers and the other header sets; the pre-port control |
| `98-wp5-counts-vs-85.log` | per-element crossing counts against log 85 |
| `100-wp5s10-conformance-nounk-py3.{12,7}.log` | conformance on `_akffi_nounk`, the `_unknown` facade check |
| `101-wp5s10-corpus-nounk-py3.{12,7}.log` | the corpus through the no-unknown build, its controls |
| `102-wp5s10-conformance-rpc-nounk-py3.{12,7}.log` | conformance on `_akffi_rpc_nounk` |
| `103-wp5s10-counts-drop-vs-nounk.log` | whole-number counts of both builds against `counts/`, and their difference |
| `105-wp7-rpc-counts.log` | RPC cells B, C, D, E per call on the rpc counting builds, both builds (req 19) |
| `104-wp6-camp-summary-test.log` | camp_summary's two-build test (R-H1) and its twin |
| `85-conformance-rpc-shim.log` | the pre-port shim's crossing counts (the reference for 98) |
| `52-r14-baseline.log` | R14 derived from `Protos/V1` |
| `99-wp5-corpus-chunk256-before-D13.log` | D13 before its fix (a defect record; uncommitted code, stated in its header) |
| `70`, `81`-`89`, `53`, `54`, `01` | superseded or older-commit correctness logs, kept as history |

**Instrumentation** (container timings; not quoted): `campaign/` (the WP6 smoke of both
builds from the clean worktree at 31fc3eecf: gate, codec, rpc, calib; headers marked
INSTRUMENTATION, **figures stripped**: no cpu_ns/wall_ns, no timing column, no pyperf JSON), `00-r13-rust-crossing.log`,
`10`, `20`, `30`/`31`, `40`/`41`, `50`/`51`/`60`/`61`, `55-allocator.log`, `56-concurrency.log`,
`57-gc-bias.log`, `62`/`63`, `86-rpc-smoke-gated.log` (timing rows deleted). The `wall` line of
log 93 is instrumentation. `20` and `40`/`41` come from the retired `mech/gen` codec arms and
cannot be reproduced from the tree.
