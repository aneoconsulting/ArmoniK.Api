# cpp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **complete: the full codec (the equivalent of the rust slice's stages 1 to 3) and the RPC arm.** Every message and payload of `design/SHAPES.md` has five arms byte-identical to the validated manifest, at C++11, C++14 and C++17, floor and target implementations, shared and static linkage. ABI v1 open decision 1 is answered with measurements |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | **C++11, and it is demonstrated, not declared.** C++14 also builds and passes (README open question 3) |
| **Target** (where the clock runs) | C++17 |
| **Incumbent** | protobuf C++ 3.21.12 (`libprotobuf-dev`, apt), non-arena and arena. `packages/cpp` pins no protobuf version and sets `CXX_STANDARD 14` on every target |
| **Machine** | 4 vCPU Intel Xeon @ 2.80 GHz, Linux 6.18.44, g++ 13.3.0, rustc 1.94.1 |
| **R13 calibration** | **this machine's rust-slice crossing is 1.5 ns forward and 2.1 to 2.2 ns forward-plus-reverse** (`logs/cpp/calibration-r13.log`), against the 1.8 ns the rust slice measured in its own container. Every absolute below is also quotable as a multiple of 1.5 ns |

## The question this slice answers

Does the amended ABI still work for the language the design was drafted for, and is it free
here as the managed reports assume? **ABI v1 open decision 1 is this slice's to settle, and
it blocks freezing the specification.**

## Arms

| arm | what it is | linkage |
|---|---|---|
| `pb` | protobuf C++, non-arena, **deterministic serialisation** (R2 needs it for any message with a map) | - |
| `pb-arena` | the same on a `google::protobuf::Arena` | - |
| `native` | the generated codec emitted into C++, over the same facade objects: R3's no-boundary control | - |
| `ffi` | the same codec through the C ABI | **shared library (primary)** and **static (second, separately labelled)** |
| `ffi-zeroed` | ABI v1 open decision 9's candidate element fill | both |
| `ffi-nobat` | the host declines to batch: one element per call | both |
| `ffi-hosttc` | the transcoder lives in the HOST, so every string costs a reverse crossing: what a string-as-a-CALL form costs | both |
| `groupfill` | the by-value group's host-side fill ALONE, no codec at all | - |

**Never form a ratio across the two linkages.** They are separate processes and separate
mechanisms (R7); they are reported as separate columns.

## What exists

```
gen/generate.py [--check]  the generator. Imports ffi/poc/rust/gen/{ir,rust_abi,rust_core}.py
                           READ-ONLY, so the core is the SAME core the rust slice measures
gen/cppnames.py cpp_header.py cpp_facade.py cpp_build.py cpp_pbbuild.py
gen/cpp_core.py cpp_binding.py cpp_layout.py cpp_cases.py
gen/run_all.sh             every gate and every clock, into ffi/logs/cpp/
gen/boundary.sh            R5 from the built artifact, both halves, 10 checks
gen/odr_check.sh           README 5.1's hard stop, with its positive control
gen/calibrate.sh           R13: the rust slice's crossing benchmark, on THIS machine
gen/audit_tracked.sh       R4's closing rule, asked of git rather than of .gitignore

core/                      ak-core-cpp: cdylib AND staticlib from one source.
                           codec.rs emitted by rust_abi.emit_codec; ak-abi and ak-rt are
                           PATH DEPENDENCIES on the rust slice's crates, not copies
include/ak_abi.h           the C header of ABI-v1.md, emitted from the rust backend's own
                           group_fields/presence_bits so a layout cannot be restated
include/ak/{vocab,rt,values}.h   the C++11 vocabulary types, the Enc/Dec runtime, the value rules
src/generated/             facade, payload builders (facade and protobuf), core-native, binding
src/{conformance,bench,counts,rpcbench}.cpp  the harnesses
src/odr_{a,b,main}.cpp odr_body.inc          the C++11 / C++17 seam
CMakeLists.txt             16 executables, each naming its own -std, impl level, guard,
                           decode policy and linkage
```

## What is measured

All from `ffi/logs/cpp/`. Ratios are to `pb` (protobuf C++ non-arena, deterministic),
formed inside one process; the range is across 9 rounds, each the minimum of five
sub-batches.

