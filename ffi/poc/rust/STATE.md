# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | stage 1 done, with a finding. Stages 2 to 4 not started |
| **Blocked on** | nothing to start stage 2. **The aggregating session owes a decision on the stage 1 finding below**, because 8 of 16 manifest hashes change if it is accepted, and every other slice diffs against them |
| **Floor** (must build and pass correctness) | MSRV 1.88 declared. **Not verified: no 1.88 toolchain exists in this container, only 1.94.1** |
| **Target** (where the clock runs) | the same, one configuration (README section 5) |
| **Incumbent** (the baseline every ratio is against) | prost 0.14.4 / tonic 0.14, plus the in-repo `armonik` crate |

## The question this slice answers

What a host language loses against full Rust, and what the new design costs against `packages/rust` today. This slice is the denominator for every other one.

## Arms

- `prost`: prost generated structs, tonic codec. Today's floor. **Exists** (`crates/shapes-prost`)
- `armonik`: hand-written types implementing `prost::Message` directly. Not built
- `core-native`: the new design's generated codec, called from Rust, no FFI. Not built
- `core-ffi-rust`: the same core through the C ABI from a Rust host. Not built

## What exists

```
Cargo.toml                       a workspace of its own; the repo has no root workspace
gen/dump_payloads.py             imports ffi/schema/emit, writes all 16 payloads to a scratch dir
gen/stage1_isolate.py            applies a candidate fix to a COPY of ffi/schema/emit and re-checks
gen/stage1.sh                    stage 1 end to end
crates/shapes-prost              build.rs: protox 0.9 -> prost-build 0.14 over
                                 ffi/schema/generated/shapes.proto. btree_map(".") so map entries
                                 sort by key. Also exports the descriptor as `DESCRIPTOR`.
                                 tests/zero_leaf.rs: the same check through prost-reflect
crates/shapes-values             the value rules of emit/values.py, hand-re-derived in Rust
crates/stage1-validate           builds all 16 payloads as prost values, encodes, compares BYTES
```

There is **no generator yet** (`gen/` holds Python helpers, not the slice's code generator). That is stage 2's first item.

## What is measured

Nothing is timed. Stage 1 is correctness only.

## The stage 1 finding, in one paragraph

`ffi/schema/emit/payloads.py` writes `Timestamp` and `Duration` through a shortcut that
emits `seconds` and `nanos` **unconditionally**, including when they hold the proto zero.
Every other scalar path in that file guards on the value. Eight of the sixteen payloads
carry those extra bytes; they are legal wire and decode to the same value, which is why
`emit/check.py` passed. **prost is right and the emitter is wrong**, and so is the schema
directory's own stated canonical form. Detail, evidence and the exact change are in
`JOURNAL.md` under 2026-09-18 and in the three logs below. **This slice did not edit
`ffi/schema/**`**; the change is proposed, not applied.

## Next step

**Stage 2**, and it can start now: the disagreement is understood and localised, and the
Rust side's payload construction is already independent of it.

1. `gen/`: a generator importing `ffi/schema/emit/shapes.py` (R1), emitting the facade
   types, the `core-native` codec, the C ABI surface of `design/ABI-v1.md` and the
   `core-ffi-rust` binding.
2. All four arms over M1 and P1.1/P1.2 in one process (R3, R4), byte identity first (R2),
   a counting build for boundary calls per payload per direction (R5), accessor guard on
   (ABI section 5).

When the aggregating session rules on the finding, re-run `gen/stage1.sh` against whatever
`ffi/schema/generated/` then holds; it needs no change to stay valid.

## Correctness

- All 16 payloads regenerate from `ffi/schema/emit` exactly as committed, and the 7
  committed `.bin` vectors are byte-identical to what the emitters produce today.
- 15 of 16 payloads are re-derived in Rust and encoded by prost. 8 match the manifest
  byte-for-byte; 8 do not, all for the one reason above.
- P7.1 cannot be encoded by prost at all (it interleaves two repeated fields on purpose).
  It is checked by decoding: prost accepts it, decodes it to the value the rules predict,
  and a contiguous re-encode is a permutation of the same fields.
- prost decodes its own encoding back to an equal value on P1.2, P1.3, P2.2, P2.5, P3.1,
  P4.1 and P6.1.
- Byte identity **across arms** is not established, because only one arm exists.

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| D1 | `ffi/schema/emit/payloads.py` (NOT this slice's to fix) | `Timestamp`/`Duration` write an implicit-presence leaf holding the proto zero. 8 of 16 manifest hashes are wrong | reported, not fixed. Awaiting the aggregating session |
| D2 | this container | no rustc 1.88, so the declared MSRV is unverified | open, cannot be fixed here. Stated on every log |

## What is not measured

Everything. Specifically, and to be narrowed as the slice proceeds:

- every timing, in every arm and every direction;
- three of the four arms do not exist;
- crossing counts (R5): no counting build yet;
- the accessor guard's cost (ABI section 5): not built;
- the RPC arm, tonic, concurrency, streaming, TLS (SHAPES.md "The RPC arm");
- content sets other than ASCII (`latin1`, `wide`): nothing touches the string path yet;
- ABI open decision 5, the grow path, whose payload is P2.4;
- unknown fields on the wire: the corpus (W8) does not exist, so nothing here executes
  the skip path;
- `packages/rust`'s own `armonik` types: read, not yet measured against.

## Slice-specific notes

- Reads `packages/rust`. Does not edit it. The `armonik` arm cannot reuse
  `armonik-macros`: its expansions reference `armonik`-internal paths and read a descriptor
  built from `Protos/V1`, so the arm reproduces the *pattern* (hand-written facade types,
  generated `prost::Message` impls) from this slice's own generator.
- This slice's `core-ffi-rust` number is what every other slice subtracts to separate
  interface cost from runtime tax, so it is the one arm that must exist before the managed
  slices are read.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `ffi/logs/rust/stage1-manifest-vs-prost.log` | rustc 1.94.1, prost 0.14.4, protox 0.9.1, 4 vCPU Xeon 2.80GHz, Ubuntu 24.04.4; nothing timed | 8 of 16 manifest payloads match prost byte-for-byte, 8 do not; P7.1 checked by decode |
| `ffi/logs/rust/stage1-isolate-zero-leaf.log` | as above | one change to a scratch copy of `emit/payloads.py` makes all 15 prost-encodable payloads byte-identical, so there is exactly one defect |
| `ffi/logs/rust/stage1-second-encoder.log` | as above, plus prost-reflect 0.16.5 | the omission rule is protobuf's, not prost's: a second implementation over the same descriptor agrees. Carries the 8 corrected sizes and sha256s |
