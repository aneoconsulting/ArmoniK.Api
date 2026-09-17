# python slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | not started |
| **Blocked on** | W1 (`design/ABI.md`) and W2 (`design/SHAPES.md`) |
| **Floor** (must build and pass correctness) | the floor `pyproject.toml` declares (`>=3.7`), open question |
| **Target** (where the clock runs) | to be decided, proposal 3.11 |
| **Incumbent** (the baseline every ratio is against) | `protobuf` (upb C extension) and `grpcio` (the gRPC C core) |

## The question this slice answers

Does the amended ABI beat an incumbent that is already native, and can it survive the GIL?

## Arms

- incumbent: protobuf-python on upb, grpcio
- `core-ffi`: the amended ABI through the binding mechanism chosen below
- no-boundary control: a generated pure-Python codec over the same facade

## What exists

Nothing yet.

## Next step

Microbenchmark the binding mechanisms (`ctypes`, `cffi` ABI and API, PyO3) for forward and reverse call cost before committing the slice to one.

## Correctness

Not established. Nothing is timed before byte identity holds across every arm,
including P1.3 and P2.5 (the absent path) and the unknown-field vectors.

## Open defects

None recorded.

## What is not measured

Everything. This list is filled as the slice narrows it.

## Slice-specific notes

- A reverse call into Python must hold the GIL, so the drafted ABI's per-field upcall is the worst possible shape here.
- `packages/python` reads no transport environment configuration today, so configuration homogeneity is a pure gain rather than a migration.
- A pure-Python control may lose to the native incumbent by an order of magnitude. That is a result, not a defect: it says the codec question in Python is native-against-native.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| | | |
