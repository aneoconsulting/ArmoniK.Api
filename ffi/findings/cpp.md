# Reading the cpp slice

Phase note: this file records facts only; container timings were removed on 2026-09-24 (design/FIX-PLAN.md WP2). The raw logs remain in logs/cpp/.

The aggregating session's reading of `poc/cpp`. It is not the slice's own
`STATE.md`. It lists what was built, what was checked and what is not
established. It makes no recommendation; decisions are the owner's.

**Configuration** (R7): g++ 13.3.0, `-O2 -DNDEBUG`, protobuf C++ 3.21.12 and
grpc++ 1.51.1 from apt, upb v25.3 built from source, rustc 1.94.1.
`packages/cpp` pins neither protobuf nor grpc and sets `CXX_STANDARD 14`.
The logs come from two different containers: `rpc.log` and `rpcflow.log` from
one, every other log from another (`poc/cpp/STATE.md`, Machine row).

## 1. What was built

- **Codec.** The generated codec for every message and payload of
  `design/SHAPES.md`, over the shared core at `poc/codec/crates/ak-core`
  (no per-slice copy; `logs/cpp/w10-one-core.log`, `logs/cpp/generator.log`).
  Floor and target implementations; the target is C++17.
- **Arms.** `pb`, `pb-det`, `pb-arena` (the incumbent, protobuf C++),
  `memcpy` (floor), `native` (the codec emitted into C++, no boundary), `ffi`
  (through the C ABI), `ffi-valtc` (validating transcoder), `ffi-zeroed`
  (ABI v1 decision 9 candidate fill), `ffi-nobat` (host declines to batch),
  `ffi-hosttc` (transcoder in the host), `groupfill` (host-side group fill
  alone), `ffi-borrow` (decode into a facade whose strings are views over the
  input buffer; a measurement arm, not a proposal), and `upb` in a separate
  binary (minitables built by reflection from `protoc`'s descriptor set, no
  Bazel, no `protoc-gen-upb`).
- **Linkage.** Shared library and static, as separately labelled arms and
  separate processes.
- **RPC arm.** The `design/SHAPES.md` four-cell grid in each of ABI v1
  section 9's three deliveries (blocking, callback, queue), over a Unix domain
  socket and loopback TCP, pinned and unpinned (`logs/cpp/rpc.log`,
  `logs/cpp/rpcflow.log`). `ak_client_new_opts` was added to the shared core so
  the transport could be configured; `ak_client_new` is now a call to it with
  NULL options.
- **Standalone checks.** Group skip (`groupskip.log`), corpus consumer
  (`corpus.log`), ODR layout check (`odr.log`), boundary/inlining check
  (`boundary.log`), concurrency suite (`concurrency.log`), content sets
  (`contentsets.log`), UTF-8 validator exhaustive check (`utf8.log`).

## 2. Correctness and byte identity

- `logs/cpp/conformance.log`: five encoders byte-identical against
  `manifest.json` on every payload, plus absent, unknown and malformed vectors.
  443 checks, 0 failures, on each of: C++17 target shared, C++17 floor shared,
  C++14 floor shared, C++11 floor shared, C++17 target static. The lossy
  decode-policy build runs 441 checks, 0 failures. One payload (P2.5) has two
  valid encodings.
- P2.5: upb writes 19,712 B, the same form protobuf C++ writes, which
  `design/SHAPES.md` records as the second valid encoding.
- `ffi-borrow` is gated by byte identity: decode into the borrowed facade,
  re-encode, compare with the manifest, every payload
  (`bench_a17_shared.log`: "byte identity holds on every payload").
- Content sets (`contentsets.log`): each set round-trips to the same bytes.
  Wire size is 1.687 to 1.748 times ASCII for Latin-1 and 2.373 to 2.495 times
  for wide. protobuf C++'s generated code calls
  `VerifyUtf8String(..., SERIALIZE)` unconditionally at 37 call sites; ABI v1
  decision 3 says the core does not validate on encode.
- `odr.log`: 144 layout facts compared between C++11 and C++17 builds, 49
  moved (for example `sizeof ak::Optional<int32_t>` 8 vs 16). Mixing levels in
  one program would be an ODR violation; the check exits nonzero on a moved
  layout.
- `AK_CXX17` reaches 11 sites in the emitted tree, all on the decode side
  (`poc/cpp/STATE.md`).

## 3. Floors

C++11 and C++14 floors are demonstrated, not declared: both build and pass the
full conformance gate (`logs/cpp/conformance.log`, `-std=201103` and
`-std=201402`), the group-skip suite (`groupskip.log`) and, for C++11, the
corpus run (`corpus.log`).

## 4. Corpus consumption (`logs/cpp/corpus.log`)

