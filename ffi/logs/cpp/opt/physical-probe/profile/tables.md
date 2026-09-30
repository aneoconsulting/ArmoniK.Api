# cpp physical probe step 4a: where the client CPU goes (A, D, Cf, Cf-q)

Main configuration: CLIENT 1-4,11-14, SERVER 5-8,15-18, server 8 workers, core 8 workers, grpc-core sized for 8 CPUs (ncpus_shim), pinned transport, retain mode; the machine at 3.3 GHz (turbo off). Each client process runs `campaign_rpc --profile N` on ONE cell (only its own channel or client open unless stated): 5 warm batches, then N batches in 10 chunks on the benchmark's own path; perf counts only that loop. Absolute per-call figures (ms = cycles / 3.3e9); median [p10-p90] over the chunks of the 3 rounds.

## 1a. Per call, each cell alone in its process, under perf stat

| workload | cell | CPU ms | wall ms | cycles u / k (M) | instr u / k (M) | cache-miss (k) | faults | ctx sw | n |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | 8.981 [8.910-9.335] | 8.473 [8.372-9.837] | 11.34 / 14.91 | 2.55 / 5.34 | 288 | 0 | 203 | 30 |
| d/16MiB k=1 | D-retain | 9.026 [8.942-9.378] | 8.573 [8.427-10.400] | 11.30 / 15.12 | 2.46 / 5.19 | 318 | 0 | 188 | 30 |
| d/16MiB k=1 | Cf-retain | 8.542 [8.269-9.020] | 8.556 [7.596-9.914] | 9.18 / 18.20 | 3.26 / 6.46 | 492 | 0 | 72 | 30 |
| d/16MiB k=1 | Cf-q-retain | 9.472 [8.896-10.037] | 8.457 [7.843-9.939] | 10.03 / 19.37 | 3.40 / 7.09 | 499 | 0 | 154 | 30 |
| d/16MiB k=8 | A | 10.214 [10.027-10.360] | 7.752 [7.529-7.972] | 13.04 / 17.71 | 2.91 / 5.43 | 501 | 3 | 170 | 30 |
| d/16MiB k=8 | D-retain | 10.246 [10.115-10.486] | 7.855 [7.573-8.055] | 13.18 / 17.84 | 2.88 / 5.45 | 509 | 0 | 175 | 30 |
| d/16MiB k=8 | Cf-retain | 9.680 [9.523-9.877] | 8.482 [8.266-8.711] | 11.30 / 19.88 | 3.30 / 6.65 | 421 | 11 | 58 | 30 |
| d/16MiB k=8 | Cf-q-retain | 9.724 [9.064-11.128] | 8.540 [8.289-8.900] | 8.25 / 22.97 | 3.37 / 9.20 | 400 | 402 | 86 | 30 |
| c/P5.4 k=1 | A | 2.399 [2.323-2.521] | 3.455 [2.945-4.118] | 2.96 / 3.99 | 0.89 / 1.42 | 93 | 0 | 45 | 30 |
| c/P5.4 k=1 | D-retain | 2.446 [2.349-2.662] | 3.635 [3.057-4.475] | 3.02 / 4.16 | 0.85 / 1.44 | 106 | 0 | 50 | 30 |
| c/P5.4 k=1 | Cf-retain | 2.122 [2.096-2.145] | 3.446 [3.086-3.954] | 2.44 / 4.17 | 0.85 / 1.61 | 95 | 0 | 18 | 30 |
| c/P5.4 k=1 | Cf-q-retain | 2.000 [1.965-2.040] | 3.128 [2.945-4.165] | 2.38 / 3.98 | 0.85 / 1.55 | 95 | 0 | 13 | 30 |

Per-thread CPU per call (ms, median of the rounds; schedstat, every thread of the class):

| workload | cell | caller | main | tokio-rt-worker | event_engine | grpc_global_tim | lifeguard | all |
|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | 3.153 (1) | 0.021 (1) | 0.000 (8) | 5.824 (8) | 0.000 (1) | 0.001 (1) | 8.963 |
| d/16MiB k=1 | D-retain | 3.288 (1) | 0.022 (1) | 0.000 (8) | 5.692 (8) | 0.000 (1) | 0.001 (1) | 9.001 |
| d/16MiB k=1 | Cf-retain | 2.139 (1) | 0.026 (1) | 6.376 (8) |  |  |  | 8.541 |
| d/16MiB k=1 | Cf-q-retain | 0.000 (1) | 2.247 (1) | 7.255 (8) |  |  |  | 9.502 |
| d/16MiB k=8 | A | 3.012 (8) | 0.009 (1) | 0.000 (8) | 7.188 (8) | 0.000 (1) | 0.000 (1) | 10.187 |
| d/16MiB k=8 | D-retain | 3.055 (8) | 0.009 (1) | 0.000 (8) | 7.248 (8) | 0.000 (1) | 0.000 (1) | 10.312 |
| d/16MiB k=8 | Cf-retain | 2.818 (8) | 0.008 (1) | 6.857 (8) |  |  |  | 9.683 |
| d/16MiB k=8 | Cf-q-retain | 0.000 (8) | 2.513 (1) | 7.337 (8) |  |  |  | 9.801 |
| c/P5.4 k=1 | A | 0.703 (1) | 0.022 (1) | 0.000 (8) | 1.676 (8) | 0.000 (1) | 0.000 (1) | 2.406 |
| c/P5.4 k=1 | D-retain | 0.723 (1) | 0.022 (1) | 0.000 (8) | 1.754 (8) | 0.000 (1) | 0.000 (1) | 2.508 |
| c/P5.4 k=1 | Cf-retain | 0.571 (1) | 0.026 (1) | 1.527 (8) |  |  |  | 2.125 |
| c/P5.4 k=1 | Cf-q-retain | 0.000 (1) | 0.516 (1) | 1.480 (8) |  |  |  | 1.996 |

