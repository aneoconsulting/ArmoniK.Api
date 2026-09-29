# physical probe, segment main: commit f57173ff; 2026-09-29T22:14:56Z; host farnsworth
# purpose: FIX-PLAN WP11 (campaign machine), coordinator step 2: direction d (16 MiB, 4 MiB; 2 MiB chunks) and c/P5.4, k = 1 and 8
# cpu        Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; 20 logical CPUs online; NUMA nodes 1
# kernel     6.18.54 (#1-NixOS SMP PREEMPT_DYNAMIC Fri Sep 25 14:35:54 UTC 2026)
# smt        active=1 control=on
# turbo      intel_pstate/no_turbo=1 intel_pstate/status=active min_perf_pct=26 max_perf_pct=100 cpufreq/boost=n/a
# governor   1:performance 2:performance 3:performance 4:performance 11:performance 12:performance 13:performance 14:performance 5:performance 6:performance 7:performance 8:performance 15:performance 16:performance 17:performance 18:performance
# scaling_min_freq (kHz) 1:3300000 2:3300000 3:3300000 4:3300000 11:3300000 12:3300000 13:3300000 14:3300000 5:3300000 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3300000 18:3300000
# scaling_max_freq (kHz) 1:3300000 2:3300000 3:3300000 4:3300000 11:3300000 12:3300000 13:3300000 14:3300000 5:3300000 6:3300000 7:3300000 8:3300000 15:3300000 16:3300000 17:3300000 18:3300000
# scaling_cur_freq at start (kHz) 1:3300346 2:3300057 3:3300000 4:3303245 11:3300000 12:3300000 13:3300000 14:3299740 5:3300098 6:3300000 7:3299129 8:3300000 15:3300000 16:3300000 17:3300971 18:3300685
# siblings   1:1,11 2:2,12 3:3,13 4:4,14 11:1,11 12:2,12 13:3,13 14:4,14 5:5,15 6:6,16 7:7,17 8:8,18 15:5,15 16:6,16 17:7,17 18:8,18
# isolation  taskset -c affinity only; /sys/devices/system/cpu/isolated='' nohz_full=''; this process's cgroup /user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.konsole-4514.scope/tab(4529).scope cpuset.cpus.effective=n/a
# cpu sets   CLIENT AK_CPU_CLIENT=1-4,11-14 (taskset -c on every client process); SERVER AK_CPU_SERVER=5-8,15-18; ffi/campaign.machine not edited
# server     shared: state /tmp/ak-physical-s8.state, pid 147567, /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-server/release/rpc_server, affinity 5-8,15-18, AK_SERVER_THREADS=8 -> 8 tokio workers; threads by name: rpc_server x1, tokio-rt-worker x8, socket /tmp/aksrv.qhnExV/pinned.sock (pinned)
# server core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target-server/release/deps/libak_core.so
# client     AK_HOST_WORKERS=8 (tokio workers of each A/D/Df/-cb cell runtime), AK_CORE_WORKERS=8 (ak_runtime_new per core client), AK_CHAN_DEPTH=1 (Df-chan); k caller threads (blocking cells) or tasks; criterion default-features off (no rayon pool); no pool in the client is sized from the affinity mask (every tokio runtime has an explicit worker count)
# client core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (probe), /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so (grid); rustc 1.95.0, cargo 1.95.0; probe sha256 012a92798163387e, grid rpc_suite-d45c66326ab63500 sha256 e0e580d3f3c74583
# load       /proc/loadavg at start: 0.28 0.49 1.72 5/859 147996; perf: perf version 7.2.8 (not used), perf_event_paranoid=1
# plan       spread passes 4 (probe cells A,A2,Df; grid cells A,Df), main passes 3 (probe cells A,A2,Df,Df-chan,Cf,Cf-cb,C,C-cb; grid cells A,Df,Cf,Cf-cb,C,C-cb), attribution pass cells A,Df,Df-chan,Cf,Cf-cb,C,C-cb,Cf-split,Cf-cb-split; probe sizes 16MiB,4MiB, k = 1, rounds 10 x calls 8, warm 4 calls per (cell, size), block order, no /proc; grid dirs c,d payloads P5.4,4MiB,16MiB k 1,8, criterion samples 10, warm-up 200 ms, measurement 800 ms per benchmark, nresamples 1000, AK_RPC_SERVER_WARMUP=8; attribution rounds 4 x calls 4 with /proc reads and the allocation shim (not timed); server warm-up serve.sh warm 16 at the start
# notable    the run-to-run spread of an in-process gap: for each workload, the range (max - min) over the SPREAD passes of the per-pass median of (Df - A), each pass its own client processes against the same server; the A2 - A range (probe, two independent A instances in one process) is the same-code floor beside it. A difference between cells or variants smaller than that range is not attributed. The main passes' own per-gap ranges are reported beside it
# clocks     per-call CPU = process CPU (CLOCK_PROCESS_CPUTIME_ID, every thread of the client): probe the delta around each call, grid per criterion sample / calls; wall beside it; context switches and minor faults per call = getrusage(RUSAGE_SELF) deltas / calls (probe per round, grid per sample)
# benchmark wall time 242 s (spread and main passes, server warm-up included); attribution pass 4 s; scaling_cur_freq at end (kHz): 1:3300804 2:3301236 3:3300397 4:3300326 11:3300548 12:3300402 13:3300000 14:3301129 5:3300000 6:3300000 7:3300000 8:3300000 15:3300928 16:3300000 17:3300000 18:3300000; loadavg at end: 2.47 1.63 1.90 2/857 149815

