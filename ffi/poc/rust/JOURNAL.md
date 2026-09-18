# rust slice: journal

What was tried, what it measured, what refuted it, in order.
Append; do not rewrite. A later reader comes here to find out that an option
was already refuted and by what.

## Entries

### 2026-09-18 — stage 1: `ffi/schema/generated/manifest.json` against prost

**Setup.** `crates/shapes-prost` compiles `ffi/schema/generated/shapes.proto` with protox 0.9.1
(there is no protoc in this container) and hands the descriptor set to prost-build 0.14.4.
`btree_map(".")` on every map field, because the canonical form sorts map entries by key and
prost's default `HashMap` iterates in an unspecified order — a `HashMap` arm could not produce
canonical bytes at all, so this is a requirement rather than a tuning knob.

`crates/shapes-values` re-derives the value rules of `emit/values.py` **by hand from the rules
they state**, not by transliterating the Python. That is deliberate: stage 1 exists to find out
whether `emit/wire.py` is semantically right, and a Rust side generated from the Python side
would inherit whatever the Python side assumed. An independent re-derivation is the thing that
can disagree. `crates/stage1-validate` builds all sixteen payloads as prost values and compares
**bytes**, not hashes — `generated/payloads/` commits only the seven vectors at or under 64 KB,
so `gen/dump_payloads.py` regenerates the other nine first.

**First, what did not go wrong.** All sixteen payloads regenerate from `ffi/schema/emit`
exactly as the manifest records, and the seven committed `.bin` vectors are byte-identical to
what the emitters produce today. `generated/` is not stale.

**The disagreement.** 8 of 16 payloads differ: P1.1, P1.2, P2.1, P2.2, P2.3, P2.4, P2.5, P4.1.
P1.3, P3.1, P5.1 to P5.4, P6.1 are byte-identical, and P7.1 is checked by decode (below).

The field is always a `Timestamp` or a `Duration`, and the bytes are always the same two:

```
P1.1  manifest: tag path 1.5.2  wire type 0  @0x63  body 00     (ResultRaw.created_at.nanos = 0)
      prost   : <not written>
P2.1  manifest: tag path 1.10.2 parses as a nested message      (TaskDetailed.options.max_duration)
      prost   : tag path 1.10.2 with an EMPTY body
```

`emit/payloads.py` reaches `Timestamp` and `Duration` through a shortcut:

```python
if f["of"] == "Timestamp":
    t = V.timestamp(path, idx)
    return W.ld(f["tag"], W.i(1, t["seconds"]) + W.i(2, t["nanos"]))
```

which writes both leaves unconditionally. Every other scalar path in the same file guards on
the value (`return W.i(f["tag"], int(v)) if v else b""`). `timestamp(0).nanos` is 0 and
`duration(0)` is `(0, 0)`, so element 0 of every payload carries the extra bytes; higher
indices do not, which is why the deltas are small and constant per payload rather than
proportional to the element count. `oneof_member()` has the same shortcut for `as_stamp`; it
is not reachable with a zero in P3.1 today (the variant only lands on `idx ≡ 3 mod 5`, and
`(idx·7919) mod 10⁹` is never 0 for `idx ≤ 199`), but it is the same defect and it is swept
rather than fixed where it was found (R10).

**Which side is right.** Three answers, all the same.

1. The schema directory's own stated canonical form: *"an implicit-presence leaf holding the
   proto zero is omitted"* (`ffi/schema/README.md`, repeated in `manifest.json` itself).
   `Timestamp.seconds`, `Timestamp.nanos`, `Duration.seconds` and `Duration.nanos` are
   implicit-presence leaves. The emitter contradicts the rule the manifest ships with.
2. prost's `#[derive(Message)]` omits them.
3. prost-reflect 0.16.5, encoding a `DynamicMessage` built from the same protox descriptor at
   runtime and sharing no encoding code with the derive, omits them too
   (`crates/shapes-prost/tests/zero_leaf.rs`).

So this is protobuf's rule, not prost's, and the finding does not rest on one implementation.

