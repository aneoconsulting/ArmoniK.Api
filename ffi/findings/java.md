# Reading the java slice

Phase note: this file records facts only; container timings were removed on 2026-09-24 (design/FIX-PLAN.md WP2). The raw logs remain in logs/java/.

The aggregating session's record of `poc/java`: what was built, what was checked,
what was counted, what defects were found, and what is not established. Every
timing in `logs/java/` was taken in a container and is instrumentation (README
section 1.1). None is quoted here.

## Configuration

- Incumbent: protobuf-java **3.25.5**, the version `packages/java`'s pins resolve
  to (the pom declares 3.19.0; grpc-java 1.74.0 brings 3.25.5). protoc 3.19.0
  generates the classes.
- Target: JDK 17 (`17.0.20`) with JNI. Floor: Java 8 (`openjdk 1.8.0_502`). JDK 21
  was used only for the FFM probe and the virtual-thread probe.
- Core: the shared crate at `poc/codec/crates/ak-core` (R0). The two transcoders
  the slice needs (`ak_tc_utf16`, `ak_tc_latin1`) live there and are resolved from
  there, checked with `ldd` and `nm` (`logs/java/w10-regate.log`).
- `packages/java` builds with `maven.compiler.release` 17, while the binding's
  floor is Java 8 (README section 5). The two are different constraints.

## What was built

- **Full codec** for the shapes in `design/SHAPES.md`, emitted by `poc/java/gen/`.
- **Two source trees from one generator**, `src/generated/java17/` and
  `src/generated/java8/`. Java has no preprocessor, so one tree per level is how
  README 5.1's first condition is met. The target tree reads `String.coder` and
  `String.value` and hands the core the string's compact storage; the floor tree
  has neither and stages strings through `getChars` as UTF-16.
- The floor binding is also emitted into its own package (`ak.floor`) so README
  5.2 arm b runs inside the target's process.
- **JNI binding**: `native/generated/shim.c`, 52 entry points, 18 encode-loop
  trampolines, 32 decode trampolines, the ABI v1 section 5 guard. Method ids are
  cached at load.
- **Arms**: `pbj` and its other modes, `R` / `R-take` (the generated pure-Java
  codec), `ffi` / `ffi-take`, `ffi-nobatch`, `ffi-zeroed` (open decision 9),
  `ffi-borrow` (open decision 13), `floor-batched` (arm b), and an R14 arm that
  calls grpc-java's `ProtoLiteUtils` marshaller (`logs/java/r14.log`).
- **Pull decode** (ABI v1 7.1): `ffi-pull` (drain) and `ffi-pull-walk`, over the
  pull family in the shared core. `ak_parse_*` makes no upcall by construction, so
  the binding hands the wire over under `GetPrimitiveArrayCritical` without copying
  it into native scratch, and the shim pushes no callback frame on that path.
- **FFM secondary arm**: a downcall probe only, on JDK 21 with `--enable-preview`
  because the container has no JDK 22 (`logs/java/ffm.log`). FFM is not built as a
  binding, and no FFM upcall exists.
- **Generated Java codec, arm R**: the no-boundary control, from `gen/java_codec.py`.
- **RPC arm** (the former `logs/java/rpc.log`, withdrawn in c23533ea7 under R-C9, no raw runner output; `git show e75585be6:ffi/logs/java/rpc.log`): grpc-java, the core's blocking call, and the
  core's completion queue (`ak_call_unary_q`). A four-cell grid (A: protobuf-java
  over grpc-java; B: protobuf-java over the core; C: core over core; D: core codec
  over grpc-java) over a UDS, with a **two-process mode**: `-Dak.rpc.serve=<path>`
  runs the server in its own process and `-Dak.rpc.connect=<path>` measures a
  counter holding only the client. `ak_call_cancel` exists; the callback delivery
  mode is not built.
- **Packaging probe**: `ak.Str17` probes the `String` layout at class init and
  falls back, so runtime capability dispatch between the two levels is built.

## Correctness

- **437 checks, 0 failures** on each of README 5.2's three arms: target on target,
  floor on target, floor on floor (Java 8 `openjdk 1.8.0_502`),
  `logs/java/floor.log` and `logs/java/conformance.log`. Seven arms, all 16
  payloads, byte identity against `schema/generated/manifest.json` and the
  committed vectors byte for byte.
- **All three content sets** in the gate on every payload: 1,297 checks, 0 failures.
- **W10 re-gate on the shared core**: 3,891 checks, 0 failures
  (`logs/java/w10-regate.log`).
- **Unknown fields**: 22 vectors, 66 checks, 0 failures (`logs/java/unknown.log`).
  protobuf-java retains unknown fields and re-emits them; the core drops them.
- **P2.5 has two valid encodings and the slice produces both**: the facade arms
  write the canonical 19,632 B, protobuf-java writes 19,712 B (+2 B per emptied map
  value), the third Google runtime observed to do so after protobuf C++ and upb.
  The gate checks the cross product: each arm parses the other's bytes and
  re-encodes to its own form.