Session: main (host 8, core 8, server 8). Every figure below is per call, in ms (client process CPU unless marked wall).

## 1. Run-to-run spread of an in-process gap (the notable threshold)

Per pass: the median of each cell's per-call CPU in that pass's process; the gap is the difference of two such medians in the same process. Range = max - min over the passes.

| source | workload | A per pass | Df - A per pass (spread passes) | range | A2 - A per pass | range | Df - A per pass (main passes) | range |
|---|---|---|---|---:|---|---:|---|---:|
| probe | d/16MiB k1 | 11.42 11.57 10.45 10.77 | -2.52 -3.29 -2.02 -2.08 | 1.26 | -2.39 -0.33 +0.83 +0.45 | 3.21 | +0.18 +0.93 -0.12 | 1.05 |
| probe | d/4MiB k1 | 2.00 1.91 1.92 1.95 | +0.07 +0.04 +0.02 +0.22 | 0.20 | -0.01 +0.20 +0.37 -0.08 | 0.45 | +0.21 +0.10 -0.08 | 0.29 |
| grid | d/16MiB k1 | 11.29 7.95 7.82 7.47 | -2.99 +0.88 +0.41 +0.92 | 3.90 | -- | -- | +0.80 +0.81 +1.02 | 0.22 |
| grid | d/4MiB k1 | 1.91 1.93 1.91 1.87 | +0.06 +0.26 +0.02 +0.12 | 0.24 | -- | -- | +0.07 +0.05 -0.16 | 0.23 |
| grid | d/16MiB k8 | 13.65 12.57 13.15 12.74 | -4.13 -2.84 -3.51 -3.05 | 1.29 | -- | -- | -3.69 -3.51 -3.32 | 0.37 |
| grid | d/4MiB k8 | 3.68 3.12 3.29 3.33 | -1.34 -0.83 -0.95 -1.00 | 0.51 | -- | -- | -0.81 -0.93 -0.77 | 0.17 |
| grid | c/P5.4 k1 | 1.97 2.08 1.97 1.98 | +0.01 -0.08 -0.01 -0.02 | 0.09 | -- | -- | +0.01 +0.01 +0.01 | 0.01 |
| grid | c/P5.4 k8 | 3.61 3.70 3.52 3.48 | -1.02 -1.09 -0.94 -0.88 | 0.21 | -- | -- | -0.81 -0.99 -0.96 | 0.19 |

## 2a. Main passes, probe (bin stream_probe, block order): direction d, k = 1: CPU median [p10-p90] pooled over 3 passes

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 7.95 [7.45-8.73] | 1.93 [1.85-2.35] |
| A2 | 7.71 [7.41-8.40] | 1.88 [1.79-2.21] |
| Df | 8.29 [7.91-9.68] | 1.96 [1.86-2.55] |
| Df-chan | 8.50 [8.16-9.21] | 2.04 [1.95-2.12] |
| Cf | 8.04 [7.85-8.48] | 2.06 [1.94-2.40] |
| Cf-cb | 9.25 [8.56-10.53] | 2.19 [2.09-2.48] |
| C | 9.73 [9.33-10.92] | 2.41 [2.33-2.68] |
| C-cb | 11.33 [10.43-12.68] | 2.60 [2.49-3.07] |

