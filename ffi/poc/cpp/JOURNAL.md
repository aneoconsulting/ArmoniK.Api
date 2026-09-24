# cpp slice: journal

What was tried, what it measured, what it refuted, in order. A later reader looks here to
find out that an option was already refuted and by what.

## W4, session 1 — the slice from zero

Starting state: `poc/cpp/` held `STATE.md` and an empty journal. The prior C++ POC
(2026-09-04) is **not** ported: it covered 16 of 179 messages, all flat, and was built
against the base design rather than ABI v1.

### 1. The generator, and what R1 actually forces

R1 forbids a hand-written codec anywhere in the comparison, and it also says one
description drives everything. The strong reading is that the *core* must be the same core
the rust slice measures, not a second one that agrees with it. So `gen/generate.py` puts
`ffi/poc/rust/gen` on `sys.path` and imports `ir.py`, `rust_abi.py` and `rust_core.py`
**read-only**: `rust_abi.emit_codec(ir)` writes `core/src/generated/codec.rs` and
`rust_core.walk_encode` / `walk_decode` drive this slice's own C++ backends. The core crate
takes `ak-abi` and `ak-rt` as **path dependencies on the rust slice's crates**, not as
copies, so there is one definition of every group and one Enc/Dec runtime on the Rust side.
Nothing under `poc/rust/` is written.

Seven new backends were needed and all of them are this slice's: the C header, the C++
facade, the facade payload builder, a second payload builder over protoc's own types, the
no-boundary control codec, the host binding and the layout export.

**Refuted before it was tried: emitting the groups in the description's order.** C inlines a
child group by value, so `ak_efix_Probe` needs `ak_efix_Empty` complete, and `Empty` is
declared after `Probe` in `shapes.json`. Rust does not care. That is a compile error, so it
is cheap — but the same order drives `core/src/generated/layout.rs`, where a mismatch would
be silent and would have made ABI v1 section 10's whole check pass vacuously. One
`abi_order_topo()` is used by both.

### 2. Correctness first (R2), and what it found

361 checks, 0 failures, against `ffi/schema/generated/manifest.json`, at `-std=c++11`,
`c++14` and `c++17`, floor and target implementations, shared and static linkage. Five arms
byte-identical on every payload: protobuf non-arena, protobuf on an Arena, the no-boundary
control, the C ABI arm, and the C ABI arm's three variants (zeroed fill, unbatched, host
transcoder). Two independent construction routes reach the same bytes.

**One divergence found, and it is the INCUMBENT's, not this slice's.** On P2.5 protobuf C++
writes 19,712 B where the canonical form is 19,632 B. The cause is arithmetic rather than a
guess: `half_absent` empties 2 of 4 map values on each of 20 elements, and protobuf C++
writes a map entry's key and value unconditionally, at 2 B for an empty value; the
manifest's canonical form omits an implicit-presence leaf holding the proto zero, and prost
does too. 40 x 2 B = 80 B, exactly the delta. The values agree, protobuf reads the canonical
bytes and re-serialises them to its own form, so this is a wire-form difference rather than
a different message. It is reported, not fixed: `schema/` is not this slice's to change.

**Also established rather than assumed: protobuf C++ REJECTS malformed UTF-8** in a proto3
`string` (`ParseFromString` returns false). So ABI v1 decision 3's "decode rejects" is the
like-for-like comparison here, and the rejecting policy is this slice's default. The lossy
policy is built as a separate binary so the two can be priced.

**protobuf C++ needs deterministic serialisation to be byte-stable** on any message with a
map, so every conformance run and the headline timing arm set it. The non-deterministic form
is carried as a separate row on P2.2 rather than folded into the incumbent, because the sort
is real work and hiding it would flatter this slice.

### 3. ABI v1 section 10, which the rust slice could not exercise

Both sides of the rust slice compiled against one generated header, so "group layouts are
exported and asserted at load" had nothing to disagree with. Here the two sides genuinely
restate the layout: `#[repr(C)]` in Rust, a `struct` in a C header written by a different
backend. The core exports 380 layout facts and the host compares them with what its own
compiler produced.

**It caught a real disagreement on its first run**: 43 of 380 facts differed, because the
`.so` on disk was built before the topological-ordering fix. That is exactly the failure
mode section 10 names — the two sides disagreeing about where a field is — and it was a
stale artifact rather than a design fault, which is the more common cause and the one a
runtime check is for.

### 4. README 5.1's hard stop, checked mechanically and seen failing

`gen/odr_check.sh`: one TU at `-std=c++11` and one at `-std=c++17`, linked together, 144
layout facts compared elementwise, facade objects passed across the seam both ways.
**0 moved.** With `-DAK_ODR_BREAK`, which gives `ak::Optional` one extra member at C++17
only, **49 move** and the check fails. A guard with no failing test is a guard nobody has
seen work; this one has been seen.

The facade therefore names no `std::optional`, `std::variant` or `std::string_view`. The
oneof is a generated C++11 sum type over an unrestricted union: `clear`, `copy_from` and one
`set_` per member, ~60 emitted lines for five variants against Rust's 14.

### 5. R5, from the artifact, and what C++ actually exposes

`gen/boundary.sh`, 10 checks. The shared arm's 37 `ak_*` symbols are undefined dynamic
imports. The static arm has them defined and **called** (3 to 5 call sites each), with entry
point sizes printed.

**R5's named C++ hazard does not fire here, and the reason is worth recording.** `-flto`
over a statically linked core cannot inline the entry points away, because gcc's LTO only
inlines across GIMPLE it produced and a Rust staticlib's archive members are native objects.
Verified: `counts_a17_static_lto` still calls `ak_encode_ListResultsResponse`, via the PLT.
That is a fact about the toolchain pair, and it would not hold for a core compiled by the
same LTO.

The exposure that *is* real in C++ is the other half: the no-boundary control being fused
into the benchmark loop. It is not — the traversal is out of line at 1,960 B against a
largest timing closure of 790 B, and it is reached through a function pointer, so its
address is taken and a body must exist. The `-flto` build is carried as the condition that
would break it.

### 6. Three defects in this slice, all found by measuring rather than by reading

**D1. The no-boundary control was slower than the arm it controls for.** First build:
`core-native-cpp` encode measured **1.245 of protobuf C++ on P1.2** while the same codec
through the C ABI measured 0.855. A control slower than its arm makes every subtraction
meaningless. Cause: `ak::Enc` was a `std::vector<uint8_t>` with `push_back`, and through an
`Enc*` the compiler must reload the finish pointer on every byte, where Rust's `&mut Vec<u8>`
carries noalias. Rewritten as a raw cursor over a reserved block, which is also what protobuf
C++'s own serialiser does. **0.443** after. The generated traversal did not change.

**D2. Decision 9's zeroed fill was memsetting 32 KB per inner loop call.** The candidate
clears the chunk so the fill can be sparse, but only an ELEMENT GROUP is filled sparsely: a
blob run and a map entry are assigned in full. The first build cleared every chunk, and the
inner loops run once per element, so P2.2 encode paid ~80 MB of memset and measured **1.944
of protobuf** against 0.647 for the total fill. The candidate looked refuted and was not.

