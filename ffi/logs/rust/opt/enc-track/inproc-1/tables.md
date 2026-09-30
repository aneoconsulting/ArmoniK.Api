# in-process comparison: commit 7e437eb1 + UNCOMMITTED; 2026-09-30T05:47:41Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 314948, pinned socket
# cells A,Cf,Cf-enc,Cf-encp,Cf-zc,Cf-zcp (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 d4k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition stack: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 58af0429cc341263, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 47 s

## d16k1

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 7.65 [7.31-8.26] | 8.88 [8.35-10.76] |  | 75.1 / 0.0 / 0.0 |
| Cf | 8.31 [7.99-9.12] | 8.15 [7.62-8.77] | +0.96 +0.87 +0.46 (0.50) | 90.9 / 0.0 / 0.1 |
| Cf-enc | 7.70 [7.47-8.02] | 8.94 [8.41-9.32] | +0.18 -0.02 +0.01 (0.20) | 72.7 / 0.0 / 0.1 |
| Cf-encp | 7.81 [7.49-8.47] | 8.68 [8.22-9.36] | +0.22 +0.12 +0.15 (0.10) | 88.1 / 0.0 / 0.1 |
| Cf-zc | 6.71 [6.33-7.16] | 7.90 [7.42-8.10] | -0.67 -1.15 -0.95 (0.48) | 130.4 / 0.0 / 0.1 |
| Cf-zcp | 6.63 [6.31-7.04] | 7.75 [7.05-9.82] | -1.01 -1.21 -0.91 (0.30) | 130.9 / 0.0 / 0.1 |

## d16k8

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 9.23 [8.81-9.95] | 8.88 [8.38-9.13] |  | 72.8 / 0.0 / 240.0 |
| Cf | 9.69 [9.44-9.91] | 8.30 [8.03-8.43] | +0.29 +0.47 +0.42 (0.19) | 77.9 / 0.1 / 0.4 |
| Cf-enc | 9.15 [8.84-9.48] | 8.55 [8.38-8.76] | -0.28 +0.06 -0.21 (0.34) | 87.0 / 0.1 / 0.4 |
| Cf-encp | 9.17 [8.74-9.65] | 8.55 [8.37-8.80] | -0.45 -0.08 +0.00 (0.45) | 93.0 / 0.1 / 0.5 |
| Cf-zc | 6.43 [6.22-6.73] | 7.96 [7.81-8.15] | -3.07 -2.77 -2.77 (0.31) | 83.7 / 0.1 / 0.6 |
| Cf-zcp | 6.36 [6.20-6.77] | 7.96 [7.64-8.28] | -2.87 -2.87 -2.97 (0.11) | 78.6 / 0.0 / 0.5 |

## d4k1

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 2.02 [1.84-2.52] | 2.40 [2.33-2.46] |  | 34.0 / 0.0 / 0.0 |
| Cf | 2.03 [1.94-2.14] | 2.42 [2.23-2.51] | +0.05 +0.09 -0.16 (0.25) | 32.2 / 0.0 / 0.0 |
| Cf-enc | 1.94 [1.88-2.02] | 2.48 [2.42-2.54] | -0.04 -0.02 -0.25 (0.23) | 16.4 / 0.0 / 0.1 |
| Cf-encp | 1.92 [1.86-2.02] | 2.51 [2.41-2.60] | -0.07 -0.03 -0.27 (0.24) | 15.9 / 0.0 / 0.0 |
| Cf-zc | 1.53 [1.42-1.64] | 2.14 [2.00-2.27] | -0.49 -0.45 -0.59 (0.15) | 25.0 / 0.0 / 0.1 |
| Cf-zcp | 1.57 [1.46-1.70] | 2.24 [2.15-2.32] | -0.45 -0.40 -0.54 (0.15) | 36.1 / 0.0 / 0.0 |

## d4k8

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 2.49 [2.22-2.96] | 2.15 [2.09-2.21] |  | 28.9 / 0.0 / 104.1 |
| Cf | 2.59 [2.51-2.74] | 2.27 [2.20-2.36] | +0.05 +0.08 +0.10 (0.05) | 22.9 / 0.0 / 0.4 |
| Cf-enc | 2.41 [2.29-2.53] | 2.21 [2.11-2.30] | -0.15 -0.03 -0.09 (0.12) | 33.4 / 0.0 / 0.3 |
| Cf-encp | 2.46 [2.30-2.62] | 2.20 [2.10-2.25] | -0.10 +0.04 -0.04 (0.14) | 34.9 / 0.0 / 0.3 |
| Cf-zc | 1.64 [1.60-1.72] | 2.12 [1.99-2.19] | -0.91 -0.83 -0.84 (0.08) | 23.4 / 0.0 / 0.3 |
| Cf-zcp | 1.65 [1.59-1.73] | 2.07 [1.94-2.15] | -0.91 -0.84 -0.83 (0.08) | 24.3 / 0.0 / 0.3 |
