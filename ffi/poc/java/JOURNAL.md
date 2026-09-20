# java slice: journal

What was tried, what it measured, what refuted it, in order.
Append; do not rewrite. A later reader comes here to find out that an option
was already refuted and by what.

## Entries

### J1. The crossing was priced before anything was designed

Before a line of the generator existed, `probe/Probe.java` and `probe/probe.c` measured
this machine's JNI crossing in one process: a forward call, a forward call taking a
buffer address, and three shapes of upcall. The published table (README section 2) says
11.2 ns forward and 98.4 ns per crossing on JNI, and both reproduce:

| path | ns, min to max over 9 rounds |
|---|---|
| forward, no argument | 11.0 to 12.7 |
| forward, with a buffer address | 11.3 to 13.6 |
| upcall, class and method id resolved per call | 301 to 314 |
| **upcall, ids cached (the generated shape)** | **76.2 to 106.5** |
| upcall passing a Java object through | 82.3 to 92.0 |

Three things followed from it and none of them would have been obvious afterwards.

**The naive upcall is four times the cached one**, so the C shim caches the method ids
at load and the vtable trampolines carry their slot as a compile-time constant. A binding
that resolved per call would have measured the JNI reflection cache, not the ABI.

**A forward crossing at 11 ns is far above the cpp slice's 2-to-4 ns crossover**, so the
batching predicate should win here, and the slice's job is to say by how much rather than
to discover the sign.

**An upcall is seven times a forward call**, which is why the encode path's five reverse
calls per `TaskDetailed` element matter more than its five forward ones, and why the
design puts the loop callbacks where it does.

### J2. `pos += readLen()` is the wrong way round in Java, and only an unknown field sees it

The first decode of P7.1 threw. The cause is a Java evaluation-order rule rather than a
protobuf one: in `pos += readLen()` the left operand is read *before* the right side runs,
so the bytes `readLen` consumed advancing over the length varint are discarded and the
skip lands inside the body.

It is only reachable from an **unknown length-delimited field**, which is precisely what
README section 10 item 1 says a corpus generated from the schema that reads it never
contains. P7.1 reached it because its permutation oracle walks raw (tag, wire type, body)
triples and therefore skips fields the reader does know.

### J3. protobuf-java writes the empty map value, and it is the third Google runtime to do so

design/SHAPES.md predicted it from protobuf C++ and upb and said protobuf-java was
"expected to follow". It does, at exactly +80 B on P2.5 -- 2 B per emptied map value,
40 of them across 20 elements -- so 19,712 B against the manifest's 19,632.

The correctness gate does not waive this. It checks the full cross product: each arm
parses the other's bytes and re-encodes to its own form, which is the "it must parse both"
half of SHAPES.md's rule and is strictly stronger than the byte identity it replaces.

### J4. Open decision 10 is not theoretical, and encode stayed byte-identical while decode lost a map

The push family delivers a repeated field's runs **before** the element's group, so by the
time `apply` arrives the host has already created the child object a run was attached to.
The obvious `apply` -- construct the child from the group, then fill it -- therefore
discards the run.

Every encode arm was byte-identical on all 16 payloads while this was live. What showed it
was the decode round trip, as `TaskOptions.options` vanishing from every M2 payload and
nothing else changing: 540,422 B in, 487,769 B out, a flat 105 B per element.

The fix is one rule and it is now in the generator: fill into the child that is there, and
on absent leave an existing child alone, because a run having created one means the field
was present on the wire whatever the group's presence bit says.

### J5. ABI v1 section 4 specifies five transcoders and the core had two

`ak_tc_utf8` and `ak_tc_bytes` are the same memcpy once decision 3 settled, and they are
all a host whose representation is already UTF-8 needs -- which C++ (`std::string`) and
Rust (`String`) both are. A JVM host is not. `ak_tc_utf16` and `ak_tc_latin1` are the
entries the specification wrote for a managed host and this is the first slice to reach
them; they are added to this slice's copy of the core's hand-written runtime.

