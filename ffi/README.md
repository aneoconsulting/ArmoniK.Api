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

**Every row of that table is provisional, and the C++ row is now known to be
wrong by a factor of five.** The rows were taken on different machines in different containers,
which makes it a table about machines as much as about runtimes. The C++ slice
measured its own crossing in the same process and build as its arms: **1.82 ns
through a shared library and 1.22 ns statically**, against a published 0.25 ns
that does not reproduce at all. On that same machine the rust slice's harness
measures 1.5 ns, and the gap is **not** a harness artifact — with a register-only
barrier in place of a memory clobber the C++ figure is 1.822-1.824. A C++ host
paying more than a Rust host through the same `.so` is itself a result nobody
predicted. **The table is re-taken in one process on one controlled physical
machine once the slices exist**, and until then the rule it produced survives
while the numbers under it do not. **A third container has now confirmed it**: the
python slice measures the rust crossing at 2.1 ns forward-plus-reverse where the
rust slice's own container gave 1.8 and the C++ slice's gave 2.1 to 2.2. Three
slices, three machines, three numbers for one quantity.

**And the table lists one number per runtime where a runtime has two.** The java
slice measured both directions on one machine, against its own 2.1 ns rust
crossing: **JNI forward 11.9 to 12.9 ns, a cached JNI upcall 75.8 to 91.0, a
naive upcall 295.9 to 309.4.** So the published 98.4 ns is an *upcall* figure and
the published 11.2 ns a *forward* figure, and they sit seven times apart in the
same column. Which direction a row reports decides the batching arithmetic that
depends on it, so every row of the re-taken table carries both, and a slice that
quotes one says which. Caching the method id is worth four times the call: that
is a binding defect the column can hide, not a property of JNI.

