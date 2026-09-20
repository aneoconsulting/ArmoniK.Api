# java slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **W6 built and measured; re-gated on the shared core (W10) and re-headlined under R14.** Full codec, both Java levels, seven arms through the correctness gate on all three content sets (1,297 checks, 0 failures), the encode verdict taken, the batching prediction tested, decisions 9 and 13 answered for a managed host, README R9's hazard measured and section 9's virtual-thread amendment confirmed. **The RPC arm is built only as the pinning question**; there is no grpc-java *transport* comparison, though R14 now puts grpc's marshaller in the codec baseline (below). |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | Java 8, `openjdk 1.8.0_502`. Builds, and passes all 437 correctness checks on the Java 8 runtime. |
| **Target** (where the clock runs) | JDK 17 (`17.0.20`) with the JNI back end |
| **Incumbent** | protobuf-java **3.25.5**, which is what `packages/java`'s pins actually resolve to: the pom declares 3.19.0 and grpc-java 1.74.0 brings 3.25.5, and the resolved one is what a consumer runs. protoc 3.19.0, the pinned one, generates the classes. |
| **Core** | the shared crate at `poc/codec/crates/ak-core` (R0). This slice defines no core of its own; its two transcoders live there and are resolved from there, checked with `ldd` and `nm` on the loaded artifact (`w10-regate.log`). |
| **R13 calibration** | this machine's rust-slice crossing is **2.1 ns** (`calibration-r13.log`), against 1.8 ns in the rust slice's container and 1.5 in the cpp slice's. Intel Xeon at 2.10 GHz, 4 vCPU, 15 GB, in a container, no pinning. |

## The question this slice answers, and the answer

**Does the encode regression survive ABI v1, and does the generated-Java-codec arm stay
ahead?**

**The encode regression does not survive. The decode half of the published verdict does.**

| direction | the C ABI (`ffi`) | the generated Java codec (`R`) |
|---|---|---|
| **encode** | **0.58 to 0.96** of protobuf-java on every real element-bearing payload | 0.66 to 0.98 on the same set |
| **decode** | **1.22 to 1.62 on every M2 payload**, 0.83 to 1.04 on the flat ones | **0.39 to 1.05**, and it is faster than the C ABI on all five M2 payloads, with a clean sign on three of them |

So the published sentence "a generated pure-Java codec beats the C ABI in both directions"
is **half reproduced**: it does on decode, decisively and for a reason that is measured
rather than inferred, and it does not on encode.

**What moved the encode column is a baseline, not the ABI**, and it is the most important
thing in this document. See "the incumbent was flattered twice" below.

**R14 was applied after the fact and did not change it.** The rule arrived with W10: the
baseline is the path gRPC's marshaller takes, not the library's best entry point. Measured
against the real `ProtoLiteUtils` marshaller the C ABI encodes at **0.60 to 0.88** on the
element-bearing payloads where against `toByteArray` it is 0.58 to 0.96, and decode is
unchanged in shape. What the rule *did* surface is that **the path production takes is 1.5
to 1.9 times slower than the `toByteArray` every benchmark reaches for**
(`logs/java/r14-summary.md`).

## What exists

Everything is emitted from one description by `gen/generate.py`, which imports the rust
slice's `ir.py` and `rust_abi.py` and the cpp slice's `cpp_header.py` and `cpp_layout.py`
**read-only**, so the core behind the C ABI is the one the other two slices measure.
Three hosts, one core, and the crossing counts agree to the digit.

| | |
|---|---|
| `gen/` | the generator: 10 backends over the shared IR, plus `build.sh` and seven measurement scripts |
| `src/java/ak/` | hand-written runtime: `Values`, `Utf8`, `Utf8View`, `Enc`, `Dec`, `Mem`, `Arena`, `Str17`, `Native`, `Callbacks`, the harnesses |
| `src/generated/java17/`, `src/generated/java8/` | **two emitted source trees, one per level.** README 5.1's first condition, and Java has no preprocessor so it is literal here |
| `src/generated/*/ak/floor/` | the floor binding emitted into a package of its own, so README 5.2 arm b is paired inside one process |
| `src/generated/*/ak/borrow/` | open decision 13's borrowed facade |
| `native/generated/shim.c` | the JNI half: 52 entry points, 18 encode-loop trampolines, 32 decode trampolines, the section 5 guard |
| `core/` | the core crate: the rust slice's runtime plus **`ak_tc_utf16` and `ak_tc_latin1`** |

### The arms

| arm | what it is |
|---|---|
| `pbj` | protobuf-java, **a message serialised once**, `toByteArray`. The baseline every ratio is against |
| `pbj-loop`, `pbj-reuse`, `pbj-det`, `pbj-parsed`, `pbj-reused-out` | the incumbent's other modes, each its own arm, because the choice between them is worth more than the effect being measured |
| `R` / `R-take` | the generated pure-Java codec. README R3's no-boundary control **and** section 13 outcome 2's architecture |
| `ffi` / `ffi-take` | the C ABI, batched, the core transcoding |
| `ffi-nobatch` | the host declines to batch (ABI v1 section 6 allows it) |
| `ffi-zeroed` | open decision 9's candidate fill |
| `ffi-borrow` | open decision 13's borrowed facade, a decode arm |
| `floor-batched` | the Java 8 binding, in the target's process: README 5.2 arm b |

## Correctness

**Established, and it gates everything.** `logs/java/conformance.log`: **437 checks, 0
failures**, across seven arms and all 16 payloads, on all three of README 5.2's arms
(target on target, floor on target, floor on floor).

- **byte identity against `schema/generated/manifest.json`** on every payload, plus the
  committed vectors byte for byte, not only their hashes.
- **P2.5 has two valid encodings and this slice produces both.** The facade arms write the
  canonical 19,632 B; **protobuf-java writes the both-fields-written form at 19,712 B**,
  which is exactly +2 B per emptied map value and makes it the third independent Google
  runtime to do so after protobuf C++ and upb. The gate does not waive this: it checks the
  full cross product, each arm parsing the other's bytes and re-encoding to its own form,
  which is stronger than the byte identity it replaces.
- **P7.1** is validated by decoding it and re-encoding contiguously to a permutation of the
  same (tag, wire type, body) triples, as design/SHAPES.md asks.
