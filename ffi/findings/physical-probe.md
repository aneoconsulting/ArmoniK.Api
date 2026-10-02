# The physical-machine probe (2026-09-29 to 2026-10-01)

The aggregating session's record of the probe run on the campaign machine after the
container work of FIX-PLAN WP11. It records facts, the logs that carry them, what is not
established, and the owner's decisions taken on them. It makes no recommendation.

Every figure is client CPU per call in ms unless stated, median, with p10-p90 where the
source gives it. "Gap" is a cell's per-process (Rust) or per-round (C++) median minus A's
in the same process or round. Figures from different sessions are not mixed in one row
unless the row says so.

## 1. Conditions

- **Machine.** Intel Core i9-7900X, 10 cores, SMT on, one NUMA node, kernel 6.18.54.
  Governor `performance`, turbo off (`no_turbo` 1), frequency locked at 3.3 GHz (scaling
  min = max = 3300000 kHz).
- **CPU sets** (owner, 2026-09-29): CLIENT 1-4,11-14, SERVER 5-8,15-18 (each set is 4
  cores with both SMT threads), OS 0,9,10,19.
- **Workers** (owner, 2026-09-29): 8 in every pool unless a variant says otherwise (server
  `AK_SERVER_THREADS`, Rust host runtime, the core runtime, grpc-core sized to 8 CPUs
  through a `sysconf` shim, because grpc-core 1.80 sizes its pool from
  `_SC_NPROCESSORS_CONF` (20 here), not from the affinity mask).
- **Isolation.** No `isolcpus`. From 2026-09-30 evening: systemd `AllowedCPUs=0,9,10,19`
  on `system.slice`, `init.scope`, `machine.slice` and `kubepods.slice`; IRQ affinity on
  the OS set; the owner's user processes pinned to the OS set with `taskset`. Harnesses
  pin the client and the server with `taskset`. Left on all CPUs: per-CPU kernel threads,
  the compositor (`kwin_wayland`, about 9% of one CPU, last seen mostly on the SERVER
  set), and a few idle helpers. One Docker container ran throughout, confined to the OS
  set. k3s was stopped.
- **Allocator** (owner, 2026-09-30): main figures under
  `GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432`
  for every cell, plus one default-allocator pass per comparison.
- **Server.** The Rust slice's `rpc_server` (poc/rust/SERVER.md), pinned configuration
  (4 MiB stream and connection windows, adaptive window off). Over a Unix domain socket
  until 2026-10-01; from then also over TCP 127.0.0.1 with `TCP_NODELAY` set on every
  accepted socket (`AK_SERVER_TCP`, commits 2c3fd644, c515296d).
- **Versions.** rustc 1.95.0, tonic 0.14.6, hyper 1.11.1, h2 0.4.19; g++ 15.3.0,
  grpc++ 1.80.0, protobuf 34.1 (ArmoniK builds against gRPC 1.54.0, not available on this
  machine; CAMPAIGN section 3 allows "a current one, both stated").
- **Incidents, all recorded in the affected headers.**
  - A re-pinning of user processes moved the C++ stability server off its set after round
    10 of `logs/cpp/opt/physical-probe/stability/run1/` (19:15:34Z to 19:16:41Z). Run 2 is
    the reference. Every driver now checks every server and client thread's affinity
    before and after each process and aborts on a mismatch.
  - Rust checks and builds ran on the OS set during C++ timed processes (run1 round 1;
    the h2 session 1 processes 0065 to 0071). The rule since: no heavy work while the
    other slice holds the bench lock.
  - The machine was suspended from 02:00:53 to 06:53:46 local on 2026-10-01, during core
    builds; 10 IRQs came back on all CPUs and were re-pinned before any timed run.

## 2. Measurement facts