## 1b. Per call, Cf and Cf-q with A, D, Cf, Cf-q all open in the process, under perf stat

| workload | cell | CPU ms | wall ms | cycles u / k (M) | instr u / k (M) | cache-miss (k) | faults | ctx sw | n |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | 8.402 [8.279-8.576] | 7.892 [7.440-10.108] | 9.08 / 17.92 | 3.24 / 6.40 | 493 | 0 | 64 | 30 |
| d/16MiB k=1 | Cf-q-retain | 9.521 [8.984-9.778] | 8.423 [7.633-9.428] | 9.98 / 19.35 | 3.38 / 7.01 | 510 | 0 | 140 | 30 |
| d/16MiB k=8 | Cf-retain | 9.708 [9.562-9.876] | 8.494 [8.157-8.700] | 11.35 / 19.89 | 3.30 / 6.68 | 425 | 14 | 60 | 30 |
| d/16MiB k=8 | Cf-q-retain | 10.317 [9.147-11.741] | 8.491 [8.234-8.877] | 8.80 / 24.11 | 3.40 / 9.24 | 400 | 427 | 105 | 30 |
| c/P5.4 k=1 | Cf-retain | 2.134 [2.101-2.179] | 3.267 [2.993-3.869] | 2.48 / 4.17 | 0.86 / 1.61 | 96 | 0 | 19 | 30 |
| c/P5.4 k=1 | Cf-q-retain | 2.026 [1.979-2.062] | 3.047 [2.879-4.029] | 2.42 / 4.03 | 0.85 / 1.56 | 93 | 0 | 12 | 30 |

Per-thread CPU per call (ms, median of the rounds; schedstat, every thread of the class):

| workload | cell | caller | main | tokio-rt-worker | event_engine | grpc_global_tim | lifeguard | all |
|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | Cf-retain | 2.106 (1) | 0.027 (1) | 6.298 (8) | 0.008 (8) | 0.000 (1) | 0.001 (1) | 8.441 |
| d/16MiB k=1 | Cf-q-retain | 0.000 (1) | 2.214 (1) | 7.232 (8) | 0.008 (8) | 0.000 (1) | 0.001 (1) | 9.454 |
| d/16MiB k=8 | Cf-retain | 2.819 (8) | 0.009 (1) | 6.880 (8) | 0.004 (8) | 0.000 (1) | 0.000 (1) | 9.700 |
| d/16MiB k=8 | Cf-q-retain | 0.000 (8) | 2.665 (1) | 7.696 (8) | 0.005 (8) | 0.000 (1) | 0.001 (1) | 10.367 |
| c/P5.4 k=1 | Cf-retain | 0.566 (1) | 0.026 (1) | 1.533 (8) | 0.004 (8) | 0.000 (1) | 0.001 (1) | 2.134 |
| c/P5.4 k=1 | Cf-q-retain | 0.000 (1) | 0.524 (1) | 1.507 (8) | 0.003 (8) | 0.000 (1) | 0.001 (1) | 2.032 |

## 1c. Per call, each cell alone in its process, NO perf attached (the control)

