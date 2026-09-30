# in-process comparison: commit b950d7a5 + UNCOMMITTED; 2026-09-30T05:20:22Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 280822, pinned socket
# cells A,Cf,Cf-zc,Cf-encp (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition p6: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 d9a25f7e9109351f, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition p6h16: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 62e06532c76520b3, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env LD_LIBRARY_PATH=/tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/core-h2/release,AK_H2_COALESCE=16,AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 50 s

## d16k1

| cell | p6: CPU ms | p6: wall ms | p6: gap to A per rep (range) | p6: vcs / ics / minflt | p6h16: CPU ms | p6h16: wall ms | p6h16: gap to A per rep (range) | p6h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 7.76 [7.32-8.50] | 8.48 [8.23-9.71] |  | 88.9 / 0.0 / 0.1 | 7.62 [7.33-8.50] | 8.87 [8.69-9.81] |  | 69.9 / 0.0 / 0.0 |
| Cf | 8.18 [7.95-8.76] | 7.38 [7.21-9.02] | +0.30 +0.56 +0.52 (0.26) | 99.4 / 0.0 / 0.1 | 6.57 [6.43-7.26] | 7.21 [6.94-8.15] | -1.27 -0.80 -1.09 (0.47) | 58.2 / 0.0 / 0.1 |
| Cf-encp | 7.74 [7.49-8.20] | 8.58 [8.16-9.29] | +0.06 -0.23 +0.19 (0.42) | 89.6 / 0.0 / 0.1 | 6.09 [5.95-6.28] | 8.41 [8.21-9.24] | -1.79 -1.45 -1.51 (0.34) | 46.5 / 0.0 / 0.1 |
| Cf-zc | 8.26 [7.99-9.00] | 8.31 [7.53-9.27] | +0.38 +0.35 +0.95 (0.60) | 90.4 / 0.0 / 0.1 | 6.47 [6.37-6.66] | 7.20 [6.84-7.64] | -1.41 -1.08 -1.12 (0.33) | 51.6 / 0.0 / 0.0 |

## d16k8

| cell | p6: CPU ms | p6: wall ms | p6: gap to A per rep (range) | p6: vcs / ics / minflt | p6h16: CPU ms | p6h16: wall ms | p6h16: gap to A per rep (range) | p6h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 9.44 [8.80-10.17] | 8.76 [8.30-9.30] |  | 87.4 / 0.0 / 144.4 | 9.31 [8.75-10.24] | 8.80 [8.52-9.17] |  | 91.0 / 0.0 / 176.0 |
| Cf | 9.55 [9.34-9.84] | 8.25 [8.13-8.55] | +0.11 +0.05 +0.25 (0.21) | 73.9 / 0.1 / 0.3 | 7.65 [7.48-7.83] | 7.00 [6.84-7.21] | -1.65 -1.68 -1.99 (0.34) | 51.4 / 0.0 / 0.5 |
| Cf-encp | 9.07 [8.76-9.56] | 8.53 [8.42-8.79] | -0.50 -0.51 -0.02 (0.48) | 88.0 / 0.1 / 0.6 | 7.16 [6.89-7.44] | 7.31 [7.13-7.47] | -2.20 -2.05 -2.39 (0.34) | 53.9 / 0.1 / 0.5 |
| Cf-zc | 9.60 [9.43-9.79] | 8.32 [8.09-8.52] | +0.14 +0.06 +0.34 (0.28) | 76.9 / 0.1 / 0.5 | 7.63 [7.46-7.83] | 6.96 [6.78-7.23] | -1.74 -1.58 -1.99 (0.42) | 51.2 / 0.1 / 0.4 |

## d4k1

| cell | p6: CPU ms | p6: wall ms | p6: gap to A per rep (range) | p6: vcs / ics / minflt | p6h16: CPU ms | p6h16: wall ms | p6h16: gap to A per rep (range) | p6h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 1.93 [1.84-2.57] | 2.44 [2.36-2.55] |  | 16.6 / 0.0 / 0.0 | 2.07 [1.88-2.56] | 2.40 [2.31-2.47] |  | 37.2 / 0.0 / 0.0 |
| Cf | 2.03 [1.94-2.12] | 2.42 [2.35-2.56] | +0.12 +0.13 -0.20 (0.33) | 27.9 / 0.0 / 0.0 | 1.63 [1.56-1.82] | 2.06 [2.02-2.20] | -0.38 -0.49 -0.35 (0.14) | 24.8 / 0.0 / 0.0 |
| Cf-encp | 1.95 [1.87-2.09] | 2.53 [2.43-2.60] | +0.01 +0.07 -0.29 (0.36) | 15.5 / 0.0 / 0.0 | 1.56 [1.49-1.67] | 2.11 [2.07-2.19] | -0.42 -0.55 -0.52 (0.13) | 21.7 / 0.0 / 0.0 |
| Cf-zc | 2.04 [1.93-2.14] | 2.44 [2.29-2.53] | +0.13 +0.10 -0.21 (0.34) | 28.2 / 0.0 / 0.1 | 1.62 [1.56-1.79] | 2.07 [2.02-2.22] | -0.40 -0.47 -0.36 (0.11) | 25.0 / 0.0 / 0.0 |
