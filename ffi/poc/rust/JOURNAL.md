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

### 2026-09-18 — stage 3, part 2: the content sets, and ABI v1 open decision 3

Done before M3 on the aggregating session's instruction, and it was the right order: the
answer is much larger than the ASCII figures suggested and it changes what decision 3 is
about.

**First, the schema change of `945d3cd1` re-validated.** Only P6.1 moved, as stated. The
hand-written prost builder in `crates/stage1-validate` gained the packed enum, and
`gen/stage1.sh` is back to **16 of 16**, P6.1 included at 123,354 bytes. That was the one
payload no protobuf implementation had seen; prost has now seen it.

**What the content sets are in Rust, which is not what they are anywhere else.** On .NET and
the JVM `latin1` and `wide` make a narrowing transcoder do real work or fail outright,
because the host holds UTF-16. A Rust `String` is UTF-8 already: there is no narrowing and
no transcoding. What changes is the byte width of the same character count -- 1, 2 and 3
bytes -- and which path the UTF-8 validator takes. So these rows price **validation and
width**, and a managed slice must not read them across. The recode lives in
`crates/shapes-values` and holds the character count so only the width moves; there is no
manifest for these sets, so correctness is byte identity of all four arms against the prost
arm, which the manifest validated on ASCII.

Wire sizes come out at 1.70 to 1.75 times ASCII for latin1 and 2.39 to 2.50 for wide, which
is the expected mix of widened strings and unwidened scalars.

#### The result: validation is not the question, the validator is

Encode, ratio to prost, and each arm against **itself on ASCII**
(`ffi/logs/rust/stage3-content-sets.log`):

| arm | P1.2 ascii | latin1 | wide | P2.2 ascii | latin1 | wide |
|---|---|---|---|---|---|---|
| `core-ffi-rust`, trusted (memcpy) | 0.719 | 0.754 | 0.753 | 0.805 | 0.852 | 0.784 |
| `core-ffi`, **validate, scalar** | 0.919 | **1.977** | **2.630** | 1.118 | **2.010** | **2.546** |
| `core-ffi`, **validate, SIMD** | 0.934 | 1.289 | 1.266 | 1.153 | 1.503 | 1.462 |

against its own ASCII:

| arm | P1.2 latin1 | P1.2 wide | P2.2 latin1 | P2.2 wide |
|---|---|---|---|---|
| trusted | 1.059 | 1.077 | 1.079 | 1.025 |
| validate, scalar | **2.153** | **2.964** | **1.831** | **2.411** |
| validate, SIMD | 1.380 | 1.403 | 1.328 | 1.343 |

**The scalar validator costs 2.2 to 3.0 times its own ASCII cost on non-ASCII content, and
turns a 0.72 to 0.81 win against prost into a 2.0 to 2.6 loss.** That is far larger than the
25 to 30 percent the ASCII pass measured, and it is the largest single effect anywhere in
this slice.

The reason is mechanical rather than about protobuf: `core::str::from_utf8` has an ASCII fast
path that consumes a `usize` at a time and a byte-at-a-time DFA for everything else, so its
cost is not proportional to bytes but to **non-ASCII** bytes with a much worse constant.
Arithmetic: `core-ffi` wide against trusted wide on P1.2 is about 290 µs for roughly 500 KB
of strings, about 1.6 cycles per byte at 2.8 GHz, which is the scalar DFA rate.

**So I added one arm rather than an argument.** `ak_tc_utf8_simd` has exactly the same
contract as `ak_tc_utf8` -- validate, and refuse malformed input -- with `simdutf8::basic`
instead of the standard library's scalar validator. It removes **half to two thirds** of the
penalty: 1.33 to 1.40 times its own ASCII instead of 1.83 to 2.96, and 1.27 to 1.50 of prost
instead of 1.98 to 2.63. It is not faster on ASCII (0.93 against 0.92; inside the noise),
because the scalar validator's ASCII path is already eight bytes an iteration.

**What that does to ABI v1 open decision 3.** The decision was framed as validate-and-fail
against validate-and-substitute against trust-the-host's-type, and the aggregating session
refused the third on the grounds that a trusted transcoder is a correctness contract a host
can be wrong about. That argument is untouched by this. What the measurement adds is that
**most of what the third option was buying is available without giving up the contract at
all**, by changing the validator rather than removing it. The remaining gap between SIMD
validation and trusting is 1.27 to 1.50 against 0.72 to 0.85, so trusting is still worth
something; it is no longer worth 2.6.

Caveats that belong with it: one x86-64 machine with AVX2, `simdutf8::basic` dispatches on
runtime CPU features and falls back where they are absent, and it is a dependency inside the
core rather than in a binding. None of that is settled here.

#### Decode: the content set scales everything and changes no verdict

Every arm validates on decode -- prost with `str::from_utf8`, the core arms with
`from_utf8_lossy` -- so all three sets cost all arms more, and the ratios barely move:

| payload | arm | ascii | latin1 | wide |
|---|---|---|---|---|
| P1.2 | `core-native` | 0.895 | 0.815 | 0.888 |
| P1.2 | `core-ffi-rust` | 0.863 | 0.802 | 0.866 |
| P2.2 | `core-native` | 0.963 | 0.875 | 0.927 |
| P2.2 | `core-ffi-rust` | 1.021 | 0.937 | 0.995 |

Decode costs every arm 1.2 to 1.7 times its ASCII self and the ordering is unchanged, which
also holds up the M2 finding from part 1: decode is allocation-bound and the codec is not
where the money is. **Decision 3 does not bite on decode**, which is worth stating because
the specification's asymmetry (the core transcodes on encode, the host on decode) is exactly
why.

#### What is still not measured on the string path

The `latin1` and `wide` sets were run on P1.2 and P2.2 only, encode and decode, one thread,
one machine. Not covered: the other payloads, the unpaired-surrogate case (which a Rust
`String` cannot hold, so this slice cannot produce the transcode-pair disagreement README
section 10 item 4 is about), and any content set on the bulk `bytes` path, where there is
nothing to validate.

### 2026-09-18 — a figure with no log, and the rule that catches it

The P6.1 re-validation after `945d3cd1` was reported without a log. The claim was true — the
builder change is in `crates/stage1-validate/src/build.rs` and the run happened — but
`stage1-manifest-vs-prost-after-fix.log` was run against the schema at `07d3e05` and still
showed P6.1 at 116,954 bytes, so the only stage 1 log in the tree contradicted the number
being quoted. Under this branch's own rule that is a claim, not a figure, and it was the
figure that mattered most: P6.1 was the one payload no protobuf implementation had seen.

Re-run and committed as `stage1-manifest-vs-prost-packed-enum.log`: **16 of 16, P6.1 at
123,354 bytes**. The older log is kept and its P6.1 row is marked superseded in the log
index rather than deleted, because every other row in it still stands and it is the record
of the zero-leaf fix.

**The rule this produces, and it is mine to keep rather than the aggregating session's to
enforce: a schema change re-runs `gen/stage1.sh` into a dated log before any number from it
is quoted.** The stage 1 harness is cheap to run and it is the only thing standing between
this slice and a wrong denominator, so there is no reason to quote it from memory. Noted as
D11 so it is visible in the defect log rather than only in prose.

### 2026-09-18 — stage 3, part 3: M3, the oneof and explicit presence, and D7 closed

Kept small on instruction: one payload, one timing row set, and the substance in a
correctness binary (`crates/harness/src/bin/shapes.rs`). Log: `ffi/logs/rust/stage3-M3.log`.

**D7 is closed.** Both contexts now begin with a `CtxHeader { kind, err }`, so `ak_fail` is
the one entry point ABI v1 section 5 says it is — "any host code holding a context may fail
the operation" — without knowing which kind it was handed. The decode guard reports through
it, the decode entry point returns it, and an element maker that fails returns a token the
codec refuses to use. It was unreachable until M3 and M3 is where that stopped being true.

#### A generator that omitted a shape and said nothing

`Message.plain` excludes oneof members, and every backend iterated `m.plain`. So when
`ListProbeResponse` was added as a root, the generator emitted a complete-looking codec for
`Probe` that **ignored `body` entirely**, with no error. It would have been caught by the
first conformance run, but only because a byte comparison existed; a slice with a weaker
oracle would have measured a message with its oneof silently missing. Both walkers now call
`b.oneof(...)` explicitly, so a backend that cannot do the shape raises instead of skipping
it. Recorded as D12.

#### Explicit presence, as three cases rather than one column

200 elements, all four arms, and the three cases are three columns because that is the whole
point of the shape:

| field | absent | present+zero | present+nonzero |
|---|---|---|---|
| `opt_count` (int32) | 67 | 19 | 114 |
| `opt_label` (string) | 50 | 21 | 129 |
| `opt_flag` (bool) | 40 | 92 | 68 |

Identical across `prost`, `armonik`, `core-native` and `core-ffi-rust`. **Every case occurs
on every field**, so the shape is exercised and not merely present. `present+zero` is the one
a by-value group cannot tell from absent unless its presence *word* carries it, and the group
here carries it: an explicit field's encode branches on the presence bit, never on the value
or on the length. A present-and-empty string has `len == 0` and is still written.

#### The oneof, including its payload-free member

40 of each of the five members over 200 elements, none unset, all four arms agreeing.
`as_nothing` is reached 40 times, so the payload-free member — present, empty, selected by
its presence alone — is exercised rather than assumed.

The group layout: a `body_case` discriminant carrying the **active member's tag**, plus every
member inlined beside it. Not a union, because ABI v1 section 6 says a group needs a fixed
shape rather than a size bound, and a union makes the layout depend on which member is
largest, which a host reproducing offsets by hand can get wrong silently. The cost is the
size of the group, and **a union is an unmeasured alternative rather than an equivalent one**.
A `body_case` the codec does not recognise is refused with `AK_ERR_ABI` rather than encoded
as nothing, since a host generated against a newer descriptor is the skew that must be loud —
built, but not exercised, because nothing in this build can produce an unknown case.

#### Fields the reader does not know, which nothing had executed

README section 10 item 1 says a corpus generated from the schema that reads it can never
contain one, so these are hand-built wire with no manifest hash. The check is that all four
arms agree on the decoded value **and** on the re-encoded bytes, with prost as the
independent fourth opinion.

| vector | arms agree | re-encode | does the unknown survive |
|---|---|---|---|
| unknown oneof tag 15 after a known member | ok | ok | no: the oneof stays at `as_int` |
| unknown oneof tag 15 before a known member | ok | ok | no: the oneof stays at `as_int` |
| unknown oneof tag 15 alone | ok | ok | no: the oneof is `None` |
| unknown fields, all four wire types, top level | ok | ok | no |
| unknown field inside a nested message | ok | ok | no |
| unknown field before every known field | ok | ok | no |
| `ResultRaw.status = 999` (unknown enum **value**) | ok | ok | **yes, as `Unknown(999)`** |

**The result on the unknown oneof member is that there is no such thing at the wire level.**
A parser cannot tell an unknown oneof tag from any other unknown field — the oneof grouping
exists only in the descriptor — so it goes to the unknown set and the case stays at the last
**known** member. Every arm here does that, prost included, and none of them retains unknown
fields, so the value is lost identically by all four. That is protobuf's behaviour and not a
property of this ABI, and the ABI cannot improve on it without retaining unknown fields,
which nothing in this design does.

**The contrast with the unknown enum value is the useful part.** There the field is known and
only the value is not, so the wire tells the reader exactly where to put it, and it round-trips
losslessly through `Unknown(999)` in the facade and through `i32` in prost. So of the two
"unknown" shapes `design/SHAPES.md` names, one round-trips by construction and the other
cannot round-trip at all in any of these arms. Worth stating plainly, because "an unknown
value must round-trip losslessly" is easy to read as covering both.

#### Crossings: the oneof and explicit presence cost none

| payload | direction | elements | forward | reverse | crossings |
|---|---|---|---|---|---|
| P3.1 | encode | 200 | 2 | 1 | 3 |
| P3.1 | decode | 200 | 1 | 2 | 3 |

Three crossings for 200 elements in both directions. Both shapes ride in the group entirely,
so neither adds a crossing — which is the claim the .NET gap left untested, and it is now
counted rather than argued.

#### Timings, and what they do to part 1's decode finding

Ratios to prost, range over three runs:

| direction | armonik | core-native | core-ffi-rust |
|---|---|---|---|
| encode | 0.966 - 0.975 | 0.406 - 0.410 | 0.846 - 0.853 |
| decode | 0.961 - 0.968 | 0.811 - 0.824 | 0.834 - 0.838 |

**M3 decode is a win, where M2 decode was parity**, and that sharpens part 1's conclusion
rather than contradicting it. The one-line version there was "decode is allocation-bound".
The more accurate version, now that a third shape has been measured, is that **decode
converges to parity in proportion to how much host-side CONTAINER construction an element
needs**, not how many bytes or strings it has:

| payload | per element | decode, core-native |
|---|---|---|
| P3.1 | a flat 5-field message, 1 to 2 strings | 0.81 - 0.82 |
| P1.2 | 6 strings or bytes, 2 optional messages, no container | 0.83 - 0.86 |
| P2.2 | 4 `Vec<String>`, a `BTreeMap`, 8 optional messages, a 27-field struct | 0.89 - 0.96 |

A `BTreeMap` insert and four `Vec` growths per element are work every arm does identically
and no codec can avoid, so the denser the element's container graph, the smaller the share of
decode any codec owns. That is the statement I would carry rather than "allocation-bound",
which is true but attributes it to the wrong thing: P3.1 and P1.2 allocate plenty of
`String`s and still show the win.

### 2026-09-18 — stage 3, parts 4 to 7: M4 to M7, and every shape covered

With this, **every message and every payload of `design/SHAPES.md` has all four arms
byte-identical to the validated manifest**, P7.1 included by the only method available to it.
Log: `ffi/logs/rust/stage3-M4-M7.log`.

#### M4: the adapter, checked by state because the payload cannot reach it

`P4.1`'s `error` is a sentence in all 200 elements, so the payload exercises exactly the one
case that works. The adapter is therefore checked by **state**, over hand-written facade code
modelled on `packages/rust`'s `armonik::Output` — whose own doc comment makes the distinction
that matters: `Invalid` is "no member set", distinct from `Ok`, which carries nothing but *is*
set, because a peer that reports no outcome is not a peer that reports success.

| state | nested wire | round trip | plain wire | round trip |
|---|---|---|---|---|
| `Invalid` | field absent | ok | `""` | ok |
| `Ok` | `success=true, error=""` | ok | `""` | **loses, comes back `Invalid`** |
| `Error("boom")` | `success=false, error="boom"` | ok | `"boom"` | ok |

The nested form round-trips every state. **The plain form cannot**: `Ok` and `Invalid` both
encode to the empty string, so one of them must come back wrong whatever the adapter author
chooses. This one returns `Invalid` and loses `Ok`; the intuitive alternative returns `Ok`
and silently claims success for a task that reported no outcome. That is precisely the defect
`SHAPES.md` says only a byte corpus catches — **and the corpus as it stands does not, because
no payload reaches either state.**

**A fourth shape-coverage finding, and the worst of the four.** At the nested site
`emit/payloads.py` fills `TaskOutput.success` and `TaskOutput.error` **independently**:
`scalar_bool` for one, `sentence` for the other. Counted over 200 elements the only
combinations that occur are `(true, non-empty)` and `(false, non-empty)`. The success state
`(true, empty)` never occurs, and `(true, non-empty)` is a state **no adapter over
`{Ok, Error(d)}` can represent at all**. So a facade that used the adapter at the nested site
could not round-trip P2.x byte-identically, and this slice's facade therefore keeps
`TaskOutput` as a plain struct there. The shape `SHAPES.md` calls "the only `with` adapter in
the Rust crate" is not reachable from the payload set in either of its two sites.

#### M5: the direct-argument path, and the refusal that goes with it

Built, and byte-identical on all four sizes. `ak_str.data` carries `AK_STR_DIRECT` and the
bytes are an argument of the call rather than a pointer into staging.

**Its value is on the JVM and this slice cannot confirm it.** The path exists so a host can
hold `GetPrimitiveArrayCritical` or FFM's `critical(true)` across the whole call, which it can
only do if the codec makes no reverse call. On a Rust host there is no pinning to avoid and
the copy is a copy either way, so P5.3 and P5.4 are a memcpy figure and are **not** evidence
for section 8's 0.16-to-0.34 claim.

**The refusal section 8 asks for now exists** (`gen/check_direct.py`, and it runs as step 1b
of `gen/stage3.sh`). Two configurations are refused at generator time, before a line is
emitted:

- a direct field on a message tree that also needs a reverse call, because a critical section
  and an upcall are mutually exclusive, so it is a contract no host can honour;
- more than one direct field in one tree, because section 8 builds the path for one field of
  one root message and "it generalises untested" — silently generalising it is how an
  untested path ships.

It was **not awkward to express**, which is the answer to the question that came with the
instruction: it is a predicate over the descriptor, computed where every other predicate in
this generator is computed, and it is eleven lines. The only thing that made it subtle is
that the first version walked singular message children only and therefore found nothing and
refused nothing — the same class of mistake as D12, and caught the same way, by running it
against a case that must fail rather than by reading it.

#### M6: a mixed table, labelled per row

`ticks`, `values`, `codes` and `flags` are **controls**: the schema has no packed scalar at
all. `statuses` is **real**: all three packed fields in the schema are enums. Quoting P6.1 as
a control result or as a real one would both be wrong, so the log labels rows.

The packed path is the one shape that crosses **once per field however long it is** — the
host's own array handed over whole. For `bool` and `enum` the binding materialises a
contiguous array first, because a `Vec<TaskStatus>` is not the wire representation; a host
that already stores the wire form hands over a pointer and copies nothing. That is a real
per-host cost the ABI's "one symbol per host layout" wording implies and does not state.

#### M7: decode only, and the permutation is the whole statement

No canonical writer can produce its bytes. All four arms decode the committed vector to the
same value, and a contiguous re-encode is a permutation of the same (tag, wire type, body)
triples — the method stage 1 used, and the only statement that can be made about it.

#### What the timings say, and one number that needed a control before it was safe

Ratios to prost, three runs (`ffi/logs/rust/stage3-M4-M7.log`):

| payload | direction | core-native | core-ffi-rust |
|---|---|---|---|
| P4.1 (adapter site) | encode | 0.45 - 0.47 | 0.80 - 0.82 |
| P4.1 | decode | 0.84 - 0.86 | 0.90 - 0.92 |
| P5.3 (1 MB bulk) | encode | 0.96 - 0.97 | 1.00 - 1.01 |
| P5.3 | decode | 0.45 - 0.46 | 0.45 - 0.46 |
| P5.4 (4 MB bulk) | encode | 1.01 - 1.04 | 0.79 - 0.83 |
| P5.4 | decode | **0.084** | **0.080** |
| P6.1 (mixed) | encode | 0.55 - 0.56 | 0.61 - 0.62 |
| P6.1 | decode | 0.86 - 0.87 | 0.56 - 0.60 |

**Two things in this table were my harness and not the codec, and one of them would have been
published as a finding.**

First: `core-native` on P5.4 encode measured **1.412** of prost. The arm was allocating a
fresh `Vec` per call and growing it by doubling from 4 KB to 4 MB — about eleven
reallocations and 8 MB of copying — while every other arm reused a warm buffer. prost's
`encode_to_vec` computes the length first and allocates once, so the comparison was a
growth-policy comparison. Fixed by giving the arm the same reused `Enc` the M1 to M3 cases
use: **1.412 becomes 1.018**. A false regression, caught only because a 4 MB payload made it
large enough to disbelieve.

Second, and the reason a control exists in this log: **P5.4 decode at 0.08** is a twelve-fold
win, which is not a number to report without asking what the floor is. So the bench gained a
raw copy of the same 4 MB as a case:

| P5.4 decode | ns | / prost |
|---|---|---|
| prost | 3,373,942 | 1.000 |
| `core-native` | 283,560 | 0.084 |
| `core-ffi-rust` | 269,521 | 0.080 |
| **raw `Bytes::copy_from_slice`** | **277,197** | **0.082** |
| **raw `slice.to_vec()`** | **270,498** | **0.080** |

The core arms are **on the memcpy floor, to within the noise**. So the honest statement is not
"the core is twelve times faster than prost"; it is **"a 4 MB bulk decode costs exactly one
copy in the core, and costs prost twelve"**. That is a bounded claim about both sides rather
than an unbounded one about ours, and the control is what makes it safe to quote. The
mechanism on prost's side is a suspicion and not a measurement: its `bytes = "vec"` merge
runs over a `Take<&[u8]>` inside the nested message, and the penalty grows with size (about
2x at 1 MB, about 12x at 4 MB), which is consistent with a chunked copy and not with a single
`extend_from_slice`. **Not verified, and labelled as such.**

It also bears on ABI v1 section 8's claim, which is that the direct-argument path takes a
4 MB upload to 0.16 to 0.34 of protobuf-java. The Rust equivalent of that claim is
*decode*-side here and it is stronger, but it is a statement about where the incumbent sits
relative to a copy, not about what the ABI bought: `core-native` makes no crossings at all and
is on the same floor.

### 2026-09-18 — re-validation after the adapter fix (`0c2d4d7f`)

D11's rule applied to a schema change that moved five payloads. `gen/stage1.sh` re-run into
`ffi/logs/rust/stage1-manifest-vs-prost-adapter.log`: **16 of 16**, with P2.1 at 1,037 B,
P2.2 at 540,422, P2.3 at 647,024, P2.4 at 979,465 and P4.1 at 65,321, matching the
aggregating session's regenerated values. P2.5 unmoved.

**The two construction routes earned their keep here.** The three facade arms, driven by the
generated builder, matched the new hashes the moment the generator was re-run. The `prost`
arm did not, because its objects come from the hand-written builder in `crates/stage1-validate`
— and the conformance run said so, per payload, rather than everything passing or everything
failing together. That is exactly why they are kept separate, and it is the first time the
separation has caught anything.

**The states are reachable now, checked off decoded values rather than from the builder**
(`crates/harness/src/bin/shapes.rs`, and it is a standing check rather than a one-off):

| site | state | count |
|---|---|---|
| nested, P2.2, 500 elements | `Ok` (success, no error) | 167 |
| | `Error` (error, no success) | 167 |
| | `Invalid` (no child) | 166 |
| | **impossible** (success AND error) | **0** |
| plain, P4.1, 200 elements | `Error` (non-empty) | 67 |
| | **`Ok` or `Invalid`, indistinguishable** | **133** |

The plain site's second row is the finding made reachable: 133 elements carry a state the
wire form cannot tell apart, so an adapter must answer one of two ways for all of them and be
wrong on the other. That row was 0 before.

**M2 and M4 re-measured** (`ffi/logs/rust/stage3-M2-M4-revalidated.log`), because the
aggregating session should not decide from a distance that ratios have not moved.

- **The mechanisms are unchanged.** Crossings are identical to the digit: 10.024 per task on
  encode, 7.004 on decode. ABI v1 decision 5 is unchanged too: P1.2 and P2.2 still converge
  to zero warm misses, and P2.4 still misses exactly 1.00 per element at the element site.
- **Most ratios did not move**, and the new runs are markedly **tighter** than the originals,
  so they supersede them: `core-ffi-rust` on P2.4 decode was 0.809 to 0.981 across three runs
  and is now 0.959 to 0.963.
- **Two moved, both toward parity on the core arms**: P2.4 encode `core-ffi-rust` 1.03 to
  1.11, and P2.2 decode 0.95-1.02 to 1.03. Consistent with the payload's composition rather
  than with any mechanism: the adapter no longer writes a string on every element, so P2.2
  carries 17,167 strings instead of 17,500 and the container work per element is a larger
  share. That is the container-construction reading holding up under a payload change nobody
  made to test it.

**New**: M4 gains timing rows it did not have, now that the shape is real: encode 0.411 to
0.415 (`core-native`) and 0.789 to 0.816 (`core-ffi-rust`); decode 0.861 to 0.864 and 0.916
to 0.920.

### 2026-09-18 — stage 4: the RPC arm

Deliberately small, as `design/SHAPES.md` says it should be. One unary RPC carrying P2.2 over
loopback against tonic 0.14, the crossing count per RPC, and CPU at 1, 8 and 16 in flight.
Log: `ffi/logs/rust/stage4-rpc.log`.

#### The number other slices cannot get from their own

**Two crossings per RPC, and zero per field.** `ak_call_unary` in, `ak_bytes_free` out, and
nothing else. The reason is structural rather than measured and it is worth stating that way:
**nothing in `crates/rpc` or in `ak-core`'s `rpc` module mentions a message type.** The RPC
half dispatches on a path string and moves opaque bytes, so there is no place for a per-field
cost to enter. If the count were a function of field count, something in those two files would
have to know about fields, and nothing in them does.

That is the claim the "adopt the RPC layer, generate the codec" fallback rests on, and it is
now a property of code rather than a sentence in a specification. The codec's crossings are
separate and already counted: 10.02 per element on encode and 7.00 on decode for this message.

#### CPU per RPC

| arm | in flight | CPU µs/RPC | wall µs/RPC | CPU / tonic |
|---|---|---|---|---|
| tonic | 1 | 1550 | 32,985 | 1.000 |
| tonic | 8 | 1450 | 855 | 1.000 |
| tonic | 16 | 1771 | 943 | 1.000 |
| `core-ffi-rust` | 1 | 1600 | 31,740 | 1.032 |
| `core-ffi-rust` | 8 | 1550 | 900 | 1.069 |
| `core-ffi-rust` | 16 | 1615 | 715 | 0.912 |

A second run, a different process, gives 1.000, 0.967 and 1.097 for the same three rows. So
across two processes the arm sits at **0.91 to 1.10 of tonic**, and with a 10 ms CPU clock and
four shared vCPUs that is **no measurable difference**, not a win and not a loss.

That is what two crossings of 1.8 ns against a 1.5 ms call should look like: the boundary is
about four parts in a million of the call and is invisible. The useful form of the result is
therefore not the ratio but its arithmetic — **the RPC half cannot cost a host anything
measurable, because two crossings per call is two crossings per call whatever the runtime**.
A host whose crossing costs 98 ns through JNI pays 196 ns on a call of this size, which is
0.013 percent. That is the number this slice can hand the other four, and it does not depend
on Rust being the host.

**CPU and wall differ by a factor of twenty at one in flight, and that is h2 flow control
rather than the RPC path.** A 540 KB response exceeds the default 64 KB stream window, so a
single call in flight spends most of its time idle waiting for `WINDOW_UPDATE`; with eight in
flight the stalls overlap and wall-clock collapses to 855 µs. `SHAPES.md` asks for **CPU** per
RPC and the first version of this harness reported wall-clock, which would have said the RPC
path costs 33 ms per call — a statement about a default h2 window and about nothing this
branch is deciding. CPU is process `utime + stime` from `/proc/self/stat`, 10 ms resolution,
divided by the calls in a round; it is a per-round measurement and a per-call division, not a
per-call measurement.

#### A defect the concurrency arm found, which is what it is for

`ak_call_unary` took `*mut ak_client` and did `&mut *c`, so `cl.grpc.ready()` and
`cl.grpc.unary()` mutated state shared by every host thread. At one in flight it worked. At
eight it failed outright — the arm did not produce a wrong number, it produced no number.

The fix is the shape ABI v1 section 9 already implies: the client holds the **`Channel`**, not
a `Grpc`, a call clones it (cheap, and clones share the connection) and builds its own `Grpc`,
and `ak_call_unary` takes a shared reference. Section 9 says "Ownership between handles is
internal. A call holds an `Arc` on its client's storage" — a client handle is meant to be
usable from many threads at once, and my first implementation was not. Recorded as D16.

**Two smaller process notes from the same episode**, both mine:

- The first attempt to apply that fix **silently matched nothing**: a `str.replace` with no
  assertion, on text I had already changed. It compiled, so I believed it, and it took a
  panic to find out. Every scripted edit in this slice now asserts its pattern matched.
- The failure surfaced as `AK_ERR_HOST` with the cause thrown away, because the call did
  `.map_err(|_| ())`. An error channel that discards the error is not an error channel; the
  trace behind `AK_RPC_TRACE` exists now, and ABI v1 open decision 9 (the diagnostic
  contract, "five distinct transport failures currently render as one string") is exactly
  this problem one level up.

#### What stage 4 does not measure, and does not substitute for

Beyond the carrier-thread row above: the callback and completion-queue delivery modes, TLS,
retry and backoff, metadata, deadlines, the gRPC status code as a number, cancellation
(section 9 gives the blocking call a handle so it can be cancelled and `ak_call_unary` takes
none), streaming, a real network, failure injection and the server side. **The RPC half's case
is behavioural** — one retry set, one backoff, one TLS configuration, one cancellation
contract — and none of that is exercised here. This measures the call path only, and the call
path is the half of section 9 whose case was never in doubt.

The concurrency rows are a **lower bound and an upper bound on nothing**: four vCPUs carry the
server's two worker threads, the client's runtime and N host threads, so at 8 and 16 the
machine is oversubscribed before the measurement starts. What survives that is the two arms'
relative behaviour under identical oversubscription, which is why the ratio column exists and
the absolute column is there to be reproducible rather than to be quoted.

## Decision 3, third framing: the UTF-8 check moves to decode, and is priced there

**Log**: `ffi/logs/rust/stage3-decode-utf8-policy.log`. **Driver**: `gen/decpolicy.sh`.
**Bin**: `crates/harness/src/bin/decpolicy.rs`.

The coordinator's third framing removes UTF-8 validation from encode entirely and asks what
validate-and-**reject** costs on decode, both validators, all three content sets, with the
encode rows falling back to the passthrough already measured.

### What was built

`crates/ak-rt/src/strings.rs`: `decode_str`, one function, three build-time policies
(`lossy` by default, `dec-reject` = `core::str::from_utf8`, `dec-reject-simd` =
`simdutf8::basic`). The **generator was swept, not the call site patched**: every string
materialisation in the generated `core_native.rs` (37 sites) and every `s_of` in the
generated `binding.rs` (38 sites) routes through it, and `from_utf8_lossy` no longer appears
in either generated file. `s_of` now takes the decode context so a malformed span reports
through `ak_fail` — which is the first thing in this slice to reach the decode error channel
D7 built, previously "reachable only through a panic".

### What it measured

**On the string path alone, in ONE process** (section 4, the table that does not depend on
three builds agreeing): scalar validate-and-reject is **0.54 to 0.75 of today's lossy path on
ascii, 0.83 to 0.97 on latin1, 0.94 to 1.10 on wide**. With `simdutf8::basic` it is **0.36 to
0.71 of lossy on every content set**. `from_utf8_lossy` already validates; it substitutes
instead of failing, and its chunked recovery path is slower than `from_utf8`.

**On a whole decode** (section 3, three builds, prost as the in-process control): P1.2 ascii
goes from 0.87-0.91 of prost under lossy to 0.75-0.79 scalar and 0.73-0.75 simd; P1.2 wide
from 0.78-0.80 to 0.80-0.82 scalar and **0.49-0.51 simd**; P2.2 wide from 0.86-0.96 to
0.87-0.98 scalar and 0.62-0.73 simd.

### What it refuted

- **That validation has a price wherever it is put.** It does not. The encode-side figure
  (2.0 to 2.6 of prost on non-ASCII, `stage3-content-sets.log`) came from adding a scan to a
  memcpy. On decode the scan is already there, so rejecting is free to cheaper and the
  worst case measured is a tenth of the string path on the widest content.
- **That a rejecting decode would cost the comparison against prost.** The opposite: prost
  rejects too, so the two sides now do the same thing and the ratio moves in this slice's
  favour. The earlier decode column was **pessimistic**, not flattering — it was paying for a
  slower validator to get a weaker guarantee.
- **That the SIMD arm was an encode-side curiosity.** On decode it is the largest single
  effect measured in this slice after the boundary itself.

### Two defects and one hazard found on the way

- **The sticky error slot was never cleared on decode.** `ak_decode_X` returned
  `(*dcx).hdr.err` and nothing set it back to `AK_OK`, so the first rejected decode poisoned
  every later one on that context. Invisible while nothing could fail. Fixed in the
  generator: the entry point clears it, which costs no crossing (counts re-run: M1 9/6, M2
  10.024/7.004, unchanged to the digit) and takes the obligation off the binding author. A
  host-side `ak_dec_err_reset` was written first and reverted, because it is one extra
  forward crossing per decode for nothing. The regression is in the bin: a good decode after
  a rejected one must succeed.
- **The first driver built and ran each policy in turn, so lossy was always first.** Its
  `core-native` P1.2 ascii row read 0.778 of prost in one invocation and 0.88 in the next,
  from the same source, with the prost control unmoved. Ordering and code placement, not
  policy. The driver now builds all three binaries first and runs them **round robin with a
  rotating order**, and section 4 was added so the policy question has an answer that does
  not depend on three builds agreeing about anything. The control row still drifts up to 7
  percent on the two `wide` rows and the log says so rather than smoothing it.
- **The corpus is harvested by reflection** over the descriptor (`prost-reflect`), not by a
  hand-written list of string fields, so "every string in the payload" is a fact about the
  descriptor. The first attempt used the wrong package name (`armonik.api.grpc.v1` instead
  of `armonik.ffi.shapes.v1`) and panicked, which is the right failure mode for it.

### What was deliberately not done

The opt-in diagnostic encode mode, by instruction. No new encode measurement: `Ctx::new()`
already builds `Tcs::trusted()`, so every `core-ffi-rust` encode figure in this slice was
already the passthrough column. `ak_tc_bytes()` and `ak_tc_utf8_trusted()` return the **same
function pointer** in this tree, so the collapse the design predicts has already happened.

### One thing raised and not taken

The rejecting path returns `AK_ERR_TRANSCODE` (-6), because that is the code the design text
names for this case. `AK_ERR_MALFORMED` (-2, "invalid wire") is the other defensible reading:
proto3 makes invalid UTF-8 a **parse** error and the rejecting path has no transcoder on it.
That is not a slice's choice to make.


## Auditing the per-element interface cost: was it the group, or was it inlining?

