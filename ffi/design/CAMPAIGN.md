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
   another**, never concurrently. Nothing else runs during a slice. **Amended by
   the owner 2026-10-01 (D15):** the reference machine's few, mostly idle Docker
   containers may stay running, confined to the `OS` set; the netfilter modules they
   load sit on the loopback TCP path and are recorded in every header.
2. **Frequency:** governor `performance`, turbo **off**, SMT state recorded (either
   is allowed, but one state for the whole campaign). **Reference machine**
   (owner, 2026-09-26): an Intel Core i9-7900X, 10 cores and 20 threads on one
   socket, 13.75 MB L3, base 3.30 GHz, Turbo Boost 2.0 up to 4.30 GHz and Turbo Boost
   Max 3.0 up to 4.50 GHz (Intel's specification page for the i9-7900X). "Turbo off"
   covers both boosts. Nothing in a harness depends on this model: the machine's
   parameters live in `ffi/campaign.machine`, which the owner edits for another
   machine.
3. **Isolation:** the CPUs used for measurement are isolated from the scheduler
   (`isolcpus`/`nohz_full` or a cpuset cgroup), and one CPU is left for the OS and
   the runner. The mechanism is recorded. **On the reference machine (2026-09-30):**
   no `isolcpus`; systemd `AllowedCPUs` on the system slices, IRQ affinity and the
   owner's user processes on the `OS` set, the client and the server pinned by
   `taskset`, and every server and client thread's affinity checked before and after
   each process (a mismatch aborts before timing). The machine's sleep is inhibited
   for the length of a run.
4. **CPU sets, three of them, disjoint, on one NUMA node, and no two sets sharing
   SMT siblings:**
   - `CLIENT`: the process that is measured (codec benchmarks and RPC clients);
   - `SERVER`: the RPC server process;
   - `OS`: everything else.
   The sets are parameters of the runner (`AK_CPU_CLIENT`, `AK_CPU_SERVER`), not
   constants in a harness. **Their sizes are fixed for the whole campaign**
   (owner, 2026-09-26, R-H34): 4 cores for `CLIENT` and 4 for `SERVER`, set in
   `ffi/campaign.machine` and checked by `ffi/campaign.sh`, because stacks size
   their thread pools from them. **Every log header records each stack's worker
   thread counts.
   **Amended by the owner 2026-09-29 (D8):** each set is 4 cores **with both SMT
   threads** (`CLIENT` 1-4,11-14, `SERVER` 5-8,15-18, `OS` 0,9,10,19 on the reference
   machine), and every pool is sized to 8 workers: the server, the host runtimes, the
   core runtime, and grpc-core (which sizes itself from `_SC_NPROCESSORS_CONF`, not
   from the affinity mask, so a slice states how it sizes it). **The pool size is 8,
   the number of threads in each set** (owner, 2026-10-03, D14; `AK_WORKERS` in
   `campaign.machine`). `campaign.sh` and `campaign.machine` encode this (2026-10-03).

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
   slice implements, excluding disputed rows. Amended by the owner 2026-09-26:
   - **content sets** (R-H26): Latin-1 and wide are required on **P1.2, P2.2 and
     P2.4** in every slice; more payloads are labelled extras;
   - **U-* rows** (R-H27): the accepted, non-disputed rows whose root is one of the
     shapes core's 7 ABI roots (92 rows as of 2026-09-26; the owner chose "the 92"), in
     all three directions (encode, decode, decode-read), through the **timed shapes
     core** in every slice; other rows and the corpus-schema core are labelled
     extras.
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
10. **Unknown fields, three modes** (amended 2026-09-25, owner): `core-ffi` and
    `host-gen` run in
    - **retain**: decision 11's mechanism, every position's options entry armed;
    - **drop**: the same build, every entry zero (dropped at run time);
    - **no-unknown**: a build with unknown-field support **compiled out** (the
      generator's `unknown="drop"` option: no `ak_unk_buf` slots in the decode groups,
      no options structs, no capture or re-emission code in the core or the binding,
      and **no unknown-field member in the facade objects**, owner 2026-09-26).
      This prices what proposing retention costs even when it is not used.
    The incumbent runs in its default mode, stated. The no-unknown build has its own
    committed crossing counts (req 19) and its own correctness gate (corpus with every
    unknown row written in the dropped form).
11. **A message is serialised once per iteration** from a fresh or reset object
    graph, so no per-instance memo is amortised (the protobuf-java lesson).
    **Encode variants, all measured** (owner, 2026-09-26, R-H29), as labelled rows,
    with graph construction outside the timed window:
    - **end state**: (i) the bytes in a reused buffer, no allocation; (ii) the form
      the arm's RPC path hands to its transport (for example a gRPC `ByteBuffer` for
      the incumbent, the core's buffer for `core-ffi`), stated per arm;
    - **input**: (i) one hot graph re-encoded; (ii) a pool of distinct graphs larger
      than the last-level cache, stated.

### 4.2 RPC grid (client pinned to `CLIENT`, server in its own process pinned to `SERVER`)

12. **Cells, defined once for every host:**

    | Cell | Codec | Transport |
    |---|---|---|
    | A | incumbent (production path) | host's gRPC stack |
    | B | incumbent | the core's transport |
    | C | core, through the C ABI | the core's transport |
    | D | core, through the C ABI | host's gRPC stack |
    | E | the generated host codec (`host-gen`) | the core's transport |
    | F | the generated host codec (`host-gen`) | host's gRPC stack |

    Cells C and D run in each unknown-field mode of req 10 (**retain**, **drop**,
    **no-unknown**), labelled `C-retain`, `C-drop`, `C-nounk` and likewise for D
    (owner, 2026-09-25); A and B run the incumbent in its default mode. Cells E and
    F (owner, 2026-09-26, R-H35) serve option 2 of README section 13 and run
    `host-gen` in each mode it has in the codec suite, labelled likewise.

13. **The server is a separate process** in every host, including Rust and C++, and
    returns **pre-serialised response bytes**, so its work is identical across cells.
    Where a host cannot pre-serialise, server serialisation is timed in full and
    reported, never partly subtracted. Amended by the owner 2026-09-26 (R-H33):
    **one server process and configuration per launch**, serving every cell of both
    builds, warmed by a stated number of calls from each client transport before
    round 1; **one channel per cell per benchmark process** (req 22a: a framework that forks
    per benchmark gives each fork its own channel), opened before the fork's warm-up.
    **Amended by the owner 2026-09-27: one server implementation for every slice, the
    Rust slice's** (tonic, `poc/rust`, its `rpc_server`), started by each slice's
    runner through one shared launcher, pinned to `SERVER`. The server's work, its
    warm-up (no JIT) and its decoding of the upload directions are then the same
    whatever the client language, so cells compare across languages; `shipped` and
    `pinned` (req 17) configure the client, and the server's configuration is the
    one Rust configuration, stated. Each slice's own server is removed.
14. **Directions:** (a) empty request, P2.2 response; (b) P2.2-sized request that
    the server decodes, empty response. **Directions (c) and (d) are required in
    every slice** (owner, 2026-09-27, amending D5: an upload and client streaming
    stress a code path that unary calls with small requests do not). They were
    built first in the Rust slice (2026-09-27):
    (c) a unary upload, request P5.3 or P5.4 (M5, 1 MB and 4 MB) that the server
    decodes, empty response; (d) the streamed upload, ArmoniK's
    `UploadResultData(stream ...)` shape: M5 messages carrying 2 MiB chunks (ids on
    the first only), 4 MiB and 16 MiB in total, through the core's client streaming
    (ABI-v1 section 9), the server checking the received byte count and digest. Both
    at 1 and 8 in flight. Each send path that exists runs beside its reference
    (the framed twins, ABI-v1 section 9). Since 2026-09-28 the framed path is the
    core's default: the framed twins (Bf, Cf, Ef) are the default rows and B, C, E
    the labelled reference rows, each harness setting the path explicitly on every
    core cell. Direction (a) is reported as `a`
    (decode only) and `a+read` (decode, then read every field) in every slice;
    cross-slice readings use `a+read` (owner, 2026-09-26, R-H36).
15. **Concurrency:** 1, 8 and 16 calls in flight.
16. **Delivery:** A, D and F use the host stack's **idiomatic** call (blocking or
    async, as its production code does), stated per slice (owner, 2026-09-26, R-H30).
    **Amended by the owner 2026-09-28: the cells on the core's transport (B, C, E and
    their framed twins) use the core delivery that is idiomatic for the host too**,
    unary and streaming alike, stated per slice: Rust, the callback delivery bridged to
    async with a oneshot channel; C++, the blocking delivery and the completion queue,
    both measured; the other slices state theirs (a callback completing the runtime's
    future or task, or the queue with one drainer, ABI-v1 section 9). The **blocking**
    delivery stays in every host as a labelled row, so the core cells keep one
    delivery that compares across hosts. Before this amendment B and C were fixed to
    the blocking delivery in every host, for that comparability and because it was the
    only delivery built at first.
