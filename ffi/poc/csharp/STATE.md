# csharp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

**This file is the handoff.** The slice is on its own branch,
`claude/ffi-slice-csharp`, which it pushed itself; the aggregating session
merges it. Everything the report needs from this slice is here.

| | |
|---|---|
| **Status** | **complete.** The managed control on all 16 payloads and all 7 shapes, gated on three runtimes; a **`ffi/corpus` consumer** (336 vectors, three arms); **`core-ffi` on every shape**, encode, push decode and PULL decode, R5 checked against the core's own counters on every row; and an **end-to-end RPC arm** over a UDS with ArmoniK's transport pinned |
| **Blocked on** | nothing |
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
the INCUMBENT, exactly the handicap R9 warns of, and **no arm crosses 1.0
under any of the three configurations**: managed encode stays at 0.265 to
0.429, managed-2pass at 0.613 to 0.915, managed decode at 0.770 to 0.852. The
default used everywhere else -- tiering and PGO ON, what a deployed service
runs -- is the configuration least favourable to the managed arms, so the
numbers here are the conservative ones.

**And one verdict in this slice was NOT robust to its own harness**, which is
why the list above is worth taking literally. Both incumbent arms were
handicapped, the decode one by a full extra traversal, and the published
decode column was flattered by 0.10 to 0.18 until `87a2e39`. A correctness
gate catches a wrong codec; nothing but reading the arm table catches a wrong
comparison.

## The question this slice answers

**Does C# look like Java on decode?** No, and not on any content set.

The C# report had a managed ENCODE control and no managed DECODE control, and
the Java report names that missing control as the one measurement that would
change its own recommendation: if C# looked like Java on decode, the conclusion
would be "the codec half of the C ABI does not suit managed runtimes" and the
ABI's scope would narrow to C++ and Python. It is built, and it does not.

**A generated pure-C# codec decodes at 0.73 to 0.87 of `Google.Protobuf` on
every shape the real schema actually has**, measured against **the codec path
ArmoniK actually runs** (R14): `parser.ParseFrom(context.PayloadAsReadOnlySequence())`,
the gRPC marshaller's own decode call. README section 13's outcome 2 does not
follow from the C# column.

**These figures are the CORRECTED ones** (`87a2e39`, JOURNAL.md entries 14 and
16). Everything this slice published before that commit is withdrawn: the
decode baseline was doing a full extra traversal the managed arm was not, and
the published decode column was flattered by 0.10 to 0.18.

**And the win is not uniform, which the earlier column hid.** Decode converges
to parity, and past it, as host-side container construction per element rises:

| shape class | payloads | `managed-parse` / `gp-parse` |
|---|---|---|
| the real schema's own shapes | P1.1, P1.2, P2.1, P2.2, P2.5, P3.1, P4.1 | **0.73 - 0.87** |
| the absent path | P1.3 | 0.50 - 0.56 |
| container-dense variants of M2 | P2.3, P2.4 | 0.86 - 0.95, **approaching parity** |
| packed scalars, a CONTROL | P6.1 | **0.98 - 1.06, AMBIGUOUS: it straddles 1.0** |
| bulk, on the memcpy floor | P5.2 - P5.4 | 0.62 - 1.19, **AMBIGUOUS** |

So the defensible claim is narrower than "never at parity": **the managed
codec wins by roughly a fifth on every shape ArmoniK sends, and that win
erodes to nothing as an element's containers come to dominate.** On M6, which
`design/SHAPES.md` labels a control because the schema has no packed scalar
field, it straddles 1.0 and is no longer callable either way; the allocation
column says why it is the worst row -- the managed arm allocates 1.31 times
the incumbent there, five `List<T>` growths against `RepeatedField`.

**Two things R14's re-baselining did, and the second moved the column in this
slice's own favour, so it is stated rather than absorbed.**

- **Encode: nothing.** `gp-marshaller`, the stub's exact
  `CalculateSize()` + `WriteTo(IBufferWriter)` sequence, measures **0.96 to
  1.01 of `gp-writeto`** on every payload. Span against buffer-writer for the
  write half is noise; what matters is the size pass, and both carry it. The
  encode column is unchanged.
- **Decode: about 2 to 4 percent, towards the managed arm.**
  `ParseFrom(ReadOnlySpan)` measures **0.91 to 1.01 of
  `ParseFrom(ReadOnlySequence)`**, so the span path this slice used before is
  the marginally *faster* one and the production baseline is marginally
  slower. Moving to the correct denominator therefore flatters the managed
  column slightly. It is the right denominator regardless, but a re-baseline
  that helps you is the one to declare.

**And a sequence is not one buffer.** `gp-parse-seg`, the same payload as a
sequence segmented at 16 KB, is 0.96 to 1.01 of the single-segment form on
everything but the bulk rows. A 540 KB response does not arrive contiguous, so
the single-segment figure is the optimistic one; the cost of segmentation
turns out to be small, which had to be measured because it is also the buffer
shape ABI v1 decision 13's borrowed views would have to live in.

This is the Rust slice's convergence finding reproduced on a managed runtime,
and here it actually crosses 1.0 rather than merely tending towards it.

**What raises the value of this column: the Python slice measured the SAME arm
at 19.4 to 20.3 times upb.** A generated pure-language codec over the facade
loses catastrophically there and wins here. So this number is the difference
between "generate the codec" being a general recommendation and a
managed-runtime one, and the contrast is load bearing for the report in a way
neither slice is on its own. The .NET result is not evidence about Python and
the Python result is not evidence about .NET; what they jointly establish is
that the answer is a property of the host runtime's incumbent, not of the
approach.

## Arms

| arm | what it is | built |
|---|---|---|
| `gp-tobytearray` | `Google.Protobuf` `msg.ToByteArray()`, the call application code writes | yes |
| `gp-writeto` | `msg.CalculateSize()` then `msg.WriteTo(Span<byte>)` into a reused buffer. **The baseline, and it is the right one**: `Grpc.Tools`' generated marshaller calls `SetPayloadLength(CalculateSize())` and then `WriteTo(bufferWriter)`, so a gRPC client pays the size pass and cannot avoid it | yes |
| `gp-bufferwriter` | `msg.WriteTo(IBufferWriter<byte>)` with no size pass, over a reused `BufWriter` that resets rather than clearing. **Not a baseline an ArmoniK client can reach**; what it prices is **the size pass in isolation, 0.68 to 0.81 of an encode** | yes |
| `managed` | **the managed control**: a generated pure-C# codec over the facade, one pass, ABI v1 section 6's learned length width | yes, encode AND decode |
| `managed-2pass` | the same generated codec, `SizeOf` then `WriteSized`: two passes, exact prefixes. The shape `Google.Protobuf` uses | yes |
| `memcpy floor` | `Buffer.BlockCopy` of the payload's own bytes: R2's bound | yes |
| `gp-parse` | `Parser.ParseFrom(ReadOnlySpan<byte>)` | yes |
| `managed-parse` | `Codec.Read` into a fresh facade graph | yes |
| **`core-ffi`** | **the amended ABI, over the ONE core at `ffi/poc/codec` (R0)**, push decode | **YES for every shape**, all 16 payloads, on arm a. **NOT on arms b or c**, see below |
| **`core-ffi pull`** | ABI v1 7.1's PULL family: `ak_parse_*` writes a record stream and the host replays it, so the decode makes **no reverse call at all** | yes, every shape |
| **`core-ffi utf16`** | the other string form of ABI v1 section 4: the host hands over UTF-16 and `ak_tc_utf16` converts, against staging UTF-8 and letting `ak_tc_bytes` copy. Both transcoders are pointers INTO the core, so neither crosses | yes, every shape |
| **`core-ffi fill`** | the host-side half of the encode arm alone: zero the by-value group, stage every string, build the run arrays, stop before calling the codec. **The difference between it and `core-ffi` is the codec plus every crossing, measured rather than subtracted** | yes, every shape |
| **`core-ffi no-string`** | the decode with NO string materialised: **a CEILING for ABI v1 decision 13 and not an implementation of it**, so the gap to the real arm is the most a borrowed span could ever save | yes, every shape |
| **strict decode** | ABI v1 decision 3's REJECTING UTF-8 policy, as a third build (`/p:AkStrict=true`) for the same reason the floor is one: a runtime flag would branch on both arms' hot path and stop the JIT devirtualising `Encoding.UTF8` | yes, arm a |
| **the RPC arm** | a real grpc-dotnet client against a real grpc-dotnet server over a Unix domain socket, the server's marshaller a `byte[]` passthrough so only the client's codec varies. Four codecs: `gp-marshaller`, `managed`, `core-ffi`, `core-ffi pull`. ArmoniK's transport pinned, the stack default and loopback TCP as labelled rows, 1/8/16 in flight | yes, P2.2 |

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
src/Harness/BufWriter.cs   the incumbent's buffer-writer sink. Hand-written because
                           ArrayBufferWriter<T> is .NET Core 3.0+ and never reached the
                           net48 floor, and because its only reset moves the position
                           and never zeroes: a per-iteration wipe is the handicap an
                           adversarial review found in the C++ slice's incumbent
