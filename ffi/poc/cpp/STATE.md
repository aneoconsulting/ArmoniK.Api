# cpp slice: state

**Read this first, and rewrite it at the end of every work unit.** It is the only thing
that survives the end of a session. It says what is true of the tree now. The history of
how it got here, including every refuted idea and every superseded figure, is in
`JOURNAL.md`.

**Phase.** The branch is in setup and design (README 1.1). No figure in this file or in
any log listed here is a performance result. Every timing ever taken by this slice is
**container instrumentation**: it shows that a harness runs, or it exposes a harness
defect. What this file reports as results are correctness outcomes and crossing counts.

| | |
|---|---|
| **Status** | 2026-09-29/30, on the PHYSICAL campaign machine (i9-7900X, NixOS, kernel 6.18.54; turbo off, 3.3 GHz locked, no isolation: taskset only), phase 1 of the physical probe: step 0 done and checked (`logs/cpp/opt/physical-probe/checks/`, at `f79e034d`), the probe driver `gen/physical_probe.sh` written and smoke-run (smoke not kept); main segment TIMED 2026-09-29 (`logs/cpp/opt/physical-probe/main/`, `--grpc-cpus 8`, 254 s of benchmark wall; physical-machine figures, not container instrumentation); var4 segment TIMED the same day (`logs/cpp/opt/physical-probe/var4/`, 4-worker server, core 4 workers, `--grpc-cpus 4`, 146 s). Later units on the same machine: step 4a, patches p1-p7, the Cf-q investigation, the stability campaign, UDS against TCP, the TCP inversion attribution (2026-10-01, `tcp-attrib/`; the harness now records softirq time, which the process clock misses on this kernel), and h2 PR #903 against p4 (2026-10-01, `h2-pr903/`). Before that: the optimisation unit (2026-09-28, container) complete, section "Optimisation unit" below |
| **WP12 (2026-10-02, container)** | WP12's done criterion for this slice: the full C++ gates run twice in this container at `1bdbaa07` (= `0f75213f` plus this unit's script commits: the core-swap call sites, inert when AK_CORE_SWAP is unset, deliv_checks' CPU sets from the environment, the WP12 driver; no C++ source, core, generator or CMake change), from a private worktree: **stock** and **h2-batch** each pass wp5_gate (C++17 target and floor, C++14, C++11, static), d11_asan, the campaign gate (Unix sockets), deliv_checks over TCP 127.0.0.1 and q_checks; 0 failures in every step on both. Section "WP12 gates" below; logs `logs/cpp/opt/wp12-gates/`. The Rust slice's gate step of the final-gate driver was not run here (the Rust agent runs it in parallel) |
| **Physical machine build** | `nix-shell gen/shell.nix` (the system's nixpkgs): g++ 15.3.0, cmake 4.1.6, protobuf 34.1 (C++ version 7.34.1), grpc++ 1.80.0 (the "current" incumbent of CAMPAIGN section 3; ArmoniK's v1.54.0 is not on this machine), abseil 20260107; rustc 1.95.0 (ambient); Google Benchmark v1.8.3 Release (gbench_release, now installed with `CMAKE_INSTALL_LIBDIR=lib`). Changes this build needed: `shapes_pb` at C++17 when protobuf is 22 or later, plus utf8_range and protobuf.pc's abseil libraries; rt.cpp's protobuf UTF-8 ceiling through `utf8_range::IsStructurallyValid` there; `Arena::Create` for `CreateMessage` (removed); every cargo invocation's `CARGO_BUILD_BUILD_DIR` = its `CARGO_TARGET_DIR` (the machine's `~/.cargo/config.toml` sets one shared `build.build-dir`); grpc++ channels set `GRPC_ARG_DEFAULT_AUTHORITY` = "localhost" (below). Only the C++17 targets were built here; the C++11 / C++14 floor targets include protobuf headers, which need C++17 from protobuf 22 on, and were not attempted |
| **Core** | the shared one at `ffi/poc/codec/crates/ak-core` (R0). CMake builds it with cargo, `init-guard` in every configuration. Full-build flavours: plain, `count`, `corpus`, `rpc`, `rpc,count`, and three planted cores (`pad-widths`, `global-widths`, both). No-unknown flavours: `--no-default-features` plus `init-guard` alone, `count`, `corpus` or `rpc`. Each flavour has its own target dir under `core-build/` |
| **Generator** | one generator (W14). `poc/codec/gen/plan.py` holds the rules. This slice's backend modules in `poc/codec/gen/` are `cpp_binding.py`, `cpp_native.py`, `cpp_facade.py`, `cpp_names.py` and `cpp_layout.py`, plus `c_abi.py`, which renders the C header for every slice. `gen/generate.py` is glue: it renders the targets from plans and imports no IR (the guard in `generate.py --check`) |
| **Floor / target** | C++11 floor, C++17 target, both builds. C++14 also builds and is gated (full build) |
| **Incumbent** | protobuf C++ 3.21.12 and grpc++ 1.51.1, apt's, the only versions in this container (2026-09-28: the container came up without them; reinstalled from apt, libprotobuf-dev 3.21.12-8.2ubuntu0.3, libgrpc++-dev 1.51.1-4.1build5, protobuf-compiler, protobuf-compiler-grpc). `packages/cpp` pins neither. The runner builds against gRPC v1.54.0 and a current version through AK_INCUMBENT_PREFIX, one run per prefix (section 3); neither prefix exists here (checklist row 3) |
| **Compiler** | g++ 13.3.0, `-O2 -g -DNDEBUG`; rustc 1.94.1 |

## Physical-machine probe (2026-09-29/30): step 0 and the driver

**Step 4a (2026-09-30): attribution of A, D, Cf, Cf-q** (`gen/profile_4a.sh`, `logs/cpp/opt/physical-probe/profile/tables.md`).
`campaign_rpc --profile N [--profile-cell L] [--perf-ctl CTL,ACK]` runs N batches of one cell on the benchmark's own
path with no Google Benchmark, perf enabled only around the loop, per-chunk CPU and wall, per-thread CPU by class
(schedstat) and getrusage. Phases (each under `flock /tmp/ak-physical-bench.lock`, own 8-worker server): `stat` (perf
stat, 3 rounds, cells alone and Cf/Cf-q with A and D open), `record` (perf record cycles, LBR stacks; kernel symbols
from the booted kernel's System.map at KASLR offset 0x6c00000, found by `gen/perf_attrib.py`), `strace` (syscalls between
the loop's markers, `gen/strace_window.py`), `plain` (the control: no perf attached). Tables: `gen/profile_tables.py`.
Measured facts there: perf stat attached adds about 0.7 ms per d/16MiB call to A and D and about 0.2 to Cf (the control
without perf: A 8.25, D 8.29, Cf 8.34, Cf-q 9.06 ms at k = 1); the core transport writes each 16 KiB DATA frame with its
own writev (about 1026 per 16 MiB call against about 46 sendmsg of about 350 KB for grpc++), and that kernel write path
is Cf's largest bucket over D (+1.76 ms at k = 1), offset by grpc-core's user time (-1.29) and futex (-0.45).

**Candidate core patches from C++** (`gen/patch_ab.sh OUT PATCH_TREE "KNOBS" [checks|measure|alloc]`, tables
`gen/patch_tables.py`): the patch applied in a private worktree (`<scratchpad>/wt-cpp`, patch removed after each use),
the slice built there (builds on 0,9,10,19); checks run the worktree's binaries (conformance, codec pre-check,
--semantics 1); timed runs use THIS tree's campaign_rpc with LD_LIBRARY_PATH on the worktree's campaign core, so the
two arms differ only in libak_core.so (paths and sha256 in runner.log); one-cell `--profile` processes, no perf, 3 per
cell and core, own 8-worker server, under the bench lock; `--profile` reports allocations of at least 1 MiB when
gen/allocprobe.c is preloaded (separate processes). p1-ring (ring size AK_SPARES, AK_SPARE_LOCK):
`logs/cpp/opt/physical-probe/patches/p1-ring/` (6 + lock) and `.../p1-ring/spares24/` (24 + lock, Cf and Cf-q only).

**Several core builds compared** (`gen/core_ab.sh OUT measure|default|strace ARM=CORE_DIR[:K=V,...] ...`, tables
`gen/core_ab_tables.py OUT ARM...`): every arm runs this tree's campaign_rpc with LD_LIBRARY_PATH on its own
libak_core.so (copies kept outside the tree, sha256 in runner.log), per-arm and per-cell knobs (AB_CELL_KNOBS), the
allocator setting of the main figures in AB_ENV and a `default` pass without it, one-cell `--profile` processes, no perf,
3 rounds, under the bench lock with an own 8-worker server. p1-p3 stack against p1-p4 (h2 coalescing):
`logs/cpp/opt/physical-probe/patches/p4-h2-coalesce/` (checks-ctl-s6, checks-ctl-s24, checks-h16-c1, checks-h16-c16:
608/0, 478/0, codec pre-check 0 failed, semantics 0 failed; measure, default, strace; tables.md).

**Deferred-encode cells (EXPERIMENT, Rust patch p5-deferred)**: `Cf-enc-<mode>` and `Cf-encp-<mode>` in
campaign_rpc, direction d only, framed, blocking delivery: each chunk is sent with `ak_call_send_deferred(h, enc,
def_encode, &job, last, wait)` (found with dlsym; a core without it refuses the cell with exit 2, checked), the callback
encoding the chunk with the core codec in the cell's mode into the context the core hands it; wait = 1 (enc) or 0
(encp; the jobs and the context stay untouched until the recv). No queue form: the patch has no queue variant of the
deferred send. `--semantics 1` adds 7 deferred cases when the core exports the entry; `--check-stream 1` routes every
d call to UploadStreamCheck (count and SHA-256 per call). Checks: `gen/deferred_checks.sh`. Measured:
`logs/cpp/opt/physical-probe/patches/p5-deferred/` (core p1-p2-p3-p5, crates.io h2; `gen/core_ab.sh` with AB_CELLS and
AB_D_ONLY).