**D3. The zeroed fill's clear was O(arena) where the fill is O(elements)** — and this one is
a finding about the CANDIDATE, not only about this build. Clearing the whole 32 KB chunk (which
is what the rust slice's emitter does) costs the same on a 4-element payload as on a
1,000-element one: measured **+156 ns/element on P1.1 (+94.6% of a protobuf encode) and +550
on P2.1 (+45.6%)**, while winning 62% on P1.3. Clearing only `min(n, chunk)` elements removes
the inversion and keeps every win. The rust slice measured P1.2, P1.3, P2.2 and P2.5 and
could not see it.

**D4. A decoded child was copied into its option.** `ak::Optional<T>::set` took `const T&`
only, so `dst.set(from_X(...))` copied. On M5's 4 MB `bytes` field that is a second copy of
the whole payload: P5.4 decode through the ABI was **3.7 ms slower than the control** before
an rvalue overload was added, and is within 0.5% of it after.

### 7. What the numbers say about decision 1

> **SUPERSEDED — read section 11 instead.** Every figure below came from a run that
> `gen/run_all.sh` later overwrote, and from a harness with the three incumbent handicaps
> and the two control defects that the review found (C7 to C11). It is kept because the
> reasoning is still the reasoning and a reader tracing how the verdict moved should be
> able to see where it started, but no number in this section matches a committed log.


Priced as within-round deltas between two arms (R4), not as ratios to a third:

- **The group** is the largest of the three, and it is a host-side cost: the fill alone is
  21.7 ns/element on M1 and 75.5 ns/element on M2, and on the absent path (P1.3) it is 22.3
  ns/element against a protobuf encode of 18.5. That is the P1.3 inversion, and it reproduces
  in C++ with the same sign and a larger magnitude than in Rust. **Decision 9's candidate
  fixes it in C++ too** (P1.3 −75% of a protobuf encode), so C++ agrees with Rust's answer.
- **String as data is a small WIN, not a cost**: the host-transcoder arm, which is what a
  string-as-a-call form costs, is 1.4 to 3.0 percentage points of an encode slower, about
  0.6 to 1.0 ns per string, which is one reverse crossing.
- **The batching predicate is a small LOSS in C++ on the shapes the control plane moves**:
  unbatched is 1.0 to 4.9 points of an encode FASTER on P1.1, P1.2, P1.3, P2.1, P2.2, P2.5 and
  P4.1, and only wins where the crossing count per element explodes (P2.3 +6.1%, P2.4 +7.2%,
  at 125 and 311 forward crossings per element unbatched). At 1.8 ns a crossing the 32 KB
  chunk's second pass over memory costs more than the crossings it saves.

And one mechanism nobody listed: **the two-pass blob write**. ABI v1 section 4 removed the
declared expansion bound, so the core opens a length prefix of a learned width, hands the
transcoder the rest of the buffer and resolves the prefix afterwards. A host that already
holds the bytes knows the length and writes key, length and body in one pass. Measured on
P1.2's 6,000 strings, in one process: **5.0 ns against 8.7 ns per string, +3.7 ns**. That is
where most of the C ABI's encode advantage over the control goes.

### 8. The RPC arm, and the two CPU columns R9 forces

One unary call carrying P2.2 against grpc++ 1.51.1 over loopback, with the server
in-process. The first version measured whole-process CPU and reported **9.0 ms per RPC for
grpc++ and 8.1 for the core, a ratio of 0.90**. That number is nearly all server: the
generated sync service deep-copies and re-serialises a 540 KB message on every call, and
that cost is inside both arms.

Measuring `CLOCK_THREAD_CPUTIME_ID` over the CLIENT threads only gives **4.76 ms against
2.65 ms, a ratio of 0.558 at 1 in flight, 0.652 at 8 and 0.622 at 16** — and the whole-process
column is printed beside it rather than dropped, because the dilution is the thing to see.
Decoding the same response standalone in the same binary costs 3.59 ms with protobuf C++ and
2.34 ms through the C ABI, so **most of that ratio is the codec** and about 1.17 ms against
0.31 ms is the transport.

R9's other half is visible in the wall-clock column: at 1 in flight it is 8.89 ms against a
client CPU of 4.76, and it more than halves at 8. A wall-clock throughput figure here would
be a figure about the 64 KB stream window.

### 9. Two results that differ from Rust, and the reason in each case

**The decode UTF-8 policy is not free in C++.** The rust slice measured validate-and-reject
as free to cheaper, because `String::from_utf8_lossy` already validates and its recovery
path is slower than failing. A C++ `std::string` holds arbitrary bytes, so a "lossy" arm in
C++ does not validate at all — and against that, **validating costs 22 to 28 percent of a
decode** (P2.2 `ffi` 0.663-0.674 of protobuf validating, 0.515-0.540 not). The headline
column keeps the rejecting policy because protobuf C++ rejects too and that is the
like-for-like comparison, but the number prices **this slice's scalar validator**, not the
policy. A SIMD validator is the obvious next step and was not tried.

**The no-boundary control is not a lower bound on decode.** On encode `core-native-cpp` is
0.25 to 0.45 of protobuf and the C ABI hands most of that back, which is the rust shape. On
decode the control is **slower than the same codec through the ABI** on most payloads (P2.2
0.729-0.738 against 0.663-0.674, P6.1 1.241-1.273 against 0.632-0.641). The two are not the
same implementation: one traversal emitter, two languages, two compilers. So in C++ the
control prices **"generate the codec into C++"** — README section 13's option 2 — rather than
the interface cost, and only the encode side has the two close enough for a subtraction to
mean what the rust slice's did. Said plainly rather than left for a reader to infer.

### 10. What the C++ column says in one line

Encode through the C ABI is **0.52 to 0.91 of protobuf C++ on every uniform payload** and
**1.64 on the absent path** until decision 9's fill is used, after which the inversion is
gone. Decode is **0.59 to 0.95**. The no-boundary control is **0.25 to 0.45 on encode**, so
the codec is two to four times protobuf's speed and the boundary hands most of it back —
which is the same sentence the rust slice ends on, with a different constant.

## W4, session 2 — 28 adversarial review findings

Four reviews, 28 deduped findings, nine raised independently by two or more reviewers.
Everything below quotes the committed logs, which is the rule this session was given
after the first one did not.

### 11. The seven findings that moved a number

**The incumbent was handicapped three ways on encode and it was worth about 8 points.**
`pb_serialize` did `out->clear()` then `out->resize(ByteSizeLong())`, and `clear()` sets
the size to 0, so the resize **value-initialised the whole output on every call** — a full
zero-fill of up to 4 MB that `SerializeToString` does not do. It also built an
`ArrayOutputStream` and a `CodedOutputStream` per call, and it forced deterministic map
ordering on every payload. `pb` is now `SerializeToString`. P1.2 encode `ffi` moves
**0.895-0.913 → 0.944-0.988**; P2.2 **0.691-0.699 → 0.772-0.795**. Confirmed and fixed.

**And there was no encode floor arm**, which R2 requires for a ratio far from 1 — `native`
rows sat at 0.25. There is one now: `memcpy` is **0.024 to 0.097 of a protobuf encode** on
the element-bearing payloads, so the incumbent and the core are both about twenty times
the cost of copying the answer.

**Protobuf validates UTF-8 on serialize and the spec says the core does not**, so the two
were not doing the same work. `ffi-valtc` makes that visible: **+26.9 % to +30.2 % of an
encode**. But the row prices *this slice's scalar validator*, which the string-path table
below shows is an order of magnitude off a real one, so it is an upper bound and not the
like-for-like row the review asked for. Confirmed, arm added, claim qualified.

**The decode control never reserved, and that was most of the gap the report leaned on.**
The binding reserves at every batched fill; the control did not. Adding `reserve` on
packed runs — exact for fixed-width, an upper bound for varints — takes **P6.1 decode from
1.241-1.273 to 0.861-0.873**. `-DNDEBUG` was also missing, so protobuf's `GOOGLE_DCHECK`s
were compiled into the incumbent's hot path, and `gen/opt.sh` shows **`-O3` does not close
what remains**. The generated oneof now has a `noexcept` move, without which the enclosing
message had none and every vector growth copied.

**So "two languages, two compilers" is withdrawn and replaced by a mechanism that is an
ABI property**: the batched run tells the host how many elements are coming, so the
binding can reserve; a streaming decoder cannot. That is the run form earning its keep,
and it is a better sentence than the one it replaces.

**Decision 1's ranges were quoted over a subset of their own tables.** The delta tables now
print **every row** with whether lo and hi share a sign. One row does have the opposite
sign and it is named: P5.2 at −3.70 % on the string-as-data delta, which is M5's 64 KB
`bytes` field on the direct-argument path, two transcoder calls in total, with a 32 percent
spread in its own denominator. Confirmed, and the omission is closed.

**The batching verdict's stated mechanism was contradicted by its own table** — P3.1 wins
and P1.2 straddles zero at an identical +1.00 forward crossings per element. Confirmed.
The replacement is not another mechanism story but a number: `gen/tax.sh` prices the
crossing up with a calibrated delay in front of every forward entry-point call, and on
P2.2 the delta goes **−20.9 ns/element at +0 ns, +2.7 at +2, +25.4 at +4.4, +47.5 at
+12.9, +147.1 at +24.6**. **The crossover is at a forward crossing of roughly 2 to 4 ns.**
Batching loses in C++ at 1.82 ns and wins on every managed runtime. That is the form the
specification needed.

**The RPC arm charged the two arms differently for transport.** Summing
`CLOCK_THREAD_CPUTIME_ID` over the harness's threads counts the grpc++ stub's transport,
which runs on the calling thread, and misses the core's, which runs on tokio workers. The
column is now `getrusage(RUSAGE_SELF)` minus the server handler's own CPU, measured the
same way for both, at 9 rounds with alternating order: **0.856 to 0.870**, against a
published 0.558 to 0.652. Confirmed and fixed; the old figure was wrong by about 0.2.

### 12. The finding that invalidated the way three conclusions were formed

**There is a 5 to 8 percent across-build drift and the slice had no error bar.** Proven on
code the build flag cannot reach: `AK_NO_GUARD` never reaches `core_native.cpp`, yet
`native` moved between two logs. `gen/drift.sh` now builds the same source twice with a
semantically neutral layout perturbation and publishes the identical-source rows:
**worst across-build RATIO drift 0.240.**

Three published conclusions were smaller than that — the floor costing nothing, the guard
not being measurable, and the decode-policy comparison — and all three were formed by
comparing two binaries. Two are now re-formed **inside one process**:

- The floor: `AK_CXX17` reaches **11 sites** in the whole emitted tree, all on the decode
  side, so two of the three cells of the a/b/c table were the same code measured twice.
  The two constructs it actually selects are now benchmarked side by side over the real
  data: **`insert_or_assign` is 1.074 of `m[k]=v`** and **`emplace_back()` is 0.967 of
  `push_back` + `back()`**. The C++11 floor costs nothing and the C++17 map construct is a
  7 percent regression — which is exactly why arm b measured *faster* than arm a on P2.2
  decode, as the review observed.
- The decode UTF-8 policy: priced on the string path alone, in one process, over all three
  content sets (which also makes `ak::values::recode` reachable, so the content sets were
  not deferred, they were unbuilt). Validating costs **+29.6 ns per string on ASCII, +140.6
  on Latin-1, +172.1 above U+00FF**, a factor of **4.5, 15 and 20**. The published "22 to
  28 percent of a decode" is **withdrawn**: it did not reconcile with its own logs, it was
  a difference of two ratios across the drift bar, and it was ASCII only.
- The guard: still a two-binary comparison, so the claim is now "nothing larger than 0.24
  was found", not "not measurable".

**And what the string-path table really says is that this slice's validator is bad** — 38
ns for a 36-byte ASCII string is about 1 ns per byte — not that validation is expensive.
`utf8_range` is now in the tree from the upb arm and unused.

### 13. What the generator guards were not doing

**Five of the nine backends dispatched on shape with a branch that emitted instead of
raising**, and `cpp_build`/`cpp_pbbuild` stopped testing cardinality after the
repeated-string arm, so a **repeated `bytes`** would have emitted a scalar store against a
`std::vector`. No instance exists in `shapes.json`, which is precisely why it needed a
test. `gen/refusal_test.py` now runs **16 must-fail cases** — ABI v1 section 8's
direct-argument refusal over this slice's own invocation, a repeated `bytes`, an unpacked
repeated enum, a repeated `double` and a `map<string, int32>`, each against every backend
separately — and all 16 are refused. The emitted text is unchanged, which `--check` shows.

Writing that test found a real defect of its own: **both gen directories contain a
`generate.py`, and `sys.path` had the rust one first**, so `import generate` silently got
the rust slice's generator and emitted its seven files instead of this slice's seventeen.
The slice's own `generate.py` worked only because it runs as `__main__`.

**A codegen rule applied in one path and not swept**: the map loop hardcoded `t.utf8` and
the validating reader for both halves, so a `map<string, bytes>` would have had UTF-8
validation applied to its value. Both are now derived from the pair message's declared
kinds, in one helper, used by the binding and by the control.

**`generate.py --check` was claimed green and run by nothing.** It, `refusal_test.py` and
`audit_tracked.sh` are now the first thing `run_all.sh` does, into `generator.log`.
`audit_tracked.sh` missed `*.proto` — the file the entire RPC arm is generated from — and
every upstream input; it now covers those and the rust generator, the ABI crate and the
schema.

**One enumeration of the 380 layout facts** now feeds both the host header and the core's
run-time export. Two separate enumerations could have agreed on the count and disagreed on
the order, which is exactly what defect C5 was, and `AK_LAYOUT_NAMES` — emitted twice and
included by nothing — is now its own generated target so a disagreement is NAMED.

### 14. Two findings confirmed, and their suspected causes refuted

**`groupfill` costs more than the total gap it is a component of** (22.6 ns/element on P1.3
against an `ffi` − `native` delta of about 18.3). Confirmed. The suspected cause — the
function-pointer parameter defeating inlining and forcing an `sret` return — is
**refuted**: a direct-call variant runs in the same rounds and measures within 0.3 percent
of the indirect one. `groupfill` is therefore reported as an **upper bound** on the
group's host-side cost, not as a component of the subtraction, and the discrepancy is
recorded as open defect C15. It was also being paired with a `pb` row taken minutes
earlier; it now runs in the same rounds as its denominator.

**R13's two crossing figures came from harnesses with different barriers.** Confirmed as a
difference; **refuted as the explanation.** With a register-only barrier the C++ figure is
1.822-1.824 ns, not 1.5. So **a C++ host pays about 1.82 ns where a Rust host pays 1.5 ns
through the same `.so` on the same machine**, and that is a result rather than an artifact.
The argument that quoted "at a crossing of 1.8 ns" — the *other* container's number — is
gone; the tax sweep replaces it with a crossover that does not depend on either.

**The static-against-shared causal claim is withdrawn.** P1.2 encode makes 9 forward
crossings in TOTAL, so 0.6 ns of saving cannot explain an 11 µs move, and the `native`
arm — which crosses nothing — moves between the two binaries too. The difference is the
build, not the boundary.

### 15. The counts, the corpus and the housekeeping

**R5 said count, do not infer, and one count was inferred.** The core's counter cannot see
whether `ak_str.tc` points into the host image or its own, so the host-transcoder arm's
crossings were argued. The host now reports them through a counting-build-only entry
point: P2.2 encode goes from 2,501 to **19,668** reverse crossings, **+34.3 per element**.

**Unknown-field coverage was thinner than claimed**: all five vectors landed at the root of
a message with no oneof. There are now three more spliced **inside a nested element**, and
one on `Probe` itself, where the case stays at the last known member and the payload is
dropped. Conformance goes from 361 to **443 checks**.

**The counting build decoded different bytes than the timing build** (`nat_enc` against
`pb_serialize`, which differ by 80 B on P2.5). Both now use the incumbent's bytes.

Smaller: the determinism ratio is computed from hoisted locals rather than from two fresh
measurements; "ns per element" says `ns/msg` on the five payloads that carry one element;
the denominator's own spread is printed beside every ratio, which is how P5.2 to P5.4 are
marked as noise-dominated; and **every per-round ratio is printed**, which is how the
systematic P1.2 decode outlier — one round in nine, about 34 percent high, in every log —
stopped being quoted as a legitimate bound of 0.790.

### 16. The upb arm: a ceiling, and the shape it shows

Not apt's `libupb-dev`, which is a July 2020 snapshot from before upb was merged into
protobuf. Not Bazel either: `protoc-gen-upb` is Bazel-only and a hand-written minitable
would be a hand-written codec. **upb v25.3 is built from the protobuf repository by
`gen/fetch_upb.sh` and the minitables come from upb's own reflection over `protoc`'s
descriptor set**, so the codec being timed is upb's and only the untimed setup differs
from a generated build.

Two dead ends worth recording so nobody repeats them: upb's `upb/cmake/CMakeLists.txt` at
v25.3 is a **stub** — four INTERFACE libraries referring to targets it never defines, so
`ninja` reports "no work to do" — and the checked-in bootstrap `descriptor.upb.c` under
`upb/cmake` does not match its own header's symbol spelling. The stage0 bootstrap copy
under `upb/reflection/stage0` does, and is what upb itself uses.

**upb decode is 0.22 to 0.58 of protobuf C++ on every element-bearing payload**, where the
core through the C ABI is 0.58 to 1.07. So the core's decode win is real and is roughly
half of what a C protobuf can do. That is a more useful sentence than any ratio in the
table it sits beside, and R2's floor rule is now satisfied from both ends: the memcpy floor
below and upb above.

**The upb encode column is not a ceiling and says so.** With a reused arena block it is
still 1.18 to 1.97 of protobuf C++ on the string-dense payloads: protobuf sizes its output
once and writes forward, upb grows a backward buffer geometrically.

**P2.5: upb writes 19,712 B, the same form protobuf C++ writes.** Two independent Google
runtimes, the same +80 B against the manifest, which `design/SHAPES.md` now records as one
of two valid encodings. The original report of this as a divergence was right and it has
cost three later slices nothing.

## W4, session 3 — two experiments about where upb's decode advantage comes from

### 17. `UPB_FASTTABLE`: an R7 omission, and a null result that is the finding

The published upb column was built with **`UPB_FASTTABLE=0`** and did not say so.
`upb/port/def.inc:227-239` defaults it to 0 and `gen/fetch_upb.sh` defined neither
`UPB_ENABLE_FASTTABLE` nor `UPB_TRY_ENABLE_FASTTABLE`. Confirmed; `upb.log`'s
configuration line now names it.

Turning it on changes nothing it could change, and the reason is in the source rather than
in the clock. `_upb_Decoder_TryFastDispatch` (`upb/wire/decode.c:766`) fires only when
`layout->table_mask != (unsigned char)-1`, and `upb/mini_descriptor/decode.c:698,712` sets
`table_mask = -1` on every minitable it builds. The fasttable entries are emitted by
`protoc-gen-upb` through `UPB_FASTTABLE_INIT` and by nothing else. **A reflection-built
minitable can never take the fast path.**

Both halves are proved from artifacts rather than asserted, which is the R5 discipline
applied to a `#if`: `upbbench` prints the runtime `table_mask` (−1) and `gen/fetch_upb.sh`
counts the fast-parse functions in the archive (**0** without the define, **42** with it).
So the code is genuinely compiled in and genuinely unreachable.

Three builds of identical sources separate the compiler from the define, because the first
attempt at this A/B confounded them (gcc has no `__attribute__((musttail))`, so the
fasttable build needs clang):

| build | P1.2 | P2.2 | P2.3 | P3.1 | P6.1 |
|---|---|---|---|---|---|
| gcc, FT=0 | 0.317 | 0.349 | 0.306 | 0.353 | 0.552 |
| clang, FT=0 | 0.245 | 0.283 | 0.245 | 0.277 | 0.522 |
| clang, FT=1 | 0.291 | 0.320 | 0.262 | 0.309 | 0.538 |

**clang is worth 6 to 23 percent** of upb's decode — so the published column understated
upb — and **`FT=1` is 3 to 19 percent slower** than `FT=0` on the same compiler, which is
what an unreachable fast path costs: an extra branch at the top of the decode loop and a
`_upb_Decoder_TryFastDispatch` that returns false every time.

**So none of upb's measured decode advantage is `UPB_MUSTTAIL` tail-call dispatch.** That
was the half the branch could not rule out, and it is the half a Rust core is structurally
locked out of. What is left — the epsilon-copy input stream, arena allocation, minitable
dispatch, and not copying strings — is all reachable from Rust. That turned Experiment 2
from interesting into decisive.

### 18. The borrowed-string facade: most of the distance to upb is `std::string`

ABI v1 already supports it and that is the first thing to say: `ak_span` is an **offset**
into the buffer the host handed in (section 4), section 7 says the span points into that
buffer, and 7.4 tells the host to resolve it against the base pointer it already holds.
So a facade whose string fields are `ak::StringView` over the input needs **no ABI
change**, and it is exactly upb's aliasing contract (`upb/wire/decode.h:29`).

Built by parameterising the two existing emitters on (namespace, string type) and
emitting a second pair of files into `shapes_borrow`. The decode and encode helpers are
**overloaded** on the destination type, so the generated call text is byte-for-byte the
same for both facades and the only difference is the type spellings — which is why
`--check` stays green on the shipping facade while a second one appears beside it. UTF-8 is
still validated on the borrowed path, so the arm isolates **the copy** and nothing else.
Singular strings, repeated strings, `bytes` and map keys and values are all borrowed,
because a singular-only arm would understate a schema with 174 string fields in 413.

Byte identity gates it: decode into the borrowed facade, re-encode, compare with the
manifest, every payload. That check found its own bug first — it compared against the
INPUT bytes rather than the manifest, and on P2.5 the input is the incumbent's 19,712 B
form while any re-encode produces the canonical 19,632 B. The owned facade does exactly
the same thing; the reference was wrong, not the arm.

| payload | `ffi` | **`ffi-borrow`** | upb (gcc) | delta, % of a protobuf decode |
|---|---|---|---|---|
| P1.2 | 0.562-0.790 | **0.234-0.241** | 0.317 | −33.8 |
| P2.2 | 0.661-0.696 | **0.387-0.407** | 0.349 | −28.4 |
| P2.3 | 0.855-0.921 | **0.383-0.396** | 0.306 | −49.5 |
| P3.1 | 0.601-0.615 | **0.326-0.330** | 0.353 | −27.9 |
| P6.1 | 0.639-0.654 | 0.599-0.646 | 0.553 | −4.3 |

**−24.5 to −49.5 percent of a protobuf decode on every element-bearing payload**, and
−85.9 to −99.5 percent on M5's bulk `bytes`, where the whole message is one field and
borrowing removes the entire copy.

**Borrowing takes the core from about twice upb to level with or below it.** On P1.2 the
borrowed core (0.234-0.241) is below even the clang upb build (0.245). So the sentence
this slice published — "the core's decode is roughly half of what a C protobuf can do" —
is **largely a statement about `std::string`, not about the codec**.

**P6.1 is the control that says the arm measures what it claims.** `MetricsBatch` is one
string and five packed scalar arrays: almost no copy to remove, and it barely moves.

**What it does not isolate**, and the residual gap on P2.2 and P2.3 is exactly this:
vectors, maps and message children are still constructed. This separates the string copy
specifically, not host-side container construction in general — which is the finding the
branch already had, and this narrows rather than replaces it.

**It is a measurement arm and not a proposal.** The views are valid only while the input
buffer lives; the shipping facade is untouched and its emitted text is unchanged.

---

## W10 — the core was forked, and the fork was not in the codec

The user's finding, not mine, and the mechanism is the whole of it. The emitted
`codec.rs` was byte-identical in the rust, cpp and java slices — same md5, 4,190 lines —
so R1 was holding and the *generator* was genuinely shared. What had forked was the
**hand-written runtime beside it**: my `core/src/lib.rs` (527 lines, `ak_enc_count_reverse`
added for review finding 19), java's (702 lines, `ak_tc_latin1` and `ak_tc_utf16`), and the
rust slice's original (492). Neither addition was wrong and neither broke a measurement,
which is why it survived three slices. **Each fork happened because a slice needed to ADD
something and there was nowhere to contribute it.** That is the five-implementations
problem this branch exists to argue about, reproduced inside the branch.

R0 is now the rule. This work unit moved the core to `ffi/poc/codec`, folded the three
copies into one, and re-pointed every slice.

### What I checked before moving anything

`diff` on the three `lib.rs`. rust→cpp is four things: a doc note, the `rpc` module behind
a feature, the `generated::layout` module, and `ak_enc_count_reverse`. rust→java is the
same minus `rpc` plus `tc_utf16`/`tc_latin1` and their two `#[no_mangle]` accessors.
**Every difference is additive**, which is what made the fold a splice rather than a merge.
`cpp/core/src/rpc.rs` and `rust/crates/ak-core/src/rpc.rs` are byte-identical (`diff` rc=0),
and so are the two `generated/layout.rs` — java's generator and mine emit the same 398
lines because the layout is a function of the schema, not of the host.

The coordinator asked me to check that java's two transcoders are the ones java actually
calls. They are: `java/native/generated/shim.c:15,16` declares them and `:501,504` returns
their addresses to the JVM, and after the move `nm -D --undefined-only` on the rebuilt
`libakjni.so` shows `U ak_tc_latin1` and `U ak_tc_utf16` resolved by the shared core.

### Two things I moved that the work item did not name

**`crates/rpc`.** `ak-core`'s `rpc` module is ABI v1 section 9 and does not compile without
it; two slices already path-depended on it. Leaving it in the rust slice would have made
the shared core depend on one slice's tree, which is the coupling R0 exists to remove.

**`cpp_layout.py`**, which was mine. It emits Rust for the core crate — nothing C++ — and
java already imported it across slice boundaries, which was the same defect one level down.
It keeps its misleading name on purpose: renaming it would be an edit to another slice's
imports rather than a path change. Moving it exposed one genuine coupling: it took
`abi_order_topo` from `cpp/gen/cppnames.py`, so that function moved into the shared `ir.py`
and `cppnames` re-exports it. That ordering is ABI-level, not C++-level — Rust does not care
about declaration order and C does — and in the *layout table* a wrong order is silent
rather than a compile error, which is why it must have one definition.

### The hazard the move created, and how it showed

`rust/crates/harness/build.rs` searched `<profile>` before `<profile>/deps`. Cargo uplifts
only a workspace **member's** cdylib to `<profile>`, and after the move `ak-core` is not a
member, so the fresh build lands in `deps` — while the stale pre-move `libak_core.so` was
still sitting in `<profile>`, first on the search path. The rust arm would have linked and
loaded a core from before the move and measured it unchanged, which is the exact shape of
"the first hypothesis is that it is not running". Order flipped, stale copy deleted,
`ldd` now shows `deps/libak_core.so`. Logged as C17.

### What it measured: nothing, which is the result

Not against the committed logs — those were taken on a loaded container on another day, and
comparing across them would have answered the wrong question. The control is the pre-move
commit (`fa5f831e`) built in a git worktree and run **minutes apart on this machine**.

| | cpp enc `ffi` | cpp dec `ffi` | rust enc `core-ffi` | rust dec `core-ffi` |
|---|---|---|---|---|
| pre-move, today | 0.634 | 0.677 | 0.729 | 1.068 |
| post-move, today | 0.621 | 0.676 | 0.726 | 1.091 |
| the committed log | 0.724 | — | 0.836 | 1.010 |

Worst move across all fifteen rows: **0.023**, against R4's across-build drift bar of
**0.240**. The arms the core cannot touch (`pb-arena`) move by the same amount as the ones
it can, which is what says this is run-to-run. And the gap to the committed figures (0.090
on cpp enc `ffi`) is present in the **pre-move** control too — so it is the day, not the
move. That contrast is the reason the worktree control was worth building.

### The rule now has a failing test

`codec/gen/one_core.sh` checks seven things: one definition of the C entry points (four
sentinels, all hand-written in `lib.rs`, because a partial fork is the likely shape), one
copy of each generated core file, one package per core crate, every `path =` dependency
resolving into `codec/crates/`, the emitters shared, no build naming a per-slice core
library, and the shared core not stale. `--selftest` copies **what git tracks** to a
scratch dir, plants five of those violations in turn, and requires each to fail. All five
fire. It runs from `gen/run_all.sh`.

Writing it found three defects in itself in the first run — two sentinels that name symbols
which do not exist, and one that matched the script's own text — which is the argument for
the positive control in miniature: a check that has only ever passed has not been seen
working.

### What refused to be verified here

Java's Java-level gates: only JDK 21 is installed and its build needs JDK 17 and JDK 8.
C#: dotnet is not installed. Both builds were shown to *resolve* the shared core — java's
core and JNI shim link against it, csharp's layout probe compiles — and no further.

---

## W11 — the concurrency suite, and a validator that was the finding rather than the figure

Two items off this slice's own next-step list, promoted by the coordinator because both
were branch-level holes rather than C++ polish.

### ABI v1 obligation 12.5

No slice had one. The suite is four payload shapes over two message types, two axes
(threads joined one at a time, then started together), three arms per encode plus a
decode-and-re-encode leg, every one memcmp'd against a reference. It runs at C++17, at the
C++11 floor and on both linkages, and once with sixteen threads on four cores so the
scheduler preempts *inside* an encode rather than between encodes.

The shipped design passes every axis. That was never the interesting part.

**The interesting part is that the first version of it passed on all three planted builds**
and I nearly reported it as a success. The plants are the two designs ABI v1 section 6
refused — pad the length prefix to the learned width, and put the learned-width table in a
process-global — and they are the right plants, because section 6 refused both with
arguments rather than measurements and a refusal with no evidence is a sentence.

Why it passed: **widths only ever grow within a context**, so only "A then B, where A left
a site wider than B needs" can show anything, and my four shapes had no such pair. P1.1
(858 B) and P1.2 (218 KB) are the same message type and learn the *same* table — the
prefixes that vary are per element, the elements are the same size in both, only the count
differs, and a top-level message carries no prefix at all. Two shapes of *different*
message types touch disjoint sites and can never over-reserve for each other. The pair that
works is P1.1 and P1.3: one message type, one of them the absent-path payload.

So T0 exists: it asks the encoder which ordered pairs have a history surface, and prints
the answer **whether or not it is empty**, because an empty one means T1, T5 and T6 are
testing nothing. Building T0 took three goes of its own —

* a probe with `static F o = MK()` inside a function templated only on `F`, so all three
  P1.x rows were really P1.1 (the identical byte counts gave it away);
* an over-reserve count that included sites the target never writes, which made T5 pick
  `after P2.2, P1.1` — two M2 sites that M1 never touches — and test a direction in which
  nothing can happen. Fixed by *priming* every width to a value nothing needs and seeing
  which come back reduced, which identifies the written-site set exactly and costs the hot
  path nothing, rather than adding a `touched` store in `begin()` that every timed arm
  would pay for;
* and then T5 was coupled to T0 at all, which broke under `AK_CONC_PAD` because that plant
  removes the very `resize_prefix` call the probe reads. T5 now runs *every* ordered pair.

The reference had the same shape of bug: it was built by re-encoding with `ak::Enc`, which
is where the plants live, so on the pad+global build the oracle itself was wrong and every
arm was being compared against it. The sha anchor caught it — which is what an anchor is
for — but the fix is an oracle no plant can reach, so it is protobuf's encoder now.

**What it settles.** 12.5's last sentence is a claim, so it is measured: one shape 0 wrong
of 24, two shapes 44 of 48. It holds. And section 6's two refusals turn out to be
**independent**:

| build | bytes wrong | threads disagree | contended scaling |
|---|---|---|---|
| shipped | 0 | 0 | 3.63-3.96x |
| pad | 46/96 | 16/16 | — |
| global | **0** | 0 | 2.80-2.87x |
| both | 46/96 | **0** | — |

A global table is a race and a throughput defect and **not** a byte defect: an unpadded
prefix is rewritten to the width the body needs whatever the guess was. Padding is the byte
defect, and it is the one that makes two threads emit two different legal encodings of the
same message. Both together is the case a naive suite would miss — the threads *agree*,
because they share the pollution, and are both wrong.

Section 6's throughput claim reproduced at **1.83-2.05x** under contention, inside the java
slice's 1.32-2.23x from another language and machine — but only under contention. When every
thread encodes the same shape the table is written only on a miss, so it is read-mostly and
shared clean, and the cost is 1.13-1.23x with *no* scaling loss. Measuring only that leg
would have reported "a global table costs nothing". Three runs per build, because one run
of a scaling leg is not a range and the single runs inside the sweep disagreed by more than
the effect.

### The validator, which turned out to be about a `bytes` field as much as about SIMD

This slice published "4.5x to 20x" for the decode-side UTF-8 check and called it the
largest single effect it measures. Two things were wrong with it.

**C20, and it is the one I did not expect.** The string set was six fields of P1.2's
elements and should have been five: `ResultRaw.opaque_id` is a **`bytes`** field. proto3
puts no UTF-8 requirement on it, the codec reaches it through `ak_tc_bytes` and
`decode_str_raw`, and no validator ever sees it. In the ASCII set its 1,000 values are
arbitrary bytes, so the check arm *rejected* them on the first bad byte and did less work
than a validation — the published ASCII row **understated** the cost. It was found by the
new differential test asserting that every string it validates is valid, which the timing
table itself had never done. One definition in `harness.h` now, shared by both.

**And the validator.** Correctness first, because byte identity cannot see a validator
defect at all: a manifest is made of things that *encode*, so it carries no malformed input
and a validator that accepts a surrogate would pass every gate this slice has. So:
exhaustively every 1-, 2- and 3-byte string (16.8 M), a structured 4-byte sweep on every
lead byte and the range boundaries (864 K), 32 named malformed classes, and the payloads'
own strings — 17.78 M differential checks against an oracle written from RFC 3629 one range
per line, which is not one of the implementations under test.

A textbook DFA was the first attempt and it is **slower than the scalar version on wide
content** (0.78x): its state is a serial dependency, one dependent load per byte, and the
branches it removes were being predicted correctly anyway on uniform content. The one that
wins keeps the scalar shape — consume a whole sequence per iteration — and removes what is
actually wasted, the code-point arithmetic. Validation needs no value, only ranges, and
every range constraint in UTF-8 is a function of the lead byte alone: overlong forms,
surrogates and everything above U+10FFFF are each exactly a constraint on the *second* byte
given the first. One packed u32 per lead byte, one load per code point. The DFA is kept and
stays in the differential test, as the evidence for that paragraph.

The ceiling cost nothing: **protobuf C++'s own `IsStructurallyValidUTF8`**, already linked
into every arm. That is a better ceiling than a fetched SIMD library for R14's purposes —
it is literally the validator the incumbent runs on every `string` field it parses — and
nobody has to believe a claim about how it was configured. (STATE.md said `utf8_range` was
in this tree. It was not; it was in a scratch directory from the upb arm that does not
survive. Corrected.)

Result: the ASCII check is **2.27x a raw copy, not 4.4x**, and the core's validator is
cheaper than the incumbent's own on all three content sets — 2.27 against 2.56, 15.9
against 19.6, 19.6 against 34.4. So decision 3's check is not a cost the core imposes on a
host that did not have one. It is cheaper than the check the host already pays.

What is *not* claimed: the effect on a whole-payload decode ratio. The arithmetic says
about 18% of an ffi decode, which is 0.10 on a ratio of 0.55 and inside R4's 0.240
across-build bar, and the validator is a build-time choice so one process cannot hold both.
The string path is where it is measured and where the claim stops — the same reasoning that
withdrew the earlier "22 to 28 percent of a decode".

`ffi-valtc` is untouched and I said otherwise in the first draft of the log: it reaches
`ak_tc_utf8()`, the core's Rust transcoder. Whether the core's encode-side validator has
the same 2x available is open and R0 makes it the aggregating session's.

### C18, and the same gawk trap twice

Half two of the boundary check asked whether a control function was *larger* than the
largest timing closure in the image. Size is a proxy for fusion rather than a test of it,
and the positive control was `-flto` — which fires only if the optimiser happens to fuse
something. After W10 moved the core the largest closure went from 1433 B to 911 B, the
1155 B control landed on the other side of the line, and the control went quiet without
anyone deciding it should.

Replaced with the question itself. The control is reached through a function pointer handed
to the case runner, so there is no direct call site to count — I tried that first and my own
fixture disproved it, reporting zero calls to `ak_probe_called` because the call goes
through a volatile pointer. What fusion would actually destroy is the out-of-line body and
the call instruction in the closure, so those are the two properties now checked.

The control is `src/fusion_probe.cpp`: one function that must be called (`noinline`, address
taken through a volatile pointer so it cannot be devirtualised) and one that must be fused
(`always_inline`, static). Both guaranteed by construction. If the counter ever reports a
call in the fused loop, or none in the called loop, the counter is broken and every other
answer it gives is worthless.

Building it found the counter broken immediately: `/\<call\>/` matched nothing, because
**mawk is what is installed and `\<` `\>` are gawk-only word boundaries**. Half two duly
reported that 10 of 10 timing closures were fused. That is the second gawk-only construct
in this one file — `strtonum` was the first, months of sessions ago — and both failed
*silently* rather than erroring. The fixture is what caught it; without a control that has
a known answer, "10 of 10 fused" would have looked like a discovery.

23 checks, 0 failed.

---

## W12 — the content sets on whole payloads, and C16 finally has a mechanism

### The content sets

SHAPES.md: "a slice that reports one string-path number without saying which content set it
came from has reported half a number." This slice had priced the *string path* over all
three sets and every *whole-payload* row over ASCII only.

The set is applied in the **value rules**, not in the generator. `guid`, `word` and
`sentence` run their result through `recode`; `blob` and `bulk` do not, because a content
set says what is in the *strings* and a `bytes` field has no encoding to be in — which is
C20 restated as code. That also means both construction routes get it for free: the facade
builder and the protobuf builder are generated separately over two object graphs and both
reach their values through those three functions, which is exactly what makes them two
independent routes to one value.

Correctness first and per set, because no manifest oracle covers latin1 or wide: 80 checks,
0 failures, every arm byte-identical to the **incumbent**, which is itself checked against
`manifest.json` on ASCII so the oracle is anchored where anchoring is possible.

Wire sizes came out at latin1 1.687–1.748× and wide 2.373–2.495×, against the rust slice's
published 1.70–1.75 and 2.39–2.50. Two generators, two languages, three digits of
agreement — a cheap R1 check I had not thought to look for.

**The finding is that the answer differs by direction.** P1.2 encode `ffi`/`pb` goes 0.988
→ 0.167 → 0.114; decode goes 0.656 → 0.671 → 0.546. So the published C++ **encode** column
is an ASCII column and nothing else, and the **decode** column survives being read without
its content set. SHAPES.md's sentence is right, and it is much more right about one half of
the codec than the other.

**And the encode number needed a guard immediately.** protobuf C++ validates UTF-8 when it
*serialises* — I checked the generated code rather than inferring it: 37 unconditional
`VerifyUtf8String(..., SERIALIZE)` call sites in `shapes.pb.cc`. ABI v1 says the core does
not. So most of that column is a check the core skips, and publishing `ffi`/`pb` alone on
these sets would have been publishing a policy difference as codec speed — the same shape
of error as C7, pointing the other way. `ffi-valtc` is in the table for every set now, and
like-for-like the core is at parity on ASCII and about twice as fast on latin1 and wide,
which is the same ratio `utf8.log` measures between the two validators directly.

The first version of the timing harness here reused one object per arm across decodes. A
reused protobuf message keeps its allocations and parses into them; a reused facade
*accumulates* into its vectors. That made the incumbent look 3.3× faster than it is and the
core's output wrong at the same time — C7's defect with the sign flipped on one arm and
pointing both ways at once. `bench.cpp` constructs a fresh object per decode and I should
have copied it rather than rewritten it.

### C16

It had survived several work units as "a systematic outlier round, about 34 percent high,
in every log". It is now characterised, and the interesting part is how much of the
investigation was refuting my own first answers.

**Refuted: the machine.** The obvious suspect was this container's own stale background
pollers, which I had stopped at the start of W10. Six runs on an idle box across two
linkages: rounds 1 and 2 high, 3–9 flat, every time, to three digits. Deterministic.

**Refuted: the arm rotation.** `ffi` runs fourth in round 0 and third in round 1, so its
position does not coincide with the affected rounds.

**Refuted as the cause, but it is why the outlier appeared when it did: the validator.**
`bench_a17_scalarv` is the same source with `AK_UTF8=0`. With the old scalar validator the
row is *flat* at 0.62; with the table validator it is 0.54 with the first two rounds at
0.67. A decode dominated by a slow validator hides a fixed per-iteration cost, and making
the validator twice as fast turned that cost into a visible fraction. Which means W11 did
not create C16 — it uncovered it.

**Demonstrated, by removal: glibc's mmap path.** The default mmap threshold is 128 KB and
it *adapts*: when an mmap'd block is freed the threshold rises to its size, so later
allocations of that size are recycled from the heap instead of faulted in fresh. Pinning
`MALLOC_MMAP_THRESHOLD_` and `MALLOC_TRIM_THRESHOLD_` high removes the outlier **and keeps
the steady state** — 0.544,0.547,0.548,… flat from round 1. Forcing the threshold to 4096
so everything always goes through mmap reproduces the outlier's *value* in every round.
And the mechanism counted rather than inferred: an order of magnitude **more minor page
faults** with the default allocator — 10.8× in the committed run, 10.8–13.1× across runs.

Pinning the threshold alone was not enough and that is worth recording: it removed the
outlier but left the steady state at 0.62, because glibc then trims the heap back and the
churn costs what mmap did. Both thresholds together is what recovers 0.54.

**What it does not explain, named rather than guessed — and one claim I had to withdraw
before it reached a log.** My first pass ran each predecessor once and concluded "P1.1
removes the outlier; P1.3, P2.1 and P4.1 do not", with a paragraph about why the smallest
predecessor warming it was mysterious. The very next run of the same command put P1.1's
outlier at round 8 instead of rounds 1–2. So the block now runs three times per
predecessor, and what it actually shows is that a predecessor **moves** the outlier rather
than removing it. That is consistent with an allocator-state effect whose timing depends on
what was allocated before, and inconsistent with the deterministic per-message warm-up a
single run had suggested. The same correction T7 needed in the concurrency suite, one work
unit later, for the same reason: one run of a noisy thing is not a range.

The residual is therefore narrower than I first wrote it: the cause is settled, the
*timing* is not. A malloc hook logging size and mmap-or-not per call would settle it, and
it is a question about glibc rather than about the ABI, so it stops here.

**No figure is withdrawn.** The slice reports min-of-rounds and prints every round, so no
published number came from an outlier round — which is what printing them was for. The
caveat it adds is real: a C++ consumer decoding large messages pays an allocator cost that
default glibc tuning only amortises after the first few messages, and two environment
variables remove it.

---

## C24: wire type 3, and the oracle that was built to miss it

**The defect, first, because it is more interesting than the fix.** `ak::Dec::skip`
switched on the wire type and had no `case 3:`, so wire type 3 -- the deprecated GROUP
form -- fell into the `default:` arm and returned `ERR_MALFORMED`. That is a **legal
message refused**. protobuf C++ accepts it, upb accepts it, protobuf-java accepts it; a
parser has to skip what it does not know whatever shape the unknown thing is, and
`START_GROUP`/`END_GROUP` are still wire types even though proto3 cannot declare one.

**443 conformance checks passed over it, five times, at three standard levels, in both
linkages, for as long as this slice has existed.** That is the part worth keeping. The
oracle is byte identity against `ffi/schema/generated/manifest.json`; the manifest is
generated from the same proto3 description the codec is generated from; proto3 cannot
express a group; therefore no payload in the manifest carries wire type 3 and **no amount
of byte identity against it can ever execute the group arm of the skip.** A generated
corpus tests the shapes the generator can write. The unknown-field skip exists precisely
for the shapes it cannot.

I recorded the defect before fixing it, with a throwaway probe against the committed
header (the output is the "before" block of `logs/cpp/groupskip.log`'s story; the probe
itself does not survive the signature change, which is why the committed evidence is the
plants instead):

```
  a group, well formed            MUST ACCEPT  reject ERR_MALFORMED (pos 1 of 6)
  nested groups                   MUST ACCEPT  reject ERR_MALFORMED (pos 1 of 6)
  X-group-unterminated            MUST REJECT  reject ERR_MALFORMED (pos 1 of 3)
  X-group-mismatched-end          MUST REJECT  reject ERR_MALFORMED (pos 1 of 4)
  an end tag with nothing open    MUST REJECT  reject ERR_MALFORMED (pos 1 of 1)
```

Two of five wrong -- and the three "right" answers are right for the wrong reason: they
were refused because wire type 3 was unknown, not because the group was unterminated or
mismatched. `pos 1` in every row says so.

**The fix follows the shared core rather than inventing a second one** (`skip(tag, wire)`,
a field-number-matching `skip_group`, recursion bounded at protobuf's own 100 with
`ERR_DEPTH`). The one thing worth restating is why the tag is a parameter: a group carries
no length, so its end is found by reading fields until an `END_GROUP` **whose field number
matches the one that opened it**. A nesting counter is the fix everyone writes first, and
it accepts `X-group-mismatched-end` and then mis-nests every group after that point.

**What the plants were for.** `AK_GROUP_PLANT=1` is the depth counter, and it fails T4 and
T5 -- exactly the two mismatched-end cases. `AK_GROUP_PLANT=2` is the `case 5:` arm dropped
while `case 3:` was added, which is not a hypothetical: it is what the shared core's own
first test run caught. It fails T8 and T10, every buffer carrying a `fixed32`. Both plants
live in `include/ak/rt.h` beside `AK_CONC_PAD` and `AK_CONC_GLOBAL`, and
`gen/groupskip.sh` inverts their exit code, because this slice has now lost two fixtures
(C18, C23) by letting a check that could no longer fail keep reporting success.

**Swept, not patched.** 13 emission sites in `gen/cpp_core.py`, including the
`sub.skip(et, ew)` inside the map-entry loop that a `default:`-only sweep would have
missed, plus two in `src/conformance.cpp`. `src/generated/core_native.cpp` was regenerated
and `--check` is green on all 23 files including the shared core's two.

**And it moved nothing on the clock**, which had to be measured rather than asserted
because the signature change recompiled every decode function in the control TU: 225 ratio
rows against the published bench log, worst move 0.164 against R4's 0.240 across-build
drift bar (`logs/cpp/c24-timing.log`). One visible side effect: gcc now inlines
`dec_list_results_response` into its caller inside the control TU, so `boundary.log`
reports 21 checks instead of 23. The checker treats an absent symbol as fine and says why
-- the question is whether the benchmark *loop* carries the traversal, and "all 10 timing
closures still call out" is unchanged.

## W8: the corpus, and what it caught that the manifest could not

Then the thing that would have found it. `ffi/corpus` is written against a SUPERSET schema
by a different tool, so it is the one oracle that is not a function of this slice's own
description. 128 of its 336 rows root at a message this slice's codec covers; the scope is
read out of `AK_ROOTS` in the generated `cases.h` so it cannot be claimed larger than the
codec is.

Three things I would do the same way again:

**1. A third arm that is the incumbent.** protobuf C++ projects every row too, through its
own reflection `ListFields` -- which IS CONTRACT.md section 3's presence rule, so there is
no second reading of the contract to be wrong about. It agrees with the generated
projector on 123 of 124 rows, which is what makes the 124th worth arguing about instead of
worth assuming.

**2. A walker for the rows the codec cannot root.** The corpus's 62 `WireZoo` vectors root
at a message this slice has no type for, so C1-C3 cannot touch them -- but their wire forms
are exactly what C24 fixed. Running them through `ak::Dec::skip` with no schema at all is
not a root decode and is labelled as a walker everywhere it appears; it answers accept or
reject and no more. **62 of 62 agree with the corpus**, and `X-group-unterminated` now
refuses with `ERR_TRUNCATED` and `X-group-mismatched-end` with `ERR_MALFORMED` -- the right
errors, not the accidental one. Without the walker this slice would have reported the
group fix as tested by three accept vectors and no reject vectors at all.

**3. Asking a third implementation instead of arguing.** `U-map-entry` -- a vector with an
unknown field inside every map entry -- came back as a C2 mismatch, and the obvious reading
was "this slice mis-parses a map entry". It does not. protobuf C++ 3.21.12 reads the four
entries into the map (`protoc --decode` prints them, unknown field `3: 7` and all), and so
does protobuf-python's pure-Python backend. **upb drops the whole entry and promotes it to
an unknown field of the parent, and the corpus's projection is upb's.** Two Google runtimes
disagree with each other on the same bytes. That is C25, it is a finding for the corpus
agent and not for me, and the only reason I have it as a fact rather than a suspicion is
that the driver re-reads the vector with two protobuf backends the moment a projection
differs. **A consumer that trusts the corpus when it disagrees is a consumer that has not
added an opinion.**

**One defect of my own, named because it looked exactly like a codec defect.** The first
run reported the `ffi` arm writing an unaccepted form on 114 of 126 rows, with the same
wrong hash repeating across unrelated vectors. It was a use-after-free in the harness:
`ak_enc_take` hands back a pointer into the context's buffer and I freed the context before
copying. The repeating hash across unrelated rows is the tell, and it is the reason I did
not start bisecting the encoder.

**What I did not do**: C5 (produce) on the 41 `E-*`/`S-*` rows, which needs the corpus's
own value rules implemented a second time in C++, and the `chunking` class, which needs a
`ChunkedResponse` codec. Both are listed by id in `STATE.md` rather than left as a number
that is silently smaller than it looks.

---

## W12: the RPC arm re-taken as a grid, in all three deliveries, and the machine changed underneath it

Two things were known to be wrong with the arm this slice had published, and a third turned
up in the first ten minutes.

**The container is a different machine.** `/proc/cpuinfo` says Intel Xeon @ **2.10 GHz**
where every other figure in `STATE.md` came from a **2.80 GHz** one. The unchanged bench
measures a forward crossing of **0.63 ns** here against the published **1.822 ns** -- a
factor of about three, on a nominally slower clock. That is not a detail to put in a
footnote: it means `rpc.log` and `rpcflow.log` share no absolute with any other log in this
slice, and it is exactly the situation R13 exists for. Which made the next thing worse.

**R13's calibration cannot be taken any more, and not because of anything here.** The rule
is that each slice runs the RUST slice's crossing benchmark on its own machine, so the
cross-language table stays reconstructible. `ffi/poc/rust` does not build on this branch:
C24 changed the shared runtime's `skip(wire)` to `skip(tag, wire)` and swept this slice's 13
emission sites, and the rust slice's generated tree was never regenerated. 20 E0061 errors.
`poc/rust/**` is not mine to write, so it is C27 and the log says plainly that the yardstick
it quotes is this slice's own crossing and not the common one. **Every future slice on every
future machine hits this until somebody regenerates that tree.**

### The grid, and what a pair could not have told anyone

Four cells -- A (protobuf/grpc++), B (protobuf/core), C (core/core), D (core/grpc++) --
times three deliveries for the two core-transport cells, times two transports, times pinned
and unpinned, times three in-flight levels. Cell D is grpc++'s own generic `ByteBuffer`
path, so the transport code is literally cell A's and only the marshaller changes.

**The first version of the delta table was useless and I threw it away.** I reported
best-of-nine differences with a bar built from the arms' full ranges, and since the arms'
own round-to-round spread is 5 to 25 percent, everything was "INSIDE THE BAR" including
differences that are plainly real. The fix is the discipline this slice already uses for
decision 1: form every difference **within a round** and report lo, hi and whether they
share a sign. With that, C - B separates in 12 of 12 and B - A in 0 of 12, out of the same
data that a minute earlier said nothing at all.

**What it says.** The transport is a wash on P2.2 (0 of 12) and the codec is the whole of
the difference (12 of 12, -6.8 % to -33.4 %). So the 0.856-0.870 "RPC" ratio this slice
published was a codec ratio. And `(C-B) - (D-A)` straddles zero in 12 of 12: **the codec is
worth the same under either transport, so the two halves of the proposal are additive** --
which was the question SHAPES.md said nobody had asked.

### The deliveries do not separate, and then the Ping block said why

Section 9 asserts the callback "suits C++ and C#" and C++ is where a reverse call is
cheapest, so this is where the claim gets confirmed or refused. It is refused, and the
refusal took two goes.

First attempt: callback and queue beat blocking at 8 and 16 in flight, blocking does not,
done. That is wrong, because the callback row keeps N calls outstanding from ONE host thread
and blocking needs N threads, so the comparison moves two things. **The control is `cb x N`:
the callback delivery in blocking's thread shape.** It does not separate from blocking (1 of
12) and where it separates from the one-thread callback it is SLOWER. So the win is the
thread count and not the delivery.

