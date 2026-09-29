# physical probe, segment variant: commit 01d3783a; 2026-09-29T22:29:38Z; host farnsworth
# purpose: FIX-PLAN WP11 (campaign machine), coordinator step 2: direction d (16 MiB, 4 MiB; 2 MiB chunks) and c/P5.4, k = 1 and 8
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; 20 logical CPUs online; NUMA nodes 1
# kernel     6.18.54 (#1-NixOS SMP PREEMPT_DYNAMIC Fri Sep 25 14:35:54 UTC 2026)
# smt        active=1 control=on
# turbo      intel_pstate/no_turbo=1 intel_pstate/status=active min_perf_pct=26 max_perf_pct=100 cpufreq/boost=n/a
# governor   1:performance 2:performance 3:performance 4:performance 11:performance 12:performance 13:performance 14:performance 5:performance 6:performance 7:performance 8:performance 15:performance 16:performance 17:performance 18:performance
# scaling_min_freq (kHz) 1:3300000 2:3300000 3:3300000 4:3300000 11:3300000 12:3300000 13:3300000 14:3300000 5:3300000 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3300000 18:3300000
# scaling_max_freq (kHz) 1:3300000 2:3300000 3:3300000 4:3300000 11:3300000 12:3300000 13:3300000 14:3300000 5:3300000 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3300000 18:3300000
# scaling_cur_freq at start (kHz) 1:3300000 2:3300000 3:3300271 4:3300000 11:3300374 12:3300018 13:3300153 14:3300010 5:3300000 6:3300000 7:3300604 8:3299913 15:3300147 16:3300003 17:3300023 18:3300000
# siblings   1:1,11 2:2,12 3:3,13 4:4,14 11:1,11 12:2,12 13:3,13 14:4,14 5:5,15 6:6,16 7:7,17 8:8,18 15:5,15 16:6,16 17:7,17 18:8,18
# isolation  taskset -c affinity only; /sys/devices/system/cpu/isolated='' nohz_full=''; this process's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope cpuset.cpus.effective=n/a
# cpu sets   CLIENT AK_CPU_CLIENT=1-4,11-14 (taskset -c on every client process); SERVER AK_CPU_SERVER=5-8,15-18; ffi/campaign.machine not edited
# server     shared: state /tmp/ak-physical-s4.state, pid 152128, /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-server/release/rpc_server, affinity 5-8,15-18, AK_SERVER_THREADS=4 -> 4 tokio workers; threads by name: rpc_server x1, tokio-rt-worker x4, socket /tmp/aksrv.lMNX4U/pinned.sock (pinned)
# server core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-server/release/deps/libak_core.so
# client     AK_HOST_WORKERS=8 (tokio workers of each A/D/Df/-cb cell runtime), AK_CORE_WORKERS=8 (ak_runtime_new per core client), AK_CHAN_DEPTH=1 (Df-chan); k caller threads (blocking cells) or tasks; criterion default-features off (no rayon pool); no pool in the client is sized from the affinity mask (every tokio runtime has an explicit worker count)
# client core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (probe), /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (grid); rustc 1.95.0, cargo 1.95.0; probe sha256 012a92798163387e, grid rpc_suite-d45c66326ab63500 sha256 e0e580d3f3c74583
# load       /proc/loadavg at start: 2.04 1.68 1.75 1/846 153621; perf: perf version 7.2.8 (not used), perf_event_paranoid=1
# plan       spread passes 0 (probe cells A,A2,Df; grid cells A,Df), main passes 2 (probe cells A,A2,Df,Cf; grid cells A,Df,Cf), attribution pass cells A,Df,Df-chan,Cf,Cf-cb; probe sizes 16MiB,4MiB, k = 1, rounds 10 x calls 8, warm 4 calls per (cell, size), block order, no /proc; grid dirs c,d payloads P5.4,4MiB,16MiB k 1,8, criterion samples 10, warm-up 200 ms, measurement 800 ms per benchmark, nresamples 1000, AK_RPC_SERVER_WARMUP=8; attribution rounds 4 x calls 4 with /proc reads and the allocation shim (not timed); server warm-up serve.sh warm 16 at the start
# notable    the run-to-run spread of an in-process gap: for each workload, the range (max - min) over the SPREAD passes of the per-pass median of (Df - A), each pass its own client processes against the same server; the A2 - A range (probe, two independent A instances in one process) is the same-code floor beside it. A difference between cells or variants smaller than that range is not attributed. The main passes' own per-gap ranges are reported beside it
# clocks     per-call CPU = process CPU (CLOCK_PROCESS_CPUTIME_ID, every thread of the client): probe the delta around each call, grid per criterion sample / calls; wall beside it; context switches and minor faults per call = getrusage(RUSAGE_SELF) deltas / calls (probe per round, grid per sample)
# benchmark wall time 57 s (spread and main passes, server warm-up included); attribution pass 2 s; scaling_cur_freq at end (kHz): 1:3298564 2:3302223 3:3300012 4:3300166 11:3300115 12:3300291 13:3300594 14:3300645 5:3300506 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3300076 18:3300077; loadavg at end: 1.91 1.73 1.77 1/847 154231

