# physical probe, segment variant: commit 01d3783a; 2026-09-29T22:27:39Z; host farnsworth
# purpose: FIX-PLAN WP11 (campaign machine), coordinator step 2: direction d (16 MiB, 4 MiB; 2 MiB chunks) and c/P5.4, k = 1 and 8
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; 20 logical CPUs online; NUMA nodes 1
# kernel     6.18.54 (#1-NixOS SMP PREEMPT_DYNAMIC Fri Sep 25 14:35:54 UTC 2026)
# smt        active=1 control=on
# turbo      intel_pstate/no_turbo=1 intel_pstate/status=active min_perf_pct=26 max_perf_pct=100 cpufreq/boost=n/a
# governor   1:performance 2:performance 3:performance 4:performance 11:performance 12:performance 13:performance 14:performance 5:performance 6:performance 7:performance 8:performance 15:performance 16:performance 17:performance 18:performance
# scaling_min_freq (kHz) 1:3300000 2:3300000 3:3300000 4:3300000 11:3300000 12:3300000 13:3300000 14:3300000 5:3300000 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3300000 18:3300000
# scaling_max_freq (kHz) 1:3300000 2:3300000 3:3300000 4:3300000 11:3300000 12:3300000 13:3300000 14:3300000 5:3300000 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3300000 18:3300000
# scaling_cur_freq at start (kHz) 1:3300000 2:3300000 3:3300000 4:3300818 11:3300010 12:3300165 13:3300000 14:3300046 5:3300000 6:3305116 7:3300011 8:3299533 15:3300850 16:3300042 17:3300000 18:3300000
# siblings   1:1,11 2:2,12 3:3,13 4:4,14 11:1,11 12:2,12 13:3,13 14:4,14 5:5,15 6:6,16 7:7,17 8:8,18 15:5,15 16:6,16 17:7,17 18:8,18
# isolation  taskset -c affinity only; /sys/devices/system/cpu/isolated='' nohz_full=''; this process's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope cpuset.cpus.effective=n/a
# cpu sets   CLIENT AK_CPU_CLIENT=1-4,11-14 (taskset -c on every client process); SERVER AK_CPU_SERVER=5-8,15-18; ffi/campaign.machine not edited
# server     shared: state /tmp/ak-physical-s4.state, pid 152128, /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-server/release/rpc_server, affinity 5-8,15-18, AK_SERVER_THREADS=4 -> 4 tokio workers; threads by name: rpc_server x1, tokio-rt-worker x4, socket /tmp/aksrv.lMNX4U/pinned.sock (pinned)
# server core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-server/release/deps/libak_core.so
# client     AK_HOST_WORKERS=4 (tokio workers of each A/D/Df/-cb cell runtime), AK_CORE_WORKERS=4 (ak_runtime_new per core client), AK_CHAN_DEPTH=1 (Df-chan); k caller threads (blocking cells) or tasks; criterion default-features off (no rayon pool); no pool in the client is sized from the affinity mask (every tokio runtime has an explicit worker count)
# client core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (probe), /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (grid); rustc 1.95.0, cargo 1.95.0; probe sha256 012a92798163387e, grid rpc_suite-d45c66326ab63500 sha256 e0e580d3f3c74583
# load       /proc/loadavg at start: 0.48 1.31 1.65 1/853 152559; perf: perf version 7.2.8 (not used), perf_event_paranoid=1
# plan       spread passes 0 (probe cells A,A2,Df; grid cells A,Df), main passes 3 (probe cells A,A2,Df,Df-chan,Cf,Cf-cb; grid cells A,Df,Cf,Cf-cb), attribution pass cells A,Df,Df-chan,Cf,Cf-cb; probe sizes 16MiB,4MiB, k = 1, rounds 10 x calls 8, warm 4 calls per (cell, size), block order, no /proc; grid dirs c,d payloads P5.4,4MiB,16MiB k 1,8, criterion samples 10, warm-up 200 ms, measurement 800 ms per benchmark, nresamples 1000, AK_RPC_SERVER_WARMUP=8; attribution rounds 4 x calls 4 with /proc reads and the allocation shim (not timed); server warm-up serve.sh warm 16 at the start
# notable    the run-to-run spread of an in-process gap: for each workload, the range (max - min) over the SPREAD passes of the per-pass median of (Df - A), each pass its own client processes against the same server; the A2 - A range (probe, two independent A instances in one process) is the same-code floor beside it. A difference between cells or variants smaller than that range is not attributed. The main passes' own per-gap ranges are reported beside it
# clocks     per-call CPU = process CPU (CLOCK_PROCESS_CPUTIME_ID, every thread of the client): probe the delta around each call, grid per criterion sample / calls; wall beside it; context switches and minor faults per call = getrusage(RUSAGE_SELF) deltas / calls (probe per round, grid per sample)
# benchmark wall time 115 s (spread and main passes, server warm-up included); attribution pass 2 s; scaling_cur_freq at end (kHz): 1:3300657 2:3298946 3:3300533 4:3300014 11:3299992 12:3300000 13:3300017 14:3300159 5:3300000 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3299913 18:3300000; loadavg at end: 2.04 1.68 1.75 1/847 153276