| workload | cell | CPU ms | wall ms | cycles u / k (M) | instr u / k (M) | cache-miss (k) | faults | ctx sw | n |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | 8.249 [8.121-8.408] | 8.223 [8.063-9.663] | - | - | - | - | - | 30 |
| d/16MiB k=1 | D-retain | 8.287 [8.229-8.495] | 8.326 [8.161-9.267] | - | - | - | - | - | 30 |
| d/16MiB k=1 | Cf-retain | 8.339 [8.194-8.679] | 8.367 [7.347-9.788] | - | - | - | - | - | 30 |
| d/16MiB k=1 | Cf-q-retain | 9.056 [8.481-9.587] | 8.339 [7.872-9.513] | - | - | - | - | - | 30 |
| d/16MiB k=8 | A | 9.523 [9.402-9.740] | 7.508 [7.321-7.672] | - | - | - | - | - | 30 |
| d/16MiB k=8 | D-retain | 9.634 [9.503-9.752] | 7.557 [7.328-7.687] | - | - | - | - | - | 30 |
| d/16MiB k=8 | Cf-retain | 9.463 [9.364-9.631] | 8.324 [8.144-8.567] | - | - | - | - | - | 30 |
| d/16MiB k=8 | Cf-q-retain | 9.313 [8.855-10.268] | 8.396 [7.990-8.602] | - | - | - | - | - | 30 |
| c/P5.4 k=1 | A | 2.241 [2.203-2.330] | 3.841 [3.503-4.213] | - | - | - | - | - | 30 |
| c/P5.4 k=1 | D-retain | 2.538 [2.265-2.744] | 5.871 [3.848-6.977] | - | - | - | - | - | 30 |
| c/P5.4 k=1 | Cf-retain | 2.044 [1.989-2.084] | 3.372 [3.067-4.404] | - | - | - | - | - | 30 |
| c/P5.4 k=1 | Cf-q-retain | 1.929 [1.893-1.961] | 3.185 [2.820-3.530] | - | - | - | - | - | 30 |

Per-thread CPU per call (ms, median of the rounds; schedstat, every thread of the class):

| workload | cell | caller | main | tokio-rt-worker | event_engine | grpc_global_tim | lifeguard | all |
|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | 3.246 (1) | 0.015 (1) | 0.000 (8) | 4.975 (8) | 0.000 (1) | 0.000 (1) | 8.261 |
| d/16MiB k=1 | D-retain | 3.233 (1) | 0.015 (1) | 0.000 (8) | 5.047 (8) | 0.000 (1) | 0.000 (1) | 8.286 |
| d/16MiB k=1 | Cf-retain | 2.063 (1) | 0.016 (1) | 6.239 (8) |  |  |  | 8.369 |
| d/16MiB k=1 | Cf-q-retain | 0.000 (1) | 2.117 (1) | 6.870 (8) |  |  |  | 8.987 |
| d/16MiB k=8 | A | 2.914 (8) | 0.008 (1) | 0.000 (8) | 6.651 (8) | 0.000 (1) | 0.000 (1) | 9.548 |
| d/16MiB k=8 | D-retain | 2.950 (8) | 0.007 (1) | 0.000 (8) | 6.651 (8) | 0.000 (1) | 0.000 (1) | 9.608 |
| d/16MiB k=8 | Cf-retain | 2.687 (8) | 0.007 (1) | 6.797 (8) |  |  |  | 9.511 |
| d/16MiB k=8 | Cf-q-retain | 0.000 (8) | 2.284 (1) | 7.102 (8) |  |  |  | 9.601 |
| c/P5.4 k=1 | A | 0.698 (1) | 0.014 (1) | 0.000 (8) | 1.544 (8) | 0.000 (1) | 0.000 (1) | 2.264 |
| c/P5.4 k=1 | D-retain | 0.710 (1) | 0.015 (1) | 0.000 (8) | 1.695 (8) | 0.000 (1) | 0.000 (1) | 2.413 |
| c/P5.4 k=1 | Cf-retain | 0.549 (1) | 0.017 (1) | 1.486 (8) |  |  |  | 2.054 |
| c/P5.4 k=1 | Cf-q-retain | 0.000 (1) | 0.516 (1) | 1.418 (8) |  |  |  | 1.933 |

## 2. Where the cycles go (perf record, cycles, LBR call stacks; ms per call; one process per cell)

Buckets from gen/perf_attrib.py (k: kernel, classed by the syscall wrapper or fault path in the call chain; u: user, classed by the leaf function, memcpy by its caller). Kernel symbols resolved with the booted kernel's System.map at the KASLR offset in each file's attrib JSON.

### d/16MiB k=1

