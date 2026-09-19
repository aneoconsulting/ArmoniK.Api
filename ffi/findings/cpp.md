# Reading the cpp slice

The aggregating session's reading of `poc/cpp`, which is not the same document as
the slice's own `STATE.md`. What is here is what the slice's results mean for the
branch: what is established, what the other slices have to do differently because
of it, and what is still an argument.

**W4 is done, and it is the slice that settles W1's blocking decision.** Full
codec plus the RPC arm, every message and payload of `SHAPES.md` byte-identical
across five arms at C++11, C++14 and C++17, floor and target implementations,
shared and static linkage, plus two arms nobody asked for at the start: upb as a
ceiling, and a borrowed-string facade that turns out to matter more than either.

**Read the numbers as shapes, not as decimals.** Cross-language absolutes are
being re-taken on a controlled physical machine (README R13). What a rerun cannot
change is what this document leans on: crossing counts, signs, size classes, and
the mechanisms behind them.

**Configuration** (R7): g++ 13.3.0, `-O2 -DNDEBUG`, protobuf C++ 3.21.12 and
grpc++ 1.51.1 from apt, upb v25.3 built from source, rustc 1.94.1, 4 vCPU Xeon at
2.80 GHz. `packages/cpp` pins neither protobuf nor grpc and sets `CXX_STANDARD 14`.
R13 calibration: this container's rust-slice crossing is 1.5 ns, against 1.8 ns in
the container the rust slice ran in.

## 1. Read this first: the published numbers moved under review, against the slice's interest

An adversarial review returned 28 findings and the slice answered all of them. The
corrections are large enough that any figure quoted from the first run is wrong:

| | first run | after |
|---|---|---|
| P1.2 encode `ffi` | 0.895-0.913 | 0.944-0.988 |
| P2.2 encode `ffi` | 0.691-0.699 | 0.772-0.795 |
| P6.1 decode `native` | 1.241-1.273 | 0.861-0.873 |
| RPC client CPU | 0.558-0.652 | 0.856-0.870 |
| "validation costs 22-28 % of a decode" | headline | **withdrawn** |

Three causes, and each is a lesson for the remaining slices rather than a C++
detail. **The incumbent was handicapped three ways on encode** (C7): the harness
cleared and resized its output string, which value-initialises, so protobuf paid a
full zero-fill of the output on every iteration; it hand-rolled a
`CodedOutputStream` instead of calling `SerializeToString`; and deterministic map
ordering was charged to the headline rather than priced separately. Worth about
**8 points of every encode ratio**. **The no-boundary control was defective twice**
(C1, C8): first a `std::vector::push_back` encoder that made the control slower
than the arm it controls for, then a decoder that never reserved while the binding
reserves at every batched fill. **The RPC arm counted different thread sets per
arm** (C11), biasing the ratio by about 0.2.

**Every managed slice should check its own baseline for the same class of defect
before it reports**, and the C# and Python sessions have been told so. A baseline
that does extra work is the most flattering possible error and the hardest to see
from inside.

## 2. ABI v1 decision 1 is answerable, and one of the four answers does not transfer

The decision asked whether the managed-motivated amendments are free at the C++11
floor. They are not all free, the signs differ, and the useful finding is that one
of them is a function of the crossing price rather than of the language.

**The group costs, and it costs the host, not the boundary.** `groupfill` prices
the host-side fill alone at **22.5 ns per element on M1, 74.1 on M2, and 22.6 on
M1's absent path where a whole protobuf encode is 16.5 ns**. That is the P1.3
inversion measured directly rather than inferred, and C++ reproduces it larger
than Rust did. **Decision 9's candidate fixes it** (−66.8 % of a protobuf encode
on P1.3) and is a win or neutral on 13 of 15 rows. C++ agrees with Rust that this
is a host-side fill change, not an ABI change.

**String as data is a win**, +1.42 % to +3.89 % of an encode with a consistent
sign on 10 of 15 rows, which is 0.3 to 1.2 ns per string, about one reverse
crossing. The one row with the opposite sign is named rather than dropped.

