# cpp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **complete, re-measured after an adversarial review of 28 findings, re-gated after W10 moved the core, closed out with a concurrency suite and a new validator, and now CONFORMANCE-FIXED for the deprecated GROUP form (C24) and wired up as a consumer of `ffi/corpus` (W8).** Full codec plus the RPC arm plus a upb ceiling arm. Every message and payload of `design/SHAPES.md`, five encoders byte-identical, at C++11, C++14 and C++17, floor and target implementations, shared and static linkage |
| **Core** | **the shared one at `ffi/poc/codec/crates/ak-core` (README R0), not a copy.** This slice no longer has a `core/` directory; `core-build/` is only its three `CARGO_TARGET_DIR`s. See `logs/cpp/w10-one-core.log` |
| **Blocked on** | nothing |
| **Floor** | **C++11, demonstrated not declared.** C++14 also builds and passes (README open question 3) |
| **Target** | C++17 |
| **Incumbent** | protobuf C++ 3.21.12 (`libprotobuf-dev`, apt), `SerializeToString` / `ParseFromString`, non-arena and arena. `packages/cpp` pins **no** protobuf and **no** grpc version (`Dependencies.cmake` pins only fmt, simdjson and gtest) and sets `CXX_STANDARD 14` |
| **Ceiling** | upb from protobuf v25.3, built from source, **`UPB_FASTTABLE=0`, gcc 13.3.0**. A bound, never a candidate |
| **Machine** | 4 vCPU Intel Xeon @ 2.80 GHz, Linux 6.18.44, g++ 13.3.0 `-O2 -g -DNDEBUG`, rustc 1.94.1 |
| **R13 calibration** | this machine's rust-slice crossing is **1.5 ns** forward (`calibration-r13.log`), against 1.8 ns in the rust slice's own container |

**Absolutes here are instrumentation** (README section 8, after `4aec4e8`): the
cross-language comparison is re-taken on a controlled physical machine. What this slice
is for is correctness, crossing counts, the within-process deltas that settle an ABI
decision, and feasibility.

## The question this slice answers

**ABI v1 open decision 1**, which blocks freezing the specification: is every amendment
free under the C++11 floor?

## Arms

| arm | what it is |
|---|---|
| `pb` | protobuf C++ `SerializeToString`/`ParseFromString` — **the incumbent, and every ratio is against it** |
| `pb-det` | the same forced to deterministic map ordering, which byte identity needs. Its own row |
| `pb-arena` | the same on a `google::protobuf::Arena` |
| `memcpy` | **R2's floor**: one copy of the finished payload into a reused buffer |
| `upb` | **the ceiling** (separate binary and table, `upb.log`) |
| `native` | the generated codec emitted into C++: R3's no-boundary control |
| `ffi` | the same codec through the C ABI, spec transcoders (no UTF-8 check on encode) |
| `ffi-valtc` | the same with a validating transcoder — the check protobuf does on serialize |
| `ffi-zeroed` | ABI v1 open decision 9's candidate element fill |
| `ffi-nobat` | the host declines to batch |
| `ffi-hosttc` | the transcoder in the HOST: what a string-as-a-CALL form costs |
| `groupfill` | the by-value group's host-side fill alone, no codec |
| `ffi-borrow` | **decode only**: the same ABI and the same entry point over a facade whose string fields are `ak::StringView` over the input buffer. A measurement arm, never a proposal |

Linkage: **shared library is the primary arm**, static is a second, separately labelled
one. **Never a ratio across the two** (R7); they are separate processes and separate
mechanisms. The arm order **rotates every round**.

## What is measured

### The C++ column, shared library — `logs/cpp/bench_a17_shared.log`

Ratios to `pb`, formed inside one process, 9 rounds, each the minimum of five sub-batches.
`spr%` is the **denominator's own** round-to-round spread; a row above about 5 percent is
noise-dominated and is marked below.

**The decode columns below are re-taken with the borrowed arm present; the encode columns
are unchanged from the previous run and are reproduced from it.**

| payload | enc `memcpy` | enc `native` | enc `ffi` | enc `ffi-valtc` | dec `pb-arena` | dec `native` | dec `ffi` |
|---|---|---|---|---|---|---|---|
| P1.1 | 0.024 | 0.527-0.537 | 1.104-1.126 | 1.406-1.443 | 0.834-0.903 | 0.820-0.834 | 0.762-0.781 |
| P1.2 | 0.043-0.049 | 0.507-0.536 | 0.944-0.988 | 1.202-1.261 | 0.630-0.646 | 0.669-0.789 | 0.582-0.790 † |
| P1.3 | 0.002 | 0.674-0.682 | **1.770-1.806** | 1.770-1.814 | 0.424-0.432 | 0.995-1.014 | 1.053-1.073 |
| P2.1 | 0.019 | 0.434-0.443 | 0.873-0.884 | 1.158-1.183 | 0.784-0.809 | 0.783-0.802 | 0.715-0.733 |
| **P2.2** | 0.029-0.033 | 0.420-0.428 | **0.772-0.795** | 1.042-1.073 | 0.652-0.684 | 0.748-0.806 | **0.676-0.722** |
| P2.3 | 0.053-0.056 | 0.274-0.286 | 0.605-0.618 | 0.867-0.878 | 0.890-0.916 | 1.053-1.079 | 0.943-0.965 |
| P2.4 | 0.063-0.069 | 0.254-0.267 | 0.584-0.602 | 0.819-0.863 | 0.703-0.766 | 0.849-0.923 | 0.747-0.814 |
| P2.5 | 0.027-0.031 | 0.437-0.625 | 0.894-0.906 | 1.183-1.201 | — | 1.029-1.095 | 0.934-0.995 |
| P3.1 | 0.006 | 0.485-0.496 | 1.060-1.082 | 1.400-1.423 | — | 0.635-0.644 | 0.608-0.613 |
| P4.1 | 0.018-0.022 | 0.373-0.382 | 0.710-0.718 | 1.111-1.132 | — | 0.730-0.746 | 0.678-0.692 |
| P5.1 | 0.091-0.097 | 0.346-0.358 | 0.621-0.643 | 0.850-0.882 | — | 0.637-0.642 | 0.661-0.669 |
| P5.2 ‡ | 0.425-0.620 | 0.493-0.697 | 0.489-0.661 | 0.477-0.606 | — | 0.843-1.072 | 0.795-1.023 |
| P5.3 ‡ | 0.561-0.623 | 0.519-0.624 | 0.523-0.588 | 0.522-0.592 | — | 0.966-1.016 | 0.974-1.004 |
| P5.4 ‡ | 0.643-0.673 | 0.631-0.688 | 0.626-0.701 | 0.632-0.689 | — | 0.908-1.089 | 0.909-1.043 |
| P6.1 | 0.040-0.047 | 0.893-0.913 | **1.252-1.269** | 1.263-1.284 | 0.762 | 0.866-0.898 | 0.637-0.703 |

† **P1.2 decode carries a systematic outlier**, not a spread: one round in nine sits about
34 percent high, in every bench log this slice has produced, always P1.2 decode, for both
`ffi` and `native`. The per-round ratios are printed in the log
(`0.781,0.784,0.582,0.583,0.790,0.763,0.719,0.724,0.727`). **Read 0.58-0.73 and treat
0.790 as the artifact it is.** Unexplained.

‡ **P5.2, P5.3 and P5.4 have an 11 to 32 percent spread in the `pb` denominator itself**
and are not comparable to three decimals with the rows above.

**Three rows that are not good news and are not smoothed.** P1.3 encode is 1.77-1.81 —
the absent-path inversion, larger in C++ than in Rust. P6.1 encode is 1.25-1.27 — the
packed control, where `vector<TaskStatus>` and `vector<bool>` are not the wire layout so
the binding materialises a contiguous array first, which is the cost ABI v1 section 6
names. P1.1 and P3.1 encode are above 1 because the per-element work is small enough that
the fixed cost of the group dominates.

### The ceiling: upb — `logs/cpp/upb.log`

Separate table, separate binary, separate configuration line. protobuf C++ remains the
incumbent; upb bounds how fast a C protobuf can be, from the direction the memcpy floor
does not reach. **upb v25.3 built from source by `gen/fetch_upb.sh`; minitables from upb
reflection over `protoc`'s descriptor set, so no Bazel, no `protoc-gen-upb` and no
hand-written codec.**

