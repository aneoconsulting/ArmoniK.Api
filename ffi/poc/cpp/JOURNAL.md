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