- **P7.1** is validated by decoding and re-encoding to a permutation of the same
  (tag, wire type, body) triples, as `design/SHAPES.md` asks.
- **Layout guard** (ABI v1 section 10): 380 facts agree between the core's export
  and the slice's table, and `gen/layout_break.sh` shows the check failing and
  naming the fact when one is perturbed (`logs/java/layout-guard.log`).
- **R5 boundary check**: every ABI entry point is an undefined import of the shim,
  resolved by the core at load (`logs/java/boundary.log`).
- **Not gated**: the pull arms `ffi-pull` and `ffi-pull-walk` appear in no
  conformance log (R-D5). **Arm R has not run the conformance corpus** (`corpus/`);
  it passes the schema manifest gate above only (R-E4).

## Crossing counts

From the counting core, `logs/java/counts.log`. Forward is a JNI call into the
core, reverse an upcall out of it.

| payload | encode fwd / rev (batched) | per element | decode fwd / rev (push) | per element |
|---|---|---|---|---|
| P1.2 (1,000 M1 rows) | 8 / 1 | 0.009 | 1 / 5 | 0.006 |
| P2.2 (500 M2 elements) | 2,511 / 2,501 | 10.024 | 1 / 3,501 | 7.004 |
| P2.3 | 629 / 626 | 10.040 | 1 / 876 | 7.016 |
| P2.4 | 403 / 401 | 10.050 | 1 / 561 | 7.025 |

- These match the rust slice's counts to the digit.
- Unbatched encode costs 22.0, 130.0 and 316.0 crossings per element on P2.2, P2.3
  and P2.4 (full table in the log).
- **Pull family**: reverse crossings are 0 on every payload in both deliveries.
  Forward is 2 for the walk delivery at any size (parse plus `ak_bdr_ptr`) and 1
  plus one per 32 KB chunk for the drain.
- **RPC**: the blocking call makes 2 forward and 0 reverse, the queue 3 forward and
  0 reverse (from the withdrawn `rpc.log`; not in any current committed log, so not established).
- **Decision 5, learned length-placeholder width**: zero prefix misses on every
  uniform payload from a warm context; on P2.4, built so a per-site width is wrong
  on every element, 80 misses (one per element) moving 979,181 of 979,465 bytes.
  Zero grow-callback invocations on any payload.

## Defects found

In the binding and its harness (all fixed, per `poc/java/STATE.md`):

- D1: `Dec.skip` used `pos += readLen()`, which caches `pos` before the varint is
  consumed, so an unknown length-delimited field skipped into its own body.
- D2: decode `apply` rebuilt a singular message child from the group, discarding
  what a run had attached (open decision 10). Encode stayed byte-identical while it
  was live; only the decode round trip saw it.
- D3: the protobuf-java baseline amortised the size pass over the loop (below).
- D4: the bench kept the parse alive with `Message.hashCode()`, a second full
  traversal no other arm paid; replaced by `System.identityHashCode`.
- D5: `RunDelta` warmup ran a fixed, very large encode count; it now warms to a
  fixed time.
- D6: the `ffi` encode arm called `encodedLength()`, one extra forward crossing per
  operation.
- D7: `takeBytes` paid two crossings and two copies where the incumbent pays one
  allocation and one copy.
- **Deadlock in the first RPC arm**: `GetPrimitiveArrayCritical` was held across
  the whole blocking call. The peer was a grpc-java server in the same process,
  which must allocate to answer, so a collection needed in that window waited on
  the critical section. It passed P2.2 and hung on the first small payload. The
  rule now in ABI v1 section 9: a host must not pin a managed array across an ABI
  call whose completion depends on another thread of that host. The pull parse is
  not affected, because it makes no upcall and needs no other host thread.

## Harness facts

- **An in-process server flipped a delta's sign.** The grid's transport delta
  `B - A` had one sign in six of six in-process runs and did not keep it with the
  server in its own process. In a difference the server term cancels, so the flip
  is not dilution; it is client and server contending for CPU and one heap in one
  JVM. In-process deltas are therefore not a conservative floor.
- **protobuf-java's wide-string encode has two JIT states.** Mechanism, from
  `-XX:+LogCompilation` (`logs/java/r9-mechanism.log`): C2 prunes the
  `StringUTF16.charAt` branch at both sites of protobuf-java's `encodeUtf8`
  (`inline_fail 'call site not reached'`) in the slow state and compiles it in the
  fast one. Runtime `uncommon_trap` counts are 9 (slow) against 11 (fast), which
  rules out deoptimisation. Which state a process lands in is probabilistic: a
  Latin-1 read before the first measurement reached the fast state 10 of 10 runs,
  unprompted about 1 run in 10. Seen on JDK 17, not on JDK 21. Nothing happens on
  ASCII content.
- **Instruments**: `-XX:+TraceDeoptimization` and `-Xlog:deoptimization` do not
  exist on a product build; `-XX:+LogCompilation` preserves the effect; **JFR
  erases it**.
