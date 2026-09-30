# in-process comparison: commit e92fc4c8; 2026-09-30T05:50:30Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 318211, pinned socket
# cells A,Cf,Cf-encp,Cf-zc,Cf-zcp,Cf-zcw (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 d4k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition stack: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 4e3d20f128fa4a41, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 46 s

## d16k1

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 7.76 [7.42-8.71] | 8.35 [8.17-9.02] |  | 102.1 / 0.0 / 0.0 |
| Cf | 8.30 [8.02-8.77] | 7.93 [7.22-8.39] | +0.50 +0.38 +0.61 (0.24) | 104.4 / 0.0 / 0.1 |
| Cf-encp | 7.74 [7.45-8.14] | 8.91 [8.42-9.16] | -0.07 -0.35 +0.09 (0.44) | 86.4 / 0.0 / 0.2 |
| Cf-zc | 6.43 [6.18-6.80] | 7.79 [6.97-8.17] | -1.41 -1.64 -1.11 (0.52) | 106.1 / 0.0 / 0.1 |
| Cf-zcp | 6.44 [6.22-6.77] | 7.98 [7.18-8.24] | -1.36 -1.54 -1.29 (0.26) | 110.3 / 0.0 / 0.1 |
| Cf-zcw | 6.63 [6.33-7.17] | 7.33 [7.01-8.72] | -1.14 -1.47 -1.01 (0.46) | 138.5 / 0.0 / 0.2 |

## d16k8

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 9.37 [8.78-9.93] | 8.77 [8.38-9.16] |  | 76.4 / 0.0 / 240.0 |
| Cf | 9.54 [9.35-9.85] | 8.23 [8.07-8.51] | +0.19 +0.34 -0.09 (0.43) | 77.2 / 0.0 / 0.5 |
| Cf-encp | 9.04 [8.81-9.38] | 8.47 [8.30-8.78] | -0.43 -0.20 -0.43 (0.23) | 84.2 / 0.1 / 0.5 |
| Cf-zc | 6.43 [6.18-6.63] | 7.93 [7.76-8.24] | -3.18 -2.76 -3.13 (0.42) | 75.6 / 0.0 / 0.8 |
| Cf-zcp | 6.43 [6.18-6.77] | 7.99 [7.72-8.20] | -3.20 -2.75 -3.08 (0.45) | 80.8 / 0.0 / 0.6 |
| Cf-zcw | 6.30 [6.15-6.56] | 7.95 [7.71-8.16] | -3.21 -2.99 -3.03 (0.22) | 80.2 / 0.0 / 0.4 |

## d4k1

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 1.96 [1.87-2.55] | 2.33 [2.28-2.43] |  | 30.2 / 0.0 / 0.0 |
| Cf | 2.06 [1.98-2.16] | 2.43 [2.25-2.48] | +0.13 +0.11 -0.21 (0.34) | 28.1 / 0.0 / 0.0 |
| Cf-encp | 1.92 [1.86-2.26] | 2.54 [2.46-2.58] | -0.01 +0.02 -0.36 (0.37) | 19.1 / 0.0 / 0.0 |
| Cf-zc | 1.50 [1.41-1.64] | 2.19 [2.13-2.26] | -0.46 -0.39 -0.80 (0.41) | 26.9 / 0.0 / 0.0 |
| Cf-zcp | 1.49 [1.40-1.59] | 2.16 [2.05-2.22] | -0.46 -0.45 -0.76 (0.31) | 20.9 / 0.0 / 0.0 |
| Cf-zcw | 1.53 [1.44-1.62] | 2.13 [2.01-2.28] | -0.40 -0.42 -0.73 (0.33) | 26.5 / 0.0 / 0.0 |

## d4k8

| cell | stack: CPU ms | stack: wall ms | stack: gap to A per rep (range) | stack: vcs / ics / minflt |
|---|---|---|---|---|
| A | 2.56 [2.27-3.02] | 2.17 [2.10-2.27] |  | 33.6 / 0.0 / 104.3 |
| Cf | 2.60 [2.52-2.74] | 2.24 [2.14-2.35] | +0.02 +0.07 +0.09 (0.06) | 23.0 / 0.0 / 0.4 |
| Cf-encp | 2.42 [2.30-2.57] | 2.15 [2.06-2.22] | -0.13 -0.17 -0.10 (0.07) | 33.9 / 0.1 / 0.3 |
| Cf-zc | 1.65 [1.62-1.73] | 2.06 [1.98-2.13] | -0.92 -0.93 -0.86 (0.08) | 23.9 / 0.0 / 0.2 |
| Cf-zcp | 1.64 [1.59-1.72] | 2.08 [1.97-2.20] | -0.97 -0.95 -0.85 (0.12) | 24.4 / 0.0 / 0.3 |
| Cf-zcw | 1.67 [1.61-1.76] | 2.09 [1.97-2.17] | -0.93 -0.95 -0.83 (0.12) | 25.1 / 0.0 / 0.3 |
