# cpp slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **complete, and re-measured after an adversarial review of 28 findings.** Full codec plus the RPC arm plus a upb ceiling arm. Every message and payload of `design/SHAPES.md`, five encoders byte-identical, at C++11, C++14 and C++17, floor and target implementations, shared and static linkage |
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

| set | bytes | raw ns/string | check ns/string | delta | check/raw |
|---|---|---|---|---|---|
| ascii | 167,989 | 8.54 | 38.13 | +29.6 | 4.47-4.54 |
| latin1 | 335,978 | 9.93 | 150.48 | +140.6 | 15.08-15.28 |
| wide | 503,967 | 9.12 | 181.27 | +172.1 | 19.87-20.05 |

**This prices THIS SLICE'S SCALAR VALIDATOR and nothing else.** 38 ns to validate a
36-byte ASCII string is about 1 ns per byte, which is an order of magnitude off a
table-driven or SIMD validator; upb's `utf8_range` is in this tree and unused. The earlier
"22 to 28 percent of a decode" figure is **withdrawn**: it was a difference of two ratios
taken in two binaries, across a drift bar of 0.24, on ASCII only.

The same caveat applies to the `ffi-valtc` column above: it prices this validator, not
protobuf's, so it is an upper bound on what encode-side validation costs and not a
like-for-like row after all.

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

## Next step

Nothing is outstanding. In the order I would do it:

1. **Borrowed spans as a real facade option**, now that the arm says what they are worth
   (−24 to −50 % of a protobuf decode, and the core level with upb). The lifetime contract
   is the hard part and it is a design question, not a measurement one.
2. **A table-driven or SIMD UTF-8 validator** on the decode path. The 4.5x to 20x above is
   a validator figure and it is the largest single effect this slice measures; `utf8_range`
   is already in the tree from the upb arm.
3. **A core fast path for `tc == ak_tc_bytes`**, worth +4.49 ns per string. The core
   emitter is shared, so this is the aggregating session's to take.
4. **The content sets on whole payloads**, now that `recode` is reachable.
5. **A concurrency suite** (ABI v1 obligation 12.5).
6. **Explain the P1.2 decode outlier round**, which appears in every log.
7. **A `protoc-gen-upb` build**, if the ceiling ever needs to include the fast decoder.
   That needs Bazel and is the one thing this slice stopped short of.

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
| C16 | this slice | a systematic outlier round on P1.2 decode, about 34 percent high, in every log | **open**, printed per round rather than hidden in a range |

## What is not measured

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

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `generator.log` | — | R1 as a gate: `--check` green on 17 files, 16 must-fail guards refused, the tracked-file audit green |
| `conformance.log` | six builds | R2. 443 checks, 0 failures, five times; 441 once and why. P2.5's two valid forms; protobuf C++ rejects malformed UTF-8 |
| `boundary.log` | the built artifacts | R5 both halves and both directions, 13 checks, plus a `-flto` positive control that FIRES |
| `odr.log` | a C++11 TU and a C++17 TU, linked | README 5.1's hard stop: 144 facts, 0 moved; 49 under the positive control |
| `calibration-r13.log` | the rust slice's own bench, here | R13: this machine's rust crossing is 1.5 ns |
| `counts.log` | the counting core, both linkages | R5. 9/6 for 1,000 M1 rows; 10.024/7.004 per M2 element; the host transcoder's +34.3 reverse crossings per element, COUNTED; the batching decomposition |
| `bench_a17_shared.log` | **arm a**, C++17, shared, guard on, reject, ASCII, 9 rounds | **the C++ column**, decision 1's four mechanisms, the group fill, the two-pass blob write, the string path's three content sets, arm b inside one process, protobuf's determinism cost |
| `bench_a17_static.log` | arm a, **static linkage** | the second column. Crossing 1.219-1.221 ns against 1.822-1.824 |
| `bench_b17_shared.log`, `bench_c11_shared.log`, `bench_c14_shared.log` | floor implementation at C++17, C++11, C++14 | a consistency check inside the 0.24 drift bar, not a measurement |
| `bench_a17_noguard.log` | the guard off | nothing larger than the drift bar |
| `bench_a17_lossy.log` | the decode UTF-8 check off | superseded by the in-process string-path table; kept because it is what the whole-payload claim came from |
| `drift.log` | the same source, a neutral layout perturbation | **R4's across-build control: worst ratio drift 0.240.** Any cross-binary claim carries this bar |
| `tax.log` | the crossing priced up | **the batching crossover: 2 to 4 ns**, with the 8 ns outlier re-run |
| `opt.log` | `-O2 -DNDEBUG` against `-O3 -DNDEBUG` | the control's decode gap is not a function of the optimisation level |
| `rpc.log` | grpc++ 1.51.1, tonic 0.14, loopback, in-process server, P2.2, 9 rounds | client CPU 0.856 to 0.870 of grpc++, the codec half separated in-process, R9's hazard visible |
| `upb.log` | upb v25.3 from source, reflection minitables, **`UPB_FASTTABLE=0`, gcc** | **the ceiling: upb decode is 0.22 to 0.58 of protobuf C++.** The encode column is not a ceiling and says so |
| `upb-fasttable.log` | three builds of identical upb sources: gcc/FT=0, clang/FT=0, clang/FT=1 | **the fast decoder is unreachable from a reflection minitable** (`table_mask = −1`, proved at run time and from the archive), so none of upb's advantage is `UPB_MUSTTAIL`. clang is worth 6-23 %; `FT=1` is 3-19 % slower |
