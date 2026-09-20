== R14: the headline against the path ArmoniK actually runs ==

R14: "The baseline is what ArmoniK runs, not the library's best entry point. Almost nothing
in `packages/` serialises directly; gRPC's generated marshaller does, so that is the path
every ratio is against. [...] Where they differ, the headline is production's and the best
path is a labelled second row."

Every other table in this slice is against `toByteArray`. This one is against the real
`io.grpc.protobuf.lite.ProtoLiteUtils` marshaller -- the actual object, obtained from the
grpc-java 1.74.0 on the classpath, not an imitation of it. Raw table: `r14.log`.

## What the marshaller does, read from its bytecode

    ProtoInputStream.drainTo(OutputStream):
        MessageLite.getSerializedSize()
        MessageLite.writeTo(OutputStream)

    MessageMarshaller.parse(InputStream):
        if it is our own ProtoInputStream -> return the same object   [not a parse]
        if KnownLength -> read into a thread-local reusable byte[], parse from the array

So **encode** is the size pass plus a write through a 4 KB `CodedOutputStream` that flushes
to the stream, where `toByteArray` allocates an exact array and writes it in one pass. And
**decode** is this slice's existing `parseFrom` baseline plus one copy.

The `parse` fast path matters: handed back its own stream it returns the same object and
measures nothing. The arm hands it a real `KnownLength` stream, which is what the transport
does.

## 1. Encode: the production path is 1.5 to 1.9 times slower than `toByteArray`

`pbj-tba` as a fraction of `pbj-grpc`, median: 0.48 to 0.65 on every element-bearing
payload. **That is the R14 finding itself** -- the entry point every benchmark reaches for
is roughly twice as fast as the one the application takes, and nothing in the branch's
three published reports says which was measured.

| payload | `ffi-sink` | `R-sink` | `pbj-tba` (the second row R14 asks for) |
|---|---|---|---|
| P1.1 | 1.129 | 0.698 | 0.484 |
| P1.2 | **0.653** | 0.779 | 0.554 |
| P1.3 (absent) | 1.963 | 0.432 | 0.607 |
| P2.1 | 0.782 | 0.619 | 0.507 |
| **P2.2** | **0.835** | 0.840 | 0.604 |
| P2.3 | **0.652** | 0.873 | 0.625 |
| P2.4 | **0.595** | 0.850 | 0.542 |
| P2.5 | 0.882 | 1.043 | 0.647 |
| P3.1 | 0.753 | 1.197 | 0.646 |
| P4.1 | 0.667 | 0.713 | 0.637 |
| P6.1 (control) | 1.088 | 0.635 | 0.755 |

Every arm writes into the same reused sink and every protobuf message is serialised once.

**The verdict does not change and it is not weakened.** Against `toByteArray` the C ABI
encodes at 0.58 to 0.96 on the element-bearing payloads; against the path production takes
it is **0.60 to 0.88 on the same set**, with P1.1 and P6.1 above parity. The absent path is
still the inversion and decision 9's fill is still what fixes it.

## 2. Decode: the marshaller costs 2 to 8 percent over `parseFrom`, and nothing else moves

`pbj-parseFrom` as a fraction of `pbj-grpc` is **0.92 to 0.98** on every element-bearing
payload, which is the one extra copy the bytecode predicted. The decode tables did not need
re-taking, and that is now a measurement rather than a claim.

| payload | `ffi` | `R` |
|---|---|---|
| P1.2 | 0.809 | 0.911 |
| P2.1 | 1.268 | 0.790 |
| P2.2 | **1.395** | 0.823 |
| P2.3 | 1.286 | 0.980 |
| P2.4 | 1.223 | 0.997 |
| P2.5 | **1.623** | 0.817 |
| P4.1 | 1.343 | 0.716 |

Unchanged in shape from the `parseFrom` tables: **the C ABI is a regression on every M2
payload and arm R is not**, which is the pull-family argument.

## 3. What this table is not

It is **noisier than the main tables**: 24 rounds rather than 36 or 40, a protobuf pool
rebuilt between every round, and a fourth arm. Several rows have a max more than twice
their median (P1.1's `R-sink` reaches 8.486). **Its conclusions rest on its medians agreeing
with the main tables, which they do on every element-bearing payload, and not on its own
precision.** Where the two disagree, neither is quoted.

The sink is a pre-sized reused array, not netty's pooled `WritableBuffer`. Draining into a
`ByteArrayOutputStream` would have charged the incumbent a growth policy this harness
invented; the real allocator is not modelled at all, and every arm writes into the same
sink so whatever it costs, it costs all of them equally.
