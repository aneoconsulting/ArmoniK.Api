# in-process comparison: commit e9a97c72 + UNCOMMITTED; 2026-09-30T05:02:16Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 257399, pinned socket
# cells A,Cf,Cf-enc,Cf-encp (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition p5: binary /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/stream_probe (sha256 50c149250dc6ea48, core /tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/wt-rust/ffi/poc/rust/target/release/deps/libak_core.so), env AK_SPARES=6,AK_SPARE_LOCK=1
# condition p5h16: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 e16b1e26ff51a910, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env LD_LIBRARY_PATH=/tmp/claude-1000/-home-csdt-work-aneo-armonik-ArmoniK-Api/25aa83e6-3099-4e10-96fd-6061e612205d/scratchpad/core-h2/release,AK_H2_COALESCE=16,AK_SPARES=6,AK_SPARE_LOCK=1
# benchmark wall time 52 s

## d16k1

| cell | p5: CPU ms | p5: wall ms | p5: gap to A per rep (range) | p5: vcs / ics / minflt | p5h16: CPU ms | p5h16: wall ms | p5h16: gap to A per rep (range) | p5h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 9.59 [7.45-12.22] | 10.45 [9.30-11.38] |  | 76.1 / 0.0 / 1032.3 | 10.47 [8.62-12.61] | 11.84 [9.38-13.10] |  | 63.8 / 0.0 / 1499.1 |
| Cf | 8.18 [7.89-9.14] | 8.43 [7.42-8.92] | -1.43 -1.85 -0.76 (1.09) | 90.6 / 0.0 / 0.1 | 6.52 [6.37-7.17] | 7.23 [7.08-8.72] | -4.29 -4.22 -3.48 (0.81) | 58.4 / 0.0 / 0.1 |
| Cf-enc | 7.76 [7.52-8.32] | 8.89 [8.62-9.30] | -1.75 -2.58 -1.24 (1.34) | 82.8 / 0.0 / 0.1 | 6.15 [5.95-6.45] | 8.35 [8.22-10.80] | -4.52 -4.74 -3.85 (0.89) | 54.8 / 0.0 / 0.2 |
| Cf-encp | 7.78 [7.48-8.56] | 9.07 [8.82-9.42] | -1.60 -2.68 -0.91 (1.77) | 95.6 / 0.0 / 0.1 | 5.98 [5.84-6.20] | 8.38 [8.22-10.78] | -4.68 -4.83 -4.05 (0.78) | 52.2 / 0.0 / 0.1 |

## d16k8

| cell | p5: CPU ms | p5: wall ms | p5: gap to A per rep (range) | p5: vcs / ics / minflt | p5h16: CPU ms | p5h16: wall ms | p5h16: gap to A per rep (range) | p5h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 13.42 [12.43-14.18] | 10.17 [9.78-10.67] |  | 109.7 / 0.0 / 1955.4 | 13.42 [12.42-14.53] | 9.86 [9.56-10.54] |  | 121.8 / 0.0 / 1927.9 |
| Cf | 9.41 [9.24-9.71] | 8.18 [7.98-8.47] | -3.90 -4.39 -3.77 (0.62) | 79.5 / 0.1 / 0.5 | 7.60 [7.42-7.74] | 6.93 [6.84-7.21] | -6.07 -6.37 -5.26 (1.11) | 52.1 / 0.1 / 0.6 |
| Cf-enc | 8.98 [8.67-9.33] | 8.41 [8.28-8.79] | -4.40 -5.00 -4.00 (1.00) | 81.0 / 0.1 / 0.6 | 7.12 [6.83-7.40] | 7.25 [7.03-7.53] | -6.50 -6.82 -5.75 (1.08) | 53.6 / 0.1 / 0.4 |
| Cf-encp | 9.01 [8.76-9.45] | 8.52 [8.38-8.84] | -4.34 -4.95 -3.98 (0.97) | 86.3 / 0.1 / 0.5 | 7.10 [6.89-7.55] | 7.32 [7.11-7.46] | -6.63 -6.86 -5.65 (1.21) | 52.6 / 0.1 / 0.5 |

## d4k1

| cell | p5: CPU ms | p5: wall ms | p5: gap to A per rep (range) | p5: vcs / ics / minflt | p5h16: CPU ms | p5h16: wall ms | p5h16: gap to A per rep (range) | p5h16: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 2.02 [1.89-2.60] | 2.43 [2.32-2.69] |  | 31.9 / 0.0 / 0.2 | 1.94 [1.84-2.44] | 2.40 [2.25-2.52] |  | 16.9 / 0.0 / 0.0 |
| Cf | 2.05 [1.96-2.19] | 2.30 [2.22-2.46] | +0.08 -0.16 -0.11 (0.24) | 26.7 / 0.0 / 0.0 | 1.60 [1.54-1.79] | 2.04 [2.01-2.22] | -0.37 -0.30 -0.34 (0.07) | 25.8 / 0.0 / 0.0 |
| Cf-enc | 1.97 [1.91-2.05] | 2.48 [2.42-2.59] | +0.01 -0.25 -0.20 (0.25) | 16.8 / 0.0 / 0.0 | 1.59 [1.52-1.79] | 2.17 [2.10-2.27] | -0.30 -0.35 -0.39 (0.10) | 23.3 / 0.0 / 0.0 |
| Cf-encp | 1.95 [1.89-2.07] | 2.51 [2.42-2.60] | -0.01 -0.26 -0.21 (0.25) | 17.4 / 0.0 / 0.0 | 1.55 [1.49-1.66] | 2.12 [2.08-2.18] | -0.37 -0.36 -0.48 (0.12) | 22.0 / 0.0 / 0.1 |