1. **Rust A's cost has two modes, set by glibc, not by A's code**
   (`logs/rust/opt/attrib/`). tonic allocates a fresh encode buffer over 2 MiB per
   message; when glibc trims the arena, the next one faults in again: 1,200 to 2,200
   minor faults per 16 MiB call, about 11 to 13 ms, against 7.7 to 8.0 ms without. The
   mode flips between processes with no code change. It produced the raised A of the
   first spread passes of `logs/rust/opt/physical-probe/main-w8/`, and the "framed cells
   below A at k = 8" of the container. The pinned allocator above removes it.
2. **On TCP the process CPU clock misses softirq time.** The kernel has
   `CONFIG_IRQ_TIME_ACCOUNTING=y`. On loopback TCP the receive path runs in softirq on the
   sending CPU, inside its `writev`, and is charged to no task. Rust A at d/16 MiB k = 1:
   process clock 10.09 ms, perf task-clock 15.47 ms
   (`logs/rust/opt/physical-probe/tcp-vs-uds/accounting/`). The missing time depends on the
   number of writes: C++ A 7.35 / 8.03 ms (process clock / task-clock), Cf 10.76 / 15.32,
   Cf-zc 8.91 / 13.45 (`logs/cpp/opt/physical-probe/tcp-attrib/`). On UDS the two agree
   within 0.25 ms and softirq on the client CPUs is about 0.
3. **`perf stat` attached to a process distorts cell comparisons** in proportion to
   context switches: about +0.7 ms per d/16 MiB call on A and D (about 200 switches), +0.2
   on Cf (about 70) (`logs/cpp/opt/physical-probe/profile/`). Absolutes come from runs with
   no perf attached, or from a counter opened by the process itself; perf record only
   splits costs into categories.
4. **About 0.8 ms per d/16 MiB call of `clear_page_erms` in every cell is the kernel
   zeroing socket-buffer pages** (`CONFIG_INIT_ON_ALLOC_DEFAULT_ON=y`), not page faults.
   The C++ classifier counted it as faults until it was fixed and the 4a tables
   regenerated (C++ STATE, C43/C44).
5. **Netfilter is on the loopback TCP path** (Docker's rules: `nf_tables`, `nf_conntrack`,
   `nf_nat`, `br_netfilter`, `xt_*`). About 28% of the Rust TCP samples are module code.
6. **The machine's cargo `build.build-dir` was shared** by every target directory of one
   workspace, so every binary loaded whichever `libak_core.so` was built last. Both slices
   now build into the target directory (`poc/rust/gen/cargo-shim/cargo`, the C++ CMake
   cargo commands) and record each binary's core path and sha256.
7. **grpc++ 1.80 sends the socket path as `:authority` for `unix:` targets**, which the
   tonic server rejects; the C++ slice sets `GRPC_ARG_DEFAULT_AUTHORITY` to "localhost", as
   1.54 sends (`logs/cpp/opt/physical-probe/checks/authority.log`).

## 3. Over a Unix domain socket (history; the owner moved to TCP on 2026-10-01)

**Stability campaigns**, allocator pinned, alternating processes, no drift beyond the
floor (`logs/rust/opt/physical-probe/stability/stability.md`, 280 processes, 858 s;
`logs/cpp/opt/physical-probe/stability/run2/tables.md`, 684 processes, 1,206 s).
Cf and Cf-cb/Cf-q are on the core as committed then; Cf-zc on the experimental stack
(section 5).

| d/16 MiB, client CPU (gap to A) | Rust k = 1 | Rust k = 8 | C++ k = 1 | C++ k = 8 |
|---|---|---|---|---|
| A (tonic / grpc++) | 7.81 [7.35-8.71] | 9.34 | 8.22 [8.15-8.36] | 9.61 [9.49-9.80] |
| floor: A2 - A (Rust), D - A (C++) | +0.06 (-0.75..+0.69) | -0.03 | +0.06 (-0.10..+0.17) | +0.05 |
| Cf, blocking | 8.20 (+0.37) | 9.54 (+0.19) | 8.23 (-0.02) | 9.58 (-0.03) |
| Cf-cb (Rust) / Cf-q (C++) | 9.19 (+1.22) | 9.97 (+0.62) | 9.51 (+1.31) | 8.57 (-1.01) |
| Cf-zc, zero copy | 6.28 (-1.65) | 6.44 (-2.88) | 6.76 (-1.50) | 6.57 (-3.04) |

