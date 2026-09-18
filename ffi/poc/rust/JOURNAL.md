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
