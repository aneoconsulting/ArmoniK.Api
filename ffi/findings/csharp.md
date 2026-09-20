# Reading the csharp slice

The aggregating session's reading of `poc/csharp`. What is here is what its
results mean for the branch.

**W5's named gap is closed and the `core-ffi` arm now exists for M1.** The slice
was scoped to its ABI-independent half while decision 1 was open; that half is
complete on all 16 payloads and all 7 shapes, gated on three runtimes. The held
arm was then built and gated for M1 in both directions, and re-gated on the
shared core after W10. **M2 to M7 are not built**, so C# has an ABI beachhead
rather than an ABI column — and section 4b below is the most consequential thing
in this document.

**Configuration** (R7): `Google.Protobuf` 3.28.3, codegen by `Grpc.Tools` 2.66.0,
target .NET 8.0.31, floor netstandard2.0 and .NET Framework 4.8 on Mono 6.8.0.105
(both build and pass; Mono is timed as arm c). R13: **this container reproduces
the rust slice's own 1.8 ns to the digit**.

**That calibration is doing real work rather than ceremony.** Because this
container agrees with the rust slice's and the C++ slice's does not (1.5 ns there,
and it cannot reproduce the published 0.25 ns C++ row at all), **a C# absolute
from this slice is comparable with a Rust absolute and is not comparable with a
C++ one.** That is a measured statement, and it is the clearest vindication R13
has had.

## 1. The question is answered: C# does not look like Java on decode

The Java report named this as the single measurement that would change its own
recommendation — if C# looked like Java on decode, the conclusion would be "the
codec half of the C ABI does not suit managed runtimes" and the ABI's scope would
narrow to C++ and Python. It does not.

**A generated pure-C# codec decodes at 0.72 to 0.82 of `Google.Protobuf` on every
shape the real schema actually has**, and 0.85 to 0.89 on the hardest content set.

**But the win is not uniform, and the earlier column hid that:**

| shape class | payloads | managed / incumbent |
|---|---|---|
| the real schema's own shapes | P1.1, P1.2, P2.1, P2.2, P2.5, P3.1, P4.1 | **0.72 - 0.82** |
| the absent path | P1.3 | 0.54 - 0.56 |
| container-dense variants of M2 | P2.3, P2.4 | 0.87 - 0.96 |
| packed scalars (a control) | P6.1 | **1.02 - 1.03, a loss** |
| bulk | P5.2 - P5.4 | **ambiguous**, marked as such |

So the defensible claim is narrower than "never at parity": **the managed codec
wins by roughly a fifth on every shape ArmoniK sends, and that win erodes to
nothing as an element's containers come to dominate.** This is the rust slice's
convergence finding reproduced on a managed runtime — and here **it actually
crosses 1.0** rather than merely tending towards it.

**The allocation column is what makes that reading safe.** On every row but P6.1
the managed arm allocates 0.91 to 1.00 of what the incumbent allocates, so the two
are building object graphs of the same size and the win is not "it built less". On
P6.1 it allocates 1.31× — five `List<T>` growths per element against
`RepeatedField` — and it is the only loss. Those two facts belong together, and no
other slice has an allocation column at all.

## 2. It found its own handicapped incumbents, before a review did

This is the behaviour the branch wants and the first time it happened without a
review. Told that an adversarial review had found the C++ slice's incumbent
handicapped three ways, the slice went looking for that defect class in its own
harness and found two:

- **The encode baseline** was `CalculateSize()` + `WriteTo(Span)`, where the size
  pass exists only to size the span. `Google.Protobuf` offers
  `WriteTo(IBufferWriter<byte>)` in the same official API family, which sizes
  nothing at the top level, and it measures **0.708 to 0.806** of what the slice
  had been quoting against. It also avoided the trap C++ fell into: it *resets* the
  reused `ArrayBufferWriter` rather than calling `Clear()`, because `Clear()` zeroes
  the written span, which is exactly the per-iteration wipe that handicapped C++.
- **The decode baseline did a full extra traversal**, and it landed on the number
  the slice exists for. The generated parse ended with `return m.CalculateSize()` to
  stop the decoded graph being optimised away — and that walks the whole decoded
  tree, where the managed arm returned a free position value. Both arms now park the
  graph in a static sink and return an O(1) value.

**Corrected decode is 0.59 to 0.84 where it had been 0.45 to 0.83** — a larger
correction than the eight points the C++ review moved. The slice's own summary is
the right one: *the verdict survives and the margin does not.*

The general rule worth carrying to every remaining arm in the branch: **whatever
stops a result being optimised away must cost the same in every arm.** C++ met the
same class of defect from the other direction, with a control that was not doing
work its arm did.

## 3. The JIT configuration was checked rather than assumed

R9 names tiering and PGO as things that move a verdict rather than a decimal, so
the slice measured all three configurations. **No arm crosses 1.0 under any of
them**, and turning tiering or PGO off slows the *incumbent* — exactly the handicap
R9 warns of. The default used everywhere else, tiering and PGO on, is therefore the
configuration **least** favourable to the managed arms, which is the right way round
for a claim that the managed codec wins.

## 4. What this contributes to the outcome space

The managed control is now measured in three languages, and it is the same arm in
each: a generated pure-host-language codec over the same facade, from the same
description.

| slice | the generated host-language codec |
|---|---|
| C# | **0.72 - 0.82** of `Google.Protobuf` on the real schema's shapes |
| Java | **0.39 - 1.05**, and it beats the C ABI on all five M2 payloads |
| Python | **19.4 - 20.3 times upb** |

