# perf: core builds compared from C++ (one-cell `campaign_rpc --profile` processes, no perf)

Arms, cores, knobs and the environment of every process: runner.log. CPU (process) and wall per call, median [p10-p90] over the 10 chunks of every process of the phase; flt = minor faults, csw = voluntary + involuntary context switches per call (median of the processes).

## perf: the split (perf attached; absolutes above are the no-perf phases')

perf stat per call (one process per cell, perf enabled around the loop): cycles and instructions user / kernel (M), cache-references and cache-misses (k), LLC-load-misses (k), faults, context switches.

| workload | cell | arm | cycles u / k | instr u / k | cache-ref | cache-miss | LLC-load-miss | faults | csw |
|---|---|---|---|---|---|---|---|---|---|
| d/16MiB k=1 | A | cur | 12.06 / 14.48 | 2.91 / 5.30 | 947 | 307 | 141 | 0.1 | 178.0 |
| d/16MiB k=1 | A | cur (SERVER) | 12.43 / 14.97 | 6.51 / 7.46 | - | 43 | - | - | 36.7 |
| d/16MiB k=1 | Cf-retain | cur | 9.12 / 17.80 | 3.23 / 6.25 | 738 | 515 | 246 | 4.5 | 68.8 |
| d/16MiB k=1 | Cf-retain | cur (SERVER) | 16.19 / 23.31 | 6.90 / 12.20 | - | 89 | - | - | 314.8 |
| d/16MiB k=1 | Cf-q-retain | cur | 10.10 / 19.38 | 3.42 / 6.89 | 757 | 532 | 270 | 0.0 | 139.0 |
| d/16MiB k=1 | Cf-q-retain | cur (SERVER) | 15.66 / 24.11 | 6.85 / 13.03 | - | 95 | - | - | 231.8 |
| d/16MiB k=1 | Cf-q-retain | q1slot | 9.52 / 18.86 | 3.36 / 6.93 | 771 | 525 | 266 | 0.1 | 151.1 |
| d/16MiB k=1 | Cf-q-retain | q1slot (SERVER) | 15.18 / 24.11 | 6.78 / 13.79 | - | 101 | - | - | 267.6 |
| d/16MiB k=8 | A | cur | 13.67 / 17.33 | 2.72 / 5.23 | 867 | 499 | 225 | 6.5 | 167.0 |
| d/16MiB k=8 | A | cur (SERVER) | 18.00 / 19.28 | 6.84 / 9.03 | - | 165 | - | - | 191.6 |
| d/16MiB k=8 | Cf-retain | cur | 11.58 / 19.70 | 3.32 / 6.48 | 696 | 424 | 202 | 10.7 | 63.8 |
| d/16MiB k=8 | Cf-retain | cur (SERVER) | 26.38 / 32.98 | 8.93 / 16.24 | - | 216 | - | - | 916.5 |
| d/16MiB k=8 | Cf-q-retain | cur | 8.02 / 18.79 | 3.34 / 6.50 | 744 | 397 | 195 | 0.2 | 82.6 |
| d/16MiB k=8 | Cf-q-retain | cur (SERVER) | 26.01 / 31.49 | 8.89 / 15.64 | - | 220 | - | - | 922.3 |
| d/16MiB k=8 | Cf-q-retain | q1slot | 8.14 / 19.48 | 3.36 / 6.61 | 747 | 413 | 202 | 0.1 | 87.3 |
| d/16MiB k=8 | Cf-q-retain | q1slot (SERVER) | 25.78 / 32.23 | 8.81 / 15.90 | - | 220 | - | - | 906.7 |

### perf record buckets, d/16MiB k=1, the client (ms per call = cycles / 3.3e9; one process per cell)

