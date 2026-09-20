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
