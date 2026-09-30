# p6-zero-copy: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## measure: main figures (every process under AB_ENV, the allocator setting; 3 rounds)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | p6 | 8.281 [8.183-8.450] | 8.234 [8.120-8.740] | 0.1 | 225.2 | 30 |
| d/16MiB k=1 | Cf-retain | p6 | 8.457 [8.303-8.713] | 8.248 [7.504-8.928] | 0.0 | 108.5 | 30 |
| d/16MiB k=1 | Cf-encp-retain | p6 | 7.938 [7.794-8.216] | 9.025 [8.435-9.940] | 0.1 | 102.6 | 30 |
| d/16MiB k=1 | Cf-zc-retain | p6 | 6.862 [6.691-7.107] | 7.788 [7.168-8.574] | 0.2 | 177.7 | 30 |
| d/16MiB k=8 | A | p6 | 9.682 [9.463-9.912] | 7.542 [7.298-7.736] | 1.7 | 191.8 | 30 |
| d/16MiB k=8 | Cf-retain | p6 | 9.698 [9.522-9.845] | 8.352 [8.093-8.523] | 0.4 | 76.0 | 30 |
| d/16MiB k=8 | Cf-encp-retain | p6 | 9.186 [8.981-9.372] | 8.609 [8.403-8.754] | 0.2 | 92.0 | 30 |
| d/16MiB k=8 | Cf-zc-retain | p6 | 6.646 [6.464-6.810] | 8.073 [7.877-8.293] | 0.3 | 82.4 | 30 |
| d/4MiB k=1 | A | p6 | 2.171 [2.094-2.294] | 2.321 [2.204-2.516] | 0.0 | 60.0 | 30 |
| d/4MiB k=1 | Cf-retain | p6 | 2.174 [2.036-2.241] | 2.466 [2.272-2.605] | 0.0 | 34.7 | 30 |
| d/4MiB k=1 | Cf-encp-retain | p6 | 2.185 [1.985-2.246] | 2.585 [2.485-2.725] | 0.0 | 27.8 | 30 |
| d/4MiB k=1 | Cf-zc-retain | p6 | 1.599 [1.531-1.709] | 2.193 [2.041-2.350] | 0.1 | 28.7 | 30 |
| d/4MiB k=8 | A | p6 | 2.596 [2.543-2.669] | 2.166 [2.078-2.282] | 0.1 | 54.0 | 30 |
| d/4MiB k=8 | Cf-retain | p6 | 2.635 [2.571-2.717] | 2.299 [2.224-2.379] | 0.1 | 21.0 | 30 |
| d/4MiB k=8 | Cf-encp-retain | p6 | 2.429 [2.348-2.482] | 2.221 [2.151-2.254] | 0.1 | 33.7 | 30 |
| d/4MiB k=8 | Cf-zc-retain | p6 | 1.691 [1.646-1.723] | 2.111 [2.012-2.168] | 0.1 | 24.0 | 30 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p6 | 2.023 | 0.018 | 6.367 |  |
| d/16MiB k=1 | Cf-encp-retain | p6 | 0.111 | 0.017 | 7.894 |  |
| d/16MiB k=1 | Cf-zc-retain | p6 | 0.113 | 0.017 | 6.797 |  |
| d/16MiB k=8 | Cf-retain | p6 | 2.682 | 0.007 | 7.004 |  |
| d/16MiB k=8 | Cf-encp-retain | p6 | 0.133 | 0.007 | 8.999 |  |
| d/16MiB k=8 | Cf-zc-retain | p6 | 0.145 | 0.007 | 6.460 |  |
| d/4MiB k=1 | Cf-retain | p6 | 0.533 | 0.012 | 1.639 |  |
| d/4MiB k=1 | Cf-encp-retain | p6 | 0.042 | 0.015 | 2.142 |  |
| d/4MiB k=1 | Cf-zc-retain | p6 | 0.042 | 0.014 | 1.540 |  |
| d/4MiB k=8 | Cf-retain | p6 | 0.858 | 0.005 | 1.765 |  |
| d/4MiB k=8 | Cf-encp-retain | p6 | 0.048 | 0.005 | 2.351 |  |
| d/4MiB k=8 | Cf-zc-retain | p6 | 0.049 | 0.005 | 1.644 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p6 | -0.001 +0.478 +0.165 | +0.091 +0.081 -0.450 |
| d/16MiB k=1 | Cf-encp-retain | p6 | -0.348 -0.205 -0.408 | +1.275 +0.773 +0.629 |
| d/16MiB k=1 | Cf-zc-retain | p6 | -1.461 -1.295 -1.426 | -0.163 -0.840 -0.626 |
| d/16MiB k=8 | Cf-retain | p6 | +0.073 -0.076 +0.125 | +0.829 +0.806 +0.917 |
| d/16MiB k=8 | Cf-encp-retain | p6 | -0.551 -0.605 -0.364 | +1.105 +1.086 +1.144 |
| d/16MiB k=8 | Cf-zc-retain | p6 | -2.999 -3.099 -2.967 | +0.700 +0.479 +0.600 |
| d/4MiB k=1 | Cf-retain | p6 | -0.062 +0.029 -0.106 | +0.094 +0.219 +0.079 |
| d/4MiB k=1 | Cf-encp-retain | p6 | +0.123 +0.014 -0.257 | +0.311 +0.351 +0.089 |
| d/4MiB k=1 | Cf-zc-retain | p6 | -0.541 -0.466 -0.694 | -0.008 +0.001 -0.441 |
| d/4MiB k=8 | Cf-retain | p6 | +0.025 +0.072 +0.021 | +0.165 +0.105 +0.118 |
| d/4MiB k=8 | Cf-encp-retain | p6 | -0.170 -0.173 -0.190 | +0.069 +0.017 +0.053 |
| d/4MiB k=8 | Cf-zc-retain | p6 | -0.921 -0.863 -0.949 | -0.037 -0.090 -0.052 |