Wall per call, median [p10-p90] (per round, calls back to back):

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 8.39 [8.18-8.98] | 2.35 [2.29-2.46] |
| A2 | 8.65 [8.35-9.02] | 2.41 [2.35-2.47] |
| Df | 8.52 [8.35-8.88] | 2.38 [2.31-2.46] |
| Df-chan | 7.95 [7.56-8.40] | 2.33 [2.24-2.44] |
| Cf | 7.93 [7.26-8.56] | 2.34 [2.22-2.53] |
| Cf-cb | 7.95 [7.54-8.68] | 2.45 [2.26-2.54] |
| C | 9.10 [8.88-9.37] | 2.78 [2.67-2.84] |
| C-cb | 8.83 [8.52-9.17] | 2.77 [2.62-2.85] |

In-process gap to A per pass (cell median - A median, same process), and its range over the passes:

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---|---|
| A2 | -0.21 +0.09 -0.43 (range 0.52) | -0.03 -0.00 -0.14 (range 0.14) |
| Df | +0.18 +0.93 -0.12 (range 1.05) | +0.21 +0.10 -0.08 (range 0.29) |
| Df-chan | +0.15 +0.84 +0.67 (range 0.70) | +0.07 +0.12 +0.10 (range 0.05) |
| Cf | -0.14 +0.40 -0.10 (range 0.54) | +0.04 +0.48 +0.07 (range 0.44) |
| Cf-cb | +1.13 +1.53 +1.16 (range 0.40) | +0.17 +0.33 +0.24 (range 0.16) |
| C | +1.48 +2.20 +1.50 (range 0.72) | +0.44 +0.61 +0.40 (range 0.22) |
| C-cb | +2.95 +3.72 +3.36 (range 0.77) | +0.66 +0.70 +0.63 (range 0.07) |

Context switches (voluntary / involuntary) and minor faults per call, medians over rounds or samples, main passes:

| cell | d/16MiB k1 | d/4MiB k1 |
|---|---:|---:|
| A | 125.7 / 0.0 / 0.0 | 15.9 / 0.0 / 0.0 |
| A2 | 86.0 / 0.0 / 0.0 | 13.8 / 0.0 / 0.0 |
| Df | 229.5 / 0.0 / 0.1 | 23.8 / 0.0 / 0.0 |
| Df-chan | 134.7 / 0.0 / 0.0 | 29.9 / 0.0 / 0.0 |
| Cf | 67.6 / 0.0 / 0.0 | 31.0 / 0.0 / 0.0 |
| Cf-cb | 214.6 / 0.0 / 188.4 | 50.6 / 0.0 / 0.0 |
| C | 103.6 / 0.0 / 0.0 | 28.2 / 0.0 / 0.0 |
| C-cb | 253.2 / 0.0 / 0.1 | 45.1 / 0.0 / 0.0 |

## 2b. Main passes, grid (benches/rpc_suite on criterion): c/P5.4 and d, k = 1 and 8: CPU median [p10-p90] pooled over 3 passes

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 7.62 [7.46-7.93] | 1.94 [1.89-2.23] | 12.73 [11.66-13.75] | 3.20 [2.93-3.45] | 1.95 [1.89-2.00] | 3.51 [3.29-3.84] |
| Df | 8.54 [8.19-9.58] | 1.99 [1.96-2.10] | 9.36 [9.14-9.73] | 2.33 [2.26-2.41] | 1.96 [1.92-2.00] | 2.55 [2.51-2.59] |
| Cf | 8.32 [8.18-8.75] | 2.07 [1.97-2.12] | 9.91 [9.52-10.06] | 2.59 [2.55-2.67] | 2.04 [2.00-2.08] | 2.55 [2.49-2.62] |
| Cf-cb | 9.74 [8.99-10.23] | 2.18 [2.10-2.33] | 10.35 [9.98-10.82] | 2.49 [2.44-2.55] | 2.05 [2.01-2.09] | 2.64 [2.62-2.69] |
| C | 10.63 [9.42-11.44] | 2.59 [2.47-2.74] | 16.86 [15.86-17.79] | 4.17 [3.87-4.57] | 2.46 [2.41-2.54] | 3.23 [3.11-3.42] |
| C-cb | 11.52 [10.78-12.68] | 2.77 [2.69-2.84] | 16.81 [15.79-18.77] | 4.16 [3.94-4.35] | 2.44 [2.39-2.53] | 4.38 [4.01-4.64] |