| payload | upb enc / pb | upb dec / pb | the core's `ffi` dec / pb |
|---|---|---|---|
| P1.1 | 1.97 | **0.44** | 0.76-0.78 |
| P1.2 | 1.55 | **0.33** | 0.58-0.73 |
| P1.3 | 1.88 | **0.29** | 1.05-1.07 |
| P2.1 | 1.32 | **0.42** | 0.72-0.73 |
| P2.2 | 1.18 | **0.35** | 0.68-0.72 |
| P2.3 | 0.52 | **0.31** | 0.94-0.97 |
| P2.4 | 0.43 | **0.22** | 0.75-0.81 |
| P2.5 | 1.19 | **0.41** | 0.93-1.00 |
| P3.1 | 1.60 | **0.37** | 0.61 |
| P4.1 | 0.91 | **0.45** | 0.68-0.69 |
| P5.2-P5.4 | 1.32-1.42 | 0.94-1.00 | 0.80-1.04 |
| P6.1 | 1.67 | **0.58** | 0.64-0.70 |

**On decode upb is 0.22 to 0.58 of protobuf C++ on every element-bearing payload, and the
core is nowhere near it.** That is the most useful thing this arm says: the core's decode
win against protobuf C++ is real and is roughly half of what a C protobuf can do, so
"faster than the incumbent" and "as fast as C can go" are different claims and only the
first is supported.

**The upb ENCODE column is not a clean ceiling and is reported as measured.** With a
reused arena block it is still 1.18 to 1.97 of protobuf C++ on the string-dense payloads
and 0.43 to 0.91 on the repeated-string ones. protobuf C++ sizes its output once
(`ByteSizeLong`) and writes forward; upb grows a backward buffer geometrically. **Do not
quote upb as an encode ceiling from this slice.**

**P2.5: upb writes 19,712 B, the same form protobuf C++ writes**, which `design/SHAPES.md`
now records as one of two valid encodings. Two independent Google runtimes, the same +80 B.

### Experiment 1: `UPB_FASTTABLE` — `logs/cpp/upb-fasttable.log`

**A correction to this slice's own log first.** `upb/port/def.inc:227-239` defaults
`UPB_FASTTABLE` to **0**; it is 1 only under `-DUPB_ENABLE_FASTTABLE`, or under
`-DUPB_TRY_ENABLE_FASTTABLE` where `UPB_MUSTTAIL` exists. `gen/fetch_upb.sh` defined
neither, so **the published upb column was measured with the tail-call fast decoder
compiled out and did not say so** — an R7 omission. `upb.log`'s configuration line now
names it.

**The A/B is a null result, and the reason is reachability rather than a build that did
nothing.** `_upb_Decoder_TryFastDispatch` (`upb/wire/decode.c:766`) fires only when
`layout->table_mask != (unsigned char)-1`, and `upb/mini_descriptor/decode.c:698,712`
sets `table_mask = -1` on **every** minitable it builds. The fasttable entries come from
`protoc-gen-upb`'s `UPB_FASTTABLE_INIT` and from nothing else, so a reflection-built
minitable can never take the fast path. Both halves are proved from artifacts:
`upbbench` prints the runtime `table_mask` (**−1**), and `gen/fetch_upb.sh` prints the
fast-parse function count from the archive (**0** without the define, **42** with it).

Three builds of identical sources, so the compiler and the define are separated:

| build | P1.2 dec | P2.2 dec | P2.3 dec | P3.1 dec | P6.1 dec |
|---|---|---|---|---|---|
| gcc 13.3.0, `UPB_FASTTABLE=0` | 0.317 | 0.349 | 0.306 | 0.353 | 0.552 |
| clang 18, `UPB_FASTTABLE=0` | **0.245** | **0.283** | **0.245** | **0.277** | **0.522** |
| clang 18, `UPB_FASTTABLE=1` | 0.291 | 0.320 | 0.262 | 0.309 | 0.538 |

**clang is worth 6 to 23 percent of upb's decode**, so the published column understated
upb. **`UPB_FASTTABLE=1` is 3 to 19 percent SLOWER** on the same compiler, because the
`#if UPB_FASTTABLE` branch at the top of the decode loop is pure added cost when the
dispatch can never fire.

**What it settles: none of upb's measured decode advantage is `UPB_MUSTTAIL` tail-call
dispatch** — the mechanism a Rust core is structurally locked out of (`become` is
unstable, so there is no guaranteed tail call). All of it is the generic decoder: the
epsilon-copy input stream's one bounds check per field
(`upb/wire/eps_copy_input_stream.h:20-25`), arena allocation, minitable dispatch, and not
copying strings. Every one of those is a work item rather than headroom. **What is not
measured is what a `protoc-gen-upb` minitable would add on top**, which needs Bazel.

### Experiment 2: the borrowed-string facade — `logs/cpp/bench_a17_shared.log`

The facade and the binding are re-emitted with **`ak::StringView` in place of
`std::string`**, into `shapes_borrow`, by the same emitters parameterised on (namespace,
string type); the decode and encode helpers are overloaded so the generated call text is
identical and the shipping facade's emitted text is unchanged (`--check` green on all 21
files). **No ABI change is needed**: `ak_span` is already an offset into the buffer the
host handed in (ABI v1 section 4), section 7 says the span points into that buffer, and
7.4 tells the host to resolve it against the base pointer it already holds. It is exactly
upb's aliasing contract (`upb/wire/decode.h:29`). **UTF-8 is still validated**, so the arm
isolates the copy and nothing else. Singular strings, repeated strings, bytes and map keys
and values are all borrowed.

**Byte identity gates it** (R2): decode into the borrowed facade, re-encode, compare with
the manifest — every payload, including P2.5 from the incumbent's 19,712 B form.

| payload | dec `ffi` | dec **`ffi-borrow`** | upb (gcc) | upb (clang) | delta, % of a protobuf decode |
|---|---|---|---|---|---|
| P1.1 | 0.765-0.791 | **0.303-0.310** | 0.44 | — | −47.7 |
| P1.2 | 0.562-0.790 | **0.234-0.241** | 0.317 | 0.245 | −33.8 |
| P1.3 | 1.014-1.027 | **0.582-0.591** | 0.29 | — | −43.3 |
| P2.1 | 0.707-0.720 | **0.433-0.438** | 0.42 | — | −27.8 |
| **P2.2** | 0.661-0.696 | **0.387-0.407** | 0.349 | 0.283 | **−28.4** |
| P2.3 | 0.855-0.921 | **0.383-0.396** | 0.306 | 0.245 | −49.5 |
| P2.4 | 0.680-0.724 | **0.286-0.305** | 0.22 | — | −41.2 |
| P2.5 | 0.950-0.965 | **0.526-0.534** | 0.41 | — | −43.0 |
| P3.1 | 0.601-0.615 | **0.326-0.330** | 0.353 | 0.277 | −27.9 |
| P4.1 | 0.686-0.705 | **0.445-0.455** | 0.45 | — | −24.5 |
| P5.2-P5.4 | 0.80-1.04 | **0.000-0.035** | 0.94-1.00 | — | −85.9 to −99.5 |
| P6.1 | 0.639-0.654 | 0.599-0.646 | 0.553 | 0.522 | **−4.3** |

**Borrowing takes the core from about twice upb to level with or below it.** On P1.2 the
borrowed core (0.234-0.241) is below even the clang upb build (0.245). So **"the core's
decode is half of what a C protobuf can do" is largely a statement about `std::string`,
not about the codec.**

**P6.1 is the internal control that says the arm measures what it claims**: `MetricsBatch`
is one string and five packed scalar arrays, so there is almost no copy to remove, and it
barely moves (−4.3 %).

**What it does not isolate**, and the residual gap on P2.2 and P2.3 is exactly this:
vectors, maps and message children are still constructed. This separates the string copy
specifically, not host-side container construction in general.

**It is a measurement arm, not a proposal.** The views are valid only while the input
buffer lives, which is not what a facade ships by default; the main facade is untouched.

### Crossing counts, from the counting core — `logs/cpp/counts.log`

Identical to the rust slice's to the digit on every shared payload.