## default: the default-allocator pass (1 round)

| workload | cell | arm | CPU ms | wall ms | flt | csw | n |
|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | p6 | 8.520 [8.471-8.671] | 8.988 [8.712-9.323] | 0.1 | 181.2 | 10 |
| d/16MiB k=1 | Cf-retain | p6 | 8.466 [8.281-8.817] | 9.453 [8.569-9.955] | 0.0 | 87.0 | 10 |
| d/16MiB k=1 | Cf-encp-retain | p6 | 8.554 [8.387-8.943] | 9.275 [9.044-9.588] | 0.1 | 181.3 | 10 |
| d/16MiB k=1 | Cf-zc-retain | p6 | 6.369 [6.247-6.660] | 8.579 [8.202-10.186] | 0.1 | 109.2 | 10 |
| d/16MiB k=8 | A | p6 | 9.871 [9.739-9.962] | 7.696 [7.559-7.814] | 3.3 | 193.7 | 10 |
| d/16MiB k=8 | Cf-retain | p6 | 9.774 [9.705-9.960] | 8.488 [8.331-8.620] | 0.4 | 86.2 | 10 |
| d/16MiB k=8 | Cf-encp-retain | p6 | 9.245 [8.931-9.575] | 8.806 [8.516-8.842] | 0.3 | 90.0 | 10 |
| d/16MiB k=8 | Cf-zc-retain | p6 | 6.519 [6.364-6.654] | 8.229 [8.014-8.409] | 0.3 | 87.7 | 10 |
| d/4MiB k=1 | A | p6 | 2.227 [2.148-2.293] | 2.379 [2.291-2.471] | 0.0 | 56.2 | 10 |
| d/4MiB k=1 | Cf-retain | p6 | 2.149 [2.104-2.220] | 2.371 [2.247-2.515] | 0.0 | 36.3 | 10 |
| d/4MiB k=1 | Cf-encp-retain | p6 | 2.065 [1.985-2.259] | 2.540 [2.496-2.573] | 0.0 | 28.2 | 10 |
| d/4MiB k=1 | Cf-zc-retain | p6 | 1.525 [1.513-1.566] | 2.143 [2.118-2.188] | 0.1 | 17.2 | 10 |
| d/4MiB k=8 | A | p6 | 2.662 [2.606-2.713] | 2.173 [2.050-2.332] | 0.0 | 55.7 | 10 |
| d/4MiB k=8 | Cf-retain | p6 | 2.628 [2.594-2.679] | 2.332 [2.236-2.366] | 0.1 | 21.5 | 10 |
| d/4MiB k=8 | Cf-encp-retain | p6 | 2.449 [2.420-2.530] | 2.218 [2.162-2.322] | 0.1 | 34.0 | 10 |
| d/4MiB k=8 | Cf-zc-retain | p6 | 1.642 [1.612-1.659] | 2.094 [2.047-2.189] | 0.2 | 23.5 | 10 |