17. **Transport configuration, both:** `shipped` (what `packages/<lang>`
    configures) and `pinned` (4 MiB stream and connection windows, adaptive
    windows off, Nagle off, stated). B and C follow the same switch as A and D.
    **The socket is a Unix domain socket in every slice** (owner, 2026-09-26,
    R-H28). `packages/java` configures no UDS channel, so Java's `shipped` is
    grpc-java's defaults over Netty epoll, stated.
    **Amended by the owner 2026-10-01 (D10): the transport is TCP over 127.0.0.1 in
    every slice**, Nagle off on every client and server socket, read back on the live
    sockets of each timed process (`getsockopt TCP_NODELAY`). Reason: over TCP the
    stacks do not keep their Unix-socket ordering (`findings/physical-probe.md`
    section 4), and TCP is closer to the target. Unix-socket figures are history.
    The shared server's TCP listener runs the pinned server configuration only
    (`poc/rust/SERVER.md`), so over TCP `shipped` and `pinned` differ on the client
    side; stated in every header.
18. **Every call is checked**: status OK and response length equal to the
    expected payload; one failure aborts the run and produces no figure.

### 4.3 Crossings and calibration

19. **Crossing counts** per payload and direction, from a counting build, are
    recorded for `core-ffi` (they are machine-independent and gate the run: a count
    that differs from the committed one stops it). Amended by the owner 2026-09-26
    (R-H31): the counts include **every exported entry point the timed loop calls,
    resets included**, with the reset's place stated; they cover the **RPC cells B,
    C, D and E per call**; and they cover **retain mode**, with the initial buffer
    sizes fixed: no pre-placed buffer. Amended by the owner 2026-09-26, for every
    slice: the counting build's `grow` is the same **geometric** grow the timed build
    uses (capacity at least the request, clamped to `INT32_MAX`; ABI-v1 decision 11
    rule 8), not an exact-size grow, so the counts are those of the code that is
    timed. A slice whose counting build still grows to the exact size requested
    switches it and regenerates its committed counts before its next timing.
