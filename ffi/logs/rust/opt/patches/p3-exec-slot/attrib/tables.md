# attribution: commit ca98c15d + UNCOMMITTED; 2026-09-30T04:38:07Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# patched core (stack p1+p2+p3) with AK_SPARES=6 AK_SPARE_LOCK=1; perf record classified with the booted kernel System.map
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 245105, pinned socket; probe /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe sha256 0a060a097f1f7edf, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so
# cells Cf-cb,Cf-cb-1rt,Cn-1rt; workloads d16k1 c54k1 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 2 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 31 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| Cf-cb | 9.40 [8.88-10.29] | 9.44 [8.89-10.50] | 29.9 29.5 | 10.4 10.3 | 0.35 0.35 | 527.1 519.2 | 0 0 | 145 131 | 9.60 9.42 | 138.0 / 0.0 / 0.3 |
| Cf-cb-1rt | 10.62 [10.02-11.86] | 9.92 [8.98-11.26] | 32.0 31.3 | 12.1 12.1 | 0.38 0.39 | 521.5 527.0 | 0 0 | 334 336 | 10.78 10.57 | 342.8 / 0.0 / 0.4 |
| Cn-1rt | 8.31 [7.98-8.79] | 10.38 [10.19-11.03] | 27.1 26.7 | 9.7 9.6 | 0.36 0.36 | 529.8 525.2 | 11 0 | 63 62 | 8.43 8.32 | 61.5 / 0.0 / 0.2 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| Cf-cb | cell-rt | 2.28 | 2 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 6.79 | 1048 | 0 | 16.79 | 0 |
| Cf-cb | main | 0.08 | 0 | 28 | 0.00 | 0 |
| Cf-cb-1rt | cell-rt | 8.71 | 1044 | 0 | 16.79 | 0 |
| Cf-cb-1rt | core-reactor | 1.58 | 0 | 0 | 0.00 | 0 |
| Cf-cb-1rt | main | 0.05 | 0 | 17 | 0.00 | 0 |
| Cn-1rt | cell-rt | 8.42 | 1046 | 0 | 16.79 | 0 |
| Cn-1rt | main | 0.05 | 0 | 15 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | mmap |
|---|---|---|---|---|---|---|---|---|
| Cf-cb | 1026 | 91 | 27 | 11 | 6 | 2 | 2 | 1 |
| Cf-cb-1rt | 1033 | 57 | 9 | 9 | 4 | 2 | 1 | 1 |
| Cn-1rt | 1026 | 48 | 14 | 10 | 3 | 2 | 1 | 1 |

## c54k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| Cf-cb | 2.09 [2.00-2.15] | 3.11 [2.97-3.43] | 6.7 6.7 | 2.4 2.4 | 0.36 0.36 | 93.1 96.5 | 0 0 | 13 14 | 2.10 2.10 | 12.9 / 0.0 / 0.0 |
| Cf-cb-1rt | 2.37 [2.17-2.77] | 3.08 [2.98-3.27] | 7.2 7.4 | 2.8 2.8 | 0.39 0.38 | 94.6 93.3 | 0 0 | 55 53 | 2.37 2.43 | 54.0 / 0.0 / 0.0 |
| Cn-1rt | 1.98 [1.92-2.04] | 3.06 [2.88-3.45] | 6.4 6.5 | 2.4 2.4 | 0.38 0.37 | 93.0 94.1 | 0 0 | 11 10 | 1.99 2.01 | 10.2 / 0.0 / 0.0 |

### c54k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| Cf-cb | cell-rt | 0.56 | 1 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 1.47 | 262 | 0 | 4.20 | 0 |
| Cf-cb | main | 0.02 | 0 | 8 | 0.00 | 0 |
| Cf-cb-1rt | cell-rt | 2.01 | 262 | 0 | 4.20 | 0 |
| Cf-cb-1rt | core-reactor | 0.24 | 0 | 0 | 0.00 | 0 |
| Cf-cb-1rt | main | 0.02 | 0 | 5 | 0.00 | 0 |
| Cn-1rt | cell-rt | 1.90 | 261 | 0 | 4.20 | 0 |
| Cn-1rt | main | 0.01 | 0 | 4 | 0.00 | 0 |

### c54k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | statx |
|---|---|---|---|---|---|---|---|---|
| Cf-cb | 258 | 16 | 8 | 3 | 2 | 1 | 1 | 0 |
| Cf-cb-1rt | 258 | 16 | 4 | 3 | 1 | 1 | 0 | 0 |
| Cn-1rt | 258 | 11 | 4 | 3 | 1 | 1 | 0 | 0 |
