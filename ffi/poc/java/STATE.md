# java slice: state

**Read this first. Rewrite it at the end of every work unit.** It is the only thing that
survives the end of a session. It says what exists and what was checked; it does not say
what a binding should choose (`CLAUDE.md`: the decision is the owner's).

**Phase** (README 1.1): setup and design. **Every timing in this directory is container
instrumentation**: it shows a harness runs or exposes a harness defect, and it is not a
result. This file quotes no timing figure. What counts as a result here is byte identity,
crossing counts, floor builds, corpus passes, feasibility and defects found.

| | |
|---|---|
| **Status** | All arms built and gated on README 5.2's three arms (rd5-gate, 2026-09-24): 9 arms, 1,389 checks per arm-set, 0 failures. Arm R run against `ffi/corpus` for the first time (R-E4): 19 failing of 392 counted rows, listed below. FIX-PLAN WP4 items 6 (R-D5) and 10 (R-D9, Java part) done here; WP5 (Java backend onto the shared generator) not started. |
| **Levels** (owner decision D3) | **floor Java 8** (correctness gate only): `openjdk 1.8.0_504`, JDK 8 `javac`. **target JDK 17** (where the campaign's clock runs): `openjdk 17.0.20.1`. JDK 21 exists in the container for the two secondary probes only (virtual threads, FFM preview). |
| **Incumbent** | protobuf-java **3.25.5**, which is what `packages/java`'s pins resolve to (the pom declares 3.19.0, grpc-java 1.74.0 brings 3.25.5); protoc 3.19.0 generates the classes. R14's baseline path is `io.grpc.protobuf.lite.ProtoLiteUtils`' marshaller (`RunR14`). |
| **Core** | the shared crate `poc/codec/crates/ak-core` (R0), no copy here. The 2026-09-24 gate built it from `git archive 817174f ffi/poc/codec` in a scratch directory, unmodified, because the core was being changed concurrently; `gen/build.sh` builds it in place from `../codec`. |
| **Generator** | `gen/generate.py` still imports the shared `ir.py`, `rust_abi.py`, `cpp_layout.py` and the cpp slice's `cpp_header.py`, and **writes the core's `codec.rs` and `layout.rs` into `poc/codec`**. That is the pre-WP5 arrangement; running it in the live tree while another agent changes the core is unsafe, which is why the gate ran from a snapshot. `java_codec.py` (arm R) holds its own wire rules: an R-E9 defect WP5 removes. |
| **Blocked on** | nothing for correctness. WP5's shared plan layer for the Java port. |

## What exists

| | |
|---|---|
| `gen/` | the Java backends over the shared IR (facade, arm R codec, binding, JNI shim, pull binding, protobuf-java arms), `build.sh`, `gate.sh` (the correctness gate on arms a, b, c), `corpus_r.py` + `corpus_r.sh` (arm R against the corpus), and the older measurement scripts |
| `src/java/ak/` | hand-written runtime (`Enc`, `Dec`, `Utf8`, `Str17`, `Mem`, `Native`, ...) and harnesses: `RunConformance`, `RunUnknown`, `RunCounts`, `RunCorpusR`, `RunRuleGaps`, `Bench`, `RunDelta`, `RunR14`, `RunRpc` |
| `src/generated/java17/`, `src/generated/java8/` | two emitted trees, one per level, from one generator (README 5.1; Java has no preprocessor). `ak.floor` carries the Java 8 binding inside the target build, so arm b can be paired in one process |
| `native/generated/shim.c` | the JNI half: per-message encode and decode entry points, loop trampolines, the pull-family parse entries, the section 5 exception guard |
| `native/rpc.c` | ABI v1 section 9 from the JVM: blocking call, completion-queue delivery, client options |
| `native/test/nullpin.*` | R-D9 fault injection (fake `JNIEnv`, `-Wl,--wrap` on the core entry) |
| `probe/` | crossing-price, FFM and JNI-accessor probes |

### Arms (every one is in the gate log `rd5-gate.log`)

| arm | what it is |
|---|---|
| `R` | the generated pure-Java codec (README R3's no-boundary control) |
| `ffi`, `ffi-nobatch`, `ffi-zeroed`, `ffi-nobatch-zeroed` | the C ABI through JNI, push family, batching and decision 9's fill as separate registrations |
| `ffi-pull`, `ffi-pull-walk` | ABI v1 7.1's pull family, drained and walked. Decode only |
| `ffi-borrow` | decision 13's borrowed facade. Decode only |
| `pbj` | protobuf-java |
| RPC arm (`RunRpc`, `native/rpc.c`) | section 9 end to end: grpc-java, core blocking, core completion queue, and a four-cell codec x transport grid. **Built and run; not in the correctness gate**, which covers the codec only |

## Correctness (results)

**The gate: `logs/java/rd5-gate.log`**, verbose per-arm logs `rd5-conformance-arm-{a,b,c}.log`.

- **Arms a (java17 on JDK 17), b (java8 on JDK 17), c (java8 on JDK 8): 1,389 checks each,
  0 failures.** All 16 payloads, all three content sets (ASCII against the manifest, Latin-1
  and above-U+00FF by cross-arm agreement and round trip). The log lists every arm with its
  row count, so an arm absent from a run is visible: the five encode-capable arms take part
  in 319 rows each, the three decode-only arms (`ffi-pull`, `ffi-pull-walk`, `ffi-borrow`)
  in 94, and every arm has 46 round trips ok.
- **P2.5's two encodings**: the facade arms write the canonical 19,632 B, protobuf-java the
  both-fields-written 19,712 B; the gate checks the cross product (each arm parses the
  other's bytes and re-encodes to its own form).
- **P7.1** is validated by permutation of (tag, wire type, body) triples, per SHAPES.md.
- **Unknown-field vectors** (`RunUnknown`): 22 vectors, 66 checks, 0 failures, in the same
  gate run. protobuf-java retains and re-emits unknown fields; the core and arm R drop them.
- **The pull family is running, not falling back**: the counting core in the same gate run
  shows 0 reverse crossings for both deliveries on every payload (push makes 3,501 on P2.2).
- **Layout guard** (`layout-guard.log`): ABI v1 section 10's comparison seen failing and
  naming the fact when a layout fact is perturbed. **Boundary** (`boundary.log`): every
  entry point an undefined import of the shim, resolved by the core.

**The corpus against arm R: `logs/java/re4-corpus-armR.log`** (R-E4, confirmation only;
WP5 fixes). Scope: the 19 messages arm R's codec has, each declared identically in
`corpus.proto` (rule 0, checked per root); 395 of 691 rows. Same result on the target and on
the floor.

| | count |
|---|---|
| accept rows | 330: C1 parsed 330, C2 projected equal 328 of 328 with a projection, C3 accepted form 329 |
| reject rows | 62: refused 44, each with its error in the log |
| disputed, excluded | 3: `U-map-entry` (arm R produces the pure-python reading), `X-tag-zero-Empty` and `X-tag-zero-nested-Empty` (arm R accepts, as upb does) |
| timeouts (5 s per row) | 0; every `X-lenwrap-*` row in scope is refused with `length past end of buffer` |
| **failing** | **19** |

The failing rows:

- **`X-tag-zero-<Root>`, 18 rows** (every in-scope root): arm R accepts field number 0.
  `Dec.readTag` returns 0 at end of buffer and the decode loop's `default:` skips a 0 key
  instead of refusing it.
- **`E-map-entry-empty`, C3**: a map entry with key `""` and value `""`. Arm R writes the
  key and omits the empty value (8 B), which is neither accepted form (both written, 10 B;
  entry body empty, 6 B).

The review's other three R-E4 gaps are **not reachable by any corpus row in arm R's
scope**, so they were checked outside it (`RunRuleGaps`, same log, protobuf C++ asked the
same question):

- **merge semantics, confirmed by running**: `ResultRaw.created_at` twice
  (`{seconds:1}` then `{nanos:2}`): arm R keeps `{nanos:2}`, protobuf C++ reads
  `{seconds:1, nanos:2}`. Same on a oneof message member (`Probe.as_stamp`).
- **unknown oneof case, confirmed by running**: `Probe` with `body_case = 99` encodes
  without a refusal and writes only the other fields (`0a0178`).
- **the int length check (E6), run**: `0a ffffffff07` on `ListResultsResponse` passes
  `readLen`'s wrapping check and is refused later (`submessage body not fully consumed`).
- **`-0.0`, confirmed by reading only**: `java_codec.py` emits `x != 0.0` as the presence
  test of a singular implicit-presence double, which drops `-0.0`. `shapes.proto` has no such
  field (its one double is repeated, and the `S-mzero-*` rows on it pass), so no generated arm
  R code contains the defect today and no in-scope row can reach it.

Arm R has not been run on the 296 out-of-scope rows (WireZoo, Surrogate, the chunking and
nesting roots): its generator has no such messages. **The ffi arm has not been run against
the corpus at all** (see next step).

## Crossing counts (results) -- `logs/java/rd5-counts.log`

Re-run on the counting core from the 817174f snapshot; every row is identical to the older
`counts.log`.

| payload | encode fwd / rev | per element | decode (push) fwd / rev | per element |
|---|---|---|---|---|
| P1.2 (1,000 M1 rows) | 8 / 1 | 0.009 | 1 / 5 | 0.006 |
| P2.2 (500 M2 elements) | 2,511 / 2,501 | 10.024 | 1 / 3,501 | 7.004 |
| P2.3 | 629 / 626 | 10.040 | 1 / 876 | 7.016 |
| P2.4 | 403 / 401 | 10.050 | 1 / 561 | 7.025 |

Unbatched, P2.2, P2.3 and P2.4 encode at 22.0, 130.0 and 316.0 crossings per element. The
pull family: reverse 0 on every payload; forward 2 for the walk delivery at any size, 1 plus
one per 32 KB chunk for the drain. Decision 5's learned width: 0 prefix moves on every
uniform payload, 80 on P2.4 (one per element, by construction), 0 grow callbacks anywhere.
RPC (`rpc.log`, counted): blocking call 2 forward / 0 reverse, queue delivery 3 forward / 0
reverse, per call, none per field.

## Feasibility findings and defects found

- **The Java 8 floor build was broken from 5241ced to this work unit.** `RunRpc.java` used
  `ProcessHandle` (Java 9) and is in the floor's compilation, so `gen/build.sh` stopped at
  the JDK 8 `javac` step (`set -e`). Fixed with a Java 8 API; arms b and c build and pass
  again (rd5-gate). No floor log was committed while it was broken: `floor.log` and
  `w10-regate.log` predate 5241ced.
- **R-D9, Java part, fixed**: the bulk-bytes encode entry did not NULL-check
  `GetPrimitiveArrayCritical`; the parse entries did. Fixed in the emitter
  (`gen/java_jni.py`), regenerated. Swept: the two `uri` pins in `native/rpc.c` were also
  unchecked and are fixed. `rd9-jni-null.log` shows the old shim entering the core with
  `(NULL, 65536)` under fault injection and the new one returning `AK_ERR_HOST` without
  entering it, frame stack balanced over 17 calls.
- **R-D2 swept into `native/rpc.c`**: it hand-declared `ak_client_opts` (6 fields, correct);
  it now includes the generated `ak_abi.h`, whose regeneration picked up the rendered struct
  with its size and offset asserts, and compiles against it.
- **Packaging (README 5.1.3)**: the floor and the target are different code because the
  target reads `String.coder`/`String.value`, which Java 8 lacks; `ak.Str17` shows runtime
  dispatch working in one tree. Both levels use `sun.misc.Unsafe`, which is terminally
  deprecated from JDK 23 with FFM (JDK 22) as its successor, above the target. `--release 8`
  on JDK 17 cannot build the floor (`ct.sym` has no `sun.misc.Unsafe`), so the floor is
  compiled by JDK 8's `javac`.
- **Critical sections**: holding `GetPrimitiveArrayCritical` across a blocking RPC call
  whose completion needs another Java thread (the in-process grpc-java server) deadlocked;
  the RPC arm copies instead. The pull parse and bulk-bytes pins are safe because the call
  makes no upcall and completes on its own.
- **Virtual threads** (`pinning.log`, JDK 21): blocking in a native frame pins the carrier
  and parking on a future does not; observed as elapsed time scaling with the pinned
  prediction at 1, 2 and 4 carriers. A container observation of a qualitative behaviour.
- **R9's JIT effect** (`r9-mechanism.log`): C2 prunes `StringUTF16.charAt` from
  protobuf-java's encoder as "call site not reached" in some runs and not others
  (`-XX:+LogCompilation`), bimodally; the core's transcoder is not in the JIT's reach. A
  harness hazard for the campaign on wide content, recorded with its mechanism.

## Instrumentation (container timings; not results)

These logs exist and show the harnesses run. **No figure from them is quoted here or is to
be quoted as a result.** Known harness issues the campaign must not repeat:

- the R14 baseline (gRPC's marshaller) is a separate harness (`RunR14`); the main `Bench`
  arms use `toByteArray` / `parseFrom` (R-C10);
- the RPC grid ran four cells in four JVMs, unpaired, and `rpc.log` is a curated summary
  with **no raw runner output committed** (R-C7, R-C9); its in-process rows had client and
  server contending in one JVM (R-C4);
- wide-content encode timings are subject to R9's bimodal JIT state (R-C12);
- none of these harnesses has been checked against `design/CAMPAIGN.md` (WP3).

Timing logs: `encode.log`, `encode-take-fix.log`, `decode.log`, `decode-pull.log`,
`delta.log`, `drift.log`, `floor.log` (its correctness block is a result, its timings are
not), `contentsets.log`, `deopt.log`, `r9-mechanism.log`, `crossing.log`,
`calibration-r13.log`, `baseline.log`, `ffm.log`, `shim-probe.log`, `pinning.log`,
`rpc.log`, `r14.log`, `r14-summary.md`, `w10-regate.log` (correctness plus a drift check).

## Open defects

| # | what | status |
|---|---|---|
| E1 | arm R accepts field number 0 (18 corpus rows) | open, WP5 (Java backend on the shared plan) |
| E2 | arm R's empty map entry form is not an accepted encoding (`E-map-entry-empty`) | open, WP5 |
| E3 | arm R replaces a repeated singular / oneof message field instead of merging | open, WP5 |
| E4 | arm R encodes an undeclared oneof case without refusing | open, WP5 |
| E5 | `java_codec.py`'s implicit-double presence test drops `-0.0` (latent: no such field in shapes) | open, WP5 |
| E6 | `Dec.readLen` checks `pos + n > limit` in `int`, the wrapping form R-D1 names. The corpus's `X-lenwrap-*` rows are refused by `n < 0` before it matters; a length of 2^31 - 1 does wrap the check and is refused only later, by the submessage-consumed check (`RunRuleGaps`, `re4-corpus-armR.log`). No out-of-bounds read is possible on the JVM | open, WP5 |
| E7 | `RunConformance`'s comment says `ak.BorrowArm` is absent from the floor build; it is present and gated on arms b and c | cosmetic, open |
| E8 | `gen/generate.py` writes into `poc/codec` | open, WP5 removes it |
| D1-D7 | earlier slice defects (`Dec.skip`, decode `apply` order, three baseline handicaps, warmup, redundant crossings) | fixed; see JOURNAL J2-J16 |

## What is not measured or not run

- **The corpus against the ffi arm** (the core through JNI), the pull arms and the borrowed
  arm. Only arm R has run it.
- **Corpus C5 (produce)** and the **chunking class**, which needs the ffi arm at more than
  one chunk; the transcode encode half (`T-enc-*`, `produce` names java) is not run, so
  `ak.Utf8`'s U+FFFD against protobuf-java's `?` is written down and not exercised.
- **The RPC arm under the correctness gate**: no committed log gates its response bytes
  against the codec gate's.
- **Concurrency** (ABI v1 obligation 12.5): one thread everywhere; the shim's thread-local
  frame stack is argued, not tested.
- **Allocation and footprint**, **depth past four levels**, **the decode recursion limit**
  (the corpus's `X-depth-*` rows are out of arm R's scope), **message size limits**,
  **`ak_init` and the lifecycle**, **the codec's rollback of a half-written field**.
- **FFM as a binding**: only a downcall probe exists, on JDK 21 preview; FFM is a JDK 22 API,
  above the target, and a secondary arm per README 5.
- **The C-shim arm of README 9.1**: not built; only its JNI accessor prices were probed.
- **Rust MSRV 1.88** for the core: not verified here (rustc 1.94.1).

## Next step

In FIX-PLAN order, as far as this slice is concerned:

1. **WP5 step 3, the Java backend**, once the shared plan layer exists: port arm R and the
   JNI binding to render plans, retire `java_codec.py`'s own rules and `generate.py`'s writes
   into `poc/codec`, then re-run `gen/gate.sh` and `gen/corpus_r.sh` (E1 to E6 should close).
2. **Run the corpus against the ffi, pull and borrow arms** with a driver shaped like
   `corpus_r.py` (the ffi arm's roots are the seven with public entry points), including the
   chunking class at more than one chunk.
3. **WP3**: conform `Bench`, `RunR14` and `RunRpc` to `design/CAMPAIGN.md` and commit raw
   runner output for the RPC harness.

## Requests to the aggregating session

Facts for the documents; this slice writes none of them.

1. The shared C header (`ak_abi.h`, rendered by `cpp_header.py`) declares no pull-family
   entry points (`ak_parse_*`, `ak_bdr_*`); `shim.c` declares them locally, transcribed from
   `ak-abi/src/lib.rs`. Nor does it declare section 9 beyond `ak_client_opts`; `rpc.c`
   declares the rest by hand (R-G5's Java instance).
2. `ak_bdr_count_forward`'s doc comment tells a host to call it after a drain, but
   `ak_bdr_drain` is itself an entry point that bumps `forward`, so a host following the
   comment counts every chunk twice.
3. The corpus has no vector for a repeated singular message field or a repeated oneof
   message member (merge semantics), and none for an implicit-presence double outside
   `WireZoo`; E3 and E5 were found outside it.
4. The floor build break (5241ced) went unnoticed because nothing ran the floor after an RPC
   harness change; `gen/gate.sh` now builds and runs all three arms in one command.

## Log index

**Results** (correctness, counts, feasibility):

| Log | What it establishes |
|---|---|
| `rd5-gate.log` | the gate on arms a, b, c with every arm listed (pull arms included), 1,389 checks each, 0 failures; unknown-field vectors 66/0; pull reverse crossings 0. Before/after for R-D5 in its header |
| `rd5-conformance-arm-{a,b,c}.log` | the verbose per-row gate output behind it |
| `rd5-counts.log` | crossing counts on the 817174f core, identical to `counts.log` |
| `rd9-jni-null.log` | R-D9: emitter sweep, generated diff, fault injection before (core entered with NULL) and after (refused) |
| `re4-corpus-armR.log` | R-E4: arm R against the corpus on target and floor, 19 failing rows by id, every refusal's error, the three rule gaps outside the corpus's reach |
| `counts.log` | the earlier count run (same figures) |
| `unknown.log` | 22 unknown-field vectors, earlier run |
| `conformance.log` | the earlier gate (437 checks, seven arms, before the pull arms existed); superseded by `rd5-gate.log` |
| `boundary.log`, `layout-guard.log` | R5's boundary check; section 10's guard seen failing |
| `flow-control.log` | grpc-java 1.74.0's window settings read with `javap` |

**Instrumentation** (container timings; see the section above): `encode.log`,
`encode-take-fix.log`, `decode.log`, `decode-pull.log`, `delta.log`, `drift.log`,
`floor.log`, `contentsets.log`, `deopt.log`, `r9-mechanism.log`, `crossing.log`,
`calibration-r13.log`, `baseline.log`, `ffm.log`, `shim-probe.log`, `pinning.log`,
`rpc.log`, `r14.log`, `r14-summary.md`, `w10-regate.log`.
