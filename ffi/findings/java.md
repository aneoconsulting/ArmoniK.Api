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

## 2. The decode regression is real, and the pull family removes it

**This section had a different title until the slice built the arm.** What follows is the
push measurement, then what pull did to it.

`ffi` decode is **1.22 to 1.62 on every M2 payload** — a regression, and the C ABI
loses to the generated Java codec by 1,163 to 1,806 ns per element there, with a
clean sign over 40 rounds on three of the five.

The slice decomposed it rather than reporting the ratio: **7.004 upcalls per
element at about 80 ns is 560 ns of it**. That is the crossing count the branch
has been quoting since the rust slice, multiplied by this host's reverse price,
and it accounts for roughly half the gap.

**This was the concrete argument for ABI v1 7.1's pull family. The family now exists
in the shared core, this slice built the binding, and the regression is gone.**

| payload | `R` (generated Java) | `ffi` push | `ffi-pull` | `ffi-pull-walk` |
|---|---|---|---|---|
| P2.1 | 0.747 | 1.311 | 0.947 | 0.923 |
| **P2.2** | 0.884 | **1.383** | **0.807** | 0.853 |
| P2.3 | 0.962 | 1.335 | 0.890 | 0.916 |
| P2.5 | 0.812 | 1.383 | 0.846 | 0.822 |
| P6.1 | 1.056 | 1.197 | **0.465** | 0.422 |
| P7.1 | 0.523 | **3.000** | 1.157 | 0.901 |

**Reverse crossings are zero on every payload in both deliveries**, where push makes
3,501 on P2.2, and pull is **never slower than push**: a clean sign in its favour on
eight of sixteen payloads, straddling zero on the rest, and not one payload with an
established sign the other way. On P2.2 the delta is 1,606 ns per element — the 7.004
upcalls at 80 ns, and then some.

**The verdict against the no-boundary control is a tie, and the slice said so rather
than taking the win.** `R - ffi-pull` straddles zero on every M2 payload. So **pull
does not make the C ABI beat a generated Java codec on decode; it stops the C ABI
losing to one.** Push loses to arm R with a clean sign on four payloads and pull loses
to it nowhere. That is a smaller claim than the table's headline invites, and it is the
right one.

**Two things fall out that the branch did not ask for.** The drain copy **does not
measure on the JVM** — `ffi-pull - ffi-pull-walk` straddles zero on all sixteen payloads
— where the C# slice estimated the same intermediate at 12 to 19 percent of a parse on
a host whose crossing is worth 10 ns rather than 80. So a binding that finds the walk
delivery awkward can drain and lose nothing, which is a better answer for the
specification than either delivery alone. And because `ak_parse_*` makes no upcall **by
construction**, the binding hands the wire over under `GetPrimitiveArrayCritical` and
never copies it into native scratch — the copy the push family forces, since an upcall
and a critical section are mutually exclusive. The shim pushes no callback frame at all,
so a future callback cannot be added without someone noticing.

**This is the one result in the slice that changes an architecture rather than a
number**, and it is the measurement decision 2 had been waiting on: four of five slices
had only ever measured push, and the push-only evidence could not support "carry both
families and let a JVM binding choose pull".

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
moves every protobuf-java arm and leaves the C ABI arm **unmoved** — because ABI v1
section 4 put the transcoder in the core, so that arm has no `charAt` site to
profile. That is an argument for the design that no benchmark was looking for, and
it is the kind of thing only a slice that measures the hazard rather than avoiding
it can find.

**R9's rule itself is wrong in four of five particulars and I have corrected it**:
it reproduces on JDK 17 and not 21; the trigger is any read of a Latin-1 `String`'s
chars rather than a numeric conversion; no narrowing loop is involved; the incumbent
gets faster rather than slower; and nothing happens on ASCII at all — **which is why
every published managed figure has been blind to it.**

### 6b. Then the slice took the JIT's log, and two of its own published claims came out

This is the best thing in the slice and it is a retraction. The mechanism above was
inferred from which triggers fire and which arms move; `logs/java/r9-mechanism.log`
replaces the inference with C2's own output, and the inference does not survive it.

