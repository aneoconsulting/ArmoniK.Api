# rust slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only
thing that survives the end of a session. A stale entry here costs a whole
session, which makes it the most expensive defect in this directory.

| | |
|---|---|
| **Status** | **stages 1 to 5 complete.** Stages 1 to 4 as before (four arms, every shape, the RPC arm, decision 3 on both sides). **Stage 5 adds three things the branch had specified and nobody had built**: ABI v1 section 7.1's **pull decode family** (open decision 2, both halves), obligation 12.5's **concurrency suite**, and section 3's **`ak_init` and lifecycle**. All three landed in the SHARED core at `poc/codec/` (R0), additively |
| **Blocked on** | nothing |
| **Floor** (must build and pass correctness) | MSRV 1.88 declared. **Not verified: no 1.88 toolchain exists in this container, only 1.94.1** |
| **Target** (where the clock runs) | the same, one configuration (README section 5) |
| **Incumbent** (the baseline every ratio is against) | prost 0.14.4, plus tonic 0.14 for stage 4. **R14, checked rather than assumed**: tonic-prost 0.14.6's `src/codec.rs` calls `Message::decode(buf)` (line 131) and `item.encode(buf)` (line 98), and prost's `Message::encode` computes `encoded_len()` before `encode_raw` — so for Rust the production path and the library's entry point are the SAME call in both directions and there is no second labelled row. Rust is the one slice where R14's check comes back empty |
| **Outstanding** | one measurement: what section 3's `AK_ERR_UNINITIALIZED` guard costs (`gen/guardprice.sh`). Everything it needs is built; the run needs the box to itself |

## The question this slice answers

What a host language loses against full Rust, and what the new design costs against `packages/rust` today. This slice is the denominator for every other one.

## Arms

