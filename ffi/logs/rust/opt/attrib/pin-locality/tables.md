# in-process comparison: commit d12bedd4 + UNCOMMITTED; 2026-09-30T04:10:28Z; Intel(R) Core(TM) i9-7900X CPU @ 3.30GHz; kernel 6.18.54; no_turbo 1; cpu1 min/max 3300000/3300000 kHz; smt on; isolation: taskset only
# client 1-4,11-14, host 8 / core 1 workers (unless a condition sets them); server 5-8,15-18, 8 workers, pid 220167, pinned socket
# cells A,Cf (one process per condition x workload x repetition, block order, cell order rotated per repetition); workloads d16k1 d4k1 (d16k1 8 x 8 calls warm 4; d16k8 6 x 2 batches of 8 warm 2; d4k1 8 x 16 warm 8; d4k8 6 x 4 x 8 warm 2; c54k1 8 x 16 warm 8; c54k8 6 x 4 x 8 warm 2); repetitions 3
# condition free: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 eb789d38975f7104, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env free=main
# condition sib: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 eb789d38975f7104, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env AK_PIN_CALLER=1,AK_PIN_CORE_RT=11
# condition apart: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 eb789d38975f7104, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env AK_PIN_CALLER=1,AK_PIN_CORE_RT=2
# condition same: binary /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/stream_probe (sha256 eb789d38975f7104, core /home/csdt/work/aneo/armonik/ArmoniK.Api/ffi/poc/rust/target/release/deps/libak_core.so), env AK_PIN_CALLER=1,AK_PIN_CORE_RT=1
# benchmark wall time 31 s

## d16k1

| cell | free: CPU ms | free: wall ms | free: gap to A per rep (range) | free: vcs / ics / minflt | sib: CPU ms | sib: wall ms | sib: gap to A per rep (range) | sib: vcs / ics / minflt | apart: CPU ms | apart: wall ms | apart: gap to A per rep (range) | apart: vcs / ics / minflt | same: CPU ms | same: wall ms | same: gap to A per rep (range) | same: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | 7.70 [7.35-8.26] | 9.41 [8.55-10.47] |  | 67.8 / 0.0 / 0.0 | 7.83 [7.47-8.51] | 9.99 [8.49-10.47] |  | 73.6 / 0.0 / 0.0 | 7.78 [7.37-8.42] | 10.09 [9.74-11.11] |  | 69.6 / 0.0 / 0.0 | 7.70 [7.31-8.53] | 9.38 [8.85-10.42] |  | 71.8 / 0.0 / 0.0 |
| Cf | 7.95 [7.76-8.81] | 8.62 [7.66-9.58] | +0.14 +0.29 +0.32 (0.19) | 35.1 / 0.0 / 64.1 | 8.98 [8.77-10.56] | 9.21 [8.74-9.67] | +0.86 +1.29 +1.34 (0.48) | 30.6 / 0.0 / 65.0 | 7.85 [7.70-8.47] | 8.81 [8.21-9.49] | +0.30 -0.04 +0.12 (0.34) | 32.6 / 0.0 / 30.6 | 7.63 [7.45-8.11] | 10.18 [9.32-10.84] | +0.01 -0.45 +0.08 (0.52) | 29.4 / 5.1 / 1.1 |

## d4k1

| cell | free: CPU ms | free: wall ms | free: gap to A per rep (range) | free: vcs / ics / minflt | sib: CPU ms | sib: wall ms | sib: gap to A per rep (range) | sib: vcs / ics / minflt | apart: CPU ms | apart: wall ms | apart: gap to A per rep (range) | apart: vcs / ics / minflt | same: CPU ms | same: wall ms | same: gap to A per rep (range) | same: vcs / ics / minflt |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | 2.01 [1.87-2.53] | 2.69 [2.44-3.62] |  | 26.8 / 0.0 / 0.2 | 1.97 [1.88-2.36] | 3.42 [2.84-4.18] |  | 15.6 / 0.0 / 15.2 | 2.00 [1.90-2.79] | 3.23 [2.94-3.76] |  | 21.1 / 0.0 / 15.2 | 2.00 [1.88-2.52] | 3.14 [2.88-3.79] |  | 25.9 / 0.0 / 0.0 |
| Cf | 1.94 [1.86-2.04] | 3.42 [3.11-4.43] | +0.06 -0.10 -0.32 (0.38) | 11.8 / 0.0 / 0.0 | 2.15 [2.10-2.25] | 3.78 [3.39-4.47] | +0.18 +0.21 +0.16 (0.05) | 11.2 / 0.0 / 0.0 | 1.94 [1.85-2.06] | 3.32 [2.75-3.76] | -0.10 -0.17 -0.00 (0.17) | 12.3 / 0.0 / 0.0 | 1.85 [1.76-1.98] | 3.60 [2.53-4.00] | -0.11 -0.27 -0.10 (0.17) | 13.7 / 2.2 / 0.0 |