| bucket | A | D-retain | Cf-retain | Cf-q-retain | Cf - D | Cf-q - Cf |
|---|---|---|---|---|---|---|
| k: epoll_wait | 0.236 | 0.247 | 0.186 | 0.402 | -0.061 | +0.215 |
| k: eventfd | 0.026 | 0.030 | 0.020 | 0.015 | -0.009 | -0.005 |
| k: futex | 0.551 | 0.567 | 0.115 | 0.182 | -0.452 | +0.067 |
| k: other | 0.000 | 0.029 | 0.000 | 0.026 | -0.029 | +0.026 |
| k: other syscalls | 0.183 | 0.171 | 0.041 | 0.067 | -0.129 | +0.026 |
| k: page faults (fault, zeroing new pages) | 0.829 | 0.798 | 0.854 | 0.835 | +0.056 | -0.019 |
| k: scheduling, interrupts | 0.033 | 0.030 | 0.031 | 0.026 | +0.001 | -0.005 |
| k: socket read (recvmsg/read) | 0.115 | 0.135 | 0.000 | 0.000 | -0.135 | +0.000 |
| k: socket write (sendmsg/writev) | 2.268 | 2.379 | 4.142 | 4.240 | +1.763 | +0.098 |
| u: allocation (malloc/free) | 0.084 | 0.066 | 0.018 | 0.024 | -0.047 | +0.005 |
| u: core rpc glue (ak_core rpc, FFI entry) | 0.000 | 0.008 | 0.047 | 0.074 | +0.039 | +0.027 |
| u: encode (core codec) | 0.000 | 0.007 | 0.002 | 0.005 | -0.005 | +0.003 |
| u: grpc-core (libgrpc, gpr, absl) | 1.290 | 1.294 | 0.000 | 0.000 | -1.294 | +0.000 |
| u: harness | 0.041 | 0.031 | 0.010 | 0.003 | -0.021 | -0.006 |
| u: hyper/h2/tonic/http/bytes | 0.000 | 0.003 | 0.369 | 0.355 | +0.366 | -0.014 |
| u: libc sync (pthread, futex wrappers) | 0.013 | 0.013 | 0.005 | 0.000 | -0.008 | -0.005 |
| u: libc syscall wrappers | 0.089 | 0.097 | 0.069 | 0.107 | -0.028 | +0.038 |
| u: memcpy in encode | 0.000 | 2.013 | 1.912 | 2.029 | -0.100 | +0.117 |
| u: memcpy in grpc/protobuf | 2.089 | 0.005 | 0.000 | 0.000 | -0.005 | +0.000 |
| u: memcpy in hyper/h2/bytes | 0.000 | 0.000 | 0.130 | 0.151 | +0.130 | +0.021 |
| u: memcpy other | 0.000 | 0.000 | 0.005 | 0.018 | +0.005 | +0.013 |
| u: other | 0.025 | 0.022 | 0.007 | 0.010 | -0.014 | +0.003 |
| u: protobuf | 0.006 | 0.000 | 0.000 | 0.000 | +0.000 | +0.000 |
| u: tokio/mio runtime and park | 0.000 | 0.000 | 0.211 | 0.293 | +0.211 | +0.082 |
| **total (sampled)** | 7.879 | 7.943 | 8.175 | 8.863 | +0.233 | +0.687 |

KASLR offset / anchor share: A 0x6c00000 0.21, D-retain 0x6c00000 0.21, Cf-retain 0x6c00000 0.21, Cf-q-retain 0x6c00000 0.21

Top symbols, d/16MiB k=1 A (ms per call): __memmove_evex_unaligned_erms 2.089; [k] rep_movs_alternative 1.091; [k] clear_page_erms 0.821; [k] _copy_from_iter 0.121; [k] syscall_return_via_sysret 0.109; [k] memset_orig 0.100; [k] entry_SYSRETQ_unsafe_stack 0.074; absl::lts_20260107::Mutex::lock() 0.063; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.063; [k] __alloc_tagging_slab_alloc_hook 0.058; [k] entry_SYSCALL_64_after_hwframe 0.055; [k] _raw_spin_lock 0.053; [k] update_curr 0.044; [k] entry_SYSCALL_64 0.044

Top symbols, d/16MiB k=1 D-retain (ms per call): __memmove_evex_unaligned_erms 2.015; [k] rep_movs_alternative 1.204; [k] clear_page_erms 0.785; [k] _copy_from_iter 0.111; [k] syscall_return_via_sysret 0.106; [k] memset_orig 0.101; absl::lts_20260107::Mutex::lock() 0.079; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.067; [k] entry_SYSRETQ_unsafe_stack 0.065; [k] get_page_from_freelist 0.056; [k] _raw_spin_lock 0.053; [k] __alloc_tagging_slab_alloc_hook 0.052; [k] entry_SYSCALL_64_after_hwframe 0.049; [k] dequeue_entities 0.048

Top symbols, d/16MiB k=1 Cf-retain (ms per call): __memmove_evex_unaligned_erms 2.048; [k] rep_movs_alternative 1.681; [k] clear_page_erms 0.842; [k] syscall_return_via_sysret 0.207; [k] entry_SYSRETQ_unsafe_stack 0.153; [k] entry_SYSCALL_64_after_hwframe 0.128; [k] native_queued_spin_lock_slowpath 0.115; [k] _raw_spin_lock_irqsave 0.108; [k] entry_SYSCALL_64 0.099; [k] _copy_from_iter 0.092; [k] __alloc_tagging_slab_alloc_hook 0.091; [k] _raw_spin_lock 0.081; h2::proto::streams::prioritize::Prioritize::buffer_pending 0.070; [k] sock_def_readable 0.066

