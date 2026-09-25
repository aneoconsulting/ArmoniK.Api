# The measurement campaign: the contract every harness meets (W11)

Status: **draft, 2026-09-25**, amended the same day after the java slice's first conformance report (req 7 row list, req 29 path). Written by the aggregating session from
`design/FIX-PLAN.md` WP3 and the owner's decisions recorded there. It is the only
definition of how performance is measured in this branch. A slice harness is
**campaign-ready** when it meets every numbered requirement below and a smoke run
in its container shows it executes (section 9).

## 1. Purpose and limits

- The campaign (W13) produces every performance figure the report uses. Nothing
  measured before it is a result (README section 1.1).
- It measures; it does not conclude. Slices produce raw samples and the summaries
  defined in section 8. No slice writes a verdict, a recommendation or a ranking.
- It is run by the owner on one physical machine. This document is written so the
  owner can run it without anyone's session context.

## 2. Machine

1. **One physical machine, bare metal, no other tenant.** Slices run **one after
   another**, never concurrently. Nothing else runs during a slice.
2. **Frequency:** governor `performance`, turbo **off**, SMT state recorded (either
   is allowed, but one state for the whole campaign).
3. **Isolation:** the CPUs used for measurement are isolated from the scheduler
   (`isolcpus`/`nohz_full` or a cpuset cgroup), and one CPU is left for the OS and
   the runner. The mechanism is recorded.
4. **CPU sets, three of them, disjoint, on one NUMA node, and no two sets sharing
   SMT siblings:**
   - `CLIENT`: the process that is measured (codec benchmarks and RPC clients);
   - `SERVER`: the RPC server process;
   - `OS`: everything else.
   The sets are parameters of the runner (`AK_CPU_CLIENT`, `AK_CPU_SERVER`), not
   constants in a harness. Suggested: 4 CPUs for `CLIENT`, 4 for `SERVER`.

## 3. Runtimes and versions (owner's levels)

| Slice | Floor (correctness only) | Target (timed) | Incumbent and its versions |
|---|---|---|---|
| Rust | MSRV 1.88 | same | prost and tonic as pinned in `poc/rust`, stated |
| C++ | C++11 | C++17 | protobuf C++ and grpc++ at the version ArmoniK builds (gRPC `v1.54.0`, `packages/cpp/tools/Dockerfile.worker`) **and** at a current one, both stated |
| C# | net6.0 and .NET Framework 4.8 (Windows) | net8.0 | Google.Protobuf 3.32.0, Grpc.Net.Client 2.71.0 (`packages/csharp`) |
| Java | 8 | 17 | protobuf-java as resolved by `packages/java` (3.25.5), grpc-java 1.74.0 |
| Python | 3.7 | CPython 3.12 (Ubuntu 24.04) | protobuf and grpcio inside `packages/python/pyproject.toml`'s ranges, with wheels for 3.12, stated |

5. Floors are gated for correctness (corpus and byte identity) on the campaign
   machine too, but produce no timing. .NET Framework 4.8 needs a Windows machine;
   if none is available the campaign records that the floor was not run there.
6. Build flags are fixed per slice and printed in every log: optimisation level,
   LTO, static or shared linkage of the core, the core's cargo features (`init-guard`
   is **on**, as in every gate), JIT and GC settings for managed hosts.

## 4. What is measured

### 4.1 Codec benchmarks (one process, pinned to `CLIENT`, no server)

7. **Payloads:** all 16 of `design/SHAPES.md`, the three content sets (ASCII,
   Latin-1, wide) where SHAPES.md defines them, and every corpus row of class
   `unknown` (the `U-*` rows of `corpus/generated/manifest.json`) whose root the
   slice implements, excluding disputed rows.
8. **Arms, per host:**
   - `incumbent-prod`: the production path (R14), i.e. what gRPC's generated
     marshaller calls in that language: grpc++ `SerializationTraits`, grpc-java's
     marshaller, Grpc.Net's marshaller over `ReadOnlySequence`, grpcio's
     `SerializeToString`/`FromString` from the stub, tonic's codec for prost;
   - `incumbent-best`: the library's fastest entry point, as a labelled second
     row;
   - `core-ffi`: the generated binding through the C ABI (push decode; pull as a
     labelled extra arm where the slice has it);
   - `host-gen`: the codec generated into the host language by the same
     generator (the no-boundary control; option 2 of README section 13);
   - Rust only: `core-native` and `armonik` (`packages/rust`).
9. **Directions:** encode, and decode reported **twice**: the bare call, and decode
   followed by reading every field. Any comparison with upb's lazy `FromString`
   names which of the two it is.
10. **Unknown fields:** `core-ffi` and `host-gen` run in **drop** and **retain**
    (ABI v1 decision 11's mechanism; retain means every position's options entry
    is armed), the incumbent in its default mode, stated.
11. **A message is serialised once per iteration** from a fresh or reset object
    graph, so no per-instance memo is amortised (the protobuf-java lesson).

### 4.2 RPC grid (client pinned to `CLIENT`, server in its own process pinned to `SERVER`)

12. **Cells, defined once for every host:**

    | Cell | Codec | Transport |
    |---|---|---|
    | A | incumbent (production path) | host's gRPC stack |
    | B | incumbent | the core's transport |
    | C | core, through the C ABI | the core's transport |
    | D | core, through the C ABI | host's gRPC stack |

13. **The server is a separate process** in every host, including Rust and C++, and
    returns **pre-serialised response bytes**, so its work is identical across cells.
    Where a host cannot pre-serialise, server serialisation is timed in full and
    reported, never partly subtracted.
