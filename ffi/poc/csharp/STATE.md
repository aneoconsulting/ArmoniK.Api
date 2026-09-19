# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

**This file is the handoff.** The slice is on its own branch,
`claude/ffi-slice-csharp`, which it pushed itself; the aggregating session
merges it. Everything the report needs from this slice is here.

| | |
|---|---|
| **Status** | **the managed control is complete, encode and decode, on all 16 payloads and all 7 shapes.** Correctness gated on three runtimes. `core-ffi` NOT built, by instruction |
| **Blocked on** | nothing that is in scope. `core-ffi` waits on ABI v1 open decision 1 |
| **Floor** | netstandard2.0 (builds, passes) and .NET Framework 4.8 on Mono 6.8.0.105 (builds, passes, and is timed as arm c) |
| **Target** | .NET 8.0.31, SDK 8.0.131 |
| **Incumbent** | `Google.Protobuf` 3.28.3, codegen by `Grpc.Tools` 2.66.0 |
| **Machine** | 4 vCPU Intel Xeon @ 2.80GHz, 15 GiB, Linux 6.18.44, x86-64 |
| **R13 calibration** | **the Rust slice's crossing benchmark measures 1.8 ns forward and 2.1 ns fwd+reverse ON THIS CONTAINER**, five runs, medians identical |

**The calibration is load bearing and it disagrees across containers.** This
container reproduces the Rust slice's own 1.8 ns to the digit. The C++ slice's
container measures the same benchmark at **1.5 ns**, and cannot reproduce the
published 0.25 ns C++ row at all (1.24 ns statically). So: **a C# absolute from
this slice is comparable with a Rust absolute and is NOT comparable with a C++
one.** That is a measured statement rather than a caveat, and it is the whole
reason R13 survives the scope change.

## Read the tables as instrumentation, not as the deliverable

`CLAUDE.md`'s invariant and README 5.2's closing paragraph, merged into this
branch at 4aec4e8: **nobody tries hard at cross-language performance yet.** The
comparison is re-taken on a controlled physical machine once the slices exist
and the ABI is validated. So nothing below was tuned for precision, and the
effort went where a rerun cannot reach.

**Final, and a rerun cannot produce it later:**

| | |
|---|---|
| correctness and byte identity, on three runtimes, including the absent path and the unknown-field vectors | the gate, not a timing question |
| the managed decode control EXISTS, encode and decode | the single most valuable thing in this slice's brief |
| oneof and explicit presence, in the facade and the managed codec | previously unmeasured on .NET |
| crossing counts | zero in every arm here, and that is a property of the interface |
| feasibility: the floor compiles and passes on netstandard2.0 and on Mono | and what could not be run at all |
| the within-process deltas that bear on an ABI decision | decisions 5 and 11; the single-pass term |

**Instrumentation, and re-taken later:** every absolute nanosecond, and the
third decimal of every ratio. What the ratios are asked to carry here is which
SIDE of 1.0 an arm lands on and roughly how far, and where they cannot carry
even that they are marked **AMBIGUOUS** and left for the controlled run.

**The verdicts are robust to the JIT configuration**, which was checked rather
than assumed because R9 names it as something that moves a verdict and not a
decimal (`stage6-tiering-sensitivity.log`). Turning tiering or PGO off slows
the INCUMBENT by 5 to 20 percent, exactly the handicap R9 warns of, and no arm
crosses 1.0 under any of the three configurations. The default used everywhere
else in this slice -- tiering and PGO ON, what a deployed service runs -- is
therefore the configuration least favourable to the managed arms, and the
numbers here are the conservative ones.

## The question this slice answers

**Does C# look like Java on decode?** No, and not on any content set.

The C# report had a managed ENCODE control and no managed DECODE control, and
the Java report names that missing control as the one measurement that would
change its own recommendation: if C# looked like Java on decode, the conclusion
would be "the codec half of the C ABI does not suit managed runtimes" and the
ABI's scope would narrow to C++ and Python. It is built, and it does not.

**A generated pure-C# codec decodes at 0.45 to 0.83 of `Google.Protobuf` on
every non-bulk payload of `design/SHAPES.md`, on ASCII, and 0.73 to 0.78 on the
hardest content set.** It is never at parity and never behind on any shape the
control plane actually moves. README section 13's outcome 2 does not follow
from the C# column.

The bulk payloads (M5, 64 KB to 4 MB) are the exception and they are not a
counter-example: there a decode IS a copy, every arm sits on the memcpy floor,
and the ratio between two arms is a ratio between two copies. Those rows run
0.83 to 1.29 and are the noisiest in the slice; R2's own lesson says to read
them as bounded by the floor rather than as a codec result.