Session: variant (host 8, core 8, server 4). Every figure below is per call, in ms (client process CPU unless marked wall).

## 1. Run-to-run spread of an in-process gap (the notable threshold)

Per pass: the median of each cell's per-call CPU in that pass's process; the gap is the difference of two such medians in the same process. Range = max - min over the passes.

| source | workload | A per pass | Df - A per pass (spread passes) | range | A2 - A per pass | range | Df - A per pass (main passes) | range |
|---|---|---|---|---:|---|---:|---|---:|
| probe | d/16MiB k1 | -- | -- | -- | -- | -- | +0.70 +0.45 | 0.24 |
| probe | d/4MiB k1 | -- | -- | -- | -- | -- | +0.27 +0.36 | 0.09 |
| grid | d/16MiB k1 | -- | -- | -- | -- | -- | +0.23 +0.62 | 0.39 |
| grid | d/4MiB k1 | -- | -- | -- | -- | -- | -0.02 +0.05 | 0.07 |
| grid | d/16MiB k8 | -- | -- | -- | -- | -- | -4.09 -3.29 | 0.80 |
| grid | d/4MiB k8 | -- | -- | -- | -- | -- | -0.91 -1.34 | 0.43 |
| grid | c/P5.4 k1 | -- | -- | -- | -- | -- | +0.02 +0.04 | 0.02 |
| grid | c/P5.4 k8 | -- | -- | -- | -- | -- | -1.07 -0.94 | 0.12 |

## 2a. Main passes, probe (bin stream_probe, block order): direction d, k = 1: CPU median [p10-p90] pooled over 2 passes

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 7.72 [7.41-8.54] | 1.89 [1.82-2.15] |
| A2 | 7.53 [7.27-8.18] | 1.91 [1.84-2.50] |
| Df | 8.31 [8.11-8.65] | 2.24 [1.94-2.57] |
| Cf | 8.24 [7.96-9.03] | 1.99 [1.93-2.13] |

Wall per call, median [p10-p90] (per round, calls back to back):

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 8.87 [8.47-9.06] | 2.44 [2.39-2.49] |
| A2 | 8.92 [8.77-9.21] | 2.44 [2.39-2.47] |
| Df | 8.92 [8.38-9.26] | 2.40 [2.33-2.49] |
| Cf | 7.97 [7.37-8.81] | 2.45 [2.36-2.63] |

In-process gap to A per pass (cell median - A median, same process), and its range over the passes:

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---|---|
| A2 | -0.05 -0.39 (range 0.34) | -0.00 +0.23 (range 0.24) |
| Df | +0.70 +0.45 (range 0.24) | +0.27 +0.36 (range 0.09) |
| Cf | +0.57 +0.53 (range 0.04) | +0.13 +0.09 (range 0.05) |

Context switches (voluntary / involuntary) and minor faults per call, medians over rounds or samples, main passes:

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 83.2 / 0.0 / 0.0 | 16.4 / 0.0 / 0.0 |
| A2 | 66.3 / 0.0 / 0.0 | 17.2 / 0.0 / 0.0 |
| Df | 217.4 / 0.0 / 0.0 | 48.6 / 0.0 / 0.0 |
| Cf | 85.8 / 0.0 / 0.3 | 32.1 / 0.0 / 0.0 |

## 2b. Main passes, grid (benches/rpc_suite on criterion): c/P5.4 and d, k = 1 and 8: CPU median [p10-p90] pooled over 2 passes

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 7.79 [7.65-8.18] | 1.94 [1.89-1.96] | 13.00 [12.48-13.88] | 3.49 [3.21-3.81] | 1.95 [1.92-1.97] | 3.55 [3.39-3.99] |
| Df | 8.23 [8.14-8.41] | 1.95 [1.92-1.98] | 9.41 [9.19-9.72] | 2.36 [2.28-2.42] | 1.99 [1.93-2.01] | 2.54 [2.49-2.59] |
| Cf | 8.10 [7.91-8.55] | 2.05 [2.03-2.08] | 9.88 [9.59-10.61] | 2.58 [2.53-2.62] | 2.01 [1.96-2.04] | 2.54 [2.49-2.64] |

Wall per call, median [p10-p90] (per sample / calls; at k = 8 the batch wall over 8 calls):

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 8.94 [8.41-9.18] | 2.51 [2.40-2.58] | 9.59 [9.13-10.75] | 2.30 [2.19-2.43] | 2.87 [2.78-3.23] | 2.74 [2.52-3.00] |
| Df | 8.68 [8.52-9.39] | 2.47 [2.44-2.54] | 8.19 [7.79-8.91] | 2.15 [2.03-2.35] | 3.45 [2.76-3.94] | 2.64 [2.40-2.82] |
| Cf | 8.37 [7.66-8.72] | 2.30 [2.27-2.50] | 7.77 [7.54-8.54] | 2.16 [2.07-2.34] | 3.13 [2.86-3.43] | 2.42 [2.32-2.74] |