Session: variant (host 4, core 4, server 4). Every figure below is per call, in ms (client process CPU unless marked wall).

## 1. Run-to-run spread of an in-process gap (the notable threshold)

Per pass: the median of each cell's per-call CPU in that pass's process; the gap is the difference of two such medians in the same process. Range = max - min over the passes.

| source | workload | A per pass | Df - A per pass (spread passes) | range | A2 - A per pass | range | Df - A per pass (main passes) | range |
|---|---|---|---|---:|---|---:|---|---:|
| probe | d/16MiB k1 | -- | -- | -- | -- | -- | +0.61 +0.73 +0.78 | 0.16 |
| probe | d/4MiB k1 | -- | -- | -- | -- | -- | +0.10 +0.08 +0.02 | 0.08 |
| grid | d/16MiB k1 | -- | -- | -- | -- | -- | +0.42 +0.48 +0.90 | 0.48 |
| grid | d/4MiB k1 | -- | -- | -- | -- | -- | +0.08 +0.09 +0.07 | 0.02 |
| grid | d/16MiB k8 | -- | -- | -- | -- | -- | -4.58 -3.67 -4.26 | 0.92 |
| grid | d/4MiB k8 | -- | -- | -- | -- | -- | -0.86 -0.31 -0.68 | 0.55 |
| grid | c/P5.4 k1 | -- | -- | -- | -- | -- | +0.02 +0.04 +0.05 | 0.03 |
| grid | c/P5.4 k8 | -- | -- | -- | -- | -- | -1.83 -1.26 -1.00 | 0.83 |

## 2a. Main passes, probe (bin stream_probe, block order): direction d, k = 1: CPU median [p10-p90] pooled over 3 passes

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 7.71 [7.36-8.17] | 1.94 [1.84-2.29] |
| A2 | 7.83 [7.43-8.77] | 1.96 [1.86-2.36] |
| Df | 8.41 [8.13-9.66] | 2.01 [1.88-2.32] |
| Df-chan | 8.70 [8.30-9.83] | 2.03 [1.92-2.24] |
| Cf | 8.07 [7.88-8.29] | 2.03 [1.94-2.17] |
| Cf-cb | 9.58 [8.70-10.88] | 2.19 [2.07-2.51] |

Wall per call, median [p10-p90] (per round, calls back to back):

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 8.99 [8.62-10.25] | 2.47 [2.40-3.38] |
| A2 | 8.91 [8.22-10.03] | 2.45 [2.34-3.66] |
| Df | 8.78 [8.42-11.64] | 2.49 [2.43-4.77] |
| Df-chan | 8.53 [7.66-10.90] | 2.38 [2.32-3.43] |
| Cf | 8.34 [7.52-8.70] | 2.45 [2.36-3.77] |
| Cf-cb | 8.50 [7.98-9.48] | 2.50 [2.32-3.61] |

In-process gap to A per pass (cell median - A median, same process), and its range over the passes:

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---|---|
| A2 | -0.10 +0.23 +0.28 (range 0.39) | +0.02 +0.05 +0.03 (range 0.03) |
| Df | +0.61 +0.73 +0.78 (range 0.16) | +0.10 +0.08 +0.02 (range 0.08) |
| Df-chan | +1.32 +0.91 +1.04 (range 0.41) | +0.16 +0.05 +0.06 (range 0.11) |
| Cf | +0.28 +0.40 +0.49 (range 0.21) | +0.11 +0.05 +0.13 (range 0.08) |
| Cf-cb | +1.71 +1.92 +2.07 (range 0.36) | +0.29 +0.18 +0.32 (range 0.13) |