**Log**: `ffi/logs/rust/stage3-inlining-term.log`. **Driver**: `gen/inlining.sh` and
`gen/inline_check.sh`. **Bin**: `crates/harness/src/bin/inlining.rs`.

The objection: `core-native` is compiled into the harness, so rustc fuses the traversal into
the benchmark loop and keeps writer state in registers, and the FFI arm cannot. Then
`core-ffi-rust − core-native` bundles "was not inlined" into "crossed a boundary and
materialised a group", and the quoted per-element figure is an upper bound.

### What I did first, and it turned out to be the answer

Before building anything I asked whether the premise was even true, from the artifact rather
than from reasoning — the same move as R5's `nm -D` check. In the `bench` binary that
produced the published numbers:

```
enc_list_results_response                4,299 bytes
decode_list_results_response            11,311 bytes
encode_into_list_results_response           20 bytes   (a thunk)
largest bench::main::{{closure}}           472 bytes
```

**No benchmark closure can hold an inlined copy of either traversal.** Both entry points are
exported globals in a PIE, so they are reached with `call *0x..(%rip)` through the GOT — an
indirect call, structurally the same shape as the call into the cdylib. `core-native` was
never inlined, so there was no inlining advantage to subtract.

### The two arms, built anyway, because the artifact check is an argument and not a measurement

`core-native-noinline` (`#[inline(never)]` on the per-message entry point: a real call, same
crate, rustc may still reason about the body — a lower bound) and `core-native-opaque` (the
same entry point through a `black_box`ed function pointer: no inlining, no devirtualisation,
no constant propagation across). Both at the granularity the FFI call sits at, both calling
the same generated traversal, both checked byte-identical against the validated manifest.

### What it measured

The inlining term, ns per element, min..max over three runs:

| payload | dir | inlining | inlining_lo | group+call |
|---|---|---|---|---|
| P1.3 | encode | −0.10 .. 0.01 | 0.13 .. 0.17 | 11.29 .. 11.42 |
| P1.3 | decode | −1.03 .. −0.82 | 0.49 .. 0.66 | 27.63 .. 28.36 |
| P1.2 | encode | 0.19 .. 0.60 | −0.47 .. 0.70 | 43.23 .. 45.13 |
| P1.1 | encode | −0.20 .. 0.17 | −0.34 .. 0.65 | 29.60 .. 30.46 |

**The absent-path inversion is not an inlining artifact.** And the dynamic call is not the
cost either: at 9 crossings per 1000 elements and 1.8 ns each it is about 0.02 ns per
element, so `group+call` is group materialisation with a rounding error attached.

### What it refuted, including one thing I had published

- That `core-native` is inlined into the benchmark loop. It is not, in either binary.
- That the per-element figure is substantially an optimiser artifact. It is 0 to 1.4 percent
  of it on encode.
- **And one of my own rows**: on P1.1 and P1.2 DECODE, `core-native` and `core-ffi-rust` are
  inside each other's spread and the sign flips between builds — `core-native` faster in
  `bench`, `core-ffi-rust` faster in `inlining`. A per-element interface cost should not be
  quoted for those two rows in either direction. That is a correction to the published
  table, but not the one the objection predicted.

### What I would have missed by building first

If I had built the two arms and measured "no difference", the honest first hypothesis is the
branch's own rule — *a combination of A and B that measures equal to B alone means A is not
in the build* — and I would have spent a session hunting a defect in the arms. The artifact
check says why there is no difference, so the null result is a result rather than a suspect.


## Arm 2: the zeroed-group element fill, as an arm (ABI v1 open decision 9 candidate)

**Log**: `ffi/logs/rust/stage3-zeroed-group.log`. **Driver**: `gen/zeroed.sh`.
**Bin**: `crates/harness/src/bin/zeroed.rs`.

The host memsets the element-group chunk once and assigns only the fields that differ from
the default, instead of ABI v1 section 6's total fill.

### What I built, and where

In the **generator**, not at a call site: `fill_<msg>_sparse` for every group type,
`loop_<root>_<slot>_zeroed` for every top-level repeated-message slot, and
`encode_into_<root>_zeroed` per root — all emitted **alongside** the default path, which is
untouched. The only other generator change was visibility: five helpers in the binding went
from private to `pub(crate)` so the arm could reach them. Conformance and the boundary-call
counts are identical to the digit afterwards, which is what says the default path did not
move.

### What it measured, six runs

| payload | what it is | ns/element | zeroed/total |
|---|---|---|---|
| P1.2 | M1, every field present | −1.66 .. +1.70 | 0.986 .. 1.014 |
| P1.3 | M1, the absent path | −4.98 .. −4.06 | 0.719 .. 0.766 |
| P2.2 | M2, the deciding shape | −26.18 .. −13.02 | 0.970 .. 0.985 |
| P2.5 | M2, the absent path | −0.42 .. +0.17 | 0.983 .. 1.007 |

P1.3 encode goes from **1.108–1.188 of prost to 0.815–0.857**: the M1 absent-path inversion
is gone. The condition set for decision 9 — wins on the absent path, costs less than 5.4 ns
per element elsewhere — is met on every payload measured, worst case +1.70 ns.

### The two things that surprised me

- **P2.2 does not lose.** It was the row expected to decide against the variant and it shows
  a consistent small saving instead. `TaskDetailed` has 27 fields and many sit at their
  default even on a populated payload, and M2's per-element cost is dominated by ten
  crossings and the inner runs rather than the outer group fill.
- **P2.5 gains nothing.** Positive control 2 explained it: breaking `owner_pod_id` changed
  P2.5's bytes, so P2.5 is not fully absent and there is little for a sparse fill to skip.

### A correction to the framing, which I would have missed by not implementing it

Section 6 prices the total fill as buying "the codec does not reset the element group between
elements, worth 5.4 ns per `ResultRaw` and 24.4 per `TaskDetailed`", and the request framed
this variant as paying that back on every element. **It does not.** The array is the host's
chunk buffer, so under this variant the codec still never resets anything; what changes is
only the host's fill — an unconditional store per field becomes a bulk memset plus a
conditional store. The reset moved from a per-field reset the codec would have done to a bulk
memset the host does, and a bulk memset is cheaper per byte than scattered stores. That is
why it can win at all, and it is why 5.4/24.4 is the right threshold to judge the cost
against but is not the cost being paid back.

### Three positive controls, because a silent fallback would have passed everything

A zeroed arm that quietly fell back to the total fill would pass byte identity AND measure
the same — the exact situation the branch's rule about A-plus-B-equals-B is for. So each
path was broken on purpose, one line at a time, and the tree regenerated afterwards:

1. drop `ResultRaw.name` from the sparse fill → P1.1 and P1.2 zeroed DIFFER, total fill
   unchanged, P1.3 unchanged (its names are all empty). The M1 path runs.
2. drop `TaskDetailed.owner_pod_id` → all five M2 payloads' zeroed rows DIFFER. The M2 path
   runs, and P2.5 is not fully absent.
3. turn the presence test into a value test on `Probe.opt_count` → P3.1 zeroed DIFFERS.
   **Present-and-zero is load-bearing**, and M3 is in the conformance list for exactly this.

That third one is the defect this variant invites: for an explicit-presence field the test
must be PRESENCE and not value, because `Some(0)` and `Some("")` are at their default value
and must still be written. For an implicit-presence field it must be the value. Getting that
backwards turns present-and-zero silently into absent, and byte identity on a payload without
that case would not notice.


### Postscript: the mechanism, and where the coordinator's version of it is too broad

The aggregating session re-derived arm 1 from the binary independently (right call: the
finding exonerates a claim that session had already published) and added the mechanism —
no `[profile.release]`, LTO off, so a traversal in `facade` cannot be inlined into a closure
in `harness`. **That last clause is too broad and I have put the narrower version in the
log.** `core_native.rs` carries 38 `#[inline]` functions, including `enc_list_results_response`
itself, and an `#[inline]` function's MIR IS exported cross-crate with LTO off. What actually
holds is:

- the two entry points the benchmark calls — `encode_into_list_results_response` and
  `decode_list_results_response` — are non-generic `pub fn` with **no** `#[inline]`, so their
  bodies cannot cross the crate boundary at all. That is the load-bearing fact;
- the inner traversal is `#[inline]` and its MIR does cross, but it is 4,299 bytes against a
  472-byte closure, and the benchmark calls the 20-byte entry thunk rather than it.

The difference matters because it names a failure mode: **if the generator ever put
`#[inline]` on a per-message entry point, or made one generic, the objection could become
true again with LTO still off**, and the published ratios would quietly start including an
inlining advantage. `gen/inline_check.sh` would catch it — the closure would grow past the
traversal — which is the reason that check is a script rather than a paragraph in a log.


### R5 grew a second half, so the check moved out of its own driver

The aggregating session generalised the narrowing correction into R5: a slice must now prove
from the artifact that its **no-boundary control is not fused into the benchmark loop**, both
directions, **as a build step** — the mirror of the existing proof that the FFI boundary is
real.

`gen/inline_check.sh` already did exactly that, but it ran only from `gen/inlining.sh`, which
is the one-off audit driver. A check that runs only when you go looking for the problem is not
a build step. It is now a step of `gen/stage2.sh` and `gen/stage3.sh`, next to the
`nm -D --undefined-only` proof, so both halves of R5 are exercised by the standard suite and a
future session cannot regress the control without the log saying so.

Worth naming the symmetry, because it is the same mistake twice in opposite directions: D3 was
an arm that claimed a boundary and did not have one (rlib, everything inlined, counters still
incrementing because the counting code inlined too). This is the shape where an arm claims
**no** boundary and might not have that either. Both were invisible to R7's configuration
discipline, because neither LTO nor `#[inline]` nor genericity appears in a configuration line,
and both are only visible from the built artifact.


## Decision 11: the unknown-field bag, priced — and a drift that matters more

**Log**: `ffi/logs/rust/stage3-unknown-fields.log`. **Drivers**: `gen/unknown.sh`,
`gen/unknown_predicate.py`. **Bin**: `crates/harness/src/bin/unknown.rs`.

### The gating question, answered before building anything

ABI v1 7.2 batches a repeated field only if its element type is transitively free of repeated
and map fields. `gen/unknown_predicate.py` puts a bag into an in-memory copy of the schema
under each modelling and runs `emit/shapes.py:is_leaf` — the generator's own predicate,
imported, not re-derived:

```
leaf messages today                 9 of 19
leaf messages, a REPEATED bag       0 of 19
leaf messages, ONE bytes blob       9 of 19
```

A repeated bag is refused on structure alone: it costs `ResultRaw`'s batched run, which is
what turns 1000 rows into 9 crossings. Everything else was built on the one-blob model.

### What it measured

The empty bag — the case production is always in, because ArmoniK ships both sides from one
release — is **free on decode** (capture on/off 0.978–1.002, per-element deltas straddling
zero) and **costs 1 to 12 percent of an encode**. And the decision-9 interaction **goes both
ways**: on P1.3 the bag costs 8–12% under the total fill and 0.3–0.7% under the zeroed
variant (the memset absorbs an extra slot; unconditional stores do not), and on P2.2 the
reverse, 0.7–1.7% against 3.9–9.4%. Pricing it under one fill alone would have got the sign
of the interaction wrong on half the payloads.

### The two design calls, and the one that was mine

Append rather than merge, by instruction — so the round-trip property splits in two and the
log checks them separately: the **bag's bytes** are preserved exactly (checkable, and
checked, against the unknown runs of the input), the **message's layout** is not when an
unknown tag sits between two known ones, and that case is validated semantically.

The slot's third word was my call. I dropped it: **two words, not three.** Every other blob
slot carries a transcoder because the host's representation may differ from the wire's; the
bag's cannot, because it IS wire bytes a decoder captured, and with decision 3 settled
`ak_tc_bytes` and `ak_tc_utf8_trusted` are already the same memcpy. A transcoder there would
be a pointer whose only legal value is the identity — dead weight on every group of every
message and an invitation to set it wrong. Emptiness becomes `len == 0`, which is the test
section 8's direct-argument path already uses, so it is not a new convention, and the saving
is 8 bytes per group, which is directly the quantity decisions 9 and 11 interact through.

### The thing I nearly mis-attributed, and the drift it uncovered

My first timing run showed `core-ffi-rust` P1.2 encode at **1.07 of prost** against a
published **0.706–0.716**, and `core-native` at 0.54 against 0.425. My first hypothesis was
my own change: I had added `unknown_fields: Vec<u8>` to every facade struct, 24 bytes on
every message instance, which every arm walks.

It was not. Two fresh worktrees, same machine, same session: **at HEAD (`4afffd9b`) and at
`7fb30be5` the published ratios do not reproduce either.** `armonik` — prost's own codec over
the facade types, untouched by anything this slice has done since — reads 1.150 and 1.146
against a published 0.967–1.032. Every arm moved together and the drift predates the
zeroed-group work.

So the published M1/M2 table is not reproducible on this container today, at the commit it
came from. That is a bigger fact than anything in this arm, and it is why every figure in
this log is a delta formed inside one process with prost as the control in that same process.

The near-miss is worth recording as a habit, not a fact: **"my change made it slower" is a
hypothesis, and the cheap way to test it is to build the unchanged commit and measure that.**
A worktree with its own target directory takes five minutes and is the whole experiment; the
first attempt, `git stash` plus a rebuild in place, segfaulted because the on-disk cdylib no
longer matched the host that loaded it — which is its own small lesson about measuring an ABI
by swapping half of it.


## The re-run: a ratio does reproduce, and the published one cannot be rebuilt

**Log**: `ffi/logs/rust/stage3-reproducibility.log`. **Driver**: `gen/stability.sh`.

R4 now says ratios do not travel between builds of the same source. The task was to re-take
M1 and M2 under it and give a verdict on which column is right.

### The instrument, after the obvious one turned out to be empty

"Rebuild between run groups" was the thing to vary — except the release binary is
**bit-identical** across rebuilds of unchanged source (sha256 twice, after `touch`ing
lib.rs). So rebuilding on its own varies nothing, and the drift cannot be a rebuild artefact.
What actually changed between the published table and today is that the crate GREW, which
moves addresses. So the instrument became a **semantically neutral layout perturbation**: k
exported no-op functions that no arm calls, k in {0, 3, 11}. The binary hash changes every
time and the size barely moves, which is what a pure layout change looks like.

### The answer, which was not the one I expected

**The ratio is reproducible.** Across three builds and five to six runs, most rows have a
total spread under 0.05, and the across-build component is no larger than the same-binary
component on nearly every row. Layout is not the driver. The container is not unstable.

**And the published figures are 0.10 to 0.27 away** — five to ten times that band.

### Then the decisive experiment refused to run, which was itself the answer

To settle it I went to build `cc7f68c6`, the commit behind `stage2-four-arms-M1.log`:

```
error: can't find bin `bench` at path `crates/harness/src/bin/bench.rs`
```

**The benchmark binary is not in that commit.** The bins were untracked until `7fb30be5`
because of the root `.gitignore`'s `[Bb]in/` — **defect D19, which I recorded as hygiene and
which turns out to be the whole answer.** Counted per tree: `cc7f68c6` 0 bins, `d03c5161` 0,
`7fb30be5` 7, `160c37be` 9. Dependency versions identical throughout (prost 0.14.4, bytes
1.12.1), so no bump is in play.

So the verdict is not "the container drifted" and not "the numbers are noisy". It is: **the
published column is unreproducible because the code that produced it was never committed**,
and the oldest rebuildable commit agrees with today. I wrote D19 up as "the logs were
committed and the code that produced them was not"; this is what that costs when a figure is
questioned, and it is worth more as a demonstrated consequence than it was as a rule.

### What the re-take costs the headline

`core-ffi-rust` P1.2 encode is **0.982** against 0.706–0.716 published: through the C ABI the
core is **at parity with prost on encode for the uniform payloads, not thirty percent
faster**. The no-boundary arm survives intact — `core-native` 0.42–0.54 on every encode row —
so the codec is about twice prost and the C ABI gives that back. The decode side largely
reproduces. The encode side moved and the decode side did not, and no single arm's code
explains that pattern, which is why the log names no cause.

### The re-confirmation that also demonstrates the new rule

All three qualitative findings hold. The one worth singling out: UTF-8 validation on
non-ASCII reads **2.75–3.74 of prost** today against a published 2.0–2.6 — the `/ prost`
column moved with everything else — while **the same finding expressed as a within-arm delta
reproduces almost exactly**: 2.17–3.37 times its own ASCII cost against a published 2.2–3.0.
Same measurement, same session, two forms; the cross-arm ratio drifted and the within-arm
delta held. That is R4's new half demonstrated rather than argued, and it is the argument for
writing findings as deltas wherever the question allows it.

---

## Work unit: the pull decode family (ABI v1 section 7.1, open decision 2)

### What I expected, and it was wrong

The slice's own next-step list had this as "turn 'push is the right default at 1.8 ns' from
an argument into a measurement", and I went in expecting to confirm it: a family that trades
reverse calls for a materialisation should lose on a host whose reverse call is 1.8 ns.
Measured over six runs in two suite invocations, **pull is 0.94 to 1.18 of push and it is at
or below push on nine of twelve payloads**, including every M2 shape. The prediction was
wrong and the reason is that a push reverse call costs this host more than a crossing: it
goes through a vtable slot reached from across the shared object, while the replay's
equivalent is a local call over a buffer already in L2.

### The thing I had to check before believing it

A replay is host code calling host code, so rustc may inline `apply_*` and `add_*` into it;
a push callback never can be. That is R5's second half in a new guise, and subtracting the
families without checking it would have charged an optimiser difference to the interface.
So `core-ffi-pull-opaque` runs the identical replay with every call through a `black_box`ed
function pointer. **It measures the same as the plain walk arm on every payload.** The
parity is not inlining. Had I not built that arm the headline would have been defensible
only until the first review.

### What travels, and it is not the nanoseconds

Push's crossings are per element, pull's are per message: **3,501 reverse calls against 16
forward ones on P2.2**, or 3 if the host drains in one chunk, and 3 is the floor for every
payload in the set. That is a property of the descriptor, final under R13, and it is what a
host paying 80 ns an upcall re-prices.

### The control I did not plan and would keep

A record is written exactly where push makes a reverse call, by construction, so the two
counts must be equal. They are, to the digit, on all thirteen counted payloads. That single
line is stronger evidence that one traversal emitter serves both families than the correctness
gate is: byte identity says the two arms agree on the answer, and the count says they agree on
the *structure*. It also cost nothing — the records were already being counted.

### Where pull loses, the byte table answered it and the clock did not

P1.3 (+5 to +13%) and P6.1 (+8 to +18%) are the two losing rows, and I was about to write
"the absent path is worse for reasons unknown" when the footprint table gave it away: on P1.3
the record stream is **63.6 times the wire**, 38,488 B for a 605 B message, because a record
carries the whole fixed group of an element that encodes to nothing. Every payload whose
record-to-wire ratio is below 1 is at or under push. So pull's cost tracks the sparseness of
the element, not the size of the payload.

## Work unit: the concurrency suite (obligation 12.5)

### The first version found nothing, and it was the suite's fault

I ran the throughput arms on P1.3 and P2.5 — one M1 shape and one M2 shape — and the
per-context and process-global width tables measured the same. They are different messages,
so they index **disjoint length-prefix sites**, and a shared table is shared in name only.
Swapping to P1.1 and P1.3, both M1, made the same site want 2 bytes and then 1, and the
ranges separated at two threads and stayed separated at four. The old pair is now the
control row rather than deleted, because "different sites, same cache lines, no penalty" is
the statement that makes the other row mean true sharing.

That is the slice's standing question in its other form: not "is the change in the build"
but "is the arm doing the thing its name says". I then counted it rather than arguing it —
a warm context on one shape misses zero prefixes, the alternating pair misses exactly one
per encode.

### The positive control took three attempts and the third one is the finding

Plan A was to share one encode context across threads in-process and count wrong bytes. It
panicked. Plan B caught the panic with `catch_unwind` and counted panics. It aborted anyway:
the panic is raised on the far side of an `extern "C"` frame, the unwind is refused there,
and `catch_unwind` in the host never gets the chance. Plan C runs the control in a child
process and reads the verdict from its exit status.

**The third attempt is worth more than the control.** A shared context is not detected as
wrong bytes; it aborts the process. Which means ABI v1 section 5's error channel — which the
specification already calls the widest hole in the drafted interface — covers a failure the
*host* reports and has nothing at all for a panic inside the core, and every codec entry
point is exposed to that, not only a misused one.

### And the width table's correctness exposure is nil

Reading `Enc::end` while writing the global arm: `Mark` carries its width by value, so a
wrong learned width is always resolved correctly and costs a memmove. A global table is
therefore a throughput hazard and never a correctness one. That is why the global arm could
be built at all as a comparable arm, and it is also why section 6's sentence is about
threads rather than about bytes.

## Work unit: `ak_init` and the lifecycle (section 3)

### Two cases failed and both failures were the specification working

