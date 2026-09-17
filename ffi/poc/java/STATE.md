# java slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | not started |
| **Blocked on** | W1 (`design/ABI.md`) and W2 (`design/SHAPES.md`) |
| **Floor** (must build and pass correctness) | Java 8 |
| **Target** (where the clock runs) | JDK 17 with the JNI back end |
| **Incumbent** (the baseline every ratio is against) | protobuf-java (the pom pins 3.19; grpc-java 1.74 resolves 3.25.5) and grpc-java |

## The question this slice answers

Does the encode regression survive the reconciled ABI, and does the generated-Java-codec fallback stay ahead in both directions?

## Arms

- incumbent: protobuf-java `toByteArray` and `parseFrom`
- `core-ffi`: the amended ABI, JNI on the target, FFM as a secondary arm on JDK 22+
- `R`: a generated pure-Java codec over the same facade. Not a floor, the alternative architecture

## What exists

Nothing yet.

## Next step

Import the slice, re-establish the primary configuration, and re-run the encode comparison against the reconciled ABI.

## Correctness

Not established. Nothing is timed before byte identity holds across every arm,
including P16 and P17 (the absent path) and the unknown-field vectors.

## Open defects

None recorded.

## What is not measured

Everything. This list is filled as the slice narrows it.

## Slice-specific notes

- FFM is a JDK 22 API, so floor and target are both JNI. An FFM-to-JNI ratio is a comparison of binding mechanisms, not of ABI shapes.
- Measurement hazard, load-bearing: on JDK 21+ one `String.format` with a numeric conversion permanently deoptimises every char narrowing loop in the process, which is protobuf-java's own encoder.
- The binding must be re-entrant before anything else is trusted: pooled buffers as instance state, not statics.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| | | |
