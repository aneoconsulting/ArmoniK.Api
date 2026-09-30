# attribution: commit 6d47eb91; 2026-09-30T19:39:49Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; scaling min/max cpu1 3300000/3300000 kHz
# stack binary (worktree p1+p2+p3+p5+p6+p7+p8, AK_CB_INLINE unset = HEAD's callback path), crates.io h2, AK_SPARES=6 AK_SPARE_LOCK=1, pinned allocator; AK_AT_INPROC=1: each process holds all four cells, perf counts one
# client 1-4,11-14 (host 8, core 8 workers); server 5-8,15-18, 8 workers, pid 480476, pinned socket; probe /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe sha256 abd544bfe4a99be1, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so
# cells A,Cf,Cf-cb,Cn-1rt; workloads d16k1 d16k8 (16MiB k1: 4 x 12 calls, warm 8; 16MiB k8: 4 x 3 batches of 8, warm 4; P5.4 k1: 4 x 40, warm 16); perf stat reps 3 (cell order rotated); perf perf version 7.2.8, events cycles:u,cycles:k,instructions:u,instructions:k,cache-misses,page-faults,context-switches,cpu-migrations,task-clock; perf_event_paranoid 1, kptr_restrict 1 (kernel symbols unresolved when 1)
# run time 122 s

## d16k1: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 7.95 [7.61-8.55] | 8.81 [8.69-8.94] |  |  |  | 426.5 430.5 435.7 | 0 0 0 | 80 54 71 | 8.32 7.85 8.13 | 66.9 / 0.0 / 0.0 |
| Cf | 8.65 [8.33-9.56] | 8.20 [7.96-8.68] |  |  |  | 482.8 486.0 479.7 | 0 0 0 | 86 64 82 | 8.82 8.50 9.12 | 80.3 / 0.0 / 0.1 |
| Cf-cb | 9.63 [9.02-10.88] | 8.34 [7.76-8.47] |  |  |  | 489.9 495.6 492.0 | 0 0 1 | 207 161 145 | 10.32 9.59 9.50 | 155.5 / 0.0 / 0.2 |
| Cn-1rt | 8.25 [7.87-8.89] | 8.41 [8.13-8.61] |  |  |  | 492.0 495.5 493.6 | 11 43 0 | 90 74 65 | 8.53 8.34 8.18 | 71.5 / 0.0 / 0.1 |

### d16k1: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|

### d16k1: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell |  |
|---|
| A |  |
| Cf |  |
| Cf-cb |  |
| Cn-1rt |  |

## d16k8: perf stat per call, timed rounds only (one value per repetition, in repetition order)

| cell | client CPU ms median [p10-p90] | wall ms | cycles (M) | instructions (M) | IPC | cache-misses (k) | page-faults | context-switches | task-clock ms | getrusage vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 9.18 [8.75-9.90] | 8.57 [8.33-8.94] |  |  |  | 456.5 457.0 459.1 | 101 176 112 | 69 52 57 | 9.45 9.19 9.23 | 54.1 / 0.0 / 149.3 |
| Cf | 9.89 [9.69-10.31] | 8.21 [8.04-8.36] |  |  |  | 414.9 412.9 410.9 | 0 1 0 | 61 62 63 | 10.00 9.97 9.91 | 60.5 / 0.1 / 0.5 |
| Cf-cb | 9.77 [9.46-10.07] | 8.20 [7.96-8.32] |  |  |  | 394.9 404.7 396.1 | 1 1 1 | 84 87 88 | 9.71 9.78 9.83 | 87.7 / 0.2 / 0.5 |
| Cn-1rt | 9.75 [9.29-10.52] | 8.22 [8.13-8.54] |  |  |  | 427.6 421.4 422.5 | 75 69 80 | 70 64 69 | 9.97 9.80 9.84 | 67.3 / 0.0 / 53.4 |

### d16k8: /proc per thread class per call (untimed run): CPU ms, write syscalls, read syscalls, MB written

| cell | class | CPU ms | syscw | syscr | wchar MB | minflt |
|---|---|---|---|---|---|---|

### d16k8: strace -f -c, calls per RPC call (warm-up and setup included; strace slows every syscall)

| cell |  |
|---|
| A |  |
| Cf |  |
| Cf-cb |  |
| Cn-1rt |  |
