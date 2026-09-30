# attribution: commit 6d47eb91; 2026-09-30T19:53:22Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# stack binary (AK_CB_INLINE unset), crates.io h2, AK_SPARES=6 AK_SPARE_LOCK=1, pinned allocator, AK_CORE_WORKERS=1
# client 1-4,11-14 (host 8, core 1 workers); server 5-8,15-18, 8 workers, pid 493158, pinned socket; probe /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so
# cells Cf,Cf-cb; workloads d16k1 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 2 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 10 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| Cf | 7.91 [7.74-8.08] | 8.31 [7.02-8.49] | 25.9 25.8 | 9.4 9.3 | 0.36 0.36 | 473.3 473.4 | 0 0 | 31 32 | 7.98 7.96 | 32.5 / 0.0 / 0.1 |
| Cf-cb | 8.23 [8.01-8.48] | 8.00 [6.79-8.37] | 26.4 26.9 | 9.5 9.6 | 0.36 0.36 | 459.5 465.9 | 0 0 | 38 44 | 8.22 8.32 | 42.5 / 0.0 / 0.1 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| Cf | caller | 2.10 | 2 | 0 | 0.00 | 0 |
| Cf | core-rt | 5.83 | 1043 | 0 | 16.79 | 0 |
| Cf | main | 0.02 | 0 | 5 | 0.00 | 0 |
| Cf-cb | cell-rt | 2.24 | 4 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 5.93 | 1041 | 0 | 16.79 | 0 |
| Cf-cb | main | 0.05 | 0 | 17 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | mmap |
|---|---|---|---|---|---|---|---|---|
| Cf | 1026 | 14 | 11 | 9 | 1 | 1 | 0 | 1 |
| Cf-cb | 1026 | 38 | 11 | 9 | 4 | 2 | 1 | 1 |
