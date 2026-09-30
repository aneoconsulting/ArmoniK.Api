# attribution: commit 26a89727 + UNCOMMITTED; 2026-09-30T03:42:51Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 195081, pinned socket; probe /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe sha256 76dcd73a0579989f, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so
# cells A,Df,Cf,Ff; workloads d16k1 d16k8 c54k1 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 3 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 88 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 10.97 [9.19-13.05] | 11.73 [11.07-12.44] | 35.3 34.6 37.9 | 15.6 16.1 18.5 | 0.44 0.47 0.49 | 464.2 468.0 482.4 | 1227 1371 1841 | 64 47 55 | 10.93 10.66 11.66 | 54.8 / 0.0 / 1474.4 |
| Df | 8.50 [8.27-8.87] | 11.53 [8.84-12.55] | 27.0 26.7 27.0 | 10.4 10.1 10.3 | 0.38 0.38 0.38 | 464.9 500.1 483.3 | 0 0 0 | 145 113 137 | 8.64 8.46 8.65 | 132.5 / 0.0 / 0.1 |
| Cf | 8.36 [8.06-9.18] | 8.66 [7.99-9.74] | 27.0 27.8 27.1 | 9.7 10.1 9.9 | 0.36 0.36 0.36 | 524.2 501.5 514.0 | 22 74 42 | 64 77 68 | 8.41 8.70 8.45 | 64.4 / 0.0 / 43.2 |
| Ff | 8.66 [8.28-9.71] | 11.54 [10.43-12.69] | 29.3 26.9 27.0 | 10.7 10.3 10.3 | 0.37 0.38 0.38 | 468.7 497.3 485.5 | 0 0 11 | 176 132 134 | 9.51 8.57 8.61 | 139.2 / 0.0 / 0.0 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 11.10 | 1044 | 0 | 16.79 | 1680 |
| A | main | 0.05 | 0 | 15 | 0.00 | 0 |
| Df | cell-rt | 8.76 | 1046 | 0 | 16.79 | 0 |
| Df | main | 0.05 | 0 | 15 | 0.00 | 0 |
| Cf | caller | 2.24 | 1 | 0 | 0.00 | 85 |
| Cf | core-rt | 6.42 | 1045 | 0 | 16.79 | 0 |
| Cf | main | 0.05 | 0 | 17 | 0.00 | 0 |
| Ff | cell-rt | 8.34 | 1044 | 0 | 16.79 | 0 |
| Ff | main | 0.05 | 0 | 15 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | mmap | madvise |
|---|---|---|---|---|---|---|---|---|
| A | 1030 | 38 | 13 | 10 | 3 | 2 | 1 | 1 |
| Df | 1030 | 54 | 14 | 10 | 3 | 2 | 1 | 0 |
| Cf | 1027 | 45 | 20 | 10 | 4 | 2 | 1 | 0 |
| Ff | 1030 | 56 | 14 | 10 | 3 | 2 | 1 | 0 |

## d16k8: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 13.55 [12.69-15.10] | 10.33 [9.72-10.65] | 43.3 43.4 46.7 | 18.9 18.6 20.6 | 0.44 0.43 0.44 | 535.6 526.3 540.1 | 1837 1717 2157 | 72 92 90 | 13.38 13.47 14.46 | 80.6 / 0.0 / 1919.0 |
| Df | 9.57 [9.23-9.76] | 8.67 [8.39-9.06] | 30.1 30.5 30.5 | 10.2 10.3 10.3 | 0.34 0.34 0.34 | 491.2 480.3 488.7 | 11 22 22 | 74 84 82 | 9.41 9.57 9.56 | 79.4 / 0.0 / 21.6 |
| Cf | 10.15 [9.77-10.82] | 8.31 [8.20-8.48] | 33.0 33.1 32.5 | 10.9 10.8 10.7 | 0.33 0.33 0.33 | 429.8 428.1 424.4 | 212 176 145 | 60 68 68 | 10.24 10.29 10.13 | 64.0 / 0.1 / 149.0 |
| Ff | 9.61 [9.30-9.92] | 8.60 [8.32-9.08] | 30.9 30.9 30.3 | 10.3 10.4 10.3 | 0.33 0.34 0.34 | 494.4 475.0 486.0 | 38 32 27 | 76 87 79 | 9.67 9.68 9.50 | 80.1 / 0.0 / 21.6 |