Without them the arm would have transcoded in the host and called `ak_tc_bytes`, which is
a different mechanism wearing the ABI's name: section 4's provenance row is explicitly
about the core doing this "once for every language" so that "every managed host stops
maintaining a UTF-8 encoder".

### J6. The floor and the target diverge in exactly one place, and it is the string

README 5.1 predicted the shape without knowing the content: "Java has no preprocessor, so
there it is literally one emitted source tree per target level". There is one divergence
and there is no second.

From JDK 9 a `String` is a `byte[] value` plus a `byte coder`, LATIN1 when every character
fits. The target reads both through `Unsafe.objectFieldOffset` and hands the core ONE bulk
copy of the string's own storage, naming `ak_tc_latin1` or `ak_tc_utf16`. Every id in the
real schema is an ASCII GUID, so the common case stages 36 bytes. The floor has no compact
form, so it stages through `getChars` into a scratch array and always as UTF-16: two
copies where the target does one, and 72 bytes where the target does 36.

It is checked rather than assumed. `Str17` encodes five probe strings through both paths at
class-init and refuses the fast one unless they agree, so a JDK that laid `String` out
differently falls back to the floor's staging instead of writing wrong bytes. The
conformance run prints which path is live and the floor's run says "NOT available",
which is the positive control for the negative case.

### J7. Three ways to handicap or flatter the incumbent, two of them mine

The cpp slice's review moved its encode column about eight points because its harness made
protobuf do extra work. This slice looked for the same class and found the opposite sign
twice, which is the more dangerous direction because a slice has no reason to go looking.

1. **protobuf-java memoizes `getSerializedSize()` on the instance**, and both
   `toByteArray` and `writeTo` call it. A loop over one message therefore pays the size
   pass -- a full walk computing every string's UTF-8 length -- once, and every other arm
   pays its own equivalent every iteration. Measured directly in `logs/java/baseline.log`:
   the size pass is **1.5 to 2.8 times the write** on every element-bearing payload, which
   is larger than the effect this slice exists to measure. The baseline is now a pool of
   messages each serialised once.

2. **The decode baseline was handicapped by the harness, not by the library.** Keeping the
   parse alive with `Message.hashCode()` charges the incumbent a second full traversal of
   the decoded tree that no other arm pays. `System.identityHashCode` replaced it.

3. **A pool built by `parseFrom` would have flattered the incumbent and does not.**
   protobuf-java's string fields hold a `java.lang.Object`: a `String` when built, a
   `ByteString` when parsed, and `writeTo` writes the raw object -- so a parsed message
   looked like it should re-serialise with no UTF-8 transcoding at all. Built as its own
   arm (`pbj-parsed`) the ratio is **0.98 to 1.05 of a built message on every payload**, so
   the effect is not there and the correction it would have justified is withdrawn. The
   pool is built from Strings anyway, because that is what a server holds.

   *Refuted by measurement, recorded so the next slice does not re-derive it.*

### J8. A pooled baseline swaps one bias for another, and both had to go

Fixing (1) above made the baseline walk N distinct message trees while every other arm
re-read one hot object -- on P1.1 that is thousands of small messages against a single
one, which is a memory-system result and not a codec one. Every encode arm now reads
`facadePool[i]` or `pbPool[i]` with the same N, and both pools are rebuilt before every
round even though the facade pool does not need it, so the arms do not differ in where
their input lives.

This is the single largest methodological correction in the slice, and the numbers moved
with it: `ffi-take` on P2.2 read 0.697 of the incumbent with one hot facade and 0.86 to
0.88 with symmetric pools, across two independent runs.

### J9. The main bench cannot see the batching delta, and the reason is its own honesty

With the pools rebuilt between rounds the collector that follows is larger than a seven
percent effect, so every delta in the main bench straddles zero. That is not a reason to
quote the median anyway; it is a reason to ask the question with a different instrument.
`ak.RunDelta` measures the same two arms with no incumbent, no pool churn and nothing
allocating between rounds -- two configurations of one binding, differing by one boolean,
reading the same objects in the same order. See STATE.md for what it found.

