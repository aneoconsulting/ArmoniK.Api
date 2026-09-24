# What a slice has to do to claim it passes the corpus

Five slices will implement this and they should not each invent it. This
document is the whole obligation; everything else in this directory is how the
vectors were built and why.

The corpus is not a hash of itself. **Byte identity against a manifest generated
from the same schema that reads it is a weaker oracle than it looks**, and this
branch has the receipts: an element run read the open field's tag from the
context at entry, so a chunking host wrote every chunk after the first under the
*inner* field's tag, and byte identity passed throughout because the outer
repeated field and the inner map field were both tag 1. So a slice does four
things with each vector, not one: it parses, it *projects*, it re-encodes, and
where the vector must fail it watches it fail.

**And the corpus is not its own authority either.** Its first version decided
every projection, every accepted encoding and every accept/reject verdict with
one runtime, upb, which makes that runtime the specification; the first two
consumers found a row where it is the minority. Three runtimes now answer, and
where they disagree the row says so instead of picking. See section 1.5.

## 0. The one rule that makes the rest mean anything

**Generate your codec from `generated/corpus.proto`, never from
`generated/corpus_superset.proto`.**

The superset is what the vectors were *written* against. The reader is what a
conformant reader is *built* against. The difference between the two files is
the corpus's entire unknown-field claim, and a slice that generates from the
superset has a codec that knows every field, executes no unknown-field skip, and
passes class `unknown` while testing nothing. That is the failure this corpus
exists to prevent, one level up.

`corpus_superset.proto` is in the tree for exactly one purpose: so that a slice
can, if it wants, build a **second, separate** decoder from it and check that
what the reader skipped is what the superset reads. `projection_superset` in the
manifest is that expectation, already computed.

## 1.5 Which runtimes decided this, and what a DISPUTED row means

The manifest's `oracles` section names them; `emit/build.py`'s docstring says why
each is asked what it is asked.

| Oracle | Asked | Independence |
|---|---|---|
| **protobuf, upb backend** | the structural checks, one reading, one re-encoding | a C parser, no code shared with the corpus's writer |
| **protobuf, pure-python backend** | a second reading and re-encoding, **in the same projection format** | a genuinely different parser reached through the same API. Less independent than protobuf C++; far easier to diff, because a disagreement is a JSON difference rather than an argument |
| **protobuf C++**, via `protoc --decode` | the accept/reject **verdict** on every row, and its text reading as evidence | fully independent |

protobuf C++ is deliberately **not** asked for the projection. `protoc --decode`
renders a map field as its wire-level repeated `MapEntry` list -- on
`E-map-dup-key` it prints two entries with the same key, which a map cannot hold
-- so it cannot answer a map-semantics question. The cpp slice's reflection arm
can, because `Reflection::ListFields` over generated code *is* the presence rule
section 3 describes, and that is where protobuf C++'s reading of `U-map-entry`
comes from.

**Every row carries a `verdict`.**

- `"agreed"` -- every oracle asked read the vector the same way.
  `verdict_agreed_by` names them, and `projection` is that shared reading.
- `"disputed"` -- they did not. The row has **no `projection`**; it carries
  `dispute.readings`, one per reading, each naming the runtimes that produced it
  and pointing at its own projection file, plus `dispute.differs_at`, the dotted
  paths the readings differ on.

**A consumer excludes a disputed row from its pass or fail count.** It does not
fail it and it does not silently drop it: it reports it as disputed, with which
reading its own codec produced. That is data the branch wants -- a sixth opinion
on an open question -- and it is not a verdict on the slice.

Three rows are disputed today. Two are about the VERDICT and are described
under C4 (`X-tag-zero-Empty`, `X-tag-zero-nested-Empty`). The third is about the
READING: **`U-map-entry`**, an unknown field inside every map
entry. upb drops the entry from the map and keeps its bytes as an unknown field
of the *parent*; protobuf-python's pure backend and protobuf C++ both put the
entry in the map. A map field is shorthand for a repeated `MapEntry` message, and
an unknown field inside a submessage is skipped while the submessage still
parses, so upb looks like the minority -- but the corpus does not say so, because
"two of the three I happened to ask" is not a specification either.

