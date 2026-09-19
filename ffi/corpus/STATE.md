# corpus/STATE.md

The handoff for W8. Read this first; the transcript is gone.

**Status: built and gated.** `ffi/corpus/` holds a generator, 336 vectors, a
manifest, projections, a consumer contract and a gate. `./run.sh --check` passes
from a clean clone at a different path. **No slice consumes it yet**, which is
each slice's own work and deliberately not done here.

## What exists

| | |
|---|---|
| Generator | `emit/` -- 6 modules, driven off `corpus.json` merged over `../schema/shapes.json` |
| Vectors | 336: 74 `unknown`, 30 `empty`, 145 `shape`, 53 `transcode`, 8 `chunking`, 18 `malformed`, 8 `baseline` |
| Of those | 287 must be accepted, 49 must be **refused** |
| Manifest | `generated/manifest.json` -- per vector: what it tests, produce/consume per slice, the **set** of accepted encodings, a projection, per-class metadata, per-language notes |
| Projections | 336 + 65 superset projections. What a reader must SEE, not just bytes |
| Contract | `CONTRACT.md`. The whole obligation, so five slices do not each invent it |
| Gate | `run.sh` / `emit/build.py --check` / `emit/selftest.py` |

Validated against **protobuf 7.36.2, upb backend**, over descriptors `protoc`
35.1 compiled from the emitted `.proto`. An implementation that shares no code
with the writer, which is the whole point: a corpus validated only by its own
writer is a hash of itself.

## The two decisions this was built on, restated because they are load-bearing

1. **The corpus is a superset and does not have to be expressible in
   `SHAPES.md`.** Section 10 item 5 needs a root whose repeated field tag differs
   from a repeated or map tag inside its element; SHAPES.md has none, and adding
   one would oblige five slices to implement a new facade message. So the corpus
   carries `ChunkedResponse` (root tag 7, inner tags 1 and 2),
   `ChunkedResponseWide` (tag 70000, a three-byte key), `LeafResponse` (the leaf
   entry-point form), `Surrogate`, `WireZoo` and `Nest` of its own. Every vector
   declares produce / consume per slice.
2. **A vector may have more than one accepted encoding.** 85 of 336 rows do.

## Covered

- **Unknown fields (item 1).** One of each proto3-expressible wire type at seven
  sites -- root, a nested element, three levels down on the message that carries
  the map, a two-field leaf, a message that **actually has a oneof**, the element
  of P2.2's message, and the corpus's own non-leaf element -- plus all seven at
  once at each site; before, after and interleaved with the known fields; the
  deprecated **group** form (wire types 3 and 4, which proto3 cannot express, so
  no schema-generated corpus contains one); the largest legal field number; an
  unknown field **inside a map entry**; three unrecognised-oneof-member cases;
  and unknown enum **values** on known fields, singular and inside a packed run.
- **Absent and empty (item 2, R6).** An empty root, empty elements, the
  `all_absent` and `half_absent` modes, present-and-zero at leaf depth, six
  degenerate map-entry forms, packed written unpacked / split / empty, oneof and
  explicit-presence emptiness, and the `Output` adapter's three states at both
  sites including the plain site's non-injective collision.
- **Every field shape (item 3), mechanically.** Every message full, at a second
  element index, and minimal; **one vector per oneof member**; varint width
  boundaries including the ten-byte negative proto3 int32; wire type 5, which
  `shapes.json` has no field of at all; double edge cases (minus zero, NaN,
  denormal); string length boundaries at 127/128 and 16383/16384 and nine content
  classes; nesting depth 1, 4, 6 and 20. The manifest's `shape_coverage` is
  computed from the description and the build **fails** on a gap.
- **The transcode pair (item 4), both halves.** 7 encode-half vectors carrying
  the UTF-16 input as metadata and the ABI's U+FFFD substitution as bytes, at
  four string sites at once, with a valid-pair control; 31 decode-half rejects
  over fifteen malformed-UTF-8 sequences at five sites; 15 controls putting the
  same bytes in a `bytes` field, which must be accepted.
- **Distinct tags and chunking (item 5).** 8 vectors, non-leaf and leaf element
  forms, a three-byte root key, and a run with an empty element every tenth
  position. Chunk arithmetic is explicit per vector.
- **Refusals.** 49 vectors a conformant parser must reject, every one of them
  **seen** being rejected by upb, with the exception it raised recorded in the
  manifest.

## Not covered, and why

1. **No slice consumes it.** By design: that is each slice's work. Until one
   does, the corpus has been validated by upb and by nothing else.
2. **One independent implementation, not two.** upb parsed and re-encoded every
   vector. prost validated `schema/`'s manifest but has never seen these
   vectors, and neither have `Google.Protobuf` or protobuf-java. **The first
   slice to consume the corpus is the second opinion**, and until then a
   systematic misreading shared by this generator and upb would survive.
3. **The encode half of the transcode pair has not been RUN.** The corpus states
   what the core must put on the wire; nothing has produced those bytes from a
   UTF-16 host. The `?`-against-U+FFFD divergence is quoted from
   `design/ABI-v1.md` section 6 and **not independently verified here**; the Java
   slice is the only thing that can verify it.
4. **Bulk bytes above 64 KB.** `S-bytes-all-256` and `B-P5_1` are the only bulk
   vectors. P5.2 to P5.4 (64 KB, 1 MB, 4 MB) stay in `schema/`'s manifest as
   hashes: a 4 MB vector in git buys nothing a hash does not, and the
   direct-argument path of ABI v1 section 8 is a mechanism question rather than a
   wire question.
5. **No content-set sweep.** `schema/` emits ASCII only and the corpus follows
   it: Latin-1, above-U+00FF and astral appear as single `S-string-*` vectors,
   not across the payload set. A slice still checks content sets the way
   SHAPES.md says, and the corpus does not replace that.