On d/4 MiB and c/P5.4 every cell except Cf-zc is within about 0.2 ms of A.

**Attributions on UDS.**
- **Cf over A (Rust, about +0.3 to +0.6 ms at d/16 k = 1):** the kernel's
  `copy_from_user` of the socket writes reads a buffer another CPU encoded: 5.7 to 5.9 M
  cycles per call against A's 4.0 to 4.3 M. Pinning Cf's caller thread and its core worker
  to one CPU puts Cf level with A and costs wall (10.18 against 8.62 ms)
  (`logs/rust/opt/attrib/pin-locality/`).
- **Df and Ff over A:** the second body frame per message (the 5-byte prefix frame),
  about 0.65 ms and 150 context switches per 16 MiB call; with one frame they are level
  with A (`logs/rust/opt/patches/p2-take-framed/`).
- **Cf-cb and Cf-q over Cf (about +1.2 to +1.3 ms at d/16 k = 1):** the core runtime's
  worker count. With 8 workers the per-send task and its wakes spread each stream over
  idle workers, each polling and parking again (Rust core-worker switches 220 to 276 per
  call against 102 to 115 for Cf). Rust Cf-cb 9.18 / 8.65 / 8.00 ms at 8 / 2 / 1 core
  workers, A 7.65 to 7.69; C++ Cf-q - Cf +1.28 / +0.59 / +0.09 ms at 8 / 2 / 1
  (`logs/rust/opt/cb-track/`, `logs/cpp/opt/physical-probe/cfq/workers-ab/`). Under more
  concurrency (`logs/rust/opt/cb-track/workers-sweep/`), 1 core worker costs no wall or
  throughput on d/16 MiB at k up to 32; on c/P5.4 with 4 connections (one core runtime per
  connection in that harness) 1 worker loses 4 to 6% throughput, 4 workers is within the
  spread of 8. **This contradicts the container's runtime probe**, which saw no CPU change
  from 1 core worker; the setups differed (1 client CPU there, worker counts 2 against 1,
  default allocator, ring of 3; poc/rust/JOURNAL.md, commit 89bc2cfb).
- **The callback bridge's per-send task** accounts for about half of the Cf-cb gap at 8
  workers: completing the send when the body takes the message (patch p9) takes Cf-cb from
  9.20 to 8.70 ms and 207 to 125 switches; at 1 worker it changes nothing.
- **The host executor slot through the C ABI** (patch p3, cell Cf-cb-1rt) costs about 1 ms
  more than Cf-cb per 16 MiB call (533 against 216 switches): the core's sockets belong to
  its own statically linked tokio, so a core thread must still drive them and every
  readiness becomes a cross-thread wake. The natively linked one-runtime cell Cn-1rt
  (harness only) is at the Cf level (`logs/rust/opt/patches/p3-exec-slot/`).

## 4. Over TCP 127.0.0.1 (Nagle off, verified on every live socket)

**The core cells invert against grpc++, and the reason is the write pattern.** C++, stock
h2, task-clock (softirq-inclusive), gap to A at d/16 MiB k = 1: Cf +7.29, Cf-zc +5.42 ms;
at k = 8 +6.51 and +2.35; Cf on d/4 and c/P5.4 about +1.9 ms
(`logs/cpp/opt/physical-probe/tcp-attrib/tables.md`). D stays within +0.02 to +0.16 ms of
A, so the core codec is not the cause.

| client kernel time per d/16 MiB k = 1 call (C++) | Cf | A |
|---|---|---|
| netfilter | 4.14 | 0.46 |
| loopback transmit + receive softirq, on the client's CPU | 3.25 | 0.66 |
| TCP send path | 3.46 | 2.86 |
| total kernel (UDS in brackets) | 12.46 (5.48) | 4.52 (4.46) |

