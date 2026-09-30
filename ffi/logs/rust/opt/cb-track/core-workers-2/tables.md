# attribution: commit 6d47eb91; 2026-09-30T19:54:23Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# stack binary (AK_CB_INLINE unset), crates.io h2, AK_SPARES=6 AK_SPARE_LOCK=1, pinned allocator, AK_CORE_WORKERS=2
# client 1-4,11-14 (host 8, core 2 workers); server 5-8,15-18, 8 workers, pid 494007, pinned socket; probe /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so
# cells Cf,Cf-cb; workloads d16k1 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 2 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 9 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| Cf | 8.38 [8.17-8.63] | 7.89 [7.14-8.62] | 27.2 26.8 | 9.8 9.6 | 0.36 0.36 | 484.8 485.3 | 0 0 | 75 61 | 8.49 8.35 | 68.0 / 0.0 / 0.1 |
| Cf-cb | 9.19 [8.81-9.64] | 7.97 [7.31-8.74] | 29.2 28.4 | 10.6 10.4 | 0.36 0.37 | 489.4 489.8 | 0 0 | 161 140 | 9.36 9.08 | 159.0 / 0.0 / 0.2 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| Cf | caller | 2.04 | 2 | 0 | 0.00 | 0 |
| Cf | core-rt | 6.12 | 1046 | 0 | 16.79 | 0 |
| Cf | main | 0.03 | 0 | 7 | 0.00 | 0 |
| Cf-cb | cell-rt | 2.23 | 5 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 6.89 | 1048 | 0 | 16.79 | 0 |
| Cf-cb | main | 0.06 | 0 | 18 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | mmap |
|---|---|---|---|---|---|---|---|---|
| Cf | 1026 | 19 | 23 | 9 | 2 | 2 | 0 | 1 |
| Cf-cb | 1026 | 41 | 30 | 9 | 4 | 2 | 1 | 1 |
