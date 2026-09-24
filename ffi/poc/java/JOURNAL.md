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

### J20. FIX-PLAN WP4 items 6 and 10, R-E4 confirmed by running, and a floor that had stopped building

2026-09-24, correctness only, no timing taken. The core was built from
`git archive 817174f ffi/poc/codec` into a scratch directory, because the rust agent was
changing it concurrently and `gen/generate.py` writes the core's generated files into
`poc/codec`: running the generator in the live tree would have overwritten someone else's
work. The whole slice was built and gated in that snapshot and only `poc/java` files were
copied back. JDK 8 and 17 were installed with apt (the network policy allowed it).

**The first build failed, and the failure was older than this session.** `RunRpc.java`
used `ProcessHandle`, a Java 9 API, and it is in the Java 8 floor's compilation, so
`gen/build.sh` has stopped at the JDK 8 `javac` step since 5241ced. Nothing noticed because
nothing ran the floor after an RPC harness change; the last floor log predates it. Replaced
with `ManagementFactory`, and `gen/gate.sh` now builds and runs arms a, b and c in one
command.

**R-D5.** Confirmed first: `decode-pull.log` times `ffi-pull` and `ffi-pull-walk`, and no
gate log (`conformance.log`, `w10-regate.log`) names them. `gen/gate.sh` prints, per arm,
how many rows it took part in, so a gate that passes with an arm absent is visible rather
than inferred. All three arms: 1,389 checks, 0 failures, both pull arms in 94 rows with 46
round trips each; the counting core in the same run shows 0 reverse crossings for both, so
they are not passing by falling back to push.

