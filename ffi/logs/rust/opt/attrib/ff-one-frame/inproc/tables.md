# in-process comparison: commit d12bedd4 + UNCOMMITTED; 2026-09-30T04:12:41Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 223639, pinned socket
# cells A,Df,Ff,Ff-1f,Cf (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 d4k1 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition head: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 30507604f4cfd5e6, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env head=main
# benchmark wall time 34 s

## d16k1

| cell | head: CPU ms | head: wall ms | head: gap to A per rep (range) | head: vcs / ics / minflt |
|---|---|---|---|---|
| A | 7.99 [7.58-8.97] | 8.95 [8.39-9.50] |  | 91.7 / 0.0 / 0.0 |
| Df | 8.90 [8.20-11.08] | 9.16 [8.66-9.65] | +0.99 +0.50 +1.17 (0.67) | 250.8 / 0.0 / 0.2 |
| Cf | 8.53 [8.07-9.63] | 7.79 [7.48-8.58] | +0.14 +0.56 +0.80 (0.66) | 107.5 / 0.0 / 65.2 |
| Ff | 8.62 [8.23-10.22] | 8.99 [8.55-9.38] | +0.65 +0.44 +0.71 (0.27) | 232.1 / 0.0 / 0.1 |
| Ff-1f | 7.98 [7.67-8.56] | 8.73 [8.42-9.16] | -0.31 +0.02 +0.11 (0.42) | 117.2 / 0.0 / 0.0 |

## d16k8

| cell | head: CPU ms | head: wall ms | head: gap to A per rep (range) | head: vcs / ics / minflt |
|---|---|---|---|---|
| A | 12.67 [11.42-13.92] | 10.04 [9.48-10.39] |  | 103.1 / 0.0 / 1584.8 |
| Df | 9.47 [9.23-9.89] | 9.04 [8.58-9.40] | -3.22 -3.87 -2.33 (1.54) | 99.4 / 0.0 / 32.3 |
| Cf | 10.24 [9.62-11.33] | 8.44 [8.17-8.82] | -2.55 -3.09 -1.47 (1.63) | 80.7 / 0.1 / 188.8 |
| Ff | 9.45 [9.00-10.24] | 8.75 [8.42-9.64] | -3.24 -4.26 -2.27 (1.99) | 101.9 / 0.0 / 16.2 |
| Ff-1f | 9.12 [8.73-9.61] | 8.87 [8.46-9.38] | -3.57 -4.24 -2.63 (1.62) | 78.2 / 0.0 / 64.3 |

## d4k1

| cell | head: CPU ms | head: wall ms | head: gap to A per rep (range) | head: vcs / ics / minflt |
|---|---|---|---|---|
| A | 2.14 [1.96-2.69] | 2.48 [2.37-2.61] |  | 40.0 / 0.0 / 30.1 |
| Df | 2.04 [1.99-2.17] | 2.48 [2.39-2.58] | -0.30 -0.07 -0.01 (0.30) | 15.2 / 0.0 / 0.1 |
| Cf | 2.14 [2.07-2.27] | 2.53 [2.43-2.62] | -0.18 -0.01 +0.09 (0.27) | 35.4 / 0.0 / 0.1 |
| Ff | 2.05 [1.98-2.53] | 2.54 [2.49-2.64] | -0.29 -0.02 -0.02 (0.28) | 17.5 / 0.0 / 0.1 |
| Ff-1f | 2.01 [1.93-2.46] | 2.51 [2.42-2.59] | -0.33 -0.07 -0.07 (0.26) | 16.9 / 0.0 / 0.0 |