| arm | what it is | where |
|---|---|---|
| `prost` | prost-build structs, prost's codec | `crates/shapes-prost`, driven from `crates/harness/src/arms.rs` |
| `armonik` | facade types with generated `prost::Message` impls, no conversion layer | `crates/facade/src/generated/prost_impl.rs` |
| `core-native` | the generated core traversal emitted into the host, no boundary (R3's control) | `crates/facade/src/generated/core_native.rs` |
| `core-ffi-rust` | the same traversal through the C ABI, over a real shared-library boundary. The PUSH family | `../codec/crates/ak-core` (cdylib) + `crates/harness/src/generated/binding.rs` |
| `core-ffi-pull` | ABI v1 7.1's **pull** family: `ak_parse_*` with zero upcalls, then `ak_bdr_drain` into host memory, then replay. The shape a managed host must use | `crates/harness/src/pull.rs` |
| `core-ffi-pull-walk` | the same without the drain copy (`ak_bdr_ptr`, replay in place). The shape a native host uses | same |
| `core-ffi-pull-opaque` | the walk arm with the replay's calls made opaque. R5's second half for this family | same |
| `core-ffi-parse-only` | `ak_parse_*` alone. **Not a decode**: the materialisation term on its own | same |

## What exists

```
gen/generate.py [--check]   the generator. Imports ffi/schema/emit/shapes.py (R1)
gen/ir.py rustnames.py rust_facade.py rust_build.py rust_core.py rust_abi.py
gen/stage1.sh               stage 1 end to end
gen/stage1_isolate.py       the stage 1 candidate fix, on a scratch copy
gen/stage2.sh               stage 2 end to end: check, conformance, counts, boundary, timings
gen/dump_payloads.py        all 16 payloads to a scratch dir
gen/decpolicy.sh            the decode UTF-8 policy: three builds, round robin, rotating order
gen/inlining.sh             arm 1: the inlining term, separated from the interface term
gen/inline_check.sh         R5's control half: is core-native fused into the loop?
                            Answered from the built artifact. Runs from stage2.sh and stage3.sh
gen/zeroed.sh               arm 2: the zeroed-group element fill, decision 9 candidate
gen/unknown.sh              the unknown-field bag, decision 11
gen/unknown_predicate.py    does the bag break the batching predicate? Run this FIRST
gen/stability.sh            is a ratio reproducible across BUILDS? (R4, as sharpened)

crates/shapes-prost         protox 0.9 -> prost-build 0.14 over the generated .proto
crates/shapes-values        the value rules of emit/values.py, hand-re-derived
crates/stage1-validate      the stage 1 harness
crates/facade               facade types, the armonik arm, core-native, the payload builder
crates/harness              the binding, the arms table, conformance, counts, bench
crates/harness/src/pull.rs  the pull family's arms
```

**The core is NOT in this tree (R0).** `ak-abi`, `ak-rt`, `ak-core` and `rpc` live once at
`poc/codec/` and this slice path-depends on them. A second copy is a defect with a
mechanical check, `poc/codec/gen/one_core.sh`. What stage 5 ADDED there, all of it additive
and all of it available to every slice:

```
ak-rt/src/bdr.rs            the pull family's record buffer (section 7.1)
ak-core: ak_parse_<Root>    one per root, zero upcalls
ak-core: ak_bdr_reserve / footprint / drain / ptr / reset / count_forward
ak-core: ak_init, ak_initialized, ak_build_id, ak_log_test, ak_panic_test  (section 3)
ak-rt feature `global-widths`   section 6's REFUSED arrangement, so it can be measured
ak-core feature `init-guard`    section 3's "every entry point requires ak_init", so it
                                can be PRICED. Emitted as a post-pass over the codec text,
                                so a new entry point gets the guard by existing
```

Every one of those is off-by-default or new surface: `git diff --stat` over `poc/codec`
for stage 5's first commit was 2,048 insertions and **zero deletions**.

Binaries: `conformance` (byte identity), `counts` (`--features count`), `bench`,
`shapes`, `content`, `rpcbench`, `decpolicy`, `inlining`, `zeroed`, `unknown`,
**`pullbench`** (the two decode families), **`concur`** (obligation 12.5),
**`lifecycle`** (section 3).
Features: `guard` (on by default, ABI v1 section 5), `count`, the decode UTF-8 policy
`dec-reject` / `dec-reject-simd` (default: lossy), **`global-widths`** and
**`init-guard`** (both off by default; each builds an arrangement so it can be measured
rather than inherited).

## What is measured

M1 over P1.1 to P1.3 (`stage2-four-arms-M1.log`) and M2 over P2.1 to P2.5
(`stage3-M2.log`), ASCII only. Ratios to prost, formed inside one process, range across three
separate processes.

| payload | direction | armonik | core-native | core-ffi-rust |
|---|---|---|---|---|
| P1.1 | encode | 0.941 - 0.954 | 0.343 - 0.349 | 0.593 - 0.622 |
| P1.2 | encode | 0.967 - 1.032 | 0.425 - 0.438 | 0.706 - 0.716 |
| P1.3 | encode | 1.173 - 1.306 | 0.468 - 0.475 | 1.162 - 1.295 |
| P1.1 | decode | 0.889 - 0.939 | 0.751 - 0.861 | 0.779 - 0.889 |
| P1.2 | decode | 0.896 - 0.935 | 0.832 - 0.861 | 0.848 - 0.879 |
| P1.3 | decode | 0.965 - 1.047 | 0.726 - 0.838 | 1.313 - 1.393 |
| P2.1 | encode | 0.963 - 0.974 | 0.457 - 0.460 | 0.944 - 0.993 |
| P2.2 | encode | 0.989 - 0.999 | 0.493 - 0.499 | 0.863 - 0.874 |
| P2.3 | encode | 0.982 - 0.990 | 0.484 - 0.492 | 0.952 - 0.953 |
| P2.4 | encode | 0.993 - 0.996 | 0.567 - 0.574 | 1.107 - 1.123 |
| P2.5 | encode | 0.976 - 1.009 | 0.454 - 0.486 | 0.891 - 0.948 |
| P2.1 | decode | 0.958 - 0.966 | 0.984 - 0.999 | 1.251 - 1.296 |
| P2.2 | decode | 0.967 - 0.974 | 0.922 - 0.927 | 1.025 - 1.034 |
| P2.3 | decode | 0.985 - 1.002 | 1.030 - 1.038 | 0.940 - 0.949 |
| P2.4 | decode | 0.942 - 0.950 | 1.033 - 1.037 | 0.959 - 0.963 |
| P2.5 | decode | 0.974 - 0.998 | 0.937 - 0.963 | 1.046 - 1.072 |
| P3.1 | encode | 0.966 - 0.975 | 0.406 - 0.410 | 0.846 - 0.853 |
| P3.1 | decode | 0.961 - 0.968 | 0.811 - 0.824 | 0.834 - 0.838 |
| P4.1 | encode | - | 0.411 - 0.415 | 0.789 - 0.816 |
| P4.1 | decode | - | 0.861 - 0.864 | 0.916 - 0.920 |
| P5.3 | encode | - | 0.96 - 0.97 | 1.00 - 1.01 |
| P5.3 | decode | - | 0.45 - 0.46 | 0.45 - 0.46 |
| P5.4 | encode | - | 1.01 - 1.04 | 0.79 - 0.83 |
| P5.4 | decode | - | 0.084 | 0.080 |
| P6.1 | encode | - | 0.55 - 0.56 | 0.61 - 0.62 |
| P6.1 | decode | - | 0.86 - 0.87 | 0.56 - 0.60 |

- **One crossing costs 1.8 ns** here (forward, and forward-plus-reverse), measured in the
  same process and build as the arms.
- **Crossings**: M1's element type is a leaf, so 9 to encode a thousand rows and 6 to decode
  them. M2's is not: **7.004 per task on decode**, exactly what ABI v1 7.2 predicts, and
  **10.02 per task on encode**, which nothing had predicted.
- **Encode survives the harder shape**: the core is 0.46 to 0.57 of prost natively and 0.83
  to 0.92 through the C ABI on every uniform M2 payload.
- **Decode converges to parity in proportion to host-side CONTAINER construction per
  element**, not to bytes or strings. P3.1 (flat, 5 fields) 0.81; P1.2 (6 blobs, 2 optional
  messages) 0.83; P2.2 (4 `Vec<String>`, a `BTreeMap`, a 27-field struct) 0.89 to 0.96. A map
  insert and four vector growths are work every arm does identically, so the denser the
  element's container graph the smaller the share of decode any codec owns. This supersedes
  the looser "decode is allocation-bound" from part 1, which attributed it to the wrong thing.
- **The per-element interface cost is the GROUP, not the optimiser** (`stage3-inlining-term.log`).
  The figure was obtained by subtracting `core-native` from `core-ffi-rust`, and the
  objection was that `core-native` is inlined into the benchmark loop and the FFI arm cannot
  be. **The premise is false for these binaries and the artifact says so**: the largest
  `bench::main::{{closure}}` is 472 bytes and the traversal it would have to contain is
  4,299 (encode) / 11,311 (decode); both entry points are exported globals in a PIE and are
  reached by GOT-indirect calls. Two added no-boundary arms — `core-native-noinline`
  (`#[inline(never)]`, a lower bound) and `core-native-opaque` (a `black_box`ed function
  pointer, no inlining, devirtualisation or constant propagation) — measure the same as
  `core-native` everywhere. On **P1.3 encode** the inlining term is −0.10 to +0.01 ns per
  element against 11.3–11.4 for the group; on **P1.3 decode** it is −1.03 to −0.82 against
  27.6–28.4. **The mechanism, precisely** (narrower than "LTO is off"): the workspace
  has no `[profile.release]`, so `lto = false`, and the two entry points the benchmark calls
  are non-generic `pub fn` with no `#[inline]`, so their MIR does not cross into `harness`.
  `core_native.rs` does carry 38 `#[inline]` functions whose MIR does cross, including the
  4,299-byte traversal itself — it was not inlined on cost grounds, and the benchmark never
  calls it directly anyway. **The failure mode**: if the generator ever put `#[inline]` on a
  per-message entry point, or made one generic, the objection could become true again with
  LTO still off. That is why `gen/inline_check.sh` is a script and not a paragraph. At 9 crossings per 1000 elements the dynamic call is about 0.02 ns/element, so
  that column is group materialisation with a rounding error attached.
- **One correction the audit did find, and it is not the one predicted**: on **P1.1 and P1.2
  DECODE** `core-native` and `core-ffi-rust` are inside each other's spread and the sign of
  the difference flips between builds (`core-native` faster in `bench`, `core-ffi-rust`
  faster in `inlining`). **No per-element interface cost should be quoted for those two rows
  in either direction.** P1.1's per-element column is also a per-MESSAGE cost divided by
  four and is not comparable with P1.2's.
- **The ZEROED-GROUP variant of the element fill answers open decision 9's condition**
  (`stage3-zeroed-group.log`). The host memsets the chunk once and assigns only what differs
  from the default, instead of section 6's total fill. Built as an **arm**; the generator
  emits it alongside the default and nothing the default path uses changed (conformance and
  crossing counts identical to the digit). Encode only, top-level element group only.
  Six runs: **P1.3 (M1 absent) 0.719–0.766 of the total fill**, −4.06 to −4.98 ns/element,
  and the encode inversion goes from 1.108–1.188 of prost to **0.815–0.857**. **P1.2 (M1
  full) 0.986–1.014**, −1.66 to +1.70 ns/element — inside the spread of zero. **P2.2, the
  shape the control plane moves, 0.970–0.985** — a consistent small saving of 13–26
  ns/element, not a loss. **P2.5 (M2 absent) 0.983–1.007**, no measurable change. So it wins
  on the absent path and costs under 5.4 ns/element everywhere else; the worst case measured
  is +1.70.
- **One correction to how that trade is described.** Section 6 prices the total fill as
  buying "the codec does not reset the element group between elements, worth 5.4 ns per
  `ResultRaw` and 24.4 per `TaskDetailed`". The zeroed variant does **not** give that back:
  the array is the host's chunk buffer, so the codec still never resets anything. What
  changes is only the host's fill — an unconditional store per field becomes a bulk memset
  plus a conditional store. The 5.4/24.4 figure is the right threshold to judge the cost
  against, and is not the cost being paid back.
- **THE PUBLISHED M1/M2 TABLE IS RETIRED AND RE-TAKEN** (`stage3-reproducibility.log`).
  **A ratio IS reproducible here**, to ±0.02–0.05: three builds with a semantically neutral
  layout perturbation, five to six runs, every arm rebuilt together, and the across-build
  spread is no larger than the same-binary spread on nearly every row. The published encode
  figures are 0.10–0.27 away — five to ten times that band.
  **And the published column cannot be re-derived at all**: `cc7f68c6`, the commit behind
  `stage2-four-arms-M1.log`, does not contain the benchmark binary (D19 — the bins were
  untracked until `7fb30be5`), so a worktree there fails with "can't find bin `bench`". The
  oldest rebuildable commit, `7fb30be5`, agrees with today and not with the published table.
  **So the question is not answerable by re-measurement, and the container is not the
  problem: the published numbers are unreproducible because the code that made them was
  never committed.** The table in `stage3-reproducibility.log` is the one to use.
- **What that costs the headline**: `core-ffi-rust` P1.2 encode is **0.982** today against
  0.706–0.716 published, so **through the C ABI the core is at parity with prost on encode
  for the uniform payloads, not thirty percent faster**. What survives intact is the
  no-boundary arm — `core-native` is 0.42–0.54 of prost on every encode row — so the codec is
  about twice prost's speed and the C ABI gives that back. The decode side largely
  reproduces.
- **The three large-effect qualitative findings all re-confirm.** The P1.3 inversion keeps
  its sign and its published magnitude on encode (1.190–1.210 against `core-native` 0.416–0.424).
  UTF-8 validation on non-ASCII is confirmed and **larger** than published: 2.75–3.74 of prost
  for the scalar validator against 2.0–2.6, and — the point worth keeping — **its within-arm
  form reproduces almost exactly** (2.17–3.37 times its own ASCII cost against a published
  2.2–3.0), which is R4's new half demonstrated rather than argued. Decode converging to
  parity with container density keeps its ordering (P1.2 0.861, P2.2 0.978).
- **The unknown-field bag answers decision 11 for Rust** (`stage3-unknown-fields.log`).
  **Structure first**: as one opaque `bytes` blob the bag changes the leafness of no message;
  as a repeated field it takes the schema from 9 leaf messages to **0** and every batched run
  fails. Run `gen/unknown_predicate.py`. **The empty bag — the case production is always in —
  is free on decode** (capture on / off 0.978–1.002, per-element deltas straddling zero) and
  **costs 1 to 12 percent of an encode**: P1.2 +2.7–4.7% total fill / +2.5–3.4% zeroed;
  P1.3 +7.8–12.2% / **+0.3–0.7%**; P2.2 +0.7–1.7% / +3.9–9.4%; P2.5 +4.2–4.6% / +2.0–8.4%.
  **The decision-9 interaction goes both ways**: on P1.3 the memset absorbs the extra slot
  and the unconditional stores do not; on P2.2 the reverse. A slice pricing the bag under one
  fill alone would have got the sign wrong on half the payloads.
- **Round-trip: the bag's BYTES are preserved exactly; the message's LAYOUT is not** when an
  unknown tag sits numerically between two known ones, because the bag is appended rather
  than merged (by instruction). That case is validated semantically — decode, re-encode,
  decode, compare values including the retained blob. **Migration note, not a defect**: a
  message round-tripped through the core is no longer byte-comparable with one round-tripped
  through protobuf-java, whose `UnknownFieldSet` writes in field-number order. Fresh encodes
  from a value are unaffected, so the manifest is untouched.
- **The oneof and explicit presence cost no crossings at all**: 3 for 200 elements in both
  directions. Both ride in the group.
- **A 4 MB bulk decode costs exactly one copy in the core and twelve in prost.** P5.4 decode
  is 0.080 to 0.084 of prost, and a raw `Bytes::copy_from_slice` of the same 4 MB measured
  0.082 in the same process: the core arms are ON the memcpy floor. The claim is bounded by
  that control, not by the ratio. Why prost sits twelve times above the floor is a suspicion
  (`bytes = "vec"` merging over a `Take` inside the nested message; the penalty grows with
  size) and is **not verified**.
- **P4.1 and P6.1 behave like M1 and M3**, not like M2: encode 0.45 to 0.62 native and 0.61
  to 0.82 through the ABI, decode 0.56 to 0.92.
- **The accessor guard is not measurable** on Rust, on M1 or on M2, and M2 makes 7 to 10
  reverse calls per element.
- **UTF-8 validation ON ENCODE was the largest single effect in the slice, and decision 3's
  third framing removes it.** The scalar validator cost 2.2 to 3.0 times its own ASCII cost
  and turned a 0.72 to 0.81 win against prost into a 2.0 to 2.6 loss; a SIMD validator with
  the same contract removed half to two thirds of that. **Those rows are now the retired
  measurement of a retired framing.** Every encode figure in this slice already quotes the
  passthrough (`Ctx::new()` builds `Tcs::trusted()`), and `ak_tc_bytes()` and
  `ak_tc_utf8_trusted()` return the same function pointer in this tree.
- **On DECODE, where validation is mandatory, validate-and-reject is free to cheaper**
  (`stage3-decode-utf8-policy.log`). On the string path alone, in one process, the scalar
  rejecting policy is 0.54 to 0.75 of today's lossy path on ascii, 0.83 to 0.97 on latin1
  and 0.94 to 1.10 on wide; with `simdutf8::basic` it is 0.36 to 0.71 of lossy on every set.
  `from_utf8_lossy` already validates — it substitutes instead of failing, and its recovery
  path is slower than `from_utf8`. On a whole decode the ratio to prost moves from 0.87-0.91
  to 0.73-0.79 (P1.2 ascii) and from 0.78-0.80 to 0.49-0.51 (P1.2 wide, SIMD). **prost
  rejects too, so this makes the comparison like-for-like and moves it in this slice's
  favour**: the old decode column was pessimistic, not flattering. The decode tables above
  were taken under the lossy policy and are not rewritten; adopting a rejecting decode
  improves the ASCII decode column by roughly 0.08-0.12 on M1 and 0.03-0.08 on M2.
- **The content set changes no decode verdict.** Every arm validates on decode, so all three
  sets scale all arms together and the ratios move by less than the run-to-run spread.
- **ABI v1 SECTION 7.1's PULL FAMILY IS BUILT, AND OPEN DECISION 2's SECOND HALF IS
  ANSWERED YES** (`stage5-pull-decode.log`). One `dec_walk` is emitted once and instantiated
  twice; the families differ in the `flush_*` macro body, the entry point's prologue and
  epilogue, and one argument naming the non-leaf element decoder. **The structural control
  is that a record is written exactly where push makes a reverse call, so the counts must be
  equal — and they are, to the digit, on all thirteen counted payloads** (P2.2 3501/3501),
  with pull's reverse count measured at zero everywhere.
- **The interface figure, which is a property of the descriptor and not of this machine.**
  Push's crossings are per ELEMENT and pull's are per MESSAGE: P2.2 is **3,501 reverse
  against 16 forward**, or **3** if the host drains in one chunk, and 3 is the floor for
  every payload in the set. The chunk size is the only knob a host has on that count.
- **Pull costs a RUST host between −5% and +18% of a push decode, and the prediction going in
  was wrong.** At a 1.8 ns reverse call pull was expected to lose; over six runs it is 0.94
  to 1.18 of push, at or below push on nine of twelve payloads and a win on every M2 shape.
  A push reverse call costs this host more than a crossing: it goes through a vtable slot
  reached from across the shared object, and the replay's equivalent is a local call over a
  buffer already in L2. **The opaque-replay control clears the obvious objection** — the
  parity is not rustc inlining the replay, and that arm measures the same as the plain walk
  arm on every payload.
- **Where pull loses, the byte table explains it and the clock does not.** P1.3 (+5 to +13%)
  and P6.1 (+8 to +18%) are the two losing rows, and the record stream is **63.6 times the
  wire** on P1.3 — 38,488 B for a 605 B message — because a record carries the whole fixed
  group of an element that encodes to nothing. **Pull's cost tracks the ratio of record bytes
  to wire bytes, which is a property of the SHAPE.** Every payload whose ratio is below 1 is
  at or under push.
- **The decomposition, so another host can re-price it**: materialise (`ak_parse_*` alone) is
  8.5% to 47% of a push decode depending on shape; the drain copy is 1% to 12%. The C# slice
  ESTIMATED the pull intermediate at 12 to 19 percent of a parse; the copy half of it is
  measured here.
- **OBLIGATION 12.5's CONCURRENCY SUITE EXISTS, AND CORRECTNESS IS CLEAN**
  (`stage5-concurrency.log`). Two shapes in sequence on one context, then 2, 4 and 8 threads
  with a context each and phases offset so they are not in lockstep; every encode compared
  byte for byte with a single-threaded reference and every decode by value. **0 wrong out of
  2,840 encodes and 2,840 decodes, on both width-table builds.** The codec half has no shared mutable state, and that
  is now a measurement rather than a reading of the source.
- **A SHARED ENCODE CONTEXT ABORTS THE PROCESS, AND THAT IS A HOLE IN SECTION 5.** The
  suite's positive control plants D16's class in the codec half — four threads, one context.
  It is **not** detected as wrong bytes: the core panics inside `Enc`, the frame the panic
  must unwind through is an `extern "C"` entry point, the unwind is refused and the runtime
  aborts. So section 5's error channel covers a failure the HOST reports and has **nothing at
  all for a panic inside the core**, and every codec entry point is exposed to it. Section
  3's panic hook changes what is printed, not whether the abort happens. Raised, not taken:
  an owning-thread id beside the context's existing `kind` word would turn this into
  `AK_ERR_INVALID_STATE` at the first misuse.
- **SECTION 6's "NEVER PROCESS-GLOBAL" IS MEASURED ON A SECOND HOST: the sign agrees and the
  magnitude does not.** `--features global-widths` builds the refused arrangement. It costs
  **0.5 to 11 percent**, only above one thread, and **only when two shapes want different
  widths at the SAME site** — a disjoint-site control shows no penalty at any thread count,
  so this is true sharing and not false sharing of the static's cache lines. Against the
  branch's inherited java figure (1.32 to 2.23 at two threads) this host sees 1.005 to 1.070
  at two and needs four to reach 1.11. R9 is why that is not a contradiction. **The
  one-thread rows are a confound and are in the log rather than removed**: a static array is
  reached more cheaply than a `Box<[u8]>` in the context, so the global arm is 2 to 6 percent
  faster at one thread and the contention figure is a difference of differences.
- **The contention arm is shown to contend rather than assumed to**: a warm context on one
  shape misses zero length prefixes; the same context alternating the pair misses exactly one
  per encode, which also says a width miss costs one element and not one message.
- **SECTION 3's LIFECYCLE IS BUILT AND EXERCISED** (`stage5-lifecycle.log`), which it had
  never been anywhere in this branch. Fourteen cases, each in its own process because
  `ak_init` is one-shot and there is no `ak_shutdown`: the state machine, the ABI-version
  check, idempotence (`AK_ALREADY_INITIALIZED` as a SUCCESS), refusal on different options,
  a null options pointer, the log bridge and `AK_INIT_OWN_LOGGING`, the panic hook and
  `AK_INIT_NO_PANIC_HOOK`, the codec still correct afterwards, and **8 threads racing
  `ak_init`: exactly one `AK_OK`, seven `AK_ALREADY_INITIALIZED`, and no caller returning
  before the installs are visible**.
- **What the `AK_ERR_UNINITIALIZED` guard COSTS is not measured yet.** `gen/guardprice.sh`
  is the run; it needs the box to itself and this session lost one attempt to two benchmarks
  overlapping, which is README section 11 reproduced. No figure is quoted until it exists.
  What the crossing counts already say without a clock: the guard is per ENTRY POINT, and
  the entry points are 1 per decode and 8 (P1.2) to 2,511 (P2.2) per encode — per message or
  per chunk, never per field.
- **Two findings came out of cases that FAILED first, and both failures were the
  specification working.** (a) **The core's panic hook does not see a Rust host's panics**,
  because a cdylib carries its own copy of `std` and the two hooks are two different globals.
  Section 3 warns about that mechanism one level up; here it is `std`'s globals, and it is
  why the hook is worth installing rather than a defect. Testing it needed a panic inside the
  core (`ak_panic_test`), and the verdict is "the host's sink got the message before the
  process aborted". (b) **The flags are a process-wide negotiation and the first caller
  wins**: two components in one process that both initialise defensively with different flags
  cannot both choose, and the second gets a hard failure for asking. Section 3 does not spell
  that out and two hosts loading one shared library is the normal case.
- **ABI v1 open decision 5 is answered.** Zero warm misses on every uniform payload; on P2.4,
  one miss per element moving 980,938 of 981,222 bytes. Isolated with two added arms whose
  mean is P2.4 exactly, and with prost carried as the floor: the mechanism costs about
  **1 to 3 percentage points of an encode** on the payload built to defeat it, and nothing
  on any uniform one.

## Next step

Nothing is outstanding. Stage 5 closed the three items the previous session's list named,
and each produced a result the list did not predict.

If more is wanted, in the order I would do it:

1. **The `latin1`/`wide` content sets on the remaining payloads**, and the SIMD validator on
   a machine without AVX2. One machine is one machine. This is the only item from the old
   list still open, and it was the lowest-value one then and now.
2. **Consume `ffi/corpus/`** (W8, 336 vectors, `CONTRACT.md`). The python slice is its first
   consumer and a second would be worth having. Not started here: items 1 to 3 of the
   session's brief came first and this was explicitly ranked below them.
3. **A concurrency suite over the RPC half.** D16 was an RPC defect and stage 5's suite
   covers the codec. `ak_call_unary` with an assertion per response, rather than stage 4's
   throughput figure, is the thing that would replace the accident that found D16.
4. **ThreadSanitizer over the cdylib.** Stage 5's suite asserts outputs; it is not a race
   detector, so a defect that races without changing bytes at this thread count passes.
5. **The unbatched element form on decode**, and the pull family under decision 11's
   unknown-field capture. Both are deliberately not built; see "what is not measured".

## Correctness

- All four arms byte-identical to `ffi/schema/generated/manifest.json` on every payload they
  cover, and each decodes its own output back to an equal value. `gen/stage2.sh` step 2.
- The manifest is validated (stage 1), so it is the oracle; no arm is checked against another
  arm alone.
- `prost` and `armonik` are two independent encoders over two independently built object
  graphs. `core-native` and `core-ffi-rust` **share the codec** and so validate only the
  binding, which the log states.
- The boundary is checked structurally, not assumed: `gen/stage3.sh` step 4 shows the ABI
  entry points as undefined dynamic imports.
- **And the no-boundary control is checked in the opposite direction** (R5's second half, as
  amended after this slice's inlining audit): `gen/inline_check.sh` prints the entry point's
  size and the calling closure's size from the artifact, both directions, and it now runs as
  a step of `gen/stage2.sh` and `gen/stage3.sh` rather than only from `gen/inlining.sh`. If a
  closure is ever larger than the traversal it calls, `core-native` has been fused into the
  benchmark loop and every subtraction against it stops being valid — silently, because the
  subtraction would go on producing a plausible number. Whether that happens depends on LTO,
  on `#[inline]` on the entry point and on whether the entry point is generic, none of which
  appear in a configuration line, which is why it is a build step and not a note.
- **The pull arms are gated by VALUE identity, not byte identity**, because pull is
  decode-only and byte identity is an encode notion. 16 payloads over 7 roots, four decoders
  each: the independent `armonik` arm, the push arm, and the two pull arms, all agreeing.
  M7 is in the set, which matters: its bytes interleave two repeated fields of one type, so
  it is the payload that exercises section 7.3's flush on a foreign tag, and that flush is
  emitted by the traversal the two families SHARE.
- **And the pull arms have a structural gate byte identity cannot give.** A record is
  written exactly where the push family makes a reverse call, so the counts must be equal.
  They are, to the digit, on all thirteen counted payloads. Byte identity says two arms
  agree on the ANSWER; this says they agree on the STRUCTURE, which is what "one traversal
  emitter, not two" needs.
- **The push family's own gate is re-run whenever the core changes.** Stage 5 added
  `ak_parse_*` beside `ak_decode_*`, a field to `DecCtxImpl` and `ak_init`; "additive" is a
  claim about behaviour, so `gen/pull.sh` step 2 re-runs byte identity across all four arms,
  and the crossing counts were re-taken and are unchanged to the digit (M1 9/6, M2
  10.024/7.004).
- **The concurrency suite's own control is that it can fail**: obligation 12.5's standard is
  that a one-shape suite reports zero wrong bytes whether a defect is present or absent, so
  a suite that has never reported one has shown nothing. `gen/concur.sh` plants a contract
  violation and requires it to be caught.

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| D1 | `ffi/schema/emit/payloads.py` | implicit-presence zero leaf written | **fixed by the aggregating session in `07d3e05`**, re-verified here at 16/16 |
| D2 | this container | no rustc 1.88, so the declared MSRV is unverified | open, cannot be fixed here. Stated on every log |
| D3 | `crates/ak-core` | an rlib let rustc inline every ABI entry point into the host, and the boundary-call counters still incremented because the counting code was inlined too | **fixed**: cdylib plus a dynamic-link build script, and `gen/stage2.sh` step 4 now checks it every run |
| D4 | `gen/rust_build.py` | `all_absent` was short-circuited by the `half_absent` rule for even-tag Timestamps | **fixed** and swept: one `absent_expr`/`zero_expr` pair for every kind |
| D5 | `gen/rust_facade.py` | the enum guard ran `to_i32()` twice, and `is_some()` was followed by `as_ref().unwrap()` | **fixed** and swept: guard and value emitted as one statement per field |
| D6 | `gen/rust_abi.py` | a zero-length span went through the lossy-UTF-8 path in the generated accessor | **fixed**: empty fast path in `s_of` and `b_of` |
| D7 | `crates/ak-core/src/lib.rs` | `ak_fail` cast its context to `EncCtxImpl` unconditionally | **closed with M3**, as agreed: both contexts begin with a `CtxHeader { kind, err }`, the decode guard reports through `ak_fail`, and the decode entry point returns it |
| D8 | `gen/rust_abi.py` | the emitted arm for a singular message child read its length prefix from the root reader instead of the reader at its own depth | **fixed**. Only reachable at depth two or more, so M1 could not see it |
| D10 | `crates/stage1-validate/src/build.rs` | the hand-written prost builder did not know M6's new packed enum field | **fixed**, and P6.1 re-validated against prost at 123,354 bytes, with the log to show it: `stage1-manifest-vs-prost-packed-enum.log` |
| D16 | `crates/ak-core/src/rpc.rs` | `ak_call_unary` took `*mut ak_client` and mutated a shared `Grpc`, so two host threads calling it at once raced. It worked at 1 in flight and failed outright at 8 | **fixed**: the client holds the `Channel`, a call clones it and builds its own `Grpc`, and the call takes a shared reference — the shape section 9 already implies. Found by the concurrency arm, which is what it is for |
| D15 | `crates/stage1-validate/src/build.rs` | the hand-written prost builder still used the old adapter rule after `0c2d4d7f` | **fixed**, and the conformance run is what said so: the three generated-builder arms matched the new hashes and the prost arm did not. First time the two independent construction routes have caught anything |
| D14 | `crates/harness/src/bin/bench.rs` | the M4 to M7 `core-native` encode case allocated a fresh `Vec` per call and grew it by doubling, while every other arm reused a buffer. It measured `core-native` at **1.412** of prost on P5.4, a false regression | **fixed**: the arm uses the same reused `Enc` as M1 to M3, and 1.412 became 1.018 |
| D13 | `gen/ir.py` | the first `direct_fields` walked singular message children only, so ABI v1 section 8's refusal found nothing and refused nothing | **fixed**, and `gen/check_direct.py` exercises both refusal cases on every run. Same class as D12, caught the same way: by running it against a case that must fail |
| D12 | `gen/rust_core.py`, `gen/rust_abi.py` | every backend iterated `Message.plain`, which excludes oneof members, so adding `ListProbeResponse` produced a complete-looking codec that ignored `Probe.body` and said nothing | **fixed**: both walkers call `b.oneof(...)` explicitly, so a backend that cannot do the shape raises rather than skipping it |
| D11 | this slice's reporting | P6.1's re-validation was reported with no log behind it, against a stale log that still showed the old size | **fixed**. Every schema change now re-runs `gen/stage1.sh` into a dated log before the result is quoted |
| D19 | the repository's root `.gitignore` | line 30's `[Bb]in/` (meant for .NET build output) silently excluded **every measurement binary in this slice** — `conformance`, `counts`, `bench`, `content`, `shapes`, `rpcbench` — from stage 2 onward. The logs were committed and the code that produced them was not. Same class as D11 with the sides swapped, and found the same way: by reading what `git status` did NOT list | **fixed** in `poc/rust/.gitignore`, which re-includes `crates/*/src/bin/**`. All seven bins are now tracked. **Other slices are likely to have the same hole** and it is worth one `git ls-files` each |
| D17 | `gen/rust_abi.py` | the decode entry point returned the sticky error slot and nothing ever cleared it, so the first rejected decode poisoned every later decode on that context. Invisible while nothing on the decode path could fail | **fixed**: `ak_decode_X` clears the slot at entry, which costs no crossing (counts re-run unchanged: M1 9/6, M2 10.024/7.004). A host-side `ak_dec_err_reset` was written first and reverted — one extra forward crossing per decode for nothing. The regression is in `decpolicy`: a good decode after a rejected one must succeed |
| D18 | `gen/decpolicy.sh` (first version) | it built and ran each policy in turn, so the lossy build was always the first process. The same source measured `core-native` P1.2 ascii at 0.778 of prost in one invocation and 0.88 in the next, with the prost control unmoved | **fixed**: the three binaries are built first and run round robin with a rotating order, and section 4 prices the policies against each other in ONE process. The control still drifts up to 7 percent on the two `wide` rows and the log says so |
| D9 | `gen/rust_abi.py` | an element run did not restore the codec's open-field state, so the second and later chunks read whatever the last element left behind. It cost 448 length-prefix misses in 500 elements, and it reads `open_tag` too, so a host that chunks would write later chunks under the inner field's tag | **fixed**: every element and run entry point saves and restores the open state, and `gen/stage3.sh` step 3 carries a regression for it. **The payload set could not have caught it**: byte identity passed only because `ListTasksDetailedResponse.tasks` and `TaskOptions.options` are both tag 1 |

## What is not measured

**A pass for completeness, not for brevity** (README section 11: the report is assembled from
these lists as much as from the verdicts). Every shape and payload of `design/SHAPES.md` is
now covered, so what follows is what remains after that.

### Shapes that the payload set cannot reach, so nothing measures them

Four, all reported to the aggregating session and none fixed here:

1. **A packed repeated enum** was claimed for M2 and existed nowhere. **Fixed in the schema**
   at `945d3cd1` as `MetricsBatch.statuses`, and now measured.
2. **Nesting past depth 3.** `SHAPES.md` claimed depth 6; the description's maximum is 3, and
   the real schema reaches 6 through the filter and request family, which this response-only
   payload set does not carry. Levels 4 to 6 are unexercised, and with them ABI v1 decision 7
   (the decode recursion limit), which nothing here can reach.
3. **The adapter's non-injective states.** `P4.1`'s `error` is a sentence in all 200 elements,
   so the plain wire form's `Ok` and `Invalid` states never occur, and they are the two the
   map cannot separate. Checked here by hand-written state instead.
4. **The adapter's nested site is worse**: `emit/payloads.py` fills `TaskOutput.success` and
   `TaskOutput.error` independently, so P2.x contains `success=true, error=<sentence>`, a
   state no adapter over `{Ok, Error(d)}` can represent. The nested adapter site is therefore
   not reachable at all, and this slice keeps `TaskOutput` as a plain struct there.

### Behaviour this design changes and this slice cannot price

- **Unknown-field retention (ABI v1 decision 11).** Nothing here retains an unrecognised
  field from decode to re-encode, and neither does prost. Rust is the one incumbent that
  already drops them, so this slice is structurally the wrong place to measure what removing
  the guarantee costs the other four languages.
- **The transcode pair** (README section 10 item 4). A Rust `String` cannot hold an unpaired
  surrogate, so this slice **cannot construct the input** that makes protobuf-java and
  Google.Protobuf disagree. Unreachable, not unbuilt.
- **The direct-argument path's actual win** (ABI v1 section 8). Built and byte-identical, but
  its value is a pinned buffer on the JVM; on a Rust host there is nothing to pin and the copy
  is a copy either way. P5.3 and P5.4 are a memcpy figure and are not evidence for it.

### ABI surface that is specified and not built

- ~~The pull decode family~~ **BUILT** (stage 5). What remains unbuilt in it: decision 11's
  unknown-field capture is deliberately not wired to pull (the bag is a candidate and pull is
  a family; pricing one through the other would make neither answerable, so `ak_parse_*`
  skips unknown fields as the default push path does); `ak_bdr_reserve` exists and no arm
  calls it, so what a COLD first parse costs is unmeasured; a record larger than the chunk is
  `AK_ERR_CAPACITY` and no test makes that happen; and the drain contract requires an
  8-aligned destination, which nothing prices for a host that cannot give one.
- **Most of the RPC half** (section 9). Built and measured: the blocking unary call over a
  channel. **Not built**: the callback and completion-queue delivery modes, metadata,
  deadlines, the gRPC status code as a number, cancellation (section 9 gives the blocking
  call a handle so it can be cancelled and `ak_call_unary` takes none), retry and backoff,
  TLS, streaming, a real network, failure injection and the server side. The RPC half's case
  is **behavioural** and none of that behaviour is exercised: stage 4 measures the call path,
  which is the half of section 9 whose case was never in doubt.
- **`ak_init` and the lifecycle** (section 3): **built and exercised in stage 5**, and what
  is NOT built there is listed rather than implied. Built: `ak_init`, the options struct and
  the flags, the `ak_err` out-parameter, the ABI-version check, idempotence and its refusal
  case, `ak_build_id`, the log bridge, the panic hook, the `AK_ERR_UNINITIALIZED` guard on
  every entry point (behind `init-guard`), and the one-shot rule under 8 racing threads.
  **Not built, and none of it is cheap to fake**: the rustls crypto provider installed by
  name (this build does not link rustls; `AK_INIT_NO_CRYPTO` names the case);
  `tracing::set_global_default` and `log::set_logger`, because the bridge is the ABI's
  `ak_log_fn` and not those crates; no runtime, context or client, so **configuration
  precedence (setter > environment > JSON > defaults) and the `worker_threads` default are
  still unexercised** — both belong to `ak_context_new` and `ak_runtime_opts`, which are the
  RPC half. A one-line install nobody has run is not evidence.
- **Group layout export and assert at load** (section 10 and obligation 12.3). Both sides
  compile against one generated header here, so there is nothing to disagree — which means the
  insurance that matters to a hand-layout host (FFM) is untested.
- **The `ak_span.coder` hint** (decision 4): present in the struct, read by nothing.
- **Message size limits and the recursion limit** (decisions 7 and 8): `AK_ERR_LIMIT` and
  `AK_ERR_DEPTH` exist and nothing sets or enforces either.
- **A union group layout for a oneof**: the flat discriminant form is built; the union is an
  alternative the aggregating session has ruled out on layout-reproducibility grounds, not a
  measured one.
- **The unbatched element form on decode**: only the run form is built.
- **The pull family's no-boundary control**: `core-native` is the PUSH traversal emitted into
  the host and there is no `core-native-pull`. The question stage 5 asks is push against pull
  THROUGH THE SAME BOUNDARY, which R4's sharpened half says to ask as a delta between two
  arms in the same rounds, and it is — but a reader wanting "what does the pull traversal
  cost with no boundary at all" will not find it here.
- **Encode has one delivery family and section 7.1 is about decode**, so nothing in stage 5
  bears on the encode column.

### Error paths

- `ak_fail` is now reached on the **decode** side by a real failure: a malformed UTF-8 span
  under either rejecting build reports `AK_ERR_TRANSCODE` through it, and `decpolicy`'s
  section 2 exercises it every run. On the **encode** side it is still reachable only through
  a panic in the generated guard: no test makes a host fail mid-run deliberately, so the
  codec's rollback of a half-written field (section 6, "the widest hole in the drafted
  interface") is written and unexercised.
- An unrecognised oneof `body_case` is refused with `AK_ERR_ABI`; nothing in this build can
  produce one, so the path is built and not exercised.
- Malformed wire: `AK_ERR_MALFORMED` and `AK_ERR_TRUNCATED` are produced by the reader and no
  vector exercises them. Malformed **UTF-8** is exercised, by one vector with one byte
  overwritten, and returns `AK_ERR_TRANSCODE`. Whether that is the right code is raised and
  not taken: `AK_ERR_MALFORMED` ("invalid wire") is the other defensible reading, since
  proto3 makes invalid UTF-8 a parse error and the rejecting path has no transcoder on it.

### Measurement coverage

- **Content sets**: `latin1` and `wide` on P1.2 and P2.2 only, encode and decode. Not on the
  other payloads, and with no manifest oracle (byte identity against the prost arm instead).
- **The decode UTF-8 policy**: priced on P1.2 and P2.2 only. Every other payload's decode
  figure in every other log is a **lossy-policy** figure. Nothing prices what a reject does
  to a CALLER: a conformant parser rejects the whole message, so one bad string loses a batch
  of a thousand results, and that is a behavioural cost this slice cannot put a number on.
- **The opt-in diagnostic encode mode** (decision 3's surviving encode-side value) is not
  built, by instruction.
- **The unknown-field bag**: the NON-empty bag's throughput (the vectors are tens of bytes,
  so "what retention costs when it is actually retaining" is unpriced); the host data-model
  cost of 24 bytes per message instance, which every arm in that build carries so it cancels
  in the deltas; a oneof's message member, which gets no bag and is handed a null capture
  buffer rather than the wrong one; the inner slots of a non-leaf element; decode-side
  crossings for unknown runs, which the counting build was not extended to count because the
  payload set has none; and merge-by-tag, not built and not priced by instruction.
- **The zeroed-group variant**: encode only, top-level element group only, M1/M2/M3 only.
  The nested groups inside an element keep the total fill and are not priced separately, and
  what the variant costs a host that is not Rust is a property of that host's branches, not
  of this measurement.
- **Concurrency**: the CODEC half is covered by stage 5's suite (obligation 12.5) and the
  rest is not. **The RPC half is not in the suite**, and D16 was an RPC defect: stage 4's
  8-in-flight arm is the accident the obligation exists to replace, and a suite over
  `ak_call_unary` with an assertion per response does not exist. **Nothing here is a race
  DETECTOR** — the suite asserts outputs and does not run under a sanitiser, so a defect that
  races without changing bytes on this machine at this thread count passes; ThreadSanitizer
  over the cdylib is the obvious next step and is not done. Two shapes, as the obligation
  asks, and not more. 4 vCPUs, which R9 makes a lower bound on contention, and the throughput
  half is visibly noisy: the per-context shared-site row moved by up to 6 percent between
  runs of the same binary, so the claim rests on the ranges not overlapping and not on any
  single figure.
- **Allocation and footprint**: nothing counts allocations, peak memory or the arena's real
  cost in any arm.
- **Linkage**: a shared library only. A C or C++ host that statically links the same core
  gets a direct call and pays less than the 1.8 ns measured here.
- **The floor**: no rustc 1.88 exists in this container, so "builds and passes on the MSRV" is
  a declaration and not a run. README section 5.2's arms b and c do not exist for Rust, and
  for Rust the floor is the target, so only arm a is meaningful — but arm a is unverified as
  being at the floor.
- **`packages/rust`'s own `armonik` crate**: the `armonik` arm reproduces its *pattern* and
  does not use `armonik-macros`, whose expansions reference `armonik`-internal paths and read
  a descriptor built from `Protos/V1`. The `armonik` column prices this generator's emitter,
  and a defect in it was worth 20 to 40 percent before it was swept, so that caveat is load
  bearing.
- **Hardware**: one 4-vCPU container, one x86-64 microarchitecture, AVX2 present. The SIMD
  validator result in particular is one machine.

## Slice-specific notes

- **THIS SLICE ADDED TO THE SHARED CORE, AND THE AGGREGATING SESSION SHOULD SAY WHETHER IT
  STAYS.** R0 allows a slice to add to `poc/codec/` additively and says a change to existing
  behaviour is not a slice's to make. Stage 5 made five additions there. Four are plainly
  additive new surface: `ak-rt/src/bdr.rs`, the `ak_parse_*` and `ak_bdr_*` entry points,
  `ak_init` and its neighbours, and `ak_panic_test`. **Two are feature-gated alternative
  arrangements and they are the ones to look at**:
  - `ak-rt` feature **`global-widths`** builds the process-global learned-width table that
    ABI v1 section 6 explicitly refuses. It exists so the refusal is measured rather than
    inherited from the java slice, and it is off by default.
  - `ak-core` feature **`init-guard`** emits section 3's `AK_ERR_UNINITIALIZED` check into
    every entry point, so the rule can be priced. Off by default, and emitted by a post-pass
    over the finished codec text so a new entry point gets it by existing rather than by
    someone remembering.

  With both off, the emitted codec and the runtime behave exactly as before: the first
  stage-5 commit was 2,048 insertions and zero deletions across `poc/codec/crates`, and the
  push family's byte-identity gate and crossing counts were re-run and are unchanged. But a
  feature that builds a refused arrangement is a judgement call and it is flagged here rather
  than buried.
- **The other slices need to regenerate.** `codec.rs` and `abi.rs` changed (additively), and
  cpp, java and this slice all write those paths from one emitter. A `gen/generate.py --check`
  in another slice will read STALE until it regenerates, and will then agree, because the
  text comes from the same emitter and the same description.
- Reads `packages/rust`. Does not edit it.
- This slice's `core-ffi-rust` number is what every other slice subtracts to separate
  interface cost from runtime tax, so it is the one arm that must exist before the managed
  slices are read. **It is now available**, with the crossing priced at 1.8 ns.
- `ffi/.gitignore` (added by the aggregating session) re-includes `logs/**` and
  `poc/*/gen/**`; `poc/rust/.gitignore` is now only `target/`.

## Log index

| Log | Configuration | What it establishes |
|---|---|---|
| `ffi/logs/rust/stage1-manifest-vs-prost.log` | rustc 1.94.1, prost 0.14.4, protox 0.9.1, 4 vCPU Xeon 2.80GHz; nothing timed | the original disagreement: 8 of 16 payloads |
| `ffi/logs/rust/stage1-isolate-zero-leaf.log` | as above | one change accounts for all eight |
| `ffi/logs/rust/stage1-second-encoder.log` | as above, plus prost-reflect 0.16.5 | the rule is protobuf's, not prost's; the corrected sizes and hashes |
| `ffi/logs/rust/stage1-manifest-vs-prost-after-fix.log` | as above, **schema at `07d3e05`** | 16 of 16 after the zero-leaf fix. **Its P6.1 row (116,954 B) is superseded by the log below**; every other row still stands |
| `ffi/logs/rust/stage1-manifest-vs-prost-packed-enum.log` | as above, **schema at `945d3cd1`** | 16 of 16 including P6.1 at 123,354 B, the payload the packed enum moved and the one nothing had checked |
| `ffi/logs/rust/stage4-rpc.log` | rustc 1.94.1 release, tonic 0.14 over loopback h2 no TLS, cdylib boundary, server in-process, 4 shared vCPUs | **Two crossings per RPC and zero per field.** CPU per RPC 0.91 to 1.10 of tonic over two processes at 1, 8 and 16 in flight: no measurable difference. The carrier-thread row is reported empty and not substituted for |
| `ffi/logs/rust/stage1-manifest-vs-prost-adapter.log` | as above, **schema at `0c2d4d7f`** | 16 of 16 after the adapter fix. Supersedes the P2.x and P4.1 rows of the two earlier stage 1 logs |
| `ffi/logs/rust/stage3-M2-M4-revalidated.log` | as stage3-M2 | M2 and M4 re-measured after `0c2d4d7f`. Crossings and decision 5 unchanged to the digit; ratios tighter and two moved toward parity. **Supersedes the M2 rows of `stage3-M2.log` and the M4 rows of `stage3-M4-M7.log`** |
| `ffi/logs/rust/stage3-M4-M7.log` | as stage3-M2, ASCII, guard on | M4 to M7: byte identity on P4.1, P5.1 to P5.4, P6.1 and P7.1, with the class labelled per row; ABI v1 section 8's generator-time refusal exercised; the adapter's two wire forms checked by state; M7 by decode and permutation |
| `ffi/logs/rust/stage3-M3.log` | as stage3-M2, ASCII, guard on | M3: byte identity on P3.1; explicit presence as three cases x three fields x four arms, all agreeing; the oneof by member including the payload-free one; seven unknown-field vectors, hand-built; 3 crossings per 200 elements in both directions; one timing row set |
| `ffi/logs/rust/stage3-reproducibility.log` | rustc 1.94.1 release, prost 0.14.4, guard on, ASCII; 3 builds under a layout perturbation, 5–6 runs, every arm rebuilt together | **The M1 and M2 tables re-taken under R4 as sharpened, and the verdict.** A ratio reproduces to ±0.02–0.05 here; the published encode figures are 0.10–0.27 away and cannot be re-derived, because `cc7f68c6` does not contain the benchmark binary (D19). Carries the replacement table, the two artifact facts it rests on, and the re-confirmation of the three qualitative findings |
| `ffi/logs/rust/stage3-unknown-fields.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on, ASCII; three runs, plus the predicate check and a positive control | **ABI v1 open decision 11, as an arm.** The bag as ONE bytes blob leaves the batching predicate untouched; as a repeated field it destroys it (9 leaf messages to 0). The empty bag is free on decode and costs 1–12% of an encode, with the decision-9 interaction going both ways. The bag's bytes round-trip exactly; the layout does not when an unknown tag is interleaved. **Also records that the published M1/M2 ratios no longer reproduce on this container** |
| `ffi/logs/rust/stage3-zeroed-group.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on, ASCII; six runs, plus three deliberate-break positive controls | **ABI v1 open decision 9 candidate, as an arm.** The zeroed-group element fill: 0.719–0.766 of the total fill on M1's absent path (the encode inversion goes 1.11–1.19 → 0.82–0.86 of prost), 0.986–1.014 on M1's full path, 0.970–0.985 on P2.2, 0.983–1.007 on P2.5. Carries the three controls that prove the zeroed path is the one running and that present-and-zero is load-bearing |
| `ffi/logs/rust/stage3-inlining-term.log` | rustc 1.94.1 release (lto OFF, PIE), prost 0.14.4, cdylib boundary, guard on, ASCII; five arms in one process, three runs, plus an artifact check | **The audit of the per-element interface cost.** `core-native` is NOT inlined into the benchmark loop in the binaries the published figures came from (largest closure 472 B against a 4,299/11,311 B traversal), so there was no inlining advantage to subtract. Two added no-boundary arms confirm it: the inlining term is −0.10 to +0.01 ns/element on P1.3 encode against 11.3–11.4 for the group. Also finds that P1.1/P1.2 decode should carry no per-element figure at all |
| `ffi/logs/rust/stage3-decode-utf8-policy.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on; three POLICY BUILDS run round robin with a rotating order, plus one in-process table; simdutf8 0.1, AVX2 present | **ABI v1 open decision 3, third framing.** Validate-and-reject on decode costs 0.54 to 1.10 of today's lossy string path depending on content set, and 0.36 to 0.71 with `simdutf8::basic`, because `from_utf8_lossy` already validates. It moves the decode ratio against prost in this slice's favour and makes the comparison like-for-like, since prost rejects too. Carries the malformed-input case, the sticky-slot regression (D17) and the ordering hazard (D18) |
| `ffi/logs/rust/stage3-content-sets.log` | as stage3-M2, plus simdutf8 0.1 as one arm; encode and decode over P1.2 and P2.2, all three content sets in ONE process | ABI v1 open decision 3: the scalar validator costs 2.2 to 3.0 times its ASCII self on non-ASCII content and loses 2.0 to 2.6 to prost; a SIMD validator with the same contract recovers half to two thirds of it; decode is unaffected in ordering |
| `ffi/logs/rust/stage3-M2.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on (section 6 off), ASCII, 4 shared vCPUs | M2 over P2.1 to P2.5: byte identity across four arms plus value identity across the three facade decoders; 7.004 crossings per task on decode and 10.02 on encode; ABI v1 open decision 5 answered and isolated; the two shape-coverage findings; the guard priced on a shape that makes 7 to 10 reverse calls per element |
| `ffi/logs/rust/stage5-pull-decode.log` | rustc 1.94.1 release, prost 0.14.4 (tonic-prost 0.14.6's decode path READ, not assumed), cdylib boundary, guard on, ASCII; two suite invocations, three timed runs each, arms interleaved in one process | **ABI v1 section 7.1's PULL family, built, and open decision 2 answered on both halves.** One `dec_walk` serves both families and the structural control says so: records written == reverse calls push would make, to the digit, on all thirteen counted payloads, with pull's reverse count zero. Crossings go from 3,501 per message (push, per element) to 16, or 3 with one drain chunk. Pull costs this host −5% to +18% of a push decode, with the opaque-replay control showing it is not an inlining artifact, and its cost tracks the record-to-wire byte ratio |
| `ffi/logs/rust/stage5-concurrency.log` | rustc 1.94.1 release, cdylib, guard on, 4 vCPU; two builds (per-context and `--features global-widths`), three runs each, separate target dirs | **Obligation 12.5's concurrency suite.** Correctness clean: 0 wrong in 2,840 encodes and 2,840 decodes across sequence, 2/4/8 threads, on both builds. The positive control (four threads, one context) is **not** wrong bytes but a PROCESS ABORT, because a panic in the core cannot unwind through an `extern "C"` frame — a hole beside the one section 5 already calls the widest. Section 6's global-table claim measured: 0.5 to 11 percent, only above one thread and only on shapes sharing a site, with a disjoint-site control and the width flipping counted |
| `ffi/logs/rust/stage5-lifecycle.log` | rustc 1.94.1 release, cdylib, two builds (default and `--features init-guard`), each case in its own process | **ABI v1 section 3, built and exercised for the first time in this branch.** Fourteen cases: the state machine, the version check, idempotence and its refusal, the log bridge and its flag, the panic hook and its flag, 8 threads racing `ak_init`. Two cases failed first and both failures were the specification working — the core's hook does not see a HOST panic (two copies of `std`), and the flags are a process-wide negotiation the first caller wins. **What the `AK_ERR_UNINITIALIZED` guard COSTS is not in it**: that run is `gen/guardprice.sh`, it needs the box to itself, and no figure is quoted until it exists |
| `ffi/logs/rust/stage2-four-arms-M1.log` | rustc 1.94.1 release, prost 0.14.4, cdylib boundary, guard on (section 6 off), ASCII, 4 shared vCPUs | byte identity across four arms; crossing counts; the boundary is a real dynamic import; the crossing costs 1.8 ns; the ratio table above; the guard is free; UTF-8 validation costs 25-30 percent of an encode |