Second problem: a 540 KB response is 4 ms of CPU and a reverse crossing is 0.30 ns. Nothing
about a delivery can show through that, so "they do not separate" was not yet a result about
the deliveries. Hence the **Ping block**: an empty request and an empty response, no codec on
either side, which is what `Ping` was added to the service proto for. Queue against callback
still separates in 0 of 3. The arithmetic is why: the deliveries differ by under a
nanosecond of boundary against a call of 130 µs, which is two parts in a hundred thousand.
**No payload this slice can build could see the difference, and that is the finding.**

**And the Ping block found something the grid could not.** With the codec gone, one empty
call in flight costs the core's transport **41 to 95 percent more CPU than grpc++'s** -- 53
to 107 µs per call. The grid's "the transport is a wash" is true of a 540 KB call and false
of a small one. Two points do not give a crossover and I am not pretending otherwise, but
the qualification matters for README 13's outcome 2, which is precisely the recommendation
to adopt the transport half.

### Counting instead of quoting, and section 9's table is wrong

R5 says count crossings, do not infer them, and section 9's per-delivery table (2/0, 2/1,
3/0) was arithmetic nobody had run. I added `ak_rpc_counters` to the shared core under
`#[cfg(feature = "count")]`, built it `--features rpc,count` into a separate binary, and
counted. Blocking is 2/0 as claimed. **The callback is 3/1 and the queue 4/0: the table does
not count `ak_call_destroy`**, which both non-blocking deliveries require or the host leaks a
handle per RPC. Identical on `Fetch` (~4,500 fields) and `Ping` (none), which is the
per-field half of the claim, counted.