**Why nothing caught it.** The extra bytes are legal wire: `Timestamp::decode` accepts them and
produces the same value. `emit/check.py` proves framing, and the framing is fine — a `nanos = 0`
field is correctly framed. There was nothing in the container to compare semantics against,
which is exactly what the schema README says and why this was the Rust slice's first task.

**Is it the only one?** `gen/stage1_isolate.py` copies `ffi/schema/emit` into a scratch
directory, applies one change there — guard both leaves on the value, in `enc_field` and in
`oneof_member` — regenerates, and compares against prost. **All 15 prost-encodable payloads
become byte-identical.** One defect, and it accounts for all eight disagreements. Nothing under
`ffi/schema/` was written: the change is a finding, not an edit.

**P7.1.** prost cannot encode it: it writes a repeated field contiguously and the payload
interleaves `left` and `right` on purpose. Checked instead by decoding — prost accepts the
bytes, decodes them to exactly the value the rules predict, and a contiguous re-encode is 98 B
and a permutation of the same (tag path, wire type, body) triples. That is as much as prost can
say about it, and it is what the payload is for: it is legal wire that a group buffer keyed by
type cannot decode, so *no* canonical writer produces it.

**Corrected figures**, for whoever owns `ffi/schema/`, all in
`ffi/logs/rust/stage1-second-encoder.log`:

| payload | manifest B | correct B | delta |
|---|---|---|---|
| P1.1 | 862 | 858 | −4 |
| P1.2 | 218,125 | 218,121 | −4 |
| P2.1 | 1,073 | 1,039 | −34 |
| P2.2 | 551,696 | 551,662 | −34 |
| P2.3 | 649,809 | 649,775 | −34 |
| P2.4 | 981,256 | 981,222 | −34 |
| P2.5 | 19,658 | 19,632 | −26 |
| P4.1 | 69,557 | 69,551 | −6 |

The other eight payloads are unchanged, hash included. The −34 is 9 Timestamps × 2 B + 3
Durations × 4 B + `options.max_duration` 4 B, on element 0 of a `TaskDetailed` only.

**What this refutes.** Nothing that was being considered — it settles the open question the
schema README raised. Two consequences worth carrying:

- **No slice should diff against `manifest.json` until this is ruled on.** Eight of sixteen
  rows are wrong by a handful of bytes, which is exactly the size of defect a slice would
  attribute to its own codec.
- **The absent-path payloads are not the ones that caught it.** P1.3 is byte-identical, because
  everything in it is absent and the shortcut is never reached. What caught it was element 0 of
  the *full* payloads, where a deterministic value rule happens to produce a zero. A payload set
  designed around "absent" and "present" missed the third case, "present and zero", at the leaf
  level — the very case `Probe`'s `optional` fields were added for at the top level.

**Not done in this entry**: nothing is timed, three of the four arms do not exist, and byte
identity across arms is meaningless with one arm. See STATE.md.

### 2026-09-18 — stage 1 re-run against the fixed schema

`07d3e05` fixed `emit/payloads.py` through one `stamp_body()` helper and regenerated. Re-ran
`gen/stage1.sh` unchanged: **16 of 16**, every prost-encodable payload byte-identical, P7.1
still validated by decode. Log: `ffi/logs/rust/stage1-manifest-vs-prost-after-fix.log`. The
manifest is now this slice's oracle rather than a second opinion, so every stage 2 arm is
checked against it rather than against another arm.

### 2026-09-18 — stage 2: the generator, and four arms over M1

**The generator** is `gen/generate.py`, importing `ffi/schema/emit/shapes.py` (R1). It emits
six files and there is no hand-written codec in the comparison:

| file | arm it serves |
|---|---|
| `crates/facade/src/generated/types.rs` | the facade: `String`, `bytes::Bytes`, a real enum with a lossless `Unknown`, `Option` for a singular message. The shape `packages/rust` already uses |
| `crates/facade/src/generated/prost_impl.rs` | `armonik` |
| `crates/facade/src/generated/build.rs` | payload construction over the facade |
| `crates/facade/src/generated/core_native.rs` | `core-native` |
| `crates/ak-abi/src/generated/abi.rs` | the C ABI groups, fixes and vtables. The generated header both sides compile against |
| `crates/ak-core/src/generated/codec.rs` | the core behind the ABI |
| `crates/harness/src/generated/binding.rs` | `core-ffi-rust` |

