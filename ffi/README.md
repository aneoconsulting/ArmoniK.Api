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
make the decision, and **neither does this branch**: the decision belongs to the
project owner. This branch records facts, states what they do and do not
establish, and describes the options neutrally (section 13). No document and no
agent here writes a recommendation.

### 1.1 Phase: setup and design

The branch is in its **setup and design phase**. Its job now is to build every
slice correctly, over one core and one generator, and to build harnesses that
can measure the question without a defect deciding the answer.

**Performance is measured later, once, in a single campaign** (W13): one
physical machine, no other tenant, the slices run **one after another** and never
concurrently, and in every RPC measurement the **client and the server run in
separate processes pinned to disjoint CPU sets**. The contract a harness meets
before that campaign is `design/CAMPAIGN.md` (W11).

Until then:

- **Every timing taken in a container is instrumentation.** It shows that a
  harness runs, and it can expose a harness defect. It is not a result, it is not
  quoted as one, and a wrong one is deleted rather than replaced by a better
  container figure.
- **What does count as a result now**: byte identity across arms; crossing
  *counts* (a property of the interface, not of the machine); floor builds and
  corpus passes; feasibility (it builds, links, round-trips); defects found.
- **The core's transport is deliberately minimal.** `packages/rust`'s
  `armonik-transport` is known not to be at parity with the other packages and
  will be brought to parity before any binding is implemented for real. The PoC
  transport may carry fewer features still (no TLS, retry, metadata, deadlines,
  numeric status or streaming): those features change neither feasibility nor
  the cost of the binding.
- **The payload set is not weighted by traffic.** No statistics on real ArmoniK
  payloads exist, so nothing here can say which payload matters most.

The adversarial review of 2026-09-24 and what follows from it are in
[`design/FIX-PLAN.md`](design/FIX-PLAN.md), with a register of open findings.

## 2. What is already established

### 2.1 Prior reports

Three reports were written before this branch existed. What each **claimed** is
recorded here as history. None of it is a result of this branch, and their
timings are container figures like any other.

