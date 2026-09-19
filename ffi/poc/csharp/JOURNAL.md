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