20. **Crossing cost**: the Rust slice's crossing benchmark, run in every host,
    reporting **forward and reverse separately**, with `perf stat` cycles and
    instructions per iteration.

## 5. How samples are taken

21. **CPU time is process CPU, per round, everywhere** (amended by the owner
    2026-09-26, R-H25): `getrusage(RUSAGE_SELF)` or `CLOCK_PROCESS_CPUTIME_ID` of the
    measured process, for codec arms and RPC cells alike, so GC, JIT and helper
    threads count for every arm (the server is another process). The earlier thread
    clocks are no longer the campaign's CPU figure.
    `Process.TotalProcessorTime` and any counter coarser than 1 microsecond are not
    allowed. **Wall time** is recorded beside CPU for RPC cells.
    **Amended 2026-10-01 for RPC cells over TCP:** on a kernel with
    `CONFIG_IRQ_TIME_ACCOUNTING=y`, softirq time is charged to no task, and on
    loopback the receive path runs in softirq on the sending CPU, so the process
    clock misses part of the client's cost, by an amount that depends on the number
    of writes (`findings/physical-probe.md` section 2). The RPC client's CPU figure is
    perf `task-clock` of the whole process (a counter the process opens itself, or
    `perf stat` around it), with the process clock recorded beside it and the
    softirq time on the `CLIENT` CPUs. A `perf stat` attached around a cell inflates
    it in proportion to its context switches, so it is used for categories, not for
    absolutes.
22. **Order of arms** (amended 2026-09-25, owner): on a machine that meets section 2,
    arms and cells may run in blocks, one after another, and need not be
    interleaved within a process. The order of arms is **rotated between launches**
    (launch 1: A, B, C; launch 2: B, C, A; ...), so a slow drift across a run does not
    fall on the same arm every time. Interleaving within a process remains allowed.
    Amended by the owner 2026-09-26: a slice randomises or interleaves the order to
    the extent its framework supports it; a framework that cannot is not a defect,
    and the order used is stated.