## 1. Read the manifest

`generated/manifest.json`. Every path in it -- `file`, `projection`,
`projection_superset` -- is **relative to the manifest's own directory**. Rows
whose `file` starts with `../../schema/` are the payloads of `ffi/schema`,
referenced rather than copied: the corpus is a superset of that payload set and
does not carry a second copy of it.

Each row carries:

| Field | What it obliges |
|---|---|
| `expect` | `accept` or `reject` |
| `produce` | the slices that must EMIT these exact bytes |
| `consume` | the slices that must PARSE them (all five, always) |
| `verdict` | `agreed` or `disputed`; see section 1.5 |
| `accepted_encodings` | every encoding a conformant implementation may write, each saying **who was seen writing it** |
| `permutation_accepted` | true where a re-encoding may be any re-ordering of an accepted form |
| `projection` | what a reader must SEE, or `null` -- with a named twin on a large row, with `dispute` on a disputed one |
| `meta` | per-class facts a slice needs: chunk arithmetic, the UTF-16 input, the bad UTF-8 bytes |
| `notes` | per-language exceptions, keyed by slice name or `all` |

## 2. The five obligations

### C1 -- parse every accept vector

For every row with `expect: "accept"`: decode `file` as `root`, with the codec
you generated from `corpus.proto`. It must succeed. There are 548 of them and a
slice that skips one records it by id.

### C2 -- project it

Where `projection` is not null, the decoded message must equal that JSON under
the encoding in section 3. **This is the obligation that byte identity does not
imply** and the reason the corpus carries projections at all: a codec can
round-trip bytes it has misunderstood. 542 rows carry one, and each is a reading
**two runtimes agreed on**, not one runtime's opinion.

`projection` is null in three different situations and they are not the same
obligation:

| Why | What you do |
|---|---|
| `verdict` is `disputed` | exclude the row from pass/fail, and **report which of `dispute.readings` your codec produced** |
| `projection_omitted` is present | one of the deliberately large runs. `projection_omitted.semantic_oracle_is` names the small vector of the same shape that carries the expectation; byte identity plus `meta.elements` is the oracle here |
| the row is `baseline` and large | the same, against `ffi/schema`'s own manifest |

### C3 -- re-encode it, and say which form you wrote

Re-encode what you parsed. The bytes must hash to one of
`accepted_encodings[*].sha256` -- **or, where `permutation_accepted` is true, to
any re-ordering of one of them**: protobuf does not fix field order on the wire,
so the same fields in a different order are the same encoding of the same
message. Four rows are permutation-accepted today, and `B-P7_1` is why the rule
exists: it interleaves two repeated fields on purpose, so its committed bytes are
a form **no conformant encoder produces** -- every one of them writes each
repeated field contiguously -- and a consumer that re-encoded correctly used to
fail this clause. `design/SHAPES.md` already validated P7.1 by permutation; the
manifest does now too.

**A vector may have more than one accepted form and that is not a weakness in the
vector.** An empty map value is an implicit-presence leaf holding the proto zero:
prost omits it, protobuf C++, upb and protobuf-java write it, both parse to the
same map and neither encoder is wrong. 334 rows have more than one form. Your
slice records which one it wrote, because which one it writes is a fact about its
incumbent, not a verdict.

Each form carries its **provenance**, and the two claims it can make are not the
same strength:

| Field | Claim |
|---|---|
| `forms` | what the form is, in words |
| `written_by` | who was seen producing exactly these bytes |
| `observed_in_a_protobuf_runtime` | **false** means only the corpus's own writer produced it: the form is asserted to be valid, and no protobuf runtime asked here was seen writing it |