## Arms

| arm | what it is | built |
|---|---|---|
| `gp-tobytearray` | `Google.Protobuf` `msg.ToByteArray()`, the call application code writes | yes |
| `gp-writeto` | `msg.CalculateSize()` then `msg.WriteTo(Span<byte>)` into a reused buffer: the fair encode baseline, and the incumbent's own `IBufferMessage` fast path | yes |
| `managed` | **the managed control**: a generated pure-C# codec over the facade, one pass, ABI v1 section 6's learned length width | yes, encode AND decode |
| `managed-2pass` | the same generated codec, `SizeOf` then `WriteSized`: two passes, exact prefixes. The shape `Google.Protobuf` uses | yes |
| `memcpy floor` | `Buffer.BlockCopy` of the payload's own bytes: R2's bound | yes |
| `gp-parse` | `Parser.ParseFrom(ReadOnlySpan<byte>)` | yes |
| `managed-parse` | `Codec.Read` into a fresh facade graph | yes |
| `core-ffi` | the amended ABI | **NO, by instruction.** ABI v1 open decision 1 is unsettled, the ABI is not frozen, and a binding against a draft measures the draft |

## What exists

```
gen/generate.py [--check]  the generator. Imports ffi/schema/emit/shapes.py (R1)
gen/ir.py                  the IR. walk() enumerates oneof members; check_walker() FAILS if it stops
gen/csnames.py             naming, in one place
gen/cs_facade.py           the facade types, and a generated structural comparer
gen/cs_values.py           the value rules of emit/values.py, re-derived, plus the content sets
gen/cs_build.py            payload construction, emitted TWICE over two object models
gen/cs_managed.py          the managed control codec: Write, SizeOf, WriteSized, Read
gen/cs_arms.py             the per-payload arm table

src/Facade/Wire.cs         THE ONE HAND-WRITTEN FILE. The runtime beneath the generated
                           traversal: varints, the growable buffer, the learned length
                           width, the reader. Knows nothing about any message. The direct
                           analogue of the Rust slice's crates/ak-rt
src/Facade/OrderedMap.cs   the facade's map, and why it is not a SortedDictionary
src/Facade/BuildInfo.cs    which sources an assembly was built from, read at RUNTIME
src/Facade/Generated/      Types, Eq, Values, Build, Codec
src/Harness/               Manifest, Conformance, UnknownFields, Counts, ContentSets, Bench
src/Harness/Generated/     BuildGp, Arms
src/HarnessFloor/          arm c: the same sources LINKED, targeting net48, run on Mono
Directory.Build.props      arm b's output redirect, so a and b exist side by side
```

Commands: `harness conformance | unknown | counts | content | bench`.
Builds: default (arm a), `/p:AkFloor=true` (arm b), `/p:AkCount=true` (the
counting build), and `src/HarnessFloor` (arm c).

## What is measured

Ranges are across **three separate processes** unless stated. Ratios are formed
inside one process (R4). ASCII unless stated.

### Decode: the number this slice exists for

Against `gp-parse`, in the same process, on the target (arm a):

| payload | shape | `managed-parse` / `gp-parse` | ns/element | alloc, managed : incumbent |
|---|---|---|---|---|
| P1.1 | M1, 4 flat | 0.629 - 0.654 | 495 | 2,624 : 2,888 |
| P1.2 | M1, 1000 flat | 0.648 - 0.694 | 540 | 641,976 : 697,928 |
| P1.3 | M1, absent path | 0.451 - 0.490 | 47.1 | 37,216 : 39,568 |
| P2.1 | M2, 1 | 0.653 - 0.665 | 3,476 | 4,408 : 4,848 |
| **P2.2** | **M2, 500: the shape the control plane moves** | **0.642 - 0.683** | **4,108** | 2,117,904 : 2,316,528 |
| P2.3 | M2, 30 repeated strings/field | 0.690 - 0.760 | 13,572 | 2,085,480 : 2,091,104 |
| P2.4 | M2, alternating 3/150 | 0.762 - 0.777 | 28,788 | 3,265,688 : 3,283,352 |
| P2.5 | M2, absent path, nested | 0.666 - 0.669 | 3,223 | 79,704 : 86,856 |
| P3.1 | M3, oneof + explicit presence | 0.660 - 0.664 | 207 | 51,952 : 49,664 |
| P4.1 | M4, the adapter site | 0.661 - 0.665 | 1,796 | 357,352 : 392,504 |
| P6.1 | M6, packed | 0.777 - 0.826 | 2,173 | 465,088 : 356,240 |
| P7.1 | M7, interleaved control | 0.556 - 0.597 | 92.1 | 656 : 776 |
| P5.1 | M5, 36 B (degenerate) | 0.631 - 0.643 | 202 | 320 : 368 |
| P5.2 | M5, 64 KB | 0.827 - 0.875 **AMBIGUOUS** | 10,064 | 65,816 : 65,864 |
| P5.3 | M5, 1 MB | **1.158 - 1.286 AMBIGUOUS** | 510,098 | 1,048,856 : 1,048,904 |
| P5.4 | M5, 4 MB | 0.825 - 0.963 **AMBIGUOUS** | 1,619,672 | 4,194,719 : 4,194,632 |