### The C++ column, shared library (the primary arm)

`logs/cpp/bench_a17_shared.log`.

| payload | dir | pb-arena | native | ffi |
|---|---|---|---|---|
| P1.1 | enc | 0.975 - 0.986 | 0.399 - 0.404 | 0.910 - 0.925 |
| P1.2 | enc | 0.952 - 0.965 | 0.431 - 0.439 | 0.895 - 0.913 |
| P1.3 | enc | 0.991 - 1.003 | 0.602 - 0.608 | **1.642 - 1.671** |
| P2.1 | enc | 1.002 - 1.007 | 0.366 - 0.370 | 0.757 - 0.766 |
| P2.2 | enc | 0.955 - 0.974 | 0.370 - 0.373 | 0.691 - 0.699 |
| P2.3 | enc | 0.942 - 0.952 | 0.259 - 0.268 | 0.591 - 0.597 |
| P2.4 | enc | 0.952 - 0.969 | 0.251 - 0.262 | 0.593 - 0.605 |
| P2.5 | enc | 0.970 - 0.992 | 0.372 - 0.380 | 0.761 - 0.783 |
| P3.1 | enc | 0.971 - 0.979 | 0.443 - 0.456 | 1.002 - 1.012 |
| P4.1 | enc | 0.980 - 0.995 | 0.293 - 0.299 | 0.588 - 0.602 |
| P5.1 | enc | 0.984 - 0.999 | 0.250 - 0.253 | 0.521 - 0.542 |
| P5.2 | enc | 0.821 - 0.897 | 0.505 - 0.622 | 0.493 - 0.564 |
| P5.3 | enc | 0.998 - 1.060 | 0.531 - 0.601 | 0.530 - 0.592 |
| P5.4 | enc | 0.993 - 1.027 | 0.680 - 0.719 | 0.665 - 0.720 |
| P6.1 | enc | 0.995 - 1.007 | 0.890 - 0.898 | 1.218 - 1.229 |
| P1.1 | dec | 0.921 - 0.943 | 0.830 - 0.857 | 0.761 - 0.784 |
| P1.2 | dec | 0.665 - 0.674 | 0.691 - 0.702 | 0.588 - 0.798 |
| P1.3 | dec | 0.430 - 0.439 | 0.907 - 0.927 | 0.951 - 0.974 |
| P2.1 | dec | 0.801 - 0.808 | 0.774 - 0.778 | 0.687 - 0.694 |
| P2.2 | dec | 0.645 - 0.660 | 0.729 - 0.738 | 0.663 - 0.674 |
| P2.3 | dec | 0.894 - 0.905 | 1.058 - 1.084 | 0.937 - 0.955 |
| P2.4 | dec | 0.707 - 0.732 | 0.839 - 0.886 | 0.747 - 0.789 |
| P2.5 | dec | 0.847 - 0.878 | 1.005 - 1.018 | 0.905 - 0.939 |
| P3.1 | dec | 0.655 - 0.667 | 0.860 - 0.871 | 0.627 - 0.637 |
| P4.1 | dec | 0.707 - 0.715 | 0.719 - 0.727 | 0.681 - 0.691 |
| P5.1 | dec | 1.137 - 1.154 | 0.628 - 0.637 | 0.665 - 0.676 |
| P5.2 | dec | 0.797 - 0.950 | 0.692 - 0.867 | 0.689 - 0.908 |
| P5.3 | dec | 0.989 - 1.009 | 0.988 - 0.996 | 0.990 - 0.996 |
| P5.4 | dec | 0.916 - 1.001 | 0.913 - 0.998 | 0.910 - 1.000 |
| P6.1 | dec | 0.761 - 0.777 | 1.241 - 1.273 | 0.632 - 0.641 |

### Static linkage, the second column (`logs/cpp/bench_a17_static.log`)

Reported separately and never divided by the column above. The crossing is what differs:
**1.23 to 1.31 ns forward statically against 1.82 to 1.84 ns through the shared library**,
in the same process and build as the arms. On P1.2 encode (9 crossings per 1,000 elements)
`ffi` moves from 0.895-0.913 to 0.842-0.854; on P2.2 encode (10.02 per element) it does not
move (0.685-0.691 against 0.691-0.699), which is the right shape: the crossings are few and
the group is what costs.

