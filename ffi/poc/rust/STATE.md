# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **stages 1 to 4 complete.** Every message and payload of `design/SHAPES.md` has all four arms byte-identical to the validated manifest, and the RPC arm is measured. Ready for the aggregating session to assemble |
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
| P2.1 | encode | 0.963 - 0.974 | 0.457 - 0.460 | 0.944 - 0.993 |
| P2.2 | encode | 0.989 - 0.999 | 0.493 - 0.499 | 0.863 - 0.874 |
| P2.3 | encode | 0.982 - 0.990 | 0.484 - 0.492 | 0.952 - 0.953 |
| P2.4 | encode | 0.993 - 0.996 | 0.567 - 0.574 | 1.107 - 1.123 |
| P2.5 | encode | 0.976 - 1.009 | 0.454 - 0.486 | 0.891 - 0.948 |
| P2.1 | decode | 0.958 - 0.966 | 0.984 - 0.999 | 1.251 - 1.296 |
| P2.2 | decode | 0.967 - 0.974 | 0.922 - 0.927 | 1.025 - 1.034 |
| P2.3 | decode | 0.985 - 1.002 | 1.030 - 1.038 | 0.940 - 0.949 |
| P2.4 | decode | 0.942 - 0.950 | 1.033 - 1.037 | 0.959 - 0.963 |
| P2.5 | decode | 0.974 - 0.998 | 0.937 - 0.963 | 1.046 - 1.072 |
| P3.1 | encode | 0.966 - 0.975 | 0.406 - 0.410 | 0.846 - 0.853 |
| P3.1 | decode | 0.961 - 0.968 | 0.811 - 0.824 | 0.834 - 0.838 |
| P4.1 | encode | - | 0.411 - 0.415 | 0.789 - 0.816 |
| P4.1 | decode | - | 0.861 - 0.864 | 0.916 - 0.920 |
| P5.3 | encode | - | 0.96 - 0.97 | 1.00 - 1.01 |
| P5.3 | decode | - | 0.45 - 0.46 | 0.45 - 0.46 |
| P5.4 | encode | - | 1.01 - 1.04 | 0.79 - 0.83 |
| P5.4 | decode | - | 0.084 | 0.080 |
| P6.1 | encode | - | 0.55 - 0.56 | 0.61 - 0.62 |
| P6.1 | decode | - | 0.86 - 0.87 | 0.56 - 0.60 |

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
- **A 4 MB bulk decode costs exactly one copy in the core and twelve in prost.** P5.4 decode
  is 0.080 to 0.084 of prost, and a raw `Bytes::copy_from_slice` of the same 4 MB measured
  0.082 in the same process: the core arms are ON the memcpy floor. The claim is bounded by
  that control, not by the ratio. Why prost sits twelve times above the floor is a suspicion
  (`bytes = "vec"` merging over a `Take` inside the nested message; the penalty grows with
  size) and is **not verified**.
- **P4.1 and P6.1 behave like M1 and M3**, not like M2: encode 0.45 to 0.62 native and 0.61
  to 0.82 through the ABI, decode 0.56 to 0.92.
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

Nothing is outstanding. The slice has done what W3 asked: four arms, every shape, and the
interface-cost decomposition available to every other slice.

If more is wanted, in the order I would do it:

1. **The `latin1`/`wide` sets on the remaining payloads**, and the SIMD validator on a
   machine without AVX2. One machine is one machine.
2. **A concurrency suite** (ABI v1 obligation 12.5): two payload shapes, threads in sequence
   and together, every encode asserted against a reference. The learned-width table is per
   context and has never been touched by two threads, which is the case section 6 says a
   global table fails at. Stage 4's defect D16 is what that suite exists to catch, and it
   was found by accident rather than by a suite.
3. **`ak_init` and the lifecycle** (section 3), which is unbuilt, so "every entry point
   requires `ak_init`" is unexercised.