Per-thread CPU per call, Cf and Cf-q (ms, median of the processes):

| workload | cell | arm | caller | main | tokio-rt-worker | event_engine |
|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p6 | 2.083 | 0.017 | 6.415 |  |
| d/16MiB k=1 | Cf-encp-retain | p6 | 0.119 | 0.017 | 8.472 |  |
| d/16MiB k=1 | Cf-zc-retain | p6 | 0.122 | 0.018 | 6.271 |  |
| d/16MiB k=8 | Cf-retain | p6 | 2.745 | 0.007 | 7.078 |  |
| d/16MiB k=8 | Cf-encp-retain | p6 | 0.141 | 0.007 | 9.103 |  |
| d/16MiB k=8 | Cf-zc-retain | p6 | 0.151 | 0.007 | 6.355 |  |
| d/4MiB k=1 | Cf-retain | p6 | 0.536 | 0.014 | 1.620 |  |
| d/4MiB k=1 | Cf-encp-retain | p6 | 0.043 | 0.015 | 2.043 |  |
| d/4MiB k=1 | Cf-zc-retain | p6 | 0.044 | 0.014 | 1.477 |  |
| d/4MiB k=8 | Cf-retain | p6 | 0.837 | 0.005 | 1.791 |  |
| d/4MiB k=8 | Cf-encp-retain | p6 | 0.050 | 0.005 | 2.409 |  |
| d/4MiB k=8 | Cf-zc-retain | p6 | 0.049 | 0.005 | 1.588 |  |

Cf-* - A per round (ms per call, CPU / wall; median of the Cf process minus median of the A process of the same arm and round):

| workload | cell | arm | per round (CPU) | per round (wall) |
|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | p6 | -0.054 | +0.465 |
| d/16MiB k=1 | Cf-encp-retain | p6 | +0.034 | +0.287 |
| d/16MiB k=1 | Cf-zc-retain | p6 | -2.151 | -0.408 |
| d/16MiB k=8 | Cf-retain | p6 | -0.097 | +0.792 |
| d/16MiB k=8 | Cf-encp-retain | p6 | -0.627 | +1.111 |
| d/16MiB k=8 | Cf-zc-retain | p6 | -3.352 | +0.533 |
| d/4MiB k=1 | Cf-retain | p6 | -0.078 | -0.007 |
| d/4MiB k=1 | Cf-encp-retain | p6 | -0.162 | +0.161 |
| d/4MiB k=1 | Cf-zc-retain | p6 | -0.701 | -0.236 |
| d/4MiB k=8 | Cf-retain | p6 | -0.034 | +0.159 |
| d/4MiB k=8 | Cf-encp-retain | p6 | -0.213 | +0.045 |
| d/4MiB k=8 | Cf-zc-retain | p6 | -1.020 | -0.079 |

## strace: syscalls per call (between the loop's markers, fewer batches)

