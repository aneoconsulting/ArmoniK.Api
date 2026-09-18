# One native core, N bindings: the exploration branch

`rust/native-core-ffi-poc`, cut from `rust/direct-message-impls`.

**This branch is never merged.** It exists to answer one question with evidence,
and the only artifact taken into account at the end is [`REPORT.md`](REPORT.md).
Everything else here (slices, generators, harnesses, logs, per-slice journals) is
working material kept in version control so that a figure in the report can be
traced back to the thing that produced it.

Working in this directory? Read [`CLAUDE.md`](CLAUDE.md) first: it is the
operating contract between the aggregating session and the slice agents.

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
per reverse call: about 0.25 ns in C++ (statically linked), 1.8 ns in Rust
through a shared library, 7.5 to 12 ns on .NET 8, about 73 ns on Mono 6.8, 98.4
ns through JNI, 33.8 ns through FFM. The design rule that follows
is the one to carry into every slice: **make the crossings fewer, not cheaper.**

### The base design is out of date, and that is work item W1

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
- **The Rust slice's quantitative results are suspended pending a controlled
  re-run**, and the encode win is the claim most at risk: see the head of
  [`findings/rust.md`](findings/rust.md). What is unaffected is anything measured
  as a delta inside one build, plus the crossing counts and the byte-identity
  results.
- **The Rust slice's codec half is done and its behavioural half is untouched.**
  Every shape and payload is measured (section 4.1, [`findings/rust.md`](findings/rust.md)):
  a crossing costs 1.8 ns; encode is 0.41 to 0.57 of prost natively and 0.79 to
  0.92 through the C ABI; decode converges to parity as an element gains
  containers; the group inverts the verdict on the absent path. **What is not
  established there is the larger list**: `ak_init` and the whole lifecycle are
  unbuilt, so "every entry point requires `ak_init`" is unexercised; the codec's
  rollback of a half-written field is written and never triggered; and on the RPC
  side the delivery modes, metadata, deadlines, the status code, cancellation,
  retry, backoff, TLS, streaming, failure injection and the server seam are all
  unmeasured. **The RPC half's case is behavioural, and none of that behaviour is
  exercised**; what stage 4 measured is the call path, whose cost was never the
  question.
- **The string path is settled, and the old arrangement was strictly dominated.**
  UTF-8 validation was the largest single effect measured anywhere, and it was on
  the wrong side of the boundary: an encoder does not need a `string`'s bytes to be
  valid, a decoder cannot trust them whatever the encoder did, and proto3 puts the
  obligation on parsers. Encode is now a memcpy (0.75 of prost on non-ASCII,
  against 2.0 to 2.6 validating), and decode rejects, which measures **free to
  cheaper than the lossy conversion it replaces**, because a lossy conversion
  already validates and its recovery path is slower. It also improves the
  comparison against prost, which rejects too. The earlier design paid for a slower
  validator to get a weaker guarantee, on both sides at once. ABI v1 decision 3 is
  settled rather than open. An encode figure measured on ASCII alone is still not a
  figure about the string path.
- **The decode half of the argument is bounded by host-side container
  construction, and that is new.** Across three message shapes the core's decode
  goes from 0.81 to 0.96 of prost as the element gains vectors and a map, and the
  *no-boundary* control converges with it, so the codec is not what decode costs
  on a container-heavy message. Every slice's decode figure now has to say which
  shape it came from, and the published managed decode wins deserve re-reading
  against P2.2.
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
- **Nothing retains unknown fields, and four of the five languages do today.**
  proto3 has preserved unknown fields since protobuf 3.5, so `Google.Protobuf`,
  protobuf-java, protobuf C++ and upb all carry an unrecognised field from decode
  through to re-encode. The core does not, and neither does prost, so adopting it
  removes a protobuf guarantee from every language except the one whose incumbent
  already lacked it. It bites anything that round-trips a message between two
  schema versions, the worker path included, and nothing has priced retention.
  ABI v1 open decision 11. **This is the largest unpriced behaviour change the
  branch has found, and a shape-coverage vector found it, not a benchmark.**
- **Concurrency, real hardware, streaming, TLS, the server seam.** Every slice so
  far is single-threaded or two-vCPU, unary, loopback, client-side.

## 4. The slices

Five, one per language, each implementing the same shapes (section 6) over the
same payloads, so that the columns of the final table mean the same thing.