22a. **Benchmark engine** (owner, 2026-09-25): a slice may time through its
    ecosystem's standard benchmark framework, and **the codec suite of every slice
    uses one** (owner, 2026-09-25): **.NET BenchmarkDotNet, Java JMH, C++ Google
    Benchmark, Python pyperf, Rust criterion.** **Why** (owner, 2026-09-27): not to
    reinvent a benchmark framework, and not to rediscover the measurement problems
    these frameworks took years to solve. So a slice uses the framework's own
    mechanisms (warm-up, iteration and invocation control, process isolation, order,
    raw export) and writes custom code only where a requirement below needs something
    the framework does not offer, stating each such piece. **The RPC grid uses the same
    framework** (owner, 2026-09-27, replacing the earlier exemption, whose premise did
    not hold): the runner starts the one server process of the launch (req 13) before
    the framework; each benchmark connects in its setup, one channel per cell per
    benchmark process; an invocation issues the cell's k calls in flight and counts k
    operations; every call is checked (req 18) and a failed check fails the benchmark,
    the framework stops at the first failure where it can (BenchmarkDotNet
    `StopOnFirstError`, JMH `-foe true`, ...), and in every case the runner discards
    the launch's output when any benchmark failed, so an aborted run produces no
    figure. **Grouping** (owner, 2026-09-27): where the framework isolates one benchmark
    per process (JMH forks, pyperf workers, BenchmarkDotNet's default toolchain), the
    campaign runs the framework's native isolation, one process per (cell, combination);
    grouping several combinations into one process is allowed only for smoke and small
    exploration runs, as a runner switch, stated in the header.
    The framework's configuration must still satisfy requirements
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
    states (the JDK 17 branch-pruning hazard, README R9). **Every warm-up is a
    parameter of the runner** (owner, 2026-09-27): the codec framework's and the RPC
    grid's, each with a campaign default stated in the header and a much shorter value
    under `--smoke` and for small exploration runs. GC and JIT state between
    blocks follow each framework's defaults, stated per slice (owner, 2026-09-26,
    R-H32).
25. **Allocator state** is warmed identically for every arm before timing (the
    Python J26 lesson); GC settings are stated, and managed GC is on.
    **Amended by the owner 2026-09-30 (D9):** glibc's trim of the arena put the same
    unchanged code in two cost modes between processes (Rust A, about 8 against 11
    ms per 16 MiB call). Main figures run every cell of a native slice under
    `GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432`,
    stated in the header, and each comparison adds one default-allocator pass with
    the minor faults per call recorded beside every gap.
    **Amended by the owner 2026-10-03:** the roles are flipped and the rule covers
    **every slice whose process allocates through glibc** (the managed hosts load the
    core, whose buffers and transport use glibc malloc; CPython's large buffers too).
    The **main figures use the default allocator**, as production does; the pinned
    `GLIBC_TUNABLES` pass is the labelled diagnostic, with the minor faults per call
    recorded beside every gap, and the report shows where the two disagree.
    **Amended by the owner 2026-10-03 (mechanics):** one switch in every slice,
    `AK_CAMPAIGN_ALLOC=default|pinned` (default `default`); the shared server never
    gets the tunables. Every measured process, before any timing, (i) checks the mode
    against `GLIBC_TUNABLES` and reads it back once with one 16 MiB malloc against
    mallinfo2's mmapped-block count (default reads mmapped, pinned reads heap), keeping
    that block mapped so the probe does not move glibc's dynamic mmap threshold, and
    refuses to run on any disagreement; (ii) pre-grows the heap in both modes:
    allocate, touch and free a block of the run's largest payload (for direction d
    the whole upload, not one chunk) until one more round faults nothing, at most 8 rounds, else refuse. The readback, the rounds and
    the last round's faults go in the header; `alloc` and the minor faults over the
    measured span go on every sample. No suite sets mallopt of its own (Python's
    M_TOP_PAD is removed).

## 6. Correctness before timing

26. Before any timing, the runner executes the slice's correctness gate on the
    campaign machine: byte identity on every payload for every timed arm, the full
    corpus through every codec arm in every unknown-field mode of req 10, and the planted
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
    range. **Ratios are formed from per-launch medians** (owner, 2026-09-26), which
    is what a framework that forks per arm allows; running every arm and build in one
    process is allowed and not required. Nothing else: no significance claims, no verdict words. Interpretation is
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