- **It is branch pruning, not deoptimisation.** C2 emits `inline_fail 'call site not
  reached'` for `StringUTF16.charAt` at both sites in protobuf-java's `encodeUtf8` in
  every slow run, and compiles the branch with `_getCharStringU` in every fast one.
  Runtime `uncommon_trap` events are **9 in the slow state against 11 in the fast**,
  which is the fact that rules deoptimisation out rather than merely failing to
  support it.
- **It is a probability, not a penalty.** Runs land at about 620 us or about 1,250 us
  with an empty gap. The Latin-1 probe reaches the fast state 10 of 10; unprompted the
  process gets there about 1 run in 10. **So "2.16×" was a ratio of two modes**, and
  the branch's real content for the branch is that **a managed figure taken once is a
  coin toss on this hazard.**
- **The shared-profile story is refuted by the same logs.** `ak.Utf8.encode` and
  `ak.Utf8.length` compile identically in all four modes, never pruned. So the 1.27×
  reported for the generated Java codec is not this effect, and what moves that arm is
  now unattributed.
- **The trigger R9 names no longer fires at all**: `deopt=1`, the `String.format` mode,
  is slow in 13 of 13 here against 4 of 4 fast when first recorded. The slice records
  it as unexplained rather than picking between the two code changes since, because
  attributing it means bisecting a probabilistic outcome. That is the right call and
  it leaves a real loose end.

**The methodological result is worth as much as the finding.** `-XX:+TraceDeoptimization`
and `-Xlog:deoptimization` do not exist on a product build, `-XX:+LogCompilation`
preserves the effect, and **JFR erases it** (642 us against 1,261). The first
measurement had to be of the instruments. Any slice chasing a JIT-shaped hazard with a
profiler attached is measuring the profiler.

Both readings stay in the tree. **What this costs the branch: two sentences I had
published in README R9 are withdrawn, and I withdrew them on the slice's own
evidence.** R1's raise-not-skip working in the direction that hurts is the reason to
trust the rest of this document.

### 6c. README 9.1's C shim, priced on the JVM and refused

I had promoted the C-shim arm to first on this slice's next-step list: it was aimed
straight at the decode regression, since 7.004 upcalls per element at about 80 ns is
560 ns of a 1.441 ratio. The slice priced its primitives before building it and the
arm is refused.

| op, net of an empty-loop control inside one native call | G1 (default) | Parallel | Serial |
|---|---|---|---|
| `SetIntField` | 11.97 | 11.97 | 11.96 |
| `SetObjectField` | **26.7** | 13.7 | 13.7 |
| `SetObjectArrayElement` | **28.0** | 15.6 | 15.5 |
| `NewObject` | 139.2 | 127.2 | 123.5 |

A cached upcall on this machine is 72 to 80 ns, so **a JNI field store is a third of a
whole reverse call.** The crossover between one upcall carrying k stores in bytecode
and k JNI stores with no upcall is at **k = 2 to 3**; `TaskDetailed`'s apply is k = 30.

**The generalisation is what matters, and it decides a question the branch had open.**
On the JVM the push family's cost is the *number* of transitions, not what happens
inside them: a transition cannot be made cheaper, because the cheapest thing that
crosses is already a third of one. It can only be made rarer, which is 7.1's pull
family and open decision 10. **So the C shim is not a second independent route to
Java's decode problem; there is one route.** That also bounds README 9.1: the shape is
a Python default, because on CPython a C-API primitive call sits far below interpreter
re-entry and on the JVM it does not.

**And the probe's own first control was defective**, logged as J17: the upcall callee
stored one value into one field k times, which C2 folds to a single store, putting the
crossover at k = 4. Both sides now write k distinct values into k distinct fields.
That is the second control-side defect in this slice in two days, and both were in
controls rather than in codecs — which is where this branch's defects keep being.

## 7. What this asks of the design documents

1. **Decision 9: adopt.** Three hosts, condition met.
2. **Section 7.1: the pull family is built, measured on this host, and decision 2 can
   be answered.** The ABI carries both families and a JVM binding takes pull: reverse
   crossings zero, the M2 regression gone (P2.2 1.383 push to 0.807 pull), pull never
   slower than push on any payload, a tie rather than a win against the no-boundary
   control, and a drain copy that does not measure.
3. **Section 9's virtual-thread amendment: promote from amendment to measured**, with
   the pinning table.
4. **R9: corrected twice** in the README, both times on this slice's evidence. The
   hazard is branch pruning rather than deoptimisation, and it shifts a probability
   rather than imposing a cost, so the rule now asks for a repeat count rather than a
   ratio.
