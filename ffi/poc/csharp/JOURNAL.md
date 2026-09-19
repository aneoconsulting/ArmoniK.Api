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