src/BenchDotNet/           the BenchmarkDotNet harness, for the controlled rerun
src/HarnessFloor/          arm c: the same sources LINKED, targeting net48, run on Mono
Directory.Build.props      arm b's output redirect, so a and b exist side by side
```

Commands: `harness conformance | unknown | counts | content | mapforms | bench`,
and `dotnet run --project src/BenchDotNet -- --filter '*'` for the
BenchmarkDotNet harness, which passes the whole BDN CLI through.
Builds: default (arm a), `/p:AkFloor=true` (arm b), `/p:AkCount=true` (the
counting build), `src/HarnessFloor` (arm c), `src/BenchDotNet` (the
BenchmarkDotNet harness the controlled rerun should use) and `src/Rpc` (the
end-to-end RPC arm).

Commands: `harness conformance | unknown | groups | corpus | utf8 | counts |
content | coreffi | mapforms | bench`, and `akrpc [--stack-default] [--tcp]
[--calls N] [--rounds N]`.

**The generator is one derivation with several backends, and that is load
bearing rather than tidy.** `gen/abi_ir.py` derives the by-value group, the
presence bits, the loop slots and which messages get a vtable, following
`ffi/poc/codec/gen/rust_abi.py`'s rules; `gen/rs_probe.py` emits the Rust layout
probe from it, `gen/cs_abi.py` the managed declaration and `gen/cs_core.py` the
host binding. Hand-writing any two of those three produced two defects in one
work unit. `gen/protoparse.py` is a second front end over `corpus.proto`, and it
is cross-checked against `shapes.json` at generation time on the nineteen
messages they share.

## What is measured

Ranges are across **three separate processes** unless stated. Ratios are formed
inside one process (R4). ASCII unless stated.

### Decode: the number this slice exists for

Against `gp-parse`, in the same process, on the target (arm a):

| payload | shape | `managed-parse` / **`gp-parse-seq`** | `gp-parse` (span) / seq | `gp-parse-seg` (16 KB) / seq |
|---|---|---|---|---|
| P1.1 | M1, 4 flat | 0.730 - 0.764 | 0.978 - 1.001 | 0.985 - 0.991 |
| P1.2 | M1, 1000 flat | 0.729 - 0.776 | 0.958 - 0.990 | 0.975 - 1.015 |
| P1.3 | M1, absent path | 0.501 - 0.561 | 0.910 - 0.981 | 0.942 - 0.993 |
| P2.1 | M2, 1 | 0.736 - 0.814 | 0.975 - 1.001 | 0.994 - 1.013 |
| **P2.2** | **M2, 500: the shape the control plane moves** | **0.745 - 0.871** | 0.964 - 1.004 | 0.964 - 1.010 |
| P2.3 | M2, 30 repeated strings/field | 0.863 - 0.897 | 0.927 - 0.964 | 0.958 - 0.970 |
| P2.4 | M2, alternating 3/150 | 0.881 - 0.950 | 0.924 - 0.992 | 0.957 - 1.054 |
| P2.5 | M2, absent path, nested | 0.772 - 0.831 | 0.986 - 1.005 | 1.000 - 1.002 |
| P3.1 | M3, oneof + explicit presence | 0.775 - 0.831 | 0.985 - 1.007 | 0.971 - 0.998 |
| P4.1 | M4, the adapter site | 0.797 - 0.833 | 0.976 - 0.998 | 1.005 - 1.006 |
| P6.1 | M6, packed (a CONTROL) | **0.981 - 1.057 AMBIGUOUS** | 0.989 - 0.998 | 0.996 - 1.005 |
| P7.1 | M7, interleaved control | 0.576 - 0.642 | 0.920 - 0.961 | 0.964 - 1.004 |
| P5.1 | M5, 36 B | 0.677 - 0.687 | 0.877 - 0.938 | 0.972 - 1.021 |
| P5.2 | M5, 64 KB | 0.815 - 0.991 **AMB** | 0.945 - 1.118 | 0.884 - 1.073 |
| P5.3 | M5, 1 MB | 0.921 - 0.938 **AMB** | 0.694 - 1.184 | 0.770 - 0.823 |
| P5.4 | M5, 4 MB | 0.616 - 1.185 **AMB** | 0.875 - 1.476 | 0.641 - 0.993 |

**Allocation, unchanged by the re-baseline**: the managed arm allocates 0.91 to
1.00 of the incumbent on every row but P6.1, so the two build object graphs of
the same size and the win is not "it built less". **P6.1 is 1.31 times, five
`List<T>` growths per element against `RepeatedField`, and it is also the only
row that reaches parity.** The two facts belong together.

**The M5 rows are a ratio between two copies and are marked AMBIGUOUS.** A bulk
decode is a copy, the memcpy floor sits far below on those rows, and the two
arms differ only in what they copy INTO: the managed arm a `byte[]`, the
incumbent a `ByteString`. P5.4 spans 0.558 to 1.096 across three processes in
one log, which is the spread saying the row does not support a ratio at all.
This is R2's lesson in the shape the Rust slice met it in, one direction over:
there a 0.08 needed a floor arm before it could be reported, here a 1.1 does.

**The Rust slice's convergence finding reproduces, and here it CROSSES 1.0.**
Decode approaches parity in proportion to host-side CONTAINER construction per
element, not to bytes or strings: the flat shapes sit at 0.72 to 0.82, the
container-dense ones (P2.3 with 120 repeated strings per element, P2.4) at 0.87
to 0.96, and P6.1 with five packed lists at 1.02 to 1.03. A list growth and a
map insert are work every arm does identically, so the denser the element's
container graph the smaller the share of decode any codec owns -- and on .NET
that share runs out before the containers do.

### Encode

Against `gp-writeto` (the fair baseline) and `gp-tobytearray` (what application
code writes):

Baseline is **`gp-marshaller`**, the stub's own
`CalculateSize()` + `WriteTo(IBufferWriter)` (R14).

| payload | `managed` | `managed-2pass` | `gp-writeto` (span) | `gp-bufferwriter` (no size pass) |
|---|---|---|---|---|
| P1.1 | 0.408 - 0.443 | 0.832 - 0.858 | 0.999 - 1.012 | 0.765 - 0.792 |
| P1.2 | 0.422 - 0.437 | 0.885 - 0.934 | 0.997 - 1.002 | 0.790 - 0.806 |
| P1.3 | 0.242 - 0.350 | 0.315 - 0.434 | 0.998 - 0.999 | 0.707 - 0.715 |
| P2.1 | 0.271 - 0.292 | 0.627 - 0.651 | 0.994 - 1.004 | 0.795 - 0.818 |
| **P2.2** | **0.284 - 0.296** | **0.722 - 0.737** | 0.995 - 1.007 | 0.787 - 0.811 |
| P2.3 | 0.354 - 0.367 | 0.782 - 0.822 | 1.002 - 1.006 | 0.744 - 0.827 |
| P2.4 | 0.393 - 0.406 | 0.777 - 0.838 | 0.996 - 1.008 | 0.733 - 0.849 |
| P2.5 | 0.273 - 0.280 | 0.652 - 0.660 | 0.992 - 1.000 | 0.791 - 0.811 |
| P3.1 | 0.335 - 0.365 | 0.639 - 0.647 | 0.998 - 1.014 | 0.685 - 0.720 |
| P4.1 | 0.228 - 0.239 | 0.608 - 0.612 | 0.997 - 1.004 | 0.792 - 0.803 |
| P6.1 | 0.309 - 0.332 | 0.610 - 0.623 | 1.001 - 1.004 | 0.808 - 0.814 |
| P7.1 | 0.351 - 0.377 | 0.570 - 0.595 | 0.962 - 0.993 | 0.747 - 0.756 |
| P5.1 | 0.455 - 0.511 | 0.664 - 0.757 | 0.969 - 0.995 | 0.790 - 0.807 |
| P5.2 | 0.923 - 1.048 **AMB** | 0.978 - 1.051 **AMB** | 1.017 - 1.060 | 0.963 - 1.024 |
| P5.3 | 0.964 - 1.031 **AMB** | 0.969 - 1.039 **AMB** | 0.991 - 1.014 | 0.997 - 1.015 |
| P5.4 | 1.007 - 1.032 **AMB** | 1.005 - 1.012 **AMB** | 0.985 - 1.000 | 0.992 - 0.997 |

**`gp-writeto` is `gp-marshaller` to within 1 percent on every row**, which is
what says the re-baseline did not move the encode column: the write half, span
against buffer-writer, is noise, and the size pass that both carry is the
whole of it.

**`gp-bufferwriter` is the size pass, priced.** It is 0.69 to 0.85 of the
baseline on every non-bulk row, and that gap IS `CalculateSize()`. It is not a
fairer baseline: `Grpc.Tools`' marshaller requires the length before the frame
header, so an ArmoniK client pays it. What the arm establishes is that **a
single-pass codec that buffers its own output and reports the length
afterwards is skipping real, unavoidable work in the incumbent's call path**,
which is what makes the `managed` column a statement about the design rather
than about the baseline. JOURNAL.md entry 16 is the correction that got here.

**`managed` / tba reproduces the published 0.22 to 0.43** on the non-bulk
payloads: measured 0.21 to 0.37. That is prior art reproduced on a rebuild
from the design documents with no access to the original sources, and it is
the one published figure this slice can check.

**R2's decomposition, and it is new.** `managed` against `managed-2pass` is a
within-arm delta in the same interleaved rounds, and it separates the two
halves of the encode win: the single pass with a learned length width is worth
about a factor two and a half (P2.2: 0.282-0.284 against 0.721-0.723, so one
pass costs 0.39 of two),
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
| P1.2 encode `managed`/inc | 0.418 | 0.535 | 0.588 |
| P2.2 encode `managed`/inc | 0.313 | 0.395 | 0.445 |
| P1.2 decode `managed`/inc | 0.799 | 0.846 | 0.881 |
| P2.2 decode `managed`/inc | 0.817 | 0.885 | 0.885 |

In R4's within-arm form, on wide: `managed` encode costs **2.12 to 2.22 times
its own ASCII self**, `gp-writeto` **1.49 to 1.58**, `managed-2pass` **1.56 to
1.68**. The single-pass arm has a higher share of its time in the transcode,
which is the part that scales with output bytes.

**So the encode headline is content-dependent and the decode headline is
nearly not.** A slice quoting only the ASCII encode number overstates the win
by about a third on the hardest content; decode moves by 0.07 to 0.08 and
stays a win throughout.

### The `core-ffi` arm (`stage8-core-ffi.log`), M1 only

The arm that was held. Built against `ffi/poc/rust/target/release/libak_core.so`,
the same artifact the Rust slice loads, with a generated binding: P/Invoke via
`LibraryImport`, reverse calls via `[UnmanagedCallersOnly]` with the abort
guard, and the by-value group declared at the offsets the Rust build reports.

**Gated first**: byte identity on P1.1, P1.2 and P1.3 including the absent
path, decode re-encoded to the same bytes, and the decoded graph compared
field by field against the one the builder made.

**Re-gated on the shared core (W10, `stage9-shared-core.log`).** That core is a
different artifact and not just a different path: default features exclude
`rpc`, so it is 618 KB with 66 `ak_` exports against the Rust slice's 3,070 KB
and 68. **Nothing moved** -- worst move 0.035 against 0.026 on arms the core
cannot touch, no consistent sign, so run-to-run drift.

**The loaded artifact is confirmed from the dynamic linker, not the build
log**, and that was needed twice: once a failed build left a stale core in the
output directory, and once **arm c failed to build with 172 errors while Mono
ran a three-hour-old binary and reported a pass**. A build log would have shown
neither.

**Arm c cannot carry this arm at all, and that is a floor finding.** .NET
Framework 4.8 has no `LibraryImport` (.NET 7+) and no `UnmanagedCallersOnly`
(.NET 5+), so the binding as generated does not compile there. A floor binding
would be `DllImport` plus delegate pointers, and **the delegates must be rooted
for the lifetime of the vtable or the collector reclaims a thunk the codec
still holds** -- a crash, not a slowdown. The project excludes it explicitly so
arm c states what it covers.

**Crossings are constant in the element count, in both directions.**

| direction | forward | reverse | why |
|---|---|---|---|
| encode | 2 (`ak_encode_*`, `ak_elem_*`) | 1 (`loop_results`) | the whole run goes over in one `ak_elem_*` |
| decode | 1 (`ak_decode_*`) | 2 (`apply`, `add_results`) | the whole run comes back in one `add_results` |

Four elements or a thousand, the count is the same. `ResultRaw` is a leaf, so
ABI v1's batching predicate admits it. .NET crosses at 7.5 to 12 ns, far above
the C++ slice's 2 to 4 ns crossover, so batching is not a close call here.

**The interface cost**, `core-ffi` against the no-boundary managed control,
three processes:

| payload | encode | decode |
|---|---|---|
| P1.1, 4 elements | 1.330 | 1.062 |
| P1.2, 1000 elements | 1.148 | **0.899** |
| P1.3, the absent path | **2.361** | **1.981** |

**Two findings, pointing opposite ways, and both matter more than the encode
column.**

**On P1.2 decode, crossing the C ABI is FASTER than the pure managed codec**:
0.899 of it, and 0.651 to 0.659 of `Google.Protobuf`. The Rust parser plus
three crossings beats a C# parser doing the same work. **That is the opposite
of the Java slice**, where a generated pure-Java codec beat the C ABI in both
directions and the case had to rest entirely on maintenance. At .NET's
crossing price the interface does not eat the core's advantage.

**The absent path collapses, and the cause is ABI v1 decision 9.** P1.3 is 300
elements that each encode to nothing. The host fills 300 by-value groups of
200 bytes each whatever is in them: 60 KB of stores to describe 605 bytes of
output. **Decision 9's sparse fill is specified and is NOT built here**, and
this arm is what prices it on .NET. The Rust slice measured the same effect
from the other side, its zeroed-group variant at 0.719 to 0.766 of the total
fill on exactly this payload.

**CORRECTED BY STAGE 11, and the correction widens it.** The cause above was
reached by elimination, and elimination was right about the mechanism and wrong
about the scope. A `core-ffi fill` arm now measures the host-side half directly:
it is **62.6 percent of P1.3's encode, and also 40 percent of P1.1's and
P1.2's, where nothing is absent at all.** The group fill is not a property of
the absent payload. It is a property of the group being filled whole, and P1.3
only makes it visible by having nothing else in the encode to hide behind. See
the M2 section below, where it is 45 to 57 percent and decides the arm.

The interface term also shrinks with density, 1.330 to 1.148 on encode from 4
elements to 1,000, which is the fixed three crossings amortising.

**Strings are staged, and that is a .NET decision with arithmetic behind it.**
`ak_str` offers two forms: point `data` at the host's own representation and
supply a `tc` callback, or transcode up front and point at UTF-8 with
`tc = ak_tc_bytes()`. The second is built, and crosses nothing, because
`ak_tc_bytes` is a function pointer INTO the core. The first would be one
reverse crossing per string: 5 per `ResultRaw`, 5,000 for P1.2, which at 7.5
to 12 ns is 37 to 60 us against a whole managed encode of about 210 us. Not
built, and named as an arm rather than argued away.

**The abort guard costs one `try/catch` per message, not per field**, because
the group is by-value and the element is a leaf. On a shape where the group
does not reach every field that stops being true, which is another reason M2
is the interesting one.

**The M1 figures above were re-taken in stage 11 against the rebuilt core** (86
exports, 800 KB, still no `rpc`) and moved a little: encode 1.330 to 1.319 on
P1.1, 1.148 to **1.250** on P1.2, 2.361 to 2.503 on P1.3; decode 1.062 to 1.053,
0.899 to **0.863**, 1.981 to 2.077. Same signs, same readings, third decimal
only. Quote the stage 8 figures for the M1 story and the stage 11 ones when
comparing with M2, because only the latter were taken in the same processes.

### The `core-ffi` arm on M2 (`stage11-core-ffi-m2.log`), and what it overturns

`TaskDetailed` is **not a leaf**: four repeated string fields and a map, so ABI
v1 section 6's batching predicate does not hold. Gated the same way as M1 --
byte identity, re-encode and field-by-field value identity -- on **all five M2
payloads, P2.1 through P2.5, first run**.

**The crossings match the rust slice to the digit**, and R5 is now CHECKED
rather than asserted: the gate runs against a `--features count` core and
compares the host's own tally with `ak_enc_counters` and `ak_dec_counters` per
payload and per direction, counting a mismatch as a failure. They are equal on
every row.

| direction | per element | rust slice |
|---|---|---|
| encode | 5 forward + 5 reverse = **10.00**/task | 10.02/task |
| decode | **7.00** reverse/task, plus 1 forward for the response | 7.004/task |

**THE TWO CROSSING COLUMNS ARE THE SAME QUANTITY AND ALWAYS WERE.** Say it
plainly, because this slice once said otherwise in prose and the report must not
repeat it. The only quantity the core counts that a host tally cannot see is a
transcoder invocation, and this binding stages its strings so it makes none. The
gap once reported against the rust slice was a CHUNK SIZE: their host hands over
150 elements per element call, this one hands over the whole run. `AK_CHUNK=150`
reproduces their counts to the digit and costs 0.7 percent, inside the spread.

**The interface cost**, `core-ffi` against the no-boundary managed control,
ratio computed inside each process:

| payload | encode | decode |
|---|---|---|
| P2.1, 1 element | 1.941 | 1.143 |
| P2.2, 500 elements | 1.701 | **1.041** [0.923, 1.052] |
| P2.3, 125 x 30 repeats | 1.685 | 1.051 |
| P2.4, mixed 3/150 | 1.686 | 0.966 [0.861, 1.042] |
| P2.5, half absent | 1.851 | 1.128 |

**1. THE PUBLISHED DECODE CLAIM DOES NOT SURVIVE M2, AND IT DOES NOT REVERSE.**
".NET's composed arm beats its own managed codec on decode" was 0.899 on M1/P1.2
and re-reads 0.863 against the current core. On M2 it is **0.97 to 1.14**, every
spread touching or crossing 1.0. BenchmarkDotNet was run three times on P2.2 and
read 0.900, 0.963 and 0.987, overlapping the interleaved harness's 0.923 to
1.052. The first BDN run's 0.900 is exactly M1's figure and taking it as the
answer would have confirmed the claim from the bottom of a spread; that is why
it was re-run. **The honest statement: on a non-leaf element the composed arm
and the pure managed codec are INDISTINGUISHABLE on decode.** Both stay well
under the incumbent -- core-ffi 0.76 to 0.92 of `gp-parse-seq`, managed-parse
0.76 to 0.89 -- so the story against `Google.Protobuf` is unchanged. What
changed is the story against the design's own control.

The mechanism is section 7.2's refusal: 7 reverse calls per element against 1
for a whole M1 response, each carrying a `GCHandle` resolve and a `castclass`
because `[UnmanagedCallersOnly]` cannot capture and there is no cheaper way to
reach the target graph from a native pointer.

**2. ON ENCODE THE COST IS NOT THE CROSSINGS. IT IS THE GROUP FILL, AND THE
CORE'S OWN WORK IS FASTER THAN THE MANAGED CODEC.** Ten crossings a task at this
slice's own measured .NET price (7.5 to 12 ns) is 75 to 120 ns; the gap between
`core-ffi` and `managed` on P2.2 is 699 ns a task. The crossings are 11 to 17
percent of it. A `core-ffi fill` arm -- everything the encode arm does except
calling the codec -- says where the rest is:

| payload | core-ffi | fill only | fill share | codec + crossings | vs managed |
|---|---|---|---|---|---|
| P1.2 | 195.2 us | 78.5 us | 0.402 | 116.7 us | **0.737** |
| P1.3 | 5.80 us | 3.62 us | **0.626** | 2.17 us | 0.939 |
| P2.2 | 850.8 us | 383.1 us | 0.450 | 467.7 us | **0.933** |
| P2.3 | 620.5 us | 344.4 us | 0.557 | 276.2 us | 0.753 |
| P2.4 | 851.4 us | 489.9 us | 0.572 | 361.6 us | 0.716 |

BenchmarkDotNet reproduces the shares: 0.412, 0.642 and 0.466 on P1.2, P1.3 and
P2.2. **On six of the eight payloads the Rust codec plus every crossing is
BELOW the whole managed encode**, and the arm loses anyway, because of what the
host must do to feed it. On P2.2 the fill alone is 76 percent of the entire
managed codec. **That is ABI v1 decision 9's sparse fill, and this is the
measurement that makes it the amendment worth building rather than a nicety.**

**3. The pull family is the arm to build next, on this slice's own evidence.**
M1 decode is 1 upcall per response and beats the managed control; M2 decode is
7.00 upcalls per element and ties it. That is the same correlation the java
slice reports at 7.004 upcalls, from an independent binding on a different
runtime. The codec already emits a pull family (`dec_*_pull`); nobody has bound
one from a managed host.

**The BDN harness gained the R14 baselines in this stage, and it did not have
them.** `gp-marshaller` on encode and `gp-parse-seq` on decode were in the
interleaved harness only. The harness the controlled rerun is meant to use would
have come back without the column the report quotes against.

**One reproducible GC difference, recorded and not chased.** The two P2.2 decode
arms allocate the same 2.02 MB per operation and build the same graph, but Gen1
collections per 1,000 operations are 85.9 for `core-ffi` against 119.1 for the
managed control, Gen0 identical at 121.1, in all three BDN runs. Fewer
promotions for the same bytes. A GC difference moves with the heap and should
not be quoted as a time.

### Layout agreement is tested, which no earlier slice could do

`abi/` is a small Rust bin depending on `ak-abi` read-only that prints every
group's size, alignment and field offsets as JSON; the generator emits C#
structs at those explicit offsets and asserts against them. "ok: 8 structs
match the Rust build; core ak_abi_version()=1".

**Its limit is a gap in the ABI, not in this slice, and it is a request.** The
core exports `ak_abi_version` and **no layout**. So this verifies the managed
declaration against the Rust SOURCE at generator time plus the version at
load; a core rebuilt with a changed group layout and an unchanged version
number would pass and then fail as a wrong payload. ABI v1 obligation 12.3
asks the CORE for a layout export and there is not one.

### The two harnesses agree (`stage7-benchmarkdotnet.log`)

144 BenchmarkDotNet benchmarks against the hand-rolled harness's three
interleaved processes, same arms, same build, same machine. 64 comparable
rows.

| | |
|---|---|
| median deviation, BDN minus hand-rolled | **+0.019** |
| mean deviation | **+0.021** |
| within +/-0.05 | 53 of 64 |
| largest deviation | 0.111 (P5.1 encode `gp-bufferwriter`, a 116-byte payload) |
| **verdict flips (which side of 1.0)** | **2 of 64** |

**So the hand-rolled numbers stand, and BenchmarkDotNet is the conservative
one.** 47 of the 64 deviations are positive, meaning BDN reports the managed
arms slightly WORSE. That direction matters: the interleaved harness is mildly
optimistic for this slice's own arms, not flattering by accident in its
favour. The bias is about +0.02 and its mechanism is **not established** --
overhead subtraction would push the other way, so the plausible candidate is
the shared GC heap and warm state that one interleaved process gives every arm
and BDN's per-benchmark isolation does not. Not chased, because absolutes are
deferred.

The two flips are both within 0.09 of parity and neither changes a
conclusion:

- **P2.4 decode, 0.928 hand-rolled against 1.020 under BDN.** Under the more
  rigorous harness the container-dense row crosses into a loss, which
  **strengthens** the convergence finding rather than contradicting it: under
  BDN both P2.4 and P6.1 are losses.
- **P5.4 encode `managed-2pass`, 1.015 against 0.990.** A bulk row, already
  marked AMBIGUOUS, sitting on the memcpy floor where the ratio is between two
  copies.

Every other row keeps its side of 1.0 in both harnesses, including the whole
real-schema decode column.

### README 5.2's three arms

All three build from ONE source tree with one define flipped, and **all three
pass the same 120 correctness checks**, which is section 5.1's binding
condition on a floor that is different code from the target.

| arm | build | runtime | result |
|---|---|---|---|
| a | target | .NET 8.0.31 | the tables above |
| b | **floor** | .NET 8.0.31 | **b/a = 0.944 to 1.092**, straddling 1.0 in both directions on every row |
| c | floor | **Mono 6.8.0.105, net48** | stands alone, never a ratio against a |

Arm b is the only fair floor-against-target ratio and it says **the floor's
missing APIs cost nothing detectable here**. b/a straddles 1.0 in both
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

**Zero in every managed arm**, which is a fact rather than an omission: they
are in-process managed code and cross nothing.

**For `core-ffi`, M1: 2 forward + 1 reverse on encode, 1 forward + 2 reverse
on decode, CONSTANT in the element count.** Counted by the host, which is the
half R5 asks a host for.

**RESOLVED (`stage10-crossing-reconciliation.log`), and not the way it looked.
The conventions never differed.** This slice now reads the CORE's own
`ak_enc_counters`, which is the counter the Rust slice reads, and the
difference is **chunk size** -- a host choice the ABI leaves open.

The Rust log reports forward 2, 3 and 8 for 4, 300 and 1000 elements.
Subtracting the one `ak_encode_*` call leaves 1, 2 and 7 `ak_elem_*` calls,
which is exactly `ceil(n/150)`: **the Rust host chunks its run at 150
elements and this slice's hands the whole run over in one call.** Set
`AK_CHUNK=150` and this slice reproduces 2 / 8 / 3 forward and 1 / 1 / 1
reverse, to the digit.

**So the cross-language table is comparable**, provided each column states its
chunk size. It is not a report-level defect.

**And the extra crossings cost nothing measurable on .NET**: P1.2 encode is
156.83 ns/element at 2 crossings and 157.95 at 8, 0.7 percent apart and inside
the spread. Six crossings over a 218 KB payload is about 60 ns in total. "Make
the crossings fewer, not cheaper" is a rule about crossings that scale with
FIELD count -- the drafted ABI's 15,137 to decode a thousand rows -- not about
2 against 8. Once the batching predicate admits a message, **chunk size on
.NET should be chosen for memory, not for crossings**: whole-run needs a group
array proportional to the element count (200 KB on P1.2), chunked needs 30 KB
whatever the payload.

## Correctness

- **136 checks, 0 failures, on all three of README 5.2's arms.** Byte identity
  against `ffi/schema/generated/manifest.json` for all FIVE encode arms on all
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
- **The five GROUP vectors from `ffi/corpus` pass on all three arms**
  (`harness groups`, `stage12-group-skip-and-d7-regate.log`), and they did not
  before. `Dec.Skip` had cases for the four wire types a proto3 schema produces
  and `ErrMalformed` for everything else, so it rejected `U-root-group`,
  `U-nested-group` and `U-oneof-group`, which `Google.Protobuf` accepts. **Byte
  identity against `ffi/schema` could never have found this**: every payload
  there is emitted from the description the decoder is emitted from, and proto3
  cannot express a group. The shared core had the identical hole (D7); the C++
  slice still does; java's was already right.
  The fix matches the END_GROUP's FIELD NUMBER rather than counting depth, and
  is bounded at 100 nests. `MapForms.Skip`, the harness rewriter, has the same
  fix; the argument for leaving it alone was that it only walks bytes this slice
  emitted, which is true and is also what was believed about the facade's.
- **Three WRONG fixes were each built and seen failing**, on the real `Wire.cs`
  rather than a re-implementation, and **no one gate catches all three**:

  | wrong implementation | `conformance` | `groups` | `unknown` |
  |---|---|---|---|
  | no group case at all | 0 fail | **3 fail** | 0 fail |
  | group case, END_GROUP field number not matched | 0 fail | **2 fail** | 0 fail |
  | group case added OVER the 32-bit arm | 0 fail | 0 fail | **1 fail** |

  The third is the aggregating session's own near-miss in the core, and it
  reproduces here exactly: byte identity AND the corpus group vectors both pass
  a decoder that has silently lost wire type 5, and the only thing that catches
  it is this slice's hand-built unknown-field suite -- the one it would have
  been easiest to retire on acquiring a corpus. Byte identity reaches no unknown
  field, the corpus vectors reach wire type 3, the hand-built ones reach 0, 1, 2
  and 5. Keep all three.
- **The depth bound is load bearing and the cheap case does not show it.** With
  the bound removed, 200 and 20,000 nests still REJECT, as `ErrTruncated`
  instead of `ErrDepth`: the buffer runs out before the stack does. At 200,000
  the process prints `Stack overflow.` and aborts with SIGABRT, which .NET
  cannot catch. The gate now carries both depths -- the small one checks the
  error code, the large one checks there is still a process to report it.
- **This slice IS a corpus consumer** (`stage13-corpus-consumer.log`). All 336
  vectors of `ffi/corpus` run, on all three arms, which report the identical
  line. The codec is generated from `generated/corpus.proto` and never from
  `corpus_superset.proto` (rule 0), over the SAME `Enc`/`Dec`/`W`/`OrderedMap`
  the measured arms use. It cost a second generator front end
  (`gen/protoparse.py`), because this generator drives off
  `ffi/schema/emit/shapes.py` and had no `.proto` reader at all; the two front
  ends are cross-checked against each other at generation time on the nineteen
  messages and three enums they share, and a disagreement fails the generator.

  **It found four defects, every one of them in code that every other gate this
  slice owns was passing:**

  | vector | defect |
  |---|---|
  | `X-tag-zero` | field number 0 was accepted and skipped as an unknown field. Zero is what a reader gets from a buffer it forgot to bounds-check, so accepting it turns a truncation into a silently empty message |
  | `X-depth-101`, `X-depth-300` | **no recursion limit.** 300 levels of nesting was 300 managed frames and a successful parse; every protobuf implementation caps at 100. ABI v1 open decision 7, which the design says no slice exercises |
  | `S-double-minus-zero` | the omit-when-zero rule was `!= 0.0`, and IEEE says `-0.0 == 0.0`, so a set field vanished. Must compare BITS. `Google.Protobuf` has the same hole and upb does not |
  | the group-skip hole | stage 12, on the same evidence |

  The depth limit is on the measured path and is **priced**: managed-parse
  against the unchanged incumbent arm is 0.725 before and after on P1.2, and
  0.800 to 0.763 on P2.2, which moved the wrong way for a cost. The encode
  control moved 0.435 to 0.428 over the same runs, so the spread is one to two
  percent and the change is inside it. No published figure moves.

- **32 vectors are open, and neither is a defect this slice can fix alone.**

  **The 31 `T-dec-*` vectors: malformed UTF-8 in a string field**, which
  CONTRACT.md says a conformant parser must reject. This codec accepts them:
  `Encoding.UTF8` substitutes U+FFFD. **So does `Google.Protobuf`** -- `harness
  utf8` runs the 15 root-site vectors through both and the incumbent accepts
  every one, returning the same character counts.

  That matters more than it looks. **The managed decode column is the single
  most valuable measurement in this slice, and if the incumbent validated where
  the control did not, part of the margin would be validation the control
  skips** -- which R14 makes a defect in the comparison and not a property of
  the design. It does not. The two arms are like for like and the decode figures
  stand. It is also a finding the corpus did not have: ABI v1 open decision 3's
  "rejected on decode" is a BEHAVIOUR CHANGE for .NET, not a description of it,
  and `new UTF8Encoding(false, throwOnInvalidBytes: true)` rejects all 15, so a
  validating managed arm is one constructor argument away and pricing it is what
  decision 3 asks for.

  **`U-map-entry`: an unknown field inside every map entry.** This decoder skips
  it and keeps the entry; the corpus's projection puts the whole entries under
  `_unknown` and leaves the map absent. `Google.Protobuf`, on the same bytes,
  keeps the map -- printed in the log. Two implementations against the
  projection, so it is raised as a question about the vector (request 5).

- **The incumbent is wired in as an independent oracle.** Nineteen of the
  corpus's thirty roots exist in `shapes.proto` too, so the runner points
  `Google.Protobuf` at the same bytes by descriptor name: **accept/reject agrees
  on all 169 vectors where both have the type.** The corpus's expectations were
  computed with upb, and where this slice and the corpus disagree the question
  R14 asks is what the library ArmoniK actually runs does.

- **What corpus conformance does NOT claim here**, by name (CONTRACT.md section
  5 item 6): C5 (produce) for the corpus's own roots, because the payload
  builders cover `ffi/schema`'s roots only and `harness conformance` is that
  claim; the chunking class as chunking, because the managed codec does not
  batch and the core-ffi arm's binding does not reach corpus-only roots;
  `_unknown` comparison, which the contract makes optional and which this slice
  answers by saying it DROPS; a second decoder built from the superset; and 32
  open vectors, below.
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
| **C6** | `gen/cs_arms.py` | **the decode baseline did a full extra traversal.** `GpParse` ended `return m.CalculateSize()`, written to stop the decoded graph being optimised away, which walks the whole decoded tree; `ManagedParse` returned a free offset. The incumbent paid a size pass the managed arm did not, **on the one column this slice exists to produce** | **fixed**: both arms park the graph in a static sink and return an O(1) value. It was worth 0.10 to 0.18 of the decode ratio and every figure before `87a2e39` is withdrawn. The rule, which is not "check your baseline": whatever stops a result being optimised away has to cost the same in every arm |
| **C7** | this slice's reporting | `87a2e39` claimed `gp-writeto` was handicapped by a size pass the incumbent could avoid, and that its best path is 20 to 29 percent faster. **The measurement was right and the conclusion was wrong**: `Grpc.Tools`' marshaller calls `SetPayloadLength(CalculateSize())` then `WriteTo(bufferWriter)`, so a gRPC client cannot avoid it | **corrected** in JOURNAL.md entry 16. `gp-writeto` stays the baseline; `gp-bufferwriter` is relabelled as pricing the size pass in isolation. Found by the aggregating session asking whether the finding was a fact about what ArmoniK ships |
| C5 | `Directory.Build.props` | the floor build's output directory was globbed into the target build's compile items and vice versa, because the SDK excludes only the intermediate directory of the build currently running | **fixed**: both excluded in both directions |

## Requests to the aggregating session

Written here rather than edited into the documents, per the contract.

0. **The RPC arm's transport configuration on .NET, established from the runtime
   source rather than relayed.** The instruction is to pin ArmoniK's settings --
   2 MiB chunking, a 4 MiB stream window -- with the stack default as a labelled
   second row, and it names two things to get right. Both are now answered, and
   one of them does not apply to .NET while the other applies harder than stated.

   **The connection window is NOT a separate knob on .NET, and there is no trap
   here.** `Http2Connection` hardcodes `ConnectionWindowSize = 64 * 1024 * 1024`
   and sends a WINDOW_UPDATE at connection setup to raise it from RFC 7540's
   `DefaultInitialWindowSize = 65535`. It is not configurable and it does not
   depend on `InitialHttp2StreamWindowSize`. So the "raise only the stream window
   and the connection stays at 65,535" hazard is real for tonic/hyper and for
   grpc-java's `flowControlWindow`, and is not reachable on .NET: at a 4 MiB
   stream window the connection window is already sixteen times it.
   [Http2Connection.cs](https://github.com/dotnet/runtime/blob/main/src/libraries/System.Net.Http/src/System/Net/Http/SocketsHttpHandler/Http2Connection.cs)

   **Setting the window does NOT disable dynamic sizing, which makes a pin a
   FLOOR and not a cap.** `Http2StreamWindowManager` takes
   `_streamWindowSize = settings._initialHttp2StreamWindowSize` as its starting
   point and then doubles from there -- `Math.Min(MaxStreamWindowSize,
   _streamWindowSize * 2)` -- whenever the bandwidth-delay product warrants it.
   `WindowScalingEnabled => !DisableDynamicHttp2WindowSizing`, which is a
   separate switch and defaults to scaling ON. `MaxHttp2StreamWindowSize`
   defaults to 16 MB.

   **So "pinned at 4 MiB" on .NET needs two settings, not one**: the property
   AND `AppContext`'s
   `System.Net.SocketsHttpHandler.Http2FlowControl.DisableDynamicWindowSizing`
   (env `DOTNET_SYSTEM_NET_HTTP_SOCKETSHTTPHANDLER_HTTP2FLOWCONTROL_DISABLEDYNAMICWINDOWSIZING`),
   or the arm starts at 4 MiB and may be measuring 8 or 16 by the end of the run.
   The configuration line will state both, and the default row will state that it
   starts at 65,535 and scales.
   [Http2StreamWindowManager.cs](https://github.com/dotnet/runtime/blob/main/src/libraries/System.Net.Http/src/System/Net/Http/SocketsHttpHandler/Http2StreamWindowManager.cs),
   [GlobalHttpSettings.cs](https://github.com/dotnet/runtime/blob/main/src/libraries/System.Net.Http/src/System/Net/Http/GlobalHttpSettings.cs)

   **And one thing about `packages/csharp` that the pinning instruction runs
   into.** Neither side sets a window today: `GrpcChannelFactory`'s
   `GrpcChannelOptions` carries `Credentials`, `DisposeHttpClient`,
   `ServiceConfig` and `LoggerFactory` and nothing else, and the worker's Kestrel
   setup touches `Limits.Http2` only for `KeepAlivePingTimeout` on the TCP
   branch. The client also builds an `HttpClientHandler`, through which
   `InitialHttp2StreamWindowSize` is not reachable at all. So pinning 4 MiB in
   the RPC arm means the arm configures something the shipped C# client cannot,
   which is worth one line in the report rather than a silent divergence.
   **UDS, by contrast, is what the shipped client already does**: `GrpcChannel`
   defaults `SocketType` to `UnixDomainSocket` at `/tmp/armonik.sock` and the
   worker calls `ListenUnixSocket(... HttpProtocols.Http2)`.

5. **`U-map-entry`'s projection disagrees with two implementations.** The vector
   is an unknown field inside every map entry, and its own `why` is right: a map
   entry is a message on the wire, so it has an unknown-field skip of its own.
   This slice skips the extra field and keeps the entry. **`Google.Protobuf`, on
   the same bytes, also keeps the entry** -- the runner prints its proto3 JSON
   beside the mismatch. The committed projection instead puts all four whole
   entries under `options._unknown` and omits the map. If that is upb dropping a
   map entry that carries an unknown field, it is worth recording as an upb
   behaviour rather than as the expectation; if it is the projector's handling of
   `_unknown` inside a map, the projection is wrong. Either way one of the two
   implementations that disagree with it is the library R14 names.

6. **The transcode class asks for a policy .NET does not have.** 31 `T-dec-*`
   vectors require a conformant parser to reject malformed UTF-8 in a string
   field. `Google.Protobuf` accepts all 15 root-site ones, so on .NET the
   incumbent is on the lossy side and ABI v1 decision 3's "rejected on decode" is
   a behaviour change rather than a description. Not asking for the vectors to
   change -- asking that the report say which languages the rejecting policy is a
   change FOR, because for C# it is one and the corpus currently reads as though
   every slice simply fails there.

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

2b. **The encode baseline is settled and it is the sizing path, which is a fact
   about what ArmoniK ships.** `packages/csharp` makes no direct message
   serialization calls at all -- the only two `ToByteArray()` hits are
   `ByteString`, copying a blob. Everything goes through gRPC's marshaller, and
   `Grpc.Tools` 2.66 emits `SetPayloadLength(message.CalculateSize())` followed
   by `WriteTo(context.GetBufferWriter())`. **So the size pass is required by
   the frame header and is not an avoidable inefficiency**, and
   `gp-bufferwriter` is a lower bound the product cannot reach rather than a
   fairer baseline. This corrects a claim this slice made in `87a2e39`; see
   JOURNAL.md entry 16.

3. **Two hazards the C# `core-ffi` arm will hit, recorded now so they are not
   rediscovered.** Neither is measurable without that arm and both were paid
   for elsewhere. (a) A managed exception inside `[UnmanagedCallersOnly]` does
   not propagate: it is a process abort. Every generated accessor needs the
   guard, and the published margins were measured without it. (b) On the floor
   there is no `UnmanagedCallersOnly` and no `SuppressGCTransition`, so the
   vtable is delegate pointers, and **the delegates must be rooted for the
   lifetime of the vtable or the collector reclaims a thunk the codec still
   holds** -- which is a crash and not a slowdown.

4. **ABI v1 decision 13 (borrowed string views) is a strong candidate arm here
   and is not built.** `ak_span` is already an offset into the host's own
   input buffer, so a decoded `string` could be a `ReadOnlyMemory<byte>` or a
   `ReadOnlySpan<char>` over a pinned buffer instead of a copy. On .NET that is
   a bigger change than in C++, because the host's native string type is
   UTF-16: a borrowed view over UTF-8 bytes is **not** a `System.String`, so
   either the facade's string type changes (and every consumer with it) or the
   borrow only pays on a field nobody converts. **That trade is the finding
   this slice would produce**, and it is worth a slice's work precisely because
   the answer is not obviously yes the way it is for C++. 174 of 413 fields are
   strings, so it is the largest single lever left.

5. **The managed control does three things less than the incumbent, and the
   decode win is partly that.** `Google.Protobuf` enforces a recursion-depth
   limit and a message-size limit, and retains unknown fields; the generated
   codec does none of the three. Two of those are ABI v1 open decisions 7 and 8
   and are unbuilt in the Rust slice too. The decode figures should carry that
   sentence wherever they are quoted.

## What is not measured

A pass for completeness, not for brevity (README R11).

### The arms that are not here

Every arm the brief scoped is built. What follows is what the finished ones
opened, and none of it is a gap in the brief.

- **Decision 9's sparse fill.** The largest remaining improvement to the C#
  encode column, and now priced on every shape: the host-side group fill is
  **28 to 71 percent of the whole `core-ffi` encode**. The ABI does not offer a
  sparse fill; what is measured is the cost of not having one.
- **A true zero-copy string form.** Both forms the ABI offers are built and are
  indistinguishable (0.96 to 1.04), because both transcoders are pointers into
  the core. The unbuilt one hands the core a pointer into the managed heap,
  which on .NET needs a pinned `GCHandle` per string -- 5,000 for P1.2 -- and is
  named rather than assumed equivalent.
- **The pull family's memory** is now measured, not omitted: `ak_bdr_footprint`
  puts the record buffer at 0.36 to 1.6 times the wire payload on the real
  shapes, and 63x on the absent path, where 300 elements that encode to nothing
  still carry 300 groups through the record stream.
- **Decision 13 itself**, as opposed to its ceiling, which stage 16 puts at 42
  to 62 percent of a decode. The facade's public surface and the lifetime rule
  are the work; the ceiling is what says whether to do it.
- **Streaming**, and the core's own tonic stack as the other end of the RPC arm.
  `design/SHAPES.md` lists streaming as not in the arm and says it is where the
  concurrency invariant actually bites; no slice has touched it.
- **`core-ffi` on the floor.** Arms b and c do not carry it, and that is a floor
  FINDING rather than a build convenience: net48 has no `LibraryImport` (.NET 7+)
  and no `UnmanagedCallersOnly` (.NET 5+), so the binding as generated cannot
  compile there at all. A floor binding would be `DllImport` plus delegate
  pointers, and the delegates must be rooted for the lifetime of the vtable or
  the collector reclaims a thunk the codec still holds: a crash, not a slowdown.
  `Core_*.cs`, `CoreArms.cs` and `CoreGate.cs` are excluded from arm c
  explicitly, so a stale binary cannot report a pass.
- **The abort guard priced.** It is present on every reverse callback and there
  is now a shape where that is not one `try/catch` per message: M2 pays five per
  element on encode and seven on decode. What is not done is an arm with the
  guard REMOVED, which is the only way to price it, and removing it makes a
  managed exception a process abort rather than an error.
- **ABI v1 open decisions still untouched on .NET**: 4, 6, 8, 10 and 12. Decision
  1 is settled, 2 is answered here (pull, on a managed host), 3 is measured as a
  non-difference between the arms, 5 and 11 are in stage 1 and 2, 7 is now
  exercised (the recursion limit the corpus forced), 9 is priced, 13 is named.

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

**The slice is at a natural stopping point, not a finished one.** What exists
is complete and gated; what is missing is named below in the order I would do
it, and the first item is much the largest.

1. ~~`core-ffi` for M2.~~ **DONE** (`stage11-core-ffi-m2.log`): all five M2
   payloads gated on byte identity, re-encode and value identity; crossings
   matching the rust slice to the digit and R5 checked against a counting core;
   the published decode claim tested and found not to survive; the encode cost
   decomposed and attributed to the group fill rather than the crossings.
   **`core-ffi` for M3 to M7 is what remains.** M3 is the oneof and explicit
   presence, and the ABI half of both of this slice's two named gaps lives
   there.
2. ~~A PULL decode arm.~~ **DONE** (`stage14-all-shapes-and-pull.log`): built,
   gated on all 16 payloads with **zero reverse calls on every shape**, and
   **faster than push everywhere** (0.69 to 0.97). It moves the composed arm
   from just above to just below the managed control on decode. What remains of
   it is the MEMORY half: the record buffer is proportional to the payload and
   `ak_bdr_footprint` reports it, and this slice measured only the time.
3. **ABI v1 decision 13's borrowed span. THE LARGEST REMAINING LEVER IN THIS
   SLICE, and now bounded rather than asserted.** A no-string decode arm puts
   the ceiling at **42 to 62 percent of a decode** (`stage16`), which is larger
   than every codec difference this slice has measured put together. It is a
   ceiling and not a forecast: a borrowed view still records the offsets and a
   consumer that needs a real `string` pays anyway. But it says where the money
   is. The ABI is ready -- the decode side already hands the host `ak_span`
   offsets into its own buffer -- and the work is the facade's public surface
   and the lifetime rule that comes with it.
4. **Decision 9's sparse fill. The largest remaining improvement to the ENCODE
   column, and priced on every shape rather than two.** The host-side group fill
   is **28 to 71 percent of the whole `core-ffi` encode**; subtract it and the
   core's own work plus every crossing is below the managed control on most
   payloads. M6 is the low end at 0.278, because a packed run is a memcpy with
   no staging; P1.3, the absent path, is the high end at 0.705. **Not this
   slice's to build**: it is an ABI addition, and `ffi/CLAUDE.md` puts a change
   to existing behaviour through the aggregating session. What is measured is the
   cost of not having one.
5. ~~An RPC arm.~~ **DONE** (`stage15-rpc-arm.log`): grpc-dotnet both ends over
   a UDS, server marshaller a `byte[]` passthrough so only the client's codec
   varies, ArmoniK's transport pinned with the stack default and loopback TCP as
   labelled rows, 1/8/16 in flight. **Its headline is a deflation and belongs in
   the report**: the codec is worth about 10 percent of CPU per call end to end
   where the in-process column says 25. What survives is allocation, 8.6 percent
   below the incumbent on every row. **What remains of it**: streaming, which
   design/SHAPES.md says is where the concurrency invariant actually bites and
   which no slice has touched; and a `Dec` over `ReadOnlySequence`, which the arm
   identified as a real improvement and which this slice has not built.
6. ~~The host-transcoder string form.~~ **DONE, and the prediction was wrong
   about the mechanism.** There is no host transcoder in the sense stage 8 meant:
   `ak_tc_utf16` is a pointer INTO the core, like `ak_tc_bytes`, so neither form
   costs a crossing and the "5,000 reverse crossings for P1.2" arithmetic was
   about something that does not exist. Measured, the two forms are
   indistinguishable (0.96 to 1.04 on every payload). **A true zero-copy form is
   what remains**: hand the core a pointer into the managed heap, which on .NET
   needs a pinned `GCHandle` per string and is probably a worse trade.
7. ~~Reconcile the crossing-count convention with the Rust slice.~~ **DONE**:
   the conventions never differed, the gap was a 150-element chunk in the Rust
   host, and matching it reproduces their counts to the digit. Stage 11 makes
   this a CHECK rather than a claim: the gate compares the host tally with the
   core's own counters per payload and per direction and fails on a mismatch.
   See `stage10-crossing-reconciliation.log` and the M2 section above.
8. ~~Corpus conformance.~~ **DONE** (`stage13-corpus-consumer.log`): all 336
   vectors on all three arms, codec generated from `corpus.proto` under rule 0,
   via a second generator front end. It found three more defects on top of the
   group-skip hole -- tag zero accepted, no recursion limit, minus zero dropped
   -- every one in code every other gate was passing. **What remains of it**: a
   validating-decode arm (ABI v1 decision 3, one constructor argument, and the
   31 open `T-dec-*` vectors are what would close); C5 for the corpus's own
   roots, which needs builders this slice has no reason to write otherwise; and
   the chunking class, which needs a core-ffi binding for a corpus-only root.
9. ~~A rejecting decode policy (ABI v1 decision 3).~~ **DONE** (`stage16`), as a
   third build, `/p:AkStrict=true`. It **closes all 31 open corpus `T-dec-*`
   vectors** and **costs nothing measurable** (-4.2% to +3.6%, inside a control
   that itself moved 4.6% between the builds), because `Encoding.UTF8` already
   validates in order to know where to substitute and only the FALLBACK differs.
   **On .NET the argument against the rejecting policy cannot be performance.**
   It is a behaviour change, which this slice can price and cannot judge.
10. ~~A `Dec` over `ReadOnlySequence`.~~ **RETIRED WITH EVIDENCE, not built.**
   Instrumented, gRPC delivers a segmented body on **every single call** (3,960
   of 3,960), so the single-segment fast path is never taken at this payload
   size. But the flatten is 540 KB, which this slice's own `memcpy floor` arm
   measures at **12.8 us against ~4,500 us of CPU per call: under 0.3 percent**.
   A segmented reader means every read handling a boundary, risking the
   single-segment path every in-process arm uses, to recover a third of a
   percent of an RPC.
11. **Streaming**, which design/SHAPES.md says is where the concurrency
   invariant actually bites and which no slice in the branch has touched. **The
   only item on this list that is scope rather than a decision**, and the only
   one left that is this slice's to build.

Deliberately NOT on the list: more rounds to tighten a spread, a cold-start
column, and any attempt to make this container's absolutes comparable with
another container's.

### Read the numbers the way stage 14 measured them, not across stages

**A within-process ratio is sound; a cross-SITTING comparison of two of them is
not, to better than about ten percent.** Two arms that no change touched moved
0.800 to 0.729 (`managed-parse / gp-parse-seq` on P2.2) and 0.292 to 0.309
(`managed / gp-marshaller`) between two sittings on this container. So
`core-ffi / managed` on P1.2 decode reading 0.899 in stage 8, 0.863 in stage 11
and 0.978 in stage 14 is the container and not the binding, and I nearly
attributed the last move to the code.

Every same-sitting comparison stands, because each is computed against an arm
that ran in the same rounds: pull against push, utf16 against staged, fill
against whole, and every row within any one log's own table. **Nothing that
compares a number in one stage with a number in another should be read past its
first digit.**

### The original next-step list, superseded above but kept for its reasoning

1. **The `core-ffi` arm. It is no longer blocked.** The C++ slice settled ABI
   v1 open decision 1: the group costs the host, string-as-data is a win, and
   the batching predicate has a **crossover at a forward crossing of roughly 2
   to 4 ns** rather than a verdict. **.NET 8 crosses at 7.5 to 12 ns, well
   above that crossover, so batching should win here** -- and this slice's own
   R13 calibration is what lets that be checked rather than assumed. This is
   the largest remaining gap and everything else in the slice exists to be
   subtracted against it. It will hit hazards 3(a) and 3(b) above first: a
   managed exception inside `[UnmanagedCallersOnly]` is a process abort, and on
   the floor the vtable is delegate pointers that must be rooted for its
   lifetime or the collector reclaims a thunk the codec still holds.
2. **ABI v1 decision 13, the borrowed string view.** Request 4 above says why
   it is a genuinely open question on .NET rather than the settled win it is in
   C++, and why it is the largest lever left on a codec that is 174/413 strings.
3. **The RPC arm**, the largest hole, and purely scope. Its valuable half
   survives a controlled rerun: the crossing count per RPC (which should be two
   and not a function of field count) and whether a `Task` can be awaited
   without pinning a carrier thread. Its CPU-per-RPC column is exactly the kind
   of absolute the rerun will take properly.
4. **A rejecting decode policy as a second managed arm.** A within-process
   delta, which is the category to spend effort on now. Both arms here run the
   LOSSY policy; the Rust slice found validate-and-reject free to cheaper.
5. **P6.1's loss is worth one session on its own.** It is the only row where
   the managed codec is behind, the allocation column already names the cause
   (five `List<T>` growths against `RepeatedField`, 1.31x the allocation), and
   a pre-sized list or a pooled array would test it directly. `design/SHAPES.md`
   labels M6's scalar rows a control, so this is not a headline -- but "the
   generated codec loses where the host builds many small lists" is a
   transferable statement about managed hosts, and it is one measurement away.

Deliberately NOT on the list: more rounds to tighten a spread, a cold-start
column, and any attempt to make this container's absolutes comparable with
another container's. R13's one calibration run stands and is not to be tuned.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `ffi/logs/csharp/calibration-rust-crossing.log` | rustc 1.94.1 release, cdylib boundary, 4 vCPU Xeon 2.80GHz; the Rust slice's own bench binary, `AK_BENCH_ONLY` filtered to the crossing cases, five runs | **R13, and it is the first thing this slice did.** 1.8 ns forward and 2.1 ns fwd+reverse on THIS container, medians identical across five runs, with the boundary shown as a real dynamic import. Reproduces the Rust slice's container to the digit, so absolutes travel between the two |
| `ffi/logs/csharp/stage1-conformance.log` | .NET SDK 8.0.131, .NET 8.0.31, Google.Protobuf 3.28.3, Mono 6.8.0.105 | **The gate.** `gen/generate.py --check`, then **136 checks with 0 failures on EACH of README 5.2's three arms**, then the seven unknown-field vectors, then `harness mapforms`. Carries the decision-11 retention asymmetry, and P2.5's two encodings: **`Google.Protobuf` on .NET OMITS the empty map value** and produces the manifest's 19,632 B, so the split is prost and Google.Protobuf against protobuf C++ and upb, not managed against native. The +80 B form is built by rewriting the committed vector and both decoders normalise it back |
| `ffi/logs/csharp/stage2-counts.log` | as above, **`/p:AkCount=true`, the counting build** | Crossings are zero in every arm and why. ABI v1 open decision 5: zero warm misses everywhere but P2.4, which misses once per element and moves 979,181 of 979,465 bytes, with the site named |
| `ffi/logs/csharp/stage3-arms.log` | .NET 8.0.31, Google.Protobuf 3.28.3, arm a, no AkCount, ASCII, tiering and PGO ON at their defaults, three processes, seven interleaved rounds | **The main table.** All 16 payloads, five encode arms and three decode arms, with ns/element, allocation per operation and the memcpy floor. The decode column this slice exists for |
| `ffi/logs/csharp/stage4-floor-arm-b.log` | arms a and b, both .NET 8.0.31, round robin with a rotating order, six payloads, each binary carrying the incumbent as its in-process control | **README 5.2 arm b.** The floor's missing APIs cost nothing: b/a is 0.93 to 1.04 |
| `ffi/logs/csharp/stage4-floor-arm-c.log` | **Mono 6.8.0.105**, net48, floor sources linked from the same tree, six payloads, two runs | **README 5.2 arm c, standalone.** Passes the same 136 checks on the floor runtime; the design's advantage survives it and is larger on decode. Mono absolutes, quoted as absolutes and never as a ratio against arm a |
| `ffi/logs/csharp/stage5-content-sets.log` | arm a, .NET 8.0.31, all three content sets in ONE process, P1.2 and P2.2 | **The string path, which is 174 of 413 fields.** Wire widths matching the Rust slice (1.70/1.75 and 2.39/2.50), and the encode advantage narrowing from 0.31-0.43 to 0.45-0.61 while decode barely moves. Carries the set-definition defect and its correction |
| `ffi/logs/csharp/stage5-content-sets-floor.log` | arm b, otherwise as above | The sharp version of arm b: b/a is 0.947 to 1.062 where the transcoder actually has work |
| `ffi/logs/csharp/stage10-crossing-reconciliation.log` | the counting core (`--features count`), `ak_enc_counters` read from the host, at two chunk sizes | **R5's cross-slice reconciliation, resolved.** The conventions never differed; the Rust host chunks at 150 and this one did not. At `AK_CHUNK=150` this slice reproduces the Rust slice's 2/8/3 forward and 1/1/1 reverse exactly. Also prices the difference: nothing measurable, 0.7 percent |
| `ffi/logs/csharp/stage9-shared-core.log` | the ONE core at `ffi/poc/codec`, default features so no `rpc`; loaded path confirmed with `LD_DEBUG=libs`; three arms gated, three timing processes, plus a pre-move control | **The W10 re-gate.** 152 checks 0 failures on all three arms; the core-ffi arm green on M1; **nothing moved** (worst 0.035 against a 0.026 floor on arms the core cannot touch). Records that arm c cannot carry the core-ffi arm and why, and that a stale binary reported a pass before the timestamp was checked |
| `ffi/logs/csharp/stage8-core-ffi.log` | the arm through `libak_core.so`, shared-library linkage, generated binding, staged strings; correctness plus three timing processes | **The `core-ffi` arm, M1.** Byte identity and value identity on P1.1/P1.2/P1.3; layout agreement on 8 structs; crossings constant in the element count in both directions; the interface cost against the no-boundary control, including the two findings that point opposite ways -- the C ABI beating the managed codec on P1.2 decode, and the absent path collapsing on the total group fill |
| `ffi/logs/csharp/stage16-decision3-and-13.log` | a third build for the rejecting decode policy, each build carrying the incumbent as its in-process control; a no-string decode arm as decision 13's ceiling; `ak_bdr_footprint`; the RPC arm instrumented for sequence shape | **Three answers the branch did not have.** Decision 3's rejecting policy **closes all 31 corpus `T-dec` vectors and costs nothing measurable**, because `Encoding.UTF8` already validates and only the fallback differs, so the case against it cannot be performance. **Decision 13's ceiling is 42 to 62 percent of a decode** -- larger than every codec difference this slice has measured combined. Pull's record buffer is 0.36 to 1.6x the wire payload, and 63x on the absent path. A `ReadOnlySequence` reader is **retired with evidence**: every body is segmented and the flatten is still under 0.3 percent of an RPC |
| `ffi/logs/csharp/stage15-rpc-arm.log` | a real grpc-dotnet client against a real grpc-dotnet server over a Unix domain socket, server marshaller a `byte[]` passthrough so only the client's codec varies; ArmoniK's 4 MiB window and 2 MiB chunking pinned, stack default and loopback TCP as labelled rows; 1/8/16 in flight | **The RPC arm, and it is partly deflating.** The codec is worth about **10 percent** of CPU per call end to end where the in-process column says 25, so sizing the change from that column overestimates it two and a half times. The three codec arms are **indistinguishable from each other** at the RPC level. What survives the noise is allocation: every facade arm is **8.6 percent below** the incumbent on every configuration. Records that pinning a window on .NET needs the AppContext switch as well as the property, and that `packages/csharp` can set neither |
| `ffi/logs/csharp/stage14-all-shapes-and-pull.log` | the ABI declaration, layout probe and host binding all derived from one module; 42 structs and 28 vtables verified; a counting core for the crossing table; three interleaved processes | **core-ffi for every shape, and the PULL family on a managed host.** All 16 payloads gate on byte identity, round trip, value identity and R5. **Pull is faster than push everywhere** (0.69-0.97) and moves the composed arm from just above to just below the managed control on decode, which is design/ABI-v1.md decision 2's answer. **The two string forms are indistinguishable** (0.96-1.04): both transcoders are pointers into the core, so this slice's "a host transcoder costs a reverse crossing per string" was wrong about the mechanism. Corrects stage 8's M1 decode crossing count -- the run is NOT one callback, it is ceil(n/arena)+1 -- and establishes that cross-SITTING ratio comparisons on this container drift up to 9 percent |
| `ffi/logs/csharp/stage13-corpus-consumer.log` | all 336 vectors of `ffi/corpus`, codec generated from `generated/corpus.proto` under rule 0 via a second generator front end, over the same `Enc`/`Dec`/`W` the measured arms use; three arms | **Corpus conformance, and four defects byte identity structurally could not reach.** Tag zero accepted; no recursion limit (ABI v1 decision 7, which the design says no slice exercises); minus zero dropped because the omit-when-zero rule compared value and not bits, where `Google.Protobuf` has the same hole and upb does not; plus stage 12's group skip. Prices the depth limit at nothing measurable. **And it settles the decode comparison**: `Google.Protobuf` does NOT validate UTF-8 either, so the managed decode margin is not bought by skipping validation. Wires the incumbent in as an independent oracle: accept/reject agrees on all 169 vectors where both have the type |
| `ffi/logs/csharp/stage12-group-skip-and-d7-regate.log` | the core rebuilt with `poc/codec/gen/build.sh` after the D7 fix, loaded path and sha256 confirmed from `LD_DEBUG=libs`; all three arms re-gated | **The GROUP-skip defect, seen failing and then fixed.** `Dec.Skip` rejected three corpus vectors `Google.Protobuf` accepts, because it had no case for the deprecated group form. Carries the reverted-fix run (7 failures) as the proof the guard works, the field-number-match and depth-bound reasoning, and the statement that this slice is NOT a corpus consumer and what it would cost to become one. The D7 core itself moved nothing: 152 checks 0 failures on each arm |
| `ffi/logs/csharp/stage11-core-ffi-m2.log` | the ONE core rebuilt after the branch merge (86 `ak_` exports, 800 KB, still no `rpc`); a second `--features count` build for the crossing table; correctness, three interleaved processes over M1 and M2 together, and four BenchmarkDotNet runs | **The `core-ffi` arm on M2, and two corrections.** All five M2 payloads gated first run; crossings 10.00 and 7.00 per task against the rust slice's 10.02 and 7.004, with R5 now CHECKED against the core's own counters rather than asserted. **The published ".NET's composed arm beats its own managed codec on decode" does not survive a non-leaf element**: 0.97 to 1.14, both harnesses straddling 1.0. **And the encode cost is the group fill, not the crossings**: a `core-ffi fill` arm puts the host-side half at 40 to 57 percent of the whole encode on every payload of both shapes, which also corrects stage 8's reading of P1.3 as an absent-path effect. Adds the R14 baseline arms to the BDN harness, which did not have them |
| `ffi/logs/csharp/stage7-benchmarkdotnet.log` + `bdn-results/*.csv`, `*-github.md` | **BenchmarkDotNet 0.15.8**, defaults, each benchmark in its own process, 144 benchmarks (16 payloads x 6 encode arms + 16 x 3 decode) | **The harness the CONTROLLED RERUN should use, and the cross-check that makes the hand-rolled one trustworthy.** It subtracts its own overhead, iterates warmup to a convergence criterion, reports a 99.9% CI, removes outliers and adds Gen0/1/2 counts. What it does not do is interleave, which is the whole point of the hand-rolled harness on a noisy shared container; on a controlled machine that noise is gone and the isolation is the better choice |
| `ffi/logs/csharp/stage6-tiering-sensitivity.log` | arm a, three processes differing ONLY in `DOTNET_TieredPGO` and `DOTNET_TieredCompilation`, P1.2 / P2.2 / P3.1 | **R9's JIT hazard, measured rather than argued.** Tiering off or PGO off slows the INCUMBENT by 5 to 20 percent, in the direction R9 names. **No arm crosses 1.0 under any configuration**, so no verdict in this slice is JIT-configuration dependent, and the default used everywhere else is the one least favourable to the managed arms |
