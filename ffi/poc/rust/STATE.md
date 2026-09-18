# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | stages 1 and 2 done. **Stage 3 parts 1 to 3 done**: M2 with decision 5 answered, the content-set pass with decision 3 reframed, and M3 with the oneof, explicit presence and the unknown-field vectors. M4 to M7 and stage 4 not started |
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

M1 over P1.1 to P1.3 (`stage2-four-arms-M1.log`) and M2 over P2.1 to P2.5
(`stage3-M2.log`), ASCII only. Ratios to prost, formed inside one process, range across three
separate processes.

| payload | direction | armonik | core-native | core-ffi-rust |
|---|---|---|---|---|
| P1.1 | encode | 0.941 - 0.954 | 0.343 - 0.349 | 0.593 - 0.622 |
| P1.2 | encode | 0.967 - 1.032 | 0.425 - 0.438 | 0.706 - 0.716 |
| P1.3 | encode | 1.173 - 1.306 | 0.468 - 0.475 | 1.162 - 1.295 |
| P1.1 | decode | 0.889 - 0.939 | 0.751 - 0.861 | 0.779 - 0.889 |
| P1.2 | decode | 0.896 - 0.935 | 0.832 - 0.861 | 0.848 - 0.879 |
| P1.3 | decode | 0.965 - 1.047 | 0.726 - 0.838 | 1.313 - 1.393 |
| P2.1 | encode | 0.697 - 0.970 | 0.342 - 0.484 | 0.704 - 0.981 |
| P2.2 | encode | 0.991 - 1.001 | 0.493 - 0.517 | 0.833 - 0.856 |
| P2.3 | encode | 0.953 - 1.000 | 0.476 - 0.508 | 0.876 - 0.924 |
| P2.4 | encode | 0.992 - 1.000 | 0.565 - 0.570 | 1.026 - 1.035 |
| P2.5 | encode | 0.978 - 1.014 | 0.463 - 0.476 | 0.893 - 0.901 |
| P2.1 | decode | 0.961 - 0.980 | 0.878 - 1.028 | 1.081 - 1.209 |
| P2.2 | decode | 0.970 - 0.974 | 0.894 - 0.961 | 0.950 - 1.015 |
| P2.3 | decode | 1.017 - 1.017 | 0.932 - 1.068 | 0.819 - 0.953 |
| P2.4 | decode | 0.967 - 0.970 | 0.897 - 1.067 | 0.809 - 0.981 |
| P2.5 | decode | 0.995 - 1.010 | 0.919 - 0.992 | 0.944 - 1.033 |
| P3.1 | encode | 0.966 - 0.975 | 0.406 - 0.410 | 0.846 - 0.853 |
| P3.1 | decode | 0.961 - 0.968 | 0.811 - 0.824 | 0.834 - 0.838 |

- **One crossing costs 1.8 ns** here (forward, and forward-plus-reverse), measured in the
  same process and build as the arms.
- **Crossings**: M1's element type is a leaf, so 9 to encode a thousand rows and 6 to decode
  them. M2's is not: **7.004 per task on decode**, exactly what ABI v1 7.2 predicts, and
  **10.02 per task on encode**, which nothing had predicted.
- **Encode survives the harder shape**: the core is 0.46 to 0.57 of prost natively and 0.83
  to 0.92 through the C ABI on every uniform M2 payload.
- **Decode converges to parity in proportion to host-side CONTAINER construction per
  element**, not to bytes or strings. P3.1 (flat, 5 fields) 0.81; P1.2 (6 blobs, 2 optional
  messages) 0.83; P2.2 (4 `Vec<String>`, a `BTreeMap`, a 27-field struct) 0.89 to 0.96. A map
  insert and four vector growths are work every arm does identically, so the denser the
  element's container graph the smaller the share of decode any codec owns. This supersedes
  the looser "decode is allocation-bound" from part 1, which attributed it to the wrong thing.
- **The oneof and explicit presence cost no crossings at all**: 3 for 200 elements in both
  directions. Both ride in the group.
- **The accessor guard is not measurable** on Rust, on M1 or on M2, and M2 makes 7 to 10
  reverse calls per element.
- **UTF-8 validation on non-ASCII content is the largest single effect in the slice.** The
  scalar validator costs 2.2 to 3.0 times its own ASCII cost and turns a 0.72 to 0.81 win
  against prost into a 2.0 to 2.6 loss. A SIMD validator with the **same contract** removes
  half to two thirds of that (1.27 to 1.50 of prost). ABI v1 open decision 3 is therefore
  less about validate-against-trust than about which validator.
- **The content set changes no decode verdict.** Every arm validates on decode, so all three
  sets scale all arms together and the ratios move by less than the run-to-run spread.
- **ABI v1 open decision 5 is answered.** Zero warm misses on every uniform payload; on P2.4,
  one miss per element moving 980,938 of 981,222 bytes. Isolated with two added arms whose
  mean is P2.4 exactly, and with prost carried as the floor: the mechanism costs about
  **1 to 3 percentage points of an encode** on the payload built to defeat it, and nothing
  on any uniform one.

## Next step

**Stage 3, parts 2 to 5.** Each is a backend gap with a `NotImplementedError` raised at the
right place, so the generator says what it refuses:

1. **M4**, the adapter site: one facade type with two wire forms. The only `with` adapter in
   the Rust crate, and the shape only a byte corpus catches.
2. **M5**, bulk bytes, including the direct-argument path of ABI v1 section 8.
3. **M6**: packed scalars (**CONTROL**) and the packed enum `945d3cd1` added (**not** a
   control: it is the one packed shape the real schema has). The two must be labelled
   differently in the same table.