- **Writes per d/16 MiB call:** the core (tonic over h2) about 1,026 writes of 16 KiB on
  both transports; grpc++ 14 of about 1.2 MB on TCP (47 of 360 KB on UDS).
- **Wall:** the core cells about 13 ms per d/16 MiB call at k = 1 and k = 8 against A 11.7
  and 9.0. The sender never waits for socket space (EAGAIN 0, notsent 0); the wall is
  attributed by elimination to the one connection's serialized writes, each carrying send,
  transmit, netfilter and receive work.
- **From the Rust host the ordering holds:** tonic A writes the same 1,035 frames per call
  and inflates like the core cells; the gaps to A keep their sign and size (process clock:
  A 7.69 -> 9.93, Cf 8.41 -> 10.49, Cf-zc 6.39 -> 8.59 ms, UDS -> TCP)
  (`logs/rust/opt/physical-probe/tcp-vs-uds/`).

## 5. Patches measured (all as patch files under `logs/rust/opt/patches/`; none committed to the core before 2026-10-01)

| patch | what | measured effect | owner, 2026-10-01 |
|---|---|---|---|
| p1-ring | ring of 6 spare encode buffers, blocking return | removes 1.6 to 2.0 fresh 4 MiB buffers per 16 MiB call; under the default allocator Cf-cb k = 8 10.62 -> 9.53 ms (Rust), Cf-q k = 8 9.91 -> 8.76 ms with a ring of 24 (C++ queue: one context shared by 8 streams); no effect under the pinned allocator | kept |
| p2-take-framed | additive entry: one frame per message for a host feeding its own gRPC stack | Df d/16 k = 1 8.40 -> 7.70 ms | dropped |
| p3-exec-slot | host executor slot through the C ABI | Cf-cb-1rt about +1 ms over Cf-cb | dropped |
| p4-h2-coalesce | patched h2: one stream's chunk written as up to N max-size DATA frames in one vectored write | see below | superseded by h2-batch |
| p5-deferred | encode on the transport's writing worker | -0.4 to -0.5 ms CPU, +0.5 to +1.2 ms wall at d/16 k = 1 | dropped |
| p6-zero-copy, p7-deferred-zc | borrowed host payload with a release callback | against A on UDS, both hosts: -1.5 ms (k = 1) to -3 ms (k = 8) per 16 MiB call; on TCP with stock h2: +5.42 (k = 1) and +2.35 (k = 8) against grpc++ A (task-clock), about -1.3 against tonic A (process clock) | not pursued: does not fit the ABI and the managed hosts |
| p8-cb-inline | complete a callback send inline when the channel has room | no change | dropped |
| p9-cb-at-take | callback send completed when the body takes the message | -0.5 ms at 8 core workers, none at 1 | dropped |

**h2 write batching.** Upstream has an open PR doing cross-stream batching,
[hyperium/h2#903](https://github.com/hyperium/h2/pull/903) (issue
[#902](https://github.com/hyperium/h2/issues/902)); its head (a1f880bc, h2 0.4.13) does
not build against hyper 1.11.1, so the Rust slice ported it onto 0.4.19 (tree 221c21e) and
combined it with p4 (tree b871798, "h2-batch"). Facts:
- #903 alone does not batch one stream: a frame cut from a larger chunk holds its stream
  back until written (`in_flight_partial_send`), so d/16 k = 1 keeps about 1,027 writes;
  at k = 8 it batches across streams (about 440 to 540 writes).
- #903 alone costs more client CPU than stock at the same write count (+1.6 to +2 ms per
  d/16 call): 18% of client cycles in its `poll_write_buf`, about 92% of that on filling a
  1,024-entry `IoSlice` array before every write (`logs/rust/opt/patches/h2-pr903/record2/`).
  A right-sized array is not measured.
- h2's own suite at N = 16 has one extra failure, `send_err_with_buffered_data` (queued
  sub-frames go out before a RST_STREAM), shared by p4 alone. burst_check (windows smaller
  than a burst, 8 multiplexed streams, cancel mid-burst, partial writes) passes.

