# in-process comparison: commit 5638bd09; 2026-09-30T05:04:03Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 259204, pinned socket
# cells A,Df,Cf,Cf-cb (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 d4k8 c54k1 c54k8 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition ctl: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e16b1e26ff51a910, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env LD_LIBRARY_PATH=/tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/core-ctl/release,AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# condition h16: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e16b1e26ff51a910, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env LD_LIBRARY_PATH=/tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/core-h2/release,AK_H2_COALESCE=16,AK_SPARES=6,AK_SPARE_LOCK=1,GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 92 s

## d16k1

| cell | ctl: CPU ms | ctl: wall ms | ctl: gap to A per rep (range) | ctl: vcs / ics / minflt | h16: CPU ms | h16: wall ms | h16: gap to A per rep (range) | h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 7.86 [7.49-8.54] | 9.00 [8.54-10.86] |  | 93.8 / 0.0 / 0.0 | 7.89 [7.46-8.71] | 8.97 [8.70-9.37] |  | 102.2 / 0.0 / 0.1 |
| Df | 8.42 [7.96-9.86] | 8.82 [8.31-10.91] | +1.28 +0.38 +0.30 (0.98) | 212.6 / 0.0 / 0.2 | 8.18 [7.92-8.67] | 9.15 [8.48-12.42] | +0.45 +0.20 +0.27 (0.25) | 195.4 / 0.0 / 0.2 |
| Cf | 8.42 [8.05-9.26] | 7.96 [7.34-8.50] | +0.46 +1.04 +0.12 (0.92) | 109.1 / 0.0 / 0.1 | 6.58 [6.48-7.21] | 7.34 [7.12-8.00] | -1.08 -1.22 -1.65 (0.57) | 52.9 / 0.0 / 0.1 |
| Cf-cb | 9.28 [8.73-10.26] | 8.60 [7.81-9.36] | +1.11 +2.22 +0.96 (1.26) | 220.5 / 0.0 / 0.1 | 7.40 [6.89-7.89] | 7.46 [7.19-8.41] | -0.58 -0.22 -1.23 (1.01) | 92.8 / 0.0 / 0.2 |

## d16k8

| cell | ctl: CPU ms | ctl: wall ms | ctl: gap to A per rep (range) | ctl: vcs / ics / minflt | h16: CPU ms | h16: wall ms | h16: gap to A per rep (range) | h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 9.37 [8.99-10.21] | 8.78 [8.52-9.18] |  | 73.7 / 0.0 / 128.5 | 9.45 [8.94-10.30] | 8.94 [8.45-9.84] |  | 73.6 / 0.0 / 208.4 |
| Df | 9.52 [9.07-10.08] | 8.76 [8.43-9.18] | -0.07 +0.68 -0.03 (0.75) | 111.8 / 0.0 / 16.5 | 9.57 [9.25-9.96] | 8.83 [8.46-8.94] | +0.26 +0.20 -0.04 (0.30) | 106.7 / 0.0 / 1.1 |
| Cf | 9.56 [9.35-9.84] | 8.37 [8.21-8.68] | +0.13 +0.42 +0.10 (0.32) | 72.3 / 0.1 / 0.5 | 7.64 [7.46-7.97] | 7.07 [6.87-7.36] | -1.79 -1.74 -1.94 (0.20) | 51.1 / 0.1 / 0.7 |
| Cf-cb | 9.60 [9.27-9.91] | 8.34 [8.22-8.51] | +0.38 +0.38 +0.01 (0.37) | 105.5 / 0.1 / 0.8 | 7.72 [7.50-7.92] | 6.93 [6.85-7.18] | -1.82 -1.68 -1.78 (0.14) | 67.9 / 0.1 / 0.8 |

## d4k1

| cell | ctl: CPU ms | ctl: wall ms | ctl: gap to A per rep (range) | ctl: vcs / ics / minflt | h16: CPU ms | h16: wall ms | h16: gap to A per rep (range) | h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.13 [1.94-2.71] | 2.55 [2.41-2.66] |  | 42.4 / 0.0 / 0.0 | 2.13 [2.01-2.67] | 2.44 [2.34-2.62] |  | 24.9 / 0.0 / 0.0 |
| Df | 2.09 [1.99-2.63] | 2.48 [2.41-2.60] | +0.14 -0.34 +0.05 (0.48) | 24.5 / 0.0 / 0.1 | 2.07 [1.99-2.34] | 2.52 [2.39-2.56] | -0.09 -0.07 -0.04 (0.05) | 16.0 / 0.0 / 0.0 |
| Cf | 2.18 [2.10-2.32] | 2.35 [2.29-2.56] | -0.05 -0.27 +0.22 (0.50) | 42.1 / 0.0 / 0.0 | 1.74 [1.70-1.83] | 2.20 [2.17-2.23] | -0.42 -0.39 -0.37 (0.05) | 23.8 / 0.0 / 0.0 |
| Cf-cb | 2.34 [2.21-2.67] | 2.52 [2.34-2.64] | +0.15 -0.12 +0.34 (0.46) | 57.4 / 0.0 / 0.1 | 1.84 [1.79-1.98] | 2.19 [2.18-2.26] | -0.31 -0.30 -0.26 (0.05) | 35.0 / 0.0 / 0.1 |

