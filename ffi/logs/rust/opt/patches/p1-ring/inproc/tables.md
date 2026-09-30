# in-process comparison: commit d12bedd4 + UNCOMMITTED; 2026-09-30T04:08:23Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 218226, pinned socket
# cells A,Cf,Cf-cb,Df (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 c54k1 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition base: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 cba2edcf3dfe338f, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env base=main
# condition p1: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 703fd6b8b01aa872, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_SPARES=6,AK_SPARE_LOCK=1
# benchmark wall time 66 s

## d16k1

| cell | base: CPU ms | base: wall ms | base: gap to A per rep (range) | base: vcs / ics / minflt | p1: CPU ms | p1: wall ms | p1: gap to A per rep (range) | p1: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 8.03 [7.42-8.89] | 8.46 [8.13-9.16] |  | 118.2 / 0.0 / 0.0 | 11.09 [7.92-13.28] | 12.57 [9.55-14.01] |  | 57.4 / 0.0 / 1935.7 |
| Df | 8.18 [7.91-9.09] | 8.97 [8.41-9.17] | -0.10 +0.47 +0.40 (0.57) | 213.9 / 0.0 / 0.1 | 8.38 [8.08-9.53] | 8.91 [8.53-11.15] | -3.48 -3.47 -0.75 (2.73) | 227.6 / 0.0 / 0.2 |
| Cf | 8.19 [7.97-9.00] | 7.46 [7.29-8.69] | +0.10 +0.54 +0.12 (0.44) | 93.6 / 0.0 / 0.2 | 8.35 [7.96-8.99] | 8.47 [7.51-9.05] | -3.38 -3.71 -0.69 (3.02) | 88.2 / 0.0 / 0.1 |
| Cf-cb | 9.37 [8.60-10.49] | 8.53 [7.60-9.18] | +1.05 +1.65 +1.55 (0.60) | 192.4 / 0.0 / 128.4 | 9.26 [8.60-10.15] | 8.54 [7.57-8.86] | -2.49 -3.03 +0.18 (3.22) | 233.7 / 0.0 / 0.3 |

## d16k8

| cell | base: CPU ms | base: wall ms | base: gap to A per rep (range) | base: vcs / ics / minflt | p1: CPU ms | p1: wall ms | p1: gap to A per rep (range) | p1: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 12.04 [11.40-13.70] | 9.67 [9.17-10.39] |  | 101.9 / 0.0 / 1440.2 | 13.25 [12.41-14.51] | 10.23 [9.59-10.58] |  | 124.7 / 0.1 / 1896.2 |
| Df | 9.33 [8.92-9.77] | 8.50 [8.28-8.79] | -3.00 -2.43 -2.95 (0.57) | 93.6 / 0.0 / 16.7 | 9.39 [9.06-9.71] | 8.57 [8.34-8.84] | -3.74 -4.47 -3.56 (0.91) | 108.8 / 0.0 / 1.1 |
| Cf | 9.71 [9.47-10.80] | 8.39 [8.11-8.60] | -2.25 -2.34 -2.44 (0.19) | 75.6 / 0.1 / 127.4 | 9.44 [9.29-9.67] | 8.28 [8.00-8.54] | -3.74 -4.40 -3.50 (0.90) | 72.5 / 0.1 / 0.6 |
| Cf-cb | 10.62 [9.85-11.02] | 8.39 [8.15-8.51] | -1.79 -1.21 -1.68 (0.58) | 106.1 / 0.1 / 413.5 | 9.53 [9.25-9.93] | 8.27 [7.97-8.51] | -3.72 -4.23 -3.45 (0.77) | 102.8 / 0.1 / 0.7 |

## d4k1

| cell | base: CPU ms | base: wall ms | base: gap to A per rep (range) | base: vcs / ics / minflt | p1: CPU ms | p1: wall ms | p1: gap to A per rep (range) | p1: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.16 [1.86-2.51] | 2.44 [2.34-2.55] |  | 41.9 / 0.0 / 0.0 | 1.91 [1.82-2.50] | 2.41 [2.29-2.47] |  | 17.1 / 0.0 / 0.0 |
| Df | 1.97 [1.88-2.46] | 2.44 [2.35-2.56] | -0.01 -0.07 -0.20 (0.19) | 30.0 / 0.0 / 0.1 | 1.93 [1.86-2.49] | 2.44 [2.40-2.51] | -0.32 -0.02 +0.37 (0.69) | 17.2 / 0.0 / 0.0 |
| Cf | 2.00 [1.95-2.09] | 2.41 [2.22-2.50] | +0.03 -0.04 -0.23 (0.26) | 27.5 / 0.0 / 0.0 | 2.02 [1.95-2.29] | 2.40 [2.24-2.47] | -0.21 +0.11 +0.18 (0.39) | 25.9 / 0.0 / 0.1 |
| Cf-cb | 2.15 [2.06-2.46] | 2.37 [2.29-2.51] | +0.12 +0.12 +0.09 (0.03) | 44.6 / 0.0 / 0.1 | 2.13 [2.05-2.53] | 2.44 [2.34-2.49] | -0.10 +0.27 +0.27 (0.38) | 45.9 / 0.0 / 0.1 |

## c54k1

| cell | base: CPU ms | base: wall ms | base: gap to A per rep (range) | base: vcs / ics / minflt | p1: CPU ms | p1: wall ms | p1: gap to A per rep (range) | p1: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 1.93 [1.84-2.01] | 2.81 [2.68-3.26] |  | 12.4 / 0.0 / 0.0 | 1.95 [1.85-2.02] | 2.98 [2.74-3.19] |  | 14.3 / 0.0 / 0.0 |
| Df | 1.92 [1.84-1.99] | 2.77 [2.68-3.15] | +0.02 -0.02 -0.04 (0.06) | 10.8 / 0.0 / 0.0 | 1.91 [1.84-1.97] | 2.86 [2.71-3.10] | -0.03 -0.07 -0.03 (0.03) | 12.6 / 0.0 / 0.0 |
| Cf | 1.99 [1.90-2.10] | 3.24 [2.81-4.47] | +0.09 +0.07 +0.01 (0.08) | 17.3 / 0.0 / 0.1 | 1.97 [1.89-2.07] | 3.08 [2.82-3.66] | +0.02 +0.00 +0.04 (0.04) | 16.4 / 0.0 / 0.1 |
| Cf-cb | 1.98 [1.90-2.09] | 2.90 [2.80-3.98] | +0.09 +0.05 +0.03 (0.07) | 14.9 / 0.0 / 0.0 | 2.00 [1.91-2.08] | 3.16 [2.82-3.68] | +0.04 +0.02 +0.11 (0.09) | 16.2 / 0.0 / 0.0 |
