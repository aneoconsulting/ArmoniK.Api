# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only thing
that survives the end of a session. It says what exists and what was checked; it
does not say what a binding should choose (`CLAUDE.md`, roles).

**Phase** (README 1.1): setup and design. A container timing is instrumentation. This
file carries **no timing figure**. Timing logs exist and are listed at the foot as
instrumentation; the numbers in them are for proving a harness runs, not for quoting.

| | |
|---|---|
| **Status** | Work unit 4 (the 2026-09-24 review) done for this slice: **R-D3 fixed** (the RPC arm is gated), **R-D4 confirmed, not fixed** (waits for WP5), **R-F2 and R-F3 fixed** in this file. Waiting to re-gate against the fixed shared core |
| **Blocked on** | (1) the rust agent's shared-core fix (WP4 item 1): re-gate when it lands; (2) WP5, the one generator, for the 3.7 floor |
| **Floor** (owner D1: 3.7) | **Does not build.** The generated shim uses 3.9/3.10 C-API calls with no `PY_VERSION_HEX` conditionals (R-D4, `82-floor-3.7.log`). **Not obtainable here either**: the apt index lists 3.7.17-1+noble2 but the egress proxy refuses the deadsnakes PPA, python.org and github (403) |
| **Target** (owner D1: CPython 3.12) | Built and gated on **3.12.3** this session (`85`, `87`). Earlier gates on 3.10, 3.11, 3.12, 3.13 (`53`, older commit). grpcio 1.84.0, protobuf 7.36.2 on upb |
| **Incumbent** (R14) | protobuf on **upb**, through gRPC's generated marshaller path: `Message.SerializeToString` / `Message.FromString`. Derived from `Protos/V1/results_service.proto` by `verify_r14.py` (`52-r14-baseline.log`) |
| **Core** | `poc/codec`, the one core (R0), never copied. This session built it from a `git archive 8864e4d` extraction through `AK_UPSTREAM`, because the rust and cpp agents were editing `poc/codec` and `poc/cpp/gen` at the same time (see "Building" below) |

## What exists

```
gen/generate.py     the slice's generator front end. Imports poc/codec/gen/ir.py and the cpp
                    slice's cpp_header.py READ-ONLY (from AK_UPSTREAM when set). Writes gen/out
gen/walk.py         the field walker; a shape outside SCOPE raises
gen/py_facade.py    the facade (plain, __slots__), with <oneof>_case
gen/py_codec.py     the pure-Python codec (R3's no-boundary control)
gen/py_binding.py   the composed arm's CPython shim: 3 facade backends x 2 directions
gen/py_shim.py      work unit 1's no-core C encoder
gen/py_values.py    facade objects carrying the manifest's values
gen/out/            emitted and committed, ak_abi.h included
native/binding.c    the module wrapper, and ABI v1 section 9 (RPC) behind -DAK_RPC
build.sh            R0 check, R1 check, three core builds, three shims, the R5 boundary proof
run.sh              the whole slice end to end (mech/build.sh for shapes_pb2 first)
conformance.py      R2 byte identity both directions, layout facts, crossing counts (both halves)
corpus.py           W8, the rows this scope can root
concurrency.py      ABI v1 obligation 12.5 (encode under threads, byte-checked)
rpc.py              the RPC grid harness (cells A, B, C; three deliveries); GATED since R-D3
rpc_gate.py         R-D3: what every RPC cell does when the RPC fails. Prints no timings
u1_map_unknown.py   U1: one map entry with an unknown field, four readers. Prints no timings
allocator.py, gcbias.py, bench.py, arms.py, verify_r14.py, mech/   harnesses (timings: instrumentation)
```

Three builds of one `libak_core.so`, three shims, and a process loads exactly one:
`_akffi` (plain core), `_akffi_count` (counting core, `AK_USE_COUNT=1`), `_akffi_rpc`
(rpc-feature core, `AK_FFI_MODULE=_akffi_rpc`). The first core loaded satisfies the others
by soname, which is why the choice is an environment variable read before `arms` imports
and why `conformance.py` now prints the core the process actually mapped.

### Building

`AK_UPSTREAM=<ffi tree> ./build.sh <python>...`. `AK_UPSTREAM` is where the shared inputs
are read from (core source, IR, header renderer, schema emitter); default is this
checkout. Point it at `git archive <commit> ffi/poc/codec ffi/poc/cpp/gen ffi/schema`
extracted somewhere, with the commit written to `<extraction>/COMMIT`, to build against a
named commit while others edit. Cargo targets go to `poc/python/build/cargo/{plain,count,rpc}`
(git-ignored); nothing in this slice writes under `poc/codec`. `mech/build.sh <python>`
writes `mech/build/pb2/shapes_pb2.py`, which the incumbent arm needs; `run.sh` runs it.

