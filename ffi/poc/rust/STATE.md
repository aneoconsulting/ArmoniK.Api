# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | stage 1 done and confirmed against the fixed schema. **Stage 2 done**: all four arms exist, are byte-identical and are timed over M1. Stages 3 and 4 not started |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | MSRV 1.88 declared. **Not verified: no 1.88 toolchain exists in this container, only 1.94.1** |
| **Target** (where the clock runs) | the same, one configuration (README section 5) |
| **Incumbent** (the baseline every ratio is against) | prost 0.14.4, plus tonic 0.14 for stage 4 |

## The question this slice answers

What a host language loses against full Rust, and what the new design costs against `packages/rust` today. This slice is the denominator for every other one.

## Arms, all four built

| arm | what it is | where |
|---|---|---|
| `prost` | prost-build structs, prost's codec | `crates/shapes-prost`, driven from `crates/harness/src/arms.rs` |
| `armonik` | facade types with generated `prost::Message` impls, no conversion layer | `crates/facade/src/generated/prost_impl.rs` |
| `core-native` | the generated core traversal emitted into the host, no boundary (R3's control) | `crates/facade/src/generated/core_native.rs` |
| `core-ffi-rust` | the same traversal through the C ABI, over a real shared-library boundary | `crates/ak-core` (cdylib) + `crates/harness/src/generated/binding.rs` |

## What exists

```
gen/generate.py [--check]   the generator. Imports ffi/schema/emit/shapes.py (R1)
gen/ir.py rustnames.py rust_facade.py rust_build.py rust_core.py rust_abi.py
gen/stage1.sh               stage 1 end to end
gen/stage1_isolate.py       the stage 1 candidate fix, on a scratch copy
gen/stage2.sh               stage 2 end to end: check, conformance, counts, boundary, timings
gen/dump_payloads.py        all 16 payloads to a scratch dir

crates/shapes-prost         protox 0.9 -> prost-build 0.14 over the generated .proto
crates/shapes-values        the value rules of emit/values.py, hand-re-derived
crates/stage1-validate      the stage 1 harness
crates/facade               facade types, the armonik arm, core-native, the payload builder
crates/ak-abi               the C ABI of design/ABI-v1.md. The generated header BOTH sides use
crates/ak-rt                Enc/Dec runtime: varints, learned length widths, counters, arena size
crates/ak-core              the core. **cdylib, not rlib**, see "Open defects" D3
crates/harness              the binding, the arms table, conformance, counts, bench
```

Binaries: `conformance` (byte identity), `counts` (`--features count`), `bench`.
Features: `guard` (on by default, ABI v1 section 5), `count`.

## What is measured

Stage 2 only: M1 (`ResultRaw` in `ListResultsResponse`) over P1.1, P1.2 and P1.3, ASCII.
Ratios to prost, formed inside one process, range across three separate processes
(`ffi/logs/rust/stage2-four-arms-M1.log`):

| payload | direction | armonik | core-native | core-ffi-rust |
|---|---|---|---|---|
| P1.1 | encode | 0.941 - 0.954 | 0.343 - 0.349 | 0.593 - 0.622 |
| P1.1 | decode | 0.889 - 0.939 | 0.751 - 0.861 | 0.779 - 0.889 |
| P1.2 | encode | 0.967 - 1.032 | 0.425 - 0.438 | 0.706 - 0.716 |
| P1.2 | decode | 0.896 - 0.935 | 0.832 - 0.861 | 0.848 - 0.879 |
| P1.3 | encode | 1.173 - 1.306 | 0.468 - 0.475 | 1.162 - 1.295 |
| P1.3 | decode | 0.965 - 1.047 | 0.726 - 0.838 | 1.313 - 1.393 |

- **One crossing costs 1.8 ns** in this configuration (forward, and forward-plus-reverse),
  measured in the same process and the same build as the arms.
- **Crossings**: 9 to encode a thousand rows, 6 to decode them. 3 and 3 on P1.1.
- **The accessor guard is not measurable** on Rust at this payload set.
- **UTF-8 validation in the transcoder costs 25 to 30 percent of an encode** (ABI v1 open
  decision 3).
- **ABI v1 open decision 5, provisionally**: one prefix move on a cold context for P1.2,
  zero warm, zero transcoder grows. M1 is the easy case; P2.4 settles it.

## Next step

**Stage 3**: widen to the full shape and payload set of `design/SHAPES.md`. In generator
order, because each is a backend gap with a `NotImplementedError` already raised at the right
place:

1. `ListTasksDetailedResponse` (M2) as a root: repeated string, map, packed enum, nesting to
   depth 6, and the non-leaf element type, which the batching predicate must refuse. Payloads
   P2.1 to P2.5. **P2.4 is the one that answers ABI v1 open decision 5.**
2. M3 (`Probe`): oneof including the payload-free member, and explicit presence. The builder
   backend raises `NotImplementedError` on a oneof today, deliberately.
3. M4 (the adapter site), M5 (bulk bytes, including the direct-argument path of ABI v1
   section 8), M6 (packed scalars, a control), M7 (`DualResponse`, decode only: no canonical
   writer can produce it).

Then **stage 4**: the RPC arm over P2.2 against tonic, 1/8/16 in flight, crossings per RPC.

## Correctness

- All four arms byte-identical to `ffi/schema/generated/manifest.json` on every payload they
  cover, and each decodes its own output back to an equal value. `gen/stage2.sh` step 2.
- The manifest is validated (stage 1), so it is the oracle; no arm is checked against another
  arm alone.
- `prost` and `armonik` are two independent encoders over two independently built object
  graphs. `core-native` and `core-ffi-rust` **share the codec** and so validate only the
  binding, which the log states.
- The boundary is checked structurally, not assumed: `gen/stage2.sh` step 4 shows the ABI
  entry points as undefined dynamic imports.

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| D1 | `ffi/schema/emit/payloads.py` | implicit-presence zero leaf written | **fixed by the aggregating session in `07d3e05`**, re-verified here at 16/16 |
| D2 | this container | no rustc 1.88, so the declared MSRV is unverified | open, cannot be fixed here. Stated on every log |
| D3 | `crates/ak-core` | an rlib let rustc inline every ABI entry point into the host, and the boundary-call counters still incremented because the counting code was inlined too | **fixed**: cdylib plus a dynamic-link build script, and `gen/stage2.sh` step 4 now checks it every run |
| D4 | `gen/rust_build.py` | `all_absent` was short-circuited by the `half_absent` rule for even-tag Timestamps | **fixed** and swept: one `absent_expr`/`zero_expr` pair for every kind |
| D5 | `gen/rust_facade.py` | the enum guard ran `to_i32()` twice, and `is_some()` was followed by `as_ref().unwrap()` | **fixed** and swept: guard and value emitted as one statement per field |
| D6 | `gen/rust_abi.py` | a zero-length span went through the lossy-UTF-8 path in the generated accessor | **fixed**: empty fast path in `s_of` and `b_of` |
| D7 | `crates/ak-core/src/lib.rs` | `ak_fail` casts its context to `EncCtxImpl` unconditionally; a decode-side failure would corrupt a `DecCtxImpl` | **open**. Not reachable today: nothing on the decode path calls `ak_fail`, and the decode guard swallows a panic instead. Fix with the decode error channel in stage 3 |

## What is not measured

- **Shapes**: only M1. No oneof, no explicit presence, no map, no packed, no repeated string,
  no adapter site, no bulk bytes, no nesting past depth 2, no non-leaf element type, no
  interleaved repeated fields. That is M2 to M7, all of stage 3.
- **Payloads**: only P1.1, P1.2, P1.3.
- **Unknown fields on the wire.** The corpus (W8) does not exist, so the skip path is written
  (`Dec::skip`) and never executed by anything measured.
- **Content sets**: ASCII only. `latin1` and `wide` are where a narrowing transcoder has work
  to do, and they are untouched. Every string figure here is half a number in SHAPES.md's
  sense.
- **The pull decode family** (ABI v1 section 7.1). This slice built the push family only,
  which is the right default for a host whose reverse call costs 1.8 ns; the claim that pull
  would be no better here is an argument, not a measurement.
- **The direct-argument path for bulk bytes** (ABI v1 section 8). Needs M5.
- **The RPC half entirely**: tonic, concurrency, streaming, TLS, the server seam.
- **Concurrency of the codec**: one thread throughout. The learned-width table is per context
  and never exercised by two threads, which is the case ABI v1 section 6 says a global table
  fails at, and obligation 12.5's concurrency suite does not exist.
- **The floor**: no 1.88 toolchain, so "builds on the MSRV" is a declaration and not a run.
- **Static linking of the core**, which a C or C++ host would use and which is cheaper than
  the shared library measured here.
- **Allocation and memory**: nothing counts allocations or peak footprint in any arm.
- **`packages/rust`'s own `armonik` crate.** The `armonik` arm reproduces its *pattern*; it
  does not use `armonik-macros`, whose expansions reference `armonik`-internal paths and read
  a descriptor built from `Protos/V1`. So the `armonik` column prices this generator's
  `prost::Message` emitter, not that crate's.

## Slice-specific notes

- Reads `packages/rust`. Does not edit it.
- This slice's `core-ffi-rust` number is what every other slice subtracts to separate
  interface cost from runtime tax, so it is the one arm that must exist before the managed
  slices are read. **It is now available**, with the crossing priced at 1.8 ns.
- `ffi/.gitignore` (added by the aggregating session) re-includes `logs/**` and
  `poc/*/gen/**`; `poc/rust/.gitignore` is now only `target/`.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `ffi/logs/rust/stage1-manifest-vs-prost.log` | rustc 1.94.1, prost 0.14.4, protox 0.9.1, 4 vCPU Xeon 2.80GHz; nothing timed | the original disagreement: 8 of 16 payloads |
| `ffi/logs/rust/stage1-isolate-zero-leaf.log` | as above | one change accounts for all eight |
| `ffi/logs/rust/stage1-second-encoder.log` | as above, plus prost-reflect 0.16.5 | the rule is protobuf's, not prost's; the corrected sizes and hashes |
| `ffi/logs/rust/stage1-manifest-vs-prost-after-fix.log` | as above, schema at `07d3e05` | 16 of 16. The manifest is this slice's oracle |
| `ffi/logs/rust/stage2-four-arms-M1.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on (section 6 off), ASCII, 4 shared vCPUs | byte identity across four arms; crossing counts; the boundary is a real dynamic import; the crossing costs 1.8 ns; the ratio table above; the guard is free; UTF-8 validation costs 25-30 percent of an encode |