| payload | encode fwd / rev | decode fwd / rev | per element |
|---|---|---|---|
| P1.2 (M1, 1000) | 8 / 1 | 1 / 5 | 9 and 6 in total, not per element |
| **P2.2 (M2, 500)** | 2511 / 2501 | 1 / 3501 | **5.022 + 5.002 = 10.024 enc; 0.002 + 7.004 = 7.004 dec** |
| P3.1 (M3, 200) | 2 / 1 | 1 / 2 | 3 per 200, both directions |

Per-element columns are printed **forward / reverse separately** in the log; the summed
figure is given only where it is labelled as summed.

**The host-transcoder arm's crossings are now COUNTED, not argued** (R5). The counter
lives in the core and cannot see which image `tc` points into, so the host reports them
through a counting-build-only entry point: P2.2 encode goes from 2,501 to **19,668**
reverse crossings, **+34.3 per element**; P1.2 from 1 to 6,001, **+6.0 per element**.

Unbatched, forward per element: P1.2 1.001, P2.2 17.002, **P2.3 125.008, P2.4 311.012**.

### ABI v1 open decision 1: the verdict

Each as a within-round delta between two arms (R4). **Every row of every table is printed
with whether its lo and hi share a sign**, so a range cannot be quoted over the subset
with the wanted sign.

**1. The group is what costs, and it costs the HOST, not the boundary.** `groupfill`:
**22.5 ns/element on M1 (P1.2), 74.1 on M2 (P2.2), and 22.6 on M1's absent path (P1.3)
where a whole protobuf encode is 16.5 ns/element.** That is the P1.3 inversion
(`ffi` 1.77-1.81 of protobuf against `native` 0.67-0.68). **Decision 9's candidate fixes
it**: P1.3 −10.9 ns/element, **−66.8 % of a protobuf encode**, and it is a win or neutral
on 13 of 15 rows (P1.1 −2.9 %, P1.2 −1.2 %, P2.2 −1.8 %, P2.5 −4.2 %, P3.1 −8.7 %,
P4.1 −2.2 %), losing only on P5.1 (+14.9 %, one tiny message). **C++ agrees with Rust.**

**2. String as data is a WIN in C++, on every payload with strings in quantity.**
`ffi-hosttc` − `ffi` is positive with a consistent sign on 10 of 15 rows: **+1.42 % to
+3.89 % of an encode**, which is **0.3 to 1.2 ns per string**, one reverse crossing.
It straddles zero where there are no strings (P1.3) or few (P6.1, P5.3, P5.4). **One row
has the opposite sign and it is named rather than dropped: P5.2 at −3.70 %**, which is
M5's 64 KB `bytes` field on the direct-argument path — it makes two transcoder calls in
total and its denominator's spread is 32 percent, so it is noise, not a counterexample.

**3. The batching predicate loses in C++ and the crossover is now a number.** Unbatched is
faster by 1.87 % to 5.93 % on P1.1, P2.1, **P2.2**, P2.5, P4.1 and P5.2; batching wins on
P2.3 (+4.85 %), P2.4 (+5.87 %) and P3.1 (+4.24 %); P1.2, P1.3, P6.1, P5.3 and P5.4
straddle zero.

**The stated mechanism was wrong and is withdrawn.** "Batching wins where the crossing
count per element explodes" does not survive its own table: P3.1 wins and P1.2 straddles
at an identical +1.00 forward crossings per element unbatched. The measurement that
settles it is `logs/cpp/tax.log`: a calibrated delay in front of every forward entry-point
call, so the crossing is priced up. On P2.2 the delta is

| added tax | −0 ns | +2.0 | +4.4 | +6.9 / +8.1 / +10.5 | +12.9 | +24.6 |
|---|---|---|---|---|---|---|
| (nobat − ffi) ns/element | **−20.9** | +2.7 | +25.4 | +25 to +28 | +47.5 | +147.1 |

**The crossover is at a forward crossing of roughly 2 to 4 ns.** So batching loses in C++
at 1.82 ns and wins comfortably on .NET 8 (7.5-12 ns), FFM (33.8) and JNI (98.4). The
verdict is a statement about the crossing price, not about C++, and it transfers.
(`tax.log`'s 8 ns row read −2.22 %; the re-run appended to that log gives +1.4 to +1.9 %,
so that point was an outlier and the curve is monotone.)

**4. A fourth mechanism nobody listed: the two-pass blob write.** ABI v1 section 4 removed
the declared expansion bound, so the core opens a prefix of a learned width, hands the
transcoder the rest of the buffer and resolves the prefix afterwards; a host that already
holds the bytes writes key, length and body in one pass. Measured on P1.2's 6,000 strings
in one process: **5.04 ns against 9.53 ns per string, +4.49 ns.** A `tc == ak_tc_bytes`
fast path in the core would remove it for every host whose representation is already
UTF-8. **This slice cannot make that change: the core emitter is shared.**

### README 5.2's arms a, b and c

**The three-binary table cannot answer this**, and that is the finding rather than the
table. `gen/drift.sh` builds the same source twice with a neutral layout perturbation and
reports **worst across-build RATIO drift 0.240**, which is larger than the effect. And
`AK_CXX17` reaches exactly **11 sites** in the whole emitted tree, all on the decode side,
so every encode row and most decode rows of arm b compile identical source.

So the two constructs the switch actually selects are measured **inside one process**
(`bench_a17_shared.log`, "README 5.2 arm b, INSIDE one process"):

| construct | items | floor ns | target ns | target/floor |
|---|---|---|---|---|
| `m[k] = v` vs `insert_or_assign` | 2,000 | 424,371 | 455,564 | **1.074** |
| `push_back` + `back()` vs `emplace_back()` | 3,750 | 161,351 | 156,060 | **0.967** |

**The C++11 floor costs nothing, and on the map construct the C++17 form is a 7 percent
regression.** That is why arm b measured *faster* than arm a on P2.2 decode. The three
binaries agree within the drift bar and are reported as a consistency check, not as a
measurement: `ffi` P1.2 encode a 0.944-0.988, b 0.940-0.972, c 0.942-0.978.

### Correctness — `logs/cpp/conformance.log`

**443 checks, 0 failures**, five times (C++17 target, C++17 floor, C++14 floor, C++11
floor, C++17 static) and **441 once** — the non-validating decode build, where two
UTF-8-rejection checks are compiled out. The difference is named rather than flattened.

Covers: every payload of `SHAPES.md`; five encoders byte-identical to `manifest.json`; the
headline `SerializeToString` path's bytes as well as the deterministic one; the memcpy
floor's bytes; decoded values identical between the two facade decoders and equal to the
built value; round trips; P7.1 by decode and by permutation of its (tag, wire type, body)
triples; **unknown fields at the root, INSIDE a nested message, and on the message that
has the oneof** (where the case stays at the last known member and the payload is
dropped); the unknown enum value 999; malformed UTF-8.

**protobuf C++ rejects malformed UTF-8** in a proto3 `string`, so the rejecting decode is
the like-for-like policy and is this slice's default. **protobuf C++ needs deterministic
serialisation to be byte-stable** on a message with a map, which costs it **2.9 to 7.5
percent on P2.2**; it is a separate row and is not inside the incumbent's headline.

**P2.5 has two valid encodings** and this slice matches the protobuf/upb one (19,712 B) in
its timed rows, which is named in the log per `design/SHAPES.md`.

### C24: the GROUP skip, and the oracle that could not see it -- `logs/cpp/groupskip.log`

**`ak::Dec::skip` had no case for wire type 3, so this slice's arms REFUSED a legal
message**: an unknown field of the deprecated GROUP form. protobuf C++ and upb both accept
it. 443 conformance checks passed over the defect, five times, at every standard level,
and that is the finding rather than the fix.

**Byte identity against `ffi/schema/generated/manifest.json` cannot reach this code at
all.** The manifest is generated from the same proto3 description the codec is generated
from; proto3 cannot express a group; so nothing the generator emits ever puts wire type 3
on the wire. An oracle built from the schema that reads it can never execute the
unknown-field skip on the one shape the skip exists for.

The fix follows the shared core's (`poc/codec/crates/ak-rt/src/dec.rs`) rather than
inventing a second one:

- **`skip` takes the field number as well as the wire type.** A group carries no length,
  so the only way to find its end is to read fields until an `END_GROUP` whose field
  number MATCHES the one that opened it. A depth counter accepts
  `X-group-mismatched-end` and mis-nests every group after it.
- **Bounded recursion**: 100, protobuf's own default limit, returning `AK_ERR_DEPTH` (-4,
  now in `ak::` beside the other codes). A payload of nothing but start tags is an error,
  not a stack overflow inside the host's process.
- `END_GROUP` with nothing open stays malformed, as do wire types 6 and 7.

**The test was written before the change and is required to fail on a plant** (R10).
11 checks at C++17 target, C++17 floor, C++14 floor and C++11 floor -- 44 runs, 0
failures -- and two planted builds of `include/ak/rt.h` that must FAIL:

| build | what it plants | what it fails |
|---|---|---|
| `AK_GROUP_PLANT=1` | count nesting depth instead of matching the field number | T4 and T5, the two mismatched-end cases |
| `AK_GROUP_PLANT=2` | the `case 5:` 32-bit arm dropped while `case 3:` was added | T8 and T10, every buffer carrying a `fixed32` |

Plant 2 is not hypothetical: it is what the first run of the SHARED core's tests caught,
and no group test would have noticed it.

**The signature change was swept across the generator, not patched where it was found**:
13 emission sites in `gen/cpp_core.py` including the `sub.skip(et, ew)` inside the map
entry loop, plus two in `src/conformance.cpp`. `src/generated/core_native.cpp` was
REGENERATED; `generate.py --check` is green on all 23 files, the shared core's two
included.

### W8: the conformance corpus -- `logs/cpp/corpus.log`

The oracle byte identity cannot be. **128 of the corpus's 336 rows** root at a message
this slice's codec covers; the scope is read out of `AK_ROOTS` in the generated
`cases.h`, so it cannot exceed the scope the codec has. Run at **C++17 target and at the
C++11 floor**, identical results.

Three arms, every row through all three: `native` (no boundary), `ffi` (the C ABI over the
shared core), and `pb` -- **protobuf C++, as an ORACLE rather than a claimant**, projecting
through its own reflection `ListFields`, which is what CONTRACT.md section 3's presence
rule actually is.

| arm | C1 parse | C2 project | C3 re-encode | C4 refuse |
|---|---|---|---|---|
| `native` | 126/126 | 123/124 | 125/126 | 2/2 |
| `ffi` | 126/126 | 123/124 | 125/126 | 2/2 |
| `pb` (the incumbent, an oracle) | 126/126 | 123/124 | 124/126 | 2/2 |

**0 failures**, 128 of 128 rows with the two arms agreeing on the decoded facade
(CONTRACT.md 5.5, which is what R2 protects). Three rows are named rather than counted:

1. **`U-map-entry` is a disagreement with the CORPUS, and the corpus is outvoted 3 to 1.**
   The vector puts an unknown field inside every map entry. This slice reads the four
   entries into the map; the corpus's projection puts them under `_unknown` at
   `TaskOptions`. **protobuf C++ 3.21.12 and protobuf-python 4.25.9's pure-Python backend
   agree with this slice; upb drops the entries, and the corpus's projection is upb's.**
   Two Google runtimes disagree with each other on the same bytes. **`corpus/**` is not
   this slice's to write**, so it is reported, not fixed.
2. **`B-P7_1` is the interleaved payload**, whose bytes no canonical writer can reproduce.
   Every arm, protobuf C++ included, writes each repeated field contiguously; the driver
   parses both byte strings into (tag, wire type, body) triples and shows they are the
   same multiset. The manifest lists one accepted form. Reported, never counted as a pass.
3. **`B-P2_5` is the incumbent's own C3 miss**, which is design/SHAPES.md's two valid
   encodings and already recorded here.

**Plus a fourth thing the corpus can reach and the schema cannot.** The corpus's 62
`WireZoo` vectors root at a message this slice has no type for, so they are out of scope
for C1-C3 -- but their wire FORMS are exactly what C24 fixed. They are run through the
**unknown-field walker**: `ak::Dec::skip` over a buffer with no schema at all, which is
the path a root decoder takes for a field it does not know. **62 of 62 agree with the
corpus's verdict**, including `X-group-unterminated` (`AK_ERR_TRUNCATED`) and
`X-group-mismatched-end` (`AK_ERR_MALFORMED`) refusing for the right reason rather than
because wire type 3 was unknown. Labelled as a walker everywhere it appears: it parses
nothing and projects nothing.

**ABI v1 open decision 11, answered**: this slice **DROPS** unknown fields in both arms
(62 rows written in the `unknown-dropped` form, 17 more `unknown-dropped, map values
always written`). protobuf C++ RETAINS them, so adopting the core removes a proto3
guarantee a C++ caller has today.

**C5 (produce) is a PARTIAL claim and is stated as one.** 49 in-scope rows name `cpp`;
the 8 `baseline` rows are `ffi/schema`'s own payloads and `conformance` already builds
each from two independent routes and checks them against `manifest.json`. The other 41
are not produced -- see "what is not measured".

### C24's effect on the clock -- `logs/cpp/c24-timing.log`

The fix changed a signature, so every decode function in the control TU was recompiled and
gcc's inlining moved with it. "It should not move" is a prediction. **225 ratio rows
compared against the published `bench_a17_shared.log`: worst move 0.164, median 0.009,
0 rows over R4's 0.240 across-build drift bar.** The largest movers are P5.2-P5.4, whose
own `pb` denominator has an 11-32 percent spread. **The published timing tables stand and
are not re-taken.**

### The proofs that the arms are what they say

- **R5 half one** (`boundary.log`, 13 checks, 0 failures): the shared arm's 37 `ak_*`
  symbols are undefined dynamic imports; the static arm has them defined **and called**.
  `-flto` over a statically linked core cannot inline them away, because gcc's LTO only
  inlines across GIMPLE it produced and a Rust staticlib's members are native objects —
  a fact about the toolchain pair, not a general result.
- **R5 half two, now in BOTH directions**: the encode and decode traversals are out of
  line and larger than any timing closure, and the control is reached through a function
  pointer so its address is taken. **The `-flto` positive control fires on 2 symbols**,
  which is what says the check is capable of failing. No figure comes from that binary.
  **After C24 the count is 21 checks, 0 failed, not 23**: `dec_list_results_response` is
  now inlined into its caller WITHIN the control TU, which the checker reports and does
  not fail, because the question is whether the benchmark LOOP carries it and the loop is
  in another TU. The load-bearing line is unchanged -- all 10 timing closures still call
  out.
- **README 5.1's hard stop** (`odr.log`): 144 layout facts compared between a `-std=c++11`
  TU and a `-std=c++17` TU that are linked together, objects passed both ways. **0 moved**;
  the `-DAK_ODR_BREAK` positive control moves **49**.
- **ABI v1 section 10 / obligation 12.3**: the core exports **380 group-layout facts** and
  the host compares them with its own compiler's. **0 disagreements**, and a mismatch is
  now NAMED and not merely counted.
- **R1 as a gate, not a claim** (`generator.log`, run first by `run_all.sh`):
  `generate.py --check` green on all 17 emitted files; `refusal_test.py` **16 must-fail
  cases, 16 refused** (ABI v1 section 8's direct-argument refusal over this slice's own
  invocation, a repeated `bytes`, an unpacked repeated enum, a repeated `double`, a
  `map<string, int32>`, each against every backend separately); `audit_tracked.sh` green.

### The RPC arm — `logs/cpp/rpc.log`

| in flight | grpc++ CPU | core-ffi CPU | ratio | grpc++ wall | core-ffi wall |
|---|---|---|---|---|---|
| 1 | 6.53 ms | 5.68 ms | **0.870** | 7.47 ms | 6.32 ms |
| 8 | 7.39 ms | 6.41 ms | **0.868** | 3.04 ms | 2.57 ms |
| 16 | 7.64 ms | 6.54 ms | **0.856** | 2.71 ms | 2.40 ms |

The CPU column is `getrusage(RUSAGE_SELF)` **minus the server handler's own CPU, measured
the same way for both arms**. The harness-thread column is printed beside it and is
**not** the one to quote: it counts the grpc++ stub's transport (which runs on the calling
thread) and misses the core's (which runs on tokio workers), and it biases the ratio by
about 0.2. R9's wall-clock hazard is visible: at 1 in flight the wall clock is more than
the CPU and it more than halves at 8.