In-process gap to A per pass (cell median - A median, same process), and its range over the passes:

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---|---|---|---|---|---|
| Df | +0.23 +0.62 (range 0.39) | -0.02 +0.05 (range 0.07) | -4.09 -3.29 (range 0.80) | -0.91 -1.34 (range 0.43) | +0.02 +0.04 (range 0.02) | -1.07 -0.94 (range 0.12) |
| Cf | +0.34 +0.28 (range 0.07) | +0.09 +0.15 (range 0.05) | -3.59 -2.87 (range 0.72) | -0.74 -1.07 (range 0.33) | +0.09 +0.03 (range 0.06) | -1.06 -0.96 (range 0.10) |

Context switches (voluntary / involuntary) and minor faults per call, medians over rounds or samples, main passes:

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 89.3 / 0.0 / 0.0 | 15.1 / 0.0 / 0.0 | 120.2 / 0.0 / 1663.5 | 48.2 / 0.0 / 423.7 | 13.0 / 0.0 / 0.0 | 30.3 / 0.0 / 633.5 |
| Df | 185.5 / 0.0 / 0.0 | 16.8 / 0.0 / 0.0 | 112.4 / 0.0 / 0.3 | 33.0 / 0.0 / 0.0 | 15.2 / 0.0 / 0.0 | 18.6 / 0.0 / 0.0 |
| Cf | 74.0 / 0.0 / 0.0 | 29.0 / 0.0 / 0.0 | 88.6 / 0.1 / 96.4 | 21.8 / 0.0 / 0.0 | 18.1 / 0.0 / 0.0 | 21.3 / 0.0 / 0.0 |

Grid samples holding ONE batch (criterion sized one iteration per sample), main passes: A d/16MiB k8: 10

## 3. Client threads by class after the warm-up (each probe process)

- `attr`: threads by class after the warm-up: {"caller": 4, "cell-rt": 32, "core-rt": 16, "main": 1} (host runtimes 4, core runtimes 2)
- `main-probe-1`: threads by class after the warm-up: {"caller": 2, "cell-rt": 24, "core-rt": 8, "main": 1} (host runtimes 3, core runtimes 1)
- `main-probe-2`: threads by class after the warm-up: {"caller": 2, "cell-rt": 24, "core-rt": 8, "main": 1} (host runtimes 3, core runtimes 1)

## 4. Attribution pass (not timed: /proc reads around every round, allocation shim loaded): medians over rounds, per call

| workload | cell | CPU ms by class main/caller/cell-rt/core-rt/other | vcs by class | ics by class | minflt (process) | allocs | allocs >= 1 MiB | host encode us/chunk | send CPU us/chunk | send wall us/chunk |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|
| d/16MiB k1 | A | 0.70/0.00/8.06/0.00/- | 0.25/0.00/74.62/0.00/- | 0.00/0.00/0.00/0.00/- | 192.00 | 55.12 | 8.00 | - | - | - |
| d/16MiB k1 | Df | 0.71/0.00/8.37/0.00/- | 0.25/0.00/225.62/0.00/- | 0.00/0.00/0.00/0.00/- | 0.25 | 62.50 | 0.00 | - | - | - |
| d/16MiB k1 | Df-chan | 0.70/2.51/6.79/0.00/- | 0.25/5.38/135.62/0.00/- | 0.00/0.00/0.00/0.00/- | 128.62 | 66.12 | 1.50 | 299.34 | 8.00 | 317.04 |
| d/16MiB k1 | Cf | 0.71/2.21/0.00/6.34/- | 0.25/5.38/0.00/86.50/- | 0.00/0.00/0.00/0.00/- | 0.25 | 48.88 | 1.50 | - | - | - |
| d/16MiB k1 | Cf-cb | 0.70/0.00/2.56/7.08/- | 0.25/0.00/18.38/202.38/- | 0.00/0.00/0.00/0.00/- | 128.38 | 77.00 | 1.50 | - | - | - |
| d/4MiB k1 | A | 0.71/0.00/2.01/0.00/- | 0.25/0.00/13.12/0.00/- | 0.00/0.00/0.00/0.00/- | 0.00 | 36.50 | 2.00 | - | - | - |
| d/4MiB k1 | Df | 0.71/0.00/2.03/0.00/- | 0.25/0.00/16.62/0.00/- | 0.00/0.00/0.00/0.00/- | 0.62 | 38.50 | 0.00 | - | - | - |
| d/4MiB k1 | Df-chan | 0.70/0.56/1.52/0.00/- | 0.25/1.25/25.25/0.00/- | 0.00/0.00/0.00/0.00/- | 0.12 | 40.25 | 0.00 | 256.20 | 4.25 | 3.97 |
| d/4MiB k1 | Cf | 0.71/0.55/0.00/1.52/- | 0.25/1.25/0.00/29.75/- | 0.00/0.00/0.00/0.00/- | 0.00 | 41.25 | 0.00 | - | - | - |
| d/4MiB k1 | Cf-cb | 0.70/0.00/0.57/1.67/- | 0.25/0.00/6.50/40.12/- | 0.00/0.00/0.00/0.00/- | 0.25 | 51.50 | 0.00 | - | - | - |