- **The unknown-field vectors** are in `logs/java/unknown.log`: 22 vectors, 66 checks, 0
  failures. See decision 11 below.
- **ABI v1 section 10's layout guard is exercised, not specified.** All 380 facts agree
  between the core's export and this slice's hand-computed table, and
  `gen/layout_break.sh` shows the comparison failing and naming the fact when one is
  perturbed (`logs/java/layout-guard.log`).
- **R5's boundary check**: `logs/java/boundary.log` shows every ABI entry point as an
  undefined import of the shim, resolved by the core at load.

## The numbers, each with the log that carries it

**Read them as signs and size classes.** README R13: absolutes are instrumentation until
the controlled physical run. The across-build drift bar for this slice's delta instrument
is **0.078** (`logs/java/drift.log`), and which conclusions clear it is tabulated there.

### Crossing counts, from the counting core -- `logs/java/counts.log`

**Already final**: a property of the interface, not of the machine.

| payload | encode fwd / rev | per element | decode fwd / rev | per element |
|---|---|---|---|---|
| P1.2 (1,000 M1 rows) | 8 / 1 | 0.009 | 1 / 5 | 0.006 |
| **P2.2 (500 M2 elements)** | 2,511 / 2,501 | **10.024** | 1 / 3,501 | **7.004** |
| P2.3 | 629 / 626 | 10.040 | 1 / 876 | 7.016 |
| P2.4 | 403 / 401 | 10.050 | 1 / 561 | 7.025 |

**10.024 and 7.004 are the rust slice's figures to the digit, and nine and six for a
thousand M1 rows is its other pair.** Third host, same interface, no re-derivation.

Unbatched, the same payloads cost **22.0, 130.0 and 316.0 crossings per element**, which
is what the batching delta below is a measurement of.

### Decision 5, the learned length-placeholder width -- `logs/java/counts.log`

**Reproduces the rust slice exactly.** Zero prefix misses and zero bytes moved on every
uniform payload from a warm context; on P2.4, built so a per-site width is wrong on every
element, **80 misses -- one per element -- moving 979,181 bytes of a 979,465-byte output**.
**Zero grow-callback invocations on any payload**, which is what handing the transcoder the
whole remaining buffer was meant to buy.

### This machine's crossing prices -- `logs/java/crossing.log`, `calibration-r13.log`

| path | ns, min to max | as a multiple of this machine's rust crossing (2.1 ns) |
|---|---|---|
| JNI forward, through the slice's own shim | 11.9 to 12.9 | 5.7 to 6.1 |
| JNI forward, a bare no-op | 11.0 to 12.8 | 5.2 to 6.1 |
| **JNI upcall, method id cached (what the binding emits)** | **75.8 to 91.0** | **36 to 43** |
| JNI upcall, resolved per call | 295.9 to 309.4 | 141 to 147 |

The published 11.2 ns forward and 98.4 ns crossing (README section 2) both reproduce as
size classes. **The naive upcall is four times the cached one**, which is why the shim
caches method ids at load and the trampolines carry their slot as a constant.

### Encode -- `logs/java/encode.log`

Median of the paired per-round ratio to `pbj`. 36 rounds, arm order rotating so every arm
occupies every position equally often, every arm reading its own pool of distinct source
objects.

| payload | `ffi-take` | `R-take` | `pbj-loop` | `pbj-parsed` |
|---|---|---|---|---|
| P1.1 | 0.802 | 0.893 | 0.517 | 1.016 |
| P1.2 | 0.738 | 0.975 | 0.529 | 1.003 |
| **P1.3 (absent path)** | **1.967** | 0.273 | 0.438 | 0.987 |
| P2.1 | 0.957 | 0.869 | 0.516 | 1.026 |
| **P2.2 (the shape the control plane moves)** | **0.875** | 0.830 | 0.538 | 1.030 |
| P2.3 | 0.643 | 0.788 | 0.484 | 1.019 |
| P2.4 | 0.708 | 0.899 | 0.525 | 1.019 |
| P2.5 | 0.930 | 0.868 | 0.514 | 1.052 |
| P3.1 | 0.576 | 0.738 | 0.457 | 1.027 |
| P4.1 | 0.760 | 0.664 | 0.433 | 0.991 |
| P6.1 (packed control) | 1.173 | 0.723 | 0.610 | 1.014 |

`ffi-take` and `R-take` hand back a fresh `byte[]`, as `pbj` does. P1.3 is the absent-path
inversion and decision 9 fixes it; P6.1 is the control twice over (see "storage" below).

### D7 and the re-measurement -- `logs/java/encode-take-fix.log`

**The `-take` arm was handicapped against itself**, which is the first of the three traps
the brief named, pointing inward. `takeBytes` asked the core for the encoded length over
the boundary, copied the bytes from native memory into a reused scratch array, then copied
the scratch array into the result: two crossings and two copies, where `R-take` and
`toByteArray` each cost one allocation and one copy. Both extras were avoidable. ABI v1's
encode entry already returns the length, so the entry points record it and `encTake` now
writes straight into the freshly allocated result. Re-gated before re-measuring: 1,297
checks, 0 failures, unchanged, and the gate runs `take()` on every payload and all three
content sets.

**The fix is mechanical and its measured effect is below this machine's noise floor.** The
instrument is the within-run paired delta `arm-take - arm`, median ns:

| payload | `ffi-take - ffi` published | re-run | `R-take - R` published | re-run |
|---|---|---|---|---|
| P5.2, 64 KB | 11,866 | 9,080 | 11,807 | 9,227 |
| P5.3, 1 MB | 269,687 | 211,277 | 202,726 | 180,648 |
| P5.4, 4 MB | 938,102 | 733,547 | 698,122 | 573,972 |

`R-take - R` is unchanged code, so its movement is run-to-run noise and it is **124 us on
P5.4**. The ffi arm moved 204 us. The 80 us difference is what the fix can claim, against a
noise floor half again as large.

