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