334 rows carry at least one form in that weaker category. "upb writes this form"
and "every conformant encoder writes this form" are different sentences and the
manifest used to make only the first.

### C4 -- refuse every reject vector, and prove you watched it refuse

For every row with `expect: "reject"` (143 of them): your decoder must return an
error. Not a crash, not a partial message, not a silently truncated one.

**Record the error you actually got, per vector id.** A rejection test that
nothing rejects is a test nobody has watched work, and that lesson cost this
branch twice. `reject.seen_failing` names every runtime watched refusing each
vector and the error each one raised. **All three refuse 141 of the 143**, so on
those "my decoder accepts this" is a finding in the slice and not a doubt about the
vector.

**The other two are disputed, and excluded from pass or fail like any disputed
row.** `X-tag-zero-Empty` and `X-tag-zero-nested-Empty` carry field number 0 in a
message that has no fields. protobuf-python's pure backend and protobuf C++ refuse
both; **upb accepts both**, keeping field 0 as an unknown field -- on a message
with no fields only, since it refuses field number 0 on every other root in the
corpus. Report which way your decoder went, as for any disputed row.

`X-lenwrap-*` (63) are lengths that wrap 2^64 from their own position: a varint
of 2^64 - pos and its neighbours, where `pos` is the offset just after the length
varint, at an unknown, a string and a message field, and inside a nested message
counted both from the start of the buffer and from the start of the enclosing
message, because a decoder with one reader over the buffer and one with a
sub-reader per message wrap at different values. `meta` gives the declared length,
the offset it is counted from and where the sum lands. A decoder that checks
`pos + n > len` passes the check when the sum wraps; the check that cannot wrap is
`n > len - pos`. `X-lenwrap-lrr-unknown-zero` is FIX-PLAN WP4 item 1's 11-byte
reproducer. A wrapping decoder does not always fail visibly: the `zero` and
`start` modes jump backwards and loop, so run them under a timeout.

`X-tag-zero-<Root>` (30) put field number 0 after the full canonical message of
every message the corpus uses as a root, so a decoder that treats key 0 as the
end of the message returns a complete-looking message instead of failing.

Two of these are about a *limit* rather than about malformed bytes:
`X-depth-101` and `X-depth-300`. A decoder that recurses without one does not
fail them, it exhausts its stack, which on a native core is a crash inside the
host's process. That is ABI v1 open decision 7 and no slice exercises it today.

### C5 -- produce what you are asked to produce

For every row where your language is in `produce`: build the message and encode
it with your own facade. The bytes must equal one of `accepted_encodings`. Say
which.

**An empty `produce` is not a coverage gap by default.** A codec cannot emit a
field it does not know, and a must-fail vector cannot be emitted at all. Where
`produce` excludes a slice for a reason that is about that slice, `notes` says
so by name.

## 3. The projection encoding

Not protobuf JSON. Proto3 JSON omits a default value, which erases the
difference between absent and present-and-zero, and that difference is the whole
of vector class `empty`. A projection is the message's **set fields**, in the
sense of `ListFields` / `HasField` / a populated repeated field -- semantics
every protobuf implementation shares:

- an implicit-presence scalar equal to its default **does not appear**;
- an explicit-presence field that is set **does appear**, zero or not;
- a message field that is present appears, empty body or not;
- a repeated or map field appears when it has at least one element.

Values encode as:

| Kind | As |
|---|---|
| `string` | a JSON string |
| `bytes` | a lowercase hex string |
| every integer, and every **enum** | a decimal string. An enum value the descriptor does not declare has no name to use, and `"999"` is the point of several vectors |
| `bool` | JSON `true` / `false` |
| `double` | a string, `%.17g` |
| message | an object |
| repeated | an array |
| map | an object, keys stringified |