`ak_rpc_counting()` exists because of how this could have gone wrong: link the non-counting
core by mistake, read zeroes, publish "the boundary is free". The counting binary refuses to
run if it returns 0.

### The flow-control probe, which I expected to be a formality

`design/SHAPES.md` asks each arm to state its windows and whether auto-tuning is on, and to
establish the two traps "from its own runtime's source rather than inheriting". I built the
probe expecting to confirm the table. Four things came out and none of them was a
confirmation.

1. **grpc++ exposes no connection-window argument at all.** The two windows are separate
   quantities -- the stall trace prints `t_win` and `s_win` side by side -- and only the
   stream one is reachable. Proof by exhaustion: with the stream window at 4 MiB the stalls
   sit at `t_win=0` with `s_win=3,653,887`, and raising the stream window SIXTEENFOLD to
   64 MiB leaves them there.
2. **An explicit window does not turn BDP probing off**, and with the probe on the window
   you configured is not even what gets announced. That is grpc-java's behaviour inverted.
3. **Turning BDP off SHRINKS the window to 64 KiB**, because grpc-core's ~4 MiB default
   initial window is the estimator's doing. "Turn auto-tuning off so the arm is
   deterministic" is, on its own, a 64x reduction.
4. **SHAPES.md's table says tonic/hyper is at 65,535. It is at 2 MiB** (hyper's
   `DEFAULT_STREAM_WINDOW`), with a 5 MiB connection window. **And this slice's own published
   `rpc.log` repeated that number as the reason its wall-clock column was flow-control
   bound.** Neither stack was at 64 KB; 540 KB fits inside both defaults with no stall; the
   sentence is withdrawn. I had inherited it from the table and never checked, which is the
   thing this branch exists to stop, so it is C30 and it is mine.