### d16k8: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 12.88 | 1048 | 0 | 16.79 | 1667 |
| A | main | 0.03 | 0 | 8 | 0.00 | 0 |
| Df | cell-rt | 9.17 | 1052 | 0 | 16.79 | 11 |
| Df | main | 0.03 | 0 | 8 | 0.00 | 0 |
| Cf | caller | 3.07 | 0 | 0 | 0.00 | 189 |
| Cf | core-rt | 6.80 | 1050 | 0 | 16.79 | 0 |
| Cf | main | 0.05 | 0 | 14 | 0.00 | 0 |
| Ff | cell-rt | 9.48 | 1051 | 0 | 16.79 | 22 |
| Ff | main | 0.03 | 0 | 8 | 0.00 | 0 |

### d16k8: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | recvfrom | write | read | mprotect | openat | mmap |
|---|---|---|---|---|---|---|---|---|
| A | 1028 | 42 | 10 | 5 | 2 | 1 | 1 | 0 |
| Df | 1026 | 53 | 10 | 5 | 1 | 0 | 1 | 1 |
| Cf | 1028 | 55 | 10 | 5 | 3 | 1 | 1 | 1 |
| Ff | 1026 | 54 | 9 | 6 | 2 | 0 | 1 | 1 |

## c54k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 2.04 [1.94-2.14] | 3.27 [2.85-4.04] | 6.6 6.6 6.4 | 2.5 2.4 2.4 | 0.37 0.37 0.38 | 99.3 94.0 91.2 | 0 0 0 | 13 10 11 | 2.07 2.07 2.00 | 10.8 / 0.0 / 0.0 |
| Df | 1.99 [1.90-2.05] | 2.84 [2.78-3.33] | 6.5 6.5 6.5 | 2.4 2.4 2.4 | 0.37 0.37 0.37 | 94.4 89.3 92.1 | 0 0 0 | 11 8 8 | 2.02 1.99 2.01 | 8.2 / 0.0 / 0.0 |
| Cf | 2.08 [1.98-2.16] | 3.10 [2.89-3.64] | 6.6 6.7 6.6 | 2.4 2.5 2.4 | 0.37 0.37 0.37 | 94.7 94.3 89.4 | 0 0 0 | 15 16 14 | 2.09 2.11 2.08 | 14.6 / 0.0 / 0.0 |
| Ff | 1.99 [1.89-2.05] | 3.00 [2.82-3.49] | 6.4 6.4 6.5 | 2.4 2.4 2.4 | 0.37 0.37 0.37 | 91.2 91.0 92.2 | 0 0 0 | 9 9 9 | 1.99 1.99 2.02 | 8.7 / 0.0 / 0.0 |

### c54k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 1.93 | 264 | 0 | 4.20 | 0 |
| A | main | 0.01 | 0 | 4 | 0.00 | 0 |
| Df | cell-rt | 1.92 | 263 | 0 | 4.20 | 0 |
| Df | main | 0.01 | 0 | 4 | 0.00 | 0 |
| Cf | caller | 0.56 | 1 | 0 | 0.00 | 0 |
| Cf | core-rt | 1.44 | 264 | 0 | 4.20 | 0 |
| Cf | main | 0.02 | 0 | 5 | 0.00 | 0 |
| Ff | cell-rt | 1.97 | 264 | 0 | 4.20 | 0 |
| Ff | main | 0.01 | 0 | 4 | 0.00 | 0 |

### c54k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | mmap | close |
|---|---|---|---|---|---|---|---|---|
| A | 258 | 14 | 5 | 3 | 1 | 1 | 0 | 0 |
| Df | 258 | 11 | 4 | 3 | 1 | 1 | 0 | 0 |
| Cf | 258 | 17 | 8 | 3 | 1 | 1 | 0 | 0 |
| Ff | 258 | 11 | 4 | 3 | 1 | 1 | 0 | 0 |