`gen/generate.py --check` fails if what is committed is not what the generator writes, and it
is step 1 of `gen/stage2.sh`.

`core_native.rs` and `codec.rs` come from **one traversal emitter** (`gen/rust_core.py`,
`walk_encode` / `walk_decode`) with two backends that decide only where a value comes from or
goes to. ABI v1 section 7.1 makes that a condition rather than a preference. It also means
the two arms are **not independent encoders**, so neither validates the other; both are
checked against the manifest, and that is stated in `crates/harness/src/arms.rs` and on the
log.

**Correctness first.** All four arms are byte-identical to the validated manifest on P1.1,
P1.2 and P1.3, and each decodes its own output back to an equal value.

#### Defect 1, found by P1.3: the absent-path precedence, in the payload builder

The first generated builder made P1.3 4,234 bytes against the manifest's 605. `absent()` in
`emit/payloads.py` checks `all_absent` FIRST and only then the `half_absent` rules; the
generator had the `half_absent` special case for a Timestamp-typed even-tag field short-circuit
the `all_absent` check, so `ResultRaw.completed_at` (tag 6) survived in a payload where
everything is supposed to be absent. **Swept rather than patched**: `absent_expr()` and
`zero_expr()` are now computed once per field and every kind goes through them, instead of a
mode check living inside the branches that happened to need one. P1.1 and P1.2 passed
throughout, which is exactly the point of P1.3.

#### Defect 2, and it is the one that would have invalidated the whole arm

The release binary contained **zero call sites** to `ak_encode_ListResultsResponse`,
`ak_elem_ResultRaw` and `ak_decode_ListResultsResponse`. With `ak-core` in the crate graph as
an rlib, rustc inlined every `extern "C"` entry point straight into the host: `decode_with`
called nothing, `loop_results` called nothing at all. The boundary-call counters still
reported 3, 9 and 6 crossings, **because the counting code was inlined along with the
function bodies**. A counter is not evidence that a call happened.

So `core-ffi-rust` was, at that point, `core-native` with extra struct copies, and any figure
from it would have been a figure about the optimiser. Found by asking the standing question
early: an arm that should cost more than its control and does not is probably not running.

The fix is `crate-type = ["cdylib"]` on `ak-core`, linked through `build.rs` and resolved by
the dynamic linker. `nm -D --undefined-only` now shows the thirteen ABI entry points as
undefined imports (section 4 of the stage 2 log), which is a property a reader can check
rather than a claim. It is also the deployment every managed host actually uses. A C or C++
host that statically links the same core gets a direct call and pays less; this slice
measures the shared-library case and says so.

**The crossing, priced in the same process and the same build**: 1.8 ns for a forward call,
1.8 ns for forward-plus-reverse, stable to 0.1 ns across three runs. That is the number that
makes this arm the interface with the runtime tax removed, and the comparison is 0.25 ns in
C++, 7.5 to 12 on .NET 8, 33.8 through FFM and 98.4 through JNI.

#### Defect 3: the `armonik` arm's encode regression was my emitter

First measurement had `armonik` at 1.16 to 1.40 of prost on encode, which would have read as
"hand-written types cost something against generated structs". It did not: the emitter
produced `if self.status.to_i32() != 0 { ...encode(&self.status.to_i32())... }`, running the
enum's match twice, and `is_some()` followed by `as_ref().unwrap()`, which is a second branch
and a panic path. Guard and value are now emitted as one statement per field, swept across
every kind. The regression closed to 0.94 to 1.03 on P1.1 and P1.2. **A number taken before
that would have been a false finding about the type shape.**

#### Defect 4: the empty span on the decode side of the binding

`core-ffi-rust` decode on P1.3 measured 2.20 of prost. `s_of` took a zero-length span through
`String::from_utf8_lossy(...).into_owned()`. A zero-length fast path in the generated
accessor took it to 1.31 to 1.39. The remainder is structural and is reported as a result,
not as a defect (below).

