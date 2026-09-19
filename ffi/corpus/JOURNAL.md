# corpus/JOURNAL.md

What was tried, what it produced, what refuted it, in order. The point of this
file is that a later reader finds out an option was already refused and by what.

## 1. Establishing the environment before designing anything

`protoc` is not on this container and `google.protobuf` is not installed, which
is why `ffi/schema/emit/wire.py` exists at all. `pip install protobuf
grpcio-tools` gives **protobuf 7.36.2 with the upb backend** and `python3 -m
grpc_tools.protoc` reporting `libprotoc 35.1`. That is a genuinely independent
second implementation and the corpus is built around having it.

Round-tripped `schema/generated/payloads/P1_1.bin` through upb over a descriptor
`protoc` compiled from `shapes.proto`: byte-identical. So the pairing works
before any corpus code existed.

## 2. Probing upb before writing a single vector

Nineteen hand-built cases, run against upb, to find out what is actually true
rather than what the wire format document says. The ones that changed the design:

- **Invalid UTF-8 in a string field is rejected** for all six sequences tried
  (lone continuation, truncated, overlong, surrogate, F5 lead, five-byte).
  `DecodeError`. So the decode half of the transcode pair can be validated here
  and not only asserted.
- **upb RETAINS unknown fields** and re-emits them, including a group with
  nesting. So an `unknown` vector's re-encoding has two legal answers -- retained
  and dropped -- and that is ABI v1 open decision 11 appearing as data rather
  than as prose. It is why every `unknown` row carries both forms.
- **upb writes a map entry's key and value unconditionally.** `map<string,string>`
  with an empty value re-encodes at +2 B per entry, which is exactly the P2.5
  `+80` SHAPES.md records, reproduced from first principles here.
- **upb reorders**: unknown fields written *before* the known ones come back
  appended. Another second form, not a defect.
- **A non-minimal varint is accepted and normalised**, and a proto3 negative
  `int32` really is ten bytes on the wire and round-trips identically.
- **A packed field written unpacked is accepted and re-emitted packed.**

Every one of these became a vector. Designing them from the specification and
checking afterwards would have produced a corpus that asserted its author's
reading of protobuf.

## 3. Why the corpus generates from a merged description rather than from `shapes.json`

Two things `shapes.json` cannot carry and should not be made to:

- the fields a reader is **not** built against. They have to exist in one
  descriptor and not in another, which is two `.proto` files out of one
  description, not one;
- a root whose repeated tag differs from a repeated or map tag inside its
  element. Putting it in `shapes.json` obliges five slices to implement a new
  facade message, one of which is already built.

So `corpus.json` is a delta merged over `shapes.json` at load, producing a
**reader** view and a **superset** view, and `ffi/schema/**` is read and never
written. `emit/rawwire.py` imports the schema's writer rather than copying it, so
the two directories cannot drift on varint framing; `values.py` is imported too,
so a slice that already implements the schema's value rules gets the corpus's for
free.

## 4. Refuted: one vector per field

The first plan for section 10 item 3 was a vector per (message, field): roughly
230 files of a message with exactly one field set. Refused, for two reasons. It
is a large number of files that say very little each, and -- the real reason --
**it does not actually prove the claim**. "Every field shape, mechanically" is a
statement about coverage, and the mechanical form of a coverage statement is a
coverage *check*, not a file count.

What replaced it: the encoder traces the shape key of every field it actually
writes, `emit/build.py` computes the set of shape keys the description contains,
and **a shape with no vector fails the build**. The vectors are then whatever it
takes to close that set -- every message full, at a second element index, and
minimal, plus the boundary sweeps.

It earned its place twice. The first run failed naming `bytes/singular/oneof` and
`message/singular/oneof`: `Probe`'s oneof has five members and the per-message
sweep reached one of them, at both element indices, because 5 mod 5 is 0. That is
R1's second half -- a walker that excludes oneof members emits a complete-looking
codec -- reproduced in the generator built to catch it. Fixed by a sweep over
oneof **members**, which is now 12 of the 145 `shape` vectors.

## 5. Two defects in this directory, both found by a guard rather than by reading

**D1. A `Timestamp` shortcut wrote unknown fields nowhere.** `enc_field` had a
special case returning `stamp_body(...)` for `Timestamp` and `Duration` without
re-entering `enc_message`, so an unknown field aimed at a two-field leaf was
written into nothing and `U-leaf-*` tested exactly nothing. Byte identity would
never have noticed: the vector was a perfectly good `ListResultsResponse`.