The consequence for the arm is awkward and is published rather than smoothed: **SHAPES.md's
"pin a 4 MiB stream window in every cell and make it the headline" is not reachable on
grpc++**, and pinning what IS reachable makes the INCUMBENT stall where its default does not.
So `rpc.log` carries pinned and unpinned in full. The verdict is the same in both, which is
the honest way to report that the pinning did not matter here.

### What I added to the shared core, and why it was not optional

`ak_client_new` took a URI and nothing else, so cells B and C ran at hyper's defaults while
cell A ran at grpc-core's -- two different transports compared as one. `ak_client_new_opts`
is an ADDITION (R0 allows those) and `ak_client_new` is now one line calling it with NULL
options, so there is one connect path rather than two that can drift. Both, plus the
counters, live inside `--features rpc`, so the default artifact is untouched at 86 `ak_`
exports. A core test dials a pinned endpoint and gets the bytes back, because a builder that
rejects a setting fails at `connect()` and from a harness that looks exactly like "the
server is not up yet".

**What I did not do**: the payload sweep that would give the transport crossover, allocation
per RPC (SHAPES.md asks for it beside CPU and a page-fault proxy dressed as a count would be
worse than the gap), cells B and D on a small payload, and re-taking the codec tables on this
machine -- which is now the largest outstanding item in `STATE.md`, because the slice
publishes two machines' absolutes in one file.