| bucket | A cur | Cf-retain cur | Cf-q-retain cur | Cf-q-retain q1slot | A - Cf | Cf-q-retain - Cf | Cf-q-retain - Cf |
|---|---|---|---|---|---|---|---|
| k: epoll_wait | 0.282 | 0.208 | 0.308 | 0.332 | +0.074 | +0.100 | +0.124 |
| k: eventfd | 0.028 | 0.013 | 0.022 | 0.023 | +0.014 | +0.009 | +0.009 |
| k: futex | 0.570 | 0.111 | 0.207 | 0.163 | +0.459 | +0.095 | +0.052 |
| k: other syscalls | 0.218 | 0.026 | 0.025 | 0.014 | +0.191 | -0.001 | -0.013 |
| k: page faults (the fault path) | 0.000 | 0.009 | 0.000 | 0.000 | -0.009 | -0.009 | -0.009 |
| k: scheduling, interrupts | 0.026 | 0.030 | 0.011 | 0.023 | -0.004 | -0.020 | -0.007 |
| k: socket read (recvmsg/read) | 0.122 | 0.051 | 0.041 | 0.050 | +0.071 | -0.011 | -0.001 |
| k: socket write (sendmsg/writev) | 3.140 | 5.106 | 5.303 | 5.255 | -1.967 | +0.197 | +0.149 |
| u: allocation (malloc/free) | 0.055 | 0.016 | 0.023 | 0.032 | +0.039 | +0.007 | +0.015 |
| u: core rpc glue (ak_core rpc, FFI entry) | 0.000 | 0.052 | 0.056 | 0.053 | -0.052 | +0.004 | +0.001 |
| u: encode (core codec) | 0.000 | 0.004 | 0.002 | 0.000 | -0.004 | -0.002 | -0.004 |
| u: grpc-core (libgrpc, gpr, absl) | 1.317 | 0.000 | 0.000 | 0.000 | +1.317 | +0.000 | +0.000 |
| u: harness | 0.030 | 0.007 | 0.003 | 0.006 | +0.023 | -0.004 | -0.001 |
| u: hyper/h2/tonic/http/bytes | 0.000 | 0.335 | 0.329 | 0.403 | -0.335 | -0.006 | +0.068 |
| u: libc sync (pthread, futex wrappers) | 0.018 | 0.005 | 0.000 | 0.000 | +0.013 | -0.005 | -0.005 |
| u: libc syscall wrappers | 0.106 | 0.072 | 0.104 | 0.060 | +0.035 | +0.032 | -0.012 |
| u: memcpy in encode | 0.000 | 1.904 | 1.979 | 1.976 | -1.904 | +0.076 | +0.072 |
| u: memcpy in grpc/protobuf | 2.060 | 0.000 | 0.000 | 0.000 | +2.060 | +0.000 | +0.000 |
| u: memcpy in hyper/h2/bytes | 0.000 | 0.141 | 0.121 | 0.153 | -0.141 | -0.019 | +0.013 |
| u: memcpy other | 0.000 | 0.007 | 0.010 | 0.004 | -0.007 | +0.002 | -0.003 |
| u: other | 0.021 | 0.007 | 0.003 | 0.012 | +0.014 | -0.005 | +0.004 |
| u: protobuf | 0.004 | 0.000 | 0.000 | 0.000 | +0.004 | +0.000 | +0.000 |
| u: tokio/mio runtime and park | 0.000 | 0.157 | 0.248 | 0.265 | -0.157 | +0.091 | +0.108 |
| **total (sampled)** | 7.998 | 8.263 | 8.794 | 8.823 | -0.265 | +0.532 | +0.560 |

Threads (ms per call, sampled): A cur: event_engine 4.787, campaign_rpc 3.211, lifeguard 0.000; Cf-retain cur: tokio-rt-worker 6.188, campaign_rpc 2.075; Cf-q-retain cur: tokio-rt-worker 6.658, campaign_rpc 2.136; Cf-q-retain q1slot: tokio-rt-worker 6.703, campaign_rpc 2.120