4. **The pull decode family**, to turn "push is the right default at 1.8 ns" from an argument
   into a measurement.

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
| D16 | `crates/ak-core/src/rpc.rs` | `ak_call_unary` took `*mut ak_client` and mutated a shared `Grpc`, so two host threads calling it at once raced. It worked at 1 in flight and failed outright at 8 | **fixed**: the client holds the `Channel`, a call clones it and builds its own `Grpc`, and the call takes a shared reference — the shape section 9 already implies. Found by the concurrency arm, which is what it is for |
| D15 | `crates/stage1-validate/src/build.rs` | the hand-written prost builder still used the old adapter rule after `0c2d4d7f` | **fixed**, and the conformance run is what said so: the three generated-builder arms matched the new hashes and the prost arm did not. First time the two independent construction routes have caught anything |
| D14 | `crates/harness/src/bin/bench.rs` | the M4 to M7 `core-native` encode case allocated a fresh `Vec` per call and grew it by doubling, while every other arm reused a buffer. It measured `core-native` at **1.412** of prost on P5.4, a false regression | **fixed**: the arm uses the same reused `Enc` as M1 to M3, and 1.412 became 1.018 |
| D13 | `gen/ir.py` | the first `direct_fields` walked singular message children only, so ABI v1 section 8's refusal found nothing and refused nothing | **fixed**, and `gen/check_direct.py` exercises both refusal cases on every run. Same class as D12, caught the same way: by running it against a case that must fail |
| D12 | `gen/rust_core.py`, `gen/rust_abi.py` | every backend iterated `Message.plain`, which excludes oneof members, so adding `ListProbeResponse` produced a complete-looking codec that ignored `Probe.body` and said nothing | **fixed**: both walkers call `b.oneof(...)` explicitly, so a backend that cannot do the shape raises rather than skipping it |
| D11 | this slice's reporting | P6.1's re-validation was reported with no log behind it, against a stale log that still showed the old size | **fixed**. Every schema change now re-runs `gen/stage1.sh` into a dated log before the result is quoted |
| D9 | `gen/rust_abi.py` | an element run did not restore the codec's open-field state, so the second and later chunks read whatever the last element left behind. It cost 448 length-prefix misses in 500 elements, and it reads `open_tag` too, so a host that chunks would write later chunks under the inner field's tag | **fixed**: every element and run entry point saves and restores the open state, and `gen/stage3.sh` step 3 carries a regression for it. **The payload set could not have caught it**: byte identity passed only because `ListTasksDetailedResponse.tasks` and `TaskOptions.options` are both tag 1 |

## What is not measured

**A pass for completeness, not for brevity** (README section 11: the report is assembled from
these lists as much as from the verdicts). Every shape and payload of `design/SHAPES.md` is
now covered, so what follows is what remains after that.

### Shapes that the payload set cannot reach, so nothing measures them

Four, all reported to the aggregating session and none fixed here:

1. **A packed repeated enum** was claimed for M2 and existed nowhere. **Fixed in the schema**
   at `945d3cd1` as `MetricsBatch.statuses`, and now measured.
2. **Nesting past depth 3.** `SHAPES.md` claimed depth 6; the description's maximum is 3, and
   the real schema reaches 6 through the filter and request family, which this response-only
   payload set does not carry. Levels 4 to 6 are unexercised, and with them ABI v1 decision 7
   (the decode recursion limit), which nothing here can reach.
3. **The adapter's non-injective states.** `P4.1`'s `error` is a sentence in all 200 elements,
   so the plain wire form's `Ok` and `Invalid` states never occur, and they are the two the
   map cannot separate. Checked here by hand-written state instead.
4. **The adapter's nested site is worse**: `emit/payloads.py` fills `TaskOutput.success` and
   `TaskOutput.error` independently, so P2.x contains `success=true, error=<sentence>`, a
   state no adapter over `{Ok, Error(d)}` can represent. The nested adapter site is therefore
   not reachable at all, and this slice keeps `TaskOutput` as a plain struct there.

### Behaviour this design changes and this slice cannot price

- **Unknown-field retention (ABI v1 decision 11).** Nothing here retains an unrecognised
  field from decode to re-encode, and neither does prost. Rust is the one incumbent that
  already drops them, so this slice is structurally the wrong place to measure what removing
  the guarantee costs the other four languages.
- **The transcode pair** (README section 10 item 4). A Rust `String` cannot hold an unpaired
  surrogate, so this slice **cannot construct the input** that makes protobuf-java and
  Google.Protobuf disagree. Unreachable, not unbuilt.
- **The direct-argument path's actual win** (ABI v1 section 8). Built and byte-identical, but
  its value is a pinned buffer on the JVM; on a Rust host there is nothing to pin and the copy
  is a copy either way. P5.3 and P5.4 are a memcpy figure and are not evidence for it.

### ABI surface that is specified and not built

- **The pull decode family** (section 7.1): only push is built, which is the right default for
  a host whose reverse call costs 1.8 ns, but the claim that pull would be no better here is
  an argument and not a measurement.
- **Most of the RPC half** (section 9). Built and measured: the blocking unary call over a
  channel. **Not built**: the callback and completion-queue delivery modes, metadata,
  deadlines, the gRPC status code as a number, cancellation (section 9 gives the blocking
  call a handle so it can be cancelled and `ak_call_unary` takes none), retry and backoff,
  TLS, streaming, a real network, failure injection and the server side. The RPC half's case
  is **behavioural** and none of that behaviour is exercised: stage 4 measures the call path,
  which is the half of section 9 whose case was never in doubt.
- **`ak_init` and the lifecycle** (section 3): no runtime, context or client, no crypto
  provider, no log or tracing bridge, no panic hook, no `worker_threads` default. The codec
  half needs none of it and this slice built none of it, so section 3's claim that every entry
  point requires `ak_init` is unexercised.
- **Group layout export and assert at load** (section 10 and obligation 12.3). Both sides
  compile against one generated header here, so there is nothing to disagree — which means the
  insurance that matters to a hand-layout host (FFM) is untested.
- **The `ak_span.coder` hint** (decision 4): present in the struct, read by nothing.
- **Message size limits and the recursion limit** (decisions 7 and 8): `AK_ERR_LIMIT` and
  `AK_ERR_DEPTH` exist and nothing sets or enforces either.