## 2026-09-24: FIX-PLAN WP4 items 1 (C++ half) and 3 -- R-D1 and R-D2

Correctness only; no timing taken. Container: 4 vCPU Xeon @ 2.10 GHz, fresh, so protobuf,
grpc++ and python3-protobuf were reinstalled from apt (same versions as before).

### R-D1: the length wrap in `ak::Dec::len_body`

Reading `rt.h` confirmed the reviewer's arithmetic: `pos + k > len` with `k` off the wire
wraps, and `pos += k` then moves the cursor backwards. The 11 generated emission sites
and the three in `conformance.cpp` all go through `len_body`, and `skip`'s fixed-width
arms add constants and are post-checked, so the fix is one function: compare `k` as
`uint64_t` against `len - pos` (with `pos <= len` checked) before narrowing, plus the
same checked form in `f64()`. No generated file changed.

A first attempt at a hand-written repro harness was abandoned before it was run; it is
not in the tree. The repro that IS in the tree uses the corpus agent's WP4 item 2
vectors, which had already landed: 55 `X-lenwrap-*` rows, one of them
(`X-lenwrap-lrr-unknown-zero`) byte for byte the reviewer's input. `gen/lenwrap_rows.sh`
builds the corpus binary a second time in a scratch directory with the UNFIXED `rt.h`
first on the include path, and runs every row in its own process under `timeout 5`.
Before: 9 hangs. Under ASan, 4 more: the nested-string rows that the plain build
"refused" with -6 read past the buffer until the UTF-8 validator met an invalid byte, so
a plain run's clean-looking refusal was an out-of-bounds read. After: 0 of 55, plain and
ASan.