| workload | cell | arm | calls | socket writes | bytes/write | socket reads | epoll_wait | futex | all syscalls |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | p6 | 12 | 40.4 | 415341 | 46.8 | 66.8 | 186.0 | 355.1 |
| d/16MiB k=1 | Cf-retain | p6 | 12 | 1026.4 | 16355 | 11.7 | 1017.8 | 48.5 | 2115.3 |
| d/16MiB k=1 | Cf-encp-retain | p6 | 12 | 1026.3 | 16356 | 10.1 | 1007.5 | 48.8 | 2101.7 |
| d/16MiB k=1 | Cf-zc-retain | p6 | 12 | 1026.1 | 16360 | 10.8 | 1017.6 | 60.8 | 2126.7 |
| d/16MiB k=8 | A | p6 | 24 | 34.0 | 493122 | 35.3 | 50.6 | 158.8 | 284.0 |
| d/16MiB k=8 | Cf-retain | p6 | 24 | 1028.1 | 16328 | 9.8 | 1000.2 | 56.0 | 2094.4 |
| d/16MiB k=8 | Cf-encp-retain | p6 | 24 | 1027.1 | 16344 | 9.4 | 996.3 | 58.6 | 2093.1 |
| d/16MiB k=8 | Cf-zc-retain | p6 | 24 | 1026.3 | 16356 | 9.8 | 962.1 | 66.6 | 2067.1 |
| d/4MiB k=1 | A | p6 | 40 | 12.3 | 341197 | 13.2 | 19.8 | 57.1 | 108.0 |
| d/4MiB k=1 | Cf-retain | p6 | 40 | 258.3 | 16248 | 3.6 | 261.1 | 26.2 | 555.0 |
| d/4MiB k=1 | Cf-encp-retain | p6 | 40 | 258.3 | 16246 | 4.3 | 262.3 | 21.4 | 550.6 |
| d/4MiB k=1 | Cf-zc-retain | p6 | 40 | 258.0 | 16266 | 3.7 | 262.9 | 21.4 | 551.3 |
| d/4MiB k=8 | A | p6 | 64 | 11.5 | 363451 | 10.2 | 15.5 | 55.4 | 95.2 |
| d/4MiB k=8 | Cf-retain | p6 | 64 | 256.6 | 16355 | 2.8 | 243.5 | 18.1 | 521.6 |
| d/4MiB k=8 | Cf-encp-retain | p6 | 64 | 256.6 | 16358 | 2.6 | 248.5 | 21.5 | 530.2 |
| d/4MiB k=8 | Cf-zc-retain | p6 | 64 | 256.5 | 16364 | 2.7 | 227.5 | 18.8 | 507.1 |

## perf: the split (perf attached; absolutes above are the no-perf phases')

perf stat per call (one process per cell, perf enabled around the loop): cycles and instructions user / kernel (M), cache-references and cache-misses (k), LLC-load-misses (k), faults, context switches.

| workload | cell | arm | cycles u / k | instr u / k | cache-ref | cache-miss | LLC-load-miss | faults | csw |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | p6 | 11.91 / 15.02 | 2.46 / 5.01 | 964 | 364 | 172 | 0.1 | 169.2 |
| d/16MiB k=1 | Cf-retain | p6 | 9.53 / 18.84 | 3.27 / 6.58 | 759 | 543 | 249 | 0.0 | 99.2 |
| d/16MiB k=1 | Cf-encp-retain | p6 | 8.97 / 17.20 | 3.23 / 6.38 | 765 | 478 | 231 | 0.0 | 84.1 |
| d/16MiB k=1 | Cf-zc-retain | p6 | 2.93 / 18.77 | 3.32 / 6.45 | 501 | 290 | 140 | 0.2 | 88.1 |
| d/16MiB k=8 | A | p6 | 13.98 / 17.92 | 2.78 / 5.37 | 857 | 508 | 231 | 0.2 | 172.8 |
| d/16MiB k=8 | Cf-retain | p6 | 11.63 / 20.62 | 3.32 / 6.61 | 716 | 427 | 198 | 0.2 | 57.1 |
| d/16MiB k=8 | Cf-encp-retain | p6 | 9.70 / 20.29 | 3.31 / 6.65 | 771 | 472 | 225 | 0.2 | 69.4 |
| d/16MiB k=8 | Cf-zc-retain | p6 | 3.09 / 18.50 | 3.34 / 6.57 | 451 | 181 | 95 | 0.4 | 66.7 |

