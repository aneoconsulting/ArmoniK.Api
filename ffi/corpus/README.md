# The conformance corpus (W8)

A fixed set of byte vectors, **generated from a descriptor rather than curated**,
checked in, and run as a gate. README section 10 is the specification and ABI v1
section 12.1 makes it a release gate. `CONTRACT.md` is what a slice has to do to
claim it passes; this file is what is here and why.

```
corpus.json              the corpus's delta over ../schema/shapes.json: the shapes
                         SHAPES.md deliberately does not have, and the fields a
                         reader is NOT built against
emit/spec.py             the merge, and the two .proto views of it
emit/rawwire.py          the schema's wire writer, plus what a corpus needs and a
                         canonical writer must never have
emit/encode.py           the descriptor-driven encoder. R1 lives here: a walker
                         with no case for a shape RAISES
emit/vectors.py          every vector, by class
emit/build.py            generate, validate against upb, write the manifest
emit/selftest.py         the guards, each one watched working
run.sh                   the gate
generated/               output. Regenerate it, never edit it
  corpus.proto             the READER view: what a conformant reader is built against
  corpus_superset.proto    the WRITER view: the reader's schema plus the unknown fields
  manifest.json            every vector: what it tests, produce/consume, accepted
                           forms, which hosts can run it
  vectors/*.bin            the bytes
  projections/*.json       what a reader must SEE
```

Run all of it:

```bash
pip install protobuf grpcio-tools       # protoc, and a upb-backed runtime
./run.sh                                # regenerate, validate, self-test, drift-check
./run.sh --check                        # the CI form
```

## Why this is not another payload set

`ffi/schema` already emits sixteen payloads with a validated manifest, and a
slice that matches every hash in it can still be wrong in three ways this branch
has actually seen:

- **An element run read the open field's tag from the context at entry**, so a
  host that chunked wrote its later chunks under the *inner* field's tag: silent
  wire corruption on every chunk after the first. **Byte identity passed**,
  because `ListTasksDetailedResponse.tasks` and `TaskOptions.options` are both
  tag 1 and the wrong tag was the right tag.
- **A field walker silently excluded oneof members**, emitting a complete-looking
  codec for a message whose oneof it ignored entirely.
- **An `Output` adapter** had one facade type with two wire forms and was wrong
  on the failure path only.

And a corpus generated from the schema that reads it never executes the
unknown-field skip, which is the whole of protobuf's forward compatibility.

So this directory is a **superset**: the same description plus fields the reader
does not know, plus shapes SHAPES.md has no instance of, plus wire a conformant
parser must refuse. It references `ffi/schema/generated`'s payloads rather than
copying them (rows `B-*` in the manifest).

## Two decisions this corpus is built on

**1. It does not have to be expressible in `SHAPES.md`.** Section 10 item 5 needs
a root whose repeated field carries a tag *different* from a repeated or map
field inside its element, and SHAPES.md has no such shape. Adding one there would
oblige five slices -- one of them a fifth already built -- to implement a new
facade message. So the corpus defines its own where it must, and **each vector
declares whether a slice is expected to produce it, only to consume it, or
both.** Coverage a slice cannot reach is recorded as a gap rather than forced.

**2. A vector may have more than one accepted encoding, and P2.5 is the proof.**
An empty map value is an implicit-presence leaf holding the proto zero: the
manifest omits it (prost's rule) and protobuf C++, upb and protobuf-java all
write it, at +80 B on P2.5. Both parse to the same message and neither encoder is
wrong. So a row carries a **set** of accepted encodings, each labelled and each
naming who was seen writing it, and a reader must parse every one of them
whatever it writes. `design/SHAPES.md` records this; the corpus generalises it.
85 of 336 rows have more than one form.

## What validates it

Not this directory. Every vector is parsed, re-encoded and projected by
**protobuf 7.36.2 with the upb backend**, over descriptors `protoc` compiled from
the emitted `.proto` -- an implementation that shares no code with the writer.
Two independent producers agreeing is worth far more than one producer and a
hash, and the same pairing is what established the P2.5 result in the first
place.

Three things **fail the build** rather than being reported:

1. an accept-vector upb will not parse, or whose re-encoding is neither a form
   the vector declares nor a legal re-ordering of the same message;
2. a reject-vector upb **accepts** -- a rejection test that nothing rejects is a
   test nobody has watched work, and that lesson cost this branch twice;
3. a field shape present in the description that **no vector exercises**. The
   coverage table is computed by walking the description and comparing it against
   what the encoder traced itself writing, so this is a mechanical claim rather
   than a prose one. It failed twice while the corpus was being built and named
   the shape both times.

`emit/selftest.py` then does to each of those guards what they do to the
vectors: hands it input it is supposed to refuse and fails if it does not. 25
checks, including that `--check` notices drift in both directions and that no
generated file carries an absolute path from the machine that wrote it.

## The vector classes

| Class | n | What it is |
|---|---|---|
| `unknown` | 74 | section 10 item 1. One of each wire type at seven sites, the deprecated group form, the largest legal tag, inside a map entry, at a oneof member's tag on a message that **actually has a oneof**, and unknown enum values on known fields |
| `empty` | 30 | section 10 item 2 and R6. Empty roots and elements, the absent path, present-and-zero at leaf depth, six map-entry degenerate forms, packed written unpacked and split, oneof and explicit-presence emptiness, and the adapter's non-injective site |
| `shape` | 145 | section 10 item 3, driven off the description. Every message full, at a second element index and minimal; **every oneof member**; varint width boundaries including the ten-byte negative int32; wire type 5, which `shapes.json` has no field of; double edge cases; string length and content-class boundaries; nesting depth |
| `transcode` | 53 | section 10 item 4, both halves |
| `chunking` | 8 | section 10 item 5, on the corpus's own shapes |
| `malformed` | 18 | wire a conformant parser must refuse, each one **seen** being refused |
| `baseline` | 8 | `ffi/schema/generated`'s committed payloads, by reference, with upb's opinion recorded beside prost's |

## The shape section 10 item 5 needed

`ChunkedResponse.items` is **tag 7**, and no repeated or map field inside a
`ChunkElement` is tag 7, transitively: the element's repeated field is tag 1, its
map is tag 2 and its inner message's packed and repeated fields are tags 1 and 2.
So a chunk written under the inner field's tag is a different byte sequence and
byte identity catches it. `ChunkedResponseWide` is the same shape at tag 70000, a
three-byte key, so a wrong tag also changes the *length* of every chunk after the
first. `LeafResponse` is the leaf-element form at tag 9, because a leaf element
goes through a different entry point (`ak_elem_X`) with the same hazard.

ABI v1 chunks element runs at 32 KB of host-side element **groups**, not of wire
bytes, so the chunk count is a function of the host's group size and cannot be
computed here. Each vector declares `elements`,
`min_group_bytes_for_two_chunks` and `chunks_if_group_is`, and `CONTRACT.md`
requires a slice to report the count it actually saw.

## Ownership

`ffi/corpus/**` and nothing else. If something outside it needs to change, it is
a request in `STATE.md`, not an edit. `ffi/schema/**` is read here and never
written: the corpus imports `schema/emit/shapes.py`, `values.py` and `wire.py`
rather than re-deriving them, so the two directories cannot drift on the value
rules or on varint framing.
