# backward-encode: the core's encode written backward (owner's experiment, 2026-10-02)

CONTAINER INSTRUMENTATION. No figure here is a campaign result. Nothing under `ffi/poc/codec`
is committed by this experiment: the change lives in the patch files below. The work was
built and checked in a private worktree (`git worktree add`, branch HEAD 1c9d8981, then the
patch), which is not kept.

## Files

| File | What |
|---|---|
| `backward-encode.patch` (sha256 ca9addd1...) | the experiment's SOURCES, against 1c9d8981: `poc/codec/crates/ak-rt/src/benc.rs` (new: the backward buffer), `ak-rt/src/lib.rs`, `ak-core/src/lib.rs` (the core uses it; `enc_blob`, `enc_blob_run`, `enc_raw`, the transcoder path, an additive count-only export `ak_enc_bwd_counters`), `poc/codec/gen/rust_abi.py` (the core's encoders rendered backward), `poc/codec/gen/rust_binding.py` (the Rust host binding's loop slots deliver last to first), and the experiment's checks in this slice's harness (`crates/harness/src/bwd_tests.rs`, `src/bin/bwd_check.rs`, `lib.rs`, `Cargo.toml`) |
| `generated.patch.gz` (uncompressed sha256 712913b8...) | what `poc/codec/gen/generate.py` writes from the patched generator: the four `codec.rs` of the core (shapes, no-unknown, corpus, corpus no-unknown) and this slice's four `binding*.rs`. Every other generated file of every slice is unchanged (`abi.rs`, `layout.rs`, every C header, the cpp/csharp/java/python outputs: `generate.py` reported them `same`) |
| `dropped-v2-sized-run.patch` | v2, on top of the patch: a packed run written forward into a hole of its summed size instead of walked last to first. Measured and DROPPED (below) |
| `checks/gate/` | `gen/bwd_gate.sh`: the gate (`gen/gate.sh`, steps unchanged) in the worktree, both builds, and `bwd_check` (full, counting, no-unknown builds, planted control) |
| `checks/mismatch/` | `gen/bwd_mismatch.sh`: this branch's binaries (forward binding) against the backward core |
| `bench/` | `gen/bwd_bench.sh`: the codec suite's encode cases, committed vs backward, alternated, and the RPC probe; `tables.md` here is `gen/bwd_tables.py bench checks/gate/bwd_check.log` |
| `bench-p6-v2/`, `bench-p6-2x2/` | `gen/bwd_ab.sh`: P6.1 only, committed / v1 / v2, then harness x core |

Reproduce: `git worktree add --detach W 1c9d8981; cd W; git apply backward-encode.patch;
(cd ffi/poc/codec && python3 gen/generate.py)`; the regenerated files then equal
`generated.patch.gz`. Then `ffi/poc/rust/gen/bwd_gate.sh W OUT` from this checkout.

## What was built

1. **Downward buffer** (`ak_rt::BEnc`): the storage is a `Vec`'s capacity, the message is
   `[pos, cap)`, the cursor moves toward the start. Every recorded position (`BMark`) is a
   distance from the end, so it survives a grow. A grow allocates at least
   max(2 x capacity, written + need + head) and copies the content to the END of the new
   buffer (geometric; `grow_is_geometric` test). `head` (5 bytes on a core context) is kept free
   at the START: `take_framed` writes the gRPC prefix just below the message and hands out
   `[pos - 5, cap)`; `take` hands out `[pos, cap)`. Both are `Bytes::from_owner` over the moved
   buffer, recycled through the same ring of 6 (`SPARES`), no copy. `reset` = cursor to the end.
   Rollback on `ak_fail`: unchanged in kind (the encode returns the sticky code; the next
   encode resets); tested mid-field below.
2. **Generator** (`rust_abi.py`): every message's encode plan rendered REVERSED (last field
   first), each nested message `open()` -> body -> `close(mark, tag)` (length, then key), the
   unknown-field bag (`enc_raw`, prepended) therefore written first and landing after every
   known field as in the forward encoder; the oneof checks still run before anything is
   written. Element entry points walk their array `n-1 .. 0`; packed runs walk the host's
   array last to first (`varint_run_rev`; doubles are one copy). Each `ak_run_*` call stays
   one packed record, as before. Length-prefix SITES are still allocated in the plan's forward
   order, so `SITES` / `SITE_NAMES` (an ABI export) are unchanged; nothing reads them at run
   time.
