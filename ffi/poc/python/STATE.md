# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It says what exists and what
was checked; it does not say what a binding should choose (`CLAUDE.md`, roles).

**Phase** (README 1.1): setup and design. This file carries **no timing figure**. Timing logs
are listed at the foot as instrumentation.

| | |
|---|---|
| **Status** | Work unit 5 done: **FIX-PLAN WP5 step 5**, the Python backend on the shared plan. Every generated file of this slice is rendered by `poc/codec/gen` from a plan; `poc/python/gen/` holds build glue only. **R-D4 fixed** (and one more floor defect, D14, found by building on real 3.7), **R-E5 fixed** (pycodec's 42 corpus failures are 0), **R-G4 fixed**, **R-G7 fixed** (every core built WITH `init-guard`), **R-G8 holds** in the pure-Python codec. D13 found and fixed |
| **Floor** (owner D1: 3.7) | **Builds and passes every gate on CPython 3.7.5** (Ubuntu 18.04's packages from archive.ubuntu.com, `fetch_py37.sh`): logs 90-97 |
| **Target** (owner D1: 3.12) | Built and gated on **3.12.3**: logs 90-96, 98. grpcio 1.84.0, protobuf 7.36.2 (upb) |
| **Incumbent** (R14) | protobuf on **upb** through gRPC's generated marshaller path (`SerializeToString` / `FromString`), `verify_r14.py` (log 52). On the 3.7 floor: protobuf 4.24.4 (upb), for the conformance gate only |
| **Core** | `poc/codec`, the one core (R0), this checkout (commit in each log header; the `AK_UPSTREAM` snapshot mechanism is retired). Four builds, all `init-guard`: plain, `count`, `rpc`, `corpus` |

## What exists

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

| # | where | what |
|---|---|---|
| U1 | the incumbent | upb drops a map entry carrying an unknown field (log 96) |
| mech/ | `mech/gen/generate.py`, `mech/pyo3` | **work unit 1's frozen M1 microbenchmark still has its own generator with wire rules** (a pure-Python encoder and the no-core `_akcodec.c`) and a PyO3 arm built `abi3-py310`. Not ported and not in any gate; its figures were container instrumentation. A slice `gen/` with wire rules is a defect by CLAUDE.md; retiring `mech/`'s codec arms is the owner's/aggregator's call |
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

1. When the aggregating session rules on the plan gaps above, re-render (`gen/generate.py`)
   and re-run `./gate.sh python3.12 build/py37/python3.7`.
2. Whenever the shared core, `plan.py`, `cpp_abi.py` or `cpp_layout.py` change: `./gate.sh`
   (the build's `--check` reports a stale tree first).
3. Retention through the C ABI (`ak_uencode_*`, `ak_unk_f`), if the owner's D4 needs a Python
   ffi-retain column.
4. WP3: conform `rpc.py`, `bench.py`, `concurrency.py` to `design/CAMPAIGN.md`.

## Log index

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
