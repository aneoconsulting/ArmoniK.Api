# corpus/STATE.md

The handoff for W8. Read this first; the transcript is gone.

**Status: built, gated, and revised once by its own consumers.** `ffi/corpus/`
holds a generator, 336 vectors, a manifest, projections, a consumer contract and
a gate. `./run.sh --check` passes from a clean clone at a different path.

**Revision of 2026-09-20.** The first two consumers (`poc/python`, `poc/cpp`)
found something about the corpus rather than about themselves: it decided every
projection, every accepted encoding and every accept/reject verdict with **one
runtime**, and one runtime deciding what the right answer is makes that runtime
the specification. On `U-map-entry` it is the minority. **Three runtimes now
answer, and where they disagree the row is DISPUTED rather than decided.** No
vector byte changed -- `generated/vectors.sha256` freezes them and the build
refuses to move one -- because consumers have already pinned them; what changed
is the manifest's claims about them.

## What exists

| | |
|---|---|
| Generator | `emit/` -- 6 modules, driven off `corpus.json` merged over `../schema/shapes.json` |
| Vectors | 336: 74 `unknown`, 30 `empty`, 145 `shape`, 53 `transcode`, 8 `chunking`, 18 `malformed`, 8 `baseline` |
| Of those | 287 must be accepted, 49 must be **refused** |
| Manifest | `generated/manifest.json` -- per vector: what it tests, produce/consume per slice, the **set** of accepted encodings, a projection, per-class metadata, per-language notes |
| Projections | 336 + 65 superset projections. What a reader must SEE, not just bytes |
| Contract | `CONTRACT.md`. The whole obligation, so five slices do not each invent it |
| Verdicts | 335 `agreed`, **1 `disputed`** (`U-map-entry`) |
| Seal | `generated/vectors.sha256`, 328 vectors. The build refuses to move a byte |
| Gate | `run.sh` / `emit/build.py --check` / `emit/selftest.py` (44 checks) |

### The three oracles

| Oracle | Asked | Why it |
|---|---|---|
| protobuf 7.36.2, **upb** | the structural checks, one reading, one re-encoding | a C parser sharing no code with the writer |
| protobuf 7.36.2, **pure-python** | a second reading and re-encoding, in the same projection format | a genuinely different parser; a disagreement is a JSON diff rather than an argument. One env var away, so nearly free |
| **protobuf C++** via `protoc --decode` | the accept/reject verdict on every row, and its text reading as evidence | fully independent |

protobuf C++ is **not** asked for the projection, and that is a finding rather
than a convenience: `protoc --decode` renders a map field as its wire-level
repeated `MapEntry` list. On `E-map-dup-key` it prints two entries with the same
key, which a map cannot hold, so it cannot answer a map-semantics question -- and
the disputed row is a map-semantics question. The cpp slice's reflection arm can
answer it, because `Reflection::ListFields` over generated code IS the presence
rule CONTRACT.md section 3 describes.

**All three refuse all 49 must-fail vectors**, with no row disputed. That is a
materially stronger claim than the one this file made yesterday.

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

## The disputed row, in full

`U-map-entry` puts an unknown varint field (tag 3) inside every entry of
`TaskOptions.options`, a `map<string, string>`.

| Runtime | The map | The unknown field | Re-encodes to |
|---|---|---|---|
| protobuf 7.36.2, **upb** | **empty** | the whole entry retained as an unknown field of the PARENT | 855 B, byte-identical to the input |
| protobuf 7.36.2, **pure-python** | all four entries | dropped | 847 B |
| **protobuf C++** 35.1, `protoc --decode` | all four entries | dropped from the text output | not comparable (text round-trip cannot re-encode an unknown field) |
| protobuf C++ 3.21.12, reported by the aggregating session | the entry present | retained *inside* the entry (`3: 7`) | -- |
| the cpp slice, both arms, reported | the entry present | -- | -- |

Reproduced here from first principles, not taken on report. A map field is
shorthand for a repeated `MapEntry` message and an unknown field inside a
submessage is skipped while the submessage still parses, so upb looks like the
one that is wrong -- **and the corpus does not say so.** It publishes both
readings, names which runtime produced each, points at protobuf C++'s text
reading as evidence, and excludes the row from a consumer's pass or fail count.
Two of the three runtimes I happened to ask is not a specification either.

