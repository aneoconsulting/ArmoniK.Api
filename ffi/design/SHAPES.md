# The shapes and payloads every slice implements

**Status: built (W2).** This document is the prose; the description a generator
actually reads is [`../schema/shapes.json`](../schema/shapes.json), and the
emitted `.proto`, the payload manifest and the committed vectors are in
[`../schema/generated/`](../schema/generated/). **On a disagreement the JSON wins**, because
it is what the slices consume.

A slice may add an arm. It may not change a shape: a column of the final table
that covers a different set of shapes is not a column, it is a second table.

## Why these and not others

Weighted by a census of the in-scope schema, so that a result read off a shape
the schema barely has is labelled a control rather than a verdict:

| Wire kind | Fields in the real schema |
|---|---|
| `string` | 174 |
| `int32` | 33 |
| `bool` | 14 |
| `int64` | 10 |
| `bytes` | 8 |
| `float`, `double`, `fixed*`, `sint*`, `uint*` | 0 |

413 fields in total; 19 oneofs; 21 enums; 3 packed repeated fields, all of them
enums; 2 maps, both `map<string, string>`; maximum static nesting depth 6.

**So this is a string codec.** Every id, session id, task id, result id and
partition name is an ASCII GUID. A verdict read off the packed-scalar payload is
a verdict about a message ArmoniK does not send.

## Messages

Reproduced from `Protos/V1` where a real message has the shape, invented only
where the schema has no instance of a shape that has to be priced anyway. The
invented ones are marked **control** and their results are never quoted as
headline figures.

| # | Message | Shape | Source |
|---|---|---|---|
| M1 | `ResultRaw` | 3 scalars, 6 strings, 2 singular nested messages | real |
| M2 | `TaskDetailed` | 27 fields: 8 strings, 14 nested messages, 4 repeated string fields, and a nested `TaskOptions` carrying `map<string, string>` | real |
| M3 | `Probe` | one oneof of 5 variants including a payload-free member, plus 3 `optional` scalars (explicit presence) | invented, but both shapes are real: 19 oneofs in the schema |
| M4 | `TaskSummary` | the `Output` adapter: one facade type with two wire forms, a nested message at one site and a plain string at another | real, and the only `with` adapter in the Rust crate. The two forms are both real and both called `Output`: `TaskDetailed.Output` is `{bool success, string error}` and `objects.proto`'s `Output` is a `oneof {Empty ok, Error error}` |
| M5 | `UploadResultData` | two ids and one payload-sized `bytes` field | real |
| M6 | `MetricsBatch` | one string id, four packed repeated scalar fields (`int64`, `double`, `int32`, `bool`) and one packed repeated enum | **split**. The four scalar fields are a **control**: the schema has no packed scalar. `statuses` is **real**, and it is the only place the schema's actual packed shape is reachable, relocated from the filter and request messages that carry it because this payload set has none |
| M7 | `DualResponse` | two repeated fields of the same message type, built so the wire order interleaves them | **control**: legal wire that no group buffer keyed by type can decode |

## Field shapes every slice must exercise

Each row is a thing the generated binding has to have a case for. A slice that
does not cover one records it in `STATE.md` and it goes in the report.