**So the published `-take` column was not materially distorted, and the earlier reading of
why P5.3 and P5.4 are above 1.5 was wrong.** Decomposing P5.4 across the two runs:
allocating and zeroing the 4 MB result costs roughly 530 us and each copy roughly 200 us.
Both arms pay the allocation identically, so it cancels; the copy that was removed was
about a fifth of the take overhead rather than most of it. The residual gap between the
arms -- 160 us in the re-run against 240 us published -- is `SetByteArrayRegion` reading
cold off-heap memory where arm R does an on-heap `System.arraycopy`. **That part is the
ABI's own cost and does not go away.** `ffi-take` over `R-take` on P5.4 is 1.535 published
and 1.538 now.

**Both logs are kept and `encode.log` remains the cited one.** The re-run drifted harder
(`pbj`'s own absolute moves -24% to +15% across payloads between the two) and three sign
verdicts weakened to no-sign at unchanged magnitudes: P1.2 from `+`, P5.3 and P5.4 from
`-`. That is a noisier run, not new information, and R13 defers absolutes to a controlled
machine either way.

### Decode -- `logs/java/decode.log`

| payload | `R` | `ffi` | `ffi-borrow` |
|---|---|---|---|
| P1.1 | 0.913 | 1.038 | 0.882 |
| P1.2 | 0.939 | 0.830 | **0.669** |
| P1.3 | 0.394 | 0.904 | 0.947 |
| P2.1 | 0.845 | 1.621 | 1.527 |
| **P2.2** | **0.838** | **1.441** | 1.382 |
| P2.3 | 1.008 | 1.260 | 1.072 |
| P2.4 | 1.051 | 1.225 | 1.004 |
| P2.5 | 0.829 | 1.618 | 1.548 |
| P3.1 | 0.765 | 0.831 | 0.694 |
| P4.1 | 0.690 | 1.341 | 1.308 |

**`R` minus `ffi` is negative on all five M2 payloads** -- -1,289 ns per element on P2.1,
-1,163 on P2.2, -1,334 on P2.3, -1,806 on P2.4 and -1,200 on P2.5 -- **with a clean sign
over 40 rounds on P2.1, P2.2 and P2.5**; P2.3 and P2.4 straddle zero. **7.004 upcalls at
about 80 ns is 560 ns of it.** That is a decomposition, not a ratio, and it
is the concrete argument for ABI v1 7.1's **pull family, which the core does not
implement**.

### The batching predicate -- `logs/java/delta.log`

Measured with `ak.RunDelta`: no incumbent, no pool churn, nothing allocating between
rounds, two configurations of one binding differing by one boolean.

**The cpp slice's prediction is confirmed and the magnitude is three times what the prior
Java report measured.** Batching wins on **every payload with a repeated field**, the
median is positive in all three drift builds, and the six payloads that clear the 0.078
bar are P1.3, P2.2, P2.3, P2.4, P2.5 and P3.1.

| payload | extra crossings/element unbatched | predicted at 11 ns | measured, ns/element | as a share of the encode |
|---|---|---|---|---|
| P1.2 | 0.99 | 10.9 | 5.9 | 1.7 % (below the bar) |
| **P1.3** | 0.99 | 10.9 | **10.9** | 16 % |
| P2.2 | 11.98 | 132 | 540 | 9.7 % |
| P2.3 | 120.0 | 1,320 | 4,126 | 25 % |
| P2.4 | 306.0 | 3,366 | 8,659 | 29 % |
| P2.5 | 11.95 | 131 | 373 | 14 % |

**On the flat M1 payloads the measured delta per element is the extra crossing count times
this machine's forward crossing price, to within a tenth.** On the nested shapes it is two
to three times that, because an unbatched element run also repeats the per-call work the
chunk amortises. **So 10 to 29 percent of an encode, against the published 2 to 9.**

### Open decision 9, the sparse fill -- `logs/java/delta.log`

**It generalises to Java, and that is what the decision asked of a managed host.**

- **P1.3, the absent path: 46 ns per element, 0.32 of the total fill, clean sign over 40
  rounds and clear of the drift bar by an order of magnitude.** The absent-path inversion
  goes from `ffi-take` 1.967 to 0.734 of protobuf-java.
- **Every other payload: within the drift bar**, medians between -0.04 and +0.02 of the
  total fill. The condition decision 9 set -- wins on the absent path, costs less than
  5.4 ns per element elsewhere -- is met.

Rust, C++ and now Java agree, and Java was one of the two slices the decision was waiting
on.

### Open decision 13, borrowed spans -- `logs/java/decode.log`

**Real on the JVM, and worth about a third of what it is in C++.**

| payload | `ffi` | `ffi-borrow` | delta, ns/element | sign |
|---|---|---|---|---|
| P1.1 | 1.038 | 0.882 | 43.3 | + |
| P1.2 | 0.830 | **0.669** | 38.7 | + |
| P2.4 | 1.225 | 1.004 | 2,355 | + |
| P2.2 | 1.441 | 1.382 | 109 | straddles zero |
| P2.3 | 1.260 | 1.072 | 1,000 | straddles zero |

On P1.2 the borrowed facade removes **16 percent of a protobuf-java decode**; the C++ slice
measured the same mechanism taking P1.2 from 0.562-0.790 to 0.234-0.241, which is two to
three times more. On the container-heavy P2.2 it barely moves, which is the branch's own
finding that decode is bounded by host-side container construction, seen from a third host.

**Three hosts, one mechanism, and the managed host gets less of it.** That is a fact the
lifetime contract has to be drafted against, not a reason to drop it.

### R14: the headline against gRPC's marshaller -- `logs/java/r14-summary.md`, `r14.log`

R14 arrived with W10 and lands on the baseline every other table here uses. The arm calls
the **real** `io.grpc.protobuf.lite.ProtoLiteUtils` marshaller, and its path was read from
the bytecode rather than remembered: encode is `getSerializedSize()` then
`writeTo(OutputStream)` through a 4 KB `CodedOutputStream`; decode reads into a thread-local
array and parses from it, with a fast path that returns the same object if handed back its
own stream, which the arm avoids.

| | against `toByteArray` (the other tables) | **against the marshaller (R14's headline)** |
|---|---|---|
| encode, `ffi`, element-bearing payloads | 0.58 to 0.96 | **0.60 to 0.88** |
| decode, `ffi`, M2 payloads | 1.22 to 1.62 | **1.22 to 1.62** |

**The verdict does not change.** What R14 surfaces instead is about the incumbent:
`toByteArray` is **0.48 to 0.65** of the marshaller path, so **the entry point every
benchmark reaches for is roughly twice as fast as the one an application takes**, and none
of the branch's three published reports says which it measured. On decode the marshaller
costs only 2 to 8 percent over `parseFrom`, so the decode tables did not need re-taking --
now a measurement rather than a claim.

That table is noisier than the main ones (24 rounds, a pool rebuilt every round, a fourth
arm) and its conclusions rest on its medians agreeing with them, which they do on every
element-bearing payload.

### The content sets -- `logs/java/contentsets.log`

design/SHAPES.md: "A slice that reports one string-path number without saying which content
set it came from has reported half a number." All three sets are in the **correctness**
gate on every payload (1,297 checks, 0 failures); P1.2 and P2.2 are also timed on all
three. On the JVM these are not only a cost: ASCII and Latin-1 are both the compact LATIN1
coder, so the target's fast path applies to both, and above U+00FF a `String` becomes UTF16
and the binding stages twice the bytes through a different core transcoder.

| | ASCII | Latin-1 | above U+00FF |
|---|---|---|---|
| **P1.2 encode**, `pbj` absolute | 478 us | 614 us | 1,333 us |
| P1.2 encode, `ffi-take` | 0.763 | 0.734 | **0.551** |
| P1.2 decode, `ffi` | 0.820 | 0.932 | **0.694** |
| P1.2 decode, `ffi-borrow` | 0.664 | **0.483** | 0.524 |
| **P2.2 encode**, `pbj` absolute | 2,185 us | 2,907 us | 5,004 us |
| P2.2 encode, `ffi-take` | 0.800 | 0.751 | **0.609** |
| P2.2 decode, `ffi` | 1.484 | 1.401 | **1.049** |

**The C ABI gets relatively better as the content widens, in both directions**, because the
incumbent's own cost rises faster than the core's: protobuf-java's P1.2 encode goes from
478 to 1,333 microseconds across the three sets and the C ABI arm's from 365 to 735. The
decode regression on P2.2 shrinks from 1.48 to 1.05 for the same reason.

**Read the JDK 17 rows of `logs/java/deopt.log` before quoting any wide number**, though:
on that runtime protobuf-java's wide encode is bistable by a factor of two depending on
what the process read first, and the figures above are the slower state.

### README 5.2, the floor as three arms -- `logs/java/floor.log`

**The floor costs 42 to 77 percent of an encode, and only where there are strings.** One
cause: the target reads `String.coder` and `String.value` and hands the core the string's
own compact storage; the floor has no compact form and stages through `getChars` as UTF-16,
which is two copies where the target does one and 72 bytes where the target does 36.

Arm b is measured **inside arm a's process**: the floor binding is emitted into `ak.floor`
over the same facade types, so the floor-against-target ratio is paired inside one round
rather than formed across two processes.

| payload | a: target/target, ns | b/a, paired | sign | c: floor/floor, ns |
|---|---|---|---|---|
| P1.1 | 1,230 | 1.660 | + | 2,117 |
| P1.2 | 343,594 | 1.607 | straddles | 1,142,314 |
| **P1.3 (no strings)** | 20,491 | **1.009** | straddles | 12,054 |
| P2.1 | 2,634 | 1.470 | + | 4,039 |
| P2.2 | 2,795,843 | 1.422 | + | 4,500,371 |
| P2.3 | 2,108,133 | 1.527 | straddles | 3,574,220 |
| P2.4 | 2,523,131 | 1.580 | + | 4,101,797 |
| P2.5 | 53,240 | 1.543 | + | 94,551 |
| P3.1 | 19,237 | 1.774 | + | 34,979 |
| P4.1 | 221,302 | 1.429 | + | 380,498 |
| **P5.2 (bulk bytes)** | 3,715 | **1.020** | straddles | 4,506 |
| **P5.4 (bulk bytes)** | 614,089 | **0.999** | straddles | 612,839 |
| **P6.1 (packed control)** | 271,223 | **1.030** | straddles | 281,031 |

**The rows that do not move are the ones with no strings to stage**: the absent path, the
bulk-bytes path (which goes through ABI v1 section 8's direct argument and never stages at
all) and the packed control. That is the positive control for the cause, inside the same
table.

**Arm c stands alone**, as README 5.2 requires: it is what a pinned consumer gets, and it
is not a ratio against arm a. Its correctness is what matters and it passes all 437 checks.

**And two negative controls.** In the arm-b and arm-c runs the classpath is the Java 8
tree, where `ak.floor` and `ak.shapes` are the *same emitted source*, so their arm-b blocks
must read zero. They do: worst median **4.06 %** on the target runtime and **2.82 %** on the
floor runtime, median across payloads 0.62 % and 0.99 %, and **not one row of thirty has a
clean sign**. The instrument's noise floor is one to four percent and the floor's effect is
forty-two to seventy-seven.

### README R9's measurement hazard, and the one result that argues for the design -- `logs/java/deopt.log`

R9 says a single `String.format` with a numeric conversion permanently deoptimises every
char narrowing loop on JDK 21 and later, protobuf-java's encoder among them. This slice
tested it. **The hazard is real and bigger than stated, and four of the five things the
rule says about it do not hold here.**

On JDK 17, above U+00FF, P1.2, reading a **Latin-1** String's characters before the first
measurement (which one `String.format` does internally) changes the absolutes like this:

| arm | before | after | |
|---|---|---|---|
| `pbj` and every other protobuf-java arm | 1,297,600 ns | 601,283 ns | **2.16x FASTER** |
| **`R`**, the generated Java codec | 598,193 ns | 757,617 ns | **1.27x slower** |
| **`ffi`**, the C ABI | 674,752 ns | 642,772 ns | **unmoved** |

Stable over four repetitions each. What the rule gets wrong: it reproduces on **JDK 17 and
not on 21**; the trigger is **any read of a Latin-1 String's chars**, not a numeric
conversion, and reading a *wide* String's chars does nothing; **no narrowing loop is
involved**, since modes 2 and 3 differ only in the probe string's coder; the incumbent gets
**faster, not slower**; and **nothing happens on ASCII at all**, on either JDK, which is why
every published managed figure has been blind to it.

**The mechanism is now settled from the JIT's own output, and the inference above was
wrong. See the next section.**

**And the immunity is a result.** The C ABI arm has no such loop, because ABI v1 section 4
put the transcoder in the core so that "every managed host stops maintaining a UTF-8
encoder". That makes it the only arm here insensitive to the host JIT's profile history --
an argument for the design that no benchmark was looking for.

### R9's mechanism, settled -- `logs/java/r9-mechanism.log`

`-XX:+TraceDeoptimization` and `-Xlog:deoptimization` do not exist on a product build.
`-XX:+LogCompilation` does, and it **preserves** the effect; `-XX:StartFlightRecording`
**erases** it (642 us at `deopt=0` against 1,261 without). So the first question was which
instrument the hazard survives being watched by, and the answer decided the rest.

**1. It is branch pruning, not deoptimisation.** `Utf8$UnsafeProcessor.encodeUtf8` has two
`String.charAt` sites. Every run inlines `isLatin1()` and `StringLatin1.charAt` at both. In
every slow run C2 emits `inline_fail reason='call site not reached'` for
`StringUTF16.charAt` at both sites; in every fast run it compiles that branch with the
`_getCharStringU` intrinsic. The payload's strings are all above U+00FF, so the pruned
branch is the one the measurement needs. Six logged runs, perfect correlation. Runtime
`uncommon_trap` events over the whole process are 9 at `deopt=0` and 11 at `deopt=3` --
**the slow state has fewer traps, not more.**

**2. The shared-profile story is refuted.** `ak.Utf8.encode` and `ak.Utf8.length` compile
**identically in all four modes** in the same logs: no pruning, the UTF-16 branch present,
`_getCharStringU` applied, every time. Only protobuf-java's encoder is pruned, so the two
encoders do not share a fate and whatever moves arm R in `deopt.log` is not this.

**3. The effect is bimodal and probabilistic, not a penalty.** Ten runs per mode, P1.2,
`pbj` us:

| mode | min | median | max | landed in the fast state |
|---|---|---|---|---|
| `deopt=0` nothing | 595 | 1,257 | 1,276 | **2 of 10** |
| `deopt=1` `String.format` | 1,246 | 1,273 | 1,283 | **0 of 10** |
| `deopt=2` wide probe | 603 | 1,224 | 1,268 | **1 of 10** |
| `deopt=3` Latin-1 probe | 617 | 634 | 648 | **10 of 10** |

Every run lands at about 620 us or about 1,250 and the gap is empty. **The probe changes
the probability of the branch surviving, not the cost when it does not.** Without a probe
the process gets there by itself about one run in ten -- which is what `deopt.log`'s one
stray mode-2 reading was, recorded at the time rather than dropped.

**4. `deopt=1` no longer reproduces**, and that is the mode R9 names. `deopt.log` has four
consecutive fast readings for it; here it is slow in 13 of 13. The payload restriction does
not explain it: over the full payload set mode 1 reads 923 to 954 us while mode 3 reads 606
to 630. What changed since `deopt.log` is the W10 re-gate and the D7 fix, both of which
change code in the measured process; attributing it needs a bisection over a probabilistic
outcome, so it is recorded as unexplained rather than guessed at.

**5. One prediction the pruning account makes, and it holds.** Over the full payload set
the *slow* value drops from about 1,250 us to about 920: a more varied string diet before
P1.2 partly saves the branch.

### README 9.1's C shim, priced and refused -- `logs/java/shim-probe.log`

**The arm is not built, and the reason is a measurement rather than a schedule.** README
9.1's shape is a generated C shim that speaks the host runtime's C API instead of calling
back into the host language. The JVM analogue writes facade fields through the JNI API
instead of upcalling into Java, and it is aimed at exactly this slice's decode regression:
7.004 reverse calls per element on P2.2 at about 80 ns is 560 ns of a 1.441 ratio. Pricing
the primitives first says it cannot work.

| op, net of an empty-loop control inside one native call | G1 (default) | Parallel | Serial |
|---|---|---|---|
| `SetIntField` | 11.97 | 11.97 | 11.96 |
| `SetObjectField` | **26.7** | 13.7 | 13.7 |
| `SetObjectArrayElement` | **28.0** | 15.6 | 15.5 |
| `GetObjectField` | 18.6 | 16.2 | 16.1 |
| `AllocObject` | 47.7 | 51.3 | 52.9 |
| `NewObject` | 139.2 | 127.2 | 123.5 |
| `NewString(16)` | 95.1 | 90.2 | 89.7 |
| an upcall on a container method (`List.set`) | 105.8 | 101.8 | 99.7 |

A cached reverse call on this machine is 72 to 80 ns (`logs/java/crossing.log`), so **a JNI
field store is a third of a whole upcall, not a rounding error against it.** The crossover
between "one upcall carrying k stores in bytecode" and "k JNI stores and no upcall" is at
**k = 2 to 3**, worst on the default collector. `TaskDetailed`'s apply is k = 30, where the
shim would pay about 790 ns of stores against about 80 ns of transition plus the same
stores at a few ns each in bytecode, and a further 123 to 139 ns per element for
`NewObject` where the Java side pays a bytecode `new`.

**The generalisation is the useful part, and it composes with the rust slice's pull
family.** On the JVM the push family's cost is the *number* of transitions, not what
happens inside them. Making a transition cheaper is not on the table, because a JNI
accessor already costs a third of one; making them fewer is, and that is the pull family
and open decision 10. The C-shim route is not a second, independent way to the same place.

**And a fact worth keeping on its own**: `SetObjectField` and `SetObjectArrayElement` both
double under G1 against Parallel and Serial while `SetIntField` does not move. That is the
G1 write barrier priced, and it applies to any native code storing references into Java
objects, not only to this design.

### ABI v1 section 9's virtual-thread amendment, confirmed -- `logs/java/pinning.log`

Section 9's fourth amendment: "At least one mode in which the caller waits in the host
language. Blocking in a native frame from a virtual thread pins its carrier; what fixes
that is parking in Java on a future, which the callback mode already provides." No slice
had measured it, and it is the one item on README's RPC list that **only a JVM slice can
answer**. It needs no RPC stack: the question is where the waiting happens.

Eight virtual threads each waiting 300 ms, on a scheduler with a known parallelism. If the
carrier is pinned the run takes `ceil(N/P) * W`; if not, `W`.

| carriers | predicted if pinned | **blocking in the native frame** | **parked on a future** |
|---|---|---|---|
| 1 | 2,400 ms | **2,420 ms** | **306 ms** |
| 2 | 1,200 ms | **1,222 ms** | **304 ms** |
| 4 | 600 ms | **622 ms** | **305 ms** |

**The blocking mode scales exactly as the pinned prediction and the callback mode is flat.**
So the amendment is right, and its consequence is the one the specification draws: the
callback mode is not a convenience, it is what makes the ABI usable from the idiom Java is
moving to. Virtual threads are a JDK 21 API, above this slice's JDK 17 target, which is the
right place for the question -- it is about what the ABI must offer a host that has them.

### The incumbent was flattered twice, and a third hypothesis was refuted

`logs/java/baseline.log`, and JOURNAL J7 and J8. This is the part of the slice most likely
to change somebody else's number.

1. **protobuf-java memoizes `getSerializedSize()` on the instance.** Both `toByteArray` and
   `writeTo` call it, so a loop over one message pays the size pass -- a full walk
   computing every string's UTF-8 length -- once and amortises it, while every other arm
   pays its own every iteration. Measured directly: the cold size pass is **1.5 to 2.8
   times the write** on every element-bearing payload.
2. **The decode baseline was handicapped by the harness**, not by the library: keeping the
   parse alive with `Message.hashCode()` charges a second full traversal of the decoded
   tree that no other arm pays. `System.identityHashCode` replaced it.
3. **Refuted rather than corrected**: a pool built by `parseFrom` looked like it must
   flatter the incumbent, because a parsed message's string fields hold `ByteString` and
   `writeTo` writes the raw object, so it should re-serialise with no transcoding at all.
   Built as its own arm, `pbj-parsed` is **0.98 to 1.05** of a built message on every
   payload. The effect is not there and the correction it would have justified is
   withdrawn.

**And the published regression is reconstructible from (1).** Dividing this slice's
`ffi-take` by its `pbj-loop` gives **1.26 to 1.85**, against a published **1.08 to 1.84**
(`logs/java/encode.log`, last section). That is a mechanism that would produce the
published shape, measured here; it is not a claim about what the prior harness did, and
nothing can settle that because its sources do not survive.

## Open defects

| # | what | status |
|---|---|---|
| D1 | `Dec.skip` used `pos += readLen()`, which in Java caches `pos` before the varint is consumed, so an unknown length-delimited field skipped into its own body | **fixed.** Reachable only from an unknown field, which is what README 10.1 says a corpus generated from the schema that reads it never contains |
| D2 | the decode `apply` constructed a singular message child from the group, discarding what a run had already attached to it (ABI v1 open decision 10) | **fixed.** Encode stayed byte-identical on all 16 payloads while it was live; only the decode round trip saw it |
| D3 | the bench's protobuf-java baseline amortised the size pass over the loop | **fixed**, and it is the largest correction in the slice |
| D4 | the bench kept the parse alive with `Message.hashCode()` | **fixed** |
| D5 | `RunDelta`'s warmup ran 64,000 encodes per configuration per payload | **fixed**; it warms to a fixed time |
| D6 | the `ffi` encode arm called `encodedLength()` for a value the JIT could not discard, adding a forward crossing per operation | **fixed**; the entry point returns its own result |
| D7 | `takeBytes` paid two crossings and two copies where the incumbent pays one allocation and one copy: the same redundant crossing D6 removed from the `ffi` arm, left standing in the arm the incumbent is actually compared against | **fixed**, and re-measured. The effect is below the noise floor and no published figure moves; see `logs/java/encode-take-fix.log` |

**None open.**

## What is not measured

- **The RPC arm is not built**, with one exception. There is no grpc-java comparison here:
  no CPU per RPC, no allocation per RPC, nothing at 1, 8 and 16 in flight. What the branch
  already knows is stronger than what this slice would have added -- the rust slice
  established two crossings per call and zero per field **by reading the code**, and ABI v1
  section 9's arithmetic makes the transferable claim host-independent (196 ns of JNI
  against about 1.5 ms of CPU is 0.013 percent). **The exception is the virtual-thread
  pinning question**, which is the one item on that list only a JVM slice can answer and
  which is now measured (above). The **completion-queue delivery mode** is still unbuilt,
  and section 9's claim that "a thread parked in a drain is in native state and costs a
  collection nothing" is still unmeasured.
- **FFM is not built as a binding**, and the downcall price is measured
  (`logs/java/ffm.log`). It is a JDK 22 API; this container has JDK 8, 17 and 21 and the
  proxy does not reach a JDK distributor, so it is measured on **JDK 21 with
  `--enable-preview`**. On the same JDK: a plain FFM downcall is **12.5 to 14.7 ns** (median
  12.7) and a JNI forward call has a **median of 11.4**, ranging 10.9 to 17.5. **FFM is not
  cheaper than JNI in the forward direction here**, where README section 2's published pair
  (33.8 against 98.4) is three times apart. `Linker.Option.isTrivial()` halves it to 6.2 to
  8.1 ns, and a codec entry point that makes reverse calls may not declare itself trivial --
  so the cheap number is available to ABI v1 section 8's bulk-bytes path and to nothing else
  in the codec. Per R7 this is **a comparison of binding mechanisms and never of ABI
  shapes**, and an FFM *upcall* -- which is what the encode path would actually pay -- is not
  measured at all.
- **The pull decode family** (ABI v1 7.1) is not implemented in the core, so the arm that
  would most change the decode verdict does not exist. This slice's decode figures are all
  push-family figures and the deficit they show is the argument for building it.
- **The content sets are measured on P1.2 and P2.2 only** (`logs/java/contentsets.log`),
  not on the rest of the payload set. All three sets are in the **correctness** gate on
  every payload (1,297 checks, 0 failures), so nothing is timed that has not been checked.
- **The transcode pair** (README 10.4). Java is one of the two slices that has to run it,
  because a Rust `String` cannot hold an unpaired surrogate. `ak.Utf8.encode` writes U+FFFD
  where protobuf-java writes `?`, which is the disagreement, and it is **written down and
  not exercised**: no vector reaches it.
- **Concurrency.** One thread everywhere. Conformance obligation 12.5's suite does not
  exist here either, and the binding's re-entrancy design (instance state, a thread-local
  frame stack in the shim) is **argued and not tested**.
