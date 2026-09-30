# attribution: commit 26a89727 + UNCOMMITTED; 2026-09-30T03:51:04Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# AK_AT_INPROC=1: every process holds A, Df, Cf, Ff (block order, rotated); perf counts one cell (AK_PERF_CELL). GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 203602, pinned socket; probe /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe sha256 cba2edcf3dfe338f, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so
# cells A,Df,Cf,Ff; workloads d16k1 d16k8 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 3 (cell order rotated); perf perf version 7.2.8, events cycles:u,cycles:k,instructions:u,instructions:k,cache-misses,dTLB-load-misses,dTLB-store-misses,page-faults,context-switches,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 121 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 7.85 [7.53-8.44] | 8.71 [8.27-8.91] |  |  |  | 429.6 428.9 434.2 | 0 0 43 | 56 57 73 | 7.95 8.01 8.20 | 57.5 / 0.0 / 0.0 |
| Df | 8.30 [8.08-9.09] | 9.04 [8.84-9.11] |  |  |  | 454.8 455.6 435.0 | 0 0 0 | 116 108 159 | 8.38 8.33 9.23 | 117.2 / 0.0 / 0.2 |
| Cf | 8.50 [8.19-9.68] | 8.15 [7.49-8.64] |  |  |  | 495.3 486.9 483.6 | 0 0 0 | 76 82 62 | 8.78 9.29 8.48 | 71.4 / 0.0 / 0.2 |
| Ff | 8.69 [8.35-10.00] | 8.66 [8.35-9.05] |  |  |  | 411.5 452.7 453.2 | 0 0 11 | 183 142 121 | 9.80 8.77 8.62 | 139.2 / 0.0 / 0.0 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell |  |
|---|
| A |  |
| Df |  |
| Cf |  |
| Ff |  |

## d16k8: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 9.41 [8.94-10.15] | 8.65 [8.24-8.90] |  |  |  | 464.5 466.3 459.8 | 134 117 187 | 63 65 64 | 9.49 9.54 9.63 | 65.0 / 0.0 / 138.6 |
| Df | 9.74 [9.37-10.26] | 8.22 [8.13-8.33] |  |  |  | 457.6 458.9 458.5 | 11 22 17 | 94 85 85 | 9.90 9.84 9.72 | 89.4 / 0.0 / 11.0 |
| Cf | 9.78 [9.55-10.34] | 8.12 [8.00-8.37] |  |  |  | 417.4 409.4 413.2 | 11 17 27 | 59 64 61 | 9.89 10.00 9.91 | 57.9 / 0.1 / 11.4 |
| Ff | 9.61 [9.18-9.98] | 8.24 [8.04-8.38] |  |  |  | 459.1 456.6 466.0 | 21 27 22 | 80 80 74 | 9.75 9.67 9.54 | 78.3 / 0.0 / 21.6 |

### d16k8: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|

### d16k8: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell |  |
|---|
| A |  |
| Df |  |
| Cf |  |
| Ff |  |