### J10. The floor's cost is one line of code, and the table proves it from inside

README 5.2 arm b is "what the floor's missing APIs cost, runtime held constant". Two source
trees with the same class names cannot share a classpath, so the floor binding is emitted a
third time into `ak.floor` over the same facade types: one process, one paired ratio, and
the only thing that differs between the two arms is how a String reaches the core.

It costs **1.42 to 1.77** on every payload with strings and **0.99 to 1.03** on the absent
path, the bulk-bytes path and the packed control. The rows that do not move are the rows
with no strings to stage, which makes the cause a control inside its own table rather than
an attribution.

Two negative controls come free: in the Java 8 build `ak.floor` and `ak.shapes` are the
same emitted source, so the same measurement must read zero there. It does -- worst median
4.06 % on the target runtime and 2.82 % on the floor runtime, and not one row of thirty with
a clean sign -- which puts the instrument's noise floor at one to four percent against a
forty-two to seventy-seven percent effect.

### J11. R9's hazard is real, and almost nothing the rule says about it is

The rule: "on JDK 21 and later a single `String.format` with a numeric conversion
permanently deoptimises every char narrowing loop in the process, which is protobuf-java's
own encoder." Testing it took four modes rather than two, because the first result was a
factor of two in the wrong direction and that is not something to report as a mystery.

Modes 2 and 3 read a three-character String's chars 20,000 times and do nothing else -- no
Formatter, no numeric conversion, no narrowing loop -- and differ only in the probe
string's coder. Mode 3 (Latin-1) reproduces mode 1 exactly and mode 2 (wide) does nothing.
So the trigger is a Latin-1 char read and `String.format` is one instance of it.

On JDK 17, above U+00FF, P1.2: protobuf-java goes from 1,297,600 ns to 601,283, arm R goes
from 598,193 to 757,617, and the C ABI arm does not move. **Two arms, opposite directions,
and the third immune** -- which is what named the mechanism. Both Java-side encoders are
`charAt` loops over a `String` sharing one compact-string dispatch profile that can only be
specialised one way; the C ABI arm has no such loop because the transcoder is in the core.

It reproduces on JDK 17 and not on 21, and on no content set but the widest. An ASCII-only
pass cannot see any of it, which is why three published reports have not.

**The rule survives its own correction.** Keeping formatting out of the measured process is
right whatever the sign, and every harness here already does it. What changes is that a
managed slice's non-ASCII string-path number is not a stable quantity on JDK 17 unless the
harness says what warmed the process, and none of the published ones does.

### J12. Two `pgrep`/`pkill` self-matches cost about twenty minutes

`pkill -f RunDelta` from a shell whose own command line contained "RunDelta" killed the
shell, and `until ! pgrep -f "ak.RunDelta"` never exited for the same reason. Recorded
because the second one is silent: it looks exactly like a benchmark that is still running.

### J13. ABI v1 section 9's virtual-thread amendment, confirmed without an RPC stack

The amendment says blocking in a native frame from a virtual thread pins its carrier, and
that parking in Java on a future -- which the callback mode provides -- does not. It reads
as a claim about gRPC and it is not: it is a claim about where the waiting happens, and it
needs no transport at all.

Eight virtual threads waiting 300 ms each, on a scheduler with a known parallelism. Pinned,
the run is `ceil(N/P)*W`; unpinned, `W`. Three carrier counts, so the reading is a curve:

| carriers | predicted if pinned | blocking in the native frame | parked on a future |
|---|---|---|---|
| 1 | 2,400 ms | 2,420 | 306 |
| 2 | 1,200 ms | 1,222 | 304 |
| 4 | 600 ms | 622 | 305 |

The blocking mode tracks the pinned prediction to within one percent at every point and the
callback mode is flat. The amendment is right, and the consequence is the one it draws: the
callback mode is not a convenience.