`panic-hook` failed first because I panicked in *host* code and expected the core's hook to
fire. It does not: a cdylib carries its own copy of `std`, so the core's hook and the host's
hook are two different globals. Section 3 warns about exactly that mechanism one level up
("two copies of the staticlib in one process either share Rust's globals or split-brain them
with no warning"); here it is `std`'s globals, and the split is the reason the hook is worth
installing rather than a defect. Testing it needed a panic *inside* the core, so the core now
has `ak_panic_test`, and the case's verdict is "the host's sink got the message before the
process aborted" — which is the whole of what the hook buys.

`codec-after-init` failed second, with `AK_ERR_INVALID_STATE` and `AK_DETAIL_OPTS_DIFFER`,
because the case called `ak_init` with one flag set and `Ctx::new()` then called it with
another. That is section 3's "a second call with different options fails", working. It is
also a consequence the specification does not spell out: **the flags are a process-wide
negotiation and the first caller wins**, so two independent components in one process cannot
both choose, and the second gets a hard failure for asking rather than the first's settings.
Two hosts loading one shared library is the normal case. It is now a case of its own
(`two-components`) rather than a fixed test.

### The guard is behind a feature on purpose

"Every entry point requires `ak_init`" is a claim with a price on the hot path, and no slice
had quoted it. Emitting the check unconditionally would have changed every slice's build and
made the price unmeasurable at the same time. Behind `init-guard` the default build is
byte-for-byte what it was and the two builds are the measurement. The check itself is
emitted as a post-pass over the finished codec text rather than at each of the ten places an
entry point is written, for the D12/D13 reason: a rule applied at nine sites out of ten is
the defect this generator keeps producing.

### The guard's price took three attempts, and the first two failed their own controls

I expected this to be the cheapest measurement of the session and it was the most expensive,
because the effect is small enough that the method kept being the thing I was measuring.

**Attempt 1, two builds with `bench` in each.** R4 says a comparison that cannot share a
process carries an in-process control, so the control is `core-native`: no guard in either
build, therefore it must not move. It moved by up to 30 percent — P1.3 encode 0.535-0.553 of
prost in one build and 0.367-0.387 in the other. I then tried re-expressing it as the
within-build ratio `core-ffi-rust / core-native`, which is R4's sharpened form, and that does
not save it either: the denominator is the thing that moved. **A ratio whose control moved is
not a figure**, so the run is in the log as evidence for the refusal rather than as a number.

I also lost the first attempt at that run to two `bench` processes overlapping on the box,
which is README section 11 word for word: "two benchmarks on one box corrupt each other
silently: the numbers still come out". `gen/guardprice.sh` now refuses to start if anything
else is benchmarking.

**Attempt 2, one process, `ak_noop` against `ak_noop_guarded`.** It reported the guarded
crossing as 0.71 ns *cheaper*. A load and a branch cannot make a call cheaper, so the arm had
the wrong sign — and by this slice's own standing rule that means the effect is under the
noise and I have not measured the noise.

**Attempt 3 measures the noise.** `ak_noop2` is a twin: same body, no guard, so it must read
zero against `ak_noop`. It reads **0.70 ns**, at a crossing of 2.1 ns. Two exported functions
with identical bodies are not the same cost — different address, different cache line,
different PLT slot — and **which one draws the penalty is not stable across builds**: attempt
2's binary had no twin and put it on `ak_noop`, which is exactly what made the guard look
like it cost 0.70 ns. With the twin present the guarded arm sits within 0.003 ns of it on
every run. So the answer is a bound and not a value: `|guard| < 0.70 ns per crossing, ~0
directly`.

**What I would keep from this.** The rule I already knew — an arm with the wrong sign means
the effect is under the noise — does not by itself tell you what to do next. What to do next
is *add an arm that must read zero*. That is the same device as `core-native-opaque` and as
the concurrency suite's planted violation, pointed at the measurement rather than at the code,
and it turned an unusable number into a statement with a number attached to its own
uncertainty.

## Work unit: the aggregating session's ruling, and what the cpp slice's log changed

The ruling approved both feature gates and told me to read `logs/cpp/concurrency.log`
before designing any more planted builds, because section 6 had been rewritten and the old
text would lead me to build the wrong suite. It did, and my suite was the wrong one in two
specific ways.

### Section 6's two refusals are independent, and I had only built half of one

The old text ran "the table lives in the context, never process-global" and "do not pad the
prefix" together. The cpp slice separated them and the rewritten section says: a global
table is a data race and a **throughput** defect and **not** a byte defect, because an
unpadded prefix is rewritten to whatever width the body actually needs; padding IS the byte
defect; and **only the combination** corrupts in the way a naive suite cannot see.

I had reached the first half independently — reading `Enc::end`, `Mark` carries its width by
value, so a wrong learned width costs a memmove and never a wrong byte — and that is why my
global arm was a throughput arm. What I had not built was the padding plant, so **my suite
had never produced a wrong byte at all**. Its only plant was the shared context, which
aborts. A suite whose failing test is "the process dies" has not shown that the byte
comparison works.

Now four builds, with the must-pass and must-fail in the script: shipped 0, global 0, pad 10,
pad+global 1,410.

### The oracle was the code under test, which is the mistake the combination exists to punish

My reference was a re-encode with a fresh `core-ffi` context. Section 6 says that cannot
catch the combination, because the threads agree with each other. I could have taken that on
authority; instead I ran both oracles over the same encodes and printed both columns:

    build                    vs prost   vs a fresh core-ffi context
    shipped                         0                            0
    pad-widths                      4                            4
    global + pad                    4                            0   <-- blind

A "fresh" context is only fresh in the state that is per-context. With a global table it
reads the same pollution, pads the same way, and agrees. That is now section 1b of the
suite, and it costs nothing on the builds that must pass — both oracles report zero.

### Where my throughput number sits, and why it is not a contradiction

Mine is 1.005-1.070 at two threads and 1.054-1.109 at four; cpp got 1.83-2.05 contended and
java 1.32-2.23. The cpp slice's own refinement resolves it: the cost tracks how often the
table is **written**, and its read-mostly leg is 1.13-1.23 with no scaling loss at all. My
pair writes the table **exactly once per encode** — counted, not assumed: the learned width
converges within an encode, the first element misses and the remaining 3 or 299 hit. So I am
on the same curve at a lower write rate. Three hosts, one sign, a magnitude that is a
function of the write rate.

### The regen turned out to be nothing, which is itself worth having checked

I expected to regenerate cpp, java and csharp. `gen/generate.py --check` reports **0 stale in
all four slices**: the other generators reproduce the committed `codec.rs` and `abi.rs`
exactly, because it is one emitter over one description with one ROOTS list. csharp does not
write the core at all. What they need is a rebuild, not a regeneration — and the ABI gained
20 entry points and lost none, so their gates should be untouched.

### The one place I did not follow the ruling literally

"If it is free, leave it on" — the guard measured free, so it is on. But turning it on in
`ak-core`'s own defaults would turn it on for cpp and java, whose hosts do not call
`ak_init`, and every call would return `AK_ERR_UNINITIALIZED`. That is a change that needs
more than a regeneration in trees I may not edit, which is the case the ruling says to stop
and report. So it is on in this slice's defaults and off in the core's, and the report says
what one line each host needs.

### The pattern, not the incident: an oracle that is the code under test

The aggregating session points out that "the reference was the code under test" is now the
third instance in this branch in two days — the cpp slice hit it on its concurrency
reference and again on its validator, and I hit it on mine. Worth recording as a class
rather than three accidents.

**The shape it takes.** You need a known-good answer. The thing nearest to hand that
produces answers is the encoder you are testing, so you run it in a configuration you
believe is clean — a fresh context, a single thread, a first call — and treat that as the
oracle. It works, and it keeps working, right up until the defect lives in state that your
"clean" configuration shares. Then the oracle moves with the thing it is checking and the
suite reports zero.

**Why it is so hard to see from inside.** The oracle is not obviously the code under test.
Mine was a *fresh context*, which is the word that does the damage: fresh sounds like
independent. It is only fresh in the state that is per-context, and the whole point of the
defect was a table that is not. The cpp slice's was the same word wearing different clothes.

**The rule that would have caught it without knowing the defect in advance**: an oracle must
not share an implementation with the thing it checks — not a "clean instance" of it, not a
"fresh" one, not a first call. Different code, ideally a different author. This slice had
one sitting there the whole time: `prost`, a different codec over a different object graph,
already checked against the validated manifest by stage 2. Switching to it cost four lines
and zero measurement (both oracles report 0 where there is nothing to find).

**And the general form of the tell.** A planted defect that the suite does not catch is the
obvious signal, but it requires having planted it. The cheaper tell is the one this branch
keeps rediscovering in other guises: **if an arm and its control can be wrong in the same
direction, the control is not one.** That is the same sentence as R5's "an arm named 'no
boundary' is only a control if it is not fused into the loop", and as the twin that had to be
added to measure the guard. Three faces of one rule.

## Work unit: the content sets on every payload, and D20

I put this last because it was the lowest-value item on the list. It found the worst defect
of the session on its first run, and not the defect it was aimed at.

### What it reported, and why the first reading was wrong

P3.1 disagreed on latin1 and wide and agreed on ascii. That reads like a string-path defect,
and I nearly wrote it up as one. It is not: **ASCII reproduces it**. What the extension
actually changed was the ORDER in which payloads share one encode context — the old
`content.rs` ran M1 and M2 only, and every other binary built its values so that the M5 cases
came last.

### The mechanism

`String::new().as_ptr()` is `0x1`. `AK_STR_DIRECT` is `0x1`. `<[u8]>::as_ptr()` on an empty
slice returns the type's dangling-but-aligned pointer, and for `u8` that is the address 1,
which is the sentinel ABI v1 section 8 reserves for "these bytes are an argument of the call".
So every empty string and every empty bytes field was taking the direct-argument path.

**And it produced the right bytes.** On a context that has never encoded a direct-argument
message, `direct_len` is 0, so the direct path writes a zero-length field, which is exactly
what an empty field should be. Stage 1 through stage 5, every arm, every payload, every
content set: green. The wrong path was indistinguishable from the right one until something
put a non-zero `direct_len` in the context first.

### The thing I want to remember

R6 says a payload generator that fills every field cannot reach the absent path. True, and
this slice has three defects that prove it. But **there are three cases, not two**: absent,
present-and-empty, present-and-non-empty. The absent path has a rule and a payload (P1.3,
P2.5). The present-and-empty path has neither — it exists in this schema only because M3's
`opt_label` is an explicit-presence field that happens to be set to `""` in 21 of 200 probes,
and that is an accident of the value generator rather than a designed vector. P1.3 does NOT
catch D20, because its strings are absent and never reach `enc_blob` at all.

The second half is a rule about state rather than about values: **a defect can live in
per-context state that only ONE message type ever writes.** Every gate in this slice built
its values fresh and its contexts fresh, or reused a context within one message type. Nothing
crossed. The corpus (README section 10) asks for distinct tags and multiple chunks; it does
not ask for a sequence across message types on one context, and after this it should.

### Where the fix went, and what I did not fix

The defect is the HOST's: the core behaves exactly as section 8 specifies, and it is the
binding that must not hand it a data pointer of 1 for a non-direct field. `str_arg` and a new
`blob_arg` route through `data_of`, which emits null for an empty slice — legal, because the
ABI discriminates absent from empty by `tc` and not by `data`.

What I did not fix is the hazard: **section 8 chose a sentinel from a range a legal empty
buffer can occupy**, and nothing in the specification warns a binding author. That is an ABI
decision and it belongs to the aggregating session. The other four slices' bindings have not
been checked for the same collision and I cannot check them.

### And the measurement the pass was actually for

It came out clean and slightly more interesting than expected. "The content set changes no
decode verdict" now holds on eleven payloads instead of two, with prost moving as much as the
core arms or more everywhere. The magnitude is new: the ratio to prost moves by up to 0.28 on
a decode row. And there is exactly one sign change in the whole table — P2.4 encode goes
1.120 (a loss) on ascii to 0.766 (a win) on wide, on the payload built to defeat the learned
width. The payload constructed to be hostile to the mechanism is also the one whose verdict
is most content-dependent, which is tidy and which nobody could have seen while it was only
ever run on ASCII.

---

## Stage 6 — section 9's deliveries at the floor, the A/B/C grid, and two corrections

**Asked for**: price the three deliveries at the floor so the managed slices' delivery
tables have a control; build the RPC arm as an A/B/C grid over a Unix socket with ArmoniK's
transport pinned; note that this slice's B − A is not a transport comparison; carry P2.2 at
1, 8 and 16 in flight with CPU as the headline.

Both landed. Neither is the most useful thing that came out of the work unit, which is worth
recording as a pattern in itself: this is the third work unit running where the deliverable
was routine and the control around it was not.

### The core could not express ArmoniK's transport

`ak_client_new` takes a URI and nothing else, so there was no way to set a stream window, a
connection window or a max message size through the ABI. Every RPC figure in this branch —
stage 4's, and the managed slices' — is therefore a 64 KiB-window figure whether or not its
log says so. Added `ak_client_opts` and `ak_client_new_opts` to the shared core (additive,
R0). This is the second time this session that a measurement I was asked to take turned out
to be unrepresentable through the ABI as specified, the first being section 3's lifecycle.

### Two harness defects found before any number was believed

1. **The callback arm spun.** It waited on its completion with `yield_now()` while blocking
   and queue parked. Callback read 1.37 to 1.67x blocking on CPU, which I nearly wrote down
   as a delivery cost. It was my loop. Replaced with a mutex+condvar `Signal`.
2. **Cell C built its context and its value inside the clock.** `Ctx::new()` and
   `armonik_arm::value(P2.2)` ran per timed thread, making C read 1.5 to 1.8x A. Hoisted out
   with a `CtxSlot` per flight slot.

Both had the same shape: **the arm that looked expensive was the arm I had written
differently**, not the arm that was different. Worth keeping next to the three earlier
instances of "the oracle was the code under test".

### The floor: what it is actually worth

The deliveries came back indistinguishable, as predicted. The measured fact is nearly
worthless on its own — the spread between the three arms reaches 71% at flight 1, so this
harness could not resolve a real difference either. What is worth something is the
arithmetic: the deliveries differ by **exactly one crossing** (2/0, 2/1, 3/0), a crossing is
1.8 ns here, an RPC is 1.5 ms, so the delivery choice is 1.2 parts per million of the call.
**No run of this harness could ever have shown a delivery difference**, and that is what
makes it a control rather than a result. I wrote the arithmetic into the binary's own output
rather than the log, so it travels with the table.

The useful form for the managed slices: if Java or C# resolves a margin between its three
deliveries, the margin is that host's price for the ONE crossing that differs plus whatever
its runtime wraps around a callback or a queue. Java prices an upcall at ~80 ns, 44x this
crossing, and callback is precisely the delivery taking a reverse crossing per call.

### Correction 1: the codec is the majority of a real RPC's CPU

Not asked for, and available only by accident: the deliveries table and the grid move the
same bytes over the same call, because `main` builds the server's fixed response as prost's
encoding of P2.2 and the deliveries arm sends exactly those bytes. So cell A minus blocking
is one encode plus one decode with everything else identical. It is **56% to 72%**.

Nobody in this branch had put the codec and the RPC on the same axis: stages 1-3 timed the
codec alone, stage 4 timed the RPC with no codec in the loop. It is the premise the whole
exploration rests on and it was unmeasured. Two caveats went in the log with it — cells A, B
and C are indistinguishable, so on THIS host there is nothing to recover; and a loopback with
a do-nothing server is the smallest denominator a deployment ever has, so it is an upper
bound on the fraction.

### Correction 2: there is no loopback-TCP penalty, and R9's mechanism is wrong

This one started as R5 asked of a new entry point — *is the window pinning actually in the
build?* — and the control answered a different question.

Pinning ArmoniK's 4 MiB windows moved the wall column by a few percent. That should not have
been possible if `README.md` R9 is right that a 540 KB response against a 65,535-octet window
"spends most of its wall clock idle waiting for `WINDOW_UPDATE`". Three steps then:

- The same 540 KB is ~2 ms over UDS and ~30 ms over loopback TCP. **Flow control is a
  property of HTTP/2 and is identical on both transports**; a transport-independent cause
  cannot produce a transport-dependent result.
- A **1 KB** response over TCP costs **44 ms**, more than the 540 KB one. So what is left is
  per-call, not per-byte.
- And the direction is the sharp part. If flow control drove it, the payload needing the
  round trips would cost more than the one needing none. It costs less. That is backwards for
  flow control and exactly right for Nagle.

Then the mechanism, from the source rather than from a guess: `crates/rpc` drives tonic
through `serve_with_incoming`, and **tonic documents that the builder's `tcp_nodelay` is
ignored on that path**; `TcpIncoming::from(listener)` leaves its own nodelay unset, so
`set_accepted_socket_options` never touches the socket. The server kept Nagle on while
tonic's client had it off by default. HEADERS, then DATA, then TRAILERS; the second small
write waits for the peer's ACK; Linux's delayed-ACK timer is 40 ms. 44 ms for 1 KB is that
timer plus a round trip.

One socket option: TCP 540 KB goes 32 ms to 2.1 ms, TCP 1 KB goes 44 ms to 0.15 ms, and
**TCP then matches UDS on both payloads and both columns**. There was never a transport gap.

Added `rpc::serve_nodelay` beside `rpc::serve` and left `rpc::serve` byte-for-byte as it was:
flipping it moves every slice's published loopback-TCP wall figure, which R0 makes the
aggregating session's call. Measured stage 6's TCP tables against the fixed server, because
reporting the defective one as a transport row would be reporting the artifact, and kept the
defective rows in the window-control table where they are the evidence.

What survives of R9: its conclusion. "Measure CPU, not wall clock" is better supported now,
not worse. What is refuted is the mechanism and the inference a reader draws from it — that a
slice showing a big wall/CPU gap on P2.2 has a window to raise. Raising the window bought a
few percent; one socket option bought 14x to 300x. `design/SHAPES.md`'s window subsection is
still correct for a real deployment; it is just not what the loopback numbers measured.

### And the waiter rule caught me writing it

`gen/rpcgrid.sh` opens with a refuse-if-benchmarking guard, which I wrote with
`pgrep -f 'target/release/(bench|...|rpcgrid)'`. It refused on its first run, because
`pgrep -f` matches the full command line of every process **including the shell running the
guard**, whose command line contained the pattern. That is failure mode 1 in `ffi/CLAUDE.md`,
committed by the same agent who read it, in the same session, into a script whose entire
purpose is hygiene. `pgrep -x` on the basename cannot self-match. Noting it because the rule
is clearly not enough on its own: the shape to watch for is *any* guard whose pattern is
written down inside the thing being guarded.

---

## Stage 6b — the flip taken, and an ABI defect underneath it

Both rulings came back as I would have hoped: `rpc::serve` flipped to `TCP_NODELAY` with
`serve_nagle` keeping the defective form reproducible, and R9 rewritten around the Nagle
diagnosis. Re-took the tables against the flipped server. Three things to record.

### The CPU column moved too, and that is the part that cost something

I was asked to say explicitly whether the loopback-TCP **CPU** column moved as well as the
wall column, because every slice has been told CPU is the trustworthy one. It did. Two rows
of one table cannot establish that on a shared box, so I interleaved them — nagle, nodelay,
nagle, nodelay, five adjacent pairs, so drift moves both members of a pair together and
cannot manufacture a consistent sign:

| pair | 1 | 2 | 3 | 4 | 5 | median |
|---|---|---|---|---|---|---|
| CPU ratio | 1.391 | 1.292 | 1.259 | 1.480 | 1.296 | **1.296** |
| wall ratio | 16.1 | 16.3 | 13.1 | 14.3 | 12.4 | **14.3** |

Five of five, same sign. Nagle on the server inflated loopback-TCP CPU by about 30%. Same
mechanism: a stalled write parks the reactor and a timer wakes it, so the call costs extra
epoll and scheduler work at both ends — and both ends are in this process, so both land in
its utime+stime.

The precise cost to the branch, which is narrower than "CPU was wrong": an **absolute**
loopback-TCP CPU figure from before the flip is inflated. A **ratio** between two arms that
both went through the same server is not. Stage 4's `CPU/tonic` column stands; its
`CPU us/RPC` column does not.

### D21: `ak_client_opts` was declared twice and the two disagreed

Rebuilding against the reconciled union did not compile, which is the only reason this was
found. `ak-core` defined six fields; `ak-abi` — the declaration **every host compiles
against** — still had four. The reconciliation updated the definition and not the
declaration. Nothing failed, nothing warned, and what a host would have got is worse than a
crash:

- its `max_recv_message` lands on the core's `adaptive_window`, and 2 MiB is `>= 0` and
  `!= 0`, so `http2_adaptive_window(true)` — **adaptive sizing on, overriding the very
  windows this entry point exists to pin**;
- its `max_send_message` lands on `max_recv_message`;
- `max_send_message` and `tcp_nagle` are read **past the end** of the host's 16-byte object,
  so `tcp_nodelay` comes from whatever was on the stack. A `1` there re-enables Nagle on the
  client and resurrects the 40 ms artifact non-deterministically.

Silent, wrong, and in the one setting the branch had just spent a day correcting.

The fix is one line of struct. The **durable** fix is the guard that was absent: a `const`
block beside the struct asserting size, alignment and **every field offset** against
`ak_abi`'s copy. Offsets and not just size, because two structs with the same six 4-byte
fields in a different order agree on size and alignment and disagree on every value.
Verified failing: reinstating the four-field declaration fails the build with the `rpc`
feature on.

The uncomfortable part is that **I had already written exactly this guard for
`ak_bdr_rec`**, in this same session, with a comment explaining why section 10 demands it.
I wrote the defence for one struct and did not generalise it to the next `#[repr(C)]` type I
added. The guard is not a clever idea that needs having twice; it is a rule that should
attach to every type declared on both sides of the boundary. Worth one sweep of the others.

### On R0 and the concurrent-addition gap

Recorded by the aggregating session as a gap in R0 rather than against either slice, which
is right: R0 stops a slice forking the core and says nothing about two slices adding the
same entry point on the same day. Worth noting what actually caught the collision, though —
not a rule and not a review, but a **compile error in a third party's harness**. If the cpp
slice and I had both stopped at "it builds for me", the union would have shipped with the
declaration mismatch in it. The layout assert is the thing that makes that structural rather
than lucky.

## WP4 item 1 / finding R-D1: the length-varint wrap, reproduced then fixed

The aggregating session handed this over as an unconfirmed review finding and authorized the
core change (FIX-PLAN WP4 item 1). The rule that mattered most here was **reproduce first, on
the current code, and log it** — the finding was "source verified, not run", and three of its
claims are the kind that a source read gets subtly wrong.

### Reproduction (`logs/rust/rd1-wrap-unfixed.log`)

`dec.rs`'s `len_body` did `if self.pos + n > self.buf.len()`. In a release build
`overflow-checks` is off (the workspace `[profile.release]` sets `lto=false` and says nothing
about overflow, so it defaults off), so for a length varint near 2^64 the add wraps and the
check passes. I built `rdrepro`, one case per process under `timeout 5`, driving each through
BOTH the C ABI (`ak_decode_ListResultsResponse`) and the core-native path:

- **(a) hang**: the finding's literal 11 bytes `7A F5 FF FF FF FF FF FF FF FF 01` — field 15,
  wire 2, length ~2^64 — hang both arms (exit 124). `pos` wraps backward, the root loop
  re-reads the same unknown field forever. Confirmed exactly as claimed.
- **(b) bad span delivered after error**: a results element whose `session_id` length is
  `u64::MAX` produces a span with `len` 0x`FFFFFFFF` (the observing vtable, which reads the
  span integers without dereferencing them, saw `session_id.len=4294967295`), and the trailing
  `flush!()`/`apply` delivered it with `apply_called=true` even though the call returned
  `rc=-2`. A host would have read 4 GiB out of bounds. Confirmed.
- **(c) abort across `extern "C"`**: a results element whose OWN length wraps made
  `&buf0[off..off + n]` panic inside `ak_decode_*`, and because a panic cannot unwind through
  an `extern "C"` frame the process aborts (SIGABRT, "panic in a function that cannot unwind").
  This is the same hole section 5 already calls the widest, reached from hostile wire rather
  than from a host panic. Confirmed.
- **u32 (R-D9)**: no entry length check, so a `>u32::MAX` length was accepted and the reader
  ran off the buffer (hang). Confirmed.

All four reproduced. Nothing had to be fixed that was not broken.

### Fix

`len_body` and `f64` now compare against the REMAINING bytes: `n > self.buf.len() - self.pos`
with `pos <= len` established (every read advances `pos` only after its own bounds check), so
the subtraction cannot underflow and there is no add to overflow. On rejection `len_body`
returns `(pos, 0)` — an empty, in-bounds span — which is what makes **every** `&buf[off..off+n]`
on the decode path sliceable by construction. So claim (c) is fixed at the source of the bad
`n`, not slice by slice.

For claim (b) the span fix alone is not enough: the trailing delivery has to stop. The decode
emitter (`gen/rust_abi.py`) now wraps the trailing `flush!()`/`apply` (push family) and the
`OP_APPLY`/`OP_APPLY_ELEM` record (pull family) in `if d.err == 0`, at all four emission sites,
so nothing is handed to the host after a decode error. This is a codegen rule, so it went into
the emitter and was regenerated, not hand-edited into `codec.rs`; the whitespace-ignoring diff
of `codec.rs` is exactly the guards plus the u32 checks and nothing else. `gen/generate.py
--check` is clean in this slice AND in the codec generator, so the two agree.

The u32 item (R-D9): `ak_decode_*` and `ak_parse_*` reject `len > u32::MAX as usize` at entry
with `AK_ERR_LIMIT`. Core-native materialises owned values, no spans, so it is untouched.

One subtlety worth recording about (b): after the fix, the `b-err` case (a valid element then a
malformed field) still shows `elems=1` — but `apply_called=false`. The valid element was flushed
by the `cur`-transition BEFORE the malformed field was even read; that is a validly-decoded
prefix, and the caller still gets `rc=-3` and must discard. What the fix stops is the delivery
of the IN-PROGRESS, post-error state — which is exactly what the `b-span` case now shows at
`elems=0`.

### Gate (`logs/rust/rd1-*.log`)

`len_body` carries three unit regressions in ak-rt (near-2^64 truncates; the returned span is
always sliceable across four hostile lengths; a valid length still decodes). Conformance is
16/16 byte-identical to the validated manifest with decode round trips; the pull family's value
identity is 16/16 across all four decoders; the unknown-field vectors and the oneof/presence
shapes all agree; the four-build concurrency suite behaved as required (shipped and global pass,
pad and both are caught). All of that is unchanged by the fix, because on valid input `d.err ==
0` and the guarded delivery runs exactly as before. `rd1-wrap-fixed.log` shows every case
returning promptly: (a) `rc=-3`, (b) `rc=-3` no bad span `apply_called=false`, (c) `rc=-3` no
abort, u32 `rc=-5`.

### Not swept here, flagged to the aggregating session

The wrap and the deliver-after-error are fixed **in the shared emitter and the shared runtime**,
so every slice that renders from `gen/rust_abi.py` and links `ak-rt` gets both. But the cpp
slice has its own `rt.h` `len_body` (FIX-PLAN WP4 item 1 names it) and the managed slices'
generated codecs render their own trailing-delivery epilogues from their own backends — those
are theirs to check against the same three claims. The u32 entry guard is likewise per-backend.

## 2026-09-24 (later) -- FIX-PLAN WP4 items 7, 9, 10 (R-D6, R-D8, R-D9), Part A of this session

Every finding reproduced before the fix, logged before and after.

### R-D6, the sticky error slot (confirmed, fixed in the shared core, c10e934)

`crates/harness/src/bin/stickyerr.rs` drives the real cdylib with hand-written callbacks
that call `ak_fail` and return AK_OK, and counts upcalls made after the failure. Before
(`logs/rust/rd6-sticky-before.log`): encode of M1 returned **2** (a successful 2-byte
encode) with `ak_enc_err=-1`; encode of M2 returned 4 with **7** upcalls after the failure;
every decode returned -1 but kept calling (`add`, `new` twice more, `apply`); a negative
element token without `ak_fail` returned **rc=0** and silently dropped all three elements.
After (`rd6-sticky-after.log`): 7 of 7 cases rc<0 and zero upcalls after the failure.
Emitter change in `poc/codec/gen/rust_abi.py` (encode upcall check, entry status,
decode flush guard, stop-after-upcall in every reader, element `new`/`apply`, root epilogue)
plus `enc_status`, the transcoder-as-upcall check in `enc_blob` and `UnkBuf::host_err` in
`ak-core`. Byte identity and crossing counts unchanged (`logs/rust/wp4-gate.log`).

### R-D8, concurrency coverage (confirmed, fixed)

`together()` now rotates all four shapes (P1.2, P2.2, P1.3, P2.5) per round with a per-thread
phase. 0 wrong on shipped and global builds; the must-fail plants now show 703 (pad) and 710
(both) wrong against 10/1,410 before -- the present-path payloads give the pad plant far more
sites to corrupt. **ThreadSanitizer ran**: nightly 1.100 installed in about a minute,
`gen/tsan.sh` instruments harness, cdylib and std (-Zbuild-std). Sections 1-3: 0 TSan
warnings; the planted shared-context control: 77 warnings, so TSan is seen working
(`logs/rust/rd8-tsan.log`). One thing needed: this nightly's target-dir layout puts a
dependency's cdylib under `build/<crate>/<hash>/out`, so `harness/build.rs` takes an
`AK_CORE_LIB_DIR` override (the tsan script sets it).

### R-D9, the core and script parts (confirmed, fixed)

- `from_raw_parts(NULL, 0)` in the transcoders: a debug-build unit test aborts on the
  standard library's precondition check (`rd9-tc-null-before.log`); guarded in all five
  transcoders and the direct-argument path (`rd9-tc-null-after.log`).
- `opts_word` XOR collision: a new lifecycle case builds two option sets that fold to the
  same word; the old core answered AK_ALREADY_INITIALIZED (`rd9-opts-collision.log`, built in
  a worktree at the base commit), the new one refuses with AK_DETAIL_OPTS_DIFFER.
- `lifecycle.sh`/`guardprice.sh`: the "guard OFF" arm was the harness DEFAULT build, and
  `init-guard` is a default feature of the HARNESS crate (not of ak-core), so both arms had
  the guard (`rd9-guard-off-arm.log`: the old OFF arm prints "init-guard: ON"). Now
  `--no-default-features --features guard`, own target dir. `stage2.sh`/`stage3.sh`'s
  accessor-guard row used `--no-default-features` alone, which dropped the init guard too:
  `--features init-guard` put back so the row changes one thing.

### New: `gen/gate.sh`, the correctness-only gate

Generators current, core unit tests, byte identity, shapes, counts, content sets
(correctness section), the four-build concurrency suite (timing section skipped via
`AK_NO_TIMING`), lifecycle, R-D1 reproductions (now with a verdict), R-D6. `wp4-gate.log`:
GATE PASSED.

### Found, not fixed in Part A (goes into WP5's rewrite)

The decode emitter names every nested reader `cd`, so at depth two the propagation line
reads `if cd.err != 0 { cd.err = cd.err; }` (rustc warns "useless assignment", four sites):
an error inside a message nested two levels below its group root is dropped at that level.

## 2026-09-24 (later still) -- FIX-PLAN WP5 step 1, the one generator (Part B)

Authorized by the aggregating session to change `poc/codec/**` for this.

### What was built

`poc/codec/gen/plan.py`, the rule layer: IR -> MessagePlan with an ENCODE PLAN (EncStep list
in tag order, a oneof as one guarded step per member at its own tag) and a DECODE PLAN
(`{(field number, wire type): DecAction}`), plus the ABI layout functions (moved here from
`rust_abi.py` and `ir.py`; `ir.py` keeps forwarding names for the unported generators), the
RPC ABI (R-G5) and the lifecycle (R-G7). Options: unknown drop/retain/both, utf8
reject/lossy, recursion limit. The contract is the module docstring, written for the
cpp/java/csharp/python agents: what a plan contains, the encode and decode rules stated
once (with R-G8's 64-bit remaining-bytes length rule), what a backend may and may not decide.

Backends: `rust_abi.py` (the core) and `rust_native.py` (core-native, new; one module per
unknown mode) render the same plans; `rust_binding.py` (the host binding, moved out of
`rust_abi.py`); `cpp_layout.py`. `rust_core.py` is retired as an emitter and kept as a
plan-ordered legacy adapter only because `poc/cpp/gen/cpp_core.py` imports its walkers.
`generate.py --check` now also fails if a backend imports the IR/schema, and self-tests that
guard with a planted import. The ir.py front end loads the corpus reader view through the
corpus's own merge (`spec.load()`), so `fixed32` and the corpus-only messages exist (R-E3).

### How it was gated, in order

1. Rewrite the encode walker over EncSteps and the decode walker over the table; regenerate
   the shapes core. `abi.rs` and `layout.rs` came out BYTE-IDENTICAL (same layout, same site
   order: the oneof's message site is allocated when the oneof is first met, as before);
   `codec.rs` changed as source. Conformance 16/16 and shapes: unchanged.
2. `rust_native.py`: core-native from the same plans. Conformance 16/16 again (the prost
   arm is the independent check; core-native and core-ffi share the plan by design).
3. The corpus core: every corpus message the ABI can carry as a root (all but `Nest`,
   refused by name), generated into `generated_corpus/` behind a test-only `corpus` feature,
   built in `poc/rust/corpus/` (its own workspace). The corpus-harness binding exposed two
   binding defects on the first build (D30: a root whose loop slot lives on an inlined child;
   D29 on the first run: an `.unwrap()` on an absent parent of the direct argument, which
   panicked on `S-UploadResultDataMessage-min`) and one harness defect (the parent polled
   the child without draining its pipes, so large rows read as timeouts).
4. First full run after those: all four arms pass. Not trusted until controls failed:
   planted projection key (C2), planted byte (C3), refusals turned into acceptances (C4),
   `ak_init` skipped on an init-guard core (R-G7) -- all four turn rows red.
5. Which form did retain write? 33 ffi rows wrote the DROPPED form. Looked: non-leaf
   elements (`U-element-*`, `U-chunkelem-*`) were never captured by the core even though the
   ABI has the `unk_` slot for them -- fixed (D33), 17 remain: `U-leaf-*`/`U-deep-*` (an
   unknown inside an inlined child: the ABI has nowhere to put it, D34) and `U-map-entry`.
6. The BEFORE: the pre-WP5 generator (c10e934) over the same corpus (WireZoo and Nest
   excluded, since it cannot express them), with the new binding generator over the old core
   because the old binding generator hit D30 (the layout is identical). 31 `T-dec-*`
   (invalid UTF-8 accepted) and 6/12 `U-wire-*` (packed at a foreign wire type read as a
   value) fail in every arm (`logs/rust/wp5-corpus-before.log`). These are the rule fixes;
   neither changes a byte on the payload set.

### Refuted / not done

- "Map key always written" (FIX-PLAN WP5 item 2) as a rule: refuted by the corpus
  (`E-map-entry-empty` accepts neither "key always, empty value omitted" nor anything but
  the canonical form or both-always), and it would move P2.5's manifest hash. The plan states
  the canonical form and says why; the question goes back.
- A retain-mode C ABI path for unknowns inside an inlined child: not built, because it needs
  an ABI slot that does not exist (D34). Not a backend's decision.
- The old binding generator could not be used for the before-run; stated in the log.

### Also found

- D28: the depth-2 reader-naming defect (found in Part A by a rustc warning) is real: on the
  old core a truncated `tasks[0].options.max_duration` decodes as a SUCCESS through the C ABI
  (`logs/rust/wp5-nested2-before.log`); `rdrepro nested2-*` is now in `rd1_repro.sh`.
- `gen/check_direct.py` could not run since R0 (imported `ir` from this directory); now asks
  `plan.py`.
- `gen/concur.sh`'s `| head` under `pipefail` made the counting run exit 101 on one gate run.
- The python generator reports `ak_abi.h` STALE at the base commit already (the cpp slice's
  header changed under it); not caused by this work.

## 2026-09-24 -- FIX-PLAN WP5 step 6: consolidation (authorized by the aggregating session)

### Done

1. `plan.FIXED` (FixedAbi): error codes and details, handles, fn types (ak_log_fn,
   transcoders, loop/unk callbacks), ak_str/ak_span/ak_blob/ak_uspan/ak_err/ak_init_opts/
   AkCounters/ak_bdr_rec, AK_STR_DIRECT/AK_TOKEN_ROOT/AK_BDR_*, the fixed entry points and
   export list, AK_INIT_* flag values and success codes, RPC counting surface; vtable member
   order, pull record numbering and `pull_slot` as plan functions. Rust renders the fixed part
   of ak-abi/src/lib.rs into a `plan.fixed` region and asserts the core's definitions with
   abi_check.rs (fn-pointer coercions).
2. `c_abi.py` (was cpp_abi.py) is the one C header; java_abi.py and cpp/gen/cpp_header.py
   deleted; C# declarations from plan.FIXED. rust_core.py deleted; references fixed
   (generate.py, one_core.sh, gen/corpus_before.py).
3. generate.py: guard over every backend (26), one command runs every slice generator.
   one_core.sh --selftest failed in the cpp gate's first run (cpp-1) because the scratch
   copy lacked ffi/corpus and saw untracked files; fixed, controls shown failing.
4. python mech generator and arms retired (3cee365), logs kept.
5. Rules: asked the oracles before changing anything. 10th byte: all three accept, kept
   "discard" (contrary to the brief). Field number > 2^29-1: upb and C++ refuse, pure-python
   accepts; rendered as ERR_MALFORMED in every generated reader and in ak-rt's skip_group.
   Map order: Java TreeMap<String> sorts by UTF-16 units and C# kept insertion order; both
   now sort by UTF-8 bytes. Before/after on a scratch probe manifest (not the corpus).

### Refuted / not done

- "The rules are now in every arm": refuted for the hand runtimes (D38) -- the in-group
  field-number check reaches only generated code and ak-rt.
- First java after-probe still accepted every field row on the ffi arms. First hypothesis
  "not running": correct. A fresh core build refused (-2) through ctypes, the java slice's
  `core-build/target-corpus` accepted; its build reused the target dir over a snapshot. Clean
  rebuild: ffi arms 0 fails. Same question asked of cpp: `build/` binaries predated the C++
  backend commit, so the second cpp gate run (cpp-2-stale-binaries) proved nothing; rebuilt
  with cmake and reran (cpp-3). D39.
- C# RPC counting not rendered (hand CoreTransport.cs would clash), D40.

## 2026-09-25 -- FIX-PLAN WP5 step 7: decision 11's unknown-field mechanism (authorized)

### Done

1. plan.py states the mechanism (UNKNOWN FIELDS ON DECODE): positions in preorder/tag order
   (`unk_positions`, `unk_offset`), `ak_dfix_M.unknown: ak_unk_buf` before `presence`,
   `ak_dec_<Root>_opts { host; ak_unk_opts <pos>... }` (owner amendment: one host pointer),
   `ak_dec_ctx_new_<Root>` / `ak_dec_reset_<Root>`, placement/ownership/discard/error
   rules. A map is `append_message` over its pair message (map_entry op removed; the five
   native backends re-pointed, output identical but a comment). dec_vtable lost `unknown`
   and `unk_<slot>`; FIXED lost `ak_unk_f`/`ak_uspan`, gained `ak_unk_buf`/`ak_unk_opts`.
2. Core: `UnkCx`/`unk_put`/`unk_arm` replace UnkBuf; the context holds its own copy of the
   armed positions. Push and pull capture at every position, inlined children and oneof
   members included; a oneof member re-entered keeps its buffer, emptied.
3. ak_grow_fn fits unchanged with realloc semantics (data/cap in = current buffer, NULL/0
   for a fresh one, first cap bytes preserved); its i32 want/cap refuse > 2 GiB (ERR_LIMIT).
4. Rust binding: `unk_grow` / `take_unk` / `unk_reclaim`; `decode_with_*_unk` =
   arm, decode, disarm (two forward crossings more than the old callback arm, and none on
   the drop path); `parse_walk_with_*_unk`; generated controls `unk_controls_*`.
5. Controls in `corpus --unk-controls` and gen/corpus.sh section 5, with a plant.

### Refuted / found

- First corpus run of the controls: S-double-nan "mismatched" -- NaN != NaN under
  PartialEq, a harness artifact; compared through Debug.
- The rust gate and one_core.sh ran the one-command check over EVERY slice, so they failed on
  the other slices' (expected) staleness; both now check `--core-only`, and the one command
  is run and reported on its own.
- Other agents were working in cpp/java/csharp/python while this ran: I regenerated the cpp
  tree once in the shared working tree, then restored every file I had written. The other
  slices were then regenerated and gated only in a scratch worktree (logs/rust/wp5s7/).
- `U-map-entry` stays a retention gap: the core delivers the entry's bytes (8), the facade map
  has nowhere to keep them (D42).

## 2026-09-25 -- FIX-PLAN WP3: the campaign harness (design/CAMPAIGN.md, req 22a: criterion)

### Done

- `crates/campaign`: codec suite on criterion 0.5 with a thread-CPU Measurement, Flat
  sampling, raw sample.json -> section 7 JSON lines; per-root arm table and read-every-field
  visitors from `gen/rust_campaign.py`; separate-process RPC grid (`rpc_server`,
  `rpc_client`, cells A-D); `calib`; `crossings` (counting build, every timed core-ffi case,
  committed as `gen/crossings.txt`); `run_campaign.sh`; gate.sh steps 11b/11c.
- Smoke run of all four suites in the container (instrumentation): `logs/rust/campaign/`.

### Found

- prost REFUSES 27 `U-wire-*` rows (a known field at a foreign wire type is a decode error in
  prost, an unknown field in protobuf and in the core). The incumbent is not timed on them;
  the header lists each.
- The in-process pre-check first demanded that retain re-encode a `U-*` row byte for byte;
  rows with unknowns before or between known fields are accepted in the "appended in tag
  order" form, so the check now uses the row's accepted encodings.
- The stock `counts` bin covers P1.x-P3.1 only; requirement 19 needed every timed input, so
  `crossings` counts all 112 inputs x 3 directions x 2 modes.
- `one_core.sh`'s R0 check (a second `codec.rs`) caught the first bench file name.
- Retain decodes cost two `ak_dec_reset_<Root>` forward calls that the core's context
  counters do not count (stated in the checklist).

## 2026-09-25 -- FIX-PLAN WP5 step 8: decision 11's confirmed implementation rules

- plan.py: rules 1-7 stated; positions: a oneof is ONE position (members with positions of
  their own refused); `unk_opts_layout` types each entry `ak_unk_opts` (occurs once per
  decode) or `ak_unk_pool` (can occur more: an element, a map entry, anything under one);
  FIXED gains `ak_unk_pool`, loses `ak_dec_ctx_new`; `ak_dec_reset_<Root>` returns i32.
- core: the host's struct is read IN PLACE through a generated per-root offset table;
  taking a buffer clears it in the host's struct; discard is decided at arm time from the
  entry as armed; a context carries its root and every decode/parse/reset of another root
  returns AK_ERR_INVALID_STATE; context creation is not init-guarded (as ak_enc_ctx_new).
- oneof: the buffer moves, emptied, from the previous message member's slot to the new
  one's (at most one member slot holds it) -- chosen so the group layout and the shared
  leaf decoders stay as they are; reported for confirmation.
- Rust binding: `DecCtxs` (one bound context per root); harness and campaign re-pointed.
- Controls (corpus --unk-controls): pool n=2 (no grow for two elements, one grow for the
  third; AK_ERR_CAPACITY without grow; entries cleared), in-place refill in `new_tasks`
  (and its no-refill control, AK_ERR_CAPACITY), oneof switch (one fresh buffer, final bag =
  the last member's run), wrong root (decode, parse and reset refused with -8).
- Found: the noinit corpus control crashed native arms too, because `DecCtxs::new` asserted
  on a NULL context from the init-guarded constructor; context creation is now unguarded,
  like `ak_enc_ctx_new`. one_core.sh's decode sentinel moved to `ak_dec_ctx_free`.
- Crossing counts: identical to gen/crossings.txt (no re-baseline).

## 2026-09-25 -- FIX-PLAN WP5 step 10: the NO-UNKNOWN variant (unknown fields compiled out)

- plan.py: `Options(unknown="drop")` on the C ABI is now a complete compile-time variant
  (docstring section THE NO-UNKNOWN VARIANT): `MessagePlan.unk_slot` False, so
  `group_fields` has no `unknown`; `unknown_compiled_out(p)`; `unk_entry_points` gives only
  `ak_dec_ctx_new_<Root>()`. Choice stated: the constructor takes NO parameter (the options
  type does not exist in the variant, so a "required NULL" parameter would name a type the
  header does not declare); no `ak_dec_reset_<Root>` (nothing to reset). The fixed
  vocabulary (`ak_unk_buf`, `ak_unk_opts`, `ak_unk_pool`) stays declared: plan.FIXED is one
  table for both variants.
- rust_abi: one traversal and one emitter for both variants; a zero-sized `UnkCx` stand-in
  compiles the position cursor away; capture (`_cap`), the oneof buffer move, the u-group
  walks' output (the walks still run so site numbering is identical) and the options
  section are omitted. The full variant's generated text is byte-unchanged (checked:
  generate.py --check against the committed files; one blank line the split first added
  was removed).
- rust_binding: the unknown-field prelude split out and re-inserted at its old place, so
  the full binding.rs is byte-unchanged; the variant has no `_unk` encoders, options,
  `decode_with_*`, controls.
- c_abi: `emit` of the drop plan renders a second complete header: same file name and
  guard, `#define AK_NO_UNKNOWN_FIELDS 1`, no u groups/opts/u-family/reset, 240 layout
  facts (400 full). The full header is byte-unchanged for every slice. Selected per build
  by include path. cpp_layout.facts per variant.
- Cargo: `unknown-fields` on ak-abi/ak-core (default on), forwarded by harness, campaign
  and the corpus harness; dependents use `default-features = false`. Found: a no-unknown
  build in the shared target overwrote `libak_core.so` for the full binaries; every
  no-unknown build now has its own target dir (target-nounk, target-count-nounk,
  target-corpus-nounk) and the runner checks each binary's core by its u-family exports.
- Gate: step 12 (byte identity + shapes + precheck + counts + c_variant.sh on the variant);
  corpus.sh section 6. All pass. The C header check is new ground: the first C++ host that
  compares the header's layout table with the core's export in this slice, and it was seen
  failing on both mismatched pairs.
- Counts: only P1.2 (all three content sets) decode reverse moves, 8 -> 5, back to the
  pre-decision-11 value. So decision 11's larger decode groups cost P1.2 three reverse
  crossings even in drop mode, and the no-unknown variant is where they come back.
- Harness: codec suite MODES per build (full: drop, retain; no-unknown: no-unknown), a
  separate binary; RPC client cells per build (full: A B C-retain C-drop D-retain D-drop;
  no-unknown: A B C-nounk D-nounk, A and B as in-process controls). run_campaign.sh builds
  both, alternates binary order by launch, runs the plant control per client, checks both
  crossing-count files. Smoke run on f0c82bb: logs/rust/campaign-wp5s10/ (instrumentation).
- For the other backends (cpp, java, csharp, python) to render the variant: relower the
  plan with unknown="drop" and render from it (groups without `unknown`, no ak_ufix, no
  opts, no reset, no u-family, `ak_dec_ctx_new_<Root>(void)`); C hosts take the second
  header from c_abi.emit (a variant include dir); C# renders its declarations and layout
  probe from the drop plan; the core for that build is ak-core `--no-default-features`
  plus the features they need, in its own target dir; their bindings' retain paths are
  omitted in the variant; each asserts its layout against the variant core's export.

## 2026-09-26 -- FIX-PLAN WP6 step 1: STATE rewritten, the gate from a clean checkout

- The owner confirmed ABI-v1 rule 4 as implemented (one options entry per oneof, filled in
  the active member's decode group); no code change.
- Deleted every build directory of the main checkout (16 `target-*`, `target`,
  `corpus/target`; disk 67% -> 49%). A fresh `git worktree` at origin HEAD d2cd0b02f
  (0 changed paths, no build directory): `run_campaign.sh --suite gate` GATE PASSED, both
  builds, header with a clean commit; ThreadSanitizer 0 warnings in the suite, 123 on the
  planted race; then the same gate under `RUSTUP_TOOLCHAIN=1.88.0`: GATE PASSED. The 1.88
  toolchain was installed with rustup (it was not in the container; STATE had said "not
  verified"), so the MSRV floor is now checked, not declared. Worktree and its builds
  deleted afterwards (disk 41%). Logs: logs/rust/wp6-clean-gate/.
- STATE.md rewritten to say what is true now. Removed: the status line's history (seven
  earlier work units, kept here); the per-unit "What was checked" sections from WP5 steps
  1, 6 and 7, whose figures (corpus 672/688, the "6 backend modules" guard, the step-7
  counts diff) were superseded; the D-table rows closed elsewhere (D2 by the floor run,
  D34 by decision 11, D35 by the owner's refusal of recursion, D38-D40 by their slices per
  FIX-PLAN R-G17, D41 by every slice regenerating); the rule questions already decided
  (map key: FIX-PLAN WP5 says the canonical form; 10th varint byte: recorded as discard;
  D34/D35). Self-contradictions fixed (R-F1): "retention inside inlined children cannot
  cross the C ABI" (closed since step 7); "other slices' trees are STALE" (all --check
  clean); "no corpus vector has a singular -0.0" (S-double-minus-zero exists); "section 10's
  load-time check is untested" (c_variant.sh tests it); "the pull family does not capture"
  (it does since step 7); "the guard covers 6 modules" vs 26. The R-C14 retired ratio table
  was already gone; no timing figure remains in STATE. The checklist now covers 22a and
  req 20 is marked not met (perf not installed); the reset calls' absence from the counts is
  stated where the counts are described.

## 2026-09-26 -- optimisation experiment, unit 0: the baseline (instrumentation)

- Owner's request (via the aggregating session): explore optimisation opportunities in this
  slice; this unit only takes the baseline later units compare against. The correctness
  gate is NOT run in the experiment (it runs once at the end); the codec process's own
  byte-identity pre-check stays on. No codec, core or generator code changed.
- Added `gen/opt_bench.sh OUT_DIR` (and `gen/opt_summary.py`): run_campaign.sh's build()
  verbatim (target/ full, target-nounk/ no-unknown, bench executables from
  `cargo bench --no-run`, the variant check by `ak_uencode_*` exports), the crossing
  counts of both builds compared with the committed files (recorded, not fatal), then one
  launch (AK_LAUNCH=1) pinned CLIENT=1 SERVER=2,3: codec payload inputs (AK_ONLY=P, 20
  inputs) samples 20, warm-up 100 iterations + 50 ms, measure 250 ms, full then no-unknown;
  codec U-* rows (AK_ONLY=U-, 92 rows) at REDUCED settings, samples 10, warm-up 20
  iterations + 5 ms, measure 20 ms, full then no-unknown; calib 5 rounds x 20M; rpc both
  transports x both clients, plant control each, 3 rounds x 32 calls, warm-up 16. Settings
  are fixed in the script and printed in every header (the runner's req-27 header plus a
  `settings` line, marked container instrumentation, "not gated by gen/gate.sh").
- Sizing probes before the run: per criterion case ~40 ms of fixed overhead (analysis) at
  10 samples; P2.2's 100 fixed warm-up iterations add 0.1-0.25 s per case. Both kept.
- Baseline run on de8b286 (clean tree): logs/rust/opt/baseline/. Crossings: 671 and 336
  rows identical to gen/crossings.txt and gen/crossings-nounk.txt. Pre-check 0 failures in
  all four codec processes (226 / 152 / 736 / 368 checks). Benchmark wall 558 s (codec P
  324 s, codec U 189 s, calib 1 s, rpc 44 s); crossings builds 118 s before it. Summaries:
  summary-codec.tsv (3,698 cases), ratios-codec.tsv (1,005 rows), summary-rpc.tsv,
  summary-calib.tsv.
- Observed, not acted on: (1) armonik's decode on P5.2-P5.4 is a constant ~87-92 ns
  regardless of size: packages/rust's leaves decode `bytes` fields as `bytes::Bytes`
  borrowing the input buffer (armonik/src/codec/leaves.rs), and the harness hands it a
  `Bytes`; every other decode arm copies. decode-read does not read payload bytes either.
  So armonik/inc on P5.2-P5.4 decode is a copy-vs-no-copy comparison. (2) The same arm on
  the same input differs up to ~20% between the full and no-unknown processes (e.g. P2.3
  incumbent decode 1.18 ms vs 0.97 ms): cross-process absolutes do not compare, as R4
  says. (3) One launch only, so the arm order (incumbent-prod first, core-ffi-pull last)
  is not rotated; drift inside the process lands on the ratios uncancelled. (4) Per-case
  spread (max-min)/median: median 0.06-0.08, p90 0.15-0.27, a few outliers above 1
  (e.g. nounk P2.2/latin1 armonik decode-read max 3x its median).

## 2026-09-26 -- optimisation experiment, step 0: harness fixes, baseline2 (instrumentation)

The owner approved the optimisation candidates (core and shared generator included); this
unit implements them one per step, each measured with gen/opt_bench.sh against the previous
kept step. Step 0 fixes the harness first.

- H2, first attempt (harness v2, 51662ef): criterion kept, the cases in one seeded shuffle
  (AK_ORDER=shuffle; criterion cannot interleave samples of different benchmarks, only
  their order), criterion resamples 1000 (analysis only; it was ~40 ms of every case).
  Run twice on one tree (logs/rust/opt/baseline2-v2, baseline2-v2-aa): single cases moved
  up to +-25% between the two processes, ratio rows by a median 6-12% and a p90 of 25-28%.
  The per-case drift is autocorrelated along the run order (lag 1: 0.53-0.77, lag 5:
  0.2-0.4, lag 20: ~0), so the container's speed drifts over seconds, and a ratio whose two
  arms criterion times seconds apart carries that drift. REFUTED as sufficient.
- H2, harness v3 (f08a3d9): AK_ORDER=interleave, the codec suite's own interleaved sampler
  (not criterion): clusters (input, direction) in a seeded order, each case warmed (fixed
  iterations, then a timed warm-up that sets iterations per sample), then round r times one
  sample of every case in the cluster, rotated by r. A/A pair (baseline2, baseline2-aa):
  ratio rows move by a median 1.2-4% and a p90 4-13% (5x tighter); per-case absolutes still
  drift (sd 0.10-0.14 of log), which the ratios no longer carry. Criterion stays the
  campaign's engine (AK_ORDER=blocks is still the default; run_campaign.sh unchanged).
- H3: rpc_client --order interleave: per (dir, in-flight) every cell built and warmed, then
  12 rounds, cell order rotated per round (6 and 4 cells both divide 12); 24 calls/round.
  A/A: 96 of 96 cell/A ratio rows inside the rounds' min/max bands. The first baseline's
  C-drop vs C-retain +30% (direction a) is gone (1.075 vs 1.047 of cell A): it was order.
- H4: U-* rows 20 samples, 45 ms measured (was 10 x 20 ms under criterion).
- H5: decode-nodrop rows on P1.2, P2.2, P4.1, P6.1 (incumbent, armonik, core-native,
  core-ffi). First form (outputs kept alive, dropped at the end = criterion's
  iter_with_large_drop) measured decode SLOWER than with the drop inside (P1.2 incumbent
  484 vs 402 us): cold allocation instead of reuse. REFUTED; the rows now time each
  operation alone and drop after its clock stops. The headline decode rows are unchanged.
- gen/opt_compare.py BEFORE AFTER [--aa A1 A2]: per ratio row before/after/change, flagged
  noise when the quartile bands overlap or (with --aa) when |change| is inside the A/A
  pair's p90 for its group; per group a geometric mean with an A/A 2-sigma band; control
  drift; the drift autocorrelation; RPC cell/A ratios.
- Wall: 560-575 s per opt_bench run (codec 435 s, rpc 125 s). Pre-check 0 failures and
  crossings identical (671 / 336 rows) in all four runs.
- The original baseline (harness v1: criterion, blocks by arm) compared with baseline2 is in
  baseline2/compare-vs-baseline-DIFFERENT-HARNESS.txt: a different harness, not a change.

## 2026-09-26 -- optimisation step 1 (D1): no host-side UTF-8 re-check (e25da96, kept)

- rust_binding.py: `s_of` renders from the plan's utf8 option, as rust_native.py does.
  utf8="reject": `from_utf8_unchecked(b).to_owned()` with a debug_assert. Checked before
  relying on it: the core's decode groups run `check_utf8` on every `string` set_blob,
  append_blob and oneof member (rust_abi.py utf8_check), map entries included (an entry is
  an element message with its own plan); a failure sets the reader's error, the loop stops
  (`at_end` is true once err != 0), the post-loop flush and every `apply` run only when
  d.err == 0, the in-loop flushes only deliver elements decoded before the failing one, and
  the pull replay runs only when `ak_parse_*` returned >= 0. So no invalid span reaches
  s_of. utf8="lossy": from_utf8_lossy (the core does not validate then).
- Consequence: the harness features dec-reject / dec-reject-simd no longer change the
  binding's decode (they still pick ak-rt's decode_str policy, which only the stage
  harness decpolicy.sh reads).
- generate --check and one_core.sh pass; only poc/rust's binding.rs / binding_nounk.rs
  (main and corpus workspaces) changed.
- Measured (logs/rust/opt/s1-d1, vs baseline2, A/A-calibrated): core-ffi/inc decode
  geometric mean 0.745 (P, drop), 0.740 (P, retain), 0.778 (P, no-unknown), 0.70-0.76 (U);
  per payload drop e.g. P1.1 1.39 -> 0.85, P1.2 1.14 -> 0.78, P2.2 1.36 -> 1.01, P5.1
  1.61 -> 0.90, P6.1 0.70 -> 0.58, P5.2-P5.4 unchanged. core-ffi-pull the same. The
  control core-native/inc decode: noise.
- Encode moved although D1 touches no encode code: core-ffi/inc encode 1.09 (full) and
  0.94 (no-unknown), core-native/inc encode 1.09 / 0.92 likewise, while core-ffi/core-native
  encode is noise (1.00). The incumbent's encode median moved (drift 0.96 P, 0.91 U): a
  relinked binary moves code layout. Hazard recorded in STATE; core-ffi/core-native is the
  check for a change that touches core-ffi only.

## 2026-09-26 -- optimisation step 2 (E1): one-pass passthrough blob write (59b16ec, kept)

- ak-core enc_blob: when the ak_str's transcoder IS tc_utf8_trusted (what
  ak_tc_utf8_trusted and ak_tc_bytes both return), write key, exact varint(len), bytes;
  no placeholder, no indirect call, no prefix resolution. Same bytes as the general path
  (its `end` always rewrites the prefix to the minimal width); ak_blob_run calls enc_blob
  and gets it too; lengths above INT32_MAX keep the general path (fail as before); the
  transcode counter is still bumped. Not covered: prefix_moves / grows counters of the
  stage harnesses no longer count these fields (not in the crossing counts).
- Measured (s2-e1 vs s1-d1): core-ffi/inc encode 0.859 (P drop), 0.865 (P retain), 0.863
  (P no-unknown), 0.83-0.84 (U); core-ffi/core-native encode the same (0.83-0.86), so it is
  core-ffi's own change; core-native/inc noise; decode noise. Per payload (drop): P1.2 0.36
  -> 0.29, P2.2 0.44 -> 0.37, P2.4 0.59 -> 0.47, P4.1 0.45 -> 0.35, P5.1 0.57 -> 0.47, P1.3
  and P5.3-P5.4 unchanged. Crossings identical, pre-check 0 failures.

## 2026-09-26 -- optimisation step 3 (E2): sparse fill with the corrected clear (fdecae0, kept)

- rust_binding.py: the zeroed (sparse) top-level loops clear min(remaining, CHUNK) element
  groups per chunk instead of the whole arena (decision 9's corrected wording). The codec
  suite's core-ffi encode arm (rust_campaign.py -> roots.rs) and the RPC cells C/D encode
  through encode_into_<root>_zeroed / _unk_zeroed; `core-ffi encode fill` is a header line
  of every codec and rpc log (campaign::FFI_ENCODE_FILL). Nested groups and the root group
  keep the total fill (only top-level element groups are sparse in this binding).
- Measured (s3-e2 vs s2-e1): core-ffi/core-native encode (the check that excludes the
  incumbent): P1.3, the absent path, 2.40 -> 2.08 (drop), 2.43 -> 1.67 (retain), 2.66 ->
  2.10 (no-unknown); P3.1 retain 1.61 -> 1.38; group means 1.003 (P drop, noise), 0.961 (P
  retain), 0.973 (P no-unknown), U noise. P5.3 moved 0.92 -> 1.14 (drop) and 0.96 -> 1.15
  (retain), a 1 MB bytes field with no loop that E2 does not touch: single-row noise above
  the A/A p90. core-ffi/inc AND core-native/inc encode both moved to 0.88-0.91 in the full
  build (core-native untouched): the incumbent's encode got slower in this binary (control
  drift 1.07), the layout hazard again.

## 2026-09-26 -- optimisation step 4 (E4 + N1) (ab133a3, kept) and a layout experiment

- rust_binding.py (E4): a packed bool field passes the host's own Vec<bool> to ak_run_u8
  (a Rust bool is one byte, 0 or 1); a packed enum is converted on the stack up to 256
  elements (heap past that); still one run call per field (two would be two packed runs,
  legal wire and different bytes). rust_native.py (N1): a packed run reserves its element
  count once (exact for fixed width; the byte count bounds a varint run).
- Measured (s4-e4n1 vs s3-e2): core-native/inc P6.1 decode 0.622 -> 0.440 (drop), 0.607 ->
  0.457 (retain), 0.653 -> 0.402 (no-unknown): N1. core-ffi/core-native P6.1 encode 1.112 ->
  1.023 (drop), 1.107 -> 0.998 (retain): E4 (nounk 1.145 -> 1.114, inside noise). Groups:
  core-ffi/core-native encode 0.963 (P drop), 0.986 (P retain), 0.978 (P no-unknown).
- Not explained: core-ffi/inc P6.1 decode 0.577 -> 0.685 and core-ffi-pull/inc 0.506 ->
  0.587, although neither decode path changed (absolute core-ffi 143k -> 150k while the
  incumbent went 248k -> 218k in these two processes). Again the incumbent's own median
  moved (encode control drift 0.93 P, 0.90 U; the /inc encode groups rose 8-16% together
  with core-native/inc, which E4/N1 do not touch on encode).
- Layout experiment (logs/rust/opt/layout-exp, analysis.txt): the step-4 tree built as A
  and as B with two pad functions (prost's decode functions shifted by 0x20 in the exe, the
  ak_* entries by 0x160 in the .so), codec-P full build, run A, B, A, B. Group means moved
  at most 3.4% between A and B (A/A at most 1.8%, B/B 1.2%); row |change| p90 0.05-0.14 vs
  0.03-0.14. So a small layout shift is worth a few percent on group means, not the 7-10%
  incumbent moves seen between steps; that cause is not identified. Consequence for reading
  the steps: a change is attributed only when core-ffi/core-native (or core-native/inc for
  a core-native change) moves beyond this band, and the /inc ratios are reported beside it.

## 2026-09-26 -- optimisation step 5 (U1 + U2) (56ca80c, kept as cleanliness; geometric growth held back)

- rust_binding.py, full variant: DecCtxs owns an UnkState (each root's retain options,
  every position grow-backed, at a stable address, and an armed flag). The retaining
  entries make ONE reset per decode with those options (rule 7) and leave the context
  armed; the drop-mode entries (decode_with, parse_walk_with, parse_drain_with,
  parse_walk_opaque_with) reset to NULL first only if the binding left it armed; _opts
  (the decision 11 controls) still arms, decodes and disarms. Harness code that arms a
  context itself (stickyerr, the corpus unkctl) calls the raw entry points or _opts and is
  unaffected. UNK_LIVE is a HashMap keyed by the buffer address (a multiplicative pointer
  hash): O(1) take / regrow instead of a linear search (quadratic over a message with many
  positions); unk_reclaim drains in place.
- U2's geometric growth in unk_grow (want.max(2 cap).max(64)) was built and counted: it
  changes 36 lines of gen/crossings.txt (retain decode and decode-pull reverse counts, e.g.
  U-deep-all decode 16 -> 10, decode-pull 8 -> 2). Per the rule "a legitimate count change
  stops the step", it is NOT applied and the committed counts are untouched; the diff is
  logs/rust/opt/s5-u1u2-heldback/crossings-geometric-growth.diff, for the aggregating
  session.
- Measured (s5-u1u2 vs s4-e4n1): every decode group noise (P and U, all modes); the
  in-process core-ffi retain/drop decode ratio on the U rows 1.281 -> 1.263 (P rows 1.011
  -> 1.009). The benchmark has at most a few unknown buffers per decode, so the linear
  search was never long. Kept: rule 7's one reset per decode, and no quadratic tracking.

## 2026-09-26 -- optimisation step 6 (E3 + D2 + D3b + D4) (b259af0, kept)

- E3 (rust_binding.py, swept over every chunked loop callback: top-level, zeroed, unk,
  unk_zeroed, inner): a 2 KB stack chunk, and a field longer than that goes to an
  out-of-line copy of the callback with the 32 KB arena. D2 (rust_abi.py): one 8-aligned
  32 KB arena per decode function for all its loop slots, each slot keeping its own
  element budget (one slot is open at a time: every other tag, a non-leaf element, an
  unknown field and the end flush it), so every run and crossing is the one per-slot
  arenas gave; a compile-time assert per slot. D3b: leaf element groups decoded in place
  (dec_<T>_fix_into). D4: ak-rt Bdr::open_run / close_run; the pull family decodes a run
  straight into the record buffer (every other deposit flushes the open run first, so the
  pointer stays valid).
- Checked: generate --check, one_core.sh; pre-check 962 / 520, 0 failures; both crossing
  files identical; gen/corpus.sh passes on both builds with every control failing as
  required (logs/rust/opt/s6-sanity/corpus.log) -- run because D4 rewrites the pull
  family's deposit.
- Measured (s6 vs s5): core-ffi/core-native decode 0.950 (P drop), 0.956 (P retain), 0.946
  (U drop, lower), 0.986 (P no-unknown); per payload (drop) P1.3 1.41 -> 1.05 (the absent
  path: five 32 KB arenas per element were the cost), P6.1 1.55 -> 1.34, P7.1 1.33 -> 1.16,
  P2.1 1.44 -> 1.35, P3.1 1.07 -> 0.98. core-ffi-pull/inc decode 0.943 (P), 0.902 (U);
  P1.3 2.11 -> 1.54, P7.1 1.06 -> 0.83. Encode (E3): core-ffi/core-native groups noise
  (1.019, 0.995, U 0.994); P5.1 1.77 -> 2.01 is one 36-byte row; E3 shows no gain on this
  payload set (no encode field here is long enough for the 32 KB probe to dominate a loop
  callback); kept as part of the step, whose decode half is the gain.

## 2026-09-26 -- optimisation step 7 (R1 + R2) (85d9034, kept: R1 cleanliness, R2 additive)

- R1: the rpc crate's RawDecoder (the core client's response path) and poc/rust
  rpc_server's decoder return `src.copy_to_bytes(n)` (owned; zero copy for a contiguous
  frame) instead of copying it into a second BytesMut.