6. **Wire types the schema does not have.** No `sint32`/`sint64` (zigzag), no
   `uint32`/`uint64`, no `fixed64`/`sfixed*`, no `float`. SHAPES.md's census says
   the real schema has zero of all of them, so this follows the census -- but a
   generator backend with no case for zigzag would be caught by nothing here.
   `fixed32` was added precisely because wire type 5 was otherwise absent, and
   the same argument would extend to zigzag if anyone wants it.
7. **Only `map<string, string>`.** Both maps in the real schema are that, and the
   encoder **raises** on any other key or value kind rather than skipping it. A
   map with a message value has no vector.
8. **Nothing about the other ABI v1 section 12 obligations.** The group layout
   assertions (12.3), the worker path's build-time subset check (12.4) and the
   concurrency suite (12.5) are not corpus artifacts and are untouched. 12.5 is
   the obligation with the most evidence behind it and the least existence.
9. **Nothing about timing.** Correctness artifact only.

## Coverage gaps per slice

Every slice must **consume** all 336. What differs is what it can **produce**.

| Slice | Cannot produce | Which |
|---|---|---|
| `csharp` | 85 of 287 | 74 `unknown` + 10 non-canonical `empty` + `S-interleaved` |
| `java` | 85 of 287 | the same |
| `cpp` | 92 of 287 | the same, plus the 7 `T-enc-*` |
| `python` | 92 of 287 | the same, plus the 7 `T-enc-*` |
| `rust` | 92 of 287 | the same, plus the 7 `T-enc-*` |

None of these is a defect:

- **74 `unknown`**: a codec cannot emit a field it does not know. That is the
  class's whole point, not a hole in it.
- **10 `empty` + `S-interleaved`**: legal wire that no canonical writer produces
  -- an implicit-presence leaf written as zero, a map entry with its value before
  its key, a packed field written unpacked, two repeated fields interleaved.
  Consume-only by construction, and `accepted_encodings` carries the canonical
  re-encoding each one must produce.
- **7 `T-enc-*` for cpp, python, rust**: a Rust `String` cannot hold an unpaired
  surrogate, and a C++ host holding `std::string` is already UTF-8. **Python is
  the arguable one**: CPython's `str` *can* hold a lone surrogate, so a python
  host could reach the encode half through `ak_tc_ucs4`. README section 10 item 4
  names only C# and Java, so the corpus does not require it. See request 5.

The real gap is item 5 of the *contract*, not of the manifest: **a slice that
runs the `chunking` vectors in a single chunk has not exercised README section 10
item 5** whatever the manifest says. The corpus cannot compute the chunk count
(ABI v1 chunks at 32 KB of host-side element *groups*), so each vector declares
`min_group_bytes_for_two_chunks` and `chunks_if_group_is`, and `CONTRACT.md`
requires the slice to report what it saw. Expect this to be the first thing a
slice quietly skips.

## Requests -- things outside `ffi/corpus/**` that this work wants

I own `ffi/corpus/**` and nothing else, so these are requests, not edits.

1. **ABI v1 should state the substitution GRANULARITY.** Section 6 says the
   converting transcoders replace unrepresentable input with U+FFFD; it does not
   say whether that is one U+FFFD per unpaired surrogate **code unit** or one per
   maximal ill-formed subsequence. The corpus encodes **one per code unit** and
   says so in every `T-enc-*` row. If the ABI decides otherwise, `T-enc-two-highs`
   and `T-enc-reversed-pair` change and nothing else does. This is the one place
   the corpus had to pick, and it should not be the corpus picking.
2. **Open decision 7 (the decode recursion limit) is still open, and the corpus
   now asserts something about it.** `X-depth-101` and `X-depth-300` require
   rejection, on protobuf's conventional 100-level cap, which is what upb does.
   If ABI v1 chooses a different limit, or chooses none, those two rows need to
   change. Nothing else exercises the decision today.
3. **Open decision 11 (retain unknown fields) is expressed as data rather than
   decided.** Every `unknown` row accepts both the retained and the dropped
   re-encoding. A decision halves that set and makes 74 rows a sharper test; the
   corpus can carry it either way and does not need the answer to be useful.
4. **`design/SHAPES.md` does not mention this directory.** Its "unknown field"
   and "unknown fields on the wire" rows say `corpus`, which is now a real path.
   Worth a pointer, and worth recording that the chunking shapes live here rather
   than in `shapes.json` and why.
5. **README section 10 item 4 says the pair can be run only by C# and Java.**
   CPython's `str` holds lone surrogates, so the python slice could reach the
   encode half through `ak_tc_ucs4`, which ABI v1 open decision 3 already
   discusses. If that is wanted, the manifest rows change in one place
   (`produce`) and `emit/vectors.py` in one line.
6. **CI.** `ffi/corpus/run.sh --check` is the gate and nothing runs it yet. It
   needs `pip install protobuf grpcio-tools` and about twenty seconds. ABI v1
   section 12.1 calls the corpus a release gate, so this is the step that makes
   that sentence true.
7. **`ffi/schema/generated/manifest.json` records one hash for P2.5.** SHAPES.md
   documents the second form at length, but a slice reading only the schema
   manifest still sees a single hash. The corpus's `B-P2_5` row now carries upb's
   `+80` as data. Not a defect; a place where two files disagree in tone.

## If you are the next session on this

Run `./run.sh` first. It regenerates, validates against upb, self-tests the
guards and clones the branch to a different path to prove the tree rebuilds
itself. If it passes, the corpus is in the state this file describes.

The next real work is **not** in this directory: it is a slice consuming the
corpus per `CONTRACT.md`, which is the first thing that will find out whether
this generator and upb share a misreading.