Top symbols, d/16MiB k=1 Cf-q-retain (ms per call): __memmove_evex_unaligned_erms 2.196; [k] rep_movs_alternative 1.624; [k] clear_page_erms 0.823; [k] syscall_return_via_sysret 0.255; [k] entry_SYSRETQ_unsafe_stack 0.164; [k] entry_SYSCALL_64_after_hwframe 0.159; [k] __alloc_tagging_slab_alloc_hook 0.123; [k] _copy_from_iter 0.104; [k] entry_SYSCALL_64 0.098; [k] _raw_spin_lock_irqsave 0.079; [k] native_queued_spin_lock_slowpath 0.078; [k] _raw_spin_lock 0.074; [k] __pgalloc_tag_add 0.071; [k] try_charge_memcg 0.068

### d/16MiB k=8

| bucket | A | D-retain | Cf-retain | Cf-q-retain | Cf - D | Cf-q - Cf |
|---|---|---|---|---|---|---|
| k: epoll_wait | 0.108 | 0.133 | 0.121 | 0.255 | -0.013 | +0.134 |
| k: eventfd | 0.005 | 0.007 | 0.004 | 0.010 | -0.003 | +0.006 |
| k: futex | 0.527 | 0.554 | 0.116 | 0.160 | -0.438 | +0.044 |
| k: mmap/munmap/madvise/brk | 0.000 | 0.000 | 0.000 | 0.074 | +0.000 | +0.074 |
| k: other | 0.127 | 0.137 | 0.048 | 0.000 | -0.088 | -0.048 |
| k: other syscalls | 0.130 | 0.100 | 0.040 | 0.193 | -0.060 | +0.154 |
| k: page faults (fault, zeroing new pages) | 0.884 | 0.817 | 0.888 | 1.262 | +0.070 | +0.374 |
| k: scheduling, interrupts | 0.075 | 0.054 | 0.044 | 0.019 | -0.010 | -0.024 |
| k: socket read (recvmsg/read) | 0.123 | 0.098 | 0.000 | 0.000 | -0.098 | +0.000 |
| k: socket write (sendmsg/writev) | 3.154 | 3.280 | 4.733 | 4.768 | +1.453 | +0.034 |
| u: allocation (malloc/free) | 0.065 | 0.051 | 0.022 | 0.024 | -0.029 | +0.001 |
| u: core rpc glue (ak_core rpc, FFI entry) | 0.000 | 0.009 | 0.056 | 0.070 | +0.047 | +0.014 |
| u: encode (core codec) | 0.000 | 0.007 | 0.008 | 0.000 | +0.001 | -0.008 |
| u: grpc-core (libgrpc, gpr, absl) | 1.238 | 1.216 | 0.000 | 0.000 | -1.216 | +0.000 |
| u: harness | 0.038 | 0.027 | 0.003 | 0.004 | -0.024 | +0.001 |
| u: hyper/h2/tonic/http/bytes | 0.000 | 0.001 | 0.342 | 0.349 | +0.341 | +0.006 |
| u: libc sync (pthread, futex wrappers) | 0.009 | 0.009 | 0.005 | 0.000 | -0.004 | -0.005 |
| u: libc syscall wrappers | 0.085 | 0.078 | 0.089 | 0.057 | +0.011 | -0.032 |
| u: memcpy in encode | 0.000 | 2.353 | 2.413 | 1.424 | +0.060 | -0.989 |
| u: memcpy in grpc/protobuf | 2.319 | 0.009 | 0.000 | 0.000 | -0.009 | +0.000 |
| u: memcpy in hyper/h2/bytes | 0.000 | 0.000 | 0.161 | 0.161 | +0.161 | -0.000 |
| u: memcpy other | 0.000 | 0.002 | 0.009 | 0.008 | +0.007 | -0.002 |
| u: other | 0.023 | 0.029 | 0.006 | 0.008 | -0.023 | +0.002 |
| u: protobuf | 0.013 | 0.000 | 0.000 | 0.000 | +0.000 | +0.000 |
| u: tokio/mio runtime and park | 0.000 | 0.000 | 0.197 | 0.223 | +0.197 | +0.026 |
| **total (sampled)** | 8.922 | 8.971 | 9.305 | 9.069 | +0.334 | -0.236 |

KASLR offset / anchor share: A 0x6c00000 0.23, D-retain 0x6c00000 0.23, Cf-retain 0x6c00000 0.21, Cf-q-retain 0x6c00000 0.20

Top symbols, d/16MiB k=8 A (ms per call): __memmove_evex_unaligned_erms 2.313; [k] rep_movs_alternative 1.911; [k] clear_page_erms 0.874; [k] _copy_from_iter 0.128; [k] memset_orig 0.116; [k] syscall_return_via_sysret 0.082; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.081; [k] _raw_spin_lock 0.068; [k] _raw_spin_lock_irqsave 0.065; [k] entry_SYSRETQ_unsafe_stack 0.064; [k] __alloc_tagging_slab_alloc_hook 0.054; [k] get_page_from_freelist 0.048; absl::lts_20260107::Mutex::lock() 0.048; [k] entry_SYSCALL_64 0.046

