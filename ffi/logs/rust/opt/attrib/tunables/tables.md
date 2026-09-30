# attribution: commit 26a89727 + UNCOMMITTED; 2026-09-30T03:46:25Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432 in the client (and perf/strace/taskset) environment: a static mmap threshold of 32 MiB and trim threshold of 256 MiB (attribution of the allocator mode, not a campaign setting)
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 198516, pinned socket; probe /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe sha256 76dcd73a0579989f, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so
# cells A,Df,Cf,Ff; workloads d16k1 d16k8 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 3 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 62 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 8.42 [7.77-9.10] | 8.15 [7.90-8.64] | 27.0 26.8 26.9 | 10.2 10.1 10.0 | 0.38 0.38 0.37 | 428.7 424.8 424.2 | 32 0 0 | 121 115 106 | 8.58 8.48 8.49 | 111.1 / 0.0 / 0.0 |
| Df | 8.56 [8.25-10.84] | 8.37 [8.24-8.59] | 28.9 26.5 28.2 | 10.9 10.3 10.8 | 0.38 0.39 0.38 | 436.5 449.6 438.0 | 0 0 0 | 194 137 187 | 9.39 8.44 9.16 | 154.9 / 0.0 / 0.2 |
| Cf | 8.77 [8.19-9.43] | 7.15 [7.00-7.91] | 26.9 27.8 28.9 | 10.0 10.1 9.9 | 0.37 0.36 0.34 | 488.1 483.2 492.2 | 0 11 11 | 104 117 94 | 8.50 8.79 9.09 | 101.6 / 0.0 / 0.2 |
| Ff | 8.54 [8.25-9.62] | 8.47 [8.22-8.92] | 26.7 27.9 27.8 | 10.4 10.6 10.6 | 0.39 0.38 0.38 | 447.2 436.7 437.2 | 0 11 11 | 153 164 171 | 8.56 8.99 8.98 | 158.0 / 0.0 / 0.0 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 7.52 | 1042 | 0 | 16.79 | 0 |
| A | main | 0.05 | 0 | 15 | 0.00 | 0 |
| Df | cell-rt | 8.40 | 1045 | 0 | 16.79 | 0 |
| Df | main | 0.05 | 0 | 15 | 0.00 | 0 |
| Cf | caller | 2.03 | 1 | 0 | 0.00 | 0 |
| Cf | core-rt | 6.44 | 1041 | 0 | 16.79 | 0 |
| Cf | main | 0.05 | 0 | 17 | 0.00 | 0 |
| Ff | cell-rt | 9.16 | 1043 | 0 | 16.79 | 0 |
| Ff | main | 0.05 | 0 | 15 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | mmap | close |
|---|---|---|---|---|---|---|---|---|
| A | 1029 | 38 | 11 | 10 | 3 | 2 | 1 | 1 |
| Df | 1031 | 56 | 12 | 10 | 3 | 2 | 1 | 1 |
| Cf | 1027 | 44 | 21 | 10 | 4 | 2 | 1 | 1 |
| Ff | 1031 | 54 | 14 | 10 | 3 | 2 | 1 | 1 |

## d16k8: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 9.37 [8.83-9.99] | 8.54 [8.37-8.71] | 30.8 30.0 30.0 | 10.8 10.6 10.3 | 0.35 0.35 0.34 | 464.6 458.1 461.2 | 208 166 128 | 64 60 54 | 9.58 9.33 9.30 | 60.3 / 0.0 / 160.2 |
| Df | 9.62 [9.12-10.03] | 8.07 [7.93-8.15] | 31.2 30.6 30.2 | 10.4 10.3 10.2 | 0.33 0.34 0.34 | 453.7 453.9 455.6 | 27 22 17 | 95 81 77 | 9.81 9.59 9.46 | 80.0 / 0.0 / 21.4 |
| Cf | 9.71 [9.42-9.95] | 8.06 [7.94-8.27] | 31.3 31.5 31.0 | 10.1 10.1 9.9 | 0.32 0.32 0.32 | 412.2 413.9 411.6 | 27 38 6 | 67 62 57 | 9.74 9.79 9.62 | 62.2 / 0.0 / 22.1 |
| Ff | 9.61 [9.27-10.09] | 8.18 [7.95-8.43] | 30.5 31.4 30.4 | 10.3 10.5 10.4 | 0.34 0.33 0.34 | 450.2 459.7 461.2 | 22 38 16 | 89 95 92 | 9.58 9.86 9.56 | 88.7 / 0.0 / 21.6 |

### d16k8: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 9.11 | 1052 | 0 | 16.79 | 214 |
| A | main | 0.03 | 0 | 8 | 0.00 | 0 |
| Df | cell-rt | 9.60 | 1048 | 0 | 16.79 | 11 |
| Df | main | 0.03 | 0 | 8 | 0.00 | 0 |
| Cf | caller | 2.73 | 0 | 0 | 0.00 | 11 |
| Cf | core-rt | 6.72 | 1050 | 0 | 16.79 | 1 |
| Cf | main | 0.05 | 0 | 14 | 0.00 | 0 |
| Ff | cell-rt | 9.48 | 1049 | 0 | 16.79 | 22 |
| Ff | main | 0.03 | 0 | 8 | 0.00 | 0 |

### d16k8: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | recvfrom | write | read | mprotect | openat | close |
|---|---|---|---|---|---|---|---|---|
| A | 1028 | 42 | 9 | 4 | 1 | 1 | 1 | 0 |
| Df | 1025 | 55 | 10 | 5 | 1 | 1 | 1 | 0 |
| Cf | 1028 | 57 | 10 | 4 | 3 | 1 | 1 | 1 |
| Ff | 1026 | 55 | 10 | 5 | 1 | 1 | 1 | 0 |