Two harness defects surfaced, and neither was the thing being fixed. The first corpus run
reported "no result" for every row from `U-wire-UploadResultDataMessage-upload-as-wt1`
onward, for all three arms. The cause: the shared core (then mid-edit by the rust agent)
panicked on that row, the panic aborted the ONE process that runs every row, and
`gen/corpus.py` never read the exit status (C32). A failure in the ffi arm was hiding the
native arm's results. Fixed: the exit status is printed and fails the run, and `--no-ffi`
gates native alone. The second: once the core stopped flushing groups after an error, 46
refused rows DISAGREED between arms. Both arms returned the same code on all 52 refused
rows; the difference was the facade left behind after the refusal, which nothing
specifies. The agreement is now the error code on refused rows, and the leftover object
is printed as its own fact (C33). That is a harness rule changing so that a gate passes,
so the fact stays visible and goes to the aggregating session to settle.

### R-D2: `ak_client_opts`

Confirmed: 3 fields declared, 6 read. `include/ak_abi.h` already existed as a generated C
header (from `gen/cpp_header.py`) with no RPC section. The struct is now rendered into it
from ak-abi's Rust declaration, the one the core asserts against its own field by field,
with size and offset asserts; the renderer refuses any field type that is not a 4-byte
scalar rather than laying it out. `rpc_common.h` asserts size 24 and a field count of 6,
and one `core_opts()` sets all six fields. Two plants are refused at compile time
(`rd2-guard.log`).