| Slice | Incumbent it is measured against | The question it answers |
|---|---|---|
| `rust` | prost and tonic | What a host language loses against full Rust, and what the new design costs against what `packages/rust` does today. The denominator for everything else. |
| `cpp` | protobuf C++ (arena and non-arena), grpc++ | Does the amended ABI still work for the language the design was drafted for, under the C++11 floor? |
| `csharp` | `Google.Protobuf`, `Grpc.Net.Client` | Closing the two gaps its own report names: a managed decode control, and oneofs plus explicit presence. |
| `java` | protobuf-java, grpc-java | Does the encode regression survive ABI v1, and does the generated-Java-codec fallback stay ahead? |
| `python` | protobuf (upb), grpcio | Section 9. The one language whose incumbent is already native. |

### 4.1 Why Rust is a slice, and not just a floor

Every existing report quotes prost as a floor and stops there, which leaves each
language's result as one number with two things mixed into it: what the
**interface** costs, and what that language's **runtime** costs on top of it. The
Rust slice separates them, because a Rust host pays a crossing price close to
C's, so an FFI arm in Rust is the interface with the runtime tax removed.

Four arms, in one process:

| Arm | What it is | What it prices |
|---|---|---|
| `prost` | prost's generated structs, tonic's codec | Today's floor, and the comparator the other reports already quote |
| `armonik` | the in-repo crate: hand-written types implementing `prost::Message` directly, no conversion layer | Whether hand-written types plus a generated codec cost anything against generated structs, which is the bet `packages/rust` already made |
| `core-native` | the new design's generated codec, called from Rust, no FFI | What the core costs as the core sees itself, against both of the above |
| `core-ffi-rust` | the same core reached through the C ABI from a Rust host | **The interface cost with the runtime tax removed** |

That gives every other slice a decomposition rather than a number:

```
cost(host H)  =  cost(core-ffi-rust)      the interface
              +  (cost(H) - cost(core-ffi-rust))   H's runtime tax
```

and it gives the report the one comparison the proposal is actually about:
`cost(H) / cost(armonik)`, what a binding loses against writing it in Rust.

It also answers a question the design has never asked out loud: **does the new
design beat tonic plus prost on the Rust side too**, or is Rust paying for the
other four? `packages/rust` is the one implementation that would carry the core
natively, so a regression there is a cost with no offsetting binding.

## 5. Language levels: floors are constraints, targets are where the clock runs

Two different things, and conflating them is how a design gets rejected for a
number taken on a runtime nobody deploys for throughput.

| Slice | Floor (design constraint: must compile, must pass correctness) | Target (where performance is measured) | Note |
|---|---|---|---|
| C++ | C++11 | C++17 | A customer is pinned to C++11. The repo's own CMake currently sets `CXX_STANDARD 14`, so which of the two is the real floor is open question 3. |
| C# | netstandard2.0, and failing that .NET Framework 4.8 | .NET 8 | No `UnmanagedCallersOnly`, no `SuppressGCTransition` on the floor, so the vtable is delegate pointers there. |
| Java | Java 8 | Java 17 | FFM is a JDK 22 API, so the floor and the target are both JNI. FFM is a secondary arm, not a target. |
| Python | the floor `pyproject.toml` declares (`>=3.7`), see open question 4 | to be decided, proposal 3.11 | |
| Rust | MSRV 1.88 | MSRV 1.88 | One configuration; the floor is the target. |

**A slice builds and passes the correctness suite on its floor**, and quotes no
ratio from it. What the floor produces is one standalone number (section 5.2),
not a column in a comparison table.

### 5.1 The floor and the target may be different code

They are allowed to diverge, and where the target is faster for it they should:
`#if NET8_0_OR_GREATER` against `#if NETSTANDARD2_0`, a JDK 17 source tree
against a Java 8 one, `if constexpr` in a generated traversal against a tag
switch. The floor's job is that the design is *reachable* from a pinned consumer,
not that the target is held back to it.

Four conditions, and they are what keep the divergence from quietly becoming two
implementations.

**One generator, target level as a parameter.** A second tree maintained by hand
is the thing this whole proposal exists to stop, and it does not become
acceptable because the two trees are in one language. Java has no preprocessor,
so there it is literally one emitted source tree per target level; C# and C++
get the same thing spelled as defines.

