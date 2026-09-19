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