**The batching predicate loses in C++ — and that is not the finding.** The slice's
first mechanism story ("batching wins where the crossing count per element
explodes") did not survive its own table, and was withdrawn rather than patched.
What replaced it is the most portable measurement in the branch: a calibrated
delay in front of every forward entry-point call, so the crossing can be priced
up. On P2.2 the delta moves from **−20.9 ns per element at no tax to +25.4 at
+4.4 ns**, monotone, with **the crossover at a forward crossing of roughly 2 to
4 ns**.

So batching loses in C++ at 1.82 ns and wins comfortably on .NET 8 (7.5-12 ns),
FFM (33.8) and JNI (98.4). **The specification must carry the crossover, not the
C++ verdict.** "Batching is a small loss" would have been true of one host and
wrong for three.

**A fourth mechanism nobody listed.** Section 4 removed the declared expansion
bound, so the core opens a length prefix of a learned width and resolves it
afterwards; a host that already holds the bytes could write key, length and body
in one pass. Measured at **5.04 against 9.53 ns per string, +4.49 ns**. The fix is
a `tc == ak_tc_bytes` fast path in the core, and **that is the aggregating
session's to take, because the core emitter is shared with the rust slice**.

## 3. The encode column is a narrower win than the branch assumed

With the incumbent unhandicapped, encode through the C ABI is **above 1.0 on four
of fifteen payloads**: P1.3 (1.77-1.81, the absent path), P6.1 (1.25-1.27, the
packed control), P3.1 (1.06-1.08) and P1.1 (1.10-1.13). It is at parity on P1.2
(0.94-0.99) and wins clearly only where an element carries enough work to amortise
the group: P2.2 0.77-0.80, P2.3 0.61, P2.4 0.58-0.60, P4.1 0.71.

The mechanism is the same one in every case and it is already named in ABI v1
section 6: the by-value group carries the whole singular subtree unconditionally,
so where per-element work is small the fixed cost dominates, and where a container
is not the wire layout (`vector<TaskStatus>`, `vector<bool>`) the binding
materialises a contiguous array first. **Decision 9 is therefore not a nicety.**
It is what makes the absent path viable, and the report should treat adopting it
as part of the proposal rather than as an open option.

## 4. The decode result is the slice's real contribution, and it arrived last

Three measurements compose into one conclusion the branch did not have.

**upb bounds the claim.** On decode upb is **0.22 to 0.58 of protobuf C++** on
every element-bearing payload, where the core through the C ABI is 0.58 to 1.07.
So the core's decode win against the incumbent is real and is **roughly half of
what a C protobuf can do**. "Faster than what ArmoniK ships" and "as fast as C can
go" are different claims and only the first is supported. With the memcpy floor
below and upb above, R2's floor rule is satisfied from both ends for the first
time in the branch.

**The upb encode column is not a ceiling and the slice says so.** upb is 1.18 to
1.97 of protobuf C++ on string-dense payloads, because protobuf sizes its output
once and writes forward while upb grows a backward buffer geometrically and
memmoves what it has written. Declining to quote a number that would have
flattered the core is the right call, and it also settles something about our own
design: the core's learned-width prefix is a third point in that trade, and
decision 5 already measured its miss rate at zero on every uniform payload.

**The tail-call hypothesis is dead, and that is good news.** upb's fast decoder is
gated on `UPB_MUSTTAIL`, which Rust cannot express today. It is also **unreachable
from a reflection-built minitable**: `decode.c:766` fires only when
`table_mask != -1` and `mini_descriptor/decode.c:698,712` sets it to −1 on every
minitable it builds. Proved from artifacts rather than asserted — the runtime
prints the mask, and the archive holds 0 fast-parse functions without the define
and 42 with it. Enabling it made upb **3 to 19 % slower**, which is what an
unreachable fast path costs. So **none of upb's measured decode advantage is
tail-call dispatch**: all of it is the generic decoder — the epsilon-copy input
stream's one bounds check per field, arena allocation, minitable dispatch, and not
copying strings. Every one of those is a work item a Rust core could take, not
headroom it is locked out of.

**And most of what is left is the string copy.** A borrowed-string facade arm —
no ABI change, because `ak_span` is already an offset into the host's own buffer —
takes decode from about twice upb to level with or below it:

| payload | `ffi` | `ffi-borrow` | upb (clang) |
|---|---|---|---|
| P1.2 | 0.562-0.790 | **0.234-0.241** | 0.245 |
| P2.2 | 0.661-0.696 | 0.387-0.407 | 0.283 |
| P2.3 | 0.855-0.921 | 0.383-0.396 | 0.245 |
| P6.1 (control) | 0.639-0.654 | 0.599-0.646 | 0.522 |

P6.1 is the internal control that says the arm measures what it claims: one string
and five packed scalar arrays, almost no copy to remove, and it barely moves. The
bulk-bytes rows fall to 0.000-0.035, which is not a speedup to quote but the
expected consequence of replacing a 4 MB copy with a pointer, and it wants an
explicit floor label under R2.

**So the core's decode gap to the fastest C protobuf is a facade ownership
question, reachable from Rust, rather than a codec or a language-runtime limit.**

## 5. This makes borrowed spans a cross-language decision, not a C++ arm

The branch already held the other half of this and had not connected it. ABI v1's
provenance table records that **decode spans as offsets into the host's buffer took
a 4 MB download from 4.2 times protobuf-java to 1.00**, a managed-host result. The
C++ arm now shows the same mechanism worth −24 to −50 % of a protobuf decode on
ordinary element-bearing payloads, against an incumbent that is not the JVM.

Three hosts, one mechanism, already expressible in the ABI as drafted. What is
unresolved is not whether it is worth it but **what the facade promises**: a
borrowed view is valid only while the input buffer lives, which is a lifetime
contract the branch has never written down, and in C# and Java it interacts with
pinning. That is a design question and it becomes ABI v1 open decision 13 rather
than something a slice settles.

The honest boundary: the arm isolates the **string copy** and nothing else.
Vectors, maps and message children are still constructed, and the residual gap to
upb on P2.2 and P2.3 is exactly that. The branch's earlier finding that decode is
bounded by host-side container construction is unchanged and is now the next thing
to price.

## 6. The floor, and README open question 3

**Either C++11 or C++14 is a viable floor and neither costs anything**, but the
three-binary table could not say so. `gen/drift.sh` measures **worst across-build
ratio drift of 0.240** with a neutral layout perturbation, which is larger than the
effect being looked for, and `AK_CXX17` reaches only 11 sites in the whole emitted
tree, all on the decode side — so most of arm b compiles source identical to arm a.

Re-formed inside one process, the two constructs the switch actually selects are
`insert_or_assign` at **1.074** of `m[k] = v` and `emplace_back()` at **0.967** of
`push_back` plus `back()`. The C++11 floor costs nothing and the C++17 map
construct is a 7 % regression, which is why arm b had appeared *faster* than arm a.

**The 0.240 drift bar is a branch-level output, not a C++ one.** Every
cross-binary claim in every slice needs it or an equivalent, and three conclusions
in this slice's first run were smaller than it.

## 7. The RPC arm

Client CPU **0.856 to 0.870** of grpc++ at 1, 8 and 16 in flight, where CPU is
`getrusage` minus the server handler's own, measured the same way for both arms.
Most of the ratio is the codec: the same response decoded standalone in the same
binary is 0.668. **Two crossings per RPC, zero per field**, checked by grepping the
core's RPC module for any message type rather than by trusting the sentence.

R9's hazard is visible rather than hidden: wall clock exceeds CPU at 1 in flight
and more than halves at 8, which is the 64 KB stream window and not throughput.

This is a weaker result than the first run's 0.558 and it is the defensible one.
It is also **not the rust slice's comparison** — that isolated the interface
(core-ffi against tonic); this measures interface plus transport plus codec
against a different stack. Only the rust column bears on the ABI.

## 8. What this slice asks of the design documents

1. **Decision 1: answered.** Record the four mechanisms with the crossover, not
   the C++ verdict.
2. **Decision 9: adopt, with the wording corrected.** The candidate's clear must
   say "clear the elements you will fill"; `rust_abi.py:2370` still clears the
   whole 32 KB chunk, which is O(arena) where the fill is O(elements).
3. **Section 4: add the two-pass blob write and the `ak_tc_bytes` fast path.**
   Mine to take; the core emitter is shared.
4. **New open decision 13: borrowed spans as a facade option**, with the lifetime
   contract as the substance.
5. **Section 6: a host that declines to batch does not merely lose its own
   crossing cost** — below the crossover it gains.
6. **README section 2: the C++ crossing row.** 1.82 ns shared and 1.22 ns static
   here, against a published 0.25 ns that does not reproduce, and against 1.5 ns
   for the rust harness through the same `.so` on the same machine. That last gap
   is not a harness artifact: with a register-only barrier the C++ figure is
   1.822-1.824.

## 9. What this slice does not establish

Beyond the slice's own list, which is longer and should be read with it:

- **Encode above parity is not explained away.** Four payloads regress and the
  group is the named cause, but no arm isolates the group's cost *inside* an
  encode the way `groupfill` does outside one. C15 is the visible symptom:
  `groupfill` exceeds the delta it is supposed to bound on P1.3, the suspected
  cause was refuted by a direct-call variant, and it is reported as an upper bound.
- **The borrowed facade is a measurement, not a design.** Nothing prices keeping
  the input buffer alive, a hybrid facade, or what either does to the public
  surface.
- **The ceiling stops at upb's generic decoder.** What `protoc-gen-upb` would add
  is unmeasured and needs Bazel.
- **One compiler on the codec arms** (g++), one thread everywhere, no allocation
  or footprint column, content sets on the string path only.
- **C16**, a systematic 34 % outlier round on P1.2 decode present in every log this
  slice produced, characterised and unexplained.
