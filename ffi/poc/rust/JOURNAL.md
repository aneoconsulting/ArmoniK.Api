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
