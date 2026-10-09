# csharp slice: journal

What was tried, what it measured, what refuted it, in order.
Append; do not rewrite. A later reader comes here to find out that an option
was already refuted and by what.

## Entries

### 1. R13 first: the calibration, before anything of this slice existed

`ffi/poc/rust` built and its crossing benchmark run on this container before a
line of C# was written. **1.8 ns forward and 2.1 ns forward-plus-reverse, five
runs, median identical in every run.** That reproduces the Rust slice's own
published figure to the digit.

Worth stating as a result rather than as a formality: R13 exists because the
1.8 ns figure is a fact about one container, and this run establishes that this
container is the same one to within the measurement. **An absolute in this
slice is therefore comparable with a Rust slice absolute** -- which had to be
measured and could not have been assumed, and would have been wrong to assume
if the containers had differed.

### 2. There was nothing to import, so this is a rebuild

W5 says "import the existing slice". `git branch -a` carries one branch and no
C# sources anywhere in the tree. The published report survives and is prior art
to reproduce against; nothing was imported.

### 3. Google.Protobuf on .NET has no deterministic map serialization

`MapField` writes entries in INSERTION order and .NET's runtime offers no
`Deterministic` switch (the C++ and Java runtimes do). The canonical form sorts
map entries by key, so **canonical bytes from the incumbent come from the HOST
inserting sorted**, which is what `BuildGp` does. Probed directly before the
facade was designed: inserting k03, k00, k02, k01 writes k03, k00, k02, k01.

This decided the facade's map type. A `SortedDictionary` facade would pay
red-black inserts on decode while the incumbent pays hash inserts, and the
managed decode column -- the single most valuable number in this slice -- would
have been a comparison of container types. `OrderedMap` is a `Dictionary` plus
a `List`, which is what `MapField` is.

The Rust slice reached the same place from the other side: prost's default
`HashMap` cannot produce canonical bytes at all, so it used `btree_map`.

### 4. The correctness gate passed on the first run, all 16 payloads, all 4 arms

Byte identity against the Rust-validated manifest, first time, with no
adjustment to any value rule. That is worth recording because it is evidence
about the DESCRIPTION rather than about this slice: the value rules of
`emit/values.py` were re-derived in C# from the Python, and two independent
implementations landing on 16 identical sha256s says the description is
unambiguous.

### 5. The single-pass encode is worth about a factor two, and it is measured

The published C# encode figure (0.22 to 0.43 of `ToByteArray`) is surprising
enough to need a control under R2. Two were built:

- **`managed-2pass`**, the same generated codec with `SizeOf` then
  `WriteSized`: two passes, exact length prefixes, no learned width. That is
  the shape `Google.Protobuf` uses, so `managed` against `managed-2pass` is a
  WITHIN-ARM delta in the same interleaved rounds (R4) and prices the single
  pass alone.
- **a memcpy floor**, `Buffer.BlockCopy` of the payload's own bytes.

Result: managed is 0.22 to 0.43 of the fair incumbent baseline, and
managed-2pass is 0.31 to 0.94. **So roughly half the encode win is the single
pass with a learned width and the other half is the generated traversal.**
Neither arm is anywhere near the memcpy floor on a non-bulk payload (it sits at
0.002 to 0.034 there), so the win is not a copy artifact.

### 6. On the BULK payloads every arm is ON the memcpy floor

P5.2 (64 KB), P5.3 (1 MB) and P5.4 (4 MB): `managed`, `managed-2pass`,
`gp-writeto` and the memcpy control all sit within a few percent of each other
and of the copy. The reportable claim is "a bulk encode costs one copy in every
arm", which bounds both sides, and not a ratio between two copies.

`ToByteArray` is 3.7 to 7.6 times `gp-writeto` on those rows, and that entire
column is the allocation of a multi-megabyte array.

This is the Rust slice's P5.4 lesson reproduced in the other direction: there
the floor arm turned a suspicious WIN into a statement about prost; here it
turns a suspicious PARITY into a statement about what a bulk payload is.

**And the same floor is what stops an M5 DECODE row being reported as a loss.**
P5.3 decode measures the managed arm at 1.16 to 1.29 of the incumbent while
P5.4 measures 0.83 to 0.96, in the same three processes, with the memcpy floor
at 0.12 to 0.27. Both arms are on the copy and what separates them is allocator
luck on a multi-megabyte buffer. Without the floor arm the P5.3 row would have
read as "the managed decoder is 20 percent slower on a 1 MB body", which is a
sentence about nothing.

### 7. Arm b was measuring nothing, twice, and both times the harness said so

**First**: the floor build reported `floor sources: no`. An earlier edit had
deleted the `AkFloor` PropertyGroup from `Facade.csproj` while re-adding it
only to `Harness.csproj`, so the define never reached the code under test. The
contract's rule -- "when a change does not do what it should, the first
hypothesis is that it is not running" -- found it in one step, because the
harness prints the flag out of the facade assembly rather than asserting it in
a log.

**Second, and worse**: `BuildInfo.Floor` was a `const bool`. A `const` is
inlined into every assembly that reads it, so the harness would have reported
the flag IT was compiled with rather than the facade's. Changed to
`static readonly`. A configuration line that can be wrong without anything
failing is the one part of a log that has to be generated.

### 8. Arm b costs nothing, and the sharp version of that test is the wide set

Floor sources on the target runtime, two binaries run round robin with a
rotating order, each carrying the incumbent as its in-process control column
(R4's rule for a comparison that cannot share a process). **b/a is 0.93 to 1.04
on every row.**

That result on ASCII alone is nearly a measurement of nothing: the two builds
differ only in which `Encoding.UTF8.GetBytes` overload the transcoder calls,
and on ASCII a narrowing transcoder has no work to do. So the same table was
taken on latin1 and wide, where it does. **b/a is 0.947 to 1.062 there too**,
and on the encode rows the floor's unsafe pointer overload is if anything
marginally the faster of the two. The floor is a viable deployment and not only
a viable compile.

### 9. The `wide` content set was not the Rust slice's `wide`, and R1 says that is a defect

`ffi/schema` emits ASCII only and names the other two sets by character RANGE.
"Above U+00FF" admits a two-byte and a three-byte encoding, and the first
version of this slice's skew picked mostly two-byte: **wide measured 1.78 to
1.84 times the ASCII wire against the Rust slice's published 2.39 to 2.50.**

That is not two runtimes disagreeing. It is two slices choosing different
characters, which under R1 is a defect rather than a difference -- and it is
exactly the failure SHAPES.md already records once, where P7 was 958 B in one
published slice and 1,016 B in another. Changed to three bytes throughout;
latin1 now measures 1.697 and 1.748 against Rust's 1.70 to 1.75, and wide
measures 2.394 and 2.495 against Rust's 2.39 to 2.50.

Agreeing by hand is what one description exists to stop, so it is raised in
STATE.md as a request rather than left as a coincidence that currently holds.

### 10. The content sets move the encode headline and do not move the decode one

The managed encode advantage narrows from 0.31 to 0.43 of the incumbent on
ASCII to 0.45 to 0.61 on wide. In R4's within-arm form, `managed` costs 2.24 to
2.32 times its own ASCII self on wide while `gp-writeto` costs 1.53 to 1.64 and
`managed-2pass` costs 1.55 to 1.69.

The mechanism is arithmetic and not a defect: the single-pass arm has a higher
SHARE of its time in the transcode, which is the part that scales with output
bytes, so the same slowdown in that part moves its total further. A slice
reporting the ASCII encode number alone would have overstated the win by about
a third on the hardest content.

Decode is barely moved: 0.65 to 0.69 on ASCII, 0.73 to 0.78 on wide. **The
decode conclusion survives every content set.**

### 11. Google.Protobuf retains unknown fields on .NET; the generated codec drops them

Seven hand-built vectors, because no payload emitted from the description a
decoder was emitted from can carry a field that decoder does not know. Both
arms accept all seven and neither loses a known value. But the incumbent writes
the unknown field back and the managed codec does not.

This is ABI v1 open decision 11 with a cost attached, and **the direction is
the opposite of Rust's**: prost drops unknown fields too, so the Rust slice is
structurally the wrong place to price what removing the guarantee costs. On
.NET it is a guarantee that exists today.

The oneof vector behaves as `design/SHAPES.md` says it must: retention makes the
BYTES of the unrecognised member survive and does not make the VALUE survive,
because the grouping lives only in the descriptor. Those are different claims
and the vector separates them.

### 12. Decision 5 reproduces the Rust slice's answer exactly, on a different runtime

Zero warm prefix-width misses on every payload except P2.4, which misses once
per element: 80 misses in 80 elements, moving 979,181 of 979,465 bytes. The
Rust slice measured one miss per element moving 980,938 of 981,222.

The per-site tally names the field -- `ListTasksDetailedResponse.tasks` -- and
it took a counting build to get it. The first version diffed the width table
between two encodes instead, and reported `none`, because the site oscillates
2 -> 3 -> 2 and lands back where it started. A lower bound that reads as zero is
worse than no number, so the counting build is now `/p:AkCount=true` and the
store is out of the measured build, which is the Rust slice's `count` feature
spelled for C#.

### 13. Scope change: absolutes deferred, and what that made worth measuring

The exploration branch's 4aec4e8 (`docs(ffi)`: today's absolutes are
instrumentation, not the deliverable) was merged into this branch, and with it
the C++ slice. Nobody tries hard at cross-language performance until the
controlled physical-machine rerun.

Almost nothing in this slice was affected, because almost nothing in it was
precision work: the correctness gate, the managed decode control existing at
all, the oneof and explicit-presence coverage, and the floor's feasibility are
exactly the list the new rule says a rerun cannot produce later. What it
changed is how the tables are LABELLED, not what is in them.

One thing it made newly worth measuring, and one thing it made worth stating.

**Worth measuring: the JIT configuration, because R9 says it moves a verdict
and not a decimal.** Three processes differing only in environment variables.
Result:

- **Tiering off or PGO off slows the INCUMBENT by 5 to 20 percent**
  (`gp-writeto` 1.10 to 1.20 times its default, `gp-parse` 1.05 to 1.11). R9's
  hazard is confirmed on this workload and in the direction it names.
- **No arm crosses 1.0 under any of the three configurations.** managed encode
  stays at 0.27 to 0.43, managed-2pass at 0.62 to 0.90, managed decode at 0.64
  to 0.72.
- So the default -- tiering and PGO ON, which is what a deployed service runs
  and what every other log here used -- is the configuration LEAST favourable
  to the managed arms. The numbers this slice reports are the conservative
  ones, which is the right way round and had to be checked rather than assumed.

**Worth stating: this container's calibration disagrees with the C++ slice's.**
The merged README now records that the C++ slice measures the Rust crossing at
**1.5 ns** on its container and cannot reproduce the 0.25 ns C++ row at all
(1.24 ns statically). This container measures **1.8 ns**, which is the Rust
slice's own figure to the digit.

So the three containers are not interchangeable and R13 is what says so. It
also means something concrete for the report: **a C# absolute here is
comparable with a Rust absolute and is NOT comparable with a C++ absolute**,
and that is a measured statement rather than a caveat.

### 14. Two handicapped incumbents, both mine, found by looking for them

The aggregating session relayed that an adversarial review of the C++ slice
found its incumbent handicapped three ways and moved its headline by about
eight points, and said to look for the analogous thing here rather than wait
for a review. There were two, and the second is worse than anything the C++
review found.

**One: the encode baseline paid a size pass the incumbent does not have to.**
`gp-writeto` was `msg.CalculateSize()` then `msg.WriteTo(Span<byte>)`, where
the `CalculateSize` exists only to size the span. `Google.Protobuf` offers
`msg.WriteTo(IBufferWriter<byte>)` in the same official API family, which
sizes nothing at the top level. Added as `gp-bufferwriter` over a reused
`ArrayBufferWriter`, **reset rather than cleared**, because
`ArrayBufferWriter.Clear()` zeroes the written span and that is precisely the
per-iteration buffer wipe that handicapped the C++ slice's incumbent.

It is **0.708 to 0.806 of `gp-writeto`** on every payload measured. So the
incumbent's best encode path is 20 to 29 percent faster than the baseline
this slice was quoting against, and every managed encode ratio in the
published stage 3 was flattered by that much.

**Two, and this one is on the single most valuable number in the slice: the
DECODE baseline did a full extra traversal.** The generated `GpParse` ended

    return m.CalculateSize();

written to keep the decoded graph from being optimised away. `CalculateSize()`
walks the entire decoded tree. `ManagedParse` returned `d.Pos`, which is free.
**The incumbent was paying a size pass the managed arm was not, on the decode
column this slice exists to produce.**

Both arms now park the graph in a static `object` sink and return an O(1)
value. A store cannot be elided and costs the same in both.

What it was worth, hand-rolled harness, same machine, same build:

| payload | published | corrected |
|---|---|---|
| P1.2 | 0.648 - 0.694 | **0.764** |
| P1.3 | 0.451 - 0.490 | **0.593** |
| P2.2 | 0.642 - 0.683 | **0.815** |
| P3.1 | 0.660 - 0.664 | **0.817** |
| P4.1 | 0.661 - 0.665 | **0.843** |

**The verdict survives and the margin does not.** Managed decode is still
below 1.0 everywhere, so C# still does not look like Java on decode and
README section 13's outcome 2 still does not follow from this column. But the
honest figure is 0.59 to 0.84, not 0.45 to 0.83, and the correction is larger
than the eight points the C++ review moved.

**The lesson, which is not "check your baseline".** It is that *whatever keeps
a benchmark result alive has to cost the same in every arm*. The guard was
added for a real reason, dead-code elimination, and it was the guard that was
asymmetric, not the codec. A reviewer reading the arm table would have seen
two methods that both "parse and return an int".

### 15. P2.5's two encodings, and a correction to what was relayed

`design/SHAPES.md` now records that an empty map value is an
implicit-presence leaf: the manifest omits it, protobuf C++ and upb write it,
at +80 B on P2.5. The expectation relayed to this slice was that
`Google.Protobuf` writes it too.

**It does not.** `harness mapforms` measures it: `ToByteArray` produces 19,632
B, the manifest's form, and so does the generated codec. That is also why
stage 1 passes byte identity on P2.5 with no special case, which was already
evidence in hand. So the split is prost and Google.Protobuf omitting against
protobuf C++ and upb writing, not managed against native.

The other half had to be tested rather than reasoned about, and was: the +80 B
form is built by rewriting the committed vector (40 insertions, 19,712 B, both
checked), and **both decoders accept it and normalise it back to the canonical
form**, because an empty and an absent map value are the same facade value.

The rewriter needed to catch its own bug first: `p += (int)ReadVarint(b, ref p)`
adds the body length to the PRE-varint offset, because C# loads the left
operand of `+=` before evaluating the right and `ReadVarint` advances `p`
itself. It surfaced as a phantom wire type 4 two fields later. The generated
decoder is unaffected, spelling it `Pos = LenEnd()`.

### 16. CORRECTION to entry 14: `gp-writeto` was not handicapped after all

Entry 14 claimed the encode baseline was handicapped by a top-level
`CalculateSize()` that `WriteTo(IBufferWriter<byte>)` avoids, and that the
incumbent's best path is therefore 20 to 29 percent faster than what this
slice quoted against. **The measurement is right and the conclusion drawn
from it is wrong**, and the aggregating session asking whether the finding is
a fact about what ArmoniK ships is what exposed it.

`packages/csharp` makes essentially no direct serialization calls: the only
two hits are `ByteString.ToByteArray()`, which copies a blob and does not
encode a message. Everything goes through gRPC's generated marshaller, and
`Grpc.Tools` 2.66 emits this:

    static void __Helper_SerializeMessage(IMessage message, SerializationContext context)
    {
      if (message is IBufferMessage)
      {
        context.SetPayloadLength(message.CalculateSize());
        MessageExtensions.WriteTo(message, context.GetBufferWriter());
        context.Complete();
        return;
      }
      context.Complete(MessageExtensions.ToByteArray(message));
    }

It calls **both**. The size pass is not an avoidable inefficiency: gRPC needs
the payload length before it writes the length-prefixed frame header, so a
client cannot skip it by choosing a different overload.

So:

- **`gp-writeto` (CalculateSize + WriteTo(Span)) is structurally what ArmoniK
  pays** and stays the baseline. It was never handicapped.
- **`gp-bufferwriter` is faster than anything an ArmoniK gRPC client can
  reach.** It is not "the incumbent's fastest official path"; it is the
  incumbent *without the size pass*, which the marshaller does not permit. It
  stays as an arm, relabelled, because what it now prices is exactly **the
  size pass in isolation**: 20 to 29 percent of an encode.
- The marshaller's own shape, CalculateSize + WriteTo(IBufferWriter), is
  bracketed by the two arms and is within noise of `gp-writeto`, the two
  differing only in span against buffer-writer for the write half.

**What this does to the managed control's case, and it strengthens it.** The
single-pass codec's advantage is not an artifact of a badly chosen baseline:
a gRPC client genuinely pays a size pass it cannot avoid, and a codec that
buffers its own output and reports the length afterwards genuinely does not.
The `managed` against `managed-2pass` delta was already the within-arm form of
that, and it now has a reason in the incumbent's own call path rather than
only in a synthetic arm.

**The decode half of entry 14 is untouched and stands.** `m.CalculateSize()`
in `GpParse` was a real defect with no counterpart in the real call path, and
the corrected decode column, 0.59 to 0.84, is the honest one.

The generalisable rule from entry 14 also stands and is worth stating as a
rule rather than as an anecdote: **whatever stops a benchmark result being
optimised away has to cost the same in every arm.** The C++ slice met the same
class of problem from the other side, with a control that was not doing the
work its arm did. A guard is code, and an asymmetric guard is an asymmetric
benchmark.

### 17. The two harnesses agree, and BenchmarkDotNet is the conservative one

144 BenchmarkDotNet benchmarks against the hand-rolled harness's three
interleaved processes, 64 comparable rows. Median deviation **+0.019**, mean
**+0.021**, 53 of 64 within +/-0.05, **2 verdict flips**.

The sign is the interesting part: **47 of 64 deviations are positive**, so BDN
reports this slice's own managed arms slightly worse than the interleaved loop
does. The hand-rolled harness was mildly optimistic in its own favour, by about
0.02, and it is better to have found that than to have found the reverse.

**The mechanism is not established and is not chased.** Overhead subtraction
would push the other way: the hand-rolled loop counts its `Consume()` guard in
every arm, and adding a constant to both sides of a sub-1.0 ratio raises it.
The plausible remaining candidate is that one interleaved process gives every
arm a shared GC heap and shared warm state, while BDN isolates each benchmark
in its own; that would systematically compress differences between arms. It is
a hypothesis and it is labelled as one. Absolutes are deferred, so a session
spent resolving it would buy something the controlled rerun measures anyway.

Both flips sit within 0.09 of parity. P2.4 decode goes 0.928 to 1.020, which
**strengthens** the convergence finding rather than denting it: under the more
rigorous harness both P2.4 and P6.1 are losses, and "the managed win erodes as
container construction dominates" is the claim either way. P5.4 encode
managed-2pass goes 1.015 to 0.990 on a bulk row already marked AMBIGUOUS and
sitting on the memcpy floor.

**What this is worth.** Neither harness validates itself. Two harnesses that
share the arm table and nothing else, agreeing to a median of 0.019 across 64
rows and disagreeing about a verdict twice, is what makes either usable. It is
also why both are kept: BenchmarkDotNet goes to the controlled rerun, and the
interleaved loop stays for the noisy shared container, where per-benchmark
isolation is the thing R4 exists to avoid.

### 18. The core-ffi arm, M1, and the two things it says

Built against the SAME `libak_core.so` the Rust slice builds. One native core
with N bindings is the proposal, so a C# slice that grew its own core would
not be testing it; `ak-core` already implements ABI v1 over these shapes and
exports 68 symbols as a cdylib, so the work was a binding.

Gated first: byte identity on P1.1, P1.2 and P1.3 against the manifest, plus
decode re-encoded to the same bytes AND compared field by field against the
graph the builder made. Layout agreement checked, 8 structs.

**Crossings are constant in the element count, in both directions.** Encode is
2 forward and 1 reverse; decode is 1 forward and 2 reverse; a thousand elements
costs the same as four. `ResultRaw` is a leaf, so encode hands the whole run
over in one `ak_elem_ResultRaw` and decode gets it back in one `add_results`.
The batching predicate does on .NET what the C++ slice's crossover model says
it should: .NET crosses at 7.5 to 12 ns, far above the 2 to 4 ns crossover, so
batching is not close.

**Against the no-boundary managed control** (`core-ffi` / `managed`, three
processes):

| payload | encode | decode |
|---|---|---|
| P1.1, 4 elements | 1.330 | 1.062 |
| P1.2, 1000 elements | 1.148 | **0.899** |
| P1.3, the absent path | **2.361** | **1.981** |

**Two findings, and they point opposite ways.**

**One: on P1.2 decode, crossing the C ABI is FASTER than the pure managed
codec.** 0.899 of it, and 0.651 to 0.659 of `Google.Protobuf`. The Rust parser
plus three crossings beats a C# parser doing the same work. That is the
opposite of the Java slice's result, where a generated pure-Java codec beat the
C ABI in both directions, and it is the case the whole proposal needs: the
interface is not eating the core's advantage on a managed runtime at .NET's
crossing price.

**Two: the absent path collapses, and the cause is decision 9.** P1.3 is 300
elements that each encode to nothing, and the arm is 2.36 times the managed
control on encode and 1.98 on decode. The host fills 300 by-value groups of 200
bytes each whatever is in them, which is 60 KB of stores to describe 605 bytes
of output. The Rust slice measured the same effect from the other side: its
zeroed-group variant is 0.719 to 0.766 of the total fill on exactly this
payload. **Decision 9's sparse fill is not built here**, and this arm is what
says how much it is worth on .NET: the gap between 2.361 and something near
1.15 is the prize.

The interface term also shrinks with payload density, 1.330 to 1.148 on encode
from 4 elements to 1,000, which is the fixed three crossings amortising.

**What this arm is not.** M1 only. `TaskDetailed` is not a leaf, so M2 is where
the batching predicate starts refusing and where the interface cost stops being
three crossings; that is a different measurement and it is not taken.

### 19. The crossing-count gap was a chunk size, not a convention

The aggregating session ruled that this slice should re-report its crossing
counts in the Rust slice's convention, the Rust one being established and
independently reproduced by C++ to the digit. Following that ruling is what
showed the premise was wrong.

The right way to adopt another slice's convention is not to re-derive it but to
**read the same counter**, so the binding now calls the core's own
`ak_enc_counters`. It reports **2 forward and 1 reverse per M1 encode**, which
is what this slice's host tally already said. So the host tally was never in a
different convention.

The Rust log's own table then explains itself. Forward is 2, 3 and 8 for 4, 300
and 1000 elements; subtract the single `ak_encode_*` and that is 1, 2 and 7
`ak_elem_*` calls, which is `ceil(n/150)` exactly. **The Rust host chunks its
run at 150 elements.** This slice's host hands the whole run over in one call,
which is why it reported 2 where they reported 8.

Setting `AK_CHUNK=150` reproduces their counts to the digit: 2 / 8 / 3 forward
and 1 / 1 / 1 reverse. The two columns of the cross-language table were always
comparable; what they needed was a chunk size beside them, not a convention.

**Then the interesting half.** The six extra crossings cost **nothing
measurable**: 156.83 ns/element at 2 crossings against 157.95 at 8, 0.7 percent
apart and inside the spread, because six crossings over a 218 KB payload is
about 60 ns in total.

That is worth stating as a limit on the branch's own slogan. "Make the
crossings fewer, not cheaper" is a rule about crossings that scale with the
FIELD count -- the drafted ABI made 15,137 to decode a thousand rows -- and it
says nothing useful about 2 against 8. Once the batching predicate admits a
message, **chunk size on .NET is free and should be chosen for memory**: the
whole-run form needs a group array proportional to the element count, 200 KB on
P1.2, where the chunked form needs 30 KB whatever the payload arrives as.