For the record, and in the form R13 asks for rather than as absolutes: a **Python**
forward crossing through a C extension is 11.4 to 12.5 ns net of the loop that
drives it, **4.1 to 4.5 times a Rust crossing on the same machine**. The row that
belongs beside the managed ones is not Python's interpreter re-entry (39.9 to 40.5
ns, which would sit between .NET 8 and JNI) but the **C-API call on a primitive at
2.83 to 20.54 ns**, because the whole point of section 9.1 is that this design
never makes the re-entry call. The rule is what the design rests on, and
nothing in it depends on which column is right.

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
- **The Rust slice is complete, and its behavioural half is now partly built.**
  Every shape and payload is measured (section 4.1, [`findings/rust.md`](findings/rust.md)):
  a crossing costs 1.8 ns; **the generated codec is 0.42 to 0.54 of prost on
  encode and crossing the C ABI hands all of that back, leaving parity**; decode
  converges to parity as an element gains containers; the group inverts the
  verdict on the absent path. An earlier encode figure of 0.79 to 0.92 through the
  ABI is **withdrawn**: it came from a harness that was never committed and the
  oldest rebuildable commit disagrees with it. Its last stage closed the three
  largest gaps this bullet used to list: **`ak_init` and the lifecycle are
  exercised** (the per-entry-point guard measures ±0.003 ns, indistinguishable from
  zero), **ABI v1 7.1's pull decode family is built** in the shared core from one
  emitter, and **obligation 12.5's concurrency suite exists** (0 wrong of 2,840
  encodes and 2,840 decodes).
  **What that suite found is the most serious defect the branch holds**: four
  threads on one encode context do not produce wrong bytes, they **abort the
  process**, because the panic's unwind is refused at the `extern "C"` frame. ABI
  v1 section 5 already makes `catch_unwind` mandatory at every entry point, and
  nothing enforced it; it is now a conformance obligation.
  **What is still not established**: the codec's rollback of a half-written field
  is written and never triggered; the MSRV of 1.88 is declared and unverified,
  because no such toolchain exists in that container; and on the RPC
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
| W1 | **Specify ABI v1.** One specification, in this branch, merging the base design with the amendments from the C# and Java reports. Every amendment carries the figure that motivated it and the language it came from. | **Drafted.** `design/ABI-v1.md` carries 12 decisions, of which **2 are now settled**: 5 (the grow path, keep the learned width) and 3 (the string path: no check on encode, reject on decode, free in both directions). The Rust slice produced all of the movement, and also created three: **9** (does the group need an empty-element path), **10** (can decode deliver the group before the runs) and **11** (does the core retain unknown fields). **11 is the one to read first**: it is a behaviour change for four of the five languages. **Decision 1 is now ANSWERED by the C++ slice and no longer gates agreement**: the group costs the host, string-as-data is a win, and batching has a crossover at a forward crossing of 2 to 4 ns rather than a verdict. Decision 9 is adopted with its wording corrected, section 4 gains the two-pass blob write, and **decision 13 is new**: borrowed decode spans as a facade option, the largest decode effect the branch has measured. |
| W2 | **Freeze the shapes and the payload set.** | **Done.** `schema/shapes.json` is the description, `schema/generated/` carries the emitted `.proto` and a payload manifest with a hash per payload, and the Rust slice has confirmed every hash against prost 0.14.4 and a second, independent encoder. One defect was found and fixed in `emit/payloads.py`; 8 of 16 hashes moved. A slice that disagrees with a hash now has a defect in itself, **with one measured exception: P2.5 has two valid encodings** and the manifest records prost's. protobuf C++ and upb both write an empty map value, at +80 B, so four of the five languages will disagree with that hash and be right. See `design/SHAPES.md`; taken literally the old sentence would have raised a false defect in three unbuilt slices. |
| W3 | **Rust slice.** Section 4.1. | **Done.** Four arms over every message and payload of `design/SHAPES.md`, all byte-identical to the validated manifest, plus the three content sets, the unknown-field vectors and the RPC arm. The decomposition every other slice subtracts is available: **a crossing costs 1.8 ns through a shared library**, and the RPC half costs **two crossings per call, zero per field**. **Stage 5 completes it** with the three things the branch had specified and nobody had built, all landing in the shared core: ABI v1 7.1's **pull decode family** (one emitter, both families; pull's reverse count is zero everywhere, so it REMOVES the upcalls rather than reducing them, at −5 to +18 percent of a push decode on this host), obligation 12.5's **concurrency suite** (0 wrong of 2,840 encodes and 2,840 decodes), and section 3's **`ak_init` and lifecycle** (the guard is ±0.003 ns). **Its positive control found the branch's most serious defect**: a shared encode context aborts the process rather than producing wrong bytes, because the panic cannot unwind through `extern "C"`. See [`findings/rust.md`](findings/rust.md) for what it does not establish, which is still longer than what it does. |
| W4 | **C++ slice on the amended ABI.** Rebuild against W1, re-measure against protobuf C++, and demonstrate the C++11 floor. **Done.** Full codec plus the RPC arm; shared library primary, static as a separately labelled second arm. It carried decision 1 and settled it. **Done.** See [`findings/cpp.md`](findings/cpp.md). "The managed amendments are free in C++" is now a measurement and the answer is no, not uniformly. 28 adversarial review findings answered, moving the encode column about 8 points and the RPC verdict 0.3, plus two arms nobody asked for: upb as a ceiling, and a borrowed-string facade. |
| W5 | **C# slice.** ~~Import the existing slice~~, rebuild against W1, then close its two named gaps: a managed decode control, and oneofs plus explicit presence. **There is nothing to import**: no branch carries the prior slice's sources and only the published report survives, so this is a rebuild and open question 1 is moot. **Done for M1, not for the shape set**; see [`findings/csharp.md`](findings/csharp.md). The named gap is closed: **C# does not look like Java on decode** (a generated pure-C# codec at 0.72-0.82 of `Google.Protobuf` on the real schema's shapes), oneof and explicit presence are covered in the facade and the managed codec, and the floor builds and passes on netstandard2.0 and on Mono. **The `core-ffi` arm is not built**, so half of W5 remains and the ABI half of the two field shapes with it. | Both gaps have numbers, and the floor (netstandard2.0 or net48) compiles and passes correctness. |
| W6 | **Java slice.** ~~Import~~, rebuild against W1, re-measure encode, and keep the generated-Java-codec arm as a first-class candidate. Nothing to import here either. **Done**; see [`findings/java.md`](findings/java.md). **The encode regression does not survive and the decode half of the verdict does**: the C ABI is 0.58-0.96 on encode and 1.22-1.62 on every M2 decode, where the generated Java codec beats it. The published regression is reconstructible from an incumbent that memoizes its size pass. The batching prediction was confirmed quantitatively, decisions 9 and 13 are answered for a managed host, and section 9's virtual-thread amendment is measured. **It then retracted two of its own published claims** on the JIT's compilation log: R9's hazard is C2 pruning an unreached branch rather than deoptimisation, and it shifts a probability rather than imposing a 2.16x cost. **README 9.1's C-shim arm is priced and refused** — a JNI field store is a third of an upcall, so on the JVM a transition can only be made rarer, not cheaper, which is 7.1's pull family and nothing else. **And it built ABI v1 7.1's pull arm, which answers open decision 2**: reverse crossings go to zero, the M2 decode regression is gone (P2.2 from 1.383 of protobuf-java on push to 0.807 on pull), pull is never slower than push on any payload, the drain copy does not measure on the JVM, and against the no-boundary control it is a tie — so pull stops the C ABI losing to a generated Java codec rather than making it win. No grpc-java transport comparison exists, and it is now the slice's largest gap. | The encode verdict is stated against ABI v1, on JDK 17 with JNI, with the Java 8 floor demonstrated. |
| W7 | **Python slice.** Section 9. **Work unit 1 done**, slice proper not started; see [`findings/python.md`](findings/python.md). The mechanism is settled (C extension; `ctypes` and `cffi` refused for the codec, their callback being 165-170x a C-to-C call), the storage is settled for encode, and 9.1's premise holds. **It also removed outcome 2 from the table for Python**: the generated pure-Python codec is 19.4 to 20.3 times upb. **Work unit 2 composed the two edges and M2 has since landed**: the shared core behind the generated CPython shim is **0.700-0.719 of upb on encode** and **0.888-0.897 on decode once both sides have produced Python values** (1.77-1.80 on the bare call, because `FromString` materialises nothing). The core and the boundary together cost about 0.10 of a upb encode, and the ABI's own crossings are **0.01 per element** against 7 for the shim-to-facade edge, so Python's crossing problem is not the ABI's. It is also the corpus's **first consumer**, which is how the shared core's group-skip defect was found. M3 to M7, most of the payload set, the RPC arm and concurrency are not built. | Python has a verdict of the same shape as the others, or a stated reason why the question is different there. |
| W8 | **Conformance corpus.** Section 10. **Done**, on its own branch: `corpus/` holds a generator, **336 vectors** (74 unknown-field, 30 empty, 145 shape, 53 transcode, 8 chunking, 18 malformed, 8 baseline), a manifest carrying a *set* of accepted encodings per vector, projections of what a reader must SEE, a consumer contract so five slices do not each invent one, and a gate with a selftest. Validated against upb, an implementation sharing no code with the writer. **No slice consumes it yet**, and the first one that does is the second opinion. | Every slice produces and consumes the same bytes, and the corpus is generated rather than curated. |
| W10 | **Consolidate the core into `poc/codec/`.** Move the emitters out of `poc/rust/gen/` and the crates out of `poc/rust/crates/`, fold in the C++ slice's counting entry point and the Java slice's two transcoders, and re-point every slice at one path dependency. | No slice contains a copy of the core, every slice's correctness gate passes against the shared one, and `codec.rs` exists once. **Low measurement risk**: the emitted codec is already byte-identical in three slices and both deltas are additive, so this re-gates rather than re-measures. |
| W9 | **The report.** | `REPORT.md` states a recommendation, the evidence for it, and what it does not establish. |