- **Allocation and footprint.** No column anywhere. On a managed runtime that is a larger
  gap than it would be in C++.
- **A generated C shim that reads facade fields through the JNI API**, which is the shape
  README 9.1 gives Python. It would move the encode reverse calls out of the JVM entirely
  and it is the most promising unbuilt arm in this slice.
- **The oneof union alternative**, **depth past 4 levels**, **the decode recursion limit**,
  **message size limits**, **`ak_init` and the lifecycle**, **the codec's rollback of a
  half-written field**, **malformed wire**: all specified, none exercised here, as in the
  other slices.
- **MSRV-equivalent**: the Rust core is built with rustc 1.94.1 and 1.88 is not verified.

## Slice-specific notes

**The storage decisions, because three of them move a number.** A facade is meant to be
hand-written and idiomatic, and four choices here are decisions rather than
transliterations (`gen/javanames.py` states each in place):

- an **enum** field is `int`, not a Java `enum`: design/SHAPES.md requires `status = 999` to
  round-trip and a Java enum cannot hold it. protobuf-java answers it the same way.
- a **packed** field is a primitive array, where protobuf-java holds `List<Long>`. ABI v1
  section 6's "the host's own array" is then literally true here and literally false for
  the incumbent, so **P6.1 compares two codecs over two data models** and is a control
  twice over.