## d4k8

| cell | ctl: CPU ms | ctl: wall ms | ctl: gap to A per rep (range) | ctl: vcs / ics / minflt | h16: CPU ms | h16: wall ms | h16: gap to A per rep (range) | h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.53 [2.30-3.01] | 2.20 [2.09-2.28] |  | 31.8 / 0.0 / 96.5 | 2.52 [2.25-3.03] | 2.21 [2.16-2.31] |  | 28.9 / 0.0 / 88.2 |
| Df | 2.35 [2.22-2.51] | 2.22 [2.13-2.29] | -0.10 -0.26 -0.07 (0.20) | 34.0 / 0.0 / 0.2 | 2.30 [2.20-2.47] | 2.23 [2.16-2.34] | -0.15 -0.27 -0.18 (0.12) | 29.7 / 0.0 / 0.2 |
| Cf | 2.58 [2.48-2.73] | 2.31 [2.19-2.44] | +0.09 +0.01 +0.05 (0.07) | 20.8 / 0.0 / 0.3 | 2.13 [2.04-2.27] | 2.05 [1.98-2.17] | -0.36 -0.38 -0.37 (0.02) | 17.3 / 0.0 / 0.4 |
| Cf-cb | 2.45 [2.34-2.63] | 2.25 [2.08-2.36] | -0.02 -0.15 -0.02 (0.13) | 25.8 / 0.1 / 0.5 | 2.04 [1.89-2.16] | 2.04 [1.94-2.10] | -0.41 -0.53 -0.47 (0.12) | 19.5 / 0.0 / 0.3 |

## c54k1

| cell | ctl: CPU ms | ctl: wall ms | ctl: gap to A per rep (range) | ctl: vcs / ics / minflt | h16: CPU ms | h16: wall ms | h16: gap to A per rep (range) | h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.00 [1.91-2.10] | 3.10 [2.71-4.60] |  | 12.7 / 0.0 / 0.0 | 2.04 [1.95-2.12] | 3.23 [2.76-4.42] |  | 12.7 / 0.0 / 0.0 |
| Df | 1.99 [1.90-2.10] | 4.23 [2.77-4.69] | -0.02 +0.07 -0.06 (0.13) | 12.9 / 0.0 / 0.1 | 2.00 [1.91-2.13] | 3.56 [2.92-4.46] | -0.01 -0.03 -0.04 (0.03) | 12.6 / 0.0 / 0.0 |
| Cf | 2.04 [1.97-2.13] | 3.11 [2.83-4.16] | +0.03 +0.07 +0.06 (0.03) | 16.8 / 0.0 / 0.1 | 1.68 [1.56-1.76] | 2.66 [2.49-3.31] | -0.40 -0.37 -0.35 (0.06) | 22.7 / 0.0 / 0.1 |
| Cf-cb | 2.06 [1.95-2.17] | 3.26 [2.83-4.85] | +0.03 +0.06 +0.11 (0.08) | 14.8 / 0.0 / 0.0 | 1.67 [1.55-1.76] | 2.64 [2.45-3.19] | -0.42 -0.36 -0.38 (0.05) | 19.4 / 0.0 / 0.0 |

## c54k8

| cell | ctl: CPU ms | ctl: wall ms | ctl: gap to A per rep (range) | ctl: vcs / ics / minflt | h16: CPU ms | h16: wall ms | h16: gap to A per rep (range) | h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.53 [2.03-3.40] | 2.51 [2.39-2.57] |  | 36.8 / 0.0 / 128.1 | 2.43 [2.06-3.09] | 2.43 [2.34-2.62] |  | 30.6 / 0.0 / 96.4 |
| Df | 2.12 [2.06-2.45] | 2.45 [2.26-2.72] | -0.04 -0.44 -0.52 (0.48) | 25.7 / 0.0 / 0.1 | 2.14 [2.06-2.48] | 2.53 [2.29-2.67] | -0.45 -0.20 +0.12 (0.58) | 32.5 / 0.0 / 0.1 |
| Cf | 2.53 [2.41-2.66] | 2.60 [2.41-2.67] | +0.23 -0.04 -0.07 (0.30) | 20.9 / 0.0 / 0.5 | 2.07 [1.98-2.22] | 2.35 [2.25-2.47] | -0.52 -0.35 -0.17 (0.35) | 19.0 / 0.0 / 0.5 |
| Cf-cb | 2.19 [2.10-2.48] | 2.56 [2.43-2.64] | -0.13 -0.42 -0.42 (0.29) | 32.2 / 0.0 / 0.2 | 1.71 [1.67-1.90] | 2.32 [2.12-2.52] | -0.77 -0.72 -0.58 (0.19) | 19.6 / 0.0 / 0.1 |