**The M5 rows are a ratio between two copies and are labelled as such.** A bulk
decode is a copy, the memcpy floor sits at 0.12 to 0.27 of `gp-parse` on those
rows, and the two arms differ only in what they copy INTO: the managed arm a
`byte[]`, the incumbent a `ByteString`. P5.3 comes out at 1.16 to 1.29 and P5.4
at 0.83 to 0.96 in the same three processes, which is the spread saying these
rows do not support a ratio at all. This is R2's lesson in the same shape the
Rust slice met it in, one direction over: there a 0.08 needed a floor arm before
it could be reported, here a 1.2 does.

**The allocation column is what bounds this.** The managed arm allocates 0.91
to 1.00 of what the incumbent allocates on every non-packed row, so the two
arms are building object graphs of the same size and the win is not "it built
less". On P6.1 the managed arm allocates MORE (1.31) and is still faster.

**The Rust slice's convergence finding reproduces, with the same ordering.**
Decode approaches parity in proportion to host-side CONTAINER construction per
element, not to bytes or strings: the flat shapes (P3.1, P1.1, P1.2) sit at
0.63 to 0.69, and the container-dense ones (P2.3 with 120 repeated strings per
element, P2.4, P6.1 with five packed lists) sit at 0.69 to 0.83. A list growth
and a map insert are work every arm does identically.

### Encode

Against `gp-writeto` (the fair baseline) and `gp-tobytearray` (what application
code writes):

| payload | `managed` / wto | `managed` / tba | `managed-2pass` / wto | memcpy / wto |
|---|---|---|---|---|
| P1.1 | 0.421 - 0.434 | 0.345 - 0.363 | 0.818 - 0.860 | 0.014 |
| P1.2 | 0.430 - 0.432 | 0.346 - 0.356 | 0.905 - 0.937 | 0.020 - 0.021 |
| P1.3 | 0.223 - 0.328 | 0.216 - 0.322 | 0.313 - 0.440 | 0.002 |
| P2.1 | 0.269 - 0.285 | 0.243 - 0.251 | 0.621 - 0.623 | 0.008 |
| **P2.2** | **0.284 - 0.288** | **0.244 - 0.254** | **0.701 - 0.737** | 0.010 |
| P2.3 | 0.354 - 0.372 | 0.308 - 0.312 | 0.787 - 0.804 | 0.022 - 0.023 |
| P2.4 | 0.388 - 0.422 | 0.301 - 0.351 | 0.789 - 0.809 | 0.032 - 0.034 |
| P2.5 | 0.273 - 0.276 | 0.250 - 0.254 | 0.633 - 0.664 | 0.012 |
| P3.1 | 0.333 - 0.355 | 0.279 - 0.291 | 0.631 - 0.660 | 0.003 - 0.004 |
| P4.1 | 0.232 - 0.233 | 0.222 - 0.224 | 0.593 - 0.612 | 0.006 |
| P6.1 | 0.308 - 0.315 | 0.278 - 0.283 | 0.581 - 0.598 | 0.010 - 0.011 |
| P7.1 | 0.338 - 0.351 | 0.295 - 0.300 | 0.511 - 0.600 | 0.026 - 0.029 |
| P5.1 | 0.461 - 0.500 | 0.283 - 0.314 | 0.796 - 0.800 | 0.100 - 0.101 |
| P5.2 | 0.952 - 0.960 **AMB** | 0.237 - 0.270 | 0.977 - 1.043 **AMB** | **0.942 - 0.964** |
| P5.3 | 0.983 - 0.987 **AMB** | 0.132 - 0.138 | 0.996 - 1.000 **AMB** | **0.968 - 0.971** |
| P5.4 | 1.049 - 1.107 **AMB** | 0.187 - 0.210 | 1.047 - 1.072 **AMB** | **0.999 - 1.056** |