Unknown fields, where an implementation retains them, appear under `_unknown` as
a list of `{tag, wire_type}` plus `hex` (wire type 2), `value` (a decimal
string), or `group` (a nested list). **Comparing `_unknown` is optional**:
whether the core retains unknown fields is ABI v1 open decision 11 and a
behaviour change for four of the five languages. Everything outside `_unknown` is
not optional.

## 4. What each class additionally demands

### `unknown` (317 vectors)

Whether you retain unknown fields or drop them, your re-encode must be one of
`accepted_encodings`: both the retained and the dropped form are there, labelled.
**Say which you did.** That is the answer to ABI v1 open decision 11 for your
language, and nobody has written it down.

Then check the skip actually happened rather than assuming it: the fields under
`unknown_tags_seen_by_reader` are the ones your decoder must have walked past
without a case for them.

`U-oneof-member-*` are the three vectors the phrase "oneof-shaped unknown field"
usually means and almost never tests. A parser cannot tell an unrecognised oneof
tag from any other unknown field, because the grouping lives only in the
descriptor: the case stays at the last *known* member and the payload is dropped.
`U-oneof-member-before-known` is the one that looks like it works.

`U-wire-*` (243) are a KNOWN field number at a wire type its kind does not use:
for every message, the first field of each shape it has, at every wire type
among 0, 1, 2 and 5 its kind cannot arrive as (a packed field's unpacked form is
not foreign). All three oracles accept every one, reading the field as an
unknown field, because they dispatch on the (field number, wire type) pair. The
foreign-typed field comes after the full canonical message, so a decoder that
dispatches on the field number alone overwrites a set field, appends to a
repeated one or switches a oneof, and fails C2. `unknown_tags_seen_by_reader` on
these rows is a known field number; `meta.wrong_wire_type` says which field,
which wire type was declared and which was sent.

### `empty` (30 vectors)

Nothing extra, and that is the point: a generator that fills every field cannot
reach any path conditioned on emptiness, and a defect that lived exactly there
passed all seven standard payloads in the Java slice. If your harness
short-circuits an empty buffer before it reaches the decoder, these vectors pass
without executing anything.

### `shape` (163 vectors)

`manifest.shape_coverage` maps every field shape in the description to the fields
that have it. The corpus's own build **fails** if a shape has no vector; your
slice's obligation is the same claim one level down, and a shape your backend has
no case for must **raise**, never skip. That rule is a rule because a field
walker that silently excluded oneof members emitted a complete-looking codec for
a message whose oneof it ignored entirely, and reported nothing wrong.

`S-neg-*` (14) put negative int32 and int64 values on the roots SHAPES.md and
every slice already implement (singular, explicit presence, oneof, packed, three
levels down), each with a projection, so a decoder that forgets to sign-extend
fails C2 and not only C3. `S-varint-*` already did this on `WireZoo`, which a
codec generator with no `fixed32` case cannot reach at all. Three of them are
consume-only: a negative int32 written as a five-byte varint, and an int32 field
carrying a varint wider than 32 bits. An int32 reads the low 32 bits and
sign-extends from bit 31; the canonical re-encoding is the ten-byte form.

`S-mzero-*` (4) put -0.0 in `MetricsBatch.values`, the one repeated double in the
schema. The projection of -0.0 is `"-0"`. The schema has no `float` and no
explicit-presence double, so neither has a vector; the implicit-presence double
is `S-double-minus-zero`, on `WireZoo`.

### `transcode` (53 vectors)

Two halves, and they are not run by the same set of slices.

**The encode half** (`T-enc-*`, 7 vectors). `meta.input_utf16le_hex` and
`meta.input_code_units` are the input a host holds; the vector bytes are what the
core must put on the wire, substituting **U+FFFD per unpaired surrogate**. Four
string sites at once, because a transcoder wired at only some of its call sites
passes a root-only vector. `T-enc-valid-pair` is the control: a transcoder that
substitutes per code unit fails that one and passes every other.