### Crossing counts, from the counting core (`logs/cpp/counts.log`)

Identical to the rust slice's to the digit, which is the point: the counts are a property of
the interface and not of the host.

| payload | encode fwd / rev | decode fwd / rev | per element |
|---|---|---|---|
| P1.2 (M1, 1000) | 8 / 1 | 1 / 5 | 0.009 enc, 0.006 dec |
| P2.2 (M2, 500) | 2511 / 2501 | 1 / 3501 | **10.024 enc, 7.004 dec** |
| P3.1 (M3, 200) | 2 / 1 | 1 / 2 | 3 per 200, both directions |

`transcode` is counted separately and is NOT a crossing with the specified transcoders,
because `ak_tc_bytes` lives in the core: 17,167 transcodes on P2.2 encode cross nothing. With
a HOST transcoder every one of them is a reverse crossing, and that is what decision 1's
string-as-data question is worth.

Unbatched, per element: P1.2 1.001, P2.2 17.002, **P2.3 125.008, P2.4 311.012** forward.

### ABI v1 open decision 1: the three mechanisms, priced

Each as a within-round delta between two arms (R4), never as a ratio to a third.
`logs/cpp/bench_a17_shared.log`, sections "the STRING-AS-DATA form", "the BATCHING
predicate", "decision 9's zeroed element fill".

**1. The group is the one that costs, and it costs the HOST, not the boundary.** The
`groupfill` arm times the fill alone with no codec: **22.1 ns per element on M1 (P1.2, 11.9%
of a protobuf encode), 75.7 ns on M2 (P2.2, 4.7%)** and **22.1 ns on M1's absent path (P1.3),
where a whole protobuf encode is 18.1 ns**. That is the P1.3 inversion, and it reproduces in
C++ with the same sign and a larger magnitude than in Rust: `ffi` 1.642-1.671 of protobuf
against `native` 0.602-0.608. **Decision 9's candidate fixes it here too** (P1.3
-13.7 ns/element, **-75% of a protobuf encode**), so **C++ agrees with the Rust answer**: the
empty-element path is a host-side fill change, not an ABI change.

**2. String as data is a small WIN in C++, not a cost.** The host-transcoder arm, which is
what a string-as-a-CALL form costs, is slower by **1.1 to 3.9 percentage points of an
encode**, or **0.3 to 1.2 ns per string**, which is one reverse crossing (measured at 0.60 to
0.62 ns through the shared library). So C++ does not pay for the amendment; it gains a
little, and the gain scales with string density exactly as the census says it should.

**3. The batching predicate is a small LOSS in C++ on the shapes the control plane moves.**
Unbatched is **faster** by 0.2 to 6.9 points of an encode on P1.1, P1.2, P1.3, P2.2, P2.5 and
P4.1 — including **-1.28% on P2.2**, the payload SHAPES.md says to read first — and batching
wins only where the crossing count per element explodes: **P2.3 +6.50% and P2.4 +6.90%**, at
125 and 311 forward crossings per element unbatched. At a crossing of 1.8 ns the 32 KB
chunk's second pass over memory costs more than the crossings it saves. ABI v1 already says
"a host may decline to batch at all and loses only what its own crossing costs"; in C++ a
host that declines **gains** on the common shapes.

**4. A fourth mechanism nobody listed, and it is larger than two of the three above.** ABI v1
section 4 removed the declared expansion bound, so the core cannot write a length-prefixed
blob in one pass: it opens a prefix of a learned width, hands the transcoder the rest of the
buffer, and resolves the prefix afterwards. A host that already holds the bytes knows the
length and writes key, length and body in one pass. Measured on P1.2's 6,000 strings in one
process: **5.13-5.17 ns against 8.75-8.82 ns per string, +3.6 ns**, and **that is where most
of the C ABI's encode gap against the no-boundary control goes** (P1.2: 86 ns/element of gap,
of which 22 is the group fill and ~6 x 3.6 = 22 is this, the rest being the run and element
entry points).

### README 5.2's arms a, b and c

