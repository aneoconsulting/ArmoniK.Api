# The shapes and payloads every slice implements

**Status: draft, work item W2.** Nothing is built against this until it is
agreed. Once it is agreed, a slice may add an arm but may not change a shape:
a column of the final table that covers a different set of shapes is not a
column, it is a second table.

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
| absent and empty everywhere | P16, P17 | where offset defects hide |

## Payloads

Defined by content rule rather than by byte size, because the byte size is an
output: **once every slice is driven by the same description, two slices
disagreeing on the wire size of a payload is a defect, not a difference.**

| Payload | Message | Elements | What it tests |
|---|---|---|---|
| P7 | M1 | 4 | small and flat: the call-rate case, where the per-message cost is spread thinnest |
| P1 | M1 | 1,000 | large and flat: strings and scalars only |
| P6 | M2 | 1 | small and nested: every field shape in one element |
| **P2** | M2 | 500 | **the shape the control plane actually moves. Read this column first** |
| P3 | M2, 30 elements in each repeated string field | 125 | repeated strings and nothing else |
| P10 | M2, alternating 3 and 150 repeated strings | 80 | element bodies alternating across a varint length boundary. The only payload that exercises the length-placeholder move |
| P11 | M6 | 200 | packed scalars. A control, and the string path's control too: 200 strings cannot move it |
| P12 | M3 | 200 | oneof and explicit presence |
| P13 | M4 | 200 | the adapter site |
| P14 | M5 | 1 | 36 B to 4 MB, upload and download |
| P15 | M7 | 3 + 3 | interleaved repeated fields of one type |
| P16 | M1, every string empty and every child absent | 300 | the absent path |
| P17 | M2, half the map values emptied and the output child removed | 20 | the absent path, nested |

P16 and P17 are not optional. A payload generator that gives every string a
value and every optional child an instance cannot reach any path conditioned on
emptiness, and a defect that lived exactly there passed all seven standard
payloads in the Java slice.

## The RPC arm

Smaller, and deliberately so. Each slice measures one unary RPC carrying a real
payload (P2), against that language's gRPC incumbent, over loopback:

- CPU per RPC and allocation per RPC, at 1, 8 and 16 calls in flight;
- the crossing count per RPC (it should be two, not a function of field count);
- whether the language's idiomatic wait (a `Task`, a `CompletableFuture`, a
  coroutine, a blocking call) can be satisfied without pinning a carrier thread.

**Not in the RPC arm, and listed as not measured**: streaming, TLS, a real
network, failure injection, the server side. Streaming is where the concurrency
invariant actually bites, and no slice has touched it.