- **A union group layout for a oneof**: the flat discriminant form is built; the union is an
  alternative the aggregating session has ruled out on layout-reproducibility grounds, not a
  measured one.
- **The unbatched element form on decode**: only the run form is built.

### Error paths

- `ak_fail` is reachable in this slice **only through a panic in the generated guard**. No
  test makes a host fail mid-run deliberately, so the codec's rollback of a half-written field
  (section 6, "the widest hole in the drafted interface") is written and unexercised.
- An unrecognised oneof `body_case` is refused with `AK_ERR_ABI`; nothing in this build can
  produce one, so the path is built and not exercised.
- Malformed wire: `AK_ERR_MALFORMED` and `AK_ERR_TRUNCATED` are produced by the reader and no
  vector exercises them.

### Measurement coverage

- **Content sets**: `latin1` and `wide` on P1.2 and P2.2 only, encode and decode. Not on the
  other payloads, and with no manifest oracle (byte identity against the prost arm instead).
- **Concurrency**: one thread everywhere. The learned-width table is per context and never
  exercised by two threads, which is the case ABI v1 section 6 says a global table fails at,
  and obligation 12.5's concurrency suite does not exist.
- **Allocation and footprint**: nothing counts allocations, peak memory or the arena's real
  cost in any arm.
- **Linkage**: a shared library only. A C or C++ host that statically links the same core
  gets a direct call and pays less than the 1.8 ns measured here.
- **The floor**: no rustc 1.88 exists in this container, so "builds and passes on the MSRV" is
  a declaration and not a run. README section 5.2's arms b and c do not exist for Rust, and
  for Rust the floor is the target, so only arm a is meaningful — but arm a is unverified as
  being at the floor.
- **`packages/rust`'s own `armonik` crate**: the `armonik` arm reproduces its *pattern* and
  does not use `armonik-macros`, whose expansions reference `armonik`-internal paths and read
  a descriptor built from `Protos/V1`. The `armonik` column prices this generator's emitter,
  and a defect in it was worth 20 to 40 percent before it was swept, so that caveat is load
  bearing.
- **Hardware**: one 4-vCPU container, one x86-64 microarchitecture, AVX2 present. The SIMD
  validator result in particular is one machine.

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
| `ffi/logs/rust/stage4-rpc.log` | rustc 1.94.1 release, tonic 0.14 over loopback h2 no TLS, cdylib boundary, server in-process, 4 shared vCPUs | **Two crossings per RPC and zero per field.** CPU per RPC 0.91 to 1.10 of tonic over two processes at 1, 8 and 16 in flight: no measurable difference. The carrier-thread row is reported empty and not substituted for |
| `ffi/logs/rust/stage1-manifest-vs-prost-adapter.log` | as above, **schema at `0c2d4d7f`** | 16 of 16 after the adapter fix. Supersedes the P2.x and P4.1 rows of the two earlier stage 1 logs |
| `ffi/logs/rust/stage3-M2-M4-revalidated.log` | as stage3-M2 | M2 and M4 re-measured after `0c2d4d7f`. Crossings and decision 5 unchanged to the digit; ratios tighter and two moved toward parity. **Supersedes the M2 rows of `stage3-M2.log` and the M4 rows of `stage3-M4-M7.log`** |
| `ffi/logs/rust/stage3-M4-M7.log` | as stage3-M2, ASCII, guard on | M4 to M7: byte identity on P4.1, P5.1 to P5.4, P6.1 and P7.1, with the class labelled per row; ABI v1 section 8's generator-time refusal exercised; the adapter's two wire forms checked by state; M7 by decode and permutation |
| `ffi/logs/rust/stage3-M3.log` | as stage3-M2, ASCII, guard on | M3: byte identity on P3.1; explicit presence as three cases x three fields x four arms, all agreeing; the oneof by member including the payload-free one; seven unknown-field vectors, hand-built; 3 crossings per 200 elements in both directions; one timing row set |
| `ffi/logs/rust/stage3-content-sets.log` | as stage3-M2, plus simdutf8 0.1 as one arm; encode and decode over P1.2 and P2.2, all three content sets in ONE process | ABI v1 open decision 3: the scalar validator costs 2.2 to 3.0 times its ASCII self on non-ASCII content and loses 2.0 to 2.6 to prost; a SIMD validator with the same contract recovers half to two thirds of it; decode is unaffected in ordering |
| `ffi/logs/rust/stage3-M2.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on (section 6 off), ASCII, 4 shared vCPUs | M2 over P2.1 to P2.5: byte identity across four arms plus value identity across the three facade decoders; 7.004 crossings per task on decode and 10.02 on encode; ABI v1 open decision 5 answered and isolated; the two shape-coverage findings; the guard priced on a shape that makes 7 to 10 reverse calls per element |
| `ffi/logs/rust/stage2-four-arms-M1.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on (section 6 off), ASCII, 4 shared vCPUs | byte identity across four arms; crossing counts; the boundary is a real dynamic import; the crossing costs 1.8 ns; the ratio table above; the guard is free; UTF-8 validation costs 25-30 percent of an encode |