#### What the numbers say (`ffi/logs/rust/stage2-four-arms-M1.log`)

Ratios to prost, formed inside one process from interleaved rounds, three separate processes,
range across them:

| payload | direction | armonik | core-native | core-ffi-rust |
|---|---|---|---|---|
| P1.1 | encode | 0.941 - 0.954 | 0.343 - 0.349 | 0.593 - 0.622 |
| P1.1 | decode | 0.889 - 0.939 | 0.751 - 0.861 | 0.779 - 0.889 |
| P1.2 | encode | 0.967 - 1.032 | 0.425 - 0.438 | 0.706 - 0.716 |
| P1.2 | decode | 0.896 - 0.935 | 0.832 - 0.861 | 0.848 - 0.879 |
| P1.3 | encode | 1.173 - 1.306 | 0.468 - 0.475 | 1.162 - 1.295 |
| P1.3 | decode | 0.965 - 1.047 | 0.726 - 0.838 | 1.313 - 1.393 |

Three things in it that are not obvious.

**The absent path inverts the verdict.** On P1.1 and P1.2 the core beats prost in both
directions through the C ABI. On P1.3, where every element encodes to nothing,
`core-ffi-rust` loses in both, while `core-native` stays at 0.47 and 0.73. The whole gap is
the by-value group: the binding fills a ~200-byte `ak_efix_ResultRaw` per element and the
core materialises a ~128-byte `ak_dfix_ResultRaw` per element, whatever the wire holds, and
on the absent path there is nothing for that fixed cost to amortise against. It is a
property of the amendment the C# report motivated, and no slice had a payload that could see
it: this is what P1.3 is for.