- 128 of 336 rows root at a message this slice covers; run with three arms:
  `native`, `ffi`, and protobuf C++ as an oracle through its own reflection.
  Run at C++17 and at the C++11 floor with the same result.
- Verdict: 0 failures, 1 disputed (`U-map-entry`, where upb disagrees with
  protobuf C++, pure-Python protobuf and this slice), 1 permuted (`B-P7_1`,
  whose accepted encoding no canonical writer produces). The verdict line keeps
  the three buckets apart.
- 128 of 128 rows: `native` and `ffi` decode to the same facade.
- The 62 `WireZoo` rows root at a message outside this slice's schema. A
  schema-less walker over `Dec::skip` agrees with the corpus's verdict on 62
  of 62, including both reject vectors.
- ABI v1 decision 11: this slice drops unknown fields in both its arms;
  protobuf C++ retains them.

## 5. Crossing counts

- Codec, per payload and arm, from the counting core (`logs/cpp/counts.log`):
  forward and reverse counts for encode, encode zeroed-fill, encode unbatched,
  encode host transcoder, and decode, shared and static. Example, P2.2 encode:
  2511 forward / 2501 reverse batched, 8501 forward unbatched, 19668 reverse
  with the host transcoder; P2.2 decode: 1 forward / 3501 reverse.
- RPC, from a counting core (`logs/cpp/rpc.log`, R5 block): blocking 2 fwd /
  0 rev, callback 3 fwd / 1 rev, queue 4 fwd / 0 rev per RPC. The counts are
  identical on a 540 KB response with about 4,500 fields and on an empty one,
  so they do not depend on field count. ABI v1 section 9's table says 2/0,
  2/1 and 3/0: it does not count `ak_call_destroy`, which the callback and
  queue deliveries require.

## 6. ABI v1 decision 1

Decision 1 (are the managed-motivated amendments free at the C++11 floor) is
reopened and goes to the campaign. The batching conclusion previously written
here rested on a `STATE.md` table that matches no committed log (R-C1). What
remains as fact: the arms that isolate each amendment exist (`ffi-zeroed`,
`ffi-nobat`, `ffi-hosttc`, `groupfill`) and their crossing counts are in
`counts.log`. A calibrated delay in front of every forward entry-point call
exists as a harness mechanism (`tax.log`); its output is instrumentation.

Mechanisms identified from the source, not from timings:

- Decision 9's candidate clear, as written in `rust_abi.py`, clears the whole
  32 KB chunk rather than the elements that will be filled.
- With the declared expansion bound removed (ABI v1 section 4), the core opens a
  length prefix of a learned width and resolves it afterwards; a host already
  holding the bytes could write key, length and body in one pass. A
  `tc == ak_tc_bytes` fast path in the shared core emitter would do this.
- Where a container is not the wire layout (`vector<TaskStatus>`,
  `vector<bool>`), the binding materialises a contiguous array before encode.

## 7. upb facts

- `upb/port/def.inc` defaults `UPB_FASTTABLE` to 0; `gen/fetch_upb.sh` first
  defined neither enabling macro, so the upb arm ran with the fast decoder
  compiled out. `upb.log`'s configuration line now says so.
- The fast dispatch is unreachable from a reflection-built minitable:
  `upb/wire/decode.c:766` requires `table_mask != -1` and
  `upb/mini_descriptor/decode.c:698,712` sets it to -1. The runtime prints
  `table_mask = -1`; the archive holds 0 fast-parse functions without the
  define and 42 with it (`upb-fasttable.log`, `STATE.md`).
- `protoc-gen-upb` output was not built (needs Bazel).

## 8. Defects found, and what found them

- **Group skip (C24).** `skip(wire)` had no case for wire type 3, so every arm
  rejected a legal message carrying an unknown GROUP field. 443 conformance
  checks passed over it, because proto3 cannot express a group and the manifest
  is generated from proto3. Found by the python slice's corpus run in the
  shared core; the same defect was in this slice's `include/ak/rt.h`. Fixed:
  `skip(tag, wire)` plus a field-number-matching `skip_group` bounded at 100
  with `ERR_DEPTH`, swept across 13 emission sites in `gen/cpp_core.py`.
  `groupskip.log`: 11 checks at four (standard, implementation) pairs, 0
  failures; the depth-counting plant fails the two mismatched-end cases and the
  dropped-`case 5:` plant fails the two cases carrying a `fixed32`. The second
  plant reproduces a regression introduced during the fix and caught by the
  tests.
- **Rust slice build break (C27, open).** The C24 signature change was not
  regenerated in `poc/rust`, so the rust slice does not build on this branch
  (20 E0061 errors). Not this slice's source.