**`managed` / tba reproduces the published 0.22 to 0.43** on the non-bulk
payloads: measured 0.22 to 0.36, with only P1.3's 0.216 to 0.322 straddling the
bottom of the published band. That is prior art reproduced on a rebuild from
the design documents with no access to the original sources, and it is the one
published figure this slice can check.

**R2's decomposition, and it is new.** `managed` against `managed-2pass` is a
within-arm delta in the same interleaved rounds, and it separates the two
halves of the encode win: the single pass with a learned length width is worth
about a factor two and a half (P2.2: 0.284-0.288 against 0.701-0.737, so one
pass costs 0.39 to 0.41 of two),
and the generated traversal against `Google.Protobuf`'s is worth the rest.
Neither half was separable before.

**The M5 rows are marked AMBIGUOUS and left for the controlled run.** They sit
on the memcpy floor, they straddle 1.0, and no amount of rounds here will move
them off it, because what separates the arms there is a copy and an allocator
rather than a codec. The claim is bounded rather than ratioed: a 64 KB to 4 MB encode costs one copy in every arm, and the copy
control is at 0.94 to 1.06 of the incumbent on exactly those rows. `ToByteArray`
is 3.7 to 7.6 times `gp-writeto` there, and that entire column is the
allocation of a multi-megabyte array.

### Content sets (`stage5-content-sets.log`, one process, all three sets)

Wire width: latin1 **1.697 / 1.748** times ASCII, wide **2.394 / 2.495**.
Those match the Rust slice's published 1.70-1.75 and 2.39-2.50, after the set
definition was corrected (journal 9).

| | ascii | latin1 | wide |
|---|---|---|---|
| P1.2 encode `managed`/inc | 0.430 | 0.534 | 0.608 |
| P2.2 encode `managed`/inc | 0.308 | 0.376 | 0.451 |
| P1.2 decode `managed`/inc | 0.650 | 0.750 | 0.781 |
| P2.2 decode `managed`/inc | 0.685 | 0.732 | 0.746 |

In R4's within-arm form, on wide: `managed` encode costs **2.24 to 2.32 times
its own ASCII self**, `gp-writeto` **1.53 to 1.64**, `managed-2pass` **1.55 to
1.69**. The single-pass arm has a higher share of its time in the transcode,
which is the part that scales with output bytes.

**So the encode headline is content-dependent and the decode headline is not.**
A slice quoting only the ASCII encode number overstates the win by about a
third on the hardest content.

### README 5.2's three arms

All three build from ONE source tree with one define flipped, and **all three
pass the same 120 correctness checks**, which is section 5.1's binding
condition on a floor that is different code from the target.

| arm | build | runtime | result |
|---|---|---|---|
| a | target | .NET 8.0.31 | the tables above |
| b | **floor** | .NET 8.0.31 | **b/a = 0.93 to 1.04** on ASCII, **0.947 to 1.062** on latin1 and wide |
| c | floor | **Mono 6.8.0.105, net48** | stands alone, never a ratio against a |

Arm b is the only fair floor-against-target ratio and it says **the floor's
missing APIs cost nothing detectable here** -- b/a straddles 1.0 in both
directions on every row, which is the shape of "no difference" rather than of a
small one, and this slice does not try to resolve it further. The two builds differ only in which
`Encoding.UTF8.GetBytes` overload the transcoder calls (the netstandard2.0
unsafe pointer overload against net8's span overload), so the ASCII table is
nearly a measurement of nothing; the latin1 and wide tables are the sharp
version and they agree, with the floor's overload marginally the faster on
encode. a and b cannot share a process (two definitions of one method in one
assembly), so per R4 they are run round robin with a rotating order and each
carries the incumbent as its in-process control column.

**Arm c, standalone.** On Mono 6.8.0.105 the design still wins, by more on
decode: `managed`/`gp-writeto` 0.307 to 0.371 on encode, `managed-parse`/
`gp-parse` 0.489 to 0.661 on decode, both formed inside the Mono process.
Mono absolutes are roughly three times .NET 8's on the same payloads (P1.2
encode 1,459 us against 488 us; P1.2 decode 2,958 us against 810 us) and those
absolutes are a Mono fact and nothing else.

### ABI v1 open decision 5 (`stage2-counts.log`)

