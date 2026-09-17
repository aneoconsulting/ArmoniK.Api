# The reconciled C ABI

**Status: not written. This is work item W1, and it blocks every slice.**

## What this document has to do

The base design's ABI predates both managed-host reports, and the C# report
measures it as a regression: decode 1.12 to 1.19 of `Google.Protobuf`, never
faster. The interface that works exists only as recommendations spread across
two companion reports. This document merges them into one specification that a
slice is built against, so that no slice is ever built against a report again.

## Sources to reconcile

| Source | What to take from it |
|---|---|
| [ArmoniK Rust Core](https://claude.ai/code/artifact/d29ed568-05eb-4ded-b22a-b1db669a56fb?sk=k-raOgdhnjAvYmpd0YdSGA) | the layering, the RPC half, handles and lifetime, the streaming contract, the requirements before shipping |
| [C# Across the ABI](https://claude.ai/artifact/WYD94FSYuq1Nxjdu6WHbtS?sk=aeQYJo8cccAcZsFgdTXRkQ), section 4 and section 10 | the amended codec interface and the rule behind it |
| [Java Across the ABI](https://claude.ai/artifact/YFSNVzYD41C1TsHmANLcBu?sk=MJlBPbq3WV9R4dxdhAv6Og), section 4 and section 11 | the transcoder triple, the batched drain, the host-owned context, the length-placeholder rule |

## The amendments to fold in, and where each came from

Each of these needs writing up properly, with the figure that motivated it and
the language that produced it. Listed here so W1 starts from an inventory rather
than from a re-read.

- **The rule itself, in the header**: the host may drive iteration over its own
  containers; the host must never need to know the wire format. Every case below
  follows from it, and a named accessor slot is safe across a schema change where
  a positional stream of values is not.
- **A string is data in the group, not a call**, in both directions: a pointer, a
  length in source code units, and a transcoder; on decode, an (offset, length)
  span into the buffer the host handed in. Removes the write callback, the
  capacity slot, the grow, the clamp and the half-open rollback.
- **`max_bytes_per_unit` as data in the transcoder**, expressed per source code
  unit rather than as a ratio over bytes, and a growth callback that makes it a
  hint rather than a correctness contract.
- **Batched element runs**, host driven and chunked, with one entry point taking
  a count rather than two symbols.
- **A host-owned decode context** carrying a bounded arena, flushed on a foreign
  tag, with the batching predicate computed from the descriptor.
- **No map case.** A map is a repeated field of a pair message.
- **Plain exports rather than a table** for everything the host calls.
- **An error channel on every accessor**, plus the generated guard: a managed
  exception escaping a reverse call terminates the process.
- **Group layouts exported and asserted at load**, and one `ak_abi_version`.
- **A learned length-placeholder width, per encode context**, never process
  global, and no padding.
- **Reserved `ptr` values for direct arguments**, which is what makes a
  multi-megabyte upload cross as a pinned argument rather than a staged copy.

## Open, and to be decided in W1 rather than discovered in a slice

- Whether every amendment is free under the **C++11 floor**. The amendments were
  motivated by managed hosts; that they cost C++ nothing is an argument today.
- The **worker path**, where Rust interprets message content rather than
  forwarding bytes: two decoders over one buffer, or typed getters that put the
  schema back into the boundary.
- Whether the accessor contract **assumes the host language implements it**. In
  Python the accessors are generated C speaking the CPython API rather than
  Python code (README 9.1), so the specification has to be written in terms of
  what a binding provides, not what a host language does.
- The **unpaired-surrogate substitution**, which Java and .NET do not currently
  agree on, and which the conformance corpus will pin.