**What the three jointly establish is that the answer is a property of the host
runtime's incumbent, not of the approach.** The .NET result is not evidence about
Python and the Python result is not evidence about .NET. README section 13's
outcome 2 is a managed-runtime recommendation, and the C# column is what stops it
being read as a general one.

## 4b. The `core-ffi` arm: C# and Java now disagree, and that is the finding

The held arm is built and gated for M1, encode and decode, on arms a and b. **M2
to M7 are not built**, so this is a beachhead rather than a column — read every
figure below as "on the flat message", not "on the shape set".

**Crossings are constant in the element count, in both directions**: 2 forward
and 1 reverse on encode, 1 forward and 2 reverse on decode, whether the payload
carries four elements or a thousand. `ResultRaw` is a leaf, so the batching
predicate admits it, and at .NET's 7.5 to 12 ns crossing — far above the C++
slice's 2 to 4 ns crossover — batching is not a close call.

**The interface cost**, against the no-boundary managed control:

| payload | encode | decode |
|---|---|---|
| P1.1, 4 elements | 1.330 | 1.062 |
| P1.2, 1000 elements | 1.148 | **0.899** |
| P1.3, the absent path | **2.361** | **1.981** |

**On P1.2 decode, crossing the C ABI is faster than the pure managed codec** —
0.899 of it, and 0.651 to 0.659 of `Google.Protobuf`. A Rust parser plus three
crossings beats a C# parser doing the same work.

**That is the opposite of Java**, where the generated pure-Java codec beat the C
ABI on all five M2 payloads and the case had to rest on maintenance alone. So the
two managed runtimes do not agree, and the branch can no longer speak of "managed
hosts" as one thing. At .NET's crossing price the interface does not eat the
core's advantage; at JNI's it does. That is the crossover argument again, arriving
from a third direction and deciding an architecture rather than a mechanism.

**The absent path collapses, and the cause is a decision that is specified and not
built here.** P1.3 is 300 elements that each encode to nothing, and the host fills
300 by-value groups of 200 bytes apiece: 60 KB of stores to describe 605 bytes of
output. **Decision 9's sparse fill is now the specified path and this arm does not
implement it**, so 2.361 is what the unfixed form costs on .NET and the distance
to roughly 1.15 is what the fix is worth there. The Rust slice measured the same
effect from the other side. This is not a finding against the ABI; it is the
strongest case yet for the decision the ABI already took.

**The floor cannot carry this arm at all, and that is a real constraint rather
than a gap.** .NET Framework 4.8 has no `LibraryImport` and no
`UnmanagedCallersOnly`, so the binding as generated does not compile on arm c. A
floor binding would be `DllImport` plus delegate pointers, **and the delegates
must be rooted for the lifetime of the vtable or the collector reclaims a thunk
the codec still holds** — a crash, not a slowdown. The slice excludes arm c
explicitly rather than quietly.

**One process note worth more than a number.** The loaded artifact is confirmed
from the dynamic linker rather than the build log, and it had to be twice: once a
failed build left a stale core in the output directory, and once **arm c failed to
build with 172 errors while Mono ran a three-hour-old binary and reported a
pass**. A build log would have shown neither.

## 5. What is not established

- **The `core-ffi` arm does not exist**, by instruction. So this slice says nothing
  about what the C ABI costs on .NET, which is half of W5. Every ratio here is
  managed-against-incumbent.
- **Therefore the two field shapes W5 named are only half closed.** Oneofs and
  explicit presence are covered in the facade and the managed codec; whether the
  by-value group reaches a oneof, and whether it can distinguish absent from empty,
  is still unmeasured on .NET and is still an ABI question.
- **The bulk rows are ambiguous and say so.** P5.4 spans 0.558 to 1.096 across three
  processes in one log. That is R2's lesson one direction over: the rust slice needed
  a floor arm before it could report a 0.08, and here a 1.1 needs one just as much.
- **No crossing counts, because there are no crossings**: zero in every arm here,
  which is itself the property that makes them managed-control arms.
- **Mono is timed as arm c and stands alone**, never as a ratio against the target.

## 6. One defect in the handoff itself

`STATE.md` contains a duplicated paragraph whose second copy contradicts the first:
it says of P6.1 that "the managed arm allocates MORE (1.31) and is still faster",
where the table and the preceding paragraph both correctly report P6.1 as the one
**loss** at 1.019 to 1.031. The table is right. Flagged to the slice rather than
edited here, but a reader of the handoff should not be able to find both sentences.

## The group-skip defect, carried here and not yet fixed

`Facade/Wire.cs`'s `Skip(int wire)` has cases for the four wire types the schema
produces and sends everything else to `ErrMalformed`, so an unknown field of the
deprecated GROUP form is rejected where `Google.Protobuf` accepts it. The shared
core had the identical hole — found by the python slice's corpus run, fixed there —
and the C++ slice's `rt.h` still has it; the java slice's `Dec.skip` is the only
host-side one that was already right, field-number match included.

The fix is not "add case 3": a group carries no length, so the skipper recurses to
an `END_GROUP` whose field number matches the one that opened it, with a depth bound.
Counting depth instead accepts `X-group-mismatched-end` and mis-nests everything
after it. `MapForms.Skip` in the harness has the same hole and matters less, being a
harness helper rather than the facade's decoder.

Byte identity against the manifest cannot find this, because proto3 cannot express a
group. Becoming a corpus consumer is what would have.
