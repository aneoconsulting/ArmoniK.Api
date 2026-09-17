# cpp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | not started |
| **Blocked on** | W1 (`design/ABI.md`) and W2 (`design/SHAPES.md`) |
| **Floor** (must build and pass correctness) | C++11 (customer pin). Open question: `packages/cpp` sets `CXX_STANDARD 14` today |
| **Target** (where the clock runs) | C++17 |
| **Incumbent** (the baseline every ratio is against) | protobuf C++ (arena and non-arena), grpc++ |

## The question this slice answers

Does the amended ABI still work for the language the design was drafted for, and is it free here as the managed reports assume?

## Arms

- incumbent: protobuf C++, arena and non-arena
- `core-ffi`: the amended ABI through a hand-written C++ facade
- no-boundary control: the same generated codec emitted into C++
- upb, as the fastest measured comparator, not as a candidate

## What exists

Nothing yet.

## Next step

Port the existing C++ slice onto the reconciled ABI of W1, then re-run its correctness suite before any timing.

## Correctness

Not established. Nothing is timed before byte identity holds across every arm,
including P1.3 and P2.5 (the absent path) and the unknown-field vectors.

## Open defects

None recorded.

## What is not measured

Everything. This list is filled as the slice narrows it.

## Slice-specific notes

- The prior slice covered 16 of 179 messages, all flat, all from one service, with 2 repeated-string fields out of 11 repeated fields. It could not see a cost that falls on repeated strings.
- The generator is a Rust binary using the `clang` crate; what defeated it before was generics (libclang exposes the primary template pattern) and C++ name resolution.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| | | |