Context switches (voluntary / involuntary) and minor faults per call, medians over rounds or samples, main passes:

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 77.1 / 0.0 / 0.0 | 17.2 / 0.0 / 0.0 |
| A2 | 91.7 / 0.0 / 0.0 | 15.6 / 0.0 / 0.0 |
| Df | 217.8 / 0.0 / 0.1 | 20.7 / 0.0 / 0.0 |
| Df-chan | 139.1 / 0.0 / 0.2 | 27.1 / 0.0 / 0.0 |
| Cf | 76.7 / 0.0 / 0.0 | 33.8 / 0.0 / 0.0 |
| Cf-cb | 234.0 / 0.0 / 94.4 | 51.2 / 0.0 / 0.0 |

## 2b. Main passes, grid (benches/rpc_suite on criterion): c/P5.4 and d, k = 1 and 8: CPU median [p10-p90] pooled over 3 passes

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 7.69 [7.58-7.81] | 1.91 [1.88-1.96] | 13.05 [11.60-14.33] | 2.96 [2.84-3.36] | 1.93 [1.90-1.98] | 3.56 [3.09-4.18] |
| Df | 8.22 [8.04-8.58] | 1.98 [1.96-2.01] | 9.20 [8.69-9.61] | 2.33 [2.15-2.61] | 1.97 [1.92-2.01] | 2.14 [2.09-2.29] |
| Cf | 8.35 [8.07-8.94] | 2.07 [2.04-2.12] | 9.66 [9.39-9.99] | 2.54 [2.49-2.65] | 2.03 [1.98-2.09] | 2.52 [2.45-2.60] |
| Cf-cb | 9.57 [9.02-10.04] | 2.17 [2.13-2.26] | 9.80 [9.54-10.29] | 2.39 [2.37-2.43] | 2.03 [1.98-2.09] | 2.29 [2.27-2.37] |

Wall per call, median [p10-p90] (per sample / calls; at k = 8 the batch wall over 8 calls):

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 8.95 [8.32-9.21] | 2.45 [2.40-2.54] | 9.25 [8.74-9.81] | 2.23 [2.10-2.29] | 2.78 [2.74-3.23] | 2.71 [2.38-2.94] |
| Df | 8.74 [8.35-9.26] | 2.53 [2.46-2.58] | 8.31 [7.97-8.91] | 2.14 [1.99-2.73] | 2.98 [2.77-3.22] | 2.49 [2.23-2.67] |
| Cf | 7.82 [7.42-8.56] | 2.35 [2.29-2.50] | 8.13 [7.54-8.48] | 2.17 [2.04-2.32] | 3.14 [2.98-3.74] | 2.42 [2.28-2.59] |
| Cf-cb | 8.45 [7.93-8.76] | 2.34 [2.29-2.50] | 8.13 [7.71-8.45] | 2.11 [1.97-2.25] | 3.11 [2.88-3.56] | 2.49 [2.26-2.66] |

In-process gap to A per pass (cell median - A median, same process), and its range over the passes:

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---|---|---|---|---|---|
| Df | +0.42 +0.48 +0.90 (range 0.48) | +0.08 +0.09 +0.07 (range 0.02) | -4.58 -3.67 -4.26 (range 0.92) | -0.86 -0.31 -0.68 (range 0.55) | +0.02 +0.04 +0.05 (range 0.03) | -1.83 -1.26 -1.00 (range 0.83) |
| Cf | +0.48 +0.62 +1.12 (range 0.64) | +0.20 +0.16 +0.15 (range 0.05) | -4.06 -3.49 -3.33 (range 0.73) | -0.46 -0.34 -0.50 (range 0.16) | +0.07 +0.12 +0.10 (range 0.05) | -1.43 -0.90 -0.63 (range 0.80) |
| Cf-cb | +1.88 +1.84 +1.95 (range 0.11) | +0.29 +0.29 +0.25 (range 0.04) | -4.07 -3.12 -3.24 (range 0.95) | -0.63 -0.47 -0.64 (range 0.18) | +0.11 +0.11 +0.07 (range 0.04) | -1.64 -1.13 -0.86 (range 0.78) |