**Zero warm prefix-width misses on every payload except P2.4**, which misses
once per element: 80 misses in 80 elements, moving 979,181 of 979,465 bytes.
The per-site tally from the counting build names the field:
`ListTasksDetailedResponse.tasks`. Cold misses (first encode on a fresh
context, every width still 1) are 0 to 3 per payload, paid once per context.

The Rust slice measured one miss per element moving 980,938 of 981,222 bytes on
the same payload. **Two runtimes, two codegen backends, the same answer.**

### ABI v1 open decision 11 (`stage1-conformance.log`, unknown section)

Seven hand-built vectors: an unknown varint at the root; an unknown
length-delimited field at the root; an unknown varint INSIDE element 0 at tag 7,
numerically between two known tags; an unknown fixed64; an unknown fixed32; an
unknown length-delimited field whose body is itself a message; and an
unrecognised ONEOF member at tag 15 of `Probe.body`.

All seven decode in both arms and neither loses a known value. **But
`Google.Protobuf` RETAINS the unknown field and writes it back, and the
generated codec drops it.** That is the opposite direction from Rust, where
prost drops too and the Rust slice could not price the loss. **On .NET,
adopting the core's codec removes a guarantee that exists today.**

The oneof vector behaves as `design/SHAPES.md` says it must: retention makes
the BYTES of the unrecognised member survive and does NOT make the value
survive, because the grouping lives only in the descriptor.

### Boundary-call counts (R5)

**Zero, in every arm, in both directions**, and that is a fact rather than an
omission: every arm in this slice is in-process managed code and crosses
nothing. The counts R5 exists for belong to `core-ffi`, which is not built.

## Correctness

- **120 checks, 0 failures, on all three of README 5.2's arms.** Byte identity
  against `ffi/schema/generated/manifest.json` for all four encode arms on all
  16 payloads, plus the committed vector byte for byte where one exists.
- **The decode half is checked twice.** Decode-then-re-encode reproduces the
  canonical bytes, AND decode-then-compare checks the graph field by field
  against the one the builder made, through a comparer emitted from the same
  walker as the codec. The first check alone passes a decoder that drops a
  field the encoder also omits; the second is what catches that.
- **Two independent construction routes.** The facade graph and the
  `Google.Protobuf` graph are built by two emitted routes that share the rules
  and no code. The manifest is an external oracle over both, so a defect in the
  shared rules is caught by the hashes rather than by the arms agreeing.
- P7.1 is validated as a permutation of the same (tag, wire type, body)
  triples, because no canonical writer can interleave two repeated fields.
- P1.3 and P2.5, the absent-path payloads, are in the gate and pass.
- **R1's walker guard is a test, not an assertion.** `ir.check_walker` fails if
  `walk()` ever stops enumerating oneof members, and fails if no message in the
  closure has a oneof at all, so the guard cannot silently prove nothing. This
  is the Rust slice's D12 turned into something that runs.
- The generator is `--check`ed at the top of every conformance log, so a
  committed file that is not what the generator writes fails the gate.

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| C1 | `src/Facade/Facade.csproj` | an edit deleted the `AkFloor` define from the facade while leaving it on the harness, so arm b built the TARGET sources and reported `floor sources: no` | **fixed.** Found in one step by the contract's "the first hypothesis is that it is not running" |
| C2 | `src/Facade/BuildInfo.cs` | `Floor` was a `const bool`, which the C# compiler inlines into every reading assembly, so the harness would have reported the flag IT was compiled with rather than the facade's | **fixed**: `static readonly`. A configuration line that can be wrong without anything failing has to be generated, not written |
| C3 | `src/Harness/Counts.cs` (first version) | the per-site miss tally was a DIFF of the learned-width table between two encodes, which reported `none` for P2.4 because the site oscillates 2 -> 3 -> 2 and lands back | **fixed**: a real counting build behind `AK_COUNT`, out of the measured build |
| C4 | `gen/cs_values.py` (first version) | the `wide` content set used mostly two-byte characters and measured 1.78-1.84 times the ASCII wire against the Rust slice's 2.39-2.50 | **fixed** to three bytes throughout; now 2.394/2.495. The underlying cause is in `ffi/schema` and is raised as a request below |
| C5 | `Directory.Build.props` | the floor build's output directory was globbed into the target build's compile items and vice versa, because the SDK excludes only the intermediate directory of the build currently running | **fixed**: both excluded in both directions |

## Requests to the aggregating session

Written here rather than edited into the documents, per the contract.

