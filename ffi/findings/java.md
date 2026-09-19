# Reading the java slice

The aggregating session's reading of `poc/java`. What is here is what its
results mean for the branch.

**W6 is done, and it inverts the published verdict rather than confirming it.**
Full codec, both Java levels, seven arms through a 437-check correctness gate on
three of README 5.2's arms, the batching prediction tested, decisions 9 and 13
answered for a managed host, R9's hazard measured, and ABI v1 section 9's
virtual-thread amendment confirmed — which no other slice could have done.

**Configuration** (R7): protobuf-java **3.25.5**, which is what `packages/java`'s
pins actually resolve to (the pom declares 3.19.0 and grpc-java 1.74.0 brings
3.25.5; the resolved one is what a consumer runs). Target JDK 17.0.20 with JNI,
floor `openjdk 1.8.0_502`. R13: this container's rust crossing is **2.1 ns**,
against 1.8 in the rust slice's and 1.5 in the C++ slice's.

## 1. The encode regression does not survive, and the cause is a baseline

Published: encode 1.08 to 1.84 of protobuf-java, a regression on every payload,
and it is the result the Java report's recommendation rested on. Measured here:

| direction | the C ABI (`ffi`) | the generated Java codec (`R`) |
|---|---|---|
| encode | **0.58 to 0.96** on every element-bearing payload | 0.66 to 0.98 |
| decode | **1.22 to 1.62 on every M2 payload**, 0.83 to 1.04 on the flat ones | **0.39 to 1.05**, beating the C ABI on all five M2 payloads |

**So "a generated pure-Java codec beats the C ABI in both directions" is half
reproduced.** It does on decode, decisively; it does not on encode.

**What moved the encode column is a property of the incumbent that nobody had
priced.** protobuf-java **memoizes `getSerializedSize()` on the instance**. Both
`toByteArray` and `writeTo` call it, so a benchmark loop over *one* message pays
the size pass — a full walk computing every string's UTF-8 length — once and
amortises it across every iteration, while every other arm pays its own each
time. Measured directly, the cold size pass is **1.5 to 2.8 times the write**.

And the slice did the thing that makes this defensible rather than a guess:
**dividing its own `ffi-take` by its own `pbj-loop` reproduces the published
range**, 1.26 to 1.85 against a published 1.08 to 1.84. That is a mechanism which
*would* produce the published shape, measured in this tree. It is not a claim
about what the prior harness did, and nothing can settle that, because its
sources do not survive.

**Three slices have now found a handicapped incumbent**, in three languages, by
three different mechanisms: a zero-filled output buffer in C++, an extra
traversal in the C# decode sink, and a memoized size pass in Java. This is no
longer a series of slice defects. It is the branch's most reliable failure mode,
and any figure in any prior report that was not taken against a checked baseline
should be read with it in mind.

## 2. The decode regression is real, and it points at a part of the ABI that does not exist

`ffi` decode is **1.22 to 1.62 on every M2 payload** — a regression, and the C ABI
loses to the generated Java codec by 1,163 to 1,806 ns per element there, with a
clean sign over 40 rounds on three of the five.

The slice decomposed it rather than reporting the ratio: **7.004 upcalls per
element at about 80 ns is 560 ns of it**. That is the crossing count the branch
has been quoting since the rust slice, multiplied by this host's reverse price,
and it accounts for roughly half the gap.

**This is the concrete argument for ABI v1 7.1's pull family, which the core does
not implement.** The push family costs one upcall per field group; a pull decode
would let the host drain the core without the core calling back. Until it exists,
Java's decode verdict is a verdict about the push family only, and section 7.1's
"which family does each binding take" is unanswerable for the host that needs it
most.

## 3. The batching crossover is confirmed quantitatively, and this is the strongest cross-slice result in the branch

The C++ slice predicted from its tax sweep that batching would win decisively on
Java, since JNI's crossing is far above the 2 to 4 ns crossover. It does, and the
agreement is better than qualitative:

**On the flat M1 payloads the measured delta per element equals the extra crossing
count times this machine's forward crossing price, to within a tenth.** On the
nested shapes it is two to three times that, because an unbatched element run also
repeats per-call work the chunk amortises.

The magnitude is **10 to 29 percent of an encode against the published 2 to 9**.
Two hosts, one model, one predicting the other's result before it was taken — that
is what the crossover form was for, and it is why the specification carries the
curve rather than a per-language verdict.