### perf record buckets, d/16MiB k=1 (ms per call = cycles / 3.3e9; one process per cell)

| bucket | A p6 | Cf-retain p6 | Cf-encp-retain p6 | Cf-zc-retain p6 | A - Cf | Cf-encp-retain - Cf | Cf-zc-retain - Cf |
|---|---|---|---|---|---|---|---|
| k: epoll_wait | 0.226 | 0.259 | 0.295 | 0.341 | -0.033 | +0.036 | +0.081 |
| k: eventfd | 0.031 | 0.006 | 0.004 | 0.009 | +0.025 | -0.003 | +0.003 |
| k: futex | 0.564 | 0.098 | 0.109 | 0.138 | +0.466 | +0.010 | +0.040 |
| k: other | 0.040 | 0.000 | 0.000 | 0.000 | +0.040 | +0.000 | +0.000 |
| k: other syscalls | 0.152 | 0.069 | 0.050 | 0.073 | +0.083 | -0.019 | +0.004 |
| k: page faults (the fault path) | 0.001 | 0.000 | 0.000 | 0.000 | +0.001 | +0.000 | +0.000 |
| k: scheduling, interrupts | 0.059 | 0.016 | 0.014 | 0.009 | +0.043 | -0.002 | -0.007 |
| k: socket read (recvmsg/read) | 0.107 | 0.000 | 0.000 | 0.000 | +0.107 | +0.000 | +0.000 |
| k: socket write (sendmsg/writev) | 3.265 | 4.974 | 4.781 | 5.245 | -1.709 | -0.194 | +0.271 |
| u: allocation (malloc/free) | 0.074 | 0.022 | 0.018 | 0.013 | +0.052 | -0.004 | -0.009 |
| u: core rpc glue (ak_core rpc, FFI entry) | 0.000 | 0.033 | 0.058 | 0.050 | -0.033 | +0.024 | +0.017 |
| u: encode (core codec) | 0.000 | 0.006 | 0.004 | 0.003 | -0.006 | -0.003 | -0.003 |
| u: grpc-core (libgrpc, gpr, absl) | 1.275 | 0.000 | 0.000 | 0.000 | +1.275 | +0.000 | +0.000 |
| u: harness | 0.050 | 0.007 | 0.011 | 0.005 | +0.043 | +0.004 | -0.002 |
| u: hyper/h2/tonic/http/bytes | 0.000 | 0.344 | 0.344 | 0.299 | -0.344 | -0.000 | -0.045 |
| u: libc sync (pthread, futex wrappers) | 0.018 | 0.004 | 0.004 | 0.002 | +0.014 | -0.001 | -0.003 |
| u: libc syscall wrappers | 0.088 | 0.081 | 0.077 | 0.070 | +0.007 | -0.004 | -0.012 |
| u: memcpy in encode | 0.000 | 1.984 | 1.980 | 0.000 | -1.984 | -0.004 | -1.984 |
| u: memcpy in grpc/protobuf | 2.060 | 0.000 | 0.000 | 0.000 | +2.060 | +0.000 | +0.000 |
| u: memcpy in hyper/h2/bytes | 0.000 | 0.130 | 0.135 | 0.146 | -0.130 | +0.005 | +0.016 |
| u: memcpy other | 0.000 | 0.007 | 0.008 | 0.001 | -0.007 | +0.001 | -0.005 |
| u: other | 0.033 | 0.008 | 0.013 | 0.013 | +0.025 | +0.005 | +0.006 |
| u: protobuf | 0.014 | 0.000 | 0.000 | 0.000 | +0.014 | +0.000 | +0.000 |
| u: tokio/mio runtime and park | 0.000 | 0.252 | 0.211 | 0.242 | -0.252 | -0.042 | -0.010 |
| **total (sampled)** | 8.058 | 8.303 | 8.113 | 6.660 | -0.245 | -0.190 | -1.643 |

