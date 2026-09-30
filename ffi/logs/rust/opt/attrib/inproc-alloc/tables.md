# in-process comparison: commit 26a89727 + UNCOMMITTED; 2026-09-30T03:48:32Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 8 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 200951, pinned socket
# cells A,A2,Df,Cf,Ff (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d16k8 c54k1 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition default: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 76dcd73a0579989f, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env default=main
# condition tun: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 76dcd73a0579989f, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env GLIBC_TUNABLES=glibc.malloc.trim_threshold=268435456:glibc.malloc.mmap_threshold=33554432
# benchmark wall time 70 s

## d16k1

| cell | default: CPU ms | default: wall ms | default: gap to A per rep (range) | default: vcs / ics / minflt | tun: CPU ms | tun: wall ms | tun: gap to A per rep (range) | tun: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 7.67 [7.33-8.11] | 8.67 [8.21-9.05] |  | 75.1 / 0.0 / 0.0 | 7.84 [7.42-8.44] | 8.86 [8.25-10.30] |  | 78.4 / 0.0 / 0.0 |
| A2 | 7.70 [7.32-8.82] | 8.73 [8.19-8.96] | +0.29 -0.12 +0.07 (0.41) | 77.1 / 0.0 / 0.0 | 7.68 [7.33-8.48] | 8.87 [8.37-10.18] | -0.32 +0.04 -0.16 (0.37) | 69.4 / 0.0 / 0.0 |
| Df | 8.33 [8.04-9.72] | 9.04 [8.48-9.26] | +0.56 +0.52 +1.10 (0.58) | 228.2 / 0.0 / 0.2 | 8.22 [7.92-9.52] | 9.08 [8.86-11.46] | +0.36 +0.59 +0.44 (0.23) | 180.6 / 0.0 / 0.1 |
| Cf | 8.32 [7.96-9.52] | 8.08 [7.42-8.60] | +1.46 +0.39 +0.61 (1.06) | 86.4 / 0.0 / 94.2 | 8.15 [7.94-9.31] | 7.96 [7.38-8.71] | +0.26 +0.63 +0.49 (0.38) | 85.8 / 0.0 / 0.1 |
| Ff | 8.29 [7.94-10.04] | 8.84 [8.51-10.16] | +0.52 +0.29 +1.86 (1.57) | 203.1 / 0.0 / 0.1 | 8.33 [8.00-9.78] | 8.90 [8.55-9.91] | +0.27 +0.81 +0.70 (0.55) | 194.3 / 0.0 / 0.0 |

## d16k8

| cell | default: CPU ms | default: wall ms | default: gap to A per rep (range) | default: vcs / ics / minflt | tun: CPU ms | tun: wall ms | tun: gap to A per rep (range) | tun: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 12.18 [11.16-13.09] | 9.88 [9.17-10.63] |  | 102.6 / 0.0 / 1331.2 | 9.21 [8.84-10.11] | 8.82 [8.38-9.37] |  | 74.6 / 0.0 / 207.9 |
| A2 | 11.74 [10.90-12.97] | 9.90 [9.32-10.27] | -0.81 -0.80 +0.62 (1.43) | 80.7 / 0.0 / 1428.5 | 9.27 [8.78-10.05] | 8.72 [8.51-9.22] | +0.14 -0.17 +0.12 (0.31) | 78.5 / 0.1 / 224.0 |
| Df | 9.31 [8.92-9.59] | 8.74 [8.29-9.48] | -2.83 -3.23 -2.38 (0.85) | 95.4 / 0.0 / 1.2 | 9.47 [9.18-9.91] | 8.73 [8.48-8.97] | +0.39 +0.27 +0.13 (0.26) | 108.1 / 0.0 / 32.1 |
| Cf | 9.99 [9.50-10.98] | 8.54 [8.22-8.69] | -2.21 -2.65 -1.38 (1.28) | 73.7 / 0.1 / 192.8 | 9.42 [9.22-9.93] | 8.30 [8.19-8.47] | +0.34 +0.02 +0.18 (0.32) | 72.7 / 0.1 / 0.7 |
| Ff | 9.45 [9.11-9.89] | 8.72 [8.37-9.28] | -2.80 -3.04 -1.95 (1.08) | 102.0 / 0.0 / 32.2 | 9.45 [9.11-9.96] | 8.72 [8.51-9.20] | +0.19 +0.27 +0.13 (0.14) | 102.1 / 0.0 / 32.1 |

## c54k1

| cell | default: CPU ms | default: wall ms | default: gap to A per rep (range) | default: vcs / ics / minflt | tun: CPU ms | tun: wall ms | tun: gap to A per rep (range) | tun: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|
| A | 1.94 [1.85-2.04] | 2.72 [2.69-2.91] |  | 11.7 / 0.0 / 0.0 | 1.95 [1.86-2.03] | 2.93 [2.69-3.21] |  | 13.5 / 0.0 / 0.0 |
| A2 | 1.92 [1.82-2.01] | 2.73 [2.70-2.92] | -0.07 -0.04 +0.02 (0.09) | 11.7 / 0.0 / 0.0 | 1.96 [1.88-2.05] | 2.86 [2.70-3.30] | -0.03 +0.07 +0.01 (0.10) | 12.3 / 0.0 / 0.0 |
| Df | 1.92 [1.82-1.99] | 2.71 [2.66-2.95] | -0.01 -0.03 -0.07 (0.06) | 10.3 / 0.0 / 0.1 | 1.88 [1.81-1.96] | 2.86 [2.66-3.07] | -0.12 -0.05 -0.05 (0.07) | 11.7 / 0.0 / 0.0 |
| Cf | 1.99 [1.90-2.08] | 2.83 [2.79-3.10] | +0.06 -0.03 +0.06 (0.09) | 16.1 / 0.0 / 0.1 | 1.94 [1.86-2.04] | 2.84 [2.80-3.13] | -0.05 +0.01 -0.02 (0.06) | 16.8 / 0.0 / 0.1 |
| Ff | 1.92 [1.83-1.98] | 2.71 [2.67-3.04] | -0.01 -0.07 -0.04 (0.05) | 10.2 / 0.0 / 0.0 | 1.89 [1.81-1.97] | 2.72 [2.67-3.16] | -0.06 -0.08 -0.09 (0.03) | 11.1 / 0.0 / 0.0 |