**Keep the slices small.** No slice covers every message or every RPC. It covers
the shapes in `design/SHAPES.md` and nothing else. Where something is not
covered, it goes in the "not measured" list rather than into a larger slice.

## 8. How a slice is conducted

These rules are what make five separate slices comparable, and most were learned
the hard way in the three that already exist. A slice that breaks one produces a
number that cannot be used.

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
facade objects. The control is not optional: on Java it is the arm that changed
the recommendation.

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
from the verdicts.

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

**R13. Slices run on separate machines, so every slice calibrates its own.** A
ratio formed inside one process survives the move to another VM; an absolute does
not, because it was a fact about one machine. The per-runtime crossing table of
section 2 (0.25 ns in C++, 1.8 ns in Rust, 7.5 to 12 ns on .NET 8, 98.4 ns
through JNI) is a table of absolutes across languages, and assembled from five
containers it becomes a table about five containers. So **every slice builds and
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

**And until the controlled run, no slice tries hard at cross-language
performance.** The cross-language comparison is being re-taken on a physical
machine, with real control over frequency scaling, pinning and isolation, once
every slice exists and the ABI is validated. So today's absolutes are
instrumentation, not the deliverable, and effort spent making them precise buys
something that is about to be measured properly anyway.

What a controlled rerun **cannot** produce later is where a slice's effort
belongs now:

- **correctness and byte identity**, which gate everything and are not a timing
  question at all;