**The wire bytes are identical across target levels.** The conformance corpus
runs on every level a slice claims, and a divergence between them is a defect,
never a variant. This is the invariant that lets the floor and the target be
different code at all.

**The public surface diverges only additively.** A higher level may add entry
points (5.1.2); it may not change or remove one, because then a consumer's source
compiles against one level and not the other, which is a migration cost with a
number attached rather than an implementation detail. Anything beyond additive is
reported as that cost.

**In C++ the divergence must not reach the layout of an installed header type**,
and this one is a hard stop rather than a preference. The consumer picks `-std`,
we do not, so a facade type whose layout depends on the standard level is an ODR
violation waiting for a consumer who compiles at a different level than the
library was built at. The base design deliberately verified its optional and its
sum type ABI-identical from C++11 through C++23. Diverge inside the codec and the
binding freely; in the headers, only under the two rules below.

#### 5.1.1 A post-C++11 vocabulary type is ours; a C++11 one is the standard's

The rule is about availability, not about taste. `std::string`, `std::vector`,
`std::map`, `std::shared_ptr` and the rest of the C++11 library are used
directly: they exist at every level the facade supports, so a header naming them
means the same thing at each. **A type that arrives after C++11 is
reimplemented**: `string_view` (C++17), `optional` (C++17), `variant` (C++17),
`span` (C++20), `expected` (C++23). Those are exactly the ones whose presence
depends on `-std`, so aliasing one makes a public header's meaning depend on a
switch the consumer owns.