**The transcoder's validation is the largest single knob on encode.** `ak_tc_utf8`
(validate-and-fail, the Java slice's choice, ABI v1 open decision 3) against
`ak_tc_utf8_trusted`: 0.79 against 0.61 on P1.1 and 0.89 against 0.71 on P1.2. That is 25 to
30 percent of an encode spent re-checking an invariant a Rust `String` already carries in its
type. The decision has a price and it differs by host: a host whose string type guarantees
UTF-8 (Rust, and a C++ host that says so) is paying for nothing.

**The guard is free here.** Guard on against guard off is inside the run-to-run spread of the
ratio itself. Structural rather than Rust-specific: with strings riding in the group the
guard lands on 1 to 5 reverse calls per message instead of one per string.

#### Crossings, counted, not inferred (`--features count`)

| payload | direction | elements | strings | forward | reverse | crossings |
|---|---|---|---|---|---|---|
| P1.1 | encode | 4 | 20 | 2 | 1 | 3 |
| P1.1 | decode | 4 | 20 | 1 | 2 | 3 |
| P1.2 | encode | 1,000 | 5,000 | 8 | 1 | **9** |
| P1.2 | decode | 1,000 | 5,000 | 1 | 5 | **6** |
| P1.3 | encode | 300 | 0 | 3 | 1 | 4 |
| P1.3 | decode | 300 | 0 | 1 | 3 | 4 |

Nine crossings to encode a thousand rows and six to decode them. The drafted ABI spent
15,137 on the same shape. Transcoder invocations (6,000 on P1.2) are counted separately and
are **not** host crossings: ABI v1 section 4 puts every representation in the core, so a
transcode is an indirect call the core makes into itself.

#### ABI v1 open decision 5, partially answered

A cold encode context, every learned width starting at one byte, costs **one** prefix move on
P1.2 — 1 in 218 KB of output, 1,000 elements and 6,000 strings. Warm: zero. The element site
learns two bytes on the first element and never misses again. Zero grow-callback invocations
at any point, because the buffer is handed over whole rather than reserved per string.

**This is the easy case and the answer is provisional.** Every string in M1 is a fixed-length
GUID or a short word, so a site's width never changes after it is learned. P2.4 exists
because a per-site learned width is wrong on every element of it by construction, and that is
where the decision is actually settled. Stage 3.

#### What stage 2 does not measure

Everything in STATE.md's list. Named here because they bear on how the table above should be
read: only M1, only ASCII, no oneof, no map, no packed, no repeated string, no adapter site,
no unknown fields, no RPC, single-threaded, and the floor is unverified for want of a 1.88
toolchain.

### 2026-09-18 — stage 3, part 1: M2, the non-leaf element

**Two shape-coverage findings before any code**, reported rather than acted on, because a
slice does not change a shape:

1. `design/SHAPES.md` line 69 says **`packed repeated enum | M2 | what the real schema
   actually has`**. `shapes.json` has no packed enum field anywhere, and `TaskDetailed` has
   no packed field at all: its cards are `singular` and `repeated` only. The only packed
   fields in the description are on M6 (`MetricsBatch`) and they are `int64`, `double`,
   `int32` and `bool`. M6 is the **stated control**. So the one packed shape the real schema
   has — 3 fields, all enums — is not reachable by this payload set, and the row that claims
   it is covered is wrong.
2. `design/SHAPES.md` says M2 covers **`nested message, depth up to 6 | the schema's maximum
   static depth`**. `emit/shapes.py`'s own `depth()` gives 3 for `ListTasksDetailedResponse`
   and 3 as the maximum over every message in the description:
   `ListTasksDetailedResponse -> tasks:TaskDetailed -> options:TaskOptions ->
   max_duration:Duration`. Nothing reaches 6.

**What M2 needed from the generator**, none of which M1 exercised: repeated string fields, a
`map<string, string>`, a singular message child that itself carries a loop field, and an
element type that fails ABI v1 7.2's batching predicate.

The map became a synthetic pair message in the IR (`gen/ir.py`, `_synthesise_entry`), once,
so no backend has a map case — ABI v1 section 11. `loop_slots()` is the other new piece: a
repeated or map field inside an inlined child keeps its loop slot and is reached through the
parent, so `TaskOptions` never appears in the ABI as a message with a vtable of its own and
`ak_evt_TaskDetailed` carries `loop_options_options`.

For a non-leaf element the encode side is `ak_elemu_TaskDetailed(ctx, elems, n, tok0)`,
naming element *i* as `tok0 + i`, and the decode side gets the protocol ABI v1 7.2 describes
as "two calls per element": `new_tasks` creates an empty element and returns its index,
`add_tasks_<field>` appends to it, and `apply_tasks` sets the singular subtree at the end.
**The order is what forces it**: `apply` has to come last, because the runs that populate
the repeated fields can arrive before the group fields are parsed, so a binding that
constructed the element from the group would discard them. That produced a codegen rule that
was swept rather than special-cased: **a message whose type carries a repeated or map field
is filled in place, never constructed; a leaf message gets a constructor.**

#### Correctness: four arms, P2.1 to P2.5, byte-identical

All four arms match the validated manifest on all five M2 payloads, P2.4 and P2.5 included,
and the three facade decoders land on the same value from the same bytes — a check byte
identity of the encoders does not give, added here because M2 is the first shape where two
decoders could agree on bytes and disagree on a map.

#### Defect 1: the nested child read from the wrong reader

`range start index 783 out of range for slice of length 179`. The emitted arm for a singular
message child took its length prefix from `d` (the root reader) instead of the reader at its
own depth. It only shows on a child at depth two or more, which is why M1 never saw it:
`ResultRaw.created_at` is depth one, `TaskDetailed.options.max_duration` is depth two. Fixed
in the emitter, which is the only place it could be fixed.

#### Defect 2: the codec's open-field state leaked across an element run, and the payload
#### set could not have caught it

Found by asking why a length-prefix site would not converge. `loop/TaskDetailed/options.options`
missed **448 times in 500 elements** on P2.2, a payload whose elements are all the same
shape, where the learned width should settle after one pass and never miss again. Probing the
site showed widths of 1, 2 **and 3** — a map entry is about 30 bytes, so something much
larger was using that slot.

It was the element body. `ak_elemu_TaskDetailed` reads the open field's tag and site from the
context at entry; the host drives iteration and calls it **once per chunk**; and encoding an
element sets the open state for that element's own repeated fields. So from the second chunk
onwards the element run read whatever the last element had left behind — the map's tag and
site rather than `tasks`'s.

**The length-prefix thrashing was the symptom, not the defect.** `open_tag` is read there
too, so a host that chunks writes its later chunks under the inner field's tag. This payload
set did not corrupt, and the reason is worth stating plainly: `ListTasksDetailedResponse.tasks`
is tag 1 and `TaskOptions.options` is tag 1. **Byte identity passed on a tag collision.** A
root whose repeated field had any other tag would have produced silently wrong wire on every
chunk after the first, and nothing in `design/SHAPES.md` has one.

Fixed by having every element and run entry point save the open state at entry and restore it
before returning, so a run leaves the codec's open field as it found it. After the fix:
P1.2, P2.2, P2.3 and P2.5 all converge to **zero** warm misses, and that is now a regression
check in the counting build (`gen/stage3.sh` step 3) rather than something to rediscover.

#### Crossings, counted (`--features count`)

| payload | class | direction | elements | forward | reverse | crossings | per element |
|---|---|---|---|---|---|---|---|
| P1.2 | real | encode | 1,000 | 8 | 1 | 9 | 0.009 |
| P1.2 | real | decode | 1,000 | 1 | 5 | 6 | 0.006 |
| P2.2 | real | encode | 500 | 2,511 | 2,501 | 5,012 | 10.02 |
| P2.2 | real | decode | 500 | 1 | 3,501 | 3,502 | **7.004** |

**7.004 crossings per task on decode is exactly what ABI v1 7.2 predicts** — "keeps two calls
per element, which is 7 crossings per task where the drafted ABI spent 43". Two calls per
element (`new`, `apply`) plus one run for each of the four repeated string fields and one for
the map. The specification's number was an argument; it is now a measurement.

Encode costs 10.02 per element on the same payload, which is higher than decode and is the
number nobody had: five reverse calls (one per loop slot) and five forward calls (four blob
runs and one pair run) per element, plus the root loop and the chunked element runs. **The
batching predicate saves the decode side and not the encode side**, because on encode the
host drives every one of its own containers.

#### ABI v1 open decision 5, answered

Per-site miss counts on a warm context, which is what makes this an answer rather than an
aggregate:

| payload | shape | warm misses | bytes moved |
|---|---|---|---|
| P1.2 | uniform | 0 | 0 |
| P2.2 | uniform, 500 elements | 0 | 0 |
| P2.3 | uniform, 125 elements | 0 | 0 |
| P2.5 | uniform, 20 elements | 0 | 0 |
| **P2.4** | **alternating 3/150** | **80, one per element** | **980,938** |

On P2.4 the encoder memmoves 980,938 bytes of a 981,222-byte output: essentially the whole
payload, once, every encode. All of it at one site, `loop/ListTasksDetailedResponse/tasks`,
and 1.00 misses per element, which is what "wrong on every element by construction" means.
Zero transcoder grow-callback invocations at any point, on any payload.

**What it costs**, isolated rather than attributed. P2.4 against P2.3 is not size-matched, so
two arms were added (allowed; a shape was not changed): P2.4a is 80 elements at 3 repeats and
P2.4b is 80 elements at 150, and 87,422 + 1,875,022 is exactly twice P2.4's 981,222, with the
element, field and string counts matching too. So `t(P2.4) - (t(P2.4a) + t(P2.4b))/2` is the
thrashing and nothing else. prost goes through the same arithmetic as a floor, because it
computes lengths in a first pass and has no learned width at all:

| arm | excess of P2.4 over the mean |
|---|---|
| prost (floor: the construction's own non-linearity) | 2.03 % |
| core-native | 5.04 % |
| core-ffi-rust | 2.91 % |

So the mechanism costs roughly **1 to 3 percentage points of an encode on the payload built
to defeat it**, and nothing at all on every uniform payload. That is much less than the
980 KB moved suggests, because the move is a sequential in-cache memmove of a ~12 KB element
body, about 70 ns each.

**The limit of the mechanism, stated because it is structural.** A per-site learned width
cannot avoid the move when a site's bodies straddle a varint boundary: over-reserving would
need a non-minimal varint, which ABI v1 section 6 refuses outright, and under-reserving needs
the move. The alternatives are a two-pass length computation (what prost does) or writing the
body to scratch first. The measurement says the learned width is the right default; it does
not say it can be made to converge on P2.4, because it cannot.

#### What M2 measures (`ffi/logs/rust/stage3-M2.log`)

See the log for the full table with spreads. The shape of it: on the uniform M2 payloads
`core-ffi-rust` encode is 0.88 to 0.92 of prost and decode 0.84 to 0.90, `core-native` encode
0.48 to 0.57, and `armonik` sits within a percent or two of prost in both directions. The
validating UTF-8 transcoder costs more on M2 than on M1 — 1.20 to 1.40 against 0.88 to 1.08 —
because M2 is a denser string payload, which is a second data point for ABI v1 open
decision 3 and it moves the wrong way for validation.

#### The M2 result that matters most, and it is not the one I expected

Ratios to prost, range over three processes (`ffi/logs/rust/stage3-M2.log`, first appendix):

| payload | direction | armonik | core-native | core-ffi-rust |
|---|---|---|---|---|
| P2.1 | encode | 0.697 - 0.970 | 0.342 - 0.484 | 0.704 - 0.981 |
| P2.2 | encode | 0.991 - 1.001 | 0.493 - 0.517 | 0.833 - 0.856 |
| P2.3 | encode | 0.953 - 1.000 | 0.476 - 0.508 | 0.876 - 0.924 |
| P2.4 | encode | 0.992 - 1.000 | 0.565 - 0.570 | 1.026 - 1.035 |
| P2.5 | encode | 0.978 - 1.014 | 0.463 - 0.476 | 0.893 - 0.901 |
| P2.1 | decode | 0.961 - 0.980 | 0.878 - 1.028 | 1.081 - 1.209 |
| P2.2 | decode | 0.970 - 0.974 | 0.894 - 0.961 | 0.950 - 1.015 |
| P2.3 | decode | 1.017 - 1.017 | 0.932 - 1.068 | 0.819 - 0.953 |
| P2.4 | decode | 0.967 - 0.970 | 0.897 - 1.067 | 0.809 - 0.981 |
| P2.5 | decode | 0.995 - 1.010 | 0.919 - 0.992 | 0.944 - 1.033 |

**Encode holds up: the core is 0.46 to 0.57 of prost natively and 0.83 to 0.92 through the C
ABI on every uniform M2 payload.** The M1 result survives the harder shape.

**Decode does not.** On M1 both core arms sat at 0.75 to 0.89. On M2 `core-native` is 0.88 to
1.07 and `core-ffi-rust` is 0.81 to 1.21: parity, with a spread wide enough that the honest
statement is "no measurable difference from prost".

**The crossings are not the reason.** Seven per element at 1.8 ns is 12.6 ns, against about
2,200 ns per element for P2.2 decode: 0.6 percent. And `core-native` makes none at all and is
at parity too. What changed between M1 and M2 is the payload: P2.2 carries 17,500 strings and
2,000 map entries in 551 KB, so decode is dominated by `String` allocation and `BTreeMap`
insertion, which every arm does identically. **On the shape the control plane actually moves,
decode is allocation-bound and the codec stops mattering.** That bounds the whole decode half
of the argument, and it is a better reason to be cautious about decode figures than any
ratio in this table.

The interface cost is still visible underneath it, and it is small: `core-ffi-rust` against
`core-native` is about 6 percent on M2 decode and about 2 percent on M1.

**The other half of the two-calls-per-element protocol.** `new` then `apply` means the host
materialises a default 27-field `TaskDetailed` and then fills it, where prost constructs it
once. The order is forced: the runs can arrive before the group fields, so a binding that
constructed from the group would discard them. Whether the ABI should let the codec defer a
flush to the end of an element body when the arena has room — which would allow `apply`
first and one construction — is a design question and it is not mine to answer.

**P2.1 is one element.** Its ratios are a call-rate figure with a huge spread (encode 0.70 to
0.98 on the same arm) and should not be read as a throughput result.
