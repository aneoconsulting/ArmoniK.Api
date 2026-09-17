# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | not started |
| **Blocked on** | W1 (`design/ABI.md`, drafted, 10 open decisions) and W2 (`design/SHAPES.md`, drafted) |
| **Floor** (must build and pass correctness) | netstandard2.0, and failing that .NET Framework 4.8 |
| **Target** (where the clock runs) | .NET 8 |
| **Incumbent** (the baseline every ratio is against) | `Google.Protobuf` and `Grpc.Net.Client` |

## The question this slice answers

Closing the two gaps the C# report names: a managed decode control, and oneofs plus explicit presence.

## Arms

- incumbent: `Google.Protobuf` (`WriteTo` is the fair encode baseline, `ToByteArray` the call application code writes)
- `core-ffi`: the amended ABI
- managed control: a generated pure-C# codec over the same facade, **encode and decode**

## What exists

Nothing yet.

## Next step

Import the existing slice, build it here, reproduce one published ratio, and only then rebuild against W1.

## Correctness

Not established. Nothing is timed before byte identity holds across every arm,
including P1.3 and P2.5 (the absent path) and the unknown-field vectors.

## Open defects

None recorded.

## What is not measured

Everything. This list is filled as the slice narrows it.

## Slice-specific notes

- The managed decode control is the single measurement that would change the recommendation: if C# looks like Java on decode, the conclusion is that the codec half does not suit managed runtimes, not that Java is special.
- An accessor that cannot fail is a process abort: a managed exception inside `[UnmanagedCallersOnly]` does not propagate. Every generated accessor needs the guard, and the published margins were measured without it.

- Floor and target may be different code. `<TargetFrameworks>netstandard2.0;net8.0</TargetFrameworks>`
  with `NET8_0_OR_GREATER` against `NETSTANDARD2_0`, both emitted by one
  generator flag. The floor has no `UnmanagedCallersOnly` and no
  `SuppressGCTransition`, so its vtable is delegate pointers, and the delegates
  must be rooted for the lifetime of the vtable or the collector reclaims a thunk
  the codec still holds, which is a crash rather than a slowdown. Arm b of README
  5.2 is the floor sources built for net8 and run on it.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| | | |