**The transcoder question is also settled by the same counters.** `transc` is a
separate column from `reverse` in both slices, because a transcoder is an
indirect call the core makes into ITSELF. The Rust slice records 6,000 of them
on P1.2 and one crossing; this slice reads reverse = 1 and the same structure.
So staging strings does not avoid crossings the callback form would have paid
-- it avoids TRANSCODER invocations, which were never crossings. The prediction
in entry 18, that a host transcoder would cost 5,000 reverse crossings, is
**wrong on the counting**, and what it would actually cost is 5,000 indirect
calls into managed code, which is a different and probably larger thing. It
still needs measuring; the arithmetic behind it does not.

### 20. M2's ABI surface, and a vtable slot invented by analogy

Declaring `ak_dvt_ListTasksDetailedResponse` for C# I wrote an
`unk_tasks_options_options` slot between `add_tasks_retry_of_ids` and
`add_tasks_options_options`, by analogy with `ak_dvt_TaskDetailed`, which does
have an `unk_options_options`. The root does not. Every slot after it would have
been read one pointer along, so the codec would have called
`add_tasks_options_options` through whatever the next field held.

Nothing would have caught it: the struct has no size assert of its own, the
layout probe covers `ak_efix_*` and `ak_dfix_*` and not vtables, and a wrong
function pointer is a segfault at the first map entry rather than a diff.

So `AbiLayout.Check()` now asserts **vtable slot counts** as well as struct
sizes and offsets. Every slot is pointer sized, so a wrong count is a wrong
size and nothing subtler is needed.

The same lesson a second time, in the same file: the presence BIT VALUES. The
obvious rule is a field's position among the singular message children, and it
is right today for all fourteen of `TaskDetailed`'s. It is still a guess. The
probe now prints the Rust constants into `gen/abi-layout.json` and the emitter
reads them, so a bit that moves moves in one place.

### 21. M2 answers the decode question, and the answer is no

The claim under test was entry 18's: on M1/P1.2 the composed arm decodes at
**0.899** of the pure managed codec, which is the opposite of the java slice and
the most surprising number in this slice. Entry 18 said it was a claim about one
flat leaf message. It was.

On M2 the interface cost on decode is **0.97 to 1.14** across the five payloads,
with every spread touching or crossing 1.0. It does not reverse; it ties.

The part worth recording is how nearly I published the opposite. The first
BenchmarkDotNet run on P2.2 read **0.900** -- M1's figure, to three digits --
against the interleaved harness's 0.987. Two harnesses disagreeing is the signal
to re-run, not to pick. Two more BDN runs read 0.963 and 0.987, so BDN's own
spread is 0.900 to 0.987 and it overlaps the other harness's 0.923 to 1.052.
0.900 was the bottom of a spread and I would have reported it as a reproduction.

This also inverts entry 17's characterisation. There, BenchmarkDotNet was the
CONSERVATIVE harness: it moved ratios against the challenger. Here it moves them
for it. So "BDN is conservative" was a property of that table, not of the tool,
and neither harness is the tie-breaker. Where they disagree, both ranges go in.

### 22. The encode cost is the group fill, and it is not the absent path

M2 encode is 1.69 to 1.94 against the managed control, up from 1.25 on M1/P1.2,
and the obvious explanation is the crossings: M1 is three for a whole response,
M2 is ten per task. The obvious explanation is 11 to 17 percent of it.

Ten crossings at this slice's own measured .NET price of 7.5 to 12 ns is 75 to
120 ns; the gap on P2.2 is 699 ns a task. So I built the arm that says where the
rest is rather than arguing it: `core-ffi fill` does everything the encode arm
does -- zero the 616-byte group, stage every string, build the run arrays -- and
returns without calling the codec.

**The fill is 40 to 57 percent of the whole core-ffi encode, on every payload of
both shapes, in both harnesses.** Subtract it and the core's own work plus every
crossing is 0.72 to 0.94 of the managed control on six of the eight payloads.
The Rust codec across a C ABI encodes faster than the C# codec does; the arm
loses on what the host must do to feed it.

**And it corrects entry 18 and stage 8.** P1.3's 2.361 was attributed to the
group fill by elimination, and read as a property of the ABSENT payload: 300
groups of 200 bytes filled to describe 605 bytes of output. The fill arm says
62.6 percent on P1.3 -- and 40 percent on P1.1 and P1.2, where nothing is absent
at all. The cost is the group being filled whole. P1.3 does not cause it; it
removes everything else that was hiding it.

That is ABI v1 decision 9, and it is now the specified amendment with the
largest measured value in this slice.

### 23. Two gaps found in the harness while measuring, not by looking

**BenchmarkDotNet did not have the R14 baseline arms.** `gp-marshaller` on
encode and `gp-parse-seq` on decode were in the hand-rolled harness only; BDN's
baselines were `gp-writeto` and `gp-parse`, the span forms. Every ratio in
STATE.md is quoted against R14's path, and the harness the controlled rerun is
supposed to use would have come back unable to produce that column. Found by
adding a core-ffi class to BDN and having to choose its baseline.

**The map fill allocated 24,000 bytes an operation on P2.2.** `OrderedMap`
exposes `At(int)` precisely so an encoder can walk it without an enumerator, and
the generated M2 fill used `foreach`, which boxes the `List` enumerator once per
ELEMENT. 500 elements, 48 bytes each. It was visible only because the encode
arm's allocation column should be zero and was not -- the arm was still correct
and still beat the incumbent. Indexed instead: zero, and 4 percent faster.

### 24. The group-skip hole, and what a reject vector does not prove

The aggregating session fixed D7 in the shared core -- `ak_rt`'s unknown-field
skip had no case for the deprecated GROUP form -- and pointed out that
`Facade/Wire.cs` has the identical hole. It did. `Dec.Skip(int wire)` had cases
for wire types 0, 1, 2 and 5 and `ErrMalformed` for everything else, so it
rejected three corpus vectors that `Google.Protobuf` accepts.

**Nothing this slice owns could have found it.** Byte identity is against
`ffi/schema/generated/manifest.json`, whose every payload is emitted from the
description the decoder is emitted from; proto3 cannot express a group; and the
slice's own hand-built unknown-field vectors cover exactly the four wire types a
proto3 writer can produce, because that is what I could think to build. The
corpus found it because it is the only oracle in the branch not generated from
the thing it tests.

Three things the fix needed beyond `case 3:`, and each is a different failure:

  * the END_GROUP's FIELD NUMBER has to MATCH the tag that opened the group.
    Counting depth instead accepts `X-group-mismatched-end` and then mis-nests
    every group after it. A wrong parse, not a rejected one;
  * the buffer has to be checked each iteration, or `X-group-unterminated`
    walks off the end;
  * the recursion has to be bounded, or 200 nested start tags is a stack
    overflow instead of an error. Bounded at 100, protobuf's own default.

**The part worth keeping is what the reject vectors did.** Both X- vectors
PASSED before the fix. An unhandled wire type is an error too, so a rejection
for the wrong reason looks exactly like a rejection for the right one. Only the
three accept vectors failed. A slice that had run only the must-fail half would
have reported a pass on a decoder that rejects a third of the group class --
which is a general point about reject vectors and not about this one.

I reverted the fix once and re-ran the gate before committing it: 7 failures
without, 0 with. This slice's own standard, applied to its own fix.

And the scope, said plainly because it would be easy to imply otherwise: **five
vectors ran, 331 did not.** `ffi/corpus/CONTRACT.md` rule 0 is "generate your
codec from `generated/corpus.proto`", and this generator has no .proto front
end at all. Corpus conformance is a work unit and it is now on the list.

### 25. Three wrong fixes, and the gate that caught the third was the one I nearly retired

Entry 24 fixed the group-skip hole and showed it failing by removing the case.
That demonstration was weaker than it looked, and the aggregating session's
follow-up said so from experience: its own first attempt at the core silently
dropped the 32-bit arm while adding the group arm, and a reading would not have
caught it. So all three plausible wrong implementations were built on the real
`Wire.cs` -- not a re-implementation, which would be the oracle being the code
under test -- and run through the real gates.

| wrong implementation | `conformance` | `groups` | `unknown` |
|---|---|---|---|
| no group case at all | 0 fail | **3 fail** | 0 fail |
| END_GROUP field number not matched | 0 fail | **2 fail** | 0 fail |
| group case added OVER the 32-bit arm | 0 fail | 0 fail | **1 fail** |

**No gate catches more than one of them, and the third is the interesting row.**
Byte identity against the schema manifest passes it -- of course, it cannot
reach an unknown field at all. The corpus group vectors pass it -- of course,
they are about wire type 3. The only thing that fails is `harness unknown`, the
hand-built vectors I wrote before there was a corpus, whose `elem-i32` row
covers exactly the arm that went missing.

That is the part worth keeping. Having acquired a corpus, the obvious tidy-up is
to retire the hand-built suite as superseded. It is not superseded: it covers
the four wire types a proto3 writer can produce and the corpus covers the fifth,
and the sets do not overlap.

**And the depth bound turned out to need a bigger demonstration than I gave it.**
With the bound removed, 200 nests still reject -- as `ErrTruncated` rather than
`ErrDepth`, because the buffer runs out before the stack does. So the 200-nest
case checks the error code and not the crash. 20,000 behaves the same. At
200,000 the process prints `Stack overflow.` and aborts with SIGABRT, which .NET
cannot catch and no `try` survives. The gate now carries both depths.

I had also decided to leave `MapForms.Skip` alone with a comment, on the grounds
that it is a harness rewriter that only ever walks bytes this slice emitted,
where proto3 cannot produce a group. That is true. It is also precisely what was
believed about the facade's skipper. Fixed.

### 26. Becoming a corpus consumer, and the four defects it found in an hour

Rule 0 of `ffi/corpus/CONTRACT.md` is "generate your codec from
`generated/corpus.proto`", and this generator had no `.proto` front end at all;
it reads a JSON description through `ffi/schema/emit/shapes.py`. So conformance
was a second front end, `gen/protoparse.py`, producing the same schema dict.
After that every backend already written emitted the reader view untouched --
the facade types, the comparer and the codec each took a namespace argument and
nothing else changed. That is the payoff of the backends having been written
against an IR rather than against a file.

The parser is cross-checked against the other front end at generation time:
nineteen messages and three enums overlap, every attribute the backends read is
compared, and a disagreement fails the generator. It found none. That check is
the only reason to believe a hand-rolled proto parser.

**Four defects, all in code that every gate this slice owns was passing.**

`X-tag-zero`: field number 0 went to the unknown-field skip, which skipped it.
Zero is what a reader gets from a buffer it forgot to bounds-check.

`X-depth-101` and `X-depth-300`: no recursion limit. 300 levels of nesting was
300 managed frames and a successful parse. This is ABI v1 open decision 7, which
the design document says no slice exercises. It does now.

`S-double-minus-zero`: the omit-when-zero rule was `!= 0.0`, and IEEE says
`-0.0 == 0.0`, so a set field disappeared. The vector's own note says it must
compare bits. **`Google.Protobuf`'s generated C# writes `if (Field != 0D)` and
has the same hole, and upb does not** -- so this is a real divergence between
two implementations, and `ffi/schema` could never have surfaced it because it
has exactly one `double` and it is packed.

Plus the group skip from entry 24.

**Why none of it was reachable.** Byte identity is against a manifest generated
from the same description the decoder is generated from. No payload in
`ffi/schema` can carry an unknown field, proto3 cannot express a group, the
value rules never emit a minus zero, and nothing is nested past depth 4. The
corpus is the only oracle in the branch not generated from the thing it tests,
and that sentence is the whole argument for it.

### 27. The measurement the corpus rescued, which I did not see coming

The 31 `T-dec-*` vectors say a conformant parser must reject malformed UTF-8 in
a string field. This codec accepts all 31, deliberately: `Encoding.UTF8`
substitutes U+FFFD, STATE.md has called that the lossy policy since stage 1, and
I was ready to write it up as a labelled divergence and move on.

Then the implication landed. **If the INCUMBENT rejects and the managed control
does not, then the managed decode column -- the single most valuable number in
this slice -- is a validating parser timed against a non-validating one, and
some part of that 0.72 to 0.82 is validation the control simply does not do.**
R14 makes that a defect in the comparison, not a property of the design.

So I measured it instead of assuming either way. `harness utf8` runs the 15
root-site vectors through `Google.Protobuf` and through the managed codec.
**The incumbent accepts every one**, with the same character counts. The arms
are like for like and the decode figures stand.

Two things worth keeping from that. The first is that a correctness artifact
found a hazard in a *performance* claim, which is not what I expected it to be
for. The second is that "we both do the lossy thing" is a cross-language finding
the corpus does not currently carry: ABI v1 decision 3's rejecting policy is a
behaviour CHANGE for C#, and `new UTF8Encoding(false, throwOnInvalidBytes: true)`
rejects all 15, so the validating arm is one constructor argument away and
pricing it is now a named next step rather than a note.

### 28. Where the corpus is wrong, or at least outnumbered

`U-map-entry` is the one vector this slice fails on C2 for a reason that is not
a policy. Its `why` is exactly right -- a map entry is a message on the wire, so
it has an unknown-field skip of its own, and a decoder that hand-rolls entries
usually does not. This decoder does skip it and keeps the entry.

The committed projection puts all four whole entries under `options._unknown`
and omits the map entirely. Before reporting that as a defect in the vector I
pointed the incumbent at the same bytes, which is possible here because
`ListTasksDetailedResponse` exists in `shapes.proto` too. `Google.Protobuf`
keeps the map: `"options": { "options": { "k00": "alpha800", ... } }`.

So two implementations against the projection, one of them the library R14
names. That is a request to the aggregating session and not a fix here, and the
runner now prints the incumbent's own reading beside any projection mismatch on
a shared root, so the next one does not need this done by hand.

The same mechanism turned out worth having generally: the runner points
`Google.Protobuf` at every vector whose root `ffi/schema` also has, by
descriptor name rather than by a switch over nineteen names. **Accept and
reject agree on all 169 of them.**

### 29. The transport pin, and the half of it that does not apply to .NET

Queued behind M3, but the two facts the aggregating session asked me to
establish are cheap and stale guidance is expensive, so they are settled now
from `dotnet/runtime` rather than from memory.

**"The connection window is a separate setting from the stream window" is true
of tonic and grpc-java and false of .NET.** `Http2Connection` hardcodes
`ConnectionWindowSize = 64 * 1024 * 1024` and raises the connection window to it
with a WINDOW_UPDATE at setup, from RFC 7540's 65,535. Not configurable, and not
a function of `InitialHttp2StreamWindowSize`. At a 4 MiB stream window the
connection window is already sixteen times it, so there is nothing to get wrong
here on this stack.

**"Setting an explicit window may disable dynamic sizing" is the opposite of
what happens, and the real hazard is sharper.** `Http2StreamWindowManager` takes
the configured size as its STARTING point and then doubles from there under BDP
pressure, up to a 16 MB cap, unless a separate switch says not to. So a pin is a
floor, not a cap: set 4 MiB and the arm may be measuring 8 or 16 by the end of
the run. Pinning on .NET is two settings -- the property and
`System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing` --
and an arm that sets only the first is not measuring what it says it is.

Worth recording as a pattern rather than as two facts: both halves of the
guidance were about the SHAPE of a stack's flow control, and both were right
about some stack and wrong about this one. A cross-language table of transport
configuration cannot be written once and applied five times.

### 30. One derivation for seven shapes, and the two defects that fell out of it

M1 and M2 had a hand-shaped core-ffi emitter each, and the second cost a vtable
slot invented by analogy with a sibling (entry 20). Writing five more the same
way would have been five more chances at the same mistake, so the ABI
declaration, the layout probe and the host binding now all come from one module
that follows `ffi/poc/codec/gen/rust_abi.py`'s own rules. 20 hand-listed structs
and 5 vtables became 42 and 28, every one verified against the Rust build's
offsets.

Two rules were not guessable from the sibling cases and both were caught by the
slot-count assert rather than by reading. A message that only ever appears as an
INLINED child gets no vtable at all. And `unk_<slot>` exists only where a run's
elements are messages, because a run of strings or packed scalars has nowhere to
carry an unknown field -- which is why `ak_dvt_MetricsBatch` has seven slots.

**M3's explicit-presence string was silently wrong**, and neither hand-written
emitter could have found it because neither covered a message with explicit
presence. The kind dispatch ran before the presence check, so an `optional
string` took the implicit path: no presence bit, and a present-but-empty string
reported identically to absent. 10,746 bytes against 12,097. That is exactly the
case design/SHAPES.md says M3 exists to test.

**And stage 8's M1 decode crossing count was wrong.** It said "1 forward, 2
reverse, constant in the element count". The codec flushes a decode run when its
element arena fills -- a BYTE budget divided by the group size, which the host
cannot know -- so `add_results` runs `ceil(n/arena)+1` times: 5 on P1.2, 3 on
P1.3. The count was PREDICTED from the graph, and the R5 cross-check caught it
the moment a general gate ran a counting core over every payload instead of
over the three whose arithmetic happened to be right. It is counted now, at the
callback, so it is correct by construction rather than by argument.

### 31. Pull, and the first managed measurement of a family with no upcalls

design/ABI-v1.md decision 2 named this exactly: "a pull arm on a managed host is
therefore the measurement that settles this decision in practice, and nobody has
built one."

`ak_parse_*` appends a record per deposit to a buffer in the decode context and
makes no reverse call at all; the host replays the buffer afterwards. Because a
drained buffer is a log of the calls push would have made, in order, the replay
is emitted from the same slot table the push vtable is -- so the two families
share their per-slot code and differ only in how it is reached.

**Zero reverse calls on all sixteen payloads**, against push's 2 to 3,501. And
faster everywhere: 0.69 to 0.97 of push, median about 0.91.

The part that matters for this slice's headline is the sign. Against the
no-boundary managed control on the real schema's shapes, push reads 0.978 /
1.142 / 1.008 / 1.056 / 1.007 and pull reads 0.870 / 1.040 / 0.978 / 0.968 /
0.955. Push straddles 1.0 from above; pull straddles it from below. **The
composed arm beats the pure C# codec on a real message when it uses the pull
family and not otherwise**, and the margin is small either way.

### 32. The transcoder prediction was wrong about the mechanism, not the size

Stage 8 said the alternative string form would cost "one reverse crossing per
string: 5 per ResultRaw, 5,000 for P1.2, 37 to 60 us", and entry 19 already
corrected the COUNTING half of that. The rest of it is also wrong, and in a more
basic way: **there is no host transcoder.** `ak_tc_utf16` is a pointer into the
core, exactly like `ak_tc_bytes`. Neither form crosses.

So what the arm actually prices is which SIDE converts: the host staging UTF-8
and the core copying it, or the host copying UTF-16 and the core converting.
Measured on every payload, 0.96 to 1.04. It does not matter.

A null result, and worth the day: a named gap in this slice's coverage closes,
and a prediction that had been quoted three times is retired. The zero-copy
variant -- a pointer into the managed heap -- is the one still open, and on .NET
it needs a pinned GCHandle per string, so it is named rather than assumed.

### 33. How far a figure in this slice travels, measured rather than assumed

Comparing this sitting with stage 11's, two arms that NEITHER change touched
moved by up to nine percent:

    P1.2 managed-parse / gp-parse-seq   0.725 -> 0.711
    P2.2 managed-parse / gp-parse-seq   0.800 -> 0.729
    P2.2 managed / gp-marshaller        0.292 -> 0.309

Identical code, same container, separate sittings. So a within-process ratio is
sound and a cross-SITTING comparison of two within-process ratios is not, to
better than about ten percent.

That is a limit on how this slice's own history may be read. core-ffi/managed on
P1.2 decode reads 0.899 in stage 8, 0.863 in stage 11 and 0.978 here, and I was
about to attribute the last move to the general emitter. The control moved as
far in the same window. Nothing that compares a number here with a number from
an earlier stage should be read past its first digit -- and every same-sitting
comparison in stage 14 stands, because each is computed against an arm that ran
in the same rounds.

### 34. The RPC arm, and the number that shrinks when you put a socket in it

design/SHAPES.md is emphatic that a marshaller arm is not an RPC arm, and it is
right for a reason I only saw once the socket was in.

In process, on P2.2, the decode arms read 0.73 (managed), 0.88 (core-ffi push)
and 0.77 (pull) of the incumbent. End to end, through a real grpc-dotnet client
against a real grpc-dotnet server over a Unix domain socket, they read **0.83 to
1.06 of CPU per call**, median about 0.88.

**A codec a quarter cheaper is a tenth cheaper once the transport is in the
measurement**, and anyone sizing the change from the in-process column alone
overestimates it by about two and a half times. That is the single most useful
thing this arm produced and it is a deflation of this slice's own headline.

Two more that only the end-to-end view gives.

The three codec arms are **indistinguishable from each other** at the RPC level,
and their ordering flips between rows: pull is best at 16 in flight over UDS and
worst at 16 over TCP. Stage 14's differences between them are real and measured;
they are simply below the transport's noise floor. So the push-versus-pull
decision is not an RPC-level one.

And the result that survives the noise is not CPU at all: every facade arm
allocates **2.136 MB per call against the incumbent's 2.336**, about 8.6 percent
less, on every configuration and every concurrency level, with a far tighter
spread than the CPU column. For a control plane moving this shape continuously
that is a GC-pressure argument, and it is the one I would put in front of
someone deciding.

The arm also found a cost of the facade's own shape that the in-process
measurement cannot see. gRPC hands the deserializer a `ReadOnlySequence`; `Dec`
is over `byte[]`; Kestrel delivers 540 KB in several segments; so every call
flattens. The buffer is reused rather than allocated, because allocating one
would charge these arms 540 KB a call no real implementation would pay, but the
copy is theirs and is in the numbers. **A `Dec` over `ReadOnlySequence` is a
real improvement, identified here and not built.**

Isolation, because it is what makes the arms comparable: the SERVER's marshaller
is a `byte[]` passthrough in every arm, so the server does no codec work and the
only codec in the process is the client's. The server registers its methods
through `IServiceMethodProvider<T>`, grpc-dotnet's own seam, because four arms
need four marshallers on one method and a generated service base fixes the
marshaller at build time.

### 35. Pinning a window, and two settings where the guidance says one

The RPC arm pins ArmoniK's transport rather than .NET's default, which is R14
applied to the transport. Doing that correctly on .NET needs two settings and
the guidance I was given named one.

`InitialHttp2StreamWindowSize` says where the stream window STARTS. It does not
cap it: `Http2StreamWindowManager` takes it as the starting value and doubles
from there under bandwidth-delay pressure, to a 16 MB default. What HOLDS a
pinned window is the AppContext switch
`System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing`, set
before the first handler exists. An arm that sets only the property is not
measuring the window it claims to.

And the connection window, which the guidance flagged as the trap, is not one
here: `Http2Connection` hardcodes 64 MiB and raises it at setup, not
configurable and not a function of the stream window. That trap is real for
tonic and for grpc-java and unreachable on .NET.

Measured, pinning is worth about 3 percent at this payload -- 5,086 against
5,240 CPU us/call -- because 540 KB fits in a 4 MiB window with room to spare
and does not stall badly even at the default. R9's hazard is real at larger
payloads; P2.2 is not where it bites. The pinned row is still the one to quote,
because R14 says the configuration under test is ArmoniK's and not the stack's.

Worth recording: `packages/csharp` can set NEITHER. Its `GrpcChannelOptions`
carries Credentials, DisposeHttpClient, ServiceConfig and LoggerFactory and
nothing else, and it builds an `HttpClientHandler`, through which the property
is not reachable at all. So the pinned arm configures something the shipped
client cannot -- a finding about the client, not about the codec.

### 36. The slice, closed out

The brief was: own `poc/csharp` and `logs/csharp`, build the incumbent arm, the
facade, the managed control codec in both directions, and the correctness gate;
R13 first; `core-ffi` held pending decision 1. Everything in it exists and is
gated, and so does everything the check-ins added afterwards.