The history question has a clean answer, and it is not the one the finding feared. Both
RPC logs were taken on 2026-09-20 (17:43 and 18:33) and committed in `af2b100`, whose core
struct had exactly the host's 3 fields. The union to 5 fields (`908dc24`) and to 6
(`ef8fea9`) came 9 and 17 minutes after that commit, on another lineage. So host and
core agreed when those logs were taken, and the TCP rows are not invalidated. The defect
was latent for any RPC run after 18:47 on 2026-09-20, and this slice made none.

Observed and not fixed, for the aggregating session: ak-abi's `ak_queue_next` takes an
`i32` timeout where the core exports `u64`; `ak_bytes`/`ak_completion` are declared twice
in Rust with no assert tying them (C34); the packed decode arm ignores the arriving wire
type (C35, from reading only).

### Re-gate against the landed core (`6ede244`)

The aggregating session landed the core's half of R-D1 at `6ede244`. The core `.so` had not
been rebuilt since 08:53, so before trusting it I checked two things: the `poc/codec`
working tree is identical to `6ede244`, and `cargo build -v` in both of this slice's target
dirs reports `ak-core` Fresh. Only then did the gates run. Conformance was unchanged (443
x5, 441 lossy). Corpus: three arms at C++17 and C++11, 213 of 691 in scope, 0 failures, arm
agreement 213/213, walker 103/103. `FFI=1 gen/lenwrap_rows.sh`: the ffi arm refuses all
42 root rows with -3, the same code as native, and the 13 WireZoo rows have no ffi arm.
Boundary 21/21. rpccounts still counts 2/0, 3/1 and 4/0 with the six-field options. The 46
refused rows where the object left behind differs between arms (C33) are still there, as
the core's no-flush-after-error behaviour predicts.

## 2026-09-24 (second work unit): FIX-PLAN WP4 items 6 (R-D5, C++ half) and 8 (R-D7)

Correctness only; no timing reported. The rust agent is changing the shared core at the
same time, so every gate here was built against a `git archive 817174f ffi/poc/codec`
snapshot (the codec tree last changed at `6ede244`) through two new cache variables,
`AK_CORE_ROOT` and `AK_CORE_TGT`, in an out-of-tree build directory. The in-tree `build/`
was not touched and is stale against these sources.

### R-D5: the timed `ffi-valtc` arm was gated nowhere

Reproduced first, on the HEAD tree built unchanged in scratch (`rd5-before.log`): no
committed gate log contains `valtc`, `conformance.cpp` never builds the validating
transcoder, `contentsets.cpp` builds it only for its timed lambda, and `bench.cpp` sinks
every arm's return code into `AK_SINK`. To see what that costs, I applied one plant to
the committed bench and nothing else: a validating transcoder that refuses every string.
The refused encode got a P1.1 row at about 0.16 of protobuf, and the bench exited 0. That
is the failure mode in its plainest form: an arm that does no work because it refused
looks six times faster than the incumbent.

Fixed in three places. In conformance: rc and sha on every payload, plus the
malformed-UTF-8 string on the ENCODE side, which `ffi` must accept (no check, by spec) and
`ffi-valtc` must refuse with -6. Without that pair a validating arm that silently was not
validating would pass a sha gate, because every payload's strings are valid. It refuses
(rc -6, take -6, err -6), and a good encode after `ak_enc_reset` on the same context
succeeds. 476 checks at every level, 474 for the lossy build (443/441 before). In
contentsets: byte identity per set, 95 checks (80 before). In the bench: every arm carries a
gate, run once before calibration. A failing arm, or one with no gate, is dropped and main
exits 1. The borrowed facade's byte-identity check used to run AFTER its timing; it is now
its gate. `bench_a17_gateplant` is the fixture: `ffi-valtc` refused on 14 of 15 payloads,
not timed, exit 1, and absent from every table in timing mode (`rd5-gate.log`). P1.3 passes
under the plant because it carries no present string: a transcoder that is never called
cannot fail. That is expected, and it is also a reminder that a string plant says nothing
about a payload without strings.

`AK_BENCH_GATE_ONLY=1` and `AK_CS_GATE_ONLY=1` (with rounds 0) run the gates and time
nothing, so a correctness log carries no container figure.

### R-D7: the concurrency must-fail control never reached the core

Reproduced (`rd7-before.log`): `conc_a17_pad` and `conc_a17_both` linked
`core-build/target/release`, the unplanted core. The plants were `AK_CONC_*` defines in
`rt.h`, the native codec only. ffi 0 and hosttc 0 in every row. The reviewer's
double-count is real, and the mechanism is specific: `roundtrip` decoded through the core
and then RE-ENCODED WITH `ak::Enc`, so it was the planted native encoder observed a second
time. native N equals roundtrip N in every row, so "44 of 48" was 22 distinct wrong
encodes and "46 of 96" was 23. A third defect turned up while I was reading the output:
the T6 labels were typed strings. After T0 reordered the table to P1.1, P1.3, P1.2, P2.2,
"P1.2 alone" was really P1.3 alone and "P1.1 + P1.2" was really P1.1 + P1.3. The numbers
were right and the names were wrong.

The shared core already has both refused designs as test-only features (`pad-widths`,
`global-widths`, ak-rt's manifest; the rust slice added them for its own suite), so
nothing in `poc/codec` needed to change. CMake now builds three planted cores. Each planted
C++ build plants `ak::Enc` and links the matching core, and `conc_a17_corepad` plants the
core ALONE. `roundtrip` is now a decode check: value equality with the built object. T4's
poisoned threads got a separate `accepts`, because otherwise a truncated input that
decoded to a different value would stop counting as "accepted". The binary prints one
whole-run line per encoder, and `gen/concurrency.sh` requires, per build, which encoder
must be wrong and which must not, and decoder 0 everywhere.

Result, identical on two runs: pad has 23 of 96 distinct wrong encodes on each of native,
ffi and hosttc (T3), and 22 of 48 on each for two shapes (T6). Core-only pad reads 0 / 23 /
23: the ffi arm fails on its own. both reads 24 / 24 / 24. global is byte-clean on both
encoders, so the core's `global-widths` behaves the way `ak::Enc`'s does. The shared core's
pad plant produces exactly the C++ plant's count on the same schedule, and the two are
independent implementations of the same refused design.

Not done: T5 (two threads, one message) and T7 still drive only the native encoder, so
"both threads agree and are both wrong" under `both` is still a native-encoder reading.
There is no TSan run.