1. **`ffi/schema` should pin the content sets, not name them by range.** It
   emits ASCII only and describes the others as "U+00A0 to U+00FF" and "above
   U+00FF". The second admits a two-byte and a three-byte encoding, and this
   slice's first choice landed 25 percent away from the Rust slice's wire
   width. Under R1 two slices disagreeing on the wire size of a payload is a
   defect, and this is that defect one level out: the payload set is pinned and
   the content sets are not. A `content_sets` entry in `shapes.json` giving a
   concrete character mapping, and a manifest row per set, would close it.
   `SHAPES.md` already records the same failure once (P7 at 958 B against
   1,016 B) and the fix there was one description.

2. **`design/SHAPES.md`'s RPC arm is not built here and the reason is scope,
   not difficulty.** Noting it so the gap is attributed correctly.

3. **Two hazards the C# `core-ffi` arm will hit, recorded now so they are not
   rediscovered.** Neither is measurable without that arm and both were paid
   for elsewhere. (a) A managed exception inside `[UnmanagedCallersOnly]` does
   not propagate: it is a process abort. Every generated accessor needs the
   guard, and the published margins were measured without it. (b) On the floor
   there is no `UnmanagedCallersOnly` and no `SuppressGCTransition`, so the
   vtable is delegate pointers, and **the delegates must be rooted for the
   lifetime of the vtable or the collector reclaims a thunk the codec still
   holds** -- which is a crash and not a slowdown.

4. **The managed control does three things less than the incumbent, and the
   decode win is partly that.** `Google.Protobuf` enforces a recursion-depth
   limit and a message-size limit, and retains unknown fields; the generated
   codec does none of the three. Two of those are ABI v1 open decisions 7 and 8
   and are unbuilt in the Rust slice too. The decode figures should carry that
   sentence wherever they are quoted.

## What is not measured

A pass for completeness, not for brevity (README R11).

### The arm that is not here

- **`core-ffi`, entirely.** No C ABI, no binding, no `[UnmanagedCallersOnly]`,
  no group, no batching predicate exercised on .NET, no accessor guard, no
  delegate rooting on the floor. By instruction: ABI v1 open decision 1 is
  unsettled. **Every crossing-count column in this slice is therefore zero, and
  the interface-cost decomposition the Rust slice makes available to every
  other slice is not subtracted against anything here.** The generator is laid
  out so the backend drops in beside `cs_managed.py` without moving anything:
  `ir.py` already computes the leafness predicate, and `cs_build.py`'s sink
  split already separates the object model from the rules.
- With it, ABI v1 open decisions 1, 2, 4, 6, 7, 8, 9, 10 and 12 are all
  untouched on .NET.

### Shapes and payloads

Every shape and payload of `design/SHAPES.md` is covered by the arms that are
built. What remains is what `SHAPES.md` itself says is unreachable:

- **Nesting past depth 3.** The description's maximum is 3; the real schema
  reaches 6 through the filter and request family, which this response-only
  payload set does not carry. ABI v1 decision 7's recursion limit is
  unexercised, and the managed codec does not implement one.
- **The adapter's non-injective states at the plain site** are reached (the
  payload cycles all three), but the FACADE here keeps `TaskOutput` as a plain
  struct rather than as a sum type, so the collision is present on the wire and
  no adapter in this slice has to choose which of Ok and Invalid to lose. The
  shape is measured; the design decision it forces is not.

### Measurement coverage

- **Content sets on P1.2 and P2.2 only**, encode and decode, arms a and b. Not
  on the other fourteen payloads, and with no manifest oracle (byte identity
  against the incumbent arm instead, which is what `SHAPES.md` prescribes).
- **Arm c on six payloads only** (P1.2, P1.3, P2.2, P2.5, P3.1, P6.1), ASCII,
  two runs. Mono is slow enough that the full set was not worth the wall clock.
- **The RPC arm does not exist.** No unary call, no CPU per RPC, no allocation
  per RPC, no 1/8/16 in flight, no carrier-thread question. `Grpc.Net.Client`
  is named as part of the incumbent in this slice's original brief and is not
  referenced anywhere in it. R9's HTTP/2 flow-control hazard on P2.2 is
  therefore untested here.
- **Concurrency: one thread everywhere.** The learned-width table is per
  encode context and has never been touched by two threads, which is the case
  ABI v1 section 6 says a global table fails at. Four vCPUs is in any case
  close to the floor at which a contention figure means anything.
- **No GC pause, working-set or steady-state-under-load figure.** Allocation
  per operation is measured; what the collector then does with it is not.
  `ServerGarbageCollection` is off, `LatencyMode` is Interactive, and a server
  would run neither.