## What was checked, and the log that carries it

| check | result | log |
|---|---|---|
| R2 encode, byte identity against `schema/generated/manifest.json`, 16 payloads, every arm | pass (P7.1 as a permutation, as `design/SHAPES.md` allows; upb's map order checked as a legal alternative form) | `85` (3.12, both `_akffi` and `_akffi_rpc`), `53` (3.10-3.13, older commit) |
| R2 decode, re-encode identity and field identity against upb's descriptor, 16 payloads | pass, absent paths P1.3 and P2.5 included | `85`, `53` |
| layout facts and ABI version at import (ABI v1 section 10) | pass | `85`, `53` |
| **`_akffi_rpc` through the same gate** (R-D3) | pass, and passed before the binding change too: the rpc shim's codec was right and unchecked | `84` (before), `85` |
| R5 boundary proof from the artifact, **all three shims** (R-D3 added `_akffi_rpc`) | pass; `_akffi_rpc` imports all 6 section 9 entry points it binds and resolves to the rpc core; must-fail control refused | `87` |
| **RPC arm under injected failure** (R-D3) | before: 68 of 80 failure rows produced a figure. After: 80 of 80 aborted, 20 of 20 healthy rows gated | `81` (before), `83` (after) |
| `rpc.py` executes end to end with the gate | exit 0, 52 gate lines, 0 aborted; every timing row deleted from the log | `86` |
| concurrency, ABI v1 obligation 12.5: encode under 1/2/4 threads, per-thread and shared facades, P1.2 and P2.2, every encode compared to the arm's own reference | 0 wrong bytes in every row | `56` |
| W8 corpus, 126 of 336 rows (the rows this scope roots) | C1 126/126, C2 123/123, C3 126/126, C4 2/2, every arm | `70` (older commit; **not re-run this session**) |
| R0 one core | pass | `87` |

Not re-run this session: `53` on 3.10/3.11/3.13 (no protobuf/grpcio installed on those
interpreters in this container), the corpus, the concurrency suite.

## Crossing counts (R5), per element, and which edge each number is

From the counting build, `logs/python/85-conformance-rpc-shim.log` (3.12, this session),
identical row for row to the 3.11 block of `53-conformance-all-shapes.log`. "Per element"
divides by the manifest's element count (P2.x: 500 `TaskDetailed`; P5.x: 1, the message).

Three edges, counted by two parties, and **they must never be added together**:

- **shim -> CPython**: calls the generated shim makes into the CPython C-API (value reads,
  list element access, `PyObject_GetAttr/SetAttr`, calls into Python). Counted by the shim.
- **core fwd**: host -> core calls across the ABI. Counted by the core.
- **core rev**: core -> host callbacks across the ABI. Counted by the core.

| payload | shim -> CPython, C ext type (enc / dec) | shim -> CPython, plain and `__slots__` (enc / dec) | core fwd (enc / dec) | core rev (enc / dec) |
|---|---|---|---|---|
| P1.2 M1 | 7.00 / 7.00 | 29.00 / 24.00 | 0.01 / 0.00 | 0.00 / 0.01 |
| **P2.2 M2** | **51.67 / 51.67** | **146.68 / 131.35** | 5.02 / 0.00 | 5.00 / 7.00 |
| P3.1 M3 | 5.40 / 3.15 | 13.40 / 9.96 | 0.01 / 0.01 | 0.01 / 0.01 |
| P4.1 M4 | 23.00 / 23.00 | 54.01 / 49.01 | 1.01 / 0.01 | 1.00 / 3.00 |
| P5.1-P5.4 M5 (36 B to 4 MB) | 3.00 / 3.00 | 7.00 / 8.00 | 1.00 / 1.00 | 0.00 / 1.00 |
| P6.1 M6 | 302.00 / 152.00 | 308.00 / 158.00 | 5.01 / 0.01 | 5.00 / 7.00 |
| P7.1 M7 | 2.00 / 2.00 | 5.33 / 4.33 | 0.50 / 0.17 | 0.33 / 1.17 |

Every payload, every backend (P1.1, P1.3, P2.1, P2.3 to P2.5 and the two `pyacc`
backends included) is in the log.

**R-F3, reconciled.** The previous version of this file gave P2.2 as "10.02 / 7.00" in the
shim -> CPython column and "32" for the Python storages. **10.02 and 7.00 are core
crossings, not shim crossings**: 10.02 = core fwd 5.02 + core rev 5.00 on encode, and 7.00 =
core rev on decode (fwd 0.00), per `TaskDetailed` -- the figure JOURNAL J22 matched against
the rust slice. They had been written into the wrong column. The shim -> CPython figures
for the same payload are 51.67 / 51.67 (C ext type) and 146.68 / 131.35 (plain and
`__slots__`), which is what the log says. **"32" has no log behind it** and is deleted.

Facts these counts carry, independent of any machine:

- A packed run (P6.1) crosses the ABI once per field (`ak_run_*`) and the shim then reads
  every Python int individually: 5.01 core calls against 302 shim calls per element.
- M5 is constant in the payload size: 3 shim calls and 1 core call per message from 36 B to
  4 MB, because section 8's direct argument hands the bytes over beside the group.
- A oneof (P3.1) costs a discriminant read plus one member: fewer shim calls than M1.
- The C extension facade removes every `GetAttr/SetAttr`; the two Python storages count
  identically.

## Facts that are not timings

- **U1, an incumbent defect**: protobuf 7.36.2 on upb drops an entire map entry that carries
  an unknown field; the same version's pure-Python backend keeps it, and so do this slice's
  core and its pure-Python control. One entry, one unknown varint inside it, against a
  control without it (`88-u1-map-unknown.log`, `u1_map_unknown.py`; until this session the
  isolation had no committed log). Found through the corpus's `U-map-entry` (`70`).
- **ABI v1 decision 11 in this slice: unknown fields are dropped.** The shim passes NULL for
  every `ak_unk_f` slot; the drop is the binding's choice, not a limit of the ABI. The
  retain mode (owner D4) is not built here.
- **M3 to M7 needed no ABI extension**: `<oneof>_case`, the presence word, `ak_run_*` and
  section 8's direct argument were already there (JOURNAL J27).
- **grpcio 1.84.0's flow control, read from the C core's own tracing** (`80-rpc-grid.log`,
  the flow-control section): static stream window 65,535; `grpc.http2.lookahead_bytes` is a
  floor, not a cap, while BDP probing is on; setting it does not turn probing off; there is
  no channel argument for the connection window.
- **Section 9's three deliveries all work from Python**: the queue's drainer is a Python
  thread that drops the GIL in `ak_queue_next`; the callback arrives on a tokio worker and
  takes the GIL with `PyGILState_Ensure`. Both now fail loudly on a failed call (`83`).

## Open defects

| # | where | what |
|---|---|---|
| **R-D4** | `gen/py_binding.py` -> `gen/out/binding.c`; `native/binding.c`; `mech/pyo3` | 3.7 floor does not compile: `Py_NewRef` x164, `PyObject_CallNoArgs` x114, `PyObject_CallOneArg` x133 in the generated shim; `Py_NewRef` x1 and `PyModule_AddObjectRef` x1 in the **hand-written** `native/binding.c` (not generated, so WP5 will not fix it); PyO3 arm is `abi3-py310`. Waits for WP5 |
| R-D2 follow-up | `native/binding.c` | restates `ak_client_opts` by hand (6 fields, currently matching the core). When the cpp agent's generated `ak_client_opts` lands in `ak_abi.h`, switch to it |
| U1 | the incumbent | upb drops a map entry carrying an unknown field (above, `88`) |
| D1-D10 | this slice | all fixed (JOURNAL) |
| `57-gc-bias.log` closing line | log text | it says `bench.py` "no longer" disables GC; `bench.py` does disable it for its rounds (JOURNAL J28). The line was printed by `gcbias.py` during the withdrawn first fix; `gcbias.py`'s text is corrected, the committed log is left as it was produced |

## What is not measured, or not built

- **The floor (3.7)**: not buildable (R-D4) and not installable here.
- **Free-threaded CPython**: not installable in this container.
- **Decode under threads**. The concurrency suite covers encode only, at 1, 2 and 4 threads.
- **Unknown-field retention** (owner D4): the core has `ak_ufix_*` groups and `ak_unk_f`
  slots; this shim passes NULL, so only the drop mode exists here.
- **An encode-side RPC arm and the server side**: every RPC cell decodes at the client
  against a server that returns pre-serialised bytes (FIX-PLAN R-A9, R-C8).
- **Streaming, TLS, deadlines, metadata** in the RPC arm (owner position 3: not required).
- **Allocation per operation**, in any arm.
- **The corpus beyond this slice's roots**: 210 of 336 rows, and the WP4 item 2 vectors
  once the corpus agent lands them.
- **abi3 on the composed arm**; the shim is built full-API only.
- **Decision 13's borrowed span** and **the pull decode family**: not built.
- **Every performance question.** Deferred to the campaign (`design/CAMPAIGN.md`). The RPC
  grid's known harness defects (R-C2 to R-C5, R-C9, R-C13) are not fixed here; R-D3 only
  makes a failed call impossible to time.

## GC in the harnesses (R-F2, made consistent with the code)

- `bench.py` runs its rounds through `mech/harness.run`, whose default is
  `gc_enabled=False`: **the collector is off** while a bench ratio is formed.
- `gcbias.py` measures the same arms with the collector off and on, one payload and one
  direction at a time; that is where the collector's cost is shown (`57`, instrumentation).
- `concurrency.py`, `rpc.py`, `rpc_gate.py` and `allocator.py` do not touch the collector.
- `57-gc-bias.log`'s last line contradicts the first bullet; see open defects.

## Log index

**Results now** (correctness, counts, feasibility, defects):

| Log | What it establishes |
|---|---|
| `01-environment.log` | interpreters present, no free-threaded build, 3.7 apt-listed, which incumbent versions 3.7 could reach |
| `52-r14-baseline.log` | R14 derived from `Protos/V1` |
| `53-conformance-all-shapes.log` | R2 both directions, 16 payloads, 3.10-3.13, older commit; R5 counts, both halves |
| `54-build-all-shapes.log` | build for 3.10-3.13, older commit; R0; R5 for two of the three shims |
| `56-concurrency.log` | obligation 12.5: 0 wrong bytes (its scaling columns are instrumentation) |
| `70-corpus-subset.log` | W8, 126 of 336, D7 re-gated, the `U-map-entry` row that led to U1 |
| `81-rpc-gate-before.log` | **R-D3 before**: failed RPCs timed as successes, 68 of 80 rows |
| `82-floor-3.7.log` | **R-D4**: the post-3.7 C-API calls, per file; 3.7 not fetchable here |
| `83-rpc-gate.log` | **R-D3 after**: 80 of 80 failure rows aborted, 20 of 20 healthy rows gated; binding returns None on failure |
| `84-conformance-rpc-shim-before.log` | `_akffi_rpc` gated for the first time, pre-fix binary: pass |
| `85-conformance-rpc-shim.log` | 3.12, `_akffi_rpc` and `_akffi`, both pass; the crossing table above |
| `86-rpc-smoke-gated.log` | `rpc.py` runs gated end to end; timing rows deleted |
| `87-build-py3.12.log` | 3.12 build at 8864e4d; R5 over all three shims with the must-fail control |
| `88-u1-map-unknown.log` | **U1**: upb returns `{}` for a map entry carrying an unknown field; protobuf's python backend, this slice's core and its pure-Python control return `{'k': 'v'}` |

**Instrumentation** (container timings; not quoted, re-measured in the campaign):
`00-r13-rust-crossing.log`, `10-build.log`, `20-conformance.log`, `30`/`31` (mechanism),
`40`/`41` (codec, work unit 1), `50`/`51`/`60`/`61` (superseded), `55-allocator.log`,
`57-gc-bias.log` (closing line stale, above), `62`/`63` (all-shapes bench),
`80-rpc-grid.log` (predates the R-D3 gate: its core-transport rows were taken by a harness
that could not tell a failed call from a successful one; no failure is known to have
occurred in it, but nothing in that harness would have shown one).

## Next step

1. **Re-gate against the fixed shared core** when told it has landed: rebuild with
   `AK_UPSTREAM` at the new commit (or unset, once the tree is quiet), run
   `conformance.py` on both shims, `rpc_gate.py`, `corpus.py`, and commit the logs.
2. When WP5 lands the shared shim generator: port this backend, emit the `PY_VERSION_HEX`
   conditionals, hand-fix `native/binding.c`'s two 3.10 calls, and build on 3.7 wherever a
   3.7 can be obtained (not this container).
3. When the cpp agent's generated `ak_client_opts` lands in `ak_abi.h`: use it in
   `native/binding.c` instead of the hand restatement.
4. WP3: conform `rpc.py`, `bench.py` and `concurrency.py` to `design/CAMPAIGN.md` once it
   exists.
