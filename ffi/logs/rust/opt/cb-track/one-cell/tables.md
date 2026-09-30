# attribution: commit 6d47eb91; 2026-09-30T19:36:29Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# stack binary (worktree p1+p2+p3+p5+p6+p7+p8, AK_CB_INLINE unset = HEAD's callback path), crates.io h2, AK_SPARES=6 AK_SPARE_LOCK=1, pinned allocator
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 477847, pinned socket; probe /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so
# cells A,Cf,Cf-cb,Cn-1rt,Cf-split,Cf-cb-split; workloads d16k1 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 1 (cell order rotated); perf perf version 7.2.8, events cycles,instructions,cache-references,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 25 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 8.25 [7.70-8.96] | 7.96 [7.84-8.31] | 26.4 | 10.1 | 0.38 | 428.3 | 32 | 114 | 8.35 | 107.5 / 0.0 / 0.2 |
| Cf | 8.67 [8.26-9.66] | 7.39 [7.32-7.44] | 27.9 | 10.0 | 0.36 | 484.7 | 0 | 107 | 8.83 | 105.9 / 0.0 / 0.2 |
| Cf-cb | 9.70 [8.95-10.86] | 7.90 [7.57-8.37] | 30.2 | 10.9 | 0.36 | 490.2 | 0 | 194 | 9.76 | 189.5 / 0.0 / 0.2 |
| Cn-1rt | 8.64 [7.93-10.00] | 7.50 [7.35-7.79] | 27.9 | 10.3 | 0.37 | 490.7 | 11 | 126 | 8.86 | 128.7 / 0.0 / 0.1 |
| Cf-cb-split | 10.16 [9.59-11.05] | 7.77 [7.61-7.90] | 31.4 | 11.1 | 0.35 | 494.6 | 0 | 211 | 10.21 | 214.0 / 0.0 / 0.2 |
| Cf-split | 8.54 [8.15-9.36] | 7.35 [7.33-7.45] | 27.7 | 9.8 | 0.35 | 481.9 | 0 | 83 | 8.72 | 83.7 / 0.0 / 0.1 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|
| A | cell-rt | 7.59 | 1045 | 0 | 16.79 | 0 |
| A | main | 0.05 | 0 | 15 | 0.00 | 0 |
| Cf | caller | 2.06 | 1 | 0 | 0.00 | 0 |
| Cf | core-rt | 6.47 | 1046 | 0 | 16.79 | 0 |
| Cf | main | 0.05 | 0 | 17 | 0.00 | 0 |
| Cf-cb | cell-rt | 2.28 | 2 | 0 | 0.00 | 0 |
| Cf-cb | core-rt | 7.04 | 1047 | 0 | 16.79 | 0 |
| Cf-cb | main | 0.08 | 0 | 28 | 0.00 | 0 |
| Cn-1rt | cell-rt | 8.85 | 1043 | 0 | 16.79 | 0 |
| Cn-1rt | main | 0.05 | 0 | 15 | 0.00 | 0 |
| Cf-cb-split | cell-rt | 2.24 | 2 | 0 | 0.00 | 0 |
| Cf-cb-split | core-rt | 7.36 | 1046 | 0 | 16.79 | 0 |
| Cf-cb-split | main | 0.08 | 0 | 28 | 0.00 | 0 |
| Cf-split | caller | 2.08 | 1 | 0 | 0.00 | 0 |
| Cf-split | core-rt | 6.36 | 1046 | 0 | 16.79 | 0 |
| Cf-split | main | 0.05 | 0 | 17 | 0.00 | 0 |

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell | writev | futex | write | recvfrom | read | openat | close | mmap |
|---|---|---|---|---|---|---|---|---|
| A | 1030 | 33 | 12 | 10 | 3 | 2 | 1 | 1 |
| Cf | 1027 | 43 | 21 | 10 | 4 | 2 | 1 | 1 |
| Cf-cb | 1027 | 92 | 27 | 11 | 6 | 2 | 2 | 1 |
| Cn-1rt | 1027 | 49 | 14 | 10 | 3 | 2 | 1 | 1 |
| Cf-cb-split | 1027 | 92 | 28 | 10 | 6 | 2 | 2 | 1 |
| Cf-split | 1027 | 45 | 22 | 10 | 4 | 2 | 1 | 1 |