**Zero-copy cell (EXPERIMENT, Rust patch p6-zero-copy)**: `Cf-zc-<mode>`, direction d only, framed, blocking:
each thread has its own zero-copy encode context (`ak_enc_set_zc(ctx, 64 KiB)`), the chunk is encoded into it and sent
with `ak_call_send_enc_zc(h, ctx, last, zc_release, &counter)`; the data stay in the process's payload (World::st);
before each call the thread checks that every message of its previous call was released (waits up to 1 s, else the
call check fails). Refused on a core without the two entries (checked). `--semantics 1` adds 5 zero-copy cases
(both payloads to the checking path with one release per message, release on cancel, release on a failed call, the
reference path refused without a release). `gen/core_ab.sh` gained a `perf` phase (perf stat with cache counters,
perf record) and `gen/core_ab_tables.py` a perf section. `gen/perf_attrib.py` (2026-09-30): clear_page_erms under
alloc_skb_with_frags is the socket write's page zeroing, not a page fault; the "page faults" bucket is now the fault
path only, and the 4a tables were regenerated with it (profile/record/*.attrib.json).

**Deferred zero-copy cells (EXPERIMENT, Rust patch p7-deferred-zc)**: `Cf-zcw-<mode>` (wait = 1) and `Cf-zcp-<mode>`
(wait = 0): `ak_call_send_deferred_zc(h, zec, def_encode, &job, last, wait, zc_release, &counter)` on the thread's
zero-copy context; release contract checked per call as for Cf-zc; refused without the entry. `--semantics 1` adds 7
cases (both waits x both payloads with one release per message, cancel after an encoded send, a failed call, a failing
encode and the reference path without release). `--profile` now also reports schedstat run-queue wait per thread class,
a batch trace of the blocking path (batch wall, per-call durations, dispatch and completion lags) and, with
AK_SERVER_PID, the server's threads' CPU and wait during the loop (`gen/wall_tables.py`).

**Cf-q investigation and the stability campaign (2026-09-30, machine confined: system slices and IRQs on
0,9,10,19; every header records the cgroup cpusets, IRQs reaching the measured CPUs and the non-kernel threads allowed
on them).** `logs/cpp/opt/physical-probe/cfq/`: checks of the q1-inline experiment (`q1-inline.patch`, AK_Q_INLINE:
a queue send the channel takes at once completes inline) and of AK_Q_SLOT_CTX (per-slot encode contexts for queue
cells); `ab/` (four arms, two runs), `perf/` (client and server perf stat and record, perf attached to the server with
-p), `strace/`, `strace-threads/` (per-thread strace -T), `workers1/`, `workers2/`, `workers-ab/` (the core runtime's
worker count 8 / 2 / 1 as interleaved arms, AK_WORKERS). `logs/cpp/opt/physical-probe/stability/`: `run1` (14 rounds; the
server was re-pinned to the OS set by the machine's confinement after round 10: tables for rounds 1-10 and 11-14) and
`run2` (19 rounds, 684 processes, 1206 s, the server's affinity checked before every process).

**Step 0: the send path, set explicitly.** The core's default send path is framed since
2026-09-28 (ABI-v1 section 9), so `core_client()` in `src/campaign_rpc.cpp` now calls
`ak_client_set_framed(cl, framed ? 1 : 0)` on EVERY core client (FIX-PLAN WP8 item 6): B, C-*,
E-* and their -q forms get 0 (the reference path, tonic's codec), Bf, Cf-*, Ef-* and their -q
forms get 1. Before it, the reference cells ran framed. The names are unchanged (C = reference,
Cf = framed, as the Rust grid). The pre-campaign `rpcbench`, `rpcflow` and `rpccounts` open
their clients without the call (never timed in the campaign; not built or run on this machine).

**Checked at `f79e034d`** (`gen/physical_checks.sh`, `logs/cpp/opt/physical-probe/checks/checks.log`,
CLIENT 1-4,11-14, SERVER 5-8,15-18, own serve.sh server):
- every binary loads its variant's core (path, sha256, unknown-field and rpc exports listed); the
  cores built with the per-variant build dir hash identically to those built before it;
- byte identity (conformance, C++17): full 608 checks, 0 failures; no-unknown 478, 0 failures;
- codec pre-check: full 120 groups / 4436 slots, no-unknown 120 / 2724, 0 failed each (the pull
  value gate included);
- `q-checks.log` (gen/q_checks.sh): `--semantics 1` passes on both builds and both send paths;
  the d-sha, d-count and c-len plants on queue cells abort with no sample; the queue-cell grid
  smoke passes (140 and 84 benchmarks); RPC counts 132 and 78 rows identical to
  `logs/cpp/rpc-counts.log` / `rpc-counts-nounk.log`;
- the grid smoke of EVERY cell (A-F, framed twins, queue and pull cells, every mode), every
  direction, k = 1 and 8, shipped and pinned, both builds: 366 and 218 samples per transport,
  every call checked and each process's pre-check passed;
- the send path is what the label says: allocations of at least 1 MiB per call
  (`gen/allocprobe.c`): the reference cells make one per request message more than their framed
  twins (C-retain d/16MiB 8.12 against Cf-retain 1.62; B 16.00 against Bf 8.00; C c/P5.4 1.12
  against Cf 0.00). The first run of the check used a criterion that did not allow for the
  framed encode ring's misses on 16 MiB and failed on two pairs
  (`checks-run1-criterion.log`); the criterion was restated (reference >= one per message,
  reference - framed >= half of that) and the same numbers pass.
- `authority.log`: grpc++ 1.80 against the shared tonic server failed EVERY call (RST_STREAM,
  PROTOCOL_ERROR): v1.80's `unix:` authority is the percent-encoded socket path
  (resolver_factory.h:70-71), where v1.51 and v1.54 send "localhost" (sockaddr_resolver.cc:151,
  170). campaign_rpc's grpc++ channels now set `GRPC_ARG_DEFAULT_AUTHORITY` = "localhost", what
  v1.54 (ArmoniK's) sends; with it cells A, D and F pass.

**What each stack runs on this machine** (printed by every client process: `cpu` and
`thread_classes*` in campaign_rpc's header and end lines; checked in the smoke):
- grpc-core v1.80 sizes itself from `sysconf(_SC_NPROCESSORS_CONF)` = 20, NOT from the affinity
  mask (src/core/util/linux/cpu.cc): 16 `event_engine` threads (Clamp(20, 4, 16),
  posix_engine.h), 1 `grpc_global_tim`, 1 `lifeguard`, whatever taskset sets. The harness has no
  setting for it. `gen/ncpus_shim.c` (LD_PRELOAD, `--grpc-cpus N`, off by default) makes sysconf
  report N: `event_engine` 8 with N = 8, 4 with N = 4 (and with N = 2: the clamp's minimum);
- the core's runtime: `--workers N` tokio workers (`tokio-rt-worker`), fixed, not from the mask;
- the caller threads: 8 (`campaign_rpc`, the k = 8 pool, plus the main thread);
- the shared server: `AK_SERVER_THREADS` tokio workers (read from its environment and its
  `/proc` task list by `gen/machine_facts.py`).

**The driver** (`gen/physical_probe.sh`, tables `gen/physical_probe.py`, header facts
`gen/machine_facts.py`): one segment per invocation against one server, transport pinned (the
pinned socket), cells in retain mode (as the Rust probes), directions c/P5.4 and d/4MiB,
d/16MiB, k = 1 and 8.
- `main`: server 8 workers, core 8 workers; 4 spread passes (A, D-retain, Cf-retain), then 3
  main passes (A, D, Cf, Cf-q, C, C-q); `var4`: server 4, core 4; 3 passes of all six cells.
- A pass is one client process; Google Benchmark interleaves the repetitions at random within
  it; the launch index (= pass number) rotates the registration order. 10 repetitions, fixed
  iterations per (payload, k) (`--iters`: P5.4 30 / 5, 4MiB 40 / 6, 16MiB 12 / 3 batches at k = 1
  / 8, about 100 ms of wall per repetition, 16 MiB k = 8 about 190 ms), warm-up 0.05 s (one
  repetition's worth).
- The spread (the "notable" threshold, the coordinator's definition): per pass and workload,
  the in-process gap median(cell) - median(A) of per-call CPU (and wall); spread = its range
  over the segment's spread passes, for D - A and Cf - A. The tables print every gap per pass
  and its range; absolute times, no ratio.
- Shared mode (`AK_SERVE_STATE` set): never starts, stops or warms the server; before every
  client process it checks the state file's pid is unchanged, the server's affinity equals
  AK_CPU_SERVER and its AK_SERVER_THREADS equals the segment's, and refuses before timing
  otherwise (both refusals checked in the smoke). Own mode (unset): starts, warms (50) and stops
  its own server at the segment's worker count.
- Recorded per pass: machine facts at start and end (CPU model, kernel, SMT, governor, scaling
  min / max / current frequency, no_turbo, the isolation mechanism from sysfs, the cgroup
  cpuset, the CPU sets and siblings, load, busiest processes, the server's pid / affinity /
  threads / AK_SERVER_THREADS), the client's affinity and sysconf counts, its threads by name,
  the core's path and sha256, getrusage deltas per repetition (ru_nvcsw, ru_nivcsw, ru_minflt,
  ru_majflt).
- Estimated benchmark wall time (from the smoke's per-call wall on this machine): main 3.9-4.6
  minutes (4 spread passes 96-112 s, 3 main passes 138-162 s), var4 2.3-2.7 minutes, about
  6.2-7.3 minutes in all. If it does not fit: var4's C and C-q first (about -50 s), then d/4MiB.
- Invocation (shared mode; the owner's CPU sets; the server state files the owner's):
  `AK_CPU_CLIENT=1-4,11-14 AK_CPU_SERVER=5-8,15-18 AK_SERVE_STATE=/tmp/ak-physical-s8.state gen/physical_probe.sh --segment main --out ../../logs/cpp/opt/physical-probe/main`,
  then the same with `AK_SERVE_STATE=/tmp/ak-physical-s4.state --segment var4 --out ../../logs/cpp/opt/physical-probe/var4`;
  `--grpc-cpus 8` / `--grpc-cpus 4` to size grpc-core with the segment (not decided).

**TCP inversion attribution (2026-10-01)** (`gen/tcp_attrib.sh OUT perf|wall|cpu|p4 STK_CORE CTL_CORE H16_CORE`,
tables `gen/tcp_attrib_tables.py OUT SYSTEM_MAP`, logs `logs/cpp/opt/physical-probe/tcp-attrib/`). One 8-worker pinned
server per phase with the TCP listener (serve.sh, AK_SERVER_TCP=0), the server's affinity checked before every process,
one-cell `--profile` processes. Cores: this tree's (A, D, Cf), the p1-p7 stack (Cf-zc, ring 6 + lock), and for p4 the
p1-p3 stack with crates.io h2 (ctl, sha256 9e0308aa, AK_H2_COALESCE=1) against p1-p3 + p4 (h16, b140a8fb,
AK_H2_COALESCE=16), both predating p5-p7. What the harness now records:
- `campaign_rpc --profile` adds `irq_time` to its JSON: /proc/stat irq and softirq time and /proc/softirqs NET_RX /
  NET_TX on the client's CPUs and on the server's (AK_SERVER_PID), across the loop. The kernel has
  CONFIG_IRQ_TIME_ACCOUNTING=y: softirq time is charged to no task, so CLOCK_PROCESS_CPUTIME_ID (every earlier CPU
  column of this slice) and getrusage miss it; perf stat task-clock and cycles include it. The `cpu` and `p4` phases
  wrap each client in `perf stat` (task-clock, cycles, cycles:u, cycles:k) enabled around the loop.
- `gen/perf_net_attrib.py`: client and server kernel cycles by network path (netfilter, send path, loopback transmit,
  loopback receive softirq, wakeup, epoll, futex, socket read), plus the receive-softirq cycles under a user-side socket
  write. `gen/perf_attrib.py`'s resolver (`resolver()`, used by both) maps an address past `_etext` to `[module]`; it
  used to return the last core symbol there. Module code at the leaf is 0.000 ms on every UDS row of this run.
- `gen/strace_threads.py`: time inside socket writes, writes over BLOCK_US, EAGAIN on write and on read.
- the header records the kernel's accounting options and the netfilter state read-only (modules loaded: nf_tables,
  nft_compat, nf_conntrack, nf_nat, br_netfilter, xt_* and others; bridge-nf-call-iptables 1; the ruleset is not
  readable without root).
Measured (per call; tables.md sections 1-4): the client's receive-side kernel work on TCP runs under its own socket
writes (rx under write equals rx anywhere to within 0.06 ms in every client row); NET_RX raises on the client's CPUs
per call follow the socket writes (Cf 1032, A 149 at d/16 k=1); with softirq time counted, the TCP gap Cf - A at d/16
k=1 is +7.29 ms of client task-clock (process clock +3.32), Cf-zc - A +5.42 (+1.49); on UDS task-clock and process clock
agree within 0.25 ms and softirq time on the client CPUs is 0.00-0.06 ms per call. The core cells' TCP wall (13.0-13.3
ms at k = 1 and 8) comes with no send-buffer wait (0 EAGAIN, sndbuf_limited never reported, Send-Q median 16-33 KiB).
With p4 (73 writes per d/16 call), Cf-h16 - A on TCP is -0.63 ms of client task-clock at d/16 k=1 and +0.09 at k = 8.

**TCP by default (owner, 2026-10-01).** Decisions: two h2 variants (stock = crates.io 0.4.19, the default;
h2-batch = PR #903 port + p4 at AK_H2_COALESCE=16), zero copy not pursued (p2, p3, p5, p8, p9 dropped; p1 kept in the
core), TCP only (127.0.0.1, Nagle off, verified), CPU from task-clock with the process clock beside it, Docker running
(netfilter recorded as a machine condition). The drivers source `gen/net_target.sh` (AK_NET=tcp default, uds an
explicit option; every TCP process refused unless every client TCP socket reads TCP_NODELAY = 1): tcp_attrib.sh,
stability.sh, core_ab.sh, physical_probe.sh (gbench; its CPU stays the process clock), deferred_checks.sh. Historical
UDS-only drivers left as they were: tcp_uds.sh (compares both), profile_4a.sh, patch_ab.sh. Zero-copy and deferred
cells are out of the default sets (TA_ZC=1, ST_ZC=1, DC_CELLS to bring them back). The worker sweep is the
`sweep` phase of tcp_attrib.sh with `gen/sweep_tables.py`. CMake: `core_camp_h2batch` and
`core_camp_nounk_h2batch` (outside `all`, built one at a time; `poc/codec/h2-batch/build.sh` and
`gen/h2batch_nounk.sh`, both restoring poc/codec/Cargo.lock), loaded at run time with LD_LIBRARY_PATH; written against
the Rust agent's uncommitted `poc/codec/h2-batch/` as found on disk, not yet built. `gen/deferred_checks.sh`
DC_STEP1_CORE=1 makes the conformance and pre-check binaries load the given core. The landed p1 (82f3712a) fixes the
ring at 6 with the lock and reads no AK_SPARES: Cf-q runs with 6 (the patch-era runs used 24); the sweep records
minor faults per call for every process and, in untimed allocprobe processes, allocations of at least 1 MiB per call
for Cf-q at k = 8 on both variants (coordinator). Agreed scope: 3 rounds, no k = 16. Since done: the CMake h2-batch targets were built against the landed core and
used (tcp-sweep/build/), the sweep ran (tcp-sweep/), and the full gates on both variants ran in the container on
2026-10-02 (section "WP12 gates" below).

**h2 PR #903 from C++ (2026-10-01)** (`gen/h2_variants_build.sh`, `gen/tcp_attrib.sh OUT h2|h2b` with H2_CTL, H2_H16,
H2_PR, H2_COMB, H2_ROUNDS, H2_STRACE, TA_NOTE; tables `gen/h2pr_tables.py OUT SESSION...`; logs
`logs/cpp/opt/physical-probe/h2-pr903/`). Four cores built in the worktree on the p1-p9 stack, differing only in the
workspace `[patch.crates-io] h2`: ctl (crates.io 0.4.19, 4cdfbb6c), h16 (p4, f4a3ab3e), pr903 (PR #903 port at the
Rust agent's 221c21e, 5fd48371), pr903p4 (PR #903 + p4 at b871798, 8e43391f). The Rust agent's PR and combined cores
differ from these by sha256 (build paths; same h2 sources, same exports), so these rebuilds are what was measured.
The committed-to-be patch file `logs/rust/opt/patches/h2-pr903/h2-pr903-on-0.4.19.patch` predates 221c21e; the src
patches actually built are in `h2-pr903/build/`. Checks per core: 0 failures, semantics 33/33, check-stream matched.
Session 1 (`h2/`, three cores, 1 round, strace of every unit) and session 2 (`h2b/`, four cores, 2 rounds, strace of
pr903p4 only); the machine was suspended before session 1 and its IRQs re-pinned (headers). Measured: PR #903 alone
keeps one stream's frames as separate writes (1027 per d/16 call at k = 1) and batches across streams (439-454 at
k = 8); with p4 combined, 73 (k = 1) and 36-52 (k = 8).

## WP12 gates on both h2 variants (2026-10-02, container)

Driver `gen/wp12_gates.sh stock|h2-batch OUT` (steps build, wp5, asan, campaign, deliv, q, marker), run from a git
worktree of `1bdbaa07` in the scratchpad (sparse: without `ffi/logs/rust`), CPU sets client 1, server 2,3. Toolchain:
g++ 13.3.0, cmake 3.28.3, rustc 1.94.1, apt protobuf 3.21.12 / grpc++ 1.51.1 (libprotobuf-dev 3.21.12-8.2ubuntu0.3,
libgrpc++-dev 1.51.1-4.1build5), Google Benchmark v1.8.3 release. The slice builds against these unchanged (after the
physical machine's protobuf 34 changes); no build-compatibility fix was needed.
- **The variants.** The binaries are the same build for both; only the core differs. stock: every binary loads its
  RUNPATH core. h2-batch: the six rpc-feature cores (target-rpc, -rpc-count, -camp, -camp-count, -camp-nounk,
  -camp-count-nounk) have h2-batch twins (CMake core_camp_h2batch / core_camp_nounk_h2batch for the two campaign cores,
  poc/codec/h2-batch/build.sh and gen/h2batch_nounk.sh for the other four; build-h2batch.log: h2 compiled in =
  h2-batch-src for each, poc/codec/Cargo.lock unmodified after), and every gate loads them through LD_LIBRARY_PATH, as
  the timed drivers do: `gen/core_swap.sh` (AK_CORE_SWAP, a map stock dir -> twin dir; inert when unset) is sourced by
  wp5_gate, nounk_gate, run_campaign, q_checks and deferred_checks at each call site of an rpc binary. The codec-only
  cores have no h2 in their crate graph (`rpc` is an optional feature): header.txt shows "h2: none compiled in" for
  every one of them, h2-0.4.19 for the six stock rpc cores, so the codec steps run the same core on both variants.
- **Proof the variant is in effect.** Every step ran with LD_DEBUG=libs; `gen/core_census.py` lists, per program, the
  libak_core.so each process loaded and its sha256 (census-<step>.txt), and fails when a C++ process loaded a core
  of the other variant: 0 such processes in every step of both runs. h2-batch: campaign gate 51 processes on
  target-camp-h2batch (campaign_rpc 48, campaign_codec 2, campaign_calib 1), 31 on target-camp-nounk-h2batch, the two
  counting clients on their twins; wp5_gate's rpccounts on target-rpc-count-h2batch; deliv 10 + 3; q 6 + 6 and the
  counting twins. The Rust server's warm-up client (rpc_warm) loads poc/rust's own core: stock by design (README of
  h2-batch, "tonic in the Rust host keeps crates.io h2"). Positive marker (marker.log, gen/wp12_marker.sh: strace -f
  of one --profile process, Cf-retain d/16MiB over TCP, gen/strace_threads.py): socket writes per call 1030.4 (k = 1)
  and 1028.9 (k = 8) stock, 72.9 and 39.2 h2-batch.
- **Results, both variants, 0 failures:** wp5_gate every step (conformance 608/0 at a17, b17, c14, c11, static; noinit
  plant 299 failures as required; corpus 680/680/696/696/680/680 at four builds, outcomes identical; decision 11
  controls; probe; byte audit 0 changed pairs against aba944a; boundary; groupskip, concurrency, ODR, bench gates,
  content sets; counts 530 rows identical; rpccounts; nounk_gate). d11_asan 0 failures. Campaign gate: every control
  fires, codec pre-checks pass, counts 530, RPC counts 132 / 78 identical. deliv_checks over TCP (TCP_NODELAY read
  back): conformance and pre-check on the variant's core, --semantics 1 24/24 both builds (cancel of a pending send
  and a pending recv included), check-stream 28 benchmarks (A, A-cb, A-q, D, Cf, Cf-q, Cf-cb at d/4 and d/16, k = 1
  and 8; Cf-q's calls reach UploadStreamCheck since 99c18cd1) with every call's count and SHA-256 matched, c grid 12.
  q_checks: semantics, 4 plants per build abort with no sample, grid 140 / 84, counts 132 / 78.
- **D16** (h2-batch: up to 15 more DATA frames before RST_STREAM after a local reset): no step failed on h2-batch only,
  so nothing was attributed to it. No check asserts what the server received after a cancel, so the gates neither
  show nor exclude the divergence.
- **Run history** (runner.log): the stock steps marker, deliv, q, campaign, asan ran at d8601658 (header-d8601658.txt);
  its first wp5_gate run used build-campaign as BUILD_DIR to save disk and is VOID (wp5-run1-VOID-build-campaign/:
  rd2_guard, boundary and the other gates name `build/`); wp5_gate was rerun on its own `./build` at 1bdbaa07. Between
  the two commits only gen/wp12_gates.sh changed. The h2-batch run is entirely at 1bdbaa07. header.txt's
  AK_H2_COALESCE column of the run was wrong (grep -x on a binary's strings); the appended correction recomputes it:
  present in the six twins only.

## What exists

```
poc/codec/gen/cpp_binding.py   the C++ host binding over the C ABI (arm core-ffi), rendered
                               from a plan. WP8: geometric unk_grow (rule 8), one reset per
                               retain decode (rule 7; dec_ctx_free forgets an armed context),
                               string spans copied without a second UTF-8 scan (utf8=reject:
                               the core checked them; decision 3). Full build:
                                 - encode_into_* (and _zeroed, _nobatch), encode_into_*_unk and
                                   (B-2) encode_into_*_unk_zeroed; the timed core arms and
                                   cells C/D use the sparse (_zeroed) fill, named in the headers;
                                 - decode_with_* (drop context), decode_with_*_opts (armed
                                   in place), decode_with_*_unk (retain everywhere),
                                   decode_with_*_pool (pre-allocated pools, refilled in place);
                                 - (X-2) the PULL family: pull_with_* (walk in place),
                                   pull_drain_with_* (drained copy), pull_with_*_unk (retain),
                                   the records replayed through the push vtable's functions;
                                 - unk_opts_*, unk_clear_*;
                               Optimisation unit (2026-09-28): geometric growth of batched
                               adds (B-1), elements and leaf children built in place (B-3/B-4),
                               packed enum/bool/bulk runs (B-6/B-7), a flat retained-buffer set
                               (B-9, B-8).
                                 - DecRoot<T>, DecCtxs, dec_ctx_new_for<T>() (contexts bound
                                   to their root, rule 6).
                               No-unknown build (plan relowered with unknown="drop"): none of
                               the unknown-field family; ak_dec_ctx_new_<Root>(void).
                               Buffers still in the options after a decode are the host's
                               and are left there (R-H7: reusable options, rule 7).
poc/codec/gen/cpp_facade.py    the facade; in the no-unknown build without `unknown_fields`
                               (owner decision R-H22). That build is a separate configuration
                               with its own header directory, so no installed header changes
                               layout under a consumer's -std (README 5.1)
poc/codec/gen/cpp_native.py    arm host-gen: the codec generated into C++ from the same plan,
                               drop and retain renderings; since the optimisation unit: an exact
                               reservation per repeated field from one key scan (HG-1),
                               emplace_back (HG-2), packed runs reserved once with raw stores and
                               little-endian memcpy (HG-4); UTF-8 through the core's
                               ak_utf8_check (HG-3)
include/ak_abi.h, include/generated/ak_layout*.h   full-build C header (c_abi.py, 400 facts)
nounk/include/...                                  no-unknown header (AK_NO_UNKNOWN_FIELDS,
                                                   240 facts)
corpus/include/..., corpus/nounk/include/...       the same two for the corpus reader schema
nounk/src/generated/types*.{h,cpp}, corpus/nounk/src/generated/types.{h,cpp}
                               the no-unknown facades (no unknown_fields), found first on the
                               nounk targets' include path (sources include <generated/types.h>)
src/generated/                 facade types, binding(_nounk), borrowed-facade binding(_nounk),
                               core_native(_retain), builders (facade and protobuf), cases,
                               projection, touch (read-every-field traversal)
corpus/src/generated/          the corpus reader schema's facade, native codecs, binding(_nounk),
                               projection, dispatch(_nounk)
build-upbclang/, build-upbft/     TRACKED build configurations of the upb FASTTABLE experiment
                               (upb-fasttable.log); not used by any gate
include/ak/rt.h, vocab.h, values.h, projjson.h    hand-written runtime for the native codec
                               (WP8: ak::Enc::take hands the encoded buffer over moved, with a
                               recycled spare, for cell F and the codec suite's transport rows)
                               and the facade vocabulary (decode-rule constants come from
                               include/generated/ak_rules.h, rendered from the plan)
```

The harness binaries, one source each (`CMakeLists.txt`):

| Source | Targets | What it does |
|---|---|---|
| `src/conformance.cpp` | `conformance_{a17,b17,c14,c11}_shared`, `conformance_a17_static`, `conformance_a17_noinit` (plant); `conformance_nounk_{a17,c11,static}` | payload byte identity against `ffi/schema/generated/manifest.json` on every encoder and decoder arm, the layout table against the core's `ak_layout_facts`, absent/unknown/malformed vectors; decision 11's pool, refill, oneof, error and wrong-root cases (full); rule 6 and the drop of unknowns (no-unknown) |
| `corpus/src/corpus_main.cpp` | `corpus_all_{a17,c14,c11,a17_static}`, `corpus_all_noinit` (plant); `corpus_nounk_{a17,c11}`, `corpus_nounk_noinit` (plant) | one corpus row per process, four arms (ffi-drop, ffi-retain, native-drop, native-retain); `--unk` runs decision 11's controls on one row |
| `src/counts.cpp` | `counts_a17_shared`, `counts_a17_static`, `counts_a17_static_lto`; `counts_nounk` | crossing counts from a counting core plus the binding's host counter (resets) and `ak_enc_take`: payloads and, with `--corpus DIR --rows TSV`, the 92 U rows; drop and retain (R5, req 19) |
| `src/bench.cpp` | `bench_*` (levels, linkages, guard off, perturbation, crossing tax, LTO, and the `bench_a17_gateplant` plant) | the pre-campaign codec timing harness; its arms are gated before timing |
| `src/contentsets.cpp`, `src/concurrency.cpp`, `src/groupskip.cpp`, `src/utf8check.cpp`, `src/odr_*.cpp`, `src/fusion_probe.cpp` | `contentsets_a17`, `conc_*` (incl. four planted), `groupskip_*` (incl. two planted), `utf8check_*`, `odrcheck`, `fusion_probe` | content sets, concurrency suite, group skip, UTF-8 validator differential, ODR across levels, the boundary checker's control |
| `src/rpcbench.cpp`, `rpcflow.cpp`, `rpccounts.cpp` | `rpcbench`, `rpcflow`, `rpccounts` | the pre-campaign RPC grid, the flow-control probe, RPC crossing counts |
| `src/campaign_codec.cpp` | `campaign_codec`, `campaign_codec_nounk` | campaign codec suite (Google Benchmark), with its own gate and plant |
| `src/campaign_rpc.cpp`, `campaign_calib.cpp`, `sha256.h`, `proto/campaign_grid.proto` | `campaign_rpc(_nounk)`, `campaign_rpc_count(_nounk)`, `campaign_server`, `campaign_calib` | campaign RPC client, cells A-F with framed twins Bf/Cf/Ef, directions a, a+read, b, c (Upload P5.3/P5.4) and d (UploadStream 4/16 MiB, SHA-256 checked; `--plant c-len|d-sha|d-count` for the gate's controls; `--payloads` narrows the jobs, `--iters [PAYLOAD/]K:N` fixes Google Benchmark's iterations, every repetition carries getrusage counters, the header the CPU facts and threads by name) (two builds; `--warm-server N` warms the server; the `_count` builds print per-call counts for B-E with `--count N`); server in its own process on two Unix sockets (shipped, pinned), one per launch; crossing-cost loops |
| `src/upbbench.cpp` | `upbbench` (needs `gen/fetch_upb.sh`) | the upb ceiling arm |

Scripts (`gen/`):
- `wp5_gate.sh`: the correctness gate, both builds. It builds everything and refuses stale
  binaries, then runs:
  - the generator checks;
  - conformance at five levels/linkages plus the noinit plant;
  - the full corpus at four builds, with retention gaps bounded to `U-map-entry`;
  - decision 11's controls at four builds, and their plant;
  - corpus plants and `--compare`;
  - the oracle-probe rows;
  - the byte audit against the retired harness;
  - boundary and layout;
  - groupskip, concurrency, ODR, bench gates, content sets, crossing counts (against
    `logs/cpp/counts-baseline.log`), RPC counts;
  - `nounk_gate.sh`.
- `nounk_gate.sh`: the no-unknown build's own gate.
  - Each binary is checked to load the core of its variant.
  - Both headers are checked against both cores (`poc/rust/gen/c_variant.sh`, read-only).
  - Byte identity at C++17, C++11 and static.
  - The corpus at C++17 and C++11, with every unknown row written in its dropped form.
  - Plants, and counts against `logs/cpp/counts-nounk-baseline.log`.
- `d11_asan.sh`: both builds' conformance and corpus under ASan+LSan.
- `wp12_gates.sh stock|h2-batch OUT` (WP12): the gates above plus deliv_checks over TCP, q_checks and the variant
  marker on one h2 variant, each step under the loader's record; `core_swap.sh` (AK_CORE_SWAP, sourced by the gate
  drivers, inert when unset), `core_census.py` (cores loaded per program, sha256, variant check), `wp12_marker.sh`
  (socket writes per d/16MiB call).
- `run_campaign.sh --suite codec|rpc|calib|gate --out <dir>`: the campaign runner (see the
  checklist).
- `campaign_summary.py`: requirement 30's summaries, ratios from per-launch medians.
- `opt_bench.sh OUT_DIR` (optimisation phase, instrumentation, NOT a gate): builds as the runner builds (header() and
  gbench_release() extracted from `run_campaign.sh`, cmake -DAK_RPC=ON, the timed and counting targets, each checked to
  load its variant's core), records the crossing counts against the committed files, then runs the codec suite
  (payloads: 5 repetitions x 0.01 s, warm-up 0.005 s; U-* rows: 3 x 0.004 s, warm-up 0.002 s; pool 1 MiB) on both
  builds and the RPC grid (3 x 0.04 s, warm-up 0.02 s, k = 1, 8, 16; one server for the run, warm 50, both
  transports, both clients), pinned AK_CPU_CLIENT=1 / AK_CPU_SERVER=2,3 by default. About 10 minutes here.
- `opt_summary.py RUN_DIR`: summary-codec.tsv, variants-codec.tsv, summary-rpc.tsv, tables-codec.md, tables-rpc.md
  from the raw samples (absolute times, no ratios; H-1 rows and single-batch RPC entries marked).
- Physical machine (2026-09-29): `shell.nix` (the build environment), `physical_checks.sh LOG_DIR` (step 0's
  checks), `physical_probe.sh --segment main|var4 --out DIR [--smoke] [--grpc-cpus N]` (the probe driver),
  `physical_probe.py DIR` (its tables), `machine_facts.py CLIENT SERVER [PID...]` (header facts from sysfs and
  /proc), `ncpus_shim.c` (optional LD_PRELOAD: the CPU count grpc-core sizes itself from).
- `u_rows.py`: the 92 U rows (accepted, non-disputed, at the seven shapes roots) from the corpus manifest, as the TSV the codec suite and counts read.
- `gbench_to_jsonl.py`: Google Benchmark JSON to section 7's lines.
- `corpus_all.py`: the corpus driver, with `--unk-controls`, `--expect-dropped`,
  `--max-retain-gap`, `--plant`, `--record` and `--compare`.
- Pre-campaign timing and probe scripts, which produce instrumentation only: `run_all.sh`,
  `rpc.sh`, `rpcflow.sh`, `contentsets.sh`, `concurrency.sh`, `utf8.sh`, `tax.sh`,
  `drift.sh`, `opt.sh`, `c16.sh`, `c24_timing.py`, `upb_ab.sh`, `calibrate.sh`,
  `fetch_upb.sh`.
- Checks: `boundary.sh`, `groupskip.sh`, `odr_check.sh`, `rd2_guard.sh`, `rd2_history.sh`,
  `refusal_test.py`, `audit_tracked.sh`, `wp5_bytes.py`.

## What was checked, and where the log is

From a fresh `git worktree` at `9997ea57` (the end of the optimisation unit), with no uncommitted changes and new
build directories: `CLEAN=1 gen/wp5_gate.sh build`, `gen/d11_asan.sh`, `gen/run_campaign.sh --suite gate`, then the
Rust slice's `run_campaign.sh --suite gate` (driver log `opt/final-gate/runner.log`). wp5_gate's first run failed
only its byte audit, because the shallow clone lacked the before-tree `aba944a`
(`opt/final-gate/wp5-bytes-run1-shallow-clone.log`); after `git fetch --deepen=400` the second run passed every step.

| Check | Result | Log |
|---|---|---|
| build | every target configured and built from scratch; every gated binary newer than its sources | `wp5-build.log` (run 1: `opt/final-gate/wp5-build-run1-clean.log`) |
| generator | `generate.py --check` every target current; the one-generator guard and its planted import; the shared `--check`; `refusal_test.py`, `rd2_guard.sh`, `audit_tracked.sh`, `one_core.sh` and `--selftest` | `wp5-generator.log` |
| payload byte identity, full build | 608 checks, 0 failures at C++17 target, C++17 floor, C++14, C++11 and static (577 before the unit; +31: the `_unk_zeroed` encodes and their decision 11 round trip); the noinit plant fails cleanly (299 failures) | `wp5-conformance.log` |
| full corpus, full build | 702 rows, four builds, six arms: ffi, ffi-pull (drop, retain) 680/0, native 696/0, 6 disputed, 16 roots not in the C ABI; 4212 (row, arm) outcomes identical across the four builds; retention gaps outside U-map-entry 0 on every retain arm; plants proj/reenc/accept/noinit fail | `wp5-corpus.log` |
| decision 11 controls | four builds: 2290 positions, 0 failing rows; the plant fails 307 rows | `wp5-corpus.log` |
| oracle-probe rows | 11/11, every arm, C++17 and C++11 | `wp5-probe.log` |
| byte audit | 0 (row, arm) pairs changed against the retired harness (`aba944a`) | `wp5-bytes.log` |
| boundary, layout | 574 corpus layout facts agree, shared and static | `wp5-boundary.log` |
| other gates | groupskip (and its two plants), concurrency, ODR, bench gates and plant, content sets, crossing counts 530 rows identical (shared and static), RPC counts | `wp5-gates.log` |
| no-unknown build | 0 u-family exports in every variant core; 478 checks 0 failures at C++17, C++11, static; corpus C++17 = C++11 (4212 outcomes, 0 differ), unknown rows written dropped; plants fail; 301 count rows identical | `wp5s10-nounk.log` |
| ASan + LSan | both builds: conformance, decision 11 controls and corpus (ffi-pull arms included) green, 0 sanitizer reports | `asan.log` |
| campaign gate | every control fires; codec pre-check (incl. the pull value gate) 0 failures; counts 530/301/72/42 identical | `opt/final-gate/campaign-gate.log`, `counts.log`, `rpc-counts*.log` |
| Rust slice's gate | PASSED on stable rustc 1.94.1; crossings 775 and 398 rows identical to `gen/crossings*.txt` | `opt/final-gate/rust-gate.log` |

**Decision 11, as the owner confirmed it** (ABI-v1 rule 4 amended 2026-09-26): there is one
options entry per oneof, and the core fills it in the active member's decode group. The
binding takes the active message member's slot into that member's bag and frees any other
member's non-NULL slot. This is unchanged by the amendment.

## Crossing counts (R5, req 19; counts, not timings)

Every count is forward = core entry points + host-counted resets (`ak_enc_reset`,
`ak_dec_reset_<Root>`, counted in the binding under AK_COUNTING) + `ak_enc_take`, with the
breakdown on each row. Reset places: an encode resets once, before it; a retain decode resets
ONCE, before it (decision 11 rule 7, WP8: its options sit at a stable per-thread address and
the context stays armed; a drop decode on a context the binding left armed disarms it first,
and the timed harnesses give retain decodes their own contexts so that never happens there);
a drop decode makes none. Retain: no pre-placed buffer, `unk_grow` grows geometrically (at
least double, at least 64 B, clamped to INT32_MAX; rule 8), the same in the counting and the
timed build.

- **Full build:** `logs/cpp/counts-baseline.log`, 530 rows: the 485 earlier rows (117 payload rows: encode, its
  unbatched, zeroed-fill and host variants, decode, retain decode and encode; 368 U rows, 92 x 4) and, since step 8d
  (X-2), 45 pull rows (`decode pull`, `decode pull drained`, `decode pull retain` per payload). The 485 earlier rows are
  byte-identical to the file before the unit: no optimisation step moved a count. Pull rows: reverse 0 on every
  payload, records = the push decode's reverse count on every payload (e.g. P2.2 3501), P2.2: walk 3 forward
  (core 2 + host 1), drained 24 (core 23 + host 1), pull retain 4 (core 2 + host 2).
  Re-taken 2026-09-27 (WP8): every `decode retain` row 1 forward lower; 18 U rows fewer grow calls.
- **No-unknown build:** `logs/cpp/counts-nounk-baseline.log`, 301 rows (271 + 30 pull rows, no retain rows). Against
  the full build it differs on P1.2 decode reverse (8 full, 5 without unknown-field support; the pull rows' records
  follow: 8 and 5) and on the drained rows' forward count (drain calls per root: P2.2 24 full, 17 no-unknown; P1.2
  10/7; P2.3 13/12; P2.4 15/14; P4.1 7/6), records equal (`wp5s10-nounk.log` section 5).
- **RPC cells, per call:** `logs/cpp/rpc-counts.log` (72 rows: B, Bf, C/Cf/D/E/Ef in retain
  and drop, x jobs a, b, c P5.3, c P5.4, d 4 MiB, d 16 MiB) and `rpc-counts-nounk.log` (42
  rows), from `campaign_rpc_count(_nounk) --count 4`. Examples: B a/b/c 2/0, B d 6 and 12; C
  c 4 (reset, encode, ak_call_unary_enc, free), C d 10 and 28; D c 4, D d 8 and 32; C-retain a
  4/3501, C-drop a 3/3501; E 2 (c) and 6/12 (d); framed twins identical to their references.
  These agree with the Rust slice's committed rows for the same cells. The campaign gate diffs
  both files. Re-taken 2026-09-27 (WP8): retain a 1 lower; C b 1 lower (ak_call_unary_enc
  replaces ak_enc_take + a copy); D b rpc 0 -> 2 (ak_enc_take_owned and its ak_bytes_free
  replace ak_enc_take + a copy); new rows for the twins and jobs c and d. Re-taken 2026-09-28
  (req. 16 as amended): +60 / +36 rows for the queue cells (B-q, Bf-q, C-q-*, Cf-q-*, E-q-*,
  Ef-q-*), 132 and 78 rows in all, every earlier row identical. A queue cell adds ak_queue_next
  per completion and ak_call_destroy: B-q a/b/c 4/0 (blocking 2), C-q c 6 (4), C-q-drop a 5/3501
  (3/3501), B-q d 9 and 21 (6 and 12: one ak_queue_next per send and one for the recv), C-q d 13
  and 37 (10 and 28), E-q 4 (c) and 9/21 (d); framed twins identical to their references.
- **Pre-campaign RPC delivery counts:** `logs/cpp/rd2-rpccounts.log`. Blocking 2/0,
  callback 3/1, queue 4/0 forward/reverse per call, counting `ak_call_destroy`.

## Timing logs in the tree (instrumentation only)

These logs exist and are committed raw. They were taken in containers: two machines, a
4 vCPU Xeon at 2.80 GHz and another at 2.10 GHz, with no isolation and no governor
control. Most predate the port to the shared plan, decision 11, `init-guard` and the
facade's `unknown_fields` member, and none was re-taken after those changes. They are
listed so that nobody re-derives them. **No figure from them is quoted here.**

- `bench_*.log`, `drift.log`, `tax.log`, `opt.log`, `c16.log`, `c24-timing.log`,
  `utf8.log`, `contentsets.log`, `w10-one-core.log`: the pre-campaign codec bench and its
  controls (2.80 GHz).
- `upb.log`, `upb-fasttable.log`: the upb v25.3 ceiling arm (2.80 GHz).
- `rpc.log`, `rpcflow.log`: the pre-campaign RPC grid and the flow-control probe (2.10
  GHz). They were taken before the 6-field `ak_client_opts` existed (`rd2-history.log`).
- `calibration-r13.log`: the rust slice's crossing bench on the 2.80 GHz container.
- `campaign/*.jsonl`, `campaign/*.gbench.json`: campaign smoke runs.
  - `codec-launch1.*`, `rpc-launch1*`, `calib-*`: the WP9/WP10 minimal smoke at `4ce48e007`,
    `"smoke": true`, full build and `shipped` only; every timing stripped. codec 3820 samples;
    rpc 255 samples (15 cells x 17 direction/payload/in-flight groups) on Google Benchmark, raw
    JSON beside them (`rpc-launch1-shipped-full.gbench.json`). `codec-nounk-launch1.*` are the
    earlier WP8 smoke's (no-unknown build), not re-taken.

- `opt/baseline/` (2026-09-28, `gen/opt_bench.sh` at `447d79e2`, code at `3cbf2216`, a 2.80 GHz 4-CPU Xeon container):
  the optimisation phase's reference run. Raw: `codec-{P,nounk-P,U,nounk-U}.jsonl` and `rpc-{shipped,pinned}{,-nounk}.jsonl`
  (section 7's lines) with the Google Benchmark JSON beside them (`*.gbench.json.gz`); `runner.log`, `header.txt`,
  `build.log`; summaries `summary-codec.tsv`, `variants-codec.tsv`, `summary-rpc.tsv`, `tables-codec.md`, `tables-rpc.md`.
  `opt/u-warmup0-evidence.txt`: why the U rows got a warm-up (the discarded first run's one-iteration samples).
- `opt/ref/` (after step 0's harness fixes, the reference for the steps), `opt/s1/` .. `opt/s9/`, `opt/s8b/`: one full
  `gen/opt_bench.sh` run per kept step, same layout as `baseline/`. `opt/final/`: the final run at `cfb3e1b2` (the tree the
  final gates passed on, plus STATE); its `tables-codec.md` and `tables-rpc.md` are that run's only.
- `opt/rpc-same-machine/` (2026-09-28, `gen/rpc_same_machine.sh` at `83fc612b`, code as at `cfb3e1b2`): the RPC grid
  only, for a side-by-side with `logs/rust/opt/rpc-same-machine/` taken just before on the same container. Shared server
  via serve.sh (4 tokio workers, CPUs 2,3, warm 50 once), client CPU 1; shipped then pinned, full then no-unknown client,
  three processes per (transport, client) (G1 a/a+read/b, G2 c, G3 d); 10 repetitions per entry at 25/25/50 ms
  (the Rust run's 250/250/500 ms criterion measurement time over its 10 samples), warm-up 30 ms; k = 1, 8. 720
  entries, 485 s. Raw: `rpc-*.jsonl`, `*.gbench.json.gz`, `*.console`; `summary-rpc.tsv`, `tables-rpc.md` (the Rust
  layout, `*` = one batch per repetition, 204 entries); `header.txt` carries a `vs rust` line with the differences.
- `opt/rpc-same-machine-q/` (2026-09-28, `gen/rpc_same_machine.sh` at `d7a556b3`): the same RPC-only grid and settings
  as `opt/rpc-same-machine/` plus the completion-queue cells (req. 16 as amended): 1168 entries x 10 repetitions, 750 s
  (past the ~12 minutes: the settings were kept); 263 single-batch entries; `tables-rpc.md` labels the core cells
  `(blk)` / `(q)` and ends with the in-process q/blk median ratios per direction. `opt/q-deliveries/`: `checks.log`
  (`gen/q_checks.sh`: semantics 14/14 per build, plants on queue cells abort, grid smoke, RPC counts identical, ASan+LSan
  0 reports) and `codec-checks.log` (codec pre-checks 0 failures, codec counts 530/301 identical).
- `opt/ab/<step>/`: narrowed alternated A/B runs (`gen/opt_ab.sh`, `gen/opt_ab.py`; gzip'd Google Benchmark JSON per
  pair, `ab.txt` ratios). `opt/probes/`: allocation probes (`gen/allocprobe.c`, `r1-allocs.log`), the C-7 worker probe,
  and the diffs of the reverted variants (`o10-reverted.diff`, `o7-reverted.diff`, `f1-reverted.diff`).
- `opt/checks/<step>.log`: each step's `gen/opt_checks.sh` (generate --check, one_core, builds, conformance both builds
  and levels, corpus, counts, codec pre-checks), ASan runs, the Rust step checks (`rust-pre8a`, `rust-s8b`, `rust-s9`).
- opt_bench settings (changed during the unit, recorded in each run's header): payloads 5 x 0.01 s (warm-up 0.005 s),
  U rows 3 x 0.003 s (warm-up 0.001 s), pool 1 MiB, RPC 3 x 0.04 s (warm-up 0.02 s) at in-flight 1 and 8 (16 dropped
  at step 8d when the run reached 613 s), server warm-up 50 calls.

## Optimisation unit (2026-09-28, owner-approved), step by step

Every figure is container instrumentation from `gen/opt_bench.sh` (logs/cpp/opt/<run>/) or a
narrowed A/B (`gen/opt_ab.sh`, logs/cpp/opt/ab/). The reference for the steps is `opt/ref/`.

| Step | What | Commit | Kept? | Evidence |
|---|---|---|---|---|
| 0 | harness fixes H-1 (pool walked), H-2 (a condition variable per caller), H-4 (registered raw methods), H-6 (cell B reused buffer), H-7 (incumbent-best fastest entry point; incumbent-arena labelled), H-8 (from=bytebuffer decode rows, labelled), H-9 (one run(iterations) per repetition); U rows 3 x 0.003 s | f00c900e | harness fix | `opt/ref/` (548 s): pre-check 0 failures (1156, 716, 2944, 1840 slots), counts identical |
| 1 | B-1: the binding's batched adds grow geometrically (`grow_by`) | 67ae45c4 | kept | A/B `ab/s1-b1`: core-ffi decode P1.2 -11%, P6.1 -6% (3/3); `opt/s1/` |
| 2 | B-2: `encode_into_*_unk_zeroed` rendered (sparse fill over u-groups); core arms and cells C/D encode with the sparse fill (header `core_encode_fill`) | ae8e0fe4 | kept | `ab/s2-b2`: P1.3 -40%, P3.1 -13%, P4.1 retain -13%; P5.1 +10 ns; P5.3/P5.4 +3..8% is drift (host-gen, unchanged, moves the same, `ab/s2-b2-p5`); `opt/s2/` |
| 3 | B-3/B-4/B-5: elements and leaf children built in place; ak::Optional without redundant T() assignments | 1794a95d | kept | `ab/s3-b345`: core-ffi decode -2..-43%, host-gen -3..-18% (P1.3 host-gen +7%, 0/3); ASan clean (`checks/s3-asan.log`); `opt/s3/` |
| 4 | HG-1/HG-2: host-gen decode counts repeated fields, reserves exactly, emplace_back | 55e6a1e5 | kept | `ab/s4-hg12`: P1.3 -66%, P7.1 -20%, P2.5/P3.1/P4.1 -8..-12%; P2.1 +3..+5%; `opt/s4/` |
| 5 | HG-4 + B-6/B-7: packed runs (host-gen reserve once + raw stores + LE memcpy; binding enum as the array, bool stack buffer, bulk adds) | 67972f58 | kept | `ab/s5-packed`: P6.1 host-gen encode -44%, decode -9..-14%; core-ffi encode -14%, decode -13..-16%; ASan clean; `opt/s5/` |
| 6 | HG-6 + B-9 + B-8: ak::Enc hand-off through an atomic single slot (no new Owned, no mutex); flat live set for retained buffers, unk_reclaim guarded; recycled ak_bytes holders | 453e20a2 | kept | `ab/s6-transport`: host-gen transport encode P5.1 -28%, P1.1/P2.1 -7..-9%; `ab/s6-unk`: U retain decode -6% geomean; ASan clean; `opt/s6/` |
| 7 | R-2 kept (D/F responses: TrySingleSlice, else a reused buffer); R-1, HG-7, C-3 measured and not kept; C-7 and HG-5 probes | 68ab3fbe | R-2 kept | `ab/s7-r1r2-rpc` (D/F a -2.5..-3.6%); R-1: `probes/r1-allocs.log` (C d fresh 2 MiB buffers 1.4 -> 1.0 per chunk) but time within drift; HG-7 `ab/s7-hg7-r2` (host-gen decode +2..+10%); C-3 `ab/s7-c3-codec` (geomean 1.005), `ab/s7-c3-rpc` (0.98 in 0.53-1.67 noise); C-7 `probes/c7-workers` (1 vs 2 workers geomean 0.992; default 2 kept); `opt/s7/` |
| 8 | X-1: labelled core-ffi-borrow decode arm | 6fc11124 | arm | fold gate; payloads only |
| 8d | X-2: the pull decode family (walk, drain, retain), corpus arms, counts rows, labelled codec arm core-ffi-pull and RPC twins Cp-*/Dp-* | fa56c163 | arm | value gate on 120 groups x modes (planted control fails); corpus ffi-pull 680/0; counts: reverse 0, records = push reverse on every payload; ASan clean; `opt/s8/` (613 s -> k=16 dropped from opt_bench) |
| 8a | O-10: exact tonic buffer on the unary reference path, exact per-message reserve, recycled body copies | not committed | dropped | allocations B/E 2 -> 1, Bf/Ef 1 -> 0 per call (`probes/r1-allocs.log`), client CPU within drift (`ab/s8a-o10-rpc` 1.003, `ab/s8a-o10-rpc-c` 1.002); the diff: `probes/o10-reverted.diff` |
| 8b | O-9: ak_blob_run in one pass (shared core, rust_abi.py + lib.rs) | 49845a60 | kept | `ab/s8b-o9`: P2.3/P2.4 encode -8% (3/3), P6.1 -3..-6%; Rust step checks 0 failures, crossings identical (`checks/rust-s8b`); `opt/s8b/` |
| 8c | O-6: skip the loop call of an empty repeated field via a presence bit | not built | STOPPED | the presence word has room (TaskDetailed 14 + 4 of 32) but no backend sets a bit for a repeated field: every fill renders `presence_bits` (plan.py 645: singular children and explicit fields only) with its own test and skips loop slots (cpp_binding `_make_field`, rust_binding make_*, cs_host `_emit_fill`, java_binding `_group_members`, py_capi fill_*); a core that skipped on a clear bit would drop data on every other slice. Reported, nothing changed |
| 9 | HG-3: `ak_utf8_check` exported additively from ak-core (the core's check_utf8, simdutf8), declared only in include/ak/rt.h; host-gen validates through it (one forward call per non-empty string) | 46c487a1 | kept (owner's choice) | utf8check 17,797,200 checks 0 failures with the new path (`checks/s9-utf8check.log`); `ab/s9-hg3`: host-gen decode wide -15..-45%, ASCII +5..+15% (P7.1 +22%, short strings pay the call); Rust step checks 0 failures (`checks/rust-s9`); `opt/s9/` |
| 10 | F-1: ak::Optional over std::unique_ptr | not committed | reverted | `ab/s10-f1`: decode +2..+70% (P3.1 +55..+70%), better only on P1.3 (-5..-8%) and host-gen encode (-5..-15%); geomean 1.038; `probes/f1-reverted.diff` |
| 11 | O-7: a ring of 3 spare buffers in ak_rt::Enc | not committed | reverted | fresh buffers per C d16 call 11.5 -> 7.9 (`probes/r1-allocs.log`); `ab/s11-o7-rpc` 0.974, confirmation `ab/s11-o7-rpc-d` 0.996 (controls D/F move as much): within drift; `probes/o7-reverted.diff` |
| final | gates from a clean worktree of `9997ea57`, then one full opt_bench | 8507f776, 5f977d4a | - | wp5_gate 0 failed steps, d11_asan 0, campaign gate passed, Rust gate PASSED (`opt/final-gate/`); `opt/final/` (611 s incl. 21 s build: 11 s over the 10-minute budget; pre-check 0 failures, counts identical) |

## CAMPAIGN.md section 10 checklist

| # | Requirement | Status |
|---|---|---|
| 1 | one machine, slices sequential | **met** on the runner's side: suites run one after another and nothing runs concurrently. The machine is the owner's |
| 2 | governor, turbo, SMT | **met (recorded)**: the header reads scaling_governor, intel_pstate/no_turbo, cpufreq/boost and smt/active. The runner does not set them (owner) |
| 3 | isolation | **met (recorded)**: `/sys/devices/system/cpu/isolated`, isolcpus/nohz_full from the kernel cmdline, the runner's cpuset |
| 4 | three disjoint CPU sets, one NUMA node, no shared SMT siblings, parameters; sizes fixed; worker thread counts in every header | **met** on the runner's side: AK_CPU_CLIENT and AK_CPU_SERVER are required (from `ffi/campaign.machine` through `ffi/campaign.sh`, which exports them; the sizes 4+4 are checked there, not here); overlap, a shared SMT sibling and a NUMA span are refused (`campaign/runner-controls.log`). Thread counts: the runner header carries a `threads` object; the codec header its process thread count; the RPC client header `caller_threads`, `core_runtime_workers` and `process_threads_after_warmup` (grpc-core sizes its own pollers, counted in the process total); the server's thread count is in READY and in its exit line. The OS set is "everything else" and is not checked |
| 3 (table) | incumbent at gRPC v1.54.0 and at a current version | **not met here**. The runner builds against the incumbent under AK_INCUMBENT_PREFIX (CMAKE_PREFIX_PATH, PKG_CONFIG_PATH and PATH, recorded as `incumbent_prefix` in every header); the campaign runs it twice, once with a gRPC v1.54.0 prefix (ArmoniK's level) and once with a current one. This container has only apt's grpc++ 1.51.1 / protobuf 3.21.12, so neither prefix exists and every run here used the system install. The owner provides the two prefixes |
| 5 | floors gated, no timing | **met**: the C++11 (and C++14) conformance and the C++11 corpus in the gate, for the full build; C++11 conformance and corpus for the no-unknown build |
| 6 | build flags printed | **met**: flags from the build, shared linkage, LTO off, core features, and the build variant (full or no-unknown) in every header |
| 7 | 16 payloads, content sets, U-* rows | **met**. Latin-1 and wide run on P1.2, P2.2 and P2.4, tagged `set=required`; P3.1, P4.1 and P6.1 run too, tagged `set=extra` (R-H26). The U-* rows are the 92 at the seven shapes roots (`gen/u_rows.py` from the manifest), tagged `row=U`, in all three directions: encode (host-gen re-encode as the check), decode and decode_read, for core-ffi drop and retain (retain in the full build only), host-gen and the incumbent (R-H27). Rows at other roots run only in the corpus build, for correctness |
| 8 | arms | **met**: incumbent-prod (grpc++ SerializationTraits), incumbent-best (since H-7 the library's fastest entry points: ByteSizeLong + SerializeWithCachedSizesToArray into a reused buffer, ParseFromArray), core-ffi (push), host-gen. Labelled extras (payloads only): incumbent-arena (decode on an Arena), core-ffi-pull (ABI v1 7.1's pull family, walked in place; X-2), core-ffi-borrow (the borrowed-string facade; X-1), and the from=bytebuffer decode rows (H-8). The Rust-only arms are not applicable |
| 9 | encode, decode twice | **met**: `decode` and `decode_read` (the generated read-every-field traversal) |
| 10 | three modes for core-ffi and host-gen | **met**: core-ffi drop and retain in `campaign_codec`, no-unknown in `campaign_codec_nounk`; host-gen drop and retain in `campaign_codec`, no-unknown in `campaign_codec_nounk` (the drop rendering over the facade without `unknown_fields`, R-H22). The no-unknown build has no retain arm of either kind. Every sample carries `unknown_mode` and `build` |
| 11 | serialise once per iteration, fresh object; encode variants | **met**. Decode goes into a fresh object every iteration; protobuf C++ recomputes ByteSizeLong on every Serialize. Encode rows are tagged `end` and `input` (R-H29), graphs built in setup, outside the timed window. end=reused: bytes in a reused buffer (incumbent-best `SerializeToString` into a reused string, core-ffi `ak_enc_take` of the context's buffer, host-gen into its reused `ak::Enc`). end=transport: the form handed to grpc++ (incumbent-prod `SerializationTraits<Message>::Serialize` to a `grpc::ByteBuffer`; core-ffi moves its bytes with `ak_enc_take_owned` and host-gen with `ak::Enc::take` into a `grpc::Slice` that owns them, wrapped as a `ByteBuffer`, what cells D and F do since WP8; the codec binaries therefore link the campaign cores with the `rpc` feature, which carry the same codec). Cell C hands the core's own buffer to the transport, which is the end=reused row, stated. input=hot: one graph; input=pool: distinct copies totalling `--pool-bytes` (runner: 2 x AK_LLC_BYTES, default LLC 13.75 MiB), round-robin |
| 12 | cells A-F; C, D, E, F in each mode; send paths | **met**: the full client runs A, B, Bf, C/Cf/D/E/Ef/F in retain and drop; the no-unknown client A, B, Bf and C/Cf/D/E/Ef/F-nounk. E is host-gen over the core's transport, F host-gen over grpc++ (R-H35). `Bf`, `Cf-*`, `Ef-*` are the core's framed send path (`ak_client_set_framed`) beside the reference (req. 14, WP8); every sample carries `send_path`. A pre-run check in every C-F mode compares the decoded and re-encoded P2.2 with the incumbent's re-serialisation, and every direction c/d request message with protobuf's bytes. Caller threads are created once and reused (R-H2) |
| 13 | server out of process, pre-serialised; one server per launch; one channel per cell | **met**, WP10: THE server is the Rust slice's tonic `rpc_server` (`poc/rust/SERVER.md`, service `armonik.ffi.campaign.v1.Grid`), built, started, warmed and stopped by the runner through `poc/rust/serve.sh`, one process per launch pinned to AK_CPU_SERVER, serving both builds' clients on two Unix sockets: `shipped` (tonic's server defaults) and `pinned` (4 MiB stream and connection windows, adaptive off); receive limit 8 MiB. P2.2 is pre-serialised once with prost (540,422 B). Warm-up: `serve.sh warm N` (N checked a, b, c calls and ceil(N/4) d calls per socket, tonic and core clients; AK_CAMPAIGN_SERVER_WARMUP). The server decodes b, c and every d message with prost. This slice's own grpc++ server is removed. One channel or `ak_client` per cell per client process, opened before any benchmark and warmed by the framework's warm-up |
| 14 | directions a, a+read, b, c, d | **met**: `a` (decode), `a+read` (decode, then read every field), `b` (R-H36); `c` = unary upload of P5.3 / P5.4 (M5, 1 MB and 4 MB), empty response; `d` = UploadStream in ArmoniK's shape, 4 MiB and 16 MiB in 2 MiB M5 chunks (splitmix64 data as the Rust slice, the ids on the first message only), the server answering an `UploadAck` (data byte count, SHA-256) the client checks. c and d at 1 and 8 in flight, every cell and send path; d with a third of the calls per sample (samples carry `payload` P5.3/P5.4/4MiB/16MiB) |
| 15 | 1, 8, 16 in flight | **met** |
| 16 | B and C blocking; A, D, F idiomatic | **met**, stated in the header's `delivery`. B, C, E: the core's blocking `ak_call_unary` (C: `ak_call_unary_enc`, the encode context moved, as the Rust slice); d: `ak_call_open(AK_CALL_CLIENT_STREAM)`, `ak_call_send` per chunk (C: `ak_call_send_enc`), `ak_call_recv`. A, D, F: grpc++'s synchronous generic call and `ClientWriter` (packages/cpp's idiom, R-H30); D hands grpc++ the core's buffer with `ak_enc_take_owned`, F host-gen's with `ak::Enc::take`, both moved into a `grpc::Slice`. Since 2026-09-28 (req. 16 as amended, c1d3db50): C++'s core reference is blocking AND the completion queue, both measured: the queue cells B-q, C-q-*, E-q-* and framed twins (labelled `(q)`, the blocking ones `(blk)`), `ak_call_unary_q` (C: `ak_call_unary_enc_q`, moved), d: `ak_call_send_q` / `ak_call_send_enc_q` then `ak_call_recv_q`; one `ak_queue` per cell, drained by the thread that issues the batch (no k caller threads), completions matched by tag; the design is the header's `queue_drainer`. The callback delivery runs only in the pre-campaign `rpcbench` |
| 17 | shipped and pinned; Unix domain socket; limits | **met**, stated. Every cell dials `unix:<path>` (grpc++ and the core, R-H28). `shipped` and `pinned` are this client's configurations, each against the shared server's socket of the same name (WP10). grpc++ shipped = packages/cpp's channel args minus its retry service config. grpc++ pinned has no connection-window argument, so only the stream half is pinned. core shipped = ak_client_new defaults. D44 (limits enforced on every path): grpc++ (both configurations), the pinned core client and the server set 8 MiB send and receive, covering P5.4 (4,194,390 B); core shipped keeps tonic's defaults (receive 4 MiB, send unlimited), which every message here fits |
| 18 | every call checked | **met**: gRPC status (non-OK = AK_ERR_RPC_STATUS on the core's paths) and response length on every call (cell A: content on every call, wire length once before the rounds); d: the server's byte count and SHA-256 against the client's own on every call. The first failure aborts and **leaves no sample** (R-H4): samples are buffered in the client and written only on success, and the runner deletes a failed launch file. The gate checks "no sample" for a wrong length, for an abort after two samples, for the runner's discard, and (WP8) for three plants in every cell and send path of both builds: `c-len` (direction c's response length), `d-sha` and `d-count` (the UploadAck). A failing `campaign_calib` is propagated the same way (R-H5) |
| 19 | crossing counts gate, every entry point, RPC B-E, retain | **met**. `counts_a17_shared` (and static) against `counts-baseline.log`, `counts_nounk` against `counts-nounk-baseline.log`: forward = core + host resets + `ak_enc_take`, with the reset's place in the header; payloads and the 92 U rows, drop and retain (retain: no pre-placed buffer, the geometric `unk_grow` of the timed build, decision 11 rule 8; one reset per retain decode, rule 7). RPC cells B, Bf, C, Cf, D, E, Ef per call, per mode, jobs a, b, c (P5.3, P5.4), d (4 MiB, 16 MiB): `campaign_rpc_count(_nounk)` against `rpc-counts.log` / `rpc-counts-nounk.log`, compared in the campaign gate (R-H31). Re-taken 2026-09-27 (WP8); every change is explained in the files' headers |
| 20 | crossing cost, fwd and rev, perf stat | **not met here**: perf is not installed. The runner builds and runs the rust slice's crossing bench, which did not build in the WP3 out-of-tree snapshot, and has not been re-run since. Reverse is reported as a fwd+rev row, from which the forward row is subtracted |
| 21 | process CPU per round | **met** for codec, RPC and calib. Codec and RPC: Google Benchmark's `MeasureProcessCPUTime()` per repetition, every sample `"cpu_clock":"process"`, real_time (wall) beside it (RPC: `UseRealTime()`). Calib: CLOCK_PROCESS_CPUTIME_ID of `campaign_calib` per round |
| 22 | order | **met**, stated. Codec and RPC: Google Benchmark's `--benchmark_enable_random_interleaving` (the repetitions of every benchmark of one process in random order, unseeded) plus registration order rotated by launch; RPC samples record `order_pos`, the position in Google Benchmark's output. The two builds' RPC binaries (and codec binaries) alternate by launch. Changed by WP9: the RPC order was a per-(round, dir, k) seeded shuffle of the cells |
| 22a | benchmark engine | **met** for codec and RPC (WP9, amended 2026-09-27): Google Benchmark v1.8.3, a Release build made by the runner from the upstream tag with its commit checked. Its warm-up, iteration control, ordering and raw export are used as they are; the custom pieces are listed under "Custom code around the framework" below. Every repetition is exported raw and converted to section 7's lines (`gen/gbench_to_jsonl.py`, codec and rpc modes). The calib suite stays on the runner (a few crossing loops, no framework needed) |
| 23 | 5 rounds x 3 launches, every round committed | **met**: rounds = Google Benchmark repetitions (runner defaults 5 x 3; the smokes used fewer, stated) |
| 24 | warm-up fixed, identical, a parameter | **met**: Google Benchmark's own `--benchmark_min_warmup_time` per benchmark, before its first repetition, codec (AK_CAMPAIGN_WARMUP_S) and RPC (AK_CAMPAIGN_RPC_WARMUP_S), and the server's warm-up calls (AK_CAMPAIGN_SERVER_WARMUP; d a tenth); AK_CAMPAIGN_SMOKE=1 shortens the defaults (0.01 s, 0.01 s, 20 against 0.5 s, 0.5 s, 200), and the header states the values run and both default sets. Changed by WP9: the harness-run codec warm-up (bytes per arm) and the RPC warm-up calls per cell are gone |
| 25 | allocator warmed identically; allocator mode | **met**: every benchmark's framework warm-up precedes its first repetition. D9 as amended 2026-10-03: the main figures run with glibc's default allocator (AK_CAMPAIGN_ALLOC=default, the runner unsets any ambient GLIBC_TUNABLES); AK_CAMPAIGN_ALLOC=pinned is the labelled diagnostic (GLIBC_TUNABLES trim 268435456, mmap 33554432 on the measured client processes, never the server; files alloc-pinned-*). The header's `malloc` states both; every sample carries `alloc` and its minor faults (RPC `ru_minflt`, `minflt_per_call`; codec `ru_minflt`, `minflt_per_op`). GC is not applicable. Startup check (`src/alloc_check.h`, in `campaign_codec*` and `campaign_rpc*`): one 16 MiB malloc against mallinfo2's mmapped-block count before any timing, once per process and kept mapped (freeing it would move glibc's dynamic mmap threshold), default must read `mmapped` and pinned `heap`; the process exits 4 with no sample if the readback disagrees with AK_CAMPAIGN_ALLOC (unset = default) or AK_CAMPAIGN_ALLOC with GLIBC_TUNABLES; the readback is a header line (`malloc_check`). Then, both modes, the heap pre-grow: allocate, touch and free a block of the run's largest message (RPC: P2.2, P5.3/P5.4 or one 2 MiB d chunk; codec: the largest payload) until a round faults nothing, at most 8 rounds, else exit 4; rounds and the last round's faults in the header (`heap_pregrow`) |
| 26 | correctness gate first | **met**: the campaign gate runs the full build's conformance, corpus, plants and counts, `nounk_gate.sh`, each codec binary's own gate and plant, both RPC clients' length and abort-after controls, the c/d plants in every cell and send path of both builds, and the RPC counts |
| 27 | header; dirty tree refused | **met**: a dirty tree is refused unless AK_CAMPAIGN_ALLOW_DIRTY=1 (smoke only). The header's `"instrumentation"` is true on a dirty tree or with AK_CAMPAIGN_SMOKE=1, which also sets `"smoke": true`, so a clean-tree smoke is marked (R-H19) |
| 28 | one JSON object per sample | **met**, plus a `build` field |
| 29 | logs in `ffi/logs/cpp/campaign/` | **met** |
| 30 | summaries only as specified | **met**: `gen/campaign_summary.py` keys on (build, arm or cell, payload, content, direction, mode, end, input, set, row) and forms ratios to incumbent-prod (end=transport for encode) or cell A from per-launch medians; no committed summary |
| 31 | runner interface; top-level `ffi/campaign.sh` | **met** for the slice runner. `ffi/campaign.sh` belongs to the aggregating session |
| 32 | smoke committed, marked | **met**: the WP5 step 10 smoke, figures stripped |

## Custom code around the framework (WP9, req. 22a amended)

Google Benchmark supplies the warm-up, the iteration count (`--benchmark_min_time`), the
repetitions, the random interleaving, the timers (process CPU, wall) and the raw JSON. What is
written here, and the requirement each piece serves:

| Piece | Where | Requirement |
|---|---|---|
| starting, warming and stopping THE server (the Rust slice's, through `poc/rust/serve.sh`), one per launch, pinned to AK_CPU_SERVER; reading its sockets and P2.2 size | `run_campaign.sh` | 13, 24 |
| one channel or `ak_client` per cell per client process, opened before any benchmark | `campaign_rpc.cpp` | 13 |
| k caller threads created once; one iteration hands one call to each of the first k and waits (a batch of k calls in flight, k operations) | `campaign_rpc.cpp` `Pool` | 15, R-H2 |
| queue cells: one `ak_queue` per cell; a batch's k calls issued back to back by the benchmark thread, which then drains the queue (`ak_queue_next`) until all k completed, tags (slot << 8 \| operation) matched, a stream's next send issued from the drain | `campaign_rpc.cpp` `q_batch` | 15, 16 |
| the check of every call, aborting the process (exit 3) at the first failure; Google Benchmark's JSON written to `FILE.part` and renamed only on success; the converter refuses any `error_occurred`; the runner deletes a failed launch's file | `campaign_rpc.cpp`, `gbench_to_jsonl.py`, `run_campaign.sh` | 18, R-H4 |
| the pre-run correctness gates (codec groups, RPC pre-check) | both binaries | 26 |
| the labels (cell, dir, payload, inflight, transport, send_path, build, unknown_mode; codec tags) encoded in the benchmark name and written as fields by the converter | `gbench_to_jsonl.py` | 28 |
| the encode input pool, built in the benchmark's setup, outside the timed loop | `campaign_codec.cpp` | 11 |
| the calibration loops (not a framework suite) | `campaign_calib.cpp` | 20 |

What the framework forces that differs from the hand-written sampler (also in the RPC header):
- one iteration is one batch of k calls, so each iteration pays one condition-variable
  hand-off to the k threads, where the old sampler paid one per sample;
- every benchmark gets the same time per repetition (`--benchmark_min_time`) rather than the
  same call count, so direction d no longer runs a third of the calls;
- the order is Google Benchmark's unseeded random interleaving, not a seeded per-group shuffle;
- the codec suite's samples are sized by time rather than by bytes (AK_CAMPAIGN_BYTES is gone).

## Register H (WP6 re-review): findings for cpp, proposed dispositions

| Finding | Evidence | Proposed disposition |
|---|---|---|
| R-H7 options reused after a decode: freed buffers left in the options | reproduced under ASan before the fix: heap-use-after-free in `unk_put` on the second decode with the same options (`logs/cpp/rh7-before.log`) | **confirmed, fixed** (`95c399de9`): `decode_with_<root>_opts` untracks every buffer still in the options, so reclaim frees only buffers the core consumed and the binding did not deliver; `_pool` frees its own leftovers. The reproduction is kept as a conformance control (`d11_reuse`), clean under ASan (`asan.log`) |
| R-H4 aborted RPC run leaves samples; control checks only rc | the client printed each sample as it was taken and the runner appended its output; the control grepped nothing | **confirmed, fixed** (`7875a980b`): samples buffered and written only on success; the runner deletes a failed launch file; three gate controls check "no sample" (`campaign/gate.log`) |
| R-H5 `campaign_calib` failure not propagated | the runner ignored its exit status; calib had no failure path | **confirmed, fixed**: calib refuses malformed arguments (exit 2) and writes only after every round; the runner propagates and deletes the file; gate control (`campaign/gate.log`) |
| R-H22 no-unknown keeps the facade member (owner decision) | `unknown_fields` rendered unconditionally | **done**: `cpp_facade` omits it under `unknown_compiled_out`; variant facades under `nounk/src`; host-gen no-unknown arm; retain arms not built in that build; full build unchanged |
| R-H2 RPC threads spawned per batch inside the window | `batch()` created k threads per call | **confirmed, fixed**: a pool created before the rounds, one condition-variable round trip per thread in the window |
| R-H23 order of arms (owner decision) | codec already used Google Benchmark's random interleaving (22a); RPC rotated with one schedule | **done**: codec unchanged (stated); RPC seeded shuffle per (launch, round, dir, in-flight) with `order_pos` (covers C++'s part of R-H18) |
| R-H19 `"instrumentation"` true only on a dirty tree | `run_campaign.sh` header | **confirmed, fixed**: AK_CAMPAIGN_SMOKE=1 marks a clean-tree smoke (`"smoke": true`) |
| R-H15 `cpp_layout.py` tests `options.unknown == "drop"` | as reported | **fixed by the rust agent** in `31fc3eecf` (with `c_abi.py`); `cpp_native.py`'s `unknown` argument is the host-gen mode, not the ABI variant, and was left |

Found while gating, both under the performance-scope rule's gate clause (a gate that would
let a wrong output be timed):
- The noinit plant aborted with a double free (`logs/cpp/rh7-noinit-doublefree.log`). When a
  reset was refused, `decode_with_*_opts` returned before leaving the options' buffers to the
  host. Fixed (`b915dc107`), with a control (a wrong root through the pool decode). The gate
  now requires the plant to exit 1, so a crash counts as a FAIL.
- `nounk_gate.sh`'s dropped-form control ran on the no-unknown build's
native-retain arm, which R-H22 removed, so the control could no longer fail. The gate at
`3a211100c` reported it as blind; it now runs on the full build's native-retain
(`8f5b575c0`).

## Open defects

| # | Where | What | Status |
|---|---|---|---|
| C6 | incumbent versions | the containers had apt's grpc++ 1.51.1 / protobuf 3.21.12; the physical machine has nixpkgs' grpc++ 1.80.0 / protobuf 34.1 (a current version). `packages/cpp` pins neither; CAMPAIGN.md asks for v1.54.0 and a current one | open: v1.54.0 is on neither; it needs its own prefix (AK_INCUMBENT_PREFIX) |
| C42 | `CMakeLists.txt`, floor targets | against protobuf 22 and later (the physical machine's 34.1) the C++11 and C++14 targets that include protobuf headers cannot build (protobuf requires C++17); only the C++17 targets were built there | open: the floors are gated against protobuf 3.21 only (the containers' logs) |
| C15 | `src/bench.cpp` | the `groupfill` arm has measured larger than the (`ffi` - `native`) delta it is a component of, on P1.3 (instrumentation, `bench_a17_shared.log`) | open; the direct-call hypothesis is refuted (JOURNAL). `groupfill` is labelled an upper bound |
| H-1 | `src/campaign_codec.cpp` | every `input=pool` encode row encoded pool[0] only | **fixed** at step 0 (`f00c900e`): a pool cursor walks the pool across iterations, and one pool is alive at a time |
| C41 | `gen/opt_bench.sh` settings | single-batch repetitions: at 0.04 s per repetition, c P5.4 and d at k=8 run one batch per repetition, so a sample is one batch and min-max is wide. Marked `†` in tables-rpc.md, column min_batches in summary-rpc.tsv. k=16 was dropped at step 8d (budget) | open, a budget trade-off (10 minutes); not a defect of the campaign harness |
| C43 | `tcp-vs-uds/tables.md` client CPU | the process clock (CLOCK_PROCESS_CPUTIME_ID) and getrusage miss softirq time on this kernel (CONFIG_IRQ_TIME_ACCOUNTING=y); on loopback TCP the receive path runs in softirq inside the client's writes, so every TCP client-CPU figure there is low, by 0.7 (A) to 4.6 ms (Cf) per d/16 call; UDS rows are unaffected (softirq 0.00-0.06 ms per call) | documented; the softirq-inclusive figures are `tcp-attrib/tables.md` section 3; the tcp-vs-uds tables stay as committed |
| C44 | `gen/perf_attrib.py` | kernel addresses past `_etext` (module code) resolved to the last core symbol before it | **fixed** (`resolver()`, "[module]"); every earlier perf attribution was on UDS, where this run finds 0.000 ms of module code at the leaf |
| C40 | `design/ABI-v1.md` section 5 vs `ak-abi` | `ak_err` is `{code, msg_len, msg}` in the specification's text and `{code, detail}` in the core; the C header follows the core | open, for the aggregating session |

The retired defects C1-C37, R-D1, R-D2 and R-G7 were fixed, or were closed by their owners,
and the record is in JOURNAL.md. The items reported against other owners were re-checked on
2026-09-26 and are closed:
- C19: the file no longer exists.
- C25: `U-map-entry` is now a disputed row.
- C26: `B-P7_1` now has `permutation_accepted`.
- C28: SHAPES.md's window table is corrected.
- C29: ABI-v1 section 9's delivery table no longer carries the counts.
- C38: `one_core.sh --selftest` passes in this gate (0 controls failed to fire), though
  `wp5_gate.sh` still labels that step as a known defect.
- C39: `rust_core.py` is named only in a comment.

## What is not measured

**TCP loopback (tcp-attrib).** The module code (2.8 to 3.6 ms per d/16 call at the leaf of the core cells' client kernel samples on TCP, 0.36 to 0.86 for A and D) is
attributed to netfilter by its callers (nf_hook_slow) and by the modules loaded, not by symbol: /proc/kallsyms and
/sys/module/*/sections need root. TCP state is sampled every 5 ms with ss, not per packet. The per-write kernel cost on
the connection's writer thread is not measured per thread (perf record covers the process, the strace durations
include strace's own overhead). Nothing was measured with netfilter unloaded (a machine setting).

**Timing, in general.**
- **Any timing on the campaign machine.** Every timing in the tree is container
  instrumentation (above). The physical probe's driver is ready and smoke-run only (the smoke's
  samples were not kept); its timed session waits for the coordinator.
- **The WP12 gates on the physical machine.** They ran in the container only (grpc++ 1.51.1 / protobuf 3.21.12);
  the campaign gate and q_checks dial Unix sockets (no TCP target in either); the Rust slice's gate step was left to
  the Rust agent. No check observes D16's extra DATA frames after a cancel.
- **The physical machine's gates beyond step 0.** On the physical machine only
  `gen/physical_checks.sh` ran: conformance at C++17 (both builds), the codec pre-checks,
  q_checks, every cell's grid smoke, the allocation probe. Not run there: `wp5_gate.sh` (corpus,
  floors, plants, boundary, ODR, ...), `d11_asan.sh`, the campaign gate, the payload/U crossing
  counts (`counts_*`; built, not run).
- **Any timing of the current tree outside the optimisation runs.** `logs/cpp/opt/` has container runs of the current
  tree (short fixed settings, one container); the pre-campaign logs predate the port and are not re-taken.
- **The price of unknown-field support on the campaign machine.** The codec suite has core-ffi drop, retain and
  no-unknown and the RPC grid C/D per mode; the only timed runs are the container opt_bench runs (two processes per
  build, so the full and no-unknown columns do not share a process).
- **The copy of a retained bag into `std::string`.** Rust adopts the core's buffer; C++
  copies it, because the facade type is `std::string`. This is not priced.

**Unknown fields.**
- **Map entries.** A map entry's unknown fields have no facade bag. They are counted,
  freed and dropped (the `U-map-entry` retention gap, a disputed corpus row).
- **Rule 5** (every capacity capped at INT32_MAX, amended 2026-09-26, core `31fc3eecf`):
  not exercised from C++. It needs a message whose unknown runs exceed 2 GiB; the core's
  own unit tests cover `unk_room`.
- **host-gen retain in the no-unknown build**: not built (its facade has no bag), so the
  no-unknown corpus reports native-retain and ffi-retain NOT BUILT.
- **The RPC server side.** The server decodes with the incumbent in every cell (b, c and each
  d message), so no cell runs the core's decoder on the server.
- **The RPC client under a sanitizer.** `d11_asan.sh` covers conformance and the corpus (the
  binding's buffers, rule 7's armed contexts); the RPC client, `ak_enc_take_owned`'s and
  `ak::Enc::take`'s hand-overs to grpc++ and the client streaming are exercised only by the
  gate's and smoke's own runs.
- **Streaming beyond client streaming.** Server-streaming and bidirectional calls are reserved
  kinds in the core and not built; `ak_call_opts` (deadline, metadata) is passed as NULL.
- **a+read for c and d.** Directions c and d carry no response to read; they have no read row.
- **The codec suite's memory at the default pool.** A pool of 2 x LLC of encoded bytes, held
  as facade graphs, peaks at about 6.9 GB in the gate (445 MB with a 1 MiB pool); the smoke in
  this shared container ran with AK_CAMPAIGN_POOL_BYTES=1048576, stated in its header.

**Coverage.**
- **The pull family's drained form in timing.** `pull_drain_with_*` (a JVM host's copy) is rendered, gated by
  value and counted, but the timed arm core-ffi-pull is the walk in place (as the Rust slice's); the drained
  form has no retain rendering.
- **RPC pull twins' crossings.** Cp-*/Dp-* are not in rpc-counts.log (their codec's crossings are the codec
  suite's pull rows).
- **O-6 (skip the loop call of an empty repeated field).** Stopped before any change (step 8c): no backend
  would set a presence bit for a repeated field.
- **A lossy (U+FFFD) decode policy** in the C++ native codec. It is not rendered, and the
  backend raises.
- **The chunking class, `Surrogate` and similar roots in timing.** The corpus build runs
  every row for correctness only. The timed codec covers the seven shapes roots.
- **CONTRACT.md C5 (produce)** for the `E-*` and `S-*` rows. It needs the corpus's value
  rules in C++. The 8 `baseline` rows are produced, by `conformance`.
- **Nesting past depth 3** and P7.1 being decode-only: gaps inherited from the payload set.
- **A C++20 coroutine surface.** A sketch only, not built.

**Campaign requirements this container cannot meet.**
- **The two incumbent versions** CAMPAIGN.md asks for (C6).
- **`perf stat` cycles and instructions** (req 20): perf is absent.
- **The rust slice's crossing bench** (R13, req 20): not re-run on this container since
  2026-09-24.

**Sanitizers and allocation.**
- **A thread sanitizer run.** The core is a Rust cdylib built without TSan, so the result
  would be noise.
- **Allocation counts and peak memory in the harnesses.** Only the ad-hoc probe `gen/allocprobe.c` (LD_PRELOAD, counts
  allocations of 1 MiB and more) and `campaign_rpc --alloc-probe` exist; nothing in a gate or a run header counts them.
- **Concurrency on the RPC half, and cancellation.** The RPC half has no byte-checked
  suite under contention and no plant. `ak_call_cancel` is exported and counted but never
  called.
- **Streaming, TLS, retry, deadlines, metadata** on the RPC path.
- **A second compiler.** clang++ 18 is installed and unused for the gates.

**Other open questions.**
- **When glibc's mmap threshold adapts**: the residual of C16. The cause of the P1.2
  outlier round was demonstrated by removal (`c16.log`); its timing is not predicted.
- **What a `protoc-gen-upb` minitable would add** above upb's generic decoder: it needs
  Bazel.

## Next step

WP12 (2026-10-02): the C++ half of the done criterion holds in this container on both variants
(`logs/cpp/opt/wp12-gates/`); the Rust slice's gate is the Rust agent's. Open, for the coordinator: the same gates on
the physical machine's grpc++ 1.80 / protobuf 34 (there the C++11 / C++14 targets that include protobuf headers do not
build, C42).

Response deliveries (2026-10-01): cells A-q (grpc++ async CompletionQueue), A-cb (grpc++ callback API), Cf-cb (core
callback delivery) added beside A, Cf, Cf-q (harness only; directions c and d); `gen/tcp_attrib.sh OUT deliv`
(DV_STOCK, DV_BATCH, DV_WORKERS, DV_WLS, DV_ROUNDS), tables `gen/sweep_tables.py OUT deliv --no-gaps`, checks
`gen/deliv_checks.sh`; logs `logs/cpp/opt/physical-probe/deliv/` (checks/, deliv/, tables.md, runner.log). Fixed:
check-stream now reaches the queue cells (it checked their byte count only before). `--semantics 1` covers the core's
callback delivery (24 checks per build).

TCP worker sweep (2026-10-01): rerun after the re-pin is the result (`tcp-sweep/tables.md`, `sweep/`, `runner.log`);
cores built and checked (logs/cpp/opt/physical-probe/tcp-sweep/build/cores.txt,
checks/: stock 788a879f, h2-batch b09e32cd, no-unknown ac194fff / d4d7c253; 0 failures on both variants over TCP). The
timed sweep that ran after the second suspend is NOT A RESULT (tcp-sweep/NOT-A-RESULT.txt). Both timed parts are
done and reported; what follows is the coordinator's.

Earlier plan, now executed up to the sweep: when the Rust agent's core lands (p1 in ffi/poc/codec; h2-batch as an opt-in
build variant), add the CMake h2-batch build following its script, rebuild both variants, run
`gen/deferred_checks.sh` on both (TCP; conformance and pre-check 0 failures, --semantics 1, check-stream), then
the TCP worker sweep: `SW_STOCK=<stock core dir> SW_BATCH=<h2-batch core dir> gen/tcp_attrib.sh OUT sweep ...`
(W 1, 2, 4, 8; A, D, Cf, Cf-q; d/16 and c/P5.4 at k = 1 and 8, k = 16 if the budget allows; about 12 minutes,
after the Rust agent's sweep, under the lock), tables `gen/sweep_tables.py OUT`.

The h2 PR #903 comparison (`logs/cpp/opt/physical-probe/h2-pr903/`) is done and reported, as is the TCP attribution
(`tcp-attrib/`); what follows is the coordinator's. Open from the h2 run: why PR #903 alone costs more client CPU than
crates.io h2 at the same write count (k = 1) while the server's CPU falls (no profile taken).
Not attributed there: why grpc++ writes 1.2 MB chunks on TCP against about 360 KB on UDS, and what the netfilter
module code is by name (the module symbols need root).

Physical probe: both segments are done (`logs/cpp/opt/physical-probe/{main,var4}/tables.md`); what follows is the coordinator's. The timed session was run in shared mode against the owner's
servers (invocation in the section "Physical-machine probe" above), then the tables
(`gen/physical_probe.py`, run by the driver) and the logs committed under
`logs/cpp/opt/physical-probe/{main,var4}/`. Open choice for the coordinator: `--grpc-cpus`
(grpc-core's pools sized for the segment or left at its own 16).

Before the physical machine: the optimisation unit is complete; what follows is the owner's (which kept steps stay, the O-/F- items not approved,
the campaign). For a rerun: `gen/opt_bench.sh logs/cpp/opt/<name>` (about 10 minutes, one process per comparison,
tables against `opt/final/`); a narrowed A/B with `gen/opt_ab.sh`; the per-step checks with `gen/opt_checks.sh NAME`.
The gate: `CLEAN=1 gen/wp5_gate.sh build` (about 16 minutes here, from a clone deep enough to hold `aba944a`), then
`gen/d11_asan.sh`, then `gen/run_campaign.sh --suite gate`, then the Rust slice's `run_campaign.sh --suite gate` when the
shared core changed. `gen/run_all.sh` takes timings and is not a gate.

## Log index

Current gate (clean worktree at `9997ea57`; the campaign and Rust gates of the same run are in `opt/final-gate/`):

| Log | What it contains |
|---|---|
| `wp5-build.log` | generate, configure and build every target, freshness check |
| `wp5-generator.log` | `generate.py --check` and the one-generator guard, the shared `--check`, `refusal_test.py`, `rd2_guard.sh`, `audit_tracked.sh`, `one_core.sh` |
| `wp5-conformance.log` | payload byte identity at C++17 target/floor, C++14, C++11, static (full build), decision 11's cases; the noinit plant |
| `wp5-corpus.log` | full corpus x4 builds, retention-gap bound, decision 11 controls x4 and their plant, corpus plants, `--compare` |
| `wp5-probe.log` | the rust slice's oracle-probe rows, four arms |
| `wp5-bytes.log` | the C++ arms before the port (`aba944a`) against after, row by row |
| `wp5-boundary.log` | `boundary.sh` (R5 from the artifact), corpus layout facts |
| `wp5-gates.log` | groupskip, concurrency (T7 off), ODR, bench gates and plant, content sets, crossing counts (+ retain rows), RPC counts |
| `wp5s10-nounk.log` | the no-unknown build's gate |
| `asan.log` | both builds under ASan+LSan |

Committed references and earlier correctness logs:

| Log | What it contains |
|---|---|
| `counts-baseline.log`, `counts-nounk-baseline.log` | the committed crossing counts, full (payloads and U rows, drop and retain) and no-unknown |
| `rpc-counts.log`, `rpc-counts-nounk.log` | the committed per-call RPC counts, cells B, Bf, C, Cf, D, E, Ef per mode, jobs a, b, c (P5.3, P5.4), d (4 MiB, 16 MiB) |
| `wp3-gate-count-stop.log` | the campaign gate stopping on the P1.2 count change decision 11 caused |
| `wp5s9-asan.log` | the WP5 step 9 ASan run (full build only), superseded by `asan.log` |
| `d38-probe-before.log`, `d39-stale-refusal.log` | D38 (field number above 2^29-1 in a skipped group) before the fix; D39's stale-binary refusal control |
| `rd1-lenwrap.log`, `rd2-guard.log`, `rd2-history.log`, `rd2-rpccounts.log`, `rd5-gate.log`, `rd5-before.log`, `rd7-before.log`, `rd-generator.log` | the R-D1, R-D2, R-D5 and R-D7 findings, before and after |
| `groupskip.log`, `odr.log`, `boundary.log`, `concurrency.log`, `conformance.log`, `generator.log` | earlier runs of checks the current gate re-runs (C24's group skip, the ODR check, R5, the concurrency suite, R2); superseded by the `wp5-*` logs |
| `corpus.log`, `corpus-native.log` | the retired subset corpus harness; superseded by `wp5-corpus.log` |
| `campaign/gate.log`, `campaign/counts.log`, `campaign/rpc-counts.log`, `campaign/rpc-counts_nounk.log`, `campaign/runner-controls.log`, `campaign/campaign_unknown_rows.tsv` | the campaign gate of the last smoke, its payload/U and RPC counts, the runner's CPU-set/dirty-tree refusals, the U-* rows the codec suite times |

Physical machine (`logs/cpp/opt/physical-probe/`):

| Log | What it contains |
|---|---|
| `checks/checks.log` | step 0's checks at `f79e034d` (gen/physical_checks.sh): build facts and each binary's core (path, sha256, variant), conformance 608/0 and 478/0, codec pre-checks 0 failed, q_checks, every cell's grid smoke on both transports and builds, the send-path allocation probe |
| `checks/q-checks.log` | gen/q_checks.sh of the same run: semantics, queue plants, queue grid smoke, RPC counts 132/78 identical |
| `checks/checks-run1-criterion.log` | the first run: the allocation criterion that failed on two 16 MiB pairs (same numbers, restated criterion) |
| `main/` | the main segment (shared 8-worker server pid 147567, core 8 workers, grpc-core sized for 8 through ncpus_shim; 4 spread + 3 main passes, 1800 repetitions, 254 s): `header.txt`, `runner.log`, `pass-*.jsonl` (samples with getrusage counters, machine facts at each pass's end, the client's header and thread classes), `pass-*.console`, `pass-*.gbench.json.gz`, `tables.md` |
| `var4/` | the var4 segment (shared 4-worker server pid 152128, core 4 workers, grpc-core sized for 4; 3 passes of six cells, 1080 repetitions, 146 s), same layout as `main/` |
| `profile/` | step 4a: `tables.md`; `stat/` (perf stat + profile JSON per process), `plain/` (no perf), `record/` (perf.data gzip'd, `*.attrib.json` buckets, `*.out`), `strace/` (raw strace gzip'd, `*.syscalls.txt`), `runner.log` (headers, machine facts per phase) |
| `patches/p1-ring/` | p1-ring from C++: `checks/checks.log` (patched core: 608/0, 478/0, codec pre-check 0 failed, semantics 0 failed both builds), `measure/*.out`, `alloc/*.out`, `tables.md`, `runner.log`; `spares24/` the follow-up with a ring of 24 |
| `patches/p4-h2-coalesce/` | the p1-p3 stack (ctl, core sha256 9e0308aa) against p1-p4 (h16, b140a8fb; h2 0.4.19 patched, AK_H2_COALESCE=16): checks per core and knob, `measure/` (GLIBC_TUNABLES trim 256 MiB, mmap 32 MiB; 3 rounds), `default/` (default allocator, 1 round), `strace/`, `tables.md` |
| `patches/p5-deferred/` | the p1-p2-p3-p5 core (sha256 b581292d; no-unknown 1137a22d) with the new Cf-enc / Cf-encp cells: `checks/checks.log` (608/0, 478/0, codec pre-check 0 failed, semantics 21/21 both builds incl. 7 deferred cases, HEAD core refuses Cf-enc, 24 check-stream benchmarks with every call's count and SHA-256), `measure/`, `default/`, `strace/`, `tables.md` |
| `patches/p6-zero-copy/` | the p1-p2-p3-p5-p6 core (sha256 9ac08de0; no-unknown 56177ce9; the patch as applied kept beside the logs): `checks/checks.log` (608/0, 478/0, codec pre-check 0 failed, semantics 26/26 both builds incl. 5 zero-copy cases, HEAD core refuses Cf-zc, 20 check-stream benchmarks), `measure/`, `default/`, `strace/`, `perf/` (perf stat and gzip'd perf record per cell, attrib JSON), `tables.md` |
| `patches/p7-deferred-zc/` | the p1-p2-p3-p5-p6-p7 core (sha256 01f731b5; no-unknown 59c08722): `checks/checks.log` (608/0, 478/0, codec pre-check 0 failed, semantics 33/33 both builds, HEAD refuses Cf-zc/zcp/zcw, 28 check-stream benchmarks), `measure/`, `default/`, `tables.md`; `wall/` (the k = 8 wall question: p7 and the p4 h16 core, server sampling and batch trace, `tables-wall.md`) |
| `cfq/` | the Cf-q investigation (see the section above), `q1-inline.patch` |
| `stability/run1/`, `stability/run2/` | the stability campaign: `proc/*.out` (one-cell processes in run order), `runner.log` (headers, machine facts after every round), `tables.md` |
| `tcp-vs-uds/` | UDS against TCP loopback (2026-10-01): `checks/client-nodelay.log` (TCP dialing, TCP_NODELAY read back on live client sockets), `proc/` (138 one-cell processes with the server's perf stat), `strace/`, `runner.log`, `tables.md`; its client CPU is the process clock, which misses softirq time (C43) |
| `tcp-attrib/` | the TCP inversion attribution (2026-10-01): `runner.log` (per-phase headers: cores' sha256, endpoints, tcp sysctls, kernel accounting options, netfilter modules, machine facts; phase times), `perf/` (client and server perf record per cell and transport, gzip'd, with `*.net.json` buckets), `wall/` (`*.ss.txt.gz` ss -tinm samples, `*.st.*` strace per process with `threads.json` and `syscalls.txt`), `cpu/` (both CPU measures and irq_time per process, 2 rounds), `p4/` (ctl against h16 with A, 3 rounds, client and server perf stat, strace), `p4-run1/` (the same before the client perf stat existed: process clock only), `tables.md` |
| `h2-pr903/` | h2 PR #903 against p4 and crates.io h2 (2026-10-01): `build/` (build logs per core with h2 source hashes and core sha256s, the src patches built, `pr-core-compare.txt`), `checks/` (deferred_checks per core), `h2/` (session 1: 98 timed processes + strace), `h2b/` (session 2: 248 timed processes + strace of pr903p4), `runner.log` (headers: suspend and IRQ re-pin note, cores, IRQs on the measured CPUs, machine facts), `tables-h2.md`, `tables-h2b.md` |
| `tcp-sweep/` | the TCP worker sweep (2026-10-01): `build/` (build.log, cores.txt: sha256 and h2 compiled in of the four cores), `checks/` (both variants over TCP, 0 failures), `chain.log`; `sweep-NOT-A-RESULT/`, `tables-NOT-A-RESULT.md`, `runner-NOT-A-RESULT.log`: the sweep that ran with the IRQs unpinned after the second suspend, NOT A RESULT (`NOT-A-RESULT.txt`); the rerun after the re-pin: `sweep/`, `runner.log`, `chain-rerun.log`, `tables.md` |
| `deliv/` | the response deliveries over TCP (2026-10-01): `checks/` (both variants: semantics with the callback cases, check-stream of every d cell, c grid of the new cells), `deliv/` (225 timed processes + strace), `runner.log`, `chain.log`, `tables.md` (raw figures, no gaps) |
| `checks/authority.log` | grpc++ 1.80's `unix:` authority reset by the shared server (RST_STREAM PROTOCOL_ERROR), and the calls passing with "localhost" |

Optimisation unit (`logs/cpp/opt/`):

| Log | What it contains |
|---|---|
| `opt/wp12-gates/{stock,h2-batch}/` | WP12 gates in the container at `1bdbaa07` (2026-10-02): `header.txt` (commit, toolchain, every core's path, sha256 and h2 compiled in, the swap map; correction appended), `runner.log`, `steps.txt`, `wp5/` (the wp5_gate logs), `asan.log`, `campaign/` (gate log, counts, RPC counts, gate.ok), `deliv/` (TCP), `q-checks.log`, `marker.log`, `census-<step>.txt`, `build-*.log`; stock also `wp5-run1-VOID-build-campaign/` |
| `opt/final-gate/` | the final gates' driver log (`runner.log`), the campaign gate (`campaign-gate.log`, `counts.log`, `rpc-counts*.log`), the Rust slice's gate (`rust-gate.log`), wp5_gate run 1's build and refused byte audit |
| `opt/final/` | the final opt_bench run: raw jsonl and gzip'd Google Benchmark JSON, `runner.log`, `header.txt`, `build.log`, summaries and `tables-codec.md`, `tables-rpc.md` |
| `opt/baseline/`, `opt/ref/`, `opt/s1/`..`opt/s9/`, `opt/s8b/` | the opt_bench runs before the unit, after step 0, and after each kept step |
| `opt/rpc-same-machine/` | the RPC-only grid for the same-machine side-by-side with the Rust slice's run |
| `opt/rpc-same-machine-q/`, `opt/q-deliveries/` | the same grid with the completion-queue cells; the queue cells' checks |
| `opt/ab/`, `opt/probes/`, `opt/checks/` | narrowed A/B runs, probes and reverted diffs, per-step checks (see "Timing logs in the tree") |

Timing logs (instrumentation only): see "Timing logs in the tree" above.