This is the whole of the RPC arm this slice built, and it is deliberate. The rest of that
arm -- CPU per RPC against grpc-java at 1, 8 and 16 in flight -- would have been a third
measurement of a call path whose cost ABI v1 section 9 already shows to be four parts in a
million, against a transport stack that is not the thing under test. The pinning question
was the only item on the list that a JVM can answer and nothing else can.

### J14. W10: re-gated on the shared core, and the stale-artifact hazard closed by deletion

R0 moved the core to `poc/codec` and folded this slice's `ak_tc_latin1` and `ak_tc_utf16`
into it. The warning that came with it was that a build locating the core by a directory
search could link the pre-move `.so` -- "a change that measures identical because it is not
in the build". Closed two ways: `build/` and the old `core/` were deleted before the first
rebuild, and the shared crate links as `-lak_core` where the old one was
`libak_core_java.so`, so a stale copy could not have satisfied it anyway. Verified from
`ldd` and `nm` on the loaded artifact rather than from the build log.

**3,891 checks, 0 failures** across all three of README 5.2's arms and all three content
sets. The content sets are what exercise the two transcoders, which is why this gate had to
run on the machine with JDK 8, 17 and 21 rather than on the one that did the move.

The delta instrument moved by at most **0.052** across 45 rows against a bar of 0.078, and
the largest move is on P5.3, a payload with no repeated field where the two arms run the
same code. Nothing re-taken.

Deleting `build/` also deleted the fetched `protoc`, which the build script now re-fetches
when it is missing. A tree that cannot be rebuilt after `rm -rf build/` is a tree where
nobody can safely check for a stale artifact.

### J15. R14 changed the baseline and not the verdict, and the interesting number is the incumbent's

R14 arrived with W10: the baseline is the path gRPC's marshaller takes, not the library's
best entry point. Every table in this slice was against `toByteArray`.

The marshaller's path was read from its bytecode rather than remembered. Encode:
`getSerializedSize()` then `writeTo(OutputStream)` through a 4 KB `CodedOutputStream`.
Decode: read into a thread-local array, parse from the array -- **with a fast path that
returns the very same object when handed back its own `ProtoInputStream`**, which is not a
parse and would have measured nothing. The arm hands it a real `KnownLength` stream.

Against the real marshaller the C ABI encodes at 0.60 to 0.88 on the element-bearing
payloads, where against `toByteArray` it is 0.58 to 0.96; decode is unchanged in shape.
**The verdict does not move.**

What moves is the incumbent: `toByteArray` is **0.48 to 0.65 of the marshaller path**. The
entry point every benchmark reaches for is about twice as fast as the one an application
takes. Together with J7's memoization finding -- a loop over one message is another factor
of two -- an encode ratio against protobuf-java can be moved by a factor of four by two
harness choices that no published report in this branch states.

### J16. The arm the incumbent is compared against was the one carrying the handicap

Reading the `-take` column out loud to explain what it does is what found D7. `-take`
exists so that an arm and `toByteArray` deliver the same thing: a fresh `byte[]`. Arm R's
version is an allocation and one `System.arraycopy`. The ffi version asked the core for
the length over the boundary, copied native memory into a reused scratch array, and then
copied the scratch array into the result. Two crossings and two copies against one and
one.

Three things worth keeping from it.

**The trap pointed inward.** The brief named a handicapped incumbent as the first of the
three traps that cost the C++ slice about 8 points, and every check in this slice was
built looking outward, at whether protobuf-java was being flattered. J7 and J8 found two
of those. Nobody checked the same question in the other direction, and the arm that had it
is the one every encode headline is quoted from.

**It was a known defect in a place nobody looked twice.** D6 removed exactly this
redundant `encodedLength` crossing from the `ffi` arm. The identical call sat four lines
away in `takeBytes` and survived, because the fix was aimed at a finding rather than at
the mechanism the finding named.