Found by the assertion in `emit/build.py` that a vector in class `unknown` must
have an unknown field that upb can *see*. Fixed by moving the stamp rule into
`enc_message` as a **value** rule, so everything after it -- the oneof, the
unknown fields -- still runs. The same defect shape as the field walker in
finding 4 above, in the code written to catch that.

**D2. A schema-wide unknown-tag set deleted a real field.** The unknown oneof
member is tag 15, chosen so that it sits next to `Probe`'s oneof members at
10--14 and *looks* like one. `TaskDetailed.pod_ttl` is also tag 15. The filter
that removed unknown tags from the reader's view of a message was a flat set, so
**`pod_ttl` was silently dropped from every `TaskDetailed` vector in the corpus**
-- including the ones built on the payload the control plane actually moves.

Found by a self-test asking whether any unknown tag is declared *at its own site*
in the reader view. Note what did not find it: a textual grep of `corpus.proto`
for `= 15;` passes, because `pod_ttl` is right there. Fixed by making unknown
tags per-message. Both defects are recorded because the class matters more than
the instances: a generator's shortcuts are invisible to byte identity, which is
the corpus's own thesis applied to itself.

## 6. Refuted: "upb's re-encoding must be one of the forms the vector declares"

The first version of the accepted-forms check failed the build whenever upb
wrote bytes the vector did not list. It fired on `U-map-entry`, and the
difference turned out to be **field order**: with an unknown field retained
inside a map entry, upb emits `TaskOptions`'s map field *after* its other fields
rather than in tag order. Without the unknown field it emits in tag order, and
`P2_1` re-encodes byte-identically.

Protobuf does not fix field order on the wire; the corpus's canonical form does,
because without one there is no byte identity to check. So the check became a
three-way classification: identical, a declared form, or -- only if re-parsing
upb's bytes projects to the same message -- **the same message encoded
differently**, recorded as an accepted encoding with a computed label. Anything
that does not re-parse to the same projection still fails the build, and a
**produce** vector still has to be identical or a declared form, because a
producer needs an unambiguous target.

Two rows land in the third case today: `U-map-entry` and
`U-oneof-member-before-known`, both field order, neither a produce vector.

## 7. Why projections exist at all

Byte identity was the oracle for `schema/` and it is not enough here, which is
the brief's own argument: a codec can round-trip bytes it has misunderstood, and
the element-run defect proved it in this branch. So every vector carries what a
reader must **see**.

Protobuf JSON was tried first and refused: proto3 JSON omits a default value,
which erases the difference between absent and present-and-zero, and that
difference is the whole of vector class `empty`. The projection is `ListFields`
semantics instead -- the fields that are *set* -- which every protobuf
implementation shares, with integers and enums as decimal strings because an enum
value the descriptor does not declare has no name to use.

Projections above 32 KB of JSON are omitted, which reaches only the deliberately
large runs; each names the small vector of the same shape that carries the
expectation instead. A projection of a 2048-element run is a third of a megabyte
saying the same thing 2048 times.

## 8. The gate, and the step that will bite a generator

`run.sh` regenerates, validates, self-tests and then **clones the branch to a
different path and runs `--check` there**. That is the python slice's D4: a
generated module carried the ffi root as an absolute path baked in at generate
time, so a clone at any other path regenerated a different file and the committed
sources could not rebuild themselves. It was found by cloning the pushed branch
and building it, not by reading the code.

`emit/selftest.py` is 25 checks and every one of them hands a guard input it is
supposed to refuse: five field shapes the walker has no case for, a second oneof
in one message, a duplicate tag, a tag in protobuf's reserved range, an encoder
option that does not exist, `--check` against a changed / missing / extra file,
the gap reporter with a shape removed, and a scan of every generated file for an
absolute path from this machine. A guard with no failing test is a guard nobody
has seen work.

## 9. What this session did not do

It did not modify any slice to consume the corpus, which is each slice's own
work. It did not touch `ffi/schema/**`, `ffi/design/**`, `ffi/findings/**`,
`ffi/README.md`, `ffi/CLAUDE.md`, `ffi/REPORT.md` or `ffi/poc/**`; seven things
that want changing outside `ffi/corpus/**` are requests in `STATE.md`.

And it did not get a second independent opinion on the vectors themselves. upb
parsed every one of them; prost, `Google.Protobuf` and protobuf-java have not.
**The first slice to consume the corpus is the second opinion**, and that is the
next real step.