What the slice ended up being, in one list:

  * the managed control, encode and decode, on 16 payloads and 7 shapes, gated
    on three runtimes (net8.0, netstandard2.0 sources, net48 on Mono);
  * `core-ffi` on every shape, encode and BOTH decode families, with the ABI
    declaration, the layout probe and the host binding derived from one module
    -- 42 structs and 28 vtables verified against the Rust build;
  * a `ffi/corpus` consumer, 336 vectors on three arms, via a second generator
    front end over `corpus.proto`;
  * an end-to-end RPC arm, grpc-dotnet both ends over a UDS with ArmoniK's
    transport pinned;
  * two harnesses that agree, and a BenchmarkDotNet build for the controlled
    rerun.

**The five results I would defend**, in the order I would put them to someone
deciding:

1. **A generated pure-C# codec decodes at 0.72 to 0.82 of `Google.Protobuf` on
   every shape the real schema has.** C# does not look like Java on decode, and
   that was the single measurement the Java report named as able to change its
   own recommendation.
2. **Pull beats push on every shape** (0.69 to 0.97) and removes the upcalls
   entirely rather than reducing them. It moves the composed arm from just above
   to just below the managed control. That is decision 2's open half, answered
   from a managed host.
3. **The encode arm loses on the group fill and nothing else.** 28 to 71 percent
   of the `core-ffi` encode is the host filling a by-value group; subtract it and
   the Rust codec plus every crossing is below the C# codec on most payloads.
   Decision 9's sparse fill is the largest available improvement.
4. **End to end, the codec is worth about 10 percent of CPU per call, not 25.**
   The transport is the rest. What survives the noise is allocation: 8.6 percent
   below the incumbent, on every configuration.
5. **Correctness found four defects that no timing arm could have.** Three of
   them -- the group skip, tag zero, the missing recursion limit -- were in code
   every other gate was passing, and the corpus is the only oracle in the branch
   not generated from the thing it tests.

**And the three corrections I would want a reader to see**, because each was a
published claim of mine:

  * ".NET's composed arm beats its own managed codec on decode" was a claim
    about one flat leaf message. On a real one it is a tie, and only pull puts
    it back on the right side of 1.0.
  * "M1 decode is 2 reverse calls, constant in the element count" is wrong. A
    decode run flushes when the codec's arena fills, so it is `ceil(n/arena)+1`:
    5 on P1.2. The count was predicted rather than counted, and is counted now.
  * "A host transcoder costs a reverse crossing per string" was wrong about the
    mechanism, not just the arithmetic. There is no host transcoder;
    `ak_tc_utf16` is a pointer into the core and neither string form crosses.

**The methodological one that limits all of it**: two arms that no change
touched moved up to nine percent between sittings on this container. A
within-process ratio is sound; comparing one stage's with another's is not, past
the first digit.

### 37. Decision 3 is free, and the reason is better than the number

The rejecting UTF-8 decode policy closes all 31 of the corpus's open `T-dec-*`
vectors and costs nothing measurable: -4.2% to +3.6% across four string-heavy
payloads, inside a control that itself moved 4.6% between the two builds.

Built as a third build rather than a runtime flag, for the same reason the floor
is one: a flag puts a branch on both arms' hot path and stops the JIT
devirtualising `Encoding.UTF8`, which would charge the lossy arm for the strict
one's existence. Each build carries the incumbent as its in-process control, so
the two are compared through that control and not across sittings.

**Why it is free is the part worth carrying.** `Encoding.UTF8` already
validates -- it has to, in order to know where to put U+FFFD. The scanning is
identical and only the `DecoderFallback` differs. So on .NET the argument
against decision 3's rejecting policy cannot be performance; it is a behaviour
change, and `Google.Protobuf` substituting is what every C# consumer sees today.
That is a judgement this slice can price and cannot make.

### 38. The biggest number in the slice is one I nearly did not measure

ABI v1 decision 13 is a borrowed string view, and STATE has carried it as "a
strong candidate, the largest lever on a codec that is 174/413 strings" since
stage 1 without a number. Redesigning the facade's public surface around one is
a large change with a lifetime rule attached, and I was going to leave it named.

Bounding it first cost an afternoon. A decode arm that does everything and
materialises no string at all is the ceiling -- R2's floor-arm logic applied to
a design question rather than to a measurement.

**42 to 62 percent of a decode is string materialisation.** P4.1 is 0.383,
P1.2 0.438, P2.2 0.466, P3.1 0.582.

For scale: every codec difference this slice has measured -- managed against
incumbent, push against pull, core-ffi against managed, staged against UTF-16 --
lives inside a band of about thirty percent. The strings are half the decode.
It is a ceiling and not a forecast, and it still says decision 13 is a larger
lever than decision 3, decision 2, the push/pull question and the codec choice
combined.

The lesson is the cheap one: a ceiling is not an implementation and costs a
fraction of one, and I had been treating "this needs a facade redesign" as a
reason not to know the size of the prize.

### 39. Retiring an item by measuring it, which is the same win as building it

Stage 15 named a `Dec` over `ReadOnlySequence` as a real improvement the RPC arm
had identified: gRPC hands the deserializer a segmented body and the facade's
reader is over `byte[]`, so every call flattens.

Instrumented, the first half is worse than I thought and the second half makes
it moot. gRPC delivered a segmented body on **every single call** -- 3,960 of
3,960, the single-segment fast path never taken at this payload size. And the
flatten is 540 KB, which this slice's own `memcpy floor` arm already measures at
12.8 us, against about 4,500 us of CPU per call. **Under 0.3 percent.**

A segmented reader means every read handling a boundary, which risks the
single-segment path every in-process arm in this slice uses, to recover a third
of a percent of an RPC. So the item is retired rather than built, and the thing
that retired it was two counters and an arm that already existed.

### 40. The list, and what is actually left

Every item is now either done, bounded, retired with evidence, or named as not
this slice's:

  * **done**: core-ffi on every shape, both decode families, the corpus, the
    RPC arm, the two string forms, the crossing reconciliation, decision 3;
  * **bounded**: decision 13, at 42-62 percent of a decode, and decision 9, at
    28-71 percent of an encode;
  * **retired with evidence**: the `ReadOnlySequence` reader;
  * **not this slice's**: decision 9's implementation is an ABI addition and
    `ffi/CLAUDE.md` routes a change to existing behaviour through the
    aggregating session;
  * **left, and it is one thing**: streaming, which design/SHAPES.md says is
    where the concurrency invariant actually bites and which no slice in the
    branch has touched.

I am not going to invent an eleventh item. An idle session is cheaper than a
fabricated one, and the branch is close to the report.

### 41. A defect in entry 40's own commit

`baa114aa` staged eight files under `src/Facade/obj-strict/` and
`src/Harness/obj-strict/`: NuGet restore intermediates for the third build.
The ignore file lists `bin/`, `obj/`, `bin-floor/` and `obj-floor/`, and I
added `bin-strict/`/`obj-strict/` as output paths in `Directory.Build.props`
without adding the matching rules. Untracked and the two rules appended. The
build output was never a measurement input, so nothing in stage 16 changes.

### 42. Streaming, and the control that stopped me publishing a ranking

Item 11 is built (`stage17-streaming.log`). Four things came out of it and only
two were the ones I expected.

**The floor is what the arm is for.** A fifth arm streams the same messages with
no codec at all, through the same contextual passthrough the other arms use, so
its ratio is the share of CPU no codec choice can reach. On P2.2 that puts the
codec at **54 to 82 percent of a streamed download and 36 to 50 percent of an
upload**, where stage 15's unary headline was 10 percent of a call. A stream
pays for headers, trailers and a stream once; what is left per message is the
bytes and the codec.

**The first floor I built was not a floor.** It used `Bench.Raw`, the SIMPLE
`Marshallers.Create` form, and gRPC then copies the returned array into its send
buffer -- a whole payload copy the contextual arms do not pay. It came out ABOVE
the incumbent, which is impossible for a floor, and that is how I found it.

**The reversed-order run is the part I nearly skipped.** Arms run in a fixed
order with the first as the ratio base. On P5.3 the floor moves from 0.690 to
1.000 on download and from 1.171 to 0.831 on upload on nothing but its position
in the order: a position effect of 17 to 31 percent, larger than every codec
difference on that shape. Without it I would have written that `core-ffi pull`
is 0.761 on ArmoniK's chunk download. It is not; nothing is. The honest
statement is that on the chunk shape no arm differs from no codec at all, and
`stage14`'s in-process column says the same thing from the other side: core-ffi
is 2.02x the incumbent on P5.3 encode and the excess is exactly one memcpy
floor -- the staging copy -- which is 67 us against a 5,000 us streamed message.

**The prediction the arm was built on is refuted.** I expected streaming to
raise the codec's SHARE relative to unary, by removing the per-call transport
cost. Same sitting, same process: it does not move outside the spread. At
540,422 bytes a message the fixed per-call cost is already small next to moving
the bytes. It should hold for a small message and I did not test one; it is in
"what is not measured" with P1.1 named as the payload that would answer it.

**The concurrency invariant is answered for a managed host, with both controls.**
One context per thread: 0 wrong of 200,000. One shared context: SIGABRT, and the
new part is that the `catch (Exception)` around the call never runs. On .NET
that is worse than in rust, because a .NET developer who shares an object
expects an exception at the seam. The lock-free answer costs 7 to 8 contexts and
7.4 to 8.4 MB of never-freed native staging on a four-processor box, and the two
runs disagreeing by one context while doing identical work is the finding: the
number follows the thread pool, not the call rate. Recorded as request 7.

### 43. The grid, and the column that hid a 600x

The core exported ABI v1 section 9's transport behind its `rpc` feature, so the
RPC arm became a 2x2 grid: incumbent and core codec, grpc-dotnet and core
transport, all four in one process against one server.

**The headline is not the one the slice has been chasing.** Cell B over cell A --
the same `Google.Protobuf` codec, only the transport swapped -- is 12 to 44
percent less CPU per call, and at 16 in flight it is 42 to 44 percent agreeing to
a hundredth across three runs and two cell orders. The codec alone (D/A) is 0.82
to 1.04. So on .NET the proposal's value is in the half this slice has spent the
least time on.

**And the question the grid was built to answer, I cannot answer.** C-B and D-A
are both "what the codec is worth", one under each transport, and they change
SIGN with the arm order. Both are inside the round-to-round spreads. I wrote the
"do they agree?" column into the harness before running it and it prints
"differ by 100%" on rows where the honest reading is that neither number is
distinguishable from zero. The thing that IS sayable is a ratio of magnitudes:
the transport half is three to ten times the codec half.

**Running both cell orders is now reflex and it earned its cost again.** Without
the reversed run I would have reported C-B as positive at 8 and 16 in flight,
which is the core codec being SLOWER, from a forward run alone.

**The three deliveries are the same in CPU, and I nearly stopped there.** The
check-in asked for that to be said plainly if it came out, and it did: callback,
queue and blocking are within their spreads of each other at every concurrency.
Which would have meant .NET is indifferent and the ABI carries the extra modes
for the JVM. Then the blocking row's WALL clock stalled -- 208 ms once, 24 ms
once, never in the other two deliveries. Two observations is an anecdote, so I
built `--park`: one batch of 16 concurrent calls from a cold pool.

Sixteen blocking calls take 2.5 to 8.3 seconds for work the callback does in 11
to 15 ms. **190 to 630 times, and the CPU column shows none of it**, because the
cost is the thread pool growing to replace threads parked in a native frame at
one or two a second. `SetMinThreads` removes it; nothing else does. I had
written into the probe's own output that this would be a one-off a long-lived
process pays once. The third row refutes that: the same batch again with the pool
grown is slow in five of six runs, because the pool retires idle threads between
bursts. Writing the falsifiable version is what let the data refute it.

**A hazard I predicted and that does not exist.** My own floor analysis found
that a delegate thunk must be rooted for the lifetime of the vtable or the
collector reclaims it. It does not apply here: `[UnmanagedCallersOnly]` compiles
to a native entry point and `&OnDone` is its address, so there is no thunk. That
rule belongs with the floor findings, not in the ABI's contract.

The whole slice is re-gated against the rpc-featured core and is identical
(152/0, the same 32 corpus rows, coreffi 0), which is what "the feature adds the
transport and changes no codec entry point" has to mean to be worth saying.

### 44. Three follow-ups, and the one that was a defect of mine

**The TCP row is not a Nagle row, and I checked instead of arguing.** The
temptation was to reason it away in a sentence -- both ends of my arm are
grpc-dotnet, not the core's test server, and both set `TCP_NODELAY` by default.
That reasoning is correct and it is not evidence. So I reproduced the rust
slice's own diagnostic on my stack: 858 bytes costs 133.2 us and 540,422 costs
905.4 over loopback TCP. The small payload is 6.8 times cheaper where the defect
made it 1.4 times dearer. Nothing I have published moves.

**Stage 18's grid had an R7 defect and it was mine.** Cells A and D pinned
ArmoniK's transport on the .NET client; cells B and C took tonic's defaults,
because `ak_client_new` was the only dial the core exported. So part of a
published 12-to-44-percent transport gap could have been the settings, and the
log said so nowhere. `ak_client_new_opts` closes it, and I kept the unpinned
cells as B* and C* so the correction is visible rather than a quiet replacement.
**Pinning moves nothing outside the spreads.** The defect was real methodology
and an immaterial number, and it is worth saying in that order: I did not know
which it would be until it ran.

**The crossings were a quote and are now a reading.** "Count crossings, do not
infer them" is one of the branch's own invariants and stage 18 broke it: I
repeated section 9's two-per-call. Counted from a core built with `count`:
blocking is 2, the callback is 3 forward plus 1 reverse, the queue is 4 forward.
**Two per call is the blocking form and the non-blocking ones cost four**, the
extra being `ak_call_destroy` on the handle that makes a call cancellable. It is
four to six parts per million of a call, so nothing moves -- but the crossing
table is meant to survive a rerun on other hardware, which is exactly the kind
of claim that has to be right rather than cheap. The harness now refuses to
print a count from a non-counting core, because a zero there reads as "free".

And `ak_client_opts` has six fields, not the five I was handed. Binding five
would have been a struct one word short of the core's, read as garbage. I found
it by reading the struct instead of the message, which is the same habit as the
first item on this list.

**On streaming.** It was scope I added. It was on my next-step list because
`design/SHAPES.md` names streaming as where the concurrency invariant bites and
because a check-in asked for the list to be emptied; the grid never needed it.
STATE.md now says so at the point where the result is claimed, and the log stays
in the tree as an offer rather than as part of the RPC arm, because deleting a
gated measurement destroys evidence rather than scope.

### 45. Reading the package instead of the description, and finding my own R14 defect

I was told the shipped clients pin no HTTP/2 window and that Nagle is off by
name. Both are true and I checked them in `packages/rust/armonik-transport`
rather than taking them: `ClientConfig` (config.rs:10) has seventeen fields and
none of them is a window, `grep -rn window src/` is empty, and
`tcp_nagle_algorithm` is "defaults to false", read from
`GrpcClient__TcpNagleAlgorithm` and applied as
`http.set_nodelay(!config.tcp_nagle_algorithm)`.

**Then I read the C# package, which I had not been told about, and it says
something neither of us had.** `GrpcChannelProvider.cs:88` sets
`Http2FlowControl.DisableDynamicWindowSizing` on the UNIX SOCKET path, under a
comment naming it a workaround for a connectivity issue. It still pins no
window. So production on .NET is not my "pinned" row and not my "stack default"
row: it is a 64 KB window with .NET's auto-tuner switched OFF, for a 540 KB
message, with nothing left to grow it. **Both of the transport rows this arm has
carried since stage 15 are wrong for R14**, and that is my defect, not a
relayed one.

Measured, it is the worst of the three: 27 to 39 percent more CPU per call than
either alternative at 1 in flight, reproduced across two sweeps. The no-codec
arm at two payload sizes is what makes it a mechanism rather than a number --
149.2 against 142.9 and 143.0 on 858 bytes, and 1,689.6 against 1,197.5 and
1,376.1 on 540,422. A cost that appears only above the window size is flow
control and cannot be anything else.

**The shape is the interesting part.** .NET's auto-tuner left alone gets most of
the way to the pinned window. The workaround turns it off and puts nothing in
its place, so the shipped configuration is worse than doing nothing at all. It
costs what it costs because it is half a change.

Every ratio I have published survives, because a ratio is taken between arms
under the same transport and the transport divides out. What changes is which
row is the headline, and STATE.md now says so at the top of the requests rather
than in a log nobody re-reads.

I am not proposing the one-line fix. Nothing under `packages/` changes, and I
have not tested whether pinning a window reintroduces the connectivity issue the
switch exists for -- which is the honest reason it is a recommendation and not a
patch.

### 46. WP4 item 10, the net6.0 floor, and two defects every gate was passing

Phase: setup and design. No timing was taken in this unit.

The container had no .NET. `apt-get install dotnet-sdk-8.0` from Ubuntu's own
repository worked (SDK 8.0.131, runtime 8.0.31). No .NET 6 package exists for
noble, and `builds.dotnet.microsoft.com` (the dotnet-install host) is refused
by the proxy with 403; `api.nuget.org` is reachable, so the net6.0 runtime came
in as a runtime pack inside a self-contained publish. The core was built from a
`git archive HEAD ffi/poc/codec` snapshot (core commit 6ede244) because the rust
agent is changing `poc/codec`; the layout probe rebuilt against the snapshot
matched `abi-layout.json` exactly.

**R-D9.** `CallCbAsync`/`CallQAsync` threw on a non-OK status and dropped the
completion's `ak_bytes`; `CallBlocking` too. Fixed with one `TakeOrThrow`. The
first gate I wrote compared forward counts exactly and failed on a SUCCESS row
(2.02 for blocking): the queue drainer's 200 ms idle polls are forward
crossings, and it was running during the blocking row. So the drainer starts
only for the queue rows and the check is on the whole part. The negative
control (frees removed) reads one crossing short on every error row, so the gate
can see the defect. The core returns an empty `ak_bytes` on failure, so nothing
was leaking; the fix removes a dependence on that.