Threads (ms per call, sampled): A p6: event_engine 4.431, campaign_rpc 3.627, lifeguard 0.000; Cf-retain p6: tokio-rt-worker 6.189, campaign_rpc 2.114; Cf-encp-retain p6: tokio-rt-worker 8.001, campaign_rpc 0.113; Cf-zc-retain p6: tokio-rt-worker 6.542, campaign_rpc 0.117

Top symbols, d/16MiB k=1 A p6 (ms per call): __memmove_evex_unaligned_erms 2.057; [k] rep_movs_alternative 1.303; [k] clear_page_erms 0.769; [k] memset_orig 0.117; [k] _copy_from_iter 0.115; [k] syscall_return_via_sysret 0.096; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.074; [k] entry_SYSRETQ_unsafe_stack 0.067; [k] __alloc_tagging_slab_alloc_hook 0.064; [k] entry_SYSCALL_64 0.059; absl::lts_20260107::Mutex::lock() 0.059; [k] dequeue_entities 0.050; [k] _raw_spin_lock 0.045; __tls_get_addr 0.044; [k] __pgalloc_tag_add 0.042; [k] update_curr 0.038

Top symbols, d/16MiB k=1 Cf-retain p6 (ms per call): __memmove_evex_unaligned_erms 2.120; [k] rep_movs_alternative 1.668; [k] clear_page_erms 0.767; [k] syscall_return_via_sysret 0.241; [k] entry_SYSRETQ_unsafe_stack 0.162; [k] _copy_from_iter 0.121; [k] entry_SYSCALL_64_after_hwframe 0.120; [k] __alloc_tagging_slab_alloc_hook 0.110; [k] entry_SYSCALL_64 0.097; [k] native_queued_spin_lock_slowpath 0.087; [k] _raw_spin_lock_irqsave 0.085; [k] _raw_spin_lock 0.073; [k] __pgalloc_tag_add 0.073; [k] get_page_from_freelist 0.068; [k] unix_stream_sendmsg 0.065; [k] try_charge_memcg 0.062

Top symbols, d/16MiB k=1 Cf-encp-retain p6 (ms per call): __memmove_evex_unaligned_erms 2.123; [k] rep_movs_alternative 1.500; [k] clear_page_erms 0.821; [k] syscall_return_via_sysret 0.247; [k] entry_SYSRETQ_unsafe_stack 0.147; [k] __alloc_tagging_slab_alloc_hook 0.113; [k] entry_SYSCALL_64 0.111; [k] entry_SYSCALL_64_after_hwframe 0.111; [k] _copy_from_iter 0.104; [k] _raw_spin_lock_irqsave 0.092; [k] _raw_spin_lock 0.084; [k] native_queued_spin_lock_slowpath 0.074; [k] __pgalloc_tag_add 0.068; [k] page_ext_get 0.065; [k] try_charge_memcg 0.061; [k] vfs_writev 0.061

Top symbols, d/16MiB k=1 Cf-zc-retain p6 (ms per call): [k] rep_movs_alternative 2.048; [k] clear_page_erms 0.876; [k] syscall_return_via_sysret 0.220; [k] entry_SYSRETQ_unsafe_stack 0.180; __memmove_evex_unaligned_erms 0.147; [k] _copy_from_iter 0.115; [k] __alloc_tagging_slab_alloc_hook 0.113; [k] entry_SYSCALL_64 0.113; [k] entry_SYSCALL_64_after_hwframe 0.108; [k] _raw_spin_lock 0.086; [k] _raw_spin_lock_irqsave 0.068; [k] __pgalloc_tag_add 0.064; tokio_util::util::poll_buf::poll_write_buf 0.061; [k] __rmqueue_pcplist 0.060; [k] __list_del_entry_valid_or_report 0.057; [k] unix_stream_sendmsg 0.053