Top symbols, d/16MiB k=8 D-retain (ms per call): __memmove_evex_unaligned_erms 2.357; [k] rep_movs_alternative 1.997; [k] clear_page_erms 0.813; [k] memset_orig 0.129; [k] _copy_from_iter 0.097; [k] syscall_return_via_sysret 0.093; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.087; [k] dequeue_entities 0.065; [k] entry_SYSRETQ_unsafe_stack 0.064; [k] __alloc_tagging_slab_alloc_hook 0.064; [k] _raw_spin_lock 0.055; [k] __rmqueue_pcplist 0.052; [k] _raw_spin_lock_irqsave 0.051; [k] get_page_from_freelist 0.047

Top symbols, d/16MiB k=8 Cf-retain (ms per call): __memmove_evex_unaligned_erms 2.583; [k] rep_movs_alternative 1.806; [k] clear_page_erms 0.857; [k] syscall_return_via_sysret 0.253; [k] _raw_spin_lock_irqsave 0.196; [k] entry_SYSRETQ_unsafe_stack 0.164; [k] native_queued_spin_lock_slowpath 0.155; [k] entry_SYSCALL_64_after_hwframe 0.118; [k] __alloc_tagging_slab_alloc_hook 0.114; [k] _copy_from_iter 0.113; [k] _raw_spin_lock 0.094; [k] entry_SYSCALL_64 0.084; [k] __pgalloc_tag_add 0.077; [k] page_ext_get 0.072

Top symbols, d/16MiB k=8 Cf-q-retain (ms per call): [k] rep_movs_alternative 1.889; __memmove_evex_unaligned_erms 1.592; [k] clear_page_erms 0.832; [k] syscall_return_via_sysret 0.247; [k] _raw_spin_lock_irqsave 0.190; [k] entry_SYSRETQ_unsafe_stack 0.154; [k] native_queued_spin_lock_slowpath 0.150; [k] asm_exc_page_fault 0.143; [k] entry_SYSCALL_64_after_hwframe 0.135; [k] __alloc_tagging_slab_alloc_hook 0.114; [k] swapgs_restore_regs_and_return_to_usermode 0.101; [k] _copy_from_iter 0.097; [k] entry_SYSCALL_64 0.095; [k] _raw_spin_lock 0.093

### c/P5.4 k=1

| bucket | A | D-retain | Cf-retain | Cf-q-retain | Cf - D | Cf-q - Cf |
|---|---|---|---|---|---|---|
| k: epoll_wait | 0.077 | 0.071 | 0.048 | 0.033 | -0.023 | -0.015 |
| k: eventfd | 0.004 | 0.005 | 0.005 | 0.004 | +0.000 | -0.002 |
| k: futex | 0.173 | 0.208 | 0.044 | 0.017 | -0.164 | -0.027 |
| k: other | 0.000 | 0.004 | 0.002 | 0.008 | -0.002 | +0.006 |
| k: other syscalls | 0.060 | 0.065 | 0.014 | 0.014 | -0.051 | -0.001 |
| k: page faults (fault, zeroing new pages) | 0.187 | 0.197 | 0.182 | 0.186 | -0.015 | +0.004 |
| k: scheduling, interrupts | 0.011 | 0.008 | 0.007 | 0.011 | -0.001 | +0.005 |
| k: socket read (recvmsg/read) | 0.032 | 0.028 | 0.000 | 0.000 | -0.028 | +0.000 |
| k: socket write (sendmsg/writev) | 0.694 | 0.718 | 0.981 | 0.974 | +0.263 | -0.006 |
| u: allocation (malloc/free) | 0.020 | 0.014 | 0.008 | 0.006 | -0.006 | -0.002 |
| u: core rpc glue (ak_core rpc, FFI entry) | 0.000 | 0.002 | 0.016 | 0.021 | +0.014 | +0.005 |
| u: encode (core codec) | 0.000 | 0.001 | 0.001 | 0.000 | +0.000 | -0.001 |
| u: grpc-core (libgrpc, gpr, absl) | 0.415 | 0.431 | 0.000 | 0.000 | -0.431 | +0.000 |
| u: harness | 0.015 | 0.010 | 0.002 | 0.002 | -0.009 | +0.001 |
| u: hyper/h2/tonic/http/bytes | 0.000 | 0.000 | 0.127 | 0.119 | +0.127 | -0.008 |
| u: libc sync (pthread, futex wrappers) | 0.004 | 0.005 | 0.002 | 0.000 | -0.003 | -0.002 |
| u: libc syscall wrappers | 0.026 | 0.027 | 0.018 | 0.018 | -0.009 | -0.000 |
| u: memcpy in encode | 0.000 | 0.476 | 0.484 | 0.468 | +0.008 | -0.016 |
| u: memcpy in grpc/protobuf | 0.450 | 0.001 | 0.000 | 0.000 | -0.001 | +0.000 |
| u: memcpy in hyper/h2/bytes | 0.000 | 0.000 | 0.037 | 0.032 | +0.037 | -0.005 |
| u: memcpy other | 0.000 | 0.000 | 0.001 | 0.004 | +0.001 | +0.003 |
| u: other | 0.007 | 0.006 | 0.004 | 0.002 | -0.002 | -0.002 |
| u: protobuf | 0.002 | 0.000 | 0.000 | 0.000 | +0.000 | +0.000 |
| u: tokio/mio runtime and park | 0.000 | 0.000 | 0.074 | 0.057 | +0.074 | -0.016 |
| **total (sampled)** | 2.178 | 2.277 | 2.056 | 1.978 | -0.220 | -0.078 |