**And the prediction about what it was worth was wrong, which is the part worth
recording.** Before re-running I said the second copy was plausibly most of the P5.3 and
P5.4 penalty. The re-run says otherwise: the fresh 4 MB allocation dominates and both arms
pay it, the removed copy was about a fifth of the take overhead, and the 80 us the fix
can claim on P5.4 sits under a 124 us noise floor measured on the *unchanged* arm in the
same pair of runs. A defect can be real, worth fixing on instrument-correctness grounds,
and change no published figure. Both logs are kept and `encode.log` stays the cited one.

**The pgrep trap, for the second time.** J12 records `pkill -f RunDelta` killing my own
shell and an `until ! pgrep -f "ak.RunDelta"` loop that never exited because it matched
itself. I then waited on this bench with `while pgrep -f "ak.Bench"`, whose own `bash -c`
command line contains `ak.Bench`. The bench finished in 3 minutes 11 seconds; the waiter
spun for 1 hour 44. Writing a trap down is not the same as not walking into it. The
working form matches on the JVM itself, `pgrep -f "bin/java.*ak.Bench"`, or better, holds
the child's pid and waits on that.

### J17. The C-shim arm was refused by a probe, and the probe's first control was defective

The aggregating session promoted README 9.1's C-shim binding arm to first: have the
generated C read and write facade fields through the JNI API instead of upcalling into
Java, removing the 7.004 reverse calls per element that are 560 ns of this slice's decode
regression. The Python slice validated that shape in its own runtime.

Pricing the primitives before building the arm took an afternoon and refused it. A JNI
`SetObjectField` is 26.7 ns under G1 and 13.7 under Parallel; a cached upcall is 72 to 80.
So a JNI accessor is a *third of a whole reverse call*, and the crossover between "one
upcall carrying k stores" and "k JNI stores and no upcall" lands at k = 2 to 3.
`TaskDetailed`'s apply is k = 30. `NewObject` at 123 to 139 ns per element makes it worse.

**The first version of the probe said the crossover was at k = 4**, because its upcall
callee stored one value into one field k times and C2 reduces that to a single store. Both
sides now write k distinct values into k distinct fields. That defect was the second of the
three traps the brief named -- a defective no-boundary control -- sitting inside the
instrument that decides whether to build an arm. Two in two days, D7 and this one, both in
controls rather than in codecs.

What the result is good for is not the refusal. It is that **on the JVM the push family's
cost is the number of transitions, not what happens inside them.** A transition cannot be
made cheaper, because the cheapest thing that crosses is already a third of one. It can
only be made rarer, which is the pull family and open decision 10, and which the rust
slice is building. So this is not a second independent route to that result; it is a
reason there is only one route.

Kept for its own sake: `SetObjectField` and `SetObjectArrayElement` both double under G1
against Parallel and Serial while `SetIntField` does not move. The G1 write barrier,
priced, for any native code that stores a reference into a Java object.

### J18. The instrument decides whether the hazard exists, and the inference it replaced was wrong

R9's mechanism was the one inference left standing in this slice, and the README had
already published the correction resting on it. Settling it needed the JIT's own output.

**First problem: the flags R9 would be settled with do not exist here.**
`-XX:+TraceDeoptimization` and `-Xlog:deoptimization` are develop-build only.
`-XX:+LogCompilation` works and **preserves** the effect. `-XX:StartFlightRecording` works
and **erases** it: 642 us at `deopt=0` where the same run without it reads 1,261. So the
hazard's existence depends on which instrument is watching, and the first measurement had
to be of the instruments rather than of the thing.

**Second: the mechanism is not what the arms suggested.** In every slow run C2 emits
`inline_fail reason='call site not reached'` for `StringUTF16.charAt` at both `charAt`
sites in protobuf-java's `encodeUtf8`; in every fast run it compiles that branch with the
`_getCharStringU` intrinsic. The payload is entirely above U+00FF, so the pruned branch is
the one the work needs. It is compile-time pruning, and the slow state has FEWER runtime
uncommon traps than the fast one -- so "deoptimises", R9's word, is wrong in this slice.