- a **map** is a `TreeMap`, because the canonical form sorts entries by key and a facade
  iterating in hash order cannot produce the manifest's bytes at all.
- **explicit presence** on a scalar is a primitive plus a `has` flag, as protobuf-java does,
  so neither arm is handed an allocation the other avoids.

**Packaging (README 5.1.3 and open question 6), answered by building it.** A single jar
carrying one bytecode level does not work, and the reason is not FFM:

- the floor and the target **must** be different code, because the target's whole advantage
  is reading `String.coder` and `String.value`, which do not exist on Java 8;
- so the emitted trees are two, and a single jar needs either a multi-release jar or
  runtime capability dispatch. **`ak.Str17` already implements the dispatch** -- it probes
  the layout at class-init and falls back -- so runtime dispatch is demonstrated to work
  and costs one static boolean test per string;
- **the deeper packaging finding is `sun.misc.Unsafe`.** The binding writes C structs at
  computed offsets and protobuf-java reaches for the same mechanism on the same paths. It
  is identical on Java 8 and JDK 17, it is terminally deprecated from JDK 23, its
  memory-access methods warn at run time from JDK 24, and its replacement is FFM -- a JDK
  22 API, above this slice's target. **The mechanism a Java binding would use today is on a
  removal path and its successor is above the floor.** That is a real constraint on the
  proposal and it is not a benchmark result;