KASLR offset / anchor share: A 0x6c00000 0.21, D-retain 0x6c00000 0.22, Cf-retain 0x6c00000 0.21, Cf-q-retain 0x6c00000 0.20

Top symbols, c/P5.4 k=1 A (ms per call): __memmove_evex_unaligned_erms 0.450; [k] rep_movs_alternative 0.368; [k] clear_page_erms 0.183; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.033; [k] memset_orig 0.032; [k] syscall_return_via_sysret 0.032; [k] _copy_from_iter 0.032; [k] __alloc_tagging_slab_alloc_hook 0.019; [k] dequeue_entities 0.018; absl::lts_20260107::Mutex::lock() 0.018; [k] entry_SYSCALL_64_after_hwframe 0.017; [k] entry_SYSRETQ_unsafe_stack 0.015; [k] get_page_from_freelist 0.013; [k] _raw_spin_lock_irqsave 0.012

Top symbols, c/P5.4 k=1 D-retain (ms per call): __memmove_evex_unaligned_erms 0.477; [k] rep_movs_alternative 0.392; [k] clear_page_erms 0.193; [k] syscall_return_via_sysret 0.035; [k] _copy_from_iter 0.032; [k] memset_orig 0.028; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.027; [k] entry_SYSRETQ_unsafe_stack 0.024; [k] dequeue_entities 0.021; [k] __alloc_tagging_slab_alloc_hook 0.017; [k] entry_SYSCALL_64 0.017; [k] entry_SYSCALL_64_after_hwframe 0.015; absl::lts_20260107::Mutex::lock() 0.014; [k] native_queued_spin_lock_slowpath 0.013

Top symbols, c/P5.4 k=1 Cf-retain (ms per call): __memmove_evex_unaligned_erms 0.522; [k] rep_movs_alternative 0.352; [k] clear_page_erms 0.181; [k] syscall_return_via_sysret 0.052; [k] entry_SYSRETQ_unsafe_stack 0.046; [k] native_queued_spin_lock_slowpath 0.038; [k] entry_SYSCALL_64_after_hwframe 0.035; [k] entry_SYSCALL_64 0.029; [k] _raw_spin_lock_irqsave 0.023; [k] _copy_from_iter 0.023; [k] __alloc_tagging_slab_alloc_hook 0.019; [k] sock_def_readable 0.017; [k] try_charge_memcg 0.015; h2::proto::streams::prioritize::Prioritize::buffer_pending 0.015

Top symbols, c/P5.4 k=1 Cf-q-retain (ms per call): __memmove_evex_unaligned_erms 0.504; [k] rep_movs_alternative 0.280; [k] clear_page_erms 0.185; [k] syscall_return_via_sysret 0.054; [k] entry_SYSRETQ_unsafe_stack 0.042; [k] native_queued_spin_lock_slowpath 0.036; [k] _copy_from_iter 0.030; [k] entry_SYSCALL_64_after_hwframe 0.029; [k] __alloc_tagging_slab_alloc_hook 0.029; [k] entry_SYSCALL_64 0.027; [k] _raw_spin_lock_irqsave 0.024; [k] page_ext_get 0.024; [k] _raw_spin_lock 0.021; [k] __pgalloc_tag_add 0.019

## 3. Syscalls per call (strace -f -yy, between the loop's markers; fewer calls than sections 1-2)