| Shape | Covered by | Why it is on the list |
|---|---|---|
| implicit-presence scalar (`int32`, `int64`, `bool`, `enum`) | M1, M2 | the cheap case, and the only one the drafted ABI got right |
| `string` | M1, M2 | 174 of 413 fields. The dominant cost of the whole codec |
| `bytes`, small | M1 | |
| `bytes`, multi-megabyte | M5 | the result upload and download path, named as common and performance sensitive |
| explicit presence (`optional` scalar) | M3 | a by-value group reports absent and empty identically unless designed not to. Unmeasured on .NET today |
| singular nested message | M1, M2 | |
| nested message, 4 levels (3 edges) | M2 | `ListTasksDetailedResponse -> TaskDetailed -> TaskOptions -> Duration`. **Not the schema's maximum**: this row claimed depth 6 and the description reaches 4. The real 6-level chains are all on filter and request messages (`ListTasksRequest -> Filters -> ...`), and this payload set carries responses only, so deepening it would be inventing depth rather than reproducing it. Levels 5 and 6, and the filter/request family with them, are **not measured**, which also leaves ABI v1 open decision 7 (the decode recursion limit) unexercised |
| repeated string | M2 | the shape the C++ slice could not see, having 2 of 11 repeated fields |
| repeated message, leaf element | M2 | the batched-run case |
| repeated message, non-leaf element | M2, M7 | the case batching must refuse, transitively |
| `map<string, string>` | M2 | and the ABI must not give it a case of its own |
| packed repeated scalar | M6 (control) | the host's own array, handed over whole |
| packed repeated enum | M6 | what the real schema actually has: all 3 of its packed fields are enums. **This row claimed M2 and was wrong** until the Rust slice checked it: `TaskDetailed` has no packed field at all, and the description had no packed enum anywhere. `MetricsBatch.statuses` now carries it, so M6's scalar rows stay a control and its enum row is a result |
| oneof, including a payload-free member | M3 | the group does not reach it. Unmeasured on .NET today |
| open enum with an **unknown value** | M1, M2 | the field is known and only the value is not, so the wire says exactly where to put it and it **does** round-trip losslessly. Measured: `status = 999` survives as `Unknown(999)` in all four arms |
| an **unknown field**, oneof members included | corpus | **cannot round-trip at all**, in any arm, and this row exists because the one above is easy to read as covering it. A parser cannot tell an unrecognised oneof tag from any other unknown field, since the grouping lives only in the descriptor: the case stays at the last known member and the payload is dropped. That is protobuf, not the ABI, but whether the core should *retain* unknown fields is ABI v1 open decision 11, and it is a behaviour change for four of the five languages |
| an adapter site (`with`) | M2 (nested), M4 (plain) | one facade type, two wire forms. Only a byte corpus catches a wrong one, **and until the Rust slice checked it the payload set could not reach the shape at either site**: `success` and `error` were filled independently, so every element was (true, non-empty), a state `TaskDetailed.Output`'s own comment forbids and no adapter over {Ok, Error} can represent, and the success state (true, empty) never occurred. The generator now cycles the three states per element. Verified on the wire: 167 Ok, 167 Error, 166 absent at the nested site of P2.2 |
| an adapter site whose map is **not injective** | M4 | the plain site flattens Ok and Invalid to the same empty string, so **one of them must come back wrong whatever the adapter author chooses**: return Invalid and lose Ok, or return Ok and silently claim success for a task that reported no outcome. The nested form round-trips all three. This is the concrete defect the row above is abstract about, and it is now in the payload rather than only in the prose |
| unknown fields on the wire | corpus | protobuf's forward compatibility, never executed by a corpus generated from the schema that reads it |
| absent and empty everywhere | P1.3, P2.5 | where offset defects hide |

## Payloads

Defined by content rule rather than by byte size, because the byte size is an
output: **once every slice is driven by the same description, two slices
disagreeing on the wire size of a payload is a defect, not a difference.**

Sizes below are what `../schema/emit/payloads.py` produces today, and every
payload also carries a sha256 in `../schema/generated/manifest.json`. **They are
no longer provisional**: the Rust slice checked every one of them against prost
0.14.4 and against `prost_reflect::DynamicMessage` over the same descriptor, and
the one defect it found (two message leaves written when they held the proto
zero) is fixed. P7.1 is the exception and it is validated by decode rather than
by encode, for the reason given in its row below.