**The binding never called `ak_init`.** Not in the task; found while writing
STATE's lifecycle line, which said "nothing, there being no core". ABI v1
section 3 says every entry point requires it. Every gate passed because the
guard is the `init-guard` cargo feature and no build here turns it on (the
snapshot's default features are empty, which is worth checking against
FIX-PLAN WP4 item 10's "init-guard became default"). Built the core with
`rpc init-guard`: core-ffi 16 failures of 16. The fix went in the generator
(`gen/cs_abi.py`): a static constructor on `Abi`, so no import can be reached
first. 0 failures against the guarded core. The same question applies to every
slice whose gate runs on an unguarded core.

**The reader narrowed a length prefix.** The corpus now has 691 vectors. One
reject row, `X-len-huge` (prefix 2^31 - 1, no body), was "refused" by an
`ArgumentOutOfRangeException` out of `Encoding.UTF8.GetString`: `LenEnd` cast
the 64-bit prefix to `int` and tested `Pos + n > End`, which overflows. The
runner counted any exception as a refusal, so it read as a pass. Two fixes: the
comparison is now 64-bit against the bytes left, and the runner fails a reject
row refused by an exception. The same narrowing also meant 2^32 + k read as k;
no corpus row reached that. `MapForms`' rewriter has the same pattern on bytes
this slice emits only (D8).

**The corpus runner's accounting.** `U-map-entry` became a disputed row with no
projection, the runner printed C2 "n/a" and then subtracted it as a failure:
"-1 other", exit status 255 on a clean run. Disputed rows are now excluded from
pass/fail and reported with the reading this codec produced (pure-python's for
`U-map-entry`; refused for both tag-zero rows, as `Google.Protobuf` does).

**net6.0.** As expected, the binding does not build: 57 `LibraryImport`
declarations in `Abi.cs`, nothing else fails. The managed half builds and passes
the same gate on .NET 6.0.36 as on net8.0. Not restructured: WP5.

**A hand-written configuration line.** `Config.Print` printed "R13 calibration
on THIS machine: 1.8 ns" as a constant on every run on every machine. Removed.
It is the class of line this slice's own C2 defect was about.

**STATE.md rewritten (R-F1).** It said the RPC arm did not exist and described
it; said there was no core; said one thread everywhere next to a concurrency
log; said there was no recursion limit next to the log that added one; quoted
120, 136 and 152 checks for the same gate; carried recommendations and
timing ratios as results; and called Mono "net48". All of that is gone; the old
text is in git at 817174f.

`BenchDotNet` does not build and has not since b59139a: recorded (D1), not
fixed, since it is a timing harness and WP3 decides what the campaign runs.

### 47. WP5 step 4: the backend onto the shared plan, and what the port found

Task: FIX-PLAN WP5 step 4, authorized by the aggregating session. Port the C# backend
onto `poc/codec/gen/plan.py`, retire the second IR, generate the binding with both import
forms in one file, and gate on net8.0 and net6.0 with core-ffi on both.

**What moved.** Five shared modules in `poc/codec/gen/` (`cs_names`, `cs_types`,
`cs_managed`, `cs_binding`, `cs_host`) plus `cs_layout_probe`, each importing the plan
only. `poc/csharp/gen/` kept the glue (values, builders, arm table, projection,
registries) rewritten to the plan's descriptor view; `ir.py`, `abi_ir.py`, `protoparse.py`,
the old emitters and `abi-layout.json` are gone. One trap on the way: with this
directory first on `sys.path`, `plan.py`'s `import ir` resolved to the slice's own
`ir.py`, which has no `enum_order`. Retiring the second IR was not optional even to run.

**The managed codec from the plan.** The encode step list and the (number, wire) decode
table render directly: a `switch` on the recombined key, with anything not in the table
going to the unknown arm, which is R-E2 by construction. Two runtime changes came with
it. A nested body is now read with `Dec.End` narrowed to it; the old reader bounded
fixed-width reads by the whole buffer, so a packed double run whose length is not a
multiple of 8 read into the next field (never hit by the corpus; the plan's sub-reader
rule makes it impossible). And the key's field number is truncated to 32 bits the way the
core does, so the two codecs disagree on nothing; that the truncation lets 2^32 + n alias
n is reported as a plan question rather than fixed in one backend. First byte-identity
run: 152/152 on the payload set. First corpus run: 688/0 on both managed arms.

**The binding from the plan.** Sequential layout everywhere (no Rust offsets copied), each
import as LibraryImport under `#if NET7_0_OR_GREATER` and DllImport under `#else`, `ak_init`
from `plan.lifecycle` in the static constructor (flags now `NO_CRYPTO | NO_PANIC_HOOK` as
the plan says; the old binding passed `NO_CRYPTO` only). Rendering from the plan found two
defects the old hand-listed declaration carried: `ak_encode_UploadResultDataMessage` was
declared WITHOUT the two direct-argument parameters the core exports (it worked because
the host staged the bytes and the core ignored the garbage registers), and
`ak_bdr_count_forward` was declared with the wrong signature (never called). M5 now crosses
as a direct argument; its counts are unchanged (1/0/1/1).

**The layout probe, R-E6.** The probe now parses the struct and member lists out of
ak-abi's Rust source and prints what rustc computes; the harness compares that with the
C# compiler's offsets by NAME both ways, plus section 10's `ak_layout_facts` against the
loaded core. The first run of the table threw: `&z->f` on a null pointer is null-checked by
the JIT, so the offset table takes addresses in a stack instance. The planted offset swap
fails as it must.

**Retain mode (D7).** Every facade class has an `UnknownFields` bag; the plan's default
`unknown = "both"` renders the capture behind `Dec.Retain`, so the host picks per call as
the core's two entry families do. On the seven hand-built vectors the retained re-encode is
byte-identical to Google.Protobuf's. Through core-ffi, `ak_uencode_*` and the capture
callbacks; on the corpus, ffi-retain writes the dropped form on the same 16 rows the rust
slice reports (unknown inside an inlined child, no carrier).

**net6.0.** Nothing special was needed once the imports carried both forms:
`TargetFrameworks net8.0;net6.0`, a self-contained publish on the NuGet 6.0.36 runtime
pack, and core-ffi passes 16/16 there with the DllImport branch (the net6 assembly
references no `LibraryImportAttribute`; the net8 one does). The corpus passes four arms on
net6.0 with the same numbers as net8.0. net48 compiles the same binding file (its host
half compiled out); nothing runs it here.

**The corpus runner** moved to `src/Corpus` (its own project because it loads the
corpus-schema core, a different ABI), one child process per row under a timeout, the
rust slice's four controls. 691 rows in about 90 s (net8.0) and 120 s (net6.0).

**Against the previous generator** (its last build, same corpus): C3 form counts
identical; the 31 T-dec rows move from accepted to refused (R-E7); one C4 code moves,
`X-varint-key-truncated` from malformed to truncated, which is the core's class. The
depth rule is now the plan's; the old codec was one level stricter; no row sits there.

**Plan gaps reported, not worked around in the plan:** vocabulary struct layouts, the
codec's fixed entry points, the values of the lifecycle's named flags, the RPC counting
surface, the 32-bit field-number truncation. The cpp slice's step-2 commit reports the
same list independently. And the shared `generate.py` does not know the C# backends
(this slice may not edit it): its guard is applied by the slice driver instead (D10).

### 48. The WP5 tail: D38, D40, and a re-gate against 41eb485

The aggregating session consolidated the plan (`57b6180`, `41eb485`): the fixed ABI and the
RPC counting surface in `plan.FIXED`, map entries in UTF-8 byte order, `MAX_FIELD_NUMBER`
refused, `GROUP_DEPTH_LIMIT`. It re-rendered my backends for those; two items were left.

**D38.** The oracle-probe row `P-field-maxplus1-in-group` (a group containing field
2^29) was accepted by both managed arms (`logs/rust/wp5s6-probe-csharp-after.log`): the
generated top-level check was right, but the hand-written `Dec.SkipGroup` still truncated
the inner key's field number to 32 bits and tested only zero. The limit now comes from the
plan: `cs_managed` renders `Codec.MaxFieldNumber` and `Codec.GroupDepthLimit` and passes
both to `Dec.Skip`, which compares the full 64-bit field number. The runtime states
neither constant.

**D40.** `cs_binding.emit_rpc` renders `plan.FIXED.rpc_counters_struct` and
`rpc_counting` with both import forms; the hand block in `CoreTransport.cs` is gone and its
two users read `ak_rpc_counters.forward/.reverse`. The RPC layout check now covers 6
structs / 20 members, `ak_rpc_counters` included.

**Re-gate** from clean core builds (all targets deleted first). The corpus runner gained
`--manifest` so the probe rows run through the same four arms; `gen/gate.sh` runs them on
both levels: 11/11 everywhere. The corpus has grown to 702 rows (three field-number rows,
disputed in the corpus, refused by all four arms). Everything else unchanged.

### 49. WP3: the campaign runner, the incumbent move, and a gate that caught the core

Contract: design/CAMPAIGN.md (a10ac81, then 0e8e9eb's amendments to requirements 7 and 29).

**What was built.** `run_campaign.sh --suite codec|rpc|calib|gate --out DIR` and a measuring
mode in akrpc (`akrpc campaign`, `src/Rpc/Campaign.cs`), with the per-root calls and the
field visitors generated as glue (`gen/cs_campaign.py`). The old RPC grid could not be
retrofitted: server in-process, `Process.TotalProcessorTime`, min-of-N. The new rpc suite
runs Kestrel in its own process on `AK_CPU_SERVER` and returns pre-serialised P2.2; cells
A-D plus the core's callback/queue rows; directions a and b; 1/8/16 in flight; shipped and
pinned for every cell; every call checked, and a planted wrong length aborts with no
sample. CPU is CLOCK_THREAD_CPUTIME_ID (codec, calib) or getrusage(RUSAGE_SELF) of the
client (rpc), via libc P/Invoke. `src/BenchDotNet` retired (D1).

**The incumbent** moved to Google.Protobuf 3.32.0, Grpc.Tools 2.72.0, Grpc.Net.Client and
Grpc.AspNetCore 2.71.0 (packages/csharp); the gate passes on it.

**Concurrency with other agents, twice.** The session scratchpad is shared between the
slice agents: another slice's runner overwrote my smoke output file, and could have hit the
core snapshot directory too. SCRATCH is now a private subdirectory. And HEAD moved under the
run (other slices commit continuously), so the runner reuses a passed gate by CONTENT of the
paths a run reads, not by commit hash.

**Decision 11 landed mid-unit (29d515e).** My generated tree went stale and the gate failed
until regenerated; the shared cs_host now decodes core-ffi in a transitional drop mode
(options not rendered, D41), so requirement 10 is pending that port. The counting gate
(requirement 19, new this unit) then failed on one row: P1.2 push-decode reverse 5 -> 8.
The decode group gained decision 11's `ak_unk_buf`, the arena is a byte budget divided by
the group size, so 1,000 ResultRaw now need 8 flushes. Nothing else moved; re-baselined.

**The smoke run** (1 launch, 1 round, reduced iterations) is committed under
logs/csharp/campaign/ with every file marked instrumentation.

Smoke run (after two runner defects the smoke itself found: the dirty check counted the
notes, and the rpc socket path under the long scratchpad exceeded the 108-byte Unix socket
limit, so Kestrel threw at startup): gate PASSED at 5d81225, then codec (1,780 samples),
rpc shipped and pinned (48 each), the abort control (0 samples, both transports), calib (2
samples, after the crossing-count gate). No figure from it is used anywhere.

### 50. Unit 4: BenchmarkDotNet restored as the codec engine (CAMPAIGN 22 amended, 22a)

Contract: design/CAMPAIGN.md at 975001b (req 22: blocks allowed, arm order rotated between
launches; req 22a: BDN is the default .NET engine).

**What was built.** `src/BenchDotNet` (retired at WP3 as D1) rebuilt on the WP5 generated
code rather than recovered file by file: the old project (84a4622^) timed hand-written
arms over the pre-WP5 facade and would not have compiled. The per-root calls come from the
same glue as before (`gen/cs_campaign.py`, target moved from src/Rpc to src/BenchDotNet).
One benchmark class, the case a `[ParamsSource]` string (arm|dir|payload|content|mode), so
the execution order is an orderer's: blocks by arm, the arm list rotated by launch number.
Job: InProcessEmit (one process, so the runner's taskset pins every case), Throughput,
LaunchCount 1, fixed warm-up and iteration counts, iteration time 100 ms (smoke 2 ms),
EvaluateOverhead=false. akrpc lost its codec suite; the gate now builds BenchDotNet.

**Requirements through configuration.** 28: a custom exporter writes every raw
Workload/Actual measurement (wall ns and op count) as a section 7 line; BDN's outlier
handling only reaches its console summary. 21: BDN has no CPU column and no per-iteration
hook short of [IterationSetup], which changes BDN's invocation defaults (not used); a
diagnoser on BeforeActualRun/AfterActualRun reads getrusage around each case, one round-0
row. **Finding while building it:** that span is warm-up + actual, not actual alone (the
first trial's span wall was about twice the one actual iteration; `bdn_stages` shows the
pilot is outside it, the warm-up inside), so cpu_ns/iters is not a per-op CPU figure; the
row says so. And BDN's jitting iterations are NOT in `AllMeasurements` (the first exporter
wrote `bdn_jitting: 0` for every case: a field reporting nothing); the field was removed and
the header states the jitting stage from BDN's log. 24: fixed warm-up per case, every stage
exported per case. 25: GC counts and heap size per case (the span holds 4 forced gen2
collections per case at 1 warm-up + 1 actual: BDN's default forced GC between iterations).
26: the in-process check now also covers incumbent-best encode (it was timed but not
compared in the first version): 560 checks.

**RPC stays on akrpc campaign** (evaluated, not ported): req 18's whole-run abort, one
sample = one batch of concurrent calls with process CPU, the pilot varying calls per
sample, and the per-transport server lifecycle all fight BDN's model. Reasons in STATE.

**Engine cost (container, instrumentation).** BDN's per-case overhead grows with the case
count in one process: 20 cases 1.4 s total, 60 cases 8 to 17 s, 400 cases 176 s, 1,780
cases 36 to 44 min, with no heap trend and GC counts identical per case, so the growth is
outside the measured iterations (not investigated further). A full default launch is hours.

**Smoke.** `run_campaign.sh --suite codec --smoke` at 7f7f6b6: gate re-run and PASSED (the
code had changed since the last passed gate), 560 checks, 1,780 cases, 0 failed, 3,560 JSON
lines, 44 min (36 min on the first attempt at 99bdc0d, whose output was discarded after the
incumbent-best check and the GC fields were added). Heap at span start 61 to 72 MB with no
trend; every case's span holds 4/4/4 collections. No figure from it is used anywhere.

### 51. Unit 4 follow-up: the per-case cost was BDN's forced GCs; the JIT tier is read back

Asked by the aggregating session: find why BDN's per-case cost grows with the case count in one
process, fix it or split the launch; read back the JIT tier (req 24).

**Where the time went.** A trace of BDN's host signals (AK_BDN_TRACE) put almost all of it
between BeforeAnythingElse and BeforeActualRun (jitting, pilot, warm-up), not in the exporter,
the diagnoser, the orderer or setup (0.1 to 0.2 ms per case, P2.2 graphs 110 to 170 ms).
GC.GetTotalPauseDuration beside it: 50 to 80 % of a smoke case was GC pause, 30 to 35 gen2
collections per case. BDN forces 4 full collections per iteration. Their cost follows the live
heap, and the heap follows the case count: after the harness's own checks the process holds
under 1 MB, so the 54 to 75 MB with 1,780 cases (against 10 to 24 MB with 60 to 110 cases) is
BDN's per-case state, tens of kB each. About 15 ms per collection against about 5 ms.
**Refuted on the way:** the parsed corpus manifest I had cached (entry 50) was one suspect;
dropping it (now only root and file per row are kept) changed nothing at 1,780 cases, because
the heap before BDN starts was already under 1 MB. Kept anyway.

**Fix: one process per arm:mode unit** (7 per launch, 40 to 336 cases each), the unit order
rotated by launch (arms, and the modes within an arm). Req 22 as amended allows arm blocks;
each process still does its own pre-timing checks. Smoke: the codec phase went from 36 to 44
min to 12.5 min (BDN itself 5.9 min); about 0.2 s per smoke case instead of 1.2 to 1.5 s.
At the default job a case is about 2.5 s, 11 % of it GC pause, so the job itself, not the
overhead, sets a launch at about 80 to 100 min in this container. Forced GC was not turned
off: it is BDN's default and is kept, stated, with its pause recorded per case.

**Correction to entry 50.** I wrote that the CPU span (BeforeActualRun..AfterActualRun)
covers warm-up + actual. It does not: BDN signals BeforeActualRun after the warm-up. At the
default job the span minus the actual-stage wall is 25 to 135 ms, the forced GCs, while the
warm-up alone is 0.5 to 1.4 s. The smoke's extra time I had read as the warm-up was those GCs.
The round-0 row now says so, carries gc_pause_ns, and cpu_ns/iters is the per-op process CPU
of the actual stage including forced collections.

**JIT tier read back.** An in-process EventListener on the runtime's JIT events
(MethodLoadVerbose, tier = MethodFlags bits 7-9, TraceEvent's OptimizationTier names),
attributed to cases by event timestamp against the host-signal times. Per case: compilations
before and inside the actual stage by tier, and hot_tier0 = methods first compiled in the case
that are still tier 0 at the end of its actual stage and promoted later (hot code measured at
tier 0). First result, the decisive one: without help, the first cases of a process measured
hot code at tier 0 (CodecSuite.Run itself, System.Text.Ascii vector helpers, Google.Protobuf
parse primitives), at the default job too; tier-up waits for a quiet 100 ms which BDN's own
start-up JIT keeps postponing. Fixes: a process-level pre-warm (rounds of 64 calls to every
case through CodecSuite.Run, 0.5 s apart, until a round compiles nothing: 7 to 9 rounds) and
two unexported prime cases (copies of the first two cases) that absorb what BDN's engine
touches first.
**A defect in the read-back, found by the smoke:** 10 of 1,780 cases were flagged for the
runtime's cast cache and a reflection stub. Those methods were first compiled before the
listener started, so their first recorded event was a promotion, read as "compiled in this
case". Attribution now requires the first recorded event to be an initial tier-0 compile.
After that fix: smoke PASS on all 1,780 cases (81 cases show a compilation inside the actual
stage, all runtime or engine methods by name), and one unit at the default job (core-ffi:drop,
336 cases) PASS. The split between measured and engine code is by name and every counted method
is named in the rows.

### 52. WP5 step 9: decision 11 ported to the C# backends (D41 closed)

Asked by the aggregating session: render decision 11's options and root-bound contexts in my
cs_* backends, take the bags into the facade, gate net8.0 and net6.0, and meet CAMPAIGN
requirement 10 in the BDN codec suite. Core at e897f57 (step 8), plan's UNKNOWN FIELDS contract.

**What was built.** cs_binding renders `ak_dec_<Root>_opts` from `plan.unk_opts_layout` and the
two per-root imports from `plan.unk_entry_points` (the untyped `ak_dec_ctx_new` went with
plan.FIXED). cs_host: one context per `CoreFfi_<Root>`, created bound in drop mode; before each
decode the native options (allocated once, never moved) are rewritten with every position
naming one `[UnmanagedCallersOnly]` grow over `NativeMemory.Realloc`, the context armed with
`ak_dec_reset_<Root>(ctx, &opts)`, and disarmed with `reset(ctx, NULL)` after, success or not.
The group readers (`D_<M>`, shared by push callbacks and pull records) take each delivered
message's slot into `UnknownFields` and free the native buffer; an absent child's slots, the
inactive oneof members' and a map entry's are freed (`F_<M>`, `G.Drop`). Every buffer grow hands
out is tracked for the one decode; after a success none may be left (UNDELIVERED, a host
defect), after a failure the rest is freed (rule 3). The layout probe prints Rust's escaped
`self_` under the plan name, and the layout comparison now requires the options structs.

**The one thing I had to decide: the resets and the crossing counts.** Arming and disarming are
two forward calls per decode the core does not count, so the R5 comparison (host tally = core
counters) broke on the first run and every decode row moved by 2. I count them apart
(`ResetCalls`); `gen/crossings.txt` is unchanged, and stays comparable with the other slices'
core-side counts. Stated in STATE.

**Controls.** `corpus --unk-controls` (in process, like the rust harness's step 5): each
position zeroed in turn must equal the retained value with that position's facade bags cleared
(`ClearPosition`, generated from `plan.unk_positions`), pull must equal push, both as retained
re-encodings; and the wrong-root refusal. Result, net8.0 and net6.0: 543 rows, 2,290 pairs, 307
rows with unknowns, 315 pairs changed by zeroing, 0 mismatches, pull == push everywhere, 0
undelivered; wrong root -8/-8/-8, own root fine. Exactly the rust slice's numbers. Plant (the
expectation's clearing skipped): 307 rows mismatch. `AK_CORPUS_RETAIN_STRICT=1` makes a retain
gap a gate failure; the `unkdrop` plant (ffi-retain decoding in drop mode) reproduces the old
307-row regression and fails it.

**Result.** ffi-retain writes the retained form on every non-disputed unknown row (0 gaps;
was 307 in the transitional drop mode, 16 before decision 11). U-map-entry stays the one
position C# drops, and it is disputed. The BDN pre-timing checks now also require core-ffi
retain and host-gen retain to re-encode every one of the 92 timed unknown rows to the
incumbent's bytes (744 checks, all pass), so requirement 10's retain rows are timed on a
decoder that retains. Not built: the placement controls (pool, refill, oneof move, CAPACITY):
this host only uses grow.

### 53. CAMPAIGN req 12 amended (85cb00f): C and D per unknown-field mode

The RPC grid's C and D now run as `C-retain`/`C-drop` and `D-retain`/`D-drop` in both
directions (retain: decision 11's options armed at every position, `ak_uencode_*`; drop: reset
with NULL, `ak_encode_*`); samples carry `unknown_mode`. A retained decode that leaves a grown
buffer undelivered fails its call, so every call still checks it. `C-nounk`/`D-nounk` wait for
the compiled-out build.

**A runner defect the first smoke found.** At 253f487 the runner printed "the correctness gate
FAILED" and still produced rpc samples: `GATE="$(gate_first)"` runs gate_first in a command
substitution, so its `exit 1` only left the subshell. It has been like that since WP3 (every
earlier smoke's gate had passed, so it never showed). Fixed (`|| exit 1` on all three suites),
the samples discarded, the suite re-run at 637e77d behind a passing gate. The failing gate log
itself was overwritten by the plant run's gate before I read it, so why that gate failed is
not established; the next two gates (on later HEADs, other slices committing meanwhile) passed.
Smoke: 60 samples per transport, no call failed, the abort control 0 samples; figures stripped.

### 54. The gate failure at 253f487: reproduced 0 of 6 times; cause not found

Asked by the aggregating session to treat it as a finding. What is known: during the first
req-12 rpc smoke, the runner's gate (gen/gate.sh, run by gate_first at 253f487) printed
"the correctness gate FAILED" into rpc1.out. Its log (campaign/gate.log) was overwritten minutes
later by the --plant run's gate before I read it, so which step failed is not known; the next
two gates (9957fc9, 637e77d) passed, and 253f487 and 9957fc9 are identical in every path the
gate reads (ffi/poc/csharp, poc/codec, schema, corpus).

Reproduction: two worktrees (253f487 and ef00211, HEAD then), the committed gate.sh run three
times in each, sequentially per worktree, the two sequences in parallel (load average 12 to 17
during them, from these two plus the other slices' builds; disk 67 to 82 %). Each run's log
under its own name, `logs/csharp/gate-repro/gate-<label>-run<N>.log`, with load and disk at start
and end. Result: **253f487 3 of 3 passed; HEAD 3 of 3 passed; 0 failures in 6 runs.** The
failure did not reproduce, and I have no evidence for its cause. Differences from the original
run that I can name but not test after the fact: it ran in the main tree (the reproductions ran
in worktrees), in the scratch directory later reused by the plant run, while the other slices
were building their decision-11 ports.

Fixed so it cannot be lost again: every runner gate run writes its own
`gate-<commit>-<utc>-<suite>[-PLANT].log` and gate.log is only a copy of a passed one (ef00211).
The runner no longer continues past a failed gate (637e77d). Both mechanisms ran in the smokes
since (gate-3cf32ba-...-rpc.log, gate-837b738-...-rpc-PLANT.log, both passed).

### 55. WP5 step 10: the NO-UNKNOWN variant (unknown fields compiled out)

**Rendering** (my backends, poc/codec/gen, commit 0c77d01): cs_binding, cs_host and
cs_layout_probe render from `plan.unknown_compiled_out` on a plan relowered with
unknown="drop": no u-groups, no `ak_uencode_*`/`ak_uelem*_*`, no options, no reset,
`ak_dec_ctx_new_<Root>()` without a parameter, no bag capture, no grow; `retain` = true is refused.
Both import forms stay in the one generated file. An `AbiVariant` class checks at load time that
the core is the same variant (the u-family export `ak_uencode_<first root>` present in the full
core, absent in the variant): the harness, the corpus runner, every BDN process and the rpc client
refuse to run on the wrong core, and the gate has a control that loads the full core under the
variant binding (fails as required).

**Build** (3b3fa36): `/p:AkNounk=true` is its own build configuration: GeneratedNounk/ replaces
the variant files, AK_NO_UNKNOWN_FIELDS is defined, output in bin-nounk/ obj-nounk/, so both
builds exist side by side. host-gen no-unknown IS plan-generated: the managed codec is rendered
from the drop plan (no capture code); the facade types are identical in both plans (checked).
Cores: target-core-nounk, target-core-count-nounk, target-core-corpus-nounk (ak-core
`--no-default-features`, own target dirs): 96/96/171 ak_* exports against 117/117/241, 0
`ak_uencode_*` against 7.

**Gate** (wp5s10-gate.log, GATE PASSED at 2410125, net8.0 and net6.0): layout by name 78 structs /
305 members and 240 section-10 facts (shapes), 340 facts (corpus); byte identity 152/152; the
loaded core is the variant; its crossing counts equal `gen/crossings-nounk.txt`; the corpus with
the two drop arms: managed 696 pass, ffi 680 pass, every unknown row in the dropped form (313 and
307 rows dropped, 0 retained); rule 6 (decode and parse refused on another root's context, no
reset exists). Crossing counts against the full build: **one row differs, P1.2 push-decode reverse
8 -> 5**, as in the rust slice (the decode group without `ak_unk_buf` fits more elements per
32 KB chunk).

**Harness**: BDN units `host-gen:no-unknown`, `core-ffi:no-unknown`, `core-ffi-pull:no-unknown`
(incumbents as in-process controls), pre-timing checks in the variant: byte identity and the
dropped form agreed by host-gen and core-ffi on every unknown row (468). RPC: the no-unknown
client runs A, B, C-nounk, D-nounk against the same server. run_campaign.sh builds both, runs both
per launch in an order alternated by launch, and checks both crossing files before calib.

**A defect of mine the smoke found:** the rpc edit described in 3b3fa36 had not reached
Campaign.cs. The python script that made it aborted on an assertion (the header string I matched
occurs twice), and its error output was suppressed; the build succeeded because nothing
referenced the missing code. The no-unknown client ran the full cell list and aborted on its first
retain call, no sample written. Fixed in 3a9b67c. I checked the other edits of this unit by their
output (every one shows in a log); from here on no edit script runs with its errors hidden.

### 56. FIX-PLAN WP6 step 1: STATE rewritten, content sets in the counts, R-C9, a clean gate

- **STATE.md rewritten** to say what is true now. Removed: the status line's history (WP3, BDN,
  decision 11, step 10 narratives: they are in entries 47 to 55); the retired-files list of step 4;
  the step-4 and WP5-tail "what was checked" sections (superseded by the clean gate); the
  closed-defect paragraphs (D1, D2, D3, D7 to D10, D38, D40, D41: closed, recorded here); the
  step-9 decision 11 paragraph and the smoke narratives (superseded); the engine-cost figures
  that came from traces never committed (per-case seconds at 20/60/400/1,780 cases, GC pause
  shares, heap sizes: their raw traces were scratch files, so the figures go); the RPC-to-BDN
  evaluation reduced to the reason req 22a asks for. Kept, labelled instrumentation: the smoke
  counts (committed logs) and the default-job unit's duration (committed log). The checklist
  is current against CAMPAIGN.md at 85cb00f, with 22a as its own row; rows 1 to 3 are marked
  not applicable in the container (the owner's machine) rather than met.
- **Content sets in the counts:** CoreGate now counts P1.2 in the Latin-1 and wide sets (the
  expected bytes are the incumbent's for the graph built under the set). Both crossing files
  gain two rows: full 2 1 1 8 2 0, no-unknown 2 1 1 5 2 0, the same as the ASCII row of each
  and as the rust slice's.
- **R-C9:** stages 18 and 19 were built against a core not on the branch. Nothing in STATE
  rests on them; their log headers now say no figure in them is usable.
- **The clean gate:** a fresh worktree at the committed HEAD, no reused build directory, both
  builds on net8.0 and net6.0 (wp6s1-gate.log).

### 57. WP6 register H: the findings assigned to csharp, confirmed or refuted, and fixed

Each finding arrived unconfirmed. Per finding: what the tree showed, what changed, the proposed
disposition.

- **R-H11 (host-gen "both" codec): confirmed.** cs_managed rendered the default plan
  (unknown="both") with the capture behind `Dec.Retain`, so the drop arm carried capture code.
  Now cs_managed refuses "both" and renders one codec per mode under a class name: `Codec`
  (drop plan) and `CodecRetain` (retain plan); `Dec.Retain` is gone from the runtime. The arms:
  BDN host-gen drop/retain, the corpus's managed-drop/managed-retain and `harness unknown` use
  the matching codec; the drop codec is also the no-unknown build's. Disposition: fixed.
- **R-H22 (the no-unknown facade member): GS2 was right, CP3 was not.** cs_types emitted
  `UnknownFields` in every class and no variant Types.cs existed; STATE:131 (at the review's
  commit) did not claim the member was removed, it said host-gen no-unknown is rendered from
  the drop plan. Now the no-unknown build has GeneratedNounk/Types.cs and Eq.cs without the
  member (owner decision, CAMPAIGN req 10), and `harness coreffi` and `corpus --variant` check
  the member's presence against the build. Disposition: fixed.
- **R-H2 (RPC threads in the window, warm-up): confirmed.** BlockingOp created `inflight`
  threads per sample inside the window; A and D ran on the thread pool; the warm-up was 32
  calls per cell with no tier read back. Now one pool of caller threads (CallerPool) is created
  before the warm-up and reused by every cell and sample; A and D call through
  BlockingUnaryCall on those threads (grpc-dotnet has no synchronous transport: the caller
  blocks while the I/O runs on the thread pool, stated), the callback/queue rows block on their
  completion. Warm-up: rounds of 64 calls per cell, 0.5 s apart, until a round compiles nothing
  (a trial took 9 rounds); each sample records the JIT compilations inside its window
  (`jit_in_window`), with a summary line (the trial: 56 of 60 samples compiled nothing,
  container instrumentation, not committed as a log). Disposition: fixed.
- **R-H3 (crossing gate completeness): confirmed.** Rows the run produced were compared, but a
  committed row not produced passed, and an empty file compared nothing. Now both fail; the gate
  has a must-fail control for each. Disposition: fixed.
- **R-H6 (no build field): confirmed.** Every codec and rpc sample now carries `"build"`.
  Disposition: fixed.
- **R-H9 (no twin for "0 undelivered"): confirmed.** A gate plant
  (`AK_GATE_PLANT_SKIP_RELEASE`, rendered in cs_host like `AK_GATE_PLANT_NO_INIT`) skips the release
  of every taken bag; `corpus --unk-controls` must then fail, and does (307 rows UNDELIVERED).
  Disposition: fixed.
- **R-H14 (C4 codes, numbering): confirmed for C#.** C4 checked only that some code came back,
  and the managed runtime numbered malformed -4 and depth -8 against plan.FIXED's -2 and -4. Now
  the runtime's codes equal plan.FIXED's (the corpus runner checks the equality at start), and
  C4 compares each refusal's code with the code the row's `reject.reason` calls for per the
  plan's DECODE RULES (a reason with no mapping fails the row); a must-fail plant swaps the
  expectation. Result: every reject row refused with its expected code by all four arms. The
  C# part of "a selected null message member writes an empty body" was not in the assignment
  and is not changed. Disposition: fixed (C# part).
- **R-H15 (backend-local tables): confirmed for RUN_FN.** Now read from `plan.FIXED.run_types`.
  cs_managed's `unknown == "drop"` test selects the managed codec's MODE, which is what the
  option means there; the variant decisions in cs_binding, cs_host and cs_types use
  `unknown_compiled_out`. Disposition: fixed (RUN_FN); the mode test stated.
- **R-H18 (JIT check only warns; rpc schedule): confirmed.** A `jit check: FAIL` now fails the
  unit and the runner stops the launch. The rpc cell order was the same rotation in every
  launch; it is now a seeded shuffle of launch and round. Disposition: fixed.
- **R-H19 (stated facts): confirmed for C#.** The smoke `.bdn.log` files carried figures without
  a header; they are headed as instrumentation, and the runner heads every smoke one. "In-process
  control" was wrong for the codec suite (each BDN unit is its own process) and is corrected; it
  is right for the rpc no-unknown client (A and B run in the same client process). Disposition:
  fixed.
- **R-H23 (order): randomised.** BDN has no built-in random order but takes an IOrderer: the case
  order in each process is a seeded shuffle (prime cases first), the unit order of a launch a
  seeded shuffle, seeds in the headers. Disposition: done.

Regenerated: this slice's `generate.py --check` is clean; the shared `poc/codec/gen/generate.py
--check` reports the csharp slice clean (exit 0) and stale files in the java slice only (other
slices' work in progress). Gate: `logs/csharp/wp6h-gate.log`.

**Found by the first gate after these fixes:** C4's new code check failed the three oracle-probe
rows (poc/rust/gen/probe_corpus.py's manifest), whose reject rows carry no `reject.reason`. A
row that states no reason cannot say which code is right, so its code is now reported as not
checked in the row's form ("refused; code -2 not checked: the row states no reason"), neither
failed nor hidden (871993214). That gate was stopped at its first failure and re-run from a new
worktree; the failed run's log was not committed (the stopped run is not a complete gate).
The final gate ran from a fresh worktree at the pushed HEAD b758b2737 (this slice unchanged since 871993214; core 31fc3eecf): GATE PASSED, `logs/csharp/wp6h-gate.log`. C4 code-checks every reject row of the corpus; it reports as not checked the 3 oracle-probe reject rows, which state no reason.

## 58. WP7: the harness on the 2026-09-26 contract (R-H22 to R-H36)

Ten items from FIX-PLAN WP7, under the owner's scope rule of the same day (fix only what can
change what a timed arm or cell does or costs). What was built, checked and found:

- **req 21, process CPU per round in BDN.** The job's clock is now `CpuClock` (Job.WithClock):
  BDN's engine calls `IClock.GetTimestamp` at the start and end of every iteration, and while the
  diagnoser has recording on (BeforeActualRun..AfterActualRun) each read also takes
  CLOCK_PROCESS_CPUTIME_ID (before the Stopwatch at a start read, after it at an end read, so
  the CPU window contains the wall window). Each exported iteration row carries `cpu_ns` of its
  own window. "Is it running": the diagnoser requires exactly 2 reads per actual iteration, else
  the case is written as FAILED and the unit exits non-zero (`cpu check` line). A smoke subset
  (P2.4 and U-deep-all, core-ffi:retain) passed the check on every case. The old per-case span
  value is kept as a `row: case-summary` line with `span_cpu_ns` (no `cpu_ns`), so it cannot be
  read as a sample. BDN's forced GCs between iterations are outside both windows.
- **req 7.** Content sets on P1.2, P2.2 and P2.4 (codec suite, pre-timing identity, CoreGate
  crossings rows for P2.2/* and P2.4/*: equal to the ASCII rows, as expected since strings are
  one run per field). U-* rows (the 92 at the 7 ABI roots): `encode-hot` added (each arm
  encodes a graph it decoded from the row itself, untimed: retain arms re-emit the unknown
  fields, drop arms the dropped form), incumbent-best added to encode/decode/decode-read. The
  pre-timing check verifies each: incumbent-best = incumbent-prod; core-ffi retain and host-gen
  retain = the incumbent; core-ffi drop = host-gen drop; no-unknown build: core-ffi = host-gen.
  decode-reencode stays as a labelled extra (no incumbent-best row). A smoke keeps 6 rows.
- **req 11.** Directions `encode` (pool + reused buffer), `encode-hot`, `encode-transport`
  (pool + the Grpc.Net form), `encode-transport-hot`, the java slice's names. The Grpc.Net form
  is the serializer the RPC grid's marshaller runs (`Ops_*.SerInc/SerHost/SerFfi`, now shared:
  the RPC grid calls the same static methods) into `GrpcFrame`, which does what Grpc.Net.Client
  2.71's internal `GrpcCallSerializationContext` does on its direct path: one ArrayPool array,
  the 5-byte header, the body, the array returned. That behaviour was checked by reflection on
  the assembly (`logs/csharp/wp7-grpcnet-context-reflection.log`: ResolveBufferWriter rents,
  WriteHeader, Reset returns). incumbent-best has no gRPC path and gets no transport row; for
  core-ffi over the core's transport (C) and host-gen over it (E) the transport form IS the
  buffer row (stated). Pool: graphs built in the case's GlobalSetup until their RETAINED heap
  (GC.GetTotalMemory(true) before and after) is at least 2 x AK_LLC_BYTES (default 13.75 MB);
  hot = a pool of one, so every encode row runs the same `Next()`. Two defects found on the way
  and fixed before any figure: the first pool measurement counted the probe graphs (186 MB for
  a 2-graph pool), and the per-graph probe of 8 small graphs read ~0 bytes, so the pool count
  ran away and BDN's in-process timeout aborted the unit; now the probe doubles until the heap
  grew 1 MiB, the pool is measured on its own, and topped up until it reaches the target.
- **req 12-17 (RPC).** Cells E (host-gen over the core's transport; blocking) and F (host-gen
  over Grpc.Net), in drop and retain (full) and no-unknown (nounk build). One server process per
  launch: two Kestrel hosts in it (Kestrel's HTTP/2 windows are per host), shipped and pinned on
  two sockets, serving both builds; `rpc-warm` sends 2,000 calls per direction per client
  transport per socket before any client (100 in a smoke); the server prints what it served on
  shutdown. One channel per cell (a GrpcChannel with its own handler, or a CoreChannel on one
  shared core runtime of 2 workers, new ctor). Directions a, a+read (Touch after the decode), b.
  A, D, F now `await CallInvoker.AsyncUnaryCall` (k in flight = k async loops on the thread
  pool), replacing BlockingUnaryCall; B, C, E stay blocking on the caller pool. D's marshaller
  takes a core-ffi context from a ConcurrentBag instead of a ThreadStatic (see counts below).
  Test against the shared server: full client 84 samples (14 cells x 3 dirs x 2 levels), nounk
  18, 0 aborts.
- **req 19.** A counting build `/p:AkHostCount=true` (AK_HOST_COUNT): cs_binding renders every
  import as a counted wrapper over `<name>__raw`, so the counts are of every exported entry point
  by name where the host calls it. Codec: `BenchDotNet --counts` runs each core-ffi case's own
  timed closure once untimed, then once counted (1,044 cases full, 544 nounk). RPC:
  `akrpc campaign --suite rpc --counts` against a server (30 rows full, 18 nounk; stable over
  two runs). Retain: no pre-placed buffer (already so) and `UnkHost.Exact` (grow allocates what
  the core asks). Resets are in `fwd` and counted apart (`reset`), two per decode, before and
  after. The gate compares all four files whole; a control with doubling growth must differ and
  does (54 retain rows, e.g. U-deep-all decode retain grow 8 exact vs 2 doubling).
  **What the new counts show that the old ones did not:** the timed encode makes 2 more forward
  calls than `gen/crossings.txt` says (`ak_enc_reset`, `ak_enc_take`), and the timed push decode
  4 more (`ak_dec_err`, `ak_dec_err_reset`, and the 2 resets; P1.2: 5 vs 1). `gen/crossings.txt`
  stays: it is the R5 comparison of the CoreArms tally with the core's own counters, not the
  timed loop. Two contaminations were found in the first RPC count and fixed: the queue
  drainer threads of the extra rows called `ak_queue_next` inside a count (the counting run now
  builds no extras), and D's ThreadStatic context was created inside a counted call when
  Grpc.Net resumed on a new pool thread (hence the pooled context, which also removes that
  creation from timed D calls).
- **req 4/22/30.** Thread-pool min/max and current thread counts, caller threads and the core
  runtime's workers in every header (codec, rpc client, server, calib). Seeded shuffles kept.
  Ratios from per-launch medians stated. The runner reads ffi/campaign.machine when AK_CPU_*
  are unset outside a smoke (campaign.sh exports the same values) and refuses a set whose size
  is not AK_SET_SIZE.
- **req 10** was done in WP6 (R-H22).

Gate from a fresh worktree at `c35bd22` (both builds, net8.0 and net6.0, step 9 included):
GATE PASSED, 0 step failures, 27 controls failing as required (`logs/csharp/wp7-gate.log`).
Smoke of every suite from the same worktree (`logs/csharp/campaign/wp7-smoke/`, figures
stripped): codec 1,548 samples over 12 units, every new row present, cpu and jit checks PASS;
rpc 126 + 54 samples per transport, 0 aborts, the abort control 0 samples in all 4 clients;
calib 2 samples. The session was stopped by the API spend limit after the smoke had finished;
nothing ran twice. One line under the scope rule: the `--plant` run reuses the names
`rpc-launch1.server.log` / `server-warm.log`, so it overwrote the real smoke's server log (no
effect on any sample; not fixed).

## 59. WP8: the Rust optimisation experiment's shared changes, and the upload directions

At HEAD 98b187ce6 / e4c7e97cd (the owner's merge; generated Abi.cs and RpcAbi.cs already
regenerated there, `generate.py --check` clean). Rebuilt the six cores from `git archive HEAD`.

- **Call sites (ABI v1 section 9 as amended).** The build failed exactly at the three
  `ak_call_unary` sites (CoreTransport.cs 167, Campaign.cs 358, 417): each now passes
  `&grpc_status` and names it in the abort. `ak_completion` is only used through the generated
  struct (by member name), so the new `grpc_status` member needed no layout work; both
  completion readers (the callback, the queue drainer) now keep it for the error message. A
  non-OK status (AK_ERR_RPC_STATUS -12) fails the call under req 18, as any rc != AK_OK already
  did. `ak_call_close` was never called here.
- **Counts on the geometric grow (req 19 as amended).** The counting build no longer sets
  `UnkHost.Exact`: it grows as the timed build does (`max(want, 64, 2 x cap)`, now clamped to
  INT32_MAX, rule 8). The gate's control is inverted: `AK_COUNT_GROW=exact` must differ, and
  does (the committed rows carry fewer grows).
- **One reset per decode (decision 11 rule 7 as amended).** cs_host's `Disarm` no longer calls
  `ak_dec_reset_<Root>(ctx, NULL)` after the decode; the arming reset before it is the only
  one (the options stay at their stable native address `_uo`). This is a timed-path change for
  every retain and drop decode of the full build.
  **What changed in the committed counts, and why:** `gen/counts.txt`: 684 decode rows fwd -1
  and reset -1 (the removed disarming reset; every push and pull decode of the full build), of
  which 54 retain rows with unknown fields also ask fewer grows (-1 to -10: the geometric grow,
  e.g. U-deep-all decode retain grow 8 -> 2); 360 rows (encodes) unchanged.
  `gen/counts-nounk.txt`: unchanged (no reset, no grow in that build). `gen/rpc-counts.txt`: the
  8 decode rows of C and D fwd -1 / reset -1; 60 new rows (c and d, below).
- **Binding-side lessons (WP8 item 4).** Re-validation: the binding does not re-validate a
  string the core accepted (`Encoding.UTF8.GetString` is the transcode to UTF-16 the facade
  needs, not a check): nothing to change. Sparse fill: each element group is cleared with
  `g = default` (that element only) and then filled; there is no arena-wide clear: nothing to
  change. One reset per decode: changed (above).
- **D44.** The client limits were already 64 MiB send and receive on both transports (core
  `ak_client_opts`, Grpc.Net channel), and Kestrel's 64 MiB; they cover P5.4 (4,194,390 B) and
  the 2 MiB stream messages. Nothing relied on them being ignored; now stated in the header.
- **Directions c and d (req 14 as amended, required).** Upload.cs: c = P5.3 / P5.4 unary to a
  new server method `Upload` (decoded with the incumbent, empty upload refused); d = the
  streamed upload, M5 messages of 2 MiB splitmix64 chunks, ids on the first only, 4 MiB and 16
  MiB, to `Stream` (every message decoded, the ids required on the first; answers the data byte
  count, 8 bytes LE) and its twin `StreamCheck` (+ the SHA-256 of the data as received). Every
  cell has both (A/D/F Grpc.Net `AsyncUnaryCall` / `AsyncClientStreamingCall`; B/C/E the core's
  `ak_call_unary` / `ak_call_open` + `ak_call_send` per message + `ak_call_recv`), plus the
  framed twins Bf, Cf-*, Ef-* (`ak_client_set_framed(1)`), in every mode; at 1 and 8 in flight,
  a quarter (c) and an eighth (d) of the calls per sample. Before any call, every message's
  bytes are checked equal across the incumbent, host-gen and core-ffi. Before the warm-up,
  every c cell is called once and every d cell once through StreamCheck (count and SHA-256).
  Every timed call checks status and the response (0 bytes on c; the count on d). The server
  warm-up adds a tenth as many c and d calls per client transport. Not used:
  `ak_call_unary_enc`, `ak_call_send_enc`, `ak_enc_take_owned` (C and D keep the copy paths
  they had on b; stated), `ak_call_opts` (NULL).
- **Req 18 controls.** The runner's `--plant` now runs, per build and transport, a wrong
  expected length on a, c and d and a wrong expected SHA-256 on d, each on cells A, B, Bf and D
  one at a time (`AK_CAMPAIGN_ONLY`); checked by hand before the gate: every one aborts with 0
  samples. The gate (step 9) runs the upload check on both builds and two must-fail plants
  (digest, count).
- One found on the way: the first RPC count run keyed its rows by (cell, dir, mode), so the two
  payloads of c and of d collapsed into one row each; the payload is now in the key.
- Test run (container instrumentation, not kept): full client 204 samples over 1 and 8 in
  flight, 0 aborts, 60 upload cells checked.

Gate from a fresh worktree at `d97ea52` (both builds, net8.0 and net6.0): GATE PASSED, 0 step
failures, 29 controls failing as required (`logs/csharp/wp8-gate.log`). Smoke from the same
worktree (`logs/csharp/campaign/wp8-smoke/`, figures stripped): codec 1,548 samples, checks
PASS; rpc 246 + 126 samples per transport, 120 + 72 of them on c and d, 0 aborts; 60 plant
controls, all aborted with 0 samples for their planted reason (the plant run now has its own
directory, so WP7's overwritten server log does not recur); calib 2 samples.

## 60. WP8 parity: C on the move path, framed twins on b

Two items the aggregating session put in scope (parity: the other slices do this work).

- **C on the move path.** cs_host gained `EncodeInto(src, retain)` (the encode left in the
  context, no `ak_enc_take`) and `EncContext`. Cell C now sends b and c through
  `ak_call_unary_enc` (the context's buffer moved into the request, no copy anywhere) and each
  d message through `ak_call_send_enc`. The copy path is kept as the labelled extra `Cc-*`
  (`ak_enc_take`, then `ak_call_unary` / `ak_call_send` copy the bytes) on b, c and d. Counts:
  C b 2,505 forward (was 2,506: no `ak_enc_take`), c 4 (was 5), d 4 MiB 10 and 16 MiB 28 (were
  12 and 36); Cc carries the old figures. Direction a is a decode: nothing to move.
- **D stays on a copy, stated.** Grpc.Net's serializer can only write into the call's own
  `IBufferWriter` or hand it a `byte[]` (`SerializationContext.Complete(byte[])`, which the
  context copies too; JOURNAL 58's reflection log), so an owned core buffer from
  `ak_enc_take_owned` would still be copied, and would add an entry and a free per call. D keeps
  copying straight from the core's encode buffer into the call's buffer.
- **Framed twins on b.** `Bf`, `Cf-*` and `Ef-*` now run b as well as c and d (a has an empty
  request, so its send path is the same on either, and no framed a row is built, as in the
  other slices' b-only framed rows).
- RPC counts regenerated: 105 rows full, 62 no-unknown. The codec counts do not change (the
  codec suite's core-ffi arm takes its buffer as before).
- Scope-rule line: the runner's per-cell plant controls cover A, B, Bf and D; C's move path is
  covered by the gate's upload check (count and SHA-256) but not by its own length plant.

Gate from a fresh worktree at `9114d6b` (both builds, net8.0 and net6.0): GATE PASSED, 29
controls failing as required; counts equal (codec 1,044 / 544, unchanged; RPC 105 / 62); upload
check 68 / 40 cells (`logs/csharp/wp8b-gate.log`). Smoke from the same worktree
(`logs/csharp/campaign/wp8b-smoke/`, figures stripped): codec 1,548 samples, checks PASS; rpc
283 + 146 samples per transport, b in every cell and twin, 0 aborts; 60 plant controls, all
aborted with 0 samples; calib 2 samples. The session was stopped by the API spend limit during
the smoke; the smoke kept running and completed, nothing was re-run.

## 61. CAMPAIGN req 24 as amended (85cfd4826): every warm-up is a runner parameter

- RPC client: `--warm-rounds`, `--warm-calls`, `--warm-settle-ms`, `--warm-jit-stop` (runner:
  AK_RPC_WARM_ROUNDS / _CALLS / _SETTLE_MS / _JIT_STOP). Campaign default unchanged: at most 10
  rounds of 64 calls per cell, direction and in-flight level (divided by 4 on c, 8 on d), a
  500 ms settle after each, stopping on a JIT-quiet round. Smoke default: 1 round of one
  sample's calls (16), no settle wait, no JIT-quiet stop. The header's warm-up line states
  every value, and says when no settle wait means late JIT events may be missed.
- Server warm-up: AK_RPC_SERVER_WARM (2,000 / 100 per direction and client transport).
- BDN: AK_BDN_ROUNDS, AK_BDN_WARMUP and AK_BDN_ITERATION_MS now reach `--rounds`, `--warmup`
  and `--iteration-ms`, which only the program took before. The pre-warm is a parameter too
  (AK_BDN_PREWARM_ROUNDS / _CALLS / _SETTLE_MS -> `--prewarm-*`, defaults 10 / 64 / 500 ms also
  under --smoke, because the unit's JIT check needs the tier-up; stated in the runner's
  comment). Each value is in the unit's header.
- Check (owner: no before/after timing; the full smoke started for it was stopped): akrpc run
  directly with the smoke defaults, one transport, full build (`logs/csharp/campaign/
  wp8c-warmup-knobs/`, figures stripped): 283 samples, 0 aborts, the header line reads "1
  round(s) run of at most 1, 16 calls ..., settle wait 0 ms ..., stop on a JIT-quiet round:
  off". The gated code paths (counts, upload check) are unchanged, so no gate was re-run.

## 62. WP9: the RPC grid on BenchmarkDotNet; the codec suite's hand-written warm-up removed

CAMPAIGN req 22a as amended (c16afb2d6, addendum bc7cf94b1: use the framework's own warm-up,
iteration and invocation control, isolation, ordering and export; custom code only where a
requirement needs it).

- **`akrpc bench`** (src/Rpc/RpcBench.cs): BenchmarkDotNet 0.15.8, InProcessEmit, one pinned
  process per UNIT = one cell (A, B, Bf, C-retain, Cf-retain, Cc-retain, D-retain, E-retain,
  Ef-retain, F-retain, the same in drop, and the four core-delivery extras: 21 units in the full
  build; 10 in the no-unknown build). A case is `cell|dir|payload|k`; three benchmark classes
  (RpcK1, RpcK8, RpcK16) because OperationsPerInvoke is an attribute constant: one invocation is
  one batch of k calls in flight, counted as k operations (`iters` = calls, `invocations` =
  batches in the export). The cell's channels are opened once per process, before BDN starts
  (one channel per cell per benchmark process); the caller pool too. The upload cells' count
  and SHA-256 check runs once in each case's GlobalSetup (skipped after the first).
- **Framework mechanisms used as they are:** the jitting stage, pilot (UnrollFactor 1, the
  pilot picks the invocations), warm-up iterations, actual iterations, InProcessEmit, the
  JoinSummary run, StopOnFirstError.
- **Custom pieces, each for a requirement:** the server process and its warm-up (req 13; the
  runner, as before); CpuClock as the job's clock (req 21: BDN has no CPU per iteration); the
  seeded IOrderer (req 22: BDN has no random order); the JSON-lines exporter with every label
  (req 28), which writes no sample when any case failed (req 18); the JIT tier read back (req
  24's "recorded"); the runner's discard of a failed launch (moves its files to *.DISCARDED).
- Checked before the gate: one unit (C-drop) 17 cases, 0 failed, 17 samples with k-scaled
  operations (e.g. d 16 MiB k = 8: 4 invocations, 32 calls); plants under BDN: a wrong length
  on c (B), on a (A), on d (D-drop), a wrong SHA-256 on d (Bf), each: the case fails, BDN
  stops, 0 samples, exit 1. The no-unknown build's Cc-nounk unit: 11 cases.
- **Removed:** the hand-written RPC sampler (`Grid`, its warm-up loop and settle wait, and the
  AK_RPC_WARM_* knobs); `campaign --suite rpc` now only takes `--counts` and `--upload-check`
  (the counting and gate paths, unchanged). The plant controls now also cover C (the move
  path), closing JOURNAL 60's gap.
- **The codec suite's hand-written pre-warm loop is removed** (and its settle wait and
  `--prewarm-*` knobs). Tried first without it and without the two prime cases, on the default
  job (10 warm-up x 100 ms): each unit had 1 case measuring hot code at tier 0 (the first case
  of the process; `System.SpanHelpers::Fill` and `RuntimeHelpers::IsReferenceOrContainsReferences`,
  runtime helpers BDN's engine touches first), so the JIT check failed. Without the pre-warm but
  WITH the two prime cases (BDN cases like any other, not exported): PASS on both units tried
  (core-ffi:drop, incumbent-prod:default). So the primes stay, stated in the header and STATE;
  the loop is gone. The JIT check stays fatal outside --smoke and is reported, not fatal, in a
  smoke, whose 1 x 2 ms warm-up cannot reach tier 1 by design.
- CpuClock and ProcCpu moved to src/BenchDotNet/CpuClock.cs, linked into akrpc.

WP9 gate from a fresh worktree at `6a7c0cf` (both builds, net8.0 and net6.0): GATE PASSED, 29
controls failing as required (`logs/csharp/wp9-gate.log`). The full smoke the gate run was
chained to was stopped under the owner's small-test rule (2026-09-27); a minimal smoke instead
(`logs/csharp/campaign/wp9-smoke/`): full build, shipped, all 21 RPC units on BDN with 1 round, 1
warm-up, 2 ms iterations: 283 samples, 0 failed cases, a planted wrong count aborting with 0
samples; one codec unit with BDN's own warm-up: JIT and CPU checks PASS.

## 63. WP10: the Rust slice's rpc_server is the server

CAMPAIGN req 13 as amended (9f6d579fa), the interface poc/rust/SERVER.md at bed13a6ea.

- Every cell now calls `armonik.ffi.campaign.v1.Grid`: Fetch (a, a+read), Push (b), Upload (c),
  UploadStream (d), UploadStreamCheck (the pre-timing check). The request and response bytes
  are the same as before (P2.2 540,422 B on Fetch; 0 bytes on Push and Upload; 8 bytes LE on
  UploadStream). One rule differed: UploadStreamCheck's SHA-256 is over every MESSAGE's bytes as
  received, where this slice's server hashed the data chunks; the client's expectation is now
  the SHA-256 of the messages' wire bytes (all codecs' bytes are checked equal first). The ids
  on the first message only: unchanged.
- A: the incumbent through hand-built `Method` objects whose marshaller is Grpc.Tools' generated
  shape (SetPayloadLength(CalculateSize) + WriteTo(IBufferWriter) + Complete; ParseFrom
  (ReadOnlySequence)), with the message classes Grpc.Tools generates from shapes.proto, as
  before. No generated Grid stub: UploadStream answers 8 raw bytes, not a protobuf message, so a
  generated stub could not carry d, and D and F need their own marshallers on the same methods.
- Plants: unchanged, client-side (SERVER.md's second option): a wrong expected length on a, c,
  d and a wrong expected SHA-256 on d.
- The runner builds, starts, warms and stops the server through poc/rust/serve.sh once per
  launch (AK_CPU_SERVER pins it; its own AK_SERVE_STATE under the scratch dir, so no other
  run's server is met); `shipped` / `pinned` are the client's configuration against the
  server's two sockets (tonic defaults / 4 MiB windows, adaptive off). New subset knobs for
  small runs: AK_RPC_TRANSPORTS, AK_RPC_BUILDS.
- Removed: this slice's Kestrel server (campaign `rpc-server`, `rpc-warm`, CampaignService,
  CampaignProvider), the pre-campaign timing modes of akrpc that ran their own in-process
  server (--grid, --stream, Bench.cs, Grid.cs, Stream.cs, StreamRun.cs, Codecs.cs; D42), and
  the ASP.NET Core dependency (the project is plain Microsoft.NET.Sdk now, Grpc.Net.Client
  only). The gate's R-D9 error-path check now runs against the campaign server (Fetch OK,
  StatusU13 a non-OK status): 6 of 6 rows PASS.
- Checked against the shared server before the gate: the upload check (68 cells full build),
  the error path, a BDN unit (A, 17 samples), and the RPC counts of both builds, equal to the
  committed files.

## 64. Req 22a as amended (e6c909630): BDN's native isolation for the campaign, grouping a switch

- WP10's minimal smoke (at 76ac71e1d, before this change; InProcessEmit, the only mode then):
  full build, shipped, all 21 units, 283 samples, 0 failed cases; 19 plant controls (wrong
  length on a, c, d and wrong SHA-256 on d, on A, B, Bf, C-drop, D-drop), every one aborting
  with 0 samples. Its logs are in `logs/csharp/campaign/wp10-smoke/`.
- Both suites now take `--toolchain process|grouped`; the runner passes `process` (BDN's
  default toolchain: a generated project, one child process per case) unless AK_BDN_GROUPED=1,
  which is the default under --smoke; the header says which.
- To make the default toolchain work: BDN looks for a project named after the assembly, so
  `src/Rpc/Rpc.csproj` is now `src/Rpc/akrpc.csproj` (AssemblyName akrpc, unchanged). The RPC
  cell context comes to the child through the environment (the child does not run Main).
- Req 21 under the default toolchain: the job's clock (CpuClock) runs in the child and the
  diagnoser in the host, so the first try wrote no sample ("process CPU per iteration not
  paired"). Now the child records every clock read and writes them at GlobalCleanup to
  AK_CPU_CHILD_DIR; the host takes the last two reads per actual iteration and checks each
  pair's wall span against BDN's own measurement of that iteration (2 percent + 20 us), so a
  wrong pairing fails the case instead of being guessed. Checked: an RPC unit (Bf, k = 1, 5
  cases, 2 rounds: 10 samples) and a codec unit (core-ffi:drop, P1.1, 6 cases): cpu check
  PASS.
- Open, for the aggregating session: under the default toolchain the JIT tier (req 24's
  "recorded") is NOT read back, because the listener sees only the host process; the rows say
  "not recorded" and the codec suite's JIT check is vacuous in that mode (stated in the
  header). Reading it back would need the child to run the listener and write its summary.

Gate from a fresh worktree at `d1a3a3b` (both builds, net8.0 and net6.0): GATE PASSED, 29
controls failing as required (`logs/csharp/wp10b-gate.log`). Minimal smoke through the runner,
grouped switch on (`logs/csharp/campaign/wp10b-smoke/`): full build, shipped, 21 units, 283
samples, 0 failed; 19 plant controls, every one aborted with 0 samples.

## 65. FIX-PLAN WP13: TCP, task-clock, pools, both h2 variants, D9

On the rewritten history (2026-10-03, logs/PURGED.md), code at `2f9ce48`.

- **TCP (D10).** Every timed cell runs against the shared server's TCP listener
  (`AK_SERVER_TCP=0`, `tcp 127.0.0.1:PORT` from serve.sh); `--sock tcp:127.0.0.1:PORT`.
  Grpc.Net: a TCP socket with `NoDelay = true` in the connect callback, address
  `http://127.0.0.1:PORT`; the core: `ak_client_opts.tcp_nagle = 0`, URI `http://127.0.0.1:PORT`.
  Readback (`src/Rpc/NoDelay.cs`): after one untimed call in each case's setup, every socket of
  the process to 127.0.0.1:PORT in state ESTABLISHED (/proc/self/net/tcp, inode to
  /proc/self/fd) is read with getsockopt TCP_NODELAY; none found, or one without it, fails the
  case. The upload check (gate) does the same. The listener runs the pinned server
  configuration only, so shipped and pinned differ on the client side only (stated in both
  headers). New control `AK_CAMPAIGN_PLANT=nagle` (Nagle left on in both client transports):
  A (Grpc.Net) and B (the core) each failed with "TCP_NODELAY read back: 0 of 1" / "0 of 2
  sockets" and 0 samples; the upload check with it reported 0 of 33 and failed. It is in the
  gate (one control more) and in `run_campaign.sh --plant` (A and B per transport and build).
- **Client CPU (req 21 as amended).** `cpu_ns` = perf task-clock of the whole process: one
  perf_event_open counter (SOFTWARE / TASK_CLOCK) per thread of /proc/self/task, new threads
  picked up at each read, read by the job's clock at the same iteration boundaries as the
  wall time; `proc_cpu_ns` = CLOCK_PROCESS_CPUTIME_ID beside it; a case without a task-clock
  per iteration writes no sample. Works under both toolchains (the child reads it; checked on
  Bf, process toolchain, 2 rounds: 10 samples). `client_softirq_ticks` / `client_irq_ticks`:
  /proc/stat summed over AK_CPU_CLIENT's CPUs across each case's actual run (USER_HZ ticks),
  on the round-1 row.
- **Pools (D8, D14).** AK_WORKERS (campaign.machine; default 8): the core runtime
  (`ak_runtime_new(AK_WORKERS)`), the .NET thread pool worker minimum and maximum
  (SetMinThreads / SetMaxThreads), the server's tokio workers (AK_SERVER_THREADS defaults to
  AK_WORKERS). The caller pool stays at the in-flight level (k dedicated caller threads, not a
  worker pool); grpc-core is not used (Grpc.Net is managed). In the runner and unit headers.
- **h2 variants (D11 as amended).** `gen/build_core.sh` builds the h2-batch source with
  `poc/codec/h2-batch/build.sh` and four more cores (`target-core[-count][-nounk]-h2b`) with
  `--config patch.crates-io.h2.path`; it prints the h2 compiled into each of the 8 transport
  cores (from the source path strings in the .so). Each akrpc process reads the loaded core's
  h2 from the library, writes it on every row (`h2`), and aborts if it differs from AK_H2
  (checked: AK_H2=h2-batch with the stock core, exit 3, no sample). `gen/gate.sh` takes
  AK_H2; the runner loops the rpc suite over AK_H2_VARIANTS (default "stock h2-batch",
  reversed on even launches), one gate per variant (`gate.log`, `gate.h2-batch.log`), file
  names `rpc-<transport>-<h2>-launch<N>[.nounk].jsonl`. Codec rows carry `h2: stock` (no
  transport call).
- **D9.** No native shim of its own; the shared core in the .NET process allocates its
  buffers with Rust's global allocator (glibc malloc), so the trim and mmap thresholds govern
  those buffers; managed objects are on the GC heap. The runner states GLIBC_TUNABLES in the
  header and does not set it.
- **A defect the second gate exposed (in scope: it blocks the runner).** The first pair of
  gates from a fresh worktree at `2f9ce48`: stock PASSED, then h2-batch FAILED at step 3, the
  net48 floor build (`HarnessFloor.csproj`) failing with CS0111 on `DualResponse`: its linked
  `../Harness/**/*.cs` glob did not exclude `obj-count*/` and `bin-count*/`, so the generated
  Shapes.cs of the counting builds made by the first gate's step 9 were compiled twice. Latent
  since WP7, harmless with one gate per tree; the runner now runs one gate per h2 variant in
  one tree. Fixed in `ea02da5` (both exclude lists); log kept as
  `logs/csharp/wp13-gate-h2-batch-FAILED-2f9ce48.log`.
- Gates at `ea02da5`, run by the runner's smoke from a fresh worktree, one per variant in the
  same tree: both GATE PASSED, 30 controls failing as required in each
  (`logs/csharp/wp13-gate-stock.log`, `wp13-gate-h2-batch.log`). Core at `59a96f8` (p1).
- Minimal smoke (`logs/csharp/campaign/wp13-smoke/`, grouped, TCP, pinned client
  configuration, full build, both variants): 21 units and 283 samples per variant, 0 failed;
  every row `net: tcp`, its variant's `h2`, `cpu_ns` and `proc_cpu_ns`. Runner `--plant`
  (stock, pinned, full): 21 controls, every one aborted with 0 samples, Nagle on included.
- Out of scope, not fixed (one line): the per-unit `# build ...` header line still says
  "server's pinned socket" and "Unix socket tcp:... (req 17: UDS)" over TCP (D45); the
  `# network` line beside it is right.

## 66. Req 25 / D9 as amended (ad1a15be5): default allocator for the main figures, pinned pass a switch

- The owner's amendment covers this slice (the core's buffers and transport use glibc malloc
  in the .NET process). Runner: `AK_CAMPAIGN_ALLOC=default` (GLIBC_TUNABLES unset; the main figures)
  or `pinned` (the labelled diagnostic, trim 256 MiB / mmap 32 MiB; files `.alloc-pinned`);
  the server is started with GLIBC_TUNABLES unset in both. Rows: `alloc`, `minflt` (getrusage
  ru_minflt read by the job's clock at each iteration boundary, carried from the child under
  the default toolchain). Headers state both modes.
- First check of the pinned pass on C-drop: minor faults per call were close in the two modes,
  which does not show the tunable is running (glibc's dynamic mmap threshold rises toward the
  pinned value on its own). So each process now reads the mode back at start: one 16 MiB
  malloc and mallinfo2().hblks around it. Default: "mmapped"; pinned: "heap" (also checked in
  python against the same glibc 2.39). A pinned process whose readback is not "heap" refuses
  to run, as does one whose AK_CAMPAIGN_ALLOC disagrees with its GLIBC_TUNABLES (control: exit 3, no
  sample).
- Smoke (`logs/csharp/wp13-d9-alloc-smoke/`, stripped): C-drop RPC unit and the codec unit
  core-ffi:retain in both modes, 0 failed, every row labelled and with minflt; Bf under the
  default toolchain, pinned: 10 rows with minflt. No gate (owner); the runner was not run end
  to end for this change.
- Renamed (owner, one name for every slice): the switch is `AK_CAMPAIGN_ALLOC=default|pinned` (was AK_ALLOC) in the runner, the processes' refusal and the headers; the startup readback is unchanged. Planted mismatch re-checked under the new name. The committed smoke logs (`wp13-d9-alloc-smoke/`) predate the rename and still say AK_ALLOC.

## 67. The allocator probe kept mapped, and the glibc pre-grow (owner decision 2026-10-03)

- Probe: once per process, its 16 MiB block never freed (java 2892e207b: a freed mmapped chunk
  raises glibc's dynamic threshold, so the probe changed the default mode it checks).
- Pre-grow: `Alloc.Startup(bytes)`, after the mode check and before any timing, both modes:
  malloc, touch each page, free, at the run's largest payload (RPC 16 MiB; codec the largest
  payload over every content set, 4,194,390 bytes), until a round has zero minor faults, cap 8
  (refuses). Host Main and the first GlobalSetup of each BDN child; rounds and last-round
  faults in the header and on every case's first row (the child's own in process mode).
- Defect found and fixed while smoking it: under the default toolchain every case aborted
  ("task-clock per iteration not recorded"). `CpuClock.ChildDir` was a static readonly read at
  class init; Startup in the host's Main now initialised the class before Main set
  AK_CPU_CHILD_DIR, so the host's ChildDir was null and no child's reads were paired. ChildDir
  is now read from the environment at each use. The minflt check also moved after the pairing
  checks, so a pairing failure reports itself rather than "minor faults not recorded".
- Smoke (`logs/csharp/wp13-pregrow-smoke/`, stripped): grouped, C-drop RPC unit, default 4
  rounds / probe mmapped, pinned 2 rounds / probe heap, 17 cases each, 0 failed; codec
  core-ffi:retain, default 3 rounds / mmapped, pinned 2 rounds / heap, 156 rows each, 0 failed;
  Bf under the default toolchain (default mode): 5 children, each probe mmapped and 4 rounds,
  10 rows. Last round 0 faults everywhere. Not exercised: the cap's refusal. No gate (owner).

## 68. The heap pre-grow reverted (owner decision 2026-10-03)

- Removed: the glibc pre-grow (JOURNAL 67), its header text, `pregrow_rounds` and
  `pregrow_last_minflt`, AK_PREGROW_BYTES and `Cases.LargestPayload`. Reason (owner): a
  pre-grow on one thread cannot reach the other threads' malloc arenas (C++ found the first
  benchmark still faulting through the core's worker threads); BDN's warm-up runs the real call
  path on every thread, and the per-row `minflt` shows whether it sufficed.
- Kept: AK_CAMPAIGN_ALLOC, the probe once per process with its block kept mapped, the refusal on
  a mismatch, `alloc_probe` on each case's first row, `alloc` and `minflt` on every row, and
  the ChildDir fix of JOURNAL 67 (independent of the pre-grow).
- Smoke (`logs/csharp/wp13-alloc-probe-smoke/`, stripped): C-drop grouped, default (probe
  mmapped) and pinned (heap), 17 cases each, 0 failed; Bf under the default toolchain, pinned,
  5 children each probing heap, 10 rows; codec core-ffi:retain, default, 156 rows, 0 failed.
  Every row has `minflt`, none a pre-grow field. No gate (owner).

## 69. Req 24 as amended (8c02e7c58): >= 20 calls per calling thread before the first measured value

- Checked against the campaign defaults (10 warm-up iterations, 100 ms), BDN child mode, C-drop
  at k = 8 (`logs/csharp/wp13-req24-warmup-count.log`, stage and op counts only). d/16 MiB at
  k = 8: jitting 1 invocation (8 calls), pilot 1 iteration of 4 invocations, warm-up 10 x 4
  invocations: 1 + 4 + 40 = 45 calls per caller thread. One invocation is k calls, one on each
  of the k caller threads (CallerPool.Run, n = k), and BDN never runs fewer than 4 invocations
  per iteration (its minimum invoke count: the pilot started at 4 even where one invocation
  exceeded the iteration time), so the warm-up alone gives every caller thread >= 40 on any
  machine. The caller threads are created in the case's process (the first GlobalSetup builds
  the context) before BDN's first stage and kept: same process, same threads.
- Met, default unchanged. Added the `# warm-up/thread` header line per unit (it states the
  bound from the run's warm-up count and flags a setting below 20). Async cells (A, D, F,
  callback/queue extras): >= 40 calls per async loop on the fixed pool of AK_WORKERS threads, but
  which pool thread runs a call is not controlled; stated, not changed.
- Housekeeping: my scratch dir for this check was named `w24` and collided with another agent's
  run in the shared scratchpad (their serve.state appeared in it, and my server's state file
  disappeared). I stopped my own server by its pid (18183, the one started with --tcp 0 at my
  start time) through a rebuilt state file, and did not touch theirs (18772).

## 70. Campaign duration estimate (computed, no benchmark run)

- Inputs: the req-24 check's BDN log (C-drop at k = 8, child mode, 10 x 100 ms warm-up, 1
  round): run time 24.9 s for 7 cases, of which 16.1 s are BDN iterations, so ~1.25 s per case
  for child start and setup; BDN's generated-project build 26.7 s per unit process; d/16 MiB at
  k = 8 runs 4 invocations (BDN's floor) of ~0.145 s per iteration. Codec: 336 cases in 17 to
  18 min in-process at the default job (`bdn-default-job-unit/`, before WP7), ~3.1 s per case;
  the codec child overhead is taken equal to the RPC one (not measured). Case counts from
  Cases.All() (22 payload-content sets, 92 U-* rows) and the units listed by both builds.
- Result: ~26 h per allocator pass (codec ~17.4 h, RPC ~8.1 h, calib and server warm-ups
  ~0.35 h), ~52 h for both passes, plus the two gates once (~0.75 h). Table in STATE. All
  container figures, for sizing only.

## 71. D18: the campaign grid (CAMPAIGN 4.0), and transport `armonik` (4.0 as amended b58543f7b)

- `AK_CAMPAIGN_GRID=core|full`, runner default core; the processes read it (unset = full, so
  the gate and every count file stay on the full grid). Codec core: 3 units in the full build
  and 2 in the no-unknown build, 49 cases each (17 encode-transport-hot, 18 decode-read, 7 U-*
  rows x 2). The U-* rows' encode at end state (ii) had no row before: byte identity of its
  frame is now checked in Cases.Verify (all arms, both builds), and the counting build over the
  core grid gives 42 rows identical to the gated count files and 7 new rows per build equal to
  the gated encode-hot rows of the same U-* rows (`logs/csharp/wp13-core-grid-counts/`). Those
  7 rows per build are not in the gate's committed files (open for the aggregating session).
- RPC core: A, Bf, Cf-retain, Ef-retain; the framed cells have no a+read (empty request), so
  their unit runs the reference cell's a+read beside them under its own name (B, C-retain,
  E-retain), whose crossing counts are already gated. h2-batch Cf-retain on c and d; the pinned
  allocator subset A and Cf-retain on c and d at k = 1, GLIBC_TUNABLES for those processes only.
- Transport `armonik` (owner addendum): cell A calls packages/csharp's
  GrpcChannelFactory.CreateChannel directly, built from `git archive HEAD packages/csharp
  Protos` into this slice (dotnet --artifacts-path; nothing under packages/ written, checked
  with git status). Observed: DisableDynamicWindowSizing not set (this slice used to set it in
  every process; now only for shipped/pinned); the system proxy bypasses loopback; TCP_NODELAY
  on by SocketsHttpHandler's default; SO_KEEPALIVE and SO_REUSEPORT 0 on the live socket (the
  ServicePoint settings do not reach .NET 8's handler); one HTTP/2 connection at k = 1 and 8.
  Readback after each case added (tcp_sockets_after and the three options on the first row).
- First smoke attempt at 3fd5702 was stopped by me when the addendum arrived (its rpc part would
  have used `shipped`); the smoke at ebbf1f6 from a fresh worktree: both gates passed (30
  controls each), codec 245 rows, rpc 40 rows, 0 failed. The background job reported exit 1
  after both suites had written rc=0 (the wrapper's last command; not a suite failure).
- Estimate (STATE): ~75 to 90 min timed for the core grid, plus ~45 min of gates; BDN's project
  build ~20 percent overall, ~40 percent of the RPC suite; fewer, larger BDN runs would save
  ~9.5 min without changing per-case isolation (not done).

## 72. Counts for the core-grid U-* rows, explicit send paths, BDN runs merged (owner, 2026-10-03)

- Counts: the gate's counting run (AK_CAMPAIGN_GRID unset) now also counts the core grid's 7
  U-* rows at end state (ii) per build (Cases.CountKeys); gen/counts.txt 1,051 rows,
  gen/counts-nounk.txt 551, the 7 added rows equal to the ones recorded in JOURNAL 71.
- Send path (Python's finding, confirmed here): the core's framed path is its default since
  e8fe14868 (2026-09-28), and this slice set the path only for the framed twins. So every
  REFERENCE core row ran framed from the first run on a core with that commit: in this slice's
  logs, the WP13 smokes and everything after (wp13-smoke, wp13-d9, pregrow, alloc-probe,
  req-24 count, core-smoke): B, C-*, E-*, Cc-*, the callback and queue rows on b, c, d (on a
  and a+read the request is empty). The earlier smokes (WP9, WP10) ran on a core before it.
  All container instrumentation; nothing timed in the campaign. Now every core channel sets its
  path explicitly: 0 on creation, 1 for Bf, Cf-*, Ef-* (Campaign.cs CoreCh and both twin sites).
  The crossing counts do not change (the send path is inside the core; the gate's counts equal).
- BDN merge: one run per codec build and per RPC run kind in the core grid (5 per launch, not
  12); the RPC host builds the case list without channels, each child builds only its case's
  cell, the row label is checked against what the child ran (plant: refused, 0 samples).
  Measured: RPC stock 237 -> 123 s, pinned 81 -> 43 s, codec 381 -> 228 s, same rows; about
  5 min per launch.
- A defect the merge measurement exposed: under the default toolchain the codec suite's U-*
  rows failed (DirectoryNotFoundException ffi/corpus/generated): the child runs from BDN's
  generated project under the artifacts directory, outside the repository when the artifacts
  are in a scratch directory (as the runner's are), and CorpusDir() walked up from there. The
  host now passes AK_CORPUS_DIR. It had not shown because the default-toolchain codec check of
  JOURNAL 64 ran a payload unit without U-* rows, and the runner's smokes are grouped. Gated
  behaviour unchanged (the gate sets no AK_CORPUS_DIR and finds the corpus as before).
- Gates at beae3d7 from a fresh worktree, both variants: PASSED, 30 controls each; counts equal
  (1,051 / 551, RPC 105 / 62). Core-grid smoke there (grouped): codec 245 rows, rpc 32 + 4 + 4,
  the same label sets and counts as the pre-merge smoke of JOURNAL 71.

## 73. The optimisation pass's short baseline; a one-CPU client measured tier-0 code (2026-10-04)

Container instrumentation throughout; nothing gated (gen/gate.sh not run); each codec process's
Cases.Verify on, every RPC call checked.

- **Toolchain.** The container had no .NET; `builds.dotnet.microsoft.com` (dotnet-install) is
  refused by the proxy (403, policy). Ubuntu noble-updates has the slice's exact versions:
  `apt-get install dotnet-sdk-8.0=8.0.131-0ubuntu1~24.04.1` (SDK 8.0.131, runtime 8.0.31). Cores
  built by gen/build_core.sh (last core commit fd69b0d6; target-core sha256 8932d205...,
  nounk 83768736..., h2b a6faa4d6...), the Rust server by poc/rust/serve.sh build.
- **Built:** `gen/opt_bench.sh` (the core grid exactly as run_campaign.sh runs it: default
  toolchain, merged runs; short BDN settings; not gated), `gen/opt_tables.py` (tables.md, codec.tsv,
  rpc.tsv), and `AK_BDN_MEMORY=1` (an exploration switch, off in the campaign): BDN's
  MemoryDiagnoser, one extra workload iteration after the actual stage, outside the job's clock;
  `mem_alloc_bytes_per_op`, `mem_gen`, `mem_ops` on each case's first row, both suites.
  Checked in child mode on host-gen:retain (smoke): fields present, plausible (encode-transport-hot
  0 B on most rows, decode-read the decoded graph's size).
- **First run VOID for the managed arms** (`logs/csharp/opt/baseline-1cpu-VOID/`, kept, marked in
  its header, jsonl and tables.md): client CLIENT=1 (one CPU), codec 4 x 40 ms warm-up. The
  aggregating session pointed out (from the .NET 8 source: TC_CallCountingDelayMs 100 ms x
  TC_DelaySingleProcMultiplier 10 on a one-CPU affinity mask) that tier-up then comes ~1 s after
  the last tier-0 JIT, later than each child measures. Confirmed (`logs/csharp/opt/tier-check/`,
  gen/tier_check.sh, gen/tier_table.py; full build, P1.1 and P6.1, encode-transport-hot and
  decode-read, three units): on 1 CPU every managed row is flat and 3 to 7 times slower than with
  DOTNET_TC_CallCountingDelayMs=0 or DOTNET_TieredCompilation=0 in the same pinning (e.g.
  incumbent-prod P6.1 encode 3.08-3.13 ms against 0.45-0.51 ms; host-gen 0.88 against 0.13-0.15 ms;
  P1.1 encode incumbent 5.1 against 1.24-1.31 us). The core-ffi encode rows (native work) barely
  move; core-ffi decode-read (managed facade fill) does.
- **Two CPUs alone did not fix it at that warm-up.** On CPUs 0,1 and on 2,3 with 4 x 40 ms, rows
  tier up DURING the actual stage (e.g. incumbent P1.1 encode 8.7, 13.5, 11.7, 2.0, 1.8, 1.3 us by
  round; host-gen P6.1 decode-read 1.8 ms then 1.1 ms), so the first fix tried (more CPUs, nothing
  else) leaves mixed-tier samples. Warm-up 10 x 40 ms: still partly tier 0 (incumbent P6.1 encode
  0.87 ms). Warm-up 10 x 100 ms (the campaign's) and 25 x 40 ms: settled on all 12 rows, matching
  the delay-0 control within the run's spread. So the 2-CPU baseline runs 25 x 40 ms (codec) and
  10 x 100 ms (RPC, the campaign's), and is longer than the 5-10 min asked (stated).
- **The slice's JIT check missed it.** In grouped mode (InProcessEmit) on one CPU the measured rows
  were tier-0 speed, yet `jit check: PASS` (all 6 runs). The check counts methods compiled IN the
  case's span that are promoted later; code first compiled before the case (Cases.Verify runs every
  arm before BDN starts) and never promoted while the process lives is not seen. Open defect D46,
  not fixed in this unit. Under the default toolchain there is no tier readback at all (JOURNAL 64).
- **Guard (CpuGuard, CpuClock.cs):** every timed process (BDN host and each child, codec and RPC,
  at Alloc.Startup) reads its affinity mask (sched_getaffinity) and refuses ONE CPU unless
  AK_ALLOW_SINGLE_CPU=1; the header states it, each case's first row carries `cpus_affinity`,
  `cpus_runtime` (Environment.ProcessorCount) and `single_cpu_override` as its own process saw
  them. Control: `1cpu-guard` in tier-check, refused (rc 134, no rows). run_campaign.sh's smoke
  defaults moved from client 0 / server 1 to client 0,1 / server 2,3 (the guard would refuse the
  old default); the campaign machine's sets are 8 CPUs.
- One line: a `pkill -f VBCSCompiler` in the same command line as the run matched its own shell
  and killed the first tier-check launch (exit 144); rerun without it.
- **The 2-CPU baseline** (`logs/csharp/opt/baseline/`, commit 24a9294, client 0,1, server 2,3):
  codec 245 cases + 4 primes, RPC 40 cases, 0 failed; every case's process saw 2 CPUs (affinity and
  ProcessorCount). Benchmark wall 745 s (codec 474, RPC 271) plus build 24 s and server warm-up
  36 s (500 calls per direction, campaign 2000). The settled tier-check rows agree with it within
  the run's spread (e.g. incumbent-prod P1.1 encode 1.31 us, P6.1 encode 0.48 ms). Flagged, not
  interpreted: (1) many rows carry one slow round (` *` in tables.md; CPU 0 is shared with the
  container's other processes); (2) core-ffi retain P5.4 decode-read 1.05 ms with 0 GC in its
  MemoryDiagnoser iteration, against 1.75-2.09 ms and gen2 collections in the four other units
  (the same 4,194,600 B allocated); (3) the RPC client CPU per call moved between the 1-CPU and
  2-CPU runs in both directions, mostly up on 2 CPUs (A b k=1 7.3 -> 16.2 ms task-clock; Cf-retain
  c k=1 3.1 -> 4.3 ms; Ef-retain b k=1 3.0 -> 1.6 ms), with task-clock above wall for A; (4) cell A's
  tier state is not checked (no RPC tier check was run; its warm-up is the campaign's 10 x 100 ms).

## 74. Optimisation pass, steps 1 to 4 (owner decisions D1, a1, a2, D7; 2026-10-04)

Container instrumentation throughout; no gate (owner), each step's net8.0 quick checks
(`AK_GATE_LEVELS=8 AK_GATE_KEEP_CORE=1 gen/gate.sh`: every net8 check and control of the gate,
counts included, no net6.0 floor, no net48, the cores not rebuilt; the last line says "NOT the
gate"). Narrowed A/B in one session, variants alternated by rep (`gen/opt_ab.sh`,
`gen/opt_ab_rpc.sh`), each variant built and run from its own tree (a git worktree at the
previous step's commit), the baseline's settings (client CPUs 0,1, codec 25 x 40 ms warm-up, 6 x
40 ms rounds; RPC 10 x 100 ms, 6 x 100 ms), MemoryDiagnoser on, drop units added
(`AK_BDN_DROP=1`, a labelled extra) beside retain and no-unknown.

- **A harness defect found first (fixed, 2610f847).** Under BDN's default toolchain every case's
  child is REBUILT from the project, and the job did not pass `/p:AkNounk=true`: every
  no-unknown "core-ffi:no-unknown" / "host-gen:no-unknown" row timed so far under the default
  toolchain (the 2-CPU baseline of JOURNAL 73 included) ran the FULL build's code in its child
  (the host, which writes the header and checks, was the no-unknown build). Found because the
  new per-process build check refused it: every process (host and child) now checks its build
  and its core against the host's (`BuildCheck`, AK_BDN_BUILD), and the no-unknown job passes
  the property (both suites). The baseline's no-unknown columns are therefore the full build's
  code with drop semantics. The figures of steps 1 to 4 and of the final run are after the fix.
- **Step 1 (D1).** The core grid's core-ffi and host-gen encode rows are now `encode-core-hot`
  (end state ii for the core's transport, `enc_end` transport-core): core-ffi `EncodeInto` (the
  encode left in the core's context, no take, no frame, as Cf hands it to ak_call_unary_enc);
  host-gen its Enc (64 KiB initial, as Ef). The Grpc.Net frame form stays a labelled extra
  (`encode-transport-hot`, full grid or AK_BDN_DIRS). Byte identity of the new form checked in
  Cases.Verify (both builds, every payload and U-* row; `ContextBytes` reads the context without
  consuming it). Counts: 95 + 7 rows added to gen/counts.txt, 51 to counts-nounk.txt (no existing
  row changed; core-ffi encode-core-hot = fwd 3, one fewer than the take form). A/B
  (`logs/csharp/opt/s1/ab/`, both forms in one process, 2 reps): see the report's table; e.g.
  core-ffi retain P5.4 0.55 ms -> 0.30 ms, P5.2 3.34 -> 1.69 us, P1.1 0.79 -> 0.71 us.
- **Step 2 (a1, cs_host.py Stage).** Blocks kept across encodes, new block max(need, 2 x the
  largest), a string reserves its maximum and commits what it wrote; pointers valid until Reset.
  Checks passed (`s2/checks.log`). A/B (`s2/ab/`, `s2/ab-rpc/`): core-ffi retain P2.4 1.54 ms
  and 408 minor faults per op -> 0.78 ms and 0; P2.3 retain (one rep 1.2 ms / 170 faults) ->
  0.60 ms / 0; the other rows within the run's spread; RPC Cf-retain b at k 8 2.43 -> 2.04 ms
  task-clock (k 1 within spread).
- **Step 3 (a2).** (i) The push decode drops ak_dec_err_reset and ak_dec_err: ak_decode_* clears
  hdr.err on entry and returns it when set (codec.rs, every root from one template; read before
  the change); new check `harness hostfail` (gate, both builds): a planted throw in every apply
  (or every add/new) callback comes back as AK_ERR_HOST from the return value alone, 32/32 (24/32
  with add: the shapes without one succeed), 16/16 and 12/16 no-unknown. (ii) one GCHandle per
  instance, Target set per decode. (iii) the options rewritten only when the mode changes (the
  core never writes a grow-only entry). (iv) the retain buffers from a per-context native arena
  (UnkArena: 64 KiB then geometric chunks kept across decodes, a grow of the last allocation in
  place, an outstanding count for UNDELIVERED); the skipped-release control still fails
  (UNDELIVERED), the decision-11 controls unchanged. Counts: every core-ffi decode row fwd -2
  (exactly ak_dec_err and ak_dec_err_reset), rev/grow/reset unchanged, 640 + 320 codec rows and
  8 + 4 RPC rows regenerated. A first run of the step-3 checks was contaminated by step-4 code
  edited in the same tree while it ran (two akrpc build failures); discarded, the step-4 work
  moved to its own worktree, the checks re-run clean (`s3/checks.log`). A/B (`s3/ab/`): core-ffi
  retain U-* decode 0.42 -> 0.29 us (UploadResultData), 0.49 -> 0.37 (oneof), 0.99 -> 0.80
  (nested), 0.69 -> 0.55 (Dual); drop and no-unknown 0.02 to 0.06 us lower (P5.1 0.25 -> 0.23);
  host-gen (unchanged code) within spread.
- **Step 4 (D7).** `src/Rpc/Deliveries.cs`: <cell>.callback (TCS with
  RunContinuationsAsynchronously), <cell>.callback-inline (TCS inline: the continuation runs on
  the core's tokio worker), <cell>.queue (one drainer, blocking ak_queue_next with a 1 s timeout,
  one completion per pop: no batch pop in the ABI), for Bf, Cf-retain, Ef-retain (Cf-nounk,
  Ef-nounk in the no-unknown build); d with ak_call_send(_enc)_cb/_q, each send awaited, then
  ak_call_recv_cb/_q. `akrpc --delivery-semantics` (in the gate): 9 cases (unary ok, non-OK 13,
  cancel; unary_enc ok, 13; stream ok, 13, cancel, send_enc ok) x 4 deliveries, each equal to
  the blocking one (the unary cancel, which the blocking unary cannot express, equal across the
  three async ones: CANCELLED): 0 failures. Observed while writing it: an inline continuation that
  reaches a BLOCKING core entry (here the next channel's ak_client_new_opts) panics the tokio
  runtime ("Cannot start a runtime from within a runtime") and aborts the process; the timed
  inline cells only start non-blocking calls. Delivery-cell crossing counts committed
  (gen/rpc-delivery-counts*.txt; the idle drainers of other cells and the first-use context
  creation on a pool thread are filtered by name, the queue cell's own pops counted). Per
  iteration the RPC rows now carry `csw` (voluntary + involuntary context switches of the
  process, getrusage, read with the minor faults).
- **Step 4 measured** (`logs/csharp/opt/s4/deliveries/`, one BDN run of 12 units, blocking and
  the three deliveries for Bf, Cf-retain, Ef-retain on a+read, b, c P5.4, d 16 MiB at k 1 and 8,
  ONE rep, 576 s): the spreads are wide (container, one rep) and no delivery is clear of the
  blocking one's spread on every row; context switches per call 2 to 10 times the blocking
  cells' (e.g. Cf-retain b k 1: 12 blocking, 77 callback, 152 inline, 52 queue); allocations per
  call +1.0 to 1.4 KB per completion for the async deliveries (TCS, Pending, GCHandle), except
  one anomaly flagged: Bf.callback d k 1 at 528 KB per call (k 8: 3.2 KB; not investigated).
- **Final combined run** (`logs/csharp/opt/s1-s4/`, gen/opt_bench.sh OPT_DROP=1 at the step-4
  commit + the tables fix): codec full 453 s, no-unknown 211 s, RPC stock (with Cf-drop, Ef-drop)
  296 s, h2-batch 56 s, pinned 44 s, no-unknown client 108 s: 1,168 s of benchmark wall plus build
  13 s and server warm-up 46 s; every case's process saw 2 CPUs; 0 failed. Many rows carry one
  slow round (` *`), more than in the baseline run.

## 75. Steps 5 (D20), 5b (static decode vtable) and 6 (D21, the string encode paths); the quiet decode re-measure (2026-10-04)

Every figure below is container instrumentation: process CPU per op, median over 2 reps x 6
rounds [min-max], client on CPUs 0,1, a quiet wait before each process (load1 < 0.5, no other
process above 10 % CPU). Logs under `ffi/logs/csharp/opt/`.

- **Quiet decode re-measure** (`s1-s4-decode-quiet/`, `9488342d`, no code change): decode-read
  after steps 1 to 4 with the cores rebuilt from HEAD (D19 in), sha256 in the header; the
  numbers there replace the decode-read columns of `s1-s4/` for that comparison.
- **Step 5 (D20).** Tried: every utf8_skip bit set (push vtables, pull via
  ak_dec_set_pvt_<Root>, called in EnsureDec) and G.Str strict (UTF8Encoding(false, true)),
  DecoderFallbackException mapped to AK_ERR_TRANSCODE at the four callback catch sites and in
  the pull replay. Cores rebuilt (the ones at 9488342d were pre-D20). The malformed-UTF-8 rows
  stay rejected with -6 through the host's check; the planted lossy decoder makes the T-dec-*
  rows pass and the corpus fail (the check is live). First check run failed on the delivery
  counts: ak_dec_set_pvt_ appeared once in a delivery cell (an inline continuation's first
  context on a new thread); filtered by name with the other first-use calls (`a79c14be`).
  Measured (`s5/ab/table.md`, before = bits 0 + lossy GetString, after = bits all + strict):
  string-dense decode rows 10 to 15 % lower, e.g. P2.2 retain 2637 -> 2239 us, drop 2594 ->
  2200, no-unknown 2245 -> 1970; P4.1 393 -> 333; P2.3, P2.4, P2.5 similar. P5.2 to P5.4 are
  bimodal across reps (spread larger than any shift); P6.1 (no strings) 332 -> 357 us drop,
  inside its spread but flagged; P1.3 (the absent path) no-unknown 18.1 -> 20.0 us, flagged;
  P1.2, P2.2/latin1 and P2.4 no-unknown each carry one slow rep that lifts the median.
- **Step 5b.** The per-decode `var vt = new ak_dvt_<Root>{...}` and the pvt moved to
  NativeMemory once per root (`static readonly` pointers). Why not a static struct: a static
  field of struct type is stored in a boxed object on the GC heap, which compaction may move;
  the core keeps the pvt pointer across calls. Counts unchanged. Measured (`s5b/ab/table.md`,
  P1.1, P5.1, P7.1, the 7 U-* rows, retain/drop/no-unknown): every row inside the overlap of
  the two variants' spreads (e.g. P5.1 retain 0.224 -> 0.233 us, no-unknown 0.198 -> 0.193).
- **Step 6 (D21).** Built: AK_STR_ENC = E0 | E1 | E2 | ETH:<n> (STATE, Optimisation pass, item
  6). Defect found on the way: E2's first form set ak_str.data = index + 1, so the first string
  had data == 1 == AK_STR_DIRECT and the core treated it as a direct string (0 bytes written);
  Cases.Verify caught it; data is now 0x10000 + index (ABI v1 section 8 reserves small values).
  Liveness of the paths: the corpus under E1 and E2 with a planted short string fails 424 rows
  each (`s6/corpus-strpaths.log`); in the A/B, E1's and E2's columns differ from E0's on every
  row (a path that was not running would measure equal).
  - **Sweep** (`s6/sweep/sweep.md`, 4..16 Ki; `s6/sweep-fine/sweep.md`, 64..512): one process,
    one UploadResultDataMessage with one string, paths interleaved and rotated. Pin alone is
    about 40 ns (38 to 51). E1 is under E0 for wide content from 96 code units (209 vs 196 ns)
    and for Latin-1 from 224 (242 vs 218; at 192 they tie, 206 vs 208); at 256 E1 is under E0
    on Latin-1 and wide in both sweep processes (255/249 vs 234/229; 438/437 vs 274/283). On
    ASCII E1 is above E0 at every length (16 Ki: 1145 vs 1171), on astral content too (16 Ki:
    24551 vs 26341; 256: 434 vs 538). E2 sits 0 to about 40 ns above E0 up to 1 Ki and below
    E0 at 16 Ki on every content (ASCII 949 vs 1145, astral 23427 vs 24551).
  - **Threshold read from it: 256 code units**, the smallest length at which E1 was under E0
    for both Latin-1 and wide content in both sweep processes (224 was under in the one fine
    process only). It is a length test: above it ASCII strings cost about 50 ns more (E1 vs E0
    at 256 to 1 Ki) and astral strings about 20 to 25 % more, and below it wide strings of 96
    to 255 units keep E0's higher cost. What a content-aware split would give is not measured.
  - **Census** (`s6/strlen-census.txt`): every string of the step-6 grid rows is under 48 code
    units (GUIDs and short ids); ETH:256 pins 0 strings on every row of both builds. So the
    grid's ETH column measures the length test only, and its E1 column measures E1 on short
    strings, the region the sweep already puts above E0.
  - **Codec A/B** (`s6/ab/table.md`, `compact.md`; P1.2, P2.2 + Latin-1 + wide, P2.3, P2.4,
    P2.5, P4.1, the 7 U-* rows; core-ffi encode-core-hot, retain/drop/no-unknown; 16 processes,
    1,223 s of benchmark plus the quiet waits): E0 vs E1 vs E2 vs ETH:256, e.g. P2.2 retain 869
    [860-902] / 2941 [2912-3150] / 1282 [1215-1497] / 906 [885-944] us; P1.2 retain 214 / 794 /
    318 / 218; P2.4 retain 896 / 4327 / 1249 / 851; U-wire-UploadResultData retain 0.125 /
    0.244 / 0.158 / 0.117. Per string (median differences / strings per encode): E1 about +100 to
    +136 ns on the large payloads and +55 to +84 on the U-* rows; E2 +13 to +29 ns. ETH within
    E0's spread on every row. E1 also shows minor faults on the large payloads (0.1 to 9 per
    op, 0 for the others) and 0 B/op managed (GCHandles are not GC allocations).
  - **Not explained:** E1 at payload scale (thousands of strings pinned at once, freed after
    the call) costs 2.5 to 3 times the sweep's one-string figure (about 45 ns over E0 at 36
    code units); the minor faults point at the GC handle table, not investigated.
  - **RPC A/B** (`s6/ab-rpc/table.md`, Cf-retain b, P2.2, 2 reps, 220 s): task-clock us/call
    E0 2825 [2186-5865] / ETH:256 3597 [2050-6288] / E1 6136 [5410-14560] at k 1; at k 8 2753
    [2138-4366] / 2857 [2118-4122] / 7495 [6542-9398]. ETH's spread covers E0's at both k
    (it pins nothing on P2.2); E1 2.2 to 2.7 times E0, with 4 minor faults per call and, at
    k 8, 81 context switches per call against 32.
  - Refuted: that E1 helps the grid's encode rows (all strings are short); that E2's reverse
    call is free (13 to 29 ns per string on the grid, about 30 ns in the sweep).
  - Time: the codec A/B ran over the 10-minute target (four variants x two builds x two reps,
    about 100 s per full-build process); the quiet waits added about 20 min.

## 76. Step 7 (D21 continued): E3, E1 without GCHandles (E1R, E1C), the kernels, the attribution, the threshold (2026-10-04/05)

Every figure is container instrumentation (process CPU, client CPUs 0,1, a quiet wait per
process). Logs under `ffi/logs/csharp/opt/s7/`.

- **Built** (cs_host.py Stage and per-root frames; `AK_STR_ENC`):
  - **E3**: E2's table and callback, the callback `fixed`s the string and calls the core's
    additive `ak_utf16_to_utf8` (declared in the C# Stage only, DllImport), after a grow to the
    worst case (3 bytes per unit) when `cap` is short. **E3L**: sized by `ak_utf16_utf8_len`
    first. Both cross once per string (counts `tc N`, `u16 N`, `u16len N`).
  - **E1R** (owner's precision): the fill marks a string (`data = PinPending`, `tc =
    ak_tc_utf16`); the root group's strings are pinned by ONE nested `fixed` scope around the
    root call (every singular string of the group and of its inlined singular and oneof
    children, as the fill assigns them); an element chunk is pinned by BOUNDED RECURSION, one
    frame per ELEMENT (one `fixed` with every singular string of the element and its inlined
    children), the deepest frame makes the element call for the chunk (K = AK_STR_PINK
    elements, default 64), unwinding releases the pins; repeated string fields and maps nested
    in an element: one frame per string / per entry, chunks of at most K, delivered as several
    `ak_blob_run` / `ak_elem_<Entry>` calls (as the ABI allows); a map at the root's top level
    takes E1's GCHandle (`hpin`, 0 on every grid row). **E1C**: the same marks and chunks,
    pinned by GCHandles freed after each chunk's call (the attribution control).
  - `<MODE>:<n>[:na]`: the mode for strings of at least n code units, E0 below; `na` sends an
    ASCII string (System.Text.Ascii.IsValid) to E0.
  - Defects found on the way: (1) the first marks/patches check compared counters read before
    `_st.Reset()`; corrected (the patch check is per encode). (2) The first step-7 build made
    the DEFAULT encode slower: the sweep's E0 12 to 17 ns per encode above 1818d178 (s7/e0-
    overhead, r1/r2); the root `fixed` scope and its pinned locals had landed in Go, and
    StrPresent's new branches in the inlined E0 path. Moved to RootPinR_/H_ methods, Stage.Defer
    read once, the other paths in Alt (NoInlining); after it E0 is within the processes' spread
    of 1818d178 (sweep r7..r9; the codec A/B's e0pre vs e0, below). (3) A stale corpus build was
    checked once (the corpus no-unknown runs failed with a TypeInitializationException: the
    build predated a Stage change); gen/s7_checks.sh now rebuilds every binary it runs. (4)
    E1R's frame size first went into the compared count rows; it depends on the JIT tier, so
    it is written apart (`s7/frames/`).
  - Per-encode skip (after the first A/B): an encode whose fill marked no string runs no
    frame (Stage.DeferNow), so a threshold variant takes the default path on encodes with only
    short strings; the threshold's length test inline before the Alt call (the first final-
    variant A/B, s7/ab-final, measured the call per string: E1R:128 +0 to +21 % over E0 on rows
    where no string reaches 128; after it, s7/ab-final2, -11 to +4 %).
  - (5) Found by the RPC A/B, not by any single-threaded check: E1R failed at k = 8 (`core
    encode -1`): Stage.Marked/Patched were process-wide, so concurrent encodes broke the
    per-encode mark check; now [ThreadStatic]. (6) Found by the new concurrent check: E1C
    released ALL of the thread's handles after a NESTED chunk (a map or a repeated field inside
    an element call), unpinning the outer chunk's strings while the core still read them
    (bytes differ in 2 of about 7,000 concurrent encodes; single-threaded under a compacting GC
    per chunk: bytes differ then a segfault); a chunk now releases only the handles it added.
    New `BenchDotNet --verify-mt` (8 threads, every payload and content set, retain and drop):
    reproduced (5) for E1R and E1C and (6) for E1C before the fixes, every path passes after
    (s7/verify-mt.log); in gen/s7_checks.sh with a pin-stress run and an early-unpin control.
    The s7/ab E1C column was measured with defect (6); s7/ab-fixed re-measures E1C, E1R and
    E1R:128 on the corrected build.
- **Checks** (`s7/checks.log`, `s7/checks2.log`, gen/s7_checks.sh; `s7/quick-checks.log`):
  Cases.Verify byte identity under E0 E1 E2 ETH:16 E3 E3L E1R E1R(K 3) E1C(K 3) E1R:16 E1R:16:na
  E3:16:na E1R:128 E1C:16(K 3) (4516 / 3304 checks, both builds); the corpus under E3, E3L,
  E1R, E1C, E1R:128, full and no-unknown; E1R and E1C with K 3 under a compacting GC after
  every chunk (AK_GATE_PIN_STRESS); planted controls fail: a short string under E3, E3L, E1R,
  E1C (corpus S-*), E1C with its handles released BEFORE the call plus a compacting GC (so the
  stress check can see a string read after its release), K 257 refused; counts per path equal
  to the committed files; gate levels 8 passed (E0 counts unchanged).
- **Counts** (gen/counts-str-{e3,e3l,e1r,e1c,e1r128}[-nounk].txt): E3/E3L = base rows + `tc`
  (and `u16`, `u16len`: additive exports, not in fwd); E1R/E1C: `mark = patch` on every row,
  fwd + the chunked element and blob calls (P2.4 403 -> 724, P2.2 2503 -> 2510, P1.2 3 -> 18);
  E1R:128 = the base rows (no grid string reaches 128). Repeated and map strings per encode
  (`rstr`, `mstr`): P2.2 6000 and 4000 of 17167, P2.3 15000 and 1000 of 17792, P2.4 24480 and
  640 of 26267, P2.5 240 and 120 of 640, P4.1 0 and 1600 of 3467, P1.2 0 of 5000;
  U-deep-u-repeated 8 and 8 of 31, U-wire-ListTaskSummary 0 and 16 of 36.
- **Stack** (`s7/frames/`): the largest E1R frame over the shapes and U-* rows is 1,152 bytes
  (P2.1, default tiers; 336 fully optimised). K is refused above 256: 2 x 256 x 1,152 = 590 KB,
  38 % of a 1.5 MB secondary-thread stack.
- **Kernels** (`s7/bench/cpu.txt`, `simdutf-probe.txt`, `kernels-probe.log`): the CPU is an
  Intel Xeon family 6 model 85 stepping 7 (Cascade Lake class): avx512f/dq/cd/bw/vl/vnni, NO
  avx512vbmi or vbmi2. simdutf 7.7.1 (the amalgamation the core links, probe built with the cc
  crate's flags) has its icelake kernel compiled in (105 symbols in libak_core.so) but it
  needs VBMI2: active = **haswell (AVX2)**. .NET 8.0.31: Avx512F/BW IsSupported = true but
  **Vector512.IsHardwareAccelerated = false** (Vector<byte>.Count 32): .NET also runs 256-bit
  by default here. The hypothesis "E1 does not beat .NET on ASCII because .NET uses AVX-512
  while simdutf picked AVX2" is **refuted**: both run 256-bit. Switches (s7/bench/k-*.md, ns per
  16 Ki-unit encode, E0 / E1 / E3): default ASCII 1127 / 1187 / 1152; DOTNET_PreferredVectorBitWidth
  =512 (Vector512 accelerated) E0 1302 (slower); DOTNET_EnableAVX512F=0 E0 1056 (no slower);
  SIMDUTF westmere (SSE4.2) E1 1475 on ASCII, 6373 on Latin-1 (haswell 5603), 7153 on wide
  (haswell 7713); fallback E1 5227 / 21119 / 31623. On ASCII both are a vectorised narrowing
  copy and end level; on Latin-1 and wide simdutf's AVX2 kernel is 2.4 to 3.1 times .NET's
  transcoder at 16 Ki.
- **E3 sizing**: the worst-case grow (E3) is at or under the length-first form (E3L) at every
  length and content (ASCII 16 Ki 1166 vs 3801 ns; E3L converts by validate + length +
  convert when cap < 3 x len). E3 ~ E1 at long lengths (Latin-1 16 Ki 5689 vs 5595, wide 7786
  vs 7698) and 0 to 20 ns above E1 short.
- **Pin microbench** (`s7/bench/pinbench.md`, 36-char strings, ns per string): GCHandle
  Alloc+Free with N live: 44 (N <= 64), 76 (256), 85 (1 Ki), 89 (5000), 93 (17167), 95 (26267);
  per chunk of 64: 44 to 47 at every N; one `fixed` frame per string, 64 deep: 7.8. No minor
  fault or collection in the microbench.
- **Codec A/B** (`s7/ab/` and, on the corrected build, `s7/ab-fixed/`; table.md and
  compact.md each; s7/ab 24 processes, 1,902 s of benchmark + 1,400 s of quiet waits, taken
  before the per-encode skip and the fixes, which do not change its E0, E1 and E3 rows;
  s7/ab-fixed 16 processes, 1,284 s + 1,110 s). E0 at the step-7 build vs 1818d178 (e0pre,
  s7/ab): within spread on every row (P2.2 retain 890 [865-1281] vs 858 [845-1406] us). Per
  string over E0, the large payloads (P1.2 to P4.1, every mode): **E1 +107 to +135 ns, E1C
  +77 to +111, E3 +36 to +48, E1R +26 to +46** (E1 and E3 from s7/ab, E1C and E1R from
  s7/ab-fixed; E1R carries its per-encode mark/patch guard, two thread-static increments per
  string); U-* rows (2 to 36 strings): E1 +58 to +92, E1C +66 to +109, E3 +38 to +62, E1R +23
  to +52, with two E1R outliers (U-wire-DualResponse retain 174, ListMetrics retain 199 ns per
  string: both reps; not investigated). E.g. P2.2 retain: E0 889, E1 3154 (s7/ab), E1C 2458,
  E1R 1506, E3 1610 (s7/ab) us; P2.4 drop: E0 793, E1 4101, E1C 3003, E1R 1925, E3 1804.
  Minor faults per op: E1 2.7 to 7.1 on the P2.x rows, E1C 0.25 to 0.75, E0/E1R/E3 0. 0 B/op
  allocated on every variant (no collection from them).
- **Attribution of E1's payload-scale excess** (the grid's strings are 36 to 47 units, where
  the sweep puts E1 about 45 ns over E0). Inside s7/ab (one run, so the columns compare): E1 ->
  E1C (handles freed per chunk) recovers 10 to 15 ns per string on P1.2 (5,000 strings), 18 to
  36 on P2.2, 48 to 52 on P2.3, 40 to 43 on P2.4: the live-handle growth (the microbench: 89 -
  46 = 43 ns at 5,000 live, 47 at 17,167, 49 at 26,267) and the faults (E1 2.7 to 7 per op,
  E1C under 1); E1C -> E1R recovers about 35 to 85 ns per string (s7/ab 42 to 84, s7/ab-fixed
  35 to 80): the GCHandle Alloc+Free itself (44 to 47 ns in the microbench) and E1C's
  per-chunk bookkeeping; what is left, E1R over E0, 26 to 46 ns per string on the corrected
  build: the core's UTF-16 transcoder against .NET's at these lengths (sweep: about +0 to
  +10), the per-element frame (7.8 ns in the microbench), the guard's two thread-static
  increments, the patch tests, and the extra element calls. Not split further.
- **Threshold** (`s7/threshold/sweep.md`, `sweep2.md`, one process each, one string per
  encode): E1R vs E0 (sweep2, after the skip): wide under E0 from 96 units (189 vs 216), Latin-1
  from 192 (202 vs 235; 128: 185 vs 176); ASCII above E0 by 41 to 51 ns from 128 to 1 Ki, equal
  at 16 Ki (1126 vs 1125 for E1R:128); `tail` (ASCII but the last unit) the same; astral above
  E0 at every length (+4 to +7 %). The ASCII split (`na`) keeps ASCII strings on E0 for +11 to
  +21 ns at 128 to 1 Ki (vs +41 to +51 without it) but costs +340 ns at 16 Ki (the scan of the
  whole string) and +2 to +16 ns on non-ASCII strings. **Chosen: E1R:128**: the smallest swept
  length at which E1R is under E0 on wide content and within 10 ns of it on Latin-1 in both
  threshold processes (sweep 1: 177 vs 173; sweep2: 185 vs 176); no ASCII split (it loses at
  long ASCII lengths and does not separate astral content, which E1R loses at every length).
  Measured as the final variant: codec rows (s7/ab-fixed, where no string reaches 128 and no
  frame runs) E1R:128 vs E0 -6.6 to +12.8 % (median about +1 %); RPC Cf-retain b on P2.2
  (`s7/ab-rpc/table.md`, 2 reps, 163 s), task-clock us per call, E0 / E1R:128 / E1R: k 1 1744
  [1536-2469] / 1713 [1491-3965] / 2271 [2105-6230]; k 8 1787 [1646-3107] / 1891 [1549-3681] /
  2976 [2159-3919]. E1R:128 inside E0's spread at both k; E1R 1.3 to 1.7 times E0. A first RPC
  run failed on E1R at k 8 (defect 5; kept as s7/ab-rpc-FAILED-E1R).
- **Interruptions**: the container restarted during the first codec A/B (kept as
  s7/ab-INTERRUPTED, not used); opt_ab.sh gained --first-rep (one invocation per rep) and the
  cores' sha256 in its header.

## 77. s8: attribution of the core-ffi decode (owner, 2026-10-08; no optimisation)

Container instrumentation; process CPU per op, `BenchDotNet --decattr`: ONE process per build
and rep, every arm interleaved and rotated per round (6 x 40 ms after >= 1 s of warm-up per row
and mode), 2 reps per build, a quiet wait per process (load1 < 0.5, no process above 10 %).
Logs `ffi/logs/csharp/opt/s8/` (tables.md, *.census.txt, counts-*.txt; the first run, without
hskip and the GC pause, kept as run1-superseded/, same figures within spread). 703 s of
benchmark for both runs (343 + 360) plus the quiet waits.

- **Arms** (harness only, labelled; nothing on a product path changes): ref (HEAD's core-ffi push
  decode-read), skip (G.SkipStrings), noop (VtNoop: every callback returns at once, new_ returns
  token 0), parse (VtParse: every callback null but new_, which the core needs: a null `new_`
  makes the core SKIP the element's body, codec.rs `None => return`, so "every callback null"
  would not parse the non-leaf elements), parsev (parse with utf8_skip 0), pull (core-ffi-pull,
  drop form), pparse (ak_parse_<Root> alone), touch (the read pass alone), strs (the graph's
  strings alone, Strict.GetString over their UTF-8), host, hskip (host-gen with the new
  Dec.SkipStrings twin), inc (incumbent-prod). Generated seams: VtParse/VtNoop/VtParseValidate,
  DecodeVt, ParseOnly (cs_host.py); RootOps DecFfiVt/DecFfiParse/DecFfiGraph/DecIncGraph/TouchF
  (cs_campaign.py). One change on a product path: host-gen's Dec.StrReject (Facade/Wire.cs:460) gained the
  `|| SkipStrings` test, one static read per string (core-ffi's G.Str has had its twin since
  WP6); not re-measured on its own. Deviation: "strings only" is the strings ALONE (no decode around them);
  the strings bucket is ref - skip, as specified, and strs is its cross-check.
- **Counts** (s8/counts-*.txt, counting build): per decode fwd 2 (reset + decode) on every
  push arm; rev ref = noop (P2.2 3,501; P2.4 561; P4.1 601; P1.2 8), parse = the non-leaf
  elements only (P2.2 500, P2.4 80, P4.1 200, P1.2 0), pparse 0; pull fwd 3 rev 0; grow per retained U-* row.
- **Census** (s8/*.census.txt): the facade graph equals the incumbent's in objects, strings,
  lists, maps on every row (P2.2: 7,835 objects, 17,167 strings / 412,032 UTF-16 units, 2,001
  non-empty lists, 500 maps, 8,500 elements; P2.4: 1,255 objects, 26,267 strings / 912,556
  units; P1.2: 3,001 objects, 5,000 strings, 1,000 byte[]). ref allocates what host-gen does
  (P2.2 2.18 MB/op; skip 0.81 MB; strs alone 1.25 MB); noop/parse/pparse allocate 0.
- **Split** (tables.md section 2, medians, us per op; P2.2 ascii retain / drop / no-unknown):
  ref 2220 / 2054 / 1920; parse 335 / 331 / 317 (15-16 %); cross (noop - parse) 26 / 25 / 19
  (1 %); build (skip - noop) 609 / 583 / 515 (27-28 %); strings (ref - skip) 1250 / 1114 / 1069
  (54-56 %); strings alone 647 / 652 / 639; GC pause per op ref 367 / 217 / 171 vs skip 41 / 36
  / 35; touch 61-65 (in build); validation if the core did it (parsev - parse) 312-327. Pull:
  pparse 337-346 (= parse), replay 1641-1911 (= cross + build + strings). Host-gen 2028 / 1874 /
  1905: hskip (parse + build) 799 / 786 / 730, strings 1230 / 1088 / 1175. Incumbent 2228 /
  2279 / 2222. Latin-1 and wide move only the strings bucket (P2.2 retain strings 1602, 2432;
  strs alone 1201, 1810). P1.2: ref 382-404, below host 428-433 and inc 511-518. U-* rows:
  parse 14-40 %, build 33-72 % (ListMetrics 69-72 %: packed runs into lists), strings 1-51 %.
- **Where core-ffi loses**: to host-gen, in parse + crossings + build, not in strings. P2.2
  skip (core parse + cross + build) 970 / 940 / 851 vs hskip 799 / 786 / 730 (+120 to +170);
  strings equal within spread (core-ffi 1069-1250, host-gen 1088-1230). Taking the build as
  equal (the same facade objects, lists and strings), host-gen's own parse is about 190-215 us
  on P2.2 against the core's 317-335 + 19-26 of crossings. P2.3, P2.4, P2.5, P4.1 the same
  pattern, smaller. To the incumbent core-ffi does not lose on the P rows at these spreads
  (ref - inc -8 to -346 on P2.2, -114 to -135 on P1.2, -54 to +51 on P2.3) nor on the U-*
  rows (-0.05 to -1.05 us).
- **Strings bucket is mostly GC, beyond GetString**: ref - skip is 1.6 to 1.9 x the strings
  alone (P2.2 ascii 1069-1250 vs 639-652), and the GC pause per op moves with it (ref 171-367
  us vs skip 35-41): each string is allocated while the half-built graph is live, so the gen0
  collections during a decode copy the graph (0.11 to 0.18 gen0 per op on P2.2 ref vs 0.04 for
  skip); host-gen pays the same.

## 78. Step 9a (decoded runs pre-size their list) and 9b (the owner's six-arm decode table) (2026-10-09)

Container instrumentation; process CPU per op; quiet wait per process. Logs `ffi/logs/csharp/opt/s9/`.

- **9a** (`cs_host.py` `_add_body`, every push `Add_*` callback and every pull-replay ADD):
  `EnsureCapacity(Count + n)` before the loop that appends a run of n (List<T> from .NET 6;
  the generated host half compiles for net6.0 and net8.0 only, so no #if). OrderedMap gained
  `EnsureCapacity(n)` (Dictionary and order list; netstandard2.0 grows the order list only,
  having no Dictionary.EnsureCapacity). Facade constructors unchanged. Quick checks (gate
  levels 8) passed, counts unchanged (`s9/checks-9a.log`). A/B core-ffi push retain
  decode-read, before 51abb00c / after, 2 reps (`s9/ab-9a/table.md`): allocated bytes per op
  down where runs are long (P2.4 3.28 -> 2.78 MB, P2.3 2.10 -> 1.93 MB, P2.2 2.18 -> 2.11 MB,
  P4.1 364 -> 337 KB, U-wire-ListMetrics 4,776 -> 2,552 B); CPU P2.2 1956 -> 1832 us (latin1
  2678 -> 2371, wide 3272 -> 3075), P2.4 1829 -> 1653, P4.1 309 -> 295, U-wire-ListMetrics 3.42
  -> 2.67 us, U-deep-u-repeated 3.07 -> 2.95; within spread or noisy on P1.2 (384 -> 374), P2.5,
  the other U-* rows; P2.3 1316 -> 1433 against the allocation drop (spreads 1231-1689 /
  1299-1848, flagged). P1.2's bytes rise slightly (666 -> 669 KB): its 5,000 results arrive in
  several runs, and the first EnsureCapacity sets an exact rather than a power-of-two capacity.
- **9b** (`AK_BDN_S9=1`, Cases.cs: the six arms only; RootOps DecIncProdDiscard = the same
  Gp parser `.WithDiscardUnknownFields(true)`, built once per root; DecFfiPullR = TryPull with
  retain): the campaign harness (BDN per-case children, core-grid settings: client CPUs 0,1,
  warm-up 25 x 40 ms, 6 rounds x 40 ms), decode-read only, full build, at 36004329 (9a in), 2
  reps (unit order shuffled per launch), 283 s each. 150 cases per rep: the 16 shapes, P2.2
  Latin-1 and wide, the 7 U-* rows, x 6 arms. Times-only table `s9/ab-9b/table.md`; spreads,
  allocation, gen0 and minflt `s9/ab-9b/reference.md` (P5.2 to P5.4 are bimodal across reps, as
  in every earlier run; P2.3 core push retain 1232-1642).

## 79. D23: the FSM decode family's C# consumer, checks and the eight-arm decode table (2026-10-09)

Container instrumentation; process CPU per op. Logs `ffi/logs/csharp/opt/s10/`.

- **Built** against plan.py's FSM contract (core 8030b7f9), then rebuilt on the owner's
  amendment (core c2b95f62: begin/next RETURN the op, `op` removed from ak_fsm_ev, which became
  slot, n, token, data, bytes). cs_binding: `fsm_imports` from plan.fsm_entry_points, FIXED's
  32-bit constants rendered (AK_BDR_*). cs_host: `_emit_fsm`, its own entry (TryFsm), mask
  (FsmPvt via ak_fsm_set_pvt_<R>, once per context), dispatch (FsmDispatch on the returned op)
  and run appends (`_fsm_append`, a renderer separate from push/pull's `_add_body`); the facade
  side (G.D_*, G.Str, Take/Drop, the arena) and decision 11's arming are shared, as push and pull
  share them. One `fixed` over begin and every next. SuppressGCTransition: illegal in retain
  (grow is a reverse call from inside begin/next), legal in drop; not rendered.
- **First contract, first run** (`s10/checks-v0/`): --verify-fsm passed both builds except
  P7.1, where the FSM made 3 calls and the Rust slice counts 7 events for the same 98 bytes.
  Cause: this slice's codec suite decoded the INCUMBENT's re-encoding of P7.1's graph, which is
  contiguous, not the committed interleaved vector (SHAPES.md P7.1). Every C# decode figure of
  P7.1 before this entry is of the contiguous form. Fixed (Cases.DecodeWire: P7.1's decode rows
  decode schema/generated/payloads/P7_1.bin, checked to be a non-identical permutation;
  Cases.Verify checks every decode arm's graph of it); 0 failures after.
- **Amended contract**: regenerated (Abi.cs, the probe), cores rebuilt at c2b95f62,
  gen/s10_checks.sh: everything passed except the committed-counts diff, which was exactly the
  six P7.1 core-ffi decode rows (push rev 3 -> 7); counts re-committed and re-run identical. FSM
  calls = the Rust slice's events on all 342 input x mode rows (both builds); graphs push = pull
  = FSM on every input; 31,014 malformed decodes per family over both builds, the FSM's code =
  pull's on every one (push's too); corpus FSM arms 680 pass / 0 fail per arm, both builds; four
  plants caught.
- **Timed** (`s10/ab-s10/`, at f2797456, 2 reps, 384 / 381 s, quiet before each): eight arms x
  25 rows. Where the FSM's medians lie above push's and pull's in both modes: P6.1 (FSM 303-316
  us median, ranges 297-565; push 246-252, pull 245-246), U-wire-ListMetricsResponse (3.02-3.20 us
  vs 2.60-2.65), P7.1 (0.64-0.66 vs 0.52-0.62), and P4.1 drop (306, range 287-619, against 287 /
  264). Elsewhere the FSM's medians fall inside the spread of push's and pull's (P1.x, P2.x, P3.1,
  the small U-* rows), or the rows are bimodal across reps as in every earlier run (P5.2 to P5.4).
  Allocation per op is identical across the three families on every row. Not attributed (no
  profile run); the Rust slice reports the same pattern on the packed rows (its JOURNAL, D23).

## 80. s11: the D23 eight-arm table rerun with the FSM's fixes A and B (2026-10-09)

Container instrumentation. Logs `ffi/logs/csharp/opt/s11/`.

- Core 081de788 (fix A 55c2771c: packed bodies in a tight loop; fix B 01c73821: the FSM's reader
  bounded to the open message; fix C 75f819f8 withdrawn by 081de788). The event contract, arena
  and imports are unchanged: generate.py rewrote nothing. Cores rebuilt; gen/s10_checks.sh
  PASSED, with the counting grid and the per-input event files byte-identical to s10's.
- Timed at 37697a69, same protocol as s10 (2 reps, 381 / 380 s, quiet). Against s10, the rows
  where the FSM lay above both push and pull on every rep (lowest FSM rep median above the
  highest of each) were P6.1 (both modes, 1.13 to 1.16 x), U-wire-ListMetricsResponse (1.10 to
  1.15 x), P7.1, P2.5 drop and three rows at 1.00 to 1.02 x; in s11 P6.1 (FSM 239-241 us
  medians against push 245-256, pull 240-251) and U-wire-ListMetrics (2.66-2.71 against
  2.60-2.74) are no longer in the list. Still in it, each at 1.01 to 1.05 x: P2.2 drop (FSM
  reps 1944 / 2555 us, the widest spread of the run), P2.2/wide retain, P2.5 drop, P3.1 drop,
  P7.1 both modes, U-deep-u-repeated retain. Two reps only: a row enters or leaves the list on
  one rep's median (s10's and s11's lists differ also on rows the fixes do not touch).