5. **Section 13 outcome 2**: Java's managed codec beats the C ABI on decode and does
   not on encode, so even in Java the fallback is direction-dependent.
6. **P2.5**: protobuf-java writes 19,712 B, making it the **third independent Google
   runtime** to do so. `design/SHAPES.md`'s two-valid-encodings rule is now measured
   in three runtimes rather than two.
7. **Section 9.1 is a Python default, not an ABI-wide shape.** The JVM analogue is
   priced and refused, and the reason — the cheapest JNI accessor is already a third
   of a transition — is now in the README beside the Python case for it.
8. **Section 2's crossing table lists one number per runtime where JNI has two**, seven
   times apart. Both directions are now stated, with caching the method id worth four
   times the call.

## 8. What is not established

- **There is no grpc-java comparison.** The RPC arm exists only as the pinning
  question. Java's transport half is unmeasured, which matters because outcome 2
  keeps the RPC layer on the C ABI and Java is the host where the crossing is
  dearest. It is now the slice's largest remaining gap, the pull family having
  closed the other one.
- **The decode regression's other half is unexplained.** 560 of 1,163 ns per element
  on P2.2 is upcalls; the rest is not decomposed.
- **The R9 mechanism is settled but one of its readings is not.** `deopt=1`, the mode
  the rule names, fired 4 of 4 when first recorded and 0 of 13 now, and the slice did
  not bisect the two code changes in between. Something in the measured process moved
  a probabilistic outcome and nobody knows which thing.
- **What moves the generated Java codec 1.27× is now unattributed**, the shared-profile
  explanation having been refuted by the compilation logs.
- **FFM is a secondary arm**, not a target, and an FFM-to-JNI ratio remains a
  comparison of binding mechanisms.

## 9. The transport arm, and the delivery mode was worth more than the gap

**The slice was right that only the blocking call existed**, and taking the arm through it
was the core's handicap rather than the slice's. With `ak_call_unary_q` built, CPU
microseconds per RPC on P2.2, JDK 17:

| in flight | grpc-java | core, blocking | core, queue |
|---|---|---|---|
| 1 | 3,867 | 3,884 | 4,978 |
| 8 | 2,781 | 3,509 | **3,025** |
| 16 | 2,754 | 3,435 | **2,979** |

**Against grpc-java the core goes from 1.26 to 1.09 at 8 in flight, by changing nothing
but the delivery.** On JDK 21 the queue is **0.70 of the blocking mode** at 16, and a
**virtual thread drains it at no cost** (2,708 against 2,869 on a platform thread).

**At 1 in flight the queue loses**, and that is the shape of the result rather than a
blemish: submit-then-wait serialises what a blocking call does in one step and pays a
third crossing for it. The queue is a concurrency mechanism, not a faster call.

**Two things the slice refused to claim, and both refusals are right.** It does *not*
reproduce the carrier-pinning comparison: a queue has one drainer by design and one
drainer needs one carrier either way, so this shows the queue is **usable** from a virtual
thread, not that it **rescues** a host from the blocking mode's pinning. And section 9's
"a thread parked in a drain costs a collection nothing" is still an assertion — no
collection was instrumented. A slice with a good number in hand that declines the two
adjacent claims it did not measure is the behaviour this branch is built to get.

### The deadlock, which is the most portable thing in the slice

The first core-RPC figures were taken with `GetPrimitiveArrayCritical` held across the
whole blocking call. **That is a deadlock, not a slow path**: a critical section blocks
the collector, the peer was a grpc-java server in the same process which must allocate to
answer, so a collection needed in that window waited on a critical section that waited on
the server that waited on the collection. **It survived P2.2 by timing and hung on the
first small payload** — the worst failure shape there is, because the large-payload run
that everyone looks at passed.

The rule it produces is now in ABI v1 section 9 and it is not a Java rule: **a host must
not pin a managed array across an ABI call whose completion depends on another thread of
that host.** It also explains why 7.1's pinned-buffer optimisation on decode *is* safe —
`ak_parse_*` makes no upcall and needs no other host thread — which is a distinction the
specification had not drawn because nobody had hit the other side of it.

Fixing it moved the core's CPU by 5 to 7 percent at concurrency, so the withdrawn figures
were contaminated as well as unsafe.