| workload | cell | calls | socket writes | bytes/write | socket reads | epoll_wait | futex | write (eventfd) | read | mmap/munmap/madvise | EAGAIN |
|---|---|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | 12 | 49.3 | 340271 | 48.2 | 67.7 | 196.7 | 8.0 | 8.0 | 0.0 | 42.5 |
| d/16MiB k=1 | D-retain | 12 | 45.6 | 368264 | 47.1 | 66.1 | 186.7 | 7.3 | 7.3 | 0.0 | 38.3 |
| d/16MiB k=1 | Cf-retain | 12 | 1026.2 | 16359 | 12.6 | 1023.0 | 47.5 | 11.9 | 0.0 | 0.0 | 4.2 |
| d/16MiB k=1 | Cf-q-retain | 12 | 1026.8 | 16348 | 11.2 | 991.8 | 68.9 | 18.5 | 0.0 | 0.0 | 4.3 |
| d/16MiB k=8 | A | 24 | 35.1 | 477913 | 35.2 | 54.1 | 161.4 | 2.9 | 2.9 | 0.0 | 26.9 |
| d/16MiB k=8 | D-retain | 24 | 33.1 | 506768 | 34.5 | 54.2 | 160.0 | 2.6 | 2.6 | 0.0 | 25.8 |
| d/16MiB k=8 | Cf-retain | 24 | 1027.6 | 16336 | 10.7 | 1006.8 | 53.8 | 1.5 | 0.0 | 0.0 | 3.0 |
| d/16MiB k=8 | Cf-q-retain | 24 | 1028.5 | 16322 | 9.9 | 988.3 | 78.8 | 3.5 | 0.0 | 0.0 | 4.1 |
| c/P5.4 k=1 | A | 40 | 14.4 | 290433 | 12.8 | 16.5 | 53.5 | 1.0 | 1.0 | 0.0 | 11.4 |
| c/P5.4 k=1 | D-retain | 40 | 13.4 | 313190 | 13.2 | 16.1 | 54.4 | 1.0 | 1.0 | 0.0 | 12.3 |
| c/P5.4 k=1 | Cf-retain | 40 | 258.0 | 16266 | 3.6 | 263.3 | 22.3 | 3.8 | 0.0 | 0.0 | 0.8 |
| c/P5.4 k=1 | Cf-q-retain | 40 | 258.0 | 16266 | 4.1 | 262.8 | 12.7 | 2.8 | 0.0 | 0.0 | 1.4 |

Socket write sizes (count per call by power-of-two ceiling, bytes):
- d/16MiB k=1 A: <=1048576: 10.0, <=131072: 2.8, <=16: 1.0, <=2097152: 1.1, <=262144: 11.4, <=32: 1.0, <=32768: 1.4, <=524288: 15.9, <=65536: 0.8
- d/16MiB k=1 D-retain: <=1048576: 9.3, <=131072: 3.6, <=16: 1.0, <=2097152: 0.8, <=262144: 9.7, <=32: 1.0, <=32768: 1.3, <=524288: 17.3, <=65536: 0.4
- d/16MiB k=1 Cf-retain: <=32: 0.1, <=32768: 1024.0, <=64: 2.1
- d/16MiB k=1 Cf-q-retain: <=128: 0.2, <=16384: 0.2, <=32: 0.2, <=32768: 1023.8, <=64: 2.1
- d/16MiB k=8 A: <=1024: 0.0, <=1048576: 12.0, <=128: 0.1, <=131072: 0.9, <=16: 0.0, <=16384: 0.0, <=2097152: 2.0, <=262144: 9.2, <=32: 0.2, <=32768: 0.1, <=524288: 8.3, <=64: 0.1, <=65536: 0.5, <=8192: 0.1
- d/16MiB k=8 D-retain: <=1048576: 10.9, <=128: 0.1, <=131072: 0.8, <=16: 0.1, <=16384: 0.1, <=2097152: 2.3, <=262144: 7.8, <=32: 0.2, <=32768: 0.3, <=4096: 0.0, <=4194304: 0.1, <=524288: 9.0, <=65536: 0.3
- d/16MiB k=8 Cf-retain: <=1024: 0.9, <=128: 0.1, <=16384: 5.5, <=2048: 0.5, <=32: 0.0, <=32768: 1018.5, <=4096: 0.3, <=512: 1.2, <=64: 0.1, <=8192: 0.2
- d/16MiB k=8 Cf-q-retain: <=1024: 0.9, <=16: 0.0, <=16384: 6.0, <=2048: 0.7, <=32: 0.1, <=32768: 1017.9, <=4096: 0.5, <=512: 0.7, <=64: 0.1, <=8192: 1.5
- c/P5.4 k=1 A: <=1048576: 1.9, <=131072: 0.8, <=262144: 6.1, <=32768: 0.0, <=524288: 3.6, <=65536: 0.4
- c/P5.4 k=1 D-retain: <=1048576: 2.2, <=131072: 1.1, <=16384: 0.0, <=2097152: 0.7, <=262144: 5.6, <=32768: 0.0, <=524288: 1.3, <=65536: 0.4, <=8192: 0.1
- c/P5.4 k=1 Cf-retain: <=128: 1.0, <=32768: 256.0, <=64: 1.0
- c/P5.4 k=1 Cf-q-retain: <=128: 1.0, <=32768: 256.0, <=64: 1.0