Wall per call, median [p10-p90] (per sample / calls; at k = 8 the batch wall over 8 calls):

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 8.76 [8.45-9.12] | 2.42 [2.36-2.57] | 9.87 [9.37-10.75] | 2.28 [2.20-2.39] | 3.26 [2.74-4.77] | 2.66 [2.48-2.91] |
| Df | 8.85 [8.42-9.24] | 2.51 [2.46-2.56] | 8.62 [8.41-8.88] | 2.23 [2.14-2.36] | 2.96 [2.74-3.83] | 2.65 [2.55-2.82] |
| Cf | 7.59 [7.34-8.50] | 2.42 [2.32-2.55] | 8.36 [8.17-8.65] | 2.29 [2.24-2.37] | 3.24 [2.97-4.62] | 2.59 [2.47-2.72] |
| Cf-cb | 8.24 [7.84-8.74] | 2.36 [2.32-2.45] | 8.36 [8.16-8.61] | 2.21 [2.14-2.33] | 3.52 [3.01-4.44] | 2.67 [2.52-2.79] |
| C | 8.85 [8.59-9.15] | 2.78 [2.64-2.92] | 10.24 [9.50-10.83] | 2.43 [2.34-2.55] | 3.90 [3.63-4.32] | 2.66 [2.55-2.86] |
| C-cb | 8.81 [8.38-9.26] | 2.84 [2.63-2.92] | 9.57 [9.19-10.13] | 2.44 [2.32-2.49] | 4.10 [3.24-5.66] | 2.83 [2.58-2.97] |

In-process gap to A per pass (cell median - A median, same process), and its range over the passes:

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---|---|---|---|---|---|
| Df | +0.80 +0.81 +1.02 (range 0.22) | +0.07 +0.05 -0.16 (range 0.23) | -3.69 -3.51 -3.32 (range 0.37) | -0.81 -0.93 -0.77 (range 0.17) | +0.01 +0.01 +0.01 (range 0.01) | -0.81 -0.99 -0.96 (range 0.19) |
| Cf | +0.73 +0.54 +0.75 (range 0.21) | +0.19 +0.17 -0.17 (range 0.35) | -3.07 -3.05 -2.81 (range 0.26) | -0.56 -0.71 -0.44 (range 0.27) | +0.09 +0.08 +0.11 (range 0.03) | -0.80 -0.98 -0.99 (range 0.19) |
| Cf-cb | +2.26 +1.97 +2.03 (range 0.28) | +0.28 +0.20 +0.13 (range 0.15) | -2.67 -2.72 -2.02 (range 0.71) | -0.66 -0.78 -0.58 (range 0.19) | +0.08 +0.13 +0.07 (range 0.05) | -0.72 -0.86 -0.89 (range 0.16) |
| C | +3.57 +1.77 +3.20 (range 1.80) | +0.60 +0.71 +0.34 (range 0.37) | +3.75 +3.98 +4.31 (range 0.56) | +1.22 +0.75 +1.03 (range 0.47) | +0.57 +0.50 +0.48 (range 0.08) | -0.09 -0.36 -0.25 (range 0.27) |
| C-cb | +4.29 +3.71 +3.87 (range 0.58) | +0.86 +0.82 +0.65 (range 0.21) | +3.29 +5.15 +4.08 (range 1.86) | +0.94 +0.85 +1.15 (range 0.30) | +0.48 +0.57 +0.47 (range 0.10) | +0.99 +0.95 +0.82 (range 0.17) |

Context switches (voluntary / involuntary) and minor faults per call, medians over rounds or samples, main passes:

| cell | d/16MiB k1 | d/4MiB k1 | d/16MiB k8 | d/4MiB k8 | c/P5.4 k1 | c/P5.4 k8 |
|---|---:|---:|---:|---:|---:|---:|
| A | 70.3 / 0.0 / 0.0 | 14.9 / 0.0 / 0.0 | 112.4 / 0.0 / 1710.8 | 43.7 / 0.0 / 329.6 | 15.4 / 0.0 / 0.0 | 36.3 / 0.0 / 572.4 |
| Df | 240.4 / 0.0 / 0.0 | 21.2 / 0.0 / 0.0 | 98.7 / 0.0 / 0.2 | 30.4 / 0.0 / 0.0 | 14.0 / 0.0 / 0.0 | 18.0 / 0.0 / 0.0 |
| Cf | 97.0 / 0.0 / 0.0 | 32.1 / 0.0 / 0.0 | 74.5 / 0.0 / 124.8 | 22.4 / 0.0 / 0.1 | 19.2 / 0.0 / 0.0 | 22.0 / 0.0 / 0.0 |
| Cf-cb | 261.8 / 0.0 / 99.3 | 44.6 / 0.0 / 0.0 | 104.9 / 0.1 / 346.0 | 26.4 / 0.1 / 0.0 | 18.8 / 0.0 / 0.0 | 20.3 / 0.0 / 0.1 |
| C | 163.9 / 0.0 / 0.0 | 28.8 / 0.0 / 0.0 | 107.2 / 0.1 / 1746.1 | 40.7 / 0.0 / 282.5 | 20.9 / 0.0 / 0.0 | 36.5 / 0.0 / 0.0 |
| C-cb | 285.2 / 0.0 / 0.0 | 45.0 / 0.0 / 0.0 | 151.2 / 0.2 / 1583.0 | 36.8 / 0.1 / 206.4 | 19.1 / 0.0 / 0.0 | 47.5 / 0.0 / 427.9 |

Grid samples holding ONE batch (criterion sized one iteration per sample), main passes: A d/16MiB k8: 40, C d/16MiB k8: 30, C-cb d/16MiB k8: 10

## 3. Client threads by class after the warm-up (each probe process)

- `attr`: threads by class after the warm-up: {"caller": 8, "cell-rt": 48, "core-rt": 48, "main": 1} (host runtimes 6, core runtimes 6)
- `main-probe-1`: threads by class after the warm-up: {"caller": 6, "cell-rt": 48, "core-rt": 32, "main": 1} (host runtimes 6, core runtimes 4)
- `main-probe-2`: threads by class after the warm-up: {"caller": 6, "cell-rt": 48, "core-rt": 32, "main": 1} (host runtimes 6, core runtimes 4)
- `main-probe-3`: threads by class after the warm-up: {"caller": 6, "cell-rt": 48, "core-rt": 32, "main": 1} (host runtimes 6, core runtimes 4)
- `spread-probe-1`: threads by class after the warm-up: {"cell-rt": 24, "main": 1} (host runtimes 3, core runtimes 0)
- `spread-probe-2`: threads by class after the warm-up: {"cell-rt": 24, "main": 1} (host runtimes 3, core runtimes 0)
- `spread-probe-3`: threads by class after the warm-up: {"cell-rt": 24, "main": 1} (host runtimes 3, core runtimes 0)
- `spread-probe-4`: threads by class after the warm-up: {"cell-rt": 24, "main": 1} (host runtimes 3, core runtimes 0)

## 4. Attribution pass (not timed: /proc reads around every round, allocation shim loaded): medians over rounds, per call

