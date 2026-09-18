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