- **The `ToByteArray` column's allocation is measured and its GC consequence is
  not.** On P5.4 that is a 4 MB LOH allocation per call, and the difference
  between it and `gp-writeto` is reported as time, not as collector pressure.
- **Startup and R2R are not measured**, and a cold-start column is deliberately
  deferred rather than missing. Every figure here is a warmed, tier-1 figure by
  construction: each case runs for a budget, the process sleeps so the
  call-counting thread can promote, and the budget runs again. What a cold
  process costs is the number a short-lived worker would care about, and it is
  an absolute, so it belongs to the controlled run.
  **Tiering and PGO themselves ARE measured** -- `stage6-tiering-sensitivity.log`
  -- because R9 names them as moving a verdict rather than a decimal, and the
  answer is that they move neither here.
- **`gp-writeto` allocates 56 bytes per message carrying a `map<string,string>`**
  in a path that otherwise allocates nothing (0 on M1, M5, M6; exactly 56 times
  the element count on every M2 and M4 payload). Measured; the cause is not
  identified and is not pursued.
- **One machine, one microarchitecture.** AVX2 present, which matters for
  `Encoding.UTF8`'s vectorised ASCII path and therefore for every string
  number here.
- **The M5 decode rows do not support a ratio.** P5.3 measures 1.16 to 1.29 and
  P5.4 measures 0.83 to 0.96 in the same three processes; the memcpy floor is
  at 0.12 to 0.27 there, so both arms are on the copy and what separates them
  is allocator luck on a multi-megabyte buffer. Reported as a bound, not a
  ratio.

### Behaviour and error paths

- **Unknown-field RETENTION is priced as a behaviour difference and not as a
  cost.** What retaining would cost the managed codec in time and allocation is
  not measured, because the codec has no bag; the Rust slice measured that side
  (1 to 12 percent of an encode, free on decode) and this slice measured the
  side Rust could not (the guarantee exists on .NET today and would be lost).
- **The transcode pair is reachable here and is only half-checked.** A C# host
  CAN hold an unpaired surrogate, which a Rust `String` cannot, so this slice
  can construct the input the Rust slice called unreachable. Probed:
  `Google.Protobuf` and `Encoding.UTF8` both SUBSTITUTE U+FFFD rather than
  throwing, so both arms agree and lose the surrogate identically. What that
  does to a round trip through protobuf-java, which is where the pair matters,
  is not tested.
- **Malformed wire is not exercised.** `ErrTruncated` and `ErrMalformed` exist
  in the reader and no vector reaches them. Malformed UTF-8 is accepted lossily
  by both arms by design and no vector asserts it.
- **The decode UTF-8 policy is LOSSY in both arms**, verified directly: invalid
  input yields U+FFFD in both and neither throws. ABI v1 open decision 3's
  rejecting policy, which the Rust slice found free to cheaper on decode, is
  NOT what either arm here runs, and a rejecting managed policy is unbuilt.
- **`ak_init` and the lifecycle**: nothing, there being no core.

## Slice-specific notes

- **`Grpc.Tools` runs `protoc` over `ffi/schema/generated/shapes.proto`
  directly**, so the incumbent's classes and the facade come from one
  description (R1). Nothing in this slice reads `Protos/V1`.
- **`packages/csharp` is not read and not measured.** The facade here is this
  generator's own, as the Rust slice's `armonik` arm reproduces a pattern from
  `packages/rust` rather than using it. What the real `ArmoniK.Api.Client`
  object model would cost is not measured.
- **The facade uses public fields rather than auto-properties.** They inline to
  the same code at tier 1 on the target; on the Mono floor there is no such
  guarantee, which is the reason for the choice.
- `.gitignore` re-includes sources and excludes `bin/`, `obj/`, `bin-floor/`,
  `obj-floor/` and `__pycache__/`. The Rust slice's D19 -- the repository root's
  `[Bb]in/` rule silently excluding every measurement binary -- was checked for
  here with `git ls-files` and does not apply: this slice's code is under
  `src/` and `gen/` and all of it is tracked.

## Next step

Nothing is outstanding within the instructed scope, and under the scope change
the list is shorter than it was: chasing absolutes is explicitly not on it.

1. **The `core-ffi` arm, when the ABI is frozen.** Everything else in this slice
   exists to be subtracted against it. Note that the merged C++ slice's commit
   message says it **answered** ABI v1 open decision 1, but `design/ABI-v1.md`
   is unchanged on this branch and the instruction relayed to this slice still
   holds the arm. **The arm stays held until the design document says
   otherwise**, which is the aggregating session's call and not a slice's.
   Hazards 3(a) and 3(b) above are what it will hit first.