The shape is one concrete type at every level, with the conversions to and from
the standard counterpart guarded by the feature macro. That is already the house
pattern: `packages/cpp/ArmoniK.Api.Common/header/utils/string_view.h` is a C++11
type that says so in its own header comment and guards only its
`std::string_view` conversions, so `armonik::string_view` is the precedent and a
hand-rolled sum type is its sibling. The cost is real (a five-variant sum type
was ~105 lines of C++11 against Rust's 14) and it is what buys one ABI across the
levels a consumer might compile at.

This buys agreement across `-std`, and nothing else. A standard type's layout can
still differ across standard *library* versions and across
`_GLIBCXX_USE_CXX11_ABI`, which is a toolchain question rather than a language
level one, and it is exactly the exposure `packages/cpp` already has today.

#### 5.1.2 Additive interfaces per level are allowed

A better surface at a higher level is welcome where it makes sense: a C++20
coroutine or `std::expected` API, say, alongside the C++11 one. Three conditions,
and the third is the one that is easy to get wrong.

- **The floor keeps a complete alternative.** The higher level buys ergonomics,
  never capability. A C++11 consumer that cannot reach a feature at all is a
  second product.
- **It is expressible over the same ABI primitives.** The ABI's completion
  callback is the primitive every host idiom is built on, so a coroutine surface
  is facade code over an entry point that already exists. An additive interface
  that needs a new C entry point is not additive; it is a second design, and it
  goes through the ABI document rather than through a define.
- **It is added as free functions or a separate adapter type, not as members of
  an installed class.** Layout is not the only thing an ODR argument covers: two
  translation units that see different definitions of the same class are already
  ill-formed, even when the members they disagree about are non-virtual and the
  layout is identical. Keeping the class definition the same at every level and
  putting the extra surface beside it costs nothing and keeps the property.

For the POC this is an allowance rather than a work item. A slice does not have to
build the C++20 surface; it has to show that the ABI primitive supports one, which
a sketch settles.

#### 5.1.3 Packaging is a separate question, and in Java it is open

A single jar is the preference. Whether that forces one bytecode level for
everything, or a multi-release jar carrying per-level classes, is not decided
here and no design is forced on the slice. Worth knowing before it is: on the
target (JDK 17, JNI) the prior slice needed two substitutions to reach Java 8 and
no third, so **there may be nothing to package differently at all** unless FFM
becomes a target, and FFM is the only divergence large enough to be worth a
packaging decision.

### 5.2 How the floor is measured

Three arms, so that "the floor costs X" separates what the missing APIs cost
from what the old runtime costs. Only the first is a headline figure.

| Arm | Build | Runtime | What it prices |
|---|---|---|---|
| a | target implementation | target runtime | the headline. Every ratio in the report comes from here |
| b | **floor implementation** | **target runtime** | what the floor's missing APIs cost, runtime held constant. The only fair floor-against-target ratio |
| c | floor implementation | floor runtime | what a pinned consumer actually gets. Quoted as a standalone number, never as a ratio against a |

Arm b is the one that answers "is the floor a viable deployment or only a viable
compile", and it is cheap: the same sources, one define flipped, on the runtime
already under the harness. Arm c mixes implementation and runtime by
construction, which is why its number stands alone. The C# report already works
this way and it is where its "the fallback costs nothing on the new ABI" result
comes from, measured as separate arms rather than inferred.

Where a language cannot run arm b (Java 8 bytecode on JDK 17 is fine; an FFM
arm below JDK 22 does not exist at all), the slice says so rather than
substituting a different comparison.

## 6. The same shapes everywhere

[`design/SHAPES.md`](design/SHAPES.md) fixes the message shapes, the field
shapes and the payload set. **Every slice implements all of it**, or records in
its journal which item it does not and why, and that omission goes in the report.

A slice is free to add an arm; it is not free to change the shapes, because a
column of the final table that covers a different set of shapes is not a column,
it is a second table. The shape list is weighted by a census of the real schema
(string 174, int32 33, bool 14, int64 10, bytes 8 across 413 fields; 19 oneofs;
21 enums; 3 packed repeated fields, all enums; 2 maps, both
`map<string, string>`), so a verdict read off a shape the schema has three of is
labelled as a control rather than a result.

## 7. Work items

W1 and W2 block the slices. W3 to W7 are independent of each other. W9 is the
deliverable.

| # | Work item | Done when |
|---|---|---|
| W1 | **Specify ABI v1.** One specification, in this branch, merging the base design with the amendments from the C# and Java reports. Every amendment carries the figure that motivated it and the language it came from. | **Drafted.** `design/ABI-v1.md` carries 12 decisions, of which **2 are now settled**: 5 (the grow path, keep the learned width) and 3 (the string path: no check on encode, reject on decode, free in both directions). The Rust slice produced all of the movement, and also created three: **9** (does the group need an empty-element path), **10** (can decode deliver the group before the runs) and **11** (does the core retain unknown fields). **11 is the one to read first**: it is a behaviour change for four of the five languages. Decision 1 (is every amendment free at the C++11 floor) still gates agreement, and the C++ slice settles it. |
| W2 | **Freeze the shapes and the payload set.** | **Done.** `schema/shapes.json` is the description, `schema/generated/` carries the emitted `.proto` and a payload manifest with a hash per payload, and the Rust slice has confirmed every hash against prost 0.14.4 and a second, independent encoder. One defect was found and fixed in `emit/payloads.py`; 8 of 16 hashes moved. A slice that disagrees with a hash now has a defect in itself. |
| W3 | **Rust slice.** Section 4.1. | **Done.** Four arms over every message and payload of `design/SHAPES.md`, all byte-identical to the validated manifest, plus the three content sets, the unknown-field vectors and the RPC arm. The decomposition every other slice subtracts is available: **a crossing costs 1.8 ns through a shared library**, and the RPC half costs **two crossings per call, zero per field**. See [`findings/rust.md`](findings/rust.md) for what it does not establish, which is longer than what it does. |
| W4 | **C++ slice on the amended ABI.** Rebuild against W1, re-measure against protobuf C++, and demonstrate the C++11 floor. | The amended ABI has a C++ column, and "the managed amendments are free in C++" is a measurement. |
| W5 | **C# slice.** Import the existing slice, rebuild against W1, then close its two named gaps: a managed decode control, and oneofs plus explicit presence. | Both gaps have numbers, and the floor (netstandard2.0 or net48) compiles and passes correctness. |
| W6 | **Java slice.** Import, rebuild against W1, re-measure encode, and keep the generated-Java-codec arm as a first-class candidate. | The encode verdict is stated against ABI v1, on JDK 17 with JNI, with the Java 8 floor demonstrated. |
| W7 | **Python slice.** Section 9. | Python has a verdict of the same shape as the others, or a stated reason why the question is different there. |
| W8 | **Conformance corpus.** Section 10. | Every slice produces and consumes the same bytes, and the corpus is generated rather than curated. |
| W9 | **The report.** | `REPORT.md` states a recommendation, the evidence for it, and what it does not establish. |

**Keep the slices small.** No slice covers every message or every RPC. It covers
the shapes in `design/SHAPES.md` and nothing else. Where something is not
covered, it goes in the "not measured" list rather than into a larger slice.

## 8. How a slice is conducted

These rules are what make five separate slices comparable, and most were learned
the hard way in the three that already exist. A slice that breaks one produces a
number that cannot be used.

**R1. One schema description drives everything, and a backend that cannot do a
shape raises rather than skips.** One description emits the `.proto`, the facade,
the Rust codec, the binding, the no-boundary control codec and the payloads. No
hand-written codec anywhere in the comparison, so a defect in one arm is a defect
in a generator backend, which is what it would be in production.

The second half is a rule the Rust slice paid for: a field walker that excluded
oneof members, used by every backend, emitted a complete-looking codec that
ignored a message's oneof entirely and reported nothing wrong. The byte oracle
caught it at once; **a slice with a weaker oracle would have measured a message
with a shape silently missing from it**, which is a wrong number rather than a
missing one. Every walker enumerates every shape, and a backend that has no case
for one raises.

**And a refusal is tested by a case that must fail.** The first build of ABI v1
section 8's generator-time refusal walked singular message children only, so it
found nothing, refused nothing, and read as working. That is the same defect as
the walker above, caught the same way: by running it against input it was supposed
to reject, not by reading it. A guard with no failing test is a guard nobody has
seen work.

**R2. Correctness before timing, and byte identity across every arm.** Every
encoder in a slice produces bytes that prost, the incumbent and the control codec
all agree on. A slice that cannot assert that is not measuring the same work in
each arm.

**A ratio far enough from 1 to be surprising gets a floor arm before it is
reported.** The Rust slice measured a 4 MB bulk decode at 0.08 of prost, added a
raw `memcpy` as a case, and found every core arm sitting *on* that floor. The
reportable claim is therefore "a 4 MB bulk decode costs one copy in the core and
twelve in prost", which bounds both sides, rather than "the core is twelve times
faster", which bounds neither and would have been the published sentence. The same
control turns a suspiciously large win into a statement about what the incumbent
is doing, which is where such a win usually comes from.

**R3. Three arms minimum, in one process.** The incumbent that language ships
today (the baseline every ratio is against), the C ABI arm, and a **no-boundary
control**: the same generated codec emitted into the host language, over the same
facade objects. The control is not optional: on Java it is the arm that changed
the recommendation.

**R4. Every ratio is formed inside one process, on one runtime, in one build.**
Absolutes do not travel between runs on shared hardware, and **ratios do not
travel between builds of the same source either** — which is stronger than this
rule used to claim and was learned by rebuilding an unchanged commit. The Rust
slice's `core-ffi-rust` on P1.2 encode reads 0.71 in the published table and 1.01
from a fresh worktree at the same commit, with every other arm moving the same way
including one whose code had not changed. So a table assembled from two builds is
not a table; what survives a rebuild is a **delta taken between arms in the same
interleaved rounds**, and that is the form a figure should take wherever the
question allows it. Any
comparison that cannot share a process (two incumbent versions, two runtimes)
says so and carries an in-process control column.

**R5. Count the crossings, do not infer them, and prove the boundary exists.**
Every slice reports boundary-call counts per payload per direction, from a
counting build. The crossing count is what makes a result portable to a runtime
nobody measured.

**The no-boundary control needs the same proof, in the opposite direction.** An
arm named "no boundary" is only a control if it is *not* fused into the benchmark
loop, and whether it is depends on things no one states in a configuration line:
LTO, `#[inline]` on the entry point, whether the entry point is generic. The Rust
slice's control turned out to be an indirect call through the GOT, which is what
made its subtraction valid — but it was one `#[inline]` away from not being, with
LTO off the whole time. So a slice prints the entry point's size and the calling
closure's size from the artifact, both directions, as a build step. In C++ the
same question is `-flto` over the control rather than over the core.

**A counting build is not evidence that a call happened**, and the Rust slice
established that the hard way: with the core in the crate graph as an rlib, rustc
inlined every `extern "C"` entry point into the host, and the counters kept
reporting 3, 9 and 6 crossings because the counting code was inlined with the
function bodies. The FFI arm was the no-boundary control with extra struct
copies, and every ratio from it would have been a figure about the optimiser. So
the count is necessary and not sufficient: **a slice whose host and core can be
compiled together shows, from the built artifact, that the entry points are
unresolved imports** (`nm -D --undefined-only`, or the platform equivalent), as a
step of its build rather than as a claim in its log. The exposure is Rust's and
C++'s, and in C++ it is spelled `-flto` over a statically linked core. The
managed hosts cannot inline across the boundary and are safe from this one.

**R6. The payload set is shared**, and it includes the absent path. A payload
generator that fills every field cannot reach any path conditioned on emptiness,
which is exactly where an offset defect hides: one such defect passed all seven
standard payloads in the Java slice.

**R7. Name the configuration.** Runtime version, incumbent library version,
binding mechanism, **linkage**, machine. Linkage is part of the mechanism and it
is not small: a shared-library crossing measured 1.8 ns in the Rust slice, and a
C++ host that statically links pays a direct call instead, so a C++ column and a
Rust column are not measuring the same thing unless both say which they used. A ratio between an arm on one binding mechanism and
an arm on another is a comparison of mechanisms, not of ABI shapes, and mistaking
one for the other has already produced retracted figures.

**R8. The floor is a correctness gate; the target is where the clock runs**, and
the two may be different code. One generator with a target level, identical wire
bytes across levels, and in C++ no divergence that reaches the layout of an
installed header type. Section 5.

**R9. State the measurement hazards each table is exposed to.** The known ones:
**an RPC arm on P2.2 measures HTTP/2 flow control unless it measures CPU.** A 540
KB response exceeds the 64 KB default h2 stream window, so a single call in flight
spends most of its wall-clock idle waiting for `WINDOW_UPDATE`, and at 8 in flight
the stalls overlap and wall-clock collapses by more than an order of magnitude. The
Rust slice's first version would have reported 33 ms per call for a path that costs
1.5 ms of CPU. SHAPES.md asks for CPU per RPC, and this is why; a wall-clock column
is reported beside it or not at all. The rest:
JIT tiering and PGO off handicaps a managed incumbent; on JDK 21 and later a
single `String.format` with a numeric conversion permanently deoptimises every
`char` narrowing loop in the process, which is protobuf-java's own encoder; two
vCPUs is the smallest contention a shared cache line can have, so a concurrency
figure from it is a lower bound and not a figure.

**R10. Keep a defect log.** Each slice records the defects found in it and what
found them. Three of the most useful findings in the existing reports are defects
in a generator, not properties of an interface, and the rule they produced
("sweep a codegen rule across the generator, do not fix it where it was found")
is worth more than most of the timings.

**R11. Every slice ends with "what is not measured".** A slice that does not name
its gaps is not finished, and the report is assembled from those lists as much as
from the verdicts.

**R12. A slice agent never writes a report.** Section 11.

## 9. The Python slice

Stated in more detail because it is the one with no prior art, and because its
binding shape differs from every other slice's.

**The incumbent is already native.** `protobuf` on the upb C extension plus
`grpcio` on the gRPC C core. Unlike C# and Java, the comparison here is
native against native, and the crossing argument may land differently.

### 9.1 The core calls Python primitives; it does not call back into Python

This is the design default for the Python binding, and it is what makes the
shape different from the managed slices. A reverse call is **a C-API call
against a Python object** (`PyUnicode_FromStringAndSize`, `PyList_SET_ITEM`,
`PyLong_FromLongLong`, a slot read on the facade object), never a call to a
Python-level accessor. Entering the interpreter costs a frame, an argument
tuple and bytecode dispatch; a C-API call on a primitive costs none of those.

What follows from it:

- **The Python binding has three layers where C# has two**: the Rust core, a
  **generated C shim that speaks the CPython API**, and the Python facade. The
  shim is still generated from the same description by the same generator, so
  this is one more backend, not a hand-written layer.
- **The binding mechanism list changes.** `ctypes` and `cffi` in ABI mode route
  a callback through the interpreter, which is the thing being avoided, so they
  are not candidates for the codec path. The candidates are a generated C
  extension module and PyO3 (which is the same C-API calls with a Rust
  spelling). They remain candidates for the RPC layer, where the crossing count
  is two per call.
- **The facade's storage becomes a measured choice**, because it decides what a
  field read costs the shim: a plain class (a dict lookup), a `__slots__` class
  (a descriptor offset), or a C extension type (a struct member, so a field read
  stops being a crossing at all). Each is more work than the last and each is
  faster; the slice prices them rather than assuming, and it keeps the facade
  idiomatic in all three.
- **The GIL is still held for every C-API call**, so batching still matters:
  fewer, larger crossings mean fewer GIL-held stretches and a longer window in
  which the pure parse can run with the GIL released.
- **One control arm settles the premise rather than assuming it**: the same
  codec with Python-level accessors, measured once. If it is not clearly worse,
  the extra layer is not earning its place.

### 9.2 The packaging constraint this creates

Speaking the CPython API ties the artifact to Python in a way the other slices
have no equivalent of, and the choice is real:

- **The full C-API** gives every fast path and needs **one wheel per minor
  version**.
- **The limited API and the stable ABI (`abi3`)** give **one wheel across 3.x**
  and take some of the fast paths away, because several of the cheapest
  accessors are macros that are not in it.

That is a packaging cost against a per-field cost, it is a decision the report
has to state rather than discover, and it belongs with open question 4.

### 9.3 The rest

- **What could still refuse the answer**: if the amended shape loses to upb
  anyway, Python is a case for generating a codec into Python (or into the C
  shim) and keeping only the RPC layer on the C ABI.
- **Also worth knowing**: `packages/python` reads no transport environment
  configuration at all today, so the configuration-homogeneity half of the
  argument is a pure gain there rather than a migration.
- **Minimum slice**: the shapes of section 6, with the facade storage and the
  binding mechanism each chosen by microbenchmark before the full slice commits
  to one.

## 10. The conformance corpus

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
3. **Every field shape, mechanically.** A curated corpus covers the shapes
   somebody thought of, and the ones nobody thought of are the ones a new backend
   gets wrong.
4. **The transcode pair.** Encode transcodes in the core, decode transcodes in
   the host, so the two have to agree on malformed input and on the
   unpaired-surrogate substitution across every facade. Java and .NET do not
   currently agree on it. **The Rust slice cannot reach this one at all**: a Rust
   `String` cannot hold an unpaired surrogate, so no content set constructed in
   Rust produces the disagreeing input. The corpus therefore has to carry those
   vectors as **raw bytes produced by something other than a Rust host**, and the
   pair has to be run by the slices whose string type can hold the input, which is
   C# and Java. This is a constraint on who validates what, not only on what the
   corpus contains.
5. **Distinct tags across a nesting level, and enough elements to force more
   than one chunk.** The Rust slice found an element run that read the open
   field's tag from the context at entry, so a host that chunked wrote its later
   chunks under the *inner* field's tag: silent wire corruption on every chunk
   after the first. **Byte identity passed anyway**, because the outer repeated
   field and the inner map field are both tag 1, and nothing in
   `design/SHAPES.md` has a root whose repeated field carries a tag different
   from a repeated or map field inside its element. A corpus that cannot tell the
   two tags apart cannot catch this class of defect in any slice.

One consequence is a constraint on the ABI rather than on the corpus: an
interface that lets the host choose emission order gives up byte identity by
construction, so the corpus cannot validate it. That belongs in the ABI decision,
not after it.

## 11. How the work is run

Three roles, and the separation between them is what keeps the report honest.

### The aggregating session (this one)

Owns `README.md`, `CLAUDE.md`, `design/**`, `findings/**`, `REPORT.md`, and the
decision about what gets built next. It spawns the slice agents, reads what they
produced, and **is the only role that writes prose about results**. It does not
run benchmarks itself.

### Slice agents, one per language, resumable

One agent per slice (`ffi-slice`), responsible for building that slice and
running its benchmarks. Each owns exactly `ffi/poc/<lang>/**` and
`ffi/logs/<lang>/**`, and writes three things there:

- `STATE.md`, the handoff contract, rewritten at the end of every work unit;
- `JOURNAL.md`, what was tried, what it measured, what refuted it, in order;
- raw logs under `ffi/logs/<lang>/`, which is what a figure is traced back to.

**A slice agent never edits `REPORT.md`, `findings/**`, `design/**` or this
file.** It reports its findings back, and the aggregating session decides what
they mean. The reason is not bureaucracy: a slice agent that writes the verdict
on its own slice has every incentive to write the verdict its last measurement
suggested, and three of the most important findings so far are corrections of
exactly that.

**Resuming.** Within one session, send the live agent another message rather than
spawning a new one, so its context survives. Across sessions, context does not
survive, so `STATE.md` is the resume mechanism: a fresh agent reads it first, and
it is a defect for it to be stale. That is why it is rewritten at the end of
every work unit and not at the end of the slice.

### Review agents, adversarial, read-only

Spawned when a review is asked for (`ffi-review`, or the `/ffi-review` command).
They read the slice, the journal and the logs, and hunt for the reasons a number
is wrong: an arm that is not running, a ratio formed across processes, a control
that shares the defect it is controlling for, a codegen rule applied in one path
and not swept.

**A review agent never writes code and never writes files.** It cannot confirm a
finding by building something, which is deliberate: confirmation is handed to the
slice agent that owns the code, so the same role never both raises and clears a
finding. A review agent's output is a list of findings, each with the evidence it
rests on and what would refute it.

## 12. Layout and deliverables

```
ffi/
  README.md              this document: the goal, the rules, the plan
  CLAUDE.md              the operating contract for anyone working in here
  REPORT.md              the deliverable. The only thing that counts at the end
  design/
    ABI-v1.md            W1: the ABI specification, version 1
    SHAPES.md            W2: the shapes and payloads every slice implements
    DESIGN.md            the base design, updated as findings land
  poc/<lang>/            one slice per language, agent-owned
    STATE.md             the handoff contract. Read first, written last
    JOURNAL.md           what was tried, measured, refuted, in order
  schema/                W2: shapes.json, the one description every slice reads
    gen/                 emitters: the .proto, the payloads, a framing check
    out/                 generated: shapes.proto, manifest.json, payloads/
  corpus/                W8: the conformance corpus, a superset of schema/generated
  findings/<lang>.md     the aggregating session's reading of a slice
  logs/<lang>/           raw measurement logs a figure traces back to
```

Rules that go with the layout:

- **Markdown in this branch is the source of truth.** Published artifacts are
  renderings of it. On a disagreement, the file in the branch wins.
- **Nothing under `packages/` changes.** The diff against `main` stays readable,
  and the branch cannot accidentally become a half-migration. The Rust slice
  *reads* `packages/rust` and measures against it; it does not edit it.
- **Every figure in a report names the log it comes from.** A figure with no log
  is a claim, and the reports are already carrying retractions of exactly that
  kind.

## 13. How the branch reaches a recommendation

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

## 14. Out of scope

- **The browser.** `packages/web` and `packages/angular` cannot load a native
  library, so they stay on generated gRPC-web whatever this branch concludes.
  Node through a native addon was considered and is not in scope.
- **Shipping anything.** No packaging, no release, no migration of a real
  consumer. Where those costs matter to the recommendation they are estimated and
  labelled as estimates.
- **Completeness.** Not every message, not every RPC, not every field shape.

## 15. Open questions

1. **W5 and W6 scope.** Importing the C# and Java slices and rebuilding them
   against ABI v1 is strictly better evidence than importing and
   re-running them as they are, and it roughly doubles both items.
2. **Does ABI v1 get a C++11 re-check as part of W1**, or does W4
   discover it? The C++11 pin is the one constraint that can disqualify an
   amendment rather than cost it.
3. **Is the C++ floor C++11 or C++14?** The design says a customer is pinned to
   C++11; `packages/cpp` sets `CXX_STANDARD 14` on every target today.
4. **Python floor, target, and wheel policy.** `pyproject.toml` declares
   `>=3.7`. Holding 3.7 as the floor rules out some binding mechanisms outright;
   3.13 free-threaded changes the GIL argument, so whether it is a target, an
   arm or out of scope needs deciding; and the full C-API against the stable ABI
   (9.2) is a wheel-per-version against wheel-per-3.x decision that the floor
   choice constrains.
5. **How much of the real schema does a slice cover?** The existing slices use 8
   to 10 messages. The alternative is to drive every slice off the real
   `Protos/V1` descriptor, which makes the corpus of section 10 a by-product
   rather than a separate build.
6. **Java packaging.** A single jar is wanted; whether that means one bytecode
   level for everything, a multi-release jar, or runtime capability dispatch is
   open, and may not need deciding at all if FFM never becomes a target (5.1.3).
7. **Who is the audience for `REPORT.md`?** A decision record for the team, or an
   AEP-shaped proposal. The base design notes this would be the largest breaking
   change in the repository's history and that an AEP process exists for exactly
   that.