`produce` is `csharp` and `java`. A Rust `String` cannot hold an unpaired
surrogate, so the input does not exist in that host's type. `meta` also carries
`observed_today_protobuf_java_hex`: design/ABI-v1.md section 6 records
protobuf-java writing `?` where Google.Protobuf writes U+FFFD, so the Java
slice's *incumbent* arm is expected to produce that instead, and the divergence
is a migration note rather than a defect. **The corpus does not verify that
claim.** The Java slice is the only thing that can.

**The decode half** (`T-dec-*`, 31 vectors) is every slice's: the wire holds
malformed UTF-8 in a string field and a conformant parser must reject it. Sites
are the root, a nested message, a map key, a map value and the second element of
a repeated field, because a validator wired at the root only passes `T-dec-root-*`
and fails the rest.

`T-bytes-*` (15 vectors) are the same bytes in a `bytes` field and must be
**accepted**. They are the control that keeps the reject half from being vacuous.

### `chunking` (8 vectors)

These are the ones SHAPES.md cannot express. The root's repeated field tag is
distinct from every repeated and map tag inside its element, transitively
(`meta.root_field_tag` against `meta.inner_repeated_and_map_tags` and
`meta.inner_tags_any_kind`), so a chunk written under the inner field's tag is a
*different byte sequence*.

**If your host batches element runs, you must run these with more than one
chunk, and report the count you saw.** ABI v1 chunks at 32 KB of host-side
element *groups*, not of wire bytes, so the corpus cannot compute the count for
you: `meta.min_group_bytes_for_two_chunks` and `meta.chunks_if_group_is` give the
arithmetic against your group size. A slice that runs `C-elemu-512` in one chunk
has not exercised README section 10 item 5 and records that as a gap rather than
as a pass. A host that declines to batch says so, and its gap is that it cannot
reach this class at all.

`C-mixed-100` has an element encoding to nothing every tenth position, so a run
carrying state across elements has to survive an element that writes no bytes.

### `malformed` (112) and `baseline` (8)

Malformed is C4. Baseline rows are `ffi/schema/generated`'s payloads by
reference; `upb_reencodes_identically` and `meta.delta_bytes` record upb's
opinion of each beside prost's, and `B-P2_5` is the `+80` that made the accepted
forms machinery necessary.

## 5. What "it passes" means

A slice claims the corpus when, for every row in the manifest:

0. every row whose `verdict` is `disputed` is **excluded** from the pass/fail
   count and reported as disputed, naming which of `dispute.readings` the slice's
   own codec produced;
1. every other `accept` vector parses, and projects equal where a projection
   exists;
2. every `reject` vector is refused, **and the slice's log names the error it
   got for each one**;
3. every vector whose `produce` names the slice reproduces one of the accepted
   encodings, and the log says which;
4. every `chunking` vector was run at more than one chunk, or the slice says it
   does not batch;
5. byte identity still holds **between that slice's own arms** on every vector
   (R2: that is what R2 is actually protecting);
6. and every vector it could not run is listed **by id, with a reason**, in the
   slice's `STATE.md`. R11: a slice that does not name its gaps is not finished.

A slice that finds a row where its own reading differs from every reading in the
manifest has found either a defect in itself or a new dispute. It reports it as a
finding either way, with the bytes and both readings, and does not adjust itself
until someone has looked. That is how `U-map-entry` arrived.

Anything less is a partial claim and is reported as one.

## 6. What the corpus does not check

Timing, crossing counts, allocation, the ABI's C symbols, the concurrency suite
of ABI v1 obligation 12.5, and the worker path's build-time subset check of
obligation 12.4. It is a correctness artifact and it says nothing about any
number.

It also does not check the **content sets**: `ffi/schema` emits ASCII only, and
`S-string-latin1`, `S-string-wide` and `S-string-astral` are single vectors
rather than a content-set sweep of the payload set. A slice checks a content set
the way SHAPES.md says -- byte identity of all its arms against the incumbent
arm, plus a decode round trip per set -- and the corpus does not replace that.