C++, TCP, task-clock, four cores on the same stack differing only in h2, 2 rounds
(`logs/cpp/opt/physical-probe/h2-pr903/tables-h2b.md`):

| d/16 MiB | A (grpc++) | stock | p4 | #903 | h2-batch |
|---|---|---|---|---|---|
| k = 1, TCP | 7.69 | 15.25 | 7.51 | 17.09 | 8.34 |
| k = 8, TCP | 9.16 | 15.41 | 9.24 | 11.61 | 8.50 |
| writes per call, k = 1 / k = 8 | | 1,027 / 1,028 | 73 / 74-83 | 1,027 / 439-454 | 73 / 36-52 |
| Cf - A, k = 1 / k = 8 | | +7.57 / +6.24 | -0.17 / +0.08 | +9.40 / +2.44 | +0.65 / -0.66 |

On d/4 MiB and c/P5.4, h2-batch is within -0.20 to -0.03 ms of A. The Rust host gives
the same orderings (`logs/rust/opt/patches/h2-pr903-p4/headline-both-sessions.md`); there
A runs stock h2 unless "host-too", and Rust TCP d/16 k = 8 is bimodal by process (A near 9
or near 15 ms).

## 6. Owner decisions taken on these results (2026-10-01)

- **h2:** the rest of the POC builds the core in two variants, stock h2 0.4.19 and
  h2-batch (#903 port + p4, `AK_H2_COALESCE` 16). Nothing is reported upstream for now.
- **Zero copy is not pursued:** it does not fit the ABI and the managed hosts, and its
  tradeoffs are complex.
- **p1 is kept** in the core; p2, p3, p5, p8 and p9 are dropped.
- **TCP only** from now on (127.0.0.1, Nagle off, verified). UDS results stay as history.
- **The core worker count** is decided after the worker sweep is repeated on TCP, with
  both h2 variants.
- **Docker stays running** during campaigns; netfilter is recorded as a machine condition.
- **h2-batch is kept as is (2026-10-02, D16):** its divergences from stock h2 (data after a
  local reset, control frames behind a burst) are documented in `poc/codec/h2-batch/README.md`
  and ABI-v1 section 9, not fixed.

## 7. Contradictions with the container results

- The container's "framed cells cost more client CPU than A at k = 1" was mostly Rust A's
  allocator mode and noise; blocking Cf is level with A on UDS from both hosts.
- The container's Cf-cb below Cf is reversed here: Cf-cb is about 1.0 to 1.2 ms above Cf
  at 8 core workers.
- The container's "core worker count changes no CPU" is contradicted (section 3).
- The container's "every framed cell below A at k = 8" holds only against Rust A in its
  faulting mode, and does not hold against grpc++.

## 8. What is not established

- Any figure over a real network: loopback charges the receiver's packet processing to
  the sender and runs netfilter; kernel zero-copy sends do not apply to loopback or Unix
  sockets.
- The worker sweep on TCP, and with a core runtime shared by several connections.
- #903 with a right-sized `IoSlice` array; h2-batch with a PING, SETTINGS or GOAWAY
  arriving mid-burst; why grpc++ writes larger chunks on TCP than on UDS.
- The managed hosts (C#, Java, Python) on this machine.
- grpc++ 1.54.0 (ArmoniK's version) on this machine.

## 9. Response deliveries over TCP (2026-10-01 evening, after the re-pin)

Client CPU (task-clock) / wall per call in ms, medians, core workers 8; p10-p90 and the
1-worker rows are in `logs/rust/opt/delivery/tables.md` and
`logs/cpp/opt/physical-probe/deliv/tables.md`. "p1" is the core as committed (stock h2);
"p1 + h2-batch" the opt-in variant. Rust A runs on stock h2 unless marked host-too; C++ A
is grpc++ 1.80 and does not use h2. The delivery forms are harness constructions and
differ by host: Rust A-blk is `block_on` per caller thread, A-cb a completion callback
counting down a latch, A-q completions posted to one mpsc queue drained by one thread;
C++ A is the sync stub, A-cb grpc++'s callback API, A-q grpc++'s CompletionQueue drained
by the issuing thread. Rust Cf-q uses one encode context per in-flight call; C++ Cf-q one
shared context (ring of 6). C++ Cf-cb sends each next chunk from the previous send's
completion, on a core thread.

| Rust | blocking | callback | queue |
|---|---|---|---|
| A, stock h2, d/16 k=1 | 15.31 / 14.70 | 15.08 / 14.79 | 15.00 / 14.75 |
| A, stock h2, d/16 k=8 | 15.15 / 13.13 | 15.51 / 13.16 | 15.47 / 13.33 |
| A, stock h2, c/P5.4 k=1 | 3.73 / 4.15 | 3.82 / 4.22 | 3.85 / 4.25 |
| A, host-too h2-batch, d/16 k=1 | 7.99 / 8.46 | 8.02 / 9.17 | 7.90 / 9.04 |
| A, host-too h2-batch, d/16 k=8 | 8.20 / 8.51 | 7.65 / 10.01 | 7.47 / 10.22 |
| Cf, p1, d/16 k=1 | 15.33 / 13.33 | 15.83 / 13.55 | 15.56 / 13.37 |
| Cf, p1, d/16 k=8 | 15.45 / 13.08 | 15.33 / 12.99 | 14.69 / 12.97 |
| Cf, p1 + h2-batch, d/16 k=1 | 7.98 / 9.14 | 7.82 / 9.34 | 7.93 / 9.04 |
| Cf, p1 + h2-batch, d/16 k=8 | 8.24 / 8.36 | 8.67 / 8.20 | 7.24 / 8.31 |
| Cf, p1 + h2-batch, c/P5.4 k=8 | 2.22 / 3.08 | 1.93 / 2.87 | 1.87 / 2.73 |

| C++ | blocking | callback | queue |
|---|---|---|---|
| A (grpc++), d/16 k=1 | 7.80 / 9.15 | 8.37 / 8.93 | 7.67 / 11.17 |
| A (grpc++), d/16 k=8 | 9.16 / 8.86 | 8.73 / 9.73 | 8.32 / 9.18 |
| A (grpc++), c/P5.4 k=8 | 2.55 / 3.77 | 2.05 / 3.47 | 2.03 / 3.38 |
| Cf, p1, d/16 k=1 | 15.28 / 13.28 | 15.55 / 13.82 | 15.61 / 13.24 |
| Cf, p1, d/16 k=8 | 15.98 / 13.06 | 15.23 / 12.97 | 13.12 / 12.14 (bimodal) |
| Cf, p1 + h2-batch, d/16 k=1 | 8.01 / 9.23 | 8.01 / 9.43 | 8.57 / 8.69 |
| Cf, p1 + h2-batch, d/16 k=8 | 8.84 / 8.13 | 8.53 / 8.23 | 7.88 / 8.46 |
| Cf, p1 + h2-batch, c/P5.4 k=8 | 2.26 / 2.93 | 1.76 / 2.97 | 1.78 / 2.74 |

Writes per d/16 MiB call: Rust A and every stock-h2 core cell about 1,030 to 1,050;
h2-batch 73 to 89 at k = 1 and 42 to 70 at k = 8; grpc++ 11 to 15. The TCP worker sweeps
(`logs/rust/opt/tcp-sweep/`, `logs/cpp/opt/physical-probe/tcp-sweep/`) move no cell by
more than about 1 ms between 1 and 8 core workers.

**A harness defect found here:** the C++ check-stream grids sent the queue cells' d calls
to the server's unchecked UploadStream since 2026-09-28, so Cf-q was verified by byte
count, not SHA-256, until this run (fixed in `campaign_rpc.cpp`; the new checks pass).