### perf record buckets, d/16MiB k=8 (ms per call = cycles / 3.3e9; one process per cell)

| bucket | A p6 | Cf-retain p6 | Cf-encp-retain p6 | Cf-zc-retain p6 | A - Cf | Cf-encp-retain - Cf | Cf-zc-retain - Cf |
|---|---|---|---|---|---|---|---|
| k: epoll_wait | 0.122 | 0.121 | 0.166 | 0.128 | +0.000 | +0.044 | +0.006 |
| k: eventfd | 0.006 | 0.000 | 0.006 | 0.002 | +0.006 | +0.006 | +0.002 |
| k: futex | 0.555 | 0.171 | 0.133 | 0.114 | +0.384 | -0.038 | -0.057 |
| k: other | 0.123 | 0.061 | 0.077 | 0.045 | +0.063 | +0.016 | -0.016 |
| k: other syscalls | 0.152 | 0.025 | 0.039 | 0.046 | +0.127 | +0.014 | +0.022 |
| k: page faults (the fault path) | 0.009 | 0.004 | 0.003 | 0.000 | +0.005 | -0.001 | -0.004 |
| k: scheduling, interrupts | 0.067 | 0.046 | 0.035 | 0.015 | +0.020 | -0.012 | -0.031 |
| k: socket read (recvmsg/read) | 0.099 | 0.000 | 0.000 | 0.000 | +0.099 | +0.000 | +0.000 |
| k: socket write (sendmsg/writev) | 4.045 | 5.786 | 5.576 | 5.249 | -1.740 | -0.210 | -0.537 |
| u: allocation (malloc/free) | 0.067 | 0.021 | 0.020 | 0.022 | +0.046 | -0.001 | +0.001 |
| u: core rpc glue (ak_core rpc, FFI entry) | 0.000 | 0.061 | 0.049 | 0.058 | -0.061 | -0.012 | -0.004 |
| u: encode (core codec) | 0.000 | 0.004 | 0.007 | 0.002 | -0.004 | +0.003 | -0.002 |
| u: grpc-core (libgrpc, gpr, absl) | 1.219 | 0.000 | 0.000 | 0.000 | +1.219 | +0.000 | +0.000 |
| u: harness | 0.038 | 0.004 | 0.003 | 0.005 | +0.034 | -0.001 | +0.000 |
| u: hyper/h2/tonic/http/bytes | 0.000 | 0.368 | 0.351 | 0.349 | -0.368 | -0.017 | -0.019 |
| u: libc sync (pthread, futex wrappers) | 0.012 | 0.003 | 0.002 | 0.003 | +0.009 | -0.002 | -0.000 |
| u: libc syscall wrappers | 0.097 | 0.059 | 0.078 | 0.082 | +0.038 | +0.019 | +0.023 |
| u: memcpy in encode | 0.000 | 2.441 | 2.060 | 0.000 | -2.441 | -0.382 | -2.441 |
| u: memcpy in grpc/protobuf | 2.484 | 0.000 | 0.000 | 0.000 | +2.484 | +0.000 | +0.000 |
| u: memcpy in hyper/h2/bytes | 0.000 | 0.195 | 0.185 | 0.160 | -0.195 | -0.010 | -0.035 |
| u: memcpy other | 0.001 | 0.010 | 0.010 | 0.005 | -0.008 | -0.000 | -0.005 |
| u: other | 0.019 | 0.003 | 0.004 | 0.006 | +0.016 | +0.001 | +0.004 |
| u: protobuf | 0.011 | 0.000 | 0.000 | 0.000 | +0.011 | +0.000 | +0.000 |
| u: tokio/mio runtime and park | 0.000 | 0.180 | 0.201 | 0.206 | -0.180 | +0.021 | +0.026 |
| **total (sampled)** | 9.126 | 9.564 | 9.003 | 6.497 | -0.438 | -0.561 | -3.066 |

