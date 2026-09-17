# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | not started |
| **Blocked on** | W1 (`design/ABI-v1.md`, drafted, 10 open decisions) and W2 (`design/SHAPES.md`, drafted) |
| **Floor** (must build and pass correctness) | MSRV 1.88 (the workspace pin) |
| **Target** (where the clock runs) | MSRV 1.88, one configuration |
| **Incumbent** (the baseline every ratio is against) | prost and tonic, plus the in-repo `armonik` crate |

## The question this slice answers

What a host language loses against full Rust, and what the new design costs against `packages/rust` today. This slice is the denominator for every other one.

## Arms

- `prost`: prost generated structs, tonic codec. Today's floor
- `armonik`: the in-repo crate, hand-written types implementing `prost::Message` directly
- `core-native`: the new design's generated codec, called from Rust, no FFI
- `core-ffi-rust`: the same core through the C ABI from a Rust host. The interface cost with the runtime tax removed

## What exists

Nothing yet.

## Next step

**Validate `ffi/schema/generated/manifest.json` against prost first.** Every hash in it
was produced by `ffi/schema/emit/wire.py`, which nothing has checked; the framing
is proved, the semantics are not. Compile `schema/generated/shapes.proto` with prost,
re-encode each payload from the same value rules, and either confirm the hashes
or report where they differ. Nothing else in any slice can be trusted until this
passes. Then stand up the four arms over M1 and P1.1/P1.2 only.

## Correctness

Not established. Nothing is timed before byte identity holds across every arm,
including P1.3 and P2.5 (the absent path) and the unknown-field vectors.

## Open defects

None recorded.

## What is not measured

Everything. This list is filled as the slice narrows it.

## Slice-specific notes

- Reads `packages/rust`. Does not edit it.
- This slice's `core-ffi-rust` number is what every other slice subtracts to separate interface cost from runtime tax, so it is the one arm that must exist before the managed slices are read.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| | | |