| workload | cell | CPU ms by class main/caller/cell-rt/core-rt/other | vcs by class | ics by class | minflt (process) | allocs | allocs >= 1 MiB | host encode us/chunk | send CPU us/chunk | send wall us/chunk |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|
| d/16MiB k1 | A | 1.38/0.00/8.22/0.00/- | 0.25/0.00/100.50/0.00/- | 0.00/0.00/0.12/0.00/- | 64.75 | 55.00 | 8.00 | - | - | - |
| d/16MiB k1 | Df | 1.38/0.00/8.44/0.00/- | 0.25/0.00/233.88/0.00/- | 0.00/0.00/0.00/0.00/- | 0.12 | 62.75 | 0.00 | - | - | - |
| d/16MiB k1 | Df-chan | 1.38/2.20/6.48/0.00/- | 0.25/5.12/106.38/0.00/- | 0.00/0.00/0.00/0.00/- | 64.75 | 66.38 | 1.25 | 259.67 | 7.49 | 355.86 |
| d/16MiB k1 | Cf | 1.38/2.05/0.00/5.98/- | 0.25/5.25/0.00/67.50/- | 0.00/0.00/0.00/0.00/- | 0.00 | 49.50 | 1.62 | - | - | - |
| d/16MiB k1 | Cf-cb | 1.38/0.00/2.74/6.83/- | 0.25/0.00/18.50/183.50/- | 0.00/0.00/0.00/0.00/- | 257.00 | 77.50 | 1.38 | - | - | - |
| d/16MiB k1 | C | 1.38/2.23/0.00/8.13/- | 0.25/5.62/0.00/170.25/- | 0.00/0.00/0.00/0.00/- | 0.62 | 72.00 | 8.00 | - | - | - |
| d/16MiB k1 | C-cb | 1.38/0.00/2.37/8.54/- | 0.25/0.00/18.38/224.12/- | 0.00/0.00/0.00/0.00/- | 64.88 | 100.00 | 8.00 | - | - | - |
| d/16MiB k1 | Cf-split | 1.38/2.05/0.00/6.10/- | 0.25/5.25/0.00/75.75/- | 0.00/0.00/0.00/0.00/- | 0.25 | 49.25 | 1.50 | 239.06 | 8.91 | 343.61 |
| d/16MiB k1 | Cf-cb-split | 1.38/0.00/2.42/7.08/- | 0.25/0.00/18.50/236.50/- | 0.00/0.00/0.00/0.00/- | 64.38 | 77.75 | 1.88 | 266.55 | 6.65 | 331.00 |
| d/4MiB k1 | A | 1.38/0.00/2.26/0.00/- | 0.25/0.00/33.25/0.00/- | 0.00/0.00/0.00/0.00/- | 0.00 | 36.50 | 2.00 | - | - | - |
| d/4MiB k1 | Df | 1.38/0.00/2.04/0.00/- | 0.25/0.00/15.50/0.00/- | 0.00/0.00/0.00/0.00/- | 0.25 | 38.50 | 0.00 | - | - | - |
| d/4MiB k1 | Df-chan | 1.38/0.57/1.49/0.00/- | 0.25/1.25/22.88/0.00/- | 0.00/0.00/0.00/0.00/- | 0.25 | 40.25 | 0.00 | 255.05 | 4.49 | 4.10 |
| d/4MiB k1 | Cf | 1.38/0.56/0.00/1.58/- | 0.25/1.25/0.00/32.88/- | 0.00/0.00/0.00/0.00/- | 0.00 | 41.25 | 0.00 | - | - | - |
| d/4MiB k1 | Cf-cb | 1.38/0.00/0.63/1.70/- | 0.25/0.00/6.50/46.38/- | 0.00/0.00/0.00/0.00/- | 0.12 | 51.50 | 0.00 | - | - | - |
| d/4MiB k1 | C | 1.38/0.56/0.00/2.01/- | 0.25/1.25/0.00/36.25/- | 0.00/0.00/0.00/0.00/- | 0.12 | 47.25 | 2.00 | - | - | - |
| d/4MiB k1 | C-cb | 1.38/0.00/0.62/2.07/- | 0.25/0.00/6.50/31.12/- | 0.00/0.00/0.00/0.00/- | 0.12 | 57.50 | 2.00 | - | - | - |
| d/4MiB k1 | Cf-split | 1.38/0.55/0.00/1.55/- | 0.25/1.25/0.00/27.62/- | 0.00/0.00/0.00/0.00/- | 0.00 | 41.25 | 0.00 | 242.70 | 5.69 | 5.28 |
| d/4MiB k1 | Cf-cb-split | 1.38/0.00/0.60/1.66/- | 0.25/0.00/6.50/37.50/- | 0.00/0.00/0.00/0.00/- | 0.00 | 51.50 | 0.00 | 246.22 | 6.34 | 24.01 |
