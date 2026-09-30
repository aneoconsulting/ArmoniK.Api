# attribution: commit 50139d33; 2026-09-30T04:52:47Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# core from core-h2 (LD_LIBRARY_PATH; stack p1-p3 + patched h2) with AK_H2_COALESCE=16, AK_SPARES=6 AK_SPARE_LOCK=1; cell A uses the harness (crates.io) h2
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 253771, pinned socket; probe /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe sha256 40f3d842d9f09751, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/core-h2/release/libak_core.so
# cells Cf,Cf-cb,A; workloads d16k1 c54k1 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 1 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 22 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 11.11 [8.25-13.44] | 11.80 [11.11-12.64] | 36.0 | 16.1 | 0.45 | 479.7 | 1299 | 79 | 11.16 | 74.5 / 0.0 / 1290.5 |
| Cf | 6.71 [6.64-6.83] | 8.45 [7.95-8.78] | 21.7 | 4.8 | 0.22 | 514.6 | 0 | 50 | 6.75 | 49.6 / 0.0 / 0.1 |
| Cf-cb | 7.32 [7.06-8.00] | 9.51 [9.42-9.66] | 23.3 | 5.3 | 0.23 | 533.9 | 0 | 88 | 7.40 | 86.6 / 0.0 / 0.2 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 9.63 | 1044 | 0 | 16.79 | 1077 |
| A | main | 0.05 | 0 | 15 | 0.00 | 0 |
| Cf | caller | 2.10 | 2 | 0 | 0.00 | 0 |
| Cf | core-rt | 4.54 | 149 | 0 | 16.79 | 0 |
| Cf | main | 0.05 | 0 | 17 | 0.00 | 0 |
| Cf-cb | cell-rt | 2.32 | 2 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 5.06 | 137 | 0 | 16.79 | 0 |
| Cf-cb | main | 0.08 | 0 | 28 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | mmap |
|---|---|---|---|---|---|---|---|---|
| A | 1029 | 36 | 12 | 10 | 3 | 1 | 1 | 1 |
| Cf | 100 | 26 | 12 | 12 | 4 | 2 | 1 | 1 |
| Cf-cb | 98 | 92 | 21 | 12 | 6 | 2 | 2 | 1 |

## c54k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 1.99 [1.90-2.06] | 2.88 [2.80-2.94] | 6.4 | 2.4 | 0.38 | 88.9 | 0 | 10 | 2.00 | 10.2 / 0.0 / 0.0 |
| Cf | 1.68 [1.65-1.70] | 2.56 [2.53-2.60] | 5.2 | 1.4 | 0.26 | 80.3 | 0 | 22 | 1.69 | 22.0 / 0.0 / 0.0 |
| Cf-cb | 1.64 [1.62-1.66] | 2.50 [2.48-2.51] | 5.2 | 1.4 | 0.26 | 80.6 | 0 | 19 | 1.65 | 18.5 / 0.0 / 0.0 |

### c54k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 1.98 | 262 | 0 | 4.20 | 0 |
| A | main | 0.01 | 0 | 4 | 0.00 | 0 |
| Cf | caller | 0.55 | 1 | 0 | 0.00 | 0 |
| Cf | core-rt | 1.06 | 48 | 0 | 4.20 | 0 |
| Cf | main | 0.02 | 0 | 5 | 0.00 | 0 |
| Cf-cb | cell-rt | 0.54 | 1 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 1.05 | 46 | 0 | 4.20 | 0 |
| Cf-cb | main | 0.02 | 0 | 8 | 0.00 | 0 |

### c54k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | statx |
|---|---|---|---|---|---|---|---|---|
| A | 258 | 14 | 5 | 3 | 1 | 1 | 0 | 0 |
| Cf | 26 | 12 | 7 | 4 | 1 | 1 | 0 | 0 |
| Cf-cb | 26 | 10 | 7 | 4 | 2 | 1 | 1 | 0 |