- one more, smaller: **`--release 8` on JDK 17 cannot build the floor**, because it builds
  against `ct.sym`, which does not carry `sun.misc.Unsafe`. The floor is compiled by the
  JDK 8 compiler. A CI that used `--release` would not be building the floor a Java 8
  consumer gets.

**The measurement hazard of README R9 is real, and four of the five things the rule says
about it are wrong here.** See `logs/java/deopt.log`; it is the largest methodological
finding in the slice and it is summarised above.

**Re-entrancy.** Every buffer is instance state. The shim's reverse-call frame is a
thread-local stack of depth 8, never a static, so two encoding threads share nothing. This
is argued, not tested: see "what is not measured".

## Next step

In the order a fresh session should take them:

1. **Settle R9's mechanism with the JIT's own output.** `-XX:+PrintCompilation` and
   `-XX:+TraceDeoptimization` across `-Dak.deopt=0` and `-Dak.deopt=3` on JDK 17, above
   U+00FF. The effect is measured and stable; the mechanism is inferred from which triggers
   fire and which arms move, and one log would replace the inference.
2. **The content sets on the rest of the payload set**, which is a rerun rather than new
   code: `-Dak.cs=` is plumbed and all three sets are already in the correctness gate.
3. **The C-shim binding arm** (README 9.1's shape, applied to Java): let the generated C
   read facade fields through the JNI API instead of upcalling into Java. Decode costs
   7.004 upcalls per element at about 80 ns; this is the arm that would remove them without
   the pull family, and it is the most promising thing unbuilt here.
4. **The completion-queue delivery mode**, which is the half of ABI v1 section 9 the
   pinning experiment did not reach: section 9 claims a thread parked in a drain is in
   native state and costs a collection nothing, and says the queue is the fastest arm on
   virtual threads. The pinning harness is 80 lines and already has the shape.
5. **A concurrency suite** per obligation 12.5, which no slice has.

## Requests to the design documents

**This slice writes none of these itself.** They are for the aggregating session.

1. **ABI v1 section 4: the core gained `ak_tc_utf16` and `ak_tc_latin1`.** They are
   specified and were unimplemented; this slice implemented them in its copy of the core's
   hand-written runtime. They belong in the shared runtime, because they are not this
   slice's and C# will need them too.
2. **Open decision 9: Java agrees.** 46 ns per element on the absent path, neutral
   elsewhere, clear of the drift bar only where the decision predicted it would be. Rust,
   C++ and Java now agree and C# is the last one.
3. **Open decision 10 is not a design question any more, it is a defect class.** `apply`
   after the runs is silently wrong in the obvious binding, and byte identity on encode
   does not catch it. Whatever the decision concludes, the ordering hazard needs a sentence
   in section 7 telling a binding author to fill into the child that is there.
4. **Open decision 13: three hosts, one mechanism, and the magnitudes differ by three
   times.** The lifetime contract has to be drafted against a managed host getting 16
   percent where C++ got 50.
5. **Open decision 11 now has an incumbent that retains.** The rust slice's incumbent
   (prost) drops unknown fields as the core does; protobuf-java keeps them and re-emits
   them, checked on 22 vectors. This is the first direct measurement of the guarantee the
   core would remove rather than an argument about one.
6. **README section 2's crossing table.** This machine: rust 2.1 ns, JNI forward 11.9 to
   12.9, JNI upcall 75.8 to 91.0 cached and 295.9 to 309.4 naive. The published 98.4 ns is
   an **upcall** figure and the published 11.2 ns a **forward** figure; they are seven times
   apart and the table lists one number per runtime.
7. **README 5.1.3 and open question 6 can be closed.** A single bytecode level does not
   work, for `String.coder` rather than for FFM; runtime capability dispatch is built and
   works; and `sun.misc.Unsafe` being on a removal path with FFM above the floor is the
   constraint that actually matters.
8. **ABI v1 7.1's pull family is the decode verdict on this host.** Arm R is faster than
   the C ABI on every M2 payload by 1,163 to 1,806 ns per element (clean sign on three of
   five), and 560 ns of that is the 7.004 upcalls at this machine's 80 ns. The push family
   is what makes the C ABI lose on decode here, and the pull family that would fix it is
   specified and unbuilt in the core.
9. **README R9 needs rewriting, and the correction already published there needs two
   further changes.** The hazard is real and larger than stated, and four clauses stand: it
   fires on JDK **17** and not 21, its trigger is a read of a Latin-1 String's characters
   rather than a numeric conversion, no narrowing loop is involved, and nothing happens on
   ASCII at all. The C ABI arm is the only arm immune, because the transcoder is in the
   core -- section 4's own argument, arriving from a direction nobody was looking in.
   **Two clauses must change, now that the mechanism is logged rather than inferred**
   (`logs/java/r9-mechanism.log`): it is C2 pruning `StringUTF16.charAt` out of
   protobuf-java's encoder as unreached, **not deoptimisation** -- the slow state has fewer
   runtime traps than the fast one -- and the effect is **a probability, not a penalty**:
   the process lands in the fast state about one run in ten unprompted and always with the
   Latin-1 probe, with nothing in between. A published figure of "2.16 times" is a ratio of
   two modes, not a cost. **And `deopt=1`, the mode R9 actually names, no longer
   reproduces here at all** (0 of 13 against `deopt.log`'s 4 of 4), which the slice cannot
   explain and has not tried to.
10. **ABI v1 section 9's fourth amendment is confirmed** at three carrier counts, to within
   one percent of its own prediction. The blocking entry point pins a virtual thread's
   carrier and the callback mode does not, so the callback mode is load-bearing rather than
   a convenience. The completion queue, which section 9 calls the fastest arm on virtual
   threads, is still unmeasured.
11. **A `.gitignore` and a tracked-harness audit.** `gen/audit_tracked.sh` asks git what is
   tracked rather than reading `.gitignore`, which is R4's closing rule one level up from
   the defect that cost the rust slice its encode column. Worth lifting into the other
   slices; it is 25 lines.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `conformance.log` | JDK 17, all seven arms | 437 checks, 0 failures. Byte identity, the committed vectors, P2.5's two forms cross-parsed, P7.1 by permutation |
| `unknown.log` | JDK 17 | 22 unknown-field vectors, 66 checks, 0 failures. Decision 11 with an incumbent that retains |
| `counts.log` | the counting core | R5. 10.024 encode and 7.004 decode crossings per `TaskDetailed`; 9 and 6 for a thousand M1 rows; decision 5's zero misses and P2.4's 80 |
| `boundary.log` | built artifacts | R5's second half: every entry point an undefined import of the shim |
| `layout-guard.log` | JDK 17 | ABI v1 section 10 seen FAILING, naming the fact |
| `crossing.log` | JDK 17 | this machine's JNI forward and upcall prices, three upcall shapes |
| `calibration-r13.log` | JDK 17 and rustc 1.94.1 | R13: this machine's rust crossing is 2.1 ns, and the slice's own shim crossing beside it |
| `baseline.log` | JDK 17 | what protobuf-java's memoized size hides: 1.5 to 2.8 times the write |
| `encode.log` | JDK 17, 36 rounds, 12 arms | the encode verdict, and the published regression reconstructed from `pbj-loop` |
| `encode-take-fix.log` | JDK 17, 36 rounds, 12 arms, after D7 | the same run with `takeBytes` at one crossing and one copy. Kept beside `encode.log`, not in place of it: the fix is below the noise floor and this run drifted harder |
| `decode.log` | JDK 17, 40 rounds, 4 arms | the decode verdict, and decision 13 |
| `delta.log` | JDK 17, 40 rounds, no incumbent | the batching predicate and decision 9, paired |
| `drift.log` | three neutrally perturbed builds | the drift bar: **0.078**, and which conclusions clear it |
| `floor.log` | JDK 17 and JDK 8 | README 5.2's three arms, correctness on all three, arm b paired in one process, and two negative controls |
| `contentsets.log` | JDK 17, P1.2 and P2.2 | all three content sets, encode and decode |
| `deopt.log` | JDK 17 and JDK 21 | README R9's hazard: real, 2.16x, opposite sign, and the C ABI arm immune. Read with `r9-mechanism.log`, which corrects its mechanism and its shape |
| `r9-mechanism.log` | JDK 17, `-XX:+LogCompilation`, 40 runs | R9 settled: C2 prunes `StringUTF16.charAt` from protobuf-java's encoder, it is not deoptimisation, the effect is bimodal and probabilistic, and `deopt=1` no longer reproduces |
| `ffm.log` | JDK 21, preview | the FFM downcall price beside JNI's on the same JDK |
| `pinning.log` | JDK 21, virtual threads | ABI v1 section 9's fourth amendment confirmed at three carrier counts |
| `shim-probe.log` | JDK 17, three collectors | README 9.1's shape priced on the JVM before building it: a JNI field store is a third of an upcall, the crossover is k=2-3, and the G1 write barrier doubles a reference store |
| `w10-regate.log` | all three arms, shared core | R0: 3,891 checks 0 failures, both transcoders resolved from `poc/codec`, worst drift move 0.052 against a 0.078 bar |
| `r14.log`, `r14-summary.md` | JDK 17, the real grpc marshaller | R14: the headline against production's path, and `toByteArray` priced against it |