The same response decoded standalone in the same binary costs protobuf C++ 3.40 ms and
the core 2.27 ms (0.668), so **most of the ratio is the codec**. **Two crossings per RPC,
zero per field**, checked by grepping the core's RPC module for any message type.

**The carrier-thread row is answered trivially**: the idiomatic C++ wait is a blocking
call on a thread the host owns and C++ has no carrier thread to pin.

### The decode UTF-8 policy — `bench_a17_shared.log`, "decision 3, decode side"

Priced **on the string path alone, in one process, over all three content sets**, which is
how the rust slice priced it. 6,000 strings of P1.2:

**Re-priced.** 5,000 strings of P1.2 per set -- the FIVE `string` fields, not six -- with
three validators in one process (`utf8.log`, `bench_a17_shared.log`):

| set | bytes | raw ns/str | scalar | **table** | protobuf | scalar/raw | **table/raw** | protobuf/raw |
|---|---|---|---|---|---|---|---|---|
| ascii | 151,989 | 6.38 | 28.12 | **14.49** | 16.36 | 4.39-4.42 | **2.27-2.28** | 2.56-2.59 |
| latin1 | 303,978 | 6.99 | 117.31 | **112.39** | 138.70 | 16.68-16.99 | **15.91-16.16** | 19.63-20.00 |
| wide | 455,967 | 6.09 | 139.93 | **119.95** | 210.59 | 22.83-23.16 | **19.58-19.71** | 34.44-34.68 |

**On ASCII the check costs 2.27x a raw copy, not 4.4x**, and **the core's validator is
cheaper than the incumbent's own on all three sets** -- `protobuf` is protobuf C++'s
`IsStructurallyValidUTF8`, the validator it runs on every `string` field it parses, which
is the comparison R14 asks for. So decision 3's check is not a cost the core imposes on a
host that did not have one: it is cheaper than the check the host already pays.

Two corrections got it there. **C20**: the set was six fields and `ResultRaw.opaque_id` is
`bytes`, which no validator ever sees -- and in the ASCII set its 1,000 values are
arbitrary bytes the check arm rejected on the first byte, so the old row *understated* the
cost. **The validator**: a lead-byte table replaced the decode-then-range-check scalar.
A textbook DFA was tried first and is slower than the scalar version on wide content,
because its state is a serial dependency; it is kept, and in the differential test, as the
evidence for that sentence.

Read the latin1 and wide multipliers with their denominator in view: `raw` is 6-7 ns for a
whole string, so 16x is +105 ns. The ratio is large because copying 60 bytes is nearly
free, not because validating them is slow.

The earlier "22 to 28 percent of a decode" figure stays **withdrawn**, and the same
reasoning now applies to this change: the whole-payload effect is arithmetic (about 18% of
an ffi decode) and is inside R4's 0.240 across-build bar, so it is not claimed as measured.

`ffi-valtc` is **not** affected: it reaches `ak_tc_utf8()`, the core's Rust transcoder.
Whether the core's encode-side validator has the same 2x available is open, and R0 makes
it the aggregating session's rather than a slice's.

### The guard, the linkages, and the crossing

- **The guard is not measurable.** `bench_a17_noguard.log` against `bench_a17_shared.log`
  — but the two are different binaries, so the claim is bounded by the 0.24 drift bar and
  is a statement that nothing larger than that was found. In C++ the guard is a `try`/
  `catch` with no throw on the path.
- **The crossing**: shared **1.822-1.824 ns** forward, static **1.219-1.221 ns**, with a
  register-only barrier. The barrier hypothesis is **refuted**: changing from a full memory
  clobber to a register-only one did not move the shared figure toward the rust harness's
  1.5 ns on the same machine and the same `.so`. **A C++ host pays about 1.82 ns where a
  Rust host pays 1.5 ns through the same shared library**, and that is itself a result.
- **Static against shared is NOT attributed to the crossing count.** P1.2 encode makes 9
  forward crossings in TOTAL, so 0.6 ns of saving cannot explain an 11 µs move. The
  `native` arm, which makes zero crossings, moves between the two binaries too, so the
  difference is the build and not the boundary. The earlier causal sentence is withdrawn.

### The content sets, on whole payloads — `logs/cpp/contentsets.log`

SHAPES.md: "a slice that reports one string-path number without saying which content set it
came from has reported half a number." This slice had priced the *string path* over all
three sets and every *whole-payload* row over ASCII only, so the whole-payload rows were
the half number.

Correctness first and per set, because no manifest oracle covers latin1 or wide: **80
checks, 0 failures** — every arm byte-identical to the **incumbent**, which is itself
anchored to `manifest.json` on ASCII, plus a decode round trip per set.

Wire size: latin1 **1.687–1.748×** ASCII, wide **2.373–2.495×**. The rust slice published
1.70–1.75 and 2.39–2.50 from its own generator over the same description; the two agree to
three digits, which is a cheap R1 check that two slices' value rules produce the same
strings.

**The answer is different for the two directions, and that is the finding:**

| | ascii | latin1 | wide |
|---|---|---|---|
| P1.2 encode, `ffi`/`pb` | 0.988 | 0.167 | **0.114** |
| P1.2 decode, `ffi`/`pb` | 0.656 | 0.671 | 0.546 |

**The encode ratio is almost entirely a fact about the content set. The decode ratio is
not** — no payload's decode ratio moves by more than about 0.15 across all three sets. The
published C++ encode column is an ASCII column and nothing else; the decode column survives
being read without its content set.

**Why, and this keeps the encode number honest.** protobuf C++ **validates UTF-8 when it
serialises** a `string` — verified in the generated code, not inferred: `shapes.pb.cc` calls
`WireFormatLite::VerifyUtf8String(..., SERIALIZE)` unconditionally, 37 call sites. ABI v1
says the core does not. So most of that column is a check the core *skips*, and reporting
`ffi` against `pb` alone would publish a policy difference as codec speed. The like-for-like
row is `ffi-valtc`:

| payload | valtc/pb ascii | latin1 | wide |
|---|---|---|---|
| P1.2 | 1.179 | 0.422 | 0.365 |
| P2.2 | 1.043 | 0.479 | 0.421 |
| P3.1 | 1.389 | 0.551 | 0.505 |
| P4.1 | 0.995 | 0.626 | 0.530 |