- R2: `ak_call_unary_enc(c, path, path_len, enc, out)`, additive, declared once in
  plan.RPC (so every slice's header gained one declaration: cpp include/, nounk/, corpus
  variants; java native/generated*; python gen/out*; csharp RpcAbi.cs). The encode
  context's buffer becomes the request body, owned by the core (Bytes::from_owner; nothing
  borrows host memory, so h2 may poll the body after the response); when the transport
  drops the body its buffer returns through a lock-guarded slot on the context and the next
  such call reuses it (two buffers alternate). A context in error is refused with its
  error; its encoded bytes are consumed. tonic's Encoder still copies the body into its
  EncodeBuf (the Codec API), so what goes is the copy at the ABI, not every copy. Cell C
  direction b uses it (rpc header line `cell C request`); cell D still copies into a tonic
  Bytes. c_variant.sh: both headers compile C99 / C++11, facts agree, mismatches caught.
- Measured (s7 vs s6): every RPC cell/A row flagged noise (rounds' min/max bands overlap).
  Cell C direction b in the full client is 0.02-0.08 of A lower at every in-flight on both
  transports, but C-nounk (the same code in the other client) is unchanged and cell B
  (untouched) moved by as much, so no gain is attributable at 12 rounds x 24 calls; the
  expected size (one 540 KB memcpy against ~1.3 ms of client CPU per call) is 3-5%. Codec
  groups noise (no codec change).

## 2026-09-26 -- optimisation step 8 (F1) (f2a88d3, kept)

- The facade crates (main, corpus) get an `unknown-fields` feature (default on); the
  `unknown_fields` member and core_native_retain exist only with it (owner decision
  R-H22). harness, campaign and corpus-harness depend on the facade with
  default-features = false and forward their own unknown-fields feature, so the no-unknown
  build (campaign --no-default-features) has the member compiled out. Generators changed:
  rust_facade.py, rust_build.py, rust_campaign.py, rust_corpus.py (poc/rust), and the
  shared rust_binding.py (binding_nounk no longer writes the member). The corpus
  no-unknown build now has 2 arms (ffi-nounk, native-drop): native-retain has nothing to
  retain into. The codec and rpc headers of the no-unknown binary state that the armonik
  control arm changes with it (it uses the same facade types; accepted by the owner).
- Checked: generate --check, one_core.sh, conformance and shapes VERDICT pass on both
  builds, pre-check 962 / 520 0 failures, both crossing files identical, gen/corpus.sh
  passes (4 and 2 arms, every control failing as required; logs/rust/opt/s8-sanity).
- Measured (s8 vs s7), no-unknown build: decode core-native/inc 0.940 (P), 0.887 (U);
  core-ffi/inc 0.957 (P), 0.903 (U); core-ffi-pull/inc 0.958 / 0.905; armonik/inc 0.961 /
  0.923 (the control arm moves with the facade, as stated); core-ffi/core-native 1.017 /
  1.015 (both got faster); encode noise. Full build: decode groups noise; the /inc encode
  groups 1.06-1.07 with core-ffi/core-native encode noise (the incumbent-drift hazard).
  Per payload (no-unknown, decode): P1.3 core-native/inc 1.705 -> 0.806, core-ffi/inc
  1.674 -> 1.140 (the absent path: every empty message was 24 bytes larger); P1.1
  core-ffi/inc 0.818 -> 0.668.

## 2026-09-26 -- optimisation step 9 (E5) (4bbaefe, kept)

- ak-rt Enc: varint, varint_field and blob_field reserve once (10, 20, 20 + len) and store
  through a raw pointer + set_len; new varint_run (one reservation of 10 bytes an element)
  and f64_run (one copy of the host's array on little-endian). ak_run_i32/i64/u8/f64
  (rust_abi.py) and core-native's packed encode (rust_native.py) use them. Same bytes:
  pre-check, conformance, shapes, both crossing files, core unit tests.
- Measured (s9 vs s8): core-native/inc encode 0.751 (P drop), 0.765 (P retain), 0.701 (U),
  0.725 / 0.647 (no-unknown P / U); core-ffi/inc encode 0.835 / 0.838 (P), 0.85 (U), 0.823 /
  0.790 (no-unknown). core-ffi/core-native encode rose to 1.10-1.23 because core-native
  (which calls Enc for every field) gained more than core-ffi (whose fields mostly go
  through enc_blob's E1 path, already one pass). Per payload (drop): core-ffi/inc P1.2
  0.28 -> 0.22, P2.2 0.35 -> 0.28, P6.1 0.22 -> 0.15; core-native/inc P1.2 0.19 -> 0.13,
  P6.1 0.20 -> 0.13. Decode noise. The incumbent's encode median moved 1.05 (drift), which
  the /inc ratios carry in the other direction.
- Process defect (mine): the first step-9 run was started, then I edited
  gen/opt_bench.sh while it ran (bash reads a script as it executes it, so the running
  shell would have resumed at a stale offset). Stopped after its first codec process, its
  directory deleted, the Z1 edits set aside (they would have entered the build), and the
  run restarted from the committed tree. Rule kept since: no edit to a running script.

## 2026-09-26 -- optimisation step 10 (Z1) (ef65732, kept as a labelled extra arm)

- rust_binding.py: decode_with_<root>_zc(ctxs, &Bytes) (+ _unk_zc): every `bytes` field a
  slice of the input Bytes (shared allocation, refcount) instead of a copy, through a
  thread-local pointer to the input that b_of consults; core-ffi keeps ABI v1 decision
  13's copy semantics and pays one thread-local load per non-empty bytes field. The arm
  `core-ffi-zc` (AK_ZC input prefixes; opt_bench sets P5.), decode and decode-read, every
  mode; the pre-check requires core-ffi-zc == core-ffi on every input (1186 / 632 checks).
- What it measures: the cost of the core-ffi decode MINUS the copy of each bytes field
  out of the input, for a host whose input is already a refcounted buffer (a tonic
  DecodeBuf gives a Bytes). It is what armonik (packages/rust) already does on decode:
  armonik/inc on P5.2-P5.4 has been ~0.05 / 0.002 / 0.0003 since the first baseline.
- Measured (s10): core-ffi-zc/core-ffi decode 0.84 (P5.1, 36 B), 0.042 (P5.2, 64 KB),
  0.002 (P5.3, 1 MB), < 0.001 (P5.4, 4 MB), the same in retain and no-unknown. Headline
  groups (s10 vs s9): noise except codec-U core-ffi/core-native decode drop 1.065 (32 of 92
  rows higher, the small ListProbeResponse / ListResultsResponse rows), which is
  core-ffi/inc 1.034 against core-native/inc 0.964 in the opposite direction (core-native
  untouched), each inside the layout experiment's 3.4% band; not attributed. The
  thread-local load is ~1 ns against those rows' ~0.5-1 us.

## 2026-09-26 -- optimisation step 11 (C2) (49b928a, kept)

- ak-rt: a DEFAULT feature `simd-utf8`; strings::check_utf8 (every `string` field the
  core decodes under utf8="reject", and core-native's decode_str_reject) uses
  simdutf8::basic with it. On ak-rt rather than ak-core because ak-core depends on ak-rt
  with default features, so every slice's core build gets it whatever it passes to
  ak-core. Checked in the other slices' scripts: cpp CMakeLists (no-unknown targets
  --no-default-features --features ...; the others explicit --features lists), csharp
  gen/build_core.sh (three --no-default-features targets and the abi probe), python
  build.sh (plain-nounk, count-nounk, ... --no-default-features), java gen/build.sh
  (--features lists): none of them turns off ak-rt's defaults, and `cargo tree -p ak-core
  --no-default-features -e features` shows simd-utf8 enabled. Nothing to change in their
  scripts. ak_rt::strings::CHECK_UTF8 names the validator in the campaign header.
- Checked: pre-check 1186 / 632 0 failures, conformance and shapes pass, both crossing
  files identical, gen/corpus.sh passes (transcode class 53/53 on every arm), core unit
  tests pass.
- Measured (s11 vs s10): decode on the non-ASCII content sets, in both core arms:
  core-ffi/inc P1.2/latin1 0.844 -> 0.544, P1.2/wide 0.881 -> 0.447, P2.2/latin1 0.954 ->
  0.753, P2.2/wide 0.984 -> 0.695 (retain and no-unknown the same; core-native/inc the same
  sizes, so core-ffi/core-native does not move). ASCII rows: no resolved change (core
  str::from_utf8's ASCII fast path was already fast). Other single rows moved 10-25% both
  ways (P1.3, P3.1, P5.2), within the run-to-run bounce those rows show over the whole
  series (e.g. P3.1 core-ffi/inc 0.74-0.94 across steps 4-11 with no change to it).

## 2026-09-26 -- optimisation step 12 (C1): codegen-units = 1 (e804e0d, REVERTED by 46d14f8)

- [profile.release] codegen-units = 1 (lto off) in poc/rust (so prost, tonic and the
  incumbent arms too), its corpus workspace, and poc/codec (the root other slices build the
  core from); build lines of run_campaign.sh / opt_bench.sh / the campaign header updated.
  Pre-check, conformance, shapes and both crossing files held.
- Measured (s12 vs s11): every core-ffi/core-native group noise (0.94-1.02); /inc groups
  both ways (core-native/inc U decode 1.065 higher, P encode retain 0.978 lower); the
  arms' absolute medians moved inside the A/A pair's own absolute drift (0.96-1.04), except
  the incumbent's (and armonik's) encode in the no-unknown process, ~0.91 (prost encode
  faster), which is why the no-unknown core-ffi/inc and core-native/inc encode groups rose
  to 1.09-1.11. The build of the two variants took ~2 min against ~1 min before.
- Kept or not: no resolved gain for the core arms, and it is not a correctness or
  cleanliness change, so by the step rules it is reverted (git revert, history kept). The
  owner may want it anyway for fairness (it is a build flag every arm shares); the data is
  in logs/rust/opt/s12-c1.

## 2026-09-26 -- optimisation experiment: final gate and final run

- Gate on the final code, 3c737d1, main checkout with no changed path: `run_campaign.sh
  --suite gate --out logs/rust/opt/final-gate` GATE PASSED (stable 1.94.1), then
  `RUSTUP_TOOLCHAIN=1.88.0 bash gen/gate.sh` GATE PASSED (1.88.0 reinstalled with rustup;
  the container had lost it). Differences from the WP6 gate, all expected: pre-check 1186 /
  632 checks (step 10's zc equality), the corpus no-unknown build 2 arms (step 8), the
  headers carry `ak_call_unary_enc` (step 7). Crossing counts identical on both files;
  every control failed as required. ThreadSanitizer not re-run.
- Final opt_bench (logs/rust/opt/final, commit d7ee631 = 3c737d1 plus the gate logs) vs
  baseline2, A/A-calibrated group geometric means (full drop / no-unknown):
  core-ffi/inc decode 0.662 / 0.699 (P), 0.684 / 0.694 (U); encode 0.804 / 0.678 (P),
  0.859 / 0.675 (U); core-ffi-pull/inc decode 0.677 / 0.704 (P); core-ffi/core-native decode
  0.735 / 0.800 (P), 0.715 / 0.749 (U); core-native/inc decode 0.901 / 0.874 (P), encode
  0.854 / 0.701 (P). Per payload (drop, core-ffi/inc decode): P1.1 1.39 -> 0.86, P1.2
  1.14 -> 0.78, P1.2/wide 1.40 -> 0.45, P2.2 1.36 -> 1.00, P1.3 2.10 -> 1.38, P5.1 1.61 ->
  0.96, P6.1 0.70 -> 0.58, P5.2-P5.4 ~1.0 (a copy either way; core-ffi-zc 0.04 / 0.002 /
  <0.001). RPC cells C and D over A: direction a 1.05-1.31 -> 0.87-1.01, direction b
  0.55-0.65 -> 0.42-0.52. The first baseline (harness v1) against final is in
  final/compare-vs-baseline-DIFFERENT-HARNESS.txt, a different harness.
- Design-doc wording the aggregating session may want to change (not edited here):
  ABI-v1 decision 9's "clear the elements you will fill" is now what the Rust binding does
  (min(n, chunk) per chunk); decision 11 rule 7 "a reset per decode" is what the binding
  now makes (one reset, no disarming reset after); decision 13's copy semantics stay the
  default and a zero-copy entry exists as a labelled extra in the Rust binding;
  section 9 gains `ak_call_unary_enc` (additive); section 4 / decision 3 could say the
  core's decode-side check is simdutf8 by default. CAMPAIGN.md req 22a describes the codec
  engine as criterion; the optimisation benchmark uses the suite's own interleaved sampler
  (the campaign default is unchanged).

## 2026-09-26 -- owner decisions after the experiment: interleaved sampler removed, U2 applied

- Interleaving (owner: "If criterion cannot interleave, then forget about interleaving.
  Benchmarking is complex enough not to reinvent it."). Removed: the codec suite's
  AK_ORDER=interleave branch and everything that existed only for it (the `interleaved`
  sampler, `sample`, `splitmix`, the `timed` closure on Case, the CRITERION_HOME
  exemption, its header lines and module comment). Kept, because they are plain criterion:
  AK_ORDER=shuffle (seeded case order), AK_NRESAMPLES, the U-* sample count, the labelled
  extra decode-nodrop rows (now criterion's `iter_custom` over `time_nodrop`: each
  operation timed alone, its output dropped after its clock stops) and the labelled extra
  core-ffi-zc arm. gen/opt_bench.sh is harness v4: criterion, AK_ORDER=shuffle AK_SEED=1
  AK_NRESAMPLES=1000, P 20 samples / 100 it + 50 ms / 200 ms, U 20 samples / 20 it + 10 ms
  / 60 ms (the v2 settings), CRITERION_HOME back. The RPC grid (the runner's own harness,
  not criterion) is left as it was: rpc_client --order interleave, cells rotated per round;
  nothing new added there. Consequence, stated in STATE: every opt run from baseline2 to
  final was taken with harness v3, which no longer exists; a new opt_bench run compares
  with baseline2-v2 / baseline2-v2-aa (the criterion A/A pair), not with them. Not run
  (instruction): opt_bench.
- U2 geometric growth (owner: accepted, "crucial to avoid quadratic growth complexity").
  rust_binding.py: `unk_grow` allocates `unk_grow_cap(want, old_cap)` =
  max(want, 2 * old_cap, 64) computed in usize (saturating, so no i32 overflow), clamped to
  INT32_MAX (ABI v1 decision 11 rule 5) and never below want; a need above INT32_MAX is
  still refused with AK_ERR_LIMIT by the core before it calls grow (unchanged). A unit test
  in the generated binding checks the doubling, the clamp at INT32_MAX from below and from
  an old capacity past it, and c >= want (cargo test -p harness --lib unk_grow_cap: 1
  passed).
- Crossing counts regenerated from the counting build: gen/crossings.txt, 36 of 671 rows
  changed, exactly the 36 of the held-back diff; gen/crossings-nounk.txt unchanged (the
  no-unknown build has no unk_grow). Every changed row is a RETAIN decode or decode-pull
  REVERSE count on a U-* row whose unknown runs grow a buffer more than once: each unk_grow
  call is one reverse crossing, and doubling makes fewer of them. No forward count, no drop
  row, no encode row changed. Rows (reverse, before -> after): U-deep-all decode 16 -> 10,
  pull 8 -> 2; U-deep-u-repeated 10 -> 9, 2 -> 1; U-element-all 16 -> 10, 8 -> 2;
  U-element-u-repeated 10 -> 9, 2 -> 1; U-leaf-all 18 -> 8, 16 -> 6; U-leaf-u-repeated
  6 -> 4, 4 -> 2; U-nested-all, U-nested-before, U-nested-interleaved, U-oneof-all,
  U-oneof-before, U-oneof-interleaved, U-root-all, U-root-before, U-root-interleaved each
  10 -> 4 and 8 -> 2; U-nested-u-repeated, U-oneof-u-repeated, U-root-u-repeated each
  4 -> 3 and 2 -> 1. Diff: logs/rust/opt/u2-growth/crossings.diff.
- Checks run (no gate, by instruction): generate --check and one_core.sh pass; cargo build
  of both variants, both bench executables and the corpus workspace; the pre-check alone
  (AK_PRECHECK_ONLY) 1186 / 632 checks, 0 failures; the regenerated gen/crossings.txt
  reproduces from the counting build; crossings-nounk identical. The final gate at 3c737d1
  predates both changes, so the current HEAD is not gated.

## 2026-09-26 -- WP6 register H: the findings assigned to rust, two core changes

Each finding was confirmed or refuted before a change; the evidence is named per line.

- **R-H21** (core, owner decision): confirmed on the core at 1e489eb40 by two scratch unit
  tests (logs/rust/wp6h/rh21-before.log): INT32_MAX + 1 without grow returned
  AK_ERR_CAPACITY (-7), and a host cap of 2^32-1 let `len` pass INT32_MAX (rc 0). Fixed
  (31fc3eecf): `unk_room` caps every capacity at INT32_MAX; the slot's `cap` is never
  rewritten (the host frees with it). Unit tests in ak-core, run by gate step 2.
- **R-H10** (core): confirmed with a new corpus control before regenerating the codec
  (logs/rust/wp6h/rh10-before.log: a refused parse took A's 480 record bytes to 0). Fixed
  through rust_abi (the root check moved above `bdr.reset()`), all four generated codecs;
  the control passes.
- **R-H20** (owner decision, no code change): a control shows the rule: all-zero options
  at reset, refill after, rc 0 and no bag; its twin armed at reset gets the 3-byte bag.
- **R-H15**: confirmed: packed fixed32 was refused in rust_abi at render; rust_abi,
  rust_binding, c_abi and cpp_layout tested `options.unknown == "drop"`. Fixed in plan and
  the four renderers; gen/check_direct.py plants a packed fixed32 and sees the refusal.
  PACKED_KIND: refuted as an ABI fact -- it sets `open_kind`, which no run symbol reads and
  no host sees; left in rust_abi with that stated. (RUN_FN and java_layout are other
  slices' backends.)
- **R-H13**: confirmed (imports only, over codec/gen; rust and python gen/ unguarded;
  one_core.sh named csharp/gen/ir.py, which no longer exists). Fixed: the slice guard
  (every poc/<slice>/gen/*.py on disk; front-end imports; wire tokens in emitted strings
  outside a listed renderer; renamed copies; three plants), one_core's stale exception
  removed and a renamed-emitter plant added. On its first run it flagged
  rust/gen/rust_facade.py, which is R-H12.
- **R-H12**: confirmed (oneofs written after plain fields; no prost-build check). Chose
  rendering from the plan: rust_facade.py moved to codec/gen, prost_impl.rs walks the
  plan's encode steps with the plan's implicit presence. shapes.json's bytes are unchanged
  (every oneof tag there is above every plain tag; conformance passes on every arm); on a
  planted description with a plain field above Probe's oneof, the order is tag order where
  the old renderer wrote the oneof last (logs/rust/wp6h/rh12-rh15-generator.log). A
  prost-build diff was not done (not needed once the order is the plan's; prost-build is
  not in this workspace).
- **R-H22** (owner decision): confirmed for Rust (types.rs kept `unknown_fields` in the
  no-unknown build). Fixed: types_nounk.rs from the no-unknown plan, facade feature, the
  retain rendering compiled out there; the no-unknown corpus runs ffi-nounk and
  native-nounk. The full build's types are unchanged (header line only).
- **R-H2**: confirmed for Rust (thread::scope spawned k threads per batch inside the
  window). Fixed: a pool per (cell, dir, k), created before the warm-up, reused.
- **R-H8**: confirmed (no switch from a message member to a scalar member). Added: the
  value is the scalar, no bag is delivered, the inactive slot's buffer is reclaimed (1),
  0 live after.
- **R-H23** (owner decision): the codec suite's arm blocks and the cases inside them, and
  the RPC cell order, are now a seeded random permutation per launch (seed = launch,
  written in the header). Criterion has no shuffle of its own but runs benchmarks in
  registration order, which the suite controls, so randomising was possible.

Found on the way: my `git mv` of rust_facade.py sat staged in the shared index and went
into the aggregating session's docs commit a9d96f87c (content unchanged); 31fc3eecf carries
the new content. Nothing is left staged between commands now.
- Gate from a clean worktree at 766f8dcd9, both builds: GATE PASSED
  (logs/rust/wp6h/clean-gate/). Crossing counts identical to both committed files: no
  count changed (the parse-order and capacity changes do not move a crossing; the facade
  changes are host-side), so gen/crossings*.txt are untouched. Floor 1.88 and TSan were not
  re-run; the campaign smoke was not re-run after the pool and order changes.

## 2026-09-26 -- FIX-PLAN WP7: the harness conformed to the 2026-09-26 contract

- req 21: criterion's measurement is now process CPU (CLOCK_PROCESS_CPUTIME_ID); calib
  too. RPC was already getrusage(RUSAGE_SELF).
- req 7: content sets added on P2.4. The U-* question (92 rows at the 7 ABI roots, or the
  213 whose root is a shapes message) went to the coordinator; answer: the 92, no core
  change (CAMPAIGN req 7 corrected, 3210f28e0).
- req 11: four encode variants per arm and mode. The first pool sizing (wire bytes with a
  64-byte floor per graph) put 430k graphs of a tiny U-* row in one pool and the pre-check
  peaked at 4.5 GB RSS in 171 s: a small wire size says nothing about the heap a facade
  graph holds. Refuted and replaced: graphs are cloned until the heap they hold (glibc
  mallinfo2, read only while building) reaches AK_POOL_BYTES; the pre-check then took 85 s,
  about 1 GB RSS (mostly the leaked per-case values).
- req 12-17: campaign::grid holds cells A-F (E, F: core-native over the core's transport and
  over tonic), campaign::server the Unix-socket server (tokio-stream's UnixListenerStream);
  A/D/F are tonic's async call from k tasks, B/C/E the blocking core call from a reused
  thread pool; directions a, a+read, b; one server per (transport, launch) for both builds,
  warmed from each client transport; one channel per cell per launch.
- req 19: the binding tallies ak_enc_reset, ak_dec_reset_<Root>, ak_enc_take and ak_dec_err
  in the counting build (rust_binding.py, no core change); `crossings` adds a resets column
  and per-call rpc:<cell> rows for B-E over an in-process server. Every existing row's
  reverse is unchanged; forward moved by exactly the tallied calls
  (logs/rust/wp7/crossings-change.txt). Files re-committed.
- req 4: worker thread counts in every header; the runner reads ffi/campaign.machine when
  the CPU sets are unset.
- Clean worktree at c8e8694eb: gate (stable, both builds) PASSED, floor 1.88.0 PASSED, TSan
  0 in the suite / 190 on the plant; smoke of codec, rpc and calib shows every new row,
  figures stripped by gen/strip_figures.py (logs/rust/campaign-wp7/). Two interruptions by
  the API spend limit; the run's script had finished before the second, so nothing re-ran.

## 2026-09-26 -- merge of origin/rust/native-core-ffi-poc (baa173a2) into the optimisation branch

The owner: "The branch rust poc has been updated with fixes. Can you merge it, and redo the
measurements and analysis?" 52 commits (WP7 harness conformance, register H fixes, R-H31
counting, the other slices' WP7). 21 conflicts, resolved:

- poc/codec/gen/rust_binding.py (3 hunks): the member comment (theirs, same content);
  decode_with_<root>_opts keeps our U1 body with their `host_reset()` tally on the disarm;
  parse_walk_with_<root>_unk keeps our U1 body (arm_<root> once, no disarm). Their R-H31
  tally (`host_reset` / `host_call`) is added to our arm_<root> and disarm_<root> too, so
  every ak_dec_reset the binding makes is counted.
- rust_facade.py (moved to poc/codec/gen by them; our F1 edit merged into it): THEIRS
  (R-H22 renders a separate types_nounk.rs from the no-unknown plan; our F1 used a cfg on
  the member). One result: theirs. The facade manifests, lib.rs files, rust_build.py (the
  same cfg literals on both sides), rust_corpus.py and the corpus main.rs (theirs names the
  no-unknown native arm native-nounk; ours had native-drop): THEIRS. rust_campaign.py:
  theirs plus our E2 (the core-ffi encode arm on the _zeroed entry points) and Z1
  (f_decode_zc).
- crates/campaign/src/lib.rs and benches/codec_suite.rs: THEIRS (process CPU R-H25,
  requirement-11 encode variants with pools, R-H23 seeded arm/case order) plus ours:
  FFI_ENCODE_FILL, the core-ffi-zc labelled extra arm (AK_ZC, run after the arm blocks,
  pre-check core-ffi-zc == core-ffi), AK_NRESAMPLES, the UTF-8 validator header line. Our
  decode-nodrop rows and the AK_ORDER shuffle are dropped: the merged campaign suite is
  the reference (same clock, arms and order) and R-H23 already randomises the order.
- rpc_client.rs / rpc_server.rs: THEIRS (grid.rs, server.rs, cells A-F, Unix socket, one
  server per launch, R-H23 cell order). Our per-round interleaved cell rotation is gone with
  our rpc_client (theirs replaces it). Re-applied on their code: R1 in server.rs's raw
  decoder (copy_to_bytes, no second buffer) and R2 in grid.rs cell C direction b
  (CoreClient::call_enc -> ak_call_unary_enc); header lines `cell C request`, `core-ffi
  encode fill`.
- crossings.rs (theirs): its text says what the merged binding does (one reset per retain
  decode or pull; geometric grow, the owner's override of R-H31's exact-size grow for this
  slice). No code change: the Rust binding's unk_grow was the only grow (no exact-grow
  switch exists for Rust in poc/codec), so geometric everywhere needed nothing else.
- Generated files (both bindings in both workspaces, dispatch.rs, roots.rs, C#
  RpcAbi.cs): regenerated with poc/codec/gen/generate.py, never hand-merged. Other slices'
  regenerated output differs from their branch only by the additive ak_call_unary_enc
  (cpp/java/python ak_abi.h variants, python binding.c counting macro, C# RpcAbi.cs).
- STATE.md: theirs, plus our optimisation section and a status line saying HEAD is not
  gated. JOURNAL.md: both sides kept (ours first, then theirs), this entry after.

Crossing counts regenerated from the merged counting builds (R-H31 as amended by the
owner: geometric grow everywhere). Against THEIR committed files (logs/rust/opt/
merge-counts/): gen/crossings.txt 232 of 696 rows differ -- every retain decode and
decode-pull row (228) has 1 fewer forward call and 1 fewer reset (U1: one ak_dec_reset per
retain decode; they count two, arm and disarm); 36 of those (18 decode, 18 pull, the U-*
rows whose unknown runs regrow a buffer) also have fewer reverse calls (U2 geometric grow:
16 -> 10, 18 -> 8, 10 -> 4, 8 -> 2, ...); rpc:C a retain 5 -> 4 forward, resets 2 -> 1 and
rpc:D a retain 3 -> 2, 2 -> 1 (U1); rpc:C b retain 2521 -> 2520 and b drop 2515 -> 2514
forward (R2: the moved request needs no ak_enc_take). gen/crossings-nounk.txt: 1 of 349
rows, rpc:C b nounk 2515 -> 2514 (R2). No other row, no encode row (E2 changes the fill,
not the calls). Z1's core-ffi-zc is not in the counts (a labelled extra arm; it makes the
calls core-ffi decode makes).

Checks (no gate, as the owner asked): builds of both variants, the bench executables and
the corpus workspace (both builds); generate --check and one_core.sh pass; pre-check alone
3706 checks (full, 114 inputs, 4228 cases) and 2240 (no-unknown, 2632 cases), 0 failures;
both regenerated crossing files reproduce from the counting builds; the unk_grow clamp test
passes.

## 2026-09-26 -- measurements after the merge (instrumentation)

- gen/opt_bench.sh v5 follows the merged campaign harness: the same codec suite (criterion,
  process CPU, requirement-11 encode variants with the pool input at the campaign default
  AK_POOL_BYTES = 2 x 13.75 MiB, which is inside this container's 260 MiB L3; R-H23 seeded
  arm-block and case order, launch 1), the core-ffi-zc extra arm on P5.*, AK_NRESAMPLES=1000
  (analysis only); P 10 samples / 20 it + 20 ms / 60 ms, U 10 / 5 it + 3 ms / 10 ms; calib
  5 x 20M; RPC the runner's grid (cells A-F, a / a+read / b, k 1/8/16, one server per
  transport on a Unix socket, plant control per client) at 3 rounds x 24 calls, server
  warm-up 16, server threads 2 (the SERVER set's size here; the runner's default is 4).
  gen/opt_summary.py writes absolute tables: summary-codec.tsv (per case, with the encode
  variant), variants-codec.tsv/.txt (one column per variant), unknown-retain-vs-drop.tsv.
  gen/opt_variants_compare.py prints before -> after per variant.
- (a) logs/rust/opt/merged, the merged HEAD dda65e6e: 450 s (build 0 s, reused; crossings
  identical to the committed files; codec 358 s; calib 1 s; rpc 86 s). Pre-check 0
  failures in the four processes (794 / 524 / 2912 / 1716 checks).
- (b) logs/rust/opt/merged-before, a clean worktree of origin/rust/native-core-ffi-poc
  baa173a2 with the SAME opt_bench.sh and opt_summary.py copied in (no other change;
  the header says "+ UNCOMMITTED CHANGES" for those two files): 983 s (build and counting
  builds 267 s, fresh; codec 623 s, longer because their suite has no AK_NRESAMPLES, so
  criterion's 100000-resample analysis runs after every case -- no sample depends on it;
  rpc 92 s). Their counting build reproduced their committed counts (696 / 349 identical).
  Pre-check 0 failures (750 / 502 / 2728 / 1624 checks; no zc arm there, so its columns
  are empty). The worktree and its 2.1 GB of target directories were deleted after.
- Before -> after, geometric mean of after/before over the 128 payload rows (input x
  direction x encode variant): prost 0.974 (the control; range 0.82-1.26), armonik 0.979,
  native-drop 0.835, native-retain 0.849, ffi-drop 0.739, ffi-retain 0.728, pull-drop
  0.700, pull-retain 0.724, prost@nounk 0.981, native-nounk 0.833, ffi-nounk 0.719,
  pull-nounk 0.684. Headline cells (us, encode reused-buffer/hot / decode): P1.2 ffi-drop
  encode 127 -> 87, decode 564 -> 427; P2.2 ffi-drop encode 433 -> 283, decode 2402 ->
  1879; P2.2/wide decode 4069 -> 2058; P1.3 ffi decode 21.0 -> 17.2; P6.1 native decode 214
  -> 137. RPC (pinned, in-flight 1, client CPU per call): cell C direction b 945 -> 755 us
  (drop), A 1710 -> 1740 (control). Tables: merged/variants-before-after.txt,
  merged/headline-before-after.txt.

## 2026-09-27 -- optimisation unit 2 (owner: "Please start"), steps 0 and 1

Per step: generate --check, one_core.sh, the codec pre-check on both builds and the
counting builds against the committed crossing files (`gen/step_checks.sh OUT`, new: the
four checks in one script, output in `<step>/checks/checks.log`), then opt_bench v5 with
unchanged settings into `logs/rust/opt/<step>/`, compared with the previous kept step as
absolute per-variant tables (`opt_variants_compare.py` -> `<step>/variants-before-after.txt`;
per direction and variant: `<step>/by-direction.txt`). prost is the control. Every figure
is container instrumentation.

- Step 0 (19b339a2, harness): the requirement-11 encode variants now name the transport
  form. `transport-ready-tonic` = what the arm hands tonic (prost/armonik: split+freeze of
  a reused BytesMut, cell A; core-ffi: `Bytes` copy of ak_enc_take's bytes, cell D;
  core-native: `Bytes` copy of its Enc buffer, cell F). `transport-ready-core`, core-native
  and core-ffi only = what the arm hands the core's transport: cell C's encode context
  (ak_call_unary_enc moves the core's buffer inside the call) and cell E's reused buffer
  (ak_call_unary copies inside the call). Nothing happens on the host after the encode, so
  that row's op IS the reused-buffer op, timed as its own row. First tried: a Rust helper in
  ak-core timing the move itself; refused by construction (ak-core is a cdylib, no rlib, so
  the harness can only reach it through the ABI), reverted before commit. Checks: pre-check
  4610 / 2692 checks, 0 failures; crossings identical (696 / 349 rows). Reference run
  `opt/t0-ref` (9669872b logs), 523 s (was 450 s: +2 rows per core arm x mode x payload).
  By-product, an in-process A/A: transport-ready-core / reused-buffer, gmean over 42 rows per
  column 0.979-1.017, per-row 0.75-1.63 (hot and pool rows alike).
- Step 1, T1 native. Three forms, one kept:
  (a) first form (amended into cf844df5, so its hash 5c1a31d1 is not on the branch; the logs of opt/t1-native-first name it): `ak_rt::Enc.buf` a `BytesMut`, `Enc::take` = split + freeze, reset
  reserving the taken length (reclaims the allocation when the taken Bytes is gone);
  ak_call_unary_enc sends Enc::take (R2's spare slot removed). `opt/t1-native-first`:
  reused-buffer encode +12-21% (gmean, every core column; prost 0.995). Cause found by
  asking whether the code runs as written: `BytesMut::clear`, `truncate` and `resize` are
  not `#[inline]`, so without LTO every reset and every `begin` (a length placeholder) was a
  call. (b) cf844df5: reset = set_len(0), begin = a 16-byte constant zeroing.
  `opt/t1-native-b`: reused-buffer 1.01-1.06 (prost 0.94 / 1.08). objdump of libak_core.so
  then still showed 17 out-of-line calls to `BytesMut::extend_from_slice` (an inline hint
  the cdylib ignored): (b') aac7120e swept every append to Enc's buffer through an
  always-inlined `Enc::put` (ak-rt, ak-core enc_blob/enc_raw, the generated core-native
  unknown tail); after it, 0 calls into bytes_mut from the codec (the one left is h2's).
  `opt/t1-native-swept`: reused-buffer native 1.00-1.02, ffi 0.96-1.08, prost 0.93 / 0.98 --
  ambiguous across processes, so an alternated A/B (new: `gen/opt_narrow.sh`, the codec
  suite on chosen inputs with opt_bench's payload settings, ~55 s; `gen/opt_ab.py`; a
  worktree of 9669872b as A): `opt/t1-native-ab`, 3 x A/B on P1.2*, P2.2*, P2.3, P5.3, full
  build: reused-buffer encode B/A 1.05-1.11 on every core column with per-pair ranges mostly
  above 1.0, while prost (unchanged code) read 0.89-0.91 in the B builds. So the BytesMut
  form costs 5-10% more on the reused-buffer encode relative to the old buffer even when
  nothing calls out; not further explained (no profiler here). (c) 38f3e701, KEPT: Enc keeps
  its `Vec<u8>`; `Enc::take` moves the buffer into a `Bytes` (from_owner) and a dropped body
  returns it to a spare slot (Arc<Mutex<Option<Vec>>>, try_lock; R2's mechanism, moved into
  ak-rt so C, F and the step 2 entry share it); `Enc::put` kept (a Vec extend_from_slice).
  Narrowed A/B `opt/t1-native-c-ab`: reused-buffer B/A 0.96-1.01 (prost 0.89),
  transport-ready-tonic native 0.74-0.76. Full run `opt/t1-native` (506 s) against
  t0-ref, gmean per direction and variant: reused-buffer native 0.98-1.00, ffi 1.01-1.02
  (prost 0.93 full / 1.00 nounk); transport-ready-tonic native-drop 0.84, native-retain
  0.82, native-nounk 0.83 -- P5.2-P5.4 0.49-0.53, P2.4* 0.59-0.77, P2.2 0.82 (190.7 -> 156.5
  us hot) -- but P5.1 0.04 -> 0.09 us and P1.1 0.19 -> 0.23 us: the move has a fixed cost
  (one from_owner allocation, an Arc clone, a lock at take and at drop), about 50 ns, more
  than copying a ~20-byte message. Not addressed (a size threshold under which take copies
  is one line; left for the owner). Decode noise (0.99-1.05, prost 1.07 / 0.99).
  RPC direction b, k=1: every cell 0.60-1.17 between the two runs, the control cell A
  1.01-1.17 -- 3 rounds do not resolve an effect here. Checks at every form: pre-check 0
  failures, crossings identical (T1 native moves no crossing), ak-rt unit tests (3 new:
  take moves, buffers alternate after a drop, a held body costs one allocation).

## 2026-09-27 -- step 2 (T1 ffi) committed and measured, then HELD on a finding

- 1c181021: additive RPC ABI entry `int32_t ak_enc_take_owned(ak_enc_ctx *enc, struct
  ak_bytes *out)` (plan.RpcAbi, so every slice's header and binding carries it; only this
  slice calls it): the encode context's output moved to the host as an owned ak_bytes
  (Enc::take, boxed), released with ak_bytes_free on any thread, the released buffer back in
  the context's spare slot; NULL enc/out -> AK_ERR_INVALID_STATE, a context in error -> its
  error with out empty. The rust host wraps it with Bytes::from_owner (campaign
  `ffi_owned_body`) for cell D and core-ffi's transport-ready-tonic row. Pre-check now also
  checks the moved forms' bytes (Enc::take, ak_enc_take_owned, twice each, and the NULL-out
  refusal): 5740 / 3257 checks, 0 failures. Crossings: rpc:D direction b +1 forward in each
  mode (ak_bytes_free; ak_enc_take_owned replaces the binding's ak_enc_take one for one),
  stable over three counting runs; committed files regenerated.
- opt/t1-ffi against opt/t1-native: core-ffi transport-ready-tonic gmean 0.84-0.87 (P5.2-P5.4
  0.50-0.52, P2.4* 0.66-0.81, P2.2 0.98, P2.2/wide 0.73), P5.1 0.06 -> 0.15 us and P1.1
  0.29 -> 0.38 us (two allocations and two crossings cost more than a small copy); every other
  row noise (reused-buffer 0.98-1.00; prost 1.11 / 1.08 the other way). RPC b rows move
  0.84-1.27 with unchanged cells moving as much: unresolved.
- The coordinator's finding, confirmed from the code: cells D and F (and C, B, E, whose core
  transport is tonic too, ak-core rpc.rs unary_once) hand tonic a Bytes through the shared
  RawEncoder, whose encode is `dst.put_slice(&item)` into tonic's per-call EncodeBuf
  (tonic-0.14.6 codec/encode.rs encode_item). So every raw-bytes cell pays that copy
  whatever produced the Bytes, and the transport-ready-tonic row times producing the Bytes,
  not RawEncoder's copy (nor, for prost, tonic's per-call buffer and its growth). Full-message
  copies per call, direction b, before T1 -> now: A 0 (prost writes into EncodeBuf; its
  doubling growth aside) -> 0; B 2 (ak_call_unary's copy_from_slice + RawEncoder) -> 2; C 1
  (R2 move + RawEncoder) -> 1; D 2 (ak_enc_take + copy_from_slice, RawEncoder) -> 1; E 2 -> 2;
  F 2 (copy_from_slice, RawEncoder) -> 1. h2 chains DATA payloads >= 256 B (h2 0.4.19
  framed_write CHAIN_THRESHOLD, vectored writes on a Unix socket), so no further copy there
  on any cell. Step 2 is held; options assessed in the report, nothing implemented.

## 2026-09-27 -- owner decisions on step 2; T1 option 3 (framed send path) as labelled extra cells

Owner: option 3 approved as OPTIONAL (labelled extra cells beside the unchanged reference,
measured in the same run); step 2 (ak_enc_take_owned) KEPT; no small-message threshold for
Enc::take.

- d18540f0: `rpc::unary_framed(svc, path, msg, max_send)` in the shared rpc crate: tonic's
  Channel (it adds origin and user-agent), the request message as two body frames (the
  5-byte prefix, then the caller's Bytes; an empty message is the prefix alone), no copy on
  the host. Request as tonic 0.14.6's GrpcConfig::prepare_request builds it for
  Grpc::new(channel): POST, HTTP/2, the path, `te: trailers`, `content-type:
  application/grpc`; tonic adds `grpc-accept-encoding` only with accept-compression enabled
  and `grpc-timeout` only from a deadline, neither used on either path. Body size_hint left
  unknown, as EncodeBody's, so hyper adds no content-length. Response as
  Grpc::create_response + client_streaming: trailers-only status from the headers
  (Status::from_header_map), else Streaming::new_response(RawDecoder, ...), one message,
  then the trailers (their grpc-status checked by Streaming). Send limit: encode_item's two
  checks with its messages (max_send None = tonic's default, no limit; 4 GiB prefix
  limit). Compression off on both paths. Core switch: additive RPC entry
  `int32_t ak_client_set_framed(ak_client *c, int32_t on)` (0 reference, the default; 1
  framed; else or NULL -> AK_ERR_INVALID_STATE), read at call time by all three deliveries
  (blocking, callback, queue). Chosen over a member of ak_client_opts (a layout change of an
  RPC struct every slice declares) and over three additive per-call entries (one switch
  covers every delivery, the call entries and their counts stay as they are). Cells: Bf,
  Cf-*, Ef-* (the core's client switched once at Conn::open) and Df-*, Ff-* (the harness
  calls unary_framed); A has no twin.
- Evidence (logs/rust/opt/framed/header-diff.txt, bin/header_diff, now gate step 11d): the
  server records each request. Headers identical, same order, on both transports (tonic
  Channel, the core's client), both methods, shipped and pinned: `te: trailers`,
  `content-type: application/grpc`, `user-agent: tonic/0.14.6`, POST http://tonic/<path>
  HTTP/2.0. Request DATA frames as received (the proof each path ran): reference Fetch [5,
  0], framed [5]; reference Push 33 x 16384 + 16139 + an empty END_STREAM frame, framed 5 +
  33 x 16384 + 16134. A refused Push comes back as the same Status (InvalidArgument, same
  message) on both paths; another path the same response; the framed send limit refuses
  a 3-byte message under a 2-byte limit with encode_item's OutOfRange message, unsent.
  Found on the way: the core applies neither ak_client_opts.max_send_message nor
  max_recv_message on EITHER path (pre-existing; not changed).
- Requirement 18 per path: the plant control now runs once per send path, each warmed
  through ONE cell so each must abort on its own (rpc_client --warm-cells A | B | Bf | Df):
  every one aborted with no output, both clients, both transports (opt_bench and
  run_campaign.sh). The check message names the full cell.
- Crossings: new rows rpc:Bf, Cf, Df, Ef, each equal to its reference twin (set_framed is
  called once at open, outside the counted call); files regenerated.
- Full run logs/rust/opt/framed (598 s; the grid is now 19 cells full / 11 no-unknown).
  framed/reference in-process, geometric means over transports x modes x k
  (framed/framed-pairs.txt): b: B 0.72, C 0.89, D 0.71, E 0.93, F 0.99, but single rows run
  0.20-5.94: at k=8 and k=16 the round distribution is bimodal (rounds at 0.5-0.7 ms and at
  2-3.5 ms per call) and 3 rounds do not resolve anything; a: 0.97-1.05 except B 1.25.
  Narrowed run logs/rust/opt/framed-rpc-narrow (gen/rpc_narrow.sh + gen/rpc_pairs.py, new:
  14 cells, a and b, k=1, 3 launches x 10 rounds x 48 calls per transport): the round
  distribution is heavy-tailed there too (max 3-5x the median), so the 10th percentile is
  printed beside the median. Direction b, framed/reference, p10 gmean (median gmean):
  B 0.89 (0.78), C 0.98 (1.02), D 0.95 (0.91), E 0.94 (1.11), F 0.95 (1.00); per p10 pair
  0.87-1.02. Direction a p10 0.91-1.04 (B's 1.25 of the full run is not reproduced: 0.94 /
  1.01 per transport). Reading, as instrumentation: removing one 540 KB copy (and tonic's
  per-call buffer growth to hold it) is worth a few percent of a call's client CPU on D, E,
  F and ~10% on B (two copies -> one); the container's tail noise is larger than the effect
  at every median. Kept as labelled extra cells, as the owner asked.

## 2026-09-27 -- step 3, N2 (kept, b1ecc8f8)

- core-native decode: a message with repeated message or string/bytes fields (maps excluded)
  first counts their occurrences in one pass over its keys (`Dec::new(&buf[d.pos..])`, key,
  then len_body or skip; an error just ends the pass, the decode proper reports it), then
  `reserve_exact` per Vec. Rendered by `_count_prescan` in rust_native.py for every such
  message (TaskDetailed's four repeated strings, every root's repeated elements).
- Narrowed alternated A/B (opt/n2-ab, 3 x A/B, P1.2*, P2.2*, P2.3, P2.4*, P4.1, P7.1, A =
  f3c406f0): native decode P2.3 0.82 / 0.90 (drop / retain), P2.4 0.90 / 0.84, P2.4/wide
  0.89 / 0.94, P1.2 0.92 / 0.94, prost 0.96-1.09 in the B builds; P2.2/latin1 1.08 / 1.13
  against prost 1.05 (the pass costs a little where every repeated field is short).
- Full run opt/n2 (581 s) against opt/framed: native decode gmean drop 0.96, retain 0.99,
  no-unknown 0.93 (prost 0.98 / 1.00); P2.3 0.82 / 0.98 / 0.74, P2.4 0.90 / 0.91 / 0.85,
  P2.2 1.08 / 0.92 / 1.05 (prost 1.04). ffi columns noise (0.97-1.01). Checks: pre-check 0
  failures, crossings identical (core-native crosses nothing).

## 2026-09-27 -- step 4, N3 (kept, 1326b546); step 5, N6 (refuted, no code change)

- N3: core-native retain encode appends a message's unknown-field bag only when it is
  non-empty (rust_native.py; the reserve inside Enc::put was paid for every message of every
  payload). Narrowed A/B opt/n3-ab (3 x A/B, P1.1, P1.3, P2.1, P2.2, P2.5, P3.1, P4.1, P6.1;
  A = b1ecc8f8): native-retain encode gmean 0.91-0.92 over the three variants, native-drop
  (unchanged code) 1.05-1.07, prost 1.02. Full run opt/n3 (576 s) against opt/n2:
  native-retain encode 0.88-0.91, native-drop 0.96-0.98, native-nounk 1.03-1.05 (both
  unchanged), prost 0.94-0.95 / 1.00-1.01. Checks: pre-check 0 failures, crossings identical.
- N6 (ffi-retain P4.1 decode ~12% over drop, from the earlier analysis): not reproduced.
  In-process ffi retain/drop on P4.1 decode, 3 narrowed processes (opt/n6-probe): 0.99,
  1.04, 1.01 (decode-read 0.94-0.96); in the six full runs of this unit: 0.98, 0.92, 1.02,
  0.90, 1.11, 0.93. The counting build shows retain costs one forward crossing more (the
  ak_dec_reset_<Root>, U1) and the same 601 reverse (P4.1 rows of gen/crossings.txt), so
  there is no per-element retain crossing to remove. The earlier 12% was taken on a harness
  and a code state that no longer exist (before U1 moved the reset, before D2-D4); no code
  changed for N6, so there is no n6 run.

## 2026-09-27 -- core-only fat LTO (owner: the lib with LTO is fine, the host not): measured, NOT kept

- Tooling (kept, opt-in, default off): gen/core_lto.sh builds libak_core.so ON ITS OWN in
  poc/codec's workspace with CARGO_PROFILE_RELEASE_LTO=fat for that invocation only
  (codegen-units default; the codec manifest's [profile.release] lto=false is what the env
  var overrides), with this host's ak-core features (full: rpc,init-guard,unknown-fields;
  nounk: rpc,init-guard), into target-core-lto[-nounk]/. The host is built with
  AK_CORE_LIB_DIR set to that directory: harness/build.rs (already) and campaign/build.rs
  (new) put it FIRST in the runpath, so the loader takes it; the non-LTO copy cargo still
  builds in the host's deps/ as the path dependency (feature plumbing) is built and never
  loaded -- checked per executable with ldd against the expected path (opt_narrow.sh
  check_so), with the loaded file's text-symbol count as the marker (LTO core 2119 / 2083
  text symbols full / nounk, the host-built one 4210 / 4222; sizes 2.2 vs 3.4 MB). An empty
  AK_CORE_LIB_DIR is now treated as unset by both build scripts. Same dependency versions in
  both lock files (hashbrown 0.15.5 extra on the host side, not in the core's graph);
  feature unification differs only in tokio-stream (+default,time), syn (+default,derive),
  serde_core (+alloc) on the host side, none of which the core's code uses. opt_narrow.sh
  prints "core cdylib: lto=fat ...; host: lto off" (or "lto off (the host's path-dependency
  build)") in its header. opt_bench.sh and run_campaign.sh are NOT changed: nothing is kept
  that would need it.
- Host not LTO'd: the codec bench executable keeps 53 out-of-line calls to
  ak_rt::dec::Dec::skip (logs/rust/opt/lto-ab/host-not-lto.txt); gen/inline_check.sh with the
  LTO core (lto-ab/inline_check-with-lto-core.txt): core-native's traversal is emitted on
  its own (decode 13586 B, encode 4067 B) and the benchmark closures are 294-472 B, so
  nothing is fused into the loop (R5). core-native gains nothing from core LTO by
  construction (it is compiled into the host).
- Narrowed alternated A/B (opt/lto-ab, 3 x A/B, P1.2*, P1.3, P2.2*, P2.3, P3.1, P4.1, P6.1,
  P7.1, full build; A = 7770b363 with the host-built core, B = the same code with the LTO
  core): decode ffi-drop 1.02, ffi-retain 1.04, pull 1.02-1.04; encode reused-buffer
  ffi-drop 1.03, ffi-retain 1.04; core-native (unchanged by construction) 1.00-1.03; prost
  0.93 / 1.02. No gain beyond drift, so no full run and nothing kept.

## 2026-09-27 -- stable gate checkpoint before N5 (owner): PASSED

`run_campaign.sh --suite gate` from a clean tree at 33636e1d (steps 0-5, option 3, the LTO
tooling): gate PASSED (logs/rust/opt/pre-n5-gate/gate.log): generators current and one core;
core unit tests; byte identity against the manifest; presence/oneof/unknown vectors;
crossing counts; content sets; the concurrency suite (shipped and global-table builds pass,
padded builds fail as they must); lifecycle guard off/on; R-D1; R-D6 all pass; the corpus on
all four arms with its controls failing; the framed path's header check (11d); the
no-unknown variant.

## 2026-09-27 -- U1-unary (owner): the labelled extra RPC direction `c`, kept (f1dc5de8)

- Direction c = a result upload: the request is P5.3 (1 MB) or P5.4 (4 MB), M5
  UploadResultDataMessage, built from the payload builders and checked against the
  validated manifest before any call (and prost re-encodes the same bytes); the response is
  empty; the server decodes the request with prost (as in b) and refuses an empty upload.
  Every cell has it (A, B/Bf, C/Cf, D/Df, E/Ef, F/Ff, and the no-unknown client's cells), at
  k = 1 and 8 only (grid::C_INFLIGHT). The paths are b's: B prost encode_to_vec + ak_call_unary;
  C ak_call_unary_enc; D ak_enc_take_owned + tonic; E Enc buffer + ak_call_unary; F Enc::take +
  tonic; framed twins through rpc::unary_framed.
- Found on the way: tonic's server refuses a message over 4 MiB by default and P5.4 is
  4,194,390 B ("decoded message length too large ... limit is: 4194304"), on every path; the
  grid's server now accepts 8 MiB (server::SERVER_MAX_RECV, in the rpc header).
- Requirement 18: status and response length (0) on every call; the plant control now runs
  per send path AND per direction (rpc_client --warm-cells A|B|Bf|Df --warm-dir a|c; the
  plant expects 1 byte on c): all 16 aborted with no output in opt/u1-unary (both clients,
  both transports).
- Crossings (req 19): new rows rpc:<cell> c/P5.3, c/P5.4: B 2 (call, free), C 4 (reset,
  encode, ak_call_unary_enc, free; 1 reset), D 4 (reset, encode, ak_enc_take_owned, free), E 2,
  framed twins identical; 0 reverse (M5 has no loop). Files regenerated.
- Full run opt/u1-unary (633 s; c-direction.txt tabulates c) and a narrowed rerun
  opt/u1-unary-narrow (11 cells, drop mode, c only, k 1 and 8, 3 launches x 5 rounds per
  transport; pairs.txt, medians.txt). Client CPU per call, narrowed medians, pinned, k=1 /
  k=8: P5.3 A 471 / 563 us, B 665 / 737, Bf 559 / 610, C 578 / 632, Cf 510 / 584, D 589 / 830,
  Df 489 / 534, F 581 / 645, Ff 493 / 532; P5.4 A 1812 / 3037, B 6392 / 6756 (p10 2507 /
  6077: bimodal), Bf 2214 / 2273, C 2196 / 2460, Cf 1875 / 2080, D 2184 / 3490, Df 1854 / 1983,
  E 2583 / 3072, Ef 2133 / 2258, F 2147 / 3316, Ff 1821 / 2020. framed/reference gmean
  (median; p10): P5.3 B 0.83, C 0.86, D 0.78, E 0.87, F 0.83 (p10 0.82-0.88); P5.4 B 0.37,
  C 0.85, D 0.68, E 0.82, F 0.72 (p10 0.58-0.84). At P5.4 k=8 the framed core-arm cells (Cf,
  Df, Ff: 1.98-2.08 ms) are below A (3.04 ms), whose prost encode grows tonic's per-call
  buffer to 4 MB. The full run's Bf shipped rows (2.0 / 4.3 ms P5.3, 4.0 / 10.3 ms P5.4, rounds
  739-4818 us) are not reproduced in the narrowed run (Bf/B 0.80-0.85, 0.35-0.44). All
  container instrumentation.

## 2026-09-27 -- stable gate checkpoint after U1-unary, before N5: PASSED

`run_campaign.sh --suite gate` from a clean tree at 186a4e52 (everything up to and including
U1-unary): gate PASSED (logs/rust/opt/pre-n5-gate2/gate.log), the same sections as the first
checkpoint.

## 2026-09-27 -- step 6, N5 EXPERIMENT (apply-first element order): built, correct, REVERTED

- 19dc9237 (reverted by 0da6fcfe): additive entries `ak_decode_<R>_af` (every root, every
  header: C, C#, the ak-abi declarations) beside the unchanged `ak_decode_<R>`; one generic
  root body `dec_root_<r><const AF: bool>` and element decoder `dec_<r>_<slot>_element<AF>`,
  so AF=false is the reference order. AF: no `new_<slot>`; each inner run is HELD (bump
  allocation in the shared D2 arena, a 64-entry table of (slot, offset, count)); at the end
  `apply_<slot>(tok = -1)` (the host constructs the element from the group and appends it),
  then the held runs with token -1 ("the element apply just made"). Fallback LIVE when a run
  cannot be held (arena full: the run's budget reaches 0; or the table full): `new`, the held
  runs with its token, then the reference order for the rest of the element. Plan option
  `Options.elem_order` ("new_apply" default, "apply_first"); only rust_binding renders
  apply_first (decode_with_* calls `_af`; apply/add accept token -1; a `from_<T>` constructor
  for slot-carrying element types); every other backend unchanged (their generated files
  change only by the added declarations). The Options repr omits elem_order at its default,
  so the other slices' generated text that embeds it did not move.
- Correctness: pre-check 0 failures on both builds (P7.1's interleaved runs included); two
  fallback inputs added to the pre-check and the counting build (N5-arena: 3000 spans in one
  run, over the 32 KB arena; N5-held: 140 alternating runs, over the 64-entry table; each a
  two-element response, the second element ordinary), core-ffi == core-native in both modes;
  the corpus (gen/corpus.sh) passed. Crossings: every push decode row -1 reverse per
  non-batchable element (P2.2 3501 -> 3001, P2.4 561 -> 481, rpc:C/Cf/D/Df direction a
  3501 -> 3001); no payload reached the fallback; N5-arena 9 reverse, N5-held 146 (derived
  in the commit and matching: new + delivered held runs + the rest live).
- Measured (narrowed alternated A/B, 3 x A/B, P2.1-P2.5, P4.1, P7.1; A = 7770b363, the codec
  identical to eaf0948d): core-ffi push decode ffi-drop 1.06, ffi-retain 1.03 (P2.2 1.12 /
  1.04, P2.5 1.18 / 1.14), core-native 0.96-0.97, prost 1.01 (opt/n5-ab). Asking whether the
  construction was the cost: a variant with apply(-1) pushing a default element and filling
  it in place (no from_) measured 1.09 / 1.10 (opt/n5b-ab), so the cost is in the core's
  element decoder (the held-run machinery: variable run pointers and budgets reset on every
  hold, the delivery loop), not in constructing the element; saving one reverse crossing per
  element (a few ns each here) does not cover it. Reverted as not paying; no full run.
- What another backend would need to use it (reported, not built): call `ak_decode_<R>_af`
  instead of `ak_decode_<R>`; its `apply_<slot>` must accept token -1 and construct+append
  the element from the group; its inner `add_<slot>_<inner>` must accept token -1 as "the
  element apply just appended"; and it must still implement `new_<slot>` for the fallback.

## 2026-09-27 -- U2-stream (owner): ABI v1 section 9's client streaming and direction `d`, kept (122dc8ae)

- Core (rpc.rs; declared through plan.RpcAbi, so every slice's header, the C# RpcAbi.cs and
  the python binding.c tallies carry them; other slices call none): `ak_call_open(c, path,
  path_len, kind) -> ak_call*` (kind AK_CALL_CLIENT_STREAM = 1, the only kind built),
  `ak_call_send(h, msg, len, last) -> i32` (copied), additive `ak_call_send_enc(h, enc,
  last)` (the context's buffer moved, Enc::take, as ak_call_unary_enc), `ak_call_recv(h,
  out) -> i32` (the response; a second recv AK_ERR_INVALID_STATE; a failure or a cancelled
  call AK_ERR_HOST), `ak_call_close(h)` (aborts the call's task: unblocks a pending recv or
  send; does not free), `ak_call_destroy` unchanged (frees). Blocking delivery (req 16):
  the request messages go to the call's task through a bounded channel of 1, so a send
  blocks while the transport has not taken the previous message. Both send paths
  (ak_client_set_framed applies): rpc::client_streaming_raw (Grpc::client_streaming +
  RawCodec) and rpc::client_streaming_framed (every message as its prefix frame and itself).
  ak-core unit test: one-message stream on both paths, send after last refused, second recv
  refused, unknown kind NULL, close unblocks a recv the server would never answer.
- Section 9's signatures, as mapped (reported, the ABI text not changed): the implemented
  unary entries flatten `ak_bytes_in` into (ptr, len), return an i32 status and take no
  `ak_call_opts` and no `ak_err*`; the streamed entries follow the same mapping.
  `ak_call_kind` and `ak_call_opts` are named in section 9 and defined nowhere, so kind is
  an i32 with one constant and opts is not taken. Section 9's `ak_call_close` ("cancels,
  unblocks a pending recv, does NOT free") has the semantics the implemented
  `ak_call_cancel` already has for unary handles: two entries for one operation now exist.
  No gRPC status number is returned (section 9's amendment), as for unary.
- The rust slice: direction d = CAMPAIGN req 14's streamed upload in 2 MiB chunks (FIX-PLAN
  D5), ArmoniK's UploadResultData shape with M5 per chunk (the ids on the first message
  only, empty strings not on the wire; no shape change), 4 MiB (2 chunks) and 16 MiB (8),
  k = 1 and 8, every cell and framed twin, a third of the calls per round. A: tonic
  client_streaming with prost; B: prost encode_to_vec + ak_call_send; C: core-ffi encode +
  ak_call_send_enc; E: core-native Enc + ak_call_send; D/F: tonic client streaming with the
  raw codec (or framed), each message encoded lazily as the transport asks (D
  ak_enc_take_owned, F Enc::take). Data: 2 MiB chunks of splitmix64 bytes, each chunk's wire
  checked prost == core-native before any call. Server: a tonic client-streaming handler
  that decodes every message with prost (ids required on the first) and answers the data
  byte count (u64 LE); its STREAM_CHECK twin adds the SHA-256 of every message's bytes as
  received.
- Correctness before timing: bin upload_check (gate step 11e, full and no-unknown builds):
  every cell, reference and framed, 4 MiB and 16 MiB: count and SHA-256 identical; direction
  c's unary uploads accepted; controls: a planted wrong SHA-256 and a planted wrong count
  detected on B, Bf, D, Df. header_diff now also compares the streamed request: headers
  identical on both transports, the framed stream received as a 5-byte frame, then 16 KB
  frames. Req 18: the response count checked on every call; the plant per send path now
  also through d (--warm-dir d): 24 controls per run aborted with no output (48 lines in
  opt/u2-stream's runner.log with both transports).
- Crossings (req 19): new rows rpc:<cell> d/4MiB, d/16MiB: B and E 6 / 12 (open, a send per
  chunk, recv, free, destroy), C 10 / 28 (+ reset and encode per chunk; resets 2 / 8), D 8 /
  32 (tonic's stream, no core call entry: reset, encode, ak_enc_take_owned and ak_bytes_free
  per chunk; resets 2 / 8); framed twins identical. The
  counting build encodes D/F's chunks on the calling thread (the binding's ak_enc_reset tally
  is thread-local; the first counting run showed D's resets as 0 and was fixed before
  commit); the entries per call are the same either way.
- Full run opt/u2-stream (706 s; d-direction.txt, framed-pairs.txt). Client CPU per call,
  pinned, k=1, 16 MiB: A 7.7 ms, B 10.1, Bf 9.6, C 8.4, Cf 8.1, D 8.4, Df 7.0, E 9.6, Ef 8.1,
  F 8.0, Ff 6.9; 4 MiB: A 1.78, B 2.43, Bf 2.13, C 2.25, Cf 1.76, D 2.19, Df 1.76, E 2.41, Ef
  2.32, F 2.10, Ff 1.78. framed/reference gmean over transports, modes and k: 16 MiB C 0.85,
  D 0.79, E 0.83, F 0.81, B 1.09; 4 MiB C 0.81, D 0.80, E 0.82, F 0.89, B 0.92 (single rows
  0.50-1.89). The full client's Bf on the shipped transport is slow again (16 MiB k=8 21.9 ms
  vs B 13.5; 4 MiB k=8 9.1 vs 4.1; nounk client's Bf 13.0 vs 13.3), as in opt/framed and
  opt/u1-unary, and it was not reproduced in the narrowed U1 run: open, cause not identified.
  Instrumentation throughout.

## 2026-09-27 -- unit 2 final gate and final run

- Final gate from a clean tree at d54ea963 (logs/rust/opt/final2-gate): `run_campaign.sh
  --suite gate` PASSED on stable (gate.log; 11d framed headers and 11e the upload byte check
  included, both builds), and `RUSTUP_TOOLCHAIN=1.88.0 bash gen/gate.sh` PASSED on rustc
  1.88.0 (gate-floor-1.88.log).
- Final run logs/rust/opt/final2 (opt_bench v5 at the gate-log commit 22e7eaa0, code
  d54ea963; 818 s, the build included) against t0-ref (variants-before-after.txt,
  by-direction.txt, headline-before-after.txt, rpc-before-after.txt, framed-pairs.txt).
  Geometric means final/t0 over the payload rows: prost 1.01 / 1.01 (full / nounk, the
  control; decode 1.08, encode 0.97), native-drop 0.98, native-retain 0.92, native-nounk
  0.97, ffi-drop 1.02, ffi-retain 1.02, ffi-nounk 0.98, pull 1.06 / 1.00. Per direction and
  variant: transport-ready-tonic native 0.81-0.85, ffi 0.88-0.92 (P5.3 / P5.4 0.44-0.54, P2.4
  0.65-0.88; P5.1 0.04-0.06 -> 0.09-0.15 us: the move's fixed cost); reused-buffer native-retain
  0.93 (N3), other reused-buffer and transport-ready-core rows 1.01-1.07; decode native 1.00-1.03
  with P2.3 0.84-0.91 and P2.4 0.91-0.97 (N2), ffi 1.00-1.05 (no ffi decode code changed in a
  kept step; t0-ref's ffi decode is low against every later run, e.g. P2.4 ffi-drop 1524 us
  there and 1694-2004 in the eight runs after: drift, D43). RPC framed/reference inside the
  final process (gmean, median; p10): b B 0.78, C 0.88, D 0.92, E 0.98, F 0.87; c/P5.3 0.79-0.91,
  c/P5.4 B 0.44, C 0.88, D 0.70, E 0.87, F 0.78; d/16MiB B 1.01, C 0.81, D 0.78, E 0.81, F 0.95;
  d/4MiB 0.66-1.03 (p10 0.68-0.93); a and a+read 0.92-1.05 except B 1.13-1.15 (O1: the full
  client's Bf again). All container instrumentation.

## 2026-09-27 -- unit 3: ABI v1 section 9 as specified (b3ac5050, f9c25d0b)

Spec: ABI-v1.md section 9 "Streaming, as built", "Two more additive entries" (fe79f874) and
"The status number, on unary calls too" (22ebb97f). All in the shared core and plan.RpcAbi,
rendered into every slice's header/binding; other slices' generated output regenerated
(generate.py --check ok).

- plan.RpcAbi: AK_ERR_RPC_STATUS -12; AK_CALL_CLIENT_STREAM 1, AK_CALL_SERVER_STREAM 2,
  AK_CALL_BIDI_STREAM 3; ak_kv, ak_call_opts; ak_call_open(c, path, path_len, kind, opts);
  ak_call_recv(h, out, grpc_status); ak_call_close removed; ak_completion {tag, status,
  grpc_status, bytes} (grpc_status sits in the old padding: size 40, bytes at 16, unchanged);
  ak_call_unary / ak_call_unary_enc trailing `int32_t *grpc_status`. New RpcAbi.layout()
  derives each RPC struct's 64-bit size/offsets once; c_abi.py emits them as AK_SASSERTs under
  a UINTPTR_MAX guard and rust_abi.py as const asserts in rpc_check.rs (before, only the
  function signatures were checked). c_variant.sh passes; the C# layout probe prints the new
  structs.
- rpc crate: CallCfg {max_send, max_recv, metadata, deadline}, CallErr {Limit, Status};
  unary_raw / unary_framed_cfg / client_streaming_raw_cfg / client_streaming_framed_cfg take
  it (old signatures kept as wrappers). The send limit is checked before anything is sent;
  tonic's decode-limit error (OUT_OF_RANGE, "decoded message length too large") is translated
  to RESOURCE_EXHAUSTED (8); the framed path checks the response length against max_recv
  itself. The deadline is Request::set_timeout (grpc-timeout on the wire) plus a client-side
  tokio timeout; tonic's server answers an expired grpc-timeout with CANCELLED "Timeout
  expired", which arrived first in the deadline test, and is mapped to DEADLINE_EXCEEDED.
- ak-core: ak_call_cancel on a cb/q handle used to abort the task and deliver no completion,
  contrary to its comment; it now fires a Notify and the task delivers a CANCELLED completion
  (status -12, grpc_status 1). On a stream it unblocks a pending send/recv the same way and
  frees nothing. Metadata: "-bin" keys binary (BinaryMetadataValue), others ASCII with an
  explicit 0x20..0x7E check (tonic's AsciiMetadataValue accepts obs-text, e.g. "cafe" with an
  e-acute passed); an invalid pair makes ak_call_open return NULL. Unary outcomes: OK ->
  (AK_OK, 0); send limit -> (AK_ERR_LIMIT, -1), no call made; receive limit -> (AK_ERR_LIMIT,
  8); other status -> (AK_ERR_RPC_STATUS, code). ak-core unit tests 11 passed.
- Campaign server: test paths under /armonik.ffi.campaign.v1.Grid/ (StatusU<n>, StatusS<n>,
  SleepU, SleepS 3 s, EchoS echoing ak-echo and ak-echo-bin). bin/rpc_semantics runs every
  case on the reference and the framed path: status on blocking, cb and queue deliveries and
  on ak_call_unary_enc and the stream; deadline 300 ms -> 4; metadata echo and grpc-timeout
  seen by the server; non-ASCII value -> NULL; cancel of a pending recv -> (-12, 1) then send
  -> AK_ERR_HOST; cancel of cb/q on SleepU -> (-12, 1); misuse -> -8 with grpc_status left
  at -99; reserved kinds -> NULL; send limit 1024 (unary 2048 B -> (-5, -1) and 0 requests at
  the server; stream [2012 B, 110 B] -> sends [-5, 0], server got 100 data bytes); receive
  limit 1024 (unary blocking and queue -> (-5, 8)), 16 on a stream -> (-12, 8). PASSED 3
  times and on the nounk build; gate step 11f.
- Grid limits: 0/0 (tonic defaults: unlimited send, 4 MiB receive) suffice for every
  campaign path: the largest response is P2.2 (540,422 B), requests (P5.4 4,194,390 B, stream
  messages 2 MiB + 57 B) are under an unlimited send; the grid server accepts 8 MiB. Stated in
  rpc_client's header; packages/rust's transport config is unchanged (sets neither).
- Checks (logs/rust/opt/abi9/checks/checks.log): generate --check, one_core ok; pre-check 0
  failures on both builds; crossings 775 / 398 rows identical (the new arguments are
  out-parameters; no entry added on a measured path).
- Other slices' hand-written call sites that no longer compile or would misread (for WP8):
  listed in STATE "Open defects".
- O1 (the full client's Bf on the shipped transport): owner dropped the investigation
  ("most likely VM contention or system activity"); recorded as observed in the container,
  not investigated, deferred to the campaign machine.

## 2026-09-27 -- unit 3 final gate, first attempt FAILED (a gate-script defect)

- `run_campaign.sh --suite gate` at 98b187ce (clean tree) stopped in step 11f after "cases
  passed: 40" with no FAIL line (logs/rust/opt/final3-gate/failed-98b187ce/gate.log). Cause:
  gate.sh runs under `set -euo pipefail`, and the step's `grep "^FAIL" ... | sed` returns 1
  when nothing failed, which ended the gate; the same `cmd > f; rc=$?` form would also have
  exited before reporting a real failure. rpc_semantics itself passed all 40 cases. Fixed:
  `|| rc=$?` and `|| true` on the reporting greps; both paths checked with a stand-in
  command (a failing one prints its FAIL line and "FAILED", exit 1; a passing one "OK", exit
  0). Swept gate.sh: no other step has the form. Gate re-run from a clean tree.
- gen/opt_tables.py: the tables of one opt_bench run (tables-codec.md, tables-rpc.md), no
  comparison.

## 2026-09-27 -- unit 3 final gate and final run

- Gate from a clean tree at 6727646b (logs/rust/opt/final3-gate): `run_campaign.sh --suite
  gate` PASSED on stable (step 11f 40 cases, the no-unknown build's too; pre-check 5,740 /
  3,257 checks, 0 failures; crossings 775 / 398 rows identical), and `RUSTUP_TOOLCHAIN=1.88.0
  bash gen/gate.sh` PASSED on rustc 1.88.0 (gate-floor-1.88.log).
- Final run logs/rust/opt/final3 (opt_bench v5 at 0a26bdc4, 855 s with the build): pre-check
  0 failures in the four codec processes; the 48 planted RPC controls (A, B, Bf, Df x a, c, d
  x both transports x both clients) aborted as required. Tables of this run only, no
  comparison: tables-codec.md (170 rows x 16 variant columns, U-* retain/drop summary) and
  tables-rpc.md (every cell and framed twin, a, a+read, b, c/P5.3, c/P5.4, d/4MiB, d/16MiB,
  both transports, every k, client CPU and wall, median [min-max] over 3 rounds; full and
  no-unknown clients). Round spread (max/min per entry, client CPU, 1,020 entries): median
  1.08, p90 1.41, max 8.08. Container instrumentation.

## 2026-09-27 -- CAMPAIGN req 24 as amended (85cfd4826): warm-ups as runner parameters

- Checked: criterion's warm-up (AK_WARMUP_MS 500 / smoke 5 ms) and the fixed iterations
  (AK_WARMUP_ITERS 100 / 3) were already knobs, shortened under smoke. The RPC warm-ups
  (AK_RPC_WARMUP, AK_RPC_SERVER_WARMUP, 64) were knobs in a campaign run, but AK_SMOKE=1
  overwrote them with 16, ignoring the environment. Fixed: the smoke default is 16 and the
  environment wins in both modes. The runner header now lists every warm-up knob with its
  campaign and smoke defaults; the values used stay in each log's own header. No gated
  path changed (run_campaign.sh is not in gen/gate.sh), so no gate was run.

## 2026-09-27 -- FIX-PLAN WP9: the RPC grid on criterion (CAMPAIGN req 22a as amended)

- benches/rpc_suite.rs replaces bin/rpc_client (removed): one criterion benchmark per
  (cell, dir, payload, k), every cell, direction (a, a+read, b, c, d), k, mode and framed
  twin kept; one iteration = one batch of k calls in flight (Throughput::Elements(k)); the
  process-CPU Measurement of the codec suite; wall from the routine's own clock around the
  same iterations (iter_custom), matched to criterion's samples by (iterations, cpu) at
  export. Runtimes, core clients, channels (one per cell per process) and the k callers are
  built outside the measured closure. A failed check panics the benchmark (criterion has no
  stop-on-error); the runner discards the launch's files. New control: AK_RPC_PLANT=bench,
  a wrong length inside the first criterion benchmark, must abort with no sample.
- The owner's addendum (bc7cf94b1): no hand-written warm-up beside criterion's. Removed the
  RPC per-benchmark pre-calls and the codec suite's AK_WARMUP_ITERS loop; criterion's
  warm-up is the warm-up (AK_WARMUP_MS, AK_RPC_WARMUP_MS). Kept, each for a requirement:
  the server and its warm-up (13), ProcessCpu and the wall column (21), abort-and-discard
  (18), the seeded registration order (22), the export (28), the codec pre-check (26) and
  pool build (11).
- gen/rpc_narrow.sh and gen/opt_bench.sh call the bench through the environment.
- Crossing counts unchanged (counting builds; the grid's per-call counts come from `crossings`).

## 2026-09-27 -- WP9 + WP10 verified (small tests)

- One clean-worktree gate at bed13a6ea (stable, both builds): PASSED; crossing counts
  identical. The campaign benches and bins build on 1.88.0. Minimal smoke (shipped, full
  build, one launch, 2 ms warm-up and 5 ms measurement per benchmark, serve.sh warm 4):
  serve.sh drove the server end to end, the 13 plants aborted with no sample, 323 RPC
  benchmarks x 10 samples exported with every label; codec P1.1 on both builds. The
  interface in SERVER.md did not change. serve.sh's target-server/ added to .gitignore.
- A first gate attempt was stopped at step 12 by hand (the WP10 unit arrived), and the
  spend limit stopped the session once after the run had finished; nothing was re-run.

## 2026-09-28 -- RPC grid only, for a same-machine side-by-side with the C++ grid (coordinator unit)

- HEAD dcbb0205 (the C++ unit's shared-core changes included: one-pass ak_blob_run, additive
  ak_utf8_check), rebuilt; no code change, no gate (coordinator). New tooling only:
  gen/rpc_same_machine.sh (serve.sh server: 4 tokio workers pinned to CPUs 2,3, serve.sh
  warm 50; client pinned to CPU 1; shipped then pinned, full then no-unknown client; k = 1, 8;
  every cell and framed twin; directions a, a+read, b, c, d) and gen/rpc_same_machine.py
  (summary-rpc.tsv with samples and batches per sample, tables-rpc.md). No planted controls
  in this run (every call is still checked inside the bench).
- Criterion 0.5, process-CPU measurement, SamplingMode::Flat, 10 samples, warm-up 30 ms,
  nresamples 1000; measurement time per direction group, each group its own process per
  (transport, client): a/a+read/b 250 ms, c 250 ms, d 500 ms. Smoke first (A, Bf; 20 ms):
  my first smoke named C-nounk in the full build and panicked ("no cell C-nounk in this
  build"), a narrowing mistake of mine, not a defect.
- Run logs/rust/opt/rpc-same-machine: 545 s from server start to stop (build excluded), 840
  entries x 10 samples, machine line "Intel(R) Xeon(R) Processor @ 2.80GHz; 4 CPUs online"
  (the earlier runs of this slice were on a 2.10 GHz container). Batches per sample 1-26;
  311 entries have ONE batch per sample (every k = 8 entry of a, a+read, b, c/P5.4, d/4MiB
  and d/16MiB, most of c/P5.3 k = 8, and some d/16MiB k = 1), marked `*` in tables-rpc.md:
  criterion's flat mode gave them one iteration at these measurement times, and larger
  times did not fit the 10-minute budget. Round spread (max/min, client CPU): 1-batch entries
  median 1.51, p90 5.44, max 12.4; entries with at least 2 batches median 1.27, p90 1.54, max
  3.0. Container instrumentation.

## 2026-09-28 -- callback and queue deliveries of client streaming; the callback cells (owner: "use callback with oneshot channel for the core-transport")

- Core (afc585db, additive, rendered through plan.RpcAbi into every header and binding):
  `ak_call_send_cb(h, msg, len, last, cb, user_data, tag)`, `ak_call_send_enc_cb(h, enc,
  last, cb, user_data, tag)`, `ak_call_recv_cb(h, cb, user_data, tag)` and the queue twins
  `ak_call_send_q(h, msg, len, last, q, tag)`, `ak_call_send_enc_q(h, enc, last, q, tag)`,
  `ak_call_recv_q(h, q, tag)`, all returning int32_t: AK_OK and ONE completion follows, or
  the refusal and none. A tag on the callback forms too, as ak_call_unary_cb has one. One
  call path per operation: send_begin (stream, send limit, the sender taken out of its slot)
  and send_end (sender back unless last, AK_ERR_HOST when the channel is closed) around the
  same `send` future, blocking_send for the blocking form and awaited on a task for cb/q;
  recv_begin + recv_once + stream_outcome for all three recvs. A send completion fires after
  the sender is back, so the next send may be issued from inside it. Also additive:
  `ak_call_unary_enc_cb` / `ak_call_unary_enc_q` (the moved-encode unary request with a
  callback or queue completion), needed so cell C-cb does the same work as blocking C
  (without it C-cb would pay a copy of the request that C does not).
- Deviation from the coordinator's wording, and why: a pending send cancelled by
  ak_call_cancel completes with AK_ERR_HOST (grpc_status -1), not CANCELLED, as the blocking
  send returns AK_ERR_HOST; the send path cannot know the call's status (the call's future is
  dropped before its status is set), and the status is read with a recv, whose completion is
  CANCELLED. Reported to the coordinator.
- Harness (7d10668d): cells B-cb, C-cb-{retain,drop} / C-cb-nounk, E-cb-*, and the framed
  twins Bf-cb, Cf-cb-*, Ef-cb-*: each call through the callback delivery, the completion
  sent into a tokio oneshot awaited by one of k tasks on the cell's own runtime (2 workers, as
  A/D/F); directions a, a+read, b, c, d (d: one oneshot per send completion and one for the
  response). CAMPAIGN req 16 as amended (owner, c1d3db50): these are Rust's reference core
  cells; the blocking cells stay, labelled. grid::stem/mode_of/cb/delivery; rows carry
  "delivery"; tables mark (ref) / (blk).
- Checks (logs/rust/opt/cb-deliveries/checks): generate --check, one_core ok; pre-check 0
  failures on both builds (5,740 / 3,257); crossing files regenerated, only additions (+60
  rows full, +36 no-unknown; every existing row unchanged): unary cb cells 3 forward (call,
  destroy, free) + 1 reverse against the blocking cells' 2 + 0; d/4MiB 6 + 3 and d/16MiB 12 + 9
  (a reverse per send completion and one for the response) against 6 + 0 and 12 + 0; C-cb a
  reverse 3,502 = the decode's 3,501 + 1. rpc_semantics 72 cases PASSED on both builds (40
  before: the stream's cb and q forms on both send paths: status, moved-encode bytes to the
  checking path, tags, cancel of a pending recv and of a pending send (server path StallS),
  misuse, send and receive limits; ak_call_unary_enc_cb / _q). upload_check PASSED on both
  builds (every cb cell, count + SHA-256; controls on B-cb and Bf-cb detected). The C++ slice
  builds against the regenerated header, default and -DAK_RPC=ON targets (no C++ change).
- Grid run logs/rust/opt/rpc-same-machine-cb (settings of 22a08fe2: G1 250 ms, G2 250 ms,
  G3 500 ms, 10 samples, warm-up 30 ms, serve.sh warm 50): 1,288 entries, 783 s (past the
  ~12 minutes: settings kept, as asked), 353 entries with one batch per sample (every k = 8
  direction). In-process cb/blocking median ratios per (dir, k), over cells, modes,
  transports and clients: gmeans 0.82-1.26 with single ratios 0.27-5.13, i.e. inside this
  run's spread; container instrumentation, no conclusion drawn.

## 2026-09-28 -- the stream probe: why the core's client streaming costs more client CPU (coordinator unit)

- Tooling (harness only): bin stream_probe (direction d, k = 1, chosen cells in ONE process,
  rounds interleaved with a rotating order, every call checked; per round the process CPU,
  wall, per-thread CPU through each thread's CPU clock, minor faults, user/system ticks and
  context switches from /proc/self/task, summed by class: caller (the blocking cells'
  caller thread), cell-rt (the cells' own tokio runtimes, now named so in grid.rs), core-rt
  (the core's runtime, tokio's default `tokio-rt-worker`); allocation calls from an
  LD_PRELOAD shim gen/probe/allocprobe.c, all and >= 1 MiB); gen/stream_probe.sh (serve.sh
  server, 4 workers on CPUs 2,3; client on CPU 1; transport pinned), gen/stream_probe.py,
  gen/stream_ab.sh / .py (alternated A/B processes: default build vs a build in another
  target directory). Settings of the A/Bs: 20 rounds x 8 calls per cell and size, 4 warm
  calls, 3 process pairs; the attribution runs 30 x 8. All in logs/rust/opt/stream-probe.
- A/A (aa: 3 pairs, same build): pooled B/A 0.97-1.04 per cell, per pair 0.93-1.10. So a
  difference under about 5% is not resolved.
- Baseline, 16 MiB, k = 1, pinned, pooled over the 6 A/A processes (ms per call, median
  [p10-p90]): A 10.08 [9.08-11.10], Df 10.33, Cf 10.94, Cf-cb 11.26, D 11.85, C 12.42, C-cb
  12.84, B 14.59, Bf 15.70; 4 MiB: A 2.44, Cf 2.55, Df 2.69, Cf-cb 2.72, D 3.03, C-cb 3.16,
  C 3.17, Bf 2.96, B 3.57. System time is the largest part everywhere (A: user ~4.2, sys ~6.1
  ms per 16 MiB call from ticks: the socket writes).
- Copies of each 2 MiB chunk on the client, from the code: A 1 (prost encodes the Vec<u8>
  data into tonic's EncodeBuf); B 3 (prost encode_to_vec, ak_call_send's copy, RawEncoder);
  Bf 2; C and C-cb 2 (the core-ffi encode into the context's buffer, moved by
  ak_call_send_enc(_cb), then RawEncoder's put_slice into EncodeBuf); Cf and Cf-cb 1; D 2
  (core-ffi encode, moved to the host by ak_enc_take_owned, RawEncoder); Df 1. Every cell
  encodes each chunk once from values built before the run (A: prost values, Vec<u8> data;
  C/D: facade values, Bytes data): comparable work; B's fresh Vec per chunk is its definition
  (prost + the core's transport).
- Allocations >= 1 MiB per 16 MiB call (shim): A 8 (tonic's EncodeBuf, one per message), D 8,
  C 10.9, C-cb 9.4, B 23.6, Bf 16, Cf 4, Cf-cb 3.9, Df 2.75 (the context's spare slot returns
  about half the buffers in time). Minor faults per call: 0 for A, C, D; Bf 2,305 and B 693
  (sys 10.7 ms for Bf: its fresh buffers are faulted in, which is why Bf costs more than B),
  Cf-cb 599, Df 192.
- Per-thread split (16 MiB): C: caller 2.97 (the encode) + core-rt 9.47; Cf: 3.71 + 7.26;
  C-cb: cell-rt 3.20 (the encode in the task) + core-rt 9.55; Cf-cb 3.95 + 7.24; A, D, Df:
  all in cell-rt (10.07, 11.82, 10.32). Context switches per call: A 39, D 40, Df 43, C 48,
  Cf 59, C-cb 68, Cf-cb 80.
- Ablations (alternated A/B, 3 pairs each, reverted; patches in the log directory):
  (a) the core's request channel capacity 1 -> 4: WORSE at 16 MiB (B 1.26, C 1.11, C-cb 1.08,
  Bf 1.09, Cf 1.04, Cf-cb 1.00; controls A 1.01, Df 1.01); 4 MiB 0.99-1.04. Reverted.
  (b) tonic BufferSettings on the reference path: not built. The reference path's extra
  cost is RawEncoder's put_slice, which tonic's Encoder API requires; BufferSettings only size
  the buffer and set the yield threshold (each 2 MiB message is over the 32 KiB threshold and
  yielded alone), so they cannot remove the copy; the framed path is the no-copy route.
  (c) the encode context's spare slot as a ring of 3 (ak-rt Enc::take / Recycle): >= 1 MiB
  allocations Cf 4 -> 1.9, Df 2.75 -> 0, C 10.9 -> 7.9; CPU, two sets of 3 pairs: Cf 0.949 /
  0.932, Cf-cb 0.965 / 0.972, Df 0.934 / 0.976, C 1.031 / 0.997, C-cb 1.008 / 1.003, D 1.023 /
  1.007, control A 1.044 / 1.001: a 3-7% gain on the framed cells at the edge of the noise,
  none on the reference ones. A core (ak-rt) change affecting every slice: reverted and
  reported, not kept.
  (d) the callback cells' runtime as a current-thread runtime (no hop between the core's
  completion and a second worker pool): C-cb 0.996, Cf-cb 1.006 (control A 1.009). Reverted.
- Attribution of C (and C-cb) vs A at 16 MiB, about 2.3-2.8 ms per call: the reference
  path's RawEncoder copy, C - Cf = D - Df = about 1.5 ms (user time +2.7 ms, core-rt 9.47 vs
  7.26, +7 fresh 2 MiB buffers); the core's transport hop (host thread -> channel -> core
  runtime), Cf - Df = about 0.6 ms (more system time and context switches: 59 vs 43 per call;
  not isolated further: capacity 4 made it worse, a current-thread host runtime changed
  nothing); Df - A = about 0.25 ms, inside the A/A noise (the core-ffi encode plus framed send
  against prost into tonic's buffer). The callback delivery costs about the blocking one
  (C-cb - C and Cf-cb - Cf 0.3-0.4 ms, under the noise). At 4 MiB the same shape, smaller
  (C - Cf 0.6 ms). What remains unexplained: the split of Cf - Df between wake-ups and the
  channel handoff, and why capacity 4 costs more (not probed). Container instrumentation.

## 2026-09-28 -- stream probe 2: the probe's level, the write pattern, and Cf vs Df measured (coordinator follow-up)

Everything container instrumentation; logs/rust/opt/stream-probe2/. No code change kept.

- Tooling added: stream_probe records per-call CPU inside each round (`cpu_calls`),
  /proc/self/task/*/io per thread class (syscw, wchar, syscr, rchar), AK_PROBE_ORDER=block
  (one (cell, size) at a time, as criterion runs one benchmark) and AK_PROBE_PROC=0 (no /proc
  reads); probe-only cells `Df-chan` (Df's connection and framed body fed as the core feeds
  its own: a host thread encodes each chunk, ak_enc_take_owned, blocking_send into an mpsc(1)
  whose ReceiverStream is the body, the call a task on the cell's runtime) and `Cf-split` /
  `C-split` (Cf's / C's call through the cell's own core client, the host's encode and
  ak_call_send_enc timed per chunk: thread CPU and wall). gen/stream_probe2.sh runs, in ONE
  server session, the grid's own client narrowed (rpc_suite: direction d, k = 1, 10 samples,
  30 ms warm-up, 500 ms measurement) and the probe variants, N iterations; gen/probe/
  h2sniff.py + h2settings.sh (a Unix-socket proxy logging SETTINGS / WINDOW_UPDATE / DATA
  frames); gen/stream_ab.sh takes AK_AB_ENV_B for env-gated ablations in one binary.
- (i) The probe's level. In one session (level/, 4 iterations), 16 MiB A: grid 9.04, probe
  block order 8.83, rotate order 9.14 (4 MiB: 2.26 / 2.24 / 2.33); every cell the same way
  (block within -4% to +2% of the grid, rotate +1% to +11%, mostly on the first call of a
  round). So the probe does not overstate A when measured beside the grid; the 10.08 of
  stream-probe vs the grid's 8.27 / 8.72 compared different sessions, and the container's
  level moved between them (the grid's own A here: 9.04-9.66 over the sessions of this
  unit). Every comparison now carries an in-session grid column; timing uses block order.
- (1) Writes. Per 16 MiB call, on the thread driving the connection: A 1,049-1,051 write
  syscalls, B 1,053, C 1,052, Cf 1,051-1,052, D 1,052, Df 1,050-1,051, Df-chan 1,051, all
  16 KiB each; 4 MiB 266-268 x 15-16 KiB. The write pattern is the same in every cell
  (the server's MAX_FRAME_SIZE is 16,384 and hyper writes a frame per syscall), so it sets
  the system time (about 6 ms per 16 MiB call, 5.7 us per write) but does not separate the cells.
- (2) HTTP/2 settings (settings/h2-settings.log), control only: A, Df, Cf and C-cb send the
  same client SETTINGS (ENABLE_PUSH 0, INITIAL_WINDOW_SIZE 4,194,304, MAX_FRAME_SIZE 16,384,
  MAX_HEADER_LIST_SIZE 16,384) and the same connection WINDOW_UPDATE (+4,128,769); the
  server's are the same to every client. Ruled out.
- (a)/(c) Feed and drive, counted (counts/, an instrumentation build of the framed body,
  patch count-framed.patch): per 16 MiB call Df's body is polled 17 times, 0 Pending, 0 wakes,
  all on the cell's runtime; Df-chan 23-29 polls, 6-12 Pending, 3-6 wakes of the body's task
  by the channel; Cf 17-23 / 0-6 / 0-3 and Cf-cb 21-22 / 4-5 / 2-3, all on the core's runtime.
  The connection's writes are the same count everywhere (above).
- The decisive comparison (chan/, 4 iterations, block order): 16 MiB Df 9.61, Df-chan 10.55,
  Cf 10.31, A 8.95 (grid Df 9.67, Cf 11.44, A 9.66). Df-chan, the harness's own connection
  fed through a host thread and an mpsc(1), costs what Cf costs, with the same thread split
  (host 4.1 ms + runtime 7.0 ms). So Cf - Df is the FEED, not the core's connection, runtime
  or h2 settings. The first unit's "hand-off" attribution by elimination is withdrawn and
  replaced by the direct measurement below.
- (3) The hand-off measured directly (split/, 3 iterations): the host's CPU inside the send
  (ak_call_send_enc / blocking_send) is 7-12 us per chunk (0.06-0.1 ms per 16 MiB call); its
  wall time 406-409 us per chunk on the framed paths (the host waits for the transport) and
  995 us on C's reference path. The host's ENCODE per 2 MiB chunk: C-split 400 us, Cf-split
  491 us, Df-chan 506 us (4 MiB: 401 / 386 / 371). The framed paths' encode costs about
  100 us more per chunk: the transport still holds the previous chunk's buffer when the next
  encode starts, so the context's spare slot is empty and the encode writes into a fresh 2
  MiB buffer (>= 1 MiB allocations per 16 MiB call: Cf 4.1, Df-chan 4.2, Df 2.6, C's own
  encode none extra), whereas on C the RawEncoder copy frees the chunk at once and the spare
  comes back; in Df the encode runs inside the body's poll, after the previous chunk is gone.
- Ablation, the spare slot as a ring of 3 (ring3/, the patch of stream-probe, 3 pairs): host
  encode per chunk Cf-split 502 -> 393 us, Df-chan 526 -> 405 us, C-split 391 -> 386 us; CPU
  B/A: Cf-split 0.880, Df-chan 0.854, Df 0.939, C-split 0.982, control A 0.937 (a noisy
  session: per-pair 0.76-1.09). The encode surplus disappears with recycled buffers; the
  call-level ratio is inside this session's spread. A shared-core (ak-rt) change: reverted,
  reported.
- (b) The 5-byte prefix: the framed body yields each message as TWO body frames, the 5-byte
  prefix Bytes and the message; on the wire that is ONE extra DATA frame per message (4 MiB x 3
  calls: 783 DATA frames framed vs 777 for A and C) and no extra write syscall (the counts
  above are equal: hyper coalesces the small frame into the next write), and 2 polls per
  message, no wake. Ablation (head/, env-gated AK_ABL_HEAD=1, patch abl-head.patch, 402 lines
  incl. the generated length returns): 5 bytes of headroom in every encode context (the codec's
  output view after them: ak_enc_take, the generated encode entries' length, and the moved
  unary / take_owned paths slice it off), the framed stream writes the prefix into it and
  sends ONE frame. Correct: codec pre-check 5,740 checks 0 failures with and without it,
  upload_check PASSED with it, and Cf's DATA frames drop to A's count (777). Timing, 3 pairs,
  B/A: Cf 0.989, Cf-split 0.933, Df (unchanged path) 0.985, C 1.015, control A 1.078 (per
  pair 0.82-1.18): not resolved; one frame in 129 per message and no syscall saved, so no
  effect above the noise was expected. Reverted, reported (shared-core internal change).
- Attribution, 16 MiB k = 1, as far as measured: C - Cf about 1.5 ms (RawEncoder's copy,
  first unit); Cf - Df about 0.7-0.9 ms, of which the fresh-buffer encode (about 100 us per
  chunk, 0.8 ms) is measured directly and removed by buffer recycling, the host's hand-off CPU
  0.06-0.1 ms; Df - A about 0.4-0.7 ms (Df 9.61 vs A 8.95; with the ring Df 10.20 vs A 9.37),
  not attributed: the core-ffi encode plus the framed body (two frames per message) against
  prost encoding into tonic's buffer. Unexplained: Df - A; the per-session level drift.

## 2026-09-28 -- framed default, spare ring, one-frame prefix (owner decision) and their measurement

- Core (2eeab59d): the framed send path is the DEFAULT of every core client (unary and
  streaming, every delivery); ak_client_set_framed(c, 0) selects the reference path.
  ak_rt::Enc gets `head` (0 or FRAME_HEAD = 5) and `take_framed`; every core encode context
  has the 5-byte headroom; the codec's output is the message alone (ak_enc_take views
  after the headroom, the generated encode entries return msg_len(): rust_abi.py), so
  non-RPC users see nothing new. The framed path sends each message as ONE body frame:
  moved entries (ak_call_unary_enc*, ak_call_send_enc*) write the prefix in place;
  copying entries (ak_call_unary*, ak_call_send*) copy WITH the prefix (the one copy they
  make anyway); the reference path slices the prefix off (O(1)). rpc: unary_preframed_cfg,
  client_streaming_preframed_cfg, framed_copy; unary_framed / client_streaming_framed
  (two frames) stay for the harness's Df/Ff. The spare slot is a ring of SPARES = 3 (ring
  size check, one process per size, 2 x 12 rounds: host encode per 2 MiB chunk on Cf-split
  506 / 437 / 396 / 401 us with 1 / 2 / 3 / 4 spares, Df-chan 508 / 441 / 409 / 404).
  Relabelling: the cell names keep their meaning (C = reference path, Cf = framed path);
  the harness now sets the path explicitly on every core cell (ak_client_set_framed(c,
  framed)); tables read Cf as the core's default and C as the labelled reference row.
- Checks (framed-default/checks): generate --check, one_core ok; pre-check 5,740 / 3,257
  checks, 0 failures; crossings 836 / 435 rows identical (ak_client_set_framed is called at
  open, outside the counted call; no entry changed on a measured path); rpc_semantics
  PASSED both builds; upload_check PASSED both builds; header_diff identical headers on both
  transports (the core's framed Push is now 33 DATA frames against the reference's 34, the
  prefix no longer its own frame).
- Session (framed-default/session, 3 iterations, grid b, c, d at k = 1 and 8 + the probe at
  k = 1; tables.md): see the tables; 16 MiB d k = 1 grid: A 9.00, Df 9.23, Cf 9.74, Cf-cb
  10.60, C 13.37, C-cb 12.18; probe: A 8.45, Df 8.88, Df-chan 9.42, Cf-split 10.06,
  Cf-cb-split 9.76. k = 8: Cf 10.67, Cf-cb 12.54, Df 12.97 against A 14.66. Unary b (P2.2
  push) every framed core cell 0.43-0.65 x A; c/P5.4 k = 1 Cf 1.11, Cf-cb 0.97, k = 8
  0.62 / 0.74.
- Per chunk (split cells, 16 MiB): host encode 355-399 us on every path now (C, Cf, C-cb,
  Cf-cb, Df-chan): the fresh-buffer surplus is gone (>= 1 MiB allocations per call Cf 2,
  Cf-cb 2, Df-chan 1.75, Df 0; the reference cells keep tonic's 8). Send-entry CPU 7-14 us.
  Send wall until the host may encode the next chunk: blocking Cf 550 us, callback Cf-cb
  586 us (the completion arrives 36 us later than the blocking return; 4 MiB 118 / 193 us).
  Body polls per 16 MiB call (counting build): Cf 9-16 (8 frames, 0-7 Pending), Cf-cb 12-15
  (3-6 Pending, 2-3 channel wakes), Df 17 (16 frames, two per message, 0 Pending).
- The copying cells (B, Bf, E, Ef and -cb) still allocate a fresh buffer per chunk for
  their copy (8-24 >= 1 MiB allocations per call) and on the framed path those pages are
  faulted in (Bf 256, Bf-cb 1,136, Ef 1,641 minor faults per call): Bf 1.50-1.83 x A.
  Not changed in this unit.
- Stable gate from a clean tree at eb2f204a (framed-default/gate): PASSED (pre-check 5,740 /
  3,257 checks 0 failures, crossings 836 / 435 identical, header_diff identical, 11e upload
  byte check, 11f rpc_semantics, both builds, corpus gates).

## 2026-09-28 -- runtime probe: worker counts, current-thread host, feed depth (coordinator unit)

Everything container instrumentation; logs/rust/opt/runtime-probe/ (tables.md). Hypothesis
under test (coordinator): Cf-cb's (and Df-chan's) extra client CPU over A in direction d is the
per-chunk cross-thread hand-off, made worse by TWO tokio runtimes (cell-rt 2 workers, the
core's ak_runtime_new(2)) on ONE client CPU.

- Harness (kept, defaults unchanged): AK_HOST_WORKERS (N or ct) and AK_CORE_WORKERS read by
  grid.rs, printed in rpc_suite's, run_campaign.sh's and the probe's headers; the probe adds
  AK_CHAN_DEPTH (Df-chan), getrusage deltas per round (ru_nvcsw, ru_nivcsw, ru_minflt per
  call) and a thread count per class after warm-up (checked: h1 halves cell-rt threads, c1
  halves core-rt threads, ct has none). Df-chan cannot run on a current-thread host runtime
  (its host thread blocks in blocking_send while nothing drives the body task): refused with
  an assert, the hct variant runs without it.
- Core experiment (reverted, not committed): AK_CORE_CHAN_DEPTH sets ak_call_open's mpsc
  depth (core-depth.patch), built in target-rtp (deleted after). The patch prints its depth
  on stderr once per process: present in every p1/d2 .err, absent from the unpatched build.
  Checks (checks/checks.log): upload_check PASSED at depth 1, 2, 3, 4 (controls detected);
  rpc_semantics PASSED at depth 1 and 2; codec pre-check at depth 2: 5,740 checks 0 failures.
- Sessions: main/ (base, h1, hct, c1, b1, p1 = patched build at depth 1, d2; 3 iterations,
  7 cells, 12 x 8 calls, block order, no /proc; 396 s) and confirm/ (base, h1, c1, b1; 5
  iterations; A, Df, Df-chan, Cf-cb; 201 s). The session level was higher than
  framed-default's (A 16 MiB 9.9-10.1 ms against 8.45) and every variant, including ones that
  do not touch A's code path (c1, p1, d2), measured A 0.2-1.0 ms under base: so the
  comparison read is the in-process gap (cell minus A of the same process).
- 16 MiB k = 1, in-process gap to A, ms, median over iterations (range), session 1 / session 2:
  Cf-cb: base +1.17 (+0.83..+2.87) / +1.19 (-1.36..+1.27); h1 +1.85 / +1.06; hct +1.81; c1
  +1.01 / +1.30; b1 +1.02 / +1.33; p1 +1.60; d2 +1.63.
  Df-chan: base +1.54 (+0.47..+2.34) / +0.46 (-0.78..+0.88); h1 +0.81 / +1.22; c1 +1.00 /
  +1.19; b1 +1.27 / +0.77; p1 +1.02; d2 +1.16.
  Df: base +0.67 / +0.47; h1 +0.68 / +0.47; b1 +1.31 / +0.38.
  No variant moves either gap beyond the spread; for h1, c1, b1 the two sessions disagree on
  the sign of the change. 4 MiB: every gap within -0.2..+0.4 ms, no pattern.
- Context switches per 16 MiB call (getrusage, voluntary / involuntary): A 27-28 / 3-4 ->
  22-25 / 0 with one host worker; Df 31 / 6-7 -> 20-21 / 0; Df-chan 48 / 16 -> 37-38 / 10;
  Cf-cb 57-58 / 19-20 -> 51-52 / 17-18 (h1), 44-45 / 15 (c1), 39 / 11 (b1); Cf and Cf-split
  40-41 / 13-14 -> 34-35 / 10 (c1). So the switches track the settings (b1 removes about 28
  per Cf-cb call, h1 about 16 per Df call) and the CPU does not follow them: Df's gap to A is
  the same with 37 or 21 switches per call. The probe resolves nothing under about 0.5 ms
  here, so a cost per switch below about 15-30 us is not excluded; it is not measured.
- Current-thread host (hct): Cf-cb's gap +1.81 (one session), Cf-cb-split +0.86, i.e. no
  gain, as the first stream probe's ablation (d) found for the -cb cells.
- Feed depth 2 (d2 vs its own-build control p1): the patch runs (send wall per chunk, 16 MiB:
  Cf-split 622 -> 378 us, Cf-cb-split 665 -> 425 us, Df-chan 604 -> 427 us; 4 MiB Cf-split
  132 -> 31, Df-chan 141 -> 41, Cf-cb-split 236 -> 254 unchanged), the wait moves to the final
  recv (recv wall per call Cf-split 6.01 -> 7.76 ms, Cf-cb-split 5.78 -> 7.89, Df-chan 6.52 ->
  8.67), per-call CPU not changed beyond the spread. Buffers (attribution pass, allocation
  shim, 16 MiB): >= 1 MiB allocations per call p1 -> d2 Cf 1.75 -> 3.00, Cf-split 1.62 ->
  2.75, Cf-cb 1.88 -> 2.50, Cf-cb-split 1.88 -> 2.50, Df-chan 2.00 -> 2.75: the ring of 3
  spares no longer returns every buffer in time at depth 2. Minor faults per call in the
  timed pass (no shim, medians over rounds) p1 -> d2: Cf 0 -> 0, Cf-split 0 -> 0, Cf-cb 95 ->
  128, Cf-cb-split 124 -> 193, Df-chan 0 -> 0 (base 0-124 across cells); in the shim pass
  they scatter 0-505 with no pattern (4 x 4 calls). Depth 3 / 4 not run: depth 2 did not move
  the result. k = 8 not run for any variant (none moved the k = 1 result; budget spent).
- What contradicts the hypothesis: (1) halving either runtime, or both, cuts 16-28 context
  switches per call and no CPU change is resolved; (2) Df, which has no second runtime and no
  channel, keeps a 0.4-1.3 ms gap to A (0.47-0.68 in base and h1) while its switch count drops to A's with one host worker;
  (3) a current-thread host, which removes the host pool entirely, is not better. What is
  left unattributed: the Cf-cb and Df-chan gaps to A (about 0.5-1.5 ms per 16 MiB call in these
  sessions, the run-to-run spread of the gap itself about +-1 ms).

## 2026-09-29 -- the physical-machine probe prepared (coordinator step 2, phase 1: nothing timed)

On the campaign machine (i9-7900X, NixOS, kernel 6.18.54, SMT on, performance governor,
no_turbo 1, scaling min = max = 3.3 GHz, no isolation). Owner decisions relayed by the
coordinator: CLIENT 1-4,11-14 and SERVER 5-8,15-18 (with the SMT siblings), passed as
environment values; main configuration 8 workers everywhere (server, host, core), a variant with 4
everywhere on the same sets.

- Found: `~/.cargo/config.toml` sets `build.build-dir = "/data/csdt/.cargo-build/{workspace-path-hash}"`,
  so target/, target-server/ (and the gate's target-nounk/ ...) of this workspace share one build
  directory and every binary's RUNPATH named its single `libak_core.so`: the last build of any
  variant would decide the core every binary loads. `gen/cargo-shim/cargo` sets the build dir to
  the target dir; serve.sh and the driver use it; ldd checked (each binary loads its own target's
  core). Not changed: the machine's config.
- Gate at cb37633f with the shim on PATH, rustc 1.95.0: GATE PASSED (prep/gate.log).
- Harness: rpc_suite writes getrusage deltas per sample and takes AK_RPC_PAYLOADS; stream_probe
  has `A2` (a second A in the same process, for the A/A gap). Defaults unchanged.
- Driver gen/physical_probe.sh (shared-server mode) and gen/physical_tables.py; smoke with its own
  server (8 workers) of every phase: 18 + 32 + 12 probe rows, 120 + 360 grid rows; the shared-mode
  control (a 4-worker server under the 8-worker preset) aborted before any timing. No figure kept.
- Pools sized from the affinity mask: none in the client or the server (every tokio runtime has an
  explicit worker count; criterion is built without rayon). With 8 workers per runtime, the main
  probe process holds 48 cell-rt and 32 core-rt threads (one runtime per cell), most idle.

## 2026-09-30 -- physical-machine probe, segment 1 (main: 8 workers everywhere), timed

Server: serve.sh start with AK_SERVE_STATE=/tmp/ak-physical-s8.state, AK_CPU_SERVER=5-8,15-18,
AK_SERVER_THREADS=8 (pid 147567; log server-s8/rpc-server.log), serve.sh warm 64 (both sockets,
checked). Client: gen/physical_probe.sh main, AK_CPU_CLIENT=1-4,11-14, host 8 / core 8 workers,
commit f57173ff, clean slice tree. Benchmark wall 242 s (driver total 4 min 7 s incl. build check
and a 4 s attribution pass); no check failed; server alive after. Tables: main-w8/tables.md.

- Main passes, 16 MiB k = 1, probe CPU per call: A 7.95, A2 7.71, Df 8.29, Df-chan 8.50, Cf 8.04,
  Cf-cb 9.25, C 9.73, C-cb 11.33 ms; grid A 7.62, Df 8.54, Cf 8.32, Cf-cb 9.74, C 10.63, C-cb 11.52.
  Grid in-process gaps over 3 passes: Df - A +0.80..+1.02, Cf - A +0.54..+0.75, Cf-cb - A
  +1.97..+2.26; probe Df - A -0.12..+0.93, Df-chan - A +0.15..+0.84, A2 - A -0.43..+0.09.
  k = 8: every framed cell below A (Df - A -3.32..-3.69 at 16 MiB). c/P5.4 k = 1: Df, Cf, Cf-cb
  within +0.01..+0.13 of A.
- Spread passes: in the probe processes holding only A, A2 and Df, A (and A2 in 3 of 4) ran
  10.45-11.57 ms per 16 MiB call, Df 8.28-8.90; in the main probe processes A 7.58-8.27. The
  spread probe's Df - A is therefore -2.02..-3.29 and its A2 - A range 3.21 ms (16 MiB); at 4 MiB
  ranges 0.20 / 0.45. Grid spread pass 1 has A d/16MiB k1 11.29 against 7.47-7.95 in passes 2-4
  (Df - A range 3.90 with it). Not investigated in this segment.
- Context switches per 16 MiB k = 1 call (grid, voluntary): A 70, Df 240, Cf 97, Cf-cb 262, C 164,
  C-cb 285; involuntary 0-0.2 everywhere. Minor faults per call are 0 at k = 1 except Cf-cb (99
  grid, 188 probe); at k = 8 A 1,711, C 1,746, C-cb 1,583, Cf-cb 346, Cf 125, Df 0.2 (16 MiB).
- Attribution pass (16 MiB, per call): host encode per chunk 239-267 us (Cf-split, Cf-cb-split,
  Df-chan); >= 1 MiB allocations A 8, C 8, C-cb 8, Df 0, Df-chan 1.25, Cf 1.62, Cf-cb 1.38.

## 2026-09-30 -- physical-machine probe, segment 2 (variant: 4 workers) and the client-only control

s8 server stopped (pid 147567); s4 started (AK_SERVER_THREADS=4, 5-8,15-18, pid 152128, state
/tmp/ak-physical-s4.state), serve.sh warm 64, left running. variant-w4: host 4 / core 4, 3
passes, benchmark wall 115 s (+2 s attribution). variant-c8: host 8 / core 8 against the same
server, 2 passes, 57 s (+2 s). No failed check. Tables: variant-w4/tables.md, variant-c8/tables.md.

The level of A in main-w8's spread passes, from the jsonl and progress.txt (no extra run):
- Probe, 16 MiB, per-process median of A (A2): spread-probe-1..4 11.42 (9.03), 11.57 (11.24),
  10.45 (11.27), 10.77 (11.22); Df in the same processes 8.28-8.90. Main probe processes A 7.58-8.27,
  A2 7.66-8.06. The raised level holds from the first to the last round of each block (e.g.
  spread-probe-2 A round 1 12.04, last 10.90). Wall per call raised with it (A 10.49-12.08 against
  8.27-8.83). 4 MiB is not raised in those processes (A 1.91-2.00).
- Time is not the separator: spread passes alternate probe and grid, and spread-grid-2 (+22-38 s),
  -3 and -4 measured A d/16 k1 7.47-7.95 between the raised spread-probe-2 (+38-42 s) and
  spread-probe-4 (+76-79 s).
- The one raised grid benchmark is spread-grid-1's A d/16MiB k1 (+6-22 s, the session's first
  grid process): all 10 samples 10.78-11.66, against 7.83-8.44 in spread-grid-2; every other
  benchmark of that process (A at 4 MiB, 16 MiB k8, c/P5.4; every Df) at its later level.
- Process composition: the spread probe processes held 3 host runtimes (24 cell-rt threads), no
  core runtime, no caller thread; the main probe processes 48 cell-rt, 32 core-rt, 6 callers.
  variant-c8's probe processes (A, A2, Df, Cf: 3 host runtimes of 8 workers plus one core client)
  measured A 7.61 / 7.88 and A2 7.55 / 7.49. variant-w4 has no spread passes; its first probe and
  grid processes measured A 7.83 and 7.71 (CPU); the first probe's wall was higher (A 9.89, Df
  11.05) than its later passes (8.74-9.00).
- Server: server CPU ticks per probe process (progress.txt, 10 ms ticks, 290-850 per process)
  per 16 MiB uploaded, warm-up calls included: spread probes 11.5, 9.5, 9.5, 9.3 ms; main probes
  10.0-10.1 ms; variant-w4 12.4 (first), 10.2, 10.1; variant-c8 9.9, 9.8. The raised client CPU is
  not accompanied by raised server CPU per byte.
- Not measured: per-thread CPU in the timed passes (no /proc reads there), so which client thread
  carries the raised A time is not known.

## 2026-09-30 -- attribution of Cf / Df against A (owner's goal 1) and the shared-runtime cells (goal 2); physical machine

Coordination: every core change in a private worktree (scratchpad wt-rust), every timed or
profiled run under `flock /tmp/ak-physical-bench.lock`, builds on CPUs 0,9,10,19, own server
(serve.sh, 8 workers, 5-8,15-18) per session, client 1-4,11-14, 8 workers everywhere.

- Tooling: stream_probe gained k (batches of k in flight), direction c (P5.3/P5.4), perf control
  (AK_PERF_CTL, AK_PERF_CELL: perf counts one cell's timed rounds), thread pinning knobs, cells
  A2, Ff-1f, Df-1f (patched core), Cn-1rt, Cf-cb-1rt (patched core); gen/attrib.sh (perf stat,
  perf record dwarf, /proc, strace), gen/perf_classify.py (leaf crates, libc callers, kernel
  buckets with the booted System.map and the KASLR offset, the C++ agent's method
  reimplemented), gen/inproc.sh + inproc_tables.py (in-process comparisons, no perf),
  gen/patch_checks.sh (pre-check both builds, upload_check, rpc_semantics), allocprobe AKP_BT
  (stacks of >= 1 MiB allocations) + gen/probe/akp_bt.py.
- A's d/16 cost is bimodal and set by glibc malloc: tonic's EncodeBuf allocates a fresh 2 MiB+
  buffer per message; when glibc trims the arena after the free, the next one faults in again
  (1,200-2,200 faults per call, 35-47 M cycles against 27-31 M). Static thresholds
  (GLIBC_TUNABLES, attribution only) remove it and put A level with Df/Cf/Ff (logs/rust/opt/
  attrib/{base,tunables,inproc-alloc}). The mode flips between processes with no change to A's
  code (main-w8's raised spread passes were this mode).
- Every Rust cell makes about 1,030 writev per d/16 call (h2 writes each 16 KiB DATA frame);
  the kernel's socket write is 66-76% of every cell's cycles; skb page zeroing
  (CONFIG_INIT_ON_ALLOC_DEFAULT_ON=y) about 2.7 M cycles per call in every cell.
- Cf's residual over fault-free A at d/16 k=1 (+0.26..+0.63 ms): copy_from_user in the socket
  writes, 5.93 M cycles against A's 4.02 (the caller thread encodes, a core worker writes).
  Pinning test (attrib/pin-locality): caller and worker on one CPU 7.63 ms, level with A; two
  cores 7.85; free 7.95; SMT siblings 8.98.
- p1-ring (patches/p1-ring): Enc::take_all allocates a fresh 4 MiB buffer (the doubled capacity)
  1.6-2.0 times per 16 MiB call in Cf and Cf-cb (ring of 3 too small, try_lock losses); a ring
  of 6 with a blocking lock: 0 fresh buffers, 0 faults; Cf-cb d/16 k=8 10.62 -> 9.53 ms.
- p2-take-framed (patches/p2-take-framed, on p1): additive ak_enc_take_owned_framed; Df-1f and
  the harness-only Ff-1f send one body frame per message: d/16 k=1 Df 8.40 -> 7.70, Ff 8.35 ->
  7.70, switches 236 -> 88; the harness's two-frame body was Df/Ff's residual.
- Goal 2: p3-exec-slot (patches/p3-exec-slot, on p1 and p2): ak_runtime_new_hosted /
  ak_task_poll / ak_task_free / waker vtable; the core keeps one ak-reactor driver thread.
  Cf-cb-1rt costs more than Cf-cb (d/16 k=1 9.73 against 8.80 ms, 533 against 216 switches;
  the reactor thread 1.58 ms per call waking host tasks). Cn-1rt (rpc crate + core-native on the
  host runtime, harness only) 8.39 ms, 62-143 switches, level with Cf; c/P5.4 1.91 against A 1.94.
  Checks pass on every patch (pre-check 0 failures both builds, upload_check, rpc_semantics, also
  with every core client hosted).

## 2026-09-30 -- p4 closed; the enc / encp / zero-copy track

- p4 with the host on the patched h2 too (patches/p4-h2-coalesce/host-too): the knob reaches both
  copies (writes per call on each cell's writing thread 1,041-1,048 at N=1, 129-158 at N=16);
  every cell drops about 1.8-1.9 ms per 16 MiB call and Cf's gap to A returns (+0.32..+1.11 at
  d/16 k=1); Df-1f and Ff-1f stay level with A. p4 closed (owner: not the target).
- p5 on crates.io h2 is the target track. First zero-copy timing (patches/p6-zero-copy/inproc,
  Cf-zc 8.26 ms at d/16 k=1): it came from the FIRST p6 build, which recorded blobs as external
  only on the transcoder path; M5's payload reaches the core as a DIRECT argument, so that build
  still copied it (perf record: memmove under ak_core::enc_blob on the caller thread,
  enc-track/attrib-base). The direct path now uses put_blob; p6-zero-copy.patch and the stack
  file were updated (STACK.txt says so) and every later Cf-zc figure is from that build.
- p7-deferred-zc: ak_call_send_deferred_zc, the deferred encode (p5) on a zero-copy context,
  cells Cf-zcp (wait 0) and Cf-zcw (wait 1). Checks pass (pre-check, upload_check,
  rpc_semantics, SHA on pinned and shipped at k 1 and 8).
- In-process, crates.io h2, pinned allocator (enc-track/inproc-1): d/16 k=1 A 7.65, Cf 8.31,
  Cf-enc 7.70, Cf-encp 7.81, Cf-zc 6.71 (wall 7.90), Cf-zcp 6.63 (wall 7.75); d/16 k=8 A 9.23,
  Cf-zc 6.43, Cf-zcp 6.36 (wall 7.96 against A 8.88). Attribution (enc-track/attrib-2): zero copy
  removes the encode memcpy (user cycles 2.6 M against 8.9-9.0), copy_from_user 4.9 M (A 4.3 hot,
  Cf 5.7 cross-CPU), cache misses 266 k against 430-490 k.
- perf_classify.py already puts clear_page_erms under the socket write path in "socket write:
  zeroing new skb pages" (not page faults) since the first System.map run.

## 2026-09-30 -- consolidated run (physical-probe/opt-stack)

HEAD core against the stack (p1+p2+p3+p5+p6+p7, crates.io h2, AK_SPARES=6 AK_SPARE_LOCK=1) and the
same core with p4 (N=16) as a separate row; pinned allocator, 3 processes, 286 s; default
allocator, 1 process, 61 s. d/16 k=1 CPU (ms): A 7.70-7.83; head Cf 8.16, Cf-cb 9.40, Cn-1rt 8.54,
C 9.95; stack Cf 8.44, Df-1f 7.90, Cf-encp 7.66, Cf-zc 6.54, Cf-zcp 6.35, Cf-zcw 6.34 (wall 7.66-7.91
against A 8.80); stack-p4 Cf 6.49, Cf-zc 4.78. d/16 k=8: A 9.39-9.53; stack Cf-zc 6.50, Cf-zcp 6.41.
c/P5.4 k=1 (no zero-copy cells): head and stack Cf 2.00-2.02, A 1.96-1.98; stack-p4 Cf 1.60.
Default allocator: A 11.65 with 1,902 faults per call in the stack process, 7.81 with 0 in the head one.

## 2026-09-30 -- stability campaign (task B) and the Cf-cb gap (task A)

- Machine: the owner confined non-benchmark work to 0,9,10,19 (system.slice, init.scope,
  machine.slice AllowedCPUs; IRQ affinity; taskset on user processes); gen/machine_header.sh reads
  it into every inproc.sh header (cgroup cpusets, IRQ affinity lists, non-kernel threads allowed
  on the benchmark CPUs, runnable threads there).
- B (physical-probe/stability, commit 6d47eb91): 28 alternating repetitions of HEAD and the stack,
  5 workloads, pinned allocator, one lock hold, 858 s. d/16 k=1 per-process gap to A over 28
  processes: HEAD Cf +0.37 (p10-p90 -0.04..+1.06), HEAD Cf-cb +1.22 (+0.81..+1.89), stack Cf-zc
  -1.65 (-2.15..-1.22), Cf-zcw -1.57; A2 - A +0.06 (-0.45..+0.35, min..max -0.75..+0.69). No
  drift beyond the floor (first against last third: A 7.79 / 7.67).
- A (cb-track, patches/p8-cb-inline, patches/p9-cb-at-take): Cf-cb's extra cycles and switches are
  on the core runtime's workers (220-276 voluntary switches per d/16 call against Cf's 102-115,
  futex 92 against 43 per call; host workers 18); pipelining unchanged (send wall 344 us per chunk
  in both). p8 (inline completion when the channel has room; channel depth knob): no change.
  The core worker count moves it: Cf-cb d/16 k=1 8.94 / 8.65 / 8.10 ms at 8 / 2 / 1 core workers
  (gap to A +1.10..+1.79 / +0.46..+0.87 / -0.09..+0.27), Cf 8.50 / 8.22 / 7.98; per-call cycles
  28.5 against 24.7 M (8 against 1 worker: epoll_wait +1.4 M, skb alloc +0.8, tokio user +0.5,
  futex/scheduling/other +0.8). p9 (a callback send queued with its completion, delivered when the
  body takes the message; no task per send): Cf-cb 9.20 -> 8.70 ms at 8 core workers, switches
  207 -> 125; at 1 core worker 8.15 either way.

## 2026-09-30 -- measurement note: the core worker count, container against the campaign machine

Two measurements disagree, and no cause is claimed here:
- Container runtime probe (logs/rust/opt/runtime-probe, 2026-09-28): 1 core worker (variant c1)
  against the default did not change client CPU beyond the session spread. d/16 k=1 in-process
  gap to A, session 1 / session 2: Cf-cb base +1.17 / +1.19, c1 +1.01 / +1.30; Cf 11.04 (base) and
  10.50 (c1) ms, A 10.08 and 9.62, spread of a gap about +-1 ms.
- Campaign machine (patches/p8-cb-inline/inproc-depth-workers, p9-cb-at-take/inproc,
  cb-track/core-workers-*, 2026-09-30): Cf-cb 8.94 / 8.65 / 8.10 ms at 8 / 2 / 1 core workers
  (gap to A +1.10..+1.79 / +0.46..+0.87 / -0.09..+0.27), Cf 8.50 / 8.22 / 7.98.
What differs between the two setups (from their headers):

| | container runtime probe | campaign machine |
|---|---|---|
| machine | Xeon VM, 2.80 GHz, frequency and SMT not controlled or recorded | i9-7900X, 3.3 GHz fixed (governor performance, no_turbo), SMT on |
| client CPU set | 1 logical CPU (AK_CPU_CLIENT=1): every client thread on one CPU | 8 logical CPUs, 4 cores with their siblings (1-4,11-14) |
| server | CPUs 2,3, 4 workers | 5-8,15-18, 8 workers |
| core workers compared | 2 (default then) against 1 | 8 (main configuration), 2, 1 |
| host runtime workers | 2 | 8 |
| allocator | glibc defaults | static thresholds (GLIBC_TUNABLES) |
| core | HEAD of 2026-09-28 (ring of 3, framed default) | stack p1-p9, ring of 6 with the blocking lock, p8/p9 knobs off unless stated |
| cells | A, Df, Df-chan, Cf, Cf-cb, Cf-split, Cf-cb-split; d/16 and d/4, k = 1 only | A, Cf, Cf-cb, Cn-1rt (and Cf-zc); d/16, d/4, c/P5.4, k = 1 and 8 |
| method | bin stream_probe, one process per variant, variants alternated, 3 and 5 iterations | gen/inproc.sh, all conditions alternated, 3 processes per condition |
| isolation | container, none recorded | taskset; from 2026-09-30 evening non-benchmark work confined to 0,9,10,19 |

## 2026-09-30 -- core worker count under concurrency (cb-track/workers-sweep)

AK_CORE_WORKERS 1, 2, 4, 8 x p9 off / on, host 8 workers, stack binary, pinned allocator, 3
processes per condition, affinity checked around every process (all 96 high-k processes passed).
Cells A, Cf, Cf-cb, Cf-zc, and at k >= 16 also Cf-m4, Cf-cb-m4, Cf-zc-m4 (4 core clients, each
its own runtime: 15 core runtimes per process at k >= 16, so 15 or 120 core worker threads).
- One connection saturates at about 120 calls/s (wall about 8.2-8.4 ms per 16 MiB call) at k = 8,
  16 and 32 for every core worker count; A about 110-113 calls/s. Four connections: 300-310
  calls/s (3.2-3.3 ms), CPU per call 13.3-14.5 ms (about 4.2 of the client's 8 logical CPUs busy).
- d/16: 1 core worker costs no wall or throughput at any k measured; it saves CPU at k = 1
  (Cf-cb 8.00 against 9.18 ms, Cf 8.14 against 8.31 with p9 off), 0.1-0.3 ms at k = 8-32 for
  one connection, and for Cf-cb-m4 0.2-0.8 ms per call at k = 16.
- c/P5.4: at k >= 16 with 4 connections, 1 core worker per client costs 4-6% of throughput
  (Cf-cb-m4 747 against 778 calls/s at k = 16, 825 against 874 at k = 32; Cf-m4 742 against 778)
  and saves 0.13-0.33 ms CPU per call; one connection shows no throughput cost.
- p9 at 8 workers: lowers Cf-cb's switches (d/16 k = 1 210 -> 171 here) and CPU by 0.1-0.4 ms at
  d/16; at c/P5.4 no gain.

## 2026-10-01 -- TCP listener; UDS against TCP loopback from the Rust host

- rpc_server --tcp / serve.sh AK_SERVER_TCP: opt-in 127.0.0.1 listener, pinned configuration,
  TCP_NODELAY set on accept (verified by strace of a verification instance; the client side
  sets it too: tonic's Endpoint default, and ak-core's ak_client_new_opts keeps it unless
  tcp_nagle = 1). Checks in logs/rust/opt/tcp-listener.
- physical-probe/tcp-vs-uds (gen/tcp_vs_uds.sh): tonic A inflates on TCP like the core cells.
  d/16 k=1 process CPU UDS -> TCP: A 7.69 -> 9.93, Cf 8.41 -> 10.49, Cf-zc 6.39 -> 8.59, Df-1f
  7.52 -> 9.88; wall A 8.84 -> 14.93, Cf 8.13 -> 13.46. Gaps to A keep their sign and size on TCP.
  Every cell writes about 1,035 16 KiB writes per 16 MiB call on both transports (h2).
- CPU accounting on TCP: the loopback receive path runs in softirq on the client's CPUs and,
  with CONFIG_IRQ_TIME_ACCOUNTING=y, is not in CLOCK_PROCESS_CPUTIME_ID: A on TCP 10.09 ms
  process CPU against 15.47 ms perf task-clock and 50.6 M cycles (15.3 ms at 3.3 GHz); on UDS
  8.04 against 8.12 ms. Sampled cycles on TCP: loopback receive path about 18-20 M per call,
  28% of samples in loadable modules (netfilter: nf_tables, nf_conntrack, ...).

## 2026-10-01 -- h2 PR 903 (hyperium/h2#903), alone and combined with p4

Task (owner, through the coordinator): test PR 903 ("perf: allow multiple DATA frames per write")
on the current stack, core-only and host-too, against stock h2 and p4, UDS and TCP.

- PR head a1f880bc6b11d71c2880240117eb569db7ee3d0e, 8 commits, base dbc204e (first tag v0.4.14),
  Cargo.toml 0.4.13. hyper 1.11.1 requires h2 >= 0.4.14, tonic 0.14.6 "0.4"; the lock has 0.4.19.
  Ported onto v0.4.19: 38 commits apart, conflicts in framed_write.rs (the #921 WriteZero check)
  and prioritize.rs (#918's buffer_pending/BufferStatus). Kept 0.4.19's structure, took the PR's
  framed_write.rs, its reclaim_frames and in_flight_partial_send.
- First port defect, found by h2's own tests: two client_request tests spun forever in poll_ready
  (trace: flush_inner(false) looping with nothing to write). Cause: #918's has_send_capacity mapped
  to the PR's queue-slot check; 0.4.19's recv-side loops (send_pending_refusal, window updates)
  buffer only non-DATA frames after poll_ready, which guarantees buffer room only. Fix (221c21e):
  has_send_capacity = buffer room, has_data_send_capacity = queue slot, used only by
  prioritize::buffer_pending. After it the port's suites equal stock v0.4.19's (lib: 437 pass, the
  same 1 failure as stock; 227 integration tests alone: all pass but hammer, which times out in
  debug on stock, on the original PR and on the port, and passes on the port in release).
- Background commands in this environment did not see the scratchpad (and #!/bin/bash does not
  exist on NixOS): the per-test runner failed silently twice; fixed with /usr/bin/env bash and
  detached `setsid nohup` launches.
- Correctness on the PR core: pre-check 5740 / 0 failures, upload_check, rpc_semantics,
  burst_check PASSED; host-too upload_check and rpc_semantics PASSED (h2_pr903_checks.sh).
- Is it running: write counts (UDS, untimed). k=1: unchanged, about 1,040 writes per d/16 call for
  Cf and A, core-only and host-too; k=8: Cf about 530 (core-only), A about 490 (host-too only).
  The unported head (version bumped to 0.4.14, compiled into a core for this check only) gives the
  same counts as the port, so the k=1 result is the PR's design, not the port: a frame cut from a
  larger chunk sets in_flight_partial_send, is_send_ready() is false until the codec has written it
  and reclaim_frames pushed the remainder back, so one stream has one partial frame per write.
- The machine was suspended 02:00:53 to 06:53:46 local; timing held until the coordinator
  confirmed the IRQ re-pin. Session 1 (05:00:50Z to 05:07:09Z, 378 s): stock, p4 N=16, PR
  core-only, PR host-too x UDS, TCP, 3 processes each, workloads d16k1 d16k8 d4k1 c54k1 c54k8,
  cells A Cf Cf-cb Cf-zc. Probe now records the inherited perf task-clock (softirq included; on UDS
  equal to the process clock within 1%), server task-clock per call (9 server threads at start
  and end of every round), /proc/self/io write and read syscalls.
- Owner: combine PR 903 with p4 (tree b871798): prioritize hands out up to N x max per pop; a
  split element carries its sub-frame heads (first in the shared buffer, the rest in the element)
  and the Buf walk interleaves heads and payload windows inside the PR's batched writev; one
  queued element per stream in flight, now up to N frames. h2 suites at N=1 equal stock; at N=16
  one more failure, stream_states::send_err_with_buffered_data (queued sub-frames go out before
  the RST of a stream reset mid-send); p4 alone at N=16 fails the same test (checked on a v0.4.19
  git tree with the p4 patch). Checks at N=1 and 16 PASSED; burst_check under strace at N=16 shows
  partial writev returns (729 of 4,261 large writes) and writes of 64 to 168 iovecs.
- Rule breach: one debug `cargo build` of the combined h2 ran without the bench lock (pinned
  0,9,10,19), 05:09:08Z to 05:09:17Z: after session 1; it overlapped the C++ agent's session h2,
  its processes 0065 to 0071 (round 1, d4k1 and c54k1).
- Session 2 (372 s): stock, p4, PR core-only, PR+p4 N=16, core-only, UDS and TCP, same cells.
- Measured (task-clock ms per call, median, sessions 1 / 2): d/16 k=1 TCP Cf stock 15.19 / 15.13,
  p4 8.03 / 7.47, PR 17.08 / 17.15, PR+p4 8.17; d/16 k=8 TCP Cf stock 15.62 / 15.35, p4 8.90 /
  9.06, PR 11.72 / 12.57, PR+p4 8.81 (TCP d/16 k=8 is bimodal by process for every condition:
  per-process gaps to A span up to 13 ms). The PR alone costs more client CPU than stock at the
  same write count at k=1, on UDS and TCP and on d/4 and c/P5.4 (about +0.5 ms per 4 MiB call).
- Attributed (perf record, d/16 k=1 UDS, Cf's timed rounds, record2/): 18% of the PR core's client
  cycles are in its poll_write_buf, and about 92% of that function's samples are the eight-store
  loop that initialises `[IoSlice::new(&[]); 1024]` on every call (16 KiB of stores per write,
  about 1,040 writes per call).
- C++ agent's finding: the patch file h2-pr903-on-0.4.19.patch (sha256 4aab4234...) did not
  rebuild the measured core. Confirmed for that file (written 01:30 local, before the fix commit at
  01:37:30); it had already been regenerated at 06:59 local (sha256 d1409e5b..., = git diff
  v0.4.19 221c21e, applied to a fresh v0.4.19 it gives a tree identical to 221c21e). Each HOWTO
  now records the tree commit, the build time and the patch sha256.
- Syscall census (strace -f -c, untimed, h2-pr903-p4/strace/): on UDS stock spends about one
  epoll_wait per writev (d/16 k=1 Cf: 1,069 writes, 1,017 epoll_wait per call); p4 and PR+p4 cut
  both (127/82 and 130/63); on TCP epoll_wait is about 30 per call for every condition.

## 2026-10-01 (evening) -- owner decisions: p1 landed, h2-batch variant, TCP only, TCP worker sweep

- Decisions relayed by the coordinator: h2 has two variants (stock = crates.io 0.4.19, the default;
  h2-batch = the PR 903 port + p4, AK_H2_COALESCE=16); zero copy (p6, p7) and p2, p3, p5, p8, p9
  are dropped; p1 is kept in the core; TCP only for every benchmark; Docker keeps running
  (netfilter recorded as a machine condition).
- p1 landed in poc/codec/crates/ak-rt/src/enc.rs (82f3712a): SPARES 3 -> 6 and `lock()` in place
  of `try_lock()` on the buffer's return. Constants, because the core had no experiment knobs (the
  only env read is the AK_RPC_TRACE diagnostic); the C++ queue cell's 24 is recorded at the
  constant. Full gate PASSED on the working tree before the commit (logs/rust/opt/p1-landed).
- h2-batch: tree 4861bb0 = b871798 + "AK_H2_COALESCE defaults to 16". The committed patch is the
  src/ part of git diff v0.4.19 4861bb0 (the h2-support mock change is test-only and not in the
  crate); the crates.io .crate was packaged from d57d1b8 and its src/ equals the tag's.
  poc/codec/h2-batch/build.sh materialises h2 from the cached .crate (sha256 checked against
  Cargo.lock), patches it, builds with --config patch.crates-io.h2.path and restores the
  Cargo.lock cargo rewrites. Both cores built by it and checked (pre-check 0 failures,
  upload_check, rpc_semantics, burst_check); an untimed TCP smoke shows 78 writes per d/16 Cf
  call on h2-batch without any env (the default is in effect), 1036 on stock.
- gen/inproc.sh: TCP by default (AK_IP_TRANSPORT), base env per process (task-clock, server
  task-clock, target, TCP_NODELAY check), netfilter modules in the header; the two historical
  drivers that alternate transports per condition pin AK_IP_TRANSPORT=uds.
- TCP worker sweep (gen/tcp_sweep.sh, logs/rust/opt/tcp-sweep; low 310 s, high 578 s): stock and
  h2-batch cores built by poc/codec/h2-batch/build.sh, AK_CORE_WORKERS 1/2/4/8, cells A, Cf, Cf-cb
  (and Cf-m4, Cf-cb-m4 at k >= 16), d/16 and c/P5.4 at k 1/8/16/32, 3 processes each. Measured
  (task-clock ms per call, medians): d/16 k=1 Cf stock 14.73 (1 worker) to 15.31 (8), h2-batch
  7.32 to 7.86; Cf-cb stock 14.73 to 15.81, h2-batch 7.49 to 8.74 (no monotone order); A 14.86 to
  15.07 in every condition. Voluntary switches grow with the core worker count (Cf stock 10 ->
  46 per call, Cf-cb 21 -> 78). One connection gives the same wall per call at k 1 to 32 (about
  13 ms stock, 8 to 9 ms h2-batch per 16 MiB: about 1.3 and 2.0 GB/s); four connections (-m4)
  4.4 to 4.8 ms stock, 3.5 to 3.9 h2-batch. Server task-clock per d/16 call at k >= 8: stock about
  21 to 23 ms, h2-batch about 14. c/P5.4: h2-batch Cf about 1.8 to 2.3 against stock 3.8 to 4.1.
  TCP k >= 8 rows are bimodal by round (p10 near 9 ms against a median near 15 for A and stock
  cells), as in the PR 903 sessions.
- Machine: a second suspend, 19:53:03 to 20:36:33 local, despite the inhibitor; the IRQs were not
  re-pinned before the gate, the checks and the sweep (10 on 0-19; eno1 effective on CPU 3, about
  7.5 interrupts/s measured after the sweep). Noticed only after the sweep, from the header's irq
  line; recorded in tcp-sweep/NOTES.txt.
- After the owner's re-pin (verified: 47 IRQs on 0,9-10,19, 2 on 0-19; `sleep infinity` inhibitor):
  the TCP sweep re-run into logs/rust/opt/tcp-sweep (308 + 578 s); the first run renamed
  tcp-sweep-unpinned-irqs. Medians of the two runs agree within the run-to-run spread (e.g. d/16
  k=1 Cf stock 14.63 to 15.26 against 14.73 to 15.31 ms; h2-batch 7.00 to 8.22 against 7.32 to
  8.13).
- Response-delivery comparison (owner): new probe cells A-blk, A-cb, A-q, Cf-q (stream_probe
  `delivery_batch`, a Caller::Batch: one caller thread per cell issues each batch of k and waits by
  the cell's delivery). Checks: every new cell passes the server's byte count and SHA-256 (d) and
  the response length (c) on stock, h2-batch and host-too; the plant control (AK_PROBE_PLANT=1,
  one byte more expected) first PASSED on A-blk: A-blk was built from build_call("A"), which
  never sees the plant (the control did its job: the cell's expectation was not the one under
  test). Fixed (A-blk, A-cb and A-q build cell A's call through one function, a_call) and every
  plant then failed with the planted error. Timed: sessions a (112 s) and cf (227 s), TCP, 3
  processes per condition. Raw (task-clock ms per call, medians): d/16 k=1 A stock 15.0 to 15.3 in
  every A form, host-too 7.7 to 8.0; Cf / Cf-cb / Cf-q stock w8 15.33 / 15.83 / 15.56, h2-batch w8
  7.98 / 7.82 / 7.93; with 1 core worker stock 14.70 / 14.72 / 14.53, h2-batch 7.82 / 8.10 / 7.90.

## 2026-10-02 -- WP12: the full gate on both h2 variants of the core (container)

Task (coordinator): close WP12's "both variants pass both slices' gates" for this slice: the gate that
produced opt/p1-landed/gate.log, both builds, once on the stock core and once on the h2-batch core
(build.sh, AK_H2_COALESCE default), with proof the variant is in effect, the RPC checks over TCP, and a
write-count marker. Container: no timing claims.

- Linkage: ak-core is `crate-type = ["cdylib", "staticlib"]` and every harness/campaign binary links the
  cdylib (DT_NEEDED libak_core.so, RUNPATH <target>/release/deps, no DT_RPATH, no SONAME). So no gate
  step links the core statically and library substitution covers every step. The obstacle is
  `cargo run` (gate steps 3-6, 10, 11c-11f, 12): cargo prepends its own deps directory to
  LD_LIBRARY_PATH, which would hide a substituted core. A second obstacle: the gate's target
  directories compile 11 different ak-core feature sets (count, global-widths, pad-widths, guard off =
  `rpc` only, corpus, no-unknown, ...), so a substituted core must match each one.
- Built: gen/wp12-shim/cargo (lock every build; after each build read ak-core's feature set from a
  no-op `--message-format=json` rebuild; gen/wp12_core.sh builds that set for the variant;
  <target>/release/ak-variant -> it; `cargo run` = build + exec), LD_LIBRARY_PATH='$ORIGIN/ak-variant:
  $ORIGIN/../ak-variant' (glibc expands $ORIGIN per executable in LD_LIBRARY_PATH: checked with ldd
  first), gate.sh's step() exporting LD_DEBUG=libs and a per-step LD_DEBUG_OUTPUT, gen/wp12_loads.py
  (the summary), gen/wp12_gate.sh (the driver: header, gate, TCP checks, marker, loads). The check bins
  upload_check, rpc_semantics and header_diff take AK_CHECK_TRANSPORT=tcp (server::spawn_check_server,
  an in-process serve_tcp), burst_check takes AK_RPC_TCP=HOST:PORT; defaults unchanged.
- build.sh limits met on the way (shared core, not changed; for the aggregating session): it passes
  `--features` only, so it cannot build a no-unknown core (unknown-fields is a default feature); and its
  last pipeline (`strings | grep framed_write.rs`) exits non-zero under `set -e` for a core holding no
  h2 (the corpus core, which has no rpc), so build.sh fails although the build succeeded. wp12_core.sh
  uses build.sh where it can and the same cargo command by hand (same --config, same h2-batch-src,
  Cargo.lock restored) otherwise; each core is then checked fresh with exactly the wanted features.
- Tooling defects found and fixed before the counted runs: (1) the loads parser took the first
  "initialize program" of a pid (an exec chain env -> bash -> cargo -> binary keeps the pid and the
  LD_DEBUG file), and "needed by" lines are not in the `libs` category; fixed by splitting images at
  the dynamic linker's own init. (2) The first stock run failed at gate step 11 (corpus) because
  wp12_core.sh called build.sh for the corpus core (build.sh's failing last line, above); fixed. The
  preliminary runs' logs were deleted; the counted runs are from the committed driver (e3f6afe8).
- Runs (e3f6afe8, clean tree; the plain run after the C++ agent's d5e23249, poc/cpp only): h2-batch
  GATE PASSED, stock GATE PASSED, plain GATE PASSED; LOADS CHECK PASSED in each (2,166 processes loaded
  a core, one each, the intended one); TCP checks PASSED in each (upload_check, rpc_semantics 72 cases,
  header_diff full; upload_check, rpc_semantics no-unknown; burst_check UDS and TCP).
- Marker (d/16 MiB, k=1, TCP, write syscalls per call, 3 rounds): Cf 77.8-78.0 on h2-batch,
  1,035.0-1,039.5 on stock, 1,039.2-1,041.0 plain; A (host tonic) 1,033.8-1,039.8 in all three.
- Cores: full h2-batch e8073eec..., no-unknown h2-batch 0ff0580d..., full stock 9e4eaa7d..., no-unknown
  stock 00de6be8...; the corpus cores (no h2) are byte-identical across variants.
- D16: no step failed on h2-batch only. The passing cancel checks look at status and the connection
  afterwards, not at the DATA frames delivered after the reset, so they neither show nor exclude it.
- Not covered: gate step 2 builds ak-core without rpc (no h2 in it on either variant); the 1.88 floor
  and the campaign machine on h2-batch.

## 2026-10-02 -- the owner's backward-encode experiment (container; patch only)

Task (coordinator, the owner's idea): encode backward as upb does (body, then length, then key; no learned
width, no placeholder, no prefix move), keep the ABI and the bytes, measure every encode payload family.
Mid-task the owner changed the design: drop the core-side reordering of call blocks; the HOST delivers each
repeated field last to first. Everything in logs/rust/opt/patches/backward-encode/ (README).

- Built in a private worktree of 1c9d8981: ak_rt::BEnc (downward buffer, positions as distances from the end,
  geometric grow copying the content to the end, head room below the message for take_framed, the ring of 6
  unchanged); ak-core on BEnc (enc_blob: known lengths placed at cursor - total; the core's validating UTF-8
  transcoders handed exactly the len bytes below the cursor; other transcoders the whole free region and a
  move of n bytes after commit; enc_blob_run written forward into a summed hole); rust_abi.py renders each
  encode plan reversed, nested messages open -> body -> close(tag), element arrays and packed runs walked last
  to first, SITES still allocated in forward order (abi.rs unchanged); rust_binding.py's four loop renderers
  iterate in reverse, fill the arena from its top, deliver the tail with tok0 = total - done - i. ak_rt::Enc
  (core-native) untouched.
- First build: byte identity held at once (conformance VERDICT pass). bwd_check (new, in the patch) 78/78:
  n = 1, chunks of 8 / 32, mixed sizes, one call; mixed transcoders (trusted, utf8, latin1, utf16) on the
  inner string fields; fresh contexts (grows mid-field and inside nested messages, P5.4's direct argument
  inside `upload`); ak_fail mid-field then a clean encode; CAPACITY. Planted forward order: 47 FAIL (the 31
  PASS rows are one-call fields or P1.3's identical empty elements), so the check can fail.
- Gate (gen/bwd_gate.sh, steps unchanged, step 7 non-fatal): GATE PASSED, both builds; crossings identical
  (836 / 435). Step 7: the pad-widths and global+pad plants cannot fail on a core with no learned width; the
  shipped and global arms 0 wrong. Recorded as vacuous, tests kept.
- Mismatched pair (gen/bwd_mismatch.sh: this branch's binaries, forward binding, + the backward core):
  silent at link and load; same lengths; conformance 5 payloads DIFFER, pre-check 69 failures, corpus 6
  arm rows; shapes passes. Only fields delivered in more than one call with differing elements expose it.
- Other slices surveyed (not touched): every element call site is generated (cpp_binding, cs_binding,
  java_binding, py_capi); containers indexable or staged; no hand-written site.
- Measured (gen/bwd_bench.sh, 453 s codec + 7 s RPC): 168 encode cases x 3 launches per variant, alternated.
  The control row core-native (same code both builds) has disjoint launch ranges on 26/84 rows, so cross-build
  differences of a few percent are drift. P2.4 family lower on the backward core; P6.1 (packed runs) higher,
  61-65 us -> 75-79 us. Is it the core or the build? P6.1's fields are each one call, so the mixed pairs write
  correct bytes there: 2x2 harness x core (bench-p6-2x2, 39 s): forward core 60.4-66.4 us, backward core
  74.2-77.6 us in BOTH harness builds; core-native moves with the harness build only (65-72 vs 56-67 us). The
  cost is the core's; not attributed (no perf).
- Tried v2 for packed runs: sum the varint lengths, write the run forward into a hole of that size. Correct
  (bwd_check, conformance, pre-check). Slower (bench-p6-v2, 29 s): 80.4-86.0 us against v1's 75.3-79.6 us and
  committed 60.3-63.3 us. Dropped; kept as dropped-v2-sized-run.patch. The worktree was restored to v1 and the
  two patch files re-derived with identical sha256.
- RPC (TCP, k = 1): c/P5.4 and d/16MiB work on every cell through the backward core; client CPU per call
  inside the round spread of the committed core (Cf d/16MiB 11.04 vs 11.63 ms medians, ranges overlapping).
- Committed tooling: codec_suite AK_CASE_* filters (defaults unchanged), gen/bwd_*.sh, gen/bwd_tables.py.

## 2026-10-03 -- D14: every pool is AK_WORKERS workers, default 8

- Checked: the server (AK_SERVER_THREADS, default 4), the core runtime (AK_CORE_WORKERS,
  default 2) and the tokio client runtimes (AK_HOST_WORKERS, default 2) did NOT follow
  AK_WORKERS. Changed: all three default to AK_WORKERS, else 8, in the code (grid.rs
  workers_default, rpc_server), in run_campaign.sh (which exports them from AK_WORKERS) and
  in serve.sh; the runner header, the RPC header and the server log state the values.
  SERVER.md's default updated (the interface is otherwise unchanged). Minimal smoke
  (logs/rust/d14/): server 8 workers, client mt8, ak_runtime_new(8).

## 2026-10-03 -- core fix: a fresh encode context needs no reset (owner-approved)

- Python found that since the framed default (e8fe14868) `ak_enc_ctx_new` set head = 5
  without laying the headroom down. New control `bin/fresh_enc`: new context, encode
  ListResultsResponse (page 7, total 3) with no reset, `ak_enc_take`, compare with prost.
  Before (logs/rust/fresh-enc/before.log): 0 bytes where prost writes 4 -- FAIL. Fix in
  ak-core lib.rs: `ak_enc_ctx_new` calls `Enc::reset()` after setting head, the state
  `ak_enc_reset` leaves. After: PASS on the full and the no-unknown build (after.log). The h2
  variants differ only in the transport, not in context creation. Added to gate.sh after
  stickyerr. The gate was NOT run (owner). No committed count changes: `ak_enc_ctx_new` is
  setup, outside every counted loop, and hosts' resets remain host calls.

## 2026-10-03 -- D9 allocator switch in the campaign runner (CAMPAIGN req 25 as amended)

- `AK_CAMPAIGN_ALLOC=default|pinned` (default `default`) in run_campaign.sh. The runner unsets
  GLIBC_TUNABLES. It then prefixes only the measured clients (codec bench, rpc_bench, calib
  and calib under perf) with `env -u GLIBC_TUNABLES` or `env GLIBC_TUNABLES=<pinned>`.
  Pinned output files get the label `-alloc-pinned`. serve.sh launches the server under
  `env -u GLIBC_TUNABLES`. The exploration scripts are unchanged.
- campaign::alloc_check runs at the start of codec_suite, rpc_suite and calib (calib's
  `--only` perf path is excluded). It checks the mode against GLIBC_TUNABLES, then one 16 MiB
  malloc against mallinfo2 hblks. On any disagreement it exits 4 and takes no sample. Rows
  gain `alloc` and `minflt`. The codec suite now uses iter_custom so it can read minflt
  outside the timer. Its rows are matched to sample.json by (iters, cpu), as rpc_suite does.
- Smoke (logs/rust/d9/, figures stripped): codec P1.1, rpc B/a/k1 against serve.sh, and
  calib at 1e5 iterations, each per mode. Default read back mmapped and pinned read back
  heap; every row carries alloc and minflt.
- Plants:
  - pinned without tunables, default with tunables, unset mode with tunables, and mode
    `jemalloc` (codec), plus pinned without tunables (calib): each exited 4 and wrote no
    output file;
  - the runner refuses a misspelt mode (exit 2).
- The readback branch (env agreeing, malloc disagreeing) was not planted. No environment
  produces it without a test hook.
- Not a defect, noted: in default mode glibc's dynamic mmap threshold rises after the first
  large free. The startup readback therefore describes the process start, not the
  steady state.