- **crossing counts**, which are a property of the interface rather than of the
  machine (the C++ slice reproduced the Rust slice's counts to the digit) and so
  are already final;
- **within-process deltas that settle an ABI decision**, because the decision is
  about a mechanism's sign and rough magnitude, not about its nanoseconds;
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
  type whose fields the shim reads as struct members. **Measured, and the
  ordering this document used to give is wrong.** Through `PyObject_GetAttr`,
  which is what a C shim actually calls, `__slots__` is *slower* than a plain
  class (11.20-11.30 ns against 9.80-9.88) and so is a C extension type reached
  through its member descriptor. The three do not differ in how fast a crossing
  is. **They differ in whether the shim can stop making one**: only the struct
  member read moves, from 29 crossings per element to 7, and only it beats upb
  (0.600-0.612 against 1.19-1.25 for the getattr arms on P1.2). The absent path
  separates them five-fold, because a getattr shim pays all 29 crossings on an
  element that encodes to nothing. Keeping the facade idiomatic in all three is
  still the requirement, and a C extension type is the least idiomatic of them,
  which is a cost against the maintenance case that nobody has priced.
- **Speaking the C API is not unconditionally cheaper than being in Python**, and
  this is the argument the section was missing. A specialised bytecode
  `LOAD_ATTR` costs 3.37-3.91 ns where `PyObject_GetAttr` from C with an interned
  key costs 9.44-9.52: CPython's interpreter has an inline cache and the C API has
  no equivalent entry point, so **a C shim reading a plain facade does the same
  work about 2.5 times more slowly than the interpreter would.** The C API pays
  when it removes a crossing, not when it replaces one.
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
of 72 to 80 ns on the same machine. **A JNI field store is a third of a whole
upcall**, so the crossover between one upcall carrying k stores in bytecode and k
JNI stores with no upcall sits at **k = 2 to 3**, and `TaskDetailed`'s apply is k
= 30 — about 790 ns of stores against about 80 ns of transition plus the same
stores at a few ns each, plus 123 to 139 ns per element for `NewObject` where Java
pays a bytecode `new`.

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

   **This outcome is not available in Python, and that is now measured rather
   than suspected.** The generated pure-Python codec is **19.4 to 20.3 times upb**
   on P1.2 — the same arm that on Java beats the C ABI in both directions. So
   "generate the codec" is a recommendation for the managed runtimes and a
   non-starter for the one language whose incumbent is already native. Stated
   without that qualification it is wrong for a fifth of the estate, and with it
   outcome 2 collapses into outcome 3.
3. **Adopt per language.** C++ and Python on the full core, managed runtimes on
   the generated codec, one RPC layer everywhere.

The report states which, and states the evidence that rules out the other two.

### 13.1 The RPC grid, and why outcome 2 is not one recommendation

Every RPC figure the branch had before today moved the codec and the transport at
once. The grid separates them: **A** is the host's codec on the host's transport,
**B** is the host's codec on the **core's** transport (which *is* outcome 2), **C**
is both on the core's. `B − A` is the transport half and `C − B` is the codec half.
Four hosts have now run it.

| host | `B − A`, the transport half | `C − B`, the codec half |
|---|---|---|
| **Java** | **+368 µs at 8 in flight — the core's transport COSTS**, 6 runs of 6 | **−250 to −330 µs — the core's codec SAVES**, 6 of 6 |
| **C++** | separates in only **1 of 12** rows (+1.4 % to +13.8 %) | **−38.4 % to −8.5 %, saves**, 12 of 12 |
| **Python** | **3 to 15 % CHEAPER than grpcio**, 6 of 6 | **+3.1 to +3.6 ms — a large loss** |
| **C#** | the transport is **3 to 10 times** the codec's cost | (the four codec arms do not separate end to end) |

**The first result is that the total hides its own components.** On Java, C against
A — the whole core stack against the whole host stack — is **+101 µs on a 2,453 µs
call, about 4 percent**, which reads as "no difference". It is **+368 and −268**. A
report quoting only A against C would have hidden a transport that costs and a codec
that saves, and would have been wrong in both directions at once.

**The second is that outcome 2 is not one recommendation, because both of its halves
change sign by host.** Outcome 2 adopts the RPC layer and generates the codec:

- **On Java it is the worst available combination.** It takes the half that costs
  (+368) and drops the half that saves (−268). That is the opposite of what the
  original Java report recommended, and the recommendation was made without the
  grid that would have shown it.
- **On Python it is exactly inverted.** The transport half is a win (3 to 15 percent
  cheaper) and the codec half is a 34× to 60× loss, so outcome 2 is **on** the table
  for Python's transport and **off** it for Python's codec — which is the shape this
  slice already established from the codec side alone.
- **On C++ the codec half is the whole of it** (12 of 12, up to 38 percent) and the
  transport half barely separates, except on an empty call where grpc++ costs 44 to
  89 percent more.

**The third is a caution against arithmetic.** Java's `D − A` — the same codec swap
under grpc-java's transport rather than the core's — is **mixed in sign** where
`C − B` is consistent. So the codec's saving is visible under one transport and not
the other, and the two halves may not simply add. An earlier Java grid appeared to
prove they could not (opposite signs, `C − B` at −703 against `D − A` at +399); that
was cell D's marshaller allocating a fresh 540 KB array per call, and **the exciting
result was the harness**.

**And the transport is most of an RPC.** Python's no-decode floor is 1.69 to 2.35 ms
of a 2.8 to 3.5 ms call, so **the transport is roughly two thirds of what a call
costs**, which is the same shape as C#'s finding that the codec is about 10 percent
of CPU per call where the in-process column says 25. **Every headline ratio in this
branch is an in-process codec ratio**, and the grid is what says by how much they
overstate what a caller feels.

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