- **Harness use-after-free.** The first corpus run reported `ffi` writing an
  unaccepted form on 114 of 126 rows: `ak_enc_take` returned a pointer into a
  freed context. Found because the same wrong hash repeated across unrelated
  vectors.
- **Baseline defects found by review (C1, C7, C8, C11).** The incumbent's
  encode harness cleared and resized its output (a zero-fill per iteration),
  hand-rolled a `CodedOutputStream` instead of `SerializeToString`, and charged
  deterministic map ordering to the headline. The no-boundary control used
  `std::vector::push_back`, then decoded without reserving where the binding
  reserves. The RPC arm counted different thread sets per arm. All fixed in the
  harness; the figures they affected are gone from this file.
- **Concurrency suite (C21, C22).** The first suite passed on all three planted
  builds because its payload pairs never shared a width-table site; the
  reference was built with the encoder that carried the plants. Fixed: P1.1 and
  P1.3 as the pair, protobuf's encoder as the oracle. See R-D7 below: the plants
  still do not reach the shared core.
- **grpc++ window.** grpc++ exposes no argument for the connection window, so
  the pinned 4 MiB stream-window configuration of `design/SHAPES.md` is not
  reachable; `rpc.log` carries both configurations (`rpcflow.log`).
- **Boundary check.** `boundary.log` reports 21 checks, not 23, because gcc now
  inlines `dec_list_results_response` into its caller inside the control TU.

## 9. Harness facts

- A baseline that does extra work (zero-fill, hand-rolled stream, extra
  ordering) is invisible from inside the harness; it was found only by review.
- A harness defect can look like a codec defect; a repeated identical wrong
  hash across unrelated inputs points to the harness.
- A must-fail plant is needed for every check: the first concurrency suite and
  the first group-skip probes both passed for the wrong reason.
- Byte identity against a schema-generated manifest cannot reach wire forms the
  schema language cannot express; a hand-built corpus can.
- The P1.2 decode round-to-round outlier (C16) traces to glibc's mmap and trim
  thresholds (`c16.log`); pinning both `MALLOC_MMAP_THRESHOLD_` and
  `MALLOC_TRIM_THRESHOLD_` removes it, pinning only the first does not.
- Two containers of nominally similar spec gave crossing figures that do not
  agree (R13); no absolute crosses between `rpc.log`/`rpcflow.log` and the
  other logs.
- Cross-binary comparisons (C++11 vs C++14 vs C++17 builds) carry layout drift
  (`drift.sh`, `drift.log`); `AK_CXX17` changes only 11 sites, so most of the
  code in those binaries is identical source.

## 10. Not measured, not established

- Every performance comparison: encode, decode, RPC, upb, borrowed facade,
  batching, string-as-data, content sets, floor cost. All container timings are
  instrumentation until the campaign (README 1.1).
- The borrowed facade is a measurement arm, not a design: no lifetime contract
  for the input buffer, no hybrid facade, no public-surface cost.
- `protoc-gen-upb` output; a second compiler on the codec arms; allocation or
  footprint; content sets beyond the string path; request direction in RPC.
- C27 is open, so the rust slice's crossing benchmark cannot be run on this
  branch.

## 11. Open review findings (design/FIX-PLAN.md section 7)

All unconfirmed until the slice agent answers them, except those the register
marks verified.

- R-B2 (verified): the empty-call RPC sign was stated reversed against
  `rpc.log:694`. Figure removed with WP2.
- R-C1 (verified): the `STATE.md` table behind the decision-1 batching verdict
  matches no committed log. Decision 1 reopened.
- R-C4: the C++ RPC grid runs client and server in one process.
- R-C5: grid rows across slices are different experiments (delivery, decode
  family, units, server accounting, direction).
- R-C8: only the response direction is measured in RPC.
- R-C10: the C++ headline incumbent is the library's best path, not gRPC's
  marshaller (R14).
- R-C15: the batching crossover was compared against other containers'
  absolutes, and the tax sweep does not reproduce run to run.
- R-D1 (source verified, not run): length-varint wrap in `rt.h` as well as the
  shared core.
- R-D2 (verified): `ak_client_opts` in `rpc_common.h` declares 3 of 6 fields;
  `tcp_nagle` is read from the stack. TCP rows of `rpc.log`/`rpcflow.log` may be
  invalid pending the history check.
- R-D5: `ffi-valtc` is not in any gate log.
- R-D7: the concurrency plants never reach the shared core; wrong encodes are
  double-counted (22, not 44).
- R-E1 (verified): the "one traversal" claim in `rust_core.py` is false.
- R-E2: wire-type acceptance differs across emitters, including `cpp_core.py`.
- R-F1: the C++ `STATE.md` contradicts itself on what exists.