2. **The RPC arm**, the largest hole, and purely scope rather than difficulty.
   Worth noting under the new rule that its valuable half is behavioural and
   survives a controlled rerun: the crossing count per RPC (which should be two
   and not a function of field count) and whether a `Task` can be awaited
   without pinning a thread. Its CPU-per-RPC column is exactly the kind of
   absolute the rerun will take properly.
3. **A rejecting decode policy as a second managed arm.** This is the one ABI v1
   decision this slice could settle for .NET without the ABI being frozen, and
   it is a within-process delta, which is the category the new rule says to
   spend effort on. The Rust slice found validate-and-reject free to cheaper on
   decode; both arms here run the LOSSY policy today.
4. **Content sets on the remaining fourteen payloads**, if and only if the
   report needs them. The two that are measured already establish the shape.

Deliberately NOT on the list any more: more rounds to tighten a spread, a
cold-start column, and any attempt to make this container's absolutes
comparable with another container's. R13's one calibration run stands and is
not to be tuned.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `ffi/logs/csharp/calibration-rust-crossing.log` | rustc 1.94.1 release, cdylib boundary, 4 vCPU Xeon 2.80GHz; the Rust slice's own bench binary, `AK_BENCH_ONLY` filtered to the crossing cases, five runs | **R13, and it is the first thing this slice did.** 1.8 ns forward and 2.1 ns fwd+reverse on THIS container, medians identical across five runs, with the boundary shown as a real dynamic import. Reproduces the Rust slice's container to the digit, so absolutes travel between the two |
| `ffi/logs/csharp/stage1-conformance.log` | .NET SDK 8.0.131, .NET 8.0.31, Google.Protobuf 3.28.3, Mono 6.8.0.105 | **The gate.** `gen/generate.py --check`, then 120 checks with 0 failures on EACH of README 5.2's three arms, then the seven unknown-field vectors. Carries the decision-11 retention asymmetry |
| `ffi/logs/csharp/stage2-counts.log` | as above, **`/p:AkCount=true`, the counting build** | Crossings are zero in every arm and why. ABI v1 open decision 5: zero warm misses everywhere but P2.4, which misses once per element and moves 979,181 of 979,465 bytes, with the site named |
| `ffi/logs/csharp/stage3-arms.log` | .NET 8.0.31, Google.Protobuf 3.28.3, arm a, no AkCount, ASCII, tiering and PGO ON at their defaults, three processes, seven interleaved rounds | **The main table.** All 16 payloads, five encode arms and three decode arms, with ns/element, allocation per operation and the memcpy floor. The decode column this slice exists for |
| `ffi/logs/csharp/stage4-floor-arm-b.log` | arms a and b, both .NET 8.0.31, round robin with a rotating order, six payloads, each binary carrying the incumbent as its in-process control | **README 5.2 arm b.** The floor's missing APIs cost nothing: b/a is 0.93 to 1.04 |
| `ffi/logs/csharp/stage4-floor-arm-c.log` | **Mono 6.8.0.105**, net48, floor sources linked from the same tree, six payloads, two runs | **README 5.2 arm c, standalone.** Passes the same 120 checks on the floor runtime; the design's advantage survives it and is larger on decode. Mono absolutes, quoted as absolutes and never as a ratio against arm a |
| `ffi/logs/csharp/stage5-content-sets.log` | arm a, .NET 8.0.31, all three content sets in ONE process, P1.2 and P2.2 | **The string path, which is 174 of 413 fields.** Wire widths matching the Rust slice (1.70/1.75 and 2.39/2.50), and the encode advantage narrowing from 0.31-0.43 to 0.45-0.61 while decode barely moves. Carries the set-definition defect and its correction |
| `ffi/logs/csharp/stage5-content-sets-floor.log` | arm b, otherwise as above | The sharp version of arm b: b/a is 0.947 to 1.062 where the transcoder actually has work |
| `ffi/logs/csharp/stage6-tiering-sensitivity.log` | arm a, three processes differing ONLY in `DOTNET_TieredPGO` and `DOTNET_TieredCompilation`, P1.2 / P2.2 / P3.1 | **R9's JIT hazard, measured rather than argued.** Tiering off or PGO off slows the INCUMBENT by 5 to 20 percent, in the direction R9 names. **No arm crosses 1.0 under any configuration**, so no verdict in this slice is JIT-configuration dependent, and the default used everywhere else is the one least favourable to the managed arms |