| arm | build | P1.2 enc `ffi` | P2.2 enc `ffi` | P2.2 dec `ffi` | log |
|---|---|---|---|---|---|
| a | target impl, `-std=c++17` | 0.895 - 0.913 | 0.691 - 0.699 | 0.663 - 0.674 | `bench_a17_shared.log` |
| b | **floor impl**, `-std=c++17` | 0.896 - 0.900 | 0.690 - 0.700 | 0.646 - 0.666 | `bench_b17_shared.log` |
| c | floor impl, `-std=c++11` | 0.899 - 0.906 | 0.685 - 0.698 | 0.639 - 0.659 | `bench_c11_shared.log` |

**The floor's missing APIs cost nothing measurable.** Arm b is inside arm a's spread on every
row. Only arm a produces ratios for the report; arm c stands alone and is the same.
The divergence that exists is real and generated (`insert_or_assign` and `emplace_back`'s
return value at C++17, `operator[]` and `push_back`+`back()` at C++11), and the wire bytes are
identical at all three levels, checked by running the whole corpus at each.

### Correctness

`logs/cpp/conformance.log`. **361 checks, 0 failures**, six times over: C++17 target, C++17
floor, C++14 floor, C++11 floor, C++17 static, C++17 lossy-decode. Every payload of
`SHAPES.md`, five encoders byte-identical to `manifest.json`, decoded values identical
between the two facade decoders and equal to the built value, round trips re-encoding to the
manifest, P7.1 by decode and by permutation of its (tag, wire type, body) triples, the
unknown-field vectors, the unknown enum value, and malformed UTF-8.

**One wire divergence, and it is the INCUMBENT's.** protobuf C++ writes a map entry's key and
value unconditionally; the canonical form of `manifest.json` omits an implicit-presence leaf
holding the proto zero, and prost does too. On P2.5 that is exactly +80 B (2 B x 40 emptied
map values). Values agree, protobuf reads the canonical bytes and re-serialises them to its
own form. **Reported, not fixed:** `schema/` is not this slice's to change, and neither
encoder is wrong.

**Two facts about the incumbent that a C++ column has to state.** protobuf C++ **rejects**
malformed UTF-8 in a proto3 `string`, so ABI v1 decision 3's rejecting decode is the
like-for-like comparison. And protobuf C++ needs **deterministic serialisation** to be
byte-stable on any message with a map, which costs it **7.6 to 8.3 percent** on P2.2; it is
on in every number above, because R2 needs it, and the non-deterministic figure is carried
beside it rather than folded in.

### The proofs that the arms are what they say

- **R5, half one** (`logs/cpp/boundary.log`): the shared arm's 37 `ak_*` symbols are
  undefined dynamic imports; the static arm has them defined and **called**, 3 to 5 call
  sites each, sizes printed. **R5's named C++ hazard does not fire here**: `-flto` over a
  statically linked core cannot inline the entry points away, because gcc's LTO only inlines
  across GIMPLE it produced and a Rust staticlib's members are native objects. Verified on a
  build made for the purpose. That is a fact about the toolchain pair, not a general result.
- **R5, half two**: the no-boundary control is not fused into the benchmark loop. Its
  traversal is out of line at 3,243 B against a largest timing closure of 869 B, and it is
  reached through a function pointer, so its address is taken and a body must exist. The
  `-flto` build is carried as the condition that would break it.
- **README 5.1's hard stop** (`logs/cpp/odr.log`): 144 layout facts compared between a
  `-std=c++11` translation unit and a `-std=c++17` one that are linked together, facade
  objects passed across the seam both ways. **0 moved.** The `-DAK_ODR_BREAK` positive
  control moves **49** and the check fails, so the guard has been seen working.
- **ABI v1 section 10 / obligation 12.3**, which the rust slice could not exercise: the core
  exports 380 group-layout facts and the host compares them with its own compiler's.
  **0 disagreements**, and it caught 43 on its first run against a stale artifact.

### The RPC arm (`logs/cpp/rpc.log`)

One unary call carrying P2.2 (540,422 B response) against grpc++ 1.51.1 over loopback, no
TLS, with the server in-process on its own threads. **R9 is why two CPU columns exist**: the
in-process server deep-copies and re-serialises the response on every call and costs more
than either client, so `client CPU` sums `CLOCK_THREAD_CPUTIME_ID` over the client threads
only and `process CPU` is printed beside it to make the dilution visible.

