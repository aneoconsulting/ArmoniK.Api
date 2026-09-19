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
