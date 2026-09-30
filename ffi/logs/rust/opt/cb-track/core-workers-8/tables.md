# attribution: commit 6d47eb91; 2026-09-30T19:54:34Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# stack binary (AK_CB_INLINE unset), crates.io h2, AK_SPARES=6 AK_SPARE_LOCK=1, pinned allocator, AK_CORE_WORKERS=8
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 494365, pinned socket; probe /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so
# cells Cf,Cf-cb; workloads d16k1 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 2 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 9 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| Cf | 8.44 [8.15-8.82] | 8.25 [7.20-8.73] | 27.1 27.0 | 9.7 9.8 | 0.36 0.36 | 483.2 482.8 | 0 0 | 66 83 | 8.51 8.49 | 66.5 / 0.0 / 0.1 |
| Cf-cb | 10.48 [9.38-11.28] | 8.53 [7.75-8.78] | 31.6 32.2 | 11.0 11.1 | 0.35 0.34 | 488.5 491.0 | 0 0 | 213 222 | 10.26 10.46 | 211.0 / 0.0 / 0.2 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| Cf | caller | 2.02 | 1 | 0 | 0.00 | 0 |
| Cf | core-rt | 6.10 | 1045 | 0 | 16.79 | 0 |
| Cf | main | 0.05 | 0 | 17 | 0.00 | 0 |
| Cf-cb | cell-rt | 2.29 | 2 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 7.12 | 1044 | 0 | 16.79 | 0 |
| Cf-cb | main | 0.08 | 0 | 28 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | mmap |
|---|---|---|---|---|---|---|---|---|
| Cf | 1027 | 44 | 21 | 10 | 4 | 2 | 1 | 1 |
| Cf-cb | 1026 | 92 | 27 | 10 | 6 | 2 | 2 | 1 |