Doing the same work, the core is at parity or slightly worse on ASCII and **about twice as
fast on latin1 and wide** — which agrees with `utf8.log` measuring the two validators
directly. Growth against each arm's own ASCII row separates the three effects: `ffi`
1.04–1.05 (width only), `ffi-valtc` 2.20–2.82 (width + the core's validator), `pb`
6.13–9.12 (width + protobuf's validator + its per-string costs).

**P6.1 is the control and behaves like one**: packed scalars with one string per batch, so
its wire size moves 1.058/1.117 where the others move 1.7/2.4. A table where every payload
moved by the same factor would be measuring the harness.

### ABI v1 obligation 12.5: the concurrency suite — `logs/cpp/concurrency.log`

No slice in the branch had one. Four payload shapes across two message types, threads in
sequence and threads together, every encode memcmp'd against a reference that protobuf
produces (so no plant can corrupt the oracle), at C++17, at the C++11 floor, on both
linkages, and with four times more threads than the machine has cores. **Zero wrong bytes
on every axis**, and no error leaks between contexts.

**The suite is shown to work rather than assumed to.** Three builds carry the two designs
ABI v1 section 6 refused, and `gen/concurrency.sh` requires each to do what section 6 says
it does:

| build | bytes wrong | two threads disagree | scaling, contended |
|---|---|---|---|
| shipped | 0 | 0 | 3.63-3.96x |
| `AK_CONC_PAD` (pad the prefix to the learned width) | 46 of 96 | 16 of 16 | — |
| `AK_CONC_GLOBAL` (the width table process-global) | **0** | 0 | 2.80-2.87x |
| both | 46 of 96 | **0** | — |

Four things this settles:

1. **12.5's own claim, measured.** "A suite with one shape reports zero wrong bytes with a
   per-thread-state defect present and absent alike." On the pad build: one shape 0 of 24,
   two shapes **44 of 48**. It holds — and the mechanism is narrower than the sentence. It
   is not two shapes that matters but two shapes that want **different widths at a shared
   length-prefix site**. P1.1 (858 B) and P1.2 (218 KB) learn the *same* table, and two
   different message types touch disjoint sites. The pair that works is P1.1 and P1.3.
   **This slice's first suite used P1.1/P1.2/P2.1/P2.2 and passed on all three plants.**
   T0 exists because of that: it asks the encoder which ordered pairs have a history
   surface at all and prints the answer even when it is empty.
2. **Section 6's two refusals are independent and only the combination corrupts.** A
   global width table is a data race and a throughput defect but **not** a byte defect,
   because an unpadded prefix is rewritten to the width the body needs whatever the guess
   was. So `conc_a17_global` is in the must-**pass** list, and that is the finding.
3. **Both together is the case a naive suite would miss**: the threads *agree* (0
   disagreements) because they share the polluted table, and are both wrong. Only the
   independent reference catches it.
4. **Section 6's throughput claim, reproduced from C++ and refined.** Under contention the
   global table is **1.83-2.05x** slower in aggregate — inside the java slice's measured
   1.32-2.23x, from another language and machine. But uncontended it is 1.13-1.23x and its
   *scaling does not degrade at all*. The cost is a function of how often the table is
   **written**, not of sharing.

## Next step

Two rows of the corpus are open questions for the CORPUS rather than for this slice (C25,
C26 below) and are reported, not fixed. Otherwise nothing is outstanding. W10 moved the
core to `ffi/poc/codec` and re-gated; nothing was re-taken, because nothing moved
(`logs/cpp/w10-one-core.log`), and C24's fix moved nothing either
(`logs/cpp/c24-timing.log`). In the order I would do it:

1. **Borrowed spans as a real facade option**, now that the arm says what they are worth
   (−24 to −50 % of a protobuf decode, and the core level with upb). The lifetime contract
   is the hard part and it is a design question, not a measurement one. **Decision 13, and
   the coordinator has said it is not this slice's.**
2. ~~A table-driven or SIMD UTF-8 validator~~ **done**, `logs/cpp/utf8.log`. What remains
   is a true SIMD one: `utf8_range` is **not** in this tree (STATE.md said it was and that
   was wrong — it was in a scratch directory from the upb arm that does not survive).
   protobuf's own validator is the ceiling instead, which is a better one for R14 and costs
   nothing. Honest expectation for SIMD on top: another 3x to 5x on ASCII.
3. **A core fast path for `tc == ak_tc_bytes`**, worth +4.49 ns per string. This is now
   a *change to existing behaviour* in the shared core, which R0 says is the
   aggregating session's to make rather than a slice's -- it moves every slice's gate
   at once. A slice may still ADD to `poc/codec`; this is not an addition.
4. ~~The content sets on whole payloads~~ **done**, `logs/cpp/contentsets.log`.
5. ~~A concurrency suite (ABI v1 obligation 12.5)~~ **done**, `logs/cpp/concurrency.log`.
   What remains is a TSan run (the core is a Rust cdylib built without it, so a TSan host
   would report the core's internals as uninstrumented) and the RPC half — the rust slice's
   shared-mutable-client defect is what motivated 12.5 and this suite covers the codec.
6. ~~Explain the P1.2 decode outlier round~~ **characterised**, `logs/cpp/c16.log`: it is
   glibc's mmap path, demonstrated by removal. One residual named there, and it is a
   question about glibc rather than about the ABI.
7. **A `protoc-gen-upb` build**, if the ceiling ever needs to include the fast decoder.
   That needs Bazel and is the one thing this slice stopped short of.
8. **More of the corpus.** The cheapest next row is the `chunking` class: it needs a
   `ChunkedResponse` codec, and this slice DOES batch element runs, so it is the one
   slice that can report a chunk count for `C-elemu-512` rather than a gap. After that,
   C5 for the 41 `E-*`/`S-*` rows, which needs the corpus's value rules in C++.
9. **The corpus's two disputed rows** (C25, C26) want a decision from the corpus agent,
   not from here.

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| C1 | `include/ak/rt.h` | `ak::Enc` used `std::vector::push_back`, so the no-boundary control was SLOWER than the arm it controls for | **fixed**: a raw cursor over a reserved block |
| C2 | `gen/cpp_binding.py` | decision 9's clear was applied to blob-run and map chunks that are filled in full, once per element | **fixed** |
| C3 | `gen/cpp_binding.py` | that clear was O(arena) where the fill is O(elements) | **fixed here.** Reported as a property of decision 9's candidate: `rust_abi.py:2370` still clears the whole chunk |
| C4 | `include/ak/vocab.h` | `Optional::set` took `const T&` only, copying a decoded child | **fixed**: an rvalue overload |
| C5 | `gen/cpp_header.py`, `cpp_layout.py` | two separate enumerations of the 380 layout facts | **fixed**: one `cpp_layout.facts` feeds both, so a permutation at constant count is impossible |
| C6 | this container | `libgrpc++` 1.51.1 and protobuf 3.21.12 are apt's; `packages/cpp` pins neither | open, cannot be fixed here |
| C7 | `src/harness.h` | the incumbent was handicapped three ways on encode: `clear()` before `resize()` (a full zero-fill per call), a hand-rolled `CodedOutputStream`, and deterministic ordering charged to the headline. **Worth about 8 points of every encode ratio** | **fixed**: `pb` is `SerializeToString` |
| C8 | `src/generated/core_native.cpp` | the control never reserved on a packed run, while the binding reserves at every batched fill. **P6.1 decode 1.241-1.273 → 0.861-0.873** | **fixed** |
| C9 | `CMakeLists.txt` | `-O2` with no `NDEBUG`, so protobuf's `GOOGLE_DCHECK`s were compiled into the incumbent | **fixed**; and `gen/opt.sh` shows `-O3` does not close what remains |
| C10 | `src/bench.cpp` | a fixed arm order every round; the rust slice hit this and fixed it with rotation | **fixed**: rotation |
| C11 | `src/rpcbench.cpp` | client CPU summed the harness's threads, counting grpc++'s transport and missing the core's tokio workers. **Biased the ratio by about 0.2** | **fixed** |
| C12 | `gen/generate.py` | both gen directories contain a `generate.py` and RUSTGEN was first on `sys.path`, so `import generate` got the RUST slice's | **fixed**: HERE first |
| C13 | five backends | every shape dispatch ended in an unconditional scalar assignment instead of raising; `cpp_build` and `cpp_pbbuild` stopped testing cardinality after the repeated-string arm, so a repeated `bytes` would have emitted a scalar store against a `std::vector` | **fixed** and tested by `gen/refusal_test.py` |
| C14 | `gen/cpp_binding.py`, `cpp_core.py` | the map path hardcoded `t.utf8` and the validating reader for both halves, so `map<string, bytes>` would have had UTF-8 validation applied to its value | **fixed**: derived from the pair message's declared kinds, and swept |
| C15 | `src/bench.cpp` | `groupfill` exceeds the (`ffi` − `native`) delta it is a component of on P1.3 (22.6 against about 18.3 ns/element) | **open.** The suspected cause is refuted: a direct-call variant measures the same as the indirect one to 0.3 percent. `groupfill` is reported as an UPPER BOUND on the group's cost, not as a component |
| C16 | the harness, not the core | the `ffi` arm's first two rounds on P1.2 decode ran 25-40 % high in every log, rounds 3-9 flat | **characterised, cause demonstrated, one residual named** (`logs/cpp/c16.log`). It is page-fault cost on **glibc's mmap path**: pinning `MALLOC_MMAP_THRESHOLD_` and `MALLOC_TRIM_THRESHOLD_` removes the outlier AND keeps the steady state, forcing always-mmap reproduces its value in every round, and the default allocator takes an order of magnitude more minor page faults (**10.8x** in the committed run, 10.8-13.1x across runs). Refuted: machine load (deterministic 6/6 on an idle box, both linkages) and the arm rotation. Not the cause but the reason it became visible now: the faster validator — with the old scalar one the row is flat at 0.62, because validation swamped a fixed per-iteration allocator cost. **Unexplained**: exactly *when* the threshold adapts. A different allocation history moves the outlier to a later round or removes it, and this does not predict which. What would settle it: a malloc hook logging size and mmap-or-not per call — a question about glibc, not about the ABI. **No figure withdrawn**: min-of-rounds plus the per-round list is exactly why |
| C17 | `../rust/crates/harness/build.rs` | the rust harness searched `<profile>` before `<profile>/deps` for `libak_core.so`. Cargo only uplifts a workspace MEMBER's cdylib, so after R0 moved `ak-core` out of that workspace the harness would have linked the STALE pre-move copy still sitting in `<profile>` -- a change measuring the same because it is not in the build | **fixed** in W10: order flipped, stale copy deleted, and `ldd` shows the arm loading `deps/libak_core.so` |
| C18 | `gen/boundary.sh` | half two asked whether a control function was LARGER than the largest timing closure and took that as evidence it was not copied into one. Size is a proxy for fusion, not a test of it, and its positive control was `-flto`, which fires only when the optimiser happens to fuse something. After W10 the largest closure went from 1433 B to 911 B and the control went quiet | **fixed.** Two direct properties instead: every timing closure still contains a call instruction, and every control traversal still has an out-of-line body. The control is now `src/fusion_probe.cpp` -- one function that MUST be called and one that MUST be fused, guaranteed by `noinline` + a volatile function pointer and by `always_inline`, not by optimisation level. 23 checks, 0 failed |
| C23 | `gen/boundary.sh` | the new call counter used `/\<call\>/`. **mawk is what is installed and `\<` `\>` are gawk-only word boundaries**, so it silently matched nothing and half two reported that 10 of 10 timing closures were fused | **fixed**: `/[ \t]call/`. This is the SECOND gawk-only construct in this one file -- `strtonum` was the first -- and both failed silently rather than erroring. Worth a grep before the next awk line |
| C20 | `src/bench.cpp`, the string-path table | the set of strings the decode-side UTF-8 policy was priced over included `ResultRaw.opaque_id`, a **`bytes`** field that no validator ever sees. In the ASCII set its 1,000 values are arbitrary bytes the check arm rejected on the first byte, so the row **understated** the cost | **fixed**: five fields, one definition in `harness.h` shared with the validator gate. Found by `src/utf8check.cpp` asserting that every string it validates IS valid, which the table never did |
| C21 | `src/concurrency.cpp`, the first version | the suite used P1.1, P1.2, P2.1 and P2.2 and **passed on all three planted builds**: two shapes of one message type can learn the same width table, and two message types touch disjoint sites, so no site ever over-reserved | **fixed**: the pair with a history surface is P1.1 and P1.3, and T0 now asks the encoder which ordered pairs have one and prints the answer even when it is empty |
| C22 | `src/concurrency.cpp`, the reference | the oracle was built by re-encoding with `ak::Enc`, which is where the plants live, so on the pad+global build the reference itself was wrong and every arm was compared against a corrupted oracle | **fixed**: the oracle is protobuf's encoder, which no plant can reach. The sha anchor caught it, which is what an anchor is for |
| C24 | `include/ak/rt.h`, `gen/cpp_core.py`, `src/conformance.cpp` | `skip(wire)` had no case for wire type 3, so every arm of this slice **refused a legal message**: an unknown field of the deprecated GROUP form, which protobuf C++ and upb both accept. 443 conformance checks passed over it because the manifest is generated from the proto3 description the codec is generated from and proto3 cannot express a group | **fixed**: `skip(tag, wire)` plus a field-number-matching `skip_group` bounded at 100 with `ERR_DEPTH`, swept across 13 emission sites and regenerated. Tested by `src/groupskip.cpp` at four (std, impl) pairs against two PLANTED defects, and by the corpus's five group vectors |
| C25 | `ffi/corpus/generated/projections/U-map-entry.json` | the corpus's projection puts the four map entries under `_unknown` at `TaskOptions`, where a map entry carries an unknown field. **protobuf C++ 3.21.12, protobuf-python's pure-Python backend and both of this slice's arms read them into the map; only upb does not, and the corpus followed upb.** Two Google runtimes disagree on the same bytes | **open, and deliberately not fixed here**: `corpus/**` is not this slice's to write. For the corpus agent. Evidence is in `logs/cpp/corpus.log`, which prints who says what |
| C26 | `ffi/corpus/generated/manifest.json`, row `B-P7_1` | the interleaved payload's only accepted encoding is the committed one, and no canonical writer can produce it -- every conformant encoder writes each repeated field contiguously, protobuf C++ included. A slice that re-encodes it correctly still fails C3 | **open, not fixed here**: same ownership. `gen/corpus.py` shows the two byte strings are the same (tag, wire type, body) multiset and reports the row separately rather than as a pass |
| C19 | `../csharp/gen/cs_abi.py` | its docstring still says "`ffi/poc/rust/crates/ak-core` is a cdylib exporting 68 `ak_` functions". The path no longer exists and the count is now 66 without `rpc` | **open, and deliberately not fixed here**: it is another slice's source, not a build file. For the csharp session |

## What is not measured

- **Exactly when glibc's mmap threshold adapts**, which is C16's residual. The *cause* of
  the outlier is demonstrated by removal; what a different allocation history does to its
  *timing* is not predicted. A malloc hook logging size and mmap-or-not per call would
  settle it, and it is a question about glibc rather than about the ABI.
- **A thread sanitizer run.** The core is a Rust cdylib built without TSan, so a TSan
  host would report its internals as uninstrumented and the result would be noise. The
  `AK_CONC_GLOBAL` race is argued from the code and its throughput, not from a detector.
- **A shared `ak_enc_ctx`.** ABI v1 makes the context host-owned and every thread in the
  suite owns its own; passing one context to two threads is not a supported use and is
  not tested as though it were.
- **The RPC half under concurrency.** The rust slice's shared-mutable-client defect is
  what motivated 12.5, and this suite covers the codec.
- **A true SIMD UTF-8 validator.** protobuf's own is the ceiling; what SIMD would add on
  top is open.
- **The validator's effect on a whole-payload decode ratio.** About 18% of an ffi decode
  by arithmetic, which is inside R4's 0.240 across-build bar, so it is not claimed.
- **The java and csharp slices' own gates after W10.** Only JDK 21 is installed here and
  java's build needs JDK 17 and JDK 8; dotnet is not installed at all. What was verified
  is that both builds RESOLVE the shared core -- java's core and JNI shim link against it,
  csharp's layout probe builds -- and nothing beyond that. `logs/cpp/w10-one-core.log`.
- **upb encode is not a ceiling** (see above), and no upb arm exists for the core's own
  shapes beyond encode/decode of the whole message.
- **What a `protoc-gen-upb` minitable would add** on top of upb's generic decoder. The
  fast decoder is unreachable from reflection minitables and the generator is Bazel-only,
  so the ceiling measured here is upb's generic decoder and nothing above it.
- **The borrowed facade's lifetime contract.** The arm measures the copy; it does not
  price what a host pays to keep the input buffer alive, nor a hybrid facade that borrows
  some fields and owns others.
- **Content sets** are measured on the string path only, not on whole payloads.
- **`ak_init` and the lifecycle**, **the pull decode family**, **the unknown-field bag**,
  **`ak_span.coder`**, **message-size and recursion limits**: unbuilt or unexercised, as in
  the rust slice.
- **The direct-argument path** is built and byte-identical; C++ has nothing to pin, so it
  says nothing about the JVM claim.
- **A C++20 coroutine surface** (README 5.1.2): a sketch only. `ak_call_unary_cb`'s
  completion callback is the one primitive such a surface needs — `co_await` over an
  awaiter whose `await_suspend` stores the `coroutine_handle` in `user_data` and whose
  completion resumes it, as free functions and an adapter type beside the installed class
  rather than as members of it. No new C entry point, and the floor keeps the blocking call
  as a complete alternative. **Not built**, by instruction.
- **Concurrency**: one thread in every codec arm.
- **Allocation and footprint**: nothing counts allocations or peak memory.
- **One compiler** (g++ 13.3.0); clang++ 18 is installed and unused.
- **Nesting past depth 3**, the adapter's non-injective states, and P7.1 being decode-only:
  structural gaps inherited from the payload set.
- **CONTRACT.md C5 (produce) on 41 of the 49 in-scope rows that name `cpp`.** Listed by
  id, per R11: `E-adapter-nested-error`, `E-adapter-nested-invalid`, `E-adapter-nested-ok`,
  `E-adapter-plain-error`, `E-adapter-plain-invalid`, `E-adapter-plain-ok`,
  `E-all-absent`, `E-elem-empty`, `E-elems-empty-3`, `E-explicit-absent`,
  `E-explicit-empty-string`, `E-explicit-zero`, `E-half-absent`, `E-map-entry-empty`,
  `E-map-key-only`, `E-map-value-only`, `E-msg-empty-present`, `E-oneof-empty-string`,
  `E-oneof-payload-free`, `E-root-empty`, and `S-<Root>-{full,alt,min}` for all seven
  roots (21 rows). Producing them needs the CORPUS's own value rules implemented a second
  time in C++; this slice has `ffi/schema`'s value rules and nothing else. The 8
  `baseline` rows that name `cpp` ARE produced, from two independent routes, by
  `conformance`.
- **The corpus's other 208 rows**, by root: `Surrogate` 56 (the transcode class; this
  slice has no `Surrogate` type and its own UTF-8 reject policy is measured in
  `utf8.log`), `ChunkedResponse`/`ChunkedResponseWide`/`ChunkElement`/`ChunkInner`/
  `ChunkLeaf` 29 (the chunking class, which needs a `ChunkedResponse` codec), `Nest` 9
  (including `X-depth-101` and `X-depth-300`, which are ABI v1 open decision 7 and which
  the unknown-field walker cannot reach because it does not recurse into a
  length-delimited body), `LeafResponse`/`LeafElement` 8, and 44 rows rooted at messages
  that are element types here rather than roots (`Probe`, `TaskOptions`, `Timestamp`,
  `Pair`, ...). `WireZoo`'s 62 are out of scope for C1-C3 but ARE run through the
  unknown-field walker, 62 of 62 agreeing.
- **The C++11 floor of the corpus consumer covers the same 128 rows**, not more: the
  floor is a correctness gate here and not a second scope.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `generator.log` | — | R1 as a gate: `--check` green on **23** files, 16 must-fail guards refused, the tracked-file audit green |
| `groupskip.log` | `ak::Dec::skip` alone, at C++17 target, C++17 floor, C++14 floor and C++11 floor, plus TWO PLANTED builds | **C24.** 11 checks x 4 configurations, 0 failures; the depth-counting plant fails the two mismatched-end cases and the dropped-`case 5:` plant fails the two that carry a `fixed32`. The decode path a schema-generated manifest can never reach |
| `corpus.log` | 128 of 336 corpus rows, three arms (`native`, `ffi`, and protobuf C++ as an ORACLE), at C++17 and at the C++11 floor, plus the 62 `WireZoo` rows through the unknown-field walker | **W8.** 0 failures; C1 126/126, C2 123/124, C3 125/126, C4 2/2 on both arms; 128/128 arm agreement; walker 62/62. Two rows named rather than counted (C25 `U-map-entry`, where protobuf C++ and pure-Python side with this slice against upb and the corpus; C26 `B-P7_1`, a permutation). Decision 11 answered: this slice DROPS |
| `c24-timing.log` | a fresh `bench_a17_shared` against the published one | **C24 moved nothing.** 225 ratio rows, worst move 0.164, median 0.009, 0 over R4's 0.240 across-build bar. The published tables stand |
| `concurrency.log` | 4 shapes x 2 message types, threads in sequence and together, C++17 + C++11 floor + both linkages, plus THREE PLANTED builds | **ABI v1 obligation 12.5, which no slice had.** Zero wrong bytes on every axis. 12.5's own claim measured: 0 wrong on one shape, 44 of 48 on two. Section 6's two refusals are independent — a global table is byte-clean and costs 1.83-2.05x under contention; padding is the byte defect |
| `utf8.log` | four validators, 17.78 M differential checks against an independent oracle, then timed in one process | **Decision 3's decode-side check re-priced: 2.27x a raw copy on ASCII, not 4.4x**, and the core's validator is cheaper than the INCUMBENT'S OWN on all three sets (R14). C20: the old set validated a `bytes` field |
| `c16.log` | one payload, one arm, six conditions incl. two `MALLOC_` tunings and a page-fault count | **C16 characterised.** The outlier is glibc's mmap page-fault cost, removed by pinning two thresholds; 13x more minor faults by default. Machine load refuted. One residual named |
| `contentsets.log` | 5 payloads x 3 content sets, one process, oracle = the incumbent per set | **SHAPES.md's sentence answered, and differently for the two directions.** The encode ratio is almost entirely a fact about the content set (0.988 → 0.114 on P1.2); the decode ratio is not (moves ≤ 0.15). Most of the encode column is protobuf validating UTF-8 on serialize, so `ffi-valtc` is the like-for-like row |
| `conformance.log` | six builds | R2. 443 checks, 0 failures, five times; 441 once and why. P2.5's two valid forms; protobuf C++ rejects malformed UTF-8 |
| `boundary.log` | the built artifacts, plus `fusion_probe` | R5 both halves and both directions, **21 checks after C24** (two symbols are now inlined inside the control TU, which the checker reports and does not fail). Half two rebuilt after C18: it tests call sites and out-of-line bodies rather than a size relation, and its control is a fixture that cannot stop firing |
| `odr.log` | a C++11 TU and a C++17 TU, linked | README 5.1's hard stop: 144 facts, 0 moved; 49 under the positive control |
| `calibration-r13.log` | the rust slice's own bench, here | R13: this machine's rust crossing is 1.5 ns |
| `counts.log` | the counting core, both linkages | R5. 9/6 for 1,000 M1 rows; 10.024/7.004 per M2 element; the host transcoder's +34.3 reverse crossings per element, COUNTED; the batching decomposition |
| `bench_a17_shared.log` | **arm a**, C++17, shared, guard on, reject, ASCII, 9 rounds. **Re-taken in W11**: the decode path now calls the table validator, and the string-path table carries three validators | **the C++ column**, decision 1's four mechanisms, the group fill, the two-pass blob write, the string path's three content sets, arm b inside one process, protobuf's determinism cost |
| `bench_a17_static.log` | arm a, **static linkage** | the second column. Crossing 1.219-1.221 ns against 1.822-1.824 |
| `bench_b17_shared.log`, `bench_c11_shared.log`, `bench_c14_shared.log` | floor implementation at C++17, C++11, C++14 | a consistency check inside the 0.24 drift bar, not a measurement |
| `bench_a17_noguard.log` | the guard off | nothing larger than the drift bar |
| `bench_a17_lossy.log` | the decode UTF-8 check off | superseded by the in-process string-path table; kept because it is what the whole-payload claim came from |
| `drift.log` | the same source, a neutral layout perturbation | **R4's across-build control: worst ratio drift 0.240.** Any cross-binary claim carries this bar |
| `tax.log` | the crossing priced up | **the batching crossover: 2 to 4 ns**, with the 8 ns outlier re-run |
| `opt.log` | `-O2 -DNDEBUG` against `-O3 -DNDEBUG` | the control's decode gap is not a function of the optimisation level |
| `w10-one-core.log` | the pre-move commit built in a worktree and run minutes apart, same machine | **W10 / R0: folding three copies of the core into one moved no number.** Worst ratio move 0.023 against a 0.240 drift bar, and the arms the core cannot touch move by the same amount. Every gate green; the `-flto` positive control no longer fires and is recorded as unproven |
| `rpc.log` | grpc++ 1.51.1, tonic 0.14, loopback, in-process server, P2.2, 9 rounds | client CPU 0.856 to 0.870 of grpc++, the codec half separated in-process, R9's hazard visible |
| `upb.log` | upb v25.3 from source, reflection minitables, **`UPB_FASTTABLE=0`, gcc** | **the ceiling: upb decode is 0.22 to 0.58 of protobuf C++.** The encode column is not a ceiling and says so |
| `upb-fasttable.log` | three builds of identical upb sources: gcc/FT=0, clang/FT=0, clang/FT=1 | **the fast decoder is unreachable from a reflection minitable** (`table_mask = −1`, proved at run time and from the archive), so none of upb's advantage is `UPB_MUSTTAIL`. clang is worth 6-23 %; `FT=1` is 3-19 % slower |