Threads (ms per call, sampled): A p6: event_engine 6.094, campaign_rpc 3.032, lifeguard 0.000; Cf-retain p6: tokio-rt-worker 6.918, campaign_rpc 2.646; Cf-encp-retain p6: tokio-rt-worker 8.886, campaign_rpc 0.117; Cf-zc-retain p6: tokio-rt-worker 6.371, campaign_rpc 0.126

Top symbols, d/16MiB k=8 A p6 (ms per call): __memmove_evex_unaligned_erms 2.482; [k] rep_movs_alternative 1.972; [k] clear_page_erms 0.831; [k] memset_orig 0.121; [k] _copy_from_iter 0.114; [k] syscall_return_via_sysret 0.087; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.084; [k] entry_SYSRETQ_unsafe_stack 0.067; [k] get_page_from_freelist 0.056; [k] __alloc_tagging_slab_alloc_hook 0.056; [k] dequeue_entities 0.055; [k] _raw_spin_lock 0.052; absl::lts_20260107::Mutex::lock() 0.052; __tls_get_addr 0.046; grpc_chttp2_encode_data(unsigned int, grpc_slice_buffer*, unsigned int 0.046; [k] entry_SYSCALL_64_after_hwframe 0.044

Top symbols, d/16MiB k=8 Cf-retain p6 (ms per call): __memmove_evex_unaligned_erms 2.645; [k] rep_movs_alternative 1.973; [k] clear_page_erms 0.833; [k] syscall_return_via_sysret 0.253; [k] native_queued_spin_lock_slowpath 0.187; [k] _raw_spin_lock_irqsave 0.177; [k] entry_SYSRETQ_unsafe_stack 0.137; [k] entry_SYSCALL_64_after_hwframe 0.120; [k] __alloc_tagging_slab_alloc_hook 0.117; [k] _copy_from_iter 0.110; [k] entry_SYSCALL_64 0.099; [k] __pgalloc_tag_add 0.081; [k] _raw_spin_lock 0.078; [k] __smp_call_single_queue 0.065; [k] page_ext_get 0.064; h2::proto::streams::prioritize::Prioritize::buffer_pending 0.062

Top symbols, d/16MiB k=8 Cf-encp-retain p6 (ms per call): __memmove_evex_unaligned_erms 2.252; [k] rep_movs_alternative 1.834; [k] clear_page_erms 0.899; [k] syscall_return_via_sysret 0.215; [k] native_queued_spin_lock_slowpath 0.189; [k] entry_SYSRETQ_unsafe_stack 0.169; [k] _raw_spin_lock_irqsave 0.160; [k] _copy_from_iter 0.128; [k] entry_SYSCALL_64_after_hwframe 0.117; [k] __alloc_tagging_slab_alloc_hook 0.114; [k] __pgalloc_tag_add 0.107; [k] entry_SYSCALL_64 0.100; [k] _raw_spin_lock 0.071; [k] try_charge_memcg 0.066; [k] unix_stream_sendmsg 0.066; [k] page_ext_get 0.062

Top symbols, d/16MiB k=8 Cf-zc-retain p6 (ms per call): [k] rep_movs_alternative 1.533; [k] clear_page_erms 0.833; [k] syscall_return_via_sysret 0.231; [k] native_queued_spin_lock_slowpath 0.195; __memmove_evex_unaligned_erms 0.165; [k] _raw_spin_lock_irqsave 0.156; [k] entry_SYSRETQ_unsafe_stack 0.148; [k] __alloc_tagging_slab_alloc_hook 0.112; [k] entry_SYSCALL_64_after_hwframe 0.101; [k] _raw_spin_lock 0.101; [k] entry_SYSCALL_64 0.097; [k] _copy_from_iter 0.087; [k] __smp_call_single_queue 0.081; h2::proto::streams::prioritize::Prioritize::buffer_pending 0.069; [k] __pgalloc_tag_add 0.063; [k] try_charge_memcg 0.060