3. **Call blocks**: NOT built, by the owner's change of design (2026-10-02): the core does no
   reordering. Instead the HOST CONTRACT changes, item below.
4. **Transcoder and direct-argument writes**: a blob whose length is known is placed directly
   at cursor - total (key, length, bytes in one pass, no move): the passthrough transcoders
   (`ak_tc_utf8_trusted` / `ak_tc_bytes`, optimisation E1), the direct argument (section 8),
   and a passthrough `ak_blob_run` (its size is summed first, then it is written forward into a
   hole of that size). The core's validating UTF-8 transcoders (`ak_tc_utf8`, `_simd`) are
   handed exactly the `len` bytes ending at the cursor (`space_known`: their output is `len`
   bytes on success), so nothing moves. Any other transcoder (latin1, utf16, a host's) is
   handed the whole free region from its bottom (`space`: ABI section 4's "whole remaining
   buffer"), writes forward, and after `commit(n)` its n bytes are moved up to just below the
   cursor (a move of n bytes), then length and key. A `grow` from inside a transcode keeps
   what the transcoder already wrote at the start of its region (test
   `transcoder_grow_keeps_partial_output`). `AK_ERR_CAPACITY` (n > the capacity handed out):
   unchanged, tested. `ak_blob_reserve` and `ak_str_elem` (named in the task) do not exist in
   this core; nothing to change.
5. **Learned widths removed** for this core: `BEnc` has no widths table, no placeholder and no
   prefix move. `ak_enc_site_moves` reports 0 for every site. The `global-widths` and
   `pad-widths` features still exist (the harness builds with them) and change nothing in the
   core (they still act on `ak_rt::Enc`, core-native's forward buffer, which is unchanged).

### The host contract (owner's design, as built)

Within one repeated field (repeated messages, strings and bytes, maps as repeated pairs,
packed runs delivered in several calls) the host delivers elements LAST TO FIRST across all
its calls, whatever the form (n = 1, 32 KB chunks, mixed). The order INSIDE one call: the host
fills each chunk or run array in FORWARD element order and the codec walks it last to first.
Chosen over "the host fills the chunk reversed and the codec walks it in index order" because
(a) a packed run is the host's own array handed over whole (ABI section 6): the reversed-fill
rule would make the host copy its array reversed, the chosen rule hands it over untouched;
(b) for a host with an indexable container the generated fill loop is unchanged except that
the chunk ranges are taken from the end, and the token of element i stays `tok0 + i` with
`tok0` = the index of the chunk's first element. One rule for every entry point
(`ak_elem*`, `ak_elemu*`, `ak_blob_run`, `ak_run_*`).

The Rust binding (generated by `rust_binding.py`, the four loop renderers): iterate the
facade container in reverse (`iter().rev()`: `Vec` and `BTreeMap` are double-ended), fill the
arena from its TOP down, deliver the filled tail (`chunk.as_ptr().add(CHUNK - i)`, `tok0 =
total - done - i`); the zeroed fill clears the top of the arena. Packed fields: unchanged (one
call per field). No hand-written Rust harness path drives element calls except `stickyerr`
(one call of two elements: no change needed; it passes).

**The call signatures do not change; the contract does.** A host built for forward order
linked with the backward core produces silently PERMUTED repeated fields (same lengths, no
error): every call's own elements come out in order (the codec walks each array), but the calls
come out in reverse. Measured, `checks/mismatch/` (this branch's binaries, unchanged, loading
the backward core):

| check | result |
|---|---|
| conformance (byte identity, 16 payloads) | FAILED: 5 payloads DIFFER (P1.2, P2.2, P2.3, P2.4, P4.1: every one with a field longer than one chunk), sizes identical; the others pass |
| shapes (presence, oneof, unknown-field vectors) | PASSED: every vector's fields fit in one call |
| codec pre-check (every timed arm, every input) | 69 failures of 5,740 checks (P1.2, P2.2, P2.3, P2.4 and content sets, P3.1 retain, P4.1); P1.1, P1.3, P2.1, P2.5, P5.x, P6.1 pass |
| corpus through the C ABI | 6 arm-row failures (B-P4_1, C-leaf-2048 drop; B-P3_1, B-P4_1, C-elemu-512, C-leaf-2048 retain): 678/680 and 676/680 |

So a mismatched pair is caught only by a payload with a field delivered in more than one call
and elements that differ; nothing at load time or in the signatures catches it.

### The other slices (not touched; what would have to change)

Every call site is GENERATED; no slice has a hand-written element, blob-run or run call
(grep of poc/cpp/src, poc/csharp/src, poc/java/src/java, poc/python). Each would change in its
backend of the shared generator:

| slice | backend, outputs | containers | reversing |
|---|---|---|---|
| cpp | `gen/cpp_binding.py`: `src/generated/binding*.cpp`, `binding_borrow*.cpp`, `corpus/src/generated/binding*.cpp` | `std::vector` (indexed), `std::map` (bidirectional) | cheap: index from the end / `rbegin` |
| csharp | `gen/cs_binding.py`: `src/Harness/Generated*/CoreFfi.cs`, `src/Corpus/Generated*/CoreFfi.cs` | the loop slots deliver from native STAGING arrays by offset (staged forward beforehand) | cheap: iterate the `off` loop from the end; staging unchanged |
| java | `gen/java_binding.py`: `src/generated*/java8|java17/**/Binding.java` (the JNI shim, `java_jni.py`, forwards one call and is unchanged) | `java.util.List` read with `get(k)` (ArrayList), `TreeMap` | cheap: index from the end, `descendingMap()`; a `LinkedList` would make `get(k)` O(n) either way |
| python | `gen/py_capi.py`: `gen/out/**/binding.c` (`py_pure.py` is a separate pure-Python encoder, not the core's host) | `PyList` (indexed); dict keys materialised as a sorted list | cheap: index from the end |

A host whose container is forward-only (an `IEnumerable`, a non-List `Iterable`, a Python
generator, `std::forward_list`, `std::unordered_map`) would have to buffer the field (or its
element references) before the first call: an O(n) pass and an allocation the forward contract
does not need. None of the slices' current facades has one.

## Checks (backward core, both builds)

| check | result | log |
|---|---|---|
| gate step 1: generators current, one core | ok (the worktree's generate --check, one_core 0 failures) | `checks/gate/gate.log` |
| step 2: ak-core / ak-rt unit tests | 6 + 23 passed (7 new BEnc tests: forward bytes across grows, geometric grow, grow inside a nested message, transcoder region + commit move + CAPACITY, partial transcoder output kept over a grow, take_framed + ring recycling, packed run reverse walk) | gate.log |
| step 3 byte identity (16 payloads, P1.3 and P2.5 included) | VERDICT pass, every arm | gate.log |
| step 4 shapes (presence, oneof, unknown vectors) | pass | gate.log |
| step 5 counts | pass; "warm misses" 0 everywhere (vacuous: no learned width) | gate.log |
| step 6 content sets | every payload and set ok | gate.log |
| step 7 concurrency suite (obligation 12.5) | shipped 3 runs 0 wrong, global 3 runs 0 wrong; the planted `pad` and `global+pad` arms do NOT fail ("the suite is blind to it"), concur.sh exit 2, recorded and not fatal: VACUOUS, see below | gate.log |
| steps 8-10 lifecycle, R-D1, R-D6 sticky error | pass | gate.log |
| step 11 corpus, decision 11 controls | ffi 680/0, native 696/0 per mode; controls fail as required; decision 11 per-position discard, pull == push, placement: pass | gate.log |
| 11b pre-check | 5,740 checks 0 failures (no-unknown 3,257) | gate.log |
| 11c crossing counts | 836 lines IDENTICAL to `gen/crossings.txt`; no-unknown 435 identical | gate.log |
| 11d header_diff, 11e upload_check, 11f rpc_semantics (72 cases) | pass, both builds | gate.log |
| 12 no-unknown build, both C headers vs both cores | GATE PASSED | gate.log |
| bwd_check (the experiment's own checks) | 78 passed, 0 failed, on the full, counting and no-unknown builds | `checks/gate/bwd_check.log` |
| planted control AK_BWD_PLANT=forward | 47 FAIL, 31 PASS (the PASS rows: a field in one call, or P1.3's identical empty elements): detected | `checks/gate/bwd_plant.log` |

bwd_check, against the facade's prost encoding (independent of the core), on a warm AND a
fresh (4 KiB, so grows happen mid-field and mid-message) context: M1 (P1.1-P1.3, a leaf
element with nested Timestamps), M2 (P2.1-P2.5: non-leaf elements and tokens, four inner
string fields through `ak_blob_run`, the options map through `ak_elem_TaskOptionsOptionsEntry`)
with the transcoder varied per string (trusted, validating utf8, latin1, utf16: the known-
length path and the region + move path), each fed by n = 1 calls, chunks of 8, chunks of 32,
mixed sizes (1, 7, 3, 12, 2 cycling) and one call; M6 (P6.1) with every packed field in
several calls (several packed records: prost's decode equals the value; one call:
byte-identical); M5 (P5.1-P5.4: the direct argument inside the nested `upload` message, fresh
context: a grow inside the nested message); rollback: `ak_fail` at element call 3 (P1.2) and 5
and 40 (P2.2, mixed forms and transcoders): the encode returns AK_ERR_HOST, the code is
sticky, a later element call gets it, and the next encode on the same context is
byte-identical; a transcoder over its capacity: AK_ERR_CAPACITY, then a byte-identical encode.

### Obligations that become vacuous on this core (their tests kept, not deleted)

- **ABI 12.5's history surface** and the learned-width half of the concurrency suite: the
  encoder keeps no state between encodes but its buffer, so no ordered pair of shapes has a
  history surface; the planted `pad-widths` and `global-widths` arms have nothing to act on in
  the core, and concur.sh reports them as not failing. The suite's shipped arm (2 shapes in
  sequence, then 2/4/8 threads with a context each, every encode byte-compared against prost)
  still applies and passes.
- **Decision 5's prefix-move counters** (`prefix_moves`, `prefix_bytes`, `ak_enc_site_moves`):
  always 0; counts.rs's "warm misses = 0" regression passes trivially.
- **ABI section 6 "Length placeholders use a learned width" and its two refusals**: no
  placeholder exists. **The open-field save/restore obligation stays** (the element entry
  points still read and restore `open_tag` / `open_site`).

## Element calls per payload (counting build, warm encode, the binding's loops)

Every element call is now delivered in reverse order; the count equals the core-counted
forward crossings minus the one `ak_encode_*` call on every payload and mode (checked by
bwd_check: "ok" on every row), and the crossing files compare identical, so the reverse
order changes no crossing count. Per payload, drop / retain: element calls (elements carried):
P1.1 1/1 (4), P1.2 7/9 (1,000), P1.3 2/3 (300), P2.1 6/6 (17), P2.2 2,510/2,516 (8,500), P2.3
628/629 (15,625), P2.4 402/403 (24,880), P2.5 101/101 (340), P3.1 1/2 (200), P4.1 202/203
(1,000), P5.x 0 (the direct argument), P6.1 1,001/1,001 (30,200). Every call carried more than
one element except 1 of P2.1's 6. Transcoder moves and grows: 0 on every warm encode (the
binding uses the passthrough transcoder; the move path is exercised by bwd_check's mixed
transcoders). Table: `tables.md`, last section.

## Measurement (CONTAINER INSTRUMENTATION)

Benchmark wall: 453 s codec (6 processes, 168 cases each) + 7 s RPC + 29 s (P6 v1/v2) + 39 s
(P6 2x2) = 528 s. Codec processes pinned to CPU 1, one at a time, alternated committed /
backward per launch with the same seed. Absolute ns of process CPU per encode for every
payload, arm, mode and end state are in `tables.md` (median of the three launch medians,
[min - max] of the launch medians, p10-p90 of all samples).

How to read it, and what it does not establish:
- The two variants are two HARNESS builds as well as two cores (the backward core needs the
  reversed binding; a mixed pair writes wrong bytes, so the pre-check would refuse it). The
  control row, core-native (unchanged code in both builds), has DISJOINT launch-median ranges
  on 26 of its 84 rows (15 higher, 11 lower in the backward build): build and process drift
  of a few percent is in every cross-variant comparison. core-ffi has disjoint ranges on 47 of
  84 rows (34 lower, 13 higher).
- Consistent across a whole family: **P2.4 and its content sets, every mode and end state,
  backward lower** (e.g. P2.4 drop reused-buffer 240.5 us [233.2-248.1] committed, 217.0 us
  [214.9-220.4] backward; P2.4/wide 408.5 vs 386.8 us); P1.3, P2.1 and P5.1's
  transport-ready rows lower (P5.1 transport-ready-tonic 176 vs 153 ns).
- **P6.1 (packed runs), every row, backward HIGHER: 60.7-65.4 us committed, 75.2-79.5 us
  backward.** Not build drift: P6.1's fields are each delivered in ONE call, so its bytes do
  not depend on the delivery order and a 2x2 is valid (pre-check 0 failures in every process).
  `bench-p6-2x2/`: with the SAME harness binary, the forward core 60.4-66.4 us and the backward
  core 74.2-77.6 us, in both harness builds; core-native moves only with the harness build
  (65-72 us in the committed build, 56-67 us in the worktree build). The cost is the core's.
  Not attributed (no perf in this container). v2 (`dropped-v2-sized-run.patch`: sum the
  varint lengths, then write the run forward into a hole of that size) is slower still
  (`bench-p6-v2/`: 80.4-86.0 us against v1's 75.3-79.6 us and the committed core's 60.3-63.3 us,
  same session): dropped.
- P1.2 (ASCII) core-ffi rows higher (85.7 vs 89.2 us drop reused-buffer) while P1.2/latin1 and
  P1.2/wide, the same shape and call pattern, are lower or overlapping: not separable from the
  drift above.
- Everything else overlaps.

RPC (`bench/`, TCP 127.0.0.1, pinned configuration, k = 1, 2 processes per variant
alternated, every call checked: status and the server's byte count): direction d (16 MiB in
2 MiB chunks) and c (P5.4) WORK through the backward core on every cell. Client CPU per call,
median [min - max] over 8 rounds: Cf d/16MiB 11.04 ms [9.85-12.77] committed, 11.63 ms
[9.68-12.39] backward; Cf c/P5.4 2.96 [2.68-3.84] vs 3.06 [2.77-3.27]; C d/16MiB 13.15 vs 12.65;
Df d/16MiB 10.81 vs 10.81; control A 10.65 vs 10.70. No difference resolved at this spread.

## What contradicts the expectation that backward encoding is faster

- P6.1 (packed runs: 30,200 values in 1,000 runs): the backward core costs about 12-16 us more
  per encode (about 0.4-0.5 ns per value), confirmed with the same harness binary; neither
  the reverse walk (v1) nor a sized forward hole (v2) recovers it.
- On most payloads the difference is inside the build-drift band of the control row; the
  removed placeholder, prefix move and widths table are not visible above it, except on the
  P2.4 family (lower) and the small payloads P1.3, P2.1, P5.1 transport-ready (lower).

## What is not measured

- The pool input (requirement 11's distinct graphs), the transport-ready-core rows, the
  no-unknown build's timings, decode (unchanged code), incumbent-prod and armonik rows.
- The 2x2 (same harness, both cores) on any payload whose bytes depend on the delivery order
  (impossible: the mixed pair writes wrong bytes).
- Attribution of the P6.1 cost (perf is not installed here).
- A forward-only host container (buffering cost): no slice has one.
- The 1.88 floor on the patched tree; the campaign machine; other slices' hosts against the
  backward core (they deliver forward; per the table above they would fail byte identity on
  multi-call fields).
- A cold first encode's grow cost (every timed row is warm; bwd_check covers grows for
  correctness only).
