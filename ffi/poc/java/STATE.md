# java slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **W6 built and measured.** Full codec, both Java levels, seven arms through the correctness gate, the encode verdict taken, the batching prediction tested, decisions 9 and 13 answered for a managed host. **The RPC arm is not built** (below). |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | Java 8, `openjdk 1.8.0_502`. Builds, and passes all 437 correctness checks on the Java 8 runtime. |
| **Target** (where the clock runs) | JDK 17 (`17.0.20`) with the JNI back end |
| **Incumbent** | protobuf-java **3.25.5**, which is what `packages/java`'s pins actually resolve to: the pom declares 3.19.0 and grpc-java 1.74.0 brings 3.25.5, and the resolved one is what a consumer runs. protoc 3.19.0, the pinned one, generates the classes. |
| **R13 calibration** | this machine's rust-slice crossing is **2.1 ns** (`calibration-r13.log`), against 1.8 ns in the rust slice's container and 1.5 in the cpp slice's. Intel Xeon at 2.10 GHz, 4 vCPU, 15 GB, in a container, no pinning. |

## The question this slice answers, and the answer

**Does the encode regression survive ABI v1, and does the generated-Java-codec arm stay
ahead?**

**The encode regression does not survive. The decode half of the published verdict does.**

| direction | the C ABI (`ffi`) | the generated Java codec (`R`) |
|---|---|---|
| **encode** | **0.58 to 0.96** of protobuf-java on every real element-bearing payload | 0.66 to 0.98 on the same set |
| **decode** | **1.22 to 1.62 on every M2 payload**, 0.83 to 1.04 on the flat ones | **0.69 to 1.05**, and it beats the C ABI on every M2 payload with a clean sign |

So the published sentence "a generated pure-Java codec beats the C ABI in both directions"
is **half reproduced**: it does on decode, decisively and for a reason that is measured
rather than inferred, and it does not on encode.

**What moved the encode column is a baseline, not the ABI**, and it is the most important
thing in this document. See "the incumbent was flattered twice" below.

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

**`R` minus `ffi` on P2.2 is -1,163 ns per element with a clean sign over 40 rounds**, and
7.004 upcalls at about 80 ns is 560 ns of it. That is a decomposition, not a ratio, and it
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

### README 5.2, the floor as three arms -- `logs/java/floor.log`

FLOOR_TABLE_PLACEHOLDER

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

**None open.**

## What is not measured

- **The RPC arm is not built.** README's RPC arm (one unary call on P2.2 against grpc-java,
  CPU per RPC at 1, 8 and 16 in flight, the crossing count, and whether the idiomatic wait
  pins a carrier thread) has no code in this slice. What the branch already knows about it
  is stronger than what this slice would have added -- the rust slice established two
  crossings per call and zero per field **by reading the code**, and ABI v1 section 9's
  arithmetic makes the transferable claim host-independent (196 ns of JNI against about
  1.5 ms of CPU is 0.013 percent). What is missing here is Java-specific and behavioural:
  the callback and completion-queue delivery modes, and the virtual-thread pinning
  question, which is the one thing on that list only a JVM slice can answer.
- **FFM is not built as a binding.** It is a JDK 22 API; this container has JDK 8, 17 and
  21, and the proxy does not reach a JDK distributor. `probe/FfmProbe.java` measures the
  **downcall price only**, on JDK 21 with `--enable-preview`, so the cross-language table
  has a Java FFM row taken here rather than inherited. Per R7 that number may be compared
  with this slice's JNI crossing as a **comparison of binding mechanisms and never of ABI
  shapes**.
- **The pull decode family** (ABI v1 7.1) is not implemented in the core, so the arm that
  would most change the decode verdict does not exist. This slice's decode figures are all
  push-family figures and the deficit they show is the argument for building it.
- **The content sets.** Everything here is the ASCII set. `ak.Values.recode` and the
  builders carry the Latin-1 and above-U+00FF sets and no arm has been run on them, so
  **every string-path number in this document is half a number** by design/SHAPES.md's own
  rule. On the JVM this is not a small gap: the target's fast path is the LATIN1 coder, and
  a non-ASCII payload takes it to UTF-16, which is the floor's path.
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

**The measurement hazard of README R9, tested rather than inherited.** Every harness here
accumulates into arrays and formats after the last measurement, and `-Dak.deopt=1` triggers
a `String.format` with a numeric conversion on purpose so the hazard can be measured on
this JDK instead of assumed from a report. **It has not been run yet** -- it is the cheapest
outstanding experiment in the slice and it is the first thing in "next step".

**Re-entrancy.** Every buffer is instance state. The shim's reverse-call frame is a
thread-local stack of depth 8, never a static, so two encoding threads share nothing. This
is argued, not tested: see "what is not measured".

## Next step

In the order a fresh session should take them:

1. **Run the R9 control** (`-Dak.deopt=1` against a clean run). One command, and it either
   confirms a hazard the whole branch has been designing around or retires it.
2. **The content sets.** Latin-1 and above U+00FF on P1.2 and P2.2, for `ffi`, `R` and the
   incumbent. Every string-path number here is half a number until this runs, and on the
   JVM it is also the test of whether the target's LATIN1 fast path survives real data.
3. **The C-shim binding arm** (README 9.1's shape, applied to Java): let the generated C
   read facade fields through the JNI API instead of upcalling into Java. Decode costs
   7.004 upcalls per element at about 80 ns; this is the arm that would remove them without
   the pull family, and it is the most promising thing unbuilt here.
4. **The RPC arm**, for the two things only a JVM can answer: the completion-queue mode
   against the callback mode, and whether the idiomatic wait pins a carrier thread.
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
8. **ABI v1 7.1's pull family is the decode verdict on this host.** Arm R beats the C ABI
   on every M2 payload by 1,163 to 1,806 ns per element, and 560 ns of that is the 7.004
   upcalls. The push family is what makes the C ABI lose on decode here.

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
| `decode.log` | JDK 17, 40 rounds, 4 arms | the decode verdict, and decision 13 |
| `delta.log` | JDK 17, 40 rounds, no incumbent | the batching predicate and decision 9, paired |
| `drift.log` | three neutrally perturbed builds | the drift bar: **0.078**, and which conclusions clear it |
| `floor.log` | JDK 17 and JDK 8 | README 5.2's three arms, correctness on all three and arm b paired in one process |
