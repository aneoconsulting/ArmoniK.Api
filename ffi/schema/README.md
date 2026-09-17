# The schema description

`shapes.json` is the single description every slice generates from (README rule
R1). It is the source; everything under `generated/` is an
output, and editing one of those files is a defect.

```
shapes.json           the description: messages, fields, enums, oneofs, payloads
emit/shapes.py        load it, and answer the questions every backend asks
emit/values.py        the deterministic value rules
emit/wire.py          a small protobuf writer, see the caveat below
emit/proto.py         -> generated/shapes.proto
emit/payloads.py      -> generated/manifest.json and generated/payloads/*.bin
emit/check.py         walks every payload as raw protobuf and proves the framing
generated/            output. Regenerate it, never edit it
```

The directory names avoid `gen/` and `out/` deliberately: the repository's
`.gitignore` matches both, so a generator written there is committed as nothing
at all and a slice agent finds an empty directory.

Run all of it:

```bash
python3 emit/proto.py && python3 emit/payloads.py && python3 emit/check.py
```

No dependencies, Python 3.8 and up. This container has neither protoc nor a
protobuf runtime, which is also why `emit/wire.py` exists.

## Ownership

`ffi/schema/**` is **shared**, and the aggregating session owns it. A slice agent
reads it and may add a generator backend for its own language under
`ffi/poc/<lang>/gen/`, importing `emit/shapes.py` rather than re-parsing the
description. **A slice never edits `shapes.json`**: a shape that is wrong is a
finding, because changing one silently turns a column of the final table into a
different table.

## The caveat that matters

**Every hash in `generated/manifest.json` is provisional.** They were produced by
`emit/wire.py`, a protobuf writer written for this purpose, which no protobuf
implementation has checked. `emit/check.py` proves the framing holds, which is the
class of defect that writer could plausibly have, but it cannot prove the
semantics: it has nothing to compare against.

**The Rust slice validates them with prost, and that is its first task.** Until
it does, a slice that disagrees with a hash has found a defect in this directory
rather than in itself.

## Canonical form

Every slice must reproduce this, and two slices disagreeing on the bytes of a
payload is a defect rather than a difference:

- fields ascending by tag, the one deliberate exception being `DualResponse`,
  which interleaves because that is what it is for;
- an implicit-presence leaf holding the proto zero is omitted;
- an explicit-presence field is written when set, zero or not;
- a message field is written when present, empty or not;
- repeated scalars are always packed;
- map entries are sorted by key. Protobuf does not define an order, so one is
  fixed here: without that there is no byte identity to check, and sorted is what
  a deterministic serializer already produces.

## Two things the payload set does on purpose

**The absent path.** `P1.3` is 300 elements that each encode to *nothing*, and
`P2.5` removes the adapter child and empties half the map values. A generator
that fills every field cannot reach any code conditioned on emptiness, and that
is where an offset defect hides: one such defect passed all seven standard
payloads in the Java slice.

**Explicit presence that is sometimes absent, and sometimes present and zero.**
`Probe`'s three `optional` fields cycle their presence per element, and one
element in seven carries a present-but-zero value. A by-value group reports
absent and empty identically unless it is designed not to, and that is the shape
the C# slice could not measure because its schema did not contain one.

## What is not here yet

The conformance corpus (W8). It is a superset of this: the same payloads plus
**fields the reader does not know**, which a corpus generated from the schema
that reads it can never contain, and the transcode pair. `emit/wire.py` is the
thing to build it from, once something has validated it.
