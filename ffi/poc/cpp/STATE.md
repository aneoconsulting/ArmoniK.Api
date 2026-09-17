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

- A vocabulary type that arrives after C++11 is ours (`string_view`, `optional`,
  `variant`, `span`): one concrete type at every standard level, conversions to
  and from the standard counterpart guarded by the feature macro, as
  `packages/cpp/ArmoniK.Api.Common/header/utils/string_view.h` already does. The
  C++11 library itself (`std::string`, `std::vector`, `std::map`,
  `std::shared_ptr`) is used directly: it means the same thing at every level.
- An additive interface per level is allowed (a C++20 coroutine surface over the
  ABI's completion callback, say), on three conditions: the floor keeps a
  complete alternative, it needs no new C entry point, and it is free functions
  or an adapter type rather than new members on an installed class. The slice
  does not have to build one; it has to show the ABI primitive supports one.
- Floor and target may be different code, with one hard stop: **the divergence
  must not reach the layout of an installed header type.** The consumer picks
  `-std`, we do not, so a facade type whose layout depends on the standard level
  is an ODR violation waiting for a consumer who compiles at a different level
  than the library was built at. The base design verified its optional and its
  sum type ABI-identical from C++11 through C++23; a `std::variant` in a public
  header surrenders that property and has to be a decision rather than a
  convenience. Inside the codec and the binding, diverge freely: `if constexpr`
  in the generated traversal and `std::string_view` on a decode span are exactly
  what C++17 is a target for. A five-variant oneof cost ~105 hand-rolled C++11
  lines against Rust's 14, so this is where the target level is worth the most.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| | | |
