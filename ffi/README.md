# One native core, N bindings: the exploration branch

`rust/native-core-ffi-poc`, cut from `rust/direct-message-impls`.

**This branch is never merged.** It exists to answer one question with evidence,
and the only artifact that is taken into account at the end is
[`REPORT.md`](REPORT.md). Everything else here (slices, generators, harnesses,
logs, per-language findings) is working material kept in version control so that
a figure in the report can be traced back to the thing that produced it.

## 1. The question

ArmoniK.Api is implemented five times: C++, C#, Java, Python and Rust. Each has
its own protobuf codec, its own gRPC client, and its own answer to retry,
backoff, TLS, chunking and cancellation. Nothing enforces that the five agree,
and the divergences that matter are the ones no type system and no name
comparison can see: which status codes are retried, what `AllowUnsafeConnection`
actually disables, the write-then-notify ordering in `send_result`.

The proposal under test replaces the five with **one Rust core behind a C ABI**:
the wire format and the RPC engine live in the core, each language keeps
hand-written idiomatic types, and the binding between the two is generated.

**The case for it is a maintenance case, not a performance case.** What the
measurements do is bound the decision: they establish whether adopting the core
costs a language anything against what that language ships today. They do not
make the decision.

## 2. What is already established

Three slices were built before this branch existed. Each is a working
implementation of a small part of the real schema, end to end, measured against
the libraries that language ships today.

