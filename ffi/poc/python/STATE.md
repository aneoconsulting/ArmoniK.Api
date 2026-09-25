# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists and what
was checked; it does not say what a binding should choose (`CLAUDE.md`, roles).

**Phase** (README 1.1): setup and design. This file carries **no timing figure**. Timing logs
are listed at the foot as instrumentation.

| | |
|---|---|
| **Status** | Work unit 8 done: **the no-unknown variant** (FIX-PLAN WP5 step 10 port) rendered, built, gated at 3.12 and 3.7 and in the codec and RPC harnesses; `AK_LAST_RECLAIMED` per thread. See the section below. Before it, work unit 7: **decision 11 through the C ABI** (FIX-PLAN WP5 step 9). The shim renders root-bound contexts **held per thread** (one per root and one encoder per thread, created lazily, reused; correction of the per-call context), per-root `ak_dec_<Root>_opts` armed per decode, a C grow, slots taken into the facade's `_unknown`, and the `ak_uencode_*` family. **ffi-retain** passes the whole corpus at 3.12 and 3.7 (no retention gap except the disputed U-map-entry), and the decision 11 controls pass. **CAMPAIGN requirement 10 is now met**: core-ffi retain is timed in the pyperf codec suite. WP3 harness (work unit 6) below |
| **Floor** (owner D1: 3.7) | **Builds and passes every gate on CPython 3.7.5** (Ubuntu 18.04's packages from archive.ubuntu.com, `fetch_py37.sh`): logs 90-97 |
| **Target** (owner D1: 3.12) | Built and gated on **3.12.3**: logs 90-96, 98. grpcio 1.84.0, protobuf 7.36.2 (upb) |
| **Incumbent** (R14) | protobuf on **upb** through gRPC's generated marshaller path (`SerializeToString` / `FromString`), `verify_r14.py` (log 52). On the 3.7 floor: protobuf 4.24.4 (upb), for the conformance gate only |
| **Core** | `poc/codec`, the one core (R0), this checkout (commit in each log header; the `AK_UPSTREAM` snapshot mechanism is retired). Four builds, all `init-guard`: plain, `count`, `rpc`, `corpus` |

## Decision 11 through the C ABI (work unit 7, WP5 step 9)

`py_capi` (in `poc/codec/gen/`) renders:
- **Contexts, per thread:** a pthread key holds each thread's block: one decode context per
  root (`ak_dec_ctx_new_<R>(NULL)`, created on that thread's first decode of R) and one encode
  context. The key's destructor frees a block at thread exit; the module's `m_free` frees the
  calling thread's block and deletes the key (another thread still alive then is not freed
  before process exit). A decode or encode re-entered on the same thread for a busy slot (a
  callback that decodes) gets a temporary context, freed after; that path is not exercised
  by any control. The reason for per-thread rather than one per process: the GIL can switch
  threads inside a decode callback (facade `__init__`, pyacc accessors are Python code).
- **Decode:** `decode_<b>_<R>(buf, acc, T, retain, zero)`:
  - the thread's context for R;
  - `ak_dec_reset_<R>(ctx, retain ? &opts : NULL)`, return checked (a reset per decode, rule 7);
  - the decode;
  - retain only: `ak_dec_reset_<R>(ctx, NULL)`, return checked (the disarm; drop mode has
    nothing armed, so it pays one reset per decode).
- **Encode:** the thread's encode context, `ak_enc_reset` before each encode except the first;
  the counting build resets the counters per call (`ak_enc_counters_reset`/`ak_dec_counters_reset`).

  `ak_dec_<R>_opts` is laid out from `plan.unk_opts_layout` in the decode's frame, so it
  stays unmoved while armed. Every armed position uses `ak_py_grow` (realloc; NULL/0 means
  a fresh buffer; each buffer carries a header linking it into the HostCtx's live list).
- **Delivery:** at every delivery, `setgroup` turns the group's own slot into the facade's
  `_unknown` and frees it. Absent children's and inactive oneof members' non-NULL slots are
  freed through `dropgroup_<M>`. Map entries' buffers are freed at `add`, because the facade
  dict has no bag for them (the U-map-entry gap). `ak_py_reclaim` frees whatever a failed
  decode never delivered.
- **Encode:** `fillu_`/`loopu_` fill the `ak_ufix` groups, including each message's
  `_unknown` as `unknown: ak_blob`, and call `ak_uencode_<R>` and `ak_uelem*_`.
- **`native/binding.c`:** `encode(..., acc, retain)`, `decode(..., acc, retain, zero_mask)`,
  `unk_positions(root)`, `wrong_root(a, b)`, `last_reclaimed()`, `tls_created()` (contexts
  created through the thread key).

Drop mode is the same code path with every entry zero. Crossing counts are unchanged (log 98,
160 rows at both levels), and the per-decode resets add no counted crossing.

**Gate at 6feff87 (python code at acb5128), 3.12 and 3.7** (logs `90`-`98`, rerun after the
per-thread correction; the same figures as the per-call gate at 883ae3b):
- Conformance passes on both shims.
- **Corpus, 6 arms** (the corpus now has 702 rows):
  - ffi-cext, ffi-attr, ffi-chunk256 and **ffi-retain** each pass 680 with 0 failures
    (6 disputed, 16 not in the C ABI).
  - py-drop and py-retain each pass 696.
  - Drop arms are byte-identical to each other on 542 rows, and ffi-retain is identical to
    py-retain on 542 rows.
  - **The retained form is written on every unknown-field row by both retain arms**; the only
    exception is `U-map-entry`, which is disputed and excluded (its reading is reported).
  - 3.7 re-encodes all 3,278 (arm, row) pairs to the 3.12 bytes.
- **Decision 11 controls:**
  - Zeroed position: 1,373 (row, position) pairs over 311 rows, 315 of which remove a bag,
    0 mismatches.
  - Wrong root: all 812 ordered pairs are refused with -8 and nothing is delivered; the
    positive control passes on 29 of 29 roots.
  - Leak: 0 undelivered buffers after a successful retain decode.
  - noinit now also covers ffi-retain.
  - **Per-thread contexts:** 39,808 further decode+encodes on one thread create 0 contexts;
    8 threads at a 1 us switch interval, 12,440 decode+encodes each, re-encode every row to
    the single-thread bytes with 0 errors and 0 reclaimed buffers, and create exactly 232
    contexts (8 x (28 roots + 1 encoder)).
- RPC gate, R-D1, U1 and the 3.7 source check all pass.

**Requirement 10:** the pyperf codec suite times core-ffi in drop AND retain, shapes and the
unknown rows, plus host-gen drop and retain and the incumbent at its default. Smoke,
instrumentation: `logs/python/campaign/codec-*` (rerun on the per-thread contexts).

## The no-unknown variant (work unit 8, FIX-PLAN WP5 step 10 port)

Unknown-field support is **compiled out**, not disabled at run time.
- **Render:** `gen/generate.py` renders `gen/out/nounk/` and `gen/out/corpus-nounk/` from
  `P.relower(p, p.options.with_unknown("drop"))`: `py_capi.emit` (this slice's backend,
  `poc/codec/gen/py_capi.py`) and `c_abi.emit`'s second header (`AK_NO_UNKNOWN_FIELDS 1`).
  - The variant shim has no `ak_ufix`, `fillu_`/`loopu_`, options, `ak_dec_reset_<Root>`,
    `ak_uencode_*`/`ak_uelem*`, grow/reclaim or `unknown` member reads.
  - It calls `ak_dec_ctx_new_<Root>(void)`. A `retain` argument is refused (ValueError).
  - The full build's generated text changed only by the thread-local `AK_LAST_RECLAIMED`.
  - The facade has no `_unknown` (work unit 8 follow-up, CAMPAIGN req 10):
    - `py_pure.emit_facade` and `py_capi`'s C types omit it when `unknown_compiled_out`: no
      attribute, no `__slots__` entry, no constructor argument, no C member.
    - The variant writes its own `gen/out/nounk/facade.py` and `gen/out/corpus-nounk/facade.py`,
      and host-gen drop beside each (`pycodec.py`, the same text as the full build's).
    - A variant process (a `_nounk` shim, or `AK_VARIANT=nounk`) imports from there and has no
      retain codec.
    - The full build's generated text is unchanged (`generate.py --check`).
    - Check (logs 100 and 101): no Plain/Slots class, no C type and no decoded object of the
      variant has `_unknown`:
      - shapes: 38 classes, 19 C types, 32 decoded objects;
      - corpus: 60 classes, 29 C types, 622 decoded objects.
    - Its must-fail twin: the full build's corpus facade shows 150 findings.
- **Build** (`build.sh`): four more cores with `--no-default-features` (`init-guard`,
  `count,init-guard`, `rpc,init-guard`, `corpus,init-guard`), each in its own target
  directory (`build/cargo/*-nounk`). Separate modules are built from them: `_akffi_nounk`,
  `_akffi_count_nounk`, `_akffi_rpc_nounk`, `_akffi_corpus_nounk`,
  `_akffi_corpus_chunk_nounk`, plus the noinit control. `native/binding.c` selects the
  variant with `-DAK_NOUNK`, and `#error`s if the shim and the header disagree.
- **Build checks:**
  - The variant cores export 0 u-family entry points, against 21 in the full cores.
  - Each module resolves `libak_core.so` to its own core and imports 0 u-family entry
    points in the variant (full: 21, corpus 70).
  - Layout facts: 240 in the variant against 400 in the full build (corpus: 340 against 574).
  - Must-fail: the variant shim linked to the full core refuses to import at the layout check.
- **Gate** (logs 100-103 at 3.12 and 3.7):
  - conformance on `_akffi_nounk` and on `_akffi_rpc_nounk`;
  - the whole corpus through the variant:
    - 4 arms, 0 failures;
    - the 307 unknown rows that have a dropped form write it on every ffi arm; the 4
      open-enum rows have no dropped form in the manifest and are checked by C3;
    - byte identity with the full build's drop arms at the same level: 2,181 re-encodings,
      0 differ;
    - the controls fail as required;
  - the variant's own controls: 0 positions; 622 of 622 retain calls refused; wrong root
    refused with -8 on 812 pairs; per-thread contexts; no `_unknown` on any facade.
- **Crossing counts** (step 103, whole numbers per call, committed in
  `counts/crossings-{drop,nounk}.txt`):
  - only P1.2 decode moves, reverse **8 to 5**, on all 5 backends; 5 of 160 rows differ,
    the same as in the rust slice;
  - the per-element table (log 98) rounds this away, which is why the files hold
    whole-call totals.
- **Harness:**
  - codec: `camp_pyperf.py --variant nounk` writes `codec-<family>-nounk-launchN`;
  - RPC: `camp_rpc.py --variant nounk` writes `rpc-nounk-launchN`;
  - `run_campaign.sh` runs both builds per launch, in an order alternated by launch.
  - No figure crosses the two builds except through the incumbent rows timed in the same
    process.
- **`AK_LAST_RECLAIMED` per thread** (`_Thread_local`, or `__thread`).
  - Control (the d11 controls, log 93): thread A's retain decode fails after growing a buffer
    (reclaims 1). Thread B then makes a successful decode (0). A reads 1 again.
  - Its must-fail twin (`ctl/_akffi_corpus_globalreclaim`, built with `AK_THREAD_LOCAL`
    empty) reads B's 0, and the control fails as required.

## RPC cells per unknown-field mode (CAMPAIGN req 12 as amended, 85cb00f)

`camp_rpc.py` runs C and D as **C-retain, C-drop, D-retain, D-drop** in every direction
(a, a+read, b). Retain passes `retain=True` to decode and encode (every position armed, on the
per-thread contexts); drop is the same build with every entry zero. Every sample row carries
`unknown_mode` (A, B: `incumbent-default`; C-queue, C-callback: `drop`). C-nounk/D-nounk are
the no-unknown build's client (work unit 8, section above).
Checks per transport, and a failed one aborts with no sample written:
- every call checked as before;
- the retain control: P2.2 with field 1000 appended is re-emitted by the retain calls and
  dropped by the drop calls;
- after the run, the binding's `unk_totals()`: leaked buffers 0, both modes ran (the decode
  counts are exact: 384 retain, 576 drop at `--calls 16`, one round), and contexts created
  stay per thread.
Smoke (instrumentation): `logs/python/campaign/rpc-launch1.jsonl`.

## WP3: the campaign harness (work unit 6)

`run_campaign.sh --suite codec|rpc|calib|gate --out <dir>` (owner's run: `--out
ffi/logs/python/campaign`). Environment: `AK_CPU_CLIENT`, `AK_CPU_SERVER`, `AK_ISOLATION`,
`AK_PY` (3.12), `AK_FLOOR_PY` (3.7 from `fetch_py37.sh`), `AK_SNAPSHOT` (default HEAD: the
core and the generator come from a `git archive` of that commit, never the working tree).
Files: `camp_lib.py` (pinning, clocks, header, JSON body), `camp_codec.py`, `camp_rpc.py` +
`camp_server.py`, `camp_calib.py`, `camp_summary.py`, `counts_expected.txt`, `arms_plan.py`.

**Smoke run** (1 launch, 1 round, reduced iterations; every log carries the INSTRUMENTATION
line): `logs/python/campaign/`. The gate passed at 3.12 and 3.7 (gate/90-98, plus the RPC
runner's must-fail control). Samples: calib 2 plus the rust crossing log, codec shapes 324,
codec unknown 3732 (311 rows), rpc 96. HEAD moved during the run (154803d at the gate,
dadea5f for the timed suites). The gate stamp is keyed on the source trees the build reads,
and it accepted the later suites, so those trees are the same at both commits. From now on
the header prints the resolved snapshot sha, where this run's logs print `HEAD`.

### CAMPAIGN.md 22a (cbd3252): the codec suite on pyperf

`run_campaign.sh --suite codec` times with **pyperf 2.10.0** (`camp_pyperf.py`, installed into
`build/pyperf` by the runner). The mapping is one pyperf invocation per launch (at least 3,
with the benchmark order rotated a third of the list per launch), `--processes 1 --values
ROUNDS --warmups 3 --min-time 0.1 --affinity $AK_CPU_CLIENT --copy-env`. Each benchmark gets
a fresh worker per launch plus pyperf's loop-calibration worker. The time_func returns
**CLOCK_THREAD_CPUTIME_ID**, so pyperf's values are CPU time per loop; the wall time of the
same call is written to a side file and joined back. `camp_pyperf_export.py` writes every raw
measurement into section 7: values (`phase` "value", `round` = the value's index), warm-ups
("warmup") and calibration ("calibration"). A value without its wall record stops the
export. The raw pyperf JSON is committed next to it. The cases, arms, directions, modes and
per-case correctness check are camp_codec's; its own loop is kept only for by-hand comparison
and is not driven by the runner. RPC and calib stay on the slice runner (requirements 13 and
18). Cost: the full unknown family is 3,732 benchmarks x 2 workers per launch. The smoke ran a
five-row subset (`--only`, stated in the log).

Smoke (instrumentation, `logs/python/campaign/codec-*`): the gate re-passed at 17b5fd5.
Shapes: 324 values, 2,621 raw measurements. Unknown (5 rows): 48 values, 672 raw measurements.

Checklist deltas from 22a: **22** interleaving within a process is replaced by pyperf's model
(one worker per benchmark), with the order rotated between launches; **23** launches =
pyperf invocations, rounds = values per worker; **24** warm-up and calibration are pyperf's,
recorded and exported; **21** CPU time is the pyperf value, wall beside; **25** GC ON in every
worker (`gc.collect()` before each timed call) and M_TOP_PAD at worker import; **28** every
raw measurement exported.

### Checklist (CAMPAIGN.md section 10)

| # | status | how, or why not |
|---|---|---|
| 1 | met (harness side) | suites run one after another; the machine and its tenancy are the owner's |
| 2 | met (recorded) | governor, turbo, SMT read from /sys into every header; setting them is the owner's |
| 3 | met (recorded) | `AK_ISOLATION`, or isolcpus/nohz_full from /proc/cmdline, in every header |
| 4 | met | client pins itself to `AK_CPU_CLIENT` and the server to `AK_CPU_SERVER` (sched_setaffinity before any thread); the affinity in force is logged. NUMA and SMT-sibling disjointness are not checked by the runner |
| 5 | met | 3.7 runs the gate (corpus, byte identity) and no timing suite |
| 6 | met | `buildinfo.json` in every header: cc, CFLAGS, shared linkage, core release profile (lto=false), features incl. `init-guard`; GC stated |
| 7 | met | 16 payloads; the content sets on P2.4 (SHAPES.md P10 row; the Rust slice's recode rule); every corpus U-* row whose root the C ABI carries, disputed excluded (311 rows; the Nest rows and U-map-entry named with the reason) |
| 8 | met | incumbent-prod (SerializeToString / FromString), incumbent-best (SerializePartialToString, ParseFromString into a reused message), core-ffi (push), host-gen; core-ffi-attr as a labelled extra. No pull arm exists in this slice |
| 9 | met | encode, decode (bare), decode+read through one reader for every arm |
| 10 | **met** (work unit 8, three modes) | core-ffi drop and retain (decision 11, every position armed; `ak_uencode_*` on encode) in the full build, and core-ffi **no-unknown** in the separately built variant (`--variant nounk`: `_akffi_nounk` / `_akffi_corpus_nounk`, own pyperf invocation with the incumbent as its control, order alternated by launch). host-gen drop and retain; host-gen has **no** third arm because `py_pure.emit_pycodec` renders byte-identical text for drop from the full and the relowered plan (checked), so host-gen drop is already the compiled-out form. Incumbent at upb's default (retain, stated) |
| 11 | met (argued) | the same object graph is re-serialised each iteration; neither upb-python nor the facades memoise a serialised size or form, so nothing is amortised. Stated, not rebuilt per iteration |
| 12 | **met** (req 12 as amended) | full build: A, B, C-retain, C-drop, D-retain, D-drop; no-unknown build (`camp_rpc.py --variant nounk`, `_akffi_rpc_nounk`, its own process): A, B (in-process controls), C-nounk, D-nounk. `unknown_mode` on every sample |
| 13 | met | `camp_server.py`, a separate pinned process; (a) pre-serialised P2.2; (b) the server decodes with upb, identically for every cell |
| 14 | met | (a) and (b); the optional streamed upload is not built |
| 15 | met | 1, 8, 16 in flight |
| 16 | met | B and C blocking; queue and callback as labelled extras, direction (a) only |
| 17 | met | shipped and pinned, the same switch for all four cells and one server process per transport. grpcio has no connection-window argument and sets TCP_NODELAY itself; both stated in the header |
| 18 | met | every call checked (status, length); the server checks every request; one failure aborts with no sample. The planted short-body control is seen failing in the gate suite |
| 19 | met | calib compares the counting build with `counts_expected.txt` and stops on a difference; gate step 98 does the same against log 85; gate step 103 compares whole-number per-call counts of the full build (`counts/crossings-drop.txt`) and the no-unknown build (`counts/crossings-nounk.txt`) with their committed files |
| 20 | partly met | host forward and fwd+reverse measured separately (reverse alone is the difference, left to the summary); the rust slice's `bench` is built from the snapshot and run pinned. `perf stat` is implemented but `perf` is absent in this container, so cycles and instructions are unverified |
| 21 | met | codec: CLOCK_THREAD_CPUTIME_ID; rpc: CLOCK_PROCESS_CPUTIME_ID of the client, wall beside |
| 22 | met (22a) | codec: pyperf, one worker per benchmark, order rotated between launches; rpc: rotated by one per round within (transport, direction, in flight) |
| 23 | met | defaults 5 rounds x 3 launches, every sample written (smoke: 1 x 1) |
| 24 | met | codec: pyperf's warm-up (3 values) and loop calibration, exported; rpc: one sample's calls per cell before round 1 |
| 25 | met | M_TOP_PAD before any allocation (J26); GC ON, `gc.collect()` before every sample, stated |
| 26 | met | codec, rpc and calib refuse without a `gate.ok` for the trees they read; the corpus runs every codec arm in both unknown-field modes (ffi-retain added), and the no-unknown variant has its own corpus gate (step 101) |
| 27 | met | header: commit (a dirty tree is refused unless `--allow-dirty`), machine, CPU sets, versions, build, transport, warm-up, repeats |
| 28 | met | one JSON object per sample with the listed fields |
| 29 | met by parameter | `--out`; the smoke is in `logs/python/campaign/` |
| 30 | met | `camp_summary.py`: median [min, max] and per-round ratio only |
| 31 | met for the slice | `run_campaign.sh` with the common interface. The top-level `ffi/campaign.sh` is outside this slice's directory (aggregating session) |
| 32 | met | the smoke runs (headers marked INSTRUMENTATION; the no-unknown files beside the full ones), and this section |

Harness changes the contract forced: the RPC server moved out of process (R-C3/R-C4); the
collector is ON for timed runs (bench.py's GC-off rule is superseded for the campaign); decode
is reported bare and with every field read in both the codec suite and the RPC cells (R-C2).

## What exists (work unit 5)

```
poc/codec/gen/py_pure.py   (shared backend) facade.py + pycodec.py (drop) + pycodec_retain.py
poc/codec/gen/py_capi.py   (shared backend) binding.c: the CPython shim, 3 accessor backends,
                           both directions, ak_init (plan.lifecycle), the section 10 table
                           (cpp_layout.facts), PY_VERSION_HEX conditionals (R-D4)
poc/codec/gen/cpp_abi.py   (the C++ backend's, used as is) ak_abi.h, plain C99, from the plan;
                           byte-identical to poc/cpp's own headers (log 98)
gen/generate.py            glue: two plans -> gen/out/ (shapes.json, 7 roots) and
                           gen/out/corpus/ (the corpus READER plan); --check runs the shared
                           one-generator guard over the python backends, planted violation incl.
gen/out/                   emitted and committed: facade.py, pycodec.py, pycodec_retain.py,
                           binding.c, ak_abi.h, and the same five under corpus/
native/binding.c           the module wrapper (hand-written, names no message or field): RPC
                           types and prototypes from the generated header (R-G4), ak_init
                           first in mod_exec, layout check at import, 3.7-compatible branches
build.sh                   R0, R1, four cores (init-guard), six shims per interpreter
                           (_akffi, _akffi_count, _akffi_rpc, _akffi_corpus,
                           _akffi_corpus_chunk, ctl/_akffi_corpus_noinit), R5, controls
gate.sh                    the correctness gate at the target and the floor -> logs 90-98
fetch_py37.sh              CPython 3.7.5 + its incumbent into build/py37 (sha256-pinned debs)
floor_check.sh             the 3.7-header source check with its pre-port control (log 97)
facts.py                   the facade's MESSAGES table re-shaped for the harnesses (walk.py's
                           replacement; states no rule)
payload_values.py          the payload builder (harness glue; moved out of gen/out, it was
                           never generated from anything)
conformance.py             R2 both directions, layout, crossing counts (both halves)
corpus.py                  the WHOLE corpus, 5 arms, worker processes under a timeout, C1-C5,
                           disputed readings, between-arm identity, 4 planted controls,
                           --dump/--compare for cross-level byte identity
rpc_gate.py, rd1_lenwrap.py, u1_map_unknown.py, concurrency.py   gates and suites
rpc.py, bench.py, allocator.py, gcbias.py, arms.py, verify_r14.py, mech/   harnesses (timings:
                           instrumentation)
run.sh                     mech/build.sh (shapes_pb2), gate.sh, R14, then the timing harnesses
                           (never on the floor)
```

## What was checked, and the log that carries it

Logs 90-98 exist for both `py3.12` and `py3.7` where the name says so.

| check | result | log |
|---|---|---|
| R0 one core; R1 generated tree current; one-generator guard over `py_pure`, `py_capi` with its planted IR import caught | pass | `90` |
| every core exports `ak_init`; every shim imports it and exports exactly one `PyInit_`; the noinit control imports no `ak_init`; a planted layout mismatch refuses to import naming the fact; a shim without `AK_RPC` imports 0 of 6 section 9 entry points | pass, both levels | `90` |
| R2 encode, byte identity against `schema/generated/manifest.json`, 16 payloads, every arm (`pycodec-retain` added) | pass (P7.1 as a permutation, upb's map order as a legal alternative) | `91`, `92` (3.12 and 3.7) |
| R2 decode, re-encode identity and field identity against upb, 16 payloads | pass | `91`, `92` |
| section 10: 380 layout facts compared with the core's AT IMPORT (was: "the build would have failed") | pass | `91`, `90` (control) |
| **crossing counts**, counting build, both halves | **identical row for row (160 rows) to log 85's pre-port shim**, on 3.12 and 3.7 | `98` |
| **the WHOLE corpus (691 rows)**, 5 arms, every row in a worker process, timeout 20 s | **ffi-cext, ffi-attr, ffi-chunk256: 672 pass, 0 fail, 3 disputed (excluded), 16 not in the C ABI (`Nest`, refused by name). py-drop, py-retain: 688 pass, 0 fail, 3 disputed.** 0 hung, 0 crashed. Between-arm identity: 534 rows, 0 differ. Obligations met: ffi arms C1 534, C2 529, C3 534, C4 138, C5 197; pure-Python arms C1 547, C2 542, C3 547, C4 141, C5 204; plus 5 large `produce` rows with no projection, checked by C3 only | `93` (3.12 and 3.7) |
| pycodec's 42 failures from log 70 (27 `U-wire-*`, 7 `S-neg-*`, 8 `X-tag-zero-*`) | **0**: fixed by rendering the plan's decode rules (JOURNAL J34) | `93` |
| cross-level byte identity: 3.7 re-encodes every (arm, row) to the 3.12 bytes | 2696 compared, 0 differ | `93` (py3.7, `--compare`) |
| corpus controls: `proj`, `reenc`, `accept` on 5 arms, `noinit` on the 2 ffi arms | each fails on every arm it applies to | `93` |
| chunking (CONTRACT 4): default build puts `C-elemu-512` in 1 chunk of 32-byte groups; the `ffi-chunk256` arm puts it in 64 and `C-leaf-2048` in 512 | pass | `93` |
| disputed rows: `U-map-entry` reads as protobuf's pure-python backend (entry kept), all arms; `X-tag-zero-Empty`, `X-tag-zero-nested-Empty` refused (-2), all arms | reported, excluded | `93` |
| RPC gate (R-D3) through the rendered header | 80 of 80 failure rows aborted, 20 of 20 healthy rows gated, both levels | `94` |
| R-D1 wrapped lengths through the shim | every input `AK_ERR_TRUNCATED`, control decodes, both levels | `95` |
| U1 | unchanged: upb drops the entry, every other reader keeps it | `96` |
| **3.7 source check**: all five translation units against real 3.7.5 headers, `-Werror`; and against 3.9.5, 3.10, 3.11, 3.12, 3.13 headers | pass; the pre-port tree fails on `Py_NewRef`, `PyObject_CallNoArgs`, `PyObject_CallOneArg`, `PyModule_AddObjectRef` | `97` |
| rendered C header vs the cpp slice's | byte-identical (payload set and corpus) | `98` |
| concurrency suite (obligation 12.5) | run once on 3.12 after the port: 0 wrong bytes; not in the gate, log not committed (its scaling columns are timings) | none |

## Crossing counts (R5), per element

Unchanged by the port: log 98 diffs every row of log 91 against the pre-port log 85 and finds
none different. The table below is log 91's (3.12; 3.7 identical). Three edges, never added:
shim -> CPython (counted by the shim), core fwd and core rev (counted by the core).

| payload | shim -> CPython, C ext type (enc / dec) | shim -> CPython, plain and `__slots__` (enc / dec) | core fwd (enc / dec) | core rev (enc / dec) |
|---|---|---|---|---|
| P1.2 M1 | 7.00 / 7.00 | 29.00 / 24.00 | 0.01 / 0.00 | 0.00 / 0.01 |
| P2.2 M2 | 51.67 / 51.67 | 146.68 / 131.35 | 5.02 / 0.00 | 5.00 / 7.00 |
| P3.1 M3 | 5.40 / 3.15 | 13.40 / 9.96 | 0.01 / 0.01 | 0.01 / 0.01 |
| P4.1 M4 | 23.00 / 23.00 | 54.01 / 49.01 | 1.01 / 0.01 | 1.00 / 3.00 |
| P5.1-P5.4 M5 | 3.00 / 3.00 | 7.00 / 8.00 | 1.00 / 1.00 | 0.00 / 1.00 |
| P6.1 M6 | 302.00 / 152.00 | 308.00 / 158.00 | 5.01 / 0.01 | 5.00 / 7.00 |
| P7.1 M7 | 2.00 / 2.00 | 5.33 / 4.33 | 0.50 / 0.17 | 0.33 / 1.17 |

## What the plan does not carry (reported, not decided here)

1. **ABI v1 sections 3-5's fixed vocabulary**: error codes, `ak_str` / `ak_span` / `ak_blob` /
   `ak_uspan`, `AK_STR_DIRECT`, `AK_TOKEN_ROOT`, the context and callback types, the fixed
   exports, the counters. Every C header renderer restates them as fixed text (`cpp_abi`'s is
   the one used).
2. **`plan.lifecycle` names but does not define** the `AK_INIT_*` values, `ak_err`'s layout,
   `ak_log_fn`'s signature; `opts_struct`'s `log` type is prose ("ak_log_fn (nullable)").
3. **Map entry ORDER** on encode: the plan states each entry's own plan, not the order of the
   entries. This backend sorts by key (code point = UTF-8 byte order), the manifest's
   canonical form and what the Rust facade gets from `BTreeMap`.
4. **`presence == "direct"`** has no ENCODE RULE: on the wire it is an implicit-presence
   bytes field (the core tests `len != 0`); the pure-Python codec treats it so.
5. **The 10th varint byte**: the plan refuses an 11th byte but does not say what becomes of
   bits past 64 in the 10th; the core drops them and the pure-Python codec does the same.
6. **Group depth**: the plan says nested groups are limited "likewise" (by
   `recursion_limit`, message depth); the core bounds groups at 100 per skip, independent of
   the message depth. The pure-Python codec follows the core.
7. **Field numbers above 2^29-1**: not stated. The core reads `(key >> 3) as u32` (a number
   >= 2^32 truncates); the pure-Python codec does not truncate. No corpus row reaches it.
8. **Unknown fields inside a map entry** have no bag in any facade; both pure-Python modes drop
   them (`U-map-entry`), as the Rust core-native control does. Retain-mode rows are otherwise
   retained (the retained form is what `py-retain` writes on the `unknown` class).

## Open defects

- (fixed in work unit 8, follow-up) **`mod_traverse`/`mod_clear` dereferenced a NULL module
  state.** Under multi-phase init the state is NULL between creation and exec, and CPython
  before 3.9 can run a GC pass in that window. It showed as a deterministic segfault of
  `rpc_gate.py` on the 3.7 floor (gdb: `mod_traverse` from `PyModule_FromDefAndSpec2`), once
  the variant changed allocation timing. Both now return 0 on a NULL state.
- The campaign smoke logs in `logs/python/campaign/` (5652847) predate the variant's facade
  without `_unknown`. The smoke was not rerun: a campaign run builds another snapshot target
  directory, and the disk is short.
- (fixed in work unit 8) **`AK_LAST_RECLAIMED` was process-global** in the py_capi render (`poc/codec/gen`, not
  mine to edit). With threads decoding, a GIL switch between its store and the read
  can misattribute a decode's figure. `unk_totals()` reads it in C immediately after the
  generated decode returns. On success, one facade attribute store runs in between, and it
  runs no bytecode for the cext facade. The fix is mine: `py_capi.py` is this slice's backend, so the fix is to make it
  per thread there. Deferred on the aggregating session's instruction: it goes in with the
  no-unknown port, once the rust agent's variant lands, not while `poc/codec` is being edited.

| # | where | what |
|---|---|---|
| U1 | the incumbent | upb drops a map entry carrying an unknown field (log 96) |
| mech/ | (retired) | **work unit 1's codec arms and their generator are RETIRED** (FIX-PLAN WP5 step 6, by the rust slice agent under the aggregating session's authorization): `mech/gen/` (generator and generated code), `mech/native/_akcodec.c`, `mech/arms.py`, `mech/bench_codec.py` and `mech/conformance.py` are deleted. Their committed logs (`logs/python/20-conformance.log`, `40-codec-*.log`, `41-codec-all.log`) are HISTORICAL: produced by a generator with its own wire rules, container instrumentation, not reproducible from the tree. What remains in `mech/` is the crossing-mechanism microbenchmark (`_akmech`, the plain C library, PyO3 `abi3-py310`, ctypes, cffi; no wire rule) and `mech/build.sh`'s `shapes_pb2` step, which `run.sh` and `arms.py` still use |
| noinit C4 | `corpus.py` | under the `noinit` control a reject row is "refused" with -10 and counts as refused; only accept rows show the control. Same as the rust harness |
| D1-D12 | this slice | fixed (JOURNAL) |
| **D13** | shim (was `py_binding.py`, now `py_capi.py`) | **fixed**: a packed run longer than 4096 values crossed as several `ak_run_*` calls, each its own LEN record (legal, not canonical, not ABI v1 section 6). Found by the `ffi-chunk256` arm (log 99); now one call per field |
| **D14** | `native/binding.c` | **fixed**: on 3.7/3.8 `PyMODINIT_FUNC` has no default visibility, so under `-fvisibility=hidden` no shim exported `PyInit_*`. Found by the first real 3.7 import |

## What is not measured, or not built

- **Unknown-field retention through the C ABI**: the shim passes NULL for every `ak_unk_f`
  slot and uses the `ak_encode_*` family, so there is no ffi-retain arm. Retain exists in the
  pure-Python codec only.
- **3.8 to 3.11 and 3.13** were not re-gated after the port (no protobuf/grpcio on those
  interpreters here; they were gated on an older commit, log 53). The conditionals switch at
  3.9 and 3.10: 3.12 takes every `>=` branch and 3.7 every `#else` branch. The mixed case
  (3.9: `PyObject_CallNoArgs` but no `Py_NewRef`) is COMPILED against focal's 3.9.5 headers,
  as are 3.10, 3.11 and 3.13 (log 97, all five variants, `-Werror`), but not run.
- **3.7 floor limits**: bionic's interpreter without libssl1.1 (no `ssl`); protobuf 4.24.4 as
  the incumbent there (7.x has no 3.7 build). The floor runs the correctness gate only.
- **Free-threaded CPython**, **decode under threads**, **allocation per operation**,
  **abi3**, **decision 13's borrowed span**, **the pull decode family**, **an encode-side RPC
  arm and the server side**, **streaming, TLS, deadlines, metadata**: not built (as before).
- **C5 for the five large rows** without a projection (`B-P2_5`, `B-P4_1`, `C-elemu-512`,
  `C-leaf-2048`, `C-mixed-100`): checked by C3 re-encode only, named in log 93.
- **Every performance question**, deferred to the campaign (`design/CAMPAIGN.md`). The RPC
  grid's known harness defects (R-C2 to R-C5, R-C9, R-C13) are not fixed here.

## GC in the harnesses (R-F2)

Unchanged: `bench.py` runs its rounds with the collector off (`mech/harness.run`);
`gcbias.py` measures off and on; the gates do not touch it. `57-gc-bias.log`'s closing line is
stale text (JOURNAL J28).

## Next step

1. The owner's campaign run: `./run_campaign.sh --suite gate|calib|codec|rpc --out
   ffi/logs/python/campaign`, without `--smoke`, with the CPU sets exported.
2. Re-render (`gen/generate.py`) and re-gate (`./gate.sh python3.12 build/py37/python3.7`)
   whenever `plan.py`, `c_abi.py` or the core changes. The gate now includes the no-unknown
   variant (steps 100-103); a changed crossing count in either build stops it until the
   committed file in `counts/` is reviewed and replaced.

## Log index

`campaign/`: the WP3 smoke run (instrumentation): gate/ (90-98 at 3.12 and 3.7, the RPC control), calib, codec shapes and unknown, rpc, summary.txt.


**Results now** (correctness, counts, feasibility, defects):

| Log | What it establishes |
|---|---|
| `90-wp5-build.log` | build at 3.12.3 and 3.7.5: R0, R1 + guard, init-guard cores, six shims each, R5, the controls |
| `91-wp5-conformance-py3.12.log`, `-py3.7.log` | R2 both directions, layout at import, crossing counts, `_akffi` |
| `92-wp5-conformance-rpc-shim-py3.12.log`, `-py3.7.log` | the same on `_akffi_rpc` |
| `93-wp5-corpus-py3.12.log`, `-py3.7.log` | the whole corpus, five arms, controls, chunk counts, refusal codes per vector; 3.7 compared byte for byte with 3.12 |
| `94-wp5-rpc-gate-py3.12.log`, `-py3.7.log` | R-D3 through the plan-rendered header |
| `95-wp5-rd1-lenwrap-py3.12.log`, `-py3.7.log` | R-D1 through the shim |
| `96-wp5-u1-py3.12.log`, `-py3.7.log` | U1 |
| `97-wp5-floor-source.log` | 3.7.5 headers, five variants, and the pre-port control |
| `100-wp5s10-conformance-nounk-py3.12.log`, `-py3.7.log` | conformance on the no-unknown variant `_akffi_nounk`, its crossing counts (whole numbers under `abs`) |
| `101-wp5s10-corpus-nounk-py3.12.log`, `-py3.7.log` | the whole corpus through the variant: dropped form on every unknown row that has one, byte identity against the full build's drop arms, the controls, the variant's own controls |
| `102-wp5s10-conformance-rpc-nounk-py3.12.log`, `-py3.7.log` | conformance on `_akffi_rpc_nounk` |
| `103-wp5s10-counts-drop-vs-nounk.log` | whole-number crossing counts of both builds against `counts/`, and their difference (P1.2 decode reverse 8 to 5) |
| `98-wp5-counts-vs-85.log` | crossing counts unchanged by the port; the header is the cpp slice's |
| `99-wp5-corpus-chunk256-before-D13.log` | D13 before the fix (71 rows, all packed) |
| `52-r14-baseline.log` | R14 derived from `Protos/V1` |
| `85-conformance-rpc-shim.log` | the pre-port shim's crossing counts (the reference for 98) |
| `70-corpus-subset.log` | pre-port: 213 rows, pycodec's 42 R-E5 failures (superseded by 93) |
| `83-rpc-gate.log`, `81-rpc-gate-before.log`, `84`, `86`, `87`, `89`, `88`, `82` | work unit 4 (R-D3, R-D1, U1, R-D4 confirmation); superseded where 9x covers them |
| `53-conformance-all-shapes.log`, `54-build-all-shapes.log`, `56-concurrency.log`, `01-environment.log` | older commits and interpreters (3.10-3.13) |

**Instrumentation** (container timings; not quoted): `00-r13-rust-crossing.log`, `10`, `20`,
`30`/`31`, `40`/`41`, `50`/`51`/`60`/`61`, `55-allocator.log`, `57-gc-bias.log`, `62`/`63`,
`80-rpc-grid.log` (predates the R-D3 gate). Log 93's `wall` line is instrumentation too.