## 4. Decision 9 can be adopted: three hosts now agree

Decision 9 said it was settled for C++ and Rust and waiting on the managed hosts.
Java meets the condition it set: **46 ns per element on the absent path, 0.32 of
the total fill, clean sign, clear of the drift bar by an order of magnitude**, and
within the drift bar everywhere else. The absent-path inversion goes from 1.967 to
0.734 of protobuf-java.

Rust, C++ and Java agree. The decision has what it was waiting for.

## 5. Decision 13 is real on the JVM and worth about a third of what it is in C++

| payload | `ffi` | `ffi-borrow` |
|---|---|---|
| P1.2 | 0.830 | **0.669** |
| P1.1 | 1.038 | 0.882 |
| P2.2 (container-heavy) | 1.441 | 1.382, straddles zero |

Borrowing removes 16 percent of a protobuf-java decode on P1.2, where the C++
slice measured the same mechanism taking P1.2 from 0.562-0.790 to 0.234-0.241 —
two to three times more. On the container-heavy payloads it barely moves, which is
the branch's own container-construction bound seen from a third host.

**Three hosts, one mechanism, and the managed host gets less of it.** That is a
fact the lifetime contract has to be drafted against, not a reason to drop it.

## 6. Two results the branch did not ask for, and both argue for the design

**ABI v1 section 9's virtual-thread amendment is confirmed, and only a JVM slice
could have done it.** Eight virtual threads waiting 300 ms on a scheduler of known
parallelism: if blocking in a native frame pins the carrier, the run takes
`ceil(N/P) × W`.

| carriers | predicted if pinned | blocking in the native frame | parked on a future |
|---|---|---|---|
| 1 | 2,400 ms | **2,420** | **306** |
| 2 | 1,200 ms | **1,222** | **304** |
| 4 | 600 ms | **622** | **305** |

The blocking mode scales exactly as the pinned prediction and the callback mode is
flat. **The completion callback is not a convenience; it is what makes the ABI
usable from the idiom Java is moving to.** It needs no RPC stack to measure, which
is why it was cheap and why nobody had done it.

**And R9's hazard makes the C ABI the only arm immune to the host JIT's profile
history.** Reading a Latin-1 `String`'s characters before the first measurement
makes every protobuf-java arm **2.16× faster**, makes the generated Java codec
1.27× slower, and leaves the C ABI arm **unmoved** — because ABI v1 section 4 put
the transcoder in the core, so that arm has no `charAt` loop to profile. That is an
argument for the design that no benchmark was looking for, and it is the kind of
thing only a slice that measures the hazard rather than avoiding it can find.

**R9's rule itself is wrong in four of five particulars and I have corrected it**:
it reproduces on JDK 17 and not 21; the trigger is any read of a Latin-1 `String`'s
chars rather than a numeric conversion; no narrowing loop is involved; the incumbent
gets faster rather than slower; and nothing happens on ASCII at all — **which is why
every published managed figure has been blind to it.**

## 7. What this asks of the design documents

1. **Decision 9: adopt.** Three hosts, condition met.
2. **Section 7.1: the pull family is now load-bearing**, not a design option. Java's
   decode regression decomposes into upcalls, and the core does not implement the
   family that would remove them.
3. **Section 9's virtual-thread amendment: promote from amendment to measured**, with
   the pinning table.
4. **R9: corrected** in the README, on this slice's evidence.
5. **Section 13 outcome 2**: Java's managed codec beats the C ABI on decode and does
   not on encode, so even in Java the fallback is direction-dependent.
6. **P2.5**: protobuf-java writes 19,712 B, making it the **third independent Google
   runtime** to do so. `design/SHAPES.md`'s two-valid-encodings rule is now measured
   in three runtimes rather than two.

## 8. What is not established

- **There is no grpc-java comparison.** The RPC arm exists only as the pinning
  question. Java's transport half is unmeasured, which matters because outcome 2
  keeps the RPC layer on the C ABI and Java is the host where the crossing is
  dearest.
- **The pull family is unbuilt**, so the decode verdict covers the push family only.
- **The decode regression's other half is unexplained.** 560 of 1,163 ns per element
  on P2.2 is upcalls; the rest is not decomposed.
- **The R9 mechanism is inferred** from which triggers fire and which arms move, not
  from a compilation log.
- **FFM is a secondary arm**, not a target, and an FFM-to-JNI ratio remains a
  comparison of binding mechanisms.
