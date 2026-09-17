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
| M4 | `TaskSummary` | the `Output` adapter: one facade type with two wire forms, a nested message at one site and a plain string at another | real, and the only `with` adapter in the Rust crate |
| M5 | `UploadResultData` | two ids and one payload-sized `bytes` field | real |
| M6 | `MetricsBatch` | one string id and four packed repeated scalar fields (`int64`, `double`, `int32`, `bool`) | **control**: the schema has 3 packed fields and they are all enums |
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
| nested message, depth up to 6 | M2 | the schema's maximum static depth |
| repeated string | M2 | the shape the C++ slice could not see, having 2 of 11 repeated fields |
| repeated message, leaf element | M2 | the batched-run case |
| repeated message, non-leaf element | M2, M7 | the case batching must refuse, transitively |
| `map<string, string>` | M2 | and the ABI must not give it a case of its own |
| packed repeated scalar | M6 (control) | the host's own array, handed over whole |
| packed repeated enum | M2 | what the real schema actually has |
| oneof, including a payload-free member | M3 | the group does not reach it. Unmeasured on .NET today |
| open enum with an unknown value | M1, M2 | an unknown wire value must round-trip losslessly |
| an adapter site (`with`) | M4 | one facade type, two wire forms. Only a byte corpus catches a wrong one |
| unknown fields on the wire | corpus | protobuf's forward compatibility, never executed by a corpus generated from the schema that reads it |
| absent and empty everywhere | P1.3, P2.5 | where offset defects hide |

## Payloads

Defined by content rule rather than by byte size, because the byte size is an
output: **once every slice is driven by the same description, two slices
disagreeing on the wire size of a payload is a defect, not a difference.**

Sizes below are what `../schema/emit/payloads.py` produces today, and every
payload also carries a sha256 in `../schema/generated/manifest.json`. They are
provisional until the Rust slice checks them against prost.

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
| P7.1 | M7 | 3 + 3 | interleaved repeated fields of one type |

**P1.3 and P2.5 are not optional.** A payload generator that gives every string a
value and every optional child an instance cannot reach any path conditioned on
emptiness, and a defect that lived exactly there passed every other payload in
the Java slice.

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

**Not in the RPC arm, and listed as not measured**: streaming, TLS, a real
network, failure injection, the server side. Streaming is where the concurrency
invariant actually bites, and no slice has touched it.