14. **Directions:** (a) empty request, P2.2 response; (b) P2.2-sized request that
    the server decodes, empty response. A streamed upload in 2 MiB chunks is
    optional and scheduled last (FIX-PLAN D5).
15. **Concurrency:** 1, 8 and 16 calls in flight.
16. **Delivery:** B and C use the core's **blocking** delivery in every host; the
    callback and queue deliveries are labelled extra rows where a slice has them.
17. **Transport configuration, both:** `shipped` (what `packages/<lang>`
    configures) and `pinned` (4 MiB stream and connection windows, adaptive
    windows off, Nagle off, stated). B and C follow the same switch as A and D.
18. **Every call is checked**: status OK and response length equal to the
    expected payload; one failure aborts the run and produces no figure.

### 4.3 Crossings and calibration

19. **Crossing counts** per payload and direction, from a counting build, are
    recorded for `core-ffi` (they are machine-independent and gate the run: a count
    that differs from the committed one stops it).
20. **Crossing cost**: the Rust slice's crossing benchmark, run in every host,
    reporting **forward and reverse separately**, with `perf stat` cycles and
    instructions per iteration.

## 5. How samples are taken

21. **CPU time** of the measured work only: `clock_gettime(CLOCK_THREAD_CPUTIME_ID)`
    or `getrusage(RUSAGE_THREAD)` per measuring thread, or `getrusage(RUSAGE_SELF)`
    of the client process for RPC cells (the server is another process).
    `Process.TotalProcessorTime` and any counter coarser than 1 microsecond are not
    allowed. **Wall time** is recorded beside CPU for RPC cells.
22. **Order of arms** (amended 2026-09-25, owner): on a machine that meets section 2,
    arms and cells may run in blocks, one after another, and need not be
    interleaved within a process. The order of arms is **rotated between launches**
    (launch 1: A, B, C; launch 2: B, C, A; ...), so a slow drift across a run does not
    fall on the same arm every time. Interleaving within a process remains allowed.
22a. **Benchmark engine** (owner, 2026-09-25): a slice may time through its
    ecosystem's standard benchmark framework, and **the codec suite of every slice
    uses one** (owner, 2026-09-25): **.NET BenchmarkDotNet, Java JMH, C++ Google
    Benchmark, Python pyperf, Rust criterion.** Suites the framework cannot express
    without breaking requirement 18 (abort on any failed call) or requirement 13
    (separate server process), in practice the RPC grid, may stay on the slice's
    runner, stated in the checklist. The framework's configuration must still satisfy requirements
    21, 23, 24, 27 and 28: every raw measurement is exported (the framework's
    outlier handling may produce its own summary, but no raw measurement is
    dropped from the committed output), the JIT tier and warm-up it used are
    recorded, the process is pinned to `CLIENT`, and the output is converted to the
    JSON lines of section 7. Where the framework measures wall time only, that is
    stated, and CPU time is added through a diagnoser or column where it can be.
23. **Repeats:** at least **5 rounds per process** and **3 separate process
    launches** per slice and suite. Every round's value is committed, not only a
    summary.
24. **Warm-up** is stated per host and identical for every arm: a fixed number of
    iterations before round 1 (managed hosts: enough to reach the optimising JIT
    tier, recorded). protobuf-java's content-set rows run in both string-coder
    states (the JDK 17 branch-pruning hazard, README R9).
25. **Allocator state** is warmed identically for every arm before timing (the
    Python J26 lesson); GC settings are stated, and managed GC is on.

## 6. Correctness before timing

26. Before any timing, the runner executes the slice's correctness gate on the
    campaign machine: byte identity on every payload for every timed arm, the full
    corpus through every codec arm in both unknown-field modes, and the planted
    controls. A failed gate stops the slice; no figure is produced.

## 7. Logs

27. **Header, every log:** commit hash (a dirty tree is refused), machine (CPU
    model, cores, SMT, governor, turbo, kernel, isolation mechanism, the three CPU
    sets), runtime and incumbent versions, build flags, core features, transport
    configuration, warm-up and repeat counts.
28. **Body:** one JSON object per sample, one per line:
    `{"slice","suite","arm","cell","payload","content","dir","unknown_mode","transport","inflight","launch","round","cpu_ns","wall_ns","iters"}`,
    fields not applicable left out. Raw runner output is committed; hand-written
    summaries are not a substitute.
29. Logs go to `ffi/logs/<lang>/campaign/`, one file per suite and launch (inside
    the slice's own log directory, per `CLAUDE.md` ownership).

## 8. Summaries a slice may produce

30. Per (arm or cell, payload, direction, mode): the median and the minimum and
    maximum over all rounds of all launches, for CPU and wall; and the **per-round
    ratio** to `incumbent-prod` (codec) or to cell A (RPC), with its median and
    range. Nothing else: no significance claims, no verdict words. Interpretation is
    the aggregating session's, and the decision is the owner's.

## 9. Runner and smoke run

31. **One runner per slice** with a common interface:
    `run_campaign.sh --suite codec|rpc|calib|gate --out <dir>` reading
    `AK_CPU_CLIENT`, `AK_CPU_SERVER` from the environment, and a top-level
    `ffi/campaign.sh` that runs the slices one after another, gate first.
32. **Smoke run (container, before the campaign):** each slice runs its runner with
    1 launch, 1 round and a reduced iteration count, commits the log with its
    figures **stripped or marked instrumentation**, and records in `STATE.md` that
    its harness is campaign-ready, listing any requirement it cannot meet and why.

## 10. Checklist a slice reports against

A slice's campaign-readiness report lists each requirement 1 to 32 as `met`,
`not applicable` (with the reason) or `not met` (with the reason and the owner
decision it needs).