- The `ffi` arm has no `charAt` site of its own (transcoding is in the core, ABI v1
  section 4) and was not moved by the probe. `ak.Utf8.encode` and `ak.Utf8.length`
  compile identically in all probe modes, never pruned.
- **A memoised size pass hid the incumbent's real per-call cost.** protobuf-java
  memoises `getSerializedSize()` on the instance, and both `toByteArray` and
  `writeTo` call it, so a loop over one message pays the size pass once while every
  other arm pays its own each time (D3, `logs/java/baseline.log`).
- **Per-call allocation dominated cell D**: its marshaller allocated a fresh large
  array per call where A materialises none and B and C reuse one. That produced an
  apparent non-additivity between grid halves which vanished once fixed.
- **Four cells in four JVMs cannot be paired**; one process with interleaved cells
  is the fix and is not built (R-C7).
- **A probe control was folded by C2** (J17, `logs/java/shim-probe.log`): a callee
  storing one value into one field k times compiled to a single store. Both sides
  now write k distinct values into k distinct fields.
- The grid's 4 MiB window on both sides is ArmoniK's intended configuration, not
  its shipped one: `packages/rust/armonik-transport` sets no window and
  `packages/csharp` cannot set one. From grpc-java 1.74.0 read with `javap`
  (`logs/java/flow-control.log`): one call sets both windows, pinning disables BDP,
  and the shipped default is 1 MiB. Nagle does not apply over a UDS.

## Packaging and levels

- The floor and the target must be different code: the target's string path reads
  `String.coder` and `String.value`, which do not exist on Java 8.
- A single jar therefore needs a multi-release jar or runtime dispatch; runtime
  dispatch is built (`ak.Str17`).
- The binding writes C structs through `sun.misc.Unsafe`, as protobuf-java does on
  the same paths. It is terminally deprecated from JDK 23, its memory-access
  methods warn at run time from JDK 24, and its replacement, FFM, is a JDK 22 API.
- `--release 8` on JDK 17 cannot build the floor (`ct.sym` does not carry
  `sun.misc.Unsafe`), so the floor is compiled by the JDK 8 compiler.
- Facade storage choices (`gen/javanames.py`): enum as `int` (so `status = 999`
  round-trips), packed fields as primitive arrays (protobuf-java uses
  `List<Long>`, so P6.1 compares two data models), maps as `TreeMap` (canonical
  order), explicit presence as a primitive plus a `has` flag.
- The virtual-thread probe (`logs/java/pinning.log`, JDK 21) shows the blocking
  entry point pinning a virtual thread's carrier and the completion-callback mode
  not pinning it. The queue can be drained from a virtual thread; whether it avoids
  the pinning is not measured (one drainer needs one carrier either way).

## What is not measured or not established

- Any performance result. All timings are container instrumentation.
- The grid has no committed raw runner log (R-C9).
- The pull arms are not in a gate log (R-D5); arm R has
  not run the corpus (R-E4).
- What moves arm R under the R9 probe is unattributed. The `String.format` probe
  mode that README R9 names no longer reproduces (0 of 13 runs against 4 of 4 when
  first recorded); the slice did not bisect the two code changes in between.
- The transcode pair (README 10.4): `ak.Utf8.encode` writes U+FFFD where
  protobuf-java writes `?` for an unpaired surrogate. Written down, reached by no
  vector.
- Concurrency: one thread everywhere; re-entrancy (instance buffers, a thread-local
  frame stack of depth 8 in the shim) is argued, not tested.
- Allocation and footprint: no column.
- Callback RPC delivery, streaming, metadata, deadlines, TLS, status codes: absent
  (owner position 3 covers the transport features).
- Whether a thread parked in a drain costs a collection nothing (ABI v1 section 9):
  no collection instrumented.
- FFM as a binding, and any FFM upcall.
- Oneof union alternative, depth past 4, decode recursion limit, size limits,
  `ak_init` lifecycle, rollback of a half-written field, malformed wire: specified,
  not exercised.
- Rust MSRV 1.88 for the core: not verified (built with rustc 1.94.1).
- `poc/java/STATE.md` contradicts itself on what exists (its "what is not measured"
  still says the RPC arm and the pull family are unbuilt), R-F1.

## Open review findings

From `design/FIX-PLAN.md` section 7. Unconfirmed until the java slice agent answers.

- R-B1: README outcome-2 bullet used a withdrawn Java RPC figure and an unsourced one (removed by WP2).
- R-C4: grids in other slices are in-process; Java's delta flipped sign when moved out.
- R-C7: Java two-process codec half below its own stated resolution; unpaired JVMs.
- R-C9: Java RPC grid has no raw runner log.
- R-C10: Java headline incumbent is the library's best path, not gRPC's marshaller (R14).
- R-C12: Java wide-content encode taken with protobuf-java in its slow JIT state.
- R-D5: Java pull arms not in any gate log.
- R-D9: JNI `GetPrimitiveArrayCritical` null check missing on the shim's encode path.
- R-E4: Java arm R never ran the corpus; five rule gaps.
- R-F1: Java `STATE.md` contradicts itself on what exists.