Top symbols, d/16MiB k=1 A cur (ms per call): __memmove_evex_unaligned_erms 2.057; [k] rep_movs_alternative 1.084; [k] clear_page_erms 0.868; [k] syscall_return_via_sysret 0.123; [k] _copy_from_iter 0.119; [k] memset_orig 0.095; [k] entry_SYSRETQ_unsafe_stack 0.083; absl::lts_20260107::Mutex::lock() 0.080; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.077; [k] entry_SYSCALL_64_after_hwframe 0.071; [k] entry_SYSCALL_64 0.067; [k] dequeue_entities 0.066; [k] _raw_spin_lock 0.056; grpc_chttp2_encode_data(unsigned int, grpc_slice_buffer*, unsigned int 0.049; [k] __alloc_tagging_slab_alloc_hook 0.045; [k] _raw_spin_lock_irqsave 0.040

Top symbols, d/16MiB k=1 Cf-retain cur (ms per call): __memmove_evex_unaligned_erms 2.052; [k] rep_movs_alternative 1.792; [k] clear_page_erms 0.858; [k] native_queued_spin_lock_slowpath 0.211; [k] syscall_return_via_sysret 0.180; [k] entry_SYSRETQ_unsafe_stack 0.159; [k] __alloc_tagging_slab_alloc_hook 0.106; [k] entry_SYSCALL_64 0.102; [k] entry_SYSCALL_64_after_hwframe 0.099; [k] _raw_spin_lock_irqsave 0.089; [k] _copy_from_iter 0.085; [k] __pgalloc_tag_add 0.077; [k] try_charge_memcg 0.067; [k] unix_stream_sendmsg 0.064; [k] _raw_spin_lock 0.064; [k] page_ext_get 0.059

Top symbols, d/16MiB k=1 Cf-q-retain cur (ms per call): __memmove_evex_unaligned_erms 2.109; [k] rep_movs_alternative 1.866; [k] clear_page_erms 0.908; [k] syscall_return_via_sysret 0.211; [k] native_queued_spin_lock_slowpath 0.169; [k] entry_SYSCALL_64_after_hwframe 0.150; [k] entry_SYSRETQ_unsafe_stack 0.136; [k] __alloc_tagging_slab_alloc_hook 0.126; [k] _copy_from_iter 0.119; [k] entry_SYSCALL_64 0.097; [k] _raw_spin_lock 0.091; [k] _raw_spin_lock_irqsave 0.087; [k] __pgalloc_tag_add 0.068; [k] skb_set_owner_w 0.058; [k] __rmqueue_pcplist 0.056; [k] ep_poll_callback 0.056

Top symbols, d/16MiB k=1 Cf-q-retain q1slot (ms per call): __memmove_evex_unaligned_erms 2.133; [k] rep_movs_alternative 1.813; [k] clear_page_erms 0.923; [k] native_queued_spin_lock_slowpath 0.243; [k] syscall_return_via_sysret 0.232; [k] entry_SYSCALL_64_after_hwframe 0.165; [k] entry_SYSRETQ_unsafe_stack 0.161; [k] _copy_from_iter 0.118; [k] __alloc_tagging_slab_alloc_hook 0.115; [k] _raw_spin_lock 0.094; [k] entry_SYSCALL_64 0.087; tokio_util::util::poll_buf::poll_write_buf 0.066; [k] unix_stream_sendmsg 0.062; [k] try_charge_memcg 0.062; [k] __pgalloc_tag_add 0.060; [k] _raw_spin_lock_irqsave 0.059

### perf record buckets, d/16MiB k=1, the server process (ms per call = cycles / 3.3e9; one process per cell)

| bucket | A cur SERVER | Cf-retain cur SERVER | Cf-q-retain cur SERVER | Cf-q-retain q1slot SERVER | A - Cf | Cf-q-retain - Cf | Cf-q-retain - Cf |
|---|---|---|---|---|---|---|---|
| k: epoll_wait | 0.148 | 0.764 | 0.702 | 0.592 | -0.616 | -0.061 | -0.171 |
| k: eventfd | 0.002 | 0.025 | 0.011 | 0.009 | -0.024 | -0.014 | -0.016 |
| k: futex | 0.005 | 0.330 | 0.242 | 0.254 | -0.324 | -0.087 | -0.076 |
| k: other | 0.000 | 0.000 | 0.002 | 0.000 | +0.000 | +0.002 | +0.000 |
| k: other syscalls | 0.125 | 0.721 | 0.675 | 0.907 | -0.596 | -0.046 | +0.186 |
| k: page faults (the fault path) | 0.231 | 0.937 | 0.927 | 1.534 | -0.706 | -0.009 | +0.597 |
| k: scheduling, interrupts | 0.024 | 0.027 | 0.025 | 0.037 | -0.003 | -0.002 | +0.010 |
| k: socket read (recvmsg/read) | 3.468 | 3.968 | 4.044 | 4.228 | -0.500 | +0.075 | +0.259 |
| k: socket write (sendmsg/writev) | 0.036 | 0.048 | 0.062 | 0.053 | -0.012 | +0.014 | +0.005 |
| u: allocation (malloc/free) | 0.408 | 0.570 | 0.631 | 0.474 | -0.162 | +0.061 | -0.096 |
| u: hyper/h2/tonic/http/bytes | 0.576 | 0.819 | 0.893 | 0.762 | -0.244 | +0.074 | -0.057 |
| u: libc sync (pthread, futex wrappers) | 0.000 | 0.010 | 0.004 | 0.009 | -0.010 | -0.006 | -0.001 |
| u: libc syscall wrappers | 0.068 | 0.066 | 0.094 | 0.112 | +0.002 | +0.028 | +0.046 |
| u: memcpy in hyper/h2/bytes | 2.037 | 2.551 | 2.552 | 2.631 | -0.514 | +0.001 | +0.080 |
| u: other | 0.055 | 0.104 | 0.118 | 0.104 | -0.049 | +0.014 | +0.000 |
| u: tokio/mio runtime and park | 0.186 | 0.332 | 0.317 | 0.321 | -0.146 | -0.015 | -0.011 |
| **total (sampled)** | 7.369 | 11.272 | 11.299 | 12.028 | -3.903 | +0.027 | +0.756 |

Threads (ms per call, sampled): A cur SERVER: tokio-rt-worker 7.369; Cf-retain cur SERVER: tokio-rt-worker 11.272; Cf-q-retain cur SERVER: tokio-rt-worker 11.299; Cf-q-retain q1slot SERVER: tokio-rt-worker 12.028

Top symbols, d/16MiB k=1 A cur SERVER (ms per call): __memmove_evex_unaligned_erms 2.037; [k] rep_movs_alternative 1.071; [k] syscall_return_via_sysret 0.434; [k] entry_SYSRETQ_unsafe_stack 0.280; [k] entry_SYSCALL_64_after_hwframe 0.195; [k] entry_SYSCALL_64 0.193; _int_malloc 0.129; [k] _raw_spin_lock 0.084; [k] asm_exc_page_fault 0.076; realloc 0.075; [k] fdget 0.074; [k] unix_stream_read_generic 0.065; [k] __check_object_size 0.055; [k] do_syscall_64 0.053; [k] _copy_to_iter 0.051; [k] unix_stream_recvmsg 0.050

Top symbols, d/16MiB k=1 Cf-retain cur SERVER (ms per call): __memmove_evex_unaligned_erms 2.548; [k] rep_movs_alternative 1.106; [k] syscall_return_via_sysret 0.555; [k] entry_SYSRETQ_unsafe_stack 0.330; [k] asm_exc_page_fault 0.298; [k] entry_SYSCALL_64_after_hwframe 0.266; [k] entry_SYSCALL_64 0.247; [k] swapgs_restore_regs_and_return_to_usermode 0.230; _int_malloc 0.157; [k] native_queued_spin_lock_slowpath 0.152; [k] __free_one_page 0.125; [k] error_entry 0.105; [k] _raw_spin_lock 0.103; [k] do_syscall_64 0.101; realloc 0.093; malloc_consolidate 0.092

Top symbols, d/16MiB k=1 Cf-q-retain cur SERVER (ms per call): __memmove_evex_unaligned_erms 2.552; [k] rep_movs_alternative 1.150; [k] syscall_return_via_sysret 0.502; [k] entry_SYSRETQ_unsafe_stack 0.355; [k] asm_exc_page_fault 0.300; [k] entry_SYSCALL_64_after_hwframe 0.277; [k] entry_SYSCALL_64 0.230; [k] swapgs_restore_regs_and_return_to_usermode 0.213; _int_malloc 0.183; [k] native_queued_spin_lock_slowpath 0.172; [k] error_entry 0.127; realloc 0.125; [k] _raw_spin_lock 0.104; [k] __check_object_size 0.086; [k] fdget 0.083; malloc_consolidate 0.082

Top symbols, d/16MiB k=1 Cf-q-retain q1slot SERVER (ms per call): __memmove_evex_unaligned_erms 2.629; [k] rep_movs_alternative 1.143; [k] asm_exc_page_fault 0.469; [k] syscall_return_via_sysret 0.426; [k] entry_SYSRETQ_unsafe_stack 0.366; [k] swapgs_restore_regs_and_return_to_usermode 0.287; [k] entry_SYSCALL_64_after_hwframe 0.284; [k] entry_SYSCALL_64 0.224; [k] error_entry 0.179; [k] native_queued_spin_lock_slowpath 0.177; [k] __list_del_entry_valid_or_report 0.139; [k] __free_one_page 0.125; [k] _raw_spin_lock 0.124; _int_malloc 0.111; [k] fdget 0.107; [k] clear_page_erms 0.100

### perf record buckets, d/16MiB k=8, the client (ms per call = cycles / 3.3e9; one process per cell)

| bucket | A cur | Cf-retain cur | Cf-q-retain cur | Cf-q-retain q1slot | A - Cf | Cf-q-retain - Cf | Cf-q-retain - Cf |
|---|---|---|---|---|---|---|---|
| k: epoll_wait | 0.127 | 0.107 | 0.252 | 0.213 | +0.020 | +0.145 | +0.106 |
| k: eventfd | 0.011 | 0.003 | 0.017 | 0.004 | +0.008 | +0.015 | +0.002 |
| k: futex | 0.563 | 0.127 | 0.172 | 0.129 | +0.437 | +0.046 | +0.002 |
| k: other | 0.133 | 0.049 | 0.000 | 0.000 | +0.084 | -0.049 | -0.049 |
| k: other syscalls | 0.137 | 0.029 | 0.026 | 0.028 | +0.109 | -0.003 | -0.001 |
| k: page faults (the fault path) | 0.011 | 0.024 | 0.013 | 0.000 | -0.014 | -0.012 | -0.024 |
| k: scheduling, interrupts | 0.055 | 0.029 | 0.017 | 0.018 | +0.026 | -0.012 | -0.011 |
| k: socket read (recvmsg/read) | 0.115 | 0.035 | 0.028 | 0.041 | +0.081 | -0.006 | +0.006 |
| k: socket write (sendmsg/writev) | 4.094 | 5.649 | 5.597 | 5.452 | -1.554 | -0.051 | -0.197 |
| u: allocation (malloc/free) | 0.059 | 0.019 | 0.029 | 0.018 | +0.040 | +0.010 | -0.001 |
| u: core rpc glue (ak_core rpc, FFI entry) | 0.000 | 0.035 | 0.079 | 0.061 | -0.035 | +0.044 | +0.025 |
| u: encode (core codec) | 0.000 | 0.004 | 0.003 | 0.001 | -0.004 | -0.001 | -0.003 |
| u: grpc-core (libgrpc, gpr, absl) | 1.392 | 0.000 | 0.000 | 0.000 | +1.392 | +0.000 | +0.000 |
| u: harness | 0.031 | 0.005 | 0.002 | 0.002 | +0.026 | -0.002 | -0.003 |
| u: hyper/h2/tonic/http/bytes | 0.000 | 0.375 | 0.388 | 0.362 | -0.375 | +0.012 | -0.013 |
| u: libc sync (pthread, futex wrappers) | 0.012 | 0.001 | 0.000 | 0.000 | +0.011 | -0.001 | -0.001 |
| u: libc syscall wrappers | 0.082 | 0.073 | 0.076 | 0.065 | +0.009 | +0.004 | -0.008 |
| u: memcpy in encode | 0.000 | 2.443 | 1.578 | 1.514 | -2.443 | -0.865 | -0.929 |
| u: memcpy in grpc/protobuf | 2.342 | 0.000 | 0.000 | 0.000 | +2.342 | +0.000 | +0.000 |
| u: memcpy in hyper/h2/bytes | 0.000 | 0.163 | 0.138 | 0.185 | -0.163 | -0.025 | +0.022 |
| u: memcpy other | 0.000 | 0.014 | 0.004 | 0.005 | -0.014 | -0.010 | -0.009 |
| u: other | 0.038 | 0.007 | 0.016 | 0.003 | +0.031 | +0.009 | -0.004 |
| u: protobuf | 0.012 | 0.000 | 0.000 | 0.000 | +0.012 | +0.000 | +0.000 |
| u: tokio/mio runtime and park | 0.000 | 0.201 | 0.254 | 0.239 | -0.201 | +0.052 | +0.037 |
| **total (sampled)** | 9.215 | 9.392 | 8.691 | 8.339 | -0.177 | -0.701 | -1.053 |

Threads (ms per call, sampled): A cur: event_engine 6.330, campaign_rpc 2.885; Cf-retain cur: tokio-rt-worker 6.732, campaign_rpc 2.660; Cf-q-retain cur: tokio-rt-worker 6.988, campaign_rpc 1.702; Cf-q-retain q1slot: tokio-rt-worker 6.730, campaign_rpc 1.610

Top symbols, d/16MiB k=8 A cur (ms per call): __memmove_evex_unaligned_erms 2.339; [k] rep_movs_alternative 2.001; [k] clear_page_erms 0.836; [k] memset_orig 0.122; [k] _copy_from_iter 0.116; [k] syscall_return_via_sysret 0.113; grpc_event_engine::experimental::PosixEndpointImpl::TcpFlush(absl::lts 0.109; absl::lts_20260107::Mutex::lock() 0.080; [k] entry_SYSRETQ_unsafe_stack 0.079; [k] __alloc_tagging_slab_alloc_hook 0.074; [k] _raw_spin_lock 0.058; [k] dequeue_entities 0.053; grpc_event_engine::experimental::SliceBuffer::RefSlice(unsigned long) 0.053; [k] __list_del_entry_valid_or_report 0.046; [k] _raw_spin_lock_irqsave 0.046; [k] get_page_from_freelist 0.044

Top symbols, d/16MiB k=8 Cf-retain cur (ms per call): __memmove_evex_unaligned_erms 2.614; [k] rep_movs_alternative 1.846; [k] clear_page_erms 0.891; [k] native_queued_spin_lock_slowpath 0.360; [k] syscall_return_via_sysret 0.203; [k] entry_SYSRETQ_unsafe_stack 0.160; [k] _raw_spin_lock_irqsave 0.146; [k] entry_SYSCALL_64_after_hwframe 0.116; [k] _copy_from_iter 0.108; [k] __alloc_tagging_slab_alloc_hook 0.107; [k] entry_SYSCALL_64 0.079; [k] __pgalloc_tag_add 0.077; [k] _raw_spin_lock 0.076; h2::proto::streams::prioritize::Prioritize::buffer_pending 0.069; [k] __list_del_entry_valid_or_report 0.066; [k] sock_def_readable 0.062

Top symbols, d/16MiB k=8 Cf-q-retain cur (ms per call): [k] rep_movs_alternative 1.765; __memmove_evex_unaligned_erms 1.719; [k] clear_page_erms 0.931; [k] native_queued_spin_lock_slowpath 0.304; [k] syscall_return_via_sysret 0.234; [k] entry_SYSRETQ_unsafe_stack 0.169; [k] entry_SYSCALL_64_after_hwframe 0.134; [k] __alloc_tagging_slab_alloc_hook 0.118; [k] _raw_spin_lock_irqsave 0.112; [k] _raw_spin_lock 0.107; [k] _copy_from_iter 0.103; [k] entry_SYSCALL_64 0.102; h2::proto::streams::prioritize::Prioritize::buffer_pending 0.070; [k] try_charge_memcg 0.069; [k] vfs_writev 0.064; [k] __list_del_entry_valid_or_report 0.061

Top symbols, d/16MiB k=8 Cf-q-retain q1slot (ms per call): [k] rep_movs_alternative 1.705; __memmove_evex_unaligned_erms 1.703; [k] clear_page_erms 0.899; [k] native_queued_spin_lock_slowpath 0.365; [k] syscall_return_via_sysret 0.254; [k] entry_SYSRETQ_unsafe_stack 0.175; [k] _raw_spin_lock_irqsave 0.132; [k] entry_SYSCALL_64 0.115; [k] _copy_from_iter 0.115; [k] entry_SYSCALL_64_after_hwframe 0.111; [k] __alloc_tagging_slab_alloc_hook 0.110; [k] _raw_spin_lock 0.105; [k] __pgalloc_tag_add 0.072; [k] __list_del_entry_valid_or_report 0.065; [k] try_charge_memcg 0.064; h2::proto::streams::prioritize::Prioritize::buffer_pending 0.062

### perf record buckets, d/16MiB k=8, the server process (ms per call = cycles / 3.3e9; one process per cell)

| bucket | A cur SERVER | Cf-retain cur SERVER | Cf-q-retain cur SERVER | Cf-q-retain q1slot SERVER | A - Cf | Cf-q-retain - Cf | Cf-q-retain - Cf |
|---|---|---|---|---|---|---|---|
| k: epoll_wait | 0.379 | 1.900 | 1.841 | 1.819 | -1.521 | -0.059 | -0.080 |
| k: eventfd | 0.009 | 0.175 | 0.196 | 0.227 | -0.166 | +0.021 | +0.052 |
| k: futex | 0.253 | 1.967 | 2.079 | 2.023 | -1.714 | +0.113 | +0.056 |
| k: other syscalls | 0.545 | 0.968 | 0.968 | 0.893 | -0.423 | +0.000 | -0.075 |
| k: page faults (the fault path) | 0.828 | 0.721 | 0.454 | 0.527 | +0.107 | -0.267 | -0.193 |
| k: scheduling, interrupts | 0.038 | 0.051 | 0.035 | 0.040 | -0.013 | -0.015 | -0.011 |
| k: socket read (recvmsg/read) | 3.780 | 4.571 | 4.488 | 4.632 | -0.792 | -0.084 | +0.060 |
| k: socket write (sendmsg/writev) | 0.057 | 0.035 | 0.029 | 0.022 | +0.022 | -0.006 | -0.013 |
| u: allocation (malloc/free) | 0.574 | 0.919 | 0.883 | 0.799 | -0.345 | -0.036 | -0.120 |
| u: hyper/h2/tonic/http/bytes | 0.764 | 1.433 | 1.531 | 1.481 | -0.669 | +0.099 | +0.048 |
| u: libc sync (pthread, futex wrappers) | 0.010 | 0.023 | 0.027 | 0.026 | -0.013 | +0.004 | +0.003 |
| u: libc syscall wrappers | 0.109 | 0.232 | 0.217 | 0.235 | -0.124 | -0.015 | +0.003 |
| u: memcpy in hyper/h2/bytes | 2.820 | 3.581 | 3.649 | 3.672 | -0.760 | +0.068 | +0.091 |
| u: other | 0.154 | 0.361 | 0.366 | 0.442 | -0.207 | +0.005 | +0.081 |
| u: tokio/mio runtime and park | 0.328 | 1.411 | 1.549 | 1.468 | -1.083 | +0.138 | +0.058 |
| **total (sampled)** | 10.646 | 18.346 | 18.312 | 18.305 | -7.700 | -0.035 | -0.041 |

Threads (ms per call, sampled): A cur SERVER: tokio-rt-worker 10.646; Cf-retain cur SERVER: tokio-rt-worker 18.346; Cf-q-retain cur SERVER: tokio-rt-worker 18.312; Cf-q-retain q1slot SERVER: tokio-rt-worker 18.305

Top symbols, d/16MiB k=8 A cur SERVER (ms per call): __memmove_evex_unaligned_erms 2.819; [k] rep_movs_alternative 1.162; [k] syscall_return_via_sysret 0.491; [k] entry_SYSRETQ_unsafe_stack 0.326; [k] entry_SYSCALL_64_after_hwframe 0.283; [k] asm_exc_page_fault 0.232; [k] entry_SYSCALL_64 0.214; [k] swapgs_restore_regs_and_return_to_usermode 0.165; _int_malloc 0.162; [k] _raw_spin_lock 0.121; realloc 0.108; [k] error_entry 0.099; [k] __check_object_size 0.090; [k] do_syscall_64 0.088; [k] unix_stream_read_generic 0.084; [k] fdget 0.083

Top symbols, d/16MiB k=8 Cf-retain cur SERVER (ms per call): __memmove_evex_unaligned_erms 3.581; [k] rep_movs_alternative 1.261; [k] syscall_return_via_sysret 0.879; [k] entry_SYSRETQ_unsafe_stack 0.675; [k] entry_SYSCALL_64_after_hwframe 0.574; [k] entry_SYSCALL_64 0.384; [k] native_queued_spin_lock_slowpath 0.326; [k] dequeue_entities 0.220; _int_malloc 0.207; [k] asm_exc_page_fault 0.202; [k] _raw_spin_lock 0.187; realloc 0.168; [k] do_syscall_64 0.153; _int_free_chunk 0.153; [k] swapgs_restore_regs_and_return_to_usermode 0.142; tokio::runtime::scheduler::multi_thread::queue::Steal<T>::steal_into 0.128

Top symbols, d/16MiB k=8 Cf-q-retain cur SERVER (ms per call): __memmove_evex_unaligned_erms 3.649; [k] rep_movs_alternative 1.306; [k] syscall_return_via_sysret 0.926; [k] entry_SYSRETQ_unsafe_stack 0.674; [k] entry_SYSCALL_64_after_hwframe 0.545; [k] entry_SYSCALL_64 0.383; [k] native_queued_spin_lock_slowpath 0.316; [k] dequeue_entities 0.206; [k] _raw_spin_lock 0.195; _int_malloc 0.176; [k] do_syscall_64 0.172; tokio::runtime::context::scoped::Scoped<T>::set 0.158; _int_free_chunk 0.151; [k] asm_exc_page_fault 0.142; <std::sys::sync::mutex::futex::Mutex>::lock_contended 0.139; realloc 0.136

Top symbols, d/16MiB k=8 Cf-q-retain q1slot SERVER (ms per call): __memmove_evex_unaligned_erms 3.672; [k] rep_movs_alternative 1.292; [k] syscall_return_via_sysret 0.933; [k] entry_SYSRETQ_unsafe_stack 0.651; [k] entry_SYSCALL_64_after_hwframe 0.517; [k] entry_SYSCALL_64 0.413; [k] native_queued_spin_lock_slowpath 0.321; [k] dequeue_entities 0.229; [k] _raw_spin_lock 0.196; [k] do_syscall_64 0.191; realloc 0.154; _int_malloc 0.143; [k] asm_exc_page_fault 0.141; [k] update_load_avg 0.132; [k] fdget 0.130; tokio::runtime::context::scoped::Scoped<T>::set 0.129