**Third: the inference this slice published was refuted by the same logs.** STATE.md said
the two encoders share one compact-string dispatch profile that can only be specialised one
way, so they move in opposite directions. `ak.Utf8.encode` and `ak.Utf8.length` compile
identically in all four modes: never pruned, intrinsic always applied. Only protobuf-java's
encoder is affected. The story was plausible, consistent with every number in `deopt.log`,
and false.

**Fourth, and the part that changes how the result should be stated: the effect is
bimodal.** Ten runs per mode land at either about 620 us or about 1,250 with nothing
between. The Latin-1 probe makes the fast state certain; with no probe the process reaches
it about one run in ten. So the probe changes a PROBABILITY, and a table with one reading
per cell -- which is what `deopt.log` is -- reports a mode rather than a value. The one
stray mode-2 reading `deopt.log` wrote down instead of dropping was the tenth run.

**Fifth: `deopt=1` no longer reproduces**, 0 of 13 against `deopt.log`'s 4 of 4, and that
is the mode R9 actually names. The payload restriction does not explain it. Two things
changed in the measured process since -- the W10 re-gate and D7 -- and choosing between
them means bisecting a probabilistic outcome, so it is written down as unexplained.

Three of the four corrections this slice published about R9 stand. The two that do not are
the two that were inferred rather than observed, and they are the two the README repeated.

### J19. The pull family, and the number that did not appear

The decode regression this slice spent W6 decomposing -- 1.22 to 1.62 on every M2 payload,
7.004 upcalls per element at about 80 ns -- is gone under ABI v1 7.1's pull family. Against
protobuf-java, M2 goes from 1.31-1.38 to 0.81-0.97. Push is never faster than pull with an
established sign on any payload, and pull is faster on eight of sixteen.

**The discipline that made it worth trusting was the order.** Correctness first: 1,389
checks, 0 failures, both deliveries registered separately so neither could pass by falling
back to the other, and the oracle is byte identity after a round trip because the replay
dispatches to the SAME per-slot methods the push vtable reaches. Then the count, which is
what the family's claim actually is: zero reverse calls, not fewer. Only then the timing.

**Three redundant crossings surfaced on the way, all D6's class, and one was on the arm
being compared against.** `beginDecode` called `Native.decErrReset` on every push decode
when `ak_decode_*` clears the sticky slot itself and says so in a comment; `parse` called
`bdrReset` when `ak_parse_*` resets the buffer itself; and the shim called
`ak_bdr_count_forward` after a drain when `ak_bdr_drain` already bumps `forward` on the way
in, so P1.2's drain read 10 crossings where it makes 6. The last one is a trap in the ABI's
own documentation rather than in this slice, and it is filed as such.

**And the honest part of the result is the one that did not appear.** Against arm R, the
no-boundary control, `R - ffi-pull` straddles zero on every M2 payload. Pull does not make
the C ABI beat a generated Java codec; it stops it losing to one, where push lost with a
clean sign on four payloads. It would have been easy to quote the protobuf-java column and
call the architecture settled. The control says a tie, so the slice says a tie.

**The drain copy is the second thing that did not appear**, and it is worth as much. The C#
slice estimated the pull family's intermediate at 12 to 19 percent of a parse; here
`ffi-pull - ffi-pull-walk` straddles zero on all sixteen payloads. On a runtime where the
crossing the family saves is worth 80 ns rather than 10, the copy it costs disappears. That
is a better answer for the specification than either delivery alone, because it means a
host can pick the one it can implement.

**The pgrep trap, for the third time, and the second time after writing it down.** J12
recorded it, J16 recorded walking into it again and prescribed `pgrep -f "bin/java.*ak.Bench"`
as the working form. That form ALSO self-matches, because the waiter's own command line
contains the string `bin/java.*ak.Bench`. I reported a finished run as still running across
three exchanges. A pattern cannot exclude the process doing the matching when the pattern
is part of that process's arguments; the fix that works is to hold the child's PID
(`echo $! > pidfile`, then `kill -0`), and that is what the last run used. Writing a trap
down twice is still not not walking into it.