Context switches (voluntary / involuntary) and minor faults per call, medians over rounds or samples, main passes:

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 69.7 / 0.0 / 0.0 | 13.6 / 0.0 / 0.0 | 135.8 / 0.0 / 1710.8 | 18.7 / 0.0 / 392.8 | 12.9 / 0.0 / 0.0 | 30.3 / 0.0 / 602.4 |
| Df | 197.7 / 0.0 / 0.0 | 19.8 / 0.0 / 0.0 | 99.7 / 0.0 / 0.0 | 39.6 / 0.0 / 0.0 | 15.3 / 0.0 / 0.0 | 16.1 / 0.0 / 0.0 |
| Cf | 80.6 / 0.0 / 0.3 | 29.3 / 0.0 / 0.0 | 75.0 / 0.1 / 65.0 | 20.5 / 0.0 / 0.0 | 20.7 / 0.0 / 0.0 | 20.0 / 0.0 / 0.0 |
| Cf-cb | 221.7 / 0.0 / 0.0 | 42.0 / 0.0 / 0.0 | 125.2 / 0.0 / 128.5 | 29.4 / 0.0 / 0.0 | 19.2 / 0.0 / 0.0 | 19.8 / 0.0 / 0.0 |

Grid samples holding ONE batch (criterion sized one iteration per sample), main passes: A d/16MiB k8: 10

## 3. Client threads by class after the warm-up (each probe process)

- `attr`: threads by class after the warm-up: {"caller": 4, "cell-rt": 16, "core-rt": 8, "main": 1} (host runtimes 4, core runtimes 2)
- `main-probe-1`: threads by class after the warm-up: {"caller": 4, "cell-rt": 20, "core-rt": 8, "main": 1} (host runtimes 5, core runtimes 2)
- `main-probe-2`: threads by class after the warm-up: {"caller": 4, "cell-rt": 20, "core-rt": 8, "main": 1} (host runtimes 5, core runtimes 2)
- `main-probe-3`: threads by class after the warm-up: {"caller": 4, "cell-rt": 20, "core-rt": 8, "main": 1} (host runtimes 5, core runtimes 2)

## 4. Attribution pass (not timed: /proc reads around every round, allocation shim loaded): medians over rounds, per call

| workload | cell | CPU ms by class main/caller/cell-rt/core-rt/other | vcs by class | ics by class | minflt (process) | allocs | allocs >= 1 MiB | host encode us/chunk | send CPU us/chunk | send wall us/chunk |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|
| d/16MiB k1 | A | 0.40/0.00/8.04/0.00/- | 0.25/0.00/68.75/0.00/- | 0.00/0.00/0.00/0.00/- | 252.88 | 55.00 | 8.00 | - | - | - |
| d/16MiB k1 | Df | 0.40/0.00/8.31/0.00/- | 0.25/0.00/196.50/0.00/- | 0.00/0.00/0.00/0.00/- | 0.25 | 62.50 | 0.00 | - | - | - |
| d/16MiB k1 | Df-chan | 0.40/2.71/6.69/0.00/- | 0.25/5.12/135.12/0.00/- | 0.00/0.00/0.00/0.00/- | 253.00 | 66.12 | 1.62 | 325.14 | 7.94 | 271.60 |
| d/16MiB k1 | Cf | 0.40/2.18/0.00/6.53/- | 0.25/5.50/0.00/119.50/- | 0.00/0.00/0.00/0.12/- | 0.38 | 49.00 | 1.62 | - | - | - |
| d/16MiB k1 | Cf-cb | 0.40/0.00/2.66/7.15/- | 0.25/0.00/18.50/241.75/- | 0.00/0.00/0.00/0.00/- | 124.88 | 77.50 | 1.88 | - | - | - |
| d/4MiB k1 | A | 0.40/0.00/1.95/0.00/- | 0.25/0.00/13.25/0.00/- | 0.00/0.00/0.00/0.00/- | 0.00 | 36.50 | 2.00 | - | - | - |
| d/4MiB k1 | Df | 0.40/0.00/2.01/0.00/- | 0.25/0.00/20.50/0.00/- | 0.00/0.00/0.00/0.00/- | 0.12 | 38.50 | 0.00 | - | - | - |
| d/4MiB k1 | Df-chan | 0.40/0.54/1.61/0.00/- | 0.25/1.25/25.88/0.00/- | 0.00/0.00/0.00/0.00/- | 0.00 | 40.25 | 0.00 | 250.29 | 4.75 | 4.21 |
| d/4MiB k1 | Cf | 0.40/0.54/0.00/1.48/- | 0.25/1.25/0.00/25.88/- | 0.00/0.00/0.00/0.00/- | 0.00 | 41.25 | 0.00 | - | - | - |
| d/4MiB k1 | Cf-cb | 0.40/0.00/0.59/1.65/- | 0.25/0.00/6.50/31.88/- | 0.00/0.00/0.00/0.00/- | 0.00 | 51.50 | 0.00 | - | - | - |