| in flight | grpc++ client CPU | core-ffi client CPU | ratio | grpc++ wall | core-ffi wall |
|---|---|---|---|---|---|
| 1 | 4.76 ms | 2.65 ms | **0.558** | 8.89 ms | 7.53 ms |
| 8 | 4.95 ms | 3.23 ms | **0.652** | 3.78 ms | 2.75 ms |
| 16 | 5.33 ms | 3.32 ms | **0.622** | 3.34 ms | 2.44 ms |

**The wall-clock column is beside the CPU column and is not a throughput figure.** 540 KB
against a 64 KB default stream window is exactly R9's hazard: at 1 in flight the wall clock
is nearly twice the CPU, and it collapses by more than half at 8.

**Most of that ratio is the CODEC, not the transport**, and the subtraction is in-process:
the same response decoded standalone in the same binary costs protobuf C++ 3.59 ms and the
core through the C ABI 2.34 ms (0.653), so what is left is about 1.17 ms of transport for
grpc++ against about 0.31 ms for tonic. The transport half's own interface cost is **two
crossings per RPC and zero per field** — 3.6 ns against ~10^6 ns, about three parts in a
million, which is ABI v1 section 9's arithmetic reproduced on this machine. It is a property
of the code rather than a measurement, and `gen/rpc.sh` greps the core's RPC module for any
mention of a message type rather than trusting the sentence.

**The carrier-thread row is answered trivially and says so.** The idiomatic C++ wait is a
blocking call on a thread the host owns, and both arms use it; C++ has no carrier-thread
notion to pin, so SHAPES.md's third RPC question is a real question only on a runtime with
virtual threads.

### The guard and the decode policy

- **The accessor guard is not measurable** (`bench_a17_noguard.log` against
  `bench_a17_shared.log`): P1.2 encode `ffi` 0.904-0.925 without it against 0.895-0.913
  with it, P2.2 encode 0.679-0.693 against 0.691-0.699. In C++ the guard is a `try`/`catch`
  with no throw on the path, which costs nothing at run time on the Itanium ABI. Same
  verdict as Rust, different mechanism.
- **The decode UTF-8 policy is NOT free in C++, and this differs from the Rust result**
  (`bench_a17_lossy.log`). P2.2 decode `ffi` is 0.663-0.674 of protobuf validating and
  0.515-0.540 not validating; P1.2 0.588-0.798 against 0.452-0.658. **Validate-and-reject
  costs 22 to 28 percent of a decode here.** The reason the Rust slice found it free is that
  `String::from_utf8_lossy` already validates, so its "lossy" arm was paying for a scan;
  a C++ `std::string` holds arbitrary bytes, so **this slice's lossy arm does not check at
  all** and the comparison is validation against nothing. The headline column uses the
  rejecting policy, because protobuf C++ rejects too and that is the like-for-like
  comparison. What the 22-28% prices is **this slice's scalar validator**, not the policy:
  the rust slice measured a SIMD validator recovering half to two thirds, and nothing here
  has tried one.

## Next step

Nothing is outstanding for W4. If more is wanted, in the order I would do it:

1. **A SIMD validator on the decode path**, since the 22-28% above is a validator figure and
   not a policy one, and it is the largest single effect this slice measured.
2. **A core fast path for `tc == ak_tc_bytes`**, which would remove the two-pass blob write
   (+3.6 ns per string) for every host whose representation is already UTF-8. This slice can
   measure the effect but cannot make the change: the core emitter is shared.
3. **The content sets** on the payloads this slice covers, which price the validator's path
   and the wire width rather than a transcoder.