| Report | Date | What it claimed |
|---|---|---|
| [ArmoniK Rust Core](https://claude.ai/code/artifact/d29ed568-05eb-4ded-b22a-b1db669a56fb?sk=k-raOgdhnjAvYmpd0YdSGA) (base design, C++ POC) | 2026-09-04 | Feasible. cxx cannot express the API; cbindgen plus a libclang-driven generator can. |
| [C# Across the ABI](https://claude.ai/artifact/WYD94FSYuq1Nxjdu6WHbtS?sk=aeQYJo8cccAcZsFgdTXRkQ) | 2026-09-13 | Feasible on an amended interface; the interface the base design drafts regresses decode. |
| [Java Across the ABI](https://claude.ai/artifact/YFSNVzYD41C1TsHmANLcBu?sk=MJlBPbq3WV9R4dxdhAv6Og) | 2026-09-12 | Split by direction; a generated pure-Java codec over the same facade competes with the C ABI. |

### 2.2 Facts this branch has established

**Feasibility.** All five slices (Rust, C++, C#, Java, Python) implement every
message and payload of [`design/SHAPES.md`](design/SHAPES.md) over **one** shared
core at [`poc/codec/`](poc/codec/) (W10; no slice carries a copy), and every
slice's arms are byte-identical to the payload manifest, with the one documented
exception of P2.5, which has two valid encodings (W2).

**The interface decides the number of crossings.** On a 1,000-row response the
drafted interface makes 15,137 boundary calls to decode and the amended one
makes 4 (prior C# report). Crossing counts are
counted, not inferred (R5), and they reproduce across slices to the digit.
Every mechanism that reduces them (a string as data inside a by-value group,
batched element runs, a host-owned decode context, the pull decode family) is a
function of the message descriptor, not a per-language workaround.

**A crossing has a price per runtime and per direction.** The container figures
differ by more than an order of magnitude between runtimes, and within one
runtime a forward call and a reverse call (an upcall) differ too, so a
crossing figure always says which direction it is. The figures themselves are
instrumentation and are re-taken in the campaign; the design rule drawn from
them, **make the crossings fewer rather than cheaper**, depends on the ordering,
not on the values.

**Floors demonstrated** (builds and passes correctness):

| Level | Evidence |
|---|---|
| C++11 (and C++14) | `logs/cpp/conformance.log` |
| Java 8 (`openjdk 1.8.0_502`), 437 of 437 checks | `logs/java/floor.log` |
| netstandard2.0 build, run on **Mono 6.8** | `poc/csharp/STATE.md`. This is not .NET Framework 4.8, and net6.0 has not been run |

**Not yet demonstrated**: .NET Framework 4.8 (Windows only), net6.0, Python 3.7,
Rust MSRV 1.88 (section 5).

**The conformance corpus exists** (W8): 336 vectors in
`corpus/generated/manifest.json`, of which 328 are sealed byte for byte in
`corpus/generated/vectors.sha256` (the 8 unsealed are the baseline rows
`B-P*`, which derive from `schema/`'s manifest). Three independent protobuf
runtimes act as oracles, and a row on which they disagree is recorded as
disputed rather than decided (section 10.2). The Python, C++ and C# slices
consume it; the Java slice's generated codec has not run it.

**Defects found, each a fact about a mechanism rather than a timing:**

- Several threads on one encode context abort the process rather than producing
  wrong bytes, because a panic cannot unwind through `extern "C"` (Rust
  concurrency suite). ABI v1 section 5 makes `catch_unwind` mandatory at every
  entry point.
- An element run that read the open field's tag from the context at entry wrote
  every chunk after the first under the wrong tag, and byte identity passed
  because the two tags were equal (section 10, item 5).
- A group-skip defect in the shared core, found by the corpus's first consumer.
- The runtimes disagree on an unknown field inside a map entry (section 10.1).
- A test server left Nagle on while the client had it off, which produced a
  transport "gap" that was a harness artifact (R9).
- The 2026-09-24 review's open defects: `design/FIX-PLAN.md` section 7.

### The base design is out of date, and that is work item W1

The base design predates both managed-host reports. The ABI it draws in "What
crosses" is the drafted interface above, and the amendments (a string as data
inside a by-value group rather than a call; batched element runs; a host-owned
decode context; no map case; plain exports rather than a table) are specified in
[`design/ABI-v1.md`](design/ABI-v1.md), which is drafted and not yet agreed.

## 3. What is not established

Written down so that the report cannot quietly inherit an assumption.

- **No performance result exists.** Every timing in this branch is container
  instrumentation (section 1.1). Which option costs what, per language and per
  direction, is the campaign's output (W13), and so is the decomposition of a
  host's cost into interface cost and runtime tax (section 4.1).
- **The maintenance side of the question is not documented.** Nothing yet lists
  where the five packages behave differently today. W12 produces that inventory,
  as facts with file and line, without a cost estimate.
- **"One generator" is not yet true of this tree.** The front end is shared, but
  the wire rules are written separately in seven traversal emitters across
  `poc/codec/gen/` and the slices' `gen/` directories, and they diverge. The
  maintenance case rests on one generator, so W14 makes it one.
- **Unknown-field retention is undecided.** The core drops unknown fields, and so
  does prost; `Google.Protobuf`, protobuf-java, protobuf C++ and upb retain them
  from decode through re-encode. Adopting the core as it stands would remove that
  behaviour from four of the five languages. The decision is open (ABI v1
  decision 11), so the core and the generator carry both behaviours and the
  campaign measures both.
- **The RPC layer's behaviour is not exercised.** What the RPC arms exercise is
  the call path. Retry, backoff, TLS, deadlines, metadata, cancellation, the
  status code, streaming, failure injection and the server seam are not built in
  the PoC transport (section 1.1).
- **Directions and paths not yet covered by any RPC arm**: a request that carries
  a payload (every host but Rust sends an empty request), the worker path
  (decode a request, encode a result), and chunked streaming over the core's
  transport, which is how ArmoniK moves bulk data (the C# slice has streamed
  over its own transport only, `logs/csharp/stage17-streaming.log`). The first
  two are W11 requirements; streaming is optional.
- **Floors**: see section 2.2.
- **Concurrency on real hardware.** The concurrency suites that exist ran on
  small containers, and their coverage has open review findings (R-D7, R-D8).
- **The cost of getting there is unpriced everywhere.** The hand-written facade
  surface, the migration of existing callers, native-binary packaging and the
  test estate are the larger half of the work, and no work item prices them.
- **The open review findings** in `design/FIX-PLAN.md` section 7, each
  unconfirmed until the slice that owns the code answers it.

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

**This subtraction is formed only in the campaign.** It subtracts absolutes
taken in two slices, which R4 and R13 forbid across machines, so it exists only
once every slice runs on the same machine (W13). The same holds for
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
| C++ | C++11 | C++17 | A customer is pinned to C++11. `packages/cpp` sets `CXX_STANDARD 14`; both levels build and pass. |
| C# | **net6.0** and **.NET Framework 4.8** | **net8.0** | The client ships netstandard2.0 and the worker net6.0. `LibraryImport` needs .NET 7 or later, so both floors use `DllImport`, in the same generated file under `#if NET7_0_OR_GREATER`. No `UnmanagedCallersOnly` or `SuppressGCTransition` on the floors, so callbacks are delegate pointers there. .NET Framework runs only on Windows, so its gate needs a Windows machine; Mono is not a substitute. |
| Java | Java 8 | Java 17 | `packages/java` builds with `release` 17; the Java 8 floor is the binding's constraint. FFM is a JDK 22 API, so the floor and the target are both JNI; FFM is a secondary arm, not a target. |
| Python | **3.7** (`pyproject.toml` declares `>=3.7`) | **CPython 3.12**, the version Ubuntu 24.04 LTS ships | The generated C shim puts calls newer than 3.7 under `PY_VERSION_HEX` conditionals in one source. |
| Rust | MSRV 1.88 | MSRV 1.88 | One configuration; the floor is the target. The MSRV is declared, not yet verified. |

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
so there it is literally one emitted source tree per target level; C#, C++ and
the Python C shim get one emitted file with the levels spelled as conditional
compilation (`#if NET7_0_OR_GREATER`, `#if __cplusplus >=`, `#if PY_VERSION_HEX >=`).

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
`map<string, string>`), so a figure read off a shape the schema has three of is
labelled as a control rather than a result.

## 7. Work items

W1 and W2 block the slices. W3 to W7 are independent of each other. W11 and W14
come before the campaign (W13). W9 is the deliverable. The status column states
what exists and what was checked; it carries no timing, by design (section 1.1).

| # | Work item | Status | Done when |
|---|---|---|---|
| W1 | **Specify ABI v1.** One specification merging the base design with the amendments from the C# and Java reports. | **Drafted, not agreed.** `design/ABI-v1.md`. Decisions 3 (the string path) and 5 (the grow path) are settled in the draft. **Decision 1 is reopened**: the C++ evidence it rested on has no matching committed log (`design/FIX-PLAN.md`, R-C1), so it is decided in the campaign. Decision 11 (unknown-field retention) is open and measured in both modes. | The owner agrees the specification. |
| W2 | **Freeze the shapes and the payload set.** | **Done.** `schema/shapes.json` is the description; `schema/generated/` carries the emitted `.proto` and a payload manifest with a hash per payload, confirmed against prost and a second, independent encoder. P2.5 has two valid encodings; the manifest records prost's, and protobuf C++ and upb write the other (`design/SHAPES.md`). | Every slice agrees with every hash, or disagrees only on P2.5 as documented. |
| W3 | **Rust slice.** Section 4.1. | **Built.** Four arms over every message and payload, byte-identical to the manifest; content sets; unknown-field vectors; RPC arm and four-cell grid; ABI v1 7.1's pull decode family; obligation 12.5's concurrency suite; `ak_init` and the lifecycle. MSRV 1.88 not verified. | Campaign-ready under W11. |
| W4 | **C++ slice on the amended ABI**, with the C++11 floor. | **Built.** Full codec and RPC arm, shared library primary and static as a labelled second arm; upb and a borrowed-string facade as extra arms. C++11 and C++14 floors demonstrated. | Campaign-ready under W11. |
| W5 | **C# slice**: a managed decode control, oneofs and explicit presence, `core-ffi`. | **Built.** Managed control on all 16 payloads and 7 shapes; corpus consumer; `core-ffi` on every shape (encode, push decode, pull decode); RPC arm and four-cell grid; streaming over its own transport. Floor built as netstandard2.0 and run on Mono 6.8 only. | Floors net6.0 and .NET Framework 4.8 pass correctness; campaign-ready under W11. |
| W6 | **Java slice**, with the generated-Java-codec arm kept as a candidate. | **Built.** Full codec, Java 8 and 17 trees from one generator, JNI; pull decode arm; generated Java codec (arm R); RPC grid with the server in a second process. Java 8 floor: 437 of 437 checks on `openjdk 1.8.0_502`. Arm R has not run the corpus. | Campaign-ready under W11. |
| W7 | **Python slice.** Section 9. | **Built.** C extension shim over the shared core; M1 to M7, all 16 payloads, three facade storages; RPC arm as a three-cell grid; concurrency suite; first corpus consumer. Builds and passes on 3.10 to 3.13 (`poc/python/STATE.md`). Floor 3.7 not demonstrated. | Floor 3.7 passes correctness; campaign-ready under W11. |
| W8 | **Conformance corpus.** Section 10. | **Done.** 336 vectors, 328 sealed; three oracles; disputed rows excluded from pass counts. Consumed by the Python, C++ and C# slices. | Every generated codec in every slice consumes it. |
| W10 | **Consolidate the core into `poc/codec/`.** | **Done.** Every slice depends on `poc/codec/` by path; no slice carries a copy. | (met) |
| W11 | **Campaign specification**: `design/CAMPAIGN.md`, the contract every harness meets before the campaign. Outline in `design/FIX-PLAN.md` WP3. | Not started. | Written, and every slice's harness conforms with a committed smoke log. |
| W12 | **Divergence inventory**: where the five packages under `packages/` behave differently today (retried status codes, what `AllowUnsafeConnection` disables, `send_result` ordering, chunk sizes, windows, Nagle, keepalive, and whatever else is found), each row citing file and line in each package. No cost estimate, no judgement. | Not started. | Every row cites its sources. |
| W13 | **The campaign**: W11 executed on one physical machine, slices sequentially, client and server pinned to disjoint CPU sets. Owner-driven. | Not started. | Every slice's campaign logs are committed. |
| W14 | **One generator implementation.** Every generated codec and binding in every language comes from `poc/codec/gen/`, with the wire rules written once and language backends rendering them. Levels are conditional compilation inside one output where the language has it. Design in `design/FIX-PLAN.md` WP5. | Not started. Seven independent traversal emitters exist today. | Every generated file of every slice comes from one command; no wire rule outside the shared layer; every generated codec passes the corpus in both unknown-field modes. |
| W9 | **The report.** | Not started. | `REPORT.md` records, per option in section 13, the facts established and what is not established. It recommends nothing. |

**Keep the slices small.** No slice covers every message or every RPC. It covers
the shapes in `design/SHAPES.md` and nothing else. Where something is not
covered, it goes in the "not measured" list rather than into a larger slice.

## 8. How a slice is conducted

These rules are what make five separate slices comparable, and most were learned
the hard way in the three that already exist. A slice that breaks one produces a
number that cannot be used.

**The figures quoted inside these rules are illustrations of harness hazards**,
taken in containers. They show why a rule exists; they are not results (section
1.1).

**R0. One core, not one emitter.** The core lives once, at
[`poc/codec/`](poc/codec/), and every slice depends on it by path. No slice
carries a copy.

This is R1 one level up, and the branch learned it by breaking it: the emitted
`codec.rs` was byte-identical in three slices, so the *generator* was genuinely
shared, but the **hand-written runtime beside it had forked three ways** — the C++
slice added a counting entry point, the Java slice added `ak_tc_latin1` and
`ak_tc_utf16`, and each did it in its own copy. Neither change was wrong and
neither broke a measurement. The point is the mechanism: **each fork happened
because a slice needed to add something and the shared core had no way to accept a
contribution**, which is precisely the failure this whole branch exists to argue
about, reproduced inside it.

The rules that go with it:

- **A slice may add to the core, additively** — a transcoder, a counting entry
  point — and the addition lands in `poc/codec/` where every slice gets it.
- **A change to existing behaviour is not a slice's to make.** It goes to the
  aggregating session, because a change under one slice's feet invalidates the
  others' gates.
- **Every addition re-runs every slice's correctness gate**, not only the
  contributor's. Byte identity is what makes that cheap and it is already there.
- **No slice's build resolves the core by anything but a path dependency on
  `poc/codec/`.** A second copy is a defect, checkable mechanically.

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
facade objects. The control is not optional: it is what separates the boundary
from the codec, and on Java it is also option 2 of section 13.

**R4. Every ratio is formed inside one process on one runtime.** Absolutes do not
travel between runs on shared hardware; ratios within one process do, and the Rust
slice confirmed that directly — ±0.02 to ±0.05 on most rows across three builds
with a deliberate layout perturbation, the across-build spread no larger than the
same-binary spread.

**But a cross-arm ratio is more fragile than a within-arm delta, and one row
demonstrates it rather than arguing it.** The same UTF-8 finding, in one session,
expressed two ways: against prost it drifted; against **its own ASCII cost** it
reproduced almost exactly (2.545 and 3.365 against a published 2.2 to 3.0). So
**where a question can be asked as a delta between two arms in the same
interleaved rounds, ask it that way** — it survives things a ratio to a third arm
does not.

**And a figure whose harness is not in the tree cannot be defended at all.** The
Rust slice's published encode column could not be re-derived because the benchmark
binaries were untracked at that commit, swallowed by a `.gitignore` rule; the
oldest rebuildable commit disagrees with it and agrees with today. Section 12
already required every figure to name its log. A log whose harness is absent is
the same failure one level down, and it cost a headline. Any
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
**an RPC arm on P2.2 measures its own harness's socket options unless it measures
CPU.** A single call in flight spent most of its wall clock idle and a slice would
have reported 33 ms per call for a path costing 1.5 ms of CPU. **The conclusion
stands and the mechanism this rule used to name was wrong** — twice over, since the
correction below is itself the second reading.

**It was Nagle, not HTTP/2 flow control, and three facts settle it.** The same 540 KB
response costs 2 ms over a Unix socket and 30 ms over loopback TCP, and flow control
is a property of the protocol that is identical on both — a transport-independent
cause cannot produce a transport-dependent result. **A 1 KB response costs MORE than
a 540 KB one over TCP** (44 ms against 30), so the cost is per call rather than per
byte, and the direction is backwards for flow control (the large response is the one
that needs `WINDOW_UPDATE` round trips) and exactly right for Nagle, where a large
response has full segments to send and never waits while a small one is a lone short
write. And the cause is in the harness: tonic documents that `tcp_nodelay` is
**ignored** when a server is driven by `serve_with_incoming`
(`transport/server/mod.rs:701`) and `TcpIncoming::from(listener)` leaves its own
`nodelay` unset (`incoming.rs:120`), so the server kept Nagle on while tonic's client
had it off. A gRPC response is HEADERS, then DATA, then TRAILERS; the second small
write waits for the peer's ACK of the first, and Linux's delayed-ACK timer is 40 ms.

**One socket option, nothing else changed: 32,416 µs of wall clock per call became
2,145, and on a 1 KB response 44,041 became 149.** With it set, loopback TCP and a
Unix socket agree on both payloads and both columns. **So the transport gap this
branch has been reporting was a harness defect and not a transport**, every
loopback-TCP wall figure taken before it is measuring Nagle, and any slice that
reported UDS as faster than TCP reported this. The core's test server now sets the
option by default; `rpc::serve_nagle` keeps the defective form so the artifact stays
reproducible and nothing else should measure against it.

**What this costs beyond the numbers**: three slices spent effort pinning HTTP/2
windows to chase this gap, and raising the window bought a few percent where one
socket option bought 14× to 300×. The per-stack window guidance in `design/SHAPES.md`
is still correct and still worth having for a real deployment — it is simply not what
the loopback numbers were measuring. **A rule that names a mechanism sends people to
work on it, so naming the wrong one is not a harmless imprecision**, and this rule has
now done it twice.

**That is a default, not a property of HTTP/2, and stating it as the latter was
wrong.** A full window throttles a sender; it never caps a message, which is why
ArmoniK moves responses far larger than 64 KB. And the stacks diverge from the
initial window immediately: **grpc-java starts at 1 MiB with BDP auto-tuning on by
default**, .NET's `SocketsHttpHandler` starts at 65,535 with dynamic sizing on by
default to a 16 MiB cap, and **tonic/hyper starts at 65,535 with adaptive window
off** — which is the stack the 33 ms came from. So the same payload stalls on one
slice's transport and not on another's, and the wall-clock columns are not
comparable across slices unless the configuration is stated. **Every RPC arm names
its stream and connection window and whether auto-tuning is on** (R7), SHAPES.md
asks for CPU per RPC, and a wall-clock column is reported beside it or not at all.

**ArmoniK's own configuration is worth reading before copying a default.**
`GrpcWorkerServer` calls `.flowControlWindow(65535)` and then
`.initialFlowControlWindow(1024)`; in grpc-java the first sets the window and turns
auto-tuning *off* and the second sets the window and turns it *on*, so the second
wins and the first is dead code — the worker server starts from a 1 KB window and
lets BDP grow it. Separately, bulk data does not ride one large unary message at
all: it is chunked over streaming RPCs, so the large-payload path is many small
messages under the same flow control. Neither fact changes a codec ratio; both
change what an RPC arm is a measurement of.

**So the RPC arms measure ArmoniK's transport configuration, not their stack's
default**, which is R14 applied to the transport instead of to the codec.
`packages/rust/armonik-transport` is the reference implementation and the only package
that expresses these options: it disables Nagle by default
(`tcp_nagle_algorithm: bool`, "defaults to false", applied as
`http.set_nodelay(!config.tcp_nagle_algorithm)`) and **pins no HTTP/2 window at all**,
so the window below is the configuration ArmoniK intends rather than the one it ships,
and an arm that pins it labels it as such. The configuration to carry:

- **chunking at 2 MiB** for upload and download, where `ArmoniK.Api.Mock` still
  shows the old 80 KB `DataChunkMaxSize`;
- **a 4 MiB stream window**, sized to the largest message the stack accepts by
  default, so one maximum-size message crosses without waiting for a
  `WINDOW_UPDATE` at all.

At 4 MiB, P2.2's 540 KB response never fills the window, so **the wall-clock hazard
above is a property of the default and not of the configuration under test** — which
is the point of pinning the configuration rather than arguing about the default.
Two things to get right when pinning it, **and both are per stack rather than
general** — I stated them as general and the C# slice checked them against the runtime
source rather than relaying them.

- **The connection window is a separate knob on some stacks and not on others.**
  grpc-java's `flowControlWindow` reaches `SETTINGS_INITIAL_WINDOW_SIZE`, which is per
  stream, and tonic and hyper take the two separately — there, raising only the stream
  window leaves the connection at 65,535 and the stall comes back unchanged. **On .NET
  the hazard is not reachable**: `Http2Connection` hardcodes a 64 MiB connection window
  and raises it by `WINDOW_UPDATE` at setup, so at a 4 MiB stream window the connection
  is already sixteen times it.
- **An explicit window turns auto-tuning off in grpc-java and does NOT on .NET.** There,
  `flowControlWindow(int)` sets `autoFlowControl = false`. On .NET
  `Http2StreamWindowManager` takes the configured size as a *starting point* and doubles
  from it up to a 16 MiB cap, and `WindowScalingEnabled` is a separate switch that
  defaults on — so **a pinned window is a floor, not a cap**, and pinning 4 MiB there
  needs the property *and* the
  `System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing` AppContext
  switch, or the arm may be measuring 8 or 16 MiB by the end of the run.

Report the pinned arm as the headline and the stack default as a labelled second row.
**And note where pinning diverges from what ArmoniK ships**: `packages/csharp` sets no
window on either side, and the client builds an `HttpClientHandler` through which
`InitialHttp2StreamWindowSize` is not reachable at all — so on .NET the pinned arm
configures something the shipped client cannot. UDS is the opposite case and needs no
caveat: `GrpcChannel` already defaults to a Unix socket at `/tmp/armonik.sock` and the
worker already calls `ListenUnixSocket`. The rest:
JIT tiering and PGO off handicaps a managed incumbent, which the C# slice checked
rather than assumed: no arm there crosses 1.0 under any of three configurations,
and the default is the one *least* favourable to the managed arms. Two vCPUs is
the smallest contention a shared cache line can have, so a concurrency figure
from it is a lower bound and not a figure.

**The JVM hazard this rule used to state is real and larger than stated, and the
correction published here has itself been corrected once.** The java slice
measured it rather than avoiding it, and then took the JIT's own compilation log
rather than inferring from which triggers fire. Both readings are in the tree
(`logs/java/deopt.log`, then `logs/java/r9-mechanism.log`); what survives is this.

**The mechanism is branch pruning, not deoptimisation.** In every slow run C2
emits `inline_fail reason='call site not reached'` for `StringUTF16.charAt` at
both `charAt` sites in protobuf-java's `encodeUtf8`, and in every fast run it
compiles that branch with the `_getCharStringU` intrinsic. The payload's strings
are all above U+00FF, so the pruned branch is the one the measurement needs.
Runtime `uncommon_trap` events are 9 in the slow state against 11 in the fast one:
**the slow state has fewer traps, not more**, which is what rules deoptimisation
out.

**The effect is a probability, not a penalty.** Ten runs per mode land at about
620 us or about 1,250 us with an empty gap between. A Latin-1 probe before the
measurement reaches the fast state 10 times in 10; with no probe the process gets
there by itself about 1 run in 10. So the **2.16 times** this rule used to quote is
a ratio of two modes, not a cost a probe removes, and a single unprobed run is a
coin toss that lands slow nine times out of ten. **A published managed figure with
no repeat count is uninterpretable on this hazard**, which is the rule's real
content.

Four clauses of the earlier correction stand: it reproduces on **JDK 17 and not on
21**; the trigger is a read of a Latin-1 `String`'s chars rather than a numeric
conversion; no narrowing loop is involved; and **nothing happens on ASCII at all,
on either JDK, which is why every published managed figure has been blind to it**.
Two do not. The **1.27 times slower** reported for a generated Java codec was
attributed to a profile shared with protobuf-java's encoder, and the same logs
refute it: `ak.Utf8.encode` and `ak.Utf8.length` compile identically in all four
modes, never pruned, intrinsic always applied. And **the `String.format` trigger
this rule names no longer reproduces at all** (0 of 13 runs, against 4 of 4 when
it was first recorded), which the slice records as unexplained rather than
explaining. A slice measures this, repeats it, and says which content set each
string-path figure came from.

**The immunity survives the correction and is itself a result.** The C ABI arm
does not move, in either reading, because ABI v1 section 4 put the transcoder in
the core: that arm has no `charAt` site for C2 to prune. It is the only arm
insensitive to the host JIT's profile history, and it is an argument for the
design that no benchmark was looking for.

**And the instrument is part of the hazard.** `-XX:+TraceDeoptimization` and
`-Xlog:deoptimization` do not exist on a product build; `-XX:+LogCompilation`
preserves the effect; **JFR erases it** (642 us against 1,261). Where a hazard is
this shape, the first measurement is of the instruments.

**R10. Keep a defect log.** Each slice records the defects found in it and what
found them. Three of the most useful findings in the existing reports are defects
in a generator, not properties of an interface, and the rule they produced
("sweep a codegen rule across the generator, do not fix it where it was found")
is worth more than most of the timings.

**R11. Every slice ends with "what is not measured".** A slice that does not name
its gaps is not finished, and the report is assembled from those lists as much as
from the facts.

**R12. A slice agent never writes a report.** Section 11.

**R14. The baseline is the codec path ArmoniK actually runs, which is the gRPC
marshaller's, not the library's fastest entry point.** Section 13 sets the bar as
"what a unified core costs each language against what ArmoniK ships today", and a
ratio against an entry point no ArmoniK process calls does not answer that
question however fair it looks.

Application code barely serialises anything: a search of `packages/` finds almost
no direct calls, because **gRPC's generated marshaller does it**. So that is the
denominator. In C#, the stub `Grpc.Tools` emits calls
`context.SetPayloadLength(message.CalculateSize())` and then
`MessageExtensions.WriteTo(message, context.GetBufferWriter())`, and decodes with
`parser.ParseFrom(context.PayloadAsReadOnlySequence())` — verified in grpc's own
`src/compiler/csharp_generator.cc`, not inferred. **The size pass is production
code**, so removing it does not fix a handicap, it replaces the incumbent with a
faster thing nobody runs; and the payload arrives as a `ReadOnlySequence<byte>`
rather than a `byte[]`, which is a different parse path and is also where decision
13's borrowed spans would have to live. Each slice establishes the equivalent for
its own language and names it in its configuration line.

**This cuts both ways and the distinction is the whole rule.** A harness that
makes the incumbent do work its own library would not do is a defect, and three
slices have had one. A harness that picks the incumbent's *best* path when
production calls a slower one is the same defect with the sign flipped, and it is
harder to see because it looks like fairness. **Where the two differ, the headline
ratio is against what production runs**, and the library's best path is reported
beside it as a second row, labelled — because "the incumbent's fastest API is 20 to
29 percent better than what gRPC drives" is a real finding about the incumbent,
worth keeping and worth not confusing with the core's margin.

**And the check runs in both directions.** Every fairness check in this branch
was built looking outward — is the incumbent flattered? — and the java slice
found one pointing **inward**: its own `ffi-take` arm, the one every encode
headline is quoted from, was paying two crossings and two copies where the
incumbent's `toByteArray` pays one allocation and one copy. An arm that does
*extra* work is as wrong as a baseline that does, and it is harder to find
because nobody is looking for a reason their own result is too low.

Two notes from how that one went, both worth more than the fix. The same
redundant crossing had already been removed from the sibling arm four lines
away, because that earlier fix was aimed at the finding rather than at the
mechanism the finding named — **R10's sweep rule, one level up, at the harness
instead of the generator**. And when it was re-measured the fix turned out to
change nothing: 80 µs, under a noise floor where an untouched arm moved 124 in
the same pair of runs. The slice published that, kept both logs rather than
replacing the cited one, and recorded that its own prediction was refuted.
**A fix that changes nothing, reported as changing nothing, is worth more than
one that appears to work.**

A message is also serialised **once** in production, so a benchmark loop over one
message instance is not the shape to measure: it amortises anything the library
memoises per instance, which is exactly how protobuf-java's size-pass memo hid a
published regression for a year.

**R13. Until the campaign, slices run on separate machines, so every slice
calibrates its own.** A ratio formed inside one process survives the move to
another VM; an absolute does not, because it was a fact about one machine. A
per-runtime crossing table is a table of absolutes across languages, and
assembled from five containers it becomes a table about five containers. So **every slice builds and
runs the Rust slice's crossing benchmark on its own machine, as a step of its own
build**, and reports that machine's Rust crossing cost beside its own absolutes.
Every absolute a slice quotes is then also quotable as a multiple of its
machine's Rust crossing, which is what keeps the cross-language table
reconstructible.

This binds the slice that shares a machine with nothing as tightly as the ones
that do: the 1.8 ns figure is a fact about the container the Rust slice ran in,
and no later slice inherits it by running the same code somewhere else. A slice
that cannot run the calibration says so, and every absolute it reports carries
that gap.

**And until the campaign, no slice tries hard at cross-language performance.**
Performance is taken once, on one physical machine, with real control over
frequency scaling, pinning and isolation (section 1.1, W13). So today's absolutes are
instrumentation, not the deliverable, and effort spent making them precise buys
something that is about to be measured properly anyway.

What a controlled rerun **cannot** produce later is where a slice's effort
belongs now:

- **correctness and byte identity**, which gate everything and are not a timing
  question at all;
- **crossing counts**, which are a property of the interface rather than of the
  machine (the C++ slice reproduced the Rust slice's counts to the digit) and so
  are already final;
- **harnesses that can settle an ABI decision in the campaign**, because the
  decision is about a mechanism's sign and rough magnitude, and a container delta
  is only a hint of it;
- **feasibility**: that the shapes can be expressed, that the floor compiles,
  that the layouts agree, that a guard has been seen failing.

R13 stays, because it is one benchmark run and it labels a number that would
otherwise be read as a cross-language fact. It is not a licence to tune a
harness.

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
  field read costs the shim: a plain class, a `__slots__` class, or a C extension
  type whose fields the shim reads as struct members. Through
  `PyObject_GetAttr`, which is what a C shim actually calls, `__slots__` is not
  faster than a plain class in the container measurements, and neither is a C
  extension type reached through its member descriptor. **The storages differ in
  whether the shim can stop making a crossing**, and that is a count: only the
  struct member read moves, from **29 crossings per element to 7**, and on the
  absent path a getattr shim pays all 29 on an element that encodes to nothing.
  Keeping the facade idiomatic in all three is still the requirement, and a C
  extension type is the least idiomatic of them, which is a cost against the
  maintenance case that nobody has priced. The timings are for the campaign.
- **Speaking the C API is not unconditionally cheaper than being in Python.**
  CPython's specialised bytecode `LOAD_ATTR` has an inline cache and the C API
  has no equivalent entry point, so a C shim reading a plain facade through
  `PyObject_GetAttr` does the same work without the cache (in the container,
  slower). The C API can pay when it removes a crossing, not when it replaces
  one.
- **The GIL is still held for every C-API call**, so batching still matters:
  fewer, larger crossings mean fewer GIL-held stretches and a longer window in
  which the pure parse can run with the GIL released.
- **One control arm settles the premise rather than assuming it**: the same
  codec with Python-level accessors, measured once. If it is not clearly worse,
  the extra layer is not earning its place.

**This shape is Python's, and the java slice established that it does not
generalise — by pricing it rather than by building it.** The JVM analogue is a
generated C shim that writes facade fields through the JNI API instead of
upcalling into Java, and it was the most promising thing left unbuilt on the host
with the dearest crossing. Its primitives say it cannot win: a `SetObjectField`
is **26.7 ns under G1** and 13.7 under Parallel or Serial, against a cached upcall
of 72 to 80 ns on the same machine (container figures). **A JNI field store is a
large fraction of a whole upcall**, so the crossover between one upcall carrying k
stores in bytecode and k JNI stores with no upcall sits at a small k, and
`TaskDetailed`'s apply is k = 30.

**The two hosts differ in the ratio the section rests on.** On CPython a C-API
call on a primitive is far below interpreter re-entry, so speaking the C API
removes crossings and pays; on the JVM the cheapest thing that crosses is already
a third of a transition, so **a transition cannot be made cheaper, only rarer** —
which is ABI v1 7.1's pull family and open decision 10, not a second independent
route to it. Section 9.1 is therefore a Python design default and not an ABI-wide
one, and any later slice tempted by the shape prices its primitives first. Kept
for its own sake from the same probe: `SetObjectField` and
`SetObjectArrayElement` both double under G1 against Parallel and Serial while
`SetIntField` does not move, which is the G1 write barrier priced for any native
code storing a reference into a Java object.

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

- **The options apply to Python as to the others** (section 13): the core's
  codec, a codec generated into Python or into the C shim, or upb left in place
  with only the RPC layer on the C ABI. Python is the language whose incumbent
  codec is already native, which is why all three are measured.
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

### 10.1 The corpus is built and validated against ONE runtime, and that is now a known defect

W8's corpus generates every vector, every accepted encoding and every projection
with **protobuf 7.36.2 on the upb backend**, and validates the accept and reject
verdicts by parsing with upb (`corpus/emit/build.py`). One runtime deciding what
the right answer is makes that runtime's behaviour the specification, and the C++
slice found a row where it is the *minority* behaviour.

**`U-map-entry` puts an unknown field inside every map entry, and the runtimes do
not agree on the same bytes.** Reproduced here directly rather than taken on
report:

| runtime | what the map contains |
|---|---|
| **upb** | `{}` — the entry is dropped from the map and its bytes are retained as an unknown field of the parent (the re-encode is byte-identical to the input) |
| protobuf-python 4.25.9, pure backend | `{'k': 'v'}` |
| protobuf C++ (`protoc --decode`) | the entry is present |
| the cpp slice, both arms | `{'k': 'v'}` |

**One qualification on that third row, from the corpus agent and it is right**:
`protoc --decode` renders a map field as its wire-level repeated `MapEntry` list, so
it prints two entries with the same key on `E-map-dup-key`. It is therefore evidence
*about* a map question rather than a vote *on* one, and the substantive disagreement
rests on the pure-Python backend. Separately, protobuf C++ 35.1 drops the unknown
field inside the entry where 3.21.12 retained it as `3: 7` — that is open decision 11
and not this disagreement.

A map field is shorthand for a repeated `MapEntry` message, and an unknown field
inside a submessage is skipped while the submessage still parses — so three
implementations read it that way and upb's map parser appears to bail to the
unknown path when the entry carries anything but its two known fields. **The
corpus's projection is upb's**, so a conformant slice fails that row.

**The consequence is larger than one vector**, and it is the reason this is in the
README rather than in a slice's defect log: **every row where upb differs from the
other runtimes silently encodes upb's answer as the expected one**, and the only
rows anyone has checked are the ones where a slice happened to disagree. Until a
second independent runtime is an oracle for the projections, a corpus failure is
evidence that a slice differs from upb and not yet evidence that it is wrong.

### 10.2 Fixed: three oracles, and the rows where they disagree are disputed rather than decided

The corpus now runs **three** oracles — upb in process, protobuf-python's pure backend
in a subprocess (the backend is fixed at import, so it cannot be a second in-process
arm), and protobuf C++ through `protoc --decode` for the verdict on every row. **Zero
vector bytes and zero `.proto` bytes moved**; what changed is the manifest's claims
about them, and `generated/vectors.sha256` now freezes all 328 so the build refuses to
change, add or drop one.

- **A disagreement makes the row `disputed`**: it carries every reading, names the
  runtime that produced each, gives the dotted paths they differ on, and is **excluded
  from a consumer's pass or fail count**. Resolving by majority was refused, on the
  grounds that two of the three runtimes someone happened to ask is not a specification
  either. That is the right call and it is the one I would have been tempted to get
  wrong.
- **The build now fails only when EVERY oracle accepts a must-fail vector, or NO oracle
  parses an accept vector.** All three refuse all 49 must-fail vectors with no row
  disputed, which is a stronger claim than the single-oracle build could make.
- **Accepted encodings carry provenance**: `written_by`, and
  `observed_in_a_protobuf_runtime`, which is **false on 87 rows** where only the
  corpus's own writer produced the form. `B-P7_1` was the row that exposed this — its
  only accepted encoding was one no conformant encoder produces, so a consumer that
  re-encoded *correctly* failed C3. The cause was that baseline rows never went through
  the reconciliation every other row did; `permutation_accepted` now marks the four rows
  where a re-encoding may be any re-ordering of an accepted form, which is how
  `design/SHAPES.md` had always validated P7.1.

**87 of the accepted encodings had never been observed in any protobuf runtime**, and
that number is the honest measure of how much of the corpus was one writer agreeing
with itself.

## 11. How the work is run

Three roles, and the separation between them is what keeps the report honest.

### The aggregating session (this one)

Owns `README.md`, `CLAUDE.md`, `design/**`, `findings/**`, `REPORT.md`, and the
decision about what gets built next. It spawns the slice agents, reads what they
produced, and **is the only role that writes prose about results**. It writes
facts and what they do not establish; **it writes no recommendation**, because
the decision is the owner's. It does not run benchmarks itself.

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

**A slice may run in its own session on its own machine**, and past two slices it
should, because the last step of a slice is always a benchmark and two benchmarks
on one box corrupt each other silently: the numbers still come out. The cost is
R13's, and R13 is what pays it. Each such slice works on its own branch off this
one, which is free because the directories are disjoint (`poc/<lang>/` and
`logs/<lang>/`), and it keeps the rule that two agents never race a push.

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
    FIX-PLAN.md          the plan after the 2026-09-24 review, with its findings register
    CAMPAIGN.md          W11: the contract a harness meets before the campaign (to be written)
  poc/<lang>/            one slice per language, agent-owned
    STATE.md             the handoff contract. Read first, written last
    JOURNAL.md           what was tried, measured, refuted, in order
  schema/                W2: shapes.json, the one description every slice reads
    emit/                emitters: the .proto, the payloads, a framing check
    generated/           generated: shapes.proto, manifest.json, payloads/
  poc/codec/             the one core (R0) and, after W14, the one generator
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

## 13. The options the report describes

The question is **what a unified core costs each language against what ArmoniK
ships today**, set beside what maintaining five implementations costs. The
report describes the options below with the facts established for each and what
is not established. **It does not choose between them, rank them, or define a
threshold for "material"**: that is the owner's decision.

1. **Codec and RPC layer on the C ABI** for every language with a native
   binding.
2. **RPC layer on the C ABI, codec generated into each host language** from the
   same schema description by the same generator (W14). The C ABI carries the
   transport engine, where the crossing count is two per RPC rather than one per
   field.
3. **RPC layer on the C ABI, codec left to each language's existing protoc
   toolchain.** The incumbent codecs are generated code already; this option
   changes only the transport. It is cell B of the grid below, which every host
   has built.
4. **Per language**: any mix of the above, chosen per language.

What each option depends on, and so what the campaign has to produce for each:

| Option | Depends on |
|---|---|
| 1 | per-language codec cost of `core-ffi` against the production-path incumbent (R14), both directions; transport cost (cell C against A); crossing counts; floors |
| 2 | per-language cost of the generated host codec against the incumbent; transport cost (cell B against A, with the generated codec); that the generator really is one implementation (W14) |
| 3 | transport cost only (cell B against A); the divergence inventory (W12), since that is the part it unifies |
| 4 | all of the above, per language |

Every option also depends on facts no benchmark produces: W12's inventory, the
unpriced migration and packaging costs (section 3), and the unknown-field
decision (ABI v1 decision 11).

### 13.1 The RPC grid

Every RPC measurement before the grid moved the codec and the transport at once.
The grid separates them. It is defined once, identically for every host, in
`design/CAMPAIGN.md`:

| Cell | Codec | Transport |
|---|---|---|
| **A** | host's (incumbent) | host's (incumbent) |
| **B** | host's (incumbent) | the core's |
| **C** | the core's (through the C ABI) | the core's |
| **D** | the core's (through the C ABI) | host's (incumbent) |

`B - A` isolates the transport under the incumbent codec, `C - B` the codec under
the core's transport, and `D - A` the codec under the incumbent transport. If
`C - B` and `D - A` differ, the two halves do not add, and the report says so
rather than summing them.

**What the container grids taught about the harness**, kept because each lesson
is a W11 requirement and none is a result:

- **An in-process server can flip the sign of a delta.** A difference between two
  cells cancels the server's CPU exactly, so a sign change when the server moves
  out cannot be dilution; it is client and server contending inside one runtime,
  a property of the harness and of neither stack. The Java slice's transport
  delta changed sign when its server moved to a second process
  (`logs/java/rpc.log`). The C++, C# and Python grids ran their servers in
  process.
- **A cell's decode has to do the same work as the cell it is compared with.**
  upb's `FromString` is lazy and builds no Python objects, so a bare decode call
  against an eager facade decode is not the same work.
- **The CPU counter's resolution bounds what a grid can resolve.**
  `Process.TotalProcessorTime` moves in 10 ms steps on Linux.
- **CPU per call and wall clock per call can point in opposite directions**: a
  single queue drainer is cheaper per call and slower per second. Both are
  reported.
- **A marshaller that allocates per call can dominate a cell** (the Java grid's
  cell D, before it reused its buffer).
- **The transport is a large part of an RPC's cost**, so a codec ratio taken in a
  codec micro-benchmark overstates what a caller of an RPC sees. The campaign
  reports both levels.

## 14. Out of scope

- **The browser.** `packages/web` and `packages/angular` cannot load a native
  library, so they stay on generated gRPC-web whatever this branch concludes.
  Node through a native addon was considered and is not in scope.
- **Shipping anything.** No packaging, no release, no migration of a real
  consumer. Where those costs matter they are estimated and labelled as
  estimates.
- **Completeness.** Not every message, not every RPC, not every field shape.
- **Transport parity.** The PoC transport is minimal by design (section 1.1).

## 15. Open questions

Answered, kept for the record:

- ~~W5 and W6 scope~~: there was nothing to import; both slices were rebuilt
  against ABI v1.
- ~~Does ABI v1 get a C++11 re-check as part of W1~~: the C++ slice built ABI v1
  at C++11 (W4).
- ~~Is the C++ floor C++11 or C++14?~~ **C++11** (owner). Both build and pass
  (`logs/cpp/conformance.log`); `packages/cpp` sets `CXX_STANDARD 14`.
- ~~Python floor and target~~: floor **3.7**, target **CPython 3.12**, the version
  Ubuntu 24.04 LTS ships (owner). The wheel policy (full C API, one wheel per
  minor version, against the stable ABI) is section 9.2 and stays open.
- ~~C# and Java levels~~: C# floors net6.0 and .NET Framework 4.8, target net8.0;
  Java floor 8, target 17 (owner).
- ~~Traffic statistics~~: none exist (section 1.1).

Still open:

1. **How much of the real schema does a slice cover?** The slices use the
   shapes of `design/SHAPES.md`. The alternative is to drive every slice off the
   real `Protos/V1` descriptor.
2. **Java packaging.** A single jar is wanted; whether that means one bytecode
   level, a multi-release jar, or runtime capability dispatch is open, and may
   not need deciding if FFM never becomes a target (5.1.3).
3. **Who is the audience for `REPORT.md`?** A decision record for the team, or an
   AEP-shaped proposal. The base design notes this would be the largest breaking
   change in the repository's history and that an AEP process exists for exactly
   that.
4. **Python wheel policy** (section 9.2).
5. **Unknown-field retention** (ABI v1 decision 11): measured in both modes;
   decided by the owner.