Worth noting for whoever resolves it: the two C++ versions differ from *each
other* on retention (3.21.12 keeps `3: 7` inside the entry, 35.1 drops it), which
is ABI v1 open decision 11 territory and is already expressed as accepted forms
everywhere else in the corpus. The map contents are the substantive
disagreement; retention is the second-order one.

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

1. **Two slices consume it, three do not.** `poc/python` (48 rows in scope) and
   `poc/cpp` (128 rows, three arms, plus a schema-less walker reaching 62 more)
   exist and found the dispute. csharp, java and rust have not run it.
2. **Three runtimes, all from one project.** upb, protobuf-python's pure backend
   and protobuf C++ are three parsers, but they are three Google parsers. prost
   validated `schema/`'s manifest and has never seen these vectors; neither have
   `Google.Protobuf` or protobuf-java. A misreading shared across the protobuf
   project's own implementations would still survive, and the row that started
   this revision is evidence that they do not always agree. **The slices remain
   the widest opinion available** -- and the corpus was wrong in exactly the way
   a single-oracle corpus is wrong until two of them ran.
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

## What this revision changed, mechanically

- `emit/oracle.py` is new: one runtime's whole reading of the corpus, importable
  and runnable as a subprocess, so a backend chosen by an environment variable at
  import time can be asked the same questions.
- `emit/build.py` runs three oracles, reconciles them, and fails only when
  **every** oracle accepts a must-fail vector or **no** oracle parses an accept
  vector. A single runtime's opinion no longer stops the build or decides a row.
- Rows carry `verdict`, and a disputed row carries `dispute.readings`,
  `dispute.differs_at` (dotted paths) and protobuf C++'s text reading as a file.
- Accepted encodings carry `written_by` and `observed_in_a_protobuf_runtime`, so
  "the corpus asserts this form is valid" and "a runtime was seen writing it" are
  no longer the same sentence. 87 rows carry a form in the weaker category.
- `permutation_accepted` marks the four rows where a re-encoding may be any
  re-ordering of an accepted form. **`B-P7_1` is why**: its only accepted
  encoding was one no conformant encoder produces, so a correct consumer failed
  C3. Baselines now go through the same reconciliation as every other row, which
  they did not before, and that was the defect behind it.
- `generated/vectors.sha256` freezes the vector bytes. The build refuses to
  change, add or drop one; `--reseal` is a deliberate decision, not a step.
- `emit/selftest.py` is 44 checks, up from 25. The new ones watch the seal refuse
  a changed, a missing and an added vector; watch the dispute machinery produce a
  non-vacuous disputed row whose readings really do differ; and refuse a build
  where the second oracle came back on the same backend as the first, because two
  readings from one parser are one reading.

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
7. **`U-map-entry` wants a resolution, and the corpus cannot supply one.** The
   candidates: it is an upb defect worth reporting upstream; or map parsing with
   an unknown field in an entry is genuinely unspecified and the ABI should say
   what the core does. Either way it is a decision for `design/`, and until it is
   made the row stays disputed and excluded. A conformant core that follows the
   majority reading is not currently failing anything.
8. **`ffi/schema/generated/manifest.json` records one hash for P2.5.** SHAPES.md
   documents the second form at length, but a slice reading only the schema
   manifest still sees a single hash. The corpus's `B-P2_5` row now carries upb's
   `+80` as data. Not a defect; a place where two files disagree in tone.

## If you are the next session on this

Run `./run.sh` first. It regenerates, validates against three runtimes,
self-tests the guards and clones the branch to a different path to prove the tree
rebuilds itself. It takes about 35 seconds, most of it `protoc` started once per
vector. If it passes, the corpus is in the state this file describes.

Two things to hold on to:

- **The vector bytes are frozen and the manifest's claims about them are not.**
  That split is what let this revision happen without breaking the two consumers
  that had already run. Keep it.
- **The next real work is still not in this directory.** Three slices have not
  run the corpus, and each one is another opinion on the rows the three protobuf
  runtimes here agree about -- which is the part no amount of care inside
  `ffi/corpus/` can check.