4. **M7** (`DualResponse`), **CONTROL**, decode only: no canonical writer can produce it.

Then **stage 4**: the RPC arm over P2.2 against tonic, 1/8/16 in flight, crossings per RPC.

Done as part 2 and no longer pending: the `latin1` and `wide` content sets.

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
| D7 | `crates/ak-core/src/lib.rs` | `ak_fail` cast its context to `EncCtxImpl` unconditionally | **closed with M3**, as agreed: both contexts begin with a `CtxHeader { kind, err }`, the decode guard reports through `ak_fail`, and the decode entry point returns it |
| D8 | `gen/rust_abi.py` | the emitted arm for a singular message child read its length prefix from the root reader instead of the reader at its own depth | **fixed**. Only reachable at depth two or more, so M1 could not see it |
| D10 | `crates/stage1-validate/src/build.rs` | the hand-written prost builder did not know M6's new packed enum field | **fixed**, and P6.1 re-validated against prost at 123,354 bytes, with the log to show it: `stage1-manifest-vs-prost-packed-enum.log` |
| D12 | `gen/rust_core.py`, `gen/rust_abi.py` | every backend iterated `Message.plain`, which excludes oneof members, so adding `ListProbeResponse` produced a complete-looking codec that ignored `Probe.body` and said nothing | **fixed**: both walkers call `b.oneof(...)` explicitly, so a backend that cannot do the shape raises rather than skipping it |
| D11 | this slice's reporting | P6.1's re-validation was reported with no log behind it, against a stale log that still showed the old size | **fixed**. Every schema change now re-runs `gen/stage1.sh` into a dated log before the result is quoted |
| D9 | `gen/rust_abi.py` | an element run did not restore the codec's open-field state, so the second and later chunks read whatever the last element left behind. It cost 448 length-prefix misses in 500 elements, and it reads `open_tag` too, so a host that chunks would write later chunks under the inner field's tag | **fixed**: every element and run entry point saves and restores the open state, and `gen/stage3.sh` step 3 carries a regression for it. **The payload set could not have caught it**: byte identity passed only because `ListTasksDetailedResponse.tasks` and `TaskOptions.options` are both tag 1 |

## What is not measured

- **Shapes**: M1, M2 and M3. Still missing: the adapter site, bulk bytes, packed scalars and
  interleaved repeated fields. That is M4 to M7.
- **An unknown oneof `body_case` from a host generated against a newer descriptor.** The
  codec refuses it with `AK_ERR_ABI`; nothing in this build can produce one, so the path is
  built and not exercised.
- **A union group layout for a oneof**, which would be smaller than the flat one built here.
  Recorded as an alternative, not measured.
- **Two shapes `design/SHAPES.md` claims are covered and are not, reported to the aggregating
  session**: a packed repeated ENUM (nothing in `shapes.json` has one; M2 has no packed field
  at all and M6's are int64/double/int32/bool), and nesting to depth 6 (the maximum static
  depth over the whole description is 3).
- **Payloads**: P1.1 to P1.3 and P2.1 to P2.5. P3.1, P4.1, P5.1 to P5.4, P6.1 and P7.1 are
  not built.
- **The pull decode family** and **the unbatched element form** on decode: only the push
  family and the run form are built.
- **Unknown fields on the wire.** The corpus (W8) does not exist, so the skip path is written
  (`Dec::skip`) and never executed by anything measured.
- **Content sets**: `latin1` and `wide` are measured on P1.2 and P2.2 only, encode and
  decode. Not on the other payloads, and there is no manifest oracle for them (byte identity
  against the prost arm instead). The unpaired-surrogate case of README section 10 item 4 is
  **unreachable from this slice at all**: a Rust `String` cannot hold one, so the transcode
  pair's disagreement cannot be produced here.
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
| `ffi/logs/rust/stage1-manifest-vs-prost-after-fix.log` | as above, **schema at `07d3e05`** | 16 of 16 after the zero-leaf fix. **Its P6.1 row (116,954 B) is superseded by the log below**; every other row still stands |
| `ffi/logs/rust/stage1-manifest-vs-prost-packed-enum.log` | as above, **schema at `945d3cd1`** | 16 of 16 including P6.1 at 123,354 B, the payload the packed enum moved and the one nothing had checked |
| `ffi/logs/rust/stage3-M3.log` | as stage3-M2, ASCII, guard on | M3: byte identity on P3.1; explicit presence as three cases x three fields x four arms, all agreeing; the oneof by member including the payload-free one; seven unknown-field vectors, hand-built; 3 crossings per 200 elements in both directions; one timing row set |
| `ffi/logs/rust/stage3-content-sets.log` | as stage3-M2, plus simdutf8 0.1 as one arm; encode and decode over P1.2 and P2.2, all three content sets in ONE process | ABI v1 open decision 3: the scalar validator costs 2.2 to 3.0 times its ASCII self on non-ASCII content and loses 2.0 to 2.6 to prost; a SIMD validator with the same contract recovers half to two thirds of it; decode is unaffected in ordering |
| `ffi/logs/rust/stage3-M2.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on (section 6 off), ASCII, 4 shared vCPUs | M2 over P2.1 to P2.5: byte identity across four arms plus value identity across the three facade decoders; 7.004 crossings per task on decode and 10.02 on encode; ABI v1 open decision 5 answered and isolated; the two shape-coverage findings; the guard priced on a shape that makes 7 to 10 reverse calls per element |
| `ffi/logs/rust/stage2-four-arms-M1.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on (section 6 off), ASCII, 4 shared vCPUs | byte identity across four arms; crossing counts; the boundary is a real dynamic import; the crossing costs 1.8 ns; the ratio table above; the guard is free; UTF-8 validation costs 25-30 percent of an encode |