4. **A concurrency suite** (ABI v1 obligation 12.5). The RPC arm ran 8 and 16 in flight
   through one client handle and found nothing, which is one shape, not a suite.

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| C1 | `include/ak/rt.h` | `ak::Enc` was a `std::vector<uint8_t>` with `push_back`; through an `Enc*` the compiler reloads the finish pointer per byte where Rust's `&mut Vec<u8>` carries noalias. **The no-boundary control measured 1.245 of protobuf on P1.2 encode while the same codec through the C ABI measured 0.855** — a control slower than the arm it controls for | **fixed**: a raw cursor over a reserved block, which is what protobuf C++'s own serialiser is. 0.443 after; the generated traversal did not change |
| C2 | `gen/cpp_binding.py` | decision 9's zeroed fill memset every chunk, including the blob-run and map-entry chunks that are assigned in full; the inner loops run once per element, so P2.2 paid ~80 MB of memset and the candidate measured 1.944 of protobuf against 0.647. **The candidate looked refuted and was not** | **fixed**: only an element-group chunk is cleared |
| C3 | `gen/cpp_binding.py` | the clear was O(arena) where the fill is O(elements): clearing the whole 32 KB chunk (**which is what the rust slice's emitter does**) cost +156 ns/element on P1.1 and +550 on P2.1, against protobuf encodes of 172 and 1,209 | **fixed here**: clear `min(n, chunk)` elements. **Reported to the aggregating session as a property of decision 9's candidate**, not only of this build |
| C4 | `include/ak/vocab.h` | `ak::Optional<T>::set` took `const T&` only, so a decoded child was built by value and then copied; on M5's 4 MB `bytes` field that is a second copy of the whole payload (P5.4 decode 3.7 ms slower than the control) | **fixed**: an rvalue overload |
| C5 | `gen/cpp_header.py`, `gen/cpp_layout.py` | the description's message order puts `Empty` after `Probe`, and C inlines a child group by value. A compile error in the header — but the SAME order drives the run-time layout table, where it would have been silent and would have made section 10's check pass vacuously | **fixed**: one `abi_order_topo()` used by both |
| C6 | this container | `libgrpc++` is 1.51.1 (apt), which is not what `packages/cpp` builds against (it pins none and takes what `find_package` finds) | open, cannot be fixed here. Stated on the RPC log |
| C7 | `src/rpcbench.cpp` | the in-process server deep-copies and re-serialises a 540 KB message per call and costs more than either client, so a whole-process CPU figure is mostly the server | **not a defect, a hazard**: `client CPU` measures the client threads only and `process CPU` is printed beside it so the dilution is visible |

## What is not measured

**A pass for completeness, not for brevity** (README R11).

### Covered but not by this slice
- **The RPC arm** is written and not yet run. Nothing in this state file's numbers touches
  transport.

### Shapes and payloads
- Every message M1 to M7 and every payload P1.1 to P7.1 of `design/SHAPES.md` is covered.
  The four gaps the rust slice records are **structural and inherited**: nesting past depth 3
  (so ABI v1 decision 7's recursion limit is unreachable), the adapter's non-injective states
  at the plain site, the adapter's nested site being filled into a state no adapter can
  represent, and P7.1 being decode-only.
- **Content sets**: ASCII only. `ak::values::recode` exists and nothing calls it. What
  `latin1` and `wide` price in C++ is **not** a transcoder — a C++ `std::string` is bytes, so
  there is no narrowing — it is the decode validator's path and the wire width, the same two
  things the rust slice measured. Unmeasured here.
- **The decode UTF-8 policy** is priced only as whole-payload arms (`bench_a17_lossy.log`),
  never on the string path in isolation.

### ABI surface built and not exercised, or not built
- **The pull decode family** (ABI v1 7.1): only push is built. For a host whose reverse call
  costs 0.6 ns that is the right default, and saying so is still an argument.
- **`ak_init` and the lifecycle** (section 3): not built, so "every entry point requires
  `ak_init`" is unexercised, exactly as in the rust slice.
- **The unknown-field bag** (decision 11): the groups exist in the header and nothing in this
  slice fills them. The rust slice answered decision 11 and this slice does not re-ask it.
- **The `ak_span.coder` hint** (decision 4): present, read by nothing.
- **Message size and recursion limits** (decisions 7 and 8): unset and unenforced.
- **The direct-argument path** (section 8): built and byte-identical on P5.x. C++ has nothing
  to pin, so it confirms the path works and says nothing about the 0.16-to-0.34 JVM claim.
- **A C++20 coroutine surface** (README 5.1.2): a sketch only, and the sketch is that
  `ak_call_unary_cb`'s completion callback is the one primitive a coroutine surface needs —
  `co_await` over a handle whose `await_suspend` stores the coroutine handle in `user_data`
  and whose completion resumes it, as free functions beside the installed class rather than
  as members of it. **Not built**, by instruction.

### Measurement coverage
- **Concurrency**: one thread everywhere in the codec arms. ABI v1 obligation 12.5's suite
  does not exist here either.
- **Allocation and footprint**: nothing counts allocations or peak memory in any arm.
- **The guard** is priced as a whole-binary arm (`bench_a17_noguard.log`), not per accessor.
- **upb** is not built: protobuf 3.21's upb is an internal implementation detail with no
  installed public C API in `libprotobuf-dev`, so the "fastest measured comparator" column
  does not exist in this slice.
- **Hardware**: one 4-vCPU container, one microarchitecture, one compiler (g++ 13.3.0).
  clang++ 18 is installed and unused, so "the C++11 floor builds" is a g++ fact.

## Slice-specific notes

- Reads `packages/cpp` and `ffi/poc/rust`. Writes neither. The rust generator is imported
  read-only and the rust crates are path dependencies, which is what makes the two columns
  two hosts over one core.
- The facade names no `std::optional`, `std::variant` or `std::string_view`. The oneof is a
  generated C++11 sum type over an unrestricted union: `clear`, `copy_from` and one `set_`
  per member, about 60 emitted lines for five variants against Rust's 14. That cost is real
  and it is what buys one ABI across every `-std` a consumer might pick.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `logs/cpp/conformance.log` | six builds: C++17 target/floor, C++14, C++11, static, lossy | R2. 361 checks, 0 failures, six times. The incumbent's P2.5 map-value divergence, and that protobuf C++ rejects malformed UTF-8 |
| `logs/cpp/boundary.log` | the built artifacts | R5 both halves, 10 checks. The shared arm is a real dynamic import; the static arm's entry points are called; the control is not fused; `-flto` over a Rust staticlib cannot inline the entry points |
| `logs/cpp/odr.log` | one C++11 TU and one C++17 TU, linked | README 5.1's hard stop: 144 facts, 0 moved, and 49 moved under the positive control |
| `logs/cpp/calibration-r13.log` | the rust slice's own bench, built and run here | R13. This machine's rust crossing: 1.5 ns forward, 2.1-2.2 forward+reverse |
| `logs/cpp/counts.log` | the counting core, shared and static | R5. 9 and 6 crossings for 1,000 M1 rows; 10.024 and 7.004 per M2 element; 3 per 200 M3 elements. Unbatched: 125 and 311 forward per element on P2.3 and P2.4 |
| `logs/cpp/bench_a17_shared.log` | **arm a**, C++17 target, shared library, guard on, reject, ASCII, 9 rounds | **the C++ column.** Plus decision 1's three mechanisms as within-round deltas, the group fill alone, the two-pass blob write, and protobuf's deterministic-serialisation cost |
| `logs/cpp/bench_a17_static.log` | arm a, **static linkage** | the second column, never divided by the first. The crossing is 1.23-1.31 ns against 1.82-1.84 |
| `logs/cpp/bench_b17_shared.log` | **arm b**: floor implementation at the target level | what the floor's missing APIs cost, runtime held constant: nothing measurable |
| `logs/cpp/bench_c11_shared.log` | **arm c**: floor implementation at `-std=c++11` | what a pinned consumer actually gets. Quoted as a standalone number |
| `logs/cpp/bench_c14_shared.log` | floor implementation at `-std=c++14` | README open question 3: `packages/cpp`'s current level |
| `logs/cpp/bench_a17_noguard.log` | arm a with ABI v1 section 5's guard OFF | what the guard costs |
| `logs/cpp/bench_a17_lossy.log` | arm a with the decode UTF-8 check OFF | ABI v1 decision 3's decode half. Validating costs 22-28% of a decode in C++, unlike Rust, because a C++ "lossy" arm does not validate at all |
| `logs/cpp/rpc.log` | grpc++ 1.51.1, tonic 0.14, loopback, no TLS, in-process server, P2.2 | the RPC arm. Client CPU per RPC 0.558 to 0.652 of grpc++ at 1, 8 and 16 in flight, with the codec half separated in-process, and R9's wall-clock hazard visible rather than hidden |