**R-D9.** Confirmed by fault injection rather than by reading alone: a fake `JNIEnv` whose
`GetPrimitiveArrayCritical` returns NULL, and `-Wl,--wrap` on the core's entry to see what
it receives. The committed shim enters the core with `(NULL, 65536)`. Fixed in the emitter
(`java_jni.py`), regenerated; the new shim returns `AK_ERR_HOST`, does not enter the core,
and keeps the frame stack balanced past its depth of 8. Swept: two `GetByteArrayElements`
on the RPC `uri` in hand-written `native/rpc.c` were unchecked too. While there, `rpc.c`'s
hand-declared `ak_client_opts` (R-D2's shape, correct here by transcription) was replaced by
the generated declaration, which regeneration had just added to `ak_abi.h` with asserts.

**R-E4, confirmed by running, not fixed** (WP5 ports arm R). A driver per CONTRACT.md:
`RunCorpusR` decodes by reflection on the codec's package-private `dec<Root>`, so all 19
messages arm R has are roots, projects by the facade's own conventions, re-encodes, 5 s per
row on its own thread. 19 failing of 392: the 18 in-scope `X-tag-zero-*` rows accepted, and
`E-map-entry-empty` re-encoded to a third form (key written, empty value omitted). Every
`X-lenwrap-*` row is refused, none hangs. Identical on the floor. The projection comparison
discriminates: on the disputed `U-map-entry` it matched the pure-python reading and not
upb's.

**Two of the review's five gaps are not reachable by the corpus in arm R's scope, and the
third is not reachable at all.** No row repeats a singular message field, and no row
reaches the encoder with an undeclared oneof case, so both were run outside it: arm R
replaces where protobuf C++ (`protoc --decode`, same bytes) merges, and encodes case 99
without refusing. `-0.0`: the emitted presence test `x != 0.0` drops it, but `shapes.proto`
has no singular implicit double, so no generated code carries it; read, not run. And one
the review did not list: `Dec.readLen`'s check `pos + n > limit` wraps in `int` for a
length of 2^31 - 1; the value is refused later by the submessage check, and the JVM's
bounds checks make it harmless, but it is R-D1's form.

**STATE.md rewritten (R-F1, WP6).** The old one said in "what is not measured" that the RPC
arm and the pull family were unbuilt while its own body measured both, carried a
recommendation ("a JVM binding should choose pull") and quoted container timings as
findings throughout. The rewrite quotes no timing, lists every timing log as
instrumentation with the harness defects the campaign must not repeat, and states the
levels as the owner fixed them: floor Java 8, target 17.

### J21. FIX-PLAN WP5 step 3: the Java backend on the shared plan (2026-09-24)

**What moved.** The Java backend now lives in `poc/codec/gen/java_*.py` (commit 2889d87)
and imports `plan` only; `poc/java/gen/` keeps build and harness glue. Retired from the
slice: `java_codec.py` (arm R's own wire rules), `java_layout.py` (its own member-list
derivation), `java_binding.py`, `java_jni.py`, `java_pull.py`, `java_slots.py`,
`java_facade.py`, `javanames.py`, and the imports of `ir.py`, `rust_abi.py`,
`cpp_layout.py` and the cpp slice's `cpp_header.py`. `gen/generate.py` no longer writes
`codec.rs` / `layout.rs` into `poc/codec` (E8); its `--check` runs
`poc/codec/gen/generate.py --check` as is and applies that file's import guard to the nine
Java modules and the slice glue, with the planted violation seen caught.

**Arm R renders the plan and E1 to E6 close by construction.** Encode walks
`MessagePlan.encode` (tag order, the plan's presence test per step, the map entry's own
plan so an empty key or value is omitted, a oneof member written whatever its value) after
`oneof_checks` (undeclared case -> `Enc.Refused`, ERR_ABI). Decode is `MessagePlan.decode`
grouped by field number into a `switch`, with anything not in the table skipped (drop) or
captured verbatim (retain), tag 0 refused on every message, merge for `merge_child` and a
same-member `oneof_set`, the depth limit, and the reject UTF-8 policy (lossy raises: not
rendered). `Dec` keeps primitives only, each written as `ak-rt/src/dec.rs` writes it, and
`readLen` is now 64-bit against the remaining bytes (R-G8). The corpus confirms: the 19 rows
re4 listed as failing all pass (`wp5-re4-closure.log`), and the three rule gaps run outside
the corpus now read as protobuf C++ reads them (merge) or as the plan states (refusal, -0.0
written, the 2^31 - 1 length refused at the length) on arm R and on the ffi arm.

**A second description, so the corpus is in scope everywhere.** The old run covered 392 of
691 rows (19 messages shared by shapes.proto and corpus.proto). The backend now renders the
corpus reader schema too (`ak.corpus`): arm R over all 30 messages, the binding over the 29
`plan.expressible_roots` allows (`Nest` refused, 16 rows reported as not in the C ABI),
linked against the core built with `corpus,init-guard` (its own shim,
`build/jnicorpus`). Six arms x 691 rows x two levels: 0 failing arm-rows. Arm R retain
writes the retained form on every unknown row (no retention gap: its facade has a bag per
message, so R-G11's C ABI limit does not apply to it).

**ak_init.** The pre-WP5 Java binding never called it and every gate passed, because no
core build carried `init-guard` (R-G7's Java instance). Now every codec core build does;
`NativeEntry.ensureInit()` is rendered from `plan.lifecycle` and every generated `Binding`
calls it in its static initialiser. The `noinit` control (`-Dak.skipInit=1`) makes every
accept row of every ffi arm fail with AK_ERR_UNINITIALIZED (-10).

**Three defects the port found, each fixed where the rule is rendered:**
- *Section 8's direct path never ran.* The fill staged the bulk `byte[]` with the
  passthrough transcoder and also passed it as the direct argument; the core takes the
  argument only when the slot carries `AK_STR_DIRECT`, so it ignored it and copied the
  staged bytes. Seen in the counts: P5.1 to P5.4 transcodes 3 -> 2 (`rd5-counts.log` vs
  `wp5-counts.log`, every other row identical). "When a change does not do what it should,
  the first hypothesis is that it is not running": here nothing had checked it ran.
- *The zeroed fill dropped -0.0*: `x != 0.0` as the "differs from the cleared slot" test,
  E5's defect in the binding. Latent (shapes.json has no singular double); now the bit test.
- *`Utf8View.valid` refused any ASCII byte after a multi-byte character*, so the borrowed
  arm refused `"A�B"` (corpus T-enc-lone-high, -low, two-highs). The payload gate's
  Latin-1 and above-U+00FF content sets recode EVERY character, so no gated string ever had
  that shape. Hand-written runtime, fixed in place.

**What the port had to restate from a Rust backend (plan gaps).** The vtable structs'
member order and shape and the pull family's record slot numbering are rendered by
`rust_abi`, not stated in the plan; `java_abi` derives them from the same plan facts. The
lifecycle flag values, `ak_err` and the record header are fixed text in `ak-abi`, named but
not valued by `plan.lifecycle`. Reported, not resolved.

**Not built: ffi retain.** The binding leaves decision 11's slots NULL and has no
`ak_uencode_*` path, so the ffi arms run in drop mode only.

### J22. WP5 tail: D38 and D39, re-gated from a clean core build (2026-09-24)

**D38, confirmed and fixed.** `Dec.skipGroup` checked a key inside a group for field
number 0 but not for a number above 2^29 - 1, and its depth limit was a constant of the
hand-written runtime. The probe row `P-field-maxplus1-in-group` (poc/rust/gen/probe_corpus.py)
was accepted by R and R-retain (logs/rust/wp5s6-probe-java-after.log). Now `java_rcodec`
renders the plan's `MAX_FIELD_NUMBER` and `GROUP_DEPTH_LIMIT` into every codec class and
passes them to `Dec.skip(tag, wire, maxField, groupDepth)`, which holds no limit of its
own (bb98e8f, 271fdd5). Probe on 8 and 17: 6 arms x 11 rows, 0 failing; the in-group row is
refused by arm R ("field number 0 or above 2^29-1 inside a group") as by the core (-2)
(`wp5s6-probe.log`). The corpus's new `X-field-over-max-in-group` row, disputed, reads the
same way.

**D39, confirmed and fixed.** `git archive` gives every file the commit's timestamp, so a
reused `CARGO_TARGET_DIR` could hold artifacts newer than the snapshot's sources and cargo
kept them. `gen/build.sh` now builds into `core-build/<key>/` with key = the git tree hash
of `ffi/poc/codec` at the snapshot revision (a source hash for `AK_CODEC`), points
`core-build/current` at it, and refuses to finish if any shim links another key's core.
`wp5s6-d39-keys.log` shows the key for 41eb485 differs from HEAD's and every shim resolving
to HEAD's key. The other scripts read `core-build/current`.

**Re-gated from `rm -rf core-build build`**, core snapshot 271fdd5 (`init-guard`): payload
gate 1,389 x 3, 0 failures, unknown 66/0 (`wp5s6-gate.log`); the corpus, now 702 rows (the
corpus agent added 11), six arms, target and floor, 0 failing arm-rows, 6 disputed, controls
failing as required (`wp5s6-corpus.log`).