| Payload | Message | Elements | What it tests |
|---|---|---|---|
| P1.1 | M1 | 4 | small and flat: the call-rate case, where the per-message cost is spread thinnest |
| P1.2 | M1 | 1,000 | large and flat: strings and scalars only |
| P1.3 | M1, every string empty and every child absent | 300 | the absent path |
| P2.1 | M2 | 1 | small and nested: every field shape in one element |
| **P2.2** | M2 | 500 | **the shape the control plane actually moves. Read this column first** |
| P2.3 | M2, 30 elements in each repeated string field | 125 | repeated strings and nothing else |
| P2.4 | M2, alternating 3 and 150 repeated strings | 80 | element bodies alternating across a varint length boundary. The only payload that exercises the length-placeholder move |
| P2.5 | M2, half the map values emptied and the output child removed | 20 | the absent path, nested |
| P3.1 | M3 | 200 | oneof and explicit presence |
| P4.1 | M4 | 200 | the adapter site |
| P5.1 | M5, 36 B | 1 | the degenerate bulk case: two ids and almost nothing else |
| P5.2 | M5, 64 KB | 1 | result upload and download, small |
| P5.3 | M5, 1 MB | 1 | result upload and download, medium |
| P5.4 | M5, 4 MB | 1 | result upload and download, the size the direct-argument path exists for |
| P6.1 | M6 | 200 | packed scalars. A control, and the string path's control too: 200 strings cannot move it |
| P7.1 | M7 | 3 + 3 | interleaved repeated fields of one type. **No canonical writer can produce it**, prost included: a writer that emits a repeated field contiguously cannot interleave two of them, which is the entire point of the control. A slice validates it by *decoding* it and by re-encoding contiguously to a permutation of the same (tag, wire type, body) triples; no slice is asked to reproduce its bytes |

**Present and zero is the third case, and it is now deliberate.** The payload set
was designed around absent and present, and it reached present-and-zero at leaf
depth only because `timestamp(0).nanos` and `duration(0)` happen to be zero. That
accident is what caught the only defect the schema directory has had, while P1.3
and P2.5 passed it untouched, so it is now a property rather than a coincidence:
a value rule may not be tuned so that no implicit-presence leaf lands on zero.

**P1.3 and P2.5 are not optional.** A payload generator that gives every string a
value and every optional child an instance cannot reach any path conditioned on
emptiness, and a defect that lived exactly there passed every other payload in
the Java slice.

**P2.5 has two valid encodings, and the manifest records only one of them.** An
empty map *value* is an implicit-presence leaf holding the proto zero, so the
canonical form omits it; protobuf C++ and upb both write a map entry's key and
value unconditionally, at 2 B for the empty value. On P2.5 that is 40 emptied map
values across 20 elements, so those runtimes emit **19,712 B where the manifest
records 19,632**, and the hash does not match. Both forms are valid proto3, both
parse to the same map, and neither encoder is wrong.

Measured, not assumed: the C++ slice found it against protobuf C++ 3.21.12, and
the aggregating session confirmed it independently against **upb** (protobuf
7.36.2, upb backend) by round-tripping every committed payload — `+80` on P2.5
and byte-identical on P1.1, P1.3, P2.1, P3.1, P4.1 and P5.1. Two independent
Google runtimes, the same delta, so this is the family's behaviour rather than
one implementation's quirk, and `Google.Protobuf` and protobuf-java are expected
to follow it.

**What that changes is the oracle, not the payload.** "A slice that disagrees
with a hash has a defect in itself" is false here, and taken literally it would
have raised a false defect in the C#, Java and Python slices, on the one payload
whose purpose is the absent path. So on P2.5, and on any future payload with an
empty map value:

- a slice matches **either** the canonical hash or the both-fields-written form,
  and says which, because which one it produces is a fact about its incumbent;
- it must **parse** both, since a peer in another language will send the other;
- a cross-arm comparison inside a slice still requires byte identity **between
  that slice's own arms**, which is what R2 is actually protecting;
- a timing row on P2.5 says which form its incumbent wrote, because 19,712 B and
  19,632 B are not the same work.

The canonical form stays as it is. It is prost's rule, it is one of the two valid
answers, and changing it would break the arms that already agree with it to
accommodate an incumbent that is not in every language.

### Reading a figure from an existing report

The three published reports use a flat numbering that carried no message in it,
and this one is keyed to the message instead. The map, so that a figure quoted
from one of them is not silently attached to the wrong payload:

| Published as | Here | Published as | Here |
|---|---|---|---|
| P7 | P1.1 | P17 | P2.5 |
| P1 | P1.2 | P12 | P3.1 |
| P16 | P1.3 | P13 | P4.1 |
| P6 | P2.1 | P14 | P5.2 to P5.4 |
| P2 | P2.2 | P11 | P6.1 |
| P3 | P2.3 | P15 | P7.1 |
| P10 | P2.4 | P18, P19, P20 | content sets, below |

Three published payloads are deliberately not carried. **P8** (200 tasks with
200-byte error strings) and **P9** (40 tasks with 150 dependencies each) exist in
the C# appendix only, to make a length prefix change varint width; P2.4 covers
that case and is the harder one, because a per-call-site learned width is wrong
on every element of it by construction. Three further payloads were retired
before publication and their numbers were never reused, which is why the
published sequence has holes.

### Content sets

A payload says how many elements and of what shape; a **content set** says what
is in the strings. Every payload above is measured with the ASCII content set by
default, because every id, session id, task id, result id and partition name in
the real schema is an ASCII GUID.

Two further content sets exist and apply only to the arms that touch the string
path, where they are named in the table rather than assumed:

- **Latin-1 but not ASCII**, and **above U+00FF**. These are where a narrowing
  transcoder has real work to do or cannot represent its input at all, and they
  are what the Java slice's pinning oracle used (published as P18, P19 and P20).
- They also decide a measurement hazard rather than only a cost: on JDK 21 and
  later the first UTF-16 string to reach `String.charAt` anywhere in the process
  permanently deoptimises every char narrowing loop in it, and protobuf-java's
  own encoder is such a loop. An incumbent measured on ASCII only is measured in
  a state production is unlikely to be in.

A slice that reports one string-path number without saying which content set it
came from has reported half a number.

**And the halves are not the same size: the set moves encode by an order of
magnitude and barely moves decode.** Measured on P1.2 in C++, the C ABI against
protobuf C++ runs **0.988, 0.167, 0.114** on ASCII, Latin-1 and wide for
**encode**, and 0.656, 0.671, 0.546 for **decode**, with no payload's decode
ratio moving more than about 0.15. So an encode figure taken on ASCII alone is
not a figure about the string path at all, while a decode figure survives being
read without its set. Both still name their set; the encode one is the number
that changes meaning without it.

**A like-for-like encode comparison on a non-ASCII set needs the validating arm.**
protobuf C++ validates UTF-8 when it *serialises* — 37 unconditional
`VerifyUtf8String(..., SERIALIZE)` sites in its generated code — and ABI v1
decision 3 says the core does not. So most of that encode movement is a policy
difference rather than codec speed, and quoting the core against the incumbent
alone would report one as the other. With the validating transcoder in the table
the core is at parity on ASCII and about twice as fast on Latin-1 and wide.

**What these sets price is not the same thing in every host, and a column must
not be read across.** On .NET and the JVM the host holds UTF-16, so they make a
*narrowing transcoder* do real work or fail. A Rust `String` is already UTF-8, so
there is no narrowing and no transcoding at all: what changes there is the byte
width of the same character count and which path the *validator* takes. The Rust
slice's content-set rows therefore price **validation and width**, and a managed
slice's price a transcoder. Both belong in the report; neither is the other's
comparator. (Measured wire sizes, Rust slice: Latin-1 is 1.70 to 1.75 times ASCII,
above U+00FF is 2.39 to 2.50 times.)

**No manifest oracle covers them**, because `schema/` emits the ASCII set only. A
slice checks a content set by byte identity of all its arms against the
*incumbent* arm, which the manifest validated on ASCII, plus a decode round trip
per set.

The published wire sizes also differ between the C# and Java slices for what was
nominally the same payload (P7 is 958 B in one and 1,016 B in the other), because
the two generators were not driven by the same description. **Under R1 that is a
defect rather than a difference**, and the first slice to be rebuilt sets the
sizes every later one has to match.

## The RPC arm

Smaller, and deliberately so. Each slice measures one unary RPC carrying a real
payload (P2.2), against that language's gRPC incumbent, over loopback:

- CPU per RPC and allocation per RPC, at 1, 8 and 16 calls in flight;
- the crossing count per RPC (it should be two, not a function of field count);
- whether the language's idiomatic wait (a `Task`, a `CompletableFuture`, a
  coroutine, a blocking call) can be satisfied without pinning a carrier thread.

**Every slice measures against its own language's gRPC stack, end to end.** A
marshaller arm is not an RPC arm: calling `ProtoLiteUtils` or a `SerializationContext`
measures the codec path gRPC drives, which R14 asks for separately, and says nothing
about the transport. The comparison the report needs is the host's real gRPC client
against the core's, carrying P2.2.

**Prefer a Unix domain socket, with loopback TCP as a labelled second row.** A UDS
removes the TCP/IP stack from both arms equally, which is kernel time neither
implementation is responsible for and which varies with the machine. All five stacks
support it: `unix:` targets in grpc++ and grpcio, a `UnixStream` connector in tonic,
netty domain sockets in grpc-java, and `UnixDomainSocketEndPoint` behind a
`SocketsHttpHandler` connect callback on .NET.

**It does not rescue R9's hazard, because that hazard is HTTP/2's and not TCP's —
but the hazard is a property of one stack's DEFAULT rather than of the protocol, and
the earlier wording here was wrong about that.** 65,535 octets is the *initial*
stream window RFC 9113 mandates, and a window that is full throttles the sender until
`WINDOW_UPDATE` arrives; it never caps a message. Where each stack goes from there
differs, and it decides whether a wall-clock column is measuring the codec:

| stack | initial stream window | auto-tuning |
|---|---|---|
| tonic / hyper (the rust slice's) | 65,535 | **off by default** |
| grpc-java (Netty) | **1 MiB** (`DEFAULT_FLOW_CONTROL_WINDOW`) | **BDP, on by default since 1.30**; calling `flowControlWindow(int)` turns it off |
| .NET `SocketsHttpHandler` (what `Grpc.Net.Client` rides) | 65,535 | **dynamic sizing on by default**, to a 16 MiB cap |

So a 540 KB P2.2 response fits inside grpc-java's default window with no stall at all,
and stalls repeatedly on tonic's. **Each RPC arm states its stream and connection
window and whether auto-tuning is on**, in its configuration line, because two slices
that do not are not measuring the same thing.

**And every arm pins the same configuration, which is ArmoniK's rather than the
stack's** — R14 applied to the transport. From the core's current settings: **2 MiB
chunking** for upload and download, and a **4 MiB stream window**, sized to the
largest message the stack accepts by default so that one maximum-size message crosses
without a `WINDOW_UPDATE` round trip. P2.2's 540 KB is then comfortably inside one
window in every arm, which is what makes the arms comparable.

Two traps in pinning it, **and each applies to some stacks and not others, so a slice
establishes them from its own runtime's source rather than inheriting them**.

- **The connection window is a separate setting from the stream window** on grpc-java
  (`flowControlWindow` reaches `SETTINGS_INITIAL_WINDOW_SIZE`, per stream) and on
  tonic/hyper, so raising only the stream window there leaves the connection at 65,535
  and changes nothing. **Not on .NET**, where `Http2Connection` hardcodes a 64 MiB
  connection window and raises it at setup.
- **Pinning a window turns auto-tuning off on grpc-java and does not on .NET**, where
  the configured size is a starting point that doubles to a 16 MiB cap unless the
  `DisableDynamicWindowSizing` AppContext switch is also set — **a floor, not a cap**.

The pinned arm is the headline and the stack default is a labelled second row. CPU per
RPC stays the headline over both, and wall clock is reported beside it or not at all.
**Where pinning configures something the shipped client cannot** — as on .NET, where
`packages/csharp` sets no window and reaches gRPC through an `HttpClientHandler` that
cannot — the arm says so rather than diverging silently.

**Not in the RPC arm, and listed as not measured**: streaming, TLS, a real
network, failure injection, the server side. Streaming is where the concurrency
invariant actually bites, and no slice has touched it.
