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

## The caveat that mattered, and how it was discharged

**Every hash in `generated/manifest.json` was provisional, and is no longer.**
They were produced by `emit/wire.py`, a protobuf writer written for this purpose,
which no protobuf implementation had checked. `emit/check.py` proves the framing
holds, which is the class of defect that writer could plausibly have, but it
cannot prove the semantics: it has nothing to compare against.

**The Rust slice validated them with prost, and that was its first task.** The
check found one defect, and it found it in this directory rather than in the
slice:

> `enc_field` wrote the two leaves of a `Timestamp` and a `Duration`
> unconditionally, the proto zero included, which contradicts the canonical form
> stated three lines above in this file and in `emit/wire.py`'s own docstring.
> Every other scalar path in `payloads.py` already guarded on the value. It moved
> 8 of the 16 payloads by 4 to 34 bytes, all of it on element 0, where the
> deterministic value rules happen to land on zero.

It is fixed as one `stamp_body()` helper rather than at the three call sites,
because the first spelling of the rule was wrong at all three. Confirmed by two
independent encoders (prost's derive, and `prost_reflect::DynamicMessage` over
the same descriptor), so the correction does not rest on prost's derive alone:
`ffi/logs/rust/stage1-manifest-vs-prost.log` and `stage1-second-encoder.log`.

**A slice that disagrees with a hash has now found a defect in itself.**

Two things the episode establishes, kept because they cost a session to learn:

- **Framing is not semantics.** `check.py` passed the whole time. A `nanos = 0`
  field is correctly framed; it simply should not have been there. A writer with
  nothing to compare against can only be checked by an implementation that shares
  no code with it.
- **The absent-path payloads did not catch it.** P1.3 was byte-identical
  throughout, precisely because everything in it is absent and the shortcut is
  never reached. What caught it was *present and zero*, the third case, at leaf
  depth inside a nested message. See below.

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

## Four things the payload set does on purpose

**The absent path.** `P1.3` is 300 elements that each encode to *nothing*, and
`P2.5` removes the adapter child and empties half the map values. A generator
that fills every field cannot reach any code conditioned on emptiness, and that
is where an offset defect hides: one such defect passed all seven standard
payloads in the Java slice.

**Present and zero, at leaf depth.** `timestamp(0).nanos` is 0 and
`duration(0)` is `(0, 0)`, so element 0 of every payload carrying a `Timestamp`
or a `Duration` exercises an implicit-presence leaf that is present in the value
and absent from the wire. That started as an accident of the value rules and is
now deliberate: it is the case that caught the only defect this directory has
had, and **a value rule may not be tuned so that no leaf lands on zero.**

**All three states of the adapter, at both of its sites.** `Output` is one facade
type over two wire forms and the map is not injective, which is the only reason M4
exists. An earlier version of this file filled `success` and `error`
independently, so every element carried (true, non-empty): a state
`TaskDetailed.Output`'s own comment forbids, that no adapter over {Ok, Error} can
represent, and which left the success state (true, empty) never generated at all.
The shape a byte corpus exists to catch a defect in was unreachable from the
payload set. `adapter_state()` now cycles Error, Ok, Invalid per element, so the
nested site carries all three and **the plain site carries the collision**: Ok and
Invalid both flatten to the empty string there, and one of them must come back
wrong whatever an adapter author picks.

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