| Report | Date | Verdict in one line |
|---|---|---|
| [ArmoniK Rust Core](https://claude.ai/code/artifact/d29ed568-05eb-4ded-b22a-b1db669a56fb?sk=k-raOgdhnjAvYmpd0YdSGA) (base design, C++ POC) | 2026-09-04 | Feasible. Decode ~22% faster than non-arena protobuf C++, ~12% behind arena; encode ~33% behind either. cxx cannot express the API; cbindgen plus a libclang-driven generator can. |
| [C# Across the ABI](https://claude.ai/artifact/WYD94FSYuq1Nxjdu6WHbtS?sk=aeQYJo8cccAcZsFgdTXRkQ) | 2026-09-13 | Feasible and faster than the incumbent in both directions, **but only on an amended interface**. Decode 0.69 to 0.89 of `Google.Protobuf`, encode 0.22 to 0.43 of `ToByteArray`. On the interface the base design drafts, decode is 1.12 to 1.19, a regression. |
| [Java Across the ABI](https://claude.ai/artifact/YFSNVzYD41C1TsHmANLcBu?sk=MJlBPbq3WV9R4dxdhAv6Og) | 2026-09-12 | Split by direction. Decode 0.59 to 0.92 of protobuf-java; encode 1.08 to 1.84, a regression on every payload. A generated pure-Java codec over the same facade beats the C ABI in both directions, so on Java the case rests entirely on maintenance. |

Two results cut across all three and set the shape of everything below.

**The interface is the finding, not Rust.** The same Rust codec behind two
different C interfaces differs by more than the incumbent does. On a 1,000-row
response the drafted interface makes 15,137 boundary calls to decode; the
amended one makes 4. Every mechanism that gets there is a function of the message
descriptor, so it is not a per-language workaround.

**A crossing is not one price, it is a price per runtime.** Same work, measured
per reverse call: about 0.25 ns in C++, 7.5 to 12 ns on .NET 8, about 73 ns on
Mono 6.8, 98.4 ns through JNI, 33.8 ns through FFM. The design rule that follows
is the one to carry into every remaining POC: **make the crossings fewer, not
cheaper.**

### The base design is out of date, and that is work item 1

The base design predates both managed-host reports. The ABI it draws in "What
crosses" is the one the C# report measures as a regression, and the amendments
that fix it (a string as data inside a by-value group rather than a call; batched
element runs; a host-owned decode context; no map case; plain exports rather than
a table) exist only in the two companion reports. The C++ POC has never been
re-measured against the amended ABI.

## 3. What is not established

Written down so that the report cannot quietly inherit an assumption.

- **Python has no POC at all.** It is also the language whose incumbent is
  already native (protobuf-python on upb, grpcio on the gRPC C core), so it is
  the one where the crossing argument could land differently from every other.
- **C++ has never been measured on the amended ABI.** The amendments were
  motivated by managed hosts; the claim that C++ pays nothing for them is
  currently an argument, not a measurement.
- **Two field shapes are unmeasured on .NET**: oneofs, which the by-value group
  does not reach, and explicit presence, where the group cannot distinguish
  absent from empty. The in-scope schema has 19 oneofs in 413 fields.
- **C# has no managed decode control.** It has a managed encode control. Java's
  report names this as the single measurement that would change its
  recommendation: if C# looks like Java on decode, the conclusion is not "Java is
  special" but "the codec half of the C ABI does not suit managed runtimes", and
  the ABI's scope narrows to C++ and Python.
- **No cross-language byte corpus exists.** All three reports converge on it as
  the missing artifact, and it is the only thing that would catch a divergence
  like the `Output` adapter, where one facade type has two wire forms and an
  adapter written from intuition is silently wrong on the failure path only.
- **The cost of getting there is unpriced everywhere.** Every report prices
  generated code. The hand-written facade surface, the migration of existing
  callers, native-binary packaging and the test estate are the larger half of the
  work and no number in any report touches them.
- **Concurrency, real hardware, streaming, TLS, the server seam.** Every slice so
  far is single-threaded or two-vCPU, unary, loopback, client-side.

## 4. What this branch does

Six work items. W1 blocks the per-language work; W2 to W5 are independent of each
other; W6 is the deliverable.

| # | Work item | Done when |
|---|---|---|
| W1 | **Reconcile the ABI.** One specification, in this branch, merging the base design with the amendments from the C# and Java reports. Every amendment carries the figure that motivated it and the language it came from. | `design/ABI.md` exists and every later slice is built against it rather than against a report. |
| W2 | **Bring the C# and Java slices in.** They were built outside this repository. Import the sources, make them build and run from `ffi/poc/`, then close the gaps their own reports name (C# managed decode control; oneofs and explicit presence on .NET; the transcoder triple measured on .NET). | Both slices run from a clean checkout, and the two named gaps have numbers. |
| W3 | **Re-validate C++ on the amended ABI.** Rebuild the C++ slice against W1 and re-measure against protobuf C++ (arena and non-arena). | The amended ABI has a C++ column, and the claim that the managed amendments are free in C++ is a measurement. |
| W4 | **Build the Python POC.** Section 6. | Python has a verdict of the same shape as the other three, or a stated reason why the question is different there. |
| W5 | **Build the conformance corpus.** Section 7. | Every slice produces and consumes the same bytes, and the corpus is generated rather than curated. |
| W6 | **Write the report.** | `REPORT.md` states a recommendation, the evidence for it, and what it does not establish. |

**Keep the POCs small.** No slice needs to cover every message or every RPC. A
slice covers the field shapes that decide the answer, and nothing else. Where a
shape is not covered, that goes in the "not measured" list rather than into a
larger slice.

## 5. How a POC is conducted

These rules are what make five separate slices comparable, and most of them were
learned the hard way in the three that already exist. A slice that breaks one of
them produces a number that cannot be used.

**R1. One schema description drives everything.** One description per slice emits
the `.proto`, the facade, the Rust codec, the binding, the no-boundary control
codec and the payloads. No hand-written codec anywhere in the comparison, so a
defect in one arm is a defect in a generator backend, which is what it would be
in production.

**R2. Correctness before timing, and byte identity across every arm.** Every
encoder in a slice produces bytes that prost, the incumbent and the control codec
all agree on. A slice that cannot assert that is not measuring the same work in
each arm.

**R3. Three arms minimum, in one process.** The incumbent that language ships
today (the baseline every ratio is against), the C ABI arm, and a **no-boundary
control**: the same generated codec emitted into the host language, over the same
facade objects. prost appears as a floor, never as a candidate. The control is
not optional: on Java it is the arm that changed the recommendation.

**R4. Every ratio is formed inside one process on one runtime.** Absolutes do not
travel between runs on shared hardware; ratios within one process do. Any
comparison that cannot share a process (two incumbent versions, a different
runtime) says so and carries an in-process control column.

**R5. Count the crossings, do not infer them.** Every slice reports boundary-call
counts per payload per direction, from a counting build. The crossing count is
what makes a result portable to a runtime nobody measured.

**R6. The payload set is shared.** P1, P2, P3, P6, P7, P11 as defined in the C#
and Java reports (flat small, flat large, nested small, nested large, repeated
strings, packed scalars), plus P12 (oneof and explicit presence), P13 (the
`Output` adapter), P14 (bulk `bytes`, the result upload and download shapes),
P16 and P17 (**everything absent or empty**, which is where offset defects hide).
A payload generator that fills every field cannot reach the absent path, and a
defect that lived there passed every other payload in the Java slice.

**R7. Name the configuration.** Runtime version, incumbent library version,
binding mechanism, machine. A ratio between an arm on one binding mechanism and
an arm on another is a comparison of mechanisms, not of ABI shapes, and mistaking
one for the other has already produced retracted figures.

**R8. State the measurement hazards you are exposed to, per table.** The known
ones: JIT tiering and PGO off handicaps a managed incumbent; on JDK 21 and later
a single `String.format` with a numeric conversion permanently deoptimises every
`char` narrowing loop in the process, which is protobuf-java's own encoder; two
vCPUs is the smallest contention a shared cache line can have, so a concurrency
figure from it is a lower bound and not a figure.

**R9. Keep a defect log.** Each slice records the defects found in it and what
found them. Three of the most useful findings in the existing reports are defects
in a generator, not properties of an interface, and the rule they produced
("sweep a codegen rule across the generator, do not fix it where it was found")
is worth more than most of the timings.

**R10. Every slice ends with "what is not measured".** A slice that does not name
its gaps is not finished, and the report is assembled from those lists as much as
from the verdicts.

## 6. The Python POC

Stated in more detail because it is the one with no prior art.

- **Incumbent**: `protobuf` (the upb C extension) plus `grpcio` (the gRPC C
  core). Unlike C# and Java, Python's incumbent is already a native library
  reached across a boundary, so the comparison is native-to-native and the
  crossing argument may land differently.
- **Binding mechanisms to price**: `ctypes`, `cffi` (ABI and API modes) and
  PyO3. These differ by an order of magnitude in per-call cost, so R5 matters
  more here than anywhere else.
- **The question that decides it**: the GIL. A reverse call into Python must hold
  it, so the drafted ABI's per-field upcall is the worst possible shape, and the
  amended ABI's batched drain and by-value group are the only ones with a chance.
  If the amended shape still loses, Python is a case for generating a codec into
  Python and keeping only the RPC layer on the C ABI.
- **Also worth knowing**: `packages/python` reads no transport environment
  configuration at all today, so the configuration-homogeneity half of the
  argument is a pure gain there rather than a migration.
- **Minimum slice**: the same messages as the other slices, the shared payload
  set, one binding mechanism measured against the other two on a microbenchmark
  before the full slice commits to one.

## 7. The conformance corpus

A fixed set of byte vectors every language must both produce and consume, checked
in, run in CI, and **generated from the descriptor rather than curated**. It is
the only thing standing behind a per-language codec, and the three reports
between them establish exactly what it has to contain:

1. **Fields the reader does not know.** Generate from a superset descriptor: the
   same messages plus fields the reader was not built against, one of each wire
   type, after the known fields and inside a nested message as well as at the top
   level. A corpus generated from the schema that reads it never executes the
   unknown-field skip, which is the whole of protobuf's forward compatibility.
2. **Fields that are absent or empty.** See R6.
3. **Every field shape, mechanically.** 413 fields, 19 oneofs, 21 enums, two
   maps. A curated corpus covers the shapes somebody thought of, and the ones
   nobody thought of are the ones a new backend gets wrong.
4. **The transcode pair.** Encode transcodes in the core, decode transcodes in
   the host, so the two have to agree on malformed input and on the
   unpaired-surrogate substitution across every facade. Java and .NET do not
   currently agree on it.

One consequence is a constraint on the ABI rather than on the corpus: an
interface that lets the host choose emission order gives up byte identity by
construction, so the corpus cannot validate it. That belongs in the ABI decision,
not after it.

## 8. Layout and deliverables

```
ffi/
  README.md              this document: the goal, the rules, the plan
  REPORT.md              the deliverable. The only thing that counts at the end
  design/
    ABI.md               W1: the reconciled ABI specification
    DESIGN.md            the base design, updated as findings land
  poc/
    cpp/  csharp/  java/  python/
  corpus/                the generated conformance corpus and its generator
  findings/
    cpp.md  csharp.md  java.md  python.md
  logs/                  raw measurement logs a figure can be traced back to
```

Rules that go with the layout:

- **Markdown in this branch is the source of truth.** Published artifacts are
  renderings of it. On a disagreement, the file in the branch wins.
- **Nothing under `packages/` changes.** The diff against `main` stays readable,
  and the branch cannot accidentally become a half-migration.
- **Every figure in a report names the log it comes from.** A figure with no log
  is a claim, and the reports are already carrying retractions of exactly that
  kind.

## 9. How the branch reaches a recommendation

The bar is not "is the Rust core the fastest possible codec for language X". It
is **what a unified core costs each language against what ArmoniK ships today**,
weighed against what maintaining five implementations costs.

Three outcomes are possible and all three are acceptable results for this branch:

1. **Adopt in full.** Codec and RPC layer on the C ABI for every language with a
   native binding. Requires no language to regress materially in both directions.
2. **Adopt the RPC layer, generate the codec.** The generator emits a codec into
   each host language from the same schema description; the C ABI carries only
   the transport, retry, TLS and cancellation engine, where the crossing count is
   two per RPC rather than one per field and the behaviour divergence is worst.
   This is Java's own fallback recommendation and it keeps most of the
   maintenance argument: one generator, one schema description, one annotation
   set, one conformance corpus, one options schema.
3. **Adopt per language.** C++ and Python on the full core, managed runtimes on
   the generated codec, one RPC layer everywhere.

The report states which, and states the evidence that rules out the other two.

## 10. Out of scope

- **The browser.** `packages/web` and `packages/angular` cannot load a native
  library, so they stay on generated gRPC-web whatever this branch concludes.
  Node through a native addon was considered and is not in scope for this branch.
- **Shipping anything.** No packaging, no release, no migration of a real
  consumer. Where those costs matter to the recommendation they are estimated and
  labelled as estimates.
- **Completeness.** Not every message, not every RPC, not every field shape. See
  R10.

## 11. Open questions for this document

To settle while iterating on it, before the slices start.

1. **W2 scope.** "Bring the C# and Java slices in" can mean import and re-run as
   they are, or rebuild both against the reconciled ABI of W1. The second is
   strictly better evidence and roughly doubles W2.
2. **Does the reconciled ABI get a C++11 re-check as part of W1**, or does W3
   discover it? The C++11 pin is the one constraint that can disqualify an
   amendment rather than cost it.
3. **How much of the real schema does a slice cover?** The existing slices use 8
   to 10 messages. The alternative is to drive every slice off the real
   `Protos/V1` descriptor and pick messages from it, which makes the corpus of
   section 7 a by-product rather than a separate build.
4. **Who is the audience for `REPORT.md`?** A decision record for the team, or an
   AEP-shaped proposal. The base design notes this would be the largest breaking
   change in the repository's history and that an AEP process exists for exactly
   that.
